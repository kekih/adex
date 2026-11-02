//! High-level session orchestration.

use crate::adb::{AdbClient, Device};
use crate::scrcpy::{ControlMessage, ScrcpyServer, VideoPacket};
use crate::{Error, Result};
use bytes::BytesMut;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_size: u32,
    pub bit_rate: u32,
    pub max_fps: u32,
    pub server_jar: PathBuf,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_size: 1920,
            bit_rate: 8_000_000,
            max_fps: 60,
            server_jar: PathBuf::from("scrcpy-server"),
        }
    }
}

#[derive(Debug)]
pub enum SessionEvent {
    Connected {
        device_name: String,
        width: u32,
        height: u32,
    },
    Video(VideoPacket),
    Disconnected,
    Error(String),
}

/// Represents a live mirroring session.
pub struct Session {
    pub device: Device,
    config: SessionConfig,
    adb: AdbClient,
    control_tx: Option<mpsc::Sender<ControlMessage>>,
}

impl Session {
    pub async fn connect(
        device: Device,
        config: SessionConfig,
    ) -> Result<(Self, mpsc::Receiver<SessionEvent>)> {
        let adb = AdbClient::new()?;
        let server = ScrcpyServer::new(adb.clone(), device.serial.clone());

        // Push server
        server.push_server(&config.server_jar).await?;

        // Start server process
        let _child = server
            .start(config.max_size, config.bit_rate, config.max_fps)
            .await?;

        // Connect to the forwarded port
        let stream = TcpStream::connect("127.0.0.1:27183")
            .await
            .map_err(|e| Error::ServerStart(format!("failed to connect to server: {e}")))?;

        let (mut reader, mut writer) = stream.into_split();

        // Read device meta (64-byte name + width + height)
        let mut meta_buf = [0u8; 64 + 8];
        reader
            .read_exact(&mut meta_buf)
            .await
            .map_err(|e| Error::Protocol(format!("handshake failed: {e}")))?;

        let meta =
            crate::scrcpy::protocol::DeviceMeta::decode(bytes::Bytes::copy_from_slice(&meta_buf))?;
        info!(
            device = %meta.device_name,
            width = meta.width,
            height = meta.height,
            "session connected"
        );

        let (event_tx, event_rx) = mpsc::channel(64);
        let (control_tx, mut control_rx) = mpsc::channel::<ControlMessage>(32);

        let _ = event_tx
            .send(SessionEvent::Connected {
                device_name: meta.device_name.clone(),
                width: meta.width,
                height: meta.height,
            })
            .await;

        // Video reader task
        let event_tx2 = event_tx.clone();
        tokio::spawn(async move {
            let mut header = [0u8; 12];
            loop {
                if let Err(e) = reader.read_exact(&mut header).await {
                    let _ = event_tx2
                        .send(SessionEvent::Error(format!("video read error: {e}")))
                        .await;
                    let _ = event_tx2.send(SessionEvent::Disconnected).await;
                    break;
                }

                let size =
                    u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
                let mut payload = vec![0u8; size];
                if let Err(e) = reader.read_exact(&mut payload).await {
                    let _ = event_tx2
                        .send(SessionEvent::Error(format!("payload read error: {e}")))
                        .await;
                    break;
                }

                let mut full = BytesMut::with_capacity(12 + size);
                full.extend_from_slice(&header);
                full.extend_from_slice(&payload);

                match VideoPacket::decode(full.freeze()) {
                    Ok(pkt) => {
                        if event_tx2.send(SessionEvent::Video(pkt)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "failed to decode video packet");
                    }
                }
            }
        });

        // Control writer task
        tokio::spawn(async move {
            while let Some(msg) = control_rx.recv().await {
                let data = msg.encode();
                if let Err(e) = writer.write_all(&data).await {
                    error!(error = %e, "control write failed");
                    break;
                }
            }
        });

        Ok((
            Self {
                device,
                config,
                adb,
                control_tx: Some(control_tx),
            },
            event_rx,
        ))
    }

    pub async fn send_control(&self, msg: ControlMessage) -> Result<()> {
        if let Some(tx) = &self.control_tx {
            tx.send(msg)
                .await
                .map_err(|_| Error::Other("control channel closed".into()))?;
        }
        Ok(())
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}

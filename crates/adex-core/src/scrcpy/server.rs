//! Lifecycle of the scrcpy-server process on the device.

use crate::adb::AdbClient;
use crate::{Error, Result};
use std::path::Path;
use tracing::{info, warn};

const SERVER_DEVICE_PATH: &str = "/data/local/tmp/adex-server.jar";

/// Manages pushing and starting the scrcpy server jar on the device.
#[derive(Debug)]
pub struct ScrcpyServer {
    adb: AdbClient,
    serial: String,
}

impl ScrcpyServer {
    pub fn new(adb: AdbClient, serial: impl Into<String>) -> Self {
        Self {
            adb,
            serial: serial.into(),
        }
    }

    /// Push the server jar to the device.
    pub async fn push_server(&self, local_jar: &Path) -> Result<()> {
        if !local_jar.exists() {
            return Err(Error::ServerStart(format!(
                "server jar not found: {}",
                local_jar.display()
            )));
        }

        info!(path = %local_jar.display(), "pushing scrcpy server");
        self.adb
            .push(
                &self.serial,
                &local_jar.to_string_lossy(),
                SERVER_DEVICE_PATH,
            )
            .await?;
        Ok(())
    }

    /// Build the command line that starts the server.
    pub fn build_start_command(
        &self,
        max_size: u32,
        bit_rate: u32,
        max_fps: u32,
        _tunnel_forward: bool,
    ) -> String {
        let args = [
            format!("CLASSPATH={SERVER_DEVICE_PATH}"),
            "app_process".into(),
            "/".into(),
            "com.genymobile.scrcpy.Server".into(),
            "3.3.3".into(),
            "log_level=info".into(),
            format!("max_size={max_size}"),
            format!("bit_rate={bit_rate}"),
            format!("max_fps={max_fps}"),
            "tunnel_forward=true".into(),
            "send_frame_meta=true".into(),
            "control=true".into(),
            "audio=false".into(),
            "video=true".into(),
        ];

        args.join(" ")
    }

    /// Start the server process (returns the child handle).
    pub async fn start(
        &self,
        max_size: u32,
        bit_rate: u32,
        max_fps: u32,
    ) -> Result<tokio::process::Child> {
        let cmd = self.build_start_command(max_size, bit_rate, max_fps, true);
        info!(%cmd, "starting scrcpy-server");

        // Clean any previous forward
        let _ = self.adb.remove_forward(&self.serial, "tcp:27183").await;

        // Forward local port to the abstract socket the server will open
        self.adb
            .forward(&self.serial, "tcp:27183", "localabstract:scrcpy")
            .await?;

        let child = self.adb.shell_exec(&self.serial, &cmd).await?;

        // Give the server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        Ok(child)
    }

    pub async fn cleanup(&self) {
        if let Err(e) = self.adb.remove_forward(&self.serial, "tcp:27183").await {
            warn!(error = %e, "failed to remove forward");
        }
    }
}

//! Scrcpy wire protocol primitives.
//!
//! Based on scrcpy v2.x / v3.x device → client video + control channels.
//! See: https://github.com/Genymobile/scrcpy/blob/master/doc/develop.md

use bytes::{Buf, BufMut, Bytes, BytesMut};
use crate::{Error, Result};

/// Device name is sent as a fixed 64-byte UTF-8 string (padded with zeros).
pub const DEVICE_NAME_FIELD_LENGTH: usize = 64;

/// Initial handshake packet from device.
#[derive(Debug, Clone)]
pub struct DeviceMeta {
    pub device_name: String,
    pub width: u32,
    pub height: u32,
}

impl DeviceMeta {
    pub fn decode(mut buf: Bytes) -> Result<Self> {
        if buf.remaining() < DEVICE_NAME_FIELD_LENGTH + 8 {
            return Err(Error::Protocol("handshake too short".into()));
        }

        let name_bytes = buf.copy_to_bytes(DEVICE_NAME_FIELD_LENGTH);
        let device_name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();

        let width = buf.get_u32();
        let height = buf.get_u32();

        Ok(Self {
            device_name,
            width,
            height,
        })
    }
}

/// One video frame from the device (raw H.264 NAL units).
#[derive(Debug, Clone)]
pub struct VideoPacket {
    pub pts: u64,
    pub is_config: bool,
    pub is_keyframe: bool,
    pub data: Bytes,
}

impl VideoPacket {
    /// Frame header is 12 bytes:
    /// - pts (u64, big-endian) with flags in high bits
    /// - packet size (u32)
    pub fn decode(mut buf: Bytes) -> Result<Self> {
        if buf.remaining() < 12 {
            return Err(Error::Protocol("video header too short".into()));
        }

        let pts_and_flags = buf.get_u64();
        let size = buf.get_u32() as usize;

        let is_config = (pts_and_flags & (1u64 << 63)) != 0;
        let is_keyframe = (pts_and_flags & (1u64 << 62)) != 0;
        let pts = pts_and_flags & !(3u64 << 62);

        if buf.remaining() < size {
            return Err(Error::Protocol(format!(
                "video payload truncated: need {size}, have {}",
                buf.remaining()
            )));
        }

        let data = buf.copy_to_bytes(size);

        Ok(Self {
            pts,
            is_config,
            is_keyframe,
            data,
        })
    }
}

/// Control messages sent from client → device.
#[derive(Debug, Clone)]
pub enum ControlMessage {
    InjectKeycode {
        action: u8, // 0 = down, 1 = up
        keycode: u32,
        repeat: u32,
        metastate: u32,
    },
    InjectText {
        text: String,
    },
    InjectTouch {
        action: u8, // 0=down, 1=up, 2=move
        pointer_id: u64,
        x: u32,
        y: u32,
        screen_width: u16,
        screen_height: u16,
        pressure: u16, // 0..=0xffff
        action_button: u32,
        buttons: u32,
    },
    InjectScroll {
        x: u32,
        y: u32,
        screen_width: u16,
        screen_height: u16,
        hscroll: i16,
        vscroll: i16,
        buttons: u32,
    },
    BackOrScreenOn {
        action: u8,
    },
    ExpandNotificationPanel,
    ExpandSettingsPanel,
    CollapsePanels,
    GetClipboard,
    SetClipboard {
        sequence: u64,
        paste: bool,
        text: String,
    },
    SetScreenPowerMode {
        mode: u8, // 0 = off, 1 = normal
    },
    RotateDevice,
}

impl ControlMessage {
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(64);

        match self {
            ControlMessage::InjectKeycode {
                action,
                keycode,
                repeat,
                metastate,
            } => {
                buf.put_u8(0); // type
                buf.put_u8(*action);
                buf.put_u32(*keycode);
                buf.put_u32(*repeat);
                buf.put_u32(*metastate);
            }
            ControlMessage::InjectText { text } => {
                buf.put_u8(1);
                let bytes = text.as_bytes();
                buf.put_u32(bytes.len() as u32);
                buf.put_slice(bytes);
            }
            ControlMessage::InjectTouch {
                action,
                pointer_id,
                x,
                y,
                screen_width,
                screen_height,
                pressure,
                action_button,
                buttons,
            } => {
                buf.put_u8(2);
                buf.put_u8(*action);
                buf.put_u64(*pointer_id);
                buf.put_u32(*x);
                buf.put_u32(*y);
                buf.put_u16(*screen_width);
                buf.put_u16(*screen_height);
                buf.put_u16(*pressure);
                buf.put_u32(*action_button);
                buf.put_u32(*buttons);
            }
            ControlMessage::InjectScroll {
                x,
                y,
                screen_width,
                screen_height,
                hscroll,
                vscroll,
                buttons,
            } => {
                buf.put_u8(3);
                buf.put_u32(*x);
                buf.put_u32(*y);
                buf.put_u16(*screen_width);
                buf.put_u16(*screen_height);
                buf.put_i16(*hscroll);
                buf.put_i16(*vscroll);
                buf.put_u32(*buttons);
            }
            ControlMessage::BackOrScreenOn { action } => {
                buf.put_u8(4);
                buf.put_u8(*action);
            }
            ControlMessage::ExpandNotificationPanel => buf.put_u8(5),
            ControlMessage::ExpandSettingsPanel => buf.put_u8(6),
            ControlMessage::CollapsePanels => buf.put_u8(7),
            ControlMessage::GetClipboard => buf.put_u8(8),
            ControlMessage::SetClipboard {
                sequence,
                paste,
                text,
            } => {
                buf.put_u8(9);
                buf.put_u64(*sequence);
                buf.put_u8(if *paste { 1 } else { 0 });
                let bytes = text.as_bytes();
                buf.put_u32(bytes.len() as u32);
                buf.put_slice(bytes);
            }
            ControlMessage::SetScreenPowerMode { mode } => {
                buf.put_u8(10);
                buf.put_u8(*mode);
            }
            ControlMessage::RotateDevice => buf.put_u8(11),
        }

        buf.freeze()
    }
}

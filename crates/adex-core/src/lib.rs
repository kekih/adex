//! Adex Core — high-performance Android mirroring foundation.

pub mod adb;
pub mod scrcpy;
pub mod session;
pub mod util;

pub use adb::{AdbClient, Device, DeviceState};
pub use scrcpy::protocol::{ControlMessage, VideoPacket};
pub use session::{Session, SessionConfig, SessionEvent};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ADB error: {0}")]
    Adb(String),

    #[error("Device not found")]
    DeviceNotFound,

    #[error("No devices connected")]
    NoDevices,

    #[error("Scrcpy protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Server failed to start: {0}")]
    ServerStart(String),

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Other(e.to_string())
    }
}

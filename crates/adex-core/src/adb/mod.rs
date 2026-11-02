//! ADB abstraction layer.
//!
//! Supports system `adb` binary (default) and optional pure-Rust path.

mod client;
mod device;

pub use client::AdbClient;
pub use device::{Device, DeviceState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Device,
    Offline,
    Unauthorized,
    Bootloader,
    Recovery,
    Sideload,
    Unknown(String),
}

impl DeviceState {
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "device" => Self::Device,
            "offline" => Self::Offline,
            "unauthorized" => Self::Unauthorized,
            "bootloader" => Self::Bootloader,
            "recovery" => Self::Recovery,
            "sideload" => Self::Sideload,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Device)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub serial: String,
    pub state: DeviceState,
    pub product: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
}

impl Device {
    pub fn display_name(&self) -> String {
        self.model
            .clone()
            .or_else(|| self.product.clone())
            .unwrap_or_else(|| self.serial.clone())
    }
}

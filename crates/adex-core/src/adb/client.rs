use super::{Device, DeviceState};
use crate::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

/// Thin async wrapper around the system `adb` binary.
#[derive(Debug, Clone)]
pub struct AdbClient {
    adb_path: PathBuf,
}

impl AdbClient {
    pub fn new() -> Result<Self> {
        let path = which::which("adb").map_err(|_| {
            Error::Adb(
                "adb not found in PATH. Install Android platform-tools or set ADB path.".into(),
            )
        })?;
        Ok(Self { adb_path: path })
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            adb_path: path.into(),
        }
    }

    async fn run(&self, args: &[&str]) -> Result<(i32, String, String)> {
        debug!(?args, "adb");
        let output = Command::new(&self.adb_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok((status, stdout, stderr))
    }

    pub async fn devices(&self) -> Result<Vec<Device>> {
        let (status, stdout, stderr) = self.run(&["devices", "-l"]).await?;
        if status != 0 {
            return Err(Error::Adb(format!("adb devices failed: {stderr}")));
        }

        let mut devices = Vec::new();
        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(serial) = parts.next() else {
                continue;
            };
            let Some(state_str) = parts.next() else {
                continue;
            };

            let mut product = None;
            let mut model = None;
            let mut device = None;
            let mut transport_id = None;

            for part in parts {
                if let Some(v) = part.strip_prefix("product:") {
                    product = Some(v.to_string());
                } else if let Some(v) = part.strip_prefix("model:") {
                    model = Some(v.to_string());
                } else if let Some(v) = part.strip_prefix("device:") {
                    device = Some(v.to_string());
                } else if let Some(v) = part.strip_prefix("transport_id:") {
                    transport_id = Some(v.to_string());
                }
            }

            devices.push(Device {
                serial: serial.to_string(),
                state: DeviceState::parse(state_str),
                product,
                model,
                device,
                transport_id,
            });
        }

        Ok(devices)
    }

    pub async fn wait_for_device(&self, serial: Option<&str>) -> Result<Device> {
        let mut args = vec!["wait-for-device"];
        if let Some(s) = serial {
            args.insert(0, "-s");
            args.insert(1, s);
        }
        let _ = self.run(&args).await?;

        let devices = self.devices().await?;
        let device = match serial {
            Some(s) => devices.into_iter().find(|d| d.serial == s),
            None => devices.into_iter().find(|d| d.state.is_ready()),
        };

        device.ok_or(Error::DeviceNotFound)
    }

    pub async fn shell(&self, serial: &str, cmd: &str) -> Result<String> {
        let (status, stdout, stderr) = self.run(&["-s", serial, "shell", cmd]).await?;
        if status != 0 {
            return Err(Error::Adb(format!("shell failed: {stderr}")));
        }
        Ok(stdout)
    }

    pub async fn push(&self, serial: &str, local: &str, remote: &str) -> Result<()> {
        let (status, _, stderr) = self.run(&["-s", serial, "push", local, remote]).await?;
        if status != 0 {
            return Err(Error::Adb(format!("push failed: {stderr}")));
        }
        Ok(())
    }

    pub async fn forward(&self, serial: &str, local: &str, remote: &str) -> Result<()> {
        let (status, _, stderr) = self
            .run(&["-s", serial, "forward", local, remote])
            .await?;
        if status != 0 {
            return Err(Error::Adb(format!("forward failed: {stderr}")));
        }
        Ok(())
    }

    pub async fn reverse(&self, serial: &str, remote: &str, local: &str) -> Result<()> {
        let (status, _, stderr) = self
            .run(&["-s", serial, "reverse", remote, local])
            .await?;
        if status != 0 {
            return Err(Error::Adb(format!("reverse failed: {stderr}")));
        }
        Ok(())
    }

    pub async fn remove_forward(&self, serial: &str, local: &str) -> Result<()> {
        let _ = self
            .run(&["-s", serial, "forward", "--remove", local])
            .await?;
        Ok(())
    }

    pub async fn shell_exec(&self, serial: &str, cmd: &str) -> Result<tokio::process::Child> {
        info!(%serial, %cmd, "starting shell process");
        let child = Command::new(&self.adb_path)
            .args(["-s", serial, "shell", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        Ok(child)
    }
}

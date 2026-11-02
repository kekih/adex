use adex_core::adb::AdbClient;
use adex_core::session::{Session, SessionConfig, SessionEvent};
use adex_core::Device;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "adex", version, about = "High-performance Android mirroring in pure Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to adb binary (default: search PATH)
    #[arg(long, global = true)]
    adb: Option<PathBuf>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List connected devices
    Devices,

    /// Start a mirroring session
    Mirror {
        /// Device serial (optional — uses first ready device)
        serial: Option<String>,

        /// Maximum video dimension
        #[arg(long, default_value_t = 1920)]
        max_size: u32,

        /// Video bitrate in bits per second
        #[arg(long, default_value_t = 8_000_000)]
        bit_rate: u32,

        /// Maximum frames per second
        #[arg(long, default_value_t = 60)]
        max_fps: u32,

        /// Path to scrcpy-server jar
        #[arg(long, default_value = "scrcpy-server")]
        server: PathBuf,
    },

    /// Show version / build info
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = match cli.verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(level.into())
                .from_env_lossy(),
        )
        .init();

    match cli.command {
        Commands::Devices => cmd_devices(cli.adb).await,
        Commands::Mirror {
            serial,
            max_size,
            bit_rate,
            max_fps,
            server,
        } => {
            cmd_mirror(cli.adb, serial, max_size, bit_rate, max_fps, server).await
        }
        Commands::Info => {
            println!("adex {}", env!("CARGO_PKG_VERSION"));
            println!("A high-performance Android desktop experience written in Rust.");
            println!("Repository: https://github.com/kekih/adex");
            Ok(())
        }
    }
}

async fn cmd_devices(adb_path: Option<PathBuf>) -> Result<()> {
    let adb = match adb_path {
        Some(p) => AdbClient::with_path(p),
        None => AdbClient::new().context("adb not found")?,
    };

    let devices = adb.devices().await.context("failed to list devices")?;

    if devices.is_empty() {
        println!("No devices found.");
        return Ok(());
    }

    println!("{:<24} {:<14} {}", "SERIAL", "STATE", "MODEL");
    println!("{:-<24} {:-<14} {:-<20}", "", "", "");
    for d in devices {
        println!(
            "{:<24} {:<14} {}",
            d.serial,
            format!("{:?}", d.state),
            d.display_name()
        );
    }
    Ok(())
}

async fn cmd_mirror(
    adb_path: Option<PathBuf>,
    serial: Option<String>,
    max_size: u32,
    bit_rate: u32,
    max_fps: u32,
    server: PathBuf,
) -> Result<()> {
    let adb = match adb_path {
        Some(p) => AdbClient::with_path(p),
        None => AdbClient::new().context("adb not found")?,
    };

    let devices = adb.devices().await?;
    let device = match serial {
        Some(s) => devices
            .into_iter()
            .find(|d| d.serial == s)
            .ok_or_else(|| anyhow::anyhow!("device {s} not found"))?,
        None => devices
            .into_iter()
            .find(|d| d.state.is_ready())
            .ok_or_else(|| anyhow::anyhow!("no ready device found"))?,
    };

    if !device.state.is_ready() {
        bail!("device {} is not ready (state: {:?})", device.serial, device.state);
    }

    info!(
        serial = %device.serial,
        model = %device.display_name(),
        "starting mirror session"
    );

    let config = SessionConfig {
        max_size,
        bit_rate,
        max_fps,
        server_jar: server,
    };

    let (session, mut events) = Session::connect(device, config).await?;

    println!("Session started for {}", session.device().display_name());
    println!("Press Ctrl+C to stop.");

    while let Some(ev) = events.recv().await {
        match ev {
            SessionEvent::Connected {
                device_name,
                width,
                height,
            } => {
                println!("Connected to {device_name} ({width}x{height})");
            }
            SessionEvent::Video(pkt) => {
                // In CLI mode we just log keyframes for now.
                // Full decode + render lives in the GUI path.
                if pkt.is_keyframe {
                    tracing::debug!(
                        pts = pkt.pts,
                        size = pkt.data.len(),
                        "keyframe"
                    );
                }
            }
            SessionEvent::Disconnected => {
                println!("Disconnected.");
                break;
            }
            SessionEvent::Error(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    Ok(())
}

mod app;
mod theme;

use app::AdexApp;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "adex",
    version,
    about = "High-performance Android mirroring in pure Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to adb binary (default: search PATH)
    #[arg(long, global = true)]
    adb: Option<PathBuf>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List connected devices (CLI)
    Devices,

    /// Start a mirroring session in CLI mode
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

fn main() -> eframe::Result<()> {
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

    // If a subcommand is given → pure CLI mode
    if let Some(cmd) = cli.command {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        return rt.block_on(async {
            match cmd {
                Commands::Devices => {
                    cli_devices(cli.adb).await;
                }
                Commands::Mirror {
                    serial,
                    max_size,
                    bit_rate,
                    max_fps,
                    server,
                } => {
                    cli_mirror(cli.adb, serial, max_size, bit_rate, max_fps, server).await;
                }
                Commands::Info => {
                    println!("adex {}", env!("CARGO_PKG_VERSION"));
                    println!("A high-performance Android desktop experience written in Rust.");
                    println!("Repository: https://github.com/kekih/adex");
                }
            }
            Ok(())
        });
    }

    // Default: launch the desktop GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Adex — Android Desktop Experience"),
        ..Default::default()
    };

    eframe::run_native(
        "Adex",
        options,
        Box::new(|cc| Ok(Box::new(AdexApp::new(cc)))),
    )
}

async fn cli_devices(adb_path: Option<PathBuf>) {
    use adex_core::adb::AdbClient;
    let adb = match adb_path {
        Some(p) => AdbClient::with_path(p),
        None => match AdbClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        },
    };

    match adb.devices().await {
        Ok(devices) if devices.is_empty() => println!("No devices found."),
        Ok(devices) => {
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
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cli_mirror(
    adb_path: Option<PathBuf>,
    serial: Option<String>,
    max_size: u32,
    bit_rate: u32,
    max_fps: u32,
    server: PathBuf,
) {
    use adex_core::adb::AdbClient;
    use adex_core::session::{Session, SessionConfig, SessionEvent};

    let adb = match adb_path {
        Some(p) => AdbClient::with_path(p),
        None => match AdbClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        },
    };

    let devices = match adb.devices().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let device = match serial {
        Some(s) => devices.into_iter().find(|d| d.serial == s),
        None => devices.into_iter().find(|d| d.state.is_ready()),
    };

    let Some(device) = device else {
        eprintln!("No suitable device found");
        return;
    };

    if !device.state.is_ready() {
        eprintln!("Device {} is not ready", device.serial);
        return;
    }

    let config = SessionConfig {
        max_size,
        bit_rate,
        max_fps,
        server_jar: server,
    };

    match Session::connect(device, config).await {
        Ok((session, mut events)) => {
            println!("Session started for {}", session.device().display_name());
            println!("Press Ctrl+C to stop.");
            while let Some(ev) = events.recv().await {
                match ev {
                    SessionEvent::Connected {
                        device_name,
                        width,
                        height,
                    } => println!("Connected to {device_name} ({width}x{height})"),
                    SessionEvent::Video(pkt) if pkt.is_keyframe => {
                        tracing::debug!(pts = pkt.pts, size = pkt.data.len(), "keyframe");
                    }
                    SessionEvent::Disconnected => {
                        println!("Disconnected.");
                        break;
                    }
                    SessionEvent::Error(e) => {
                        eprintln!("Error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => eprintln!("Failed to start session: {e}"),
    }
}

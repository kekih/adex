//! Main application — desktop shell inspired by Android DeX.

use crate::theme;
use adex_core::adb::{AdbClient, Device};
use egui::{self, Align, Color32, Frame, Layout, RichText, Sense, Ui, Vec2};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavItem {
    Home,
    Devices,
    Mirror,
    Apps,
    Media,
    Settings,
}

struct DeviceCache {
    devices: Vec<Device>,
    last_refresh: Instant,
    error: Option<String>,
}

pub struct AdexApp {
    nav: NavItem,
    device_cache: Arc<Mutex<DeviceCache>>,
    selected_serial: Option<String>,
    status_message: String,
    connected: bool,
    mirror_title: String,
    show_sidebar: bool,
}

impl AdexApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        // Prefer a nice font if available, otherwise default
        let mut fonts = egui::FontDefinitions::default();
        cc.egui_ctx.set_fonts(fonts);

        let cache = Arc::new(Mutex::new(DeviceCache {
            devices: Vec::new(),
            last_refresh: Instant::now() - Duration::from_secs(60),
            error: None,
        }));

        // Background refresh of devices
        let cache_clone = cache.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().ok();
            loop {
                if let Some(rt) = &rt {
                    let result = rt.block_on(async {
                        match AdbClient::new() {
                            Ok(adb) => adb.devices().await.map_err(|e| e.to_string()),
                            Err(e) => Err(e.to_string()),
                        }
                    });
                    if let Ok(mut guard) = cache_clone.lock() {
                        match result {
                            Ok(devs) => {
                                guard.devices = devs;
                                guard.error = None;
                            }
                            Err(e) => guard.error = Some(e),
                        }
                        guard.last_refresh = Instant::now();
                    }
                }
                std::thread::sleep(Duration::from_secs(3));
            }
        });

        Self {
            nav: NavItem::Home,
            device_cache: cache,
            selected_serial: None,
            status_message: "Ready".into(),
            connected: false,
            mirror_title: "No active session".into(),
            show_sidebar: true,
        }
    }

    fn devices(&self) -> Vec<Device> {
        self.device_cache
            .lock()
            .map(|g| g.devices.clone())
            .unwrap_or_default()
    }

    fn adb_error(&self) -> Option<String> {
        self.device_cache.lock().ok().and_then(|g| g.error.clone())
    }
}

impl eframe::App for AdexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint so device list stays fresh
        ctx.request_repaint_after(Duration::from_millis(500));

        // Top status bar
        egui::TopBottomPanel::top("top_bar")
            .exact_height(44.0)
            .frame(Frame::none().fill(theme::BG_PANEL).stroke(egui::Stroke::new(
                1.0,
                theme::BORDER,
            )))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Adex")
                            .strong()
                            .size(18.0)
                            .color(theme::ACCENT_SOFT),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("•").color(theme::TEXT_DIM));
                    ui.label(
                        RichText::new("Android Desktop Experience")
                            .size(13.0)
                            .color(theme::TEXT_DIM),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(12.0);

                        // Connection indicator
                        let (dot, label) = if self.connected {
                            (theme::SUCCESS, "Connected")
                        } else {
                            (theme::TEXT_DIM, "Disconnected")
                        };
                        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_DIM));
                        ui.colored_label(dot, "●");

                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(&self.status_message)
                                .size(12.0)
                                .color(theme::TEXT_DIM),
                        );
                    });
                });
            });

        // Left sidebar (DeX-style navigation)
        if self.show_sidebar {
            egui::SidePanel::left("sidebar")
                .exact_width(200.0)
                .frame(Frame::none().fill(theme::BG_PANEL))
                .show(ctx, |ui| {
                    ui.add_space(12.0);
                    self.nav_button(ui, NavItem::Home, "⌂  Home");
                    self.nav_button(ui, NavItem::Devices, "📱  Devices");
                    self.nav_button(ui, NavItem::Mirror, "🖥  Mirror");
                    self.nav_button(ui, NavItem::Apps, "▣  Apps");
                    self.nav_button(ui, NavItem::Media, "♪  Media");
                    self.nav_button(ui, NavItem::Settings, "⚙  Settings");

                    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(4.0);
                    });
                });
        }

        // Central content
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::BG_DARK).inner_margin(16.0))
            .show(ctx, |ui| match self.nav {
                NavItem::Home => self.ui_home(ui),
                NavItem::Devices => self.ui_devices(ui),
                NavItem::Mirror => self.ui_mirror(ui),
                NavItem::Apps => self.ui_apps(ui),
                NavItem::Media => self.ui_media(ui),
                NavItem::Settings => self.ui_settings(ui),
            });
    }
}

impl AdexApp {
    fn nav_button(&mut self, ui: &mut Ui, item: NavItem, label: &str) {
        let selected = self.nav == item;
        let fill = if selected {
            theme::ACCENT.linear_multiply(0.25)
        } else {
            Color32::TRANSPARENT
        };
        let text_color = if selected {
            theme::ACCENT_SOFT
        } else {
            theme::TEXT
        };

        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(14.0).color(text_color))
                .fill(fill)
                .min_size(Vec2::new(176.0, 36.0))
                .rounding(8.0),
        );
        if resp.clicked() {
            self.nav = item;
        }
    }

    fn ui_home(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Welcome to Adex").size(26.0).color(theme::TEXT));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Turn your Android phone into a desktop experience.")
                .size(14.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(24.0);

        // Quick cards
        ui.horizontal(|ui| {
            self.card(ui, "Devices", &format!("{}", self.devices().len()), "Connected phones");
            ui.add_space(12.0);
            self.card(
                ui,
                "Session",
                if self.connected { "Active" } else { "Idle" },
                "Mirror status",
            );
            ui.add_space(12.0);
            self.card(ui, "Latency", "—", "Target < 30 ms");
        });

        ui.add_space(28.0);

        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Quick start").strong().size(16.0));
            ui.add_space(8.0);
            ui.label("1. Enable USB debugging (or Wireless debugging) on your phone");
            ui.label("2. Connect the device and authorize the computer");
            ui.label("3. Open the Devices tab and select your phone");
            ui.label("4. Go to Mirror and start the session");
            ui.add_space(8.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("Open Devices").color(Color32::WHITE))
                        .fill(theme::ACCENT)
                        .rounding(8.0)
                        .min_size(Vec2::new(140.0, 34.0)),
                )
                .clicked()
            {
                self.nav = NavItem::Devices;
            }
        });
    }

    fn card(&self, ui: &mut Ui, title: &str, value: &str, subtitle: &str) {
        Frame::none()
            .fill(theme::BG_CARD)
            .rounding(12.0)
            .inner_margin(16.0)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(160.0, 90.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(12.0).color(theme::TEXT_DIM));
                    ui.add_space(4.0);
                    ui.label(RichText::new(value).size(22.0).strong().color(theme::TEXT));
                    ui.add_space(2.0);
                    ui.label(RichText::new(subtitle).size(11.0).color(theme::TEXT_DIM));
                });
            });
    }

    fn ui_devices(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Devices").size(22.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.status_message = "Refreshing…".into();
                }
            });
        });
        ui.add_space(12.0);

        if let Some(err) = self.adb_error() {
            ui.colored_label(theme::ERROR, format!("ADB error: {err}"));
            ui.add_space(8.0);
            ui.label("Make sure platform-tools are installed and `adb` is in PATH.");
            return;
        }

        let devices = self.devices();
        if devices.is_empty() {
            ui.label(
                RichText::new("No devices detected. Connect a phone via USB or pair wirelessly.")
                    .color(theme::TEXT_DIM),
            );
            return;
        }

        for d in &devices {
            let selected = self.selected_serial.as_deref() == Some(&d.serial);
            let ready = d.state.is_ready();

            Frame::none()
                .fill(if selected {
                    theme::ACCENT.linear_multiply(0.18)
                } else {
                    theme::BG_CARD
                })
                .rounding(10.0)
                .inner_margin(14.0)
                .stroke(egui::Stroke::new(
                    1.0,
                    if selected {
                        theme::ACCENT
                    } else {
                        theme::BORDER
                    },
                ))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(d.display_name())
                                    .size(15.0)
                                    .strong()
                                    .color(theme::TEXT),
                            );
                            ui.label(
                                RichText::new(&d.serial)
                                    .size(12.0)
                                    .color(theme::TEXT_DIM),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let state_color = if ready {
                                theme::SUCCESS
                            } else {
                                theme::WARNING
                            };
                            ui.colored_label(state_color, format!("{:?}", d.state));
                            if ui
                                .add_enabled(
                                    ready,
                                    egui::Button::new("Select").rounding(6.0),
                                )
                                .clicked()
                            {
                                self.selected_serial = Some(d.serial.clone());
                                self.status_message =
                                    format!("Selected {}", d.display_name());
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
    }

    fn ui_mirror(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Mirror").size(22.0));
        ui.add_space(8.0);

        let selected = self.selected_serial.clone();
        if selected.is_none() {
            ui.label(
                RichText::new("Select a device first in the Devices tab.")
                    .color(theme::TEXT_DIM),
            );
            return;
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Start Session").color(Color32::WHITE))
                        .fill(theme::ACCENT)
                        .rounding(8.0)
                        .min_size(Vec2::new(130.0, 34.0)),
                )
                .clicked()
            {
                self.connected = true;
                self.mirror_title = format!("Mirroring {}", selected.as_ref().unwrap());
                self.status_message = "Session starting… (video pipeline coming next)".into();
            }
            if ui
                .add(
                    egui::Button::new("Stop")
                        .rounding(8.0)
                        .min_size(Vec2::new(80.0, 34.0)),
                )
                .clicked()
            {
                self.connected = false;
                self.mirror_title = "No active session".into();
                self.status_message = "Session stopped".into();
            }
        });

        ui.add_space(16.0);

        // Placeholder mirror surface — later this will be a wgpu texture
        Frame::none()
            .fill(Color32::from_rgb(12, 12, 16))
            .rounding(12.0)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .inner_margin(0.0)
            .show(ui, |ui| {
                let avail = ui.available_size();
                let size = Vec2::new(avail.x.max(400.0), (avail.x * 9.0 / 16.0).min(avail.y - 20.0));
                let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());

                ui.painter().rect_filled(rect, 12.0, Color32::from_rgb(12, 12, 16));

                let text = if self.connected {
                    format!("{}

Video surface ready
H.264 decode + render next", self.mirror_title)
                } else {
                    "Mirror surface\n\nStart a session to begin streaming".into()
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(16.0),
                    theme::TEXT_DIM,
                );
            });
    }

    fn ui_apps(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Apps").size(22.0));
        ui.add_space(8.0);
        ui.label(
            RichText::new("Per-app windows and launcher will live here (multi-window mode).")
                .color(theme::TEXT_DIM),
        );
        ui.add_space(16.0);

        // Fake app grid as visual placeholder
        ui.horizontal_wrapped(|ui| {
            for name in ["Chrome", "YouTube", "Settings", "Gallery", "Messages", "Camera"] {
                Frame::none()
                    .fill(theme::BG_CARD)
                    .rounding(12.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(100.0, 90.0));
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("▢").size(28.0).color(theme::ACCENT_SOFT));
                            ui.add_space(4.0);
                            ui.label(RichText::new(name).size(12.0));
                        });
                    });
                ui.add_space(8.0);
            }
        });
    }

    fn ui_media(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Media").size(22.0));
        ui.add_space(8.0);
        ui.label(
            RichText::new("Audio routing, now-playing and media controls (coming soon).")
                .color(theme::TEXT_DIM),
        );
    }

    fn ui_settings(&mut self, ui: &mut Ui) {
        ui.heading(RichText::new("Settings").size(22.0));
        ui.add_space(12.0);

        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Video").strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Max size");
                ui.label(RichText::new("1920").color(theme::TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label("Bitrate");
                ui.label(RichText::new("8 Mbps").color(theme::TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label("Max FPS");
                ui.label(RichText::new("60").color(theme::TEXT_DIM));
            });
        });

        ui.add_space(12.0);
        ui.checkbox(&mut self.show_sidebar, "Show sidebar");
    }
}

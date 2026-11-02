//! Visual theme inspired by modern desktop / Android DeX style.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(28, 28, 36);
pub const BG_CARD: Color32 = Color32::from_rgb(36, 36, 48);
pub const BG_HOVER: Color32 = Color32::from_rgb(48, 48, 64);
pub const ACCENT: Color32 = Color32::from_rgb(99, 102, 241);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(129, 140, 248);
pub const TEXT: Color32 = Color32::from_rgb(240, 240, 250);
pub const TEXT_DIM: Color32 = Color32::from_rgb(160, 160, 180);
pub const SUCCESS: Color32 = Color32::from_rgb(52, 211, 153);
pub const WARNING: Color32 = Color32::from_rgb(251, 191, 36);
pub const ERROR: Color32 = Color32::from_rgb(248, 113, 113);
pub const BORDER: Color32 = Color32::from_rgb(55, 55, 70);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = dark_visuals();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.window_fill = BG_PANEL;
    v.panel_fill = BG_DARK;
    v.extreme_bg_color = BG_DARK;
    v.faint_bg_color = BG_CARD;
    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.inactive.bg_fill = BG_CARD;
    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.open.bg_fill = BG_HOVER;
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(10);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);
    v.widgets.active.corner_radius = CornerRadius::same(6);
    v.override_text_color = Some(TEXT);
    v
}

//! MusicCraft's own colour scheme: deep slate panels, teal accents, amber
//! highlights. Deliberately distinct from any other product's skin.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BACKGROUND: Color32 = Color32::from_rgb(0x1b, 0x1f, 0x26);
pub const PANEL: Color32 = Color32::from_rgb(0x24, 0x29, 0x32);
pub const PANEL_DARK: Color32 = Color32::from_rgb(0x16, 0x19, 0x1f);
pub const RAISED: Color32 = Color32::from_rgb(0x2e, 0x34, 0x3f);
pub const RAISED_HOVER: Color32 = Color32::from_rgb(0x3a, 0x42, 0x50);
pub const LCD_BG: Color32 = Color32::from_rgb(0x0f, 0x12, 0x17);
pub const LCD_EDGE: Color32 = Color32::from_rgb(0x2a, 0x30, 0x3a);
pub const ACCENT: Color32 = Color32::from_rgb(0x2e, 0xc4, 0xb6);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x1d, 0x6f, 0x69);
pub const HIGHLIGHT: Color32 = Color32::from_rgb(0xff, 0xb3, 0x47);
pub const DANGER: Color32 = Color32::from_rgb(0xe8, 0x5d, 0x5d);
pub const TEXT: Color32 = Color32::from_rgb(0xd8, 0xde, 0xe9);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x93, 0xa3);
pub const STEP_A: Color32 = Color32::from_rgb(0x39, 0x40, 0x4d);
pub const STEP_B: Color32 = Color32::from_rgb(0x30, 0x36, 0x41);
pub const GRID_LINE: Color32 = Color32::from_rgb(0x2b, 0x31, 0x3b);
pub const GRID_BEAT: Color32 = Color32::from_rgb(0x38, 0x3f, 0x4b);
pub const GRID_BAR: Color32 = Color32::from_rgb(0x50, 0x5a, 0x6a);
pub const WHITE_KEY_ROW: Color32 = Color32::from_rgb(0x22, 0x27, 0x30);
pub const BLACK_KEY_ROW: Color32 = Color32::from_rgb(0x1c, 0x20, 0x27);
pub const METER_BG: Color32 = Color32::from_rgb(0x0c, 0x0e, 0x12);
pub const AUTOMATION: Color32 = Color32::from_rgb(0xe0, 0x8c, 0xff);
pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0xe0, 0x8a);

/// Distinct clip/channel colours, cycled by index.
pub const PALETTE: [Color32; 8] = [
    Color32::from_rgb(0x2e, 0xc4, 0xb6),
    Color32::from_rgb(0x6c, 0x8c, 0xf5),
    Color32::from_rgb(0xff, 0xb3, 0x47),
    Color32::from_rgb(0xc7, 0x7d, 0xf0),
    Color32::from_rgb(0x8b, 0xd4, 0x5c),
    Color32::from_rgb(0xf0, 0x7a, 0x9a),
    Color32::from_rgb(0x5c, 0xc8, 0xf0),
    Color32::from_rgb(0xe0, 0xd0, 0x5c),
];

/// Colour `index` (0..16) of the playlist track palette: the clip colours,
/// then darker versions of them.
#[must_use]
pub fn track_color(index: u8) -> Color32 {
    let base = palette(usize::from(index));
    if index >= 8 {
        base.gamma_multiply(0.6).to_opaque()
    } else {
        base
    }
}

/// Colour for item `index`.
#[must_use]
pub fn palette(index: usize) -> Color32 {
    PALETTE[index % PALETTE.len()]
}

/// Installs the MusicCraft visuals on a context.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = PANEL_DARK;
    visuals.faint_bg_color = RAISED;
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x3a, 0x42, 0x50));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.weak_bg_fill = RAISED;
    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x3a, 0x42, 0x50);
    visuals.widgets.active.weak_bg_fill = ACCENT_DIM;
    ctx.set_visuals(visuals);
}

/// Faint product name in the empty workspace.
pub fn watermark(ui: &egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().text(
        rect.right_bottom() - egui::vec2(16.0, 12.0),
        egui::Align2::RIGHT_BOTTOM,
        "MusicCraft",
        egui::FontId::proportional(28.0),
        Color32::from_rgba_unmultiplied(0xd8, 0xde, 0xe9, 14),
    );
}

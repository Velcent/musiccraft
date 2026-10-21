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
pub const STEP_A: Color32 = Color32::from_rgb(0x3d, 0x45, 0x53);
pub const STEP_B: Color32 = Color32::from_rgb(0x2a, 0x2f, 0x39);
pub const GRID_LINE: Color32 = Color32::from_rgb(0x2b, 0x31, 0x3b);
pub const GRID_BEAT: Color32 = Color32::from_rgb(0x38, 0x3f, 0x4b);
/// Faint lines joining a browser row to its parent folder.
pub const TREE_LINE: Color32 = GRID_BEAT;
pub const GRID_BAR: Color32 = Color32::from_rgb(0x50, 0x5a, 0x6a);
pub const WHITE_KEY_ROW: Color32 = Color32::from_rgb(0x22, 0x27, 0x30);
pub const BLACK_KEY_ROW: Color32 = Color32::from_rgb(0x1c, 0x20, 0x27);
pub const METER_BG: Color32 = Color32::from_rgb(0x0c, 0x0e, 0x12);
pub const AUTOMATION: Color32 = Color32::from_rgb(0xe0, 0x8c, 0xff);
pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0xe0, 0x8a);

// Role names from the interface style guide (`docs/spec/style/`). Some share
// a value with a token above; the name says what the colour is for.

/// Header rows of windows that are not focused.
pub const HEADER: Color32 = PANEL;
/// The focused window's header: `PANEL` lifted halfway toward `RAISED`.
pub const HEADER_ACTIVE: Color32 = Color32::from_rgb(0x29, 0x2e, 0x38);
/// The outer edge of a floating window, and the hairline round menus.
pub const WINDOW_EDGE: Color32 = RAISED_HOVER;
/// Menus, popups and tooltips: one step above the panel they open over.
pub const MENU_BG: Color32 = RAISED;
/// Separators and the edges of wells.
pub const SEPARATOR: Color32 = LCD_EDGE;
/// Text and glyphs of disabled controls: `TEXT_DIM` moved toward `PANEL`.
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x57, 0x5e, 0x6b);
/// Digits in display wells and live values.
pub const LCD_TEXT: Color32 = PLAYHEAD;
/// An off step under the pointer: one step lighter than `STEP_A`.
pub const STEP_HOVER: Color32 = Color32::from_rgb(0x48, 0x51, 0x60);
/// The column over the step that is playing: `PLAYHEAD` at low strength.
pub const STEP_PLAYING: Color32 = Color32::from_rgba_unmultiplied_const(0xff, 0xe0, 0x8a, 40);
/// The fill of a rubber-band selection: `ACCENT` at low strength.
pub const SELECTION_FILL: Color32 = Color32::from_rgba_unmultiplied_const(0x2e, 0xc4, 0xb6, 24);
/// Playlist lanes, one tone for every track.
pub const LANE: Color32 = WHITE_KEY_ROW;
/// Piano-roll keyboard keys and the seam between white keys.
pub const KEY_WHITE: Color32 = Color32::from_rgb(0xdf, 0xe3, 0xea);
pub const KEY_BLACK: Color32 = Color32::from_rgb(0x1e, 0x22, 0x29);
pub const KEY_SEAM: Color32 = Color32::from_rgb(0x9a, 0xa0, 0xab);
/// Clips with no colour of their own.
pub const CLIP_NEUTRAL: Color32 = Color32::from_rgb(0x4a, 0x53, 0x63);
/// The title strip and outline of a selected clip: near `TEXT`, faintly
/// toward `ACCENT`.
pub const CLIP_SELECTED: Color32 = Color32::from_rgb(0xd2, 0xec, 0xe8);
/// The body of a selected note: near-white, a little toward the accent, so a
/// selection reads on every channel colour.
pub const NOTE_SELECTED: Color32 = CLIP_SELECTED;
/// Shading over everything after a pattern's end.
pub const PAST_END: Color32 = shade(60);
/// Notes of other channels shown behind the edited one: one neutral tone.
pub const GHOST_NOTE: Color32 = Color32::from_rgba_unmultiplied_const(0x8a, 0x93, 0xa3, 70);
/// The first bar line of each bar group: one step stronger than `GRID_BAR`.
pub const GRID_GROUP: Color32 = Color32::from_rgb(0x64, 0x6f, 0x81);
/// The faint lighter wash over alternate bar groups.
pub const BAR_GROUP: Color32 = Color32::from_rgba_unmultiplied_const(0xff, 0xff, 0xff, 5);
/// The faint product name in the empty workspace.
pub const WATERMARK: Color32 = Color32::from_rgba_unmultiplied_const(0xd8, 0xde, 0xe9, 14);

/// Distinct clip/channel colours, cycled by index. The teal and the amber are
/// a step lighter than `ACCENT` and `HIGHLIGHT`, so content never shares a
/// signal colour.
pub const PALETTE: [Color32; 8] = [
    Color32::from_rgb(0x6c, 0xdc, 0xd0),
    Color32::from_rgb(0x6c, 0x8c, 0xf5),
    Color32::from_rgb(0xff, 0xcc, 0x7a),
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

/// The colour of piano-roll note group `index`, from the same 16 colours as
/// tracks and patterns so a pattern, its channel and its notes can match.
#[must_use]
pub fn note_color(index: u8) -> Color32 {
    track_color(index % 16)
}

/// Black at strength `alpha`: the one shading used to darken content (clip
/// title strips, past-the-end areas, edges drawn on colour).
#[must_use]
pub const fn shade(alpha: u8) -> Color32 {
    Color32::from_black_alpha(alpha)
}

/// `color` at strength `alpha` (0–255): how content and signal tokens are
/// drawn faintly (muted clips, loop shading, scale tints).
#[must_use]
pub fn at_strength(color: Color32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// `a` moved toward `b` by `t` (0 is `a`, 1 is `b`), opaque.
#[must_use]
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

/// A clip's body: its colour moved most of the way toward `LANE`, slightly
/// see-through so grid lines show.
#[must_use]
pub fn clip_body(color: Color32) -> Color32 {
    at_strength(mix(LANE, color, 0.22), 225)
}

/// A clip's content (waveform): its colour moved toward `TEXT`, the brightest
/// tone in the clip.
#[must_use]
pub fn clip_content(color: Color32) -> Color32 {
    mix(color, TEXT, 0.45)
}

/// Text that reads on `fill`: `TEXT` on dark fills, `BACKGROUND` on light ones.
#[must_use]
pub fn text_on(fill: Color32) -> Color32 {
    let luma =
        0.299 * f32::from(fill.r()) + 0.587 * f32::from(fill.g()) + 0.114 * f32::from(fill.b());
    if luma > 150.0 { BACKGROUND } else { TEXT }
}

/// Colour for item `index`.
#[must_use]
pub fn palette(index: usize) -> Color32 {
    PALETTE[index % PALETTE.len()]
}

/// The bundled fonts, with the monospace face as the last fallback for
/// ordinary text: the proportional face has no arrows (→) or small triangles
/// (▲ ▼), which would otherwise draw as empty boxes.
#[must_use]
pub fn fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let mono = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .and_then(|f| f.first())
        .cloned();
    if let (Some(mono), Some(proportional)) = (
        mono,
        fonts.families.get_mut(&egui::FontFamily::Proportional),
    ) {
        if !proportional.contains(&mono) {
            proportional.push(mono);
        }
    }
    fonts
}

/// Text sizes, in points (`docs/spec/style/design-language.md`, Typography).
/// LCD value lines.
pub const FONT_DISPLAY: f32 = 16.0;
/// Window header titles.
pub const FONT_TITLE: f32 = 13.0;
/// Knob captions, track and strip names, note names.
pub const FONT_LABEL: f32 = 11.0;
/// LCD captions, unit marks, empty-slot text.
pub const FONT_CAPTION: f32 = 8.5;
/// Small changing numbers (mono).
pub const FONT_VALUE: f32 = 10.0;

/// Installs the MusicCraft fonts and visuals on a context.
pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL;
    // Popups, menus and tooltips take the window fill; windows draw their own
    // frame (`window_frame`).
    visuals.window_fill = MENU_BG;
    visuals.extreme_bg_color = PANEL_DARK;
    visuals.faint_bg_color = RAISED;
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    // Tiled windows read as one surface: barely rounded, no shadow. Menus,
    // popups and tooltips lift off the window with a faint one.
    visuals.window_corner_radius = CornerRadius::same(2);
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = Stroke::new(1.0, WINDOW_EDGE);
    visuals.menu_corner_radius = CornerRadius::same(2);
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 2],
        blur: 6,
        spread: 0,
        color: shade(70),
    };
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, SEPARATOR);
    visuals.widgets.inactive.weak_bg_fill = RAISED;
    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.hovered.weak_bg_fill = RAISED_HOVER;
    visuals.widgets.active.weak_bg_fill = ACCENT_DIM;
    ctx.set_visuals(visuals);
}

/// The frame of a floating window: `PANEL` body, barely rounded, no shadow,
/// with a `WINDOW_EDGE` outline, or `ACCENT_DIM` when `focused`.
pub fn window_frame(focused: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(2)
        .inner_margin(6)
        .stroke(Stroke::new(
            1.0,
            if focused { ACCENT_DIM } else { WINDOW_EDGE },
        ))
}

/// The frame of a modal dialog: like a window, with one faint shadow so it
/// lifts off the workspace.
pub fn dialog_frame() -> egui::Frame {
    window_frame(false).shadow(egui::Shadow {
        offset: [0, 3],
        blur: 10,
        spread: 0,
        color: shade(90),
    })
}

/// Faint product name in the empty workspace.
pub fn watermark(ui: &egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().text(
        rect.right_bottom() - egui::vec2(16.0, 12.0),
        egui::Align2::RIGHT_BOTTOM,
        "MusicCraft",
        egui::FontId::proportional(28.0),
        WATERMARK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_never_shares_a_signal_colour() {
        for colour in PALETTE {
            for signal in [ACCENT, ACCENT_DIM, HIGHLIGHT, DANGER, AUTOMATION, PLAYHEAD] {
                assert_ne!(colour, signal);
            }
        }
    }

    #[test]
    fn note_groups_use_the_track_palette() {
        for group in 0..16 {
            assert_eq!(note_color(group), track_color(group));
        }
        assert_eq!(note_color(16), track_color(0));
    }

    #[test]
    fn step_hover_lifts_both_step_groups() {
        let lum = |c: Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
        assert!(lum(STEP_HOVER) > lum(STEP_A));
        assert!(lum(STEP_A) > lum(STEP_B));
        assert!(lum(HEADER_ACTIVE) > lum(HEADER) && lum(HEADER_ACTIVE) < lum(RAISED));
    }

    #[test]
    fn clip_content_is_brighter_than_the_body_and_names_stay_readable() {
        let luma = |c: Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
        for color in PALETTE.into_iter().chain([CLIP_NEUTRAL, AUTOMATION]) {
            assert!(luma(clip_content(color)) > luma(clip_body(color)));
            assert!(luma(clip_body(color)) < luma(color));
        }
        assert_eq!(text_on(KEY_WHITE), BACKGROUND);
        assert_eq!(text_on(KEY_BLACK), TEXT);
        assert_eq!(mix(BACKGROUND, TEXT, 0.0), BACKGROUND);
        assert_eq!(mix(BACKGROUND, TEXT, 1.0), TEXT);
    }
}

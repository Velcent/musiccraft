//! Small custom widgets drawn with our own artwork.

use std::f32::consts::PI;

use egui::{Color32, Pos2, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::theme;

/// What a knob did this frame.
pub struct KnobOutput {
    /// The new value, if it changed.
    pub changed: Option<f32>,
    /// A drag started (begin a coalesced gesture).
    pub started: bool,
    /// The widget's response.
    pub response: Response,
}

/// A rotary knob. Drag vertically to change; double-click resets to `default`.
pub fn knob(
    ui: &mut Ui,
    value: f32,
    min: f32,
    max: f32,
    default: f32,
    bipolar: bool,
) -> KnobOutput {
    let size = 22.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let range = (max - min).max(1e-6);
    let mut changed = None;
    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let speed = if fine { 1_000.0 } else { 160.0 };
        let delta = -response.drag_delta().y / speed * range;
        if delta != 0.0 {
            changed = Some((value + delta).clamp(min, max));
        }
    }
    if response.double_clicked() {
        changed = Some(default);
    }
    let shown = changed.unwrap_or(value);
    let painter = ui.painter_at(rect.expand(2.0));
    let center = rect.center();
    let radius = size / 2.0 - 2.0;
    painter.circle_filled(center, radius, theme::RAISED);
    let t = ((shown - min) / range).clamp(0.0, 1.0);
    let start = PI * 0.75;
    let sweep = PI * 1.5;
    let angle_of = |t: f32| start + sweep * t;
    let from = if bipolar { 0.5 } else { 0.0 };
    let arc: Vec<Pos2> = (0..=16)
        .map(|i| {
            let k = from + (t - from) * i as f32 / 16.0;
            let a = angle_of(k);
            center + vec2(a.cos(), a.sin()) * (radius + 1.0)
        })
        .collect();
    painter.add(egui::Shape::line(arc, Stroke::new(2.0, theme::ACCENT)));
    let a = angle_of(t);
    painter.line_segment(
        [center, center + vec2(a.cos(), a.sin()) * (radius - 2.0)],
        Stroke::new(2.0, theme::TEXT),
    );
    let started = response.drag_started();
    KnobOutput {
        changed,
        started,
        response,
    }
}

/// A round on/off indicator button.
pub fn dot(ui: &mut Ui, on: bool, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
    let fill = if on { color } else { theme::PANEL_DARK };
    ui.painter().circle_filled(rect.center(), 5.0, fill);
    ui.painter()
        .circle_stroke(rect.center(), 5.0, Stroke::new(1.0, theme::TEXT_DIM));
    response
}

/// Converts linear gain to a 0..1 meter position (-60 dB .. +6 dB).
fn meter_position(gain: f32) -> f32 {
    if gain <= 1e-6 {
        return 0.0;
    }
    let db = 20.0 * gain.log10();
    ((db + 60.0) / 66.0).clamp(0.0, 1.0)
}

/// A vertical stereo peak meter.
pub fn meter(ui: &mut Ui, levels: (f32, f32), size: Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, theme::METER_BG);
    let half = rect.width() / 2.0;
    for (i, level) in [levels.0, levels.1].into_iter().enumerate() {
        let x0 = rect.left() + half * i as f32 + 1.0;
        let h = rect.height() * meter_position(level);
        let bar = egui::Rect::from_min_max(
            pos2(x0, rect.bottom() - h),
            pos2(x0 + half - 2.0, rect.bottom()),
        );
        let color = if level >= 1.0 {
            theme::DANGER
        } else if level > 0.5 {
            theme::HIGHLIGHT
        } else {
            theme::ACCENT
        };
        painter.rect_filled(bar, 1.0, color);
    }
    // 0 dB tick.
    let zero = rect.bottom() - rect.height() * meter_position(1.0);
    painter.line_segment(
        [pos2(rect.left(), zero), pos2(rect.right(), zero)],
        Stroke::new(1.0, theme::TEXT_DIM),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_scale() {
        assert!(meter_position(0.0).abs() < f32::EPSILON);
        assert!((meter_position(1.0) - 60.0 / 66.0).abs() < 1e-4);
        assert!((meter_position(100.0) - 1.0).abs() < f32::EPSILON);
    }
}

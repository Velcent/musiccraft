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

/// A larger knob over a normalized 0..1 value, with a caption that shows the
/// formatted value while hovered or dragged. Double-click resets to `default`.
pub fn labeled_knob(
    ui: &mut Ui,
    label: &str,
    t: f32,
    default: f32,
    bipolar: bool,
    value: &str,
) -> KnobOutput {
    let size = vec2(52.0, 60.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let mut changed = None;
    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let delta = -response.drag_delta().y / if fine { 600.0 } else { 150.0 };
        if delta != 0.0 {
            changed = Some((t + delta).clamp(0.0, 1.0));
        }
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            changed = Some((t + scroll / 600.0).clamp(0.0, 1.0));
        }
    }
    if response.double_clicked() {
        changed = Some(default);
    }
    let shown = changed.unwrap_or(t);
    let painter = ui.painter_at(rect);
    let center = pos2(rect.center().x, rect.top() + 22.0);
    let radius = 15.0;
    painter.circle_filled(center, radius + 3.0, theme::PANEL_DARK);
    painter.circle_filled(center, radius - 2.0, theme::RAISED);
    let start = PI * 0.75;
    let sweep = PI * 1.5;
    let angle = |k: f32| start + sweep * k;
    let arc = |from: f32, to: f32| -> Vec<Pos2> {
        (0..=24)
            .map(|i| {
                let a = angle(from + (to - from) * i as f32 / 24.0);
                center + vec2(a.cos(), a.sin()) * (radius + 1.0)
            })
            .collect()
    };
    painter.add(egui::Shape::line(
        arc(0.0, 1.0),
        Stroke::new(3.0, theme::PANEL),
    ));
    let from = if bipolar { 0.5 } else { 0.0 };
    painter.add(egui::Shape::line(
        arc(from, shown),
        Stroke::new(3.0, theme::ACCENT),
    ));
    let a = angle(shown);
    painter.line_segment(
        [
            center + vec2(a.cos(), a.sin()) * 4.0,
            center + vec2(a.cos(), a.sin()) * (radius - 3.0),
        ],
        Stroke::new(2.0, theme::TEXT),
    );
    let active = response.hovered() || response.dragged();
    painter.text(
        pos2(rect.center().x, rect.bottom() - 9.0),
        egui::Align2::CENTER_CENTER,
        if active { value } else { label },
        egui::FontId::proportional(10.5),
        if active {
            theme::PLAYHEAD
        } else {
            theme::TEXT_DIM
        },
    );
    let started = response.drag_started();
    KnobOutput {
        changed,
        started: started
            || (response.hovered()
                && ui.input(|i| i.smooth_scroll_delta.y != 0.0)
                && !response.dragged()),
        response,
    }
}

pub use musiccraft_project::params::{
    volume_from_unit as fader_to_gain, volume_to_unit as gain_to_fader,
};

/// A vertical mixer fader. Drag the cap; double-click returns to unity.
pub fn fader(ui: &mut Ui, gain: f32, max: f32, size: Vec2) -> KnobOutput {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let travel = rect.shrink2(vec2(0.0, 10.0));
    let mut changed = None;
    let position = gain_to_fader(gain, max);
    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let delta = -response.drag_delta().y / travel.height() * if fine { 0.25 } else { 1.0 };
        if delta != 0.0 {
            changed = Some(fader_to_gain(position + delta, max));
        }
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            changed = Some(fader_to_gain(position + scroll / 800.0, max));
        }
    }
    if response.double_clicked() {
        changed = Some(1.0);
    }
    let shown = changed.map_or(position, |g| gain_to_fader(g, max));
    let painter = ui.painter_at(rect.expand(2.0));
    let x = rect.center().x;
    painter.rect_filled(
        egui::Rect::from_center_size(egui::pos2(x, travel.center().y), vec2(4.0, travel.height())),
        2.0,
        theme::METER_BG,
    );
    let unity = travel.bottom() - travel.height() * 0.75;
    painter.line_segment(
        [egui::pos2(x - 9.0, unity), egui::pos2(x + 9.0, unity)],
        Stroke::new(1.0, theme::TEXT_DIM),
    );
    let y = travel.bottom() - travel.height() * shown;
    let cap = egui::Rect::from_center_size(egui::pos2(x, y), vec2(rect.width().min(26.0), 16.0));
    let fill = if response.dragged() || response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    painter.rect_filled(cap, 3.0, fill);
    painter.rect_stroke(
        cap,
        3.0,
        Stroke::new(1.0, theme::TEXT_DIM),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(cap.left() + 3.0, y),
            egui::pos2(cap.right() - 3.0, y),
        ],
        Stroke::new(2.0, theme::ACCENT),
    );
    let started = response.drag_started()
        || (response.hovered()
            && ui.input(|i| i.smooth_scroll_delta.y != 0.0)
            && !response.dragged());
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
    fn fader_curve_round_trips_and_puts_unity_at_three_quarters() {
        let max = 1.25;
        assert!((gain_to_fader(1.0, max) - 0.75).abs() < 1e-6);
        assert!(gain_to_fader(0.0, max).abs() < f32::EPSILON);
        assert!((gain_to_fader(max, max) - 1.0).abs() < 1e-4);
        for gain in [0.001, 0.05, 0.3, 0.8, 1.0, 1.2] {
            let back = fader_to_gain(gain_to_fader(gain, max), max);
            assert!((back - gain).abs() / gain < 1e-3, "{gain} -> {back}");
        }
    }

    #[test]
    fn meter_scale() {
        assert!(meter_position(0.0).abs() < f32::EPSILON);
        assert!((meter_position(1.0) - 60.0 / 66.0).abs() < 1e-4);
        assert!((meter_position(100.0) - 1.0).abs() < f32::EPSILON);
    }
}

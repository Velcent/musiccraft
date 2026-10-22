//! Small custom widgets drawn with our own artwork.

use std::f32::consts::PI;

use egui::{Color32, PointerButton, Pos2, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

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

/// Whether the user asked to reset a control to its default: a middle-click
/// or a double-click (see `docs/spec/controls.md`).
#[must_use]
pub fn wants_reset(response: &Response) -> bool {
    response.middle_clicked() || response.double_clicked()
}

/// A rotary knob. Drag vertically to change; middle-click or double-click
/// resets to `default`.
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
    if response.dragged_by(PointerButton::Primary) {
        let fine = ui.input(|i| i.modifiers.shift);
        let speed = if fine { 1_000.0 } else { 160.0 };
        let delta = -response.drag_delta().y / speed * range;
        if delta != 0.0 {
            changed = Some((value + delta).clamp(min, max));
        }
    }
    if wants_reset(&response) {
        changed = Some(default);
    }
    let shown = changed.unwrap_or(value);
    let painter = ui.painter_at(rect.expand(2.0));
    let center = rect.center();
    let radius = size / 2.0 - 2.0;
    painter.circle_filled(center, radius, theme::RAISED);
    // A thin ring keeps a centred knob visible on dark wells.
    painter.circle_stroke(center, radius, Stroke::new(1.0, theme::GRID_BAR));
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
    // A reset is its own undo step.
    let started = response.drag_started_by(PointerButton::Primary) || wants_reset(&response);
    KnobOutput {
        changed,
        started,
        response,
    }
}

/// A larger knob over a normalized 0..1 value, with a caption that shows the
/// formatted value while hovered or dragged. Middle-click or double-click
/// resets to `default`.
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
    if response.dragged_by(PointerButton::Primary) {
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
    if wants_reset(&response) {
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
            theme::LCD_TEXT
        } else {
            theme::TEXT_DIM
        },
    );
    // A reset is its own undo step.
    let started = response.drag_started_by(PointerButton::Primary) || wants_reset(&response);
    KnobOutput {
        changed,
        started: started
            || (response.hovered()
                && ui.input(|i| i.smooth_scroll_delta.y != 0.0)
                && !response.dragged()),
        response,
    }
}

pub use musiccraft_params::{volume_from_unit as fader_to_gain, volume_to_unit as gain_to_fader};

/// A vertical mixer fader. Drag the cap; middle-click or double-click returns
/// to unity.
pub fn fader(ui: &mut Ui, gain: f32, max: f32, size: Vec2) -> KnobOutput {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let travel = rect.shrink2(vec2(0.0, 10.0));
    let mut changed = None;
    let position = gain_to_fader(gain, max);
    let cap_y = travel.bottom() - travel.height() * position;
    // A press on the rail away from the cap glides the cap toward the
    // pointer while held, instead of jumping.
    let on_rail = ui
        .input(|i| i.pointer.press_origin())
        .is_some_and(|p| (p.y - cap_y).abs() > 8.0);
    let mut glide_started = false;
    let primary = ui.input(|i| i.pointer.primary_down());
    if on_rail && primary && response.is_pointer_button_down_on() {
        if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
            let target = ((travel.bottom() - p.y) / travel.height()).clamp(0.0, 1.0);
            let step = (target - position).clamp(-0.02, 0.02);
            if step.abs() > 1e-4 {
                changed = Some(fader_to_gain(position + step, max));
                ui.ctx().request_repaint();
            }
            glide_started = ui.input(|i| i.pointer.primary_pressed());
        }
    } else if response.dragged_by(PointerButton::Primary) {
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
    if wants_reset(&response) {
        changed = Some(1.0);
    }
    let shown = changed.map_or(position, |g| gain_to_fader(g, max));
    let painter = ui.painter_at(rect.expand(2.0));
    let x = rect.center().x;
    let groove =
        egui::Rect::from_center_size(egui::pos2(x, travel.center().y), vec2(4.0, travel.height()));
    painter.rect_filled(groove, 2.0, theme::METER_BG);
    painter.rect_stroke(
        groove,
        2.0,
        Stroke::new(1.0, theme::LCD_EDGE),
        egui::StrokeKind::Outside,
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
    let started = (response.drag_started_by(PointerButton::Primary) && !on_rail)
        || glide_started
        || wants_reset(&response)
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

/// The peak of one meter channel held for 1.5 s, then falling, and whether
/// it clipped in the last 3 s.
pub fn held_peak(ui: &Ui, id: egui::Id, level: f32) -> (f32, bool) {
    let now = ui.input(|i| i.time);
    let (mut peak, mut until, mut clip_until) = ui
        .data(|d| d.get_temp::<(f32, f64, f64)>(id))
        .unwrap_or((0.0, 0.0, 0.0));
    if level >= peak {
        peak = level;
        until = now + 1.5;
    } else if now > until {
        peak = (peak * 0.9).max(level);
    }
    if level >= 1.0 {
        clip_until = now + 3.0;
    }
    ui.data_mut(|d| d.insert_temp(id, (peak, until, clip_until)));
    (peak, now < clip_until)
}

/// A vertical stereo peak meter.
pub fn meter(ui: &mut Ui, levels: (f32, f32), size: Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, theme::METER_BG);
    // The edge keeps a silent meter visible on any background.
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, theme::LCD_EDGE),
        egui::StrokeKind::Inside,
    );
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
        // The peak holds for a moment as a thin line; a clip leaves a
        // red mark at the top for a few seconds.
        let (peak, clip) = held_peak(ui, response.id.with(i), level);
        if peak > 1e-6 {
            let y = rect.bottom() - rect.height() * meter_position(peak);
            painter.line_segment(
                [pos2(x0, y), pos2(x0 + half - 2.0, y)],
                Stroke::new(1.0, theme::TEXT),
            );
        }
        if clip {
            let mark = egui::Rect::from_min_size(pos2(x0, rect.top() + 1.0), vec2(half - 2.0, 3.0));
            painter.rect_filled(mark, 0.0, theme::DANGER);
        }
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

    /// Runs `widget` at the top left for a few frames of `events` each and
    /// returns every value it reported changing to, with whether a gesture
    /// started then.
    fn drive(
        frames: Vec<Vec<egui::Event>>,
        mut widget: impl FnMut(&mut Ui) -> KnobOutput,
    ) -> Vec<(f32, bool)> {
        let ctx = egui::Context::default();
        let mut out = Vec::new();
        for (i, events) in frames.into_iter().enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 400.0))),
                time: Some(i as f64 / 60.0),
                events,
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                let result = widget(ui);
                if let Some(value) = result.changed {
                    out.push((value, result.started));
                }
            })
            .textures_delta
            .clear();
        }
        out
    }

    fn click(at: Pos2, button: PointerButton) -> Vec<Vec<egui::Event>> {
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![
            vec![],
            vec![egui::Event::PointerMoved(at), press(true)],
            vec![press(false)],
            vec![],
        ]
    }

    fn drag(from: Pos2, to: Pos2, button: PointerButton) -> Vec<Vec<egui::Event>> {
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut frames = vec![
            vec![],
            vec![egui::Event::PointerMoved(from), press(from, true)],
        ];
        for i in 1..=4 {
            frames.push(vec![egui::Event::PointerMoved(
                from + (to - from) * (i as f32 / 4.0),
            )]);
        }
        frames.push(vec![press(to, false)]);
        frames
    }

    #[test]
    fn middle_click_resets_knobs_and_faders_as_their_own_gesture() {
        // Every widget sits at the top left of the panel; aim inside it.
        let at = pos2(20.0, 20.0);
        let knob_run = drive(click(at, PointerButton::Middle), |ui| {
            knob(ui, 0.3, 0.0, 1.0, 0.8, false)
        });
        assert_eq!(knob_run, [(0.8, true)]);
        let labeled = drive(click(at, PointerButton::Middle), |ui| {
            labeled_knob(ui, "Cutoff", 0.2, 0.6, false, "")
        });
        assert_eq!(labeled, [(0.6, true)]);
        let fader_run = drive(click(at, PointerButton::Middle), |ui| {
            fader(ui, 0.2, 1.25, vec2(30.0, 120.0))
        });
        assert_eq!(fader_run, [(1.0, true)]);
    }

    #[test]
    fn holding_the_fader_rail_glides_the_cap_toward_the_pointer() {
        // The cap of a 120-point fader at unity sits near y = 37; hold the
        // button down near the bottom of the rail.
        let at = pos2(15.0, 100.0);
        let mut frames = vec![
            vec![],
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        ];
        frames.extend(std::iter::repeat_n(vec![], 5));
        let run = drive(frames, |ui| fader(ui, 1.0, 1.25, vec2(30.0, 120.0)));
        assert!(run.len() >= 3, "{run:?}");
        assert!(run.first().is_some_and(|(_, started)| *started));
        // Each frame moves a small step down, never a jump.
        let first = run.first().map_or(1.0, |(g, _)| *g);
        assert!(first < 1.0 && first > 0.8, "{run:?}");
    }

    #[test]
    fn only_the_left_button_drags_a_knob() {
        let (from, to) = (pos2(18.0, 18.0), pos2(18.0, -40.0));
        for button in [PointerButton::Secondary, PointerButton::Middle] {
            let moved = drive(drag(from, to, button), |ui| {
                knob(ui, 0.3, 0.0, 1.0, 0.8, false)
            });
            assert!(
                moved.iter().all(|(value, _)| (*value - 0.8).abs() < 1e-6),
                "{button:?} drag moved the knob: {moved:?}"
            );
        }
        let moved = drive(drag(from, to, PointerButton::Primary), |ui| {
            knob(ui, 0.3, 0.0, 1.0, 0.8, false)
        });
        assert!(moved.iter().any(|(value, _)| *value > 0.3), "{moved:?}");
    }

    #[test]
    fn meter_scale() {
        assert!(meter_position(0.0).abs() < f32::EPSILON);
        assert!((meter_position(1.0) - 60.0 / 66.0).abs() < 1e-4);
        assert!((meter_position(100.0) - 1.0).abs() < f32::EPSILON);
    }
}

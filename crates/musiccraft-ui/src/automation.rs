//! Automation editing: the curve editor window, and the "Create automation
//! clip" menu attached to knobs and faders.

use crate::hint::Hint;
use egui::{Align2, FontId, PointerButton, Pos2, RichText, Sense, Stroke, pos2, vec2};
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_project::{Automation, AutomationPoint, Curve, Meter, PLAYLIST_TRACKS, Project};

use crate::geometry::Snap;
use crate::{Action, Frame, UiState, theme};

/// The first playlist row with no clips (1-based), or the last row.
fn free_track(project: &Project) -> u32 {
    (0..PLAYLIST_TRACKS)
        .find(|row| !project.playlist.clips.iter().any(|c| c.track == *row))
        .unwrap_or(PLAYLIST_TRACKS - 1)
        + 1
}

/// One undo step that creates an automation for `target` and places a clip of
/// it on a free playlist track at the start of the song.
pub(crate) fn create_invocation(project: &Project, target: &str) -> Invocation {
    let number = project.automations.len() + 1;
    Invocation::new(
        BATCH,
        [
            Invocation::new("automation.create", [target.to_owned()]).to_string(),
            format!(
                "playlist.add_automation_clip {number} {} 0",
                free_track(project)
            ),
        ],
    )
}

/// Adds a right-click menu that automates `target` (command syntax such as
/// `mixer:0:volume`). The UI resolves the request with the current project.
pub(crate) fn menu(response: &egui::Response, target: &str, actions: &mut Vec<Action>) {
    response.context_menu(|ui| {
        if ui.button("Create automation clip").clicked() {
            actions.push(Action::Run(Invocation::new(
                "ui.automate",
                [target.to_owned()],
            )));
            ui.close();
        }
        ui.menu_button("Add controller", |ui| {
            for (label, kind) in [
                ("LFO", "lfo"),
                ("Macro knob", "macro"),
                ("Envelope (first channel's notes)", "envelope"),
                ("Keyboard (first channel's key)", "keyboard"),
                ("Peak (insert 1's level)", "peak"),
                ("Formula", "formula"),
            ] {
                if ui.button(label).clicked() {
                    actions.push(Action::Run(Invocation::new(
                        "controller.add",
                        [kind.to_owned(), target.to_owned()],
                    )));
                    ui.close();
                }
            }
        });
    });
}
const HEIGHT: f32 = 180.0;
const POINT_RADIUS: f32 = 5.0;

/// A segment's tension being dragged (Alt+drag).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TensionDrag {
    /// The point that starts the segment.
    tick: u32,
    /// Its shape.
    curve: Curve,
    /// The tension shown.
    tension: f32,
}

/// A point being dragged: (original tick, current tick, current value).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointDrag {
    from: u32,
    tick: u32,
    value: f32,
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let Some(id) = state.automation_editor else {
        return;
    };
    let project = frame.project;
    let Some(index) = project.automation_position(id) else {
        state.automation_editor = None;
        return;
    };
    let automation = &project.automations[index];
    let mut open = true;
    let area = state.workspace;
    let window_id = egui::Id::new("automation-editor");
    let title = format!("Automation — {}", automation.name);
    let default = egui::Rect::from_min_size(
        area.left_bottom() + vec2(60.0, -HEIGHT - 120.0),
        vec2((area.width() - 120.0).max(400.0), 0.0),
    );
    crate::chrome::window(ctx, window_id, "Automation", area, default, true).show(ctx, |ui| {
        let folded = crate::chrome::Header::new(window_id, &title).show(
            ui,
            &mut open,
            &mut (),
            |_, _| {},
            |_, _| {},
        );
        if folded {
            return;
        }
        // Curve ticks count from the clip start: bars of the project
        // signature.
        let meter = Meter::new(project.time_signature);
        canvas(ui, state, (index, automation), &meter, actions);
    });
    if !open {
        state.automation_editor = None;
    }
}

fn canvas(
    ui: &mut egui::Ui,
    state: &mut UiState,
    (index, automation): (usize, &Automation),
    meter: &Meter,
    actions: &mut Vec<Action>,
) {
    let length =
        meter.round_up_to_bar(automation.length_ticks().max(meter.bar_start(4))) + meter.bar(0).len;
    let width = ui.available_width().max(200.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::click_and_drag());
    let response = response
        .hint("Automation curve: click to add a point · drag to move · right-click to delete");
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let inner = rect.shrink2(vec2(8.0, 10.0));
    state.automation_view = Some((inner, length));
    let x_of = |tick: f32| inner.left() + tick / length as f32 * inner.width();
    let tick_of = |x: f32| ((x - inner.left()) / inner.width() * length as f32).max(0.0);
    let y_of = |v: f32| inner.bottom() - v * inner.height();
    let value_of = |y: f32| ((inner.bottom() - y) / inner.height()).clamp(0.0, 1.0);
    for beat in meter.beats(0, length + 1) {
        let x = x_of(beat.tick as f32);
        let color = if beat.downbeat {
            theme::GRID_BAR
        } else {
            theme::GRID_LINE
        };
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, color),
        );
        if beat.downbeat {
            painter.text(
                pos2(x + 3.0, rect.top() + 2.0),
                Align2::LEFT_TOP,
                (beat.bar + 1).to_string(),
                FontId::monospace(10.0),
                theme::TEXT_DIM,
            );
        }
    }
    let snap = Snap::Step;
    let drag = state.automation_drag;
    let bend = state.tension_drag;
    // Points as currently shown (with a point or tension drag applied).
    let mut shown: Vec<AutomationPoint> = Vec::new();
    for p in &automation.points {
        match drag {
            Some(d) if d.from == p.tick => shown.push(AutomationPoint {
                tick: d.tick,
                value: d.value,
                ..*p
            }),
            _ => shown.push(*p),
        }
        if let (Some(b), Some(last)) = (bend, shown.last_mut()) {
            if b.tick == p.tick {
                last.tension = b.tension;
            }
        }
    }
    shown.sort_by_key(|p| p.tick);
    let curve = Automation {
        points: shown.clone(),
        ..automation.clone()
    };
    if !shown.is_empty() {
        let steps = (inner.width() / 2.0).max(2.0) as usize;
        let line: Vec<Pos2> = (0..=steps)
            .filter_map(|i| {
                let x = inner.left() + inner.width() * i as f32 / steps as f32;
                let v = curve.value_at(f64::from(tick_of(x)))?;
                Some(pos2(x, y_of(v)))
            })
            .collect();
        painter.add(egui::Shape::line(line, Stroke::new(2.0, theme::AUTOMATION)));
    }
    let shown: Vec<(u32, f32)> = shown.iter().map(|p| (p.tick, p.value)).collect();
    for (t, v) in &shown {
        painter.circle_filled(
            pos2(x_of(*t as f32), y_of(*v)),
            POINT_RADIUS,
            theme::AUTOMATION,
        );
        painter.circle_stroke(
            pos2(x_of(*t as f32), y_of(*v)),
            POINT_RADIUS,
            Stroke::new(1.0, theme::BACKGROUND),
        );
    }
    let hit = |pos: Pos2| {
        automation
            .points
            .iter()
            .find(|p| pos2(x_of(p.tick as f32), y_of(p.value)).distance(pos) <= POINT_RADIUS + 3.0)
            .copied()
    };
    let number = (index + 1).to_string();
    let t = |ticks: u32| format!("{ticks}t");
    // The point that starts the segment under `x`.
    let segment = |x: f32| {
        let tick = tick_of(x) as u32;
        automation
            .points
            .windows(2)
            .find(|w| w[0].tick <= tick && tick < w[1].tick)
            .map(|w| w[0])
    };
    let alt = ui.input(|i| i.modifiers.alt);
    if response.drag_started_by(PointerButton::Primary) {
        let origin = ui.input(|i| i.pointer.press_origin());
        if let Some(point) = origin.and_then(hit) {
            state.automation_drag = Some(PointDrag {
                from: point.tick,
                tick: point.tick,
                value: point.value,
            });
        } else if let Some(start) = origin.filter(|_| alt).and_then(|p| segment(p.x)) {
            // Alt+drag on a segment bends it.
            state.tension_drag = Some(TensionDrag {
                tick: start.tick,
                curve: start.curve,
                tension: start.tension,
            });
        }
    }
    if let Some(b) = &mut state.tension_drag {
        if response.dragged() {
            let dy = response.drag_delta().y;
            b.tension = (b.tension + dy / 80.0).clamp(-1.0, 1.0);
        }
        if response.drag_stopped() {
            actions.push(Action::Run(Invocation::new(
                "automation.set_curve",
                [
                    number.clone(),
                    t(b.tick),
                    b.curve.name().to_owned(),
                    format!("{:.3}", b.tension),
                ],
            )));
            state.tension_drag = None;
        }
    }
    if let (Some(d), Some(pos)) = (&mut state.automation_drag, response.interact_pointer_pos()) {
        if response.dragged() {
            d.tick = snap.round(tick_of(pos.x), meter);
            d.value = value_of(pos.y);
        }
    }
    if response.drag_stopped() {
        if let Some(d) = state.automation_drag.take() {
            actions.push(Action::Run(Invocation::new(
                "automation.move_point",
                [
                    number.clone(),
                    t(d.from),
                    t(d.tick),
                    format!("{:.4}", d.value),
                ],
            )));
        }
    }
    if response.clicked_by(PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos() {
            if hit(pos).is_none() && inner.expand(4.0).contains(pos) {
                let tick = snap.round(tick_of(pos.x), meter).min(length);
                actions.push(Action::Run(Invocation::new(
                    "automation.add_point",
                    [number.clone(), t(tick), format!("{:.4}", value_of(pos.y))],
                )));
            }
        }
    }
    if response.secondary_clicked() {
        let pos = response.interact_pointer_pos();
        state.automation_menu = None;
        if let Some(point) = pos.and_then(hit) {
            if automation.points.len() > 1 {
                actions.push(Action::Run(Invocation::new(
                    "automation.remove_point",
                    [number.clone(), t(point.tick)],
                )));
            }
        } else if let Some(start) = pos.and_then(|p| segment(p.x)) {
            state.automation_menu = Some(start.tick);
        }
    }
    response.context_menu(|ui| {
        let Some(start) = state
            .automation_menu
            .and_then(|tick| automation.points.iter().find(|p| p.tick == tick))
        else {
            ui.close();
            return;
        };
        ui.label(
            RichText::new("Segment shape")
                .small()
                .color(theme::TEXT_DIM),
        );
        for (curve, label) in [
            (Curve::Single, "Curve"),
            (Curve::Double, "S-curve"),
            (Curve::Hold, "Hold"),
            (Curve::Stairs, "Steps"),
        ] {
            if ui.radio(start.curve == curve, label).clicked() {
                actions.push(Action::Run(Invocation::new(
                    "automation.set_curve",
                    [
                        number.clone(),
                        t(start.tick),
                        curve.name().to_owned(),
                        format!("{:.3}", start.tension),
                    ],
                )));
                ui.close();
            }
        }
        if ui.button("Reset tension").clicked() {
            actions.push(Action::Run(Invocation::new(
                "automation.set_curve",
                [number.clone(), t(start.tick), start.curve.name().to_owned()],
            )));
            ui.close();
        }
        ui.label(
            RichText::new("Alt+drag a segment to bend it")
                .small()
                .color(theme::TEXT_DIM),
        );
    });
    if let Some(pos) = response.hover_pos() {
        if let Some(point) = hit(pos) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            painter.text(
                pos + vec2(10.0, -14.0),
                Align2::LEFT_BOTTOM,
                format!("{:.0}%", point.value * 100.0),
                FontId::monospace(11.0),
                theme::TEXT,
            );
        }
    }
}

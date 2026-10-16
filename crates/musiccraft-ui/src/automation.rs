//! Automation editing: the curve editor window, and the "Create automation
//! clip" menu attached to knobs and faders.

use egui::{Align2, Color32, FontId, PointerButton, Pos2, RichText, Sense, Stroke, pos2, vec2};
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_project::params::{PPQ, TICKS_PER_BAR};
use musiccraft_project::{Automation, PLAYLIST_TRACKS, Project};

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
    egui::Window::new(format!("Automation — {}", automation.name))
        .id(egui::Id::new("automation-editor"))
        .open(&mut open)
        .default_pos(area.left_bottom() + vec2(60.0, -HEIGHT - 120.0))
        .default_width((area.width() - 120.0).max(400.0))
        .constrain_to(area)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&automation.name)
                        .strong()
                        .color(theme::AUTOMATION),
                );
                ui.label(
                    RichText::new("Click to add a point · drag to move · right-click to delete")
                        .small()
                        .color(theme::TEXT_DIM),
                );
            });
            canvas(ui, state, index, automation, actions);
        });
    if !open {
        state.automation_editor = None;
    }
}

fn canvas(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    automation: &Automation,
    actions: &mut Vec<Action>,
) {
    let length = automation.length_ticks().max(4 * TICKS_PER_BAR) + TICKS_PER_BAR;
    let width = ui.available_width().max(200.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let inner = rect.shrink2(vec2(8.0, 10.0));
    state.automation_view = Some((inner, length));
    let x_of = |tick: f32| inner.left() + tick / length as f32 * inner.width();
    let tick_of = |x: f32| ((x - inner.left()) / inner.width() * length as f32).max(0.0);
    let y_of = |v: f32| inner.bottom() - v * inner.height();
    let value_of = |y: f32| ((inner.bottom() - y) / inner.height()).clamp(0.0, 1.0);
    for beat in (0..=length).step_by(PPQ as usize) {
        let x = x_of(beat as f32);
        let color = if beat % TICKS_PER_BAR == 0 {
            theme::GRID_BAR
        } else {
            theme::GRID_LINE
        };
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, color),
        );
        if beat % TICKS_PER_BAR == 0 {
            painter.text(
                pos2(x + 3.0, rect.top() + 2.0),
                Align2::LEFT_TOP,
                (beat / TICKS_PER_BAR + 1).to_string(),
                FontId::monospace(10.0),
                theme::TEXT_DIM,
            );
        }
    }
    let snap = Snap::Step;
    let drag = state.automation_drag;
    // Points as currently shown (with the drag applied).
    let mut shown: Vec<(u32, f32)> = automation
        .points
        .iter()
        .filter(|p| drag.is_none_or(|d| d.from != p.tick))
        .map(|p| (p.tick, p.value))
        .collect();
    if let Some(d) = drag {
        shown.push((d.tick, d.value));
    }
    shown.sort_by_key(|p| p.0);
    if let (Some(first), Some(last)) = (shown.first(), shown.last()) {
        let mut line = vec![pos2(inner.left(), y_of(first.1))];
        line.extend(shown.iter().map(|(t, v)| pos2(x_of(*t as f32), y_of(*v))));
        line.push(pos2(inner.right(), y_of(last.1)));
        painter.add(egui::Shape::line(line, Stroke::new(2.0, theme::AUTOMATION)));
    }
    for (t, v) in &shown {
        painter.circle_filled(
            pos2(x_of(*t as f32), y_of(*v)),
            POINT_RADIUS,
            theme::AUTOMATION,
        );
        painter.circle_stroke(
            pos2(x_of(*t as f32), y_of(*v)),
            POINT_RADIUS,
            Stroke::new(1.0, Color32::BLACK),
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
    if response.drag_started_by(PointerButton::Primary) {
        if let Some(point) = ui.input(|i| i.pointer.press_origin()).and_then(hit) {
            state.automation_drag = Some(PointDrag {
                from: point.tick,
                tick: point.tick,
                value: point.value,
            });
        }
    }
    if let (Some(d), Some(pos)) = (&mut state.automation_drag, response.interact_pointer_pos()) {
        if response.dragged() {
            d.tick = snap.round(tick_of(pos.x));
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
                let tick = snap.round(tick_of(pos.x)).min(length);
                actions.push(Action::Run(Invocation::new(
                    "automation.add_point",
                    [number.clone(), t(tick), format!("{:.4}", value_of(pos.y))],
                )));
            }
        }
    }
    if response.secondary_clicked() {
        if let Some(point) = response.interact_pointer_pos().and_then(hit) {
            if automation.points.len() > 1 {
                actions.push(Action::Run(Invocation::new(
                    "automation.remove_point",
                    [number, t(point.tick)],
                )));
            }
        }
    }
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

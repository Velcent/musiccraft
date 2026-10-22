//! Automation editing: the curve editor window, and the "Create automation
//! clip" menu attached to knobs and faders.

use crate::hint::Hint;
use egui::{Align2, FontId, PointerButton, Pos2, RichText, Sense, Stroke, pos2, vec2};
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_editors::widgets;
use musiccraft_project::{
    Automation, AutomationPoint, AutomationTarget, Curve, Meter, PLAYLIST_TRACKS, Project,
};
use std::collections::HashMap;
use std::sync::Arc;

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

/// The command syntax for `target` (`channel:2:volume`, `mixer:3:1:mix`), as
/// the control menus name their targets.
fn target_name(project: &Project, target: &AutomationTarget) -> Option<String> {
    let channel = |id| project.channel_position(id).map(|i| i + 1);
    Some(match target {
        AutomationTarget::ChannelVolume { channel: id } => {
            format!("channel:{}:volume", channel(*id)?)
        }
        AutomationTarget::ChannelPan { channel: id } => format!("channel:{}:pan", channel(*id)?),
        AutomationTarget::ChannelParam { channel: id, param } => {
            format!("channel:{}:{param}", channel(*id)?)
        }
        AutomationTarget::MixerVolume { track } => format!("mixer:{track}:volume"),
        AutomationTarget::MixerPan { track } => format!("mixer:{track}:pan"),
        AutomationTarget::EffectParam { track, slot, param } => {
            format!("mixer:{track}:{}:{param}", slot + 1)
        }
    })
}

/// What drives one control.
#[derive(Clone, Default)]
struct Source {
    /// Text the hint appends, such as " · automated (clip 'Master fade')".
    hint: String,
    /// The automation for it (1-based), if any.
    automation: Option<usize>,
    /// The controllers driving it (1-based).
    controllers: Vec<usize>,
}

/// What every control menu needs to know this frame: the driven controls by
/// target name, and the copied control position.
#[derive(Clone, Default)]
struct Published {
    driven: Arc<HashMap<String, Source>>,
    copied: Option<f32>,
    /// The open menu's target and its default position.
    default: Option<(String, f32)>,
}

fn published_id() -> egui::Id {
    egui::Id::new("control-menus")
}

/// The target whose control menu was open last frame.
fn open_menu_id() -> egui::Id {
    egui::Id::new("control-menu-open")
}

/// Records which controls automation clips and controllers drive, so their
/// menus can mark them (style guide, controls M), and the copied value
/// their Paste item uses. Call once per frame.
pub(crate) fn publish(ctx: &egui::Context, project: &Project, copied: Option<f32>) {
    let mut driven: HashMap<String, Source> = HashMap::new();
    for (i, automation) in project.automations.iter().enumerate() {
        if let Some(name) = target_name(project, &automation.target) {
            let source = driven.entry(name).or_default();
            source.automation = Some(i + 1);
            source.hint = format!(" · automated (clip '{}')", automation.name);
        }
    }
    for (i, controller) in project.controllers.iter().enumerate() {
        if let Some(name) = target_name(project, &controller.target) {
            let source = driven.entry(name).or_default();
            source.controllers.push(i + 1);
            if source.hint.is_empty() {
                source.hint = format!(" · driven by {}", controller.name);
            }
        }
    }
    // Only the open menu's default is looked up, a frame after it opens.
    let default = ctx
        .data_mut(|d| d.remove_temp::<String>(open_menu_id()))
        .and_then(|target| {
            let info = musiccraft_commands::control_info(project, &target)?;
            Some((target, info.default))
        });
    let published = Published {
        driven: Arc::new(driven),
        copied,
        default,
    };
    ctx.data_mut(|d| d.insert_temp(published_id(), published));
}

/// Draws the status dot on a driven control and adds its source to the hint.
fn mark(response: &egui::Response, source: Option<&Source>) {
    if source.is_some() {
        let painter = response
            .ctx
            .layer_painter(response.layer_id)
            .with_clip_rect(response.interact_rect.expand(6.0));
        widgets::status_dot(&painter, widgets::status_anchor(response));
    }
    let hint = source.map(|s| s.hint.clone());
    musiccraft_editors::hint::set_suffix(&response.ctx, response.id, hint);
}

/// A dim heading row inside a menu.
fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(text)
            .size(theme::FONT_LABEL)
            .color(theme::TEXT_DIM),
    );
}

/// Adds the control menu to `target` (command syntax such as
/// `mixer:0:volume`), and marks the control when something drives it. The UI
/// resolves `ui.*` requests with the current project.
pub(crate) fn menu(response: &egui::Response, target: &str, actions: &mut Vec<Action>) {
    let published = response
        .ctx
        .data(|d| d.get_temp::<Published>(published_id()))
        .unwrap_or_default();
    let source = published.driven.get(target);
    mark(response, source);
    let run = |actions: &mut Vec<Action>, name: &str, args: Vec<String>| {
        actions.push(Action::Run(Invocation::new(name, args)));
    };
    let target_arg = || vec![target.to_owned()];
    response.context_menu(|ui| {
        ui.data_mut(|d| d.insert_temp(open_menu_id(), target.to_owned()));
        if ui.button("Create automation clip").clicked() {
            run(actions, "ui.automate", target_arg());
            ui.close();
        }
        let automation = source.and_then(|s| s.automation);
        if crate::widgets::add_enabled(
            ui,
            automation.is_some(),
            egui::Button::new("Open its automation…"),
        )
        .clicked()
            && let Some(number) = automation
        {
            run(actions, "ui.open_automation", vec![number.to_string()]);
            ui.close();
        }
        ui.separator();
        heading(ui, "Control from");
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
                    run(
                        actions,
                        "controller.add",
                        vec![kind.to_owned(), target.to_owned()],
                    );
                    ui.close();
                }
            }
        });
        let controllers = source.map(|s| s.controllers.as_slice()).unwrap_or_default();
        if !controllers.is_empty() && ui.button("Remove controller").clicked() {
            // Highest first, so the numbers stay valid.
            let removals = controllers
                .iter()
                .rev()
                .map(|n| Invocation::new("controller.remove", [n.to_string()]).to_string());
            run(actions, BATCH, removals.collect());
            ui.close();
        }
        ui.separator();
        heading(ui, "Value");
        if ui.button("Copy").clicked() {
            run(actions, "ui.copy_control", target_arg());
            ui.close();
        }
        if crate::widgets::add_enabled(ui, published.copied.is_some(), egui::Button::new("Paste"))
            .clicked()
            && let Some(position) = published.copied
        {
            run(
                actions,
                "control.set",
                vec![target.to_owned(), format!("{position}")],
            );
            ui.close();
        }
        if ui.button("Enter exact value…").clicked() {
            let at = response.rect.left_bottom();
            run(
                actions,
                "ui.type_in",
                vec![target.to_owned(), format!("{}", at.x), format!("{}", at.y)],
            );
            ui.close();
        }
        let default = published
            .default
            .as_ref()
            .filter(|(t, _)| t == target)
            .map(|(_, position)| format!("{:.0}%", position * 100.0))
            .unwrap_or_default();
        let reset = egui::Button::new("Reset to default")
            .right_text(RichText::new(default).color(theme::TEXT_DIM));
        if ui.add(reset).clicked() {
            run(actions, "control.reset", target_arg());
            ui.close();
        }
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
                FontId::monospace(theme::FONT_VALUE),
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
                FontId::monospace(theme::FONT_LABEL),
                theme::TEXT,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_control_menu_gets_its_default_a_frame_later() {
        let ctx = egui::Context::default();
        let project = Project::default();
        publish(&ctx, &project, None);
        let published = |ctx: &egui::Context| {
            ctx.data(|d| d.get_temp::<Published>(published_id()))
                .unwrap_or_default()
        };
        assert!(published(&ctx).default.is_none());
        // The menu for the master's pan was drawn open.
        ctx.data_mut(|d| d.insert_temp(open_menu_id(), "mixer:0:pan".to_owned()));
        publish(&ctx, &project, None);
        let (target, position) = published(&ctx).default.expect("the default");
        assert_eq!(target, "mixer:0:pan");
        assert!((position - 0.5).abs() < 1e-6);
        // Closed again: nothing is looked up.
        publish(&ctx, &project, None);
        assert!(published(&ctx).default.is_none());
    }
}

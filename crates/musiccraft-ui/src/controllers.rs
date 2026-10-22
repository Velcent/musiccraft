//! The browser's controller list: each controller's switch, kind and
//! settings (see `docs/spec/controllers.md`). Emits `controller.*`
//! invocations only.

use egui::RichText;
use musiccraft_commands::Invocation;
use musiccraft_project::{Controller, ControllerKind, Project};

use crate::{Action, UiState, theme};

/// Names of the LFO controller's shapes, by `shape`.
const LFO_SHAPES: [&str; 5] = ["sine", "triangle", "saw", "square", "random"];

fn set(number: usize, setting: &str, value: String) -> Invocation {
    Invocation::new(
        "controller.set",
        [number.to_string(), setting.to_owned(), value],
    )
}

/// A dragged number that emits one undo step per drag.
#[allow(clippy::too_many_arguments)]
fn number(
    ui: &mut egui::Ui,
    state: &mut UiState,
    actions: &mut Vec<Action>,
    controller: usize,
    label: &str,
    setting: &str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
) {
    ui.label(RichText::new(label).small().color(theme::TEXT_DIM));
    let mut edited = value;
    let speed = (range.end() - range.start()) / 300.0;
    let response = ui.add(
        egui::DragValue::new(&mut edited)
            .range(range)
            .speed(speed)
            .max_decimals(3),
    );
    if response.drag_started() {
        state.begin_gesture();
    }
    if response.changed() && edited != value {
        actions.push(Action::Coalesced(
            set(controller, setting, edited.to_string()),
            state.gesture(),
        ));
    }
}

fn settings(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    number_: usize,
    controller: &Controller,
    actions: &mut Vec<Action>,
) {
    let n = number_;
    let channel_name = |id| {
        project
            .channel_position(id)
            .map_or_else(|| "(removed)".to_owned(), |i| format!("ch {}", i + 1))
    };
    ui.horizontal_wrapped(|ui| match &controller.kind {
        ControllerKind::Lfo {
            shape,
            period,
            phase,
        } => {
            egui::ComboBox::from_id_salt(("controller-shape", n))
                .width(70.0)
                .selected_text(LFO_SHAPES.get(usize::from(*shape)).copied().unwrap_or("?"))
                .show_ui(ui, |ui| {
                    for (i, name) in LFO_SHAPES.iter().enumerate() {
                        if ui
                            .selectable_label(i == usize::from(*shape), *name)
                            .clicked()
                        {
                            actions.push(Action::Run(set(n, "shape", i.to_string())));
                        }
                    }
                });
            number(
                ui,
                state,
                actions,
                n,
                "beats",
                "period",
                *period,
                0.0625..=64.0,
            );
            number(ui, state, actions, n, "phase", "phase", *phase, 0.0..=1.0);
        }
        ControllerKind::Envelope {
            channel,
            attack,
            decay,
            sustain,
            release,
        } => {
            ui.label(RichText::new(channel_name(*channel)).small());
            number(ui, state, actions, n, "A", "attack", *attack, 0.0..=10.0);
            number(ui, state, actions, n, "D", "decay", *decay, 0.0..=10.0);
            number(ui, state, actions, n, "S", "sustain", *sustain, 0.0..=1.0);
            number(ui, state, actions, n, "R", "release", *release, 0.0..=10.0);
        }
        ControllerKind::Keyboard { channel, low, high } => {
            ui.label(RichText::new(channel_name(*channel)).small());
            number(
                ui,
                state,
                actions,
                n,
                "low",
                "low",
                f32::from(*low),
                0.0..=127.0,
            );
            number(
                ui,
                state,
                actions,
                n,
                "high",
                "high",
                f32::from(*high),
                0.0..=127.0,
            );
        }
        ControllerKind::Peak {
            track,
            gain,
            attack,
            release,
        } => {
            ui.label(RichText::new(format!("insert {track}")).small());
            number(ui, state, actions, n, "gain", "gain", *gain, 0.0..=8.0);
            number(
                ui,
                state,
                actions,
                n,
                "att ms",
                "attack",
                *attack,
                0.1..=1_000.0,
            );
            number(
                ui,
                state,
                actions,
                n,
                "rel ms",
                "release",
                *release,
                1.0..=5_000.0,
            );
        }
        ControllerKind::Formula { expr } => {
            let id = egui::Id::new(("controller-expr", n));
            let mut text = ui
                .data(|d| d.get_temp::<String>(id))
                .unwrap_or_else(|| expr.clone());
            let response = ui.add(egui::TextEdit::singleline(&mut text).desired_width(150.0));
            if response.lost_focus() && text != *expr {
                actions.push(Action::Run(set(n, "expr", text.clone())));
            }
            if response.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, text));
            } else {
                ui.data_mut(|d| d.remove::<String>(id));
            }
        }
        ControllerKind::Macro { value } => {
            number(ui, state, actions, n, "value", "value", *value, 0.0..=1.0);
        }
    });
    ui.horizontal(|ui| {
        number(
            ui,
            state,
            actions,
            n,
            "min",
            "min",
            controller.min,
            0.0..=1.0,
        );
        number(
            ui,
            state,
            actions,
            n,
            "max",
            "max",
            controller.max,
            0.0..=1.0,
        );
    });
}

/// The controller list the browser shows under "This project".
pub(crate) fn list(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    if project.controllers.is_empty() {
        ui.label(
            RichText::new("Right-click a knob and add a controller")
                .small()
                .color(theme::TEXT_DIM),
        );
    }
    for (i, controller) in project.controllers.iter().enumerate() {
        let n = i + 1;
        ui.horizontal(|ui| {
            let mut enabled = controller.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                let flag = if enabled { "on" } else { "off" };
                actions.push(Action::Run(set(n, "enabled", flag.to_owned())));
            }
            ui.label(RichText::new(&controller.name).color(theme::AUTOMATION));
            if crate::icons::header_button(
                ui,
                crate::icons::Icon::Close,
                false,
                true,
                "Delete this controller",
            )
            .clicked()
            {
                actions.push(Action::Run(Invocation::new(
                    "controller.remove",
                    [n.to_string()],
                )));
            }
        });
        settings(ui, state, project, n, controller, actions);
        ui.separator();
    }
}

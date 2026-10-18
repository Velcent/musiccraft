//! Windows for mixer effects. The editor itself (preset bar, graph, knobs) is
//! `musiccraft_editors::effect`, shared with the exported plugins; this module
//! places it in a window and turns its edits into commands. Hosted plugins
//! show their parameter panel instead.

use egui::vec2;
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_editors::effect::{self, Edit, EffectView, pretty};
use musiccraft_project::EffectSlot;
use musiccraft_project::params::{EffectKind, ParamSpec};

use crate::hint::Hint;
use crate::{Action, Frame, UiState};

/// Opens (or closes) the window of effect `slot` on `track`.
pub(crate) fn toggle(state: &mut UiState, track: usize, slot: usize) {
    if let Some(i) = state
        .effect_windows
        .iter()
        .position(|w| *w == (track, slot))
    {
        state.effect_windows.remove(i);
    } else {
        state.effect_windows.push((track, slot));
    }
}

fn set_param(track: usize, slot: usize, name: &str, value: f32) -> Invocation {
    Invocation::new(
        "mixer.set_effect_param",
        [
            track.to_string(),
            (slot + 1).to_string(),
            name.to_owned(),
            format!("{value}"),
        ],
    )
}

/// One undo step setting several parameters.
fn batch(invocations: Vec<Invocation>) -> Option<Invocation> {
    match invocations.len() {
        0 => None,
        1 => invocations.into_iter().next(),
        _ => Some(Invocation::new(
            BATCH,
            invocations.iter().map(ToString::to_string),
        )),
    }
}

/// What the editor shows for a project slot.
fn view(effect: &EffectSlot) -> EffectView {
    EffectView {
        kind: effect.kind,
        enabled: effect.enabled,
        values: effect.values,
        file: effect.file.clone(),
    }
}

pub(crate) fn show_all(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    // Windows whose slot is gone close.
    state.effect_windows.retain(|(t, s)| {
        project
            .mixer
            .get(*t)
            .is_some_and(|track| *s < track.effects.len())
    });
    let open = state.effect_windows.clone();
    for (index, (track, slot)) in open.into_iter().enumerate() {
        let Some(effect) = project.mixer.get(track).and_then(|t| t.effects.get(slot)) else {
            continue;
        };
        let track_name = project.mixer.get(track).map_or("", |t| t.name.as_str());
        let title = format!("{track_name} › {}", effect_title(effect));
        let mut keep = true;
        egui::Window::new(title)
            .id(egui::Id::new(("effect-window", track, slot)))
            .open(&mut keep)
            .resizable(false)
            .default_pos(
                state.workspace.left_top()
                    + vec2(60.0 + 24.0 * index as f32, 60.0 + 24.0 * index as f32),
            )
            .constrain_to(state.workspace)
            .show(ctx, |ui| {
                contents(ui, state, frame, track, slot, effect, actions)
            });
        if !keep {
            toggle(state, track, slot);
        }
    }
}

fn effect_title(effect: &EffectSlot) -> String {
    match &effect.plugin {
        Some(plugin) => plugin.name.clone(),
        None => crate::browser::effect_title(effect.kind).to_owned(),
    }
}

fn contents(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    if let Some(plugin) = &effect.plugin {
        let target = crate::plugins::Target::Effect(track, slot + 1);
        crate::plugins::panel(ui, state, frame.plugins, plugin, target, actions);
        return;
    }
    let id = egui::Id::new(("effect-editor", track, slot));
    let view = view(effect);
    let target = KnobTarget::Slot(track, slot);
    let mut edits = Vec::new();
    effect::preset_bar(ui, id, &view, &mut edits);
    ui.add_space(4.0);
    let sample_rate = match frame.settings_info.running_rate {
        0 => 48_000.0,
        rate => rate as f32,
    };
    if effect.kind == EffectKind::Convolution {
        impulse_picker(ui, state, track, slot, effect, actions);
    }
    if matches!(effect.kind, EffectKind::PitchCorrect | EffectKind::Tuner) {
        let detected = frame
            .engine
            .pitches
            .iter()
            .find(|(t, s, _)| *t == track && *s == slot)
            .map(|(_, _, hz)| *hz);
        effect::tuner(ui, &view, detected);
    } else if effect.kind == EffectKind::Rack {
        crate::rack_view::show(ui, state, track, slot, effect, actions);
    } else {
        state.effect_graph = Some(effect::graph(ui, id, &view, sample_rate, &mut edits));
    }
    ui.add_space(4.0);
    apply(state, target, effect, edits, actions);
    knob_row(ui, state, target, effect, actions);
}

/// The convolution reverb's impulse response: generated rooms or a file.
fn impulse_picker(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    let current = effect.file.clone().unwrap_or_else(|| "ir:hall".to_owned());
    let label = current.strip_prefix("ir:").map_or_else(
        || {
            std::path::Path::new(&current)
                .file_name()
                .map_or_else(|| current.clone(), |n| n.to_string_lossy().into_owned())
        },
        pretty,
    );
    ui.horizontal(|ui| {
        ui.label("Impulse");
        egui::ComboBox::from_id_salt(("impulse", track, slot))
            .selected_text(label)
            .show_ui(ui, |ui| {
                for name in musiccraft_project::params::IMPULSE_RESPONSES {
                    let key = format!("ir:{name}");
                    if ui.selectable_label(current == key, pretty(name)).clicked() {
                        actions.push(Action::Run(Invocation::new(
                            "mixer.set_effect_file",
                            [track.to_string(), (slot + 1).to_string(), key],
                        )));
                    }
                }
            })
            .response
            .hint("Generated rooms, from small to huge");
        if ui
            .button("Load file…")
            .hint("Use an audio file (a recorded impulse response) as the room")
            .clicked()
        {
            let start = effect.file.clone().filter(|f| !f.starts_with("ir:"));
            state.open_dialog(crate::dialogs::Dialog::set_effect_file(
                track,
                slot,
                start.unwrap_or_default(),
            ));
        }
    });
}

/// Which effect a row of knobs edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnobTarget {
    /// Effect `slot` (0-based) of mixer `track`.
    Slot(usize, usize),
    /// Effect `n` of `branch` (both 0-based) of the rack in `slot` of `track`.
    Rack(usize, usize, usize, usize),
}

impl KnobTarget {
    fn set(self, name: &str, value: f32) -> Invocation {
        match self {
            Self::Slot(track, slot) => set_param(track, slot, name, value),
            Self::Rack(track, slot, branch, n) => Invocation::new(
                "mixer.rack_param",
                [
                    track.to_string(),
                    (slot + 1).to_string(),
                    (branch + 1).to_string(),
                    (n + 1).to_string(),
                    name.to_owned(),
                    format!("{value}"),
                ],
            ),
        }
    }

    /// Bypasses the effect when `on`, turns it on otherwise.
    fn toggle(self, on: bool) -> Invocation {
        match self {
            Self::Slot(track, slot) => Invocation::new(
                "mixer.toggle_effect",
                [track.to_string(), (slot + 1).to_string()],
            ),
            Self::Rack(track, slot, branch, n) => Invocation::new(
                "mixer.rack_enable",
                [
                    track.to_string(),
                    (slot + 1).to_string(),
                    (branch + 1).to_string(),
                    (n + 1).to_string(),
                    if on { "off" } else { "on" }.to_owned(),
                ],
            ),
        }
    }
}

/// Turns the editor's edits into commands: drags coalesce into one undo step
/// per gesture, clicks are steps of their own.
fn apply(
    state: &mut UiState,
    target: KnobTarget,
    effect: &EffectSlot,
    edits: Vec<Edit>,
    actions: &mut Vec<Action>,
) {
    let invocations = |changes: Vec<(&str, f32)>| {
        batch(
            changes
                .into_iter()
                .map(|(name, value)| target.set(name, value))
                .collect(),
        )
    };
    for edit in edits {
        match edit {
            Edit::Begin => {
                state.begin_gesture();
            }
            Edit::Drag(changes) => {
                if let Some(invocation) = invocations(changes) {
                    actions.push(Action::Coalesced(invocation, state.gesture()));
                }
            }
            Edit::Set(changes) => {
                if let Some(invocation) = invocations(changes) {
                    actions.push(Action::Run(invocation));
                }
            }
            Edit::ToggleBypass => actions.push(Action::Run(target.toggle(effect.enabled))),
        }
    }
}

/// An effect's knobs (the ones its graph does not edit). Inside a rack the
/// routing knobs are left out.
pub(crate) fn knob_row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    target: KnobTarget,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    let in_rack = matches!(target, KnobTarget::Rack(..));
    let kind = effect.kind;
    let hidden = |p: usize| in_rack && effect::is_routing(kind, p);
    let mut edits = Vec::new();
    effect::knob_row(
        ui,
        &view(effect),
        &hidden,
        &mut edits,
        &mut |response: &egui::Response, spec: &ParamSpec| {
            if let KnobTarget::Slot(track, slot) = target {
                crate::automation::menu(
                    response,
                    &format!("mixer:{track}:{}:{}", slot + 1, spec.name),
                    actions,
                );
            }
        },
    );
    apply(state, target, effect, edits, actions);
}

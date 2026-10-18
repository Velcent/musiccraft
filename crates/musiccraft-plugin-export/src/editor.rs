//! The plugin's editor: the shared effect editor in a host window, with the
//! plugin's own controls (the convolution room, the sidechain and follow
//! switches). Edits become host parameter gestures.

use std::sync::{Arc, Mutex};

use musiccraft_editors::effect::{self, Edit, EffectView, GRAPH, has_knob, is_routing};
use musiccraft_editors::hint::Hint;
use musiccraft_params::{EffectKind, IMPULSE_RESPONSES};
use musiccraft_plugin_editor::EguiEditor;
use nih_plug::prelude::{GuiContext, ParamSetter};

use crate::params::{EffectParams, HostParam, Slot};

/// Knobs per row in the editor's width.
const KNOBS_PER_ROW: usize = 6;
/// Height of one row of knobs, with spacing.
const KNOB_ROW: u32 = 63;
/// Height of a row of ordinary controls.
const LINE: u32 = 26;

/// What the plugin draws when the host opens its GUI.
pub fn editor(params: Arc<EffectParams>) -> EguiEditor {
    let size = size(&params);
    // Parameters inside a host gesture; only the GUI thread touches it.
    let active: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    EguiEditor::new(size, move |ui, gui| draw(ui, gui, &params, &active))
}

/// The editor's logical size for an effect.
pub fn size(params: &EffectParams) -> (u32, u32) {
    let kind = params.kind;
    let knobs = (0..kind.params().len())
        .filter(|&p| has_knob(kind, p) && !is_routing(kind, p))
        .count();
    let rows = knobs.div_ceil(KNOBS_PER_ROW) as u32;
    let mut extra = 0;
    if kind == EffectKind::Convolution {
        extra += LINE;
    }
    if params.slots.iter().any(|s| is_routing(kind, s.index)) {
        extra += LINE;
    }
    let width = GRAPH.x as u32 + 20;
    let height = 20 + 2 * LINE + 8 + GRAPH.y as u32 + rows * KNOB_ROW + extra + 8;
    (width, height)
}

/// What the shared editor shows for the plugin's current values.
fn view(params: &EffectParams) -> EffectView {
    EffectView {
        kind: params.kind,
        enabled: !params.bypass.value(),
        values: params.values(),
        file: (params.kind == EffectKind::Convolution)
            .then(|| format!("ir:{}", params.shared.impulse_name())),
    }
}

fn draw(
    ui: &mut egui::Ui,
    gui: &dyn GuiContext,
    params: &EffectParams,
    active: &Mutex<Vec<&'static str>>,
) {
    let setter = ParamSetter::new(gui);
    let Ok(mut active) = active.lock() else {
        return;
    };
    ui.label(egui::RichText::new(params.kind.title()).strong().size(15.0));
    if params.kind == EffectKind::Convolution {
        room_picker(ui, params);
    }
    let view = view(params);
    let mut edits = Vec::new();
    let sample_rate = params.shared.sample_rate().unwrap_or(48_000.0);
    effect::editor(
        ui,
        egui::Id::new("musiccraft-effect"),
        &view,
        sample_rate,
        params.shared.detected(),
        &mut edits,
    );
    switches(ui, &setter, params);
    for edit in edits {
        apply(&setter, params, &mut active, edit);
    }
    // A gesture ends when the mouse lets go.
    if !ui.input(|i| i.pointer.any_down()) {
        for name in active.drain(..) {
            if let Some(slot) = params.slot(name) {
                end(&setter, slot);
            }
        }
    }
}

/// The convolution reverb's generated rooms.
fn room_picker(ui: &mut egui::Ui, params: &EffectParams) {
    let current = params.shared.impulse_name();
    ui.horizontal(|ui| {
        ui.label("Impulse");
        egui::ComboBox::from_id_salt("impulse")
            .selected_text(effect::pretty(&current))
            .show_ui(ui, |ui| {
                for room in IMPULSE_RESPONSES {
                    if ui
                        .selectable_label(current == room, effect::pretty(room))
                        .clicked()
                    {
                        params.shared.set_impulse(room);
                    }
                }
            })
            .response
            .hint("Generated rooms, from small to huge");
    });
}

/// The switches that replace routing: sidechain input, following notes.
fn switches(ui: &mut egui::Ui, setter: &ParamSetter<'_>, params: &EffectParams) {
    let kind = params.kind;
    for slot in params.slots.iter().filter(|s| is_routing(kind, s.index)) {
        let (text, hint) = match slot.spec.name {
            "sidechain" => (
                "Use the sidechain input",
                "Compress (or vocode) by the sidechain input instead of the signal itself",
            ),
            _ => (
                "Follow MIDI notes",
                "Correct toward the highest MIDI note held instead of the scale",
            ),
        };
        let mut on = slot.param.value() >= 0.5;
        if ui.checkbox(&mut on, text).hint(hint).changed() {
            set_once(setter, slot, if on { 1.0 } else { 0.0 });
        }
    }
}

fn apply(
    setter: &ParamSetter<'_>,
    params: &EffectParams,
    active: &mut Vec<&'static str>,
    edit: Edit,
) {
    match edit {
        Edit::Begin => {}
        Edit::Drag(changes) => {
            for (name, value) in changes {
                let Some(slot) = params.slot(name) else {
                    continue;
                };
                if !active.contains(&slot.spec.name) {
                    begin(setter, slot);
                    active.push(slot.spec.name);
                }
                set(setter, slot, value);
            }
        }
        Edit::Set(changes) => {
            for (name, value) in changes {
                if let Some(slot) = params.slot(name) {
                    set_once(setter, slot, value);
                }
            }
        }
        Edit::ToggleBypass => {
            let bypass = &params.bypass;
            setter.begin_set_parameter(bypass);
            setter.set_parameter(bypass, !bypass.value());
            setter.end_set_parameter(bypass);
        }
    }
}

fn begin(setter: &ParamSetter<'_>, slot: &Slot) {
    match &slot.param {
        HostParam::Float(p) => setter.begin_set_parameter(p),
        HostParam::Int(p) => setter.begin_set_parameter(p),
    }
}

fn set(setter: &ParamSetter<'_>, slot: &Slot, value: f32) {
    let value = slot.spec.sanitize(value);
    match &slot.param {
        HostParam::Float(p) => setter.set_parameter(p, value),
        HostParam::Int(p) => setter.set_parameter(p, value.round() as i32),
    }
}

fn end(setter: &ParamSetter<'_>, slot: &Slot) {
    match &slot.param {
        HostParam::Float(p) => setter.end_set_parameter(p),
        HostParam::Int(p) => setter.end_set_parameter(p),
    }
}

/// A one-off change: a gesture of its own.
fn set_once(setter: &ParamSetter<'_>, slot: &Slot, value: f32) {
    begin(setter, slot);
    set(setter, slot, value);
    end(setter, slot);
}

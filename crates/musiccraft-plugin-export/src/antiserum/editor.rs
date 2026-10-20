//! Antiserum's plugin editor: the app's Antiserum editor, with its edits
//! turned into host parameter gestures and patch changes. Choosing a sample,
//! adding a zone, saving and opening a patch use a path field at the top
//! instead of the app's dialogs.

use std::sync::{Arc, Mutex};

use egui::{RichText, Ui};
use musiccraft_antiserum::Data;
use musiccraft_antiserum_presets as presets;
use musiccraft_editors::antiserum::{AntiserumEdit, AntiserumEditor};
use musiccraft_editors::theme;
use musiccraft_patch::{Patch, SourceRef, ZoneRef};
use musiccraft_plugin_editor::EguiEditor;
use nih_plug::prelude::{GuiContext, ParamSetter};

use super::params::AntiserumParams;
use crate::editor::{begin, end, set, set_once};

/// The editor's logical size.
pub const SIZE: (u32, u32) = (740, 840);

/// What a path field at the top is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    Save,
    Open,
    /// An oscillator's sample.
    Sample(usize),
    /// A new zone for an oscillator.
    Zone(usize),
}

/// What the editor remembers between frames (GUI thread only).
#[derive(Default)]
struct View {
    editor: AntiserumEditor,
    /// Parameters inside a host gesture.
    active: Vec<usize>,
    ask: Option<(Ask, String)>,
    /// The last file operation's outcome.
    status: Option<String>,
}

/// What the plugin draws when the host opens its GUI.
pub fn editor(params: Arc<AntiserumParams>) -> EguiEditor {
    let view = Mutex::new(View::default());
    EguiEditor::new(SIZE, move |ui, gui| {
        if let Ok(mut view) = view.lock() {
            draw(ui, gui, &params, &mut view);
        }
    })
}

/// Where a new patch file goes: the home folder, named after the patch.
fn default_path(patch: &Patch) -> String {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
    let name = if patch.name.is_empty() {
        "Antiserum"
    } else {
        &patch.name
    };
    let file = format!("{}.json", name.replace(['/', '\\'], "-"));
    home.map_or(file.clone(), |h| {
        std::path::Path::new(&h).join(&file).display().to_string()
    })
}

fn draw(ui: &mut Ui, gui: &dyn GuiContext, params: &AntiserumParams, view: &mut View) {
    let setter = ParamSetter::new(gui);
    ask_row(ui, &setter, params, view);
    for warning in params.shared.warnings() {
        ui.label(RichText::new(warning).small().color(theme::HIGHLIGHT));
    }
    let values = params.values();
    let patch = params.patch();
    let mut edits = Vec::new();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            view.editor
                .show(ui, &values, &patch, &mut edits, &mut |_, _| {});
        });
    for edit in edits {
        apply(&setter, params, view, &patch, edit);
    }
    // A gesture ends when the mouse lets go.
    if !ui.input(|i| i.pointer.any_down()) {
        for index in view.active.drain(..) {
            if let Some(slot) = params.slots.get(index) {
                end(&setter, slot);
            }
        }
    }
}

fn apply(
    setter: &ParamSetter<'_>,
    params: &AntiserumParams,
    view: &mut View,
    patch: &Patch,
    edit: AntiserumEdit,
) {
    let shared = &params.shared;
    match edit {
        AntiserumEdit::Begin => {}
        AntiserumEdit::Set(index, value) => {
            let Some(slot) = params.slots.get(index) else {
                return;
            };
            if !view.active.contains(&index) {
                begin(setter, slot);
                view.active.push(index);
            }
            set(setter, slot, value);
        }
        AntiserumEdit::Table(osc, table) => shared.change(
            |p| p.set_table(osc, table),
            |p, _| {
                let table = presets::wavetable(p.table(osc))
                    .unwrap_or_else(|_| Arc::new(musiccraft_antiserum::Wavetable::basic()));
                Data::Table(osc, table)
            },
        ),
        AntiserumEdit::LfoShape(lfo, points) => shared.change(
            |p| p.set_lfo_shape(lfo, points),
            |p, _| Data::Shape(lfo, Box::new(presets::lfo_shape(p, lfo))),
        ),
        AntiserumEdit::Step(step, data) => shared.change(
            |p| p.set_step(step, data, (true, 0, 1.0, 0.5)),
            |p, _| Data::Sequence(Box::new(presets::sequence(p))),
        ),
        AntiserumEdit::Preset(name) => {
            if let Some(preset) = presets::find(&name) {
                load(setter, params, preset.clone());
            }
        }
        AntiserumEdit::Save => view.ask = Some((Ask::Save, default_path(patch))),
        AntiserumEdit::Open => view.ask = Some((Ask::Open, default_path(patch))),
        AntiserumEdit::ChooseSample(osc) => {
            view.ask = Some((Ask::Sample(osc), patch.source(osc).sample));
        }
        AntiserumEdit::AddZone(osc) => view.ask = Some((Ask::Zone(osc), String::new())),
        AntiserumEdit::SetZone(osc, zone, low, high, root) => change_source(params, osc, |s| {
            if let Some(z) = s.zones.get_mut(zone) {
                (z.low, z.high, z.root) = (low.min(high), low.max(high), root);
            }
        }),
        AntiserumEdit::RemoveZone(osc, zone) => change_source(params, osc, |s| {
            if zone < s.zones.len() {
                s.zones.remove(zone);
            }
        }),
    }
}

/// Changes oscillator `osc`'s source and hands the rebuilt one over.
fn change_source(params: &AntiserumParams, osc: usize, edit: impl FnOnce(&mut SourceRef)) {
    params.shared.change(
        |p| {
            let mut source = p.source(osc);
            edit(&mut source);
            p.set_source(osc, source);
        },
        |p, rate| {
            let (source, _) =
                presets::source(&p.source(osc), &mut |name| presets::load_sample(name, rate));
            Data::Source(osc, Arc::new(source))
        },
    );
}

/// Loads `patch`: every parameter through the host, then its data.
fn load(setter: &ParamSetter<'_>, params: &AntiserumParams, patch: Patch) {
    let values = presets::values(&patch);
    for slot in &params.slots {
        if let Some(value) = values.get(slot.index).copied()
            && value != slot.param.value()
        {
            set_once(setter, slot, value);
        }
    }
    params.shared.set_patch(patch);
}

/// The path field for saving, opening, a sample or a zone.
fn ask_row(ui: &mut Ui, setter: &ParamSetter<'_>, params: &AntiserumParams, view: &mut View) {
    if let Some(status) = &view.status {
        ui.label(RichText::new(status).small().color(theme::TEXT_DIM));
    }
    let Some((ask, text)) = view.ask.as_mut() else {
        return;
    };
    let ask = *ask;
    let mut done = None;
    ui.horizontal(|ui| {
        let label = match ask {
            Ask::Save => "Save patch to",
            Ask::Open => "Open patch",
            Ask::Sample(_) => "Sample (file or builtin:<sound>; empty clears)",
            Ask::Zone(_) => "Zone sample",
        };
        ui.label(RichText::new(label).small());
        let field = ui.add(egui::TextEdit::singleline(text).desired_width(320.0));
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if entered || ui.small_button("OK").clicked() {
            done = Some(true);
        }
        if ui.small_button("Cancel").clicked() {
            done = Some(false);
        }
    });
    let Some(ok) = done else {
        return;
    };
    let text = view
        .ask
        .take()
        .map(|(_, t)| t.trim().to_owned())
        .unwrap_or_default();
    if !ok {
        return;
    }
    view.status = match ask {
        Ask::Save => Some(
            params
                .patch()
                .to_json()
                .and_then(|json| std::fs::write(&text, json).map_err(|e| e.to_string()))
                .map_or_else(|e| format!("not saved: {e}"), |()| format!("saved {text}")),
        ),
        Ask::Open => Some(
            std::fs::read_to_string(&text)
                .map_err(|e| e.to_string())
                .and_then(|json| Patch::from_json(&json))
                .map_or_else(
                    |e| format!("not opened: {e}"),
                    |patch| {
                        load(setter, params, patch);
                        format!("opened {text}")
                    },
                ),
        ),
        Ask::Sample(osc) => {
            change_source(params, osc, |s| s.sample = text);
            None
        }
        Ask::Zone(osc) => {
            if !text.is_empty() {
                change_source(params, osc, |s| {
                    s.zones.push(ZoneRef::new(text, 0, 127, 60))
                });
            }
            None
        }
    };
}

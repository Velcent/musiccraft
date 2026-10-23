//! Antiserum's plugin editor: the app's Antiserum editor, with its edits
//! turned into host parameter gestures and patch changes. Choosing a sample
//! file, adding a zone from a file, saving and opening a patch use the
//! system's file dialog, as the app does; the sample slots' menus offer the
//! user's music and sample folders in place of the app's browser.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::{RichText, Ui};
use musiccraft_antiserum::Data;
use musiccraft_antiserum_presets as presets;
use musiccraft_editors::antiserum::{AntiserumEdit, AntiserumEditor};
use musiccraft_editors::theme;
use musiccraft_file_dialog::{FileDialog, Mode, Picker};
use musiccraft_patch::{Patch, SourceRef, ZoneRef};
use musiccraft_plugin_editor::EguiEditor;
use nih_plug::prelude::{GuiContext, ParamSetter};

use super::params::AntiserumParams;
use crate::editor::{begin, end, home, set, set_once, sound_dialog, sound_folders};

/// The editor's logical size.
pub const SIZE: (u32, u32) = (740, 840);

/// What a file dialog is for.
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
    /// The open file dialog.
    files: Picker<Ask>,
    /// The folders the sample slots' menus offer.
    folders: Vec<PathBuf>,
    /// The last file operation's outcome.
    status: Option<String>,
}

/// What the plugin draws when the host opens its GUI.
pub fn editor(params: Arc<AntiserumParams>) -> EguiEditor {
    let view = Mutex::new(View {
        folders: sound_folders(),
        ..View::default()
    });
    EguiEditor::new(SIZE, move |ui, gui| {
        if let Ok(mut view) = view.lock() {
            draw(ui, gui, &params, &mut view);
        }
    })
}

/// The file dialog for `ask`. Patches start in the user presets folder,
/// shared with the app (a patch saved there is a preset in both), named
/// after the patch.
fn dialog(ask: Ask, patch: &Patch) -> (FileDialog, Mode) {
    let dialog = FileDialog::new().set_can_create_directories(true);
    match ask {
        Ask::Save | Ask::Open => {
            let dialog = dialog
                .add_filter("Antiserum patch", &["json"])
                .set_directory(presets::user::folder().or_else(home).unwrap_or_default());
            if ask == Ask::Open {
                return (dialog.set_title("Open Antiserum patch"), Mode::File);
            }
            let name = if patch.name.is_empty() {
                "Antiserum"
            } else {
                &patch.name
            };
            let file = format!("{}.json", name.replace(['/', '\\'], "-"));
            let dialog = dialog.set_title("Save Antiserum patch").set_file_name(file);
            (dialog, Mode::Save)
        }
        Ask::Sample(_) => (sound_dialog("Oscillator sample"), Mode::File),
        Ask::Zone(_) => (sound_dialog("Zone sample"), Mode::File),
    }
}

/// Opens the file dialog for `ask`, unless one is open already.
fn ask(view: &mut View, ask: Ask, patch: &Patch, ctx: &egui::Context) {
    let (dialog, mode) = dialog(ask, patch);
    let repaint = ctx.clone();
    if !view
        .files
        .show(dialog, mode, ask, move || repaint.request_repaint())
    {
        view.status = Some("a file dialog is already open".to_owned());
    }
}

fn draw(ui: &mut Ui, gui: &dyn GuiContext, params: &AntiserumParams, view: &mut View) {
    let setter = ParamSetter::new(gui);
    answers(ui, &setter, params, view);
    view.editor.set_sounds(&view.folders, None);
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
        apply(&setter, params, view, &patch, edit, ui.ctx());
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
    ctx: &egui::Context,
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
            if let Some(preset) = presets::user::find_any(&name) {
                load(setter, params, preset);
            }
        }
        AntiserumEdit::Save => ask(view, Ask::Save, patch, ctx),
        AntiserumEdit::Open => ask(view, Ask::Open, patch, ctx),
        AntiserumEdit::ChooseSample(osc) => ask(view, Ask::Sample(osc), patch, ctx),
        AntiserumEdit::AddZone(osc) => ask(view, Ask::Zone(osc), patch, ctx),
        AntiserumEdit::Sample(osc, sample) => change_source(params, osc, |s| s.sample = sample),
        AntiserumEdit::Zone(osc, sample) => add_zone(params, osc, sample),
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

/// Adds a zone playing `sample` over the whole keyboard.
fn add_zone(params: &AntiserumParams, osc: usize, sample: String) {
    if !sample.is_empty() {
        change_source(params, osc, |s| {
            s.zones.push(ZoneRef::new(sample, 0, 127, 60));
        });
    }
}

/// The last file operation's outcome, and the answer of the open file
/// dialog once there is one.
fn answers(ui: &mut Ui, setter: &ParamSetter<'_>, params: &AntiserumParams, view: &mut View) {
    if let Some(status) = &view.status {
        ui.label(RichText::new(status).small().color(theme::TEXT_DIM));
    }
    let Some((ask, paths)) = view.files.poll() else {
        return;
    };
    // Cancelled: nothing changes.
    let Some(path) = paths.into_iter().next() else {
        return;
    };
    let text = path.display().to_string();
    view.status = match ask {
        Ask::Save => Some(
            params
                .patch()
                .to_json()
                .and_then(|json| {
                    // The user presets folder may not exist yet.
                    if let Some(parent) = std::path::Path::new(&text).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::write(&text, json).map_err(|e| e.to_string())
                })
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
            add_zone(params, osc, text);
            None
        }
    };
}

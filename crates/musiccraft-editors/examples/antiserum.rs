//! Antiserum's editor in a window, applying its own edits to a patch:
//! `cargo run -p musiccraft-editors --example antiserum [preset] [page]`
//! (pages: osc, mixer, filters, mod, matrix, fx, arp, global).

use musiccraft_antiserum::index_of;
use musiccraft_editors::antiserum::{AntiserumEdit, AntiserumEditor, Page};
use musiccraft_editors::theme;
use musiccraft_patch::Patch;

struct App {
    patch: Patch,
    editor: AntiserumEditor,
}

impl App {
    fn apply(&mut self, edit: AntiserumEdit) {
        match edit {
            AntiserumEdit::Set(index, value) => {
                if let Some(spec) = musiccraft_antiserum::params().get(index) {
                    self.patch.params.insert(spec.name.to_owned(), value);
                }
            }
            AntiserumEdit::Table(osc, table) => self.patch.set_table(osc, table),
            AntiserumEdit::LfoShape(lfo, points) => self.patch.set_lfo_shape(lfo, points),
            AntiserumEdit::Step(step, data) => {
                self.patch.set_step(step, data, (true, 0, 1.0, 0.5));
            }
            AntiserumEdit::Preset(name) => {
                if let Some(p) = musiccraft_antiserum_presets::find(&name) {
                    self.patch = p.clone();
                }
            }
            AntiserumEdit::Begin | AntiserumEdit::Save | AntiserumEdit::Open => {}
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            let values = musiccraft_antiserum_presets::values(&self.patch);
            let mut edits = Vec::new();
            self.editor
                .show(ui, &values, &self.patch, &mut edits, &mut |_, _| {});
            for edit in edits {
                self.apply(edit);
            }
        });
    }
}

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let patch = args
        .next()
        .and_then(|name| musiccraft_antiserum_presets::find(&name).cloned())
        .unwrap_or_default();
    let mut editor = AntiserumEditor::default();
    editor.page = match args.next().as_deref() {
        Some("mixer") => Page::Mixer,
        Some("filters") => Page::Filters,
        Some("mod") => Page::Modulation,
        Some("matrix") => Page::Matrix,
        Some("fx") => Page::Effects,
        Some("arp") => Page::ArpSeq,
        Some("global") => Page::Global,
        _ => Page::Oscillators,
    };
    // Keep the binary honest about the parameter names it shows.
    debug_assert!(index_of("volume").is_some());
    eframe::run_native(
        "Antiserum",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([680.0, 820.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(App { patch, editor }))
        }),
    )
}

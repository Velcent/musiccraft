//! The wavetable editor in a window, on the factory tables, with undo:
//! `cargo run -p musiccraft-editors --example wavetable`.

use musiccraft_editors::theme;
use musiccraft_editors::wavetable::{WavetableEdit, WavetableEditor};
use musiccraft_wavetables::{Table, factory};

struct App {
    table: Table,
    editor: WavetableEditor,
    undo: Vec<Table>,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            if ui.button("Undo").clicked()
                && let Some(previous) = self.undo.pop()
            {
                self.table = previous;
            }
            let mut edits = Vec::new();
            self.editor.show(ui, &self.table, &mut edits);
            for edit in edits {
                match edit {
                    WavetableEdit::Begin => self.undo.push(self.table.clone()),
                    WavetableEdit::Op(op) => {
                        if let Err(error) = op.apply(&mut self.table) {
                            eprintln!("{error}");
                        }
                    }
                    WavetableEdit::Factory(name) => {
                        self.table = factory::table(name).unwrap_or_default();
                    }
                }
            }
        });
    }
}

fn main() -> eframe::Result {
    let table =
        factory::table(std::env::args().nth(1).as_deref().unwrap_or("formant")).unwrap_or_default();
    eframe::run_native(
        "Wavetable editor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([460.0, 520.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(App {
                table,
                editor: WavetableEditor::default(),
                undo: Vec::new(),
            }))
        }),
    )
}

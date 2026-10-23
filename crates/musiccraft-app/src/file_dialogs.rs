//! The system's file dialogs for the UI's requests (`musiccraft_ui::FilePick`,
//! `docs/spec/style/dialogs.md`): one at a time, attached to the main window,
//! without blocking it.

use musiccraft_file_dialog::{FileDialog, Mode};
use musiccraft_ui::{FilePick, PickMode};

/// The dialog `pick` asks for, attached to `parent`.
pub fn dialog(pick: &FilePick, parent: &eframe::Frame) -> (FileDialog, Mode) {
    let mut dialog = FileDialog::new()
        .set_title(pick.title)
        .set_can_create_directories(true)
        .set_parent(parent);
    if let Some(dir) = &pick.directory {
        dialog = dialog.set_directory(dir);
    }
    if let Some(name) = &pick.file_name {
        dialog = dialog.set_file_name(name);
    }
    for (name, extensions) in &pick.filters {
        dialog = dialog.add_filter(*name, extensions);
    }
    let mode = match pick.mode() {
        PickMode::File => Mode::File,
        PickMode::Files => Mode::Files,
        PickMode::Save => Mode::Save,
        PickMode::Folder => Mode::Folder,
    };
    (dialog, mode)
}

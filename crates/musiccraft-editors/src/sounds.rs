//! Choosing the sound a sample slot plays (`docs/spec/style/dialogs.md`,
//! "Sample slots"): a menu with "Browse files…" (the system's file dialog,
//! which the caller opens), the built-in sounds and the browser's folders as
//! submenus, and "Clear"; and a drop target for sounds dragged from the
//! browser. The app's Antiserum and sampler slots and the Antiserum plugin
//! all use it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Rect, RichText, Ui};
use musiccraft_params::{BUILTIN_PREFIX, BUILTIN_SAMPLES};

use crate::hint::Hint;
use crate::theme;

/// A folder entry: its name, path and whether it is a folder.
pub type Entry = (String, PathBuf, bool);

/// How deep folders can be opened (and searched).
pub const MAX_DEPTH: usize = 8;

/// The file types sample slots play.
pub const EXTENSIONS: [&str; 1] = ["wav"];

/// Whether `path` is a sound a slot can play.
#[must_use]
pub fn is_sound(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| EXTENSIONS.iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// The visible subfolders of `dir`, then its sounds, each by name.
#[must_use]
pub fn list(dir: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .filter_map(|e| {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        return None;
                    }
                    let is_dir = path.is_dir();
                    (is_dir || is_sound(&path)).then_some((name, path, is_dir))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by_key(|e| (!e.2, e.0.to_lowercase()));
    entries
}

/// Folder listings, read once and kept until cleared.
#[derive(Debug, Clone, Default)]
pub struct Listings(BTreeMap<PathBuf, Vec<Entry>>);

impl Listings {
    /// The listing of `dir`.
    pub fn get(&mut self, dir: &Path) -> &[Entry] {
        self.0.entry(dir.to_path_buf()).or_insert_with(|| list(dir))
    }

    /// Forgets every listing, so folders are read from disk again.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// What the user chose in a sample slot's menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleChoice {
    /// Open the system's file dialog.
    Browse,
    /// Play this sound (a file path or a `builtin:` name).
    Use(String),
    /// Leave the slot empty.
    Clear,
}

/// Where a slot's menu finds sounds: the browser's folders, and their
/// listings.
pub struct Sounds<'a> {
    pub folders: &'a [PathBuf],
    pub listings: &'a mut Listings,
}

/// The menu's items, for a menu or context menu the caller opens. `clear`
/// offers "Clear".
pub fn menu_items(ui: &mut Ui, sounds: &mut Sounds<'_>, clear: bool) -> Option<SampleChoice> {
    let mut choice = None;
    if ui
        .button("Browse files…")
        .hint("Choose a sound file with the system's file dialog")
        .clicked()
    {
        choice = Some(SampleChoice::Browse);
    }
    ui.separator();
    ui.menu_button("Sound packs", |ui| {
        for name in BUILTIN_SAMPLES {
            if ui.button(name.replace('_', " ")).clicked() {
                choice = Some(SampleChoice::Use(format!("{BUILTIN_PREFIX}{name}")));
            }
        }
    });
    for dir in sounds.folders {
        let label = dir.file_name().map_or_else(
            || dir.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        folder(ui, sounds.listings, dir, &label, 0, &mut choice);
    }
    if clear {
        ui.separator();
        if ui.button("Clear").clicked() {
            choice = Some(SampleChoice::Clear);
        }
    }
    if choice.is_some() {
        ui.close();
    }
    choice
}

/// A folder as a submenu: its subfolders, then its sounds.
fn folder(
    ui: &mut Ui,
    listings: &mut Listings,
    dir: &Path,
    label: &str,
    depth: usize,
    choice: &mut Option<SampleChoice>,
) {
    ui.menu_button(label, |ui| {
        let entries = listings.get(dir).to_vec();
        if entries.is_empty() {
            ui.label(RichText::new("no sounds").color(theme::TEXT_DIM));
        }
        egui::ScrollArea::vertical()
            .max_height(420.0)
            .show(ui, |ui| {
                for (name, path, is_dir) in entries {
                    if is_dir {
                        if depth < MAX_DEPTH {
                            folder(ui, listings, &path, &name, depth + 1, choice);
                        }
                    } else if ui.button(&name).clicked() {
                        *choice = Some(SampleChoice::Use(path.display().to_string()));
                    }
                }
            });
    });
}

/// A small button that opens the menu. Folders are read afresh each time it
/// opens.
pub fn button(
    ui: &mut Ui,
    text: &str,
    hint: &str,
    sounds: &mut Sounds<'_>,
    clear: bool,
) -> Option<SampleChoice> {
    let (response, inner) = egui::containers::menu::MenuButton::from_button(egui::Button::new(
        RichText::new(text).size(theme::FONT_LABEL),
    ))
    .ui(ui, |ui| menu_items(ui, sounds, clear));
    if response.clicked() {
        sounds.listings.clear();
    }
    response.hint(hint);
    inner.and_then(|inner| inner.inner)
}

/// While `dragged` (a sound dragged from the browser) is over `rect`, and
/// nothing is drawn above it there, outlines `rect`; returns the sound when
/// it is released there.
pub fn drop_target(ui: &Ui, rect: Rect, dragged: Option<&str>) -> Option<String> {
    let dragged = dragged?;
    let pointer = ui.ctx().pointer_hover_pos()?;
    let over = rect.contains(pointer)
        && ui.clip_rect().contains(pointer)
        && ui.ctx().layer_id_at(pointer) == Some(ui.layer_id());
    if !over {
        return None;
    }
    ui.ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("sample-slot-drop"),
        ))
        .rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    ui.input(|i| i.pointer.any_released())
        .then(|| dragged.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listings_show_folders_then_sounds_and_skip_the_rest() {
        let dir = std::env::temp_dir().join(format!("mc-sounds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Loops")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        for file in ["b.WAV", "a.wav", "notes.txt"] {
            std::fs::write(dir.join(file), b"").unwrap();
        }
        let names: Vec<_> = list(&dir).into_iter().map(|e| (e.0, e.2)).collect();
        assert_eq!(
            names,
            [
                ("Loops".to_owned(), true),
                ("a.wav".to_owned(), false),
                ("b.WAV".to_owned(), false)
            ]
        );
        let mut listings = Listings::default();
        assert_eq!(listings.get(&dir).len(), 3);
        std::fs::write(dir.join("c.wav"), b"").unwrap();
        assert_eq!(listings.get(&dir).len(), 3, "kept until cleared");
        listings.clear();
        assert_eq!(listings.get(&dir).len(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

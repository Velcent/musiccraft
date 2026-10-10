//! Side panel for finding sounds: built-in sounds, instruments, samples used by
//! the project, and folders on disk. Click previews, double-click adds a
//! channel, and items can be dragged onto the channel rack.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{RichText, Sense};
use musiccraft_commands::Invocation;
use musiccraft_project::Instrument;
use musiccraft_project::params::{BUILTIN_PREFIX, BUILTIN_SAMPLES};

use crate::{Action, Frame, UiState, dialogs::Dialog, theme};

/// Something being dragged out of the browser: a sample reference, or `None`
/// for a new synth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dragged(pub Option<String>);

impl Dragged {
    /// The command that adds this item as a channel.
    #[must_use]
    pub fn add_channel(&self) -> Invocation {
        match &self.0 {
            Some(sample) => Invocation::new("channel.add_sampler", [sample.clone()]),
            None => Invocation::bare("channel.add_synth"),
        }
    }

    /// Short label for the drag ghost.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.0 {
            Some(sample) => sample
                .strip_prefix(BUILTIN_PREFIX)
                .map(str::to_owned)
                .or_else(|| {
                    Path::new(sample)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| sample.clone()),
            None => "Synth".to_owned(),
        }
    }
}

/// Browser state: search text, folders and their cached listings.
#[derive(Debug, Clone, Default)]
pub struct BrowserState {
    search: String,
    /// Folders shown under "Folders".
    pub folders: Vec<PathBuf>,
    listings: BTreeMap<PathBuf, Vec<(String, PathBuf, bool)>>,
}

impl BrowserState {
    /// Starts with the user's music and sample folders, when they exist.
    #[must_use]
    pub fn with_default_folders() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let folders = home
            .map(|h| {
                ["Music", "Samples", "Music/Samples"]
                    .iter()
                    .map(|d| h.join(d))
                    .filter(|p| p.is_dir())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            folders,
            ..Self::default()
        }
    }

    /// Subfolders and WAV files of `dir`, cached until refreshed.
    fn listing(&mut self, dir: &Path) -> &[(String, PathBuf, bool)] {
        self.listings.entry(dir.to_path_buf()).or_insert_with(|| {
            let mut entries: Vec<(String, PathBuf, bool)> = std::fs::read_dir(dir)
                .map(|it| {
                    it.filter_map(Result::ok)
                        .filter_map(|e| {
                            let path = e.path();
                            let name = e.file_name().to_string_lossy().into_owned();
                            if name.starts_with('.') {
                                return None;
                            }
                            let is_dir = path.is_dir();
                            let is_wav = path
                                .extension()
                                .is_some_and(|x| x.eq_ignore_ascii_case("wav"));
                            (is_dir || is_wav).then_some((name, path, is_dir))
                        })
                        .collect()
                })
                .unwrap_or_default();
            entries.sort_by_key(|e| (!e.2, e.0.to_lowercase()));
            entries
        })
    }
}

fn matches(search: &str, name: &str) -> bool {
    search.is_empty() || name.to_lowercase().contains(&search.to_lowercase())
}

/// One browser entry. Returns nothing; pushes actions as the user interacts.
fn item(ui: &mut egui::Ui, label: &str, dragged: Dragged, actions: &mut Vec<Action>) {
    let response = ui
        .add(
            egui::Button::new(RichText::new(format!("♪  {label}")))
                .frame(false)
                .sense(Sense::click_and_drag()),
        )
        .on_hover_text("Click to preview · double-click to add · drag onto the channel rack");
    if response.drag_started() {
        response.dnd_set_drag_payload(dragged.clone());
    }
    if response.double_clicked() {
        actions.push(Action::Run(dragged.add_channel()));
    } else if response.clicked() {
        match &dragged.0 {
            Some(sample) => actions.push(Action::PreviewSample(sample.clone())),
            None => actions.push(Action::Run(dragged.add_channel())),
        }
    }
}

fn folder(
    ui: &mut egui::Ui,
    state: &mut UiState,
    dir: &Path,
    label: &str,
    depth: usize,
    actions: &mut Vec<Action>,
) {
    egui::CollapsingHeader::new(RichText::new(label).color(theme::TEXT))
        .id_salt(dir)
        .default_open(false)
        .show(ui, |ui| {
            let entries = state.browser.listing(dir).to_vec();
            if entries.is_empty() {
                ui.label(RichText::new("no WAV files").small().color(theme::TEXT_DIM));
            }
            let search = state.browser.search.clone();
            for (name, path, is_dir) in entries {
                if is_dir {
                    if depth < 8 {
                        folder(ui, state, &path, &name, depth + 1, actions);
                    }
                } else if matches(&search, &name) {
                    item(
                        ui,
                        &name,
                        Dragged(Some(path.display().to_string())),
                        actions,
                    );
                }
            }
        });
}

/// Installed plugins: click an instrument to add a channel, an effect to add
/// it to the selected mixer track.
fn plugin_section(
    ui: &mut egui::Ui,
    state: &UiState,
    frame: &Frame<'_>,
    search: &str,
    actions: &mut Vec<Action>,
) {
    let plugins = frame.plugins;
    if !plugins.available {
        return;
    }
    egui::CollapsingHeader::new(
        RichText::new(format!("Plugins ({})", plugins.catalog.len())).strong(),
    )
    .default_open(false)
    .show(ui, |ui| {
        for entry in plugins.catalog.iter().filter(|p| matches(search, &p.name)) {
            let (icon, hint, invocation) = if entry.instrument {
                (
                    "♪",
                    "Click to add a channel",
                    Invocation::new("channel.add_plugin", [entry.id.clone(), entry.name.clone()]),
                )
            } else {
                (
                    "fx",
                    "Click to add to the selected mixer track",
                    Invocation::new(
                        "mixer.add_plugin",
                        [
                            state.selected_track.to_string(),
                            entry.id.clone(),
                            entry.name.clone(),
                        ],
                    ),
                )
            };
            let button = egui::Button::new(format!("{icon}  {}", entry.name)).frame(false);
            if ui
                .add(button)
                .on_hover_text(format!("{} · {}\n{hint}", entry.vendor, entry.id))
                .clicked()
            {
                actions.push(Action::Run(invocation));
            }
        }
        if ui.small_button("Rescan").clicked() {
            actions.push(Action::RescanPlugins);
        }
    });
}

pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::left("browser")
        .resizable(true)
        .default_size(190.0)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("BROWSER")
                        .small()
                        .strong()
                        .color(theme::TEXT_DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("⟳")
                        .on_hover_text("Refresh folders")
                        .clicked()
                    {
                        state.browser.listings.clear();
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(&mut state.browser.search)
                    .hint_text("Search…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("browser-scroll")
                .show(ui, |ui| {
                    let search = state.browser.search.clone();
                    egui::CollapsingHeader::new(RichText::new("Built-in sounds").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            for name in BUILTIN_SAMPLES.iter().filter(|n| matches(&search, n)) {
                                item(
                                    ui,
                                    &name.replace('_', " "),
                                    Dragged(Some(format!("{BUILTIN_PREFIX}{name}"))),
                                    actions,
                                );
                            }
                        });
                    egui::CollapsingHeader::new(RichText::new("Instruments").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            if matches(&search, "synth") {
                                item(ui, "Synth", Dragged(None), actions);
                            }
                        });
                    plugin_section(ui, state, frame, &search, actions);
                    let mut used: Vec<&str> = frame
                        .project
                        .channels
                        .iter()
                        .filter_map(|c| match &c.instrument {
                            Instrument::Sampler { sample }
                                if !sample.starts_with(BUILTIN_PREFIX) =>
                            {
                                Some(sample.as_str())
                            }
                            _ => None,
                        })
                        .collect();
                    used.sort_unstable();
                    used.dedup();
                    egui::CollapsingHeader::new(
                        RichText::new(format!("Project samples ({})", used.len())).strong(),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        for sample in used {
                            let name = Path::new(sample).file_name().map_or_else(
                                || sample.to_owned(),
                                |n| n.to_string_lossy().into_owned(),
                            );
                            if matches(&search, &name) {
                                item(ui, &name, Dragged(Some(sample.to_owned())), actions);
                            }
                        }
                    });
                    egui::CollapsingHeader::new(RichText::new("Folders").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            for dir in state.browser.folders.clone() {
                                let label = dir.file_name().map_or_else(
                                    || dir.display().to_string(),
                                    |n| n.to_string_lossy().into_owned(),
                                );
                                folder(ui, state, &dir, &label, 0, actions);
                            }
                            ui.horizontal(|ui| {
                                if ui.small_button("+ Add folder…").clicked() {
                                    state.open_dialog(Dialog::add_folder());
                                }
                                if ui
                                    .small_button("+ WAV file…")
                                    .on_hover_text("Add a channel from a file path")
                                    .clicked()
                                {
                                    state.open_dialog(Dialog::add_sample(frame.project));
                                }
                            });
                        });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragged_items_add_the_right_channel() {
        let kick = Dragged(Some("builtin:kick".into()));
        assert_eq!(
            kick.add_channel().to_string(),
            "channel.add_sampler builtin:kick"
        );
        assert_eq!(kick.label(), "kick");
        let file = Dragged(Some("/x/My Loop.wav".into()));
        assert_eq!(
            file.add_channel().to_string(),
            "channel.add_sampler \"/x/My Loop.wav\""
        );
        assert_eq!(file.label(), "My Loop.wav");
        assert_eq!(Dragged(None).add_channel().to_string(), "channel.add_synth");
    }

    #[test]
    fn folder_listing_shows_dirs_then_wavs() {
        let dir = std::env::temp_dir().join(format!("musiccraft-browser-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Drums")).unwrap();
        std::fs::write(dir.join("b.WAV"), b"").unwrap();
        std::fs::write(dir.join("a.wav"), b"").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();
        let mut browser = BrowserState::default();
        let names: Vec<String> = browser.listing(&dir).iter().map(|e| e.0.clone()).collect();
        assert_eq!(names, ["Drums", "a.wav", "b.WAV"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

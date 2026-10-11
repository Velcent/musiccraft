//! Side panel for finding things: the current project, the plugin database
//! (generators and effects), sound packs and folders on disk. Click previews,
//! double-click adds a channel, and sounds can be dragged onto the channel
//! rack or the playlist.

use crate::hint::Hint;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{RichText, Sense};
use musiccraft_commands::Invocation;
use musiccraft_project::Instrument;
use musiccraft_project::params::{BUILTIN_PREFIX, BUILTIN_SAMPLES, EFFECT_KINDS};

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

/// Browser state: search text and cached folder listings. The folders
/// themselves are a setting.
#[derive(Debug, Clone, Default)]
pub struct BrowserState {
    search: String,
    listings: BTreeMap<PathBuf, Vec<(String, PathBuf, bool)>>,
}

/// The user's music and sample folders, when they exist.
#[must_use]
pub fn default_folders() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    home.map(|h| {
        ["Music", "Samples", "Music/Samples"]
            .iter()
            .map(|d| h.join(d))
            .filter(|p| p.is_dir())
            .collect()
    })
    .unwrap_or_default()
}

impl BrowserState {
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
        .hint("Click to preview · double-click to add · drag onto the channel rack · right-click for more");
    if response.drag_started() {
        response.dnd_set_drag_payload(dragged.clone());
    }
    if let Some(sample) = dragged.0.clone() {
        response.context_menu(|ui| {
            if ui.button("Add as sampler").clicked() {
                actions.push(Action::Run(dragged.add_channel()));
                ui.close();
            }
            if ui.button("Add as granular instrument").clicked() {
                actions.push(Action::Run(Invocation::new(
                    "channel.add_granular",
                    [sample.clone()],
                )));
                ui.close();
            }
        });
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

fn heading(text: &str) -> RichText {
    RichText::new(text).strong()
}

fn plugin_button(ui: &mut egui::Ui, label: &str, hint: String) -> bool {
    ui.add(egui::Button::new(label).frame(false).truncate())
        .hint(hint)
        .clicked()
}

/// Generators (instruments) and effects, built-in and installed. Click a
/// generator to add a channel, an effect to add it to the selected mixer track.
fn plugin_database(
    ui: &mut egui::Ui,
    state: &UiState,
    frame: &Frame<'_>,
    search: &str,
    actions: &mut Vec<Action>,
) {
    let plugins = frame.plugins;
    let track = state.selected_track.to_string();
    egui::CollapsingHeader::new(heading("Plugin database"))
        .default_open(true)
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Generators")
                .default_open(true)
                .show(ui, |ui| {
                    if matches(search, "synth") {
                        item(ui, "Synth", Dragged(None), actions);
                    }
                    for kind in musiccraft_project::params::DRUM_TYPES {
                        let label = format!("Drum synth: {kind}");
                        if matches(search, &label)
                            && plugin_button(
                                ui,
                                &format!("♪  {label}"),
                                "Built-in drum synth — click to add a channel".to_owned(),
                            )
                        {
                            actions.push(Action::Run(Invocation::new("channel.add_drum", [kind])));
                        }
                    }
                    if matches(search, "pluck")
                        && plugin_button(
                            ui,
                            "♪  Pluck",
                            "Built-in plucked string — click to add a channel".to_owned(),
                        )
                    {
                        actions.push(Action::Run(Invocation::bare("channel.add_pluck")));
                    }
                    let more: [(&str, &str, Invocation); 7] = [
                        (
                            "FM synth",
                            "Four-operator FM synth",
                            Invocation::bare("channel.add_fm"),
                        ),
                        (
                            "Keys: piano",
                            "Piano",
                            Invocation::new("channel.add_keys", ["piano"]),
                        ),
                        (
                            "Keys: electric piano",
                            "Electric piano",
                            Invocation::new("channel.add_keys", ["epiano"]),
                        ),
                        (
                            "Keys: organ",
                            "Organ",
                            Invocation::new("channel.add_keys", ["organ"]),
                        ),
                        (
                            "Additive",
                            "Additive (harmonic) synth",
                            Invocation::bare("channel.add_additive"),
                        ),
                        (
                            "Drum kit",
                            "16-pad sample kit",
                            Invocation::bare("channel.add_kit"),
                        ),
                        (
                            "Layer",
                            "Plays several channels at once",
                            Invocation::bare("channel.add_layer"),
                        ),
                    ];
                    for (label, hint, invocation) in more {
                        if matches(search, label)
                            && plugin_button(
                                ui,
                                &format!("♪  {label}"),
                                format!("{hint} — click to add a channel"),
                            )
                        {
                            actions.push(Action::Run(invocation));
                        }
                    }
                    for entry in plugins.of_kind(true).filter(|p| matches(search, &p.name)) {
                        let hint =
                            format!("{} · {} — click to add a channel", entry.vendor, entry.id);
                        if plugin_button(ui, &format!("♪  {}", entry.name), hint) {
                            actions.push(Action::Run(Invocation::new(
                                "channel.add_plugin",
                                [entry.id.clone(), entry.name.clone()],
                            )));
                        }
                    }
                });
            egui::CollapsingHeader::new("Effects")
                .default_open(false)
                .show(ui, |ui| {
                    for kind in EFFECT_KINDS.iter().filter(|k| matches(search, k.name())) {
                        let hint = format!(
                            "Built-in {} — click to add to the selected mixer track",
                            kind.name()
                        );
                        if plugin_button(ui, &format!("fx  {}", effect_title(*kind)), hint) {
                            actions.push(Action::Run(Invocation::new(
                                "mixer.add_effect",
                                [track.clone(), kind.name().to_owned()],
                            )));
                        }
                    }
                    for entry in plugins.of_kind(false).filter(|p| matches(search, &p.name)) {
                        let hint = format!(
                            "{} · {} — click to add to the selected mixer track",
                            entry.vendor, entry.id
                        );
                        if plugin_button(ui, &format!("fx  {}", entry.name), hint) {
                            actions.push(Action::Run(Invocation::new(
                                "mixer.add_plugin",
                                [track.clone(), entry.id.clone(), entry.name.clone()],
                            )));
                        }
                    }
                });
            if plugins.available && ui.small_button("Rescan plugins").clicked() {
                actions.push(Action::RescanPlugins);
            }
        });
}

/// Display name of a built-in effect.
#[must_use]
pub fn effect_title(kind: musiccraft_project::params::EffectKind) -> &'static str {
    use musiccraft_project::params::EffectKind as K;
    match kind {
        K::Filter => "Filter",
        K::Eq => "Equalizer",
        K::Delay => "Delay",
        K::Reverb => "Reverb",
        K::Compressor => "Compressor",
        K::Distortion => "Distortion",
        K::Chorus => "Chorus",
        K::Limiter => "Limiter",
        K::Phaser => "Phaser",
        K::Flanger => "Flanger",
        K::Bitcrusher => "Bitcrusher",
        K::Stereo => "Stereo tool",
        K::Gate => "Trance gate",
        K::Autopan => "Auto-pan",
        K::BassBoost => "Bass boost",
        K::Overdrive => "Overdrive",
        K::SoftClipper => "Soft clipper",
        K::Waveshaper => "Waveshaper",
        K::DcBlock => "DC remover",
        K::RingMod => "Ring modulator",
        K::FreqShift => "Frequency shifter",
        K::Formant => "Vowel filter",
        K::AutoFilter => "Auto filter",
        K::NoiseGate => "Noise gate",
        K::Transient => "Transient shaper",
        K::MultiTap => "Multi-tap delay",
        K::GraphicEq => "Graphic EQ",
        K::ParametricEq => "Parametric EQ",
        K::Multiband => "Multiband compressor",
        K::Vocoder => "Vocoder",
        K::PitchShift => "Pitch shifter",
        K::Stutter => "Stutter",
        K::Convolution => "Convolution reverb",
        K::Plugin => "Plugin",
    }
}

/// Patterns, automation and samples of the open project.
fn current_project(ui: &mut egui::Ui, frame: &Frame<'_>, search: &str, actions: &mut Vec<Action>) {
    let project = frame.project;
    egui::CollapsingHeader::new(heading("Current project"))
        .default_open(false)
        .show(ui, |ui| {
            egui::CollapsingHeader::new(format!("Patterns ({})", project.patterns.len())).show(
                ui,
                |ui| {
                    for (i, pattern) in project.patterns.iter().enumerate() {
                        if !matches(search, &pattern.name) {
                            continue;
                        }
                        let text =
                            RichText::new(format!("▪ {}", pattern.name)).color(theme::palette(i));
                        if ui
                            .add(egui::Button::new(text).frame(false))
                            .hint("Click to select this pattern")
                            .clicked()
                        {
                            actions.push(Action::Run(Invocation::new(
                                "pattern.select",
                                [(i + 1).to_string()],
                            )));
                        }
                    }
                },
            );
            egui::CollapsingHeader::new(format!("Automation ({})", project.automations.len()))
                .show(ui, |ui| {
                    for automation in project
                        .automations
                        .iter()
                        .filter(|a| matches(search, &a.name))
                    {
                        ui.label(
                            RichText::new(format!("~ {}", automation.name))
                                .color(theme::AUTOMATION),
                        );
                    }
                });
            let mut used: Vec<&str> = project
                .channels
                .iter()
                .filter_map(|c| match &c.instrument {
                    Instrument::Sampler { sample } if !sample.starts_with(BUILTIN_PREFIX) => {
                        Some(sample.as_str())
                    }
                    _ => None,
                })
                .collect();
            used.sort_unstable();
            used.dedup();
            egui::CollapsingHeader::new(format!("Samples ({})", used.len())).show(ui, |ui| {
                for sample in used {
                    let name = Path::new(sample)
                        .file_name()
                        .map_or_else(|| sample.to_owned(), |n| n.to_string_lossy().into_owned());
                    if matches(search, &name) {
                        item(ui, &name, Dragged(Some(sample.to_owned())), actions);
                    }
                }
            });
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
                    if ui.small_button("⟳").hint("Refresh folders").clicked() {
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
                    current_project(ui, frame, &search, actions);
                    plugin_database(ui, state, frame, &search, actions);
                    egui::CollapsingHeader::new(heading("Packs"))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::CollapsingHeader::new("Drums (built-in)")
                                .default_open(true)
                                .show(ui, |ui| {
                                    for name in
                                        BUILTIN_SAMPLES.iter().filter(|n| matches(&search, n))
                                    {
                                        item(
                                            ui,
                                            &name.replace('_', " "),
                                            Dragged(Some(format!("{BUILTIN_PREFIX}{name}"))),
                                            actions,
                                        );
                                    }
                                });
                        });
                    egui::CollapsingHeader::new(heading("Folders"))
                        .default_open(true)
                        .show(ui, |ui| {
                            for dir in frame.settings.browser_folders.clone() {
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
                                    .hint("Add a channel from a file path")
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

//! Small modal prompts (file paths and names) and the shortcut reference.

use std::path::Path;

use egui::{Key, RichText};
use musiccraft_commands::Invocation;
use musiccraft_project::{FILE_EXTENSION, Project};

use crate::{Action, Frame, UiState, theme};

/// What a prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    Open,
    SaveAs,
    Export,
    ExportStems,
    ExportMidi,
    AddSample,
    SetSample(usize),
    SetPad(usize, usize),
    AddZone(usize),
    SetEffectFile(usize, usize),
    RenameChannel(usize),
    RenamePattern(usize),
    RenameTrack(usize),
    RenamePlaylistTrack(u32),
    ChannelGroup(usize),
    AddFolder,
    AddPluginFolder,
    RenameAutomation(usize),
    AddMarker(u32),
    SavePatch(usize),
    OpenPatch(usize),
    /// Antiserum channel and oscillator (0-based).
    AntiserumSample(usize, usize),
    AntiserumZone(usize, usize),
}

/// An open prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    purpose: Purpose,
    text: String,
    bits: &'static str,
    focused: bool,
}

fn project_dir(project: &Project) -> String {
    project
        .path()
        .and_then(Path::parent)
        .map(|p| format!("{}/", p.display()))
        .unwrap_or_default()
}

/// Where Antiserum patch files start: the user presets folder (a patch saved
/// there is a preset), or the project's folder without one.
fn preset_dir(project: &Project) -> String {
    musiccraft_antiserum_presets::user::folder()
        .map_or_else(|| project_dir(project), |dir| format!("{}/", dir.display()))
}

impl Dialog {
    fn new(purpose: Purpose, text: String) -> Self {
        Self {
            purpose,
            text,
            bits: "16",
            focused: false,
        }
    }

    pub fn open(project: &Project) -> Self {
        Self::new(Purpose::Open, project_dir(project))
    }

    pub fn save_as(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::SaveAs,
            format!("{}{name}.{FILE_EXTENSION}", project_dir(project)),
        )
    }

    pub fn export(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::Export,
            format!("{}{name}.wav", project_dir(project)),
        )
    }

    pub fn export_midi(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::ExportMidi,
            format!("{}{name}.mid", project_dir(project)),
        )
    }

    pub fn export_stems(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::ExportStems,
            format!("{}{name} stems", project_dir(project)),
        )
    }

    pub fn add_sample(project: &Project) -> Self {
        Self::new(Purpose::AddSample, project_dir(project))
    }

    pub fn set_sample(channel: usize, current: String) -> Self {
        Self::new(Purpose::SetSample(channel), current)
    }

    pub fn add_zone(channel: usize, project: &Project) -> Self {
        Self::new(Purpose::AddZone(channel), project_dir(project))
    }

    pub fn set_pad(channel: usize, pad: usize, current: String) -> Self {
        Self::new(Purpose::SetPad(channel, pad), current)
    }

    pub fn set_effect_file(track: usize, slot: usize, current: String) -> Self {
        Self::new(Purpose::SetEffectFile(track, slot), current)
    }

    pub fn rename_channel(channel: usize, current: &str) -> Self {
        Self::new(Purpose::RenameChannel(channel), current.to_owned())
    }

    pub fn rename_pattern(pattern: usize, current: &str) -> Self {
        Self::new(Purpose::RenamePattern(pattern), current.to_owned())
    }

    pub fn rename_automation(automation: usize, current: &str) -> Self {
        Self::new(Purpose::RenameAutomation(automation), current.to_owned())
    }

    pub fn add_marker(tick: u32) -> Self {
        Self::new(Purpose::AddMarker(tick), String::new())
    }

    pub fn add_folder() -> Self {
        let home = std::env::var("HOME")
            .map(|h| format!("{h}/"))
            .unwrap_or_default();
        Self::new(Purpose::AddFolder, home)
    }

    pub fn add_plugin_folder() -> Self {
        let home = std::env::var("HOME")
            .map(|h| format!("{h}/"))
            .unwrap_or_default();
        Self::new(Purpose::AddPluginFolder, home)
    }

    pub fn channel_group(channel: usize, current: &str) -> Self {
        Self::new(Purpose::ChannelGroup(channel), current.to_owned())
    }

    pub fn rename_playlist_track(track: u32, current: &str) -> Self {
        Self::new(Purpose::RenamePlaylistTrack(track), current.to_owned())
    }

    pub fn rename_track(track: usize, current: &str) -> Self {
        Self::new(Purpose::RenameTrack(track), current.to_owned())
    }

    /// Save an Antiserum channel's patch: a file named after it, next to the
    /// project.
    pub fn save_patch(channel: usize, name: &str, project: &Project) -> Self {
        let file = name.replace(['/', '\\'], "-");
        let path = format!("{}{file}.json", preset_dir(project));
        Self::new(Purpose::SavePatch(channel), path)
    }

    pub fn open_patch(channel: usize, project: &Project) -> Self {
        Self::new(Purpose::OpenPatch(channel), preset_dir(project))
    }

    /// An Antiserum oscillator's sample, starting from the current one.
    pub fn antiserum_sample(channel: usize, osc: usize, current: &str) -> Self {
        Self::new(Purpose::AntiserumSample(channel, osc), current.to_owned())
    }

    pub fn antiserum_zone(channel: usize, osc: usize, project: &Project) -> Self {
        Self::new(Purpose::AntiserumZone(channel, osc), project_dir(project))
    }

    fn title(&self) -> &'static str {
        match self.purpose {
            Purpose::Open => "Open project",
            Purpose::SaveAs => "Save project as",
            Purpose::Export => "Export audio",
            Purpose::ExportStems => "Export stems to folder",
            Purpose::ExportMidi => "Export MIDI file",
            Purpose::AddSample => "Add WAV sample",
            Purpose::SetSample(_) => "Change sample",
            Purpose::SetPad(_, _) => "Pad sample (file path or builtin:<sound>; - clears)",
            Purpose::AddZone(_) => "Zone sample (file path or builtin:<sound>)",
            Purpose::SetEffectFile(_, _) => "Impulse response (audio file or ir:<name>)",
            Purpose::RenameChannel(_) => "Rename channel",
            Purpose::RenamePattern(_) => "Rename pattern",
            Purpose::RenameTrack(_) => "Rename mixer track",
            Purpose::RenamePlaylistTrack(_) => "Rename playlist track",
            Purpose::ChannelGroup(_) => "Channel group (empty: none)",
            Purpose::AddFolder => "Add folder to the browser",
            Purpose::AddPluginFolder => "Add a plugin folder",
            Purpose::RenameAutomation(_) => "Rename automation",
            Purpose::AddMarker(_) => "Marker name",
            Purpose::SavePatch(_) => "Save Antiserum patch (.json)",
            Purpose::OpenPatch(_) => "Open Antiserum patch (.json)",
            Purpose::AntiserumSample(_, _) => {
                "Oscillator sample (file path or builtin:<sound>; - clears)"
            }
            Purpose::AntiserumZone(_, _) => "Zone sample (file path or builtin:<sound>)",
        }
    }

    /// The invocation to run when the prompt is confirmed.
    fn invocation(&self) -> Invocation {
        let text = self.text.trim().to_owned();
        match self.purpose {
            Purpose::Open => Invocation::new("project.load", [text]),
            Purpose::SaveAs => Invocation::new("project.save", [text]),
            Purpose::Export => match self.bits {
                "flac16" => Invocation::new("project.export_flac", [text, "16".to_owned()]),
                "flac24" => Invocation::new("project.export_flac", [text, "24".to_owned()]),
                "mp3" => Invocation::new("project.export_mp3", [text]),
                "ogg" => Invocation::new("project.export_ogg", [text]),
                bits => Invocation::new("project.export_wav", [text, bits.to_owned()]),
            },
            Purpose::ExportStems => Invocation::new("project.export_stems", [text]),
            Purpose::ExportMidi => Invocation::new("project.export_midi", [text]),
            Purpose::AddSample => Invocation::new("channel.add_sampler", [text]),
            Purpose::AddZone(channel) => {
                Invocation::new("channel.add_zone", [(channel + 1).to_string(), text])
            }
            Purpose::SetPad(channel, pad) => Invocation::new(
                "channel.set_pad",
                [(channel + 1).to_string(), (pad + 1).to_string(), text],
            ),
            Purpose::SetEffectFile(track, slot) => Invocation::new(
                "mixer.set_effect_file",
                [track.to_string(), (slot + 1).to_string(), text],
            ),
            Purpose::SetSample(channel) => {
                Invocation::new("channel.set_sample", [(channel + 1).to_string(), text])
            }
            Purpose::RenameChannel(channel) => {
                Invocation::new("channel.rename", [(channel + 1).to_string(), text])
            }
            Purpose::RenamePattern(pattern) => {
                Invocation::new("pattern.rename", [(pattern + 1).to_string(), text])
            }
            Purpose::RenameTrack(track) => {
                Invocation::new("mixer.rename", [track.to_string(), text])
            }
            Purpose::ChannelGroup(channel) => {
                let mut args = vec![(channel + 1).to_string()];
                if !text.is_empty() {
                    args.push(text);
                }
                Invocation::new("channel.set_group", args)
            }
            Purpose::RenamePlaylistTrack(track) => {
                Invocation::new("playlist.rename_track", [(track + 1).to_string(), text])
            }
            Purpose::AddFolder => Invocation::new("ui.add_folder", [text]),
            Purpose::AddPluginFolder => Invocation::new("ui.add_plugin_folder", [text]),
            Purpose::AddMarker(tick) => {
                Invocation::new("playlist.add_marker", [format!("{tick}t"), text])
            }
            Purpose::RenameAutomation(automation) => {
                Invocation::new("automation.rename", [(automation + 1).to_string(), text])
            }
            Purpose::SavePatch(channel) => {
                Invocation::new("antiserum.save_patch", [(channel + 1).to_string(), text])
            }
            Purpose::OpenPatch(channel) => {
                Invocation::new("antiserum.open_patch", [(channel + 1).to_string(), text])
            }
            Purpose::AntiserumSample(channel, osc) => Invocation::new(
                "antiserum.set_sample",
                [(channel + 1).to_string(), (osc + 1).to_string(), text],
            ),
            Purpose::AntiserumZone(channel, osc) => Invocation::new(
                "antiserum.add_zone",
                [(channel + 1).to_string(), (osc + 1).to_string(), text],
            ),
        }
    }
}

impl UiState {
    /// Shows a prompt, replacing any open one.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    _frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let Some(dialog) = &mut state.dialog else {
        return;
    };
    let mut close = false;
    let mut confirm = false;
    egui::Window::new(dialog.title())
        .frame(theme::dialog_frame())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            let field = ui.add(egui::TextEdit::singleline(&mut dialog.text).desired_width(400.0));
            if !dialog.focused {
                field.request_focus();
                dialog.focused = true;
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                confirm = true;
            }
            if dialog.purpose == Purpose::Export {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Format:");
                    for (bits, label) in [
                        ("16", "WAV 16"),
                        ("24", "WAV 24"),
                        ("32f", "WAV 32f"),
                        ("flac16", "FLAC 16"),
                        ("flac24", "FLAC 24"),
                        ("mp3", "MP3 320k"),
                        ("ogg", "Ogg q8"),
                    ] {
                        if ui.selectable_value(&mut dialog.bits, bits, label).changed() {
                            // Keep the file extension in step with the format.
                            let ext = match bits {
                                "mp3" | "ogg" => bits,
                                _ if bits.starts_with("flac") => "flac",
                                _ => "wav",
                            };
                            let path = std::path::Path::new(dialog.text.trim());
                            if path.extension().is_some() {
                                dialog.text = path.with_extension(ext).display().to_string();
                            }
                        }
                    }
                });
                ui.label(
                    RichText::new("Renders the pattern or song, following the play mode.")
                        .small()
                        .color(theme::TEXT_DIM),
                );
            }
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    close = true;
                }
            });
        });
    if confirm && !dialog.text.trim().is_empty() {
        actions.push(Action::Run(dialog.invocation()));
        close = true;
    }
    if close {
        state.dialog = None;
    }
}

pub fn shortcuts(ctx: &egui::Context, state: &mut UiState) {
    let mut open = state.windows.shortcuts;
    if !open {
        return;
    }
    let id = egui::Id::new("keyboard-shortcuts");
    let area = state.workspace;
    let default = egui::Rect::from_min_size(
        area.left_top() + egui::vec2(80.0, 40.0),
        egui::vec2(320.0, 0.0),
    );
    crate::chrome::window(ctx, id, "Keyboard shortcuts", area, default, true)
        .default_height((area.height() - 80.0).max(200.0))
        .show(ctx, |ui| {
            let folded = crate::chrome::Header::new(id, "Keyboard shortcuts").show(
                ui,
                &mut open,
                &mut (),
                |_, _| {},
                |_, _| {},
            );
            if folded {
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("shortcut-grid")
                    .striped(true)
                    .show(ui, |ui| {
                        for (chord, invocation) in state.keymap.iter() {
                            ui.label(
                                RichText::new(chord.to_string())
                                    .monospace()
                                    .color(theme::ACCENT),
                            );
                            ui.label(invocation.to_string());
                            ui.end_row();
                        }
                    });
            });
        });
    state.windows.shortcuts = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_build_quoted_invocations() {
        let mut dialog = Dialog::rename_channel(2, "Kick");
        dialog.text = "Big Kick".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "channel.rename 3 \"Big Kick\""
        );
        let mut export = Dialog::export(&Project::default());
        export.bits = "24";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_wav Untitled.wav 24"
        );
        export.text = "Untitled.mp3".into();
        export.bits = "mp3";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_mp3 Untitled.mp3"
        );
        export.text = "Untitled.ogg".into();
        export.bits = "ogg";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_ogg Untitled.ogg"
        );
        assert_eq!(
            Dialog::save_as(&Project::default())
                .invocation()
                .to_string(),
            "project.save Untitled.mcproj"
        );
    }
}

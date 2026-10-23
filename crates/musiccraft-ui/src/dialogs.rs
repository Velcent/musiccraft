//! Small modal prompts (names, and the export format), the requests for the
//! system's file dialogs (`docs/spec/style/dialogs.md`, "File dialogs are the
//! operating system's own"), and the shortcut reference.

use std::path::{Path, PathBuf};

use egui::{Key, RichText};
use musiccraft_commands::Invocation;
use musiccraft_project::{FILE_EXTENSION, Project};

use crate::{Action, Frame, UiState, theme};

/// What a prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// Export audio to the file in the prompt's text, in the chosen format.
    Export,
    RenameChannel(usize),
    RenamePattern(usize),
    RenameTrack(usize),
    RenamePlaylistTrack(u32),
    ChannelGroup(usize),
    /// Rename the channel group with this name.
    RenameGroup(String),
    RenameAutomation(usize),
    AddMarker(u32),
    /// A signature change at this tick (`none` removes it).
    TimeSignature(u32),
}

/// An open prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    purpose: Purpose,
    text: String,
    bits: &'static str,
    focused: bool,
}

/// What a system file dialog chooses a path for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    Open,
    SaveAs,
    /// Where to export audio; the format prompt follows.
    Export,
    ExportStems,
    ExportMidi,
    /// New sampler channels, one per file.
    AddSample,
    SetSample(usize),
    SetPad(usize, usize),
    AddZone(usize),
    SetEffectFile(usize, usize),
    AddFolder,
    AddPluginFolder,
    /// Settings → Folders: the FL Studio factory and user data folders.
    FlFactory,
    FlUser,
    /// Settings → Folders: the local folder of a drive mapping being added.
    DriveFolder,
    SavePatch(usize),
    OpenPatch(usize),
    /// Antiserum channel and oscillator (0-based).
    AntiserumSample(usize, usize),
    AntiserumZone(usize, usize),
}

/// What a file dialog lets the user choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickMode {
    /// One existing file.
    File,
    /// One or more existing files.
    Files,
    /// A file to write.
    Save,
    /// A folder.
    Folder,
}

/// A request for the system's file dialog, which the app shows. Its answer
/// comes back through [`UiState::picked`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePick {
    /// What the chosen path is for.
    pub pick: Pick,
    /// The dialog's title.
    pub title: &'static str,
    /// The folder it opens on.
    pub directory: Option<PathBuf>,
    /// The suggested file name, when saving.
    pub file_name: Option<String>,
    /// File-type filters: a name and its extensions.
    pub filters: Vec<(&'static str, &'static [&'static str])>,
}

const PROJECT_FILES: (&str, &[&str]) = ("MusicCraft project", &[FILE_EXTENSION]);
const SOUND_FILES: (&str, &[&str]) = ("WAV audio", &musiccraft_editors::sounds::EXTENSIONS);
const PATCH_FILES: (&str, &[&str]) = ("Antiserum patch", &["json"]);
const MIDI_FILES: (&str, &[&str]) = ("MIDI file", &["mid", "midi"]);
/// Export formats, the first the default.
const EXPORT_FILES: [(&str, &[&str]); 4] = [
    ("WAV audio", &["wav"]),
    ("FLAC audio", &["flac"]),
    ("MP3 audio", &["mp3"]),
    ("Ogg Vorbis audio", &["ogg"]),
];

fn project_dir(project: &Project) -> Option<PathBuf> {
    project
        .path()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(home_dir)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Where Antiserum patch files start: the user presets folder (a patch saved
/// there is a preset), or the project's folder without one.
fn preset_dir(project: &Project) -> Option<PathBuf> {
    musiccraft_antiserum_presets::user::folder().or_else(|| project_dir(project))
}

/// The project's name as a file name.
fn file_stem(project: &Project) -> String {
    project.name().replace(['/', '\\'], "-")
}

impl FilePick {
    fn new(pick: Pick, title: &'static str, directory: Option<PathBuf>) -> Self {
        let filters = match pick {
            Pick::Open | Pick::SaveAs => vec![PROJECT_FILES],
            Pick::Export => EXPORT_FILES.to_vec(),
            Pick::ExportMidi => vec![MIDI_FILES],
            Pick::SavePatch(_) | Pick::OpenPatch(_) => vec![PATCH_FILES],
            Pick::AddSample
            | Pick::SetSample(_)
            | Pick::SetPad(_, _)
            | Pick::AddZone(_)
            | Pick::SetEffectFile(_, _)
            | Pick::AntiserumSample(_, _)
            | Pick::AntiserumZone(_, _) => vec![SOUND_FILES],
            Pick::ExportStems
            | Pick::AddFolder
            | Pick::AddPluginFolder
            | Pick::FlFactory
            | Pick::FlUser
            | Pick::DriveFolder => Vec::new(),
        };
        Self {
            pick,
            title,
            directory,
            file_name: None,
            filters,
        }
    }

    fn named(mut self, file_name: String) -> Self {
        self.file_name = Some(file_name);
        self
    }

    /// What the dialog chooses.
    #[must_use]
    pub fn mode(&self) -> PickMode {
        match self.pick {
            Pick::SaveAs | Pick::Export | Pick::ExportMidi | Pick::SavePatch(_) => PickMode::Save,
            Pick::ExportStems
            | Pick::AddFolder
            | Pick::AddPluginFolder
            | Pick::FlFactory
            | Pick::FlUser
            | Pick::DriveFolder => PickMode::Folder,
            Pick::AddSample => PickMode::Files,
            _ => PickMode::File,
        }
    }

    pub fn open(project: &Project) -> Self {
        Self::new(Pick::Open, "Open project", project_dir(project))
    }

    pub fn save_as(project: &Project) -> Self {
        Self::new(Pick::SaveAs, "Save project as", project_dir(project))
            .named(format!("{}.{FILE_EXTENSION}", file_stem(project)))
    }

    pub fn export(project: &Project) -> Self {
        Self::new(Pick::Export, "Export audio", project_dir(project))
            .named(format!("{}.wav", file_stem(project)))
    }

    /// Export audio again, starting from the file chosen before.
    fn export_again(path: &Path) -> Self {
        let mut pick = Self::new(Pick::Export, "Export audio", None);
        pick.directory = path.parent().map(Path::to_path_buf);
        pick.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        pick
    }

    pub fn export_midi(project: &Project) -> Self {
        Self::new(Pick::ExportMidi, "Export MIDI file", project_dir(project))
            .named(format!("{}.mid", file_stem(project)))
    }

    pub fn export_stems(project: &Project) -> Self {
        Self::new(
            Pick::ExportStems,
            "Export stems to folder",
            project_dir(project),
        )
    }

    pub fn add_sample(project: &Project) -> Self {
        Self::new(
            Pick::AddSample,
            "Add channels from files",
            project_dir(project),
        )
    }

    /// A sample slot's file: the dialog opens on the folder of the slot's
    /// current file, or the project's.
    pub fn sample(pick: Pick, current: &str, project: &Project) -> Self {
        let title = match pick {
            Pick::SetSample(_) => "Change sample",
            Pick::SetPad(_, _) => "Pad sample",
            Pick::AddZone(_) | Pick::AntiserumZone(_, _) => "Zone sample",
            Pick::SetEffectFile(_, _) => "Impulse response",
            Pick::AntiserumSample(_, _) => "Oscillator sample",
            _ => "Choose a file",
        };
        let folder = Path::new(current)
            .parent()
            .filter(|p| p.is_absolute() && p.is_dir())
            .map(Path::to_path_buf);
        Self::new(pick, title, folder.or_else(|| project_dir(project)))
    }

    pub fn add_folder() -> Self {
        Self::new(Pick::AddFolder, "Add folder to the browser", home_dir())
    }

    pub fn add_plugin_folder() -> Self {
        Self::new(Pick::AddPluginFolder, "Add a plugin folder", home_dir())
    }

    /// One of the folders in Settings → Folders.
    pub fn folder(pick: Pick) -> Self {
        let title = match pick {
            Pick::FlFactory => "FL Studio factory data folder",
            Pick::FlUser => "FL Studio user data folder",
            _ => "Choose a folder",
        };
        Self::new(pick, title, home_dir())
    }

    /// Save an Antiserum channel's patch: a file named after it, in the user
    /// presets folder.
    pub fn save_patch(channel: usize, name: &str, project: &Project) -> Self {
        let file = name.replace(['/', '\\'], "-");
        Self::new(
            Pick::SavePatch(channel),
            "Save Antiserum patch",
            preset_dir(project),
        )
        .named(format!("{file}.json"))
    }

    pub fn open_patch(channel: usize, project: &Project) -> Self {
        Self::new(
            Pick::OpenPatch(channel),
            "Open Antiserum patch",
            preset_dir(project),
        )
    }

    /// The invocation for one chosen path (`None` for the export, whose
    /// format prompt comes first).
    fn invocation(&self, path: &Path) -> Option<Invocation> {
        let text = path.display().to_string();
        let ch = |c: usize| (c + 1).to_string();
        Some(match self.pick {
            Pick::Export | Pick::DriveFolder => return None,
            Pick::Open => Invocation::new("project.load", [text]),
            Pick::SaveAs => Invocation::new("project.save", [text]),
            Pick::ExportStems => Invocation::new("project.export_stems", [text]),
            Pick::ExportMidi => Invocation::new("project.export_midi", [text]),
            Pick::AddSample => Invocation::new("channel.add_sampler", [text]),
            Pick::SetSample(c) => Invocation::new("channel.set_sample", [ch(c), text]),
            Pick::AddZone(c) => Invocation::new("channel.add_zone", [ch(c), text]),
            Pick::SetPad(c, pad) => Invocation::new("channel.set_pad", [ch(c), ch(pad), text]),
            Pick::SetEffectFile(track, slot) => {
                Invocation::new("mixer.set_effect_file", [track.to_string(), ch(slot), text])
            }
            Pick::AddFolder => Invocation::new("ui.add_folder", [text]),
            Pick::AddPluginFolder => Invocation::new("ui.add_plugin_folder", [text]),
            Pick::FlFactory => Invocation::new("ui.set_fl_folder", ["factory".to_owned(), text]),
            Pick::FlUser => Invocation::new("ui.set_fl_folder", ["user".to_owned(), text]),
            Pick::SavePatch(c) => Invocation::new("antiserum.save_patch", [ch(c), text]),
            Pick::OpenPatch(c) => Invocation::new("antiserum.open_patch", [ch(c), text]),
            Pick::AntiserumSample(c, osc) => {
                Invocation::new("antiserum.set_sample", [ch(c), ch(osc), text])
            }
            Pick::AntiserumZone(c, osc) => {
                Invocation::new("antiserum.add_zone", [ch(c), ch(osc), text])
            }
        })
    }
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

    /// The export format prompt for the file chosen in the export dialog,
    /// its format taken from the file's extension.
    pub fn export_to(path: &Path) -> Self {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let mut dialog = Self::new(Purpose::Export, path.display().to_string());
        dialog.bits = match ext.as_str() {
            "flac" => "flac16",
            "mp3" => "mp3",
            "ogg" => "ogg",
            _ => "16",
        };
        dialog
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

    /// Puts a time signature change at `tick`, starting from `current`.
    pub fn time_signature(tick: u32, current: &str) -> Self {
        Self::new(Purpose::TimeSignature(tick), current.to_owned())
    }

    /// Renames the marker at `tick` (re-adding a marker keeps its action).
    pub fn rename_marker(tick: u32, current: &str) -> Self {
        Self::new(Purpose::AddMarker(tick), current.to_owned())
    }

    pub fn channel_group(channel: usize, current: &str) -> Self {
        Self::new(Purpose::ChannelGroup(channel), current.to_owned())
    }

    pub fn rename_group(group: &str) -> Self {
        Self::new(Purpose::RenameGroup(group.to_owned()), group.to_owned())
    }

    pub fn rename_playlist_track(track: u32, current: &str) -> Self {
        Self::new(Purpose::RenamePlaylistTrack(track), current.to_owned())
    }

    pub fn rename_track(track: usize, current: &str) -> Self {
        Self::new(Purpose::RenameTrack(track), current.to_owned())
    }

    fn title(&self) -> &'static str {
        match self.purpose {
            Purpose::Export => "Export audio",
            Purpose::RenameChannel(_) => "Rename channel",
            Purpose::RenamePattern(_) => "Rename pattern",
            Purpose::RenameTrack(_) => "Rename mixer track",
            Purpose::RenamePlaylistTrack(_) => "Rename playlist track",
            Purpose::ChannelGroup(_) => "Channel group (empty: none)",
            Purpose::RenameGroup(_) => "Group name (empty: remove the group)",
            Purpose::RenameAutomation(_) => "Rename automation",
            Purpose::AddMarker(_) => "Marker name",
            Purpose::TimeSignature(_) => "Time signature from here (n/d; none removes the change)",
        }
    }

    /// The export file with the chosen format's extension (added when the
    /// file dialog left it off, changed when the format was changed).
    fn export_path(&self) -> String {
        let ext = match self.bits {
            "mp3" | "ogg" => self.bits,
            bits if bits.starts_with("flac") => "flac",
            _ => "wav",
        };
        Path::new(self.text.trim())
            .with_extension(ext)
            .display()
            .to_string()
    }

    /// The invocation to run when the prompt is confirmed.
    fn invocation(&self) -> Invocation {
        let text = self.text.trim().to_owned();
        match self.purpose {
            Purpose::Export => {
                let path = self.export_path();
                match self.bits {
                    "flac16" => Invocation::new("project.export_flac", [path, "16".to_owned()]),
                    "flac24" => Invocation::new("project.export_flac", [path, "24".to_owned()]),
                    "mp3" => Invocation::new("project.export_mp3", [path]),
                    "ogg" => Invocation::new("project.export_ogg", [path]),
                    bits => Invocation::new("project.export_wav", [path, bits.to_owned()]),
                }
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
            Purpose::RenameGroup(ref group) => {
                let mut args = vec![group.clone()];
                if !text.is_empty() {
                    args.push(text);
                }
                Invocation::new("channel.rename_group", args)
            }
            Purpose::RenamePlaylistTrack(track) => {
                Invocation::new("playlist.rename_track", [(track + 1).to_string(), text])
            }
            Purpose::AddMarker(tick) => {
                Invocation::new("playlist.add_marker", [format!("{tick}t"), text])
            }
            Purpose::TimeSignature(tick) => {
                Invocation::new("playlist.set_time_signature", [format!("{tick}t"), text])
            }
            Purpose::RenameAutomation(automation) => {
                Invocation::new("automation.rename", [(automation + 1).to_string(), text])
            }
        }
    }
}

impl UiState {
    /// Asks the app to show the system's file dialog for `pick`, replacing
    /// any request it has not taken yet.
    pub(crate) fn pick_file(&mut self, pick: FilePick) {
        self.file_pick = Some(pick);
    }

    /// The file dialog the UI asked for since the last call, for the app to
    /// show.
    pub fn take_file_pick(&mut self) -> Option<FilePick> {
        self.file_pick.take()
    }

    /// The answer to a file dialog: the actions to run for the chosen paths
    /// (none when it was cancelled). An export opens its format prompt.
    pub fn picked(&mut self, pick: &FilePick, paths: &[PathBuf]) -> Vec<Action> {
        match (&pick.pick, paths.first()) {
            (Pick::Export, Some(path)) => self.dialog = Some(Dialog::export_to(path)),
            (Pick::DriveFolder, Some(path)) => self.drive_draft.1 = Some(path.clone()),
            _ => {}
        }
        let paths = if pick.mode() == PickMode::Files {
            paths
        } else {
            paths.get(..1).unwrap_or_default()
        };
        paths
            .iter()
            .filter_map(|path| pick.invocation(path))
            .map(Action::Run)
            .collect()
    }

    /// Shows a prompt, replacing any open one.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }

    /// Turns the name drawn in `rect` into a text field there, for a rename
    /// that `dialog` describes (`docs/spec/controls.md`, "Rename in place").
    pub(crate) fn rename_in_place(&mut self, dialog: Dialog, rect: egui::Rect) {
        self.renaming = Some(InPlace {
            dialog,
            rect,
            opened: false,
        });
    }
}

/// A name being edited where it is drawn.
#[derive(Debug, Clone)]
pub(crate) struct InPlace {
    dialog: Dialog,
    rect: egui::Rect,
    /// Whether the field has been shown once (its text is then selected).
    opened: bool,
}

/// Draws the in-place rename field. Enter or a click elsewhere renames as
/// one undo step; Esc, or an empty name, leaves the name as it was.
pub(crate) fn show_in_place(ctx: &egui::Context, state: &mut UiState, actions: &mut Vec<Action>) {
    let Some(mut open) = state.renaming.take() else {
        return;
    };
    let id = egui::Id::new("rename-in-place");
    let field_id = id.with("field");
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(open.rect.min)
        .show(ctx, |ui| {
            let field = ui.add_sized(
                open.rect.size(),
                egui::TextEdit::singleline(&mut open.dialog.text)
                    .id(field_id)
                    .font(egui::FontId::proportional(theme::FONT_LABEL))
                    .margin(egui::vec2(4.0, 2.0)),
            );
            if !open.opened {
                field.request_focus();
                let all = egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(open.dialog.text.chars().count()),
                );
                let mut text_state =
                    egui::TextEdit::load_state(ui.ctx(), field_id).unwrap_or_default();
                text_state.cursor.set_char_range(Some(all));
                text_state.store(ui.ctx(), field_id);
                open.opened = true;
            }
            field
        });
    let (enter, escape) = ctx.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, Key::Enter),
            i.consume_key(egui::Modifiers::NONE, Key::Escape),
        )
    });
    let field = area.inner;
    let clicked_outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p))
    });
    if escape {
        return;
    }
    if enter || clicked_outside || field.lost_focus() {
        if !open.dialog.text.trim().is_empty() {
            actions.push(Action::Run(open.dialog.invocation()));
        }
        return;
    }
    state.renaming = Some(open);
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
    let mut change = None;
    egui::Window::new(dialog.title())
        .frame(theme::dialog_frame())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            if dialog.purpose == Purpose::Export {
                // The file comes from the system's save dialog; "Change…"
                // opens it again.
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(RichText::new(dialog.export_path()).monospace().small())
                            .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Change…").clicked() {
                            change = Some(FilePick::export_again(Path::new(dialog.text.trim())));
                        }
                    });
                });
                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    confirm = true;
                }
            } else {
                let field =
                    ui.add(egui::TextEdit::singleline(&mut dialog.text).desired_width(400.0));
                if !dialog.focused {
                    field.request_focus();
                    dialog.focused = true;
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    confirm = true;
                }
            }
            if dialog.purpose == Purpose::Export {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Format:");
                    // The file's extension follows the format.
                    for (bits, label) in [
                        ("16", "WAV 16"),
                        ("24", "WAV 24"),
                        ("32f", "WAV 32f"),
                        ("flac16", "FLAC 16"),
                        ("flac24", "FLAC 24"),
                        ("mp3", "MP3 320k"),
                        ("ogg", "Ogg q8"),
                    ] {
                        ui.selectable_value(&mut dialog.bits, bits, label);
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
    if let Some(pick) = change {
        state.dialog = None;
        state.pick_file(pick);
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
    fn the_time_signature_prompt_sets_a_change_at_its_bar() {
        let mut dialog = Dialog::time_signature(7680, "4/4");
        assert_eq!(dialog.text, "4/4");
        dialog.text = "7/8".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "playlist.set_time_signature 7680t 7/8"
        );
        let mut session = musiccraft_commands::Session::new(
            Project::default(),
            musiccraft_commands::Registry::with_builtin(),
        );
        session.run(&dialog.invocation()).unwrap();
        assert_eq!(session.project().playlist.markers.len(), 1);
    }

    #[test]
    fn prompts_build_quoted_invocations() {
        let mut dialog = Dialog::rename_channel(2, "Kick");
        dialog.text = "Big Kick".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "channel.rename 3 \"Big Kick\""
        );
    }

    #[test]
    fn the_export_prompt_takes_its_format_from_the_chosen_file() {
        let mut export = Dialog::export_to(Path::new("/songs/Untitled.wav"));
        assert_eq!(export.bits, "16");
        export.bits = "24";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_wav /songs/Untitled.wav 24"
        );
        // A format chip changes the extension.
        export.bits = "mp3";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_mp3 /songs/Untitled.mp3"
        );
        let ogg = Dialog::export_to(Path::new("/songs/Untitled.ogg"));
        assert_eq!(
            ogg.invocation().to_string(),
            "project.export_ogg /songs/Untitled.ogg"
        );
        // A portal that leaves the extension off gets the format's.
        let bare = Dialog::export_to(Path::new("/songs/Mix"));
        assert_eq!(
            bare.invocation().to_string(),
            "project.export_wav /songs/Mix.wav 16"
        );
        let flac = Dialog::export_to(Path::new("/songs/Mix.FLAC"));
        assert_eq!(
            flac.invocation().to_string(),
            "project.export_flac /songs/Mix.flac 16"
        );
    }

    #[test]
    fn file_picks_suggest_a_name_and_filter_by_type() {
        let project = Project::default();
        let save = FilePick::save_as(&project);
        assert_eq!(save.mode(), PickMode::Save);
        assert_eq!(save.file_name.as_deref(), Some("Untitled.mcproj"));
        assert_eq!(save.filters, [("MusicCraft project", &["mcproj"][..])]);
        let export = FilePick::export(&project);
        assert_eq!(export.file_name.as_deref(), Some("Untitled.wav"));
        assert_eq!(export.filters.len(), 4);
        assert_eq!(FilePick::export_stems(&project).mode(), PickMode::Folder);
        assert_eq!(FilePick::add_folder().mode(), PickMode::Folder);
        assert_eq!(FilePick::add_sample(&project).mode(), PickMode::Files);
        let sample = FilePick::sample(Pick::SetSample(0), "builtin:kick", &project);
        assert_eq!(sample.mode(), PickMode::File);
        assert_eq!(sample.filters, [("WAV audio", &["wav"][..])]);
        let patch = FilePick::save_patch(1, "Big/Lead", &project);
        assert_eq!(patch.file_name.as_deref(), Some("Big-Lead.json"));
    }

    #[test]
    fn answers_to_file_picks_become_commands() {
        let project = Project::default();
        let mut state = UiState::default();
        let run = |actions: Vec<Action>| -> Vec<String> {
            actions
                .into_iter()
                .map(|a| match a {
                    Action::Run(i) => i.to_string(),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        let song = [PathBuf::from("/songs/My Song.mcproj")];
        assert_eq!(
            run(state.picked(&FilePick::open(&project), &song)),
            ["project.load \"/songs/My Song.mcproj\""]
        );
        assert_eq!(
            run(state.picked(&FilePick::save_as(&project), &song)),
            ["project.save \"/songs/My Song.mcproj\""]
        );
        // Cancelled: nothing.
        assert!(state.picked(&FilePick::open(&project), &[]).is_empty());
        // Several files: one channel each.
        let files = [PathBuf::from("/s/a.wav"), PathBuf::from("/s/b.wav")];
        assert_eq!(
            run(state.picked(&FilePick::add_sample(&project), &files)),
            [
                "channel.add_sampler /s/a.wav",
                "channel.add_sampler /s/b.wav"
            ]
        );
        let pad = FilePick::sample(Pick::SetPad(2, 4), "", &project);
        assert_eq!(
            run(state.picked(&pad, &files)),
            ["channel.set_pad 3 5 /s/a.wav"]
        );
        let osc = FilePick::sample(Pick::AntiserumSample(0, 1), "", &project);
        assert_eq!(
            run(state.picked(&osc, &files)),
            ["antiserum.set_sample 1 2 /s/a.wav"]
        );
        // The export asks for the format next.
        let mix = [PathBuf::from("/songs/Mix.flac")];
        assert!(state.picked(&FilePick::export(&project), &mix).is_empty());
        let dialog = state.dialog.as_ref().expect("format prompt");
        assert_eq!(dialog.purpose, Purpose::Export);
        assert_eq!(dialog.bits, "flac16");
    }

    #[test]
    fn the_group_prompt_renames_or_removes_the_group() {
        let mut dialog = Dialog::rename_group("Drums");
        dialog.text = "Kit one".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "channel.rename_group Drums \"Kit one\""
        );
        dialog.text = "  ".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "channel.rename_group Drums"
        );
    }
}

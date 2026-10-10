//! User interface. Contains no business logic: it renders project state and turns
//! input into [`Action`]s (command invocations, previews and UI-only actions)
//! for the app to run. Keyboard shortcuts come from a data-driven [`Keymap`].

pub mod keymap;

mod browser;
mod channel_rack;
mod dialogs;
mod geometry;
mod mixer;
mod piano_roll;
mod playlist;
mod theme;
mod toolbar;
mod widgets;

use musiccraft_commands::Invocation;
use musiccraft_project::Project;

pub use geometry::Snap;
pub use keymap::{Chord, DEFAULT_KEYMAP, Keymap};
pub use theme::apply as apply_theme;

/// Something the user asked for, produced by one frame of UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Run a command.
    Run(Invocation),
    /// Run a command as part of a continuous gesture; runs with the same key
    /// merge into one undo step.
    Coalesced(Invocation, u64),
    /// Start or stop an audition note on a channel (0-based index).
    Preview {
        /// Channel index.
        channel: usize,
        /// MIDI key.
        key: u8,
        /// `true` for note on, `false` for note off.
        on: bool,
    },
    /// A live performance note on the selected channel (typing keyboard). The app
    /// records these into the pattern while recording.
    Perform {
        /// Channel index.
        channel: usize,
        /// MIDI key.
        key: u8,
        /// `true` for note on, `false` for note off.
        on: bool,
    },
    /// Play a short audition note that the app stops by itself.
    PreviewShort {
        /// Channel index.
        channel: usize,
        /// MIDI key.
        key: u8,
    },
    /// Drop decoded samples and reload them from disk.
    ReloadSamples,
    /// Close the application.
    Quit,
}

/// Engine and device information shown by the UI. Read-only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineStatus {
    /// Playhead position in ticks within the current loop.
    pub position_ticks: f64,
    /// Whether the engine is playing.
    pub playing: bool,
    /// Peak levels (left, right) per mixer track since the last frame.
    pub peaks: Vec<(f32, f32)>,
    /// Sounding voices.
    pub voices: u32,
    /// Audio device description or error.
    pub audio: String,
    /// MIDI input description.
    pub midi: String,
}

/// Undo history information for menus.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    /// The command `edit.undo` would reverse.
    pub undo: Option<String>,
    /// Whether `edit.redo` is possible.
    pub can_redo: bool,
}

/// Which floating windows are open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Windows {
    /// Channel rack.
    pub channel_rack: bool,
    /// Piano roll.
    pub piano_roll: bool,
    /// Playlist.
    pub playlist: bool,
    /// Mixer.
    pub mixer: bool,
    /// Browser side panel.
    pub browser: bool,
    /// Keyboard shortcut reference.
    pub shortcuts: bool,
}

impl Default for Windows {
    fn default() -> Self {
        Self {
            channel_rack: true,
            piano_roll: false,
            playlist: true,
            mixer: false,
            browser: true,
            shortcuts: false,
        }
    }
}

/// A status line message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusLine {
    /// Text to show.
    pub text: String,
    /// Whether it describes a failure.
    pub error: bool,
}

/// State that belongs to the interface, not to the project.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Open windows.
    pub windows: Windows,
    /// Selected channel (0-based), shown in the piano roll.
    pub selected_channel: usize,
    /// Selected mixer track (0 = master).
    pub selected_track: usize,
    /// Keyboard shortcuts.
    pub keymap: Keymap,
    /// Last command result.
    pub status: StatusLine,
    /// Smoothed meter levels per mixer track.
    pub meters: Vec<(f32, f32)>,
    /// Location of the last autosave, offered by File → Recover autosave.
    pub autosave_path: Option<String>,
    /// Letter keys play notes on the selected channel.
    pub typing_keyboard: bool,
    /// Live notes are written into the selected pattern while playing.
    pub record: bool,
    gesture: u64,
    roll: piano_roll::RollState,
    playlist: playlist::PlaylistState,
    dialog: Option<dialogs::Dialog>,
}

impl Default for UiState {
    fn default() -> Self {
        Self::new(Keymap::default_keymap())
    }
}

impl UiState {
    /// Fresh interface state using `keymap`.
    #[must_use]
    pub fn new(keymap: Keymap) -> Self {
        Self {
            windows: Windows::default(),
            selected_channel: 0,
            selected_track: 0,
            keymap,
            status: StatusLine::default(),
            meters: Vec::new(),
            autosave_path: None,
            typing_keyboard: false,
            record: false,
            gesture: 0,
            roll: piano_roll::RollState::default(),
            playlist: playlist::PlaylistState::default(),
            dialog: None,
        }
    }

    /// Starts a new continuous gesture and returns its key.
    pub(crate) fn begin_gesture(&mut self) -> u64 {
        self.gesture += 1;
        self.gesture
    }

    /// The key of the gesture in progress.
    pub(crate) fn gesture(&self) -> u64 {
        self.gesture
    }

    /// Records a command result for the status line.
    pub fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = StatusLine {
            text: text.into(),
            error,
        };
    }

    fn update_meters(&mut self, peaks: &[(f32, f32)], dt: f32) {
        self.meters.resize(peaks.len(), (0.0, 0.0));
        // Peaks jump up and fall at about 24 dB per second.
        let fall = 10.0_f32.powf(-1.2 * dt);
        for (meter, peak) in self.meters.iter_mut().zip(peaks) {
            meter.0 = peak.0.max(meter.0 * fall);
            meter.1 = peak.1.max(meter.1 * fall);
        }
    }
}

/// Inputs to one frame of UI.
pub struct Frame<'a> {
    /// The project to show.
    pub project: &'a Project,
    /// Engine status.
    pub engine: &'a EngineStatus,
    /// Undo information.
    pub history: &'a History,
}

/// Draws the whole interface and returns what the user asked for.
pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>) -> Vec<Action> {
    let mut actions = Vec::new();
    let dt = ui.ctx().input(|i| i.stable_dt).min(0.1);
    state.update_meters(&frame.engine.peaks, dt);
    let channels = frame.project.channels.len();
    if state.selected_channel >= channels {
        state.selected_channel = channels.saturating_sub(1);
    }
    if state.selected_track >= frame.project.mixer.len() {
        state.selected_track = 0;
    }

    toolbar::show(ui, state, frame, &mut actions);
    toolbar::status_bar(ui, state, frame);
    if state.windows.browser {
        browser::show(ui, state, frame, &mut actions);
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme::BACKGROUND))
        .show(ui, |ui| {
            theme::watermark(ui);
        });

    let ctx = ui.ctx().clone();
    channel_rack::show(&ctx, state, frame, &mut actions);
    piano_roll::show(&ctx, state, frame, &mut actions);
    playlist::show(&ctx, state, frame, &mut actions);
    mixer::show(&ctx, state, frame, &mut actions);
    dialogs::show(&ctx, state, frame, &mut actions);
    dialogs::shortcuts(&ctx, state);

    handle_keys(&ctx, state, frame, &mut actions);
    resolve_ui_actions(state, frame, actions)
}

/// Typing-keyboard layout: (key, semitones above C4). The bottom letter row is
/// one octave, the top row the next, with the number row as black keys.
const TYPING_KEYS: [(egui::Key, u8); 29] = {
    use egui::Key::*;
    [
        (Z, 0),
        (S, 1),
        (X, 2),
        (D, 3),
        (C, 4),
        (V, 5),
        (G, 6),
        (B, 7),
        (H, 8),
        (N, 9),
        (J, 10),
        (M, 11),
        (Comma, 12),
        (Q, 12),
        (Num2, 13),
        (W, 14),
        (Num3, 15),
        (E, 16),
        (R, 17),
        (Num5, 18),
        (T, 19),
        (Num6, 20),
        (Y, 21),
        (Num7, 22),
        (U, 23),
        (I, 24),
        (Num9, 25),
        (O, 26),
        (P, 28),
    ]
};
/// Key that the first typing-keyboard note (Z) plays: C4.
const TYPING_BASE: u8 = 48;

fn typing_note(key: egui::Key, modifiers: egui::Modifiers) -> Option<u8> {
    if modifiers.any() && !modifiers.shift_only() {
        return None;
    }
    TYPING_KEYS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, offset)| TYPING_BASE + offset)
}

fn handle_keys(ctx: &egui::Context, state: &UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let events: Vec<(egui::Key, bool, bool, egui::Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } => Some((*key, *pressed, *repeat, *modifiers)),
                _ => None,
            })
            .collect()
    });
    let channel_exists = state.selected_channel < frame.project.channels.len();
    for (key, pressed, repeat, modifiers) in events {
        if state.typing_keyboard && channel_exists {
            if let Some(note) = typing_note(key, modifiers) {
                if !repeat {
                    actions.push(Action::Perform {
                        channel: state.selected_channel,
                        key: note,
                        on: pressed,
                    });
                }
                continue;
            }
        }
        if !pressed || repeat {
            continue;
        }
        if let Some(invocation) = state.keymap.lookup(&Chord::from_event(key, modifiers)) {
            actions.push(Action::Run(invocation.clone()));
        }
    }
}

/// Handles `view.*` and `ui.*` invocations here; passes the rest through.
fn resolve_ui_actions(state: &mut UiState, frame: &Frame<'_>, actions: Vec<Action>) -> Vec<Action> {
    let mut out = Vec::with_capacity(actions.len());
    for action in actions {
        let Action::Run(invocation) = &action else {
            out.push(action);
            continue;
        };
        match invocation.name.as_str() {
            "view.toggle" => {
                let windows = &mut state.windows;
                let flag = match invocation.args.first().map(String::as_str) {
                    Some("playlist") => Some(&mut windows.playlist),
                    Some("channel_rack") => Some(&mut windows.channel_rack),
                    Some("piano_roll") => Some(&mut windows.piano_roll),
                    Some("mixer") => Some(&mut windows.mixer),
                    Some("browser") => Some(&mut windows.browser),
                    Some("shortcuts") => Some(&mut windows.shortcuts),
                    _ => None,
                };
                if let Some(flag) = flag {
                    *flag = !*flag;
                }
            }
            "ui.new" => out.push(Action::Run(Invocation::bare("project.new"))),
            "ui.open" => state.dialog = Some(dialogs::Dialog::open(frame.project)),
            "ui.save" => match frame.project.path() {
                Some(_) => out.push(Action::Run(Invocation::bare("project.save"))),
                None => state.dialog = Some(dialogs::Dialog::save_as(frame.project)),
            },
            "ui.save_as" => state.dialog = Some(dialogs::Dialog::save_as(frame.project)),
            "ui.export" => state.dialog = Some(dialogs::Dialog::export(frame.project)),
            "ui.quit" => out.push(Action::Quit),
            "ui.typing_keyboard" => state.typing_keyboard = !state.typing_keyboard,
            "ui.toggle_record" => state.record = !state.record,
            _ => out.push(action),
        }
    }
    out
}

#[cfg(test)]
mod interaction_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_for(project: &Project) -> (EngineStatus, History) {
        let _ = project;
        (EngineStatus::default(), History::default())
    }

    #[test]
    fn view_actions_toggle_windows_and_are_consumed() {
        let project = Project::default();
        let (engine, history) = frame_for(&project);
        let frame = Frame {
            project: &project,
            engine: &engine,
            history: &history,
        };
        let mut state = UiState::default();
        assert!(!state.windows.mixer);
        let out = resolve_ui_actions(
            &mut state,
            &frame,
            vec![
                Action::Run(Invocation::parse("view.toggle mixer").unwrap()),
                Action::Run(Invocation::parse("transport.toggle").unwrap()),
                Action::Run(Invocation::parse("ui.save").unwrap()),
            ],
        );
        assert!(state.windows.mixer);
        assert_eq!(out, [Action::Run(Invocation::bare("transport.toggle"))]);
        assert!(state.dialog.is_some(), "unsaved project asks for a path");
    }

    #[test]
    fn meters_jump_up_and_decay() {
        let mut state = UiState::default();
        state.update_meters(&[(1.0, 0.5)], 0.0);
        assert_eq!(state.meters, [(1.0, 0.5)]);
        state.update_meters(&[(0.0, 0.0)], 1.0);
        assert!(state.meters[0].0 < 0.07 && state.meters[0].0 > 0.05);
    }
}

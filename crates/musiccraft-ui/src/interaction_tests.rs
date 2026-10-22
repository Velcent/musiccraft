//! Drives the real views headlessly with synthetic input and checks the commands
//! they produce.

use std::collections::BTreeSet;

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use musiccraft_commands::{Registry, Session};
use musiccraft_project::Project;
use musiccraft_project::params::TICKS_PER_STEP;

use crate::piano_roll::row_of_key;
use crate::{
    Action, EngineStatus, Frame, History, LivePluginView, PluginEntry, PluginParamView, Plugins,
    Settings, SettingsInfo, UiState, show,
};

/// A 4/4 bar.
const TICKS_PER_BAR: u32 = 384;

struct Harness {
    ctx: egui::Context,
    state: UiState,
    session: Session,
    time: f64,
    /// Modifier keys held during the next frames.
    modifiers: Modifiers,
    /// Plugin information handed to the views.
    plugins: Plugins,
    /// Actions other than commands from the last frames.
    other: Vec<Action>,
    /// The pointer shape the last frame asked for.
    cursor: egui::CursorIcon,
}

impl Harness {
    fn new(setup: &[&str]) -> Self {
        let mut session = Session::new(Project::default(), Registry::with_builtin());
        for line in setup {
            session.run_line(line).unwrap();
        }
        let mut state = UiState::default();
        state.windows.channel_rack = false;
        state.windows.playlist = false;
        state.windows.browser = false;
        state.windows.mixer = false;
        state.windows.piano_roll = false;
        Self {
            ctx: egui::Context::default(),
            state,
            session,
            time: 0.0,
            modifiers: Modifiers::NONE,
            plugins: Plugins::default(),
            other: Vec::new(),
            cursor: egui::CursorIcon::Default,
        }
    }

    /// Runs one frame with `events` and returns the project commands produced
    /// (previews are dropped), applying them to the session.
    fn frame(&mut self, mut events: Vec<Event>) -> Vec<String> {
        events.insert(0, Event::ModifiersChanged(self.modifiers));
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let engine = EngineStatus::default();
        let history = History::default();
        let mut actions = Vec::new();
        {
            let Self {
                ctx,
                state,
                session,
                plugins,
                ..
            } = self;
            let frame = Frame {
                project: session.project(),
                engine: &engine,
                history: &history,
                waveform: None,
                waveform_seconds: None,
                clip_waveforms: &[],
                clip_durations: &[],
                plugins,
                settings: &Settings::default(),
                settings_info: &SettingsInfo::default(),
            };
            let mut output = ctx.run_ui(input, |ui| actions = show(ui, state, &frame));
            // No renderer here: acknowledge texture uploads.
            output.textures_delta.clear();
            self.cursor = output.platform_output.cursor_icon;
        }
        let mut commands = Vec::new();
        for action in actions {
            match action {
                Action::Run(invocation) => {
                    commands.push(invocation.to_string());
                    self.session.run(&invocation).unwrap();
                }
                Action::Coalesced(invocation, key) => {
                    commands.push(invocation.to_string());
                    self.session.run_coalesced(&invocation, key).unwrap();
                }
                other => self.other.push(other),
            }
        }
        commands
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(Vec::new());
        }
    }

    fn press(&mut self, pos: Pos2, button: PointerButton) -> Vec<String> {
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: self.modifiers,
            },
        ])
    }

    fn release(&mut self, pos: Pos2, button: PointerButton) -> Vec<String> {
        let modifiers = self.modifiers;
        self.frame(vec![Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers,
        }])
    }

    fn click(&mut self, pos: Pos2, button: PointerButton) -> Vec<String> {
        let mut out = self.press(pos, button);
        out.extend(self.release(pos, button));
        out
    }

    fn drag(&mut self, from: Pos2, to: Pos2) -> Vec<String> {
        self.drag_with(from, to, PointerButton::Primary)
    }

    fn drag_with(&mut self, from: Pos2, to: Pos2, button: PointerButton) -> Vec<String> {
        let mut out = self.press(from, button);
        for i in 1..=6 {
            let t = i as f32 / 6.0;
            out.extend(self.frame(vec![Event::PointerMoved(from + (to - from) * t)]));
        }
        out.extend(self.release(to, button));
        out
    }

    fn roll_point(&self, tick: u32, key: u8) -> Pos2 {
        let (grid, clip) = self.state.roll.last_view.expect("piano roll drawn");
        let p = pos2(
            grid.x(tick as f32),
            grid.y(row_of_key(key)) + grid.row_height / 2.0,
        );
        assert!(clip.contains(p), "{p:?} not visible in {clip:?}");
        p
    }

    fn playlist_point(&self, tick: u32, track: u32) -> Pos2 {
        let (grid, clip) = self.state.playlist.last_view.expect("playlist drawn");
        let p = pos2(grid.x(tick as f32), grid.y(track) + grid.row_height / 2.0);
        assert!(clip.contains(p), "{p:?} not visible in {clip:?}");
        p
    }
}

#[test]
fn piano_roll_click_drag_resize_delete() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.piano_roll = true;
    h.settle();

    let step = TICKS_PER_STEP;
    let added = h.click(h.roll_point(2 * step + 5, 62), PointerButton::Primary);
    assert_eq!(added, ["note.add 1 48t 24t 62 100"]);
    h.settle();

    let moved = h.drag(
        h.roll_point(2 * step + 6, 62),
        h.roll_point(5 * step + 6, 64),
    );
    assert_eq!(moved, ["note.move 1 48t 62 120t 64"]);
    h.settle();

    // Grab the right edge of the note (it spans 120..144 ticks).
    let edge = h.roll_point(6 * step, 64) - vec2(2.0, 0.0);
    let resized = h.drag(edge, h.roll_point(9 * step, 64));
    assert_eq!(resized, ["note.resize 1 120t 64 96t"]);
    h.settle();

    let removed = h.click(h.roll_point(6 * step, 64), PointerButton::Secondary);
    assert_eq!(removed, ["note.remove 1 120t 64"]);
    assert!(h.session.project().current_pattern().notes.is_empty());

    // New notes use the last drawn length.
    let added = h.click(h.roll_point(2, 60), PointerButton::Primary);
    assert_eq!(added, ["note.add 1 0t 96t 60 100"]);
}

#[test]
fn playlist_place_move_delete() {
    let mut h = Harness::new(&["pattern.add Verse"]);
    h.state.windows.playlist = true;
    h.settle();

    let placed = h.click(
        h.playlist_point(TICKS_PER_BAR + 10, 1),
        PointerButton::Primary,
    );
    assert_eq!(placed, ["playlist.add_clip 2 2 384t"]);
    h.settle();

    let moved = h.drag(
        h.playlist_point(TICKS_PER_BAR + 40, 1),
        h.playlist_point(3 * TICKS_PER_BAR + 40, 3),
    );
    assert_eq!(moved, ["playlist.move_clip 2 384t 4 1152t"]);
    h.settle();

    let removed = h.click(
        h.playlist_point(3 * TICKS_PER_BAR + 40, 3),
        PointerButton::Secondary,
    );
    assert_eq!(removed, ["playlist.remove_clip 4 1152t"]);
    assert!(h.session.project().playlist.clips.is_empty());
}

#[test]
fn single_key_tools_slice_and_mute() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 4 C5"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;
    // C picks the slice tool everywhere; a click cuts the note on the grid.
    assert!(h.key(egui::Key::C, Modifiers::NONE).is_empty());
    assert_eq!(h.state.roll.tool, crate::piano_roll::Tool::Slice);
    assert_eq!(h.state.playlist.tool, crate::piano_roll::Tool::Slice);
    let cut = h.click(h.roll_point(2 * step + 3, 60), PointerButton::Primary);
    assert_eq!(cut, ["note.slice 1 0t 60 48t"]);
    // S and O toggle slide and porta for new notes; P is draw again.
    h.key(egui::Key::S, Modifiers::NONE);
    h.key(egui::Key::O, Modifiers::NONE);
    assert!(h.state.roll.slide && h.state.roll.porta);
    h.key(egui::Key::P, Modifiers::NONE);
    assert_eq!(h.state.roll.tool, crate::piano_roll::Tool::Draw);
}

#[test]
fn playlist_slice_and_mute_tools() {
    let mut h = Harness::new(&["pattern.add", "playlist.add_clip 1 1 0 2"]);
    h.state.windows.playlist = true;
    h.settle();
    h.key(egui::Key::C, Modifiers::NONE);
    // On the playlist the slice tool cuts the clip.
    let cut = h.click(
        h.playlist_point(TICKS_PER_BAR + 2, 0),
        PointerButton::Primary,
    );
    assert_eq!(cut, ["playlist.slice_clip 1 384t"]);
    h.settle();
    // T: the mute tool mutes the clicked clip.
    h.key(egui::Key::T, Modifiers::NONE);
    let muted = h.click(
        h.playlist_point(TICKS_PER_BAR + 40, 0),
        PointerButton::Primary,
    );
    assert_eq!(muted, ["playlist.toggle_clip_mute 1 384t"]);
    assert!(h.session.project().playlist.clips[1].muted);
}

#[test]
fn number_keys_select_patterns_or_mute_channels_over_the_rack() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "channel.add_synth",
        "pattern.add",
        "pattern.select 1",
    ]);
    h.settle();
    // Anywhere else, 2 selects pattern 2.
    assert_eq!(
        h.key(egui::Key::Num2, Modifiers::NONE),
        ["pattern.select 2"]
    );
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::new("channel-rack")))
        .expect("rack drawn");
    h.frame(vec![Event::PointerMoved(rack.center())]);
    assert_eq!(
        h.key(egui::Key::Num2, Modifiers::NONE),
        ["channel.toggle_mute 2"]
    );
    assert_eq!(
        h.key(egui::Key::Num1, Modifiers::COMMAND),
        ["channel.solo 1"]
    );
    h.state.selected_channel = 1;
    assert_eq!(
        h.key(egui::Key::ArrowUp, Modifiers::ALT),
        ["channel.move 2 1"]
    );
    assert_eq!(h.state.selected_channel, 0);
    assert_eq!(
        h.key(egui::Key::L, Modifiers::COMMAND),
        ["channel.route_free 1"]
    );
    assert_eq!(h.session.project().channels[0].mixer_track, 1);
    assert_eq!(
        h.key(egui::Key::ArrowRight, Modifiers::COMMAND | Modifiers::SHIFT),
        ["step.shift 1 1"]
    );
}

#[test]
fn the_step_graph_editor_edits_the_note_under_a_bar() {
    let mut h = Harness::new(&["channel.add_synth", "step.toggle 1 0", "step.toggle 1 4"]);
    h.state.windows.channel_rack = true;
    h.state.graph = Some(crate::piano_roll::LaneMode::Velocity);
    h.settle();
    let (rect, steps) = h.state.graph_view.expect("graph drawn");
    let width = rect.width() / steps as f32;
    // High on the bar of step 4: a loud velocity for the note there.
    let found = h.click(
        pos2(rect.left() + width * 4.5, rect.top() + 8.0),
        PointerButton::Primary,
    );
    assert!(
        found
            .iter()
            .any(|c| c.starts_with("note.set_velocity 1 96t 60")),
        "{found:?}"
    );
}

#[test]
fn shortcuts_run_commands_but_not_while_typing() {
    let mut h = Harness::new(&[]);
    h.settle();
    let space = |pressed| Event::Key {
        key: egui::Key::Space,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(h.frame(vec![space(true)]), ["transport.toggle"]);
    h.frame(vec![space(false)]);
    assert!(h.session.project().transport().is_playing());

    let undo = Event::Key {
        key: egui::Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    h.session.run_line("project.set_tempo 99").unwrap();
    assert_eq!(h.frame(vec![undo]), ["edit.undo"]);

    // F9 toggles the mixer window without touching the project.
    let f9 = Event::Key {
        key: egui::Key::F9,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    assert!(h.frame(vec![f9]).is_empty());
    assert!(h.state.windows.mixer);

    // With a text field focused (rename prompt), Space types instead.
    h.state
        .open_dialog(crate::dialogs::Dialog::rename_pattern(0, "P"));
    h.settle();
    assert!(h.frame(vec![space(true)]).is_empty());
}

#[test]
fn typing_keyboard_performs_notes_instead_of_shortcuts() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.settle();
    let key = |key, pressed| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    // Off: L toggles the play mode as usual.
    assert_eq!(
        h.frame(vec![key(egui::Key::L, true)]),
        ["transport.toggle_mode"]
    );
    h.state.typing_keyboard = true;
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
        events: vec![
            key(egui::Key::Z, true),
            key(egui::Key::Q, true),
            key(egui::Key::Z, false),
        ],
        ..Default::default()
    };
    let engine = EngineStatus::default();
    let history = History::default();
    let frame = Frame {
        project: h.session.project(),
        engine: &engine,
        history: &history,
        waveform: None,
        waveform_seconds: None,
        clip_waveforms: &[],
        clip_durations: &[],
        plugins: &Plugins::default(),
        settings: &Settings::default(),
        settings_info: &SettingsInfo::default(),
    };
    let mut actions = Vec::new();
    let mut out = h
        .ctx
        .run_ui(input, |ui| actions = show(ui, &mut h.state, &frame));
    out.textures_delta.clear();
    assert_eq!(
        actions,
        [
            Action::Perform {
                channel: 0,
                key: 48,
                on: true
            },
            Action::Perform {
                channel: 0,
                key: 60,
                on: true
            },
            Action::Perform {
                channel: 0,
                key: 48,
                on: false
            },
        ]
    );
}

#[test]
fn velocity_lane_drag_is_one_undo_step() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 4 C5 100"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let (grid, clip) = h.state.roll.last_view.unwrap();
    let x = grid.x(10.0);
    let from = pos2(x, clip.bottom() - 45.0);
    let to = pos2(x, clip.bottom() - 90.0);
    let commands = h.drag(from, to);
    assert!(!commands.is_empty());
    assert!(
        commands
            .iter()
            .all(|c| c.starts_with("note.set_velocity 1 0t 60 ")),
        "{commands:?}"
    );
    let velocity = h.session.project().current_pattern().notes[0].velocity;
    assert!(velocity > 100, "dragged up to {velocity}");
    assert_eq!(
        h.session.undo_depth(),
        3,
        "two setup commands plus one gesture"
    );
    h.session.run_line("edit.undo").unwrap();
    assert_eq!(h.session.project().current_pattern().notes[0].velocity, 100);
}

impl Harness {
    fn key(&mut self, key: egui::Key, modifiers: Modifiers) -> Vec<String> {
        let held = self.modifiers;
        self.modifiers = modifiers;
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        let out = self.frame(vec![event(true), event(false)]);
        self.modifiers = held;
        out
    }

    fn roll_notes(&self) -> Vec<(u32, u8)> {
        self.session
            .project()
            .current_pattern()
            .notes
            .iter()
            .map(|n| (n.start, n.key))
            .collect()
    }

    /// Keeps the pointer over the piano roll so its editing keys are active.
    fn hover_roll(&mut self, tick: u32, key: u8) {
        let p = self.roll_point(tick, key);
        self.frame(vec![Event::PointerMoved(p)]);
    }
}

#[test]
fn piano_roll_selection_editing() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "note.add 1 0 1 60",
        "note.add 1 2 1 62",
        "note.add 1 8 1 64",
    ]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;

    // Ctrl+drag a box around the first two notes.
    h.modifiers = Modifiers::COMMAND;
    let selected = h.drag(
        h.roll_point(step / 2 + 300, 66),
        h.roll_point(step / 2 + 300, 58),
    );
    assert!(selected.is_empty(), "selecting runs no commands");
    h.modifiers = Modifiers::NONE;
    // The box above started right of every note: nothing selected yet.
    assert!(h.state.roll.selection.is_empty());
    h.modifiers = Modifiers::COMMAND;
    h.drag(h.roll_point(3 * step + 6, 63), h.roll_point(2, 59));
    h.modifiers = Modifiers::NONE;
    assert_eq!(h.state.roll.selection.len(), 2);

    // Drag one of them: both move two steps right.
    let moved = h.drag(h.roll_point(6, 60), h.roll_point(2 * step + 6, 60));
    assert_eq!(moved.len(), 1);
    assert!(moved[0].starts_with("edit.batch"), "{moved:?}");
    assert_eq!(h.roll_notes(), [(48, 60), (96, 62), (192, 64)]);
    h.session.run_line("edit.undo").unwrap();
    assert_eq!(h.roll_notes(), [(0, 60), (48, 62), (192, 64)]);
    h.session.run_line("edit.redo").unwrap();
    h.settle();

    // Plain arrows leave notes alone; Shift+Up transposes the selection a
    // semitone, Ctrl+Up/Down an octave, Shift+Left/Right a snap step.
    h.hover_roll(10 * step, 60);
    h.key(egui::Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.roll_notes(), [(48, 60), (96, 62), (192, 64)]);
    h.key(egui::Key::ArrowUp, Modifiers::COMMAND);
    assert_eq!(h.roll_notes(), [(48, 72), (96, 74), (192, 64)]);
    h.key(egui::Key::ArrowDown, Modifiers::COMMAND);
    h.key(egui::Key::ArrowRight, Modifiers::SHIFT);
    h.key(egui::Key::ArrowLeft, Modifiers::SHIFT);
    assert_eq!(h.roll_notes(), [(48, 60), (96, 62), (192, 64)]);
    h.key(egui::Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(h.roll_notes(), [(48, 61), (96, 63), (192, 64)]);
    // Ctrl+B duplicates the selection to the right.
    h.key(egui::Key::B, Modifiers::COMMAND);
    assert_eq!(h.roll_notes().len(), 5);
    assert!(
        h.roll_notes().contains(&(120, 61)),
        "copy starts where the selection ends: {:?}",
        h.roll_notes()
    );

    // Ctrl+A then Delete clears the pattern in one undo step.
    h.key(egui::Key::A, Modifiers::COMMAND);
    h.key(egui::Key::Delete, Modifiers::NONE);
    assert!(h.roll_notes().is_empty());
    h.session.run_line("edit.undo").unwrap();
    assert_eq!(h.roll_notes().len(), 5);

    // Copy one note and paste it where the pointer is.
    h.settle();
    h.click(h.roll_point(192 + 6, 64), PointerButton::Primary);
    h.key(egui::Key::C, Modifiers::COMMAND);
    h.hover_roll(12 * step + 2, 70);
    h.key(egui::Key::V, Modifiers::COMMAND);
    assert!(h.roll_notes().contains(&(288, 64)), "{:?}", h.roll_notes());
}

#[test]
fn f4_goes_to_the_next_empty_pattern_and_offers_to_name_it() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 1 C5"]);
    h.settle();
    assert_eq!(
        h.key(egui::Key::F4, Modifiers::NONE),
        ["pattern.next_empty"]
    );
    assert_eq!(h.session.project().patterns.len(), 2);
    assert!(h.state.dialog.is_some(), "a rename prompt for pattern 2");
    h.settle();
    let mut typed = h.frame(vec![
        Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        },
        Event::Text("Drop".to_owned()),
    ]);
    typed.extend(h.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]));
    assert_eq!(typed, ["pattern.rename 2 Drop"]);
}

#[test]
fn playlist_selection_editing() {
    let mut h = Harness::new(&[
        "pattern.add",
        "playlist.add_clip 1 1 0",
        "playlist.add_clip 2 2 0",
        "playlist.add_clip 1 1 4",
    ]);
    h.state.windows.playlist = true;
    h.settle();
    let clips = |h: &Harness| -> Vec<(u32, u32)> {
        h.session
            .project()
            .playlist
            .clips
            .iter()
            .map(|c| (c.track, c.start))
            .collect()
    };
    let bar = TICKS_PER_BAR;

    // Shift+click selects the first two clips; dragging one moves both.
    h.click(h.playlist_point(40, 0), PointerButton::Primary);
    h.modifiers = Modifiers::SHIFT;
    h.click(h.playlist_point(40, 1), PointerButton::Primary);
    h.modifiers = Modifiers::NONE;
    assert_eq!(h.state.playlist.selection.len(), 2);
    let moved = h.drag(h.playlist_point(40, 0), h.playlist_point(bar + 40, 0));
    assert!(moved[0].starts_with("edit.batch"), "{moved:?}");
    assert_eq!(clips(&h), [(0, bar), (0, 4 * bar), (1, bar)]);

    // Ctrl+B duplicates the selection right after itself; Delete removes it.
    h.settle();
    h.frame(vec![Event::PointerMoved(h.playlist_point(10 * bar, 5))]);
    h.key(egui::Key::B, Modifiers::COMMAND);
    assert_eq!(
        clips(&h),
        [(0, bar), (0, 2 * bar), (0, 4 * bar), (1, bar), (1, 2 * bar)]
    );
    h.key(egui::Key::Delete, Modifiers::NONE);
    assert_eq!(clips(&h), [(0, bar), (0, 4 * bar), (1, bar)]);

    // Ctrl+drag a box over track 1 selects both of its clips.
    h.settle();
    h.modifiers = Modifiers::COMMAND;
    h.drag(h.playlist_point(6 * bar, 0), h.playlist_point(10, 0));
    h.modifiers = Modifiers::NONE;
    assert_eq!(
        h.state.playlist.selection,
        BTreeSet::from([(0, bar), (0, 4 * bar)])
    );
}

#[test]
fn automation_editor_adds_moves_and_removes_points() {
    let mut h = Harness::new(&["channel.add_synth", "automation.create channel:1:cutoff"]);
    let id = h.session.project().automations[0].id;
    h.state.automation_editor = Some(id);
    h.settle();
    let (area, length) = h.state.automation_view.expect("editor drawn");
    let at = |tick: u32, value: f32| {
        pos2(
            area.left() + tick as f32 / length as f32 * area.width(),
            area.bottom() - value * area.height(),
        )
    };
    let before = h.session.project().automations[0].points.clone();
    let v = before[0].value;

    let added = h.click(at(TICKS_PER_BAR, 0.25), PointerButton::Primary);
    assert_eq!(added.len(), 1);
    assert!(
        added[0].starts_with("automation.add_point 1 384t 0.2"),
        "{added:?}"
    );
    h.settle();

    let moved = h.drag(at(TICKS_PER_BAR, 0.25), at(2 * TICKS_PER_BAR, 0.9));
    assert!(
        moved[0].starts_with("automation.move_point 1 384t 768t 0.9"),
        "{moved:?}"
    );
    h.settle();

    let removed = h.click(at(2 * TICKS_PER_BAR, 0.9), PointerButton::Secondary);
    assert_eq!(removed, ["automation.remove_point 1 768t"]);
    let points = &h.session.project().automations[0].points;
    assert_eq!(points.len(), 2);
    assert!((points[0].value - v).abs() < 1e-6);
}

#[test]
fn automate_request_creates_and_places_a_clip() {
    let mut h = Harness::new(&["channel.add_synth", "playlist.add_clip 1 1 0"]);
    h.settle();
    let engine = EngineStatus::default();
    let history = History::default();
    let frame = Frame {
        project: h.session.project(),
        engine: &engine,
        history: &history,
        waveform: None,
        waveform_seconds: None,
        clip_waveforms: &[],
        clip_durations: &[],
        plugins: &Plugins::default(),
        settings: &Settings::default(),
        settings_info: &SettingsInfo::default(),
    };
    let out = crate::resolve_ui_actions(
        &mut h.state,
        &frame,
        vec![Action::Run(
            musiccraft_commands::Invocation::parse("ui.automate mixer:0:volume").unwrap(),
        )],
    );
    let Action::Run(batch) = &out[0] else {
        panic!("{out:?}")
    };
    h.session.run(batch).unwrap();
    let project = h.session.project();
    assert_eq!(project.automations.len(), 1);
    let clip = project
        .playlist
        .clips
        .iter()
        .find(|c| c.automation().is_some())
        .unwrap();
    assert_eq!(clip.track, 1, "placed on the first free track");
    h.session.run_line("edit.undo").unwrap();
    assert!(h.session.project().automations.is_empty());
}

#[test]
fn ruler_sets_loop_and_jumps_to_markers() {
    let mut h = Harness::new(&["playlist.add_marker 3 Drop"]);
    h.state.windows.playlist = true;
    h.settle();
    let (grid, clip) = h.state.playlist.last_view.unwrap();
    let on_ruler = |tick: u32| pos2(grid.x(tick as f32), clip.top() + 10.0);
    h.modifiers = Modifiers::SHIFT;
    let set = h.drag(
        on_ruler(TICKS_PER_BAR + 10),
        on_ruler(4 * TICKS_PER_BAR - 10),
    );
    h.modifiers = Modifiers::NONE;
    assert_eq!(set, ["transport.set_loop 384t 1536t"]);
    assert_eq!(h.session.project().playlist.loop_range, Some((384, 1536)));
    // Clicking a marker's label seeks exactly to it.
    let engine = EngineStatus::default();
    let history = History::default();
    let pos = on_ruler(3 * TICKS_PER_BAR) + vec2(20.0, 0.0);
    h.frame(vec![Event::PointerMoved(pos)]);
    let mut actions = Vec::new();
    for pressed in [true, false] {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            time: Some(h.time),
            events: vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        };
        h.time += 1.0 / 60.0;
        let frame = Frame {
            project: h.session.project(),
            engine: &engine,
            history: &history,
            waveform: None,
            waveform_seconds: None,
            clip_waveforms: &[],
            clip_durations: &[],
            plugins: &Plugins::default(),
            settings: &Settings::default(),
            settings_info: &SettingsInfo::default(),
        };
        let mut out = h
            .ctx
            .run_ui(input, |ui| actions.extend(show(ui, &mut h.state, &frame)));
        out.textures_delta.clear();
    }
    assert!(
        actions.contains(&Action::Seek(f64::from(3 * TICKS_PER_BAR))),
        "{actions:?}"
    );
}

#[test]
fn plugin_channels_show_parameters_and_open_their_window() {
    let mut h = Harness::new(&["channel.add_plugin com.example.keys Keys"]);
    let instance = h.session.project().channels[0]
        .instrument
        .plugin()
        .unwrap()
        .instance;
    h.plugins = Plugins {
        available: true,
        catalog: vec![PluginEntry {
            id: "com.example.keys".into(),
            name: "Keys".into(),
            vendor: "Example".into(),
            instrument: true,
            format: "CLAP".into(),
        }],
        live: vec![LivePluginView {
            instance,
            params: vec![PluginParamView {
                id: 7,
                name: "Cutoff".into(),
                module: String::new(),
                min: 0.0,
                max: 10.0,
                default: 5.0,
                stepped: false,
                read_only: false,
                value: 0.0,
                text: "0.0".into(),
            }],
            has_editor: true,
            editor_open: false,
        }],
        running: vec![instance],
    };
    h.state.windows.channel_settings = true;
    h.settle();
    let (button, sliders) = h.state.plugin_layout.clone();
    let slider = sliders
        .iter()
        .find(|(id, _)| *id == 7)
        .map(|(_, r)| *r)
        .expect("slider drawn");
    let depth = h.session.undo_depth();
    let commands = h.drag(slider.left_center() + vec2(2.0, 0.0), slider.right_center());
    assert!(
        commands
            .iter()
            .all(|c| c.starts_with("channel.set_plugin_param 1 7 ")),
        "{commands:?}"
    );
    let value = h.session.project().channels[0]
        .instrument
        .plugin()
        .unwrap()
        .params[&7];
    assert!(value > 9.0, "dragged to the top, got {value}");
    assert_eq!(
        h.session.undo_depth(),
        depth + 1,
        "one drag is one undo step"
    );

    h.other.clear();
    h.click(
        button.expect("window button").center(),
        PointerButton::Primary,
    );
    assert!(h.other.contains(&Action::PluginEditor {
        instance,
        open: true
    }));
}

#[test]
fn missing_plugins_explain_themselves_instead_of_showing_knobs() {
    let mut h = Harness::new(&["channel.add_plugin com.nowhere.synth"]);
    h.plugins.available = true;
    h.state.windows.channel_settings = true;
    h.settle();
    assert!(h.state.plugin_layout.1.is_empty());
    assert!(h.state.plugin_layout.0.is_none());
}

#[test]
fn hints_show_in_the_bar_and_settings_open_from_f10() {
    let mut h = Harness::new(&[]);
    h.settle();
    // Hover along both toolbar rows until the play button's hint reaches
    // the hint bar (it shows the next frame).
    let rows = [14.0, 38.0, 60.0];
    let points = rows
        .into_iter()
        .flat_map(|y| (0..120).map(move |i| pos2(8.0 + 8.0 * i as f32, y)));
    let found = points.into_iter().any(|p| {
        h.frame(vec![Event::PointerMoved(p)]);
        h.frame(vec![]);
        h.state
            .hint
            .as_deref()
            .is_some_and(|hint| hint.contains("Play"))
    });
    assert!(found, "no play hint along the toolbar");
    h.frame(vec![Event::Key {
        key: egui::Key::F10,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(h.state.settings_tab, Some(crate::settings::Tab::Audio));
}

#[test]
fn dragging_an_eq_point_is_one_undoable_change() {
    let mut h = Harness::new(&["mixer.add_effect 0 eq"]);
    h.state.effect_windows.push((0, 0));
    h.settle();
    let graph = h.state.effect_graph.expect("graph drawn");
    let depth = h.session.undo_depth();
    // The middle point starts at 1 kHz, 0 dB: near the graph's middle.
    let x = graph.left() + graph.width() * (1_000.0_f32 / 20.0).ln() / 1_000.0_f32.ln();
    let start = pos2(x, graph.center().y);
    let commands = h.drag(start, start + vec2(40.0, -30.0));
    assert!(!commands.is_empty());
    assert!(
        commands.iter().all(|c| c.starts_with("edit.batch")),
        "{commands:?}"
    );
    let eq = &h.session.project().mixer[0].effects[0];
    assert!(eq.values[2] > 3.0, "mid gain raised: {}", eq.values[2]);
    assert!(eq.values[3] > 1_200.0, "mid freq raised: {}", eq.values[3]);
    assert_eq!(h.session.undo_depth(), depth + 1);
}

#[test]
fn effect_windows_close_with_their_slot() {
    let mut h = Harness::new(&["mixer.add_effect 0 reverb"]);
    h.state.effect_windows.push((0, 0));
    h.settle();
    h.session.run_line("mixer.remove_effect 0 1").unwrap();
    h.settle();
    assert!(h.state.effect_windows.is_empty());
}

#[test]
fn piano_roll_paint_delete_and_select_tools() {
    use crate::piano_roll::Tool;
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;

    // Paint: one drag lays down a run of notes in one undo step.
    h.state.roll.tool = Tool::Paint;
    let depth = h.session.undo_depth();
    let painted = h.drag(h.roll_point(step / 2, 62), h.roll_point(5 * step + 5, 62));
    assert_eq!(painted.len(), 1, "{painted:?}");
    assert!(painted[0].starts_with("edit.batch"), "{painted:?}");
    let notes = &h.session.project().current_pattern().notes;
    assert!(notes.len() >= 4, "painted {} notes", notes.len());
    assert!(
        notes
            .windows(2)
            .all(|w| w[1].start >= w[0].start + w[0].length)
    );
    assert_eq!(h.session.undo_depth(), depth + 1);
    h.settle();

    // Select: a box around the first two notes selects them.
    h.state.roll.tool = Tool::Select;
    let from = h.roll_point(0, 63) + vec2(2.0, 0.0);
    let to = h.roll_point(2 * step - 2, 61);
    assert!(h.drag(from, to).is_empty());
    assert_eq!(h.state.roll.selection.len(), 2);
    h.click(h.roll_point(14 * step, 70), PointerButton::Primary);
    assert!(h.state.roll.selection.is_empty());

    // Delete: dragging across notes removes all it touched, in one step.
    h.state.roll.tool = Tool::Delete;
    let before = h.session.project().current_pattern().notes.len();
    let erased = h.drag(h.roll_point(5, 62), h.roll_point(3 * step + 5, 62));
    assert_eq!(erased.len(), 1, "{erased:?}");
    let after = h.session.project().current_pattern().notes.len();
    assert!(after + 3 <= before, "{before} -> {after}");
}

#[test]
fn playlist_paint_and_delete_tools_are_single_steps() {
    use crate::piano_roll::Tool;
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 4 C4"]);
    h.state.windows.playlist = true;
    h.settle();
    let bar = TICKS_PER_BAR;
    h.state.playlist.tool = Tool::Paint;
    let depth = h.session.undo_depth();
    let painted = h.drag(
        h.playlist_point(bar / 4, 1),
        h.playlist_point(4 * bar + bar / 4, 1),
    );
    assert_eq!(painted.len(), 1, "{painted:?}");
    let clips = &h.session.project().playlist.clips;
    assert!(clips.len() >= 4, "painted {} clips", clips.len());
    assert!(clips.iter().all(|c| c.track == 1));
    assert_eq!(h.session.undo_depth(), depth + 1);
    h.settle();
    h.state.playlist.tool = Tool::Delete;
    let erased = h.drag(
        h.playlist_point(bar / 2, 1),
        h.playlist_point(2 * bar + bar / 2, 1),
    );
    assert_eq!(erased.len(), 1, "{erased:?}");
    assert!(h.session.project().playlist.clips.len() + 3 <= clips_len(&painted));
}

fn clips_len(painted: &[String]) -> usize {
    painted[0].matches("playlist.add_clip").count()
}

#[test]
fn slide_toggle_draws_slide_notes() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.piano_roll = true;
    h.state.roll.slide = true;
    h.settle();
    let added = h.click(
        h.roll_point(4 * TICKS_PER_STEP + 5, 62),
        PointerButton::Primary,
    );
    assert_eq!(added, ["note.add 1 96t 24t 62 100 0 slide"]);
    assert!(h.session.project().current_pattern().notes[0].slide);
}

#[test]
fn right_drag_deletes_every_note_it_passes() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "note.add 1 0 1 62",
        "note.add 1 2 1 62",
        "note.add 1 4 1 62",
        "note.add 1 8 1 62",
        "note.add 1 2 1 70",
    ]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;
    let depth = h.session.undo_depth();
    let removed = h.drag_with(
        h.roll_point(5, 62),
        h.roll_point(4 * step + 5, 62),
        PointerButton::Secondary,
    );
    assert_eq!(removed.len(), 1, "{removed:?}");
    let left: Vec<(u32, u8)> = h
        .session
        .project()
        .current_pattern()
        .notes
        .iter()
        .map(|n| (n.start, n.key))
        .collect();
    assert_eq!(left, [(2 * step, 70), (8 * step, 62)]);
    assert_eq!(h.session.undo_depth(), depth + 1);
    h.settle();
    let single = h.click(h.roll_point(8 * step + 5, 62), PointerButton::Secondary);
    assert_eq!(single.len(), 1);
    assert_eq!(h.session.project().current_pattern().notes.len(), 1);
}

#[test]
fn note_kinds_from_the_piano_roll() {
    use crate::piano_roll::{LaneMode, Tool};
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.piano_roll = true;
    h.state.roll.porta = true;
    h.state.roll.color = 3;
    h.settle();
    let step = TICKS_PER_STEP;
    let added = h.click(h.roll_point(2 * step + 5, 62), PointerButton::Primary);
    assert_eq!(added.len(), 1);
    assert!(added[0].starts_with("note.restore 1 "), "{added:?}");
    let n = h.session.project().current_pattern().notes[0];
    assert!(n.props.porta && n.props.color == 3);
    h.state.roll.porta = false;
    h.state.roll.color = 0;
    h.settle();
    h.click(h.roll_point(6 * step + 5, 62), PointerButton::Primary);
    h.settle();

    // The mute tool toggles every note it sweeps, in one step.
    h.state.roll.tool = Tool::Mute;
    let depth = h.session.undo_depth();
    h.drag(
        h.roll_point(2 * step + 5, 62),
        h.roll_point(6 * step + 5, 62),
    );
    assert!(
        h.session
            .project()
            .current_pattern()
            .notes
            .iter()
            .all(|n| n.props.muted)
    );
    assert_eq!(h.session.undo_depth(), depth + 1);
    h.state.roll.tool = Tool::Draw;

    // The lane edits fine pitch.
    h.state.roll.lane_mode = LaneMode::Fine;
    h.settle();
    let (grid, clip) = h.state.roll.last_view.unwrap();
    let x = grid.x((2 * step) as f32) + 2.0;
    h.drag(pos2(x, clip.bottom() - 40.0), pos2(x, clip.bottom() - 70.0));
    let fine = h.session.project().current_pattern().notes[0].props.fine;
    assert!(fine > 20, "fine raised: {fine}");
}

/// Steps lit on channel `n` (1-based) of the current pattern.
fn lit_steps(h: &Harness, n: usize) -> Vec<u32> {
    let project = h.session.project();
    let id = project.channels[n - 1].id;
    let mut steps: Vec<u32> = project
        .current_pattern()
        .notes_of(id)
        .map(|note| note.start / TICKS_PER_STEP)
        .collect();
    steps.sort_unstable();
    steps
}

#[test]
fn step_buttons_paint_with_the_left_button_and_erase_with_the_right() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick", "step.toggle 1 2"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rects: Vec<Rect> = h
        .state
        .step_layout
        .iter()
        .filter(|(channel, _)| *channel == 0)
        .map(|(_, rect)| *rect)
        .collect();
    let [first, second] = rects[..] else {
        panic!("steps drawn: {rects:?}");
    };
    let pitch = second.center() - first.center();
    let step = |n: usize| first.center() + pitch * n as f32;

    // A click toggles one step.
    h.click(step(0), PointerButton::Primary);
    assert_eq!(lit_steps(&h, 1), [0, 2]);
    // Right-click turns a lit step off and leaves an unlit one alone.
    h.click(step(0), PointerButton::Secondary);
    h.click(step(1), PointerButton::Secondary);
    assert_eq!(lit_steps(&h, 1), [2]);

    // A left drag from an unlit step paints every step it passes, as one
    // undo step.
    let depth = h.session.undo_depth();
    h.drag(step(4), step(7));
    assert_eq!(lit_steps(&h, 1), [2, 4, 5, 6, 7]);
    assert_eq!(h.session.undo_depth(), depth + 1);

    // A right drag erases.
    h.drag_with(step(3), step(5), PointerButton::Secondary);
    assert_eq!(lit_steps(&h, 1), [2, 6, 7]);
    h.session.run_line("edit.undo").unwrap();
    assert_eq!(lit_steps(&h, 1), [2, 4, 5, 6, 7]);

    // A left drag from a lit step clears the steps it passes.
    h.drag(step(6), step(7));
    assert_eq!(lit_steps(&h, 1), [2, 4, 5]);
}

#[test]
fn the_playlist_snaps_to_bars_of_the_song_meter() {
    // 3/4 bars of 288 ticks, then 7/8 bars of 336 from the third bar.
    let mut h = Harness::new(&[
        "pattern.add Verse",
        "project.set_time_signature 3/4",
        "playlist.set_time_signature 2 7/8",
    ]);
    h.state.windows.playlist = true;
    h.settle();
    let placed = h.click(h.playlist_point(288 + 10, 1), PointerButton::Primary);
    assert_eq!(placed, ["playlist.add_clip 2 2 288t"]);
    h.settle();
    let placed = h.click(h.playlist_point(576 + 300, 2), PointerButton::Primary);
    assert_eq!(placed, ["playlist.add_clip 2 3 576t"]);
    h.settle();
    // Shift+drag on the ruler sets a loop on bar lines of the meter.
    let (grid, clip) = h.state.playlist.last_view.unwrap();
    let on_ruler = |tick: u32| pos2(grid.x(tick as f32), clip.top() + 10.0);
    h.modifiers = Modifiers::SHIFT;
    let set = h.drag(on_ruler(288 + 10), on_ruler(576 + 336 - 10));
    h.modifiers = Modifiers::NONE;
    assert_eq!(set, ["transport.set_loop 288t 912t"]);
}

#[test]
fn the_rack_and_roll_follow_the_pattern_signature() {
    let mut h = Harness::new(&[
        "channel.add_sampler builtin:kick",
        "pattern.set_time_signature 1 7/8",
    ]);
    h.state.windows.piano_roll = true;
    h.settle();
    // One bar of 7/8 is 14 steps; the roll's bar snap is a 7/8 bar.
    assert_eq!(h.session.project().current_pattern().length_steps, 14);
    assert_eq!(h.state.roll.meter.first().to_string(), "7/8");
    assert_eq!(
        h.state.step_edit_ticks(0, &h.state.roll.meter),
        TICKS_PER_STEP
    );
}

#[test]
fn channel_settings_pages_switch_and_keep_the_header() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick"]);
    h.state.windows.channel_settings = true;
    h.settle();
    for tab in [
        crate::channel_settings::ChannelTab::Functions,
        crate::channel_settings::ChannelTab::Envelope,
        crate::channel_settings::ChannelTab::Sound,
    ] {
        h.state.channel_tab = tab;
        h.settle();
        assert_eq!(h.state.channel_tab, tab);
    }
    // A drum synth has no shared envelope: its page falls back to Sound.
    h.session.run_line("channel.add_drum kick").unwrap();
    h.state.selected_channel = 1;
    h.state.channel_tab = crate::channel_settings::ChannelTab::Envelope;
    h.settle();
    assert_eq!(
        h.state.channel_tab,
        crate::channel_settings::ChannelTab::Sound
    );
}

#[test]
fn alt_dragging_a_segment_bends_it_in_one_step() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "automation.create channel:1:cutoff",
        "automation.set_points 1 [[0,0.0],[1536,1.0]]",
    ]);
    let id = h.session.project().automations[0].id;
    h.state.automation_editor = Some(id);
    h.settle();
    let (area, length) = h.state.automation_view.expect("editor drawn");
    let at = |tick: u32, value: f32| {
        pos2(
            area.left() + tick as f32 / length as f32 * area.width(),
            area.bottom() - value * area.height(),
        )
    };
    let depth = h.session.undo_depth();
    h.modifiers = Modifiers::ALT;
    let bent = h.drag(at(768, 0.5), at(768, 0.5) + vec2(0.0, 60.0));
    h.modifiers = Modifiers::NONE;
    assert_eq!(bent.len(), 1, "{bent:?}");
    assert!(
        bent[0].starts_with("automation.set_curve 1 0t single 0."),
        "{bent:?}"
    );
    let point = h.session.project().automations[0].points[0];
    assert!(point.tension > 0.3, "bent down: {}", point.tension);
    assert_eq!(h.session.undo_depth(), depth + 1);
}

#[test]
fn pattern_miniatures_in_the_picker_select_the_pattern_to_paint() {
    let mut h = Harness::new(&[
        "channel.add_sampler builtin:kick",
        "step.toggle 1 1",
        "pattern.add",
        "pattern.select 1",
    ]);
    h.state.windows.playlist = true;
    h.state.playlist.paint_audio = Some(h.session.project().channels[0].id);
    h.settle();
    let items = h.state.playlist.picker_items.clone();
    assert_eq!(items.len(), 2, "one miniature per pattern: {items:?}");
    assert!(items[0].height() >= 30.0 && items[1].top() > items[0].bottom());
    // Clicking the second pattern's miniature selects it and paints patterns
    // again instead of audio.
    let commands = h.click(items[1].center(), PointerButton::Primary);
    assert_eq!(commands, ["pattern.select 2"]);
    assert_eq!(h.state.playlist.paint_audio, None);
    // Clicking the selected one again changes nothing in the project.
    let items = h.state.playlist.picker_items.clone();
    assert!(
        h.click(items[1].center(), PointerButton::Primary)
            .is_empty()
    );
}

#[test]
fn a_long_pattern_list_scrolls_without_covering_the_miniatures() {
    // Nine patterns are taller than the list, so it shows a scroll bar.
    let mut setup = vec!["channel.add_sampler builtin:kick"];
    setup.extend(["pattern.add"; 8]);
    setup.push("pattern.select 1");
    let mut h = Harness::new(&setup);
    h.state.windows.playlist = true;
    h.settle();
    let items = h.state.playlist.picker_items.clone();
    assert_eq!(items.len(), 9);
    // The right edge of a miniature still selects its pattern.
    let edge = pos2(items[2].right() - 2.0, items[2].center().y);
    assert_eq!(h.click(edge, PointerButton::Primary), ["pattern.select 3"]);
}

#[test]
fn the_picker_paints_audio_clips_at_their_length() {
    let mut h = Harness::new(&[
        "channel.add_sampler builtin:kick",
        "playlist.add_audio_clip 1 1 0 1",
    ]);
    h.state.windows.playlist = true;
    h.state.playlist.picker = crate::playlist::Picker::Audio;
    h.state.playlist.paint_audio = Some(h.session.project().channels[0].id);
    h.settle();
    let placed = h.click(
        h.playlist_point(TICKS_PER_BAR + 10, 1),
        PointerButton::Primary,
    );
    assert_eq!(placed, ["playlist.add_audio_clip 1 2 384t 384t"]);
}

/// The outer rectangle of window `id`.
fn window_rect(h: &Harness, id: &str) -> Rect {
    h.ctx
        .memory(|m| m.area_rect(egui::Id::new(id)))
        .expect("window drawn")
}

/// The centre of header button `n`, counted from the right end (0 = close).
fn header_button(h: &Harness, id: &str, n: f32) -> Pos2 {
    let rect = window_rect(h, id);
    pos2(rect.right() - 7.0 - 9.0 - 20.0 * n, rect.top() + 6.0 + 10.0)
}

#[test]
fn window_buttons_collapse_maximize_restore_and_close() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let before = window_rect(&h, "channel-rack");
    // Maximize fills the workspace; restore returns to the old rectangle.
    h.click(
        header_button(&h, "channel-rack", 1.0),
        PointerButton::Primary,
    );
    h.settle();
    assert!(crate::chrome::maximized(
        &h.ctx,
        egui::Id::new("channel-rack")
    ));
    let full = window_rect(&h, "channel-rack");
    assert!(
        (full.width() - h.state.workspace.width()).abs() < 2.0,
        "{full:?}"
    );
    assert!(
        (full.height() - h.state.workspace.height()).abs() < 2.0,
        "{full:?}"
    );
    h.click(
        header_button(&h, "channel-rack", 1.0),
        PointerButton::Primary,
    );
    h.settle();
    assert!(!crate::chrome::maximized(
        &h.ctx,
        egui::Id::new("channel-rack")
    ));
    let back = window_rect(&h, "channel-rack");
    assert!(
        (back.width() - before.width()).abs() < 2.0,
        "{back:?} vs {before:?}"
    );
    assert!(
        (back.left() - before.left()).abs() < 2.0,
        "{back:?} vs {before:?}"
    );
    // Collapse rolls the window up to its header.
    h.click(
        header_button(&h, "channel-rack", 2.0),
        PointerButton::Primary,
    );
    h.settle();
    assert!(crate::chrome::collapsed(
        &h.ctx,
        egui::Id::new("channel-rack")
    ));
    assert!(window_rect(&h, "channel-rack").height() < 40.0);
    // Close hides it.
    h.click(
        header_button(&h, "channel-rack", 0.0),
        PointerButton::Primary,
    );
    h.settle();
    assert!(!h.state.windows.channel_rack);
}

#[test]
fn a_narrow_header_folds_its_tools_instead_of_wrapping() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick"]);
    h.state.windows.piano_roll = true;
    h.settle();
    // Rolled up, the header is a single row however many tools it has.
    h.click(header_button(&h, "piano-roll", 2.0), PointerButton::Primary);
    h.settle();
    assert!(crate::chrome::collapsed(
        &h.ctx,
        egui::Id::new("piano-roll")
    ));
    let rect = window_rect(&h, "piano-roll");
    assert!(rect.height() < 40.0, "{rect:?}");
}

#[test]
fn the_playlist_is_a_window_in_the_lower_part_of_the_workspace() {
    let mut h = Harness::new(&["pattern.add"]);
    h.state.windows.playlist = true;
    h.settle();
    let rect = window_rect(&h, "playlist-window");
    let area = h.state.workspace;
    assert!(
        rect.top() > area.center().y - area.height() * 0.2,
        "{rect:?} in {area:?}"
    );
    assert!(
        (rect.bottom() - area.bottom()).abs() < 8.0,
        "{rect:?} in {area:?}"
    );
    assert!(rect.width() > area.width() - 16.0, "{rect:?} in {area:?}");
    // Its header closes it like any other window.
    h.click(
        header_button(&h, "playlist-window", 0.0),
        PointerButton::Primary,
    );
    h.settle();
    assert!(!h.state.windows.playlist);
}

#[test]
fn the_mixer_opens_at_the_top_beside_the_channel_rack() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick"]);
    h.state.windows.channel_rack = true;
    h.state.windows.mixer = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let mixer = window_rect(&h, "mixer-window");
    let area = h.state.workspace;
    assert!(mixer.left() >= rack.right(), "{mixer:?} beside {rack:?}");
    assert!(
        (mixer.right() - area.right()).abs() < 8.0,
        "{mixer:?} in {area:?}"
    );
    assert!(mixer.top() - area.top() < 8.0, "{mixer:?} in {area:?}");
    assert!(mixer.bottom() < area.center().y, "{mixer:?} in {area:?}");
}

#[test]
fn the_count_in_toggle_sits_in_the_toolbar() {
    let mut h = Harness::new(&[]);
    h.settle();
    let before = h.session.project().record.count_in;
    let spot = [14.0, 38.0]
        .into_iter()
        .flat_map(|y| (0..120).map(move |i| pos2(8.0 + 8.0 * i as f32, y)))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|hint| hint.starts_with("Count-in"))
        })
        .expect("a count-in toggle in the toolbar");
    h.click(spot, PointerButton::Primary);
    h.settle();
    assert_ne!(h.session.project().record.count_in, before);
}

#[test]
fn the_piano_roll_opens_over_the_right_half_with_its_own_bars() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let roll = window_rect(&h, "piano-roll");
    let area = h.state.workspace;
    assert!(roll.left() >= area.center().x - 8.0, "{roll:?} in {area:?}");
    assert!(roll.width() < area.width() * 0.6, "{roll:?} in {area:?}");

    // Dragging the zoom box up makes key rows taller; a double-click resets.
    let (grid, clip) = h.state.roll.last_view.expect("piano roll drawn");
    let bar = crate::scrollbars::BAR;
    let zoom = pos2(clip.right() + bar / 2.0, clip.top() - bar / 2.0);
    h.drag(zoom, zoom - vec2(0.0, 40.0));
    h.settle();
    let taller = h.state.roll.last_view.unwrap().0.row_height;
    assert!(taller > grid.row_height, "{taller} > {}", grid.row_height);
    h.click(zoom, PointerButton::Primary);
    h.click(zoom, PointerButton::Primary);
    h.settle();
    let reset = h.state.roll.last_view.unwrap().0.row_height;
    assert!((reset - grid.row_height).abs() < 0.01, "{reset}");

    // The corner block carries the Porta toggle at its top right.
    assert!(!h.state.roll.porta);
    let porta = pos2(clip.left() + 52.0 - 6.0, clip.top() - bar + 6.0);
    h.click(porta, PointerButton::Primary);
    h.settle();
    assert!(h.state.roll.porta);
}

#[test]
fn the_playlist_zoom_box_sets_the_track_height() {
    let mut h = Harness::new(&["pattern.add"]);
    h.state.windows.playlist = true;
    h.settle();
    let (grid, clip) = h.state.playlist.last_view.expect("playlist drawn");
    let bar = crate::scrollbars::BAR;
    let zoom = pos2(clip.right() + bar / 2.0, clip.top() - bar / 2.0);
    h.drag(zoom, zoom - vec2(0.0, 60.0));
    h.settle();
    let taller = h.state.playlist.last_view.unwrap().0.row_height;
    assert!(taller > grid.row_height, "{taller} > {}", grid.row_height);

    // The right chevron scrolls the lanes one bar to the left.
    let right = pos2(clip.right() - bar / 2.0, clip.top() - bar / 2.0);
    let before = h.state.playlist.last_view.unwrap().0.origin.x;
    h.click(right, PointerButton::Primary);
    h.settle();
    let after = h.state.playlist.last_view.unwrap().0.origin.x;
    assert!(after < before, "{after} < {before}");
}

#[test]
fn a_long_channel_rack_gets_the_plain_scroll_bar() {
    let setup: Vec<&str> = std::iter::repeat_n("channel.add_sampler builtin:kick", 24).collect();
    let mut h = Harness::new(&setup);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let bar = crate::scrollbars::BAR;
    let hint_at = |h: &mut Harness, p: Pos2| {
        h.frame(vec![Event::PointerMoved(p)]);
        h.frame(vec![]);
        h.state.hint.clone().unwrap_or_default()
    };
    let x = (0..40)
        .map(|i| rack.right() - 2.0 - i as f32)
        .find(|x| hint_at(&mut h, pos2(*x, rack.center().y)).starts_with("Click to page"))
        .expect("a vertical rail at the rack's right edge");
    let down = (0..200)
        .map(|i| pos2(x, rack.bottom() - i as f32 * 2.0))
        .find(|p| hint_at(&mut h, *p) == "Scroll down")
        .expect("a down chevron under the rail");
    assert!(rack.right() - x < 4.0 * bar, "{x} near {rack:?}");
    let offset =
        |h: &Harness| crate::scrollbars::plain_offset(&h.ctx, egui::Id::new("rack-bars")).y;
    assert_eq!(offset(&h), 0.0);
    for _ in 0..3 {
        h.click(down, PointerButton::Primary);
    }
    h.settle();
    assert!(offset(&h) > 0.0, "the rows scrolled");
}

#[test]
fn dragging_the_piano_roll_ruler_selects_a_time_range() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "note.add 1 0 1 60",
        "note.add 1 2 1 62",
        "note.add 1 8 1 64",
    ]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;
    let (grid, clip) = h.state.roll.last_view.expect("piano roll drawn");
    let y = clip.top() + 8.0;
    let at = |tick: u32| pos2(grid.x(tick as f32), y);
    let ran = h.drag(at(step / 2), at(4 * step));
    assert!(ran.is_empty(), "a time selection runs no commands: {ran:?}");
    assert_eq!(h.state.roll.time_selection, Some((0, 4 * step)));
    let selected: Vec<_> = h.state.roll.selection.iter().copied().collect();
    assert_eq!(selected, [(0, 60), (2 * step, 62)]);
    // A click on the ruler clears the range; Escape clears it too.
    h.click(at(step), PointerButton::Primary);
    h.settle();
    assert_eq!(h.state.roll.time_selection, None);
}

#[test]
fn ctrl_clicking_a_key_selects_its_notes() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "note.add 1 0 1 60",
        "note.add 1 4 1 60",
        "note.add 1 8 1 64",
    ]);
    h.state.windows.piano_roll = true;
    h.settle();
    let step = TICKS_PER_STEP;
    let (_, clip) = h.state.roll.last_view.expect("piano roll drawn");
    let key = pos2(clip.left() + 40.0, h.roll_point(0, 60).y);
    h.modifiers = Modifiers::COMMAND;
    let ran = h.click(key, PointerButton::Primary);
    h.modifiers = Modifiers::NONE;
    h.settle();
    assert!(ran.is_empty(), "{ran:?}");
    let selected: Vec<_> = h.state.roll.selection.iter().copied().collect();
    assert_eq!(selected, [(0, 60), (4 * step, 60)]);
    assert!(h.state.roll.preview.is_none(), "no note was left sounding");
}

#[test]
fn double_clicking_an_audio_clip_opens_its_channel_settings() {
    let mut h = Harness::new(&[
        "channel.add_synth",
        "channel.add_sampler builtin:kick",
        "playlist.add_audio_clip 2 1 0 2",
    ]);
    h.state.windows.playlist = true;
    h.state.selected_channel = 0;
    h.settle();
    let at = h.playlist_point(musiccraft_project::params::PPQ * 2, 0);
    h.click(at, PointerButton::Primary);
    h.click(at, PointerButton::Primary);
    h.settle();
    assert!(h.state.windows.channel_settings);
    assert_eq!(h.state.selected_channel, 1);
}

#[test]
fn dragging_a_browser_item_over_a_window_button_opens_that_window() {
    let mut h = Harness::new(&[]);
    h.settle();
    assert!(!h.state.windows.mixer);
    let spot = [14.0, 38.0, 60.0]
        .into_iter()
        .flat_map(|y| (0..160).map(move |i| pos2(8.0 + 8.0 * i as f32, y)))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|hint| hint.starts_with("Mixer (F9)"))
        })
        .expect("a mixer button in the toolbar");
    assert!(!h.state.windows.mixer, "hovering alone opens nothing");
    egui::DragAndDrop::set_payload(
        &h.ctx,
        crate::browser::Dragged::Sample("kick.wav".to_owned()),
    );
    h.frame(vec![Event::PointerMoved(spot + vec2(1.0, 0.0))]);
    h.frame(vec![]);
    assert!(h.state.windows.mixer);
}

#[test]
fn enter_maximizes_the_playlist_and_ctrl_shift_h_restores_the_default_layout() {
    let mut h = Harness::new(&[]);
    h.settle();
    let key = |key, modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let playlist = egui::Id::new("playlist-window");
    h.frame(vec![key(egui::Key::Enter, Modifiers::NONE)]);
    h.settle();
    assert!(h.state.windows.playlist, "Enter opens the playlist");
    assert!(crate::chrome::maximized(&h.ctx, playlist));
    let maximized = window_rect(&h, "playlist-window");
    assert!(maximized.width() > 1300.0, "{maximized:?}");

    let layout = Modifiers::COMMAND | Modifiers::SHIFT;
    h.modifiers = layout;
    h.frame(vec![key(egui::Key::H, layout)]);
    h.modifiers = Modifiers::NONE;
    h.settle();
    let w = &h.state.windows;
    assert!(w.playlist && w.channel_rack && w.mixer && w.piano_roll && w.browser);
    assert!(!crate::chrome::maximized(&h.ctx, playlist));
    let rack = window_rect(&h, "channel-rack");
    let list = window_rect(&h, "playlist-window");
    assert!(rack.bottom() <= list.top() + 1.0, "{rack:?} over {list:?}");
}

#[test]
fn window_keys_follow_the_focused_window_not_the_pointer() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let one = Event::Key {
        key: egui::Key::Num1,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let rack = window_rect(&h, "channel-rack");
    h.click(
        rack.left_top() + vec2(rack.width() / 2.0, 8.0),
        PointerButton::Primary,
    );
    h.settle();
    // The pointer leaves the rack; its keys still go to it.
    h.frame(vec![Event::PointerMoved(pos2(1390.0, 890.0))]);
    assert_eq!(h.frame(vec![one.clone()]), ["channel.toggle_mute 1"]);

    // Another window in front takes the keys.
    h.state.windows.piano_roll = true;
    h.settle();
    let roll = window_rect(&h, "piano-roll");
    h.click(
        roll.left_top() + vec2(roll.width() / 2.0, 8.0),
        PointerButton::Primary,
    );
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    h.frame(vec![Event::PointerMoved(rack.center())]);
    let out = h.frame(vec![one]);
    assert!(
        !out.iter().any(|c| c.starts_with("channel.toggle_mute")),
        "{out:?}"
    );
}

#[test]
fn esc_closes_the_focused_window_and_f12_hides_them_all() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 4 C5"]);
    h.state.windows.channel_rack = true;
    h.state.windows.piano_roll = true;
    h.settle();
    let key = |key, modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let roll = window_rect(&h, "piano-roll");
    h.click(
        roll.left_top() + vec2(roll.width() / 2.0, 8.0),
        PointerButton::Primary,
    );
    h.settle();
    // A selection takes Esc first.
    h.state.roll.selection.insert((0, 72));
    h.frame(vec![key(egui::Key::Escape, Modifiers::NONE)]);
    assert!(h.state.roll.selection.is_empty());
    assert!(h.state.windows.piano_roll);
    // Then Esc closes the focused window only.
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![key(egui::Key::Escape, Modifiers::NONE)]);
    assert!(!h.state.windows.piano_roll);
    assert!(h.state.windows.channel_rack);

    h.state.windows.mixer = true;
    h.settle();
    h.frame(vec![key(egui::Key::F12, Modifiers::NONE)]);
    let w = &h.state.windows;
    assert!(!w.channel_rack && !w.mixer && !w.piano_roll && !w.playlist);
}

#[test]
fn right_clicking_the_empty_workspace_opens_the_window_menu() {
    let mut h = Harness::new(&[]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    h.click(rack.center(), PointerButton::Secondary);
    h.settle();
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "a window takes its own clicks"
    );
    h.click(pos2(1200.0, 800.0), PointerButton::Secondary);
    h.settle();
    assert!(egui::Popup::is_any_open(&h.ctx));
}

#[test]
fn pressing_a_note_makes_its_length_and_velocity_the_defaults() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 8 C5 64"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let note = *h.session.project().current_pattern().notes.first().unwrap();
    assert_ne!(h.state.roll.last_length, note.length);
    let p = h.roll_point(note.start + 10, note.key);
    h.click(p, PointerButton::Primary);
    h.settle();
    assert_eq!(h.state.roll.last_length, note.length);
    assert_eq!(h.state.roll.velocity, 64);
}

#[test]
fn piano_roll_cursors_follow_the_tool() {
    let mut h = Harness::new(&["channel.add_synth", "note.add 1 0 8 C5"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let note = *h.session.project().current_pattern().notes.first().unwrap();
    let body = h.roll_point(note.start + 10, note.key);
    let empty = h.roll_point(note.start + 10, note.key - 5);
    h.frame(vec![Event::PointerMoved(body)]);
    assert_eq!(h.cursor, egui::CursorIcon::Move);
    h.frame(vec![Event::PointerMoved(empty)]);
    assert_eq!(h.cursor, egui::CursorIcon::Crosshair);
    h.state.roll.tool = crate::piano_roll::Tool::Delete;
    h.frame(vec![Event::PointerMoved(body)]);
    assert_eq!(h.cursor, egui::CursorIcon::Default);
}

#[test]
fn double_clicking_the_tempo_display_types_a_tempo() {
    let mut h = Harness::new(&[]);
    h.settle();
    let spot = [14.0, 38.0, 60.0]
        .into_iter()
        .flat_map(|y| (0..160).map(move |i| pos2(8.0 + 8.0 * i as f32, y)))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|hint| hint.starts_with("Tempo "))
        })
        .expect("a tempo display in the toolbar");
    h.click(spot, PointerButton::Primary);
    h.click(spot, PointerButton::Primary);
    h.settle();
    h.frame(vec![Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Text("98.5".to_owned())]);
    let out = h.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(out, ["project.set_tempo 98.500"]);
    assert!(!h.state.windows.playlist, "Enter stays in the field");
}

#[test]
fn mixer_strips_step_by_whole_strips() {
    let mut h = Harness::new(&[]);
    h.state.windows.mixer = true;
    h.settle();
    let bars = egui::Id::new("mixer-strip-bars");
    let offset = |h: &Harness| crate::scrollbars::plain_offset(&h.ctx, bars).x;
    let before = offset(&h);
    // What one wheel notch over a strip's name asks for.
    crate::scrollbars::plain_step(&h.ctx, bars, vec2(42.0, 0.0));
    h.settle();
    assert!(
        (offset(&h) - before - 42.0).abs() < 0.5,
        "{before} → {}",
        offset(&h)
    );
}

#[test]
fn ctrl_clicking_a_mute_light_solos_the_channel() {
    let mut h = Harness::new(&["channel.add_synth", "channel.add_synth"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let light = (0..40)
        .flat_map(|j| (0..40).map(move |i| (i, j)))
        .map(|(i, j)| rack.left_top() + vec2(4.0 + 3.0 * i as f32, 20.0 + 3.0 * j as f32))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|t| t.starts_with("Mute · Ctrl+click"))
        })
        .expect("a mute light");
    h.modifiers = Modifiers::COMMAND;
    let out = h.click(light, PointerButton::Primary);
    h.modifiers = Modifiers::NONE;
    assert_eq!(out, ["channel.solo 1"]);
}

#[test]
fn shift_wheel_over_the_rack_header_steps_patterns() {
    let mut h = Harness::new(&["pattern.add", "pattern.add"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let wheel = Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::SHIFT,
    };
    h.modifiers = Modifiers::SHIFT;
    let stepped = (0..60)
        .map(|i| rack.left_top() + vec2(30.0 + 6.0 * i as f32, 11.0))
        .any(|p| {
            h.frame(vec![Event::PointerMoved(p)]);
            h.frame(vec![wheel.clone()])
                .iter()
                .any(|c| c == "pattern.next")
        });
    h.modifiers = Modifiers::NONE;
    assert!(stepped, "no header spot stepped the pattern");
}

#[test]
fn automated_controls_name_their_source_in_the_hint() {
    let mut h = Harness::new(&["channel.add_synth", "automation.create channel:1:pan"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let hints: Vec<String> = (0..40)
        .flat_map(|j| (0..60).map(move |i| (i, j)))
        .map(|(i, j)| rack.left_top() + vec2(4.0 + 3.0 * i as f32, 20.0 + 3.0 * j as f32))
        .filter_map(|p| {
            h.frame(vec![Event::PointerMoved(p)]);
            h.frame(vec![]);
            h.state.hint.clone()
        })
        .collect();
    let pan = hints
        .iter()
        .find(|t| t.starts_with("Pan "))
        .expect("the pan knob's hint");
    assert!(pan.contains(" · automated (clip '"), "{pan}");
    let volume = hints
        .iter()
        .find(|t| t.starts_with("Volume") || t.starts_with("Level"))
        .expect("the volume knob's hint");
    assert!(!volume.contains("automated"), "{volume}");
}

#[test]
fn the_type_in_box_sets_a_control_and_esc_cancels() {
    let key = |key: egui::Key, modifiers: Modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let release = |key: egui::Key| Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let mut h = Harness::new(&[]);
    let open = |h: &mut Harness| {
        crate::type_in::open(
            &mut h.state,
            h.session.project(),
            "mixer:0:pan",
            egui::pos2(200.0, 200.0),
        );
        h.settle();
    };
    open(&mut h);
    assert!(h.state.type_in.is_some());
    h.frame(vec![key(egui::Key::A, Modifiers::COMMAND)]);
    h.frame(vec![Event::Text("25".to_owned())]);
    let out = h.frame(vec![key(egui::Key::Enter, Modifiers::NONE)]);
    assert_eq!(out, ["control.set mixer:0:pan 0.25"]);
    assert!(h.state.type_in.is_none());
    h.frame(vec![release(egui::Key::Enter)]);
    open(&mut h);
    let out = h.frame(vec![key(egui::Key::Escape, Modifiers::NONE)]);
    assert!(out.is_empty(), "{out:?}");
    assert!(h.state.type_in.is_none());
}

#[test]
fn double_clicking_a_channel_name_renames_it_in_place() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let name = (0..40)
        .flat_map(|j| (0..60).map(move |i| (i, j)))
        .map(|(i, j)| rack.left_top() + vec2(4.0 + 4.0 * i as f32, 20.0 + 3.0 * j as f32))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state.hint.as_deref() == Some("Synth")
        })
        .expect("the channel's name");
    h.click(name, PointerButton::Primary);
    h.click(name, PointerButton::Primary);
    h.settle();
    assert!(h.state.renaming.is_some(), "a double-click opens the field");
    assert!(h.state.windows.channel_settings, "the settings stay open");
    h.frame(vec![Event::Text("Lead".to_owned())]);
    let out = h.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(out, ["channel.rename 1 Lead"]);
    assert!(h.state.renaming.is_none());
}

#[test]
fn dropping_a_sound_on_a_sampler_row_replaces_its_sample() {
    let mut h = Harness::new(&["channel.add_sampler builtin:kick", "channel.add_synth"]);
    h.state.windows.channel_rack = true;
    h.settle();
    let rack = window_rect(&h, "channel-rack");
    let span = |h: &Harness, index: usize| {
        h.state
            .rack_rows
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, span)| span.center())
            .expect("the row is drawn")
    };
    let drop_at = |h: &mut Harness, y: f32| {
        egui::DragAndDrop::set_payload(
            &h.ctx,
            crate::browser::Dragged::Sample("builtin:clap".to_owned()),
        );
        let at = pos2(rack.center().x, y);
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }])
    };
    let sampler = span(&h, 0);
    assert_eq!(
        drop_at(&mut h, sampler),
        ["channel.set_sample 1 builtin:clap"]
    );
    // Over the synth's row it is added as a new channel instead.
    let synth = span(&h, 1);
    assert_eq!(drop_at(&mut h, synth), ["channel.add_sampler builtin:clap"]);
}

#[test]
fn mixer_strip_lights_solo_on_ctrl_click() {
    let mut h = Harness::new(&["mixer.add_effect 1 delay"]);
    h.state.windows.mixer = true;
    h.settle();
    let mixer = window_rect(&h, "mixer-window");
    let find = |h: &mut Harness, prefix: &str| {
        (0..100)
            .flat_map(|j| (0..60).map(move |i| (i, j)))
            .map(|(i, j)| mixer.left_top() + vec2(4.0 + 4.0 * i as f32, 20.0 + 4.0 * j as f32))
            .find(|p| {
                h.frame(vec![Event::PointerMoved(*p)]);
                h.frame(vec![]);
                h.state
                    .hint
                    .as_deref()
                    .is_some_and(|t| t.starts_with(prefix))
            })
    };
    let light =
        find(&mut h, "Enable (on) · click to mute or unmute · Ctrl").expect("insert 1's light");
    h.modifiers = Modifiers::COMMAND;
    let out = h.click(light, PointerButton::Primary);
    h.modifiers = Modifiers::NONE;
    assert_eq!(out, ["mixer.toggle_solo 1"]);
}

#[test]
fn the_piano_roll_play_button_plays_the_pattern() {
    let mut h = Harness::new(&["channel.add_synth", "transport.set_mode song"]);
    h.state.windows.piano_roll = true;
    h.settle();
    let roll = window_rect(&h, "piano-roll");
    let button = (0..120)
        .map(|i| roll.left_top() + vec2(4.0 + 4.0 * i as f32, 11.0))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|t| t.starts_with("Play the pattern"))
        })
        .expect("a play button in the piano roll header");
    let out = h.click(button, PointerButton::Primary);
    assert_eq!(out, ["transport.set_mode pattern", "transport.play"]);
}

#[test]
fn double_clicking_stop_silences_every_sound() {
    let mut h = Harness::new(&[]);
    h.settle();
    let stop = [14.0, 38.0, 60.0]
        .into_iter()
        .flat_map(|y| (0..170).map(move |i| pos2(8.0 + 8.0 * i as f32, y)))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state
                .hint
                .as_deref()
                .is_some_and(|hint| hint.starts_with("Stop and return"))
        })
        .expect("a stop button in the toolbar");
    let mut out = h.click(stop, PointerButton::Primary);
    out.extend(h.click(stop, PointerButton::Primary));
    assert_eq!(out, ["transport.stop", "transport.panic"]);
}

#[test]
fn the_channel_strip_switches_pages_mutes_and_routes_to_a_free_track() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.channel_settings = true;
    h.settle();
    let window = window_rect(&h, "channel-settings");
    let find = |h: &mut Harness, prefix: &str| -> Pos2 {
        (0..14)
            .flat_map(|j| (0..80).map(move |i| (i, j)))
            .map(|(i, j)| window.left_top() + vec2(4.0 + 6.0 * i as f32, 24.0 + 5.0 * j as f32))
            .find(|p| {
                h.frame(vec![Event::PointerMoved(*p)]);
                h.frame(vec![]);
                h.state
                    .hint
                    .as_deref()
                    .is_some_and(|t| t.starts_with(prefix))
            })
            .unwrap_or_else(|| panic!("no strip control hinted {prefix:?}"))
    };
    let functions = find(&mut h, "Functions —");
    h.click(functions, PointerButton::Primary);
    h.settle();
    assert_eq!(
        h.state.channel_tab,
        crate::channel_settings::ChannelTab::Functions
    );
    let on = find(&mut h, "On:");
    h.click(on, PointerButton::Primary);
    h.settle();
    assert!(h.session.project().channels[0].mute);
    let before = h.session.project().channels[0].mixer_track;
    let caption = find(&mut h, "Click to route this channel to a free mixer track");
    h.click(caption, PointerButton::Primary);
    h.settle();
    assert_ne!(h.session.project().channels[0].mixer_track, before);
}

#[test]
fn the_effect_window_strip_bypasses_and_its_header_steps_presets() {
    let mut h = Harness::new(&["mixer.add_effect 1 delay"]);
    h.state.effect_windows.push((1, 0));
    h.settle();
    let window = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::new(("effect-window", 1usize, 0usize))))
        .expect("effect window drawn");
    let find = |h: &mut Harness, prefix: &str| -> Pos2 {
        (0..12)
            .flat_map(|j| (0..80).map(move |i| (i, j)))
            .map(|(i, j)| window.left_top() + vec2(4.0 + 5.0 * i as f32, 8.0 + 5.0 * j as f32))
            .find(|p| {
                h.frame(vec![Event::PointerMoved(*p)]);
                h.frame(vec![]);
                h.state
                    .hint
                    .as_deref()
                    .is_some_and(|t| t.starts_with(prefix))
            })
            .unwrap_or_else(|| panic!("nothing hinted {prefix:?}"))
    };
    let light = find(&mut h, "Effect on");
    h.click(light, PointerButton::Primary);
    h.settle();
    assert!(!h.session.project().mixer[1].effects[0].enabled);
    let before = h.session.project().mixer[1].effects[0].values;
    // The default delay also matches its second preset, so step backwards
    // (to the last one) to see a change.
    let previous = find(&mut h, "Previous preset");
    h.click(previous, PointerButton::Primary);
    h.settle();
    assert_ne!(h.session.project().mixer[1].effects[0].values, before);
}

#[test]
fn instrument_presets_step_from_the_channel_window_header() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.channel_settings = true;
    h.settle();
    let window = window_rect(&h, "channel-settings");
    let previous = (0..80)
        .map(|i| window.left_top() + vec2(4.0 + 4.0 * i as f32, 16.0))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state.hint.as_deref() == Some("Previous preset")
        })
        .expect("the header's previous-preset chevron");
    let before = h.session.project().channels[0].params;
    h.click(previous, PointerButton::Primary);
    h.settle();
    assert_ne!(h.session.project().channels[0].params, before);
    h.session.run_line("edit.undo").unwrap();
    assert_eq!(h.session.project().channels[0].params, before);
}

#[test]
fn midi_output_and_stack_windows_show_a_short_strip() {
    let mut h = Harness::new(&["channel.add_midi_out", "channel.add_layer"]);
    h.state.windows.channel_settings = true;
    h.state.channel_tab = crate::channel_settings::ChannelTab::Functions;
    h.settle();
    // MIDI output: only the Sound page, and the strip carries the MIDI channel.
    assert_eq!(
        h.state.channel_tab,
        crate::channel_settings::ChannelTab::Sound
    );
    let window = window_rect(&h, "channel-settings");
    let hints: Vec<String> = (0..12)
        .flat_map(|j| (0..80).map(move |i| (i, j)))
        .map(|(i, j)| window.left_top() + vec2(4.0 + 6.0 * i as f32, 24.0 + 5.0 * j as f32))
        .filter_map(|p| {
            h.frame(vec![Event::PointerMoved(p)]);
            h.frame(vec![]);
            h.state.hint.clone()
        })
        .collect();
    assert!(
        hints
            .iter()
            .any(|t| t.starts_with("The MIDI channel (1–16)"))
    );
    assert!(hints.iter().any(|t| t.starts_with("On:")));
    assert!(!hints.iter().any(|t| t.starts_with("Functions —")));
    assert!(!hints.iter().any(|t| t.starts_with("Mixer track:")));
    // A channel stack keeps the Sound page too.
    h.state.selected_channel = 1;
    h.state.channel_tab = crate::channel_settings::ChannelTab::Functions;
    h.settle();
    assert_eq!(
        h.state.channel_tab,
        crate::channel_settings::ChannelTab::Sound
    );
}

#[test]
fn dragging_the_envelope_peak_sets_the_attack_in_one_step() {
    let mut h = Harness::new(&["channel.add_synth"]);
    h.state.windows.channel_settings = true;
    h.state.channel_tab = crate::channel_settings::ChannelTab::Envelope;
    h.settle();
    let window = window_rect(&h, "channel-settings");
    let handle = (0..40)
        .flat_map(|j| (0..100).map(move |i| (i, j)))
        .map(|(i, j)| window.left_top() + vec2(4.0 + 4.0 * i as f32, 60.0 + 4.0 * j as f32))
        .find(|p| {
            h.frame(vec![Event::PointerMoved(*p)]);
            h.frame(vec![]);
            h.state.hint.as_deref() == Some("Attack: drag across")
        })
        .expect("the attack handle");
    let attack = |h: &Harness| {
        h.session.project().channels[0]
            .params
            .get(musiccraft_project::params::inst::ATTACK)
    };
    let before = attack(&h);
    h.drag(handle, handle + vec2(40.0, 0.0));
    h.settle();
    assert!(attack(&h) > before + 0.01, "{} → {}", before, attack(&h));
    h.session.run_line("edit.undo").unwrap();
    assert!((attack(&h) - before).abs() < 1e-6);
}

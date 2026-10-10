//! Drives the real views headlessly with synthetic input and checks the commands
//! they produce.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use musiccraft_commands::{Registry, Session};
use musiccraft_project::Project;
use musiccraft_project::params::{TICKS_PER_BAR, TICKS_PER_STEP};

use crate::piano_roll::row_of_key;
use crate::{Action, EngineStatus, Frame, History, UiState, show};

struct Harness {
    ctx: egui::Context,
    state: UiState,
    session: Session,
    time: f64,
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
        Self {
            ctx: egui::Context::default(),
            state,
            session,
            time: 0.0,
        }
    }

    /// Runs one frame with `events` and returns the project commands produced
    /// (previews are dropped), applying them to the session.
    fn frame(&mut self, events: Vec<Event>) -> Vec<String> {
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
                ..
            } = self;
            let frame = Frame {
                project: session.project(),
                engine: &engine,
                history: &history,
            };
            let mut output = ctx.run_ui(input, |ui| actions = show(ui, state, &frame));
            // No renderer here: acknowledge texture uploads.
            output.textures_delta.clear();
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
                _ => {}
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
                modifiers: Modifiers::NONE,
            },
        ])
    }

    fn release(&mut self, pos: Pos2, button: PointerButton) -> Vec<String> {
        self.frame(vec![Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        }])
    }

    fn click(&mut self, pos: Pos2, button: PointerButton) -> Vec<String> {
        let mut out = self.press(pos, button);
        out.extend(self.release(pos, button));
        out
    }

    fn drag(&mut self, from: Pos2, to: Pos2) -> Vec<String> {
        let mut out = self.press(from, PointerButton::Primary);
        for i in 1..=6 {
            let t = i as f32 / 6.0;
            out.extend(self.frame(vec![Event::PointerMoved(from + (to - from) * t)]));
        }
        out.extend(self.release(to, PointerButton::Primary));
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
    let from = pos2(x, clip.bottom() - 30.0);
    let to = pos2(x, clip.bottom() - 60.0);
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

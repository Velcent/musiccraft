use musiccraft_project::params::{TICKS_PER_BAR, TICKS_PER_STEP, inst};
use musiccraft_project::{PlayMode, Project};

use crate::{CommandError, Registry, Session};

fn session() -> Session {
    Session::new(Project::default(), Registry::with_builtin())
}

fn run(s: &mut Session, line: &str) {
    if let Err(err) = s.run_line(line) {
        panic!("`{line}` failed: {err}");
    }
}

fn fails(s: &mut Session, line: &str) -> CommandError {
    let before = s.project().clone();
    let err = s.run_line(line).expect_err(line);
    assert_eq!(
        s.project(),
        &before,
        "`{line}` changed the project on error"
    );
    err
}

/// Every undoable command, in an order where each line is valid.
const SCRIPT: &[&str] = &[
    "project.set_tempo 140",
    "project.rename My Song",
    "channel.add_sampler builtin:kick",
    "channel.add_sampler builtin:snare Snare Drum",
    "channel.add_synth Lead",
    "channel.rename 3 Bass",
    "channel.set_volume 1 0.5",
    "channel.set_pan 2 -0.25",
    "channel.set_mute 2 on",
    "channel.toggle_mute 2",
    "channel.solo 1",
    "channel.solo 1",
    "channel.route 1 2",
    "channel.set_sample 1 builtin:clap",
    "channel.set_param 3 cutoff 1200",
    "channel.set_param 3 wave1 3",
    "step.toggle 1 0",
    "step.toggle 1 4",
    "step.set 2 4 on",
    "step.toggle 1 4",
    "note.add 3 0 2 C4 90",
    "note.add 3 2 2 E4",
    "note.add 3 4 1.5 G4",
    "note.move 3 2 E4 3 F4",
    "note.resize 3 0 C4 4",
    "note.set_velocity 3 0 C4 127",
    "note.transpose 3 12",
    "note.quantize 3 2",
    "channel.clone 1",
    "channel.move 4 1",
    "pattern.add Verse",
    "step.toggle 2 8",
    "pattern.clone 1",
    "pattern.rename 2 Chorus",
    "pattern.set_length 2 32",
    "note.clear 3 @1",
    "mixer.add_track Drums",
    "mixer.rename 1 Kick bus",
    "mixer.set_volume 1 0.7",
    "mixer.set_pan 0 0.1",
    "mixer.set_mute 3 on",
    "mixer.toggle_mute 3",
    "mixer.set_solo 2 on",
    "mixer.toggle_solo 2",
    "mixer.add_effect 2 reverb",
    "mixer.add_effect 2 delay",
    "mixer.add_effect 0 compressor",
    "mixer.set_effect_param 2 1 size 0.9",
    "mixer.set_effect_enabled 2 2 off",
    "mixer.toggle_effect 2 2",
    "mixer.move_effect 2 2 1",
    "mixer.remove_effect 2 1",
    "playlist.add_clip 1 1 0",
    "playlist.add_clip 2 1 1",
    "playlist.add_clip 3 2 0 4",
    "playlist.move_clip 2 2 3 4",
    "playlist.resize_clip 1 0 0.5",
    "playlist.set_track_mute 3 on",
    "playlist.toggle_track_mute 3",
    "playlist.remove_clip 1 1.5",
    "mixer.remove_track 2",
    "pattern.remove 1",
    "channel.remove 2",
    "playlist.clear",
    r#"edit.batch "project.set_tempo 90" "channel.set_volume 1 0.3""#,
];

#[test]
fn every_command_undoes_and_redoes_exactly() {
    let mut s = session();
    let mut snapshots = vec![s.project().clone()];
    for line in SCRIPT {
        run(&mut s, line);
        snapshots.push(s.project().clone());
    }
    assert_eq!(s.undo_depth(), SCRIPT.len(), "every line is one undo step");
    for (i, line) in SCRIPT.iter().enumerate().rev() {
        run(&mut s, "edit.undo");
        assert_eq!(s.project(), &snapshots[i], "undo of `{line}` was not exact");
    }
    for (i, line) in SCRIPT.iter().enumerate() {
        run(&mut s, "edit.redo");
        assert_eq!(
            s.project(),
            &snapshots[i + 1],
            "redo of `{line}` was not exact"
        );
    }
    // Inverses recorded on the first pass must still apply after a redo.
    for (i, line) in SCRIPT.iter().enumerate().rev() {
        run(&mut s, "edit.undo");
        assert_eq!(
            s.project(),
            &snapshots[i],
            "second undo of `{line}` was not exact"
        );
    }
}

#[test]
fn steps_are_sixteenth_notes_at_the_step_key() {
    let mut s = session();
    run(&mut s, "channel.add_sampler builtin:hat");
    run(&mut s, "step.toggle 1 3");
    let notes = &s.project().current_pattern().notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].start, 3 * TICKS_PER_STEP);
    assert_eq!(notes[0].length, TICKS_PER_STEP);
    assert_eq!(notes[0].key, 60);
    run(&mut s, "step.toggle 1 3");
    assert!(s.project().current_pattern().notes.is_empty());
}

#[test]
fn note_times_accept_steps_fractions_and_ticks() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    run(&mut s, "note.add 1 0.5 1 60");
    run(&mut s, "note.add 1 7t 5t 61");
    let notes = &s.project().current_pattern().notes;
    assert_eq!((notes[0].start, notes[0].length), (7, 5));
    assert_eq!((notes[1].start, notes[1].length), (12, TICKS_PER_STEP));
}

#[test]
fn undo_after_switching_patterns_edits_the_original_pattern() {
    let mut s = session();
    run(&mut s, "channel.add_sampler builtin:kick");
    run(&mut s, "step.toggle 1 0");
    run(&mut s, "pattern.add");
    s.run_line("pattern.select 2").unwrap();
    // Undo pattern.add, then the step toggle: pattern 1 must lose its note.
    run(&mut s, "edit.undo");
    run(&mut s, "edit.undo");
    assert!(s.project().patterns[0].notes.is_empty());
}

#[test]
fn transport_mode_commands() {
    let mut s = session();
    run(&mut s, "transport.set_mode song");
    assert_eq!(s.project().play_mode, PlayMode::Song);
    run(&mut s, "transport.toggle_mode");
    assert_eq!(s.project().play_mode, PlayMode::Pattern);
    fails(&mut s, "transport.set_mode album");
    assert_eq!(s.undo_depth(), 0);
}

#[test]
fn pattern_navigation_clamps() {
    let mut s = session();
    run(&mut s, "pattern.add");
    run(&mut s, "pattern.add");
    run(&mut s, "pattern.next");
    assert_eq!(s.project().current_pattern, 2);
    run(&mut s, "pattern.select 1");
    run(&mut s, "pattern.prev");
    assert_eq!(s.project().current_pattern, 0);
}

#[test]
fn removing_a_pattern_removes_its_clips() {
    let mut s = session();
    run(&mut s, "pattern.add");
    run(&mut s, "playlist.add_clip 2 1 0");
    run(&mut s, "playlist.add_clip 1 2 0");
    run(&mut s, "pattern.remove 2");
    assert_eq!(s.project().playlist.clips.len(), 1);
    fails(&mut s, "pattern.remove 1");
}

#[test]
fn removing_a_mixer_track_reroutes_channels() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    run(&mut s, "channel.add_synth");
    run(&mut s, "channel.route 1 3");
    run(&mut s, "channel.route 2 5");
    run(&mut s, "mixer.remove_track 3");
    assert_eq!(s.project().channels[0].mixer_track, 0);
    assert_eq!(s.project().channels[1].mixer_track, 4);
    fails(&mut s, "mixer.remove_track 0");
}

#[test]
fn playlist_rejects_overlaps() {
    let mut s = session();
    run(&mut s, "playlist.add_clip 1 1 0 2");
    fails(&mut s, "playlist.add_clip 1 1 1");
    run(&mut s, "playlist.add_clip 1 1 2");
    fails(&mut s, "playlist.move_clip 1 2 1 1");
    fails(&mut s, "playlist.resize_clip 1 0 3");
    assert_eq!(s.project().playlist.length_ticks(), 3 * TICKS_PER_BAR);
}

#[test]
fn errors_leave_the_project_unchanged() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    for line in [
        "channel.add_sampler builtin:cowbell",
        "channel.add_sampler /no/such/file.wav",
        "channel.set_volume 1 2",
        "channel.set_volume 2 0.5",
        "channel.set_volume 0 0.5",
        "channel.set_pan 1 -3",
        "channel.set_param 1 wobble 1",
        "channel.set_param 1 cutoff 5",
        "channel.set_sample 1 builtin:kick",
        "channel.route 1 99",
        "note.add 1 0 0 60",
        "note.add 1 0 1 C99",
        "note.add 1 0 1 60 0",
        "note.add 1 -1 1 60",
        "note.add 1 0 1 60 @7",
        "note.remove 1 0 60",
        "note.transpose 1 200",
        "note.set_all 1 [[0,0,60,100]]",
        "step.toggle 1 99999",
        "pattern.set_length 1 0",
        "pattern.select 4",
        "mixer.add_effect 1 phaser",
        "mixer.remove_effect 1 1",
        "mixer.set_effect_param 0 1 size 1",
        "playlist.add_clip 5 1 0",
        "playlist.add_clip 1 65 0",
        "playlist.remove_clip 1 0",
        "project.set_tempo fast",
        "project.load /no/such/project.mcproj",
    ] {
        fails(&mut s, line);
    }
}

#[test]
fn transpose_and_quantize() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    run(&mut s, "note.add 1 1t 1 60");
    run(&mut s, "note.add 1 13t 1 62");
    run(&mut s, "note.transpose 1 -12");
    run(&mut s, "note.quantize 1 1");
    let notes: Vec<_> = s
        .project()
        .current_pattern()
        .notes
        .iter()
        .map(|n| (n.start, n.key))
        .collect();
    assert_eq!(notes, [(0, 48), (24, 50)]);
}

#[test]
fn set_param_rounds_integer_params() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    run(&mut s, "channel.set_param 1 wave1 2.6");
    assert!((s.project().channels[0].params.get(inst::WAVE1) - 3.0).abs() < f32::EPSILON);
}

#[test]
fn save_and_load_round_trip() {
    let dir = std::env::temp_dir().join(format!("musiccraft-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("song.mcproj");
    let mut s = session();
    run(&mut s, "channel.add_sampler builtin:kick");
    run(&mut s, "step.toggle 1 0");
    run(&mut s, "project.set_tempo 101");
    let line = format!("project.save \"{}\"", path.display());
    run(&mut s, &line);
    let saved = s.project().clone();
    run(&mut s, "project.new");
    assert!(s.project().channels.is_empty());
    run(&mut s, &format!("project.load \"{}\"", path.display()));
    assert_eq!(s.project().channels, saved.channels);
    assert_eq!(s.project().patterns, saved.patterns);
    assert_eq!(s.undo_depth(), 0);
    run(&mut s, "project.rename Again");
    run(&mut s, "project.save");
    assert!(std::fs::read_to_string(&path).unwrap().contains("Again"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn save_without_a_path_needs_one() {
    let mut s = session();
    fails(&mut s, "project.save");
}

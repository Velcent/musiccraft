use musiccraft_commands::{Registry, Session};
use musiccraft_engine::{EngineMessage, engine_pair};
use musiccraft_params::{TICKS_PER_BAR, TICKS_PER_STEP};
use musiccraft_project::Project;

use crate::{Bridge, RenderOptions, build_sequence, render};

fn session() -> Session {
    let mut registry = Registry::with_builtin();
    crate::register(&mut registry).unwrap();
    Session::new(Project::default(), registry)
}

fn run(s: &mut Session, lines: &str) {
    for line in lines.lines().map(str::trim).filter(|l| !l.is_empty()) {
        s.run_line(line).unwrap_or_else(|e| panic!("`{line}`: {e}"));
    }
}

fn rms(data: &[f32]) -> f32 {
    (data.iter().map(|s| s * s).sum::<f32>() / data.len().max(1) as f32).sqrt()
}

#[test]
fn pattern_mode_sequence_uses_channel_positions() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:kick
         channel.add_sampler builtin:hat
         step.toggle 2 4
         step.toggle 1 0",
    );
    let seq = build_sequence(s.project());
    assert_eq!(seq.length(), u64::from(16 * TICKS_PER_STEP));
    let events: Vec<_> = seq.events().iter().map(|e| (e.start, e.channel)).collect();
    assert_eq!(events, [(0, 0), (4 * u64::from(TICKS_PER_STEP), 1)]);
}

#[test]
fn song_mode_repeats_patterns_inside_long_clips_and_skips_muted_tracks() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:kick
         step.toggle 1 0
         playlist.add_clip 1 1 0 3
         playlist.add_clip 1 2 4
         playlist.set_track_mute 2 on
         transport.set_mode song",
    );
    let seq = build_sequence(s.project());
    let bar = u64::from(TICKS_PER_BAR);
    assert_eq!(seq.length(), 5 * bar);
    let starts: Vec<_> = seq.events().iter().map(|e| e.start).collect();
    assert_eq!(starts, [0, bar, 2 * bar]);
}

#[test]
fn clips_truncate_notes_at_their_end() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_synth
         note.add 1 0 16 C5
         playlist.add_clip 1 1 0 0.5
         transport.set_mode song",
    );
    let seq = build_sequence(s.project());
    assert_eq!(seq.events()[0].length, u64::from(TICKS_PER_BAR / 2));
}

#[test]
fn sync_sends_only_what_changed() {
    let mut s = session();
    run(&mut s, "channel.add_sampler builtin:kick");
    let (mut engine, mut handle) = engine_pair(44_100);
    let mut bridge = Bridge::new(44_100);
    let first = bridge.sync(s.project(), &mut handle);
    assert!(first.rebuilt_graph && first.new_sequence);
    assert!(first.warnings.is_empty());

    let idle = bridge.sync(s.project(), &mut handle);
    assert_eq!(idle.messages, 0);

    run(&mut s, "channel.set_volume 1 0.5");
    let volume = bridge.sync(s.project(), &mut handle);
    assert_eq!(volume.messages, 1);
    assert!(!volume.rebuilt_graph);

    run(&mut s, "step.toggle 1 0");
    let step = bridge.sync(s.project(), &mut handle);
    assert!(step.new_sequence && !step.rebuilt_graph);

    run(&mut s, "mixer.add_effect 0 reverb");
    assert!(bridge.sync(s.project(), &mut handle).rebuilt_graph);

    run(&mut s, "transport.play");
    let play = bridge.sync(s.project(), &mut handle);
    assert_eq!(play.messages, 1);
    let mut out = vec![0.0; 2048];
    engine.process(&mut out, 2);
    assert!(engine.is_playing());
    // Replaced graphs and sequences come back for dropping on this thread.
    let _ = bridge.sync(s.project(), &mut handle);
}

#[test]
fn missing_samples_warn_and_play_silence() {
    let mut s = session();
    run(&mut s, "channel.add_synth");
    let mut project = s.project().clone();
    project.channels[0].instrument = musiccraft_project::Instrument::Sampler {
        sample: "/no/such/file.wav".into(),
    };
    let (_engine, mut handle) = engine_pair(44_100);
    let report = Bridge::new(44_100).sync(&project, &mut handle);
    assert_eq!(report.warnings.len(), 1);
}

#[test]
fn rendered_drum_pattern_has_hits_where_steps_are() {
    let mut s = session();
    run(
        &mut s,
        "project.set_tempo 120
         channel.add_sampler builtin:kick
         step.toggle 1 0
         step.toggle 1 8",
    );
    let (left, right, warnings) = render(
        s.project(),
        RenderOptions {
            sample_rate: 48_000,
            loops: 1,
            max_tail_seconds: 0.0,
        },
    );
    assert!(warnings.is_empty());
    // One bar at 120 BPM = 2 s.
    assert_eq!(left.len(), 96_000);
    assert_eq!(left, right, "centred mono sample");
    let before_second_hit = &left[47_000..47_990];
    let second_hit = &left[48_000..49_000];
    assert!(rms(second_hit) > 10.0 * rms(before_second_hit));
    assert!(
        left[..200].iter().any(|s| s.abs() > 0.01),
        "first hit sounds at once"
    );
}

#[test]
fn render_includes_tail_then_stops_at_silence() {
    let mut s = session();
    run(
        &mut s,
        "project.set_tempo 120
         channel.add_sampler builtin:crash
         step.toggle 1 15",
    );
    let (left, _, _) = render(s.project(), RenderOptions::default());
    let body = 2 * 44_100;
    assert!(left.len() > body + 22_050, "tail kept");
    assert!(left.len() < body + 10 * 44_100, "stopped at silence");
}

#[test]
fn render_is_deterministic() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_synth
         note.add 1 0 4 C5
         note.add 1 4 4 E5
         mixer.add_effect 0 delay
         mixer.add_effect 0 reverb",
    );
    let options = RenderOptions {
        max_tail_seconds: 1.0,
        ..RenderOptions::default()
    };
    assert_eq!(
        render(s.project(), options).0,
        render(s.project(), options).0
    );
}

#[test]
fn mixer_mute_and_solo_silence_tracks() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_synth
         note.add 1 0 16 A5
         channel.route 1 1
         mixer.set_solo 2 on",
    );
    let options = RenderOptions {
        max_tail_seconds: 0.0,
        ..RenderOptions::default()
    };
    let (soloed_out, _, _) = render(s.project(), options);
    assert!(
        rms(&soloed_out) < 1e-6,
        "insert 1 is silenced by insert 2's solo"
    );
    run(&mut s, "mixer.set_solo 2 off");
    let (audible, _, _) = render(s.project(), options);
    assert!(rms(&audible) > 0.01);
}

#[test]
fn export_command_writes_a_wav() {
    let dir = std::env::temp_dir().join(format!("musiccraft-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.wav");
    let mut s = session();
    run(&mut s, "channel.add_sampler builtin:snare\nstep.toggle 1 4");
    let line = format!("project.export_wav \"{}\" 24 2 48000", path.display());
    let outcome = s.run_line(&line).unwrap();
    assert!(outcome.message.starts_with("exported"));
    assert!(outcome.inverse.is_none());
    let buffer = musiccraft_samples::load_wav(&path).unwrap();
    assert!((buffer.sample_rate() - 48_000.0).abs() < f64::EPSILON);
    assert!(buffer.peak() > 0.1);
    assert!(s.run_line("project.export_wav out.wav 12").is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stop_message_rewinds_engine() {
    let (mut engine, _handle) = engine_pair(44_100);
    engine.apply(EngineMessage::Play);
    let mut out = vec![0.0; 1024];
    engine.process(&mut out, 2);
    engine.apply(EngineMessage::Stop);
    assert!(engine.position_ticks().abs() < f64::EPSILON);
}

#[test]
fn swing_delays_offbeat_sixteenths_only() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:hat
         step.toggle 1 0
         step.toggle 1 1
         step.toggle 1 2
         step.toggle 1 3
         pattern.set_swing 1 50",
    );
    let starts: Vec<u64> = build_sequence(s.project())
        .events()
        .iter()
        .map(|e| e.start)
        .collect();
    assert_eq!(starts, [0, 24 + 6, 48, 72 + 6]);
}

#[test]
fn exports_never_contain_the_metronome() {
    let mut s = session();
    run(&mut s, "transport.toggle_metronome");
    assert!(s.project().metronome);
    let (left, _, _) = render(
        s.project(),
        RenderOptions {
            max_tail_seconds: 0.0,
            ..RenderOptions::default()
        },
    );
    assert!(left.iter().all(|x| x.abs() < 1e-6));
}

#[test]
fn stems_split_by_mixer_track_and_line_up() {
    let dir = std::env::temp_dir().join(format!("musiccraft-stems-{}", std::process::id()));
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:kick
         channel.add_sampler builtin:crash
         channel.add_synth
         step.toggle 1 0
         step.toggle 2 0
         note.add 3 0 4 C5
         channel.route 1 1
         channel.route 2 1
         channel.set_mute 3 off
         mixer.rename 1 Drums/Perc",
    );
    let line = format!("project.export_stems \"{}\"", dir.display());
    let outcome = s.run_line(&line).unwrap();
    assert!(outcome.message.contains("2 stems"), "{}", outcome.message);
    let drums = musiccraft_samples::load_wav(&dir.join("01 Drums_Perc.wav")).unwrap();
    let master = musiccraft_samples::load_wav(&dir.join("00 Master.wav")).unwrap();
    assert_eq!(
        drums.len(),
        master.len(),
        "stems are padded to the same length"
    );
    assert!(drums.peak() > 0.1 && master.peak() > 0.01);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn soloing_a_source_keeps_its_bus_audible() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_synth
         note.add 1 0 16 A5
         channel.route 1 1
         mixer.route_to_master 1 off
         mixer.set_send 1 3 1
         mixer.set_solo 1 on",
    );
    let options = RenderOptions {
        max_tail_seconds: 0.0,
        ..RenderOptions::default()
    };
    let (out, _, _) = render(s.project(), options);
    assert!(
        rms(&out) > 0.01,
        "insert 1 reaches the master only through bus 3"
    );
    run(&mut s, "mixer.set_solo 1 off\nmixer.set_solo 2 on");
    let (out, _, _) = render(s.project(), options);
    assert!(rms(&out) < 1e-6);
}

#[test]
fn automation_clip_fades_the_master_in_song_mode() {
    let mut s = session();
    run(
        &mut s,
        "project.set_tempo 120
         channel.add_synth
         note.add 1 0 16 A5
         playlist.add_clip 1 1 0 4
         automation.create mixer:0:volume
         automation.set_points 1 [[0,0.75],[1536,0.0]]
         playlist.add_automation_clip 1 2 0 4
         transport.set_mode song",
    );
    let seq = build_sequence(s.project());
    assert_eq!(seq.automation().len(), 1);
    let options = RenderOptions {
        max_tail_seconds: 0.0,
        ..RenderOptions::default()
    };
    let (out, _, _) = render(s.project(), options);
    let second = 44_100;
    let early = rms(&out[second / 2..second + second / 2]);
    let late = rms(&out[7 * second + second / 2..8 * second - 100]);
    assert!(early > 0.05, "early {early}");
    assert!(late < early / 5.0, "faded: early {early}, late {late}");
}

#[test]
fn audio_clips_play_their_sample_unpitched() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:crash
         channel.set_param 1 one_shot 0
         channel.set_param 1 root_key 48
         playlist.add_audio_clip 1 1 1 1
         transport.set_mode song",
    );
    let seq = build_sequence(s.project());
    let event = seq.events()[0];
    assert_eq!(
        (event.start, event.length, event.key),
        (u64::from(TICKS_PER_BAR), u64::from(TICKS_PER_BAR), 48)
    );
}

#[test]
fn song_loop_region_limits_playback_but_not_export() {
    let mut s = session();
    run(
        &mut s,
        "channel.add_sampler builtin:kick
         step.toggle 1 0
         playlist.add_clip 1 1 0 8
         transport.set_mode song
         transport.set_loop 2 4",
    );
    let seq = build_sequence(s.project());
    let bar = u64::from(TICKS_PER_BAR);
    assert_eq!((seq.loop_start(), seq.length()), (2 * bar, 4 * bar));
    let full = crate::build_full_sequence(s.project());
    assert_eq!(full.length(), 8 * bar);
}

#[test]
fn sidechained_compressor_ducks_on_each_kick() {
    let mut s = session();
    run(
        &mut s,
        "project.set_tempo 120
         channel.add_sampler builtin:kick
         channel.add_synth Pad
         step.toggle 1 0
         step.toggle 1 8
         note.add 2 0 16 A4
         channel.route 1 1
         channel.route 2 2
         mixer.route_to_master 1 off
         mixer.add_effect 2 compressor
         mixer.set_effect_param 2 1 threshold -40
         mixer.set_effect_param 2 1 ratio 20
         mixer.set_effect_param 2 1 sidechain 1",
    );
    let options = RenderOptions {
        max_tail_seconds: 0.0,
        ..RenderOptions::default()
    };
    let ducked = render(s.project(), options).0;
    // The pad holds steady before the second kick (at 1 s) and drops under it.
    let before = rms(&ducked[44_100 - 8_000..44_100 - 500]);
    let under = rms(&ducked[44_100 + 500..44_100 + 4_000]);
    assert!(
        under < 0.5 * before,
        "pad ducks under the kick: before {before}, under {under}"
    );
}

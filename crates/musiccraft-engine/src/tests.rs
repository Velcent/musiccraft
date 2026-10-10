use std::sync::Arc;

use musiccraft_dsp::{Effect, SampleBuffer};
use musiccraft_params::{EffectKind, PPQ, inst};

use super::*;

const SR: u32 = 48_000;
/// Frames per beat at 120 BPM and 48 kHz.
const BEAT: usize = 24_000;

fn impulse_params() -> ChannelParams {
    let mut params = ChannelParams {
        volume: 1.0,
        ..ChannelParams::default()
    };
    params.instrument[inst::ATTACK] = 0.0;
    params.instrument[inst::DECAY] = 0.0;
    params.instrument[inst::SUSTAIN] = 1.0;
    params
}

/// An engine at 120 BPM with one sampler channel playing a single-frame impulse.
fn impulse_engine() -> (Engine, EngineHandle) {
    let (mut engine, handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let impulse = Arc::new(SampleBuffer::mono(vec![1.0], f64::from(SR)));
    graph.add_channel(Instrument::Sampler(impulse), impulse_params());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetTempo(120.0));
    (engine, handle)
}

fn note(start_beats: u64, channel: u16) -> NoteEvent {
    NoteEvent {
        start: start_beats * u64::from(PPQ),
        length: u64::from(PPQ) / 4,
        channel,
        key: 60,
        velocity: 1.0,
    }
}

fn render_mono(engine: &mut Engine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames];
    // Use an odd device buffer size to exercise block splitting.
    for chunk in out.chunks_mut(333) {
        engine.process(chunk, 1);
    }
    out
}

fn render_stereo(engine: &mut Engine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    engine.process(&mut out, 2);
    out
}

fn nonzero_frames(samples: &[f32]) -> Vec<usize> {
    samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.abs() > 1e-6)
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn stopped_engine_is_silent_and_does_not_advance() {
    let (mut engine, _handle) = impulse_engine();
    let mut buffer = vec![1.0_f32; 512];
    engine.process(&mut buffer, 2);
    assert!(buffer.iter().all(|s| *s == 0.0));
    assert_eq!(engine.frames_played(), 0);
    assert!(engine.position_beats().abs() < f64::EPSILON);
}

#[test]
fn one_second_at_120_bpm_is_two_beats() {
    let (mut engine, _handle) = impulse_engine();
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![],
        16 * u64::from(PPQ),
    ))));
    engine.apply(EngineMessage::Play);
    render_stereo(&mut engine, SR as usize);
    assert_eq!(engine.frames_played(), u64::from(SR));
    assert!((engine.position_beats() - 2.0).abs() < 1e-9);
}

#[test]
fn tempo_change_does_not_make_position_jump() {
    let (mut engine, _handle) = impulse_engine();
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![],
        16 * u64::from(PPQ),
    ))));
    engine.apply(EngineMessage::Play);
    render_stereo(&mut engine, SR as usize);
    let before = engine.position_beats();
    engine.apply(EngineMessage::SetTempo(60.0));
    assert!((engine.position_beats() - before).abs() < f64::EPSILON);
    render_stereo(&mut engine, SR as usize);
    assert!((engine.position_beats() - (before + 1.0)).abs() < 1e-9);
}

#[test]
fn stop_rewinds_and_pause_keeps_position() {
    let (mut engine, _handle) = impulse_engine();
    engine.apply(EngineMessage::Play);
    render_stereo(&mut engine, 1_000);
    engine.apply(EngineMessage::Pause);
    let paused = engine.position_ticks();
    assert!(paused > 0.0);
    render_stereo(&mut engine, 1_000);
    assert!((engine.position_ticks() - paused).abs() < f64::EPSILON);
    engine.apply(EngineMessage::Stop);
    assert!(engine.position_ticks().abs() < f64::EPSILON);
}

#[test]
fn invalid_tempo_messages_are_ignored() {
    let (mut engine, _handle) = impulse_engine();
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        engine.apply(EngineMessage::SetTempo(bad));
    }
    assert!((engine.tempo_bpm() - 120.0).abs() < f64::EPSILON);
}

#[test]
fn zero_channels_does_not_panic() {
    let (mut engine, _handle) = impulse_engine();
    engine.apply(EngineMessage::Play);
    let mut buffer = vec![0.0_f32; 64];
    engine.process(&mut buffer, 0);
    assert_eq!(engine.frames_played(), 0);
}

#[test]
fn notes_start_on_the_exact_frame() {
    let (mut engine, _handle) = impulse_engine();
    let seq = Sequence::new(vec![note(1, 0), note(3, 0)], 4 * u64::from(PPQ));
    engine.apply(EngineMessage::SetSequence(Box::new(seq)));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 4 * BEAT);
    assert_eq!(nonzero_frames(&out), [BEAT, 3 * BEAT]);
    assert!((out[BEAT] - 1.0).abs() < 1e-6);
}

#[test]
fn sequence_loops_at_its_length() {
    let (mut engine, _handle) = impulse_engine();
    let seq = Sequence::new(vec![note(0, 0)], u64::from(PPQ));
    engine.apply(EngineMessage::SetSequence(Box::new(seq)));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 3 * BEAT + 10);
    assert_eq!(nonzero_frames(&out), [0, BEAT, 2 * BEAT, 3 * BEAT]);
}

#[test]
fn replaced_values_come_back_as_garbage() {
    let (mut engine, mut handle) = impulse_engine();
    handle
        .send(EngineMessage::SetGraph(Box::default()))
        .unwrap();
    handle
        .send(EngineMessage::SetSequence(Box::default()))
        .unwrap();
    handle
        .send(EngineMessage::SetSequence(Box::default()))
        .unwrap();
    render_stereo(&mut engine, 16);
    // Old graph from impulse_engine + first sequence replaced by the second.
    assert_eq!(handle.collect_garbage(), 2);
}

#[test]
fn full_garbage_queue_defers_swaps_instead_of_dropping() {
    let (mut engine, mut handle) = impulse_engine();
    for _ in 0..GARBAGE_CAPACITY + 10 {
        handle
            .send(EngineMessage::SetSequence(Box::default()))
            .unwrap();
    }
    render_stereo(&mut engine, 16);
    // The first swap replaces nothing; then the queue fills up and swaps wait.
    assert_eq!(handle.collect_garbage(), GARBAGE_CAPACITY);
    render_stereo(&mut engine, 16);
    assert_eq!(handle.collect_garbage(), 9);
}

#[test]
fn live_notes_hold_until_released() {
    let (mut engine, mut handle) = engine_pair(SR);
    let mut graph = Graph::new();
    graph.add_channel(Instrument::Synth, ChannelParams::default());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    let mut live = handle.take_live_sender().unwrap();
    assert!(handle.take_live_sender().is_none());
    live.send(LiveEvent::NoteOn {
        channel: 0,
        key: 69,
        velocity: 1.0,
    });
    let out = render_stereo(&mut engine, SR as usize);
    assert_eq!(engine.active_voices(), 1);
    assert!(out[SR as usize..].iter().any(|s| s.abs() > 0.01));
    live.send(LiveEvent::NoteOff {
        channel: 0,
        key: 69,
    });
    render_stereo(&mut engine, SR as usize);
    assert_eq!(engine.active_voices(), 0);
}

#[test]
fn muted_channel_is_silent() {
    let (mut engine, _handle) = impulse_engine();
    let params = ChannelParams {
        mute: true,
        ..impulse_params()
    };
    engine.apply(EngineMessage::Channel { channel: 0, params });
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(0, 0)],
        384,
    ))));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 1_000);
    assert!(nonzero_frames(&out).is_empty());
}

#[test]
fn channel_routes_through_insert_track_to_master() {
    let (mut engine, handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let impulse = Arc::new(SampleBuffer::mono(vec![1.0], f64::from(SR)));
    let params = ChannelParams {
        track: 1,
        ..impulse_params()
    };
    graph.add_channel(Instrument::Sampler(impulse), params);
    graph.add_track(
        TrackParams {
            volume: 0.5,
            pan: 1.0,
            ..TrackParams::default()
        },
        Vec::new(),
    );
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetTempo(120.0));
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(0, 0)],
        384,
    ))));
    engine.apply(EngineMessage::Play);
    let out = render_stereo(&mut engine, 64);
    assert!(out[0].abs() < 1e-6, "panned hard right, left is silent");
    assert!((out[1] - 0.5).abs() < 1e-6);
    let meters = handle.meters();
    assert!((meters.take_peak(1).1 - 0.5).abs() < 1e-6);
    assert!((meters.take_peak(0).1 - 0.5).abs() < 1e-6);
}

#[test]
fn bypassed_and_enabled_effects() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let dc = Arc::new(SampleBuffer::mono(vec![1.0; 4_800], f64::from(SR)));
    graph.add_channel(Instrument::Sampler(dc), impulse_params());
    let mut values = EffectKind::Distortion.defaults();
    values[2] = -6.0206; // output gain 0.5
    values[1] = 0.0; // dry only
    let fx = EffectParams {
        enabled: true,
        values,
    };
    graph.set_master(
        TrackParams::default(),
        vec![(Effect::new(EffectKind::Distortion, SR as f32), fx)],
    );
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::NoteOn {
        channel: 0,
        key: 60,
        velocity: 1.0,
    });
    let out = render_mono(&mut engine, 100);
    assert!((out[50] - 0.5).abs() < 1e-3, "got {}", out[50]);
    engine.apply(EngineMessage::Effect {
        track: 0,
        slot: 0,
        params: EffectParams {
            enabled: false,
            values,
        },
    });
    let out = render_mono(&mut engine, 100);
    assert!((out[50] - 1.0).abs() < 1e-3, "got {}", out[50]);
}

#[test]
fn voice_pool_steals_when_full() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    graph.add_channel(Instrument::Synth, ChannelParams::default());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    for key in 0..(MAX_VOICES as u8 + 10) {
        engine.apply(EngineMessage::NoteOn {
            channel: 0,
            key: 30 + key,
            velocity: 0.5,
        });
    }
    assert_eq!(engine.active_voices(), MAX_VOICES);
    let out = render_stereo(&mut engine, 4_800);
    assert!(out.iter().all(|s| s.is_finite()));
    engine.apply(EngineMessage::AllNotesOff);
    render_stereo(&mut engine, SR as usize);
    assert_eq!(engine.active_voices(), 0);
}

#[test]
fn one_shot_samples_ignore_note_length() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let long = Arc::new(SampleBuffer::mono(vec![0.5; BEAT], f64::from(SR)));
    graph.add_channel(Instrument::Sampler(long), impulse_params());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetTempo(120.0));
    // A 1/16 note on a one-beat sample: the whole sample still plays.
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(0, 0)],
        8 * 96,
    ))));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 2 * BEAT);
    assert!((out[BEAT - 10] - 0.5).abs() < 1e-6);
    assert!(out[BEAT + 10].abs() < 1e-6);
}

#[test]
fn seek_moves_playhead_within_loop() {
    let (mut engine, _handle) = impulse_engine();
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(2, 0)],
        384,
    ))));
    engine.apply(EngineMessage::Seek(f64::from(PPQ) * 1.5));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, BEAT);
    assert_eq!(nonzero_frames(&out), [BEAT / 2]);
    engine.apply(EngineMessage::Seek(f64::from(PPQ) * 9.0));
    assert!(
        (engine.position_beats() - 1.0).abs() < 1e-9,
        "wraps into the loop"
    );
}

#[test]
fn metronome_clicks_on_every_beat_and_accents_bars() {
    let (mut engine, _handle) = engine_pair(SR);
    engine.apply(EngineMessage::SetGraph(Box::default()));
    engine.apply(EngineMessage::SetTempo(120.0));
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![],
        8 * 96,
    ))));
    engine.apply(EngineMessage::SetMetronome(true));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 5 * BEAT);
    let onsets: Vec<usize> = (0..5)
        .map(|beat| beat * BEAT)
        .filter(|start| out[start + 1..start + 200].iter().any(|s| s.abs() > 0.05))
        .collect();
    assert_eq!(onsets, [0, BEAT, 2 * BEAT, 3 * BEAT, 4 * BEAT]);
    assert!(
        out[BEAT / 2..BEAT - 10].iter().all(|s| s.abs() < 1e-3),
        "silent between clicks"
    );
    engine.apply(EngineMessage::SetMetronome(false));
    let quiet = render_mono(&mut engine, 2 * BEAT);
    assert!(quiet.iter().all(|s| s.abs() < 1e-3));
}

#[test]
fn sends_feed_higher_inserts_and_master_routing_can_be_cut() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let impulse = Arc::new(SampleBuffer::mono(vec![1.0], f64::from(SR)));
    let params = ChannelParams {
        track: 1,
        ..impulse_params()
    };
    graph.add_channel(Instrument::Sampler(impulse), params);
    // Insert 1 sends half its signal to insert 2 and does not feed the master.
    let mut source = TrackParams {
        to_master: false,
        send_count: 1,
        ..TrackParams::default()
    };
    source.sends[0] = SendParams {
        target: 2,
        level: 0.5,
    };
    graph.add_track(source, Vec::new());
    graph.add_track(TrackParams::default(), Vec::new());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetTempo(120.0));
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(0, 0)],
        384,
    ))));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 16);
    assert!(
        (out[0] - 0.5).abs() < 1e-6,
        "only the send reaches the master: {}",
        out[0]
    );
    // Re-enable the master route: direct 1.0 plus send 0.5.
    let both = TrackParams {
        to_master: true,
        ..source
    };
    engine.apply(EngineMessage::Track {
        track: 1,
        params: both,
    });
    engine.apply(EngineMessage::Stop);
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 16);
    assert!((out[0] - 1.5).abs() < 1e-6, "got {}", out[0]);
}

#[test]
fn sends_to_lower_tracks_are_ignored() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let impulse = Arc::new(SampleBuffer::mono(vec![1.0], f64::from(SR)));
    let params = ChannelParams {
        track: 2,
        ..impulse_params()
    };
    graph.add_channel(Instrument::Sampler(impulse), params);
    graph.add_track(TrackParams::default(), Vec::new());
    let mut backwards = TrackParams {
        send_count: 1,
        ..TrackParams::default()
    };
    backwards.sends[0] = SendParams {
        target: 1,
        level: 1.0,
    };
    graph.add_track(backwards, Vec::new());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetSequence(Box::new(Sequence::new(
        vec![note(0, 0)],
        384,
    ))));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 16);
    assert!((out[0] - 1.0).abs() < 1e-6);
}

#[test]
fn preview_plays_on_master_and_is_returned_as_garbage() {
    let (mut engine, mut handle) = engine_pair(SR);
    engine.apply(EngineMessage::SetGraph(Box::default()));
    let sample = Arc::new(SampleBuffer::mono(vec![0.5; 100], f64::from(SR)));
    handle
        .send(EngineMessage::PlayPreview(Box::new(Preview::new(
            Arc::clone(&sample),
            1.0,
        ))))
        .unwrap();
    let out = render_mono(&mut engine, 200);
    assert!((out[50] - 0.5).abs() < 1e-6);
    assert!(out[150].abs() < 1e-6, "stops at the end of the sample");
    handle
        .send(EngineMessage::PlayPreview(Box::new(Preview::new(
            sample, 1.0,
        ))))
        .unwrap();
    handle.send(EngineMessage::StopPreview).unwrap();
    let out = render_mono(&mut engine, 50);
    assert!(out.iter().all(|s| s.abs() < 1e-6));
    assert_eq!(handle.collect_garbage(), 1, "the replaced first preview");
}

#[test]
fn automation_moves_a_track_volume_during_playback() {
    let (mut engine, _handle) = engine_pair(SR);
    let mut graph = Graph::new();
    let dc = Arc::new(SampleBuffer::mono(vec![1.0; 4 * BEAT], f64::from(SR)));
    graph.add_channel(Instrument::Sampler(dc), impulse_params());
    engine.apply(EngineMessage::SetGraph(Box::new(graph)));
    engine.apply(EngineMessage::SetTempo(120.0));
    let lane = AutomationLane {
        target: AutomationTarget::TrackVolume(0),
        points: vec![(0, 1.0), (u64::from(PPQ) * 2, 0.0)],
    };
    let sequence = Sequence::new(vec![note(0, 0)], 4 * u64::from(PPQ)).with_automation(vec![lane]);
    engine.apply(EngineMessage::SetSequence(Box::new(sequence)));
    engine.apply(EngineMessage::Play);
    let out = render_mono(&mut engine, 3 * BEAT);
    // Halfway through the ramp (one beat in) the master is at about half.
    assert!((out[BEAT] - 0.5).abs() < 0.02, "got {}", out[BEAT]);
    assert!(out[2 * BEAT + 500].abs() < 0.01, "ramp reached zero");
}

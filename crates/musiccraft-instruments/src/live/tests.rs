use super::*;
use crate::default_params;

/// 25 frames per tick: 120 BPM, 96 ticks a beat, 4800 frames a second.
const RATE: f64 = 4800.0;
const TICK: u64 = 25;
const BLOCK: u32 = 64;

fn timing(playing: bool, frame: u64) -> Timing {
    Timing {
        tempo: 120.0,
        sample_rate: RATE,
        beats: playing.then(|| frame as f64 / TICK as f64 / f64::from(PPQ)),
    }
}

fn set(params: &mut Params, index: usize, value: f32) {
    params[index] = value;
}

/// Runs `inputs` (absolute frame, note) through block after block until
/// `frames`, returning every note for the voices with its absolute frame.
fn play(
    params: &Params,
    swing: f32,
    playing: bool,
    inputs: &[(u64, Note)],
    frames: u64,
) -> Vec<(u64, Note)> {
    let mut f = NoteFunctions::new();
    let mut out = Vec::new();
    let mut start = 0;
    while start < frames {
        let block: Vec<(u32, Note)> = inputs
            .iter()
            .filter(|(at, _)| (start..start + u64::from(BLOCK)).contains(at))
            .map(|(at, n)| ((at - start) as u32, *n))
            .collect();
        f.run(
            params,
            swing,
            timing(playing, start),
            BLOCK,
            &block,
            &mut |frame, note| {
                out.push((start + u64::from(frame), note));
            },
        );
        start += u64::from(BLOCK);
    }
    out
}

fn on(key: u8) -> Note {
    Note::On { key, velocity: 0.8 }
}

fn off(key: u8) -> Note {
    Note::Off { key }
}

#[test]
fn plain_notes_pass_through_on_their_frame() {
    let out = play(
        &default_params(),
        0.0,
        true,
        &[(10, on(60)), (300, off(60))],
        1000,
    );
    assert_eq!(out, [(10, on(60)), (300, off(60))]);
}

#[test]
fn gate_fixes_the_length_and_shift_delays() {
    let mut p = default_params();
    set(&mut p, inst::GATE, 12.0);
    set(&mut p, inst::TIME_SHIFT, 4.0);
    let out = play(&p, 0.0, false, &[(10, on(60)), (3000, off(60))], 5000);
    let start = 10 + 4 * TICK;
    assert_eq!(out, [(start, on(60)), (start + 12 * TICK, off(60))]);
    assert_eq!(latency(&p, 120.0, RATE), 0);
    set(&mut p, inst::TIME_SHIFT, -8.0);
    assert_eq!(latency(&p, 120.0, RATE), 8 * TICK as u32);
    let out = play(&p, 0.0, false, &[(10, on(60))], 1000);
    assert_eq!(out.first(), Some(&(10, on(60))), "played at once");
}

#[test]
fn echoes_repeat_fade_and_climb() {
    let mut p = default_params();
    set(&mut p, inst::ECHO_COUNT, 2.0);
    set(&mut p, inst::ECHO_TIME, 24.0);
    set(&mut p, inst::ECHO_FEED, 0.5);
    set(&mut p, inst::ECHO_PITCH, 12.0);
    let out = play(&p, 0.0, false, &[(0, on(60)), (100, off(60))], 3000);
    let step = 24 * TICK;
    assert_eq!(
        out,
        [
            (0, on(60)),
            (100, off(60)),
            (
                step,
                Note::On {
                    key: 72,
                    velocity: 0.4
                }
            ),
            (step + 100, off(72)),
            (
                2 * step,
                Note::On {
                    key: 84,
                    velocity: 0.2
                }
            ),
            (2 * step + 100, off(84)),
        ]
    );
}

#[test]
fn swing_moves_offbeat_sixteenths_while_playing() {
    let step = u64::from(TICKS_PER_STEP) * TICK;
    let inputs = [
        (0, on(60)),
        (100, off(60)),
        (step, on(62)),
        (step + 100, off(62)),
    ];
    let mut p = default_params();
    set(&mut p, inst::SWING_MIX, 1.0);
    let out = play(&p, 1.0, true, &inputs, 4000);
    let delay = 12 * TICK;
    assert_eq!(
        out,
        [
            (0, on(60)),
            (100, off(60)),
            (step + delay, on(62)),
            (step + 100 + delay, off(62)),
        ]
    );
    // Truncate keeps the end; a stopped host does not swing.
    set(&mut p, inst::SWING_TRUNCATE, 1.0);
    let late = [(step, on(62)), (step + delay + 100, off(62))];
    let out = play(&p, 1.0, true, &late, 4000);
    assert_eq!(out, [(step + delay, on(62)), (step + delay + 100, off(62))]);
    // A note shorter than the swing keeps one tick.
    let out = play(&p, 1.0, true, &inputs, 4000);
    assert_eq!(
        out[2..],
        [(step + delay, on(62)), (step + delay + TICK, off(62))]
    );
    assert_eq!(play(&p, 1.0, false, &inputs, 4000), inputs);
}

#[test]
fn the_arpeggiator_steps_through_held_keys_on_the_grid() {
    let mut p = default_params();
    set(&mut p, inst::ARP_MODE, 1.0);
    set(&mut p, inst::ARP_RATE, 1.0); // 1/16: 24 ticks
    set(&mut p, inst::ARP_GATE, 0.5);
    let step = 24 * TICK;
    // A chord pressed mid-step plays at once, then on every boundary.
    let chord = [(100, on(60)), (100, on(64)), (100, on(67))];
    let mut inputs = chord.to_vec();
    inputs.extend([
        (3 * step + 50, off(60)),
        (3 * step + 50, off(64)),
        (3 * step + 50, off(67)),
    ]);
    let out = play(&p, 0.0, true, &inputs, 5 * step);
    let ons: Vec<(u64, u8)> = out
        .iter()
        .filter_map(|(t, n)| match n {
            Note::On { key, .. } => Some((*t, *key)),
            Note::Off { .. } => None,
        })
        .collect();
    assert_eq!(ons, [(100, 60), (step, 64), (2 * step, 67), (3 * step, 60)]);
    // Each note lasts half a step, the first only to the boundary's half.
    assert!(out.contains(&(step + 12 * TICK, off(64))));
    assert!(out.contains(&(100 + 12 * TICK, off(60))));
    // Up-down and two octaves, and random repeats itself.
    set(&mut p, inst::ARP_MODE, 4.0);
    assert_eq!(
        play(&p, 0.0, true, &inputs, 5 * step),
        play(&p, 0.0, true, &inputs, 5 * step)
    );
}

#[test]
fn the_arpeggiator_restarts_after_a_silence() {
    let mut p = default_params();
    set(&mut p, inst::ARP_MODE, 1.0);
    set(&mut p, inst::ARP_RATE, 1.0);
    let step = 24 * TICK;
    let inputs = [
        (0, on(60)),
        (0, on(64)),
        (step - 10, off(60)),
        (step - 10, off(64)),
        (4 * step, on(60)),
        (4 * step, on(64)),
        (5 * step - 10, off(60)),
        (5 * step - 10, off(64)),
    ];
    let keys: Vec<u8> = play(&p, 0.0, false, &inputs, 6 * step)
        .iter()
        .filter_map(|(_, n)| match n {
            Note::On { key, .. } => Some(*key),
            Note::Off { .. } => None,
        })
        .collect();
    assert_eq!(keys, [60, 60]);
}

fn render(poly: &mut Poly, params: &Params) {
    let mut l = vec![0.0; 64];
    let mut r = vec![0.0; 64];
    poly.render(&Instrument::Synth, params, &mut l, &mut r);
}

#[test]
fn voices_follow_mono_polyphony_and_self_choke() {
    let mut p = default_params();
    let mut poly = Poly::new(48_000.0);
    for key in [60, 64, 67] {
        poly.note_on(key, 0.8, &Instrument::Synth, &p);
    }
    render(&mut poly, &p);
    assert_eq!(poly.active(), 3);
    set(&mut p, inst::POLY_MAX, 2.0);
    poly.note_on(72, 0.8, &Instrument::Synth, &p);
    for _ in 0..10 {
        render(&mut poly, &p);
    }
    assert_eq!(poly.active(), 2, "the two oldest were cut");
    set(&mut p, inst::POLY_MAX, 0.0);
    set(&mut p, inst::MONO, 1.0);
    poly.note_on(48, 0.8, &Instrument::Synth, &p);
    for _ in 0..10 {
        render(&mut poly, &p);
    }
    assert_eq!(poly.active(), 1);
    set(&mut p, inst::MONO, 0.0);
    set(&mut p, inst::CUT_GROUP, 3.0);
    set(&mut p, inst::CUT_BY, 3.0);
    poly.note_on(50, 0.8, &Instrument::Synth, &p);
    for _ in 0..10 {
        render(&mut poly, &p);
    }
    assert_eq!(poly.active(), 1, "a note chokes its own group");
    poly.cut_all();
    for _ in 0..10 {
        render(&mut poly, &p);
    }
    assert_eq!(poly.active(), 0);
}

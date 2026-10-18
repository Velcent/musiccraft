use std::f32::consts::TAU;

use crate::import::{Slicing, clm_frame_len, from_wav, slice};
use crate::spectrum::{HARMONICS, Partial, from_partials, partials};
use crate::{FRAME_LEN, MAX_FRAMES, MorphMode, Op, Table, export, factory};

fn sine(harmonic: f32, phase: f32) -> Table {
    Table::from_fn(1, |_, x| (TAU * (harmonic * x + phase)).sin())
}

fn peak(frame: &[f32]) -> f32 {
    frame.iter().fold(0.0, |m, x| m.max(x.abs()))
}

fn apply(table: &mut Table, op: Op) {
    op.apply(table).unwrap();
}

#[test]
fn limits_match_the_synth() {
    assert_eq!(FRAME_LEN, musiccraft_antiserum::wavetable::FRAME_LEN);
    assert_eq!(MAX_FRAMES, musiccraft_antiserum::wavetable::MAX_FRAMES);
    let table = factory::table("basic").unwrap();
    let synth = musiccraft_antiserum::Wavetable::new(table.samples()).unwrap();
    assert_eq!(synth.frames(), 64);
}

#[test]
fn tables_hold_whole_finite_frames() {
    assert!(Table::new(&[0.0; FRAME_LEN - 1]).is_none());
    let mut samples = vec![0.5; FRAME_LEN * 2 + 7];
    samples[3] = f32::NAN;
    let table = Table::new(&samples).unwrap();
    assert_eq!(table.frames(), 2);
    assert_eq!(table.frame(0).unwrap()[3], 0.0);
    assert!(table.frame(2).is_none());
    let big = vec![0.0; FRAME_LEN * (MAX_FRAMES + 3)];
    assert_eq!(Table::new(&big).unwrap().frames(), MAX_FRAMES);
}

#[test]
fn partials_read_magnitude_and_phase() {
    let p = partials(sine(3.0, 0.25).frame(0).unwrap());
    assert_eq!(p.len(), HARMONICS);
    assert!((p[2].magnitude - 1.0).abs() < 1e-4, "{:?}", p[2]);
    assert!((p[2].phase - 0.25).abs() < 1e-4, "{:?}", p[2]);
    assert!(p[0].magnitude < 1e-4 && p[3].magnitude < 1e-4);
    // And back.
    let rebuilt = from_partials(&p);
    let original = sine(3.0, 0.25);
    for (a, b) in rebuilt.iter().zip(original.frame(0).unwrap()) {
        assert!((a - b).abs() < 1e-4);
    }
}

#[test]
fn set_partial_changes_one_harmonic_only() {
    let mut table = sine(1.0, 0.0);
    apply(
        &mut table,
        Op::SetPartial {
            frame: 0,
            harmonic: 2,
            magnitude: 0.5,
            phase: 0.0,
        },
    );
    let p = partials(table.frame(0).unwrap());
    assert!((p[0].magnitude - 1.0).abs() < 1e-4);
    assert!((p[1].magnitude - 0.5).abs() < 1e-4);
    let bad = Op::SetPartial {
        frame: 0,
        harmonic: 0,
        magnitude: 1.0,
        phase: 0.0,
    };
    assert!(bad.apply(&mut table).is_err());
}

#[test]
fn drawing_wraps_and_clamps() {
    let mut table = Table::silent();
    apply(
        &mut table,
        Op::Draw {
            frame: 0,
            start: FRAME_LEN - 1,
            values: vec![0.5, 2.0, f32::NAN],
        },
    );
    let f = table.frame(0).unwrap();
    assert_eq!((f[FRAME_LEN - 1], f[0], f[1]), (0.5, 1.0, 0.0));
    let before = table.clone();
    let out_of_range = Op::Draw {
        frame: 1,
        start: 0,
        values: vec![1.0],
    };
    assert!(out_of_range.apply(&mut table).is_err());
    assert_eq!(table, before);
}

#[test]
fn formulas_fill_a_range_by_x_and_y() {
    let mut table = Table::from_fn(3, |_, _| 0.0);
    apply(
        &mut table,
        Op::Formula {
            text: "sin(x*pi*2) * (1-y)".to_owned(),
            first: 0,
            last: 2,
        },
    );
    let quarter = FRAME_LEN / 4;
    assert!((table.frame(0).unwrap()[quarter] - 1.0).abs() < 1e-5);
    assert!((table.frame(1).unwrap()[quarter] - 0.5).abs() < 1e-5);
    assert!(table.frame(2).unwrap()[quarter].abs() < 1e-5);
    let before = table.clone();
    let bad = Op::Formula {
        text: "t".to_owned(),
        first: 0,
        last: 0,
    };
    assert!(bad.apply(&mut table).is_err());
    assert_eq!(table, before);
}

#[test]
fn crossfade_and_spectral_morphs() {
    let mut table = Table::from_fn(5, |y, x| if y == 0.0 { (TAU * x).sin() } else { 0.0 });
    let last = sine(1.0, 0.5); // the same sine, inverted
    table.set_frame(4, last.frame(0).unwrap());
    let mut crossfade = table.clone();
    apply(
        &mut crossfade,
        Op::Morph {
            first: 0,
            last: 4,
            mode: MorphMode::Crossfade,
        },
    );
    // Halfway, an inverted copy cancels.
    assert!(peak(crossfade.frame(2).unwrap()) < 1e-5);
    // Against a sine 0.4 turns on, so the shorter way round is clear.
    let mut spectral = table;
    spectral.set_frame(4, sine(1.0, 0.4).frame(0).unwrap());
    apply(
        &mut spectral,
        Op::Morph {
            first: 0,
            last: 4,
            mode: MorphMode::Spectral,
        },
    );
    // Halfway, the phase has turned 0.2 and the level is kept.
    let p = partials(spectral.frame(2).unwrap());
    assert!((p[0].magnitude - 1.0).abs() < 1e-3, "{:?}", p[0]);
    assert!((p[0].phase - 0.2).abs() < 1e-3, "{:?}", p[0]);
}

#[test]
fn normalize_dc_and_reversal() {
    let mut table = Table::from_fn(2, |y, x| 0.25 + 0.5 * x * (1.0 - y));
    apply(&mut table, Op::RemoveDc { first: 0, last: 1 });
    let mean: f32 = table.frame(0).unwrap().iter().sum::<f32>() / FRAME_LEN as f32;
    assert!(mean.abs() < 1e-5);
    assert!(peak(table.frame(1).unwrap()) < 1e-6);
    apply(&mut table, Op::Normalize { first: 0, last: 1 });
    assert!((peak(table.frame(0).unwrap()) - 1.0).abs() < 1e-6);
    assert!(peak(table.frame(1).unwrap()) < 1e-6, "silence stays silent");
    let first = table.frame(0).unwrap().to_vec();
    apply(&mut table, Op::ReverseSamples { first: 0, last: 0 });
    assert_eq!(table.frame(0).unwrap()[0], first[FRAME_LEN - 1]);
    apply(&mut table, Op::ReverseFrames);
    assert!(peak(table.frame(0).unwrap()) < 1e-6);
}

#[test]
fn frames_are_inserted_and_deleted_within_limits() {
    let mut table = Table::from_fn(2, |y, _| y);
    apply(&mut table, Op::InsertFrame { at: 0 });
    assert_eq!(table.frames(), 3);
    assert_eq!(table.frame(1), table.frame(0));
    apply(&mut table, Op::DeleteFrame { at: 2 });
    apply(&mut table, Op::DeleteFrame { at: 1 });
    assert!(Op::DeleteFrame { at: 0 }.apply(&mut table).is_err());
    let mut full = Table::from_fn(MAX_FRAMES, |_, _| 0.0);
    assert!(Op::InsertFrame { at: 0 }.apply(&mut full).is_err());
}

#[test]
fn every_factory_table_is_deterministic_and_normalized() {
    for name in factory::names() {
        let table = factory::table(name).unwrap();
        assert_eq!(table, factory::table(name).unwrap(), "{name}");
        assert_eq!(table.frames(), 64, "{name}");
        for i in 0..table.frames() {
            let p = peak(table.frame(i).unwrap());
            assert!((p - 1.0).abs() < 1e-5, "{name} frame {i}: {p}");
        }
        // Frames differ from end to end: every table morphs.
        assert_ne!(table.frame(0), table.frame(63), "{name}");
    }
    assert!(factory::table("nope").is_none());
}

#[test]
fn factory_tables_have_their_spectra() {
    let additive = factory::table("additive").unwrap();
    for (frame, count) in [(0, 1), (9, 10)] {
        let p = partials(additive.frame(frame).unwrap());
        let present = p.iter().filter(|q| q.magnitude > 1e-3).count();
        assert_eq!(present, count, "frame {frame}");
    }
    // A 50 % pulse has odd harmonics only; a narrow one has even ones too.
    let pwm = factory::table("pwm").unwrap();
    let wide = partials(pwm.frame(0).unwrap());
    assert!(wide[1].magnitude < 1e-3 && wide[2].magnitude > 0.1);
    let narrow = partials(pwm.frame(63).unwrap());
    assert!(narrow[1].magnitude > 0.01);
    let odd_even = factory::table("odd_even").unwrap();
    assert!(partials(odd_even.frame(0).unwrap())[1].magnitude < 1e-3);
    assert!(partials(odd_even.frame(63).unwrap())[0].magnitude < 1e-3);
    // Sampled tables keep nothing above the frame's harmonics.
    let fm = factory::table("fm").unwrap();
    assert!(partials(fm.frame(0).unwrap())[0].magnitude > 0.99);
}

#[test]
fn export_then_import_is_lossless() {
    let table = factory::table("formant").unwrap();
    let bytes = export::to_wav(&table);
    assert_eq!(clm_frame_len(&bytes), Some(FRAME_LEN));
    assert_eq!(from_wav(&bytes, Slicing::Auto).unwrap(), table);
}

/// A 16-bit mono WAV of `samples` with an optional `clm ` chunk.
fn wav(samples: &[f32], clm: Option<&str>) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
    for &x in samples {
        writer.write_sample((x * 32_767.0) as i16).unwrap();
    }
    writer.finalize().unwrap();
    let mut bytes = cursor.into_inner();
    if let Some(text) = clm {
        // Insert the chunk after "fmt ", before "data".
        let mut chunk = b"clm ".to_vec();
        chunk.extend_from_slice(&(text.len() as u32).to_le_bytes());
        chunk.extend_from_slice(text.as_bytes());
        if text.len() % 2 == 1 {
            chunk.push(0);
        }
        let at = bytes.windows(4).position(|w| w == b"data").unwrap();
        bytes.splice(at..at, chunk.iter().copied());
        let riff = (bytes.len() - 8) as u32;
        bytes.splice(4..8, riff.to_le_bytes());
    }
    bytes
}

#[test]
fn auto_import_honours_the_clm_chunk_and_frame_counts() {
    // 4 frames of 256 samples, marked with a clm chunk.
    let samples: Vec<f32> = (0..1024)
        .map(|i| ((i % 256) as f32 / 128.0) - 1.0)
        .collect();
    // Even length: hound does not skip the pad byte after an odd chunk.
    let table = from_wav(&wav(&samples, Some("<!>256 wavetable")), Slicing::Auto).unwrap();
    assert_eq!(table.frames(), 4);
    // Without the chunk, 1024 samples are one single cycle.
    let single = from_wav(&wav(&samples, None), Slicing::Auto).unwrap();
    assert_eq!(single.frames(), 1);
    // Whole 2048-sample frames are a wavetable.
    let two = from_wav(&wav(&vec![0.1; FRAME_LEN * 2], None), Slicing::Auto).unwrap();
    assert_eq!(two.frames(), 2);
    assert!(from_wav(b"not a wav", Slicing::Auto).is_err());
}

#[test]
fn audio_is_sliced_by_pitch_or_by_size() {
    let rate = 48_000.0;
    // Two seconds of 200 Hz: periods of 240 samples.
    let audio: Vec<f32> = (0..96_000)
        .map(|i| (TAU * 200.0 * i as f32 / rate).sin() * 0.8)
        .collect();
    let table = slice(&audio, rate, Slicing::Pitch).unwrap();
    assert_eq!(table.frames(), MAX_FRAMES);
    // Each frame is one period, so harmonic 1 dominates.
    let p = partials(table.frame(10).unwrap());
    assert!(p[0].magnitude > 0.7, "{:?}", p[0]);
    assert!(p[1].magnitude < 0.05, "{:?}", p[1]);
    let fixed = slice(&audio[..2400], rate, Slicing::Fixed(240)).unwrap();
    assert_eq!(fixed.frames(), 10);
    let silence = vec![0.0; 48_000];
    assert!(slice(&silence, rate, Slicing::Pitch).is_err());
    assert!(slice(&[0.0], rate, Slicing::SingleCycle).is_err());
}

#[test]
fn slicing_names_parse() {
    assert_eq!(Slicing::parse("auto"), Some(Slicing::Auto));
    assert_eq!(Slicing::parse("pitch"), Some(Slicing::Pitch));
    assert_eq!(Slicing::parse("single"), Some(Slicing::SingleCycle));
    assert_eq!(Slicing::parse("fixed:600"), Some(Slicing::Fixed(600)));
    assert_eq!(Slicing::parse("frames:256"), Some(Slicing::Wavetable(256)));
    assert_eq!(Slicing::parse("fixed:1"), None);
    assert_eq!(Slicing::parse("other"), None);
}

#[test]
fn partial_defaults_are_silent() {
    assert_eq!(Partial::default().magnitude, 0.0);
    assert!(from_partials(&[]).iter().all(|&x| x == 0.0));
}

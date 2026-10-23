use super::*;

const RATE: f64 = 22_050.0;

fn sine(hz: f64, amp: f32, len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| amp * (2.0 * std::f64::consts::PI * hz * i as f64 / RATE).sin() as f32)
        .collect()
}

/// Short noise bursts four times a second, as a stand-in for drum hits.
fn hits(len: usize) -> Vec<f32> {
    let mut seed = 1_u32;
    let period = (RATE / 4.0) as usize;
    let decay = (RATE * 0.004) as f32;
    (0..len)
        .map(|i| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (seed >> 9) as f32 / (1 << 22) as f32 - 1.0;
            noise * 0.8 * (-((i % period) as f32) / decay).exp()
        })
        .collect()
}

fn add(parts: &[&[f32]]) -> Vec<f32> {
    (0..parts[0].len())
        .map(|i| parts.iter().map(|p| p[i]).sum())
        .collect()
}

/// How far `got` is from `want`, relative to `want`'s energy, away from the
/// ends.
fn error(got: &[f32], want: &[f32]) -> f32 {
    let edge = (RATE * 0.2) as usize;
    let range = edge..want.len() - edge;
    let diff: f32 = range.clone().map(|i| (got[i] - want[i]).powi(2)).sum();
    let energy: f32 = range.map(|i| want[i].powi(2)).sum();
    diff / energy
}

fn run(left: &[f32], right: &[f32]) -> Stems {
    split(left, right, RATE, &mut |_| true).unwrap()
}

#[test]
fn the_stems_add_up_to_the_sound() {
    let len = (RATE * 1.5) as usize;
    let left = add(&[&sine(220.0, 0.3, len), &hits(len)]);
    let right = add(&[&sine(330.0, 0.3, len), &hits(len)]);
    let stems = run(&left, &right);
    for i in 0..len {
        let (l, r) = Stem::ALL.iter().fold((0.0, 0.0), |(l, r), s| {
            let a = stems.get(*s);
            (l + a.left[i], r + a.right[i])
        });
        assert!((l - left[i]).abs() < 1e-5 && (r - right[i]).abs() < 1e-5);
    }
}

#[test]
fn a_centred_tone_goes_to_the_vocals() {
    let len = (RATE * 1.5) as usize;
    let voice = add(&[&sine(440.0, 0.3, len), &sine(880.0, 0.15, len)]);
    let stems = run(&voice, &voice);
    let vocals = stems.get(Stem::Vocals);
    assert!(
        error(&vocals.left, &voice) < 0.01,
        "{}",
        error(&vocals.left, &voice)
    );
    assert!(error(&vocals.right, &voice) < 0.01);
}

#[test]
fn a_mix_splits_into_its_parts() {
    let len = (RATE * 2.0) as usize;
    let voice = add(&[&sine(440.0, 0.3, len), &sine(880.0, 0.15, len)]);
    let bass = sine(65.0, 0.3, len);
    let lead = sine(1_500.0, 0.3, len);
    let drums = hits(len);
    let left = add(&[&voice, &bass, &lead, &drums]);
    let right = add(&[&voice, &bass, &drums]);
    let stems = run(&left, &right);
    let silent = vec![0.0; len];
    let checks = [
        (Stem::Vocals, &voice, &voice),
        (Stem::Bass, &bass, &bass),
        (Stem::Drums, &drums, &drums),
        (Stem::Other, &lead, &silent),
    ];
    for (stem, want_left, want_right) in checks {
        let got = stems.get(stem);
        let e = error(&got.left, want_left);
        assert!(e < 0.3, "{} left: error {e}", stem.name());
        if stem != Stem::Other {
            let e = error(&got.right, want_right);
            assert!(e < 0.3, "{} right: error {e}", stem.name());
        }
    }
    // The lead is hard left, so it leaves nothing on the right of "other".
    let other = stems.get(Stem::Other);
    let leak: f32 = other.right.iter().map(|s| s * s).sum();
    let lead_energy: f32 = lead.iter().map(|s| s * s).sum();
    assert!(leak < 0.3 * lead_energy, "{leak} vs {lead_energy}");
}

#[test]
fn silence_stays_silent() {
    let silent = vec![0.0; 5_000];
    let stems = run(&silent, &silent);
    for stem in Stem::ALL {
        assert!(stems.get(stem).left.iter().all(|s| *s == 0.0));
    }
}

#[test]
fn short_and_empty_sounds_split() {
    let stems = run(&[], &[]);
    assert!(stems.get(Stem::Vocals).left.is_empty());
    let click = [0.5_f32; 10];
    let stems = run(&click, &click);
    assert_eq!(stems.get(Stem::Other).left.len(), 10);
}

#[test]
fn progress_can_cancel() {
    let len = (RATE * 2.0) as usize;
    let tone = sine(440.0, 0.3, len);
    let mut calls = 0;
    let result = split(&tone, &tone, RATE, &mut |done| {
        calls += 1;
        assert!((0.0..=1.0).contains(&done));
        calls < 2
    });
    assert_eq!(result, Err(Cancelled));
    let mut last = 0.0;
    split(&tone, &tone, RATE, &mut |done| {
        assert!(done >= last);
        last = done;
        true
    })
    .unwrap();
    assert!((last - 1.0).abs() < f32::EPSILON);
}

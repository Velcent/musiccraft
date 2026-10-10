//! Built-in drum sounds, synthesized from first principles (own design). Every
//! sound is deterministic: the same name and sample rate always give the same
//! audio.

use std::f64::consts::TAU;

use musiccraft_dsp::{SampleBuffer, Svf, SvfMode};

/// Peak level every built-in sound is normalized to.
const PEAK: f32 = 0.9;

/// Deterministic white noise.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// Exponential decay reaching -60 dB after `t60` seconds.
fn decay(t: f64, t60: f64) -> f32 {
    (-6.9 * t / t60).exp() as f32
}

fn render(seconds: f64, sample_rate: f64, mut f: impl FnMut(f64) -> f32) -> Vec<f32> {
    let frames = (seconds * sample_rate) as usize;
    (0..frames).map(|i| f(i as f64 / sample_rate)).collect()
}

fn normalize(mut data: Vec<f32>) -> Vec<f32> {
    let peak = data.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
    if peak > 0.0 {
        let gain = PEAK / peak;
        for s in &mut data {
            *s *= gain;
        }
    }
    // Short fade-out so no sound ends on a click.
    let fade = data.len().min(64);
    let len = data.len();
    for (i, s) in data.iter_mut().skip(len - fade).enumerate() {
        *s *= 1.0 - i as f32 / fade as f32;
    }
    data
}

/// A sine sweeping exponentially from `from` to `to` hertz over `sweep` seconds.
fn swept_sine(
    sample_rate: f64,
    seconds: f64,
    from: f64,
    to: f64,
    sweep: f64,
    t60: f64,
) -> Vec<f32> {
    let mut phase = 0.0;
    render(seconds, sample_rate, |t| {
        let k = (t / sweep).min(1.0);
        let freq = from * (to / from).powf(k);
        phase += freq / sample_rate;
        (phase * TAU).sin() as f32 * decay(t, t60)
    })
}

fn filtered_noise(
    sample_rate: f64,
    seconds: f64,
    mode: SvfMode,
    cutoff: f32,
    resonance: f32,
    seed: u32,
    mut envelope: impl FnMut(f64) -> f32,
) -> Vec<f32> {
    let mut noise = Noise(seed);
    let mut filter = Svf::new();
    filter.set(cutoff, resonance, sample_rate as f32);
    render(seconds, sample_rate, |t| {
        filter.process(noise.next(), mode) * envelope(t)
    })
}

fn kick(sr: f64) -> Vec<f32> {
    let body = swept_sine(sr, 0.5, 160.0, 45.0, 0.08, 0.45);
    let click = filtered_noise(sr, 0.5, SvfMode::HighPass, 3_000.0, 0.0, 11, |t| {
        decay(t, 0.008) * 0.4
    });
    body.iter()
        .zip(&click)
        .map(|(a, b)| (a * 1.6).tanh() + b)
        .collect()
}

fn snare(sr: f64) -> Vec<f32> {
    let tone = swept_sine(sr, 0.35, 240.0, 180.0, 0.03, 0.12);
    let rattle = filtered_noise(sr, 0.35, SvfMode::HighPass, 1_800.0, 0.1, 22, |t| {
        decay(t, 0.22)
    });
    tone.iter()
        .zip(&rattle)
        .map(|(a, b)| a * 0.6 + b * 0.8)
        .collect()
}

fn clap(sr: f64) -> Vec<f32> {
    filtered_noise(sr, 0.4, SvfMode::BandPass, 1_200.0, 0.35, 33, |t| {
        // Three quick bursts, then a diffuse tail.
        let burst = |start: f64| {
            if t >= start {
                decay(t - start, 0.03)
            } else {
                0.0
            }
        };
        let tail = if t >= 0.024 {
            decay(t - 0.024, 0.25) * 0.8
        } else {
            0.0
        };
        (burst(0.0) + burst(0.011) + burst(0.022)).max(tail)
    })
}

fn hat(sr: f64, t60: f64, seed: u32) -> Vec<f32> {
    // Six detuned square partials give a metallic colour before the high-pass.
    let ratios = [2.0, 3.0, 4.16, 5.43, 6.79, 8.21];
    let base = 330.0;
    let mut noise = Noise(seed);
    let mut filter = Svf::new();
    filter.set(7_500.0, 0.2, sr as f32);
    render(t60 * 1.2, sr, |t| {
        let metal: f32 = ratios
            .iter()
            .map(|r| {
                if (t * base * r).fract() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            })
            .sum::<f32>()
            / 6.0;
        let x = metal * 0.6 + noise.next() * 0.4;
        filter.process(x, SvfMode::HighPass) * decay(t, t60)
    })
}

fn tom(sr: f64) -> Vec<f32> {
    swept_sine(sr, 0.6, 180.0, 95.0, 0.15, 0.5)
}

fn rim(sr: f64) -> Vec<f32> {
    render(0.08, sr, |t| {
        let a = (t * 820.0 * TAU).sin();
        let b = (t * 1_690.0 * TAU).sin();
        ((a + b) * 0.5) as f32 * decay(t, 0.035)
    })
}

fn crash(sr: f64) -> Vec<f32> {
    filtered_noise(sr, 2.2, SvfMode::HighPass, 4_000.0, 0.15, 44, |t| {
        let attack = (t / 0.004).min(1.0) as f32;
        attack * decay(t, 1.8)
    })
}

/// Synthesizes the built-in sound `name` (without the `builtin:` prefix) at
/// `sample_rate`. Returns `None` for unknown names.
#[must_use]
pub fn builtin(name: &str, sample_rate: u32) -> Option<SampleBuffer> {
    let sr = f64::from(sample_rate.max(8_000));
    let data = match name {
        "kick" => kick(sr),
        "snare" => snare(sr),
        "clap" => clap(sr),
        "hat" => hat(sr, 0.06, 55),
        "open_hat" => hat(sr, 0.35, 66),
        "tom" => tom(sr),
        "rim" => rim(sr),
        "crash" => crash(sr),
        _ => return None,
    };
    Some(SampleBuffer::mono(normalize(data), sr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::BUILTIN_SAMPLES;

    #[test]
    fn every_listed_sound_exists_and_is_normalized() {
        for name in BUILTIN_SAMPLES {
            let buffer = builtin(name, 44_100).unwrap_or_else(|| panic!("{name} missing"));
            assert!(!buffer.is_empty(), "{name} is empty");
            assert!(
                (buffer.peak() - PEAK).abs() < 1e-3,
                "{name} peak {}",
                buffer.peak()
            );
            assert!(buffer.left().iter().all(|s| s.is_finite()));
            let last = *buffer.left().last().unwrap();
            assert!(last.abs() < 1e-3, "{name} ends on a click");
        }
        assert!(builtin("cowbell", 44_100).is_none());
    }

    #[test]
    fn sounds_are_deterministic() {
        assert_eq!(builtin("snare", 48_000), builtin("snare", 48_000));
    }
}

use crate::effect::ProcessContext;
use crate::util::flush_denormal;

/// Longest delay time supported, in seconds. The buffer is sized for this.
const MAX_DELAY_SECONDS: f32 = 4.0;

/// Lengths of the feedback diffusers' all-pass delays, in seconds.
const DIFFUSER_SECONDS: [f32; 2] = [0.002_3, 0.007_1];

/// Longest stereo offset, in seconds.
const MAX_OFFSET_SECONDS: f32 = 0.02;

/// Longest modulation depth, in seconds.
const MAX_MOD_SECONDS: f32 = 0.01;

/// A Schroeder all-pass diffuser with a fixed delay.
#[derive(Debug, Clone)]
struct Diffuser {
    line: Vec<f32>,
    pos: usize,
}

impl Diffuser {
    fn new(seconds: f32, sample_rate: f32) -> Self {
        let len = ((seconds * sample_rate.max(1.0)) as usize).max(1);
        Self {
            line: vec![0.0; len],
            pos: 0,
        }
    }

    fn process(&mut self, x: f32, gain: f32) -> f32 {
        let Some(slot) = self.line.get_mut(self.pos) else {
            return x;
        };
        let out = *slot;
        let v = flush_denormal(x + gain * out);
        *slot = v;
        self.pos = (self.pos + 1) % self.line.len().max(1);
        out - gain * v
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
    }
}

/// Reads `buffer` `delay` samples behind `write`, interpolating linearly.
fn read_frac(buffer: &[f32], write: usize, delay: f32) -> f32 {
    let len = buffer.len();
    if len < 3 {
        return 0.0;
    }
    let delay = delay.clamp(1.0, (len - 2) as f32);
    let whole = delay as usize;
    let frac = delay - whole as f32;
    let a = buffer
        .get((write + len - whole) % len)
        .copied()
        .unwrap_or(0.0);
    if frac == 0.0 {
        return a;
    }
    let b = buffer
        .get((write + len - whole - 1) % len)
        .copied()
        .unwrap_or(0.0);
    a + (b - a) * frac
}

/// Per-channel state of the delay's feedback path.
#[derive(Debug, Clone)]
struct DelayChannel {
    line: Vec<f32>,
    damp: f32,
    low_cut: f32,
    diffusers: [Diffuser; 2],
}

impl DelayChannel {
    fn new(sample_rate: f32) -> Self {
        let seconds = MAX_DELAY_SECONDS + MAX_OFFSET_SECONDS + MAX_MOD_SECONDS;
        Self {
            line: vec![0.0; (sample_rate.max(1.0) * seconds) as usize + 3],
            damp: 0.0,
            low_cut: 0.0,
            diffusers: DIFFUSER_SECONDS.map(|s| Diffuser::new(s, sample_rate)),
        }
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
        self.damp = 0.0;
        self.low_cut = 0.0;
        for d in &mut self.diffusers {
            d.reset();
        }
    }
}

/// Feedback-path settings of the delay for one block.
#[derive(Debug, Clone, Copy)]
struct FeedbackPath {
    damping: f32,
    low_cut: Option<f32>,
    diffusion: f32,
    drive: Option<f32>,
}

impl DelayChannel {
    /// The delayed sample `x` shaped by the feedback path.
    fn shape(&mut self, x: f32, path: FeedbackPath) -> f32 {
        self.damp = flush_denormal(x + (self.damp - x) * path.damping);
        let mut y = self.damp;
        if let Some(a) = path.low_cut {
            self.low_cut = flush_denormal(self.low_cut + (y - self.low_cut) * a);
            y -= self.low_cut;
        }
        if path.diffusion > 0.0 {
            for d in &mut self.diffusers {
                y = d.process(y, path.diffusion);
            }
        }
        if let Some(k) = path.drive {
            y = (y * k).tanh() / k;
        }
        y
    }
}

/// Tempo-synced stereo delay with optional ping-pong, feedback damping, a
/// stereo offset, modulation, and a feedback path with a low cut, diffusion
/// and saturation.
///
/// Parameters: `[time (beats), feedback, mix, ping_pong, damping, offset ms,
/// mod rate Hz, mod depth ms, low cut Hz, diffusion, saturation]`.
#[derive(Debug, Clone)]
pub struct Delay {
    channels: [DelayChannel; 2],
    write: usize,
    phase: f32,
}

impl Delay {
    /// Allocates the delay lines. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            channels: [
                DelayChannel::new(sample_rate),
                DelayChannel::new(sample_rate),
            ],
            write: 0,
            phase: 0.0,
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.channels[0].line.len();
        if len < 4 || self.channels[1].line.len() != len {
            return;
        }
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let sr = ctx.sample_rate.max(1.0);
        let seconds_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) as f32;
        let delay_seconds = (param(0, 0.75) * seconds_per_beat).clamp(0.0, MAX_DELAY_SECONDS);
        let base = ((delay_seconds * sr) as usize).max(1) as f32;
        let feedback = param(1, 0.4).clamp(0.0, 0.95);
        let mix = param(2, 0.3).clamp(0.0, 1.0);
        let ping_pong = param(3, 1.0) >= 0.5;
        let offset_ms = param(5, 0.0).clamp(-20.0, 20.0);
        let offset = (offset_ms.abs() * 0.001 * sr).round();
        let times = if offset_ms >= 0.0 {
            [base, base + offset]
        } else {
            [base + offset, base]
        };
        let mod_step = param(6, 0.5).max(0.0) / sr;
        let mod_depth = param(7, 0.0).clamp(0.0, 10.0) * 0.001 * sr;
        let low_cut_hz = param(8, 20.0);
        let path = FeedbackPath {
            damping: param(4, 0.3).clamp(0.0, 0.99),
            low_cut: (low_cut_hz > 20.0)
                .then(|| 1.0 - (-std::f32::consts::TAU * low_cut_hz / sr).exp()),
            diffusion: param(9, 0.0).clamp(0.0, 0.9),
            drive: {
                let saturation = param(10, 0.0).clamp(0.0, 1.0);
                (saturation > 0.0).then_some(1.0 + saturation * 4.0)
            },
        };

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut delayed = [0.0_f32; 2];
            for (ch, (out, channel)) in delayed.iter_mut().zip(&self.channels).enumerate() {
                let wobble = if mod_depth > 0.0 {
                    let offset = if ch == 0 { 0.0 } else { 0.25 };
                    mod_depth * (0.5 + 0.5 * ((self.phase + offset) * std::f32::consts::TAU).sin())
                } else {
                    0.0
                };
                *out = read_frac(&channel.line, self.write, times[ch] + wobble);
            }
            let [dl, dr] = delayed;
            let [left_ch, right_ch] = &mut self.channels;
            let shaped = [left_ch.shape(dl, path), right_ch.shape(dr, path)];
            let (fb_l, fb_r) = if ping_pong {
                (shaped[1] * feedback, shaped[0] * feedback)
            } else {
                (shaped[0] * feedback, shaped[1] * feedback)
            };
            let (in_l, in_r) = if ping_pong {
                ((*l + *r) * 0.5, 0.0)
            } else {
                (*l, *r)
            };
            for (channel, value) in [(left_ch, in_l + fb_l), (right_ch, in_r + fb_r)] {
                if let Some(slot) = channel.line.get_mut(self.write) {
                    *slot = if value.is_finite() {
                        flush_denormal(value)
                    } else {
                        0.0
                    };
                }
            }
            self.write = (self.write + 1) % len;
            self.phase = (self.phase + mod_step).fract();
            *l += (dl - *l) * mix;
            *r += (dr - *r) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_repeats_after_one_beat() {
        let sample_rate = 1_000.0;
        let mut delay = Delay::new(sample_rate);
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate,
            tempo_bpm: 60.0,
        };
        // 1 beat at 60 BPM = 1 s = 1000 samples; full wet, no feedback, no ping-pong.
        let params = [1.0, 0.0, 1.0, 0.0, 0.0];
        let mut l = vec![0.0; 1_500];
        let mut r = vec![0.0; 1_500];
        l[0] = 1.0;
        r[0] = 1.0;
        delay.process(&mut l, &mut r, &params, ctx);
        assert!(l[0].abs() < 1e-6);
        assert!((l[1_000] - 1.0).abs() < 1e-6);
        assert!((r[1_000] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn feedback_decays() {
        let sample_rate = 100.0;
        let mut delay = Delay::new(sample_rate);
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate,
            tempo_bpm: 60.0,
        };
        let params = [0.1, 0.5, 1.0, 0.0, 0.0];
        let mut l = vec![0.0; 100];
        let mut r = vec![0.0; 100];
        l[0] = 1.0;
        delay.process(&mut l, &mut r, &params, ctx);
        assert!((l[10] - 1.0).abs() < 1e-6);
        assert!((l[20] - 0.5).abs() < 1e-6);
        assert!((l[30] - 0.25).abs() < 1e-6);
    }

    fn ctx(sample_rate: f32) -> ProcessContext {
        ProcessContext {
            beats: 0.0,
            sample_rate,
            tempo_bpm: 60.0,
        }
    }

    #[test]
    fn offset_delays_one_side_more() {
        let mut delay = Delay::new(1_000.0);
        // 0.1 beat = 100 samples; +10 ms offset delays the right side 10 more.
        let params = [0.1, 0.0, 1.0, 0.0, 0.0, 10.0, 0.5, 0.0, 20.0, 0.0, 0.0];
        let (mut l, mut r) = (vec![0.0; 200], vec![0.0; 200]);
        l[0] = 1.0;
        r[0] = 1.0;
        delay.process(&mut l, &mut r, &params, ctx(1_000.0));
        assert!((l[100] - 1.0).abs() < 1e-6 && (r[110] - 1.0).abs() < 1e-6);
        assert!(r[100].abs() < 1e-6);
    }

    #[test]
    fn modulation_moves_the_echo() {
        let mut delay = Delay::new(48_000.0);
        let params = [0.25, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 5.0, 20.0, 0.0, 0.0];
        let (mut l, mut r): (Vec<f32>, Vec<f32>) = (
            (0..48_000).map(|i| (i as f32 * 0.05).sin()).collect(),
            vec![0.0; 48_000],
        );
        let dry = l.clone();
        delay.process(&mut l, &mut r, &params, ctx(48_000.0));
        // A fixed 12_000-sample delay would copy the input exactly.
        let off = l[24_000..30_000]
            .iter()
            .zip(&dry[12_000..18_000])
            .any(|(a, b)| (a - b).abs() > 0.05);
        assert!(off, "modulated echo differs from a fixed one");
        assert!(l.iter().all(|s| s.is_finite() && s.abs() <= 1.01));
    }

    #[test]
    fn feedback_low_cut_thins_repeats_and_stays_stable() {
        let echo_energy = |low_cut: f32, diffusion: f32, saturation: f32| {
            let mut delay = Delay::new(8_000.0);
            let params = [
                0.25, 0.9, 1.0, 0.0, 0.0, 0.0, 0.5, 0.0, low_cut, diffusion, saturation,
            ];
            let (mut l, mut r): (Vec<f32>, Vec<f32>) = (vec![0.0; 40_000], vec![0.0; 40_000]);
            for s in l.iter_mut().take(400) {
                *s = 0.5; // a low thump
            }
            delay.process(&mut l, &mut r, &params, ctx(8_000.0));
            assert!(l.iter().all(|s| s.is_finite() && s.abs() < 4.0));
            l[30_000..].iter().map(|s| s * s).sum::<f32>()
        };
        let plain = echo_energy(20.0, 0.0, 0.0);
        assert!(
            echo_energy(800.0, 0.0, 0.0) < plain * 0.1,
            "low cut removes the thump"
        );
        assert!(echo_energy(20.0, 0.7, 0.0).is_finite());
        assert!(
            echo_energy(20.0, 0.0, 1.0) <= plain * 1.01,
            "saturation never adds level"
        );
    }
}

/// Longest chorus delay (base plus depth) in seconds.
const MAX_CHORUS_SECONDS: f32 = 0.05;

/// Stereo chorus: a short delay per channel modulated by sine LFOs a quarter
/// cycle apart, mixed with the dry signal.
///
/// Parameters: `[rate Hz, depth ms, delay ms, mix, voices, tone Hz]`.
#[derive(Debug, Clone)]
pub struct Chorus {
    left: Vec<f32>,
    right: Vec<f32>,
    write: usize,
    phase: f32,
    tone: [f32; 2],
}

impl Chorus {
    /// Allocates the delay lines. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * MAX_CHORUS_SECONDS) as usize + 2;
        Self {
            left: vec![0.0; len],
            right: vec![0.0; len],
            write: 0,
            phase: 0.0,
            tone: [0.0; 2],
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.tone = [0.0; 2];
    }

    fn read(buffer: &[f32], write: usize, delay: f32) -> f32 {
        let len = buffer.len();
        let delay = delay.clamp(1.0, (len - 2) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let a = buffer
            .get((write + len - whole) % len)
            .copied()
            .unwrap_or(0.0);
        let b = buffer
            .get((write + len - whole - 1) % len)
            .copied()
            .unwrap_or(0.0);
        a + (b - a) * frac
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.left.len();
        if len < 4 || self.right.len() != len {
            return;
        }
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let sr = ctx.sample_rate.max(1.0);
        let rate = param(0, 0.8).max(0.0);
        let depth = param(1, 3.0).max(0.0) * 0.001 * sr;
        let base = param(2, 12.0).max(0.0) * 0.001 * sr;
        let mix = param(3, 0.4).clamp(0.0, 1.0);
        let voices = (param(4, 1.0).round() as usize).clamp(1, 3);
        let tone_hz = param(5, 20_000.0);
        let tone =
            (tone_hz < 20_000.0).then(|| 1.0 - (-std::f32::consts::TAU * tone_hz / sr).exp());
        let step = rate / sr;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if let Some(slot) = self.left.get_mut(self.write) {
                *slot = *l;
            }
            if let Some(slot) = self.right.get_mut(self.write) {
                *slot = *r;
            }
            let mut wet = [0.0_f32; 2];
            for v in 0..voices {
                let phase = self.phase + v as f32 / voices as f32;
                for (ch, (w, line)) in wet.iter_mut().zip([&self.left, &self.right]).enumerate() {
                    let offset = if ch == 0 { 0.0 } else { 0.25 };
                    let lfo = ((phase + offset) * std::f32::consts::TAU).sin();
                    *w += Self::read(line, self.write, base + depth * (0.5 + 0.5 * lfo));
                }
            }
            for (w, state) in wet.iter_mut().zip(&mut self.tone) {
                *w /= voices as f32;
                if let Some(a) = tone {
                    *state = crate::flush_denormal(*state + (*w - *state) * a);
                    *w = *state;
                }
            }
            let [wet_l, wet_r] = wet;
            *l += (wet_l - *l) * mix;
            *r += (wet_r - *r) * mix;
            self.write = (self.write + 1) % len;
            self.phase = (self.phase + step).fract();
        }
    }
}

#[cfg(test)]
mod chorus_tests {
    use super::*;

    #[test]
    fn chorus_delays_and_modulates() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 10_000.0,
            tempo_bpm: 120.0,
        };
        let mut chorus = Chorus::new(ctx.sample_rate);
        // Fully wet, no modulation: a pure 10 ms delay.
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        r[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0], ctx);
        assert!(l[0].abs() < 1e-6);
        assert!((l[100] - 1.0).abs() < 1e-4, "got {}", l[100]);
        // With modulation the output stays bounded and finite.
        let mut l: Vec<f32> = (0..10_000).map(|i| (i as f32 * 0.03).sin()).collect();
        let mut r = l.clone();
        chorus.process(&mut l, &mut r, &[5.0, 10.0, 30.0, 0.5], ctx);
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() <= 1.01));
    }

    #[test]
    fn chorus_voices_average_and_tone_darkens() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 10_000.0,
            tempo_bpm: 120.0,
        };
        // Three voices with no modulation are three identical copies: the
        // average is the same 10 ms delay.
        let mut chorus = Chorus::new(ctx.sample_rate);
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0, 3.0, 20_000.0], ctx);
        assert!((l[100] - 1.0).abs() < 1e-4, "got {}", l[100]);
        // A low tone smears the echo's click into a gentler bump.
        let mut chorus = Chorus::new(ctx.sample_rate);
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0, 1.0, 500.0], ctx);
        assert!(l[100] < 0.5 && l[101] > 0.05, "{} {}", l[100], l[101]);
    }
}

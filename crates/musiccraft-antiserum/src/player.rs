//! The sample, multisample, granular and spectral oscillators of one voice
//! (`docs/spec/antiserum-oscillators.md`). Settings are made once per control
//! block; samples are produced one at a time without allocating.

use musiccraft_dsp::SampleBuffer;
use musiccraft_dsp::resynth::RESYNTH_PARTIALS;

use crate::source::Source;
use crate::voice::MAX_UNISON;

/// Grains playing at once.
const GRAINS: usize = 16;
/// Entries in the sine table (one cycle; one more for interpolation).
pub const SINE_LEN: usize = 1_024;

/// A sine cycle for the spectral oscillator.
pub(crate) fn sine_table() -> Box<[f32; SINE_LEN + 1]> {
    let mut table = Box::new([0.0; SINE_LEN + 1]);
    for (i, s) in table.iter_mut().enumerate() {
        *s = (std::f32::consts::TAU * i as f32 / SINE_LEN as f32).sin();
    }
    table
}

fn sine(table: &[f32; SINE_LEN + 1], phase: f32) -> f32 {
    let x = phase * SINE_LEN as f32;
    let i = (x as usize).min(SINE_LEN - 1);
    let t = x - i as f32;
    let a = table.get(i).copied().unwrap_or(0.0);
    let b = table.get(i + 1).copied().unwrap_or(0.0);
    a + (b - a) * t
}

/// What an oscillator plays.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Kind {
    #[default]
    Wavetable,
    Sample,
    Multisample,
    Granular,
    Spectral,
}

impl Kind {
    pub fn from_param(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Sample,
            2 => Self::Multisample,
            3 => Self::Granular,
            4 => Self::Spectral,
            _ => Self::Wavetable,
        }
    }
}

/// How the sample oscillator follows the key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Repitch,
    Fixed,
    Stretch,
}

impl Mode {
    pub fn from_param(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Fixed,
            2 => Self::Stretch,
            _ => Self::Repitch,
        }
    }
}

/// Loop modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Loop {
    #[default]
    Off,
    Forward,
    PingPong,
}

impl Loop {
    pub fn from_param(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Forward,
            2 => Self::PingPong,
            _ => Self::Off,
        }
    }
}

/// The settings for one control block.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Settings {
    pub kind: Kind,
    pub mode: Mode,
    pub looping: Loop,
    /// Loop start and end, 0..1 of the sample.
    pub loop_start: f32,
    pub loop_end: f32,
    /// Each unison voice's frequency, hertz.
    pub freq: [f32; MAX_UNISON],
    pub unison: usize,
    /// The note's own frequency (no octave, semitone or fine), hertz.
    pub note_hz: f32,
    /// The frequency `root` stands for, hertz.
    pub root_hz: f32,
    pub grain_seconds: f32,
    pub density: f32,
    pub spread: f32,
    pub grain_pitch: f32,
    pub scan: f32,
    pub shift: f32,
    pub stretch: f32,
    pub tilt: f32,
    pub speed: f32,
    pub sample_rate: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Grain {
    position: f64,
    step: f64,
    age: u32,
    length: u32,
}

/// One oscillator's playback state in one voice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Player {
    settings: Settings,
    /// Sample and multisample: per unison voice, the read position, whether
    /// it moves backwards (ping-pong) and whether it has stopped.
    pos: [f64; MAX_UNISON],
    backwards: [bool; MAX_UNISON],
    stopped: [bool; MAX_UNISON],
    /// The multisample zone this note plays (`usize::MAX`: none).
    zone: usize,
    grains: [Grain; GRAINS],
    grain_timer: f32,
    scan_pos: f64,
    seed: u32,
    /// Spectral: seconds into the analysis, levels and partial phases.
    time: f32,
    gains: [f32; RESYNTH_PARTIALS],
    incs: [f32; RESYNTH_PARTIALS],
    phases: [f32; RESYNTH_PARTIALS],
}

impl Default for Player {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            pos: [0.0; MAX_UNISON],
            backwards: [false; MAX_UNISON],
            stopped: [false; MAX_UNISON],
            zone: usize::MAX,
            grains: [Grain::default(); GRAINS],
            grain_timer: 0.0,
            scan_pos: 0.0,
            seed: 1,
            time: 0.0,
            gains: [0.0; RESYNTH_PARTIALS],
            incs: [0.0; RESYNTH_PARTIALS],
            phases: [0.0; RESYNTH_PARTIALS],
        }
    }
}

impl Player {
    /// A note starts: playback from `position` (0..1).
    pub fn start(
        &mut self,
        source: &Source,
        kind: Kind,
        position: f32,
        key: u8,
        velocity: f32,
        seed: u32,
    ) {
        self.seed = seed | 1;
        self.zone = if kind == Kind::Multisample {
            source
                .zones
                .iter()
                .position(|z| z.plays(key, velocity))
                .unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        let position = position.clamp(0.0, 1.0);
        let len = self.buffer(source, kind).map_or(0.0, |b| b.len() as f64);
        self.pos = [f64::from(position) * len; MAX_UNISON];
        self.backwards = [false; MAX_UNISON];
        self.stopped = [false; MAX_UNISON];
        self.grains = [Grain::default(); GRAINS];
        self.grain_timer = 0.0;
        self.scan_pos = f64::from(position) * len;
        self.time = position * source.harmonics.duration();
        self.phases = [0.0; RESYNTH_PARTIALS];
    }

    /// The sample this player reads.
    fn buffer<'s>(&self, source: &'s Source, kind: Kind) -> Option<&'s SampleBuffer> {
        match kind {
            Kind::Multisample => source.zones.get(self.zone).map(|z| &*z.buffer),
            _ => source.sample.as_deref(),
        }
    }

    /// Takes this block's settings; `frames` is the block's length.
    pub fn control(&mut self, source: &Source, settings: Settings, frames: usize) {
        self.settings = settings;
        if settings.kind != Kind::Spectral {
            return;
        }
        let table = &source.harmonics;
        let looped = settings.looping != Loop::Off;
        let levels = table.at(self.time, looped);
        self.time += frames as f32 / settings.sample_rate.max(1.0) * settings.speed;
        let f0 = settings.freq.first().copied().unwrap_or(0.0);
        let nyquist = settings.sample_rate * 0.5;
        let exponent = 1.0 + settings.stretch * 0.5;
        let mut plain = 0.0;
        let mut weighted = 0.0;
        for (n, ((gain, inc), level)) in self
            .gains
            .iter_mut()
            .zip(self.incs.iter_mut())
            .zip(levels)
            .enumerate()
        {
            let nf = (n + 1) as f32;
            let freq = f0 * nf.powf(exponent) + settings.shift;
            let audible = freq > 0.0 && freq < nyquist;
            let weight = nf.powf(2.0 * settings.tilt);
            *inc = if audible {
                freq / settings.sample_rate
            } else {
                0.0
            };
            *gain = if audible { level * weight } else { 0.0 };
            plain += level;
            weighted += *gain;
        }
        // Tilt reshapes the spectrum without changing the overall level.
        let norm = if weighted > 1e-9 {
            plain / weighted / table.peak_sum.max(1e-3)
        } else {
            0.0
        };
        for gain in &mut self.gains {
            *gain *= norm;
        }
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// The next sample: (left, right) with the unison voices' gains, and
    /// the mono sum for cross-modulation.
    pub fn next(
        &mut self,
        source: &Source,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        sine_table: &[f32; SINE_LEN + 1],
    ) -> (f32, f32, f32) {
        let s = self.settings;
        match s.kind {
            Kind::Wavetable => (0.0, 0.0, 0.0),
            Kind::Sample if s.mode == Mode::Stretch => {
                let mono = self.grain(source, s.kind, true);
                let (gl, gr) = (gains.0[0], gains.1[0]);
                (mono * gl, mono * gr, mono)
            }
            Kind::Sample | Kind::Multisample => self.sample(source, gains),
            Kind::Granular => {
                let mono = self.grain(source, s.kind, false);
                (mono * gains.0[0], mono * gains.1[0], mono)
            }
            Kind::Spectral => {
                let mut sum = 0.0;
                for ((phase, inc), gain) in self.phases.iter_mut().zip(self.incs).zip(self.gains) {
                    if gain != 0.0 {
                        sum += gain * sine(sine_table, *phase);
                    }
                    *phase = (*phase + inc).fract();
                }
                (sum * gains.0[0], sum * gains.1[0], sum)
            }
        }
    }

    /// Sample and multisample playback, every unison voice.
    fn sample(
        &mut self,
        source: &Source,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
    ) -> (f32, f32, f32) {
        let s = self.settings;
        let (buffer, root_hz) = match s.kind {
            Kind::Multisample => match source.zones.get(self.zone) {
                Some(zone) => (
                    &*zone.buffer,
                    musiccraft_dsp::key_to_hz(f32::from(zone.root)),
                ),
                None => return (0.0, 0.0, 0.0),
            },
            _ => match source.sample.as_deref() {
                Some(b) => (b, s.root_hz),
                None => return (0.0, 0.0, 0.0),
            },
        };
        let len = buffer.len() as f64;
        let rate = buffer.sample_rate() / f64::from(s.sample_rate.max(1.0));
        let reference = if s.kind == Kind::Sample && s.mode == Mode::Fixed {
            s.note_hz
        } else {
            root_hz
        };
        let loop_start = f64::from(s.loop_start.min(s.loop_end)) * len;
        let loop_end = (f64::from(s.loop_end.max(s.loop_start)) * len)
            .max(loop_start + 1.0)
            .min(len);
        let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
        for u in 0..s.unison.min(MAX_UNISON) {
            let (Some(pos), Some(back), Some(stopped)) = (
                self.pos.get_mut(u),
                self.backwards.get_mut(u),
                self.stopped.get_mut(u),
            ) else {
                break;
            };
            if *stopped {
                continue;
            }
            let (sl, sr) = buffer.read(*pos);
            let x = (sl + sr) * 0.5;
            mono += x;
            l += x * gains.0.get(u).copied().unwrap_or(0.0);
            r += x * gains.1.get(u).copied().unwrap_or(0.0);
            let freq = s.freq.get(u).copied().unwrap_or(0.0);
            let step = rate * f64::from(freq / reference.max(1e-3));
            match s.looping {
                Loop::Off => {
                    *pos += step;
                    if *pos >= len - 1.0 {
                        *stopped = true;
                    }
                }
                Loop::Forward => {
                    *pos += step;
                    if *pos >= loop_end {
                        let span = loop_end - loop_start;
                        *pos = loop_start + (*pos - loop_end).rem_euclid(span.max(1.0));
                    }
                }
                Loop::PingPong => {
                    *pos += if *back { -step } else { step };
                    if *pos >= loop_end {
                        *pos = loop_end - (*pos - loop_end);
                        *back = true;
                    } else if *back && *pos <= loop_start {
                        *pos = loop_start + (loop_start - *pos);
                        *back = false;
                    }
                }
            }
        }
        let n = s.unison.max(1) as f32;
        (l, r, mono / n)
    }

    /// One sample of grains: the granular oscillator, or the sample
    /// oscillator's stretch mode (`stretch`: grains follow real time and
    /// overlap twice, without spread or random pitch).
    fn grain(&mut self, source: &Source, kind: Kind, stretch: bool) -> f32 {
        let s = self.settings;
        let Some(buffer) = self.buffer(source, kind) else {
            return 0.0;
        };
        let len = buffer.len() as f64;
        let sr = s.sample_rate.max(1.0);
        let rate = buffer.sample_rate() / f64::from(sr);
        let size = (s.grain_seconds * sr).max(2.0) as u32;
        let (interval, spread, jitter, scan) = if stretch {
            (size as f32 / 2.0, 0.0, 0.0, 1.0)
        } else {
            (sr / s.density.max(0.1), s.spread, s.grain_pitch, s.scan)
        };
        let base_step =
            rate * f64::from(s.freq.first().copied().unwrap_or(0.0) / s.root_hz.max(1e-3));
        self.grain_timer -= 1.0;
        if self.grain_timer <= 0.0 {
            self.grain_timer += interval.max(1.0);
            let offset = f64::from(self.random()) * f64::from(spread) * len * 0.5;
            let detune = f64::from(self.random() * jitter);
            let position = (self.scan_pos + offset).rem_euclid(len);
            if let Some(grain) = self.grains.iter_mut().find(|g| g.length == 0) {
                *grain = Grain {
                    position,
                    step: base_step * (detune / 12.0).exp2(),
                    age: 0,
                    length: size,
                };
            }
        }
        self.scan_pos = (self.scan_pos + f64::from(scan) * rate).rem_euclid(len);
        let overlap = (size as f32 / interval.max(1.0)).max(1.0);
        let mut sum = 0.0;
        for grain in self.grains.iter_mut().filter(|g| g.length > 0) {
            let x = grain.age as f32 / grain.length as f32;
            let window = 0.5 - 0.5 * (x * std::f32::consts::TAU).cos();
            let (gl, gr) = buffer.read(grain.position.rem_euclid(len));
            sum += (gl + gr) * 0.5 * window;
            grain.position += grain.step;
            grain.age += 1;
            if grain.age >= grain.length {
                grain.length = 0;
            }
        }
        sum / overlap.sqrt()
    }
}

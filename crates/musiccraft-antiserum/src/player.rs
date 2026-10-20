//! The sample, multisample, granular and spectral oscillators of one voice
//! (`docs/spec/antiserum-oscillators.md`): in stereo, with unison and the
//! warps. Settings are made once per control block; samples are produced
//! one at a time without allocating.

use musiccraft_dsp::SampleBuffer;
use musiccraft_dsp::resynth::RESYNTH_PARTIALS;

use crate::source::Source;
use crate::voice::MAX_UNISON;
use crate::warp::Warp;

/// Grains playing at once (every unison voice's together).
const GRAINS: usize = 64;
/// Entries in the sine table (one cycle; one more for interpolation).
pub const SINE_LEN: usize = 1_024;
/// The longest hold of the quantize warp on sample types, in frames.
const MAX_HOLD: f32 = 32.0;
/// Points in the spectral oscillator's built cycle.
const CYCLE: usize = 512;
/// Frames between rebuilds of the spectral cycle (it crossfades to each new
/// one over this time).
const REBUILD: u32 = 1_024;
/// Partials a stretched spectral oscillator plays, shared by its unison
/// voices.
const STRETCH_BUDGET: usize = 48;
/// Partials each unison voice of a stretched oscillator plays at least.
const STRETCH_MIN: usize = 3;

/// The spectral cycle's tables: left sine, left cosine, right sine, right
/// cosine.
type Cycles = [[f32; CYCLE + 1]; 4];

fn read(table: &[f32; CYCLE + 1], phase: f32) -> f32 {
    let phase = if (0.0..1.0).contains(&phase) {
        phase
    } else {
        phase.rem_euclid(1.0)
    };
    let x = phase * CYCLE as f32;
    let i = (x as usize).min(CYCLE - 1);
    let t = x - i as f32;
    let a = table.get(i).copied().unwrap_or(0.0);
    let b = table.get(i + 1).copied().unwrap_or(0.0);
    a + (b - a) * t
}

/// A sine cycle for the spectral oscillator.
pub(crate) fn sine_table() -> Box<[f32; SINE_LEN + 1]> {
    let mut table = Box::new([0.0; SINE_LEN + 1]);
    for (i, s) in table.iter_mut().enumerate() {
        *s = (std::f32::consts::TAU * i as f32 / SINE_LEN as f32).sin();
    }
    table
}

fn sine(table: &[f32; SINE_LEN + 1], phase: f32) -> f32 {
    // Phases are almost always in 0..1 already; wrapping is slow.
    let phase = if (0.0..1.0).contains(&phase) {
        phase
    } else {
        phase.rem_euclid(1.0)
    };
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
    /// The two warps and their amounts.
    pub warps: [(Warp, f32); 2],
    pub sample_rate: f32,
}

impl Settings {
    /// Whether no warp is on.
    fn plain(&self) -> bool {
        self.warps.iter().all(|(w, _)| *w == Warp::Off)
    }

    /// Cycles of the note the FM warps move a read by, for mod source
    /// `source`.
    fn fm_cycles(&self, source: f32) -> f32 {
        self.warps
            .iter()
            .filter(|(w, _)| *w == Warp::Fm)
            .map(|(_, a)| a.clamp(0.0, 1.0) * 2.0 * source)
            .sum()
    }

    /// The level the level warps (PWM, flip, AM, RM) give a voice at note
    /// cycle phase `phase`.
    fn level(&self, phase: f32, source: f32) -> f32 {
        self.warps
            .iter()
            .filter(|(w, _)| matches!(w, Warp::Pwm | Warp::Flip | Warp::Am | Warp::Rm))
            .map(|(w, a)| w.apply(*a, phase, source).1)
            .product()
    }

    /// Frames the quantize warps hold a sample-type output for.
    fn hold(&self) -> f32 {
        self.warps
            .iter()
            .filter(|(w, _)| *w == Warp::Quantize)
            .map(|(_, a)| 1.0 + a.clamp(0.0, 1.0).powi(2) * (MAX_HOLD - 1.0))
            .fold(1.0, f32::max)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Grain {
    position: f64,
    step: f64,
    age: u32,
    length: u32,
    /// The unison voice it belongs to.
    voice: u8,
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
    /// Per unison voice, a phase at the note's pitch, for the level warps.
    cycle: [f32; MAX_UNISON],
    /// The multisample zone this note plays (`usize::MAX`: none).
    zone: usize,
    grains: [Grain; GRAINS],
    grain_timer: f32,
    scan_pos: f64,
    seed: u32,
    /// Quantize on sample types: the held output and frames left to hold.
    held: (f32, f32, f32),
    hold_left: f32,
    /// Spectral: seconds into the analysis; each partial's left and right
    /// level and its frequency factor (`n^(1 + stretch/2)`).
    time: f32,
    gains: [[f32; RESYNTH_PARTIALS]; 2],
    harmonic: [f32; RESYNTH_PARTIALS],
    /// Unstretched spectral: the built cycle and the one before it, how far
    /// the crossfade between them has come, frames to the next build, and
    /// the phase of the frequency shift.
    now: Cycles,
    before: Cycles,
    fade: f32,
    rebuild_in: u32,
    built: bool,
    shift_phase: f32,
    /// Stretched spectral: every unison voice's partial phases and
    /// increments, and how many partials each voice plays.
    stretched: bool,
    phases: [[f32; RESYNTH_PARTIALS]; MAX_UNISON],
    incs: [[f32; RESYNTH_PARTIALS]; MAX_UNISON],
    budget: usize,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            pos: [0.0; MAX_UNISON],
            backwards: [false; MAX_UNISON],
            stopped: [false; MAX_UNISON],
            cycle: [0.0; MAX_UNISON],
            zone: usize::MAX,
            grains: [Grain::default(); GRAINS],
            grain_timer: 0.0,
            scan_pos: 0.0,
            seed: 1,
            held: (0.0, 0.0, 0.0),
            hold_left: 0.0,
            time: 0.0,
            gains: [[0.0; RESYNTH_PARTIALS]; 2],
            harmonic: [0.0; RESYNTH_PARTIALS],
            now: [[0.0; CYCLE + 1]; 4],
            before: [[0.0; CYCLE + 1]; 4],
            fade: 1.0,
            rebuild_in: 0,
            built: false,
            shift_phase: 0.0,
            stretched: false,
            phases: [[0.0; RESYNTH_PARTIALS]; MAX_UNISON],
            incs: [[0.0; RESYNTH_PARTIALS]; MAX_UNISON],
            budget: RESYNTH_PARTIALS,
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
        self.cycle = [0.0; MAX_UNISON];
        self.grains = [Grain::default(); GRAINS];
        self.grain_timer = 0.0;
        self.scan_pos = f64::from(position) * len;
        self.held = (0.0, 0.0, 0.0);
        self.hold_left = 0.0;
        self.time = position * source.harmonics[0].duration();
        self.built = false;
        self.fade = 1.0;
        self.rebuild_in = 0;
        self.shift_phase = 0.0;
        // Unison voices start at different phases, as detuned oscillators
        // drift apart.
        for (u, phase) in self.cycle.iter_mut().enumerate() {
            *phase = ((u * 7) % 16) as f32 / 16.0;
        }
        for (u, phases) in self.phases.iter_mut().enumerate() {
            for (n, phase) in phases.iter_mut().enumerate() {
                *phase = if u == 0 {
                    0.0
                } else {
                    ((u * 7 + n * 13) % 31) as f32 / 31.0
                };
            }
        }
    }

    /// The sample this player reads.
    fn buffer<'s>(&self, source: &'s Source, kind: Kind) -> Option<&'s SampleBuffer> {
        match kind {
            Kind::Multisample => source.zones.get(self.zone).map(|z| &*z.buffer),
            _ => source.sample.as_deref(),
        }
    }

    /// Takes this block's settings; `frames` is the block's length.
    pub fn control(
        &mut self,
        source: &Source,
        settings: Settings,
        frames: usize,
        sine_table: &[f32; SINE_LEN + 1],
    ) {
        self.settings = settings;
        if settings.kind != Kind::Spectral {
            return;
        }
        let looped = settings.looping != Loop::Off;
        let left = source.harmonics[0].at(self.time, looped);
        let right = source.harmonics[1].at(self.time, looped);
        self.time += frames as f32 / settings.sample_rate.max(1.0) * settings.speed;
        let exponent = 1.0 + settings.stretch * 0.5;
        let mut plain = 0.0;
        let mut weighted = 0.0;
        for n in 0..RESYNTH_PARTIALS {
            let nf = (n + 1) as f32;
            let weight = nf.powf(2.0 * settings.tilt);
            let (l, r) = (
                left.get(n).copied().unwrap_or(0.0),
                right.get(n).copied().unwrap_or(0.0),
            );
            if let Some(h) = self.harmonic.get_mut(n) {
                *h = nf.powf(exponent);
            }
            let [gains_l, gains_r] = &mut self.gains;
            if let (Some(gl), Some(gr)) = (gains_l.get_mut(n), gains_r.get_mut(n)) {
                *gl = l * weight;
                *gr = r * weight;
            }
            plain += (l + r) * 0.5;
            weighted += (l + r) * 0.5 * weight;
        }
        // Tilt reshapes the spectrum without changing the overall level.
        let peak = source.harmonics[0]
            .peak_sum
            .max(source.harmonics[1].peak_sum)
            .max(1e-3);
        let norm = if weighted > 1e-9 {
            plain / weighted / peak
        } else {
            0.0
        };
        for gain in self.gains.iter_mut().flatten() {
            *gain *= norm;
        }
        self.stretched = settings.stretch.abs() > 1e-4;
        let sr = settings.sample_rate.max(1.0);
        if self.stretched {
            // Partials one by one, within a budget shared by the voices.
            self.budget =
                (STRETCH_BUDGET / settings.unison.max(1)).clamp(STRETCH_MIN, RESYNTH_PARTIALS);
            for (incs, freq) in self
                .incs
                .iter_mut()
                .zip(settings.freq)
                .take(settings.unison)
            {
                for (inc, h) in incs.iter_mut().zip(self.harmonic) {
                    *inc = (freq * h + settings.shift) / sr;
                }
            }
            self.built = false;
        } else {
            self.rebuild_in = self.rebuild_in.saturating_sub(frames as u32);
            if !self.built || self.rebuild_in == 0 {
                self.build(sine_table);
            }
        }
    }

    /// Builds the current spectrum into one cycle (sine and cosine phase,
    /// both channels), band-limited for the highest unison voice, and starts
    /// crossfading to it.
    fn build(&mut self, sine_table: &[f32; SINE_LEN + 1]) {
        let s = self.settings;
        let top = s
            .freq
            .iter()
            .take(s.unison.max(1))
            .fold(0.0_f32, |m, f| m.max(*f));
        let nyquist = s.sample_rate * 0.5;
        if self.built {
            self.before = self.now;
            self.fade = 0.0;
        }
        for table in &mut self.now {
            table.fill(0.0);
        }
        let step = SINE_LEN / CYCLE;
        let quarter = SINE_LEN / 4;
        for n in 0..RESYNTH_PARTIALS {
            let h = n + 1;
            if top * h as f32 + s.shift.max(0.0) >= nyquist {
                break;
            }
            let gl = self.gains[0].get(n).copied().unwrap_or(0.0);
            let gr = self.gains[1].get(n).copied().unwrap_or(0.0);
            if gl == 0.0 && gr == 0.0 {
                continue;
            }
            let [ls, lc, rs, rc] = &mut self.now;
            for (k, (((ls, lc), rs), rc)) in ls
                .iter_mut()
                .zip(lc.iter_mut())
                .zip(rs.iter_mut())
                .zip(rc.iter_mut())
                .take(CYCLE)
                .enumerate()
            {
                let i = (h * k * step) % SINE_LEN;
                let sin = sine_table.get(i).copied().unwrap_or(0.0);
                let cos = sine_table
                    .get((i + quarter) % SINE_LEN)
                    .copied()
                    .unwrap_or(0.0);
                *ls += gl * sin;
                *lc += gl * cos;
                *rs += gr * sin;
                *rc += gr * cos;
            }
        }
        // The extra point repeats the first, for interpolation.
        for table in &mut self.now {
            if let Some(first) = table.first().copied()
                && let Some(last) = table.last_mut()
            {
                *last = first;
            }
        }
        if !self.built {
            self.before = self.now;
            self.fade = 1.0;
        }
        self.built = true;
        self.rebuild_in = REBUILD;
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// Moves every unison voice's note-pitch phase on by one frame.
    fn advance_cycles(&mut self) {
        let s = self.settings;
        let sr = s.sample_rate.max(1.0);
        for (phase, freq) in self.cycle.iter_mut().zip(s.freq).take(s.unison) {
            *phase = (*phase + freq / sr).fract();
        }
    }

    /// The next sample: (left, right) with the unison voices' gains, and
    /// the mono mix for cross-modulation. `src` is the oscillator's mod
    /// source (for the FM, AM and RM warps).
    pub fn next(
        &mut self,
        source: &Source,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        sine_table: &[f32; SINE_LEN + 1],
        src: f32,
    ) -> (f32, f32, f32) {
        let s = self.settings;
        if s.kind == Kind::Wavetable {
            return (0.0, 0.0, 0.0);
        }
        if s.kind == Kind::Spectral {
            return self.spectral(gains, sine_table, src);
        }
        // Quantize holds the output of the sample types.
        if self.hold_left >= 1.0 {
            self.hold_left -= 1.0;
            self.advance_cycles();
            self.skip(source);
            return self.held;
        }
        let out = match s.kind {
            Kind::Sample if s.mode == Mode::Stretch => self.grain(source, gains, src, true),
            Kind::Granular => self.grain(source, gains, src, false),
            _ => self.sample(source, gains, src),
        };
        self.advance_cycles();
        let hold = s.hold();
        if hold > 1.0 {
            self.held = out;
            self.hold_left += hold - 1.0;
        }
        out
    }

    /// Moves the read positions on without producing output (while
    /// quantize holds).
    fn skip(&mut self, source: &Source) {
        let s = self.settings;
        if matches!(s.kind, Kind::Sample | Kind::Multisample) && s.mode != Mode::Stretch {
            let _ = self.sample(source, (&[0.0; MAX_UNISON], &[0.0; MAX_UNISON]), 0.0);
        } else {
            let stretch = s.kind == Kind::Sample && s.mode == Mode::Stretch;
            let _ = self.grain(
                source,
                (&[0.0; MAX_UNISON], &[0.0; MAX_UNISON]),
                0.0,
                stretch,
            );
        }
    }

    /// The spectral oscillator, each unison voice with its own left and
    /// right level: unstretched, the built cycle played like a wavetable
    /// (warps reshape it the same way) and moved by the frequency shift;
    /// stretched, partials one by one, each warped as a one-partial cycle.
    fn spectral(
        &mut self,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        sine_table: &[f32; SINE_LEN + 1],
        src: f32,
    ) -> (f32, f32, f32) {
        let out = if self.stretched {
            self.partials(gains, sine_table, src)
        } else {
            self.cycle_voices(gains, sine_table, src)
        };
        self.advance_cycles();
        out
    }

    /// Unstretched spectral playback from the built cycle.
    fn cycle_voices(
        &mut self,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        sine_table: &[f32; SINE_LEN + 1],
        src: f32,
    ) -> (f32, f32, f32) {
        let s = self.settings;
        let plain = s.plain();
        let [(w1, a1), (w2, a2)] = s.warps;
        let shifted = s.shift != 0.0;
        // sin(nφ + θ) = sin(nφ)·cos θ + cos(nφ)·sin θ moves every partial by
        // the shift (θ runs at the shift's frequency).
        let (shift_sin, shift_cos) = if shifted {
            (
                sine(sine_table, self.shift_phase),
                sine(sine_table, self.shift_phase + 0.25),
            )
        } else {
            (0.0, 1.0)
        };
        let fading = self.fade < 1.0;
        let fade = self.fade;
        let tables = |cycles: &Cycles, p: f32| -> (f32, f32) {
            let [ls, lc, rs, rc] = cycles;
            let (mut l, mut r) = (read(ls, p) * shift_cos, read(rs, p) * shift_cos);
            if shifted {
                l += read(lc, p) * shift_sin;
                r += read(rc, p) * shift_sin;
            }
            (l, r)
        };
        let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
        for u in 0..s.unison.min(MAX_UNISON) {
            let (Some(phase), Some(gl), Some(gr)) =
                (self.cycle.get(u), gains.0.get(u), gains.1.get(u))
            else {
                break;
            };
            let (p, g) = if plain {
                (*phase, 1.0)
            } else {
                let (p1, g1) = w1.apply(a1, *phase, src);
                let (p2, g2) = w2.apply(a2, p1, src);
                (p2, g1 * g2)
            };
            let (mut vl, mut vr) = tables(&self.now, p);
            if fading {
                let (bl, br) = tables(&self.before, p);
                vl = bl + (vl - bl) * fade;
                vr = br + (vr - br) * fade;
            }
            l += vl * g * gl;
            r += vr * g * gr;
            mono += (vl + vr) * 0.5 * g;
        }
        if fading {
            self.fade = (self.fade + 1.0 / REBUILD as f32).min(1.0);
        }
        if shifted {
            self.shift_phase =
                (self.shift_phase + s.shift / s.sample_rate.max(1.0)).rem_euclid(1.0);
        }
        (l, r, mono / s.unison.max(1) as f32)
    }

    /// Stretched spectral playback, partial by partial, within the budget.
    fn partials(
        &mut self,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        sine_table: &[f32; SINE_LEN + 1],
        src: f32,
    ) -> (f32, f32, f32) {
        let s = self.settings;
        let plain = s.plain();
        let [(w1, a1), (w2, a2)] = s.warps;
        let budget = self.budget;
        let Self {
            phases: all_phases,
            incs: all_incs,
            gains: [levels_l, levels_r],
            ..
        } = self;
        let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
        for u in 0..s.unison.min(MAX_UNISON) {
            let (Some(phases), Some(incs), Some(gl), Some(gr)) = (
                all_phases.get_mut(u),
                all_incs.get(u),
                gains.0.get(u),
                gains.1.get(u),
            ) else {
                break;
            };
            let (mut vl, mut vr) = (0.0, 0.0);
            for (((phase, inc), left), right) in phases
                .iter_mut()
                .zip(incs)
                .zip(levels_l.iter())
                .zip(levels_r.iter())
                .take(budget)
            {
                let (left, right) = (*left, *right);
                if *inc > 0.0 && *inc < 0.5 && (left != 0.0 || right != 0.0) {
                    let x = if plain {
                        sine(sine_table, *phase)
                    } else {
                        let (p1, g1) = w1.apply(a1, *phase, src);
                        let (p2, g2) = w2.apply(a2, p1, src);
                        sine(sine_table, p2) * g1 * g2
                    };
                    vl += x * left;
                    vr += x * right;
                }
                *phase += inc.max(0.0);
                if *phase >= 1.0 {
                    *phase -= 1.0;
                }
            }
            l += vl * gl;
            r += vr * gr;
            mono += (vl + vr) * 0.5;
        }
        (l, r, mono / s.unison.max(1) as f32)
    }

    /// Sample and multisample playback, every unison voice, in stereo.
    fn sample(
        &mut self,
        source: &Source,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        src: f32,
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
        let fm = f64::from(s.fm_cycles(src));
        let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
        for u in 0..s.unison.min(MAX_UNISON) {
            let level = s.level(self.cycle.get(u).copied().unwrap_or(0.0), src);
            let freq = s.freq.get(u).copied().unwrap_or(0.0);
            let step = rate * f64::from(freq / reference.max(1e-3));
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
            // FM moves the read by cycles of the note: frames per cycle is
            // the step over the cycle's increment.
            let cycle_frames = step * f64::from(s.sample_rate.max(1.0)) / f64::from(freq.max(1e-3));
            let at = (*pos + fm * cycle_frames).clamp(0.0, (len - 1.0).max(0.0));
            let (sl, sr) = buffer.read(at);
            mono += (sl + sr) * 0.5 * level;
            l += sl * level * gains.0.get(u).copied().unwrap_or(0.0);
            r += sr * level * gains.1.get(u).copied().unwrap_or(0.0);
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

    /// One sample of grains, in stereo: the granular oscillator, or the
    /// sample oscillator's stretch mode (`stretch`: grains follow real time
    /// and overlap twice, without spread or random pitch). Each new grain
    /// starts once per unison voice.
    fn grain(
        &mut self,
        source: &Source,
        gains: (&[f32; MAX_UNISON], &[f32; MAX_UNISON]),
        src: f32,
        stretch: bool,
    ) -> (f32, f32, f32) {
        let s = self.settings;
        let kind = if stretch { Kind::Sample } else { s.kind };
        let Some(buffer) = self.buffer(source, kind) else {
            return (0.0, 0.0, 0.0);
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
        let unison = s.unison.clamp(1, MAX_UNISON);
        self.grain_timer -= 1.0;
        if self.grain_timer <= 0.0 {
            self.grain_timer += interval.max(1.0);
            let offset = f64::from(self.random()) * f64::from(spread) * len * 0.5;
            let position = (self.scan_pos + offset).rem_euclid(len);
            for u in 0..unison {
                let detune = f64::from(self.random() * jitter);
                let freq = s.freq.get(u).copied().unwrap_or(0.0);
                let step = rate * f64::from(freq / s.root_hz.max(1e-3)) * (detune / 12.0).exp2();
                if let Some(grain) = self.grains.iter_mut().find(|g| g.length == 0) {
                    *grain = Grain {
                        position,
                        step,
                        age: 0,
                        length: size,
                        voice: u as u8,
                    };
                }
            }
        }
        self.scan_pos = (self.scan_pos + f64::from(scan) * rate).rem_euclid(len);
        let overlap = (size as f32 / interval.max(1.0)).max(1.0);
        let fm = f64::from(s.fm_cycles(src));
        let mut levels = [1.0; MAX_UNISON];
        for (level, phase) in levels.iter_mut().zip(self.cycle).take(unison) {
            *level = s.level(phase, src);
        }
        let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
        for grain in self.grains.iter_mut().filter(|g| g.length > 0) {
            let u = usize::from(grain.voice);
            let x = grain.age as f32 / grain.length as f32;
            let window = (0.5 - 0.5 * (x * std::f32::consts::TAU).cos())
                * levels.get(u).copied().unwrap_or(1.0);
            let freq = s.freq.get(u).copied().unwrap_or(0.0);
            let cycle_frames = grain.step * f64::from(sr) / f64::from(freq.max(1e-3));
            let (gl, gr) = buffer.read((grain.position + fm * cycle_frames).rem_euclid(len));
            mono += (gl + gr) * 0.5 * window;
            l += gl * window * gains.0.get(u).copied().unwrap_or(0.0);
            r += gr * window * gains.1.get(u).copied().unwrap_or(0.0);
            grain.position += grain.step;
            grain.age += 1;
            if grain.age >= grain.length {
                grain.length = 0;
            }
        }
        let norm = overlap.sqrt();
        (l / norm, r / norm, mono / norm / unison as f32)
    }
}

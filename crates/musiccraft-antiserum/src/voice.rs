//! One Antiserum voice: three wavetable oscillators with unison, sub, noise,
//! two filters, envelopes, LFOs and the mod matrix. Modulation runs once per
//! control block; the audio path per sample. Nothing here allocates after
//! [`Voice::new`].

use std::sync::Arc;

use musiccraft_dsp::{Oscillator, Waveform, key_to_hz, pan_gains};
use musiccraft_params::ParamSpec;

use crate::filter::{Filter, FilterType};
use crate::modulation::{
    EnvParams, Envelope, Lfo, LfoParams, LfoShape, shape as curve_shape, slot_curve,
};
use crate::noise::{NOISE_LEN, NOISE_RATE, NoiseBank};
use crate::params::{
    BEND_RANGE, ENVS, FILTER_ROUTING, FILTERS, GLIDE_CURVE, LFOS, MACRO1, MACROS, NOISE_BASE, OSCS,
    PARAM_COUNT, SLOTS, SOURCES, SUB_BASE, SYNC_RATES, VELOCITY, VOLUME, env, env_param, filter,
    filter_param, lfo, lfo_param, noise, osc, osc_param, slot, slot_param, source, sub,
};
use crate::warp::Warp;
use crate::wavetable::{FramePair, Wavetable};

/// Samples per control block.
pub const CONTROL: usize = 32;
/// Most unison voices per oscillator.
pub const MAX_UNISON: usize = 16;
/// Stolen voices fade out over this long.
const FADE_SECONDS: f32 = 0.002;

/// Everything a voice reads that the synth owns.
pub(crate) struct Ctx<'a> {
    pub specs: &'a [ParamSpec],
    pub base: &'a [f32; PARAM_COUNT],
    /// `base` as knob positions (0..1), for modulation.
    pub unit: &'a [f32; PARAM_COUNT],
    pub tables: &'a [Arc<Wavetable>; OSCS],
    pub noise: &'a NoiseBank,
    pub shapes: &'a [LfoShape; LFOS],
    pub free_phase: &'a [f32; LFOS],
    pub sample_rate: f32,
    pub tempo: f32,
    /// −1..1.
    pub bend: f32,
    pub wheel: f32,
    pub aftertouch: f32,
}

impl Ctx<'_> {
    fn base(&self, index: usize) -> f32 {
        self.base.get(index).copied().unwrap_or(0.0)
    }

    fn on(&self, index: usize) -> bool {
        self.base(index) >= 0.5
    }
}

/// A voice's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    Idle,
    Playing,
    /// Stolen: fading out, then idle.
    Fading,
}

/// Where cross-modulation reads: the previous sample of each source.
const SRC_SUB: usize = 3;
const SRC_NOISE: usize = 4;

/// One oscillator's state and its settings for the current block.
#[derive(Debug, Clone, Copy, Default)]
struct OscRun {
    on: bool,
    unison: usize,
    phases: [f32; MAX_UNISON],
    inc: [f32; MAX_UNISON],
    left: [f32; MAX_UNISON],
    right: [f32; MAX_UNISON],
    level: usize,
    frame_a: usize,
    frame_b: usize,
    frame_mix: f32,
    warp1: Warp,
    amount1: f32,
    warp2: Warp,
    amount2: f32,
    source: usize,
    /// Filter 1, filter 2, dry.
    sends: [f32; 3],
}

/// The sub, noise or an oscillator's mix settings.
#[derive(Debug, Clone, Copy, Default)]
struct Mix {
    on: bool,
    left: f32,
    right: f32,
    sends: [f32; 3],
}

/// One voice.
pub(crate) struct Voice {
    pub state: State,
    pub key: u8,
    /// Key still down (or sustained by the host).
    pub held: bool,
    pub age: u64,
    pub poly_aftertouch: f32,
    velocity: f32,
    random: f32,
    seed: u32,
    /// Current pitch (key plus glide), in semitones.
    pitch: f32,
    target: f32,
    glide_from: f32,
    glide_t: f32,
    glide_time: f32,
    oscs: [OscRun; OSCS],
    /// Previous sample of osc 1–3, sub and noise, for cross-modulation.
    prev: [f32; 5],
    sub: Oscillator,
    sub_wave: Waveform,
    sub_mix: Mix,
    noise_mix: Mix,
    noise_kind: usize,
    noise_pos: f32,
    noise_rate: f32,
    noise_one_shot: bool,
    /// `[filter][channel]`.
    filters: [[Filter; 2]; FILTERS],
    filter_on: [bool; FILTERS],
    filter_mix: [f32; FILTERS],
    parallel: bool,
    envs: [Envelope; ENVS],
    lfos: [Lfo; LFOS],
    sources: [f32; SOURCES.len()],
    /// Modulation, as knob-position offsets, by parameter.
    offsets: Box<[f32; PARAM_COUNT]>,
    touched: [usize; SLOTS],
    touched_len: usize,
    amp: f32,
    amp_target: f32,
    fade: f32,
    sample_rate: f32,
}

impl Voice {
    /// An idle voice. Allocates its filters' delay lines.
    pub fn new(sample_rate: f32) -> Self {
        let filter = || [Filter::new(sample_rate), Filter::new(sample_rate)];
        Self {
            state: State::Idle,
            key: 0,
            held: false,
            age: 0,
            poly_aftertouch: 0.0,
            velocity: 0.0,
            random: 0.0,
            seed: 1,
            pitch: 60.0,
            target: 60.0,
            glide_from: 60.0,
            glide_t: 1.0,
            glide_time: 0.0,
            oscs: [OscRun::default(); OSCS],
            prev: [0.0; 5],
            sub: Oscillator::new(),
            sub_wave: Waveform::Sine,
            sub_mix: Mix::default(),
            noise_mix: Mix::default(),
            noise_kind: 0,
            noise_pos: 0.0,
            noise_rate: 1.0,
            noise_one_shot: false,
            filters: [filter(), filter()],
            filter_on: [false; FILTERS],
            filter_mix: [1.0; FILTERS],
            parallel: false,
            envs: [Envelope::default(); ENVS],
            lfos: [Lfo::default(); LFOS],
            sources: [0.0; SOURCES.len()],
            offsets: Box::new([0.0; PARAM_COUNT]),
            touched: [0; SLOTS],
            touched_len: 0,
            amp: 0.0,
            amp_target: 0.0,
            fade: 1.0,
            sample_rate: sample_rate.max(1.0),
        }
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1 << 24) as f32
    }

    /// Starts a note from silence. `glide_from` is the previous note's pitch
    /// when this one should glide.
    pub fn start(
        &mut self,
        ctx: &Ctx<'_>,
        key: u8,
        velocity: f32,
        seed: u32,
        age: u64,
        glide_from: Option<f32>,
    ) {
        self.state = State::Playing;
        self.held = true;
        self.age = age;
        self.poly_aftertouch = 0.0;
        self.seed = seed | 1;
        self.random = self.random();
        self.prev = [0.0; 5];
        self.amp = 0.0;
        self.fade = 1.0;
        for (o, run) in self.oscs.iter_mut().enumerate() {
            let start = ctx.base(osc_param(o, osc::PHASE));
            let spread = ctx.base(osc_param(o, osc::RANDOM_PHASE));
            for phase in &mut run.phases {
                self.seed ^= self.seed << 13;
                self.seed ^= self.seed >> 17;
                self.seed ^= self.seed << 5;
                let r = (self.seed >> 8) as f32 / (1 << 24) as f32;
                *phase = (start + spread * r).fract();
            }
        }
        self.sub.reset(0.0);
        let one_shot = ctx.on(NOISE_BASE + noise::ONE_SHOT);
        self.noise_pos = if one_shot {
            0.0
        } else {
            self.random() * NOISE_LEN as f32
        };
        for pair in &mut self.filters {
            for filter in pair {
                filter.reset();
            }
        }
        for env in &mut self.envs {
            env.reset();
            env.gate_on();
        }
        for (i, lfo) in self.lfos.iter_mut().enumerate() {
            lfo.start(seed.wrapping_mul(2_654_435_761).wrapping_add(i as u32));
        }
        self.offsets.fill(0.0);
        self.touched_len = 0;
        self.set_key(ctx, key, velocity, glide_from);
    }

    /// Moves a playing voice to a new key: retriggering its envelopes (mono)
    /// or not (legato).
    pub fn retarget(
        &mut self,
        ctx: &Ctx<'_>,
        key: u8,
        velocity: f32,
        retrigger: bool,
        glide_from: Option<f32>,
    ) {
        self.held = true;
        if retrigger {
            for env in &mut self.envs {
                env.gate_on();
            }
        }
        self.set_key(ctx, key, velocity, glide_from);
    }

    fn set_key(&mut self, ctx: &Ctx<'_>, key: u8, velocity: f32, glide_from: Option<f32>) {
        self.key = key;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.target = f32::from(key);
        self.glide_time = ctx.base(crate::params::GLIDE);
        match glide_from {
            Some(from) if self.glide_time > 0.0 => {
                self.glide_from = from;
                self.glide_t = 0.0;
                self.pitch = from;
            }
            _ => {
                self.glide_t = 1.0;
                self.pitch = self.target;
            }
        }
    }

    /// The pitch it is sounding now (for the next note's glide).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Key up: every envelope releases.
    pub fn release(&mut self) {
        self.held = false;
        for env in &mut self.envs {
            env.gate_off();
        }
    }

    /// Whether it is releasing (key up, still sounding).
    pub fn releasing(&self) -> bool {
        self.state == State::Playing && !self.held
    }

    /// Fades out quickly (a stolen voice).
    pub fn steal(&mut self) {
        if self.state == State::Playing {
            self.state = State::Fading;
            self.fade = 1.0;
        }
    }

    /// Stops at once.
    pub fn kill(&mut self) {
        self.state = State::Idle;
        self.held = false;
    }

    /// This voice's modulation of parameter `index`, as a knob-position
    /// offset (the effects follow the newest voice's).
    pub fn offset(&self, index: usize) -> f32 {
        self.offsets.get(index).copied().unwrap_or(0.0)
    }

    /// A parameter's value with this voice's modulation.
    fn value(&self, ctx: &Ctx<'_>, index: usize) -> f32 {
        let base = ctx.base(index);
        let offset = self.offsets.get(index).copied().unwrap_or(0.0);
        if offset == 0.0 {
            return base;
        }
        match (ctx.specs.get(index), ctx.unit.get(index)) {
            (Some(spec), Some(unit)) => spec.from_unit(unit + offset),
            _ => base,
        }
    }

    /// Runs modulation for the next `frames` samples and sets up the audio
    /// path for them.
    pub fn control(&mut self, ctx: &Ctx<'_>, frames: usize) {
        let dt = frames as f32 / ctx.sample_rate;
        self.update_sources(ctx, dt);
        self.update_matrix(ctx);
        self.update_glide(ctx, dt);
        let pitch = self.pitch + ctx.bend * ctx.base(BEND_RANGE);
        for o in 0..OSCS {
            self.update_osc(ctx, o, pitch);
        }
        self.update_sub_noise(ctx, pitch);
        self.update_filters(ctx, pitch);
        let velocity = self.value(ctx, VELOCITY);
        let volume = self.value(ctx, VOLUME);
        let env = self.sources.get(source::ENV1).copied().unwrap_or(0.0);
        self.amp_target = env * (1.0 - velocity + velocity * self.velocity) * volume * volume;
    }

    fn update_sources(&mut self, ctx: &Ctx<'_>, dt: f32) {
        for e in 0..ENVS {
            let p = EnvParams {
                attack: self.value(ctx, env_param(e, env::ATTACK)),
                hold: self.value(ctx, env_param(e, env::HOLD)),
                decay: self.value(ctx, env_param(e, env::DECAY)),
                sustain: self.value(ctx, env_param(e, env::SUSTAIN)),
                release: self.value(ctx, env_param(e, env::RELEASE)),
                attack_curve: self.value(ctx, env_param(e, env::ATTACK_CURVE)),
                decay_curve: self.value(ctx, env_param(e, env::DECAY_CURVE)),
                release_curve: self.value(ctx, env_param(e, env::RELEASE_CURVE)),
            };
            let level = self.envs.get_mut(e).map_or(0.0, |env| env.advance(dt, &p));
            self.set_source(source::ENV1 + e, level);
        }
        for l in 0..LFOS {
            let rate = if ctx.on(lfo_param(l, lfo::SYNC)) {
                let index = ctx.base(lfo_param(l, lfo::SYNC_RATE)) as usize;
                let beats = SYNC_RATES.get(index).map_or(1.0, |r| r.1);
                ctx.tempo / 60.0 / beats
            } else {
                self.value(ctx, lfo_param(l, lfo::RATE))
            };
            let p = LfoParams {
                shape: ctx.base(lfo_param(l, lfo::SHAPE)) as u8,
                mode: ctx.base(lfo_param(l, lfo::MODE)) as u8,
                rate,
                delay: self.value(ctx, lfo_param(l, lfo::DELAY)),
                rise: self.value(ctx, lfo_param(l, lfo::RISE)),
                phase: self.value(ctx, lfo_param(l, lfo::PHASE)),
            };
            let free = ctx.free_phase.get(l).copied().unwrap_or(0.0);
            let value = match (self.lfos.get_mut(l), ctx.shapes.get(l)) {
                (Some(lfo), Some(shape)) => lfo.advance(dt, &p, free, shape),
                _ => 0.0,
            };
            self.set_source(source::LFO1 + l, value);
        }
        let aftertouch = if self.poly_aftertouch > 0.0 {
            self.poly_aftertouch
        } else {
            ctx.aftertouch
        };
        self.set_source(source::VELOCITY, self.velocity);
        self.set_source(source::NOTE, f32::from(self.key) / 127.0);
        self.set_source(source::AFTERTOUCH, aftertouch);
        self.set_source(source::MOD_WHEEL, ctx.wheel);
        self.set_source(source::PITCH_BEND, ctx.bend * 0.5 + 0.5);
        self.set_source(source::RANDOM, self.random);
        for m in 0..MACROS {
            let value = self.value(ctx, MACRO1 + m);
            self.set_source(source::MACRO1 + m, value);
        }
    }

    fn set_source(&mut self, index: usize, value: f32) {
        if let Some(slot) = self.sources.get_mut(index) {
            *slot = value.clamp(0.0, 1.0);
        }
    }

    fn update_matrix(&mut self, ctx: &Ctx<'_>) {
        for &index in self.touched.iter().take(self.touched_len) {
            if let Some(offset) = self.offsets.get_mut(index) {
                *offset = 0.0;
            }
        }
        self.touched_len = 0;
        for s in 0..SLOTS {
            let src = ctx.base(slot_param(s, slot::SOURCE)) as usize;
            let dest = ctx.base(slot_param(s, slot::DEST)) as usize;
            if src == source::NONE || dest == 0 {
                continue;
            }
            let index = dest - 1;
            if ctx.specs.get(index).is_none_or(|spec| spec.integer) {
                continue;
            }
            let amount = ctx.base(slot_param(s, slot::AMOUNT));
            let curve = ctx.base(slot_param(s, slot::CURVE));
            let mut v = slot_curve(self.sources.get(src).copied().unwrap_or(0.0), curve);
            if ctx.on(slot_param(s, slot::BIPOLAR)) {
                v = v * 2.0 - 1.0;
            }
            let aux = ctx.base(slot_param(s, slot::AUX)) as usize;
            if aux != source::NONE {
                v *= self.sources.get(aux).copied().unwrap_or(0.0);
            }
            if let Some(offset) = self.offsets.get_mut(index) {
                *offset += amount * v;
            }
            if let Some(t) = self.touched.get_mut(self.touched_len) {
                *t = index;
                self.touched_len += 1;
            }
        }
    }

    fn update_glide(&mut self, ctx: &Ctx<'_>, dt: f32) {
        if self.glide_t >= 1.0 || self.glide_time <= 0.0 {
            self.pitch = self.target;
            return;
        }
        self.glide_t = (self.glide_t + dt / self.glide_time).min(1.0);
        let t = curve_shape(self.glide_t, ctx.base(GLIDE_CURVE));
        self.pitch = self.glide_from + (self.target - self.glide_from) * t;
    }

    fn update_osc(&mut self, ctx: &Ctx<'_>, o: usize, pitch: f32) {
        let p = |field| osc_param(o, field);
        let on = ctx.on(p(osc::ON));
        let Some(table) = ctx.tables.get(o) else {
            return;
        };
        let semis = pitch
            + 12.0 * ctx.base(p(osc::OCTAVE))
            + ctx.base(p(osc::SEMI))
            + self.value(ctx, p(osc::FINE)) / 100.0;
        let hz = key_to_hz(semis);
        let unison = (ctx.base(p(osc::UNISON)) as usize).clamp(1, MAX_UNISON);
        let detune = self.value(ctx, p(osc::DETUNE));
        let spread = detune * detune * 100.0;
        let blend = self.value(ctx, p(osc::BLEND));
        let width = self.value(ctx, p(osc::WIDTH));
        let pan = self.value(ctx, p(osc::PAN));
        let level = self.value(ctx, p(osc::LEVEL));
        let stack = ctx.base(p(osc::STACK)) as usize;
        let amount1 = self.value(ctx, p(osc::WARP1_AMOUNT));
        let amount2 = self.value(ctx, p(osc::WARP2_AMOUNT));
        let position = self.value(ctx, p(osc::POSITION));
        let f1 = self.value(ctx, p(osc::FILTER1));
        let f2 = self.value(ctx, p(osc::FILTER2));
        let source = match ctx.base(p(osc::MOD_SOURCE)) as usize {
            0 => usize::from(o == 0),
            1 => {
                if o == 2 {
                    1
                } else {
                    2
                }
            }
            2 => SRC_SUB,
            _ => SRC_NOISE,
        };
        let Some(run) = self.oscs.get_mut(o) else {
            return;
        };
        run.on = on;
        if !on {
            return;
        }
        run.unison = unison;
        run.warp1 = Warp::from_param(ctx.base(p(osc::WARP1)));
        run.warp2 = Warp::from_param(ctx.base(p(osc::WARP2)));
        run.amount1 = amount1;
        run.amount2 = amount2;
        run.source = source;
        run.sends = [f1, f2, (1.0 - f1 - f2).max(0.0)];
        let mut max_inc = 0.0_f32;
        let mut power = 0.0;
        for u in 0..unison {
            let x = if unison > 1 {
                -1.0 + 2.0 * u as f32 / (unison - 1) as f32
            } else {
                0.0
            };
            let octaves = match (stack, u % 2, u % 4) {
                (1, 1, _) => 1.0,
                (2, 1, _) => 2.0,
                (3, 1, _) => -1.0,
                (4, _, 1) => 1.0,
                (4, _, 3) => -1.0,
                _ => 0.0,
            };
            let ratio = (x * spread / 1_200.0 + octaves).exp2();
            let inc = (hz * ratio / ctx.sample_rate).clamp(0.0, 0.49);
            max_inc = max_inc.max(inc);
            let centre = unison == 1 || u == unison / 2 || (unison % 2 == 0 && u + 1 == unison / 2);
            let gain = if centre { 1.0 } else { blend };
            power += gain * gain;
            let (l, r) = pan_gains(pan + x * width);
            if let (Some(i), Some(gl), Some(gr)) = (
                run.inc.get_mut(u),
                run.left.get_mut(u),
                run.right.get_mut(u),
            ) {
                *i = inc;
                *gl = gain * l;
                *gr = gain * r;
            }
        }
        let norm = level / f32::sqrt(power.max(1e-6));
        for (l, r) in run.left.iter_mut().zip(run.right.iter_mut()).take(unison) {
            *l *= norm;
            *r *= norm;
        }
        let speed = run.warp1.speed(amount1) * run.warp2.speed(amount2);
        run.level = table.level_for(max_inc * speed);
        let (a, b, mix) = table.frame_pos(position);
        run.frame_a = a;
        run.frame_b = b;
        run.frame_mix = mix;
    }

    fn update_sub_noise(&mut self, ctx: &Ctx<'_>, pitch: f32) {
        let s = |field| SUB_BASE + field;
        let n = |field| NOISE_BASE + field;
        self.sub_mix = self.mix(
            ctx,
            ctx.on(s(sub::ON)),
            [s(sub::LEVEL), s(sub::PAN)],
            [s(sub::FILTER1), s(sub::FILTER2)],
        );
        self.sub_wave = match ctx.base(s(sub::SHAPE)) as usize {
            1 => Waveform::Triangle,
            2 => Waveform::Saw,
            3 => Waveform::Square,
            _ => Waveform::Sine,
        };
        let hz = key_to_hz(pitch + 12.0 * ctx.base(s(sub::OCTAVE)));
        self.sub
            .set_frequency(f64::from(hz), f64::from(ctx.sample_rate));
        self.noise_mix = self.mix(
            ctx,
            ctx.on(n(noise::ON)),
            [n(noise::LEVEL), n(noise::PAN)],
            [n(noise::FILTER1), n(noise::FILTER2)],
        );
        self.noise_kind = ctx.base(n(noise::TYPE)) as usize;
        self.noise_one_shot = ctx.on(n(noise::ONE_SHOT));
        let track = if ctx.on(n(noise::TRACK)) {
            pitch - 60.0
        } else {
            0.0
        };
        let semis = track + self.value(ctx, n(noise::PITCH));
        self.noise_rate = (semis / 12.0).exp2() * NOISE_RATE / ctx.sample_rate;
    }

    fn mix(&self, ctx: &Ctx<'_>, on: bool, [level, pan]: [usize; 2], [f1, f2]: [usize; 2]) -> Mix {
        let level = self.value(ctx, level);
        let (l, r) = pan_gains(self.value(ctx, pan));
        let f1 = self.value(ctx, f1);
        let f2 = self.value(ctx, f2);
        Mix {
            on,
            left: l * level,
            right: r * level,
            sends: [f1, f2, (1.0 - f1 - f2).max(0.0)],
        }
    }

    fn update_filters(&mut self, ctx: &Ctx<'_>, pitch: f32) {
        self.parallel = ctx.on(FILTER_ROUTING);
        for f in 0..FILTERS {
            let p = |field| filter_param(f, field);
            let kind = FilterType::from_param(ctx.base(p(filter::TYPE)));
            let track = self.value(ctx, p(filter::KEY_TRACK));
            let cutoff =
                self.value(ctx, p(filter::CUTOFF)) * ((pitch - 60.0) / 12.0 * track).exp2();
            let resonance = self.value(ctx, p(filter::RESONANCE));
            let drive = self.value(ctx, p(filter::DRIVE));
            let mix = self.value(ctx, p(filter::MIX));
            if let (Some(on), Some(m)) = (self.filter_on.get_mut(f), self.filter_mix.get_mut(f)) {
                *on = ctx.on(p(filter::ON));
                *m = mix;
            }
            if let Some(pair) = self.filters.get_mut(f) {
                for filter in pair {
                    filter.set(kind, cutoff, resonance, drive);
                }
            }
        }
    }

    /// Adds the next block (as set up by [`Voice::control`]) to the buffers.
    pub fn render(&mut self, ctx: &Ctx<'_>, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        if frames == 0 {
            return;
        }
        let step = (self.amp_target - self.amp) / frames as f32;
        let fade_step = 1.0 / (FADE_SECONDS * self.sample_rate);
        let pairs: [Option<FramePair<'_>>; OSCS] = std::array::from_fn(|o| {
            let run = self.oscs.get(o).filter(|run| run.on)?;
            ctx.tables
                .get(o)?
                .frames_at(run.level, run.frame_a, run.frame_b, run.frame_mix)
        });
        for (l_out, r_out) in left.iter_mut().zip(right.iter_mut()).take(frames) {
            let prev = self.prev;
            // Filter 1, filter 2, dry; left and right.
            let mut bus = [[0.0_f32; 2]; 3];
            for (o, run) in self.oscs.iter_mut().enumerate() {
                let Some(pair) = pairs.get(o).copied().flatten() else {
                    continue;
                };
                let src = prev.get(run.source).copied().unwrap_or(0.0);
                let (mut l, mut r, mut mono) = (0.0, 0.0, 0.0);
                for u in 0..run.unison.min(MAX_UNISON) {
                    let (Some(phase), Some(inc), Some(gl), Some(gr)) = (
                        run.phases.get_mut(u),
                        run.inc.get(u),
                        run.left.get(u),
                        run.right.get(u),
                    ) else {
                        break;
                    };
                    let (p1, g1) = run.warp1.apply(run.amount1, *phase, src);
                    let (p2, g2) = run.warp2.apply(run.amount2, p1, src);
                    let s = pair.read(p2) * g1 * g2;
                    mono += s;
                    l += s * gl;
                    r += s * gr;
                    *phase = (*phase + inc).fract();
                }
                if let Some(p) = self.prev.get_mut(o) {
                    *p = mono / run.unison.max(1) as f32;
                }
                for (b, send) in bus.iter_mut().zip(run.sends) {
                    b[0] += l * send;
                    b[1] += r * send;
                }
            }
            if self.sub_mix.on {
                let s = self.sub.next_sample(self.sub_wave);
                self.prev[SRC_SUB] = s;
                add(&mut bus, &self.sub_mix, s);
            }
            if self.noise_mix.on && self.noise_pos < NOISE_LEN as f32 {
                let s = ctx.noise.read(self.noise_kind, self.noise_pos);
                self.prev[SRC_NOISE] = s;
                add(&mut bus, &self.noise_mix, s);
                self.noise_pos += self.noise_rate;
                if !self.noise_one_shot && self.noise_pos >= NOISE_LEN as f32 {
                    self.noise_pos -= NOISE_LEN as f32;
                }
            }
            let mut out = bus[2];
            for (ch, o) in out.iter_mut().enumerate() {
                let a = self.filter(0, ch, bus[0][ch]);
                *o += if self.parallel {
                    a + self.filter(1, ch, bus[1][ch])
                } else {
                    self.filter(1, ch, a + bus[1][ch])
                };
            }
            self.amp += step;
            let mut gain = self.amp;
            if self.state == State::Fading {
                gain *= self.fade;
                self.fade -= fade_step;
                if self.fade <= 0.0 {
                    self.state = State::Idle;
                    break;
                }
            }
            *l_out += out[0] * gain;
            *r_out += out[1] * gain;
        }
        self.amp = self.amp_target;
        if self.state == State::Playing && self.envs[0].is_idle() {
            self.state = State::Idle;
        }
    }

    fn filter(&mut self, f: usize, ch: usize, x: f32) -> f32 {
        let on = self.filter_on.get(f).copied().unwrap_or(false);
        let mix = self.filter_mix.get(f).copied().unwrap_or(1.0);
        match self.filters.get_mut(f).and_then(|pair| pair.get_mut(ch)) {
            Some(filter) if on => {
                let y = filter.process(x);
                x + (y - x) * mix
            }
            _ => x,
        }
    }
}

fn add(bus: &mut [[f32; 2]; 3], mix: &Mix, s: f32) {
    for (b, send) in bus.iter_mut().zip(mix.sends) {
        b[0] += s * mix.left * send;
        b[1] += s * mix.right * send;
    }
}

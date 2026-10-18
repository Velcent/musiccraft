//! The effects rack (`docs/spec/antiserum-fx-arp.md`): ten slots that order
//! fixed effect modules, each with its own parameters.

use musiccraft_dsp::{Effect, ProcessContext};
use musiccraft_params::{EffectKind, MAX_EFFECT_PARAMS, ParamSpec};

/// Number of order slots.
pub const FX_SLOTS: usize = 10;

/// One module: its name, the mixer effect it plays (none for the
/// dimension widener) and how many of that effect's parameters it exposes.
#[derive(Debug, Clone, Copy)]
pub struct Module {
    /// Name in parameter names (`fx_<name>_…`) and in the slot menus.
    pub name: &'static str,
    /// The mixer effect, if any.
    pub kind: Option<EffectKind>,
    /// Parameters in the synth's table.
    pub count: usize,
}

const fn module(name: &'static str, kind: Option<EffectKind>, count: usize) -> Module {
    Module { name, kind, count }
}

/// The modules, in slot-menu order. Their parameters follow one another in
/// this order in the table, so it is frozen.
pub const MODULES: [Module; 13] = [
    module("dimension", None, 2),
    module("distortion", Some(EffectKind::Distortion), 5),
    module("chorus", Some(EffectKind::Chorus), 6),
    module("flanger", Some(EffectKind::Flanger), 5),
    module("phaser", Some(EffectKind::Phaser), 6),
    module("delay", Some(EffectKind::Delay), 11),
    // The first five: everything but the sidechain.
    module("compressor", Some(EffectKind::Compressor), 5),
    module("multiband", Some(EffectKind::Multiband), 14),
    module("eq", Some(EffectKind::ParametricEq), 22),
    module("filter", Some(EffectKind::Filter), 4),
    module("reverb", Some(EffectKind::HallReverb), 10),
    module("shift", Some(EffectKind::FreqShift), 2),
    // The stereo effect's four, then a gain.
    module("utility", Some(EffectKind::Stereo), 5),
];

/// Every module parameter, in table order.
pub const FX_PARAMS: usize = {
    let mut total = 0;
    let mut i = 0;
    while i < MODULES.len() {
        total += MODULES[i].count;
        i += 1;
    }
    total
};

/// Index of the utility module (its last parameter is our gain).
const UTILITY: usize = 12;

/// The dimension widener's own parameters.
pub const DIMENSION_PARAMS: [ParamSpec; 2] = [
    ParamSpec {
        name: "size",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
        integer: false,
    },
    ParamSpec {
        name: "mix",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
        integer: false,
    },
];

/// The utility module's gain.
pub const UTILITY_GAIN: ParamSpec = ParamSpec {
    name: "gain",
    min: -24.0,
    max: 24.0,
    default: 0.0,
    unit: "dB",
    integer: false,
};

/// The parameter specs of module `m`, before naming (`fx_<module>_` is
/// prefixed by the table).
pub fn module_specs(m: &Module) -> Vec<ParamSpec> {
    match m.kind {
        None => DIMENSION_PARAMS.to_vec(),
        Some(kind) => {
            let mut specs: Vec<ParamSpec> = kind.params().iter().take(m.count).copied().collect();
            if kind == EffectKind::Stereo {
                specs.truncate(m.count - 1);
                specs.push(UTILITY_GAIN);
            }
            specs
        }
    }
}

/// The first parameter of each module, relative to the first module parameter.
pub const fn module_offset(m: usize) -> usize {
    let mut total = 0;
    let mut i = 0;
    while i < m && i < MODULES.len() {
        total += MODULES[i].count;
        i += 1;
    }
    total
}

/// The dimension widener: the mid signal through two short delays swept in
/// opposite directions, one added to the left and one subtracted from the
/// right.
#[derive(Debug, Clone)]
pub struct Dimension {
    buffer: Vec<f32>,
    write: usize,
    phase: [f32; 2],
    sample_rate: f32,
}

/// Longest delay the widener reads, in seconds.
const DIMENSION_MAX: f32 = 0.03;
/// Sweep rates (Hz) and depth (seconds) of the two delays.
const DIMENSION_RATES: [f32; 2] = [0.27, 0.33];
const DIMENSION_DEPTH: f32 = 0.0015;

impl Dimension {
    /// Allocates the delay line. Main thread.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (DIMENSION_MAX * sample_rate) as usize + 4;
        Self {
            buffer: vec![0.0; len.next_power_of_two()],
            write: 0,
            phase: [0.0, 0.5],
            sample_rate,
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
    }

    fn read(&self, delay_samples: f32) -> f32 {
        let mask = self.buffer.len() - 1;
        let pos = self.write as f32 - delay_samples;
        let floor = pos.floor();
        let t = pos - floor;
        let i = (floor as i64).rem_euclid(self.buffer.len() as i64) as usize;
        let a = self.buffer.get(i & mask).copied().unwrap_or(0.0);
        let b = self.buffer.get((i + 1) & mask).copied().unwrap_or(0.0);
        a + (b - a) * t
    }

    /// Processes a block with `size` and `mix` (0..1).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], size: f32, mix: f32) {
        let base = (0.005 + 0.02 * size.clamp(0.0, 1.0)) * self.sample_rate;
        let depth = DIMENSION_DEPTH * self.sample_rate;
        let mask = self.buffer.len() - 1;
        let tau = std::f32::consts::TAU;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mid = 0.5 * (*l + *r);
            if let Some(slot) = self.buffer.get_mut(self.write & mask) {
                *slot = mid;
            }
            let d1 = base + depth * (tau * self.phase[0]).sin();
            let d2 = base * 1.13 + depth * (tau * self.phase[1]).sin();
            let a = self.read(d1);
            let b = self.read(d2);
            *l += mix * a;
            *r -= mix * b;
            self.write = (self.write + 1) & mask;
            for (phase, rate) in self.phase.iter_mut().zip(DIMENSION_RATES) {
                *phase = (*phase + rate / self.sample_rate).fract();
            }
        }
    }
}

/// One built module.
enum Unit {
    Dimension(Dimension),
    Effect {
        effect: Box<Effect>,
        /// Its parameters, starting from the effect's defaults.
        params: [f32; MAX_EFFECT_PARAMS],
    },
}

/// Every module, built once; the slots pick which play and in what order.
pub struct Rack {
    units: Vec<Unit>,
}

impl Rack {
    /// Builds every module. Main thread (allocates).
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let units = MODULES
            .iter()
            .map(|m| match m.kind {
                None => Unit::Dimension(Dimension::new(sample_rate)),
                Some(kind) => Unit::Effect {
                    effect: Box::new(Effect::new(kind, sample_rate)),
                    params: kind.defaults(),
                },
            })
            .collect();
        Self { units }
    }

    /// Clears every module's tails.
    pub fn reset(&mut self) {
        for unit in &mut self.units {
            match unit {
                Unit::Dimension(d) => d.reset(),
                Unit::Effect { effect, .. } => effect.reset(),
            }
        }
    }

    /// Runs the modules named by `slots` (module number + 1, 0 for none),
    /// in order, each at most once, with module parameter values `values`.
    pub fn process(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        slots: &[usize; FX_SLOTS],
        values: &[f32; FX_PARAMS],
        ctx: ProcessContext,
    ) {
        let mut seen = 0_u32;
        for &slot in slots {
            let Some(m) = slot.checked_sub(1) else {
                continue;
            };
            if m >= MODULES.len() || seen & (1 << m) != 0 {
                continue;
            }
            seen |= 1 << m;
            let offset = module_offset(m);
            let count = MODULES.get(m).map_or(0, |module| module.count);
            let Some(own) = values.get(offset..offset + count) else {
                continue;
            };
            match self.units.get_mut(m) {
                Some(Unit::Dimension(d)) => {
                    let get = |i: usize| own.get(i).copied().unwrap_or(0.0);
                    d.process(left, right, get(0), get(1));
                }
                Some(Unit::Effect { effect, params }) => {
                    let effect_count = if m == UTILITY { count - 1 } else { count };
                    for (p, v) in params.iter_mut().zip(own.iter().take(effect_count)) {
                        *p = *v;
                    }
                    effect.process(left, right, params, ctx);
                    if m == UTILITY {
                        let db = own.get(count - 1).copied().unwrap_or(0.0);
                        let gain = 10.0_f32.powf(db / 20.0);
                        for x in left.iter_mut().chain(right.iter_mut()) {
                            *x *= gain;
                        }
                    }
                }
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_counts_match_their_effects() {
        for m in &MODULES {
            if let Some(kind) = m.kind {
                let available = kind.params().len();
                match kind {
                    EffectKind::Compressor => {
                        assert_eq!(kind.sidechain_index(), Some(m.count), "{}", m.name);
                    }
                    EffectKind::Stereo => assert_eq!(available, m.count - 1, "{}", m.name),
                    _ => assert_eq!(available, m.count, "{}", m.name),
                }
            }
            assert_eq!(module_specs(m).len(), m.count, "{}", m.name);
        }
        assert_eq!(FX_PARAMS, 97);
        assert_eq!(module_offset(1), 2);
        assert_eq!(module_offset(MODULES.len()), FX_PARAMS);
    }
}

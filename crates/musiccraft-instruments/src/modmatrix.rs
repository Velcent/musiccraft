//! The synth's second LFO, modulation envelope and three-slot modulation
//! matrix. Runs at control rate inside a voice; allocation-free.

use musiccraft_params::{MOD_SLOTS, inst};

use crate::Params;

fn param(params: &Params, index: usize) -> f32 {
    params.get(index).copied().unwrap_or(0.0)
}

/// What the matrix changes, summed over its slots.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct SynthMods {
    /// Semitones.
    pub pitch: f32,
    /// Octaves.
    pub cutoff: f32,
    /// Octaves.
    pub cutoff2: f32,
    pub resonance: f32,
    pub osc_mix: f32,
    /// Gain multiplier, 0..2.
    pub gain: f32,
    pub pan: f32,
    /// Cents.
    pub detune: f32,
    pub osc3: f32,
}

/// The values the matrix can read, each about −1..1 or 0..1.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ModSources {
    pub lfo1: f32,
    pub velocity: f32,
    /// Key position, −1 two octaves below C5 to 1 two octaves above.
    pub key: f32,
    pub amp_env: f32,
}

/// Per-voice state of the second LFO and the modulation envelope.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ModState {
    lfo2_phase: f32,
    /// The current random step of the second LFO.
    lfo2_hold: f32,
    /// Seconds since the note started.
    env2_time: f32,
}

impl ModState {
    /// Restarts the LFO and envelope for a new note.
    pub fn start(&mut self) {
        *self = Self::default();
    }

    /// The second LFO's value for `shape` at `phase`, −1..1.
    fn lfo2(&self, shape: i32) -> f32 {
        let p = self.lfo2_phase;
        match shape {
            1 => 4.0 * ((p + 0.75).fract() - 0.5).abs() - 1.0,
            2 => p * 2.0 - 1.0,
            3 => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            4 => self.lfo2_hold,
            _ => (p * std::f32::consts::TAU).sin(),
        }
    }

    /// The modulation envelope: rises 0→1 over the attack, falls back to 0
    /// over the decay.
    fn env2(&self, attack: f32, decay: f32) -> f32 {
        let t = self.env2_time;
        if t < attack {
            t / attack.max(1e-6)
        } else {
            (1.0 - (t - attack) / decay.max(1e-6)).max(0.0)
        }
    }

    /// Advances by `frames` at `sample_rate` and computes the matrix.
    /// `random` supplies a new random step (−1..1) when the LFO wraps.
    pub fn update(
        &mut self,
        params: &Params,
        sources: ModSources,
        frames: u32,
        sample_rate: f32,
        random: impl FnOnce() -> f32,
    ) -> SynthMods {
        let seconds = frames as f32 / sample_rate.max(1.0);
        let next = self.lfo2_phase + param(params, inst::LFO2_RATE).max(0.0) * seconds;
        if next >= 1.0 {
            self.lfo2_hold = random();
        }
        self.lfo2_phase = next.fract();
        self.env2_time += seconds;
        let shape = param(params, inst::LFO2_SHAPE).round() as i32;
        let env2 = self.env2(
            param(params, inst::ENV2_ATTACK),
            param(params, inst::ENV2_DECAY),
        );
        let mut mods = SynthMods::default();
        for slot in 0..MOD_SLOTS {
            let base = inst::MOD1_SOURCE + slot * 3;
            let value = match param(params, base).round() as i32 {
                1 => sources.lfo1,
                2 => self.lfo2(shape),
                3 => env2,
                4 => sources.velocity,
                5 => sources.key,
                6 => sources.amp_env,
                _ => continue,
            };
            let amount = param(params, base + 2).clamp(-1.0, 1.0) * value;
            match param(params, base + 1).round() as i32 {
                0 => mods.pitch += amount * 12.0,
                1 => mods.cutoff += amount * 4.0,
                2 => mods.cutoff2 += amount * 4.0,
                3 => mods.resonance += amount,
                4 => mods.osc_mix += amount,
                5 => mods.gain += amount,
                6 => mods.pan += amount,
                7 => mods.detune += amount * 100.0,
                8 => mods.osc3 += amount,
                _ => {}
            }
        }
        // `gain` was summed as an offset; it multiplies the level.
        mods.gain = (1.0 + mods.gain).clamp(0.0, 2.0);
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(values: &[(usize, f32)]) -> Params {
        let mut p = crate::default_params();
        for (i, v) in values {
            p[*i] = *v;
        }
        p
    }

    #[test]
    fn empty_matrix_changes_nothing() {
        let mut state = ModState::default();
        let mods = state.update(
            &crate::default_params(),
            ModSources::default(),
            32,
            48_000.0,
            || 0.0,
        );
        assert_eq!(
            mods,
            SynthMods {
                gain: 1.0,
                ..SynthMods::default()
            }
        );
    }

    #[test]
    fn slots_route_sources_to_destinations_and_add_up() {
        let p = params(&[
            (inst::MOD1_SOURCE, 4.0), // velocity
            (inst::MOD1_DEST, 1.0),   // cutoff
            (inst::MOD1_AMOUNT, 0.5),
            (inst::MOD2_SOURCE, 4.0),
            (inst::MOD2_DEST, 1.0),
            (inst::MOD2_AMOUNT, 0.25),
            (inst::MOD3_SOURCE, 5.0), // key
            (inst::MOD3_DEST, 5.0),   // volume
            (inst::MOD3_AMOUNT, -1.0),
        ]);
        let sources = ModSources {
            velocity: 1.0,
            key: 0.5,
            ..ModSources::default()
        };
        let mods = ModState::default().update(&p, sources, 32, 48_000.0, || 0.0);
        assert!((mods.cutoff - 3.0).abs() < 1e-6, "(0.5 + 0.25) × 4 octaves");
        assert!((mods.gain - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mod_envelope_rises_and_falls() {
        let p = params(&[
            (inst::ENV2_ATTACK, 0.1),
            (inst::ENV2_DECAY, 0.1),
            (inst::MOD1_SOURCE, 3.0),
            (inst::MOD1_DEST, 0.0),
            (inst::MOD1_AMOUNT, 1.0),
        ]);
        let mut state = ModState::default();
        // 0.05 s in: halfway up; 0.15 s: halfway down; 0.3 s: gone.
        let at = |state: &mut ModState, frames| {
            state
                .update(&p, ModSources::default(), frames, 1_000.0, || 0.0)
                .pitch
        };
        assert!((at(&mut state, 50) - 6.0).abs() < 1e-3);
        assert!((at(&mut state, 100) - 6.0).abs() < 1e-3);
        assert!(at(&mut state, 150).abs() < 1e-3);
    }

    #[test]
    fn random_lfo_steps_when_it_wraps() {
        let p = params(&[
            (inst::LFO2_RATE, 10.0),
            (inst::LFO2_SHAPE, 4.0),
            (inst::MOD1_SOURCE, 2.0),
            (inst::MOD1_DEST, 6.0),
            (inst::MOD1_AMOUNT, 1.0),
        ]);
        let mut state = ModState::default();
        let before = state
            .update(&p, ModSources::default(), 50, 1_000.0, || 0.7)
            .pan;
        let after = state
            .update(&p, ModSources::default(), 60, 1_000.0, || 0.7)
            .pan;
        assert_eq!(before, 0.0);
        assert!((after - 0.7).abs() < 1e-6);
    }
}

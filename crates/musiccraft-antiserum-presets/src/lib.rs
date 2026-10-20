//! Antiserum patches in use (`docs/spec/antiserum-app.md`): applying a
//! [`Patch`] to a synth, the display text of any parameter, and the factory
//! bank. Main thread only (building wavetables allocates).

mod bank;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use musiccraft_antiserum::fx::{MODULES, module_specs};
use musiccraft_antiserum::notes::SEQ_STEPS;
use musiccraft_antiserum::params::{FX_BASE, LFOS, OSCS, PARAM_COUNT, fx_param};
use musiccraft_antiserum::{
    Antiserum, LfoPoint, LfoShape, Sequence, Step, Wavetable, choices, index_of, params,
};
use musiccraft_patch::{Patch, StepData, TableRef};

pub use bank::{factory, find};

/// Every parameter's value for `patch`: defaults, then the patch's values
/// (sanitized). Unknown names are ignored.
#[must_use]
pub fn values(patch: &Patch) -> Box<[f32; PARAM_COUNT]> {
    let specs = params();
    let mut out = Box::new([0.0; PARAM_COUNT]);
    for (value, spec) in out.iter_mut().zip(specs) {
        *value = spec.default;
    }
    for (name, value) in &patch.params {
        if let Some(i) = index_of(name)
            && let (Some(slot), Some(spec)) = (out.get_mut(i), specs.get(i))
        {
            *slot = if value.is_finite() {
                spec.sanitize(*value)
            } else {
                spec.default
            };
        }
    }
    out
}

/// The wavetable a table reference names. Factory tables are built once and
/// shared.
///
/// # Errors
///
/// An unknown factory table, bad data, or data with no whole frame.
pub fn wavetable(reference: &str) -> Result<Arc<Wavetable>, String> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Wavetable>>>> = OnceLock::new();
    match TableRef::parse(reference)? {
        TableRef::Factory(name) => {
            let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            if let Some(table) = cache.lock().ok().and_then(|c| c.get(&name).cloned()) {
                return Ok(table);
            }
            let table = musiccraft_wavetables::factory::table(&name)
                .ok_or_else(|| format!("no factory wavetable `{name}`"))?;
            let built =
                Arc::new(Wavetable::new(table.samples()).ok_or("the factory table has no frames")?);
            if let Ok(mut cache) = cache.lock() {
                cache.insert(name, Arc::clone(&built));
            }
            Ok(built)
        }
        TableRef::Data(samples) => Wavetable::new(&samples)
            .map(Arc::new)
            .ok_or_else(|| "the table data holds no whole 2048-sample frame".to_owned()),
    }
}

/// The editable frames a table reference names (for the wavetable editor).
///
/// # Errors
///
/// As [`wavetable`].
pub fn table_frames(reference: &str) -> Result<musiccraft_wavetables::Table, String> {
    match TableRef::parse(reference)? {
        TableRef::Factory(name) => musiccraft_wavetables::factory::table(&name)
            .ok_or_else(|| format!("no factory wavetable `{name}`")),
        TableRef::Data(samples) => musiccraft_wavetables::Table::new(&samples)
            .ok_or_else(|| "the table data holds no whole 2048-sample frame".to_owned()),
    }
}

/// LFO `lfo`'s (0-based) shape in `patch` (the default triangle if none).
#[must_use]
pub fn lfo_shape(patch: &Patch, lfo: usize) -> LfoShape {
    patch
        .lfo_shape(lfo)
        .map_or_else(LfoShape::default, |points| {
            let points: Vec<LfoPoint> = points
                .iter()
                .map(|p| LfoPoint {
                    x: p[0],
                    y: p[1],
                    curve: p[2],
                })
                .collect();
            LfoShape::new(&points)
        })
}

/// A sequencer step as patch data.
#[must_use]
pub fn step_data(step: Step) -> StepData {
    (step.on, step.transpose, step.velocity, step.gate)
}

/// The sequencer pattern in `patch`.
#[must_use]
pub fn sequence(patch: &Patch) -> Sequence {
    let mut sequence = Sequence::default();
    for (slot, data) in sequence
        .steps
        .iter_mut()
        .zip(&patch.sequence)
        .take(SEQ_STEPS)
    {
        *slot = Step {
            on: data.0,
            transpose: data.1,
            velocity: data.2,
            gate: data.3,
        };
    }
    sequence
}

/// Sets a synth up to play `patch`. Returns a warning per table that could
/// not be built (that oscillator keeps the basic table).
pub fn apply(patch: &Patch, synth: &mut Antiserum) -> Vec<String> {
    let mut warnings = Vec::new();
    for (i, value) in values(patch).iter().enumerate() {
        synth.set_param(i, *value);
    }
    for osc in 0..OSCS {
        let table = wavetable(patch.table(osc)).or_else(|err| {
            warnings.push(format!("oscillator {}: {err}", osc + 1));
            wavetable(musiccraft_patch::DEFAULT_TABLE)
        });
        if let Ok(table) = table {
            drop(synth.set_wavetable(osc, table));
        }
    }
    for lfo in 0..LFOS {
        synth.set_lfo_shape(lfo, lfo_shape(patch, lfo));
    }
    synth.set_sequence(&sequence(patch));
    warnings
}

/// A synth playing `patch` at `sample_rate`.
#[must_use]
pub fn synth(patch: &Patch, sample_rate: f32) -> Antiserum {
    let mut synth = Antiserum::new(sample_rate);
    let _ = apply(patch, &mut synth);
    synth
}

/// The effect module parameter `index` belongs to, with its unprefixed spec.
fn fx_spec(index: usize) -> Option<(musiccraft_params::EffectKind, musiccraft_params::ParamSpec)> {
    let field = index.checked_sub(FX_BASE)?;
    for (m, module) in MODULES.iter().enumerate() {
        let first = fx_param(m, 0) - FX_BASE;
        if (first..first + module.count).contains(&field) {
            let spec = *module_specs(module).get(field - first)?;
            return module.kind.map(|kind| (kind, spec));
        }
    }
    None
}

/// Display text for parameter `index` at `value`: a choice's name, an effect
/// module's text as the mixer shows it, or the value with its unit.
#[must_use]
pub fn text(index: usize, value: f32) -> String {
    let Some(spec) = params().get(index) else {
        return String::new();
    };
    if let Some(list) = choices(spec.name) {
        let i = (spec.sanitize(value) - spec.min).max(0.0) as usize;
        if let Some(name) = list.get(i) {
            return (*name).to_owned();
        }
    }
    if let Some((kind, own)) = fx_spec(index) {
        return musiccraft_dsp::describe::text(kind, &own, value);
    }
    match spec.unit {
        "" if !spec.integer && spec.min < 0.0 => format!("{:+.0}%", value * 100.0),
        _ => spec.format(value),
    }
}

/// A short knob label for parameter `name`: its name without the group
/// prefix (`osc1_unison` → "unison"), with spaces.
#[must_use]
pub fn label(name: &str) -> String {
    let rest = name
        .strip_prefix("fx_")
        .and_then(|r| {
            MODULES
                .iter()
                .find_map(|m| r.strip_prefix(m.name)?.strip_prefix('_'))
        })
        .or_else(|| {
            let (group, rest) = name.split_once('_')?;
            let grouped = [
                "osc", "sub", "noise", "filter", "env", "lfo", "mod", "arp", "seq",
            ]
            .iter()
            .any(|g| group.starts_with(g));
            grouped.then_some(rest)
        })
        .unwrap_or(name);
    rest.replace('_', " ")
}

#[cfg(test)]
mod tests;

//! Host parameters of an effect plugin, derived from the effect's
//! `ParamSpec` table: one per entry, identified by the parameter name.

use std::collections::BTreeMap;
use std::sync::Arc;

use musiccraft_dsp::describe;
use musiccraft_params::{EffectKind, MAX_EFFECT_PARAMS, ParamSpec};
use nih_plug::prelude::{
    BoolParam, FloatParam, FloatRange, IntParam, IntRange, Param, ParamPtr, Params,
};
use serde::{Deserialize, Serialize};

use crate::shared::Shared;

/// Version of the `musiccraft` state field.
pub const STATE_FORMAT: u32 = 1;
/// Key of our own JSON in the plugin state.
pub const STATE_KEY: &str = "musiccraft";
/// ID of the bypass parameter.
pub const BYPASS: &str = "bypass";

/// Names of a switch's values.
const OFF_ON: [&str; 2] = ["off", "on"];

/// One host parameter.
pub enum HostParam {
    /// A continuous value.
    Float(FloatParam),
    /// A whole number or a choice.
    Int(IntParam),
}

impl HostParam {
    /// The plain value.
    pub fn value(&self) -> f32 {
        match self {
            Self::Float(p) => p.value(),
            Self::Int(p) => p.value() as f32,
        }
    }

    /// The pointer nih-plug's parameter map holds.
    pub fn ptr(&self) -> ParamPtr {
        match self {
            Self::Float(p) => p.as_ptr(),
            Self::Int(p) => p.as_ptr(),
        }
    }
}

/// A host parameter and the effect parameter it drives.
pub struct Slot {
    /// Index into the effect's parameter table.
    pub index: usize,
    /// Its spec (for a switch, the plugin's own 0–1 spec).
    pub spec: ParamSpec,
    /// The host parameter.
    pub param: HostParam,
}

/// All parameters of one effect plugin.
pub struct EffectParams {
    /// The effect.
    pub kind: EffectKind,
    /// The host's bypass switch.
    pub bypass: BoolParam,
    /// The effect's parameters, in table order.
    pub slots: Vec<Slot>,
    /// State shared with the audio thread and the editor.
    pub shared: Arc<Shared>,
}

/// The plugin's own switch that replaces routing parameter `name`, if the
/// effect keeps one: the sidechain input and following MIDI notes.
fn switch(name: &'static str) -> Option<ParamSpec> {
    match name {
        "sidechain" | "follow" => Some(ParamSpec {
            name,
            min: 0.0,
            max: 1.0,
            default: 0.0,
            unit: "",
            integer: true,
        }),
        _ => None,
    }
}

/// The skew that puts the middle of a frequency range at its geometric mean,
/// close to the app's logarithmic knobs.
fn frequency_skew(min: f32, max: f32) -> f32 {
    let middle = (min * max).sqrt();
    let t = ((middle - min) / (max - min)).clamp(1e-6, 1.0 - 1e-6);
    0.5_f32.ln() / t.ln()
}

/// The host range for a continuous spec.
pub fn float_range(spec: &ParamSpec) -> FloatRange {
    let (min, max) = (spec.min, spec.max);
    match spec.unit {
        "Hz" if min > 0.0 => FloatRange::Skewed {
            min,
            max,
            factor: frequency_skew(min, max),
        },
        "s" | "ms" => FloatRange::Skewed {
            min,
            max,
            factor: 1.0 / 3.0,
        },
        _ => FloatRange::Linear { min, max },
    }
}

impl EffectParams {
    /// Parameters for `kind` at their defaults.
    pub fn new(kind: EffectKind, shared: Arc<Shared>) -> Self {
        let slots = kind
            .params()
            .iter()
            .enumerate()
            .filter_map(|(index, spec)| {
                let spec = if musiccraft_editors::effect::is_routing(kind, index) {
                    switch(spec.name)?
                } else {
                    *spec
                };
                Some(Slot {
                    index,
                    spec,
                    param: host_param(kind, index, spec),
                })
            })
            .collect();
        Self {
            kind,
            bypass: BoolParam::new("Bypass", false).make_bypass(),
            slots,
            shared,
        }
    }

    /// The values to process with: every table entry, with the host's values
    /// where the plugin has a parameter. Allocation-free.
    pub fn values(&self) -> [f32; MAX_EFFECT_PARAMS] {
        let mut values = self.kind.defaults();
        for slot in &self.slots {
            if let Some(value) = values.get_mut(slot.index) {
                *value = slot.param.value();
            }
        }
        values
    }

    /// The slot of parameter `name`.
    pub fn slot(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.spec.name == name)
    }

    /// Our own state: the effect, its values by name and the impulse.
    pub fn state(&self) -> PluginState {
        PluginState {
            format: STATE_FORMAT,
            effect: self.kind.name().to_owned(),
            values: self
                .slots
                .iter()
                .map(|s| (s.spec.name.to_owned(), s.param.value()))
                .collect(),
            impulse: (self.kind == EffectKind::Convolution).then(|| self.shared.impulse_name()),
        }
    }
}

/// The `musiccraft` field of the plugin state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginState {
    /// [`STATE_FORMAT`].
    pub format: u32,
    /// The effect's stable name.
    pub effect: String,
    /// Parameter values by name (the app's preset shape).
    pub values: BTreeMap<String, f32>,
    /// The convolution reverb's room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impulse: Option<String>,
}

/// `value` rounded to a millionth of `spec`'s range, so a value that went
/// through the host's normalized range (1000 Hz coming back as 999.9997, 0 as
/// −0.00001) reads as itself.
fn snap(spec: &ParamSpec, value: f32) -> f32 {
    let span = (spec.max - spec.min).abs();
    if !(value.is_finite() && span > 0.0) {
        return value;
    }
    let step = 10_f32.powi(span.log10().floor() as i32 - 6);
    let snapped = (value / step).round() * step;
    // No "-0".
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// The host parameter for entry `index` of `kind`'s table, described by
/// `spec` (the plugin's switch for a routing entry).
fn host_param(kind: EffectKind, index: usize, spec: ParamSpec) -> HostParam {
    let label = musiccraft_params::describe::label(spec.name);
    let is_switch = musiccraft_editors::effect::is_routing(kind, index);
    let text = move |value: f32| {
        if is_switch {
            OFF_ON[usize::from(value >= 0.5)].to_owned()
        } else {
            describe::text(kind, &spec, value)
        }
    };
    let parse = move |typed: &str| {
        if is_switch {
            OFF_ON
                .iter()
                .position(|n| n.eq_ignore_ascii_case(typed.trim()))
                .map(|i| i as f32)
        } else {
            describe::parse(kind, &spec, typed)
        }
    };
    host_param_with(label, &spec, text, parse)
}

/// A host parameter for `spec`, named `label`, with its own display text and
/// typed-text parsing (plain values).
pub fn host_param_with(
    label: String,
    spec: &ParamSpec,
    text: impl Fn(f32) -> String + Send + Sync + 'static,
    parse: impl Fn(&str) -> Option<f32> + Send + Sync + 'static,
) -> HostParam {
    let spec = *spec;
    let text = move |value: f32| text(snap(&spec, value));
    if spec.integer {
        let range = IntRange::Linear {
            min: spec.min.round() as i32,
            max: spec.max.round() as i32,
        };
        HostParam::Int(
            IntParam::new(label, spec.default.round() as i32, range)
                .with_value_to_string(Arc::new(move |v| text(v as f32)))
                .with_string_to_value(Arc::new(move |s| parse(s).map(|v| v.round() as i32))),
        )
    } else {
        HostParam::Float(
            FloatParam::new(label, spec.default, float_range(&spec))
                .with_value_to_string(Arc::new(text))
                .with_string_to_value(Arc::new(parse)),
        )
    }
}

// SAFETY: every `ParamPtr` points into `self` (the bypass field or an element
// of `slots`, which is never resized after construction), and nih-plug keeps
// this object alive in an `Arc` for as long as it uses the pointers.
unsafe impl Params for EffectParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        let mut map = vec![(BYPASS.to_owned(), self.bypass.as_ptr(), String::new())];
        map.extend(
            self.slots
                .iter()
                .map(|s| (s.spec.name.to_owned(), s.param.ptr(), String::new())),
        );
        map
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        if let Ok(json) = serde_json::to_string(&self.state()) {
            fields.insert(STATE_KEY.to_owned(), json);
        }
        fields
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        // The values themselves come back through the parameter map (same
        // names); only the impulse lives outside it.
        let Some(state) = serialized
            .get(STATE_KEY)
            .and_then(|json| serde_json::from_str::<PluginState>(json).ok())
        else {
            return;
        };
        if state.effect != self.kind.name() {
            return;
        }
        if let Some(room) = state.impulse {
            self.shared.set_impulse(&room);
        }
    }
}

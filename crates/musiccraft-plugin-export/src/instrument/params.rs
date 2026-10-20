//! Host parameters of an instrument plugin: one per name in
//! `Kind::params()` (identified by the name), plus the plugin's own swing
//! amount and pitch-bend range (`docs/spec/plugin-instruments.md`).

use std::collections::BTreeMap;
use std::sync::Arc;

use musiccraft_instruments::{Kind, Params, default_params};
use musiccraft_params::{INSTRUMENT_PARAMS, ParamSpec, describe, index_of};
use nih_plug::prelude::{ParamPtr, Params as NihParams};
use serde::{Deserialize, Serialize};

use super::data::{SampleData, Shared};
use crate::params::{STATE_FORMAT, STATE_KEY, Slot, host_param_with};

/// The plugin's swing amount: what the project's swing is in the app.
pub const SWING: ParamSpec = ParamSpec {
    name: "swing",
    min: 0.0,
    max: 1.0,
    default: 0.0,
    unit: "",
    integer: false,
};

/// How far pitch bend moves the voices, in semitones.
pub const BEND_RANGE: ParamSpec = ParamSpec {
    name: "bend_range",
    min: 0.0,
    max: 24.0,
    default: 2.0,
    unit: "st",
    integer: true,
};

/// The `musiccraft` field of an instrument plugin's state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentState {
    /// [`STATE_FORMAT`].
    pub format: u32,
    /// The instrument's name as project files write it.
    pub instrument: String,
    /// Parameter values by name (the app's preset shape).
    pub values: BTreeMap<String, f32>,
    /// Samples, pads and zones, for the instruments that use them.
    #[serde(flatten)]
    pub data: SampleData,
}

/// All parameters of one instrument plugin.
pub struct InstrumentParams {
    pub kind: Kind,
    /// The instrument's name as project files write it.
    pub name: &'static str,
    /// Instrument table entries, in `Kind::params()` order; `index` is the
    /// entry's index in `INSTRUMENT_PARAMS`.
    pub slots: Vec<Slot>,
    /// [`SWING`] and [`BEND_RANGE`].
    pub own: [Slot; 2],
    defaults: Params,
    /// Samples, shared with the editor and the audio thread.
    pub shared: Arc<Shared>,
}

fn own_slot(spec: ParamSpec) -> Slot {
    let label = describe::label(spec.name);
    Slot {
        index: usize::MAX,
        spec,
        param: host_param_with(
            label,
            &spec,
            move |v| describe::display(&spec, v),
            move |t| describe::parse(&spec, t),
        ),
    }
}

impl InstrumentParams {
    /// Parameters for `kind` at their defaults.
    #[must_use]
    pub fn new(kind: Kind, name: &'static str, shared: Arc<Shared>) -> Self {
        let slots = kind
            .params()
            .filter_map(|n| {
                let index = index_of(INSTRUMENT_PARAMS, n)?;
                let spec = *INSTRUMENT_PARAMS.get(index)?;
                Some(Slot {
                    index,
                    spec,
                    param: host_param_with(
                        describe::label(spec.name),
                        &spec,
                        move |v| describe::display(&spec, v),
                        move |t| describe::parse(&spec, t),
                    ),
                })
            })
            .collect();
        Self {
            kind,
            name,
            slots,
            own: [own_slot(SWING), own_slot(BEND_RANGE)],
            defaults: default_params(),
            shared,
        }
    }

    /// The instrument parameters to play with: the table's defaults, with
    /// the host's values where the plugin has a parameter. Allocation-free.
    #[must_use]
    pub fn values(&self) -> Params {
        let mut values = self.defaults;
        for slot in &self.slots {
            if let Some(value) = values.get_mut(slot.index) {
                *value = slot.param.value();
            }
        }
        values
    }

    /// The swing amount.
    #[must_use]
    pub fn swing(&self) -> f32 {
        self.own[0].param.value()
    }

    /// The pitch-bend range in semitones.
    #[must_use]
    pub fn bend_range(&self) -> f32 {
        self.own[1].param.value()
    }

    /// The slot of instrument parameter `index`.
    #[must_use]
    pub fn slot(&self, index: usize) -> Option<&Slot> {
        self.slots.iter().find(|s| s.index == index)
    }

    /// The slot named `name`, the plugin's own included.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Slot> {
        self.slots
            .iter()
            .chain(&self.own)
            .find(|s| s.spec.name == name)
    }

    /// Our own state: the instrument, its values by name and its samples.
    #[must_use]
    pub fn state(&self) -> InstrumentState {
        InstrumentState {
            format: STATE_FORMAT,
            instrument: self.name.to_owned(),
            values: self
                .slots
                .iter()
                .chain(&self.own)
                .map(|s| (s.spec.name.to_owned(), s.param.value()))
                .collect(),
            data: self.shared.data(),
        }
    }
}

// SAFETY: every `ParamPtr` points into `self` (an element of `slots`, which
// is never resized after construction, or of `own`), and nih-plug keeps this
// object alive in an `Arc` for as long as it uses the pointers.
unsafe impl NihParams for InstrumentParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.slots
            .iter()
            .chain(&self.own)
            .map(|s| (s.spec.name.to_owned(), s.param.ptr(), String::new()))
            .collect()
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        if let Ok(json) = serde_json::to_string(&self.state()) {
            fields.insert(STATE_KEY.to_owned(), json);
        }
        fields
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        // Values come back through the parameter map; samples live here.
        let Some(state) = serialized
            .get(STATE_KEY)
            .and_then(|json| serde_json::from_str::<InstrumentState>(json).ok())
        else {
            return;
        };
        if state.format != STATE_FORMAT || state.instrument != self.name {
            return;
        }
        self.shared.set_data(state.data);
    }
}

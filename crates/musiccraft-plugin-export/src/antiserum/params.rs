//! Antiserum's host parameters (its whole parameter table, identified by
//! name), its state (the app's patch format) and the patch data that is not
//! a parameter, handed to the audio thread as `Data` swaps
//! (`docs/spec/plugin-instruments.md`).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use musiccraft_antiserum::{Data, PARAM_COUNT, params as specs};
use musiccraft_antiserum_presets as presets;
use musiccraft_patch::Patch;
use nih_plug::prelude::{ParamPtr, Params};
use serde::{Deserialize, Serialize};

use crate::handoff;
use crate::params::{STATE_FORMAT, STATE_KEY, Slot, host_param_with};

/// Antiserum's name in states, as project files write it.
pub const NAME: &str = "antiserum";
/// Data swaps waiting for the audio thread.
const QUEUE: usize = 64;

/// The `musiccraft` field of Antiserum's state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntiserumState {
    /// [`STATE_FORMAT`].
    pub format: u32,
    /// Always [`NAME`].
    pub instrument: String,
    /// The sound, as a patch file holds it.
    pub patch: Patch,
}

/// The patch's data that is not a parameter, shared by the editor, state
/// loading and the audio thread's hand-over. The locks are never taken on
/// the audio thread.
pub struct Shared {
    sample_rate: AtomicU32,
    patch: Mutex<Patch>,
    warnings: Mutex<Vec<String>>,
    sender: Mutex<handoff::Sender<Data>>,
}

impl Shared {
    fn new() -> (Self, handoff::Receiver<Data>) {
        let (sender, receiver) = handoff::channel(QUEUE);
        (
            Self {
                sample_rate: AtomicU32::new(0),
                patch: Mutex::new(Patch::default()),
                warnings: Mutex::new(Vec::new()),
                sender: Mutex::new(sender),
            },
            receiver,
        )
    }

    /// Records the sample rate the plugin was set up with.
    pub fn set_sample_rate(&self, rate: f32) {
        self.sample_rate.store(rate.to_bits(), Ordering::Release);
    }

    fn rate(&self) -> Option<f32> {
        let rate = f32::from_bits(self.sample_rate.load(Ordering::Acquire));
        (rate > 0.0).then_some(rate)
    }

    /// The patch's data (its parameter values are the host's).
    #[must_use]
    pub fn patch(&self) -> Patch {
        self.patch.lock().map(|p| p.clone()).unwrap_or_default()
    }

    /// Tables or samples that could not be built.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.warnings.lock().map(|w| w.clone()).unwrap_or_default()
    }

    /// Every swap that sets a synth up for the current patch (main thread).
    #[must_use]
    pub fn all_data(&self) -> Vec<Data> {
        let (data, warnings) = presets::data(&self.patch(), self.rate().unwrap_or(48_000.0));
        if let Ok(mut slot) = self.warnings.lock() {
            *slot = warnings;
        }
        data
    }

    /// Uses `patch`'s data from now on, and hands it to a running synth.
    pub fn set_patch(&self, patch: Patch) {
        if let Ok(mut current) = self.patch.lock() {
            *current = patch;
        }
        if self.rate().is_some() {
            let data = self.all_data();
            self.send(data);
        }
    }

    /// Changes the patch with `edit` and hands a running synth what `build`
    /// makes of the changed patch.
    pub fn change(&self, edit: impl FnOnce(&mut Patch), build: impl FnOnce(&Patch, f32) -> Data) {
        let patch = {
            let Ok(mut patch) = self.patch.lock() else {
                return;
            };
            edit(&mut patch);
            patch.clone()
        };
        if let Some(rate) = self.rate() {
            self.send(vec![build(&patch, rate)]);
        }
    }

    fn send(&self, data: Vec<Data>) {
        if let Ok(mut sender) = self.sender.lock() {
            for item in data {
                let _ = sender.send(item);
            }
        }
    }

    /// Frees data the audio thread handed back.
    pub fn collect_garbage(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.collect();
        }
    }
}

/// Antiserum's host parameters.
pub struct AntiserumParams {
    /// Every parameter, in table order (`index` is the table index).
    pub slots: Vec<Slot>,
    pub shared: Arc<Shared>,
}

impl AntiserumParams {
    /// Every parameter at its default, and the audio thread's end of the data
    /// hand-over.
    #[must_use]
    pub fn new() -> (Self, handoff::Receiver<Data>) {
        let (shared, receiver) = Shared::new();
        let slots = specs()
            .iter()
            .enumerate()
            .map(|(index, spec)| Slot {
                index,
                spec: *spec,
                param: host_param_with(
                    presets::label(spec.name),
                    spec,
                    move |v| presets::text(index, v),
                    move |t| presets::parse(index, t),
                ),
            })
            .collect();
        (
            Self {
                slots,
                shared: Arc::new(shared),
            },
            receiver,
        )
    }

    /// Every parameter's current value.
    #[must_use]
    pub fn values(&self) -> Box<[f32; PARAM_COUNT]> {
        let mut values = Box::new(musiccraft_antiserum::params::defaults());
        for slot in &self.slots {
            if let Some(v) = values.get_mut(slot.index) {
                *v = slot.param.value();
            }
        }
        values
    }

    /// The sound as a patch file holds it: the data, and every parameter that
    /// is not at its default.
    #[must_use]
    pub fn patch(&self) -> Patch {
        let mut patch = self.shared.patch();
        patch.params = self
            .slots
            .iter()
            .filter(|s| s.param.value() != s.spec.default)
            .map(|s| (s.spec.name.to_owned(), s.param.value()))
            .collect();
        patch
    }
}

// SAFETY: every `ParamPtr` points into an element of `slots`, which is never
// resized after construction, and nih-plug keeps this object alive in an
// `Arc` for as long as it uses the pointers.
unsafe impl Params for AntiserumParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.slots
            .iter()
            .map(|s| (s.spec.name.to_owned(), s.param.ptr(), String::new()))
            .collect()
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let state = AntiserumState {
            format: STATE_FORMAT,
            instrument: NAME.to_owned(),
            patch: self.patch(),
        };
        let mut fields = BTreeMap::new();
        if let Ok(json) = serde_json::to_string(&state) {
            fields.insert(STATE_KEY.to_owned(), json);
        }
        fields
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        // Values come back through the parameter map; the data lives here.
        let Some(state) = serialized
            .get(STATE_KEY)
            .and_then(|json| serde_json::from_str::<AntiserumState>(json).ok())
        else {
            return;
        };
        if state.format == STATE_FORMAT && state.instrument == NAME {
            self.shared.set_patch(state.patch);
        }
    }
}

//! An instrument plugin's samples: what the state names (paths and built-in
//! sounds), loading them off the audio thread into an [`Instrument`], and
//! handing that to the audio thread (`docs/spec/plugin-instruments.md`).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use musiccraft_dsp::SampleBuffer;
use musiccraft_dsp::resynth::HarmonicTable;
use musiccraft_instruments::{Instrument, Kind, Zone};
use musiccraft_params::{BUILTIN_PREFIX, BUILTIN_SAMPLES};
use musiccraft_patch::ZoneRef;
use serde::{Deserialize, Serialize};

use crate::handoff;

/// Kit pads.
pub const PADS: usize = 16;
/// Columns in a waveform overview.
const OVERVIEW: usize = 240;
/// Instruments waiting for the audio thread.
const QUEUE: usize = 8;

/// The samples an instrument plays, as its state names them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SampleData {
    /// Sampler, granular and resynth: the sample.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sample: String,
    /// Kit: sixteen pads, empty for an empty pad.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pads: Vec<String>,
    /// Zones: the zones, in the order notes look for a match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<ZoneRef>,
}

impl SampleData {
    /// What a new plugin of `kind` starts with.
    #[must_use]
    pub fn default_for(kind: Kind) -> Self {
        let builtin = |name: &str| format!("{BUILTIN_PREFIX}{name}");
        match kind {
            Kind::Sampler => Self {
                sample: builtin("kick"),
                ..Self::default()
            },
            Kind::Granular | Kind::Resynth => Self {
                sample: builtin("crash"),
                ..Self::default()
            },
            Kind::Kit => {
                let mut pads: Vec<String> = BUILTIN_SAMPLES.iter().map(|s| builtin(s)).collect();
                pads.resize(PADS, String::new());
                Self {
                    pads,
                    ..Self::default()
                }
            }
            Kind::Zones => Self {
                zones: vec![ZoneRef::new(builtin("tom"), 0, 127, 60)],
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

/// What the editor shows about the loaded samples.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    /// Samples that could not be read, with why.
    pub errors: Vec<String>,
    /// Min/max columns of the main sample, for the waveform view.
    pub overview: Vec<(f32, f32)>,
}

/// An instrument's samples, shared by the editor, state loading and the
/// audio thread's hand-over. The locks are never taken on the audio thread.
pub struct Shared {
    kind: Kind,
    sample_rate: AtomicU32,
    data: Mutex<SampleData>,
    loaded: Mutex<Loaded>,
    sender: Mutex<handoff::Sender<Box<Instrument>>>,
}

/// Reads a sample file or built-in sound.
fn load(name: &str, rate: f32) -> Result<Arc<SampleBuffer>, String> {
    musiccraft_antiserum_presets::load_sample(name, rate)
        .ok_or_else(|| format!("cannot read {name}"))
}

/// Min/max columns of `buffer`, both channels together.
fn overview(buffer: &SampleBuffer) -> Vec<(f32, f32)> {
    let len = buffer.len();
    if len == 0 {
        return Vec::new();
    }
    let columns = OVERVIEW.min(len);
    (0..columns)
        .map(|c| {
            let (from, to) = (
                c * len / columns,
                ((c + 1) * len / columns).max(c * len / columns + 1),
            );
            buffer
                .left()
                .get(from..to)
                .into_iter()
                .chain(buffer.right().get(from..to))
                .flatten()
                .fold((0.0_f32, 0.0_f32), |(lo, hi), s| (lo.min(*s), hi.max(*s)))
        })
        .collect()
}

impl Shared {
    /// A new instrument's samples and the audio thread's end of its
    /// hand-over.
    #[must_use]
    pub fn new(kind: Kind) -> (Self, handoff::Receiver<Box<Instrument>>) {
        let (sender, receiver) = handoff::channel(QUEUE);
        (
            Self {
                kind,
                sample_rate: AtomicU32::new(0),
                data: Mutex::new(SampleData::default_for(kind)),
                loaded: Mutex::new(Loaded::default()),
                sender: Mutex::new(sender),
            },
            receiver,
        )
    }

    /// The instrument.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Records the sample rate the plugin was set up with.
    pub fn set_sample_rate(&self, rate: f32) {
        self.sample_rate.store(rate.to_bits(), Ordering::Release);
    }

    fn rate(&self) -> Option<f32> {
        let rate = f32::from_bits(self.sample_rate.load(Ordering::Acquire));
        (rate > 0.0).then_some(rate)
    }

    /// The samples, as the state names them.
    #[must_use]
    pub fn data(&self) -> SampleData {
        self.data.lock().map(|d| d.clone()).unwrap_or_default()
    }

    /// Errors and the waveform of the last load.
    #[must_use]
    pub fn loaded(&self) -> Loaded {
        self.loaded.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// Uses `data` from now on: loads it and, when the plugin is running,
    /// hands the new instrument to the audio thread.
    pub fn set_data(&self, data: SampleData) {
        if let Ok(mut current) = self.data.lock() {
            *current = data;
        }
        if self.rate().is_some() {
            let instrument = self.build();
            if let Ok(mut sender) = self.sender.lock() {
                let _ = sender.send(instrument);
            }
        }
    }

    /// Frees instruments the audio thread handed back.
    pub fn collect_garbage(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.collect();
        }
    }

    /// Loads the current samples into an instrument (main or GUI thread),
    /// recording errors and the waveform for the editor.
    #[must_use]
    pub fn build(&self) -> Box<Instrument> {
        let rate = self.rate().unwrap_or(48_000.0);
        let data = self.data();
        let mut loaded = Loaded::default();
        let one = |name: &str, loaded: &mut Loaded| -> Arc<SampleBuffer> {
            match load(name, rate) {
                Ok(buffer) => buffer,
                Err(error) => {
                    loaded.errors.push(error);
                    Arc::new(SampleBuffer::silent(f64::from(rate)))
                }
            }
        };
        let instrument = match self.kind {
            Kind::Sampler | Kind::Granular | Kind::Resynth => {
                let buffer = one(&data.sample, &mut loaded);
                loaded.overview = overview(&buffer);
                match self.kind {
                    Kind::Sampler => Instrument::Sampler(buffer),
                    Kind::Granular => Instrument::Granular(buffer),
                    _ => Instrument::Resynth(Arc::new(HarmonicTable::analyze(&buffer))),
                }
            }
            Kind::Kit => Instrument::Kit(
                (0..PADS)
                    .map(|pad| {
                        let name = data.pads.get(pad).filter(|p| !p.is_empty())?;
                        Some(one(name, &mut loaded))
                    })
                    .collect(),
            ),
            Kind::Zones => Instrument::Zones(
                data.zones
                    .iter()
                    .map(|z| Zone {
                        buffer: one(&z.sample, &mut loaded),
                        low: z.low,
                        high: z.high,
                        root: z.root,
                        vel_low: z.vel_low,
                        vel_high: z.vel_high,
                    })
                    .collect(),
            ),
            Kind::Synth => Instrument::Synth,
            Kind::DrumSynth => Instrument::DrumSynth,
            Kind::Pluck => Instrument::Pluck,
            Kind::Fm => Instrument::Fm,
            Kind::Keys => Instrument::Keys,
            Kind::Additive => Instrument::Additive,
            Kind::Layer | Kind::External | Kind::MidiOut => Instrument::Synth,
        };
        if let Ok(mut slot) = self.loaded.lock() {
            *slot = loaded;
        }
        Box::new(instrument)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_load_and_reach_the_audio_thread() {
        let (shared, mut receiver) = Shared::new(Kind::Kit);
        let built = shared.build();
        let Instrument::Kit(pads) = &*built else {
            panic!("a kit");
        };
        assert_eq!(pads.len(), PADS);
        assert_eq!(
            pads.iter().filter(|p| p.is_some()).count(),
            BUILTIN_SAMPLES.len()
        );
        // Before setup nothing is sent; afterwards a new set of pads is.
        let mut data = shared.data();
        data.pads[0] = "/no/such/file.wav".to_owned();
        shared.set_data(data.clone());
        let mut current = built;
        receiver.take(|new| std::mem::swap(&mut current, new));
        shared.set_sample_rate(48_000.0);
        shared.set_data(data);
        receiver.take(|new| std::mem::swap(&mut current, new));
        let Instrument::Kit(pads) = &*current else {
            panic!("a kit");
        };
        assert!(pads[0].as_ref().is_some_and(|p| p.is_empty()), "silent");
        assert_eq!(shared.loaded().errors, ["cannot read /no/such/file.wav"]);
        shared.collect_garbage();
    }

    #[test]
    fn sample_players_get_a_waveform() {
        let (shared, _) = Shared::new(Kind::Sampler);
        let built = shared.build();
        assert!(matches!(&*built, Instrument::Sampler(b) if !b.is_empty()));
        let loaded = shared.loaded();
        assert!(loaded.errors.is_empty());
        assert!(!loaded.overview.is_empty());
        assert!(loaded.overview.iter().any(|(lo, hi)| hi - lo > 0.1));
    }
}

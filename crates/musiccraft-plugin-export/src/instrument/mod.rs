//! One nih-plug plugin per exported built-in instrument:
//! `InstrumentPlugin<K>` plays `INSTRUMENT_KINDS[K]` with the app's own
//! voices and real-time note functions (`docs/spec/plugin-instruments.md`).

pub mod data;
mod editor;
pub mod params;

use std::num::NonZeroU32;
use std::sync::Arc;

use musiccraft_instruments::live::{self, Note, NoteFunctions, Poly, Timing};
use musiccraft_instruments::{Instrument, Kind};
use musiccraft_params::inst;
use nih_plug::prelude::*;

use crate::handoff;
use crate::ids::{
    INSTRUMENT_CLAP_IDS, INSTRUMENT_KINDS, INSTRUMENT_NAMES, INSTRUMENT_TITLES,
    instrument_features, instrument_subcategories, vst3_instrument_class_id,
};
use data::Shared;
use params::InstrumentParams;

/// Host events one block holds at most; later ones wait for the next block.
const MAX_EVENTS: usize = 1024;
/// Notes for the voices one block holds at most.
const MAX_NOTES: usize = 2048;

/// No audio input, stereo output.
pub(crate) const OUTPUT_ONLY: [AudioIOLayout; 1] = [AudioIOLayout {
    main_input_channels: None,
    main_output_channels: NonZeroU32::new(2),
    ..AudioIOLayout::const_default()
}];

/// The instrument played by plugin `K`.
pub const fn kind_of(k: usize) -> Kind {
    INSTRUMENT_KINDS[k]
}

/// What the host sent, besides notes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Control {
    /// Pitch bend, −1..1.
    Bend(f32),
    /// Cut the voices of this key.
    Choke(u8),
    /// All notes off.
    AllOff,
}

/// An exported instrument. See `docs/spec/plugin-instruments.md`.
pub struct InstrumentPlugin<const K: usize> {
    params: Arc<InstrumentParams>,
    instrument: Box<Instrument>,
    incoming: handoff::Receiver<Box<Instrument>>,
    notes: NoteFunctions,
    poly: Poly,
    sample_rate: f32,
    latency: u32,
    bend: f32,
    inputs: Box<[(u32, Note); MAX_EVENTS]>,
    controls: Box<[(u32, Control); MAX_EVENTS]>,
    out: Box<[(u32, Note); MAX_NOTES]>,
}

impl<const K: usize> Default for InstrumentPlugin<K> {
    fn default() -> Self {
        let kind = kind_of(K);
        let (shared, incoming) = Shared::new(kind);
        let shared = Arc::new(shared);
        let instrument = shared.build();
        Self {
            params: Arc::new(InstrumentParams::new(kind, INSTRUMENT_NAMES[K], shared)),
            instrument,
            incoming,
            notes: NoteFunctions::new(),
            poly: Poly::new(48_000.0),
            sample_rate: 48_000.0,
            latency: 0,
            bend: 0.0,
            inputs: Box::new([(0, Note::Off { key: 0 }); MAX_EVENTS]),
            controls: Box::new([(0, Control::AllOff); MAX_EVENTS]),
            out: Box::new([(0, Note::Off { key: 0 }); MAX_NOTES]),
        }
    }
}

impl<const K: usize> InstrumentPlugin<K> {
    /// Reads the block's host events: notes into `inputs`, the rest into
    /// `controls`. Returns how many of each.
    fn read_events(&mut self, context: &mut impl ProcessContext<Self>) -> (usize, usize) {
        let (mut notes, mut controls) = (0, 0);
        while let Some(event) = context.next_event() {
            let (timing, item) = match event {
                NoteEvent::NoteOn {
                    timing,
                    note,
                    velocity,
                    ..
                } => (
                    timing,
                    Ok(Note::On {
                        key: note,
                        velocity,
                    }),
                ),
                NoteEvent::NoteOff { timing, note, .. } => (timing, Ok(Note::Off { key: note })),
                NoteEvent::Choke { timing, note, .. } => (timing, Err(Control::Choke(note))),
                NoteEvent::MidiPitchBend { timing, value, .. } => {
                    (timing, Err(Control::Bend(value * 2.0 - 1.0)))
                }
                // All sound off, all notes off.
                NoteEvent::MidiCC { timing, cc, .. } if cc == 120 || cc == 123 => {
                    (timing, Err(Control::AllOff))
                }
                _ => continue,
            };
            match item {
                Ok(note) => {
                    if let Some(slot) = self.inputs.get_mut(notes) {
                        *slot = (timing, note);
                        notes += 1;
                    }
                }
                Err(control) => {
                    if let Some(slot) = self.controls.get_mut(controls) {
                        *slot = (timing, control);
                        controls += 1;
                    }
                }
            }
        }
        (notes, controls)
    }
}

impl<const K: usize> Plugin for InstrumentPlugin<K> {
    const NAME: &'static str = INSTRUMENT_TITLES[K];
    const VENDOR: &'static str = "MusicCraft";
    const URL: &'static str = "https://github.com/bhimio1/musiccraft";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &OUTPUT_ONLY;
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(editor::editor(Arc::clone(&self.params))))
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = config.sample_rate;
        let shared = &self.params.shared;
        shared.set_sample_rate(config.sample_rate);
        // Built here, off the audio thread, for this sample rate.
        self.poly = Poly::new(config.sample_rate);
        self.instrument = shared.build();
        shared.collect_garbage();
        true
    }

    fn reset(&mut self) {
        self.notes.reset();
        self.poly.reset();
        self.bend = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let instrument = &mut self.instrument;
        self.incoming.take(|new| std::mem::swap(instrument, new));
        let frames = buffer.samples();
        let (note_count, control_count) = self.read_events(context);
        let transport = context.transport();
        let tempo = transport.tempo.filter(|t| *t > 0.0).unwrap_or(120.0);
        let timing = Timing {
            tempo,
            sample_rate: f64::from(self.sample_rate),
            beats: transport.playing.then(|| transport.pos_beats()).flatten(),
        };
        let mut values = self.params.values();
        let latency = live::latency(&values, tempo, f64::from(self.sample_rate));
        if latency != self.latency {
            self.latency = latency;
            context.set_latency_samples(latency);
        }
        // The note functions turn the host's notes into the voices' notes.
        let mut out_count = 0;
        let out = &mut self.out;
        self.notes.run(
            &values,
            self.params.swing(),
            timing,
            frames as u32,
            self.inputs.get(..note_count).unwrap_or(&[]),
            &mut |frame, note| {
                if let Some(slot) = out.get_mut(out_count) {
                    *slot = (frame, note);
                    out_count += 1;
                }
            },
        );
        let [left, right, ..] = buffer.as_slice() else {
            return ProcessStatus::Normal;
        };
        left.fill(0.0);
        right.fill(0.0);
        let pitch = values.get(inst::PITCH).copied().unwrap_or(0.0);
        let range = self.params.bend_range();
        let (mut next_note, mut next_control) = (0, 0);
        let mut start = 0;
        while start < frames {
            // Everything due at `start`: controls, then notes.
            while let Some(&(at, control)) = self.controls.get(next_control) {
                if next_control >= control_count || at as usize > start {
                    break;
                }
                match control {
                    Control::Bend(bend) => self.bend = bend,
                    Control::Choke(key) => self.poly.choke(key),
                    Control::AllOff => {
                        self.poly.cut_all();
                        self.notes.reset();
                    }
                }
                next_control += 1;
            }
            if let Some(slot) = values.get_mut(inst::PITCH) {
                *slot = pitch + self.bend * range;
            }
            while let Some(&(at, note)) = self.out.get(next_note) {
                if next_note >= out_count || at as usize > start {
                    break;
                }
                match note {
                    Note::On { key, velocity } => {
                        self.poly.note_on(key, velocity, &self.instrument, &values);
                    }
                    Note::Off { key } => self.poly.note_off(key),
                }
                next_note += 1;
            }
            let next_out = (next_note < out_count)
                .then(|| self.out.get(next_note).map(|(f, _)| *f as usize))
                .flatten();
            let next_ctl = (next_control < control_count)
                .then(|| self.controls.get(next_control).map(|(f, _)| *f as usize))
                .flatten();
            let end = [next_out, next_ctl]
                .into_iter()
                .flatten()
                .filter(|f| *f > start)
                .fold(frames, usize::min);
            if let (Some(l), Some(r)) = (left.get_mut(start..end), right.get_mut(start..end)) {
                self.poly.render(&self.instrument, &values, l, r);
            }
            start = end;
        }
        ProcessStatus::Normal
    }
}

impl<const K: usize> ClapPlugin for InstrumentPlugin<K> {
    const CLAP_ID: &'static str = INSTRUMENT_CLAP_IDS[K];
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A MusicCraft instrument");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = instrument_features(kind_of(K));
}

impl<const K: usize> Vst3Plugin for InstrumentPlugin<K> {
    const VST3_CLASS_ID: [u8; 16] = vst3_instrument_class_id(INSTRUMENT_CLAP_IDS[K]);
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = instrument_subcategories(kind_of(K));
}

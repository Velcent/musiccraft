//! Antiserum as a plugin (`docs/spec/plugin-instruments.md`): the same
//! `musiccraft_antiserum::Antiserum` the app runs, its whole parameter table
//! as host parameters, the app's patch format as state, and the app's editor.

mod editor;
pub mod params;

use std::sync::Arc;

use musiccraft_antiserum::{Antiserum, Data, PARAM_COUNT};
use nih_plug::prelude::*;

use crate::handoff;
use crate::ids::{ANTISERUM_CLAP_ID, vst3_instrument_class_id};
use crate::instrument::OUTPUT_ONLY;
use params::AntiserumParams;

/// Antiserum, exported. See `docs/spec/plugin-instruments.md`.
pub struct AntiserumPlugin {
    params: Arc<AntiserumParams>,
    synth: Box<Antiserum>,
    incoming: handoff::Receiver<Data>,
    /// The values last given to the synth.
    sent: Box<[f32; PARAM_COUNT]>,
}

impl Default for AntiserumPlugin {
    fn default() -> Self {
        let (params, incoming) = AntiserumParams::new();
        Self {
            params: Arc::new(params),
            synth: Box::new(Antiserum::new(48_000.0)),
            incoming,
            sent: Box::new([f32::NAN; PARAM_COUNT]),
        }
    }
}

impl AntiserumPlugin {
    /// Gives the synth every parameter the host changed. Allocation-free.
    fn send_params(&mut self) {
        for slot in &self.params.slots {
            let value = slot.param.value();
            if let Some(sent) = self.sent.get_mut(slot.index) {
                if *sent != value {
                    *sent = value;
                    self.synth.set_param(slot.index, value);
                }
            }
        }
    }

    fn event(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => self.synth.note_on(note, velocity),
            NoteEvent::NoteOff { note, .. } | NoteEvent::Choke { note, .. } => {
                self.synth.note_off(note);
            }
            NoteEvent::MidiPitchBend { value, .. } => self.synth.set_pitch_bend(value * 2.0 - 1.0),
            NoteEvent::MidiCC { cc: 1, value, .. } => self.synth.set_mod_wheel(value),
            NoteEvent::MidiCC { cc: 120 | 123, .. } => self.synth.all_notes_off(),
            NoteEvent::MidiChannelPressure { pressure, .. } => self.synth.set_aftertouch(pressure),
            NoteEvent::PolyPressure { note, pressure, .. } => {
                self.synth.set_poly_aftertouch(note, pressure);
            }
            _ => {}
        }
    }
}

fn timing(event: &NoteEvent<()>) -> u32 {
    event.timing()
}

impl Plugin for AntiserumPlugin {
    const NAME: &'static str = "Antiserum";
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
        let shared = &self.params.shared;
        shared.set_sample_rate(config.sample_rate);
        // Built here, off the audio thread, for this sample rate.
        let mut synth = Box::new(Antiserum::new(config.sample_rate));
        for mut data in shared.all_data() {
            synth.swap(&mut data);
        }
        self.synth = synth;
        *self.sent = [f32::NAN; PARAM_COUNT];
        shared.collect_garbage();
        true
    }

    fn reset(&mut self) {
        self.synth.all_notes_off();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let synth = &mut self.synth;
        self.incoming.take(|data| synth.swap(data));
        self.send_params();
        let transport = context.transport();
        let tempo = transport.tempo.filter(|t| *t > 0.0).unwrap_or(120.0);
        self.synth.set_tempo(tempo as f32);
        if transport.playing
            && let Some(beats) = transport.pos_beats()
        {
            self.synth.set_beat_position(beats);
        }
        let frames = buffer.samples();
        let [left, right, ..] = buffer.as_slice() else {
            return ProcessStatus::Normal;
        };
        // Split the block at every event, so notes start on their frame.
        let mut start = 0;
        while let Some(event) = context.next_event() {
            let at = (timing(&event) as usize).min(frames);
            if at > start {
                if let (Some(l), Some(r)) = (left.get_mut(start..at), right.get_mut(start..at)) {
                    self.synth.process(l, r);
                }
                start = at;
            }
            self.event(event);
        }
        if let (Some(l), Some(r)) = (left.get_mut(start..frames), right.get_mut(start..frames)) {
            self.synth.process(l, r);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for AntiserumPlugin {
    const CLAP_ID: &'static str = ANTISERUM_CLAP_ID;
    const CLAP_DESCRIPTION: Option<&'static str> = Some("MusicCraft's wavetable synth");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for AntiserumPlugin {
    const VST3_CLASS_ID: [u8; 16] = vst3_instrument_class_id(ANTISERUM_CLAP_ID);
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

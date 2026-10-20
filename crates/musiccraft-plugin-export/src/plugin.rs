//! One nih-plug plugin per exported effect: `EffectPlugin<K>` wraps
//! `musiccraft_dsp::Effect` of kind [`KINDS`]`[K]`.

use std::num::NonZeroU32;
use std::sync::Arc;

use musiccraft_dsp::{Effect, ProcessContext as DspContext, flush_subnormals};
use musiccraft_params::EffectKind;
use nih_plug::prelude::*;

use crate::ids::{CLAP_IDS, KINDS, vst3_class_id};
use crate::params::EffectParams;
use crate::shared::{AudioSide, Shared};

/// Frames processed per call into the effect at most.
pub const MAX_BLOCK: usize = 512;

/// Stereo in and out.
const STEREO: [AudioIOLayout; 1] = [AudioIOLayout {
    main_input_channels: NonZeroU32::new(2),
    main_output_channels: NonZeroU32::new(2),
    ..AudioIOLayout::const_default()
}];

/// Stereo in and out with a stereo sidechain input.
const KEYED: [AudioIOLayout; 1] = [AudioIOLayout {
    main_input_channels: NonZeroU32::new(2),
    main_output_channels: NonZeroU32::new(2),
    aux_input_ports: &[new_nonzero_u32(2)],
    names: PortNames {
        aux_inputs: &["Sidechain"],
        ..PortNames::const_default()
    },
    ..AudioIOLayout::const_default()
}];

/// Whether `kind` takes a sidechain input.
pub const fn keyed(kind: EffectKind) -> bool {
    matches!(kind, EffectKind::Compressor | EffectKind::Vocoder)
}

/// Whether `kind` follows MIDI notes.
pub const fn follows_notes(kind: EffectKind) -> bool {
    matches!(kind, EffectKind::PitchCorrect)
}

/// The effect wrapped by plugin `K`.
pub const fn kind_of(k: usize) -> EffectKind {
    KINDS[k]
}

/// An exported effect. See `docs/spec/plugin-export.md`.
pub struct EffectPlugin<const K: usize> {
    params: Arc<EffectParams>,
    effect: Box<Effect>,
    audio: AudioSide,
    sample_rate: f32,
    /// Beats run while the host is stopped.
    free_beats: f64,
    /// MIDI keys held, for the pitch corrector's `follow`.
    held: [bool; 128],
}

impl<const K: usize> Default for EffectPlugin<K> {
    fn default() -> Self {
        let kind = kind_of(K);
        let (shared, audio) = Shared::new();
        let sample_rate = 48_000.0;
        Self {
            params: Arc::new(EffectParams::new(kind, Arc::new(shared))),
            effect: Box::new(Effect::new(kind, sample_rate)),
            audio,
            sample_rate,
            free_beats: 0.0,
            held: [false; 128],
        }
    }
}

impl<const K: usize> EffectPlugin<K> {
    /// The highest MIDI key held.
    fn highest_held(&self) -> Option<u8> {
        self.held.iter().rposition(|&on| on).map(|k| k as u8)
    }

    fn read_notes(&mut self, context: &mut impl ProcessContext<Self>) {
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, .. } => {
                    if let Some(key) = self.held.get_mut(usize::from(note)) {
                        *key = true;
                    }
                }
                NoteEvent::NoteOff { note, .. } | NoteEvent::Choke { note, .. } => {
                    if let Some(key) = self.held.get_mut(usize::from(note)) {
                        *key = false;
                    }
                }
                _ => {}
            }
        }
    }
}

impl<const K: usize> Plugin for EffectPlugin<K> {
    const NAME: &'static str = kind_of(K).title();
    const VENDOR: &'static str = "MusicCraft";
    const URL: &'static str = "https://github.com/bhimio1/musiccraft";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] =
        if keyed(kind_of(K)) { &KEYED } else { &STEREO };
    const MIDI_INPUT: MidiConfig = if follows_notes(kind_of(K)) {
        MidiConfig::Basic
    } else {
        MidiConfig::None
    };
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(crate::editor::editor(Arc::clone(&self.params))))
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let kind = kind_of(K);
        self.sample_rate = config.sample_rate;
        let shared = &self.params.shared;
        shared.set_sample_rate(config.sample_rate);
        // Built here, off the audio thread, for this sample rate.
        *self.effect = if kind == EffectKind::Convolution {
            shared.convolver(config.sample_rate)
        } else {
            Effect::new(kind, config.sample_rate)
        };
        shared.collect_garbage();
        true
    }

    fn reset(&mut self) {
        self.effect.reset();
        self.held = [false; 128];
        self.free_beats = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let kind = kind_of(K);
        self.audio.take_new(&mut self.effect);
        self.read_notes(context);
        let frames = buffer.samples();
        let transport = context.transport();
        let tempo = transport.tempo.filter(|t| *t > 0.0).unwrap_or(120.0);
        let host_beats = transport.playing.then(|| transport.pos_beats()).flatten();
        let start_beats = host_beats.unwrap_or(self.free_beats);
        let beats_per_frame = tempo / 60.0 / f64::from(self.sample_rate);
        self.free_beats += frames as f64 * beats_per_frame;

        if self.params.bypass.value() {
            return ProcessStatus::Normal;
        }
        for channel in buffer.as_slice().iter_mut() {
            flush_subnormals(channel);
        }
        for input in aux.inputs.iter_mut() {
            for channel in input.as_slice().iter_mut() {
                flush_subnormals(channel);
            }
        }
        let values = self.params.values();
        let held = self.highest_held();
        if let Effect::PitchCorrect(corrector) = self.effect.as_mut() {
            corrector.set_follow(held);
        }
        let sidechain_on = keyed(kind)
            && self
                .params
                .slot("sidechain")
                .is_some_and(|s| s.param.value() >= 0.5);
        let key = match aux.inputs.first_mut() {
            Some(input) if sidechain_on => match input.as_slice() {
                [l, r, ..] => Some((&**l, &**r)),
                _ => None,
            },
            _ => None,
        };
        let [left, right, ..] = buffer.as_slice() else {
            return ProcessStatus::Normal;
        };
        let mut start = 0;
        while start < frames {
            let end = (start + MAX_BLOCK).min(frames);
            let ctx = DspContext {
                sample_rate: self.sample_rate,
                tempo_bpm: tempo,
                beats: start_beats + start as f64 * beats_per_frame,
            };
            let block_key = key.map(|(l, r)| {
                (
                    l.get(start..end).unwrap_or(&[]),
                    r.get(start..end).unwrap_or(&[]),
                )
            });
            if let (Some(l), Some(r)) = (left.get_mut(start..end), right.get_mut(start..end)) {
                self.effect.process_keyed(l, r, &values, ctx, block_key);
            }
            start = end;
        }
        self.params
            .shared
            .set_detected(self.effect.detected_pitch());
        ProcessStatus::Normal
    }
}

impl<const K: usize> ClapPlugin for EffectPlugin<K> {
    const CLAP_ID: &'static str = CLAP_IDS[K];
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A MusicCraft effect");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = crate::ids::clap_features(kind_of(K));
}

impl<const K: usize> Vst3Plugin for EffectPlugin<K> {
    const VST3_CLASS_ID: [u8; 16] = vst3_class_id(CLAP_IDS[K]);
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        crate::ids::vst3_subcategories(kind_of(K));
}

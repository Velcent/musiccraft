use std::sync::Arc;

use musiccraft_dsp::{Effect, PanLaw, SampleBuffer};
use musiccraft_params::{INSTRUMENT_PARAMS, MAX_EFFECT_PARAMS, MAX_INSTRUMENT_PARAMS, defaults};

use crate::MAX_BLOCK;

/// Per-channel settings that can change without rebuilding the graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelParams {
    /// Linear gain.
    pub volume: f32,
    /// Pan, -1 (left) to 1 (right).
    pub pan: f32,
    /// How `pan` splits the channel between left and right.
    pub pan_law: PanLaw,
    /// Whether the channel is silenced.
    pub mute: bool,
    /// Mixer track that receives this channel (0 = master).
    pub track: u16,
    /// Instrument parameters indexed by `musiccraft_params::inst`.
    pub instrument: [f32; MAX_INSTRUMENT_PARAMS],
}

impl Default for ChannelParams {
    fn default() -> Self {
        Self {
            volume: 0.8,
            pan: 0.0,
            pan_law: PanLaw::default(),
            mute: false,
            track: 0,
            instrument: defaults(INSTRUMENT_PARAMS),
        }
    }
}

impl ChannelParams {
    /// Left/right gains from volume, pan and mute.
    #[must_use]
    pub fn gains(&self) -> (f32, f32) {
        if self.mute {
            return (0.0, 0.0);
        }
        let (l, r) = self.pan_law.gains(self.pan);
        (self.volume * l, self.volume * r)
    }
}

impl TrackParams {
    /// Applies polarity, side swap and stereo separation to a block.
    pub(crate) fn stereo(&self, left: &mut [f32], right: &mut [f32]) {
        let separation = if self.separation.is_finite() {
            self.separation.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        if !self.invert && !self.swap && separation == 0.0 {
            return;
        }
        let polarity = if self.invert { -1.0 } else { 1.0 };
        let width = 1.0 + separation;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (a, b) = if self.swap { (*r, *l) } else { (*l, *r) };
            let mid = (a + b) * 0.5;
            let side = (a - b) * 0.5 * width;
            *l = (mid + side) * polarity;
            *r = (mid - side) * polarity;
        }
    }

    /// Left/right fader gains from volume, pan and mute.
    #[must_use]
    pub fn gains(&self) -> (f32, f32) {
        if self.mute {
            return (0.0, 0.0);
        }
        let (l, r) = musiccraft_dsp::pan_gains(self.pan);
        (self.volume * l, self.volume * r)
    }
}

/// Most sends one mixer track can have.
pub const MAX_SENDS: usize = 8;

/// A post-fader send from one insert to a higher-numbered insert.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SendParams {
    /// Receiving track; must be greater than the sending track's index.
    pub target: u16,
    /// Linear send level.
    pub level: f32,
}

/// Per-track mixer settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackParams {
    /// Linear gain.
    pub volume: f32,
    /// Balance, -1 (left) to 1 (right).
    pub pan: f32,
    /// Whether the track is silenced (solo is resolved into mute by the sender).
    pub mute: bool,
    /// Whether an insert feeds the master (ignored for the master itself).
    pub to_master: bool,
    /// Sends; only the first `send_count` are used.
    pub sends: [SendParams; MAX_SENDS],
    /// Number of active sends.
    pub send_count: u8,
    /// Stereo separation, -1 (mono) .. 0 (unchanged) .. 1 (side doubled).
    pub separation: f32,
    /// Flip the polarity of both sides.
    pub invert: bool,
    /// Exchange left and right.
    pub swap: bool,
    /// The built-in EQ, as the three-band EQ effect's parameters
    /// `[low gain, low freq, mid gain, mid freq, mid q, high gain, high freq]`.
    pub eq: [f32; 7],
}

/// A flat track EQ.
pub const FLAT_EQ: [f32; 7] = [0.0, 120.0, 0.0, 1_000.0, 0.7, 0.0, 8_000.0];

impl Default for TrackParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            mute: false,
            to_master: true,
            sends: [SendParams::default(); MAX_SENDS],
            send_count: 0,
            separation: 0.0,
            invert: false,
            swap: false,
            eq: FLAT_EQ,
        }
    }
}

impl TrackParams {
    /// Whether the EQ changes anything (any band has gain).
    #[must_use]
    pub fn eq_active(&self) -> bool {
        [0, 2, 5]
            .iter()
            .any(|&i| self.eq.get(i).is_some_and(|g| g.abs() > 1e-4))
    }

    /// The active sends.
    #[must_use]
    pub fn active_sends(&self) -> &[SendParams] {
        let count = usize::from(self.send_count).min(MAX_SENDS);
        self.sends.get(..count).unwrap_or_default()
    }
}

/// Settings of one effect slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParams {
    /// When false the slot is bypassed.
    pub enabled: bool,
    /// Parameter values in the order of the effect's parameter table.
    pub values: [f32; MAX_EFFECT_PARAMS],
    /// Dry/wet mix, 0..=1.
    pub mix: f32,
}

pub use musiccraft_instruments::{Instrument, Zone};

/// A channel: instrument, settings and a scratch buffer.
#[derive(Debug, Clone)]
pub struct ChannelNode {
    pub(crate) instrument: Instrument,
    pub(crate) params: ChannelParams,
    /// Left/right gains applied at the end of the last block; the next block
    /// ramps from these to the current settings.
    pub(crate) gains: (f32, f32),
    pub(crate) left: Vec<f32>,
    pub(crate) right: Vec<f32>,
    /// MIDI Out: the control-change values and program last sent (`u8::MAX`
    /// = not sent yet).
    pub(crate) midi_sent: [u8; musiccraft_params::MIDI_CCS + 1],
}

#[derive(Debug, Clone)]
pub(crate) struct EffectSlot {
    pub(crate) effect: Effect,
    pub(crate) params: EffectParams,
    /// When set, the external processor in this engine slot does the work.
    pub(crate) external: Option<u16>,
}

/// A mixer track: settings, effect chain and a scratch buffer.
#[derive(Debug, Clone)]
pub struct TrackNode {
    pub(crate) params: TrackParams,
    /// Fader gains at the end of the last block (see [`ChannelNode`]).
    pub(crate) gains: (f32, f32),
    pub(crate) effects: Vec<EffectSlot>,
    pub(crate) left: Vec<f32>,
    pub(crate) right: Vec<f32>,
    /// Louder side's peak in the last block, for peak controllers.
    pub(crate) last_peak: f32,
    /// The built-in EQ's filter state.
    pub(crate) eq: musiccraft_dsp::ThreeBandEq,
    /// The input of an effect slot with a partial mix, kept for blending.
    pub(crate) dry_left: Vec<f32>,
    pub(crate) dry_right: Vec<f32>,
}

/// Most tracks whose peaks controllers can follow.
pub(crate) const MAX_PEAK_TRACKS: usize = musiccraft_params::MAX_MIXER_TRACKS;

/// Everything the engine renders except the note sequence. Built on the main
/// thread; every buffer is allocated here so the audio thread never allocates.
#[derive(Debug, Clone)]
pub struct Graph {
    pub(crate) channels: Vec<ChannelNode>,
    pub(crate) tracks: Vec<TrackNode>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// A graph with only the master track.
    #[must_use]
    pub fn new() -> Self {
        let mut graph = Self {
            channels: Vec::new(),
            tracks: Vec::new(),
        };
        graph.add_track(TrackParams::default(), Vec::new());
        graph
    }

    /// Appends a channel and returns its index.
    pub fn add_channel(&mut self, instrument: Instrument, params: ChannelParams) -> usize {
        self.channels.push(ChannelNode {
            instrument,
            gains: params.gains(),
            params,
            left: vec![0.0; MAX_BLOCK],
            right: vec![0.0; MAX_BLOCK],
            midi_sent: [u8::MAX; musiccraft_params::MIDI_CCS + 1],
        });
        self.channels.len() - 1
    }

    /// Appends a mixer track with an effect chain and returns its index. The
    /// first track is the master.
    pub fn add_track(
        &mut self,
        params: TrackParams,
        effects: Vec<(Effect, EffectParams)>,
    ) -> usize {
        self.tracks.push(TrackNode {
            gains: params.gains(),
            params,
            effects: effects
                .into_iter()
                .map(|(effect, params)| EffectSlot {
                    effect,
                    params,
                    external: None,
                })
                .collect(),
            left: vec![0.0; MAX_BLOCK],
            right: vec![0.0; MAX_BLOCK],
            last_peak: 0.0,
            eq: musiccraft_dsp::ThreeBandEq::default(),
            dry_left: vec![0.0; MAX_BLOCK],
            dry_right: vec![0.0; MAX_BLOCK],
        });
        self.tracks.len() - 1
    }

    /// Replaces the master track's settings and effects.
    pub fn set_master(&mut self, params: TrackParams, effects: Vec<(Effect, EffectParams)>) {
        if let Some(master) = self.tracks.first_mut() {
            master.gains = params.gains();
            master.params = params;
            master.effects = effects
                .into_iter()
                .map(|(effect, params)| EffectSlot {
                    effect,
                    params,
                    external: None,
                })
                .collect();
        }
    }

    /// Makes effect `slot` of `track` run the external processor in engine
    /// slot `external`. Returns `false` if there is no such effect slot.
    pub fn attach_external(&mut self, track: usize, slot: usize, external: u16) -> bool {
        match self
            .tracks
            .get_mut(track)
            .and_then(|t| t.effects.get_mut(slot))
        {
            Some(effect) => {
                effect.external = Some(external);
                true
            }
            None => false,
        }
    }

    /// Number of channels.
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Number of mixer tracks including master.
    #[must_use]
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }
}

/// A sample auditioned from the browser, played straight to the output.
#[derive(Debug, Clone)]
pub struct Preview {
    pub(crate) buffer: Arc<SampleBuffer>,
    pub(crate) position: f64,
    pub(crate) gain: f32,
}

impl Preview {
    /// A preview of `buffer` at `gain`, starting from the beginning.
    #[must_use]
    pub fn new(buffer: Arc<SampleBuffer>, gain: f32) -> Self {
        Self {
            buffer,
            position: 0.0,
            gain,
        }
    }

    /// Adds the next frames into `left`/`right`.
    pub(crate) fn render(&mut self, left: &mut [f32], right: &mut [f32], sample_rate: f64) {
        let step = self.buffer.sample_rate() / sample_rate.max(1.0);
        let end = self.buffer.len() as f64;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if self.position >= end {
                return;
            }
            let (sl, sr) = self.buffer.read(self.position);
            *l += sl * self.gain;
            *r += sr * self.gain;
            self.position += step;
        }
    }
}

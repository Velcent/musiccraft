use std::sync::Arc;

use musiccraft_dsp::{Effect, SampleBuffer};
use musiccraft_params::{INSTRUMENT_PARAMS, MAX_EFFECT_PARAMS, MAX_INSTRUMENT_PARAMS, defaults};

use crate::MAX_BLOCK;

/// Per-channel settings that can change without rebuilding the graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelParams {
    /// Linear gain.
    pub volume: f32,
    /// Balance, -1 (left) to 1 (right).
    pub pan: f32,
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
            mute: false,
            track: 0,
            instrument: defaults(INSTRUMENT_PARAMS),
        }
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
}

impl Default for TrackParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            mute: false,
            to_master: true,
            sends: [SendParams::default(); MAX_SENDS],
            send_count: 0,
        }
    }
}

impl TrackParams {
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
}

/// The sound source of a channel.
#[derive(Debug, Clone)]
pub enum Instrument {
    /// Plays a sample, pitched relative to the `root_key` parameter.
    Sampler(Arc<SampleBuffer>),
    /// Two-oscillator subtractive synthesizer.
    Synth,
}

/// A channel: instrument, settings and a scratch buffer.
#[derive(Debug, Clone)]
pub struct ChannelNode {
    pub(crate) instrument: Instrument,
    pub(crate) params: ChannelParams,
    pub(crate) left: Vec<f32>,
    pub(crate) right: Vec<f32>,
}

#[derive(Debug, Clone)]
pub(crate) struct EffectSlot {
    pub(crate) effect: Effect,
    pub(crate) params: EffectParams,
}

/// A mixer track: settings, effect chain and a scratch buffer.
#[derive(Debug, Clone)]
pub struct TrackNode {
    pub(crate) params: TrackParams,
    pub(crate) effects: Vec<EffectSlot>,
    pub(crate) left: Vec<f32>,
    pub(crate) right: Vec<f32>,
}

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
            params,
            left: vec![0.0; MAX_BLOCK],
            right: vec![0.0; MAX_BLOCK],
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
            params,
            effects: effects
                .into_iter()
                .map(|(effect, params)| EffectSlot { effect, params })
                .collect(),
            left: vec![0.0; MAX_BLOCK],
            right: vec![0.0; MAX_BLOCK],
        });
        self.tracks.len() - 1
    }

    /// Replaces the master track's settings and effects.
    pub fn set_master(&mut self, params: TrackParams, effects: Vec<(Effect, EffectParams)>) {
        if let Some(master) = self.tracks.first_mut() {
            master.params = params;
            master.effects = effects
                .into_iter()
                .map(|(effect, params)| EffectSlot { effect, params })
                .collect();
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

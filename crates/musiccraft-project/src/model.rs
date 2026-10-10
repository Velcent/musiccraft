//! Plain data types that make up a project.

use std::collections::{BTreeMap, BTreeSet};

use musiccraft_params::{
    EffectKind, INSTRUMENT_PARAMS, MAX_EFFECT_PARAMS, MAX_INSTRUMENT_PARAMS, ParamSpec, defaults,
    inst,
};
use serde::{Deserialize, Serialize};

/// Stable identity of a channel; survives reordering and removal of others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelId(pub u32);

/// Stable identity of a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PatternId(pub u32);

/// Lowest channel or mixer volume (linear gain).
pub const MIN_VOLUME: f32 = 0.0;
/// Highest channel or mixer volume (linear gain, about +1.9 dB).
pub const MAX_VOLUME: f32 = 1.25;
/// Default channel volume.
pub const DEFAULT_CHANNEL_VOLUME: f32 = 0.8;
/// Most steps a pattern may have.
pub const MAX_PATTERN_STEPS: u32 = 1024;
/// Number of playlist track rows.
pub const PLAYLIST_TRACKS: u32 = 64;
/// Inserts in a new project's mixer (plus master).
pub const DEFAULT_INSERTS: usize = 8;
/// Key that steps toggled in the channel rack play.
pub const STEP_KEY: u8 = 60;
/// Velocity of notes created without an explicit velocity.
pub const DEFAULT_VELOCITY: u8 = 100;

/// Values for a parameter table, stored as `name → value` so files stay readable
/// and survive tables gaining parameters.
fn params_to_map(specs: &[ParamSpec], values: &[f32]) -> BTreeMap<String, f32> {
    specs
        .iter()
        .zip(values)
        .map(|(spec, value)| (spec.name.to_owned(), *value))
        .collect()
}

fn params_from_map<const N: usize>(specs: &[ParamSpec], map: &BTreeMap<String, f32>) -> [f32; N] {
    let mut values: [f32; N] = defaults(specs);
    for (value, spec) in values.iter_mut().zip(specs) {
        if let Some(stored) = map.get(spec.name) {
            if stored.is_finite() {
                *value = spec.sanitize(*stored);
            }
        }
    }
    values
}

/// Instrument parameter values, indexed by `musiccraft_params::inst`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(into = "BTreeMap<String, f32>", from = "BTreeMap<String, f32>")]
pub struct InstrumentParams(pub [f32; MAX_INSTRUMENT_PARAMS]);

impl Default for InstrumentParams {
    fn default() -> Self {
        Self(defaults(INSTRUMENT_PARAMS))
    }
}

impl InstrumentParams {
    /// Defaults suited to a sampler: full sustain so samples play unshaped.
    #[must_use]
    pub fn sampler() -> Self {
        let mut params = Self::default();
        params.0[inst::DECAY] = 0.0;
        params.0[inst::SUSTAIN] = 1.0;
        params.0[inst::RELEASE] = 0.05;
        params
    }

    /// Value at an index from `musiccraft_params::inst`.
    #[must_use]
    pub fn get(&self, index: usize) -> f32 {
        self.0.get(index).copied().unwrap_or(0.0)
    }
}

impl From<InstrumentParams> for BTreeMap<String, f32> {
    fn from(params: InstrumentParams) -> Self {
        params_to_map(INSTRUMENT_PARAMS, &params.0)
    }
}

impl From<BTreeMap<String, f32>> for InstrumentParams {
    fn from(map: BTreeMap<String, f32>) -> Self {
        Self(params_from_map(INSTRUMENT_PARAMS, &map))
    }
}

/// The sound source of a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Instrument {
    /// Plays an audio file or a built-in sound (`builtin:<name>`).
    Sampler {
        /// File path (absolute, or relative to the project file) or built-in name.
        sample: String,
    },
    /// Two-oscillator subtractive synthesizer.
    Synth,
}

impl Instrument {
    /// Short label: `sampler` or `synth`.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sampler { .. } => "sampler",
            Self::Synth => "synth",
        }
    }
}

/// A row in the channel rack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    /// Stable identity.
    pub id: ChannelId,
    /// Display name.
    pub name: String,
    /// Sound source.
    pub instrument: Instrument,
    /// Instrument parameters.
    #[serde(default)]
    pub params: InstrumentParams,
    /// Linear gain.
    pub volume: f32,
    /// Balance, -1..=1.
    pub pan: f32,
    /// Whether the channel is silenced.
    #[serde(default)]
    pub mute: bool,
    /// Mixer track receiving this channel; 0 is master.
    #[serde(default)]
    pub mixer_track: usize,
}

impl Channel {
    /// A channel with default settings.
    #[must_use]
    pub fn new(id: ChannelId, name: impl Into<String>, instrument: Instrument) -> Self {
        let params = match instrument {
            Instrument::Sampler { .. } => InstrumentParams::sampler(),
            Instrument::Synth => InstrumentParams::default(),
        };
        Self {
            id,
            name: name.into(),
            instrument,
            params,
            volume: DEFAULT_CHANNEL_VOLUME,
            pan: 0.0,
            mute: false,
            mixer_track: 0,
        }
    }
}

/// A note in a pattern. Times are in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Channel that plays the note.
    pub channel: ChannelId,
    /// Start, in ticks from the pattern start.
    pub start: u32,
    /// Duration in ticks (at least 1).
    pub length: u32,
    /// MIDI key, 0..=127.
    pub key: u8,
    /// Velocity, 1..=127.
    pub velocity: u8,
}

/// A pattern: notes for any number of channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    /// Stable identity.
    pub id: PatternId,
    /// Display name.
    pub name: String,
    /// Minimum length in steps; notes past the end extend the pattern by bars.
    pub length_steps: u32,
    /// Notes, kept sorted by (channel, start, key).
    #[serde(default)]
    pub notes: Vec<Note>,
}

impl Pattern {
    /// An empty one-bar pattern.
    #[must_use]
    pub fn new(id: PatternId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            length_steps: 16,
            notes: Vec::new(),
        }
    }

    /// Notes of one channel.
    pub fn notes_of(&self, channel: ChannelId) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |n| n.channel == channel)
    }

    /// Index of the note of `channel` at `start` with `key`.
    #[must_use]
    pub fn find_note(&self, channel: ChannelId, start: u32, key: u8) -> Option<usize> {
        self.notes
            .iter()
            .position(|n| n.channel == channel && n.start == start && n.key == key)
    }

    /// Inserts a note, replacing one at the same channel, start and key. Returns
    /// the replaced note.
    pub fn put_note(&mut self, note: Note) -> Option<Note> {
        let replaced = self
            .find_note(note.channel, note.start, note.key)
            .map(|i| self.notes.remove(i));
        self.notes.push(note);
        self.sort_notes();
        replaced
    }

    /// Keeps notes in canonical order.
    pub fn sort_notes(&mut self) {
        self.notes.sort_by_key(|n| (n.channel, n.start, n.key));
    }

    /// Length in ticks: the step length, extended to whole bars to fit every note.
    #[must_use]
    pub fn length_ticks(&self) -> u32 {
        let steps = self.length_steps.max(1) * musiccraft_params::TICKS_PER_STEP;
        let last = self
            .notes
            .iter()
            .map(|n| n.start.saturating_add(1))
            .max()
            .unwrap_or(0);
        if last > steps {
            crate::music::round_up_to_bar(last)
        } else {
            steps
        }
    }
}

/// One effect slot on a mixer track.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(into = "EffectRepr", from = "EffectRepr")]
pub struct EffectSlot {
    /// Which effect.
    pub kind: EffectKind,
    /// Whether the effect processes audio (false = bypassed).
    pub enabled: bool,
    /// Parameter values in the order of `kind.params()`.
    pub values: [f32; MAX_EFFECT_PARAMS],
}

impl EffectSlot {
    /// A new enabled effect with default settings.
    #[must_use]
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            enabled: true,
            values: kind.defaults(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EffectRepr {
    kind: EffectKind,
    enabled: bool,
    params: BTreeMap<String, f32>,
}

impl From<EffectSlot> for EffectRepr {
    fn from(slot: EffectSlot) -> Self {
        Self {
            kind: slot.kind,
            enabled: slot.enabled,
            params: params_to_map(slot.kind.params(), &slot.values),
        }
    }
}

impl From<EffectRepr> for EffectSlot {
    fn from(repr: EffectRepr) -> Self {
        Self {
            kind: repr.kind,
            enabled: repr.enabled,
            values: params_from_map(repr.kind.params(), &repr.params),
        }
    }
}

/// A mixer track. Track 0 is the master; the others are inserts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixerTrack {
    /// Display name.
    pub name: String,
    /// Linear gain.
    pub volume: f32,
    /// Balance, -1..=1.
    pub pan: f32,
    /// Whether the track is silenced.
    #[serde(default)]
    pub mute: bool,
    /// Whether the track is soloed.
    #[serde(default)]
    pub solo: bool,
    /// Insert effects, processed in order.
    #[serde(default)]
    pub effects: Vec<EffectSlot>,
}

impl MixerTrack {
    /// A track at unity gain with no effects.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            effects: Vec::new(),
        }
    }
}

/// A pattern placed on the playlist. Times are in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    /// Pattern played by the clip.
    pub pattern: PatternId,
    /// Track row, 0-based.
    pub track: u32,
    /// Start in ticks from the song start.
    pub start: u32,
    /// Duration in ticks; the pattern repeats if the clip is longer.
    pub length: u32,
}

impl Clip {
    /// End tick (exclusive).
    #[must_use]
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.length)
    }
}

/// The song arrangement.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Playlist {
    /// Placed clips, sorted by (track, start).
    #[serde(default)]
    pub clips: Vec<Clip>,
    /// Track rows that are muted (0-based).
    #[serde(default)]
    pub muted_tracks: BTreeSet<u32>,
}

impl Playlist {
    /// Keeps clips in canonical order.
    pub fn sort_clips(&mut self) {
        self.clips.sort_by_key(|c| (c.track, c.start));
    }

    /// Index of the clip on `track` that covers tick `at`.
    #[must_use]
    pub fn clip_at(&self, track: u32, at: u32) -> Option<usize> {
        self.clips
            .iter()
            .position(|c| c.track == track && c.start <= at && at < c.end())
    }

    /// End of the last clip in ticks.
    #[must_use]
    pub fn length_ticks(&self) -> u32 {
        self.clips.iter().map(Clip::end).max().unwrap_or(0)
    }
}

/// What the transport plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    /// Loop the selected pattern.
    #[default]
    Pattern,
    /// Play the playlist arrangement.
    Song,
}

impl PlayMode {
    /// `pattern` or `song`.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pattern => "pattern",
            Self::Song => "song",
        }
    }
}

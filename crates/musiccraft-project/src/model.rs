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

/// A hosted (CLAP) plugin and everything the project keeps about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginRef {
    /// CLAP plugin id, for example `com.example.synth`.
    pub id: String,
    /// Plugin name when it was added (shown if the plugin is not installed).
    pub name: String,
    /// Instance number, unique within the project. The host keeps one live
    /// plugin instance per number, so moving a slot keeps its instance.
    pub instance: u32,
    /// Parameter values by CLAP parameter id, in the plugin's plain units.
    /// Applied after `state`, so they win over it.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "param_map"
    )]
    pub params: BTreeMap<u32, f64>,
    /// The plugin's saved state, base64-encoded; empty if never saved.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
}

/// Stores plugin parameter maps with string keys. Integer keys do not survive
/// serde's buffering inside internally tagged enums such as [`Instrument`].
mod param_map {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(map: &BTreeMap<u32, f64>, s: S) -> Result<S::Ok, S::Error> {
        let text: BTreeMap<String, f64> = map.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        text.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<u32, f64>, D::Error> {
        let text = BTreeMap::<String, f64>::deserialize(d)?;
        text.into_iter()
            .filter(|(_, v)| v.is_finite())
            .map(|(k, v)| {
                k.parse()
                    .map(|k| (k, v))
                    .map_err(|_| serde::de::Error::custom(format!("`{k}` is not a parameter id")))
            })
            .collect()
    }
}

impl PluginRef {
    /// A reference with no stored parameters or state.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, instance: u32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            instance,
            params: BTreeMap::new(),
            state: String::new(),
        }
    }
}

/// Where a plugin sits in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginSlot {
    /// The instrument of the channel at this index (0-based).
    Channel(usize),
    /// An effect: mixer track index (0 = master) and slot index (0-based).
    Effect(usize, usize),
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
    /// A hosted plugin instrument.
    Plugin {
        /// The plugin.
        plugin: PluginRef,
    },
}

impl Instrument {
    /// Short label: `sampler`, `synth` or `plugin`.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sampler { .. } => "sampler",
            Self::Synth => "synth",
            Self::Plugin { .. } => "plugin",
        }
    }

    /// The plugin, for a plugin instrument.
    #[must_use]
    pub fn plugin(&self) -> Option<&PluginRef> {
        match self {
            Self::Plugin { plugin } => Some(plugin),
            _ => None,
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
            Instrument::Synth | Instrument::Plugin { .. } => InstrumentParams::default(),
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
    /// Pan, -100 (left) to 100 (right).
    #[serde(default)]
    pub pan: i8,
    /// A slide note glides the sounding note of its channel to its key
    /// instead of starting a new one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub slide: bool,
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
    /// Swing amount 0..=1: notes on off-beat sixteenths are delayed by up to
    /// half a step.
    #[serde(default)]
    pub swing: f32,
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
            swing: 0.0,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "EffectRepr", from = "EffectRepr")]
pub struct EffectSlot {
    /// Which effect; [`EffectKind::Plugin`] when `plugin` is set.
    pub kind: EffectKind,
    /// Whether the effect processes audio (false = bypassed).
    pub enabled: bool,
    /// Parameter values in the order of `kind.params()`.
    pub values: [f32; MAX_EFFECT_PARAMS],
    /// The hosted plugin of a plugin slot.
    pub plugin: Option<Box<PluginRef>>,
}

impl EffectSlot {
    /// A new enabled effect with default settings.
    #[must_use]
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            enabled: true,
            values: kind.defaults(),
            plugin: None,
        }
    }

    /// A new enabled plugin effect.
    #[must_use]
    pub fn plugin(plugin: PluginRef) -> Self {
        Self {
            plugin: Some(Box::new(plugin)),
            ..Self::new(EffectKind::Plugin)
        }
    }

    /// Display name: the plugin's name or the effect kind.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.plugin {
            Some(plugin) => plugin.name.clone(),
            None => self.kind.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EffectRepr {
    kind: EffectKind,
    enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    params: BTreeMap<String, f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plugin: Option<PluginRef>,
}

impl From<EffectSlot> for EffectRepr {
    fn from(slot: EffectSlot) -> Self {
        Self {
            kind: slot.kind,
            enabled: slot.enabled,
            params: params_to_map(slot.kind.params(), &slot.values),
            plugin: slot.plugin.map(|p| *p),
        }
    }
}

impl From<EffectRepr> for EffectSlot {
    fn from(repr: EffectRepr) -> Self {
        // A plugin slot without its plugin cannot play; keep it as a bypassed
        // passthrough rather than failing the whole file.
        let enabled = repr.enabled && (repr.kind != EffectKind::Plugin || repr.plugin.is_some());
        Self {
            kind: repr.kind,
            enabled,
            values: params_from_map(repr.kind.params(), &repr.params),
            plugin: repr.plugin.map(Box::new),
        }
    }
}

/// A post-fader send to a higher-numbered insert.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MixerSend {
    /// Receiving mixer track (greater than the sending track).
    pub target: usize,
    /// Linear level, 0 to [`MAX_VOLUME`].
    pub level: f32,
}

/// Most sends a mixer track can have.
pub const MAX_SENDS: usize = 8;

fn yes() -> bool {
    true
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
    /// Whether this insert feeds the master (always true for the master).
    #[serde(default = "yes")]
    pub to_master: bool,
    /// Sends to higher-numbered inserts, sorted by target.
    #[serde(default)]
    pub sends: Vec<MixerSend>,
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
            to_master: true,
            sends: Vec::new(),
        }
    }
}

/// Stable identity of an automation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutomationId(pub u32);

/// What a playlist clip plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipSource {
    /// A pattern, repeated for the clip's length.
    Pattern(PatternId),
    /// An automation curve.
    Automation(AutomationId),
    /// A sampler channel's sample, played once from the clip's start.
    Audio(ChannelId),
}

/// Something placed on the playlist. Times are in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    /// What the clip plays.
    #[serde(flatten)]
    pub source: ClipSource,
    /// Track row, 0-based.
    pub track: u32,
    /// Start in ticks from the song start.
    pub start: u32,
    /// Duration in ticks; a pattern repeats if the clip is longer.
    pub length: u32,
}

impl Clip {
    /// The pattern a pattern clip plays.
    #[must_use]
    pub fn pattern(&self) -> Option<PatternId> {
        match self.source {
            ClipSource::Pattern(id) => Some(id),
            _ => None,
        }
    }

    /// The automation an automation clip plays.
    #[must_use]
    pub fn automation(&self) -> Option<AutomationId> {
        match self.source {
            ClipSource::Automation(id) => Some(id),
            _ => None,
        }
    }

    /// The channel an audio clip plays.
    #[must_use]
    pub fn audio(&self) -> Option<ChannelId> {
        match self.source {
            ClipSource::Audio(id) => Some(id),
            _ => None,
        }
    }
}

/// A parameter an automation drives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationTarget {
    /// A channel's volume.
    ChannelVolume {
        /// The channel.
        channel: ChannelId,
    },
    /// A channel's pan.
    ChannelPan {
        /// The channel.
        channel: ChannelId,
    },
    /// An instrument parameter of a channel.
    ChannelParam {
        /// The channel.
        channel: ChannelId,
        /// Parameter name from the instrument table.
        param: String,
    },
    /// A mixer track's volume.
    MixerVolume {
        /// Track index (0 = master).
        track: usize,
    },
    /// A mixer track's pan.
    MixerPan {
        /// Track index (0 = master).
        track: usize,
    },
    /// A parameter of an effect slot.
    EffectParam {
        /// Track index (0 = master).
        track: usize,
        /// Slot index (0-based).
        slot: usize,
        /// Parameter name from the effect's table.
        param: String,
    },
}

/// One point of an automation curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    /// Ticks from the start of the clip.
    pub tick: u32,
    /// Control position 0..=1, mapped through the target's curve.
    pub value: f32,
}

/// A curve that moves one parameter over time when its clips play.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Automation {
    /// Stable identity.
    pub id: AutomationId,
    /// Display name.
    pub name: String,
    /// The parameter driven.
    pub target: AutomationTarget,
    /// Points sorted by tick; values are linearly interpolated between them.
    pub points: Vec<AutomationPoint>,
}

impl Automation {
    /// Length covered by the points, rounded up to whole bars (at least one).
    #[must_use]
    pub fn length_ticks(&self) -> u32 {
        let last = self.points.iter().map(|p| p.tick).max().unwrap_or(0);
        crate::music::round_up_to_bar(last.max(1))
    }

    /// Value at `tick`, interpolating between points and holding the ends.
    #[must_use]
    pub fn value_at(&self, tick: f64) -> Option<f32> {
        let first = self.points.first()?;
        if tick <= f64::from(first.tick) {
            return Some(first.value);
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if tick <= f64::from(b.tick) {
                let span = f64::from(b.tick.saturating_sub(a.tick)).max(1e-9);
                let t = ((tick - f64::from(a.tick)) / span) as f32;
                return Some(a.value + (b.value - a.value) * t);
            }
        }
        self.points.last().map(|p| p.value)
    }

    /// Keeps points sorted with one point per tick.
    pub fn sort_points(&mut self) {
        self.points.sort_by_key(|p| p.tick);
        self.points.dedup_by_key(|p| p.tick);
    }
}

impl Clip {
    /// End tick (exclusive).
    #[must_use]
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.length)
    }
}

/// A named position on the song timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    /// Position in ticks.
    pub tick: u32,
    /// Label.
    pub name: String,
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
    /// Song-mode loop region `(start, end)` in ticks, if set.
    #[serde(default)]
    pub loop_range: Option<(u32, u32)>,
    /// Markers sorted by tick.
    #[serde(default)]
    pub markers: Vec<Marker>,
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

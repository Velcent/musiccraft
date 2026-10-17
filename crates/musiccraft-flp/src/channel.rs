// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Channels and the channel rack (`pyflp.channel`).

use crate::events::{Event, Value};
use crate::ids::{channel as id, display_group, plugin as plugin_id, rack};
use crate::plugin::{Accepts, Plugin};
use crate::structs::{self, Automation, AutomationPoint, EnvelopeLfo, Parameters, Tracking};
use crate::tree::EventTree;
use crate::types::{MusicalTime, Rgba};
use crate::{ModelError, boolean, color, int, prop, text};

/// `ChannelType`: what a channel is.
pub mod channel_type {
    pub const SAMPLER: i64 = 0;
    pub const NATIVE: i64 = 2;
    pub const LAYER: i64 = 3;
    pub const INSTRUMENT: i64 = 4;
    pub const AUTOMATION: i64 = 5;
}

/// The model PyFLP uses for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// An unrecognised type (`Channel`).
    Channel,
    /// An automation clip.
    Automation,
    /// A layer of other channels.
    Layer,
    /// The sampler, or an audio clip.
    Sampler,
    /// A native or third-party plugin.
    Instrument,
}

impl ChannelKind {
    /// PyFLP's class name.
    #[must_use]
    pub fn class(self) -> &'static str {
        match self {
            Self::Channel => "Channel",
            Self::Automation => "Automation",
            Self::Layer => "Layer",
            Self::Sampler => "Sampler",
            Self::Instrument => "Instrument",
        }
    }
}

/// The channel rack.
#[derive(Debug, Clone)]
pub struct ChannelRack<'a> {
    events: EventTree<'a>,
}

impl<'a> ChannelRack<'a> {
    pub(crate) fn new(events: EventTree<'a>) -> Self {
        Self { events }
    }

    /// Display group (channel filter) names.
    #[must_use]
    pub fn groups(&self) -> Vec<Option<String>> {
        self.events
            .separate(display_group::NAME)
            .iter()
            .map(|t| text(t, &[display_group::NAME]))
            .collect()
    }

    /// The channels, in rack order.
    ///
    /// # Errors
    ///
    /// Where PyFLP raises: a channel without a type or group number, or a
    /// group number with no display group.
    pub fn channels(&self) -> Result<Vec<Channel<'a>>, ModelError> {
        let groups = self.groups();
        let ids = [id::ALL, plugin_id::ALL].concat();
        let mut out = Vec::new();
        for tree in self.events.divide(id::NEW, &ids) {
            let iid = int(&tree, &[id::NEW]).ok_or("a channel has no index")?;
            let kind_value = int(&tree, &[id::TYPE]).ok_or("a channel has no type")?;
            let group_num = int(&tree, &[id::GROUP_NUM]).ok_or("a channel has no group")?;
            let mut kind = match kind_value {
                channel_type::AUTOMATION => ChannelKind::Automation,
                channel_type::LAYER => ChannelKind::Layer,
                channel_type::SAMPLER => ChannelKind::Sampler,
                channel_type::INSTRUMENT | channel_type::NATIVE => ChannelKind::Instrument,
                _ => ChannelKind::Channel,
            };
            // Audio clips are stored as instruments until a sample is loaded.
            if tree.contains(id::SAMPLE_PATH)
                && tree.contains(plugin_id::INTERNAL_NAME)
                && text(&tree, &[plugin_id::INTERNAL_NAME]).is_some_and(|n| n.is_empty())
                && kind == ChannelKind::Instrument
            {
                kind = ChannelKind::Sampler;
            }
            // Python indexing: negative counts from the end.
            let len = i64::try_from(groups.len()).unwrap_or(i64::MAX);
            let index = if group_num < 0 {
                len + group_num
            } else {
                group_num
            };
            let group = usize::try_from(index)
                .ok()
                .and_then(|i| groups.get(i))
                .ok_or("a channel's group does not exist")?
                .clone();
            out.push(Channel {
                events: tree,
                kind,
                iid,
                group,
            });
        }
        Ok(out)
    }

    /// Global swing (0–128).
    #[must_use]
    pub fn swing(&self) -> Option<i64> {
        int(&self.events, &[rack::SWING])
    }

    /// The rack window's height in pixels.
    #[must_use]
    pub fn height(&self) -> Option<i64> {
        int(&self.events, &[rack::WINDOW_HEIGHT])
    }

    #[must_use]
    pub fn fit_to_steps(&self) -> Option<i64> {
        int(&self.events, &[rack::FIT_TO_STEPS])
    }
}

/// A channel in the rack.
#[derive(Debug, Clone)]
pub struct Channel<'a> {
    events: EventTree<'a>,
    pub kind: ChannelKind,
    /// Internal index, referenced by notes and playlist clips.
    pub iid: i64,
    /// Name of the display group it is filed under.
    pub group: Option<String>,
}

/// A channel's tracking settings.
pub type TrackingPair = (Option<Tracking>, Option<Tracking>);

impl<'a> Channel<'a> {
    /// A channel preset's events read as a `kind` channel.
    pub(crate) fn preset(events: EventTree<'a>, kind: ChannelKind) -> Self {
        let iid = int(&events, &[id::NEW]).unwrap_or(0);
        Self {
            events,
            kind,
            iid,
            group: None,
        }
    }

    fn struct_event(&self, event_id: u8) -> Option<&'a Event> {
        self.events.first(event_id)
    }

    fn parameters(&self) -> Option<&'a Parameters> {
        match &self.struct_event(id::PARAMETERS)?.value {
            Value::Parameters(p) => Some(p),
            _ => None,
        }
    }

    fn levels(&self) -> Option<structs::Levels> {
        match self.struct_event(id::LEVELS)?.value {
            Value::Levels(l) => Some(l),
            _ => None,
        }
    }

    /// The channel's colour.
    #[must_use]
    pub fn color(&self) -> Option<Rgba> {
        color(&self.events, &[plugin_id::COLOR])
    }

    /// The plugin's internal name: `Fruity Wrapper` for VSTs, empty for
    /// most native plugins.
    #[must_use]
    pub fn internal_name(&self) -> Option<String> {
        text(&self.events, &[plugin_id::INTERNAL_NAME])
    }

    /// The name given to the channel (or the VST's name).
    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[plugin_id::NAME, id::NAME])
    }

    /// The name FL Studio shows: the name, else the internal name.
    #[must_use]
    pub fn display_name(&self) -> Option<String> {
        self.name()
            .filter(|n| !n.is_empty())
            .or_else(|| self.internal_name())
    }

    #[must_use]
    pub fn enabled(&self) -> Option<bool> {
        boolean(&self.events, &[id::IS_ENABLED])
    }

    #[must_use]
    pub fn icon(&self) -> Option<i64> {
        int(&self.events, &[plugin_id::ICON])
    }

    #[must_use]
    pub fn locked(&self) -> Option<bool> {
        boolean(&self.events, &[id::IS_LOCKED])
    }

    /// Whether the channel is folded in the rack.
    #[must_use]
    pub fn zipped(&self) -> bool {
        boolean(&self.events, &[id::ZIPPED]).unwrap_or(false)
    }

    /// Pan, 0 (left) to 12800 (right), 6400 centred.
    #[must_use]
    pub fn pan(&self) -> Option<i64> {
        if self.events.contains(id::LEVELS) {
            return self.levels()?.pan.map(i64::from);
        }
        int(&self.events, &[id::PAN_WORD, id::PAN_BYTE])
    }

    /// Volume, 0 to 12800 (10000 by default), nonlinear.
    #[must_use]
    pub fn volume(&self) -> Option<i64> {
        if self.events.contains(id::LEVELS) {
            return self.levels()?.volume.map(i64::from);
        }
        int(&self.events, &[id::VOL_WORD, id::VOL_BYTE])
    }

    /// The plugin in the channel (instruments only).
    #[must_use]
    pub fn plugin(&self) -> Option<Plugin<'a>> {
        (self.kind == ChannelKind::Instrument)
            .then(|| Plugin::of(&self.events, Accepts::Channel))
            .flatten()
    }

    // --- samplers and instruments --------------------------------------------

    fn sampler_or_instrument(&self) -> bool {
        matches!(self.kind, ChannelKind::Sampler | ChannelKind::Instrument)
    }

    /// The mixer insert it plays through (-1 is "current").
    #[must_use]
    pub fn insert(&self) -> Option<i64> {
        self.sampler_or_instrument()
            .then(|| int(&self.events, &[id::ROUTED_TO]))
            .flatten()
    }

    /// Pitch shift in cents, −4800 to 4800.
    #[must_use]
    pub fn pitch_shift(&self) -> Option<i64> {
        if !self.sampler_or_instrument() {
            return None;
        }
        self.levels()?.pitch_shift.map(i64::from)
    }

    /// Cut group: (cuts itself, cut by).
    #[must_use]
    pub fn cut_group(&self) -> Option<(u16, u16)> {
        if !self.sampler_or_instrument() {
            return None;
        }
        match prop(&self.events, &[id::CUT_GROUP])?.value {
            Value::U16Pair(a, b) => Some((a, b)),
            _ => None,
        }
    }

    /// Fine tune in cents.
    #[must_use]
    pub fn fine_tune(&self) -> Option<i64> {
        int(&self.events, &[id::FINE_TUNE])
    }

    /// Root note (MIDI key, C5 = 60 by default).
    #[must_use]
    pub fn root_note(&self) -> i64 {
        int(&self.events, &[id::ROOT_NOTE]).unwrap_or(60)
    }

    /// Whether the master pitch affects it.
    #[must_use]
    pub fn keyboard_main_pitch(&self) -> Option<bool> {
        self.parameters()?.keyboard_main_pitch
    }

    #[must_use]
    pub fn keyboard_add_root(&self) -> Option<bool> {
        self.parameters()?.keyboard_add_root
    }

    /// Playable key range.
    #[must_use]
    pub fn keyboard_key_region(&self) -> Option<(u32, u32)> {
        self.parameters()?.keyboard_key_region
    }

    /// Polyphony: (max voices, slide, flags). PyFLP reads these only when
    /// the channel has exactly one polyphony event.
    #[must_use]
    pub fn polyphony(&self) -> Option<structs::Polyphony> {
        if self.events.count(id::POLYPHONY) != 1 {
            return None;
        }
        match self.struct_event(id::POLYPHONY)?.value {
            Value::Polyphony(p) => Some(p),
            _ => None,
        }
    }

    /// Swing mix (0–128), time gate, time shift and full portamento.
    #[must_use]
    pub fn time_swing(&self) -> Option<i64> {
        int(&self.events, &[id::SWING])
    }

    #[must_use]
    pub fn time_gate(&self) -> Option<u16> {
        self.parameters()?.time_gate
    }

    #[must_use]
    pub fn time_shift(&self) -> Option<i64> {
        int(&self.events, &[id::TIME_SHIFT])
    }

    #[must_use]
    pub fn time_full_porta(&self) -> Option<bool> {
        self.parameters()?.time_full_porta
    }

    /// The arpeggiator. PyFLP's properties raise here (they look up the
    /// wrong field names); these return the fields they mean.
    #[must_use]
    pub fn arp(&self) -> Option<&'a Parameters> {
        self.parameters()
    }

    /// The echo delay.
    #[must_use]
    pub fn delay(&self) -> Option<structs::Delay> {
        match self.struct_event(id::DELAY)?.value {
            Value::Delay(d) => Some(d),
            _ => None,
        }
    }

    /// Delay modulation X and Y.
    #[must_use]
    pub fn delay_mod(&self) -> Option<(u16, u16)> {
        match prop(&self.events, &[id::DELAY_MOD_XY])?.value {
            Value::U16Pair(a, b) => Some((a, b)),
            _ => None,
        }
    }

    /// `_DelayFlags` from the parameters.
    #[must_use]
    pub fn delay_flags(&self) -> Option<u8> {
        self.parameters()?.delay_flags
    }

    /// Level adjustments (PyFLP reads them only when there is exactly one
    /// event).
    #[must_use]
    pub fn level_adjusts(&self) -> Option<structs::LevelAdjusts> {
        if self.events.count(id::LEVEL_ADJUSTS) != 1 {
            return None;
        }
        match self.struct_event(id::LEVEL_ADJUSTS)?.value {
            Value::LevelAdjusts(l) => Some(l),
            _ => None,
        }
    }

    /// Volume and keyboard tracking.
    #[must_use]
    pub fn tracking(&self) -> Option<TrackingPair> {
        let list: Vec<Tracking> = self
            .events
            .get(&[id::TRACKING])
            .filter_map(|e| match e.value {
                Value::Tracking(t) => Some(t),
                _ => None,
            })
            .collect();
        (!list.is_empty()).then(|| (list.first().copied(), list.get(1).copied()))
    }

    // --- samplers ---------------------------------------------------------------

    fn is_sampler(&self) -> bool {
        self.kind == ChannelKind::Sampler
    }

    /// The sample file (`%FLStudioFactoryData%` marks stock samples).
    #[must_use]
    pub fn sample_path(&self) -> Option<String> {
        self.is_sampler()
            .then(|| text(&self.events, &[id::SAMPLE_PATH]))
            .flatten()
    }

    #[must_use]
    pub fn au_sample_rate(&self) -> Option<i64> {
        self.is_sampler()
            .then(|| int(&self.events, &[id::AU_SAMPLE_RATE]))
            .flatten()
    }

    /// The sampler's parameters (precomputed effects, playback, stretching).
    #[must_use]
    pub fn sampler_parameters(&self) -> Option<&'a Parameters> {
        self.is_sampler().then(|| self.parameters()).flatten()
    }

    /// Filter cutoff, resonance and type, from the levels.
    #[must_use]
    pub fn filter(&self) -> Option<structs::Levels> {
        self.is_sampler().then(|| self.levels()).flatten()
    }

    /// Envelopes and LFOs for panning, volume, mod X, mod Y and pitch.
    #[must_use]
    pub fn envelopes(&self) -> Option<Vec<EnvelopeLfo>> {
        if !self.is_sampler() || !self.events.contains(id::ENVELOPE_LFO) {
            return None;
        }
        Some(
            self.events
                .get(&[id::ENVELOPE_LFO])
                .filter_map(|e| match e.value {
                    Value::EnvelopeLfo(env) => Some(env),
                    _ => None,
                })
                .take(5)
                .collect(),
        )
    }

    /// A plain integer event of a sampler.
    #[must_use]
    pub fn sampler_int(&self, event_id: u8) -> Option<i64> {
        self.is_sampler()
            .then(|| int(&self.events, &[event_id]))
            .flatten()
    }

    /// Ring modulation (mix, frequency).
    #[must_use]
    pub fn ringmod(&self) -> Option<(u16, u16)> {
        if !self.is_sampler() {
            return None;
        }
        match prop(&self.events, &[id::RING_MOD])?.value {
            Value::U16Pair(a, b) => Some((a, b)),
            _ => None,
        }
    }

    /// Reverb: (type 0 for A or 65536 for B, mix).
    #[must_use]
    pub fn reverb(&self) -> Option<(i64, i64)> {
        let value = self.sampler_int(id::REVERB)?;
        let kind = if value >= 65536 { 65536 } else { 0 };
        Some((kind, value - kind))
    }

    /// Whether the ping-pong loop is on.
    #[must_use]
    pub fn ping_pong_loop(&self) -> Option<bool> {
        self.is_sampler()
            .then(|| boolean(&self.events, &[id::PING_PONG_LOOP]))
            .flatten()
    }

    /// Time-stretch length.
    #[must_use]
    pub fn stretching_time(&self) -> Option<MusicalTime> {
        self.sampler_parameters()?.stretching_time
    }

    // --- layers -------------------------------------------------------------------

    /// The `iid`s of a layer's children.
    #[must_use]
    pub fn children(&self) -> Option<Vec<i64>> {
        (self.kind == ChannelKind::Layer).then(|| {
            self.events
                .get(&[id::CHILDREN])
                .filter_map(Event::int)
                .collect()
        })
    }

    /// `_LayerFlags` (bit 0 random, bit 1 crossfade).
    #[must_use]
    pub fn layer_flags(&self) -> Option<i64> {
        (self.kind == ChannelKind::Layer)
            .then(|| int(&self.events, &[id::LAYER_FLAGS]))
            .flatten()
    }

    // --- automation clips -------------------------------------------------------

    fn automation(&self) -> Option<&'a Automation> {
        if self.kind != ChannelKind::Automation {
            return None;
        }
        match &self.struct_event(id::AUTOMATION)?.value {
            Value::Automation(a) => Some(a),
            _ => None,
        }
    }

    /// An automation clip's points.
    #[must_use]
    pub fn points(&self) -> Option<&'a [AutomationPoint]> {
        self.automation().map(|a| a.points.as_slice())
    }

    /// An automation clip's LFO amount.
    #[must_use]
    pub fn automation_lfo_amount(&self) -> Option<i32> {
        self.automation().map(|a| a.lfo_amount)
    }

    /// A raw integer event, whatever the channel kind.
    #[must_use]
    pub fn raw_int(&self, event_id: u8) -> Option<i64> {
        int(&self.events, &[event_id])
    }
}

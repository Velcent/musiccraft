// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! The mixer, its inserts and effect slots (`pyflp.mixer`).

use crate::events::Value;
use crate::ids::{insert as id, mixer, plugin as plugin_id, slot};
use crate::plugin::{Accepts, Plugin};
use crate::structs::MixerParam;
use crate::tree::EventTree;
use crate::types::{FlVersion, Rgba};
use crate::{color, int, text};

/// `_InsertFlags`.
pub mod insert_flags {
    pub const POLARITY_REVERSED: i64 = 1 << 0;
    pub const SWAP_LEFT_RIGHT: i64 = 1 << 1;
    pub const ENABLE_EFFECTS: i64 = 1 << 2;
    pub const ENABLED: i64 = 1 << 3;
    pub const DOCK_MIDDLE: i64 = 1 << 6;
    pub const DOCK_RIGHT: i64 = 1 << 7;
    pub const SEPARATOR_SHOWN: i64 = 1 << 10;
    pub const LOCKED: i64 = 1 << 11;
    pub const SOLO: i64 = 1 << 12;
    pub const AUDIO_TRACK: i64 = 1 << 15;
}

/// `_MixerParamsID`: what a mixer parameter record sets.
/// `InsertDock`; the discriminants are PyFLP's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertDock {
    Left = 1,
    Middle = 2,
    Right = 3,
}

pub mod param {
    pub const SLOT_ENABLED: u8 = 0;
    pub const SLOT_MIX: u8 = 1;
    /// 64–191 are send levels.
    pub const ROUTE_VOL_START: u8 = 64;
    pub const VOLUME: u8 = 192;
    pub const PAN: u8 = 193;
    pub const STEREO_SEPARATION: u8 = 194;
    pub const LOW_GAIN: u8 = 208;
    pub const MID_GAIN: u8 = 209;
    pub const HIGH_GAIN: u8 = 210;
    pub const LOW_FREQ: u8 = 216;
    pub const MID_FREQ: u8 = 217;
    pub const HIGH_FREQ: u8 = 218;
    pub const LOW_Q: u8 = 224;
    pub const MID_Q: u8 = 225;
    pub const HIGH_Q: u8 = 226;
}

/// One insert's records from the mixer parameter list: its own, in the
/// order each id first appears (later records replace earlier values), and
/// each slot's.
#[derive(Debug, Clone, Default)]
pub struct InsertParams {
    pub own: Vec<(u8, MixerParam)>,
    pub slots: Vec<(usize, Vec<(u8, MixerParam)>)>,
}

fn upsert(list: &mut Vec<(u8, MixerParam)>, item: MixerParam) {
    match list.iter_mut().find(|(id, _)| *id == item.id) {
        Some(entry) => entry.1 = item,
        None => list.push((item.id, item)),
    }
}

/// Splits the parameter list by insert (PyFLP's `MixerParamsEvent.items_`).
fn split_params(items: &[MixerParam]) -> Vec<(usize, InsertParams)> {
    let mut out: Vec<(usize, InsertParams)> = Vec::new();
    for item in items {
        let index = item.insert();
        let position = match out.iter().position(|(i, _)| *i == index) {
            Some(p) => p,
            None => {
                out.push((index, InsertParams::default()));
                out.len() - 1
            }
        };
        let Some((_, insert)) = out.get_mut(position) else {
            continue;
        };
        if item.id == param::SLOT_ENABLED || item.id == param::SLOT_MIX {
            let slot = item.slot();
            match insert.slots.iter_mut().find(|(s, _)| *s == slot) {
                Some((_, list)) => upsert(list, *item),
                None => insert.slots.push((slot, vec![(item.id, *item)])),
            }
        } else {
            upsert(&mut insert.own, *item);
        }
    }
    out
}

/// Inserts each FL Studio version allows (master and "current" included),
/// by the last version with that limit.
const MAX_INSERTS: [([u32; 3], usize); 7] = [
    ([1, 6, 5], 5),
    ([2, 0, 1], 8),
    ([3, 0, 0], 18),
    ([3, 3, 0], 20),
    ([4, 0, 0], 64),
    ([9, 0, 0], 105),
    ([12, 9, 0], 127),
];

const MAX_SLOTS: [([u32; 3], usize); 2] = [([1, 6, 5], 4), ([3, 0, 0], 8)];

fn by_version(table: &[([u32; 3], usize)], version: Option<FlVersion>, latest: usize) -> usize {
    let Some(version) = version else {
        return latest;
    };
    table
        .iter()
        .find(|(limit, _)| version.cmp_tuple(limit) != std::cmp::Ordering::Greater)
        .map_or(latest, |(_, n)| *n)
}

/// The mixer.
#[derive(Debug, Clone)]
pub struct Mixer<'a> {
    events: EventTree<'a>,
    version: Option<FlVersion>,
}

impl<'a> Mixer<'a> {
    pub(crate) fn new(events: EventTree<'a>, version: Option<FlVersion>) -> Self {
        Self { events, version }
    }

    /// Whether plugin delay compensation is on.
    #[must_use]
    pub fn apdc(&self) -> Option<i64> {
        int(&self.events, &[mixer::APDC])
    }

    /// Estimated inserts for the file's version (master and "current" too).
    #[must_use]
    pub fn max_inserts(&self) -> usize {
        by_version(&MAX_INSERTS, self.version, 127)
    }

    /// Estimated effect slots per insert.
    #[must_use]
    pub fn max_slots(&self) -> usize {
        by_version(&MAX_SLOTS, self.version, 10)
    }

    /// The inserts: "current" (iid -1) first, then master (0), 1, 2…
    #[must_use]
    pub fn inserts(&self) -> Vec<Insert<'a>> {
        let ids = [id::ALL, plugin_id::ALL, slot::ALL].concat();
        let params = match self.events.first(mixer::PARAMS).map(|e| &e.value) {
            Some(Value::MixerParams(items)) => split_params(items),
            _ => Vec::new(),
        };
        let trees = self.events.subtrees(
            |e| {
                if e.id == id::OUTPUT {
                    Some(false)
                } else if ids.contains(&e.id) {
                    Some(true)
                } else {
                    None
                }
            },
            i64::try_from(self.max_inserts()).unwrap_or(i64::MAX),
        );
        trees
            .into_iter()
            .enumerate()
            .map(|(i, events)| Insert {
                events,
                iid: i64::try_from(i).unwrap_or(i64::MAX) - 1,
                params: params.iter().find(|(n, _)| *n == i).map(|(_, p)| p.clone()),
            })
            .collect()
    }
}

/// A mixer insert (track).
#[derive(Debug, Clone)]
pub struct Insert<'a> {
    events: EventTree<'a>,
    /// -1 for "current", 0 for master, then 1, 2…
    pub iid: i64,
    /// This insert's records from the mixer parameter list.
    pub params: Option<InsertParams>,
}

impl<'a> Insert<'a> {
    /// An insert preset: every event, with the first insert's parameter
    /// records (presets have no `Output` event to end the insert).
    pub(crate) fn preset(events: EventTree<'a>) -> Self {
        let params = match events.first(mixer::PARAMS).map(|e| &e.value) {
            Some(Value::MixerParams(items)) => split_params(items).into_iter().next(),
            _ => None,
        };
        Self {
            events,
            iid: 0,
            params: params.map(|(_, p)| p),
        }
    }

    fn own(&self, id: u8) -> Option<i64> {
        let params = self.params.as_ref()?;
        params
            .own
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| i64::from(p.msg))
    }

    fn flags(&self) -> Option<i64> {
        match self.events.first(id::FLAGS)?.value {
            Value::InsertFlags(f) => f.flags.map(i64::from),
            _ => None,
        }
    }

    fn flag(&self, bit: i64) -> Option<bool> {
        crate::flag(self.flags(), bit)
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[id::NAME])
    }

    /// Post-fader volume: 0 to 16000, 12800 is 0 dB.
    #[must_use]
    pub fn volume(&self) -> Option<i64> {
        self.own(param::VOLUME)
    }

    /// Pan: −6400 (left) to 6400 (right).
    #[must_use]
    pub fn pan(&self) -> Option<i64> {
        self.own(param::PAN)
    }

    /// Stereo separation: 64 merged to −64 separated.
    #[must_use]
    pub fn stereo_separation(&self) -> Option<i64> {
        self.own(param::STEREO_SEPARATION)
    }

    /// An EQ band's (gain, freq, reso); `None` where the record is missing.
    #[must_use]
    pub fn eq_band(&self, band: usize) -> [Option<i64>; 3] {
        let (gain, freq, reso) = match band {
            0 => (param::LOW_GAIN, param::LOW_FREQ, param::LOW_Q),
            1 => (param::MID_GAIN, param::MID_FREQ, param::MID_Q),
            _ => (param::HIGH_GAIN, param::HIGH_FREQ, param::HIGH_Q),
        };
        [self.own(gain), self.own(freq), self.own(reso)]
    }

    #[must_use]
    pub fn enabled(&self) -> Option<bool> {
        self.flag(insert_flags::ENABLED)
    }

    /// Whether every slot is bypassed.
    #[must_use]
    pub fn bypassed(&self) -> Option<bool> {
        self.flag(insert_flags::ENABLE_EFFECTS).map(|on| !on)
    }

    #[must_use]
    pub fn channels_swapped(&self) -> Option<bool> {
        self.flag(insert_flags::SWAP_LEFT_RIGHT)
    }

    #[must_use]
    pub fn polarity_reversed(&self) -> Option<bool> {
        self.flag(insert_flags::POLARITY_REVERSED)
    }

    #[must_use]
    pub fn is_solo(&self) -> Option<bool> {
        self.flag(insert_flags::SOLO)
    }

    #[must_use]
    pub fn locked(&self) -> Option<bool> {
        self.flag(insert_flags::LOCKED)
    }

    #[must_use]
    pub fn separator_shown(&self) -> Option<bool> {
        self.flag(insert_flags::SEPARATOR_SHOWN)
    }

    /// Where the insert is docked in the mixer.
    #[must_use]
    pub fn dock(&self) -> Option<InsertDock> {
        self.events.first(id::FLAGS)?;
        let flags = self.flags().unwrap_or(0);
        Some(if flags & insert_flags::DOCK_MIDDLE != 0 {
            InsertDock::Middle
        } else if flags & insert_flags::DOCK_RIGHT != 0 {
            InsertDock::Right
        } else {
            InsertDock::Left
        })
    }

    #[must_use]
    pub fn color(&self) -> Option<Rgba> {
        color(&self.events, &[id::COLOR])
    }

    #[must_use]
    pub fn icon(&self) -> Option<i64> {
        int(&self.events, &[id::ICON])
    }

    #[must_use]
    pub fn input(&self) -> Option<i64> {
        int(&self.events, &[id::INPUT])
    }

    #[must_use]
    pub fn output(&self) -> Option<i64> {
        int(&self.events, &[id::OUTPUT])
    }

    /// Send levels, as PyFLP pairs them: every own record with an id of 64
    /// or more, in order, takes the next routing flag, and its value is
    /// listed when that flag is set.
    #[must_use]
    pub fn routes(&self) -> Option<Vec<i64>> {
        let routing = match &self.events.first(id::ROUTING)?.value {
            Value::InsertRouting(flags) => flags,
            _ => return None,
        };
        let params = self.params.as_ref()?;
        let mut flags = routing.iter();
        let mut out = Vec::new();
        for (record, item) in &params.own {
            if *record >= param::ROUTE_VOL_START {
                if let Some(true) = flags.next() {
                    out.push(i64::from(item.msg));
                }
            }
        }
        Some(out)
    }

    /// The effect slots (only when the insert has parameter records, as in
    /// PyFLP).
    #[must_use]
    pub fn slots(&self) -> Option<Vec<Slot<'a>>> {
        let params = self.params.as_ref()?;
        let ids = [slot::ALL, plugin_id::ALL].concat();
        Some(
            self.events
                .divide(slot::INDEX, &ids)
                .into_iter()
                .enumerate()
                .map(|(index, events)| Slot {
                    events,
                    params: params
                        .slots
                        .iter()
                        .find(|(s, _)| *s == index)
                        .map(|(_, p)| p.clone())
                        .unwrap_or_default(),
                })
                .collect(),
        )
    }
}

/// An effect slot.
#[derive(Debug, Clone)]
pub struct Slot<'a> {
    events: EventTree<'a>,
    params: Vec<(u8, MixerParam)>,
}

impl<'a> Slot<'a> {
    /// 0-based index.
    #[must_use]
    pub fn index(&self) -> Option<i64> {
        int(&self.events, &[slot::INDEX])
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[plugin_id::NAME])
    }

    /// `Fruity Wrapper` for VSTs, else the native plugin's name.
    #[must_use]
    pub fn internal_name(&self) -> Option<String> {
        text(&self.events, &[plugin_id::INTERNAL_NAME])
    }

    #[must_use]
    pub fn color(&self) -> Option<Rgba> {
        color(&self.events, &[plugin_id::COLOR])
    }

    #[must_use]
    pub fn icon(&self) -> Option<i64> {
        int(&self.events, &[plugin_id::ICON])
    }

    fn param(&self, id: u8) -> Option<i64> {
        self.params
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| i64::from(p.msg))
    }

    /// Whether the slot is on. (PyFLP raises here: it reads the slot's
    /// records as if they were an insert's.)
    #[must_use]
    pub fn enabled(&self) -> Option<bool> {
        self.param(param::SLOT_ENABLED).map(|v| v != 0)
    }

    /// Dry/wet mix: −6400 to 6400. (PyFLP raises here, as for `enabled`.)
    #[must_use]
    pub fn mix(&self) -> Option<i64> {
        self.param(param::SLOT_MIX)
    }

    /// The effect in the slot.
    #[must_use]
    pub fn plugin(&self) -> Option<Plugin<'a>> {
        Plugin::of(&self.events, Accepts::Slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_limits_compare_like_python() {
        let v = |major, minor, patch, build| FlVersion {
            major,
            minor,
            patch,
            build,
        };
        assert_eq!(
            by_version(&MAX_INSERTS, Some(v(20, 8, 4, Some(1))), 127),
            127
        );
        assert_eq!(by_version(&MAX_INSERTS, Some(v(9, 0, 0, None)), 127), 105);
        assert_eq!(
            by_version(&MAX_INSERTS, Some(v(9, 0, 0, Some(5))), 127),
            127,
            "9.0.0.5 is past (9, 0, 0)"
        );
    }

    #[test]
    fn parameter_records_split_by_insert_and_slot() {
        let rec = |id, insert: u16, slot: u16, msg| MixerParam {
            id,
            channel_data: (insert << 6) | slot,
            msg,
        };
        let split = split_params(&[
            rec(param::VOLUME, 1, 0, 100),
            rec(param::SLOT_MIX, 1, 2, 7),
            rec(param::PAN, 1, 0, 5),
            rec(param::VOLUME, 1, 0, 200),
        ]);
        assert_eq!(split.len(), 1);
        let (_, insert) = &split[0];
        let own: Vec<(u8, i32)> = insert.own.iter().map(|(i, p)| (*i, p.msg)).collect();
        assert_eq!(
            own,
            [(param::VOLUME, 200), (param::PAN, 5)],
            "first position kept"
        );
        assert_eq!(insert.slots[0].0, 2);
    }
}

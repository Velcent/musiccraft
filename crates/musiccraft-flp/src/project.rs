// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! The whole project (`pyflp.project.Project`).

use crate::arrangement::{ARRANGEMENT_IDS, Arrangements};
use crate::channel::{Channel, ChannelKind, ChannelRack};
use crate::events::{self, Event, FlpError, Header, Value};
use crate::ids::{self, arrangement, project};
use crate::mixer::{Insert, Mixer};
use crate::pattern::{Pattern, Patterns};
use crate::plugin::{Accepts, Plugin};
use crate::tree::EventTree;
use crate::types::FlVersion;

/// The project's panning law (`PanLaw`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanLaw {
    Circular,
    Triangular,
    /// A value PyFLP does not name.
    Other(u8),
}

/// A parsed FL Studio file.
#[derive(Debug, Clone)]
pub struct Project {
    header: Header,
    events: Vec<Event>,
}

impl Project {
    /// Parses a file's bytes (PyFLP's `parse`).
    ///
    /// # Errors
    ///
    /// Returns why the file cannot be read, where PyFLP raises.
    pub fn parse(file: &[u8]) -> Result<Self, FlpError> {
        let (header, events) = events::parse(file)?;
        Ok(Self { header, events })
    }

    /// Every event, in file order.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    fn tree(&self) -> EventTree<'_> {
        EventTree::root(&self.events)
    }

    /// The file format (see [`crate::events::format`]).
    #[must_use]
    pub fn format(&self) -> i16 {
        self.header.format
    }

    /// Pulses (ticks) per quarter note; every position and length uses it.
    #[must_use]
    pub fn ppq(&self) -> u16 {
        self.header.ppq
    }

    /// Number of channels in the rack, from the header.
    #[must_use]
    pub fn channel_count(&self) -> u16 {
        self.header.channel_count
    }

    /// The FL Studio version that saved the file.
    #[must_use]
    pub fn version(&self) -> Option<FlVersion> {
        let text = crate::text(&self.tree(), &[project::FL_VERSION])?;
        let parts: Vec<u32> = text
            .split('.')
            .map(|p| p.parse().ok())
            .collect::<Option<_>>()?;
        Some(FlVersion {
            major: *parts.first()?,
            minor: parts.get(1).copied().unwrap_or(0),
            patch: parts.get(2).copied().unwrap_or(0),
            build: parts.get(3).copied(),
        })
    }

    /// Tempo in BPM (stored × 1000; very old files store a coarse and a fine
    /// part).
    #[must_use]
    pub fn tempo(&self) -> Option<f64> {
        let tree = self.tree();
        if let Some(tempo) = crate::int(&tree, &[project::TEMPO]) {
            return Some(tempo as f64 / 1000.0);
        }
        let coarse = crate::int(&tree, &[project::TEMPO_COARSE])?;
        let fine = crate::int(&tree, &[project::TEMPO_FINE]).unwrap_or(0);
        Some(coarse as f64 + fine as f64 / 1000.0)
    }

    #[must_use]
    pub fn title(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::TITLE])
    }

    #[must_use]
    pub fn artists(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::ARTISTS])
    }

    #[must_use]
    pub fn genre(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::GENRE])
    }

    /// Comments (very old files store RTF, kept as it is).
    #[must_use]
    pub fn comments(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::COMMENTS, project::RTF_COMMENTS])
    }

    #[must_use]
    pub fn url(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::URL])
    }

    /// The folder FL Studio renders into.
    #[must_use]
    pub fn data_path(&self) -> Option<String> {
        crate::text(&self.tree(), &[project::DATA_PATH])
    }

    #[must_use]
    pub fn pan_law(&self) -> Option<PanLaw> {
        let value = crate::int(&self.tree(), &[project::PAN_LAW])?;
        Some(match value {
            0 => PanLaw::Circular,
            2 => PanLaw::Triangular,
            other => PanLaw::Other(u8::try_from(other).unwrap_or(u8::MAX)),
        })
    }

    /// The raw pan law value.
    #[must_use]
    pub fn pan_law_value(&self) -> Option<i64> {
        crate::int(&self.tree(), &[project::PAN_LAW])
    }

    /// Master pitch in cents, −1200 to 1200.
    #[must_use]
    pub fn main_pitch(&self) -> Option<i64> {
        crate::int(&self.tree(), &[project::PITCH])
    }

    /// Master volume (old files).
    #[must_use]
    pub fn main_volume(&self) -> Option<i64> {
        crate::int(&self.tree(), &[project::VOLUME])
    }

    /// Whether a part of the playlist is selected as a loop.
    #[must_use]
    pub fn looped(&self) -> Option<bool> {
        crate::boolean(&self.tree(), &[project::LOOP_ACTIVE])
    }

    #[must_use]
    pub fn show_info(&self) -> Option<bool> {
        crate::boolean(&self.tree(), &[project::SHOW_INFO])
    }

    #[must_use]
    pub fn licensed(&self) -> Option<bool> {
        crate::boolean(&self.tree(), &[project::LICENSED])
    }

    /// Who the saving copy of FL Studio was licensed to (stored jumbled).
    #[must_use]
    pub fn licensee(&self) -> Option<String> {
        let jumbled = crate::text(&self.tree(), &[project::LICENSEE])?;
        let mut out = String::new();
        for (index, ch) in jumbled.chars().enumerate() {
            let code = i64::from(u32::from(ch));
            let index = i64::try_from(index).ok()?;
            let picked = [code - 26 + index, code + 49 + index]
                .into_iter()
                .find_map(|n| {
                    char::from_u32(u32::try_from(n).ok()?).filter(|c| c.is_alphanumeric())
                });
            if let Some(c) = picked {
                // PyFLP decodes the result as ASCII and raises otherwise.
                if !c.is_ascii() {
                    return None;
                }
                out.push(c);
            }
        }
        Some(out)
    }

    /// Days since 30 Dec 1899 at which the project was created, and days
    /// spent on it.
    #[must_use]
    pub fn timestamp(&self) -> Option<(f64, f64)> {
        match &self.tree().first(project::TIMESTAMP)?.value {
            Value::Timestamp(t) => Some((t.created_on, t.time_spent)),
            _ => None,
        }
    }

    /// A channel preset (`.fst`) read as a `kind` channel.
    #[must_use]
    pub fn preset_channel(&self, kind: ChannelKind) -> Channel<'_> {
        Channel::preset(self.tree(), kind)
    }

    /// A mixer insert preset (`.fst`).
    #[must_use]
    pub fn preset_insert(&self) -> Insert<'_> {
        Insert::preset(self.tree())
    }

    /// A score (`.fsc`): one pattern's notes.
    #[must_use]
    pub fn preset_pattern(&self) -> Pattern<'_> {
        Pattern::new(self.tree().subtree(|e| ids::pattern::ALL.contains(&e.id)))
    }

    /// A plugin preset (`.fst`).
    #[must_use]
    pub fn preset_plugin(&self) -> Option<Plugin<'_>> {
        Plugin::of(&self.tree(), Accepts::Any)
    }

    /// The channel rack.
    #[must_use]
    pub fn channels(&self) -> ChannelRack<'_> {
        let ids = channel_rack_ids();
        let tree = self.tree().subtree(|e| ids.contains(&e.id));
        ChannelRack::new(tree)
    }

    /// The patterns.
    #[must_use]
    pub fn patterns(&self) -> Patterns<'_> {
        let mut arrangement_started = false;
        let tree = self.tree().subtree(|e| {
            if e.id == arrangement::NEW {
                arrangement_started = true;
                false
            } else if ids::time_marker::ALL.contains(&e.id) {
                // Time markers after the first arrangement belong to it.
                !arrangement_started
            } else {
                ids::pattern::ALL.contains(&e.id) || ids::patterns::ALL.contains(&e.id)
            }
        });
        Patterns::new(tree)
    }

    /// The arrangements (playlists).
    #[must_use]
    pub fn arrangements(&self) -> Arrangements<'_> {
        let mut arrangement_started = false;
        let tree = self.tree().subtree(|e| {
            if e.id == arrangement::NEW {
                arrangement_started = true;
            }
            (ids::time_marker::ALL.contains(&e.id) && arrangement_started)
                || ARRANGEMENT_IDS.iter().any(|group| group.contains(&e.id))
        });
        Arrangements::new(tree, self.version(), self.channels(), self.patterns())
    }

    /// The mixer.
    #[must_use]
    pub fn mixer(&self) -> Mixer<'_> {
        let mut inserts_began = false;
        let tree = self.tree().subtree(|e| {
            if ids::mixer::ALL.contains(&e.id)
                || ids::insert::ALL.contains(&e.id)
                || ids::slot::ALL.contains(&e.id)
            {
                inserts_began = true;
                return true;
            }
            inserts_began && ids::plugin::ALL.contains(&e.id)
        });
        Mixer::new(tree, self.version())
    }
}

/// Ids the channel rack collects: channel, display group, plugin and rack
/// events.
fn channel_rack_ids() -> Vec<u8> {
    [
        ids::channel::ALL,
        ids::display_group::ALL,
        ids::plugin::ALL,
        ids::rack::ALL,
    ]
    .concat()
}

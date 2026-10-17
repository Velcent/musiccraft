// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Arrangements (playlists), their tracks and clips (`pyflp.arrangement`).

use crate::channel::{Channel, ChannelRack};
use crate::events::Value;
use crate::ids::{arrangement as id, arrangements, track};
use crate::pattern::{Pattern, Patterns, TimeMarker};
use crate::structs::{PlaylistItem, TrackData};
use crate::tree::EventTree;
use crate::types::{FlVersion, Rgba};
use crate::{ModelError, int, text};

/// Ids the arrangements collect (besides their time markers).
pub(crate) const ARRANGEMENT_IDS: [&[u8]; 3] = [id::ALL, arrangements::ALL, track::ALL];

fn since_multiple_arrangements(version: Option<FlVersion>) -> bool {
    version.is_some_and(|v| v >= FlVersion::new(12, 9, 1))
}

/// The project's arrangements and playlist settings.
#[derive(Debug, Clone)]
pub struct Arrangements<'a> {
    events: EventTree<'a>,
    version: Option<FlVersion>,
    channels: ChannelRack<'a>,
    patterns: Patterns<'a>,
}

impl<'a> Arrangements<'a> {
    pub(crate) fn new(
        events: EventTree<'a>,
        version: Option<FlVersion>,
        channels: ChannelRack<'a>,
        patterns: Patterns<'a>,
    ) -> Self {
        Self {
            events,
            version,
            channels,
            patterns,
        }
    }

    /// The arrangements.
    ///
    /// # Errors
    ///
    /// When there are none (PyFLP's `NoModelsFound`).
    pub fn arrangements(&self) -> Result<Vec<Arrangement<'a>>, ModelError> {
        let count = self.events.count(id::NEW);
        if count == 0 {
            return Err("the project has no arrangements");
        }
        let mut started = false;
        let trees = self.events.subtrees(
            |e| {
                if e.id == id::NEW {
                    if started {
                        return Some(false);
                    }
                    started = true;
                }
                if id::ALL.contains(&e.id)
                    || crate::ids::time_marker::ALL.contains(&e.id)
                    || track::ALL.contains(&e.id)
                {
                    return Some(true);
                }
                if e.id == arrangements::CURRENT {
                    // Ends the last arrangement.
                    return Some(false);
                }
                None
            },
            i64::try_from(count).unwrap_or(i64::MAX),
        );
        Ok(trees
            .into_iter()
            .map(|events| Arrangement {
                events,
                version: self.version,
                channels: self.channels.clone(),
                patterns: self.patterns.clone(),
            })
            .collect())
    }

    /// The selected arrangement's position in [`Self::arrangements`].
    #[must_use]
    pub fn current_index(&self) -> Option<i64> {
        int(&self.events, &[arrangements::CURRENT])
    }

    /// The playlist's loop or selection: (start, end) in ticks.
    #[must_use]
    pub fn loop_pos(&self) -> Option<(Option<u32>, Option<u32>)> {
        if let Some(event) = self.events.first(arrangements::PL_SELECTION) {
            if let Value::PlSelection(s) = event.value {
                return Some((s.start, s.end));
            }
        }
        match self.events.first(arrangements::LOOP_POS)?.value {
            Value::U16Pair(a, b) => Some((Some(u32::from(a)), Some(u32::from(b)))),
            _ => None,
        }
    }

    /// Tracks per arrangement: 500 since FL Studio 12.9.1, 199 before.
    #[must_use]
    pub fn max_tracks(&self) -> u32 {
        if since_multiple_arrangements(self.version) {
            500
        } else {
            199
        }
    }

    /// Project time signature: (numerator, beat).
    #[must_use]
    pub fn time_signature(&self) -> (Option<i64>, Option<i64>) {
        (
            int(&self.events, &[arrangements::TIME_SIG_NUM]),
            int(&self.events, &[arrangements::TIME_SIG_BEAT]),
        )
    }
}

/// An arrangement.
#[derive(Debug, Clone)]
pub struct Arrangement<'a> {
    events: EventTree<'a>,
    version: Option<FlVersion>,
    channels: ChannelRack<'a>,
    patterns: Patterns<'a>,
}

/// What a playlist clip plays.
#[derive(Debug, Clone)]
pub enum ClipRef<'a> {
    /// A pattern.
    Pattern(Pattern<'a>),
    /// A channel: an audio clip or automation clip.
    Channel(Channel<'a>),
}

/// A clip on a playlist track.
#[derive(Debug, Clone)]
pub struct Clip<'a> {
    pub item: &'a PlaylistItem,
    pub target: ClipRef<'a>,
}

impl<'a> Arrangement<'a> {
    /// A 1-based internal index.
    #[must_use]
    pub fn iid(&self) -> Option<i64> {
        int(&self.events, &[id::NEW])
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[id::NAME])
    }

    #[must_use]
    pub fn timemarkers(&self) -> Vec<TimeMarker<'a>> {
        TimeMarker::all(&self.events)
    }

    /// The tracks with their clips.
    ///
    /// # Errors
    ///
    /// Where PyFLP raises: a clip naming a channel or pattern that does not
    /// exist, or a channel list that cannot be read.
    pub fn tracks(&self) -> Result<Vec<Track<'a>>, ModelError> {
        let max_index: i64 = if since_multiple_arrangements(self.version) {
            499
        } else {
            198
        };
        let channels = self.channels.channels()?;
        let patterns = self.patterns.patterns();
        let playlist: &'a [PlaylistItem] = match self.events.first(id::PLAYLIST).map(|e| &e.value) {
            Some(Value::Playlist(items)) => items,
            _ => &[],
        };
        let has_playlist = self.events.contains(id::PLAYLIST);
        let mut out = Vec::new();
        for (index, events) in self
            .events
            .divide(track::DATA, track::ALL)
            .into_iter()
            .enumerate()
        {
            let mut clips = Vec::new();
            if has_playlist {
                let index = i64::try_from(index).unwrap_or(i64::MAX);
                for item in playlist {
                    if max_index - i64::from(item.track_rvidx) != index {
                        continue;
                    }
                    let target = if item.item_index <= item.pattern_base {
                        let iid = i64::from(item.item_index);
                        ClipRef::Channel(
                            channels
                                .iter()
                                .find(|c| c.iid == iid)
                                .ok_or("a playlist clip names a channel that does not exist")?
                                .clone(),
                        )
                    } else {
                        let num = i64::from(item.item_index - item.pattern_base);
                        ClipRef::Pattern(crate::pattern::find(&patterns, num)?)
                    };
                    clips.push(Clip { item, target });
                }
            }
            out.push(Track { events, clips });
        }
        Ok(out)
    }
}

/// A playlist track.
#[derive(Debug, Clone)]
pub struct Track<'a> {
    events: EventTree<'a>,
    pub clips: Vec<Clip<'a>>,
}

impl Track<'_> {
    /// The track's settings.
    #[must_use]
    pub fn data(&self) -> Option<&TrackData> {
        match &self.events.first(track::DATA)?.value {
            Value::Track(data) => Some(data),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[track::NAME])
    }

    /// The track colour (stored as little-endian RGBA bytes).
    #[must_use]
    pub fn color(&self) -> Option<Rgba> {
        let raw = self.data()?.color?;
        Some(crate::structs::rgba(raw.to_le_bytes()))
    }

    /// Height as PyFLP shows it: a whole percentage.
    #[must_use]
    pub fn height_percent(&self) -> Option<String> {
        let height = self.data()?.height?;
        // Python's `int()` truncates toward zero.
        Some(format!("{}%", (f64::from(height) * 100.0).trunc() as i64))
    }
}

// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Patterns, their notes and controller events, and time markers
//! (`pyflp.pattern`, `pyflp.timemarker`).

use crate::events::Value;
use crate::ids::{pattern as id, patterns, time_marker};
use crate::structs::{ControllerItem, NoteItem};
use crate::tree::EventTree;
use crate::types::Rgba;
use crate::{ModelError, boolean, color, int, text};

/// Time marker positions at or above this are time signature changes.
pub const SIGNATURE_MARKER: i64 = 134_217_728;

/// A marker in a pattern's or an arrangement's timeline.
#[derive(Debug, Clone)]
pub struct TimeMarker<'a> {
    events: EventTree<'a>,
}

impl<'a> TimeMarker<'a> {
    pub(crate) fn all(events: &EventTree<'a>) -> Vec<Self> {
        events
            .group(time_marker::ALL)
            .into_iter()
            .map(|events| Self { events })
            .collect()
    }

    fn raw_position(&self) -> Option<i64> {
        int(&self.events, &[time_marker::POSITION])
    }

    /// Position in ticks.
    #[must_use]
    pub fn position(&self) -> Option<i64> {
        self.raw_position().map(|p| {
            if p < SIGNATURE_MARKER {
                p
            } else {
                p - SIGNATURE_MARKER
            }
        })
    }

    /// Whether this marks a time signature (else it is a text marker).
    #[must_use]
    pub fn is_signature(&self) -> Option<bool> {
        self.raw_position().map(|p| p >= SIGNATURE_MARKER)
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[time_marker::NAME])
    }

    #[must_use]
    pub fn numerator(&self) -> Option<i64> {
        int(&self.events, &[time_marker::NUMERATOR])
    }

    #[must_use]
    pub fn denominator(&self) -> Option<i64> {
        int(&self.events, &[time_marker::DENOMINATOR])
    }
}

/// The project's patterns.
#[derive(Debug, Clone)]
pub struct Patterns<'a> {
    events: EventTree<'a>,
}

impl<'a> Patterns<'a> {
    pub(crate) fn new(events: EventTree<'a>) -> Self {
        Self { events }
    }

    /// The patterns, in the order they first appear. A pattern's events can
    /// come in two runs (notes first, then the rest), both started by its
    /// `New` event.
    #[must_use]
    pub fn patterns(&self) -> Vec<Pattern<'a>> {
        let ids = [id::ALL, time_marker::ALL].concat();
        self.events
            .by_key(|e| (e.id == id::NEW).then(|| e.int()).flatten(), &ids)
            .into_iter()
            .map(|(_, events)| Pattern { events })
            .collect()
    }

    /// Whether truncated notes play in playlist clips.
    #[must_use]
    pub fn play_cut_notes(&self) -> Option<bool> {
        boolean(&self.events, &[patterns::PLAY_TRUNCATED_NOTES])
    }

    /// The selected pattern's `iid`.
    #[must_use]
    pub fn current(&self) -> Option<i64> {
        let wanted = int(&self.events, &[patterns::CURRENTLY_SELECTED])?;
        self.patterns()
            .iter()
            .find(|p| p.iid() == Some(wanted))
            .and_then(Pattern::iid)
    }
}

/// A pattern.
#[derive(Debug, Clone)]
pub struct Pattern<'a> {
    events: EventTree<'a>,
}

impl<'a> Pattern<'a> {
    pub(crate) fn new(events: EventTree<'a>) -> Self {
        Self { events }
    }

    /// Internal index, from 1.
    #[must_use]
    pub fn iid(&self) -> Option<i64> {
        int(&self.events, &[id::NEW])
    }

    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.events, &[id::NAME])
    }

    /// Length in ticks; `None` in automatic length mode.
    #[must_use]
    pub fn length(&self) -> Option<i64> {
        int(&self.events, &[id::LENGTH])
    }

    /// Whether it is in live loop mode.
    #[must_use]
    pub fn looped(&self) -> bool {
        boolean(&self.events, &[id::LOOPED]).unwrap_or(false)
    }

    #[must_use]
    pub fn color(&self) -> Option<Rgba> {
        color(&self.events, &[id::COLOR])
    }

    /// The notes.
    #[must_use]
    pub fn notes(&self) -> &'a [NoteItem] {
        match self.events.first(id::NOTES).map(|e| &e.value) {
            Some(Value::Notes(notes)) => notes,
            _ => &[],
        }
    }

    /// Controller (parameter automation) events.
    #[must_use]
    pub fn controllers(&self) -> &'a [ControllerItem] {
        match self.events.first(id::CONTROLLERS).map(|e| &e.value) {
            Some(Value::Controllers(items)) => items,
            _ => &[],
        }
    }

    #[must_use]
    pub fn timemarkers(&self) -> Vec<TimeMarker<'a>> {
        TimeMarker::all(&self.events)
    }
}

/// `Note.key` as a name, `C5` style (sharps only).
#[must_use]
pub fn key_name(key: u16) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[usize::from(key % 12)], key / 12)
}

/// Lets callers treat a missing pattern as an error where PyFLP raises.
pub(crate) fn find<'a>(list: &[Pattern<'a>], iid: i64) -> Result<Pattern<'a>, ModelError> {
    list.iter()
        .find(|p| p.iid() == Some(iid))
        .cloned()
        .ok_or("a playlist clip names a pattern that does not exist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_put_middle_c_at_c5() {
        assert_eq!(key_name(60), "C5");
        assert_eq!(key_name(61), "C#5");
        assert_eq!(key_name(0), "C0");
    }
}

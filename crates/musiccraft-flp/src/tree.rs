// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! `EventTree`: an ordered view of some of a file's events, which the models
//! read their properties from. PyFLP's trees are also mutable views that
//! propagate edits back to the file; this port only reads.

use crate::events::Event;

/// A view of events, each with its index in the whole file.
#[derive(Debug, Clone, Default)]
pub struct EventTree<'a> {
    events: Vec<(usize, &'a Event)>,
}

impl<'a> EventTree<'a> {
    /// A tree over every event of a file.
    #[must_use]
    pub fn root(events: &'a [Event]) -> Self {
        Self {
            events: events.iter().enumerate().collect(),
        }
    }

    fn of(events: Vec<(usize, &'a Event)>) -> Self {
        Self { events }
    }

    /// The events in order.
    pub fn iter(&self) -> impl Iterator<Item = &'a Event> + '_ {
        self.events.iter().map(|(_, e)| *e)
    }

    /// Number of events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether there are no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Whether an event with `id` is in the tree.
    #[must_use]
    pub fn contains(&self, id: u8) -> bool {
        self.iter().any(|e| e.id == id)
    }

    /// The first event with `id`.
    #[must_use]
    pub fn first(&self, id: u8) -> Option<&'a Event> {
        self.iter().find(|e| e.id == id)
    }

    /// Events whose id is one of `ids`.
    pub fn get<'b>(&'b self, ids: &'b [u8]) -> impl Iterator<Item = &'a Event> + 'b {
        self.iter().filter(move |e| ids.contains(&e.id))
    }

    /// Number of events with `id`.
    #[must_use]
    pub fn count(&self, id: u8) -> usize {
        self.iter().filter(|e| e.id == id).count()
    }

    /// The events with an id in `ids`, split before each `separator` (the
    /// events before the first separator join the first part).
    #[must_use]
    pub fn divide(&self, separator: u8, ids: &[u8]) -> Vec<Self> {
        let mut out = Vec::new();
        let mut part = Vec::new();
        let mut first = true;
        for &(index, event) in &self.events {
            if event.id == separator {
                if first {
                    first = false;
                } else {
                    out.push(Self::of(std::mem::take(&mut part)));
                }
            }
            if ids.contains(&event.id) {
                part.push((index, event));
            }
        }
        out.push(Self::of(part));
        out
    }

    /// Groups the n-th events of each id in `ids` together (`zip_longest`).
    #[must_use]
    pub fn group(&self, ids: &[u8]) -> Vec<Self> {
        let lists: Vec<Vec<(usize, &'a Event)>> = ids
            .iter()
            .map(|id| {
                self.events
                    .iter()
                    .filter(|(_, e)| e.id == *id)
                    .copied()
                    .collect()
            })
            .collect();
        let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
        (0..longest)
            .map(|n| {
                let mut part: Vec<(usize, &'a Event)> =
                    lists.iter().filter_map(|l| l.get(n).copied()).collect();
                // PyFLP's trees keep events sorted by file position.
                part.sort_by_key(|(index, _)| *index);
                Self::of(part)
            })
            .collect()
    }

    /// One tree per event with `id`.
    #[must_use]
    pub fn separate(&self, id: u8) -> Vec<Self> {
        self.events
            .iter()
            .filter(|(_, e)| e.id == id)
            .map(|&ie| Self::of(vec![ie]))
            .collect()
    }

    /// The events for which `select` is true.
    #[must_use]
    pub fn subtree(&self, mut select: impl FnMut(&Event) -> bool) -> Self {
        Self::of(
            self.events
                .iter()
                .filter(|(_, e)| select(e))
                .copied()
                .collect(),
        )
    }

    /// Up to `repeat` trees (all of them if negative): `select` returns
    /// `Some(true)` to include an event, `None` to skip it, and `Some(false)`
    /// to end the current tree (that event starts the next one). Events after
    /// the last `Some(false)` are not returned.
    #[must_use]
    pub fn subtrees(
        &self,
        mut select: impl FnMut(&Event) -> Option<bool>,
        repeat: i64,
    ) -> Vec<Self> {
        let mut out = Vec::new();
        let mut part = Vec::new();
        let mut repeat = repeat;
        for &(index, event) in &self.events {
            if repeat == 0 {
                break;
            }
            match select(event) {
                Some(false) => {
                    out.push(Self::of(std::mem::replace(&mut part, vec![(index, event)])));
                    repeat -= 1;
                }
                Some(true) => part.push((index, event)),
                None => {}
            }
        }
        out
    }

    /// Splits into consecutive groups started by events with `id` (as PyFLP
    /// collects patterns): events before the first such event form a group
    /// with key `start`.
    #[must_use]
    pub fn by_key(&self, key: impl Fn(&Event) -> Option<i64>, ids: &[u8]) -> Vec<(i64, Self)> {
        let mut out: Vec<(i64, Vec<(usize, &'a Event)>)> = Vec::new();
        let mut current = 0;
        for &(index, event) in &self.events {
            if let Some(k) = key(event) {
                current = k;
            }
            if ids.contains(&event.id) {
                match out.iter_mut().find(|(k, _)| *k == current) {
                    Some((_, list)) => list.push((index, event)),
                    None => out.push((current, vec![(index, event)])),
                }
            }
        }
        out.into_iter()
            .map(|(k, list)| (k, Self::of(list)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Value;

    fn ev(id: u8) -> Event {
        Event {
            id,
            value: Value::U8(0),
        }
    }

    fn ids(tree: &EventTree<'_>) -> Vec<u8> {
        tree.iter().map(|e| e.id).collect()
    }

    #[test]
    fn divide_and_group() {
        let events: Vec<Event> = [5, 1, 2, 1, 3, 9, 1].into_iter().map(ev).collect();
        let root = EventTree::root(&events);
        let parts = root.divide(1, &[1, 2, 3, 5]);
        let parts: Vec<Vec<u8>> = parts.iter().map(ids).collect();
        assert_eq!(parts, [vec![5, 1, 2], vec![1, 3], vec![1]]);
        let groups: Vec<Vec<u8>> = root.group(&[1, 3]).iter().map(ids).collect();
        assert_eq!(groups, [vec![1, 3], vec![1], vec![1]]);
    }

    #[test]
    fn subtrees_end_at_false_and_drop_the_tail() {
        let events: Vec<Event> = [7, 1, 2, 7, 3, 4].into_iter().map(ev).collect();
        let root = EventTree::root(&events);
        let trees = root.subtrees(
            |e| match e.id {
                7 => Some(false),
                4 => None,
                _ => Some(true),
            },
            -1,
        );
        let trees: Vec<Vec<u8>> = trees.iter().map(ids).collect();
        assert_eq!(
            trees,
            [vec![], vec![7, 1, 2]],
            "the part after the last 7 is dropped"
        );
    }
}

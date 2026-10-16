// musiccraft-flp: a Rust port of PyFLP, the FL Studio project file parser.
//
// PyFLP is Copyright (C) 2022 demberto (https://github.com/demberto/PyFLP).
// This port is part of MusicCraft. Both are free software: you can
// redistribute them and/or modify them under the terms of the GNU General
// Public License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version. They are
// distributed in the hope that they will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License (`LICENSE`)
// for more details.

//! Reads FL Studio project files (`.flp`) and the state files that share
//! their format (`.fst`, `.fsc`).
//!
//! This is a port of PyFLP's reading side: the same event decoding, the same
//! models and properties, under the same names where Rust allows. PyFLP can
//! also edit and save files; this port only reads. Everything known about
//! the format comes from PyFLP, whose authors worked it out; this crate is
//! checked against PyFLP's output, not against FL Studio itself (see
//! `docs/clean-room.md`).
//!
//! Where PyFLP raises while reading a property, the port returns `None`;
//! where PyFLP has a plain bug (noted on the property), the port returns the
//! value the property is meant to have.
//!
//! ```no_run
//! let project = musiccraft_flp::Project::parse(&std::fs::read("song.flp")?)?;
//! println!("{} BPM", project.tempo().unwrap_or(140.0));
//! for channel in project.channels().channels()? {
//!     println!("{}", channel.display_name().unwrap_or_default());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod arrangement;
pub mod channel;
pub mod events;
pub mod ids;
pub mod mixer;
pub mod pattern;
pub mod plugin;
pub mod plugin_data;
mod project;
mod reader;
pub mod structs;
pub mod tree;
pub mod types;

pub use events::{Event, FlpError, Value};
pub use project::{PanLaw, Project};
pub use types::{FlVersion, MusicalTime, Rgba};

/// Why a model could not be read (PyFLP raises in these cases).
pub type ModelError = &'static str;

use tree::EventTree;

/// The first event with one of `ids` (PyFLP's `EventProp` lookup).
pub(crate) fn prop<'a>(tree: &EventTree<'a>, ids: &[u8]) -> Option<&'a Event> {
    ids.iter().find_map(|id| tree.first(*id))
}

/// An integer property.
pub(crate) fn int(tree: &EventTree<'_>, ids: &[u8]) -> Option<i64> {
    prop(tree, ids).and_then(Event::int)
}

/// A boolean property.
pub(crate) fn boolean(tree: &EventTree<'_>, ids: &[u8]) -> Option<bool> {
    prop(tree, ids).and_then(Event::bool)
}

/// A text property.
pub(crate) fn text(tree: &EventTree<'_>, ids: &[u8]) -> Option<String> {
    prop(tree, ids).and_then(Event::text).map(str::to_owned)
}

/// A colour property.
pub(crate) fn color(tree: &EventTree<'_>, ids: &[u8]) -> Option<Rgba> {
    match prop(tree, ids)?.value {
        Value::Color(c) => Some(c),
        _ => None,
    }
}

/// Whether `flag` is set in an integer (`FlagProp` on a plain event).
pub(crate) fn flag(value: Option<i64>, flag: i64) -> Option<bool> {
    value.map(|v| v & flag != 0)
}

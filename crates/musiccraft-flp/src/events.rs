// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! The event stream: the file header, the events and their decoded values
//! (PyFLP's `parse` and `_events`).
//!
//! A file is a header (`FLhd`, format, channel count, PPQ) and a data chunk
//! (`FLdt`) of events. Each event is an id byte followed by its value: one
//! byte for ids below 64, two below 128, four below 192, and otherwise a
//! variable-length size and that many bytes. What the value means depends on
//! the id (see [`crate::ids`]).

use std::fmt;

use crate::ids;
use crate::plugin_data::{Native, VST_INTERNAL_NAME, VstPlugin};
use crate::reader::Reader;
use crate::structs::{
    self, Automation, ControllerItem, Delay, EnvelopeLfo, InsertFlags, LevelAdjusts, Levels,
    MixerParam, NoteItem, Parameters, PlSelection, PlaylistItem, Polyphony, Timestamp, TrackData,
    Tracking, Wrapper,
};
use crate::types::Rgba;

/// First id of two-byte events.
pub const WORD: u8 = 64;
/// First id of four-byte events.
pub const DWORD: u8 = 128;
/// First id of variable-size (text) events.
pub const TEXT: u8 = 192;
/// First id of variable-size (data) events.
pub const DATA: u8 = 208;
/// Data-range ids that hold text (arrangement, display group and track
/// names).
pub const NEW_TEXT_IDS: [u8; 3] = [TEXT + 49, TEXT + 39, TEXT + 47];

/// PPQs FL Studio supports.
pub const VALID_PPQS: [u16; 11] = [24, 48, 72, 96, 120, 144, 168, 192, 384, 768, 960];

/// File formats (`FileFormat`).
pub mod format {
    /// Temporary file.
    pub const NONE: i16 = -1;
    /// A project (`.flp`).
    pub const PROJECT: i16 = 0;
    /// A score (`.fsc`): pattern notes and controller events.
    pub const SCORE: i16 = 0x10;
    pub const AUTOMATION: i16 = 24;
    /// A whole channel (`.fst`).
    pub const CHANNEL_STATE: i16 = 0x20;
    pub const PLUGIN_STATE: i16 = 0x30;
    pub const GENERATOR_STATE: i16 = 0x31;
    pub const FX_STATE: i16 = 0x32;
    /// An insert and its slots (`.fst`).
    pub const INSERT_STATE: i16 = 0x40;
    pub const PROBABLY_PATCHER: i16 = 0x50;
    /// Every format PyFLP accepts.
    pub const ALL: [i16; 10] = [
        NONE,
        PROJECT,
        SCORE,
        AUTOMATION,
        CHANNEL_STATE,
        PLUGIN_STATE,
        GENERATOR_STATE,
        FX_STATE,
        INSERT_STATE,
        PROBABLY_PATCHER,
    ];
}

/// Why a file could not be read (PyFLP's exceptions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlpError {
    /// The header is wrong (`HeaderCorrupted`).
    HeaderCorrupted(&'static str),
    /// A text event came before the version event (`VersionNotDetected`).
    VersionNotDetected,
    /// The version text is not `major.minor.patch[.build]` numbers.
    BadVersion(String),
    /// A fixed-size event was cut short (`InvalidEventChunkSize`).
    InvalidEventChunkSize { id: u8, expected: usize, got: usize },
    /// An event's value could not be decoded.
    Event { id: u8, reason: &'static str },
}

impl fmt::Display for FlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderCorrupted(why) => write!(f, "not a valid FL Studio file: {why}"),
            Self::VersionNotDetected => write!(f, "text before the FL Studio version"),
            Self::BadVersion(text) => write!(f, "unreadable FL Studio version `{text}`"),
            Self::InvalidEventChunkSize { id, expected, got } => write!(
                f,
                "event {id} has {got} bytes where {expected} were expected"
            ),
            Self::Event { id, reason } => write!(f, "event {id}: {reason}"),
        }
    }
}

impl std::error::Error for FlpError {}

/// A structured event's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructKind {
    Automation,
    Controller,
    Delay,
    EnvelopeLFO,
    InsertFlags,
    InsertRouting,
    LevelAdjusts,
    Levels,
    MixerParams,
    Notes,
    PLSelection,
    Parameters,
    Playlist,
    Polyphony,
    Timestamp,
    Track,
    Tracking,
    Wrapper,
}

/// The value type PyFLP declares for an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    U16Pair,
    Color,
    Ascii,
    Struct(StructKind),
}

/// A decoded event value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    F32(f32),
    U16Pair(u16, u16),
    Color(Rgba),
    Text(String),
    Timestamp(Timestamp),
    Delay(Delay),
    EnvelopeLfo(EnvelopeLfo),
    LevelAdjusts(LevelAdjusts),
    Levels(Levels),
    Parameters(Box<Parameters>),
    Polyphony(Polyphony),
    Tracking(Tracking),
    Automation(Automation),
    Controllers(Vec<ControllerItem>),
    Notes(Vec<NoteItem>),
    PlSelection(PlSelection),
    Playlist(Vec<PlaylistItem>),
    Track(Box<TrackData>),
    InsertFlags(InsertFlags),
    InsertRouting(Vec<bool>),
    MixerParams(Vec<MixerParam>),
    Wrapper(Wrapper),
    Vst(VstPlugin),
    Native(Native),
    /// Data PyFLP does not decode.
    Unknown(Vec<u8>),
}

/// One event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: u8,
    pub value: Value,
}

impl Event {
    /// The value as an integer, for the simple numeric types.
    #[must_use]
    pub fn int(&self) -> Option<i64> {
        Some(match self.value {
            Value::Bool(v) => i64::from(v),
            Value::U8(v) => i64::from(v),
            Value::I8(v) => i64::from(v),
            Value::U16(v) => i64::from(v),
            Value::I16(v) => i64::from(v),
            Value::U32(v) => i64::from(v),
            Value::I32(v) => i64::from(v),
            _ => return None,
        })
    }

    /// The value as a boolean.
    #[must_use]
    pub fn bool(&self) -> Option<bool> {
        match self.value {
            Value::Bool(v) => Some(v),
            _ => self.int().map(|v| v != 0),
        }
    }

    /// The value as text.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match &self.value {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// The header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub format: i16,
    pub channel_count: u16,
    pub ppq: u16,
}

/// How text is stored: ASCII before FL Studio 11.5, UTF-16 since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrType {
    Ascii,
    Unicode,
}

fn declared(id: u8) -> Option<Option<Declared>> {
    ids::DECLARED
        .iter()
        .find(|(declared_id, _)| *declared_id == id)
        .map(|(_, kind)| *kind)
}

fn decode_text(data: &[u8], kind: StrType, id: u8) -> Result<String, FlpError> {
    let text = match kind {
        StrType::Ascii => {
            if !data.is_ascii() {
                return Err(FlpError::Event {
                    id,
                    reason: "text is not ASCII",
                });
            }
            String::from_utf8_lossy(data).into_owned()
        }
        StrType::Unicode => {
            if data.len() % 2 != 0 {
                return Err(FlpError::Event {
                    id,
                    reason: "UTF-16 text has an odd length",
                });
            }
            let units: Vec<u16> = data
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&units).map_err(|_| FlpError::Event {
                id,
                reason: "invalid UTF-16 text",
            })?
        }
    };
    Ok(text.trim_end_matches('\0').to_owned())
}

fn decode_struct(kind: StructKind, data: &[u8], id: u8) -> Result<Value, FlpError> {
    let fail = |reason| FlpError::Event { id, reason };
    Ok(match kind {
        StructKind::Automation => Value::Automation(Automation::read(data).map_err(fail)?),
        StructKind::Controller => Value::Controllers(structs::controllers(data)),
        StructKind::Delay => Value::Delay(Delay::read(data)),
        StructKind::EnvelopeLFO => Value::EnvelopeLfo(EnvelopeLfo::read(data)),
        StructKind::InsertFlags => Value::InsertFlags(InsertFlags::read(data)),
        StructKind::InsertRouting => Value::InsertRouting(structs::insert_routing(data)),
        StructKind::LevelAdjusts => Value::LevelAdjusts(LevelAdjusts::read(data)),
        StructKind::Levels => Value::Levels(Levels::read(data)),
        StructKind::MixerParams => Value::MixerParams(structs::mixer_params(data)),
        StructKind::Notes => Value::Notes(structs::notes(data)),
        StructKind::PLSelection => Value::PlSelection(PlSelection::read(data)),
        StructKind::Parameters => {
            Value::Parameters(Box::new(Parameters::read(data).map_err(fail)?))
        }
        StructKind::Playlist => Value::Playlist(structs::playlist(data)),
        StructKind::Polyphony => Value::Polyphony(Polyphony::read(data)),
        StructKind::Timestamp => Value::Timestamp(Timestamp::read(data).map_err(fail)?),
        StructKind::Track => Value::Track(Box::new(TrackData::read(data))),
        StructKind::Tracking => Value::Tracking(Tracking::read(data)),
        StructKind::Wrapper => Value::Wrapper(Wrapper::read(data)),
    })
}

fn decode_declared(kind: Declared, data: &[u8], id: u8) -> Result<Value, FlpError> {
    let mut r = Reader::new(data);
    let short = || FlpError::Event {
        id,
        reason: "value too short",
    };
    Ok(match kind {
        Declared::Bool => Value::Bool(r.flag().ok_or_else(short)?),
        Declared::U8 => Value::U8(r.u8().ok_or_else(short)?),
        Declared::I8 => Value::I8(r.i8().ok_or_else(short)?),
        Declared::U16 => Value::U16(r.u16().ok_or_else(short)?),
        Declared::I16 => Value::I16(r.i16().ok_or_else(short)?),
        Declared::U32 => Value::U32(r.u32().ok_or_else(short)?),
        Declared::I32 => Value::I32(r.i32().ok_or_else(short)?),
        Declared::F32 => Value::F32(r.f32().ok_or_else(short)?),
        Declared::U16Pair => Value::U16Pair(r.u16().ok_or_else(short)?, r.u16().ok_or_else(short)?),
        Declared::Color => {
            let bytes: [u8; 4] = r
                .bytes(4)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(short)?;
            Value::Color(structs::rgba(bytes))
        }
        Declared::Ascii => Value::Text(decode_text(data, StrType::Ascii, id)?),
        Declared::Struct(kind) => decode_struct(kind, data, id)?,
    })
}

/// Parses a whole file: its header and every event.
///
/// # Errors
///
/// Returns why the file is not a readable FL Studio file, in the cases where
/// PyFLP raises.
pub fn parse(file: &[u8]) -> Result<(Header, Vec<Event>), FlpError> {
    let mut r = Reader::new(file);
    let short = FlpError::HeaderCorrupted("couldn't read the header entirely");
    let magic = r.bytes(4).ok_or(short.clone())?;
    let header_size = r.u32().ok_or(short.clone())?;
    let format = r.i16().ok_or(short.clone())?;
    let channel_count = r.u16().ok_or(short.clone())?;
    let ppq = r.u16().ok_or(short)?;
    if magic != b"FLhd" {
        return Err(FlpError::HeaderCorrupted(
            "unexpected header chunk magic; expected 'FLhd'",
        ));
    }
    if header_size != 6 {
        return Err(FlpError::HeaderCorrupted(
            "unexpected header chunk size; expected 6",
        ));
    }
    if !format::ALL.contains(&format) {
        return Err(FlpError::HeaderCorrupted("unsupported project file format"));
    }
    if !VALID_PPQS.contains(&ppq) {
        return Err(FlpError::HeaderCorrupted("invalid PPQ"));
    }
    if r.bytes(4) != Some(b"FLdt") {
        return Err(FlpError::HeaderCorrupted(
            "unexpected data chunk magic; expected 'FLdt'",
        ));
    }
    let events_size = r.u32().unwrap_or(0);
    if events_size == 0 {
        return Err(FlpError::HeaderCorrupted(
            "data chunk size couldn't be read",
        ));
    }
    if file.len() as u64 != u64::from(events_size) + 22 {
        return Err(FlpError::HeaderCorrupted("data chunk size corrupted"));
    }

    let mut events = Vec::new();
    let mut str_type: Option<StrType> = None;
    let mut plugin_name: Option<String> = None;
    while r.remaining() > 0 {
        let Some(id) = r.u8() else { break };
        let data: &[u8] = if id < TEXT {
            let expected = if id < WORD {
                1
            } else if id < DWORD {
                2
            } else {
                4
            };
            let got = r.remaining().min(expected);
            let data = r.bytes(got).unwrap_or_default();
            if got != expected {
                return Err(FlpError::InvalidEventChunkSize { id, expected, got });
            }
            data
        } else {
            let size = r.varint().ok_or(FlpError::Event {
                id,
                reason: "size cut short",
            })?;
            let take = usize::try_from(size)
                .unwrap_or(usize::MAX)
                .min(r.remaining());
            r.bytes(take).unwrap_or_default()
        };

        if id == ids::project::FL_VERSION {
            let text = decode_text(data, StrType::Ascii, id)?;
            let parts: Result<Vec<u32>, _> = text.split('.').map(str::parse).collect();
            let parts = parts.map_err(|_| FlpError::BadVersion(text.clone()))?;
            str_type = Some(if parts.get(..2) >= Some(&[11, 5][..]) {
                StrType::Unicode
            } else {
                StrType::Ascii
            });
        }

        let value = match declared(id) {
            Some(Some(kind)) => decode_declared(kind, data, id)?,
            _ => {
                if id < WORD {
                    Value::U8(data.first().copied().unwrap_or(0))
                } else if id < DWORD {
                    decode_declared(Declared::U16, data, id)?
                } else if id < TEXT {
                    decode_declared(Declared::U32, data, id)?
                } else if id < DATA || NEW_TEXT_IDS.contains(&id) {
                    let kind = str_type.ok_or(FlpError::VersionNotDetected)?;
                    let text = decode_text(data, kind, id)?;
                    if id == ids::plugin::INTERNAL_NAME {
                        plugin_name = Some(text.clone());
                    }
                    Value::Text(text)
                } else if id == ids::plugin::DATA && plugin_name.is_some() {
                    plugin_value(plugin_name.as_deref().unwrap_or_default(), data, id)?
                } else {
                    Value::Unknown(data.to_vec())
                }
            }
        };
        events.push(Event { id, value });
    }
    Ok((
        Header {
            format,
            channel_count,
            ppq,
        },
        events,
    ))
}

/// A plugin data event, decoded by the plugin named before it.
fn plugin_value(name: &str, data: &[u8], id: u8) -> Result<Value, FlpError> {
    let fail = |reason| FlpError::Event { id, reason };
    if name == VST_INTERNAL_NAME {
        return Ok(Value::Vst(VstPlugin::read(data).map_err(fail)?));
    }
    match Native::read(name, data) {
        Some(native) => Ok(Value::Native(native.map_err(fail)?)),
        None => Ok(Value::Unknown(data.to_vec())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file with the given events after a valid header.
    fn file(events: &[u8]) -> Vec<u8> {
        let mut out = b"FLhd".to_vec();
        out.extend_from_slice(&6u32.to_le_bytes());
        out.extend_from_slice(&0i16.to_le_bytes());
        out.extend_from_slice(&3u16.to_le_bytes());
        out.extend_from_slice(&96u16.to_le_bytes());
        out.extend_from_slice(b"FLdt");
        out.extend_from_slice(&(events.len() as u32).to_le_bytes());
        out.extend_from_slice(events);
        out
    }

    fn text_event(id: u8, text: &[u8]) -> Vec<u8> {
        let mut out = vec![id, text.len() as u8];
        out.extend_from_slice(text);
        out
    }

    #[test]
    fn header_checks_match_pyflp() {
        assert!(matches!(parse(b"FLhd"), Err(FlpError::HeaderCorrupted(_))));
        let mut wrong_ppq = file(&[9, 1]);
        wrong_ppq[12] = 97;
        assert!(matches!(
            parse(&wrong_ppq),
            Err(FlpError::HeaderCorrupted("invalid PPQ"))
        ));
        let mut too_long = file(&[9, 1]);
        too_long.push(0);
        assert!(matches!(
            parse(&too_long),
            Err(FlpError::HeaderCorrupted("data chunk size corrupted"))
        ));
    }

    #[test]
    fn text_encoding_follows_the_version() {
        let mut events = text_event(ids::project::FL_VERSION, b"20.8.4.2576\0");
        events.extend(text_event(ids::project::TITLE, &[b'H', 0, b'i', 0, 0, 0]));
        let (header, parsed) = parse(&file(&events)).unwrap();
        assert_eq!(header.ppq, 96);
        assert_eq!(parsed[1].text(), Some("Hi"), "UTF-16 since 11.5");

        let mut old = text_event(ids::project::FL_VERSION, b"9.0.0\0");
        old.extend(text_event(ids::project::TITLE, b"Hi\0"));
        assert_eq!(parse(&file(&old)).unwrap().1[1].text(), Some("Hi"));

        let early = text_event(ids::project::TITLE, b"Hi\0");
        assert_eq!(parse(&file(&early)), Err(FlpError::VersionNotDetected));
    }

    #[test]
    fn fixed_sizes_and_undeclared_ids() {
        // 9 is a declared bool; 60 is undeclared (a byte); 100 a declared
        // u16; 250 undeclared data.
        let events = [9, 1, 60, 7, 100, 3, 0, 250, 2, 0xAB, 0xCD];
        let (_, parsed) = parse(&file(&events)).unwrap();
        assert_eq!(parsed[0].value, Value::Bool(true));
        assert_eq!(parsed[1].value, Value::U8(7));
        assert_eq!(parsed[2].value, Value::U16(3));
        assert_eq!(parsed[3].value, Value::Unknown(vec![0xAB, 0xCD]));
        let cut = file(&[100, 3]);
        assert!(matches!(
            parse(&cut),
            Err(FlpError::InvalidEventChunkSize { id: 100, .. })
        ));
    }
}

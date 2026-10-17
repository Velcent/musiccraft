// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! The structured events: fixed-layout records (`StructEventBase` in PyFLP)
//! whose fields are mostly optional, and lists of records (`ListEventBase`).
//! Field names, types and order follow PyFLP's `construct` definitions; the
//! byte offset each field ends at is noted where PyFLP notes it.

use crate::reader::Reader;
use crate::types::{MusicalTime, Rgba};

/// Why a structured event could not be read.
pub(crate) type Failed = &'static str;

/// `Optional(field)`: the field, or `None` if it does not fit.
fn opt<'a, T>(r: &mut Reader<'a>, read: impl FnOnce(&mut Reader<'a>) -> Option<T>) -> Option<T> {
    r.attempt(read)
}

// --- project ------------------------------------------------------------------

/// `TimestampEvent`: days since 1899-12-30 when created, and days worked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp {
    pub created_on: f64,
    pub time_spent: f64,
}

impl Timestamp {
    pub(crate) fn read(data: &[u8]) -> Result<Self, Failed> {
        let mut r = Reader::new(data);
        Ok(Self {
            created_on: r.f64().ok_or("timestamp too short")?,
            time_spent: r.f64().ok_or("timestamp too short")?,
        })
    }
}

// --- channel --------------------------------------------------------------------

/// `DelayEvent`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Delay {
    pub feedback: Option<u32>,
    pub pan: Option<i32>,
    pub pitch_shift: Option<i32>,
    pub echoes: Option<u32>,
    pub time: Option<u32>,
}

impl Delay {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            feedback: opt(&mut r, Reader::u32),
            pan: opt(&mut r, Reader::i32),
            pitch_shift: opt(&mut r, Reader::i32),
            echoes: opt(&mut r, Reader::u32),
            time: opt(&mut r, Reader::u32),
        }
    }
}

/// `EnvelopeLFOEvent` (FL Studio 2.5.0+).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvelopeLfo {
    pub flags: Option<i32>,
    pub envelope_enabled: Option<i32>,
    pub envelope_predelay: Option<i32>,
    pub envelope_attack: Option<i32>,
    pub envelope_hold: Option<i32>,
    pub envelope_decay: Option<i32>,
    pub envelope_sustain: Option<i32>,
    pub envelope_release: Option<i32>,
    pub envelope_amount: Option<i32>,
    pub lfo_predelay: Option<u32>,
    pub lfo_attack: Option<u32>,
    pub lfo_amount: Option<i32>,
    pub lfo_speed: Option<u32>,
    pub lfo_shape: Option<i32>,
    pub envelope_attack_tension: Option<i32>,
    pub envelope_decay_tension: Option<i32>,
    pub envelope_release_tension: Option<i32>,
}

impl EnvelopeLfo {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            flags: opt(&mut r, Reader::i32),                    // 4
            envelope_enabled: opt(&mut r, Reader::i32),         // 8
            envelope_predelay: opt(&mut r, Reader::i32),        // 12
            envelope_attack: opt(&mut r, Reader::i32),          // 16
            envelope_hold: opt(&mut r, Reader::i32),            // 20
            envelope_decay: opt(&mut r, Reader::i32),           // 24
            envelope_sustain: opt(&mut r, Reader::i32),         // 28
            envelope_release: opt(&mut r, Reader::i32),         // 32
            envelope_amount: opt(&mut r, Reader::i32),          // 36
            lfo_predelay: opt(&mut r, Reader::u32),             // 40
            lfo_attack: opt(&mut r, Reader::u32),               // 44
            lfo_amount: opt(&mut r, Reader::i32),               // 48
            lfo_speed: opt(&mut r, Reader::u32),                // 52
            lfo_shape: opt(&mut r, Reader::i32),                // 56
            envelope_attack_tension: opt(&mut r, Reader::i32),  // 60
            envelope_decay_tension: opt(&mut r, Reader::i32),   // 64
            envelope_release_tension: opt(&mut r, Reader::i32), // 68
        }
    }
}

/// `LevelAdjustsEvent`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LevelAdjusts {
    pub pan: Option<i32>,
    pub volume: Option<u32>,
    pub mod_x: Option<i32>,
    pub mod_y: Option<i32>,
}

impl LevelAdjusts {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        let pan = opt(&mut r, Reader::i32);
        let volume = opt(&mut r, Reader::u32);
        let _u1 = opt(&mut r, Reader::u32);
        Self {
            pan,
            volume,
            mod_x: opt(&mut r, Reader::i32),
            mod_y: opt(&mut r, Reader::i32),
        }
    }
}

/// `LevelsEvent`: channel volume, pan, pitch shift and filter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Levels {
    pub pan: Option<i32>,
    pub volume: Option<u32>,
    pub pitch_shift: Option<i32>,
    pub filter_mod_x: Option<u32>,
    pub filter_mod_y: Option<u32>,
    pub filter_type: Option<u32>,
}

impl Levels {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            pan: opt(&mut r, Reader::i32),
            volume: opt(&mut r, Reader::u32),
            pitch_shift: opt(&mut r, Reader::i32),
            filter_mod_x: opt(&mut r, Reader::u32),
            filter_mod_y: opt(&mut r, Reader::u32),
            filter_type: opt(&mut r, Reader::u32),
        }
    }
}

/// `LinearMusical`: a time stored as bars of 768 and beats of 48 units of
/// five ticks.
fn linear_musical(raw: u32) -> MusicalTime {
    let (bars, rest) = (raw / 768, raw % 768);
    let (beats, rest) = (rest / 48, rest % 48);
    MusicalTime {
        bars,
        beats,
        ticks: rest * 5,
    }
}

/// `LogNormal(Int16ul[2], (0, 61440))`: a 0–1 value stored logarithmically.
/// PyFLP raises (failing the whole file) when the second number is not 63.
fn log_normal(r: &mut Reader<'_>) -> Result<Option<f64>, Failed> {
    let Some((first, second)) = r.attempt(|r| Some((r.u16()?, r.u16()?))) else {
        return Ok(None);
    };
    if first == 0 {
        return Ok(Some(0.0));
    }
    if second != 63 {
        return Err("not a LogNormal: its second number must be 63");
    }
    let value = 2f64.powf(f64::from(first) / 4096.0) / 32768.0;
    Ok(Some(value.clamp(0.0, 1.0)))
}

/// `ParametersEvent`: many channel settings in one record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parameters {
    pub fx_remove_dc: Option<bool>,
    pub delay_flags: Option<u8>,
    pub keyboard_main_pitch: Option<bool>,
    pub arp_direction: Option<u32>,
    pub arp_range: Option<u32>,
    pub arp_chord: Option<u32>,
    pub arp_time: Option<f32>,
    pub arp_gate: Option<f32>,
    pub arp_slide: Option<bool>,
    pub time_full_porta: Option<bool>,
    pub keyboard_add_root: Option<bool>,
    pub time_gate: Option<u16>,
    pub keyboard_key_region: Option<(u32, u32)>,
    pub fx_normalize: Option<bool>,
    pub fx_inverted: Option<bool>,
    pub content_declick_mode: Option<u8>,
    pub fx_crossfade: Option<u32>,
    pub fx_trim: Option<u32>,
    pub arp_repeat: Option<u32>,
    pub stretching_time: Option<MusicalTime>,
    pub stretching_pitch: Option<i32>,
    pub stretching_multiplier: Option<f64>,
    pub stretching_mode: Option<i32>,
    pub fx_start: Option<f64>,
    pub fx_length: Option<f64>,
    pub playback_start_offset: Option<u32>,
    pub fx_fix_trim: Option<bool>,
}

impl Parameters {
    pub(crate) fn read(data: &[u8]) -> Result<Self, Failed> {
        let mut r = Reader::new(data);
        let r = &mut r;
        let skip = |r: &mut Reader<'_>, n| {
            let _ = opt(r, |r| r.bytes(n));
        };
        skip(r, 9); // 9
        let fx_remove_dc = opt(r, Reader::flag); // 10
        let delay_flags = opt(r, Reader::u8); // 11
        let keyboard_main_pitch = opt(r, Reader::flag); // 12
        skip(r, 28); // 40
        let arp_direction = opt(r, Reader::u32); // 44
        let arp_range = opt(r, Reader::u32); // 48
        let arp_chord = opt(r, Reader::u32); // 52
        let arp_time = opt(r, Reader::f32); // 56
        let arp_gate = opt(r, Reader::f32); // 60
        let arp_slide = opt(r, Reader::flag); // 61
        skip(r, 1); // 62
        let time_full_porta = opt(r, Reader::flag); // 63
        let keyboard_add_root = opt(r, Reader::flag); // 64
        let time_gate = opt(r, Reader::u16); // 66
        skip(r, 2); // 68
        let keyboard_key_region = opt(r, |r| Some((r.u32()?, r.u32()?))); // 76
        skip(r, 4); // 80
        let fx_normalize = opt(r, Reader::flag); // 81
        let fx_inverted = opt(r, Reader::flag); // 82
        skip(r, 1); // 83
        let content_declick_mode = opt(r, Reader::u8); // 84
        let fx_crossfade = opt(r, Reader::u32); // 88
        let fx_trim = opt(r, Reader::u32); // 92
        let arp_repeat = opt(r, Reader::u32); // 96; FL 4.5.2+
        let stretching_time = opt(r, Reader::u32).map(linear_musical); // 100
        let stretching_pitch = opt(r, Reader::i32); // 104
        let stretching_multiplier = opt(r, Reader::i32).map(|v| 2f64.powf(f64::from(v) / 10000.0)); // 108
        let stretching_mode = opt(r, Reader::i32); // 112
        skip(r, 21); // 133
        let fx_start = log_normal(r)?; // 137
        skip(r, 4); // 141
        let fx_length = log_normal(r)?; // 145
        skip(r, 3); // 148
        let playback_start_offset = opt(r, Reader::u32); // 152
        skip(r, 5); // 157
        let fx_fix_trim = opt(r, Reader::flag); // 158 (FL 20.8.4 max)
        Ok(Self {
            fx_remove_dc,
            delay_flags,
            keyboard_main_pitch,
            arp_direction,
            arp_range,
            arp_chord,
            arp_time,
            arp_gate,
            arp_slide,
            time_full_porta,
            keyboard_add_root,
            time_gate,
            keyboard_key_region,
            fx_normalize,
            fx_inverted,
            content_declick_mode,
            fx_crossfade,
            fx_trim,
            arp_repeat,
            stretching_time,
            stretching_pitch,
            stretching_multiplier,
            stretching_mode,
            fx_start,
            fx_length,
            playback_start_offset,
            fx_fix_trim,
        })
    }
}

/// `PolyphonyEvent`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Polyphony {
    pub max: Option<u32>,
    pub slide: Option<u32>,
    pub flags: Option<u8>,
}

impl Polyphony {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            max: opt(&mut r, Reader::u32),
            slide: opt(&mut r, Reader::u32),
            flags: opt(&mut r, Reader::u8),
        }
    }
}

/// `TrackingEvent`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Tracking {
    pub middle_value: Option<u32>,
    pub pan: Option<i32>,
    pub mod_x: Option<i32>,
    pub mod_y: Option<i32>,
}

impl Tracking {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            middle_value: opt(&mut r, Reader::u32),
            pan: opt(&mut r, Reader::i32),
            mod_x: opt(&mut r, Reader::i32),
            mod_y: opt(&mut r, Reader::i32),
        }
    }
}

/// One point of an automation clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutomationPoint {
    /// Change in X from the previous point.
    pub offset: f64,
    /// Position on X: the running sum of offsets (PyFLP's `position`).
    pub position: f64,
    /// 0–1.
    pub value: f64,
    /// 0–1.
    pub tension: f32,
}

/// `AutomationEvent`: an automation clip's LFO amount and points.
#[derive(Debug, Clone, PartialEq)]
pub struct Automation {
    pub lfo_amount: i32,
    pub points: Vec<AutomationPoint>,
}

impl Automation {
    pub(crate) fn read(data: &[u8]) -> Result<Self, Failed> {
        const SHORT: Failed = "automation data too short";
        let mut r = Reader::new(data);
        r.bytes(4).ok_or(SHORT)?; // 4
        let lfo_amount = r.i32().ok_or(SHORT)?; // 8
        r.bytes(1 + 2 + 2 + 4).ok_or(SHORT)?; // 17
        let count = r.u32().ok_or(SHORT)?; // 21
        let mut points = Vec::new();
        let mut position = 0.0;
        for _ in 0..count {
            let offset = r.f64().ok_or(SHORT)?;
            let value = r.f64().ok_or(SHORT)?;
            let tension = r.f32().ok_or(SHORT)?;
            r.bytes(4).ok_or(SHORT)?;
            position += offset;
            points.push(AutomationPoint {
                offset,
                // PyFLP's first point is its own offset; later ones sum.
                position: if points.is_empty() { offset } else { position },
                value,
                tension,
            });
        }
        Ok(Self { lfo_amount, points })
    }
}

// --- pattern --------------------------------------------------------------------

/// One record of `ControllerEvent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControllerItem {
    /// Can be a delta as well.
    pub position: u32,
    pub channel: u8,
    pub flags: u8,
    pub value: f32,
}

pub(crate) fn controllers(data: &[u8]) -> Vec<ControllerItem> {
    Reader::new(data).greedy(|r| {
        let position = r.u32()?; // 4
        r.bytes(2)?; // 6
        let channel = r.u8()?; // 7
        let flags = r.u8()?; // 8
        let value = r.f32()?; // 12
        Some(ControllerItem {
            position,
            channel,
            flags,
            value,
        })
    })
}

/// One record of `NotesEvent`: a note in a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteItem {
    pub position: u32,
    pub flags: u16,
    /// The channel's `iid`.
    pub rack_channel: u16,
    pub length: u32,
    pub key: u16,
    pub group: u16,
    pub fine_pitch: u8,
    pub release: u8,
    pub midi_channel: u8,
    pub pan: u8,
    pub velocity: u8,
    pub mod_x: u8,
    pub mod_y: u8,
}

impl NoteItem {
    /// `_NoteFlags.Slide`.
    pub const SLIDE: u16 = 1 << 3;
}

pub(crate) fn notes(data: &[u8]) -> Vec<NoteItem> {
    Reader::new(data).greedy(|r| {
        let position = r.u32()?; // 4
        let flags = r.u16()?; // 6
        let rack_channel = r.u16()?; // 8
        let length = r.u32()?; // 12
        let key = r.u16()?; // 14
        let group = r.u16()?; // 16
        let fine_pitch = r.u8()?; // 17
        r.bytes(1)?; // 18
        let release = r.u8()?; // 19
        let midi_channel = r.u8()?; // 20
        let pan = r.u8()?; // 21
        let velocity = r.u8()?; // 22
        let mod_x = r.u8()?; // 23
        let mod_y = r.u8()?; // 24
        Some(NoteItem {
            position,
            flags,
            rack_channel,
            length,
            key,
            group,
            fine_pitch,
            release,
            midi_channel,
            pan,
            velocity,
            mod_x,
            mod_y,
        })
    })
}

// --- arrangement ----------------------------------------------------------------

/// `PLSelectionEvent`: the playlist's selected (looped) range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlSelection {
    pub start: Option<u32>,
    pub end: Option<u32>,
}

impl PlSelection {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            start: opt(&mut r, Reader::u32),
            end: opt(&mut r, Reader::u32),
        }
    }
}

/// One record of `PlaylistEvent`: a clip in an arrangement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaylistItem {
    pub position: u32,
    /// Always 20480.
    pub pattern_base: u16,
    pub item_index: u16,
    pub length: u32,
    /// Stored reversed: track 1 is 499 (198 before FL 12.9.1).
    pub track_rvidx: u16,
    pub group: u16,
    pub item_flags: u16,
    pub start_offset: f32,
    pub end_offset: f32,
}

/// Records are 60 bytes (FL 21+) when the data divides by 60, else 32.
pub(crate) fn playlist(data: &[u8]) -> Vec<PlaylistItem> {
    let new = data.len() % 60 == 0;
    Reader::new(data).greedy(|r| {
        let position = r.u32()?; // 4
        let pattern_base = r.u16()?; // 6
        let item_index = r.u16()?; // 8
        let length = r.u32()?; // 12
        let track_rvidx = r.u16()?; // 14
        let group = r.u16()?; // 16
        r.bytes(2)?; // 18
        let item_flags = r.u16()?; // 20
        r.bytes(4)?; // 24
        let start_offset = r.f32()?; // 28
        let end_offset = r.f32()?; // 32
        if new {
            r.bytes(28)?; // 60
        }
        Some(PlaylistItem {
            position,
            pattern_base,
            item_index,
            length,
            track_rvidx,
            group,
            item_flags,
            start_offset,
            end_offset,
        })
    })
}

/// `TrackEvent`: a playlist track's settings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackData {
    pub iid: Option<u32>,
    pub color: Option<u32>,
    pub icon: Option<u32>,
    pub enabled: Option<bool>,
    /// A fraction; PyFLP shows it as a percentage string.
    pub height: Option<f32>,
    pub locked_height: Option<i32>,
    pub content_locked: Option<bool>,
    pub motion: Option<u32>,
    pub press: Option<u32>,
    pub trigger_sync: Option<u32>,
    pub queued: Option<bool>,
    pub tolerant: Option<bool>,
    pub position_sync: Option<u32>,
    pub grouped: Option<bool>,
    pub locked: Option<bool>,
}

impl TrackData {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        Self {
            iid: opt(&mut r, Reader::u32),             // 4
            color: opt(&mut r, Reader::u32),           // 8
            icon: opt(&mut r, Reader::u32),            // 12
            enabled: opt(&mut r, Reader::flag),        // 13
            height: opt(&mut r, Reader::f32),          // 17
            locked_height: opt(&mut r, Reader::i32),   // 21
            content_locked: opt(&mut r, Reader::flag), // 22
            motion: opt(&mut r, Reader::u32),          // 26
            press: opt(&mut r, Reader::u32),           // 30
            trigger_sync: opt(&mut r, Reader::u32),    // 34
            queued: opt(&mut r, Reader::bool32),       // 38
            tolerant: opt(&mut r, Reader::bool32),     // 42
            position_sync: opt(&mut r, Reader::u32),   // 46
            grouped: opt(&mut r, Reader::flag),        // 47
            locked: opt(&mut r, Reader::flag),         // 48
        }
    }
}

// --- mixer ------------------------------------------------------------------------

/// `InsertFlagsEvent`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InsertFlags {
    pub flags: Option<u32>,
}

impl InsertFlags {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        let _u1 = opt(&mut r, |r| r.bytes(4));
        Self {
            flags: opt(&mut r, Reader::u32),
        }
    }
}

/// `InsertRoutingEvent`: one flag per insert this insert may send to.
pub(crate) fn insert_routing(data: &[u8]) -> Vec<bool> {
    data.iter().map(|b| *b != 0).collect()
}

/// One record of `MixerParamsEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixerParam {
    pub id: u8,
    /// Insert in bits 6–12, slot in bits 0–5.
    pub channel_data: u16,
    pub msg: i32,
}

impl MixerParam {
    pub fn insert(&self) -> usize {
        usize::from((self.channel_data >> 6) & 0x7F)
    }

    pub fn slot(&self) -> usize {
        usize::from(self.channel_data & 0x3F)
    }
}

pub(crate) fn mixer_params(data: &[u8]) -> Vec<MixerParam> {
    Reader::new(data).greedy(|r| {
        r.bytes(4)?; // 4
        let id = r.u8()?; // 5
        r.bytes(1)?; // 6
        let channel_data = r.u16()?; // 8
        let msg = r.i32()?; // 12
        Some(MixerParam {
            id,
            channel_data,
            msg,
        })
    })
}

// --- plugin wrapper ---------------------------------------------------------------

/// `WrapperEvent`: the plugin window's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Wrapper {
    pub flags: Option<u16>,
    pub page: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Wrapper {
    pub(crate) fn read(data: &[u8]) -> Self {
        let mut r = Reader::new(data);
        let _u1 = opt(&mut r, |r| r.bytes(16)); // 16
        let flags = opt(&mut r, Reader::u16); // 18
        let _u2 = opt(&mut r, |r| r.bytes(2)); // 20
        let page = opt(&mut r, Reader::u8); // 21
        let _u3 = opt(&mut r, |r| r.bytes(23)); // 44
        Self {
            flags,
            page,
            width: opt(&mut r, Reader::u32),  // 48
            height: opt(&mut r, Reader::u32), // 52
        }
    }
}

/// `RGBA.from_bytes`: each byte over 255.
pub(crate) fn rgba(bytes: [u8; 4]) -> Rgba {
    let c = |b: u8| f64::from(b) / 255.0;
    Rgba {
        red: c(bytes[0]),
        green: c(bytes[1]),
        blue: c(bytes[2]),
        alpha: c(bytes[3]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_struct_fields_read_what_fits() {
        // Max and slide (8 bytes), then flags from a ninth byte.
        let data = [8, 0, 0, 0, 0x34, 0x03, 0, 0, 3];
        assert_eq!(
            Polyphony::read(&data),
            Polyphony {
                max: Some(8),
                slide: Some(820),
                flags: Some(3)
            }
        );
        // Five bytes: max fits, slide does not, so its byte becomes flags.
        assert_eq!(
            Polyphony::read(&data[..5]),
            Polyphony {
                max: Some(8),
                slide: None,
                flags: Some(0x34)
            }
        );
    }

    #[test]
    fn musical_time_and_log_normal() {
        assert_eq!(
            linear_musical(768 * 2 + 48 * 3 + 7),
            MusicalTime {
                bars: 2,
                beats: 3,
                ticks: 35
            }
        );
        let mut r = Reader::new(&[0, 0, 0, 0]);
        assert_eq!(log_normal(&mut r), Ok(Some(0.0)));
        let mut r = Reader::new(&[0, 0xF0, 63, 0]);
        assert_eq!(log_normal(&mut r), Ok(Some(1.0)), "61440 is the top");
        let mut r = Reader::new(&[1, 0, 5, 0]);
        assert!(log_normal(&mut r).is_err(), "PyFLP rejects this");
    }

    #[test]
    fn playlist_records_follow_the_data_size() {
        let mut item = vec![0u8; 32];
        item[4..6].copy_from_slice(&20480u16.to_le_bytes());
        item[6..8].copy_from_slice(&20481u16.to_le_bytes());
        let old = playlist(&[item.clone(), item.clone()].concat());
        assert_eq!(old.len(), 2);
        let mut new = item.clone();
        new.extend_from_slice(&[0; 28]);
        assert_eq!(playlist(&new).len(), 1);
        assert_eq!(playlist(&new)[0].item_index, 20481);
    }
}

//! `project.export_midi`: writes what the transport plays as a Standard MIDI
//! File (format 1), following the public MIDI 1.0 file format.

use std::path::Path;

use musiccraft_commands::{Command, CommandError, Outcome};
use musiccraft_params::PPQ;
use musiccraft_project::{Meter, Project, TimeSig};

use crate::sequence::midi_events;

/// Appends a variable-length quantity.
fn vlq(out: &mut Vec<u8>, value: u32) {
    let mut bytes = [0_u8; 5];
    let mut n = 0;
    let mut v = value;
    loop {
        bytes[n] = (v & 0x7f) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        out.push(if i > 0 { bytes[i] | 0x80 } else { bytes[i] });
    }
}

/// Wraps track data in an `MTrk` chunk, adding the end-of-track event.
fn chunk(mut data: Vec<u8>) -> Vec<u8> {
    data.extend([0x00, 0xff, 0x2f, 0x00]);
    let mut out = b"MTrk".to_vec();
    out.extend(u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend(data);
    out
}

/// A meta event at delta 0.
fn meta(out: &mut Vec<u8>, kind: u8, payload: &[u8]) {
    out.extend([0x00, 0xff, kind]);
    vlq(out, u32::try_from(payload.len()).unwrap_or(0));
    out.extend(payload);
}

/// The FF 58 payload of `sig`: numerator, denominator as a power of two,
/// MIDI clocks per beat and 32nd notes per quarter.
fn sig_payload(sig: TimeSig) -> [u8; 4] {
    let num = u8::try_from(sig.num()).unwrap_or(4);
    let power = u8::try_from(sig.den().trailing_zeros()).unwrap_or(2);
    let clocks = u8::try_from(96 / sig.den().max(1)).unwrap_or(24);
    [num, power, clocks, 8]
}

/// Time signature events: the start's, then one per change.
fn signatures(out: &mut Vec<u8>, meter: &Meter) {
    meta(out, 0x58, &sig_payload(meter.first()));
    let mut last = 0;
    for (tick, sig) in meter.changes() {
        vlq(out, tick - last);
        last = tick;
        out.extend([0xff, 0x58, 4]);
        out.extend(sig_payload(sig));
    }
}

/// The whole file: a tempo track, then one track per channel with notes.
#[must_use]
pub fn midi_file(project: &Project) -> Vec<u8> {
    let (events, _) = midi_events(project);
    let mut tracks = Vec::new();
    let mut tempo = Vec::new();
    meta(&mut tempo, 0x03, project.name().as_bytes());
    let micros = (60_000_000.0 / project.tempo_bpm()).round() as u32;
    meta(&mut tempo, 0x51, &micros.to_be_bytes()[1..]);
    signatures(&mut tempo, &project.playing_meter());
    tracks.push(chunk(tempo));
    for (index, channel) in project.channels.iter().enumerate() {
        let mine: Vec<_> = events
            .iter()
            .filter(|e| usize::from(e.channel) == index)
            .collect();
        if mine.is_empty() {
            continue;
        }
        let midi_channel = (index % 16) as u8;
        // (tick, is note on, key, velocity): offs sort before ons on a tick.
        let mut messages: Vec<(u64, bool, u8, u8)> = Vec::new();
        for e in mine {
            let velocity = (e.velocity * 127.0).round().clamp(1.0, 127.0) as u8;
            messages.push((e.start, true, e.key, velocity));
            messages.push((e.start + e.length.max(1), false, e.key, 0));
        }
        messages.sort_by_key(|&(tick, on, key, _)| (tick, on, key));
        let mut data = Vec::new();
        meta(&mut data, 0x03, channel.name.as_bytes());
        let mut last = 0;
        for (tick, on, key, velocity) in messages {
            vlq(&mut data, u32::try_from(tick - last).unwrap_or(u32::MAX));
            last = tick;
            let status = if on { 0x90 } else { 0x80 } | midi_channel;
            data.extend([status, key, velocity]);
        }
        tracks.push(chunk(data));
    }
    let mut out = b"MThd".to_vec();
    out.extend(6_u32.to_be_bytes());
    out.extend(1_u16.to_be_bytes());
    out.extend(
        u16::try_from(tracks.len())
            .unwrap_or(u16::MAX)
            .to_be_bytes(),
    );
    out.extend(u16::try_from(PPQ).unwrap_or(96).to_be_bytes());
    for track in tracks {
        out.extend(track);
    }
    out
}

/// `project.export_midi <path>`.
#[derive(Default)]
pub struct ExportMidiCommand;

impl Command for ExportMidiCommand {
    fn name(&self) -> &'static str {
        "project.export_midi"
    }

    fn summary(&self) -> &'static str {
        "Write the pattern or song (by play mode) as a Standard MIDI File, one track per channel."
    }

    fn usage(&self) -> &'static str {
        "project.export_midi <path>"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let [path] = args else {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        };
        let bytes = midi_file(project);
        std::fs::write(Path::new(path), &bytes).map_err(|e| CommandError::Io(e.to_string()))?;
        Ok(Outcome::done(format!(
            "exported MIDI ({} bytes) to {path}",
            bytes.len()
        )))
    }
}

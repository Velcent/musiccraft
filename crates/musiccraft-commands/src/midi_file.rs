//! Reader for Standard MIDI Files (SMF formats 0 and 1), enough to import notes.
//! Follows the public MIDI 1.0 specification's file format.

/// A note read from a MIDI file, timed in the file's own ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileNote {
    pub start: u64,
    pub length: u64,
    pub key: u8,
    pub velocity: u8,
}

/// Notes and the file's ticks per quarter note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MidiFile {
    pub ticks_per_quarter: u16,
    pub notes: Vec<FileNote>,
}

/// Most notes accepted from one file.
const MAX_NOTES: usize = 100_000;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.data.get(self.pos).ok_or("unexpected end of file")?;
        self.pos += 1;
        Ok(b)
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("length overflow")?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or("unexpected end of file")?;
        self.pos = end;
        Ok(slice)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let b = self.bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let b = self.bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Variable-length quantity (at most four bytes).
    fn vlq(&mut self) -> Result<u64, String> {
        let mut value = 0_u64;
        for _ in 0..4 {
            let b = self.byte()?;
            value = (value << 7) | u64::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("variable-length number is too long".to_owned())
    }
}

fn read_track(data: &[u8], notes: &mut Vec<FileNote>) -> Result<(), String> {
    let mut r = Reader { data, pos: 0 };
    let mut time = 0_u64;
    let mut running: Option<u8> = None;
    // Sounding notes: (channel, key) → (start, velocity), oldest first.
    let mut open: Vec<(u8, u8, u64, u8)> = Vec::new();
    while r.pos < data.len() {
        time = time.saturating_add(r.vlq()?);
        let first = r.byte()?;
        let status = if first & 0x80 != 0 {
            first
        } else {
            // Running status: `first` is already the first data byte.
            r.pos -= 1;
            running.ok_or("data byte without a status")?
        };
        match status {
            0xff => {
                running = None;
                let kind = r.byte()?;
                let len = usize::try_from(r.vlq()?).map_err(|e| e.to_string())?;
                r.bytes(len)?;
                if kind == 0x2f {
                    break;
                }
            }
            0xf0 | 0xf7 => {
                running = None;
                let len = usize::try_from(r.vlq()?).map_err(|e| e.to_string())?;
                r.bytes(len)?;
            }
            0xf1..=0xfe => return Err(format!("unexpected system message {status:#x}")),
            _ => {
                running = Some(status);
                let channel = status & 0x0f;
                match status & 0xf0 {
                    0x80 | 0x90 => {
                        let key = r.byte()? & 0x7f;
                        let velocity = r.byte()? & 0x7f;
                        if status & 0xf0 == 0x90 && velocity > 0 {
                            open.push((channel, key, time, velocity));
                        } else if let Some(i) = open
                            .iter()
                            .position(|(c, k, ..)| *c == channel && *k == key)
                        {
                            let (_, key, start, velocity) = open.remove(i);
                            notes.push(FileNote {
                                start,
                                length: (time - start).max(1),
                                key,
                                velocity,
                            });
                        }
                    }
                    0xa0 | 0xb0 | 0xe0 => {
                        r.bytes(2)?;
                    }
                    _ => {
                        r.bytes(1)?;
                    }
                }
            }
        }
        if notes.len() > MAX_NOTES {
            return Err(format!("more than {MAX_NOTES} notes"));
        }
    }
    // Notes still sounding at the end of the track stop there.
    for (_, key, start, velocity) in open {
        notes.push(FileNote {
            start,
            length: (time - start).max(1),
            key,
            velocity,
        });
    }
    Ok(())
}

/// Parses a Standard MIDI File.
pub(crate) fn parse(data: &[u8]) -> Result<MidiFile, String> {
    let mut r = Reader { data, pos: 0 };
    if r.bytes(4)? != b"MThd" {
        return Err("not a MIDI file".to_owned());
    }
    let header_len = r.u32()? as usize;
    let format = r.u16()?;
    let tracks = r.u16()?;
    let division = r.u16()?;
    r.bytes(header_len.saturating_sub(6))?;
    if format > 1 {
        return Err(format!("MIDI file format {format} is not supported"));
    }
    if division & 0x8000 != 0 || division == 0 {
        return Err("SMPTE-timed MIDI files are not supported".to_owned());
    }
    let mut notes = Vec::new();
    for _ in 0..tracks {
        if r.pos >= data.len() {
            break;
        }
        let kind = r.bytes(4)?;
        let len = r.u32()? as usize;
        let chunk = r.bytes(len)?;
        if kind == b"MTrk" {
            read_track(chunk, &mut notes)?;
        }
    }
    notes.sort_by_key(|n| (n.start, n.key));
    Ok(MidiFile {
        ticks_per_quarter: division,
        notes,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a format-0 file from raw track bytes.
    pub fn smf(division: u16, track: &[u8]) -> Vec<u8> {
        let mut out = b"MThd".to_vec();
        out.extend(6_u32.to_be_bytes());
        out.extend(0_u16.to_be_bytes());
        out.extend(1_u16.to_be_bytes());
        out.extend(division.to_be_bytes());
        out.extend(b"MTrk");
        out.extend((track.len() as u32).to_be_bytes());
        out.extend(track);
        out
    }

    /// C4 for one beat, then E4 and G4 together using running status and
    /// velocity-0 note-offs; a tempo meta event; and a sysex message.
    pub fn sample_track() -> Vec<u8> {
        vec![
            0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, // tempo
            0x00, 0x90, 60, 100, // C on
            0x81, 0x40, 0x80, 60, 0, // after 192 ticks: C off
            0x00, 0x90, 64, 90, // E on
            0x00, 67, 80, // running status: G on
            0x00, 0xf0, 0x02, 0x7e, 0xf7, // sysex
            0x60, 0x90, 64, 0, // after 96: E off (velocity 0)
            0x00, 67, 0, // G off
            0x00, 0xff, 0x2f, 0x00, // end of track
        ]
    }

    #[test]
    fn reads_notes_with_running_status_and_meta_events() {
        let file = parse(&smf(192, &sample_track())).unwrap();
        assert_eq!(file.ticks_per_quarter, 192);
        assert_eq!(
            file.notes,
            [
                FileNote {
                    start: 0,
                    length: 192,
                    key: 60,
                    velocity: 100
                },
                FileNote {
                    start: 192,
                    length: 96,
                    key: 64,
                    velocity: 90
                },
                FileNote {
                    start: 192,
                    length: 96,
                    key: 67,
                    velocity: 80
                },
            ]
        );
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse(b"RIFF....").is_err());
        assert!(parse(&smf(192, &[0x00, 0x90, 60])).is_err(), "truncated");
        assert!(parse(&smf(0xe728, &[])).is_err(), "SMPTE");
        assert!(
            parse(&smf(96, &[0x00, 60, 100])).is_err(),
            "data without status"
        );
    }

    #[test]
    fn unterminated_notes_end_with_the_track() {
        let file = parse(&smf(96, &[0x00, 0x90, 50, 64, 0x60, 0xff, 0x2f, 0x00])).unwrap();
        assert_eq!(file.notes[0].length, 96);
    }
}

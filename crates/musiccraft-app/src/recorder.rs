//! Turns live notes into pattern notes while recording.

/// A finished recorded note, in ticks within the pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recorded {
    pub start: u32,
    pub length: u32,
    pub key: u8,
    pub velocity: u8,
}

/// Notes currently held, with the loop position where they started.
#[derive(Debug, Default)]
pub struct Recorder {
    open: Vec<(u8, f64, u8)>,
}

impl Recorder {
    pub fn note_on(&mut self, key: u8, velocity: u8, position: f64) {
        self.open.retain(|(k, ..)| *k != key);
        self.open.push((key, position, velocity.clamp(1, 127)));
    }

    /// Ends a held note. `loop_ticks` is the length of the loop the playhead
    /// wraps at; `pattern_ticks` is the pattern the note is written into.
    pub fn note_off(
        &mut self,
        key: u8,
        position: f64,
        loop_ticks: u32,
        pattern_ticks: u32,
    ) -> Option<Recorded> {
        let index = self.open.iter().position(|(k, ..)| *k == key)?;
        let (key, start, velocity) = self.open.remove(index);
        let loop_ticks = f64::from(loop_ticks.max(1));
        let mut length = position - start;
        if length < 0.0 {
            // The playhead wrapped while the key was held.
            length += loop_ticks;
        }
        let start = (start.max(0.0) as u32) % pattern_ticks.max(1);
        Some(Recorded {
            start,
            length: (length.round() as u32).max(1),
            key,
            velocity,
        })
    }

    /// Forgets held notes (for example when recording stops).
    pub fn clear(&mut self) {
        self.open.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_held_notes_and_handles_wraps() {
        let mut r = Recorder::default();
        r.note_on(60, 100, 96.4);
        assert_eq!(
            r.note_off(60, 144.0, 384, 384),
            Some(Recorded {
                start: 96,
                length: 48,
                key: 60,
                velocity: 100
            })
        );
        r.note_on(62, 0, 370.0);
        let wrapped = r.note_off(62, 10.0, 384, 384).unwrap();
        assert_eq!(
            (wrapped.start, wrapped.length, wrapped.velocity),
            (370, 24, 1)
        );
        assert_eq!(r.note_off(99, 0.0, 384, 384), None, "never pressed");
    }

    #[test]
    fn song_positions_fold_into_the_pattern() {
        let mut r = Recorder::default();
        r.note_on(64, 90, 384.0 * 5.0 + 24.0);
        let note = r.note_off(64, 384.0 * 5.0 + 48.0, 384 * 8, 384).unwrap();
        assert_eq!((note.start, note.length), (24, 24));
    }
}

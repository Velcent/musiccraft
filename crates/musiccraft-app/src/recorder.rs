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

/// Replace recording: which pattern ticks the playhead has passed during a
/// take, and which notes the take itself recorded (never removed).
#[derive(Debug, Default)]
pub struct Sweep {
    last: Option<u32>,
    recorded: Vec<(u32, u8)>,
}

impl Sweep {
    /// Moves to `position` (song ticks) and returns the pattern tick ranges
    /// passed since the last call, folded into `pattern_ticks` (two ranges when
    /// the pattern wraps). The first call of a take only sets the start.
    pub fn advance(&mut self, position: f64, pattern_ticks: u32) -> [Option<(u32, u32)>; 2] {
        let len = pattern_ticks.max(1);
        let now = (position.max(0.0) as u32) % len;
        let Some(last) = self.last.replace(now) else {
            return [None, None];
        };
        if now >= last {
            [(now > last).then_some((last, now)), None]
        } else {
            [
                (last < len).then_some((last, len)),
                (now > 0).then_some((0, now)),
            ]
        }
    }

    /// Remembers a note recorded in this take.
    pub fn recorded(&mut self, start: u32, key: u8) {
        self.recorded.push((start, key));
    }

    /// Whether `note` (start, key) should go: it starts in a passed range and
    /// was not recorded in this take.
    pub fn removes(&self, ranges: &[Option<(u32, u32)>; 2], start: u32, key: u8) -> bool {
        let passed = ranges
            .iter()
            .flatten()
            .any(|(from, to)| (*from..*to).contains(&start));
        passed && !self.recorded.contains(&(start, key))
    }

    /// Ends the take.
    pub fn reset(&mut self) {
        self.last = None;
        self.recorded.clear();
    }
}

/// Step edit: keys held together go in at one cursor; the cursor moves on
/// when they are all released.
#[derive(Debug, Default)]
pub struct StepEntry {
    held: Vec<u8>,
    cursor: Option<f64>,
}

impl StepEntry {
    /// A key went down with the playhead at `position` (song ticks, already
    /// snapped). Returns where the note goes: the chord's cursor.
    pub fn note_on(&mut self, key: u8, position: f64) -> f64 {
        if !self.held.contains(&key) {
            self.held.push(key);
        }
        *self.cursor.get_or_insert(position)
    }

    /// A key came up. When it was the last one held, returns the chord's
    /// cursor so the caller can move on from it.
    pub fn note_off(&mut self, key: u8) -> Option<f64> {
        let before = self.held.len();
        self.held.retain(|k| *k != key);
        if before > 0 && self.held.is_empty() {
            self.cursor.take()
        } else {
            None
        }
    }

    /// Forgets held keys (step edit turned off or playback started).
    pub fn clear(&mut self) {
        self.held.clear();
        self.cursor = None;
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

    #[test]
    fn a_sweep_reports_passed_ticks_and_spares_recorded_notes() {
        let mut sweep = Sweep::default();
        assert_eq!(sweep.advance(10.0, 384), [None, None], "the take starts");
        let ranges = sweep.advance(100.0, 384);
        assert_eq!(ranges, [Some((10, 100)), None]);
        assert!(sweep.removes(&ranges, 10, 60));
        assert!(sweep.removes(&ranges, 99, 60));
        assert!(!sweep.removes(&ranges, 100, 60), "not passed yet");
        sweep.recorded(48, 62);
        assert!(!sweep.removes(&ranges, 48, 62), "recorded in this take");
        // Wrapping at the pattern end (a song position folds in too).
        let ranges = sweep.advance(384.0 * 3.0 + 20.0, 384);
        assert_eq!(ranges, [Some((100, 384)), Some((0, 20))]);
        assert!(sweep.removes(&ranges, 0, 60) && sweep.removes(&ranges, 383, 60));
        sweep.reset();
        assert_eq!(sweep.advance(30.0, 384), [None, None]);
    }

    #[test]
    fn step_entry_keeps_chords_on_one_cursor() {
        let mut steps = StepEntry::default();
        assert!((steps.note_on(60, 96.0) - 96.0).abs() < f64::EPSILON);
        assert!(
            (steps.note_on(64, 120.0) - 96.0).abs() < f64::EPSILON,
            "same chord"
        );
        assert_eq!(steps.note_off(60), None, "64 still held");
        assert_eq!(steps.note_off(64), Some(96.0));
        assert!(
            (steps.note_on(67, 120.0) - 120.0).abs() < f64::EPSILON,
            "next step"
        );
        assert_eq!(steps.note_off(99), None, "never pressed");
    }
}

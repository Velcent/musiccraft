/// A note to play, positioned in ticks from the start of the sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteEvent {
    /// Start, in ticks.
    pub start: u64,
    /// Duration, in ticks.
    pub length: u64,
    /// Channel index.
    pub channel: u16,
    /// MIDI key.
    pub key: u8,
    /// Velocity, 0..=1.
    pub velocity: f32,
}

/// A looping list of notes, sorted by start time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sequence {
    events: Vec<NoteEvent>,
    length: u64,
}

impl Sequence {
    /// Builds a sequence that loops after `length` ticks. Events are sorted;
    /// events starting at or after `length` are dropped.
    #[must_use]
    pub fn new(mut events: Vec<NoteEvent>, length: u64) -> Self {
        events.retain(|e| e.start < length);
        events.sort_by_key(|e| (e.start, e.channel, e.key));
        Self { events, length }
    }

    /// The events in start order.
    #[must_use]
    pub fn events(&self) -> &[NoteEvent] {
        &self.events
    }

    /// Loop length in ticks.
    #[must_use]
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Index of the first event starting at or after `position` ticks.
    #[must_use]
    pub fn first_at_or_after(&self, position: f64) -> usize {
        self.events.partition_point(|e| (e.start as f64) < position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(start: u64) -> NoteEvent {
        NoteEvent {
            start,
            length: 10,
            channel: 0,
            key: 60,
            velocity: 1.0,
        }
    }

    #[test]
    fn events_are_sorted_and_clipped() {
        let seq = Sequence::new(vec![note(50), note(10), note(200)], 100);
        let starts: Vec<_> = seq.events().iter().map(|e| e.start).collect();
        assert_eq!(starts, [10, 50]);
        assert_eq!(seq.first_at_or_after(10.0), 0);
        assert_eq!(seq.first_at_or_after(10.5), 1);
        assert_eq!(seq.first_at_or_after(99.0), 2);
    }
}

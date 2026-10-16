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
    /// Note pan, -1..=1, applied on top of the channel pan.
    pub pan: f32,
    /// Glide the sounding note to this key instead of starting a new one.
    pub slide: bool,
    /// Per-note shaping.
    pub shape: NoteShape,
}

/// Per-note properties beyond pitch, velocity and pan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteShape {
    /// Fine tuning in semitones (−1..1).
    pub fine: f32,
    /// Multiplier on the release time.
    pub release: f32,
    /// Filter cutoff offset in octaves.
    pub cutoff: f32,
    /// Filter resonance offset (−0.5..0.5).
    pub resonance: f32,
    /// Glide from the channel's previous note, retriggering.
    pub porta: bool,
    /// Colour group, 0..=15: slide notes bend only notes of their own group.
    pub color: u8,
}

impl Default for NoteShape {
    fn default() -> Self {
        Self {
            fine: 0.0,
            release: 1.0,
            cutoff: 0.0,
            resonance: 0.0,
            porta: false,
            color: 0,
        }
    }
}

/// A parameter an automation lane writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationTarget {
    /// Channel volume (linear gain).
    ChannelVolume(u16),
    /// Channel pan (-1..1).
    ChannelPan(u16),
    /// Channel instrument parameter by index.
    ChannelParam(u16, u8),
    /// Mixer track volume (linear gain).
    TrackVolume(u16),
    /// Mixer track pan (-1..1).
    TrackPan(u16),
    /// Effect parameter: track, slot, parameter index.
    EffectParam(u16, u8, u8),
}

/// Values for one parameter over a span of the song: (tick, value) points in
/// tick order, linearly interpolated. Outside the first and last point the lane
/// does nothing, so the parameter keeps its last value.
#[derive(Debug, Clone, PartialEq)]
pub struct AutomationLane {
    /// What the lane writes.
    pub target: AutomationTarget,
    /// Points sorted by tick, in real parameter units.
    pub points: Vec<(u64, f32)>,
}

impl AutomationLane {
    /// The value at `position` ticks, if the lane covers it.
    #[must_use]
    pub fn value_at(&self, position: f64) -> Option<f32> {
        let first = self.points.first()?;
        let last = self.points.last()?;
        if position < first.0 as f64 || position > last.0 as f64 {
            return None;
        }
        let i = self.points.partition_point(|p| (p.0 as f64) <= position);
        let a = self.points.get(i.checked_sub(1)?)?;
        let Some(b) = self.points.get(i) else {
            return Some(a.1);
        };
        let span = (b.0 - a.0) as f64;
        if span <= 0.0 {
            return Some(b.1);
        }
        let t = ((position - a.0 as f64) / span) as f32;
        Some(a.1 + (b.1 - a.1) * t)
    }
}

/// A looping list of notes, sorted by start time, plus automation lanes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sequence {
    events: Vec<NoteEvent>,
    length: u64,
    automation: Vec<AutomationLane>,
    loop_start: u64,
    /// Where Stop returns to, when it is not the loop start.
    home: Option<u64>,
    /// Ticks where playback pauses (pause markers), sorted.
    pauses: Vec<u64>,
}

impl Sequence {
    /// Builds a sequence that loops after `length` ticks. Events are sorted;
    /// events starting at or after `length` are dropped. On the same tick and
    /// channel, slide notes come after the notes they bend, lowest key first.
    #[must_use]
    pub fn new(mut events: Vec<NoteEvent>, length: u64) -> Self {
        events.retain(|e| e.start < length);
        events.sort_by_key(|e| (e.start, e.channel, e.slide, e.key));
        Self {
            events,
            length,
            automation: Vec::new(),
            loop_start: 0,
            home: None,
            pauses: Vec::new(),
        }
    }

    /// Makes Stop return to `tick` instead of the loop start (a song that wraps
    /// to a song-loop marker still stops back at its beginning).
    #[must_use]
    pub fn with_home(mut self, tick: u64) -> Self {
        self.home = Some(tick);
        self
    }

    /// Where Stop returns to.
    #[must_use]
    pub fn home(&self) -> u64 {
        self.home.unwrap_or(self.loop_start)
    }

    /// Pauses playback when it reaches any of `ticks`.
    #[must_use]
    pub fn with_pauses(mut self, mut ticks: Vec<u64>) -> Self {
        ticks.sort_unstable();
        ticks.dedup();
        self.pauses = ticks;
        self
    }

    /// The first pause point strictly after `from` and before `to` (ticks).
    /// Playback that starts exactly on a pause point continues past it.
    #[must_use]
    pub fn next_pause(&self, from: f64, to: f64) -> Option<f64> {
        self.pauses
            .iter()
            .map(|&t| t as f64)
            .find(|&t| t > from && t < to)
    }

    /// Restarts the loop at `start` ticks instead of 0 (ignored unless it is
    /// before the end).
    #[must_use]
    pub fn with_loop_start(mut self, start: u64) -> Self {
        self.loop_start = start;
        self
    }

    /// Where playback wraps to at the end.
    #[must_use]
    pub fn loop_start(&self) -> u64 {
        self.loop_start
    }

    /// Adds automation lanes. Points in each lane are sorted by tick.
    #[must_use]
    pub fn with_automation(mut self, mut lanes: Vec<AutomationLane>) -> Self {
        for lane in &mut lanes {
            lane.points.sort_by_key(|p| p.0);
        }
        lanes.retain(|lane| !lane.points.is_empty());
        self.automation = lanes;
        self
    }

    /// The automation lanes.
    #[must_use]
    pub fn automation(&self) -> &[AutomationLane] {
        &self.automation
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
            shape: crate::NoteShape::default(),
            start,
            length: 10,
            channel: 0,
            key: 60,
            velocity: 1.0,
            pan: 0.0,
            slide: false,
        }
    }

    #[test]
    fn pauses_are_found_strictly_inside_a_span() {
        let seq = Sequence::new(vec![note(0)], 1_000).with_pauses(vec![500, 100, 500]);
        assert_eq!(seq.next_pause(0.0, 1_000.0), Some(100.0));
        assert_eq!(
            seq.next_pause(100.0, 1_000.0),
            Some(500.0),
            "starting on one"
        );
        assert_eq!(seq.next_pause(100.0, 500.0), None, "end is exclusive");
        assert_eq!(seq.home(), 0);
        let seq = seq.with_loop_start(200);
        assert_eq!(seq.home(), 200, "Stop returns to the loop start by default");
        assert_eq!(seq.with_home(0).home(), 0);
    }

    #[test]
    fn lanes_interpolate_inside_their_span_only() {
        let lane = AutomationLane {
            target: AutomationTarget::TrackVolume(1),
            points: vec![(100, 0.0), (200, 1.0), (300, 1.0)],
        };
        assert_eq!(lane.value_at(99.0), None);
        assert_eq!(lane.value_at(100.0), Some(0.0));
        assert_eq!(lane.value_at(150.0), Some(0.5));
        assert_eq!(lane.value_at(250.0), Some(1.0));
        assert_eq!(lane.value_at(300.0), Some(1.0));
        assert_eq!(lane.value_at(301.0), None);
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

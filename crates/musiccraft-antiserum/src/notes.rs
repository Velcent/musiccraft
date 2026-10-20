//! The arpeggiator and the step sequencer (`docs/spec/antiserum-fx-arp.md`):
//! held keys go in, timed note events come out. No allocation; times are in
//! samples so steps land on exact samples.

/// Keys the generator remembers.
pub const MAX_HELD: usize = 16;
/// Steps in a [`Sequence`].
pub const SEQ_STEPS: usize = 64;
/// Generated notes sounding at once.
const MAX_SOUNDING: usize = 32;
/// Most events one call to [`Generator::due`] returns.
const MAX_EVENTS: usize = MAX_SOUNDING + MAX_HELD;
/// Most keys in an arpeggio: held keys × 4 octaves.
const MAX_ORDER: usize = MAX_HELD * 4;

/// One step of the sequencer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// Whether the step plays.
    pub on: bool,
    /// Semitones from the held key, −24..24.
    pub transpose: i8,
    /// 0..1, multiplied by the key's velocity.
    pub velocity: f32,
    /// 0.05..1 of a step.
    pub gate: f32,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            on: true,
            transpose: 0,
            velocity: 1.0,
            gate: 0.5,
        }
    }
}

impl Step {
    fn sanitized(self) -> Self {
        let finite = |x: f32, fallback: f32| if x.is_finite() { x } else { fallback };
        Self {
            on: self.on,
            transpose: self.transpose.clamp(-24, 24),
            velocity: finite(self.velocity, 1.0).clamp(0.0, 1.0),
            gate: finite(self.gate, 0.5).clamp(0.05, 1.0),
        }
    }
}

/// The sequencer's pattern: always 64 steps; `seq_length` says how many play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sequence {
    /// The steps, in order.
    pub steps: [Step; SEQ_STEPS],
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            steps: [Step::default(); SEQ_STEPS],
        }
    }
}

/// A note event for the voices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Key and velocity.
    On(u8, f32),
    /// Key.
    Off(u8),
}

/// Events from one call, in the order to apply them.
pub struct Events {
    buf: [Event; MAX_EVENTS],
    len: usize,
}

impl Events {
    fn new() -> Self {
        Self {
            buf: [Event::Off(0); MAX_EVENTS],
            len: 0,
        }
    }

    fn push(&mut self, event: Event) {
        if let Some(slot) = self.buf.get_mut(self.len) {
            *slot = event;
            self.len += 1;
        }
    }

    /// The events.
    pub fn as_slice(&self) -> &[Event] {
        self.buf.get(..self.len).unwrap_or(&[])
    }
}

/// What the generator plays, read from the parameters each step.
#[derive(Debug, Clone, Copy)]
pub enum Pattern {
    /// Arpeggiator: mode 1–4 (up, down, up-down, random), octaves 1–4, gate.
    Arp { mode: u8, octaves: u8, gate: f32 },
    /// Sequencer: steps 1–64.
    Seq { length: usize },
}

/// Timing for the next step.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    /// Samples in one step.
    pub step_samples: f64,
}

/// Held keys in, timed notes out.
pub struct Generator {
    held: [(u8, f32); MAX_HELD],
    held_len: usize,
    running: bool,
    step: u64,
    /// Samples until the next step starts.
    to_next: f64,
    /// Generated notes: key and samples until their note-off.
    sounding: [(u8, f64); MAX_SOUNDING],
    sounding_len: usize,
    sequence: Sequence,
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            held: [(0, 0.0); MAX_HELD],
            held_len: 0,
            running: false,
            step: 0,
            to_next: 0.0,
            sounding: [(0, 0.0); MAX_SOUNDING],
            sounding_len: 0,
            sequence: Sequence::default(),
        }
    }
}

impl Generator {
    /// The sequencer's pattern.
    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    /// Replaces the sequencer's pattern (values are clamped into range).
    pub fn set_sequence(&mut self, sequence: &Sequence) {
        for (out, step) in self.sequence.steps.iter_mut().zip(sequence.steps) {
            *out = step.sanitized();
        }
    }

    fn held(&self) -> &[(u8, f32)] {
        self.held.get(..self.held_len).unwrap_or(&[])
    }

    /// Whether `key` is sounding as a generated note.
    pub fn is_sounding(&self, key: u8) -> bool {
        self.sounding
            .get(..self.sounding_len)
            .is_some_and(|s| s.iter().any(|(k, _)| *k == key))
    }

    /// A key went down. Starts the pattern when it is the first.
    pub fn press(&mut self, key: u8, velocity: f32) {
        self.remove_held(key);
        if self.held_len == MAX_HELD {
            self.held.copy_within(1.., 0);
            self.held_len -= 1;
        }
        if let Some(slot) = self.held.get_mut(self.held_len) {
            *slot = (key, velocity);
            self.held_len += 1;
        }
        if !self.running {
            self.running = true;
            self.step = 0;
            self.to_next = 0.0;
        }
    }

    /// A key went up. Releasing the last one stops the pattern and returns
    /// the note-offs of what was sounding.
    pub fn release(&mut self, key: u8) -> Events {
        self.remove_held(key);
        if self.held_len == 0 {
            self.stop()
        } else {
            Events::new()
        }
    }

    /// Stops and forgets the held keys, returning note-offs for every
    /// generated note.
    pub fn stop(&mut self) -> Events {
        let mut events = Events::new();
        for &(key, _) in self.sounding.get(..self.sounding_len).unwrap_or(&[]) {
            events.push(Event::Off(key));
        }
        self.sounding_len = 0;
        self.held_len = 0;
        self.running = false;
        events
    }

    /// Whether a pattern runs or notes are still to be released.
    pub fn busy(&self) -> bool {
        self.running || self.sounding_len > 0
    }

    fn remove_held(&mut self, key: u8) {
        let len = self.held_len;
        if let Some(i) = self
            .held
            .get(..len)
            .and_then(|h| h.iter().position(|(k, _)| *k == key))
        {
            self.held.copy_within(i + 1..len, i);
            self.held_len -= 1;
        }
    }

    /// Samples until the next event (a step or a note-off), if any.
    pub fn until_next(&self) -> Option<f64> {
        let offs = self
            .sounding
            .get(..self.sounding_len)
            .unwrap_or(&[])
            .iter()
            .map(|(_, t)| *t);
        let step = self.running.then_some(self.to_next);
        offs.chain(step).reduce(f64::min)
    }

    /// Moves time on by `samples`.
    pub fn advance(&mut self, samples: f64) {
        if self.running {
            self.to_next -= samples;
        }
        for (_, t) in self.sounding.iter_mut().take(self.sounding_len) {
            *t -= samples;
        }
    }

    /// The events due now: note-offs first, then the step's notes.
    pub fn due(&mut self, pattern: Pattern, clock: Clock) -> Events {
        let mut events = Events::new();
        let mut i = 0;
        while i < self.sounding_len {
            match self.sounding.get(i) {
                Some(&(key, t)) if t <= 0.0 => {
                    events.push(Event::Off(key));
                    self.sounding.copy_within(i + 1..self.sounding_len, i);
                    self.sounding_len -= 1;
                }
                _ => i += 1,
            }
        }
        if self.running && self.to_next <= 0.0 && self.held_len > 0 {
            let step_samples = clock.step_samples.max(1.0);
            match pattern {
                Pattern::Arp {
                    mode,
                    octaves,
                    gate,
                } => self.arp_step(mode, octaves, gate, step_samples, &mut events),
                Pattern::Seq { length } => self.seq_step(length, step_samples, &mut events),
            }
            self.step += 1;
            self.to_next += step_samples;
            // After a long stall (a tempo change), don't fire a burst.
            if self.to_next <= 0.0 {
                self.to_next = step_samples;
            }
        }
        events
    }

    fn start_note(&mut self, key: u8, velocity: f32, length: f64, events: &mut Events) {
        if self.sounding_len == MAX_SOUNDING || velocity <= 0.0 {
            return;
        }
        // A key still sounding from an earlier step ends first.
        if let Some(i) = self
            .sounding
            .get(..self.sounding_len)
            .and_then(|s| s.iter().position(|(k, _)| *k == key))
        {
            events.push(Event::Off(key));
            self.sounding.copy_within(i + 1..self.sounding_len, i);
            self.sounding_len -= 1;
        }
        events.push(Event::On(key, velocity));
        if let Some(slot) = self.sounding.get_mut(self.sounding_len) {
            *slot = (key, length);
            self.sounding_len += 1;
        }
    }

    fn arp_step(
        &mut self,
        mode: u8,
        octaves: u8,
        gate: f32,
        step_samples: f64,
        events: &mut Events,
    ) {
        let mut order = [(0_u8, 0.0_f32); MAX_ORDER];
        let mut len = 0;
        for octave in 0..octaves.clamp(1, 4) {
            for &(key, velocity) in self.held() {
                let Some(k) = key.checked_add(12 * octave).filter(|k| *k <= 127) else {
                    continue;
                };
                if let Some(slot) = order.get_mut(len) {
                    *slot = (k, velocity);
                    len += 1;
                }
            }
        }
        let keys = order.get_mut(..len).unwrap_or(&mut []);
        keys.sort_unstable_by_key(|(k, _)| *k);
        // Remove repeated keys (sorted, so they are neighbours).
        let mut unique = 0_usize;
        for i in 0..keys.len() {
            let current = keys.get(i).copied();
            let previous = unique.checked_sub(1).and_then(|p| keys.get(p)).copied();
            if let Some(entry) = current
                && previous.is_none_or(|p| p.0 != entry.0)
            {
                if let Some(slot) = keys.get_mut(unique) {
                    *slot = entry;
                }
                unique += 1;
            }
        }
        if unique == 0 {
            return;
        }
        let n = unique as u64;
        let index = match mode {
            2 => (n - 1 - self.step % n) as usize,
            3 if n > 2 => {
                let cycle = 2 * n - 2;
                let pos = self.step % cycle;
                (if pos < n { pos } else { cycle - pos }) as usize
            }
            4 => (hash(self.step) % n) as usize,
            _ => (self.step % n) as usize,
        };
        if let Some(&(key, velocity)) = keys.get(index) {
            let length = step_samples * f64::from(gate.clamp(0.05, 1.0));
            self.start_note(key, velocity, length, events);
        }
    }

    fn seq_step(&mut self, length: usize, step_samples: f64, events: &mut Events) {
        let length = length.clamp(1, SEQ_STEPS) as u64;
        let Some(&step) = self.sequence.steps.get((self.step % length) as usize) else {
            return;
        };
        if !step.on {
            return;
        }
        let held = self.held;
        for &(key, velocity) in held.get(..self.held_len).unwrap_or(&[]) {
            let note = i16::from(key) + i16::from(step.transpose);
            let Ok(note) = u8::try_from(note) else {
                continue;
            };
            if note > 127 {
                continue;
            }
            let length = step_samples * f64::from(step.gate);
            self.start_note(note, velocity * step.velocity, length, events);
        }
    }
}

/// A stable pseudo-random number for a step, so random order repeats.
fn hash(step: u64) -> u64 {
    let mut x = step.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

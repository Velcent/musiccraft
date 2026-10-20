//! Antiserum channels in the engine (`docs/spec/antiserum-app.md`): the synth
//! wrapped as an [`External`] processor, and the data the bridge swaps into
//! it without freeing memory on the audio thread.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use musiccraft_antiserum::{Antiserum, Source};
use musiccraft_engine::{External, ExternalTransport, MAX_BLOCK};
use musiccraft_patch::Patch;

/// Notes waiting for their frame.
const MAX_PENDING: usize = 128;
/// Notes the processor ends itself (sequence notes with a gate).
const MAX_GATES: usize = 128;

/// Data the bridge sends to a running synth.
pub(crate) use musiccraft_antiserum::Data as Swap;

#[derive(Debug, Clone, Copy)]
enum Pending {
    On(u8, f32),
    Off(u8),
}

/// One Antiserum, as the engine runs it.
pub(crate) struct AntiserumExternal {
    synth: Antiserum,
    /// Events and the frame (from the start of the next block) they fall on.
    pending: [(u32, Pending); MAX_PENDING],
    pending_len: usize,
    /// Keys to release and frames until then.
    gates: [(u8, u64); MAX_GATES],
    gates_len: usize,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl fmt::Debug for AntiserumExternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AntiserumExternal")
            .field("pending", &self.pending_len)
            .field("gates", &self.gates_len)
            .finish_non_exhaustive()
    }
}

impl AntiserumExternal {
    /// A synth playing `patch` with the oscillators' `sources` (built by the
    /// bridge, which resolves sample paths), and warnings for tables that
    /// failed. Main thread (allocates).
    pub(crate) fn new(
        patch: &Patch,
        sample_rate: f32,
        sources: Vec<Arc<Source>>,
    ) -> (Self, Vec<String>) {
        let mut synth = Antiserum::new(sample_rate);
        let warnings = musiccraft_antiserum_presets::apply(patch, &mut synth);
        for (osc, source) in sources.into_iter().enumerate() {
            drop(synth.set_source(osc, source));
        }
        (
            Self {
                synth,
                pending: [(0, Pending::Off(0)); MAX_PENDING],
                pending_len: 0,
                gates: [(0, 0); MAX_GATES],
                gates_len: 0,
                left: vec![0.0; MAX_BLOCK],
                right: vec![0.0; MAX_BLOCK],
            },
            warnings,
        )
    }

    fn push(&mut self, delay: u32, event: Pending) {
        if let Some(slot) = self.pending.get_mut(self.pending_len) {
            *slot = (delay, event);
            self.pending_len += 1;
        }
    }

    /// Plays every event and gate due at `frame` of this block.
    fn fire(&mut self, frame: u64) {
        let mut i = 0;
        while i < self.gates_len {
            match self.gates.get(i) {
                Some(&(key, at)) if at <= frame => {
                    self.synth.note_off(key);
                    self.gates.copy_within(i + 1..self.gates_len, i);
                    self.gates_len -= 1;
                }
                _ => i += 1,
            }
        }
        let mut i = 0;
        while i < self.pending_len {
            match self.pending.get(i) {
                Some(&(at, event)) if u64::from(at) <= frame => {
                    match event {
                        Pending::On(key, velocity) => self.synth.note_on(key, velocity),
                        Pending::Off(key) => self.synth.note_off(key),
                    }
                    self.pending.copy_within(i + 1..self.pending_len, i);
                    self.pending_len -= 1;
                }
                _ => i += 1,
            }
        }
    }

    /// The first frame after `frame` with something to do, or `end`.
    fn next(&self, frame: u64, end: u64) -> u64 {
        let events = self
            .pending
            .iter()
            .take(self.pending_len)
            .map(|(at, _)| u64::from(*at));
        let gates = self.gates.iter().take(self.gates_len).map(|(_, at)| *at);
        events
            .chain(gates)
            .filter(|at| *at > frame)
            .fold(end, u64::min)
    }
}

impl External for AntiserumExternal {
    fn note_on(&mut self, delay: u32, key: u8, velocity: f32, _pan: f32, gate: Option<u64>) {
        self.push(delay, Pending::On(key, velocity));
        if let Some(gate) = gate
            && let Some(slot) = self.gates.get_mut(self.gates_len)
        {
            *slot = (key, u64::from(delay) + gate.max(1));
            self.gates_len += 1;
        }
    }

    fn note_off(&mut self, delay: u32, key: u8) {
        self.push(delay, Pending::Off(key));
    }

    fn all_notes_off(&mut self) {
        self.pending_len = 0;
        self.gates_len = 0;
        self.synth.all_notes_off();
    }

    fn set_param(&mut self, id: u32, value: f64) {
        self.synth.set_param(id as usize, value as f32);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], transport: &ExternalTransport) {
        self.synth.set_tempo(transport.tempo_bpm as f32);
        if transport.playing {
            self.synth.set_beat_position(transport.position_beats);
        }
        let frames = left.len().min(right.len()).min(self.left.len());
        let end = frames as u64;
        let mut frame = 0;
        while frame < end {
            self.fire(frame);
            let next = self.next(frame, end);
            let (from, to) = (frame as usize, next as usize);
            if let (Some(l), Some(r)) = (self.left.get_mut(from..to), self.right.get_mut(from..to))
            {
                self.synth.process(l, r);
            }
            frame = next;
        }
        for (out, x) in left.iter_mut().zip(self.left.iter().take(frames)) {
            *out += x;
        }
        for (out, x) in right.iter_mut().zip(self.right.iter().take(frames)) {
            *out += x;
        }
        // What is left moves to the next block.
        for (at, _) in self.pending.iter_mut().take(self.pending_len) {
            *at = at.saturating_sub(frames as u32);
        }
        for (_, at) in self.gates.iter_mut().take(self.gates_len) {
            *at = at.saturating_sub(end);
        }
    }

    fn stop(&mut self) {}

    fn receive(&mut self, mut data: Box<dyn Any + Send>) -> Box<dyn Any + Send> {
        if let Some(swap) = data.downcast_mut::<Swap>() {
            // The box takes the old data back; nothing is freed here.
            self.synth.swap(swap);
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transport() -> ExternalTransport {
        ExternalTransport {
            playing: false,
            tempo_bpm: 120.0,
            position_beats: 0.0,
            time_signature: musiccraft_meter::TimeSig::default(),
            bar_start_beats: 0.0,
            bar_number: 0,
            sample_rate: 48_000.0,
        }
    }

    #[test]
    fn notes_start_on_their_frame_and_gates_end_them() {
        let patch = musiccraft_antiserum_presets::find("Deep Sub").unwrap();
        let (mut synth, warnings) = AntiserumExternal::new(patch, 48_000.0, Vec::new());
        assert!(warnings.is_empty());
        synth.note_on(100, 48, 1.0, 0.0, Some(200));
        let mut l = vec![0.0; 256];
        let mut r = vec![0.0; 256];
        synth.process(&mut l, &mut r, &transport());
        assert!(l[..100].iter().all(|x| *x == 0.0), "silent before the note");
        assert!(l[100..].iter().any(|x| *x != 0.0));
        assert_eq!(synth.gates_len, 1, "the gate carries into the next block");
        let mut l2 = vec![0.0; 256];
        let mut r2 = vec![0.0; 256];
        synth.process(&mut l2, &mut r2, &transport());
        assert_eq!(synth.gates_len, 0, "released at frame 300");
    }

    #[test]
    fn swapped_data_comes_back_holding_what_it_replaced() {
        let (mut synth, _) = AntiserumExternal::new(&Patch::default(), 48_000.0, Vec::new());
        let new = musiccraft_antiserum_presets::wavetable("factory:pwm").unwrap();
        let old = Arc::clone(synth.synth.wavetable(1).unwrap());
        let back = synth.receive(Box::new(Swap::Table(1, Arc::clone(&new))));
        let back = back.downcast::<Swap>().unwrap();
        assert!(matches!(&*back, Swap::Table(1, t) if Arc::ptr_eq(t, &old)));
        assert!(Arc::ptr_eq(synth.synth.wavetable(1).unwrap(), &new));
        // Other data passes through untouched.
        let other = synth.receive(Box::new(5_u32));
        assert_eq!(other.downcast::<u32>().ok().map(|b| *b), Some(5));
    }
}

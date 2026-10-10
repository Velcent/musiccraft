//! Real-time audio engine.
//!
//! The engine is driven entirely by plain [`EngineMessage`] values and never
//! reads the project model. Everything in [`Engine::process`] must stay free of
//! allocation, locks, I/O, and panics.

/// A fixed-size, allocation-free message sent from the main thread to the engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineMessage {
    /// Start advancing the playhead.
    Play,
    /// Halt the playhead, keeping its position.
    Stop,
    /// Change the tempo, in beats per minute. Non-finite or non-positive
    /// values are ignored.
    SetTempo(f64),
}

/// The audio engine state owned by the audio thread.
#[derive(Debug, Clone)]
pub struct Engine {
    sample_rate: f64,
    tempo_bpm: f64,
    playing: bool,
    position_beats: f64,
    frames_played: u64,
}

impl Engine {
    /// Creates a stopped engine at 130 BPM for the given sample rate in hertz.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate.max(1)),
            tempo_bpm: 130.0,
            playing: false,
            position_beats: 0.0,
            frames_played: 0,
        }
    }

    /// Applies a message from the main thread.
    pub fn apply(&mut self, message: EngineMessage) {
        match message {
            EngineMessage::Play => self.playing = true,
            EngineMessage::Stop => self.playing = false,
            EngineMessage::SetTempo(bpm) => {
                if bpm.is_finite() && bpm > 0.0 {
                    self.tempo_bpm = bpm;
                }
            }
        }
    }

    /// Fills an interleaved output buffer and advances the playhead.
    ///
    /// For now the engine renders silence; it only tracks time. `channels` is the
    /// number of interleaved channels in `output`.
    pub fn process(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);
        if channels == 0 || !self.playing {
            return;
        }
        let frames = output.len() / channels;
        self.position_beats += frames as f64 * self.tempo_bpm / 60.0 / self.sample_rate;
        self.frames_played += frames as u64;
    }

    /// Whether the playhead is advancing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Current tempo in beats per minute.
    #[must_use]
    pub fn tempo_bpm(&self) -> f64 {
        self.tempo_bpm
    }

    /// Playhead position in beats.
    #[must_use]
    pub fn position_beats(&self) -> f64 {
        self.position_beats
    }

    /// Total frames rendered while playing.
    #[must_use]
    pub fn frames_played(&self) -> u64 {
        self.frames_played
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_at_120() -> Engine {
        let mut engine = Engine::new(44_100);
        engine.apply(EngineMessage::SetTempo(120.0));
        engine
    }

    #[test]
    fn stopped_engine_is_silent_and_does_not_advance() {
        let mut engine = engine_at_120();
        let mut buffer = vec![1.0_f32; 512];
        engine.process(&mut buffer, 2);
        assert!(buffer.iter().all(|s| *s == 0.0));
        assert_eq!(engine.frames_played(), 0);
        assert!(engine.position_beats().abs() < f64::EPSILON);
    }

    #[test]
    fn one_second_at_120_bpm_is_two_beats() {
        let mut engine = engine_at_120();
        engine.apply(EngineMessage::Play);
        let mut buffer = vec![0.0_f32; 88_200];
        engine.process(&mut buffer, 2);
        assert_eq!(engine.frames_played(), 44_100);
        assert!((engine.position_beats() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn tempo_change_does_not_make_position_jump() {
        let mut engine = engine_at_120();
        engine.apply(EngineMessage::Play);
        let mut buffer = vec![0.0_f32; 88_200];
        engine.process(&mut buffer, 2);
        let before = engine.position_beats();
        engine.apply(EngineMessage::SetTempo(60.0));
        assert!((engine.position_beats() - before).abs() < f64::EPSILON);
        engine.process(&mut buffer, 2);
        assert!((engine.position_beats() - (before + 1.0)).abs() < 1e-9);
    }

    #[test]
    fn stop_keeps_position() {
        let mut engine = engine_at_120();
        engine.apply(EngineMessage::Play);
        let mut buffer = vec![0.0_f32; 44_100];
        engine.process(&mut buffer, 1);
        let position = engine.position_beats();
        engine.apply(EngineMessage::Stop);
        engine.process(&mut buffer, 1);
        assert!((engine.position_beats() - position).abs() < f64::EPSILON);
    }

    #[test]
    fn invalid_tempo_messages_are_ignored() {
        let mut engine = engine_at_120();
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            engine.apply(EngineMessage::SetTempo(bad));
        }
        assert!((engine.tempo_bpm() - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_channels_does_not_panic() {
        let mut engine = engine_at_120();
        engine.apply(EngineMessage::Play);
        let mut buffer = vec![0.0_f32; 64];
        engine.process(&mut buffer, 0);
        assert_eq!(engine.frames_played(), 0);
    }
}

//! The editable table: whole frames of [`FRAME_LEN`] samples.

/// Samples in one frame (the synth's frame length).
pub const FRAME_LEN: usize = 2048;
/// Most frames a table holds (the synth's limit).
pub const MAX_FRAMES: usize = 256;

/// 1 to [`MAX_FRAMES`] frames of [`FRAME_LEN`] finite samples, end to end.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    data: Vec<f32>,
}

impl Default for Table {
    fn default() -> Self {
        Self::silent()
    }
}

impl Table {
    /// A table from frames laid end to end. A partial last frame and frames
    /// past [`MAX_FRAMES`] are dropped; non-finite samples become 0. `None`
    /// if there is no whole frame.
    #[must_use]
    pub fn new(samples: &[f32]) -> Option<Self> {
        let frames = (samples.len() / FRAME_LEN).min(MAX_FRAMES);
        if frames == 0 {
            return None;
        }
        let data = samples
            .iter()
            .take(frames * FRAME_LEN)
            .map(|&x| if x.is_finite() { x } else { 0.0 })
            .collect();
        Some(Self { data })
    }

    /// A table of `frames` (clamped to 1..=256) from a function of
    /// (frame position `y` 0..1, phase `x` 0..1).
    #[must_use]
    pub fn from_fn(frames: usize, f: impl Fn(f32, f32) -> f32) -> Self {
        let frames = frames.clamp(1, MAX_FRAMES);
        let mut data = Vec::with_capacity(frames * FRAME_LEN);
        for frame in 0..frames {
            let y = position(frame, frames);
            data.extend((0..FRAME_LEN).map(|i| f(y, i as f32 / FRAME_LEN as f32)));
        }
        Self::new(&data).unwrap_or_default()
    }

    /// One silent frame.
    #[must_use]
    pub fn silent() -> Self {
        Self {
            data: vec![0.0; FRAME_LEN],
        }
    }

    /// Number of frames (at least 1).
    #[must_use]
    pub fn frames(&self) -> usize {
        self.data.len() / FRAME_LEN
    }

    /// Frame `i`, if it exists.
    #[must_use]
    pub fn frame(&self, i: usize) -> Option<&[f32]> {
        self.data.get(i * FRAME_LEN..(i + 1) * FRAME_LEN)
    }

    /// Every sample, frame after frame: what `Wavetable::new` takes.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.data
    }

    pub(crate) fn frame_mut(&mut self, i: usize) -> Option<&mut [f32]> {
        self.data.get_mut(i * FRAME_LEN..(i + 1) * FRAME_LEN)
    }

    /// Replaces frame `i` with `samples` (made finite). Unchanged when `i` is
    /// out of range.
    pub(crate) fn set_frame(&mut self, i: usize, samples: &[f32]) {
        if let Some(frame) = self.frame_mut(i) {
            for (out, &x) in frame.iter_mut().zip(samples) {
                *out = if x.is_finite() { x } else { 0.0 };
            }
        }
    }

    /// Inserts a copy of frame `at` after it.
    pub(crate) fn duplicate_frame(&mut self, at: usize) {
        if let Some(copy) = self.frame(at).map(<[f32]>::to_vec) {
            let end = (at + 1) * FRAME_LEN;
            self.data.splice(end..end, copy);
        }
    }

    pub(crate) fn remove_frame(&mut self, at: usize) {
        if at < self.frames() {
            self.data.drain(at * FRAME_LEN..(at + 1) * FRAME_LEN);
        }
    }

    pub(crate) fn reverse_frames(&mut self) {
        let frames: Vec<Vec<f32>> = self
            .data
            .chunks_exact(FRAME_LEN)
            .rev()
            .map(<[f32]>::to_vec)
            .collect();
        self.data = frames.concat();
    }
}

/// Position 0..1 of frame `i` of `frames` (0 for a single frame).
#[must_use]
pub(crate) fn position(i: usize, frames: usize) -> f32 {
    if frames > 1 {
        i as f32 / (frames - 1) as f32
    } else {
        0.0
    }
}

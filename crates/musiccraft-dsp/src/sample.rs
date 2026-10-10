/// Decoded audio held in memory, stored as separate left and right channels.
/// Mono material has identical channels.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleBuffer {
    left: Vec<f32>,
    right: Vec<f32>,
    sample_rate: f64,
}

impl SampleBuffer {
    /// Builds a stereo buffer. If the channels differ in length the longer one is
    /// truncated.
    #[must_use]
    pub fn stereo(mut left: Vec<f32>, mut right: Vec<f32>, sample_rate: f64) -> Self {
        let len = left.len().min(right.len());
        left.truncate(len);
        right.truncate(len);
        Self {
            left,
            right,
            sample_rate: sample_rate.max(1.0),
        }
    }

    /// Builds a buffer from mono data.
    #[must_use]
    pub fn mono(data: Vec<f32>, sample_rate: f64) -> Self {
        Self::stereo(data.clone(), data, sample_rate)
    }

    /// An empty buffer.
    #[must_use]
    pub fn silent(sample_rate: f64) -> Self {
        Self::stereo(Vec::new(), Vec::new(), sample_rate)
    }

    /// Length in frames.
    #[must_use]
    pub fn len(&self) -> usize {
        self.left.len()
    }

    /// Whether the buffer holds no frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left.is_empty()
    }

    /// Native sample rate in hertz.
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Duration in seconds.
    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.len() as f64 / self.sample_rate
    }

    /// The left channel.
    #[must_use]
    pub fn left(&self) -> &[f32] {
        &self.left
    }

    /// The right channel.
    #[must_use]
    pub fn right(&self) -> &[f32] {
        &self.right
    }

    /// Linearly interpolated stereo frame at a fractional position. Positions
    /// outside the buffer read as silence.
    #[must_use]
    pub fn read(&self, position: f64) -> (f32, f32) {
        if position.is_nan() || position < 0.0 {
            return (0.0, 0.0);
        }
        let index = position as usize;
        let frac = (position - index as f64) as f32;
        let at = |data: &[f32], i: usize| data.get(i).copied().unwrap_or(0.0);
        let l = at(&self.left, index) + (at(&self.left, index + 1) - at(&self.left, index)) * frac;
        let r =
            at(&self.right, index) + (at(&self.right, index + 1) - at(&self.right, index)) * frac;
        (l, r)
    }

    /// Largest absolute sample value.
    #[must_use]
    pub fn peak(&self) -> f32 {
        self.left
            .iter()
            .chain(&self.right)
            .fold(0.0, |peak, s| peak.max(s.abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_between_frames() {
        let buffer = SampleBuffer::mono(vec![0.0, 1.0, 0.0], 10.0);
        assert_eq!(buffer.read(0.5), (0.5, 0.5));
        assert_eq!(buffer.read(1.0), (1.0, 1.0));
        assert_eq!(buffer.read(2.5), (0.0, 0.0));
        assert_eq!(buffer.read(99.0), (0.0, 0.0));
        assert_eq!(buffer.read(-1.0), (0.0, 0.0));
        assert_eq!(buffer.read(f64::NAN), (0.0, 0.0));
        assert!((buffer.duration_seconds() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn mismatched_channels_are_truncated() {
        let buffer = SampleBuffer::stereo(vec![1.0; 4], vec![1.0; 2], 44_100.0);
        assert_eq!(buffer.len(), 2);
        assert!((buffer.peak() - 1.0).abs() < f32::EPSILON);
    }
}

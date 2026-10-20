//! Reading a signal between its samples (cubic Hermite / Catmull-Rom).

use crate::FRAME_LEN;

/// The sample at fractional `pos`. With `wrap`, the signal is one cycle and
/// indices wrap around; otherwise they stop at its ends.
pub(crate) fn read(signal: &[f32], pos: f64, wrap: bool) -> f32 {
    let n = signal.len();
    if n == 0 {
        return 0.0;
    }
    let floor = pos.floor();
    let t = (pos - floor) as f32;
    let at = |offset: i64| {
        let i = floor as i64 + offset;
        let i = if wrap {
            i.rem_euclid(n as i64)
        } else {
            i.clamp(0, n as i64 - 1)
        };
        signal.get(i as usize).copied().unwrap_or(0.0)
    };
    let (y0, y1, y2, y3) = (at(-1), at(0), at(1), at(2));
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

/// `len` samples of `signal` from `start`, stretched to one frame.
pub(crate) fn frame(signal: &[f32], start: f64, len: f64, wrap: bool) -> Vec<f32> {
    let step = len / FRAME_LEN as f64;
    (0..FRAME_LEN)
        .map(|i| read(signal, start + i as f64 * step, wrap))
        .collect()
}

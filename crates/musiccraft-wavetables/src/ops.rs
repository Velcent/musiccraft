//! Editing operations: plain descriptions of one change each.

use musiccraft_formula::Formula;

use crate::spectrum::{self, HARMONICS, Partial};
use crate::table::position;
use crate::{FRAME_LEN, MAX_FRAMES, Table};

/// How [`Op::Morph`] blends two frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MorphMode {
    /// Sample by sample.
    #[default]
    Crossfade,
    /// Partial by partial: magnitudes linearly, phases the shorter way round.
    Spectral,
}

/// One change to a [`Table`]. Frames are 0-based and ranges inclusive.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// The whole table.
    Replace(Table),
    /// Writes `values` into `frame` from sample `start`, wrapping.
    Draw {
        frame: usize,
        start: usize,
        values: Vec<f32>,
    },
    /// Rebuilds `frame` with harmonic `harmonic` (1–1023) changed.
    SetPartial {
        frame: usize,
        harmonic: usize,
        magnitude: f32,
        phase: f32,
    },
    /// Fills frames `first..=last` from a formula of `x` and `y`.
    Formula {
        text: String,
        first: usize,
        last: usize,
    },
    /// Rewrites the frames strictly between `first` and `last` as a morph.
    Morph {
        first: usize,
        last: usize,
        mode: MorphMode,
    },
    /// Scales each frame of the range to a peak of 1.
    Normalize { first: usize, last: usize },
    /// Subtracts each frame's mean.
    RemoveDc { first: usize, last: usize },
    /// Plays each frame of the range backwards.
    ReverseSamples { first: usize, last: usize },
    /// Reverses the order of the frames.
    ReverseFrames,
    /// Inserts a copy of frame `at` after it.
    InsertFrame { at: usize },
    /// Removes frame `at` (never the last frame left).
    DeleteFrame { at: usize },
}

impl Op {
    /// Applies the change. On an error the table is untouched.
    ///
    /// # Errors
    ///
    /// An out-of-range frame or harmonic, a bad formula, a table that is
    /// full (insert) or has one frame (delete).
    pub fn apply(&self, table: &mut Table) -> Result<(), String> {
        match self {
            Self::Replace(new) => *table = new.clone(),
            Self::Draw {
                frame,
                start,
                values,
            } => {
                let samples = table.frame_mut(*frame).ok_or_else(|| no_frame(*frame))?;
                for (i, &v) in values.iter().take(FRAME_LEN).enumerate() {
                    if let Some(s) = samples.get_mut((start + i) % FRAME_LEN) {
                        *s = if v.is_finite() {
                            v.clamp(-1.0, 1.0)
                        } else {
                            0.0
                        };
                    }
                }
            }
            Self::SetPartial {
                frame,
                harmonic,
                magnitude,
                phase,
            } => set_partial(table, *frame, *harmonic, *magnitude, *phase)?,
            Self::Formula { text, first, last } => {
                let formula = Formula::parse_table(text)?;
                let (first, last) = range(table, *first, *last)?;
                let count = last - first + 1;
                for (k, frame) in (first..=last).enumerate() {
                    let y = position(k, count);
                    let samples: Vec<f32> = (0..FRAME_LEN)
                        .map(|i| formula.eval_table(i as f32 / FRAME_LEN as f32, y))
                        .collect();
                    table.set_frame(frame, &samples);
                }
            }
            Self::Morph { first, last, mode } => morph(table, *first, *last, *mode)?,
            Self::Normalize { first, last } => {
                each_frame(table, *first, *last, normalize)?;
            }
            Self::RemoveDc { first, last } => each_frame(table, *first, *last, |f| {
                let mean = f.iter().sum::<f32>() / f.len().max(1) as f32;
                for x in f.iter_mut() {
                    *x -= mean;
                }
            })?,
            Self::ReverseSamples { first, last } => {
                each_frame(table, *first, *last, <[f32]>::reverse)?;
            }
            Self::ReverseFrames => table.reverse_frames(),
            Self::InsertFrame { at } => {
                if table.frame(*at).is_none() {
                    return Err(no_frame(*at));
                }
                if table.frames() >= MAX_FRAMES {
                    return Err(format!("a table holds at most {MAX_FRAMES} frames"));
                }
                table.duplicate_frame(*at);
            }
            Self::DeleteFrame { at } => {
                if table.frame(*at).is_none() {
                    return Err(no_frame(*at));
                }
                if table.frames() == 1 {
                    return Err("a table keeps at least one frame".to_owned());
                }
                table.remove_frame(*at);
            }
        }
        Ok(())
    }
}

fn no_frame(frame: usize) -> String {
    format!("there is no frame {}", frame + 1)
}

/// The range, checked and in order.
fn range(table: &Table, first: usize, last: usize) -> Result<(usize, usize), String> {
    let (first, last) = (first.min(last), first.max(last));
    if last >= table.frames() {
        return Err(no_frame(last));
    }
    Ok((first, last))
}

fn each_frame(
    table: &mut Table,
    first: usize,
    last: usize,
    f: impl Fn(&mut [f32]),
) -> Result<(), String> {
    let (first, last) = range(table, first, last)?;
    for frame in first..=last {
        if let Some(samples) = table.frame_mut(frame) {
            f(samples);
        }
    }
    Ok(())
}

/// Scales to a peak of 1; a silent frame stays silent.
pub(crate) fn normalize(frame: &mut [f32]) {
    let peak = frame.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
    if peak > 1e-9 {
        for x in frame.iter_mut() {
            *x /= peak;
        }
    }
}

fn set_partial(
    table: &mut Table,
    frame: usize,
    harmonic: usize,
    magnitude: f32,
    phase: f32,
) -> Result<(), String> {
    if !(1..=HARMONICS).contains(&harmonic) {
        return Err(format!("harmonics run from 1 to {HARMONICS}"));
    }
    let samples = table.frame(frame).ok_or_else(|| no_frame(frame))?;
    let mut bins = spectrum::transform(samples);
    let partial = Partial {
        magnitude: if magnitude.is_finite() {
            magnitude.max(0.0)
        } else {
            0.0
        },
        phase: if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.0
        },
    };
    if let Some(bin) = bins.get_mut(harmonic) {
        *bin = spectrum::bin_of(partial);
    }
    let rebuilt = spectrum::inverse(bins);
    table.set_frame(frame, &rebuilt);
    Ok(())
}

fn morph(table: &mut Table, first: usize, last: usize, mode: MorphMode) -> Result<(), String> {
    let (first, last) = range(table, first, last)?;
    if last - first < 2 {
        return Ok(());
    }
    let a = table.frame(first).map(<[f32]>::to_vec).unwrap_or_default();
    let b = table.frame(last).map(<[f32]>::to_vec).unwrap_or_default();
    let (pa, pb) = match mode {
        MorphMode::Crossfade => (Vec::new(), Vec::new()),
        MorphMode::Spectral => (spectrum::partials(&a), spectrum::partials(&b)),
    };
    // DC blends linearly in both modes.
    let mean = |f: &[f32]| f.iter().sum::<f32>() / FRAME_LEN as f32;
    let (dc_a, dc_b) = (mean(&a), mean(&b));
    for frame in first + 1..last {
        let t = (frame - first) as f32 / (last - first) as f32;
        let samples: Vec<f32> = match mode {
            MorphMode::Crossfade => a.iter().zip(&b).map(|(x, y)| x + (y - x) * t).collect(),
            MorphMode::Spectral => {
                let partials: Vec<Partial> = pa
                    .iter()
                    .zip(&pb)
                    .map(|(p, q)| Partial {
                        magnitude: p.magnitude + (q.magnitude - p.magnitude) * t,
                        phase: p.phase + shortest_turn(p.phase, q.phase) * t,
                    })
                    .collect();
                let dc = dc_a + (dc_b - dc_a) * t;
                spectrum::from_partials(&partials)
                    .into_iter()
                    .map(|x| x + dc)
                    .collect()
            }
        };
        table.set_frame(frame, &samples);
    }
    Ok(())
}

/// The signed distance from phase `a` to `b` in turns, between -0.5 and 0.5.
fn shortest_turn(a: f32, b: f32) -> f32 {
    (b - a + 0.5).rem_euclid(1.0) - 0.5
}

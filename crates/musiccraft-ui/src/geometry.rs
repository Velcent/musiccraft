//! Pure layout maths shared by the piano roll and playlist editors.

use egui::{Pos2, Rect, pos2};
use musiccraft_project::Meter;
use musiccraft_project::params::TICKS_PER_STEP;

/// Grid resolution for placing and moving things in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Snap {
    /// No snapping (single ticks).
    Off,
    /// A sixth of a step (triplet 1/64 notes).
    SixthStep,
    /// A quarter of a step (1/64 note).
    QuarterStep,
    /// A third of a step (1/48 note, triplet 1/32s).
    ThirdStep,
    /// Half a step (1/32 note).
    HalfStep,
    /// One step (1/16 note).
    Step,
    /// A third of a beat (triplet eighths in 4/4).
    ThirdBeat,
    /// Half a beat (eighths in 4/4).
    HalfBeat,
    /// One beat of the time signature.
    Beat,
    /// One bar of the time signature.
    Bar,
}

impl Snap {
    /// Every option, finest first.
    pub const ALL: [Self; 10] = [
        Self::Off,
        Self::SixthStep,
        Self::QuarterStep,
        Self::ThirdStep,
        Self::HalfStep,
        Self::Step,
        Self::ThirdBeat,
        Self::HalfBeat,
        Self::Beat,
        Self::Bar,
    ];

    /// Grid size in ticks at `tick` of `meter` (beats and bars vary with the
    /// time signature).
    #[must_use]
    pub fn ticks_at(self, tick: u32, meter: &Meter) -> u32 {
        match self {
            Self::Off => 1,
            Self::SixthStep => TICKS_PER_STEP / 6,
            Self::QuarterStep => TICKS_PER_STEP / 4,
            Self::ThirdStep => TICKS_PER_STEP / 3,
            Self::HalfStep => TICKS_PER_STEP / 2,
            Self::Step => TICKS_PER_STEP,
            Self::ThirdBeat => (meter.sig_at(tick).beat_ticks() / 3).max(1),
            Self::HalfBeat => (meter.sig_at(tick).beat_ticks() / 2).max(1),
            Self::Beat => meter.sig_at(tick).beat_ticks(),
            Self::Bar => meter.bar(tick).len,
        }
    }

    /// Menu label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "None",
            Self::SixthStep => "1/6 step",
            Self::QuarterStep => "1/4 step",
            Self::ThirdStep => "1/3 step",
            Self::HalfStep => "1/2 step",
            Self::Step => "Step",
            Self::ThirdBeat => "1/3 beat",
            Self::HalfBeat => "1/2 beat",
            Self::Beat => "Beat",
            Self::Bar => "Bar",
        }
    }

    /// Rounds a non-negative tick position down to the grid of `meter`.
    #[must_use]
    pub fn floor(self, ticks: f32, meter: &Meter) -> u32 {
        let tick = ticks.max(0.0) as u32;
        match self {
            Self::Beat => meter.floor_to_beat(tick),
            Self::Bar => meter.floor_to_bar(tick),
            Self::ThirdBeat | Self::HalfBeat => {
                let beat = meter.floor_to_beat(tick);
                let grid = self.ticks_at(tick, meter);
                beat + (tick - beat) / grid * grid
            }
            _ => {
                let grid = self.ticks_at(tick, meter);
                tick / grid * grid
            }
        }
    }

    /// Rounds a non-negative tick position to the nearest grid line of
    /// `meter`.
    #[must_use]
    pub fn round(self, ticks: f32, meter: &Meter) -> u32 {
        let ticks = ticks.max(0.0);
        let below = self.floor(ticks, meter);
        // The next line: a grid step on, but never past the bar line.
        let bar_end = meter.bar(below).end();
        let above = below
            .saturating_add(self.ticks_at(below, meter))
            .min(bar_end);
        if ticks - below as f32 >= (above - below) as f32 / 2.0 {
            above
        } else {
            below
        }
    }
}

/// Maps between screen positions and (tick, row) in a time grid whose row 0 is at
/// the top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Screen position of tick 0, row 0.
    pub origin: Pos2,
    /// Horizontal zoom.
    pub px_per_tick: f32,
    /// Row height.
    pub row_height: f32,
    /// Number of rows.
    pub rows: u32,
    /// Rows with their own height: each row's share of `row_height` in
    /// percent (the playlist's per-track heights). `None`: every row is
    /// `row_height` tall.
    pub scales: Option<RowScales>,
}

/// Rows covered by [`Grid::scales`]; later rows are 100%.
pub const SCALED_ROWS: usize = 64;

/// Each row's height in percent of the grid's row height.
pub type RowScales = [u16; SCALED_ROWS];

impl Grid {
    /// X coordinate of a tick.
    #[must_use]
    pub fn x(&self, ticks: f32) -> f32 {
        self.origin.x + ticks * self.px_per_tick
    }

    /// Y coordinate of the top of a row.
    #[must_use]
    pub fn y(&self, row: u32) -> f32 {
        let Some(scales) = &self.scales else {
            return self.origin.y + row as f32 * self.row_height;
        };
        let row = row as usize;
        let scaled: u32 = scales.iter().take(row).map(|&s| u32::from(s)).sum();
        let rest = row.saturating_sub(SCALED_ROWS) as f32;
        self.origin.y + self.row_height * (scaled as f32 / 100.0 + rest)
    }

    /// Height of one row.
    #[must_use]
    pub fn height_of(&self, row: u32) -> f32 {
        let scale = self
            .scales
            .as_ref()
            .and_then(|s| s.get(row as usize))
            .map_or(100, |&s| s);
        self.row_height * f32::from(scale) / 100.0
    }

    /// Tick under an x coordinate (never negative).
    #[must_use]
    pub fn tick_at(&self, x: f32) -> f32 {
        ((x - self.origin.x) / self.px_per_tick).max(0.0)
    }

    /// Row under a y coordinate, if inside the grid.
    #[must_use]
    pub fn row_at(&self, y: f32) -> Option<u32> {
        if self.scales.is_none() {
            let row = ((y - self.origin.y) / self.row_height).floor();
            return (row >= 0.0 && row < self.rows as f32).then_some(row as u32);
        }
        if y < self.origin.y {
            return None;
        }
        (0..self.rows).find(|&row| y < self.y(row + 1))
    }

    /// Screen rectangle of a span of ticks on a row.
    #[must_use]
    pub fn span(&self, row: u32, start: u32, length: u32) -> Rect {
        Rect::from_min_max(
            pos2(self.x(start as f32), self.y(row)),
            pos2(self.x((start + length) as f32), self.y(row + 1)),
        )
    }
}

/// Where a pointer is relative to a set of boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// Over nothing.
    Empty,
    /// Over the body of item `n`.
    Body(usize),
    /// Over the right edge (resize handle) of item `n`.
    Edge(usize),
}

/// Width of the resize handle at an item's right edge, in pixels.
pub const EDGE_GRAB: f32 = 6.0;

/// Finds the topmost (last) box under `pos`.
pub fn hit_test(boxes: impl DoubleEndedIterator<Item = (usize, Rect)>, pos: Pos2) -> Hit {
    for (index, rect) in boxes.rev() {
        if rect.contains(pos) {
            let edge = (rect.right() - pos.x) <= EDGE_GRAB.min(rect.width() / 3.0);
            return if edge {
                Hit::Edge(index)
            } else {
                Hit::Body(index)
            };
        }
    }
    Hit::Empty
}

/// Formats a tick position as `bar.beat.step` (1-based) in `meter`, as shown
/// in the toolbar.
#[must_use]
pub fn position_label(ticks: f64, meter: &Meter) -> String {
    let p = meter.position(ticks.max(0.0) as u32);
    format!("{}.{}.{}", p.bar, p.beat, p.step)
}

/// `bar.beat.tick`: the tick within the beat in place of the step.
#[must_use]
pub fn tick_label(ticks: f64, meter: &Meter) -> String {
    let tick = ticks.max(0.0) as u32;
    let p = meter.position(tick);
    let bar = meter.bar(tick);
    let into = tick - bar.start.min(tick);
    format!(
        "{}.{}.{:02}",
        p.bar,
        p.beat,
        into % bar.sig.beat_ticks().max(1)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid {
            origin: pos2(100.0, 50.0),
            px_per_tick: 0.5,
            row_height: 10.0,
            rows: 128,
            scales: None,
        }
    }

    #[test]
    fn scaled_rows_stack_their_own_heights() {
        let mut scales = [100; SCALED_ROWS];
        scales[1] = 200;
        scales[2] = 50;
        let g = Grid {
            scales: Some(scales),
            ..grid()
        };
        assert!((g.y(1) - 60.0).abs() < 1e-4);
        assert!((g.y(2) - 80.0).abs() < 1e-4);
        assert!((g.y(3) - 85.0).abs() < 1e-4);
        assert!((g.height_of(1) - 20.0).abs() < 1e-4);
        assert_eq!(g.row_at(79.0), Some(1));
        assert_eq!(g.row_at(82.0), Some(2));
        assert_eq!(g.row_at(49.0), None);
        // Rows past the scaled ones are 100%.
        assert!((g.y(70) - (50.0 + 10.0 * 70.5)).abs() < 1e-3);
    }

    #[test]
    fn maps_both_ways() {
        let g = grid();
        assert!((g.x(96.0) - 148.0).abs() < f32::EPSILON);
        assert!((g.tick_at(148.0) - 96.0).abs() < f32::EPSILON);
        assert!(g.tick_at(0.0).abs() < f32::EPSILON);
        assert_eq!(g.row_at(55.0), Some(0));
        assert_eq!(g.row_at(65.0), Some(1));
        assert_eq!(g.row_at(49.0), None);
        assert_eq!(g.row_at(50.0 + 1280.0), None);
        let r = g.span(2, 24, 24);
        assert_eq!(
            (r.left(), r.top(), r.width(), r.height()),
            (112.0, 70.0, 12.0, 10.0)
        );
    }

    #[test]
    fn snapping() {
        let m = Meter::default();
        assert_eq!(Snap::Step.floor(47.9, &m), 24);
        assert_eq!(Snap::Step.round(36.0, &m), 48);
        assert_eq!(Snap::Bar.floor(383.0, &m), 0);
        assert_eq!(Snap::Off.floor(7.6, &m), 7);
        assert_eq!(Snap::Beat.floor(-5.0, &m), 0);
    }

    #[test]
    fn snapping_follows_the_signature() {
        let sig = |s: &str| s.parse::<musiccraft_project::TimeSig>().unwrap();
        // 6/8, then 3/4 from tick 200 (cutting the first bar short).
        let m = Meter::with_changes(sig("6/8"), [(200, sig("3/4"))]);
        assert_eq!(Snap::Beat.floor(100.0, &m), 96);
        assert_eq!(Snap::Beat.round(198.0, &m), 200, "a change is a bar line");
        assert_eq!(Snap::Bar.floor(250.0, &m), 200);
        assert_eq!(Snap::Bar.round(450.0, &m), 488);
        assert_eq!(Snap::Bar.ticks_at(10, &m), 200);
        assert_eq!(Snap::Beat.ticks_at(300, &m), 96);
    }

    #[test]
    fn triplet_and_beat_fraction_snaps() {
        let m = Meter::default();
        assert_eq!(Snap::ThirdStep.ticks_at(0, &m), 8);
        assert_eq!(Snap::SixthStep.round(13.0, &m), 12);
        assert_eq!(Snap::ThirdBeat.floor(100.0, &m), 96);
        assert_eq!(Snap::ThirdBeat.round(120.0, &m), 128);
        assert_eq!(Snap::HalfBeat.floor(150.0, &m), 144);
        // Beat fractions count from each beat, after a bar cut short.
        let sig = |s: &str| s.parse::<musiccraft_project::TimeSig>().unwrap();
        let m = Meter::with_changes(sig("4/4"), [(200, sig("4/4"))]);
        assert_eq!(Snap::HalfBeat.floor(250.0, &m), 248);
        let ticks: Vec<u32> = Snap::ALL
            .iter()
            .map(|s| s.ticks_at(0, &Meter::default()))
            .collect();
        assert!(
            ticks.windows(2).all(|w| w[0] < w[1]),
            "finest first: {ticks:?}"
        );
    }

    #[test]
    fn hits_prefer_topmost_and_detect_edges() {
        let a = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 10.0));
        let b = Rect::from_min_max(pos2(50.0, 0.0), pos2(150.0, 10.0));
        let boxes = || [(0, a), (1, b)].into_iter();
        assert_eq!(hit_test(boxes(), pos2(60.0, 5.0)), Hit::Body(1));
        assert_eq!(hit_test(boxes(), pos2(10.0, 5.0)), Hit::Body(0));
        assert_eq!(hit_test(boxes(), pos2(148.0, 5.0)), Hit::Edge(1));
        assert_eq!(hit_test(boxes(), pos2(200.0, 5.0)), Hit::Empty);
    }

    #[test]
    fn position_labels() {
        let m = Meter::default();
        assert_eq!(position_label(0.0, &m), "1.1.1");
        assert_eq!(position_label(f64::from(384 + 96 + 24), &m), "2.2.2");
        assert_eq!(tick_label(f64::from(384 + 96 + 30), &m), "2.2.30");
        assert_eq!(tick_label(f64::from(384 + 5), &m), "2.1.05");
        let m = Meter::new("7/8".parse().unwrap());
        assert_eq!(position_label(f64::from(336 + 48), &m), "2.2.1");
    }
}

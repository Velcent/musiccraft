//! Pure layout maths shared by the piano roll and playlist editors.

use egui::{Pos2, Rect, pos2};
use musiccraft_project::Meter;
use musiccraft_project::params::TICKS_PER_STEP;

/// Grid resolution for placing and moving things in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Snap {
    /// No snapping (single ticks).
    Off,
    /// A quarter of a step (1/64 note).
    QuarterStep,
    /// Half a step (1/32 note).
    HalfStep,
    /// One step (1/16 note).
    Step,
    /// One beat of the time signature.
    Beat,
    /// One bar of the time signature.
    Bar,
}

impl Snap {
    /// Every option, finest first.
    pub const ALL: [Self; 6] = [
        Self::Off,
        Self::QuarterStep,
        Self::HalfStep,
        Self::Step,
        Self::Beat,
        Self::Bar,
    ];

    /// Grid size in ticks at `tick` of `meter` (beats and bars vary with the
    /// time signature).
    #[must_use]
    pub fn ticks_at(self, tick: u32, meter: &Meter) -> u32 {
        match self {
            Self::Off => 1,
            Self::QuarterStep => TICKS_PER_STEP / 4,
            Self::HalfStep => TICKS_PER_STEP / 2,
            Self::Step => TICKS_PER_STEP,
            Self::Beat => meter.sig_at(tick).beat_ticks(),
            Self::Bar => meter.bar(tick).len,
        }
    }

    /// Menu label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "None",
            Self::QuarterStep => "1/4 step",
            Self::HalfStep => "1/2 step",
            Self::Step => "Step",
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
}

impl Grid {
    /// X coordinate of a tick.
    #[must_use]
    pub fn x(&self, ticks: f32) -> f32 {
        self.origin.x + ticks * self.px_per_tick
    }

    /// Y coordinate of the top of a row.
    #[must_use]
    pub fn y(&self, row: u32) -> f32 {
        self.origin.y + row as f32 * self.row_height
    }

    /// Tick under an x coordinate (never negative).
    #[must_use]
    pub fn tick_at(&self, x: f32) -> f32 {
        ((x - self.origin.x) / self.px_per_tick).max(0.0)
    }

    /// Row under a y coordinate, if inside the grid.
    #[must_use]
    pub fn row_at(&self, y: f32) -> Option<u32> {
        let row = ((y - self.origin.y) / self.row_height).floor();
        (row >= 0.0 && row < self.rows as f32).then_some(row as u32)
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
    format!("{:>3}.{}.{}", p.bar, p.beat, p.step)
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
        }
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
        assert_eq!(position_label(0.0, &m), "  1.1.1");
        assert_eq!(position_label(f64::from(384 + 96 + 24), &m), "  2.2.2");
        let m = Meter::new("7/8".parse().unwrap());
        assert_eq!(position_label(f64::from(336 + 48), &m), "  2.2.1");
    }
}

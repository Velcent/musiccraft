//! Pure layout maths shared by the piano roll and playlist editors.

use egui::{Pos2, Rect, pos2};
use musiccraft_project::params::{PPQ, TICKS_PER_BAR, TICKS_PER_STEP};

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
    /// One beat (1/4 note).
    Beat,
    /// One bar.
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

    /// Grid size in ticks.
    #[must_use]
    pub fn ticks(self) -> u32 {
        match self {
            Self::Off => 1,
            Self::QuarterStep => TICKS_PER_STEP / 4,
            Self::HalfStep => TICKS_PER_STEP / 2,
            Self::Step => TICKS_PER_STEP,
            Self::Beat => PPQ,
            Self::Bar => TICKS_PER_BAR,
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

    /// Rounds a non-negative tick position down to the grid.
    #[must_use]
    pub fn floor(self, ticks: f32) -> u32 {
        let grid = self.ticks();
        (ticks.max(0.0) as u32 / grid) * grid
    }

    /// Rounds a non-negative tick position to the nearest grid line.
    #[must_use]
    pub fn round(self, ticks: f32) -> u32 {
        let grid = self.ticks() as f32;
        ((ticks.max(0.0) / grid).round() * grid) as u32
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

/// Formats a tick position as `bar.beat.step` (1-based), as shown in the toolbar.
#[must_use]
pub fn position_label(ticks: f64) -> String {
    let ticks = ticks.max(0.0) as u32;
    let bar = ticks / TICKS_PER_BAR + 1;
    let beat = ticks % TICKS_PER_BAR / PPQ + 1;
    let step = ticks % PPQ / TICKS_PER_STEP + 1;
    format!("{bar:>3}.{beat}.{step}")
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
        assert_eq!(Snap::Step.floor(47.9), 24);
        assert_eq!(Snap::Step.round(36.0), 48);
        assert_eq!(Snap::Bar.floor(383.0), 0);
        assert_eq!(Snap::Off.floor(7.6), 7);
        assert_eq!(Snap::Beat.floor(-5.0), 0);
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
        assert_eq!(position_label(0.0), "  1.1.1");
        assert_eq!(
            position_label(f64::from(TICKS_PER_BAR + PPQ + TICKS_PER_STEP)),
            "  2.2.2"
        );
    }
}

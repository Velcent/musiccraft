//! Time signatures and the bar and beat grid of a timeline.
//!
//! A [`Meter`] is a time signature plus the changes along a timeline. Building
//! one allocates; every query is allocation-free and cannot panic, so the
//! engine may query a meter on the audio thread.

use std::fmt;
use std::str::FromStr;

use musiccraft_params::{PPQ, TICKS_PER_STEP};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Largest numerator.
pub const MAX_NUMERATOR: u32 = 32;
/// Accepted denominators.
pub const DENOMINATORS: [u32; 4] = [2, 4, 8, 16];

/// A time signature `num/den`: `num` beats per bar, each a `1/den` note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSig {
    num: u8,
    den: u8,
}

impl TimeSig {
    /// 4/4.
    pub const COMMON: Self = Self { num: 4, den: 4 };

    /// A signature, if `num` is 1..=[`MAX_NUMERATOR`] and `den` one of
    /// [`DENOMINATORS`].
    #[must_use]
    pub fn new(num: u32, den: u32) -> Option<Self> {
        let ok = (1..=MAX_NUMERATOR).contains(&num) && DENOMINATORS.contains(&den);
        ok.then(|| Self {
            num: u8::try_from(num).unwrap_or(4),
            den: u8::try_from(den).unwrap_or(4),
        })
    }

    /// Beats per bar.
    #[must_use]
    pub fn num(self) -> u32 {
        u32::from(self.num)
    }

    /// The note value of a beat (4 = quarter note).
    #[must_use]
    pub fn den(self) -> u32 {
        u32::from(self.den)
    }

    /// Length of a beat in ticks.
    #[must_use]
    pub fn beat_ticks(self) -> u32 {
        PPQ * 4 / self.den().max(1)
    }

    /// Length of a bar in ticks.
    #[must_use]
    pub fn bar_ticks(self) -> u32 {
        self.num() * self.beat_ticks()
    }

    /// Length of a bar in steps (sixteenths); always whole.
    #[must_use]
    pub fn bar_steps(self) -> u32 {
        self.bar_ticks() / TICKS_PER_STEP
    }

    /// Whether this is 4/4.
    #[must_use]
    pub fn is_common(&self) -> bool {
        *self == Self::COMMON
    }
}

impl Default for TimeSig {
    fn default() -> Self {
        Self::COMMON
    }
}

impl fmt::Display for TimeSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl FromStr for TimeSig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || {
            format!(
                "`{s}` is not a time signature: write n/d with n 1..={MAX_NUMERATOR} and d one of 2, 4, 8, 16"
            )
        };
        let (num, den) = s.trim().split_once('/').ok_or_else(bad)?;
        let num = num.trim().parse().map_err(|_| bad())?;
        let den = den.trim().parse().map_err(|_| bad())?;
        Self::new(num, den).ok_or_else(bad)
    }
}

impl Serialize for TimeSig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TimeSig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// One bar of a meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    /// 0-based bar number.
    pub index: u32,
    /// First tick.
    pub start: u32,
    /// Length in ticks: a bar of `sig`, or less where a change cuts it short.
    pub len: u32,
    /// The signature in force.
    pub sig: TimeSig,
}

impl Bar {
    /// First tick after the bar.
    #[must_use]
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.len)
    }
}

/// A beat of a meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beat {
    /// Position in ticks.
    pub tick: u32,
    /// 0-based bar number.
    pub bar: u32,
    /// Whether this is the bar's first beat.
    pub downbeat: bool,
}

/// A position as bar, beat and step, all 1-based; the step counts sixteenths
/// within the beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Bar, from 1.
    pub bar: u32,
    /// Beat in the bar, from 1.
    pub beat: u32,
    /// Step in the beat, from 1.
    pub step: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    tick: u32,
    bar: u32,
    sig: TimeSig,
}

/// A timeline's signature and its changes. Each change starts a new bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meter {
    /// The first starts at tick 0, bar 0; ticks strictly rise. Empty means
    /// 4/4 throughout ([`Meter::COMMON`]).
    segments: Vec<Segment>,
}

impl Default for Meter {
    fn default() -> Self {
        Self::new(TimeSig::COMMON)
    }
}

impl From<TimeSig> for Meter {
    fn from(sig: TimeSig) -> Self {
        Self::new(sig)
    }
}

impl Meter {
    /// 4/4 throughout, built without allocating (usable in a `static`).
    pub const COMMON: Self = Self {
        segments: Vec::new(),
    };

    /// One signature throughout.
    #[must_use]
    pub fn new(sig: TimeSig) -> Self {
        Self {
            segments: vec![Segment {
                tick: 0,
                bar: 0,
                sig,
            }],
        }
    }

    /// `sig` from the start, then each `(tick, sig)` change. A change at tick 0
    /// replaces `sig`; of two at one tick the later wins.
    #[must_use]
    pub fn with_changes(sig: TimeSig, changes: impl IntoIterator<Item = (u32, TimeSig)>) -> Self {
        let mut changes: Vec<_> = changes.into_iter().collect();
        changes.sort_by_key(|c| c.0);
        let mut meter = Self::new(sig);
        for (tick, sig) in changes {
            let Some(last) = meter.segments.last_mut() else {
                continue;
            };
            if tick == last.tick {
                last.sig = sig;
                continue;
            }
            let bars = (tick - last.tick).div_ceil(last.sig.bar_ticks().max(1));
            let bar = last.bar.saturating_add(bars);
            meter.segments.push(Segment { tick, bar, sig });
        }
        meter
    }

    /// The signature at the start.
    #[must_use]
    pub fn first(&self) -> TimeSig {
        self.segments.first().map_or(TimeSig::COMMON, |s| s.sig)
    }

    /// Changes after the start, as `(tick, sig)`.
    pub fn changes(&self) -> impl Iterator<Item = (u32, TimeSig)> + '_ {
        self.segments.iter().skip(1).map(|s| (s.tick, s.sig))
    }

    fn segment(&self, i: usize) -> Segment {
        self.segments.get(i).copied().unwrap_or(Segment {
            tick: 0,
            bar: 0,
            sig: TimeSig::COMMON,
        })
    }

    /// Index of the segment holding `tick`.
    fn segment_index(&self, tick: u32) -> usize {
        self.segments
            .partition_point(|s| s.tick <= tick)
            .saturating_sub(1)
    }

    /// Bar `k` of segment `i`, cut at the next change.
    fn bar_in(&self, i: usize, k: u32) -> Bar {
        let seg = self.segment(i);
        let bar_ticks = seg.sig.bar_ticks();
        let start = seg.tick.saturating_add(k.saturating_mul(bar_ticks));
        let mut end = start.saturating_add(bar_ticks);
        if let Some(next) = self.segments.get(i + 1) {
            end = end.min(next.tick);
        }
        Bar {
            index: seg.bar.saturating_add(k),
            start,
            len: end.saturating_sub(start).max(1),
            sig: seg.sig,
        }
    }

    /// The signature in force at `tick`.
    #[must_use]
    pub fn sig_at(&self, tick: u32) -> TimeSig {
        self.segment(self.segment_index(tick)).sig
    }

    /// The bar holding `tick`.
    #[must_use]
    pub fn bar(&self, tick: u32) -> Bar {
        let i = self.segment_index(tick);
        let seg = self.segment(i);
        let k = (tick - seg.tick.min(tick)) / seg.sig.bar_ticks().max(1);
        self.bar_in(i, k)
    }

    /// Bar number `index` (0-based).
    #[must_use]
    pub fn bar_at_index(&self, index: u32) -> Bar {
        let i = self
            .segments
            .partition_point(|s| s.bar <= index)
            .saturating_sub(1);
        let seg = self.segment(i);
        self.bar_in(i, index.saturating_sub(seg.bar))
    }

    /// First tick of bar `index` (0-based).
    #[must_use]
    pub fn bar_start(&self, index: u32) -> u32 {
        self.bar_at_index(index).start
    }

    /// Position in bars (0-based, fractional) of `tick`.
    #[must_use]
    pub fn ticks_to_bars(&self, tick: f64) -> f64 {
        let whole = tick.max(0.0).floor().min(f64::from(u32::MAX)) as u32;
        let bar = self.bar(whole);
        f64::from(bar.index) + (tick.max(0.0) - f64::from(bar.start)) / f64::from(bar.len)
    }

    /// The tick `bars` bars (0-based, fractional) from the start, rounded.
    #[must_use]
    pub fn bars_to_ticks(&self, bars: f64) -> u32 {
        if bars.is_nan() || bars <= 0.0 {
            return 0;
        }
        let whole = bars.floor().min(f64::from(u32::MAX));
        let bar = self.bar_at_index(whole as u32);
        let tick = f64::from(bar.start) + (bars - whole) * f64::from(bar.len);
        tick.round().min(f64::from(u32::MAX)) as u32
    }

    /// Ticks from `start` to `bars` bars later (bars of whatever signatures lie
    /// in between).
    #[must_use]
    pub fn span(&self, start: u32, bars: f64) -> u32 {
        let from = self.ticks_to_bars(f64::from(start));
        self.bars_to_ticks(from + bars).saturating_sub(start)
    }

    /// `tick` rounded up to the next bar line; at least the end of the first
    /// bar.
    #[must_use]
    pub fn round_up_to_bar(&self, tick: u32) -> u32 {
        let bar = self.bar(tick);
        if tick > 0 && tick == bar.start {
            tick
        } else {
            bar.end()
        }
    }

    /// The bar line at or before `tick`.
    #[must_use]
    pub fn floor_to_bar(&self, tick: u32) -> u32 {
        self.bar(tick).start
    }

    /// The beat at or before `tick`.
    #[must_use]
    pub fn floor_to_beat(&self, tick: u32) -> u32 {
        let bar = self.bar(tick);
        let beat = bar.sig.beat_ticks().max(1);
        bar.start + (tick - bar.start.min(tick)) / beat * beat
    }

    /// The first beat at or after `tick`.
    #[must_use]
    pub fn next_beat(&self, tick: f64) -> Beat {
        let at = tick.max(0.0).ceil().min(f64::from(u32::MAX)) as u32;
        let bar = self.bar(at);
        let beat = bar.sig.beat_ticks().max(1);
        let k = (at - bar.start.min(at)).div_ceil(beat);
        let tick = bar.start.saturating_add(k.saturating_mul(beat));
        if tick < bar.end() {
            Beat {
                tick,
                bar: bar.index,
                downbeat: k == 0,
            }
        } else {
            Beat {
                tick: bar.end(),
                bar: bar.index.saturating_add(1),
                downbeat: true,
            }
        }
    }

    /// Beats from `from` (inclusive) to `to` (exclusive).
    #[must_use]
    pub fn beats(&self, from: u32, to: u32) -> Beats<'_> {
        Beats {
            meter: self,
            next: from,
            to,
        }
    }

    /// Bar, beat and step of `tick`, 1-based.
    #[must_use]
    pub fn position(&self, tick: u32) -> Position {
        let bar = self.bar(tick);
        let beat = bar.sig.beat_ticks().max(1);
        let into = tick - bar.start.min(tick);
        Position {
            bar: bar.index.saturating_add(1),
            beat: into / beat + 1,
            step: into % beat / TICKS_PER_STEP + 1,
        }
    }
}

/// Iterator over a meter's beats; see [`Meter::beats`].
#[derive(Debug, Clone)]
pub struct Beats<'a> {
    meter: &'a Meter,
    next: u32,
    to: u32,
}

impl Iterator for Beats<'_> {
    type Item = Beat;

    fn next(&mut self) -> Option<Beat> {
        if self.next >= self.to {
            return None;
        }
        let beat = self.meter.next_beat(f64::from(self.next));
        if beat.tick >= self.to || beat.tick < self.next {
            self.next = self.to;
            return None;
        }
        self.next = beat.tick.saturating_add(1);
        Some(beat)
    }
}

#[cfg(test)]
mod tests;

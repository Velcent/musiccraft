// Ported from serum2vital's `serum_tables.py`
// (https://github.com/btesser/serum2vital), Copyright (C) 2026 Ben Tesser,
// GPL-3.0-or-later; see `lib.rs`.

//! Serum 1's menu orders and knob laws, as serum2vital read them back from
//! the plugin. A menu parameter is stored as `index / (count - 1)`; older
//! builds have shorter menus in the same order.

/// Filter types (stored as index / 95; older builds: / 89, / 88).
pub const FILTER_NAMES: [&str; 96] = [
    "MG Low 6",
    "MG Low 12",
    "MG Low 18",
    "MG Low 24",
    "Low 6",
    "Low 12",
    "Low 18",
    "Low 24",
    "High 6",
    "High 12",
    "High 18",
    "High 24",
    "Band 12",
    "Band 24",
    "Peak 12",
    "Peak 24",
    "Notch 12",
    "Notch 24",
    "LH 6",
    "LH 12",
    "LB 12",
    "LP 12",
    "LN 12",
    "HB 12",
    "HP 12",
    "HN 12",
    "BP 12",
    "BN 12",
    "PP 12",
    "PN 12",
    "NN 12",
    "L/B/H 12",
    "L/B/H 24",
    "L/P/H 12",
    "L/P/H 24",
    "L/N/H 12",
    "L/N/H 24",
    "B/P/N 12",
    "B/P/N 24",
    "Cmb +",
    "Cmb -",
    "Cmb L6+",
    "Cmb L6-",
    "Cmb H6+",
    "Cmb H6-",
    "Cmb HL6+",
    "Cmb HL6-",
    "Flg +",
    "Flg -",
    "Flg L6+",
    "Flg L6-",
    "Flg H6+",
    "Flg H6-",
    "Flg HL6+",
    "Flg HL6-",
    "Phs 12+",
    "Phs 12-",
    "Phs 24+",
    "Phs 24-",
    "Phs 36+",
    "Phs 36-",
    "Phs 48+",
    "Phs 48-",
    "Phs 48L6+",
    "Phs 48L6-",
    "Phs 48H6+",
    "Phs 48H6-",
    "Phs 48HL6+",
    "Phs 48HL6-",
    "FPhs 12HL6+",
    "FPhs 12HL6-",
    "Low EQ 6",
    "Low EQ 12",
    "Band EQ 12",
    "High EQ 6",
    "High EQ 12",
    "Ring Mod",
    "Ring Modx2",
    "SampHold",
    "SampHold-",
    "Combs",
    "Allpasses",
    "Reverb",
    "French LP",
    "German LP",
    "Add Bass",
    "Formant-I",
    "Formant-II",
    "Formant-III",
    "Bandreject",
    "Dist.Comb 1 LP",
    "Dist.Comb 1 BP",
    "Dist.Comb 2 LP",
    "Dist.Comb 2 BP",
    "Scream LP",
    "Scream BP",
];

/// Warp modes (index / 23); entries 18-20 read "from A" on oscillator B.
pub const WARP_NAMES: [&str; 24] = [
    "Off",
    "Sync",
    "Sync 1/2 Win.",
    "Sync Window",
    "Bend +",
    "Bend -",
    "Bend +/-",
    "PWM",
    "Asym +",
    "Asym -",
    "Asym +/-",
    "Flip",
    "Mirror",
    "Remap 1",
    "Remap 2",
    "Remap 3",
    "Remap 4",
    "Quantize",
    "FM (from B)",
    "AM (from B)",
    "RM (from B)",
    "FM (Noise)",
    "FM (Sub)",
    "",
];

/// Distortion modes (index / 15; older builds with 13 modes: / 12).
pub const DIST_MODE_NAMES: [&str; 16] = [
    "Tube",
    "SoftClip",
    "HardClip",
    "Diode 1",
    "Diode 2",
    "Lin.Fold",
    "Sin Fold",
    "Zero-Square",
    "Downsample",
    "Asym",
    "Rectify",
    "X-Shaper",
    "X-Shaper (Asym)",
    "Sine Shaper",
    "Stomp Box",
    "Tape Sat.",
];

/// Unison stacks (index / 8).
pub const UNISON_STACK_NAMES: [&str; 9] = [
    "off",
    "12 (1x)",
    "12 (2x)",
    "12 (3x)",
    "12+7(1x)",
    "12+7(2x)",
    "12+7(3x)",
    "Center-12",
    "Center-24",
];

/// Sub oscillator shapes (index / 4).
pub const SUB_SHAPE_NAMES: [&str; 5] = ["Sine", "RoundRect", "Saw", "Square", "Pulse"];

/// EQ band types (index / 2).
pub const EQ_TYPE_NAMES: [&str; 3] = ["Shelf", "Peak", "LPF"];

/// Delay modes (index / 2).
pub const DELAY_MODE_NAMES: [&str; 3] = ["Normal", "Ping-Pong", "Tap->Delay"];

/// LFO rate knob in BPM mode: (first step, last step, division) over 229 steps (stored × 228).
pub const LFO_BPM_RUNS: [(u8, u8, &str); 15] = [
    (0, 13, "32 bar"),
    (14, 29, "16 bar"),
    (30, 46, "8 bar"),
    (47, 62, "4 bar"),
    (63, 78, "2 bar"),
    (79, 94, "bar"),
    (95, 111, "1/2"),
    (112, 127, "1/4"),
    (128, 143, "1/8"),
    (144, 160, "1/16"),
    (161, 176, "1/32"),
    (177, 192, "1/64"),
    (193, 208, "1/128"),
    (209, 225, "1/256"),
    (226, 228, "fast"),
];

/// Delay time knob, synced: (first step, last step, division).
pub const DELAY_SYNC_RUNS: [(u8, u8, &str); 12] = [
    (0, 10, "fast"),
    (11, 31, "1/256"),
    (32, 51, "1/128"),
    (52, 72, "1/64"),
    (73, 93, "1/32"),
    (94, 114, "1/16"),
    (115, 134, "1/8"),
    (135, 155, "1/4"),
    (156, 176, "1/2"),
    (177, 196, "bar"),
    (197, 217, "2 bar"),
    (218, 228, "4 bar"),
];

/// LFO rate knob in Hz mode: 0..100 Hz, quartic.
#[must_use]
pub fn lfo_hz(n: f32) -> f32 {
    100.0 * n.powi(4)
}

/// Chorus, flanger and phaser rate knobs, unsynced: 0..20 Hz, quartic.
#[must_use]
pub fn fx_rate_hz(n: f32) -> f32 {
    20.0 * n.powi(4)
}

/// Envelope attack, hold, decay and release: 0..32 s, quintic.
#[must_use]
pub fn env_seconds(n: f32) -> f32 {
    32.0 * n.powi(5)
}

/// Portamento time: 0..8 s, quintic.
#[must_use]
pub fn portamento_seconds(n: f32) -> f32 {
    8.0 * n.powi(5)
}

/// Filter cutoff knobs: 8 Hz .. 22050 Hz, log-linear.
#[must_use]
pub fn cutoff_hz(n: f32) -> f32 {
    8.0 * (22_050.0_f32 / 8.0).powf(n)
}

/// A log-linear frequency knob from `lo` to `hi` Hz.
#[must_use]
pub fn log_hz(n: f32, lo: f32, hi: f32) -> f32 {
    lo * (hi / lo).powf(n)
}

/// Compressor attack and release: 0.1..1000 ms, quadratic.
#[must_use]
pub fn comp_ms(n: f32) -> f32 {
    (1_000.0 * n * n).max(0.1)
}

/// Delay time knob, unsynced: 1 + 500·n⁴ ms, in seconds.
#[must_use]
pub fn delay_seconds(n: f32) -> f32 {
    (1.0 + 500.0 * n.powi(4)) / 1_000.0
}

/// Delay offset knob: a 0.5x .. 1.5x multiplier of the time.
#[must_use]
pub fn delay_offset(n: f32) -> f32 {
    0.5 + n
}

/// The step (0..228) of a 229-step knob.
#[must_use]
pub fn step(n: f32) -> u8 {
    (n.clamp(0.0, 1.0) * 228.0).round() as u8
}

/// The division of an LFO rate knob step in BPM mode.
#[must_use]
pub fn lfo_division(step: u8) -> &'static str {
    run(&LFO_BPM_RUNS, step)
}

/// The division of a synced delay time knob step.
#[must_use]
pub fn delay_division(step: u8) -> &'static str {
    run(&DELAY_SYNC_RUNS, step)
}

fn run(runs: &[(u8, u8, &'static str)], step: u8) -> &'static str {
    runs.iter()
        .find(|&&(a, b, _)| (a..=b).contains(&step))
        .or(runs.last())
        .map_or("", |r| r.2)
}

/// Recovers a menu index from a stored `index / (count - 1)` value, trying
/// each divisor (`count - 1`, newest build first); the first that lands on
/// a whole number wins.
#[must_use]
pub fn indexed(value: f32, divisors: &[u16]) -> Option<usize> {
    divisors.iter().find_map(|&d| {
        let scaled = value * f32::from(d);
        ((scaled - scaled.round()).abs() < 0.02).then(|| scaled.round().max(0.0) as usize)
    })
}

/// A menu entry by stored value, or `None` when it does not decode.
#[must_use]
pub fn entry<'a>(names: &[&'a str], value: f32, divisors: &[u16]) -> Option<&'a str> {
    indexed(value, divisors).and_then(|i| names.get(i).copied())
}

/// Beats in a division (`"1/4"` is one beat, `"bar"` four, `"8 bar"` 32);
/// `"fast"` counts as 1/512.
#[must_use]
pub fn division_beats(division: &str) -> f32 {
    match division {
        "fast" => 4.0 / 512.0,
        "bar" => 4.0,
        _ => {
            if let Some(bars) = division.strip_suffix(" bar") {
                bars.trim().parse::<f32>().map_or(4.0, |b| 4.0 * b)
            } else if let Some(d) = division.strip_prefix("1/") {
                d.parse::<f32>().map_or(1.0, |d| 4.0 / d)
            } else {
                1.0
            }
        }
    }
}

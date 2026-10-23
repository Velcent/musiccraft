//! Pieces of the mapping shared by Serum 1 and Serum 2: sources, warps, LFO
//! curves and rates, pitch and categories. Our own design, using the unit
//! laws serum2vital measured.

use musiccraft_antiserum::params::{SYNC_RATES, source};
use musiccraft_patch::PointData;

use crate::fxp::LfoSettings;
use crate::menus;
use crate::out::Out;

/// Serum's cutoff knob spans 8 Hz–22.05 kHz, Antiserum's 20 Hz–20 kHz, both
/// logarithmic: a modulation amount is rescaled by the ratio of their spans.
pub const CUTOFF_MOD_SCALE: f32 = 1.146_8;

/// Antiserum's sub oscillator mod source and noise mod source (`mod_source`).
pub const FROM_OTHER: usize = 0;
pub const FROM_SUB: usize = 2;
pub const FROM_NOISE: usize = 3;

/// Antiserum's source for a Serum source name, and whether it is the same
/// thing (`false`: a stand-in). `None`: no counterpart.
#[must_use]
pub fn source(name: &str) -> Option<(usize, bool)> {
    let numbered = |prefix: &str, base: usize, count: usize| {
        name.strip_prefix(prefix)
            .and_then(|n| n.parse::<usize>().ok())
            .filter(|&n| (1..=count).contains(&n))
            .map(|n| (base + n - 1, true))
    };
    numbered("env_", source::ENV1, 4)
        .or_else(|| numbered("lfo_", source::LFO1, 8))
        .or_else(|| numbered("macro_", source::MACRO1, 8))
        .or(match name {
            "velocity" => Some((source::VELOCITY, true)),
            "note" => Some((source::NOTE, true)),
            "aftertouch" => Some((source::AFTERTOUCH, true)),
            "poly_aftertouch" | "mpe_z" => Some((source::AFTERTOUCH, false)),
            "mod_wheel" => Some((source::MOD_WHEEL, true)),
            "pitch_bend" => Some((source::PITCH_BEND, true)),
            "note_random_1" => Some((source::RANDOM, true)),
            "note_random_2" => Some((source::RANDOM, false)),
            _ => None,
        })
}

/// A readable name for a Serum source in notes.
#[must_use]
pub fn source_label(name: &str) -> String {
    name.replace('_', " ")
}

/// Serum's envelope curve knob (0..1, ½ straight) as Antiserum's curve
/// (−1..1, 0 straight; both bend towards a fast start as they grow).
#[must_use]
pub fn env_curve(knob: f32) -> f32 {
    (3.0 * (knob - 0.5)).clamp(-1.0, 1.0)
}

/// An LFO curve as Antiserum points. `flip` turns Serum 1's screen y (0 at
/// the top) into a value; `close` ties the last point to the first, as
/// Serum 1 plays its curves.
#[must_use]
pub fn lfo_points(
    xs: &[f64],
    ys: &[f64],
    curves: &[f64],
    flip: bool,
    close: bool,
) -> Vec<PointData> {
    let mut ys: Vec<f64> = ys.to_vec();
    if close && ys.len() >= 3 {
        let first = ys[0];
        let mut closed = ys.clone();
        if let Some(last) = closed.last_mut() {
            *last = first;
        }
        // A flat curve would do nothing; keep the open one then.
        let (lo, hi) = closed
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)));
        if hi - lo > 1e-3 {
            ys = closed;
        }
    }
    xs.iter()
        .zip(&ys)
        .enumerate()
        .map(|(i, (&x, &y))| {
            let tension = curves.get(i).copied().unwrap_or(0.5);
            let y = if flip { 1.0 - y } else { y };
            [
                x.clamp(0.0, 1.0) as f32,
                y.clamp(0.0, 1.0) as f32,
                // Serum's tension above ½ starts slowly; Antiserum's curve
                // starts slowly below 0.
                ((0.5 - tension) * 4.0).clamp(-1.0, 1.0) as f32,
            ]
        })
        .collect()
}

/// The nearest of Antiserum's synced rates to a cycle of `beats`, and
/// whether it is within 1 %.
#[must_use]
pub fn sync_rate(beats: f32) -> (usize, bool) {
    let mut best = (7, f32::MAX);
    for (i, &(_, b)) in SYNC_RATES.iter().enumerate() {
        let distance = (beats / b).ln().abs();
        if distance < best.1 {
            best = (i, distance);
        }
    }
    (best.0, best.1 < 0.01)
}

/// Sets LFO `lfo`'s (0-based) mode and rate from Serum's switches and its
/// rate knob position `knob` (0..1). `used`: whether the matrix uses it, so
/// approximations are worth a note.
pub fn apply_lfo(out: &mut Out, lfo: usize, settings: &LfoSettings, knob: f32, used: bool) {
    let n = lfo + 1;
    if !settings.known && used {
        out.note(format!(
            "approximated: LFO {n}'s switches are not stored in this preset; Serum's defaults assumed"
        ));
    }
    out.set(
        &format!("lfo{n}_mode"),
        match settings.mode {
            "trig" => 0.0,
            "env" => 1.0,
            _ => 2.0,
        },
    );
    if settings.hz_mode {
        let hz = menus::lfo_hz(knob);
        out.flag(&format!("lfo{n}_sync"), false);
        out.set(&format!("lfo{n}_rate"), hz);
        if used && !(0.01..=50.0).contains(&hz) {
            out.note(format!(
                "approximated: LFO {n} runs at {hz:.3} Hz, outside Antiserum's 0.01–50 Hz; clamped"
            ));
        }
    } else {
        let division = menus::lfo_division(menus::step(knob));
        let mut beats = menus::division_beats(division);
        let mut label = division.to_owned();
        if settings.dotted {
            beats *= 1.5;
            label.push_str(" dotted");
        } else if settings.triplet {
            beats *= 2.0 / 3.0;
            label.push_str(" triplet");
        }
        let (index, exact) = sync_rate(beats);
        out.flag(&format!("lfo{n}_sync"), true);
        out.set(&format!("lfo{n}_sync_rate"), index as f32);
        if used && !exact {
            out.note(format!(
                "approximated: LFO {n}'s rate {label} plays as {}",
                SYNC_RATES[index].0
            ));
        }
    }
}

/// Antiserum's warp for a Serum 1 warp menu entry: (warp, mod source,
/// exact). `None`: no counterpart.
#[must_use]
pub fn warp1(name: &str) -> Option<(usize, Option<usize>, bool)> {
    Some(match name {
        "Off" | "" => (0, None, true),
        "Sync" => (1, None, true),
        "Sync 1/2 Win." | "Sync Window" => (1, None, false),
        "Bend +" => (2, None, true),
        "Bend -" => (3, None, true),
        "Bend +/-" => (2, None, false),
        "PWM" => (4, None, true),
        "Asym +" => (5, None, true),
        "Asym -" => (6, None, true),
        "Asym +/-" => (5, None, false),
        "Mirror" => (7, None, true),
        "Flip" => (8, None, true),
        "Quantize" => (9, None, true),
        "Remap 1" | "Remap 2" | "Remap 3" | "Remap 4" => (10, None, false),
        "FM (from B)" => (11, Some(FROM_OTHER), true),
        "AM (from B)" => (12, Some(FROM_OTHER), true),
        "RM (from B)" => (13, Some(FROM_OTHER), true),
        "FM (Noise)" => (11, Some(FROM_NOISE), true),
        "FM (Sub)" => (11, Some(FROM_SUB), true),
        _ => return None,
    })
}

/// Antiserum's warp for a Serum 2 warp menu name, as [`warp1`].
#[must_use]
pub fn warp2(name: &str) -> Option<(usize, Option<usize>, bool)> {
    let from = |suffix: &str| match suffix {
        "OSC" => Some(FROM_OTHER),
        "OSC2" => Some(1),
        "SUB" => Some(FROM_SUB),
        "NOISE" => Some(FROM_NOISE),
        _ => None,
    };
    if let Some((kind, suffix)) = name.strip_prefix('k').and_then(|n| n.split_once('_')) {
        let source = from(suffix)?;
        return Some(match kind {
            "FM" => (11, Some(source), true),
            "AM" => (12, Some(source), true),
            "RM" => (13, Some(source), true),
            "PD" | "FMP" | "FMX" => (11, Some(source), false),
            _ => return None,
        });
    }
    Some(match name {
        "kSync" => (1, None, true),
        "kBendPos" => (2, None, true),
        "kBendNeg" => (3, None, true),
        "kBendPosNeg" => (2, None, false),
        "kPWM" => (4, None, true),
        "kASYMPos" => (5, None, true),
        "kASYMNeg" => (6, None, true),
        "kQuantize" => (9, None, true),
        _ => return None,
    })
}

/// Antiserum's warp menu label, for notes.
#[must_use]
pub fn warp_label(warp: usize) -> &'static str {
    musiccraft_antiserum::params::WARPS
        .get(warp)
        .copied()
        .unwrap_or("off")
}

/// Antiserum's noise type for a Serum noise sample, chosen from its name,
/// and whether the name said so.
#[must_use]
pub fn noise_type(sample: &str) -> (usize, bool) {
    let name = sample.to_lowercase();
    let types = musiccraft_antiserum::params::NOISE_TYPES;
    types
        .iter()
        .position(|t| *t != "white" && name.contains(t))
        .map_or_else(|| (0, name.contains("white")), |i| (i, true))
}

/// Splits a transpose in semitones into Antiserum's octave (−4..4) and
/// semitone (−12..12) knobs; `true` when it did not fit.
#[must_use]
pub fn split_transpose(semitones: i32) -> (i32, i32, bool) {
    let octave = ((semitones as f32) / 12.0).round() as i32;
    let octave_clamped = octave.clamp(-4, 4);
    let semi = semitones - 12 * octave_clamped;
    let semi_clamped = semi.clamp(-12, 12);
    (
        octave_clamped,
        semi_clamped,
        octave != octave_clamped || semi != semi_clamped,
    )
}

/// One of Antiserum's preset categories, guessed from a preset's name
/// prefix or words and from its tags.
#[must_use]
pub fn category(name: &str, tags: &[String]) -> Option<&'static str> {
    const WORDS: [(&str, &[&str]); 7] = [
        ("bass", &["bass", "ba", "bs", "808", "reese", "sub"]),
        ("lead", &["lead", "ld", "ld_", "screech"]),
        ("pad", &["pad", "pd", "atmo", "atmosphere"]),
        ("pluck", &["pluck", "pl", "plk"]),
        (
            "keys",
            &[
                "keys", "key", "ky", "piano", "pn", "organ", "or", "bell", "bl",
            ],
        ),
        ("fx", &["fx", "sfx", "riser", "impact", "sweep"]),
        (
            "drums",
            &["drum", "drums", "dr", "kick", "kck", "snare", "hat", "perc"],
        ),
    ];
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .chain(tags.iter().map(|t| t.to_lowercase()))
        .collect();
    let first = words.first().cloned().unwrap_or_default();
    // The prefix decides first ("BA Growl", "LD_Saw"), then any word.
    WORDS
        .iter()
        .find(|(_, keys)| keys.contains(&first.as_str()))
        .or_else(|| {
            WORDS.iter().find(|(_, keys)| {
                words
                    .iter()
                    .any(|w| w.len() > 2 && keys.contains(&w.as_str()))
            })
        })
        .map(|(category, _)| *category)
}

//! Serum 2's effect racks onto Antiserum's rack. Module names, defaults and
//! unit laws from serum2vital's `serum2_fx.py` and `fx_common.py`; the
//! choice of module and control is our own design.

use crate::cbor::Value;
use crate::filters;
use crate::fx1::{
    self, CHORUS, COMPRESSOR, DELAY, DIMENSION, DISTORTION, EQ, FILTER, FLANGER, MULTIBAND, PHASER,
    REVERB, TEMPO, comp_threshold_db, interp,
};
use crate::mapping::CUTOFF_MOD_SCALE;
use crate::out::Out;
use crate::serum2::{Serum2Patch, lookup};

/// Module keys by rack entry `type`.
const TYPE_NAMES: [&str; 16] = [
    "FXDistortion",
    "FXFlanger",
    "FXPhaser",
    "FXChorus",
    "FXDelay",
    "FXComp",
    "FXReverb",
    "FXEQ",
    "FXFilter",
    "FXHyperD",
    "FXBode",
    "FXConv",
    "FXUtils",
    "FXSplit",
    "FXSplit3",
    "FXSplitMS",
];

/// A text default.
enum D {
    N(f64),
    T(&'static str),
}

/// Module defaults: Serum 2 stores only the parameters that differ.
fn defaults(module: &str) -> &'static [(&'static str, D)] {
    use D::{N, T};
    match module {
        "FXDistortion" => &[
            ("kParamMode", T("kTube")),
            ("kParamDrive", N(0.0)),
            ("kParamWet", N(100.0)),
            ("kParamPrePost", N(0.0)),
        ],
        "FXFlanger" => &[
            ("kParamRate", N(0.5)),
            ("kParamBeatSync", N(0.0)),
            ("kParamDepth", N(50.0)),
            ("kParamFeedback", N(50.0)),
            ("kParamWet", N(50.0)),
        ],
        "FXPhaser" => &[
            ("kParamRate", N(0.5)),
            ("kParamBeatSync", N(0.0)),
            ("kParamDepth", N(50.0)),
            ("kParamFeedback", N(50.0)),
            ("kParamNumPoles", N(6.0)),
            ("kParamWet", N(50.0)),
        ],
        "FXChorus" => &[
            ("kParamRate", N(0.25)),
            ("kParamBeatSync", N(0.0)),
            ("kParamDelay", N(5.0)),
            ("kParamDepth", N(4.0)),
            ("kParamFeedback", N(0.0)),
            ("kParamFilt", N(20_000.0)),
            ("kParamFiltMode", N(0.0)),
            ("kParamWet", N(50.0)),
        ],
        "FXDelay" => &[
            ("kParamBeatSync", N(1.0)),
            ("kParamMode", N(0.0)),
            ("kParamTimeL", N(0.083_165_867_600_189_29)),
            ("kParamTimeR", N(0.083_165_867_600_189_29)),
            ("kParamOffsetL", N(1.0)),
            ("kParamOffsetR", N(1.0)),
            ("kParamLink", N(0.0)),
            ("kParamFeedback", N(50.0)),
            ("kParamWet", N(30.0)),
        ],
        "FXComp" => &[
            ("kParamAttack", N(10.0)),
            ("kParamRelease", N(100.0)),
            ("kParamThresh", N(0.25)),
            ("kParamRatio", N(2.0)),
            ("kParamMakeup", N(1.0)),
            ("kParamMultiband", N(0.0)),
            ("kParamWet", N(100.0)),
            ("kParamGain0", N(0.0)),
            ("kParamGain1", N(0.0)),
            ("kParamGain2", N(0.0)),
            ("kParamThreshUD0", N(100.0)),
            ("kParamThreshUD1", N(100.0)),
            ("kParamThreshUD2", N(100.0)),
        ],
        "FXReverb" => &[
            ("kParamType", T("kPlate")),
            ("kParamSize", N(50.0)),
            ("kParamWet", N(30.0)),
            ("kParamWidth", N(100.0)),
            ("kParamFreq", N(0.0)),
            ("kParamFreqB", N(0.0)),
            ("kParamPreDelay", N(0.0)),
            ("kParamDelay", N(0.0)),
        ],
        "FXEQ" => &[
            ("kParamFreq1", N(200.0)),
            ("kParamFreq2", N(2_000.0)),
            ("kParamGain1", N(0.0)),
            ("kParamGain2", N(0.0)),
            ("kParamReso1", N(43.33)),
            ("kParamReso2", N(43.33)),
            ("kParamType1", N(0.0)),
            ("kParamType2", N(0.0)),
        ],
        "FXFilter" => &[
            ("kParamType", T("MgL6")),
            ("kParamFreq", N(1.0)),
            ("kParamReso", N(10.0)),
            ("kParamDrive", N(0.0)),
            ("kParamWet", N(100.0)),
        ],
        "FXHyperD" => &[
            ("kParamWet", N(50.0)),
            ("kParamDimESize", N(0.0)),
            ("kParamDimEWet", N(0.0)),
        ],
        _ => &[],
    }
}

/// One rack entry with default-filled parameters.
struct Module<'a> {
    name: String,
    params: &'a [(String, Value)],
}

impl<'a> Module<'a> {
    fn new(entry: &'a Value) -> Self {
        let body = entry
            .as_map()
            .unwrap_or_default()
            .iter()
            .find(|(k, v)| k.starts_with("FX") && v.as_map().is_some());
        let name = body.map_or_else(
            || {
                let kind = entry.get("type").and_then(Value::as_f64).unwrap_or(-1.0);
                TYPE_NAMES
                    .get(kind as usize)
                    .map_or_else(|| format!("FXType{kind}"), |n| (*n).to_owned())
            },
            |(k, _)| k.clone(),
        );
        let params = body
            .and_then(|(_, v)| v.get("plainParams"))
            .and_then(Value::as_map)
            .unwrap_or_default();
        Self { name, params }
    }

    fn short(&self) -> &str {
        self.name.strip_prefix("FX").unwrap_or(&self.name)
    }

    fn has(&self, key: &str) -> bool {
        lookup(self.params, key).is_some()
    }

    fn num(&self, key: &str) -> f32 {
        if let Some(v) = lookup(self.params, key).and_then(Value::as_f64) {
            return v as f32;
        }
        defaults(&self.name)
            .iter()
            .find_map(|(k, d)| match d {
                D::N(n) if *k == key => Some(*n as f32),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    /// A percentage knob as 0..1.
    fn unit(&self, key: &str) -> f32 {
        (self.num(key) / 100.0).clamp(0.0, 1.0)
    }

    fn text(&self, key: &str) -> &str {
        if let Some(v) = lookup(self.params, key).and_then(Value::as_str) {
            return v;
        }
        defaults(&self.name)
            .iter()
            .find_map(|(k, d)| match d {
                D::T(t) if *k == key => Some(*t),
                _ => None,
            })
            .unwrap_or("")
    }

    fn enabled(&self) -> bool {
        lookup(self.params, "kParamEnable")
            .and_then(Value::as_f64)
            .is_none_or(|v| v > 0.5)
    }
}

fn rack(p: &Serum2Patch, n: usize) -> Vec<Module<'_>> {
    p.module(&format!("FXRack{n}"))
        .and_then(|r| r.get("FX"))
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter(|e| e.as_map().is_some())
        .map(Module::new)
        .collect()
}

pub fn effects(out: &mut Out, p: &Serum2Patch) {
    let mut placed = Vec::new();
    for m in rack(p, 0) {
        if !m.enabled() {
            continue;
        }
        if target(&m).is_some_and(|t| placed.contains(&t)) {
            // Antiserum's rack holds each module once.
            out.note(format!(
                "dropped: a second {} effect (the first one's settings are kept)",
                m.short()
            ));
            continue;
        }
        let module = match m.name.as_str() {
            "FXDistortion" => distortion(out, &m),
            "FXFlanger" => flanger(out, &m),
            "FXPhaser" => phaser(out, &m),
            "FXChorus" => chorus(out, &m),
            "FXDelay" => delay(out, &m),
            "FXComp" => compressor(out, &m),
            "FXReverb" => reverb(out, &m),
            "FXEQ" => eq(out, &m),
            "FXFilter" => filter(out, &m),
            "FXHyperD" => hyper(out, &m),
            _ => {
                out.note(format!(
                    "dropped: the {} effect has no counterpart",
                    m.short()
                ));
                None
            }
        };
        let Some(module) = module else { continue };
        if m.has("kParamLevelOut") && (m.num("kParamLevelOut") - 0.5).abs() > 0.02 {
            out.note(format!("dropped: the {}'s output level", m.short()));
        }
        placed.push(module);
    }
    fx1::place(out, &placed);
    for bus in 1..=2 {
        let modules = rack(p, bus);
        if !modules.is_empty() {
            let names: Vec<&str> = modules.iter().map(Module::short).collect();
            out.note(format!(
                "dropped: FX bus {bus} ({}); Antiserum has one effect rack",
                names.join(", ")
            ));
        }
    }
}

/// The Antiserum module a Serum 2 module would fill.
fn target(m: &Module) -> Option<usize> {
    Some(match m.name.as_str() {
        "FXDistortion" => DISTORTION,
        "FXFlanger" => FLANGER,
        "FXPhaser" => PHASER,
        "FXChorus" => CHORUS,
        "FXDelay" => DELAY,
        "FXComp" if m.num("kParamMultiband") > 0.5 => MULTIBAND,
        "FXComp" => COMPRESSOR,
        "FXReverb" => REVERB,
        "FXEQ" => EQ,
        "FXFilter" => FILTER,
        "FXHyperD" => DIMENSION,
        _ => return None,
    })
}

fn distortion(out: &mut Out, m: &Module) -> Option<usize> {
    let mode = m.text("kParamMode");
    let key: String = mode
        .trim_start_matches('k')
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_lowercase();
    let hard = matches!(key.as_str(), "hardclip" | "zerosquare");
    if !matches!(key.as_str(), "tube" | "softclip" | "hardclip" | "tapesat") {
        out.note(format!(
            "approximated: distortion mode '{mode}' plays as {} clipping",
            if hard { "hard" } else { "soft" }
        ));
    }
    out.set("fx_distortion_mode", f32::from(u8::from(hard)));
    out.set("fx_distortion_drive", 48.0 * m.unit("kParamDrive"));
    out.set("fx_distortion_mix", m.unit("kParamWet"));
    out.set("fx_distortion_output", -6.0);
    if m.num("kParamPrePost") > 0.5 {
        out.note("dropped: the distortion's pre/post filter");
    }
    Some(DISTORTION)
}

/// A rate stored in Hz on the 20·n⁴ law, synced or free.
fn rate(out: &mut Out, what: &str, m: &Module) -> f32 {
    let knob = (m.num("kParamRate").max(0.0) / 20.0).powf(0.25).min(1.0);
    fx1::fx_rate(out, what, m.num("kParamBeatSync") > 0.5, knob)
}

fn flanger(out: &mut Out, m: &Module) -> Option<usize> {
    let hz = rate(out, "flanger", m);
    out.set("fx_flanger_rate", hz);
    out.set("fx_flanger_depth", 5.0 * m.unit("kParamDepth"));
    out.set("fx_flanger_feedback", m.unit("kParamFeedback"));
    out.set("fx_flanger_mix", m.unit("kParamWet"));
    Some(FLANGER)
}

fn phaser(out: &mut Out, m: &Module) -> Option<usize> {
    let hz = rate(out, "phaser", m);
    out.set("fx_phaser_rate", hz);
    out.set("fx_phaser_depth", m.unit("kParamDepth"));
    out.set("fx_phaser_feedback", m.unit("kParamFeedback"));
    out.set("fx_phaser_stages", m.num("kParamNumPoles"));
    out.set("fx_phaser_mix", m.unit("kParamWet"));
    Some(PHASER)
}

fn chorus(out: &mut Out, m: &Module) -> Option<usize> {
    let hz = rate(out, "chorus", m);
    out.set("fx_chorus_rate", hz);
    out.set("fx_chorus_depth", m.num("kParamDepth"));
    out.set("fx_chorus_delay", m.num("kParamDelay"));
    out.set("fx_chorus_voices", 2.0);
    out.set("fx_chorus_mix", m.unit("kParamWet"));
    if m.num("kParamFiltMode") > 0.5 {
        out.note("dropped: the chorus's high-pass filter");
    } else {
        out.set("fx_chorus_tone", m.num("kParamFilt"));
    }
    if m.num("kParamFeedback").abs() > 5.0 {
        out.note("dropped: the chorus feedback");
    }
    Some(CHORUS)
}

/// Beats in a synced delay knob position: Serum 2 quantises the stored
/// seconds to a division (`fx_common.DELAY_SYNC_BOUNDS`, at 120 BPM).
fn synced_delay_beats(seconds: f32) -> f32 {
    const BOUNDS: [(f32, f32); 8] = [
        (0.007, 1.0 / 16.0),
        (0.014, 1.0 / 8.0),
        (0.0295, 0.25),
        (0.065, 0.5),
        (0.11, 1.0),
        (0.1795, 2.0),
        (0.27, 4.0),
        (0.41, 8.0),
    ];
    BOUNDS
        .iter()
        .find(|(bound, _)| seconds < *bound)
        .map_or(16.0, |(_, beats)| *beats)
}

fn delay(out: &mut Out, m: &Module) -> Option<usize> {
    let mode = m.num("kParamMode").round() as i32;
    out.flag("fx_delay_ping_pong", mode == 1);
    let tap = mode == 2;
    if tap {
        out.note("approximated: delay mode Tap->Delay plays as a plain delay");
    }
    let linked = m.num("kParamLink") > 0.5;
    let left = (m.num("kParamTimeL"), m.num("kParamOffsetL"));
    let right = if linked {
        left
    } else {
        (m.num("kParamTimeR"), m.num("kParamOffsetR"))
    };
    // Tap->Delay plays the right channel's time.
    let (time, offset) = if tap { right } else { left };
    let beats = if m.num("kParamBeatSync") > 0.5 {
        synced_delay_beats(time) * offset
    } else {
        out.note(format!(
            "approximated: the delay time is set in beats at {TEMPO} BPM"
        ));
        time * offset * TEMPO / 60.0
    };
    out.set("fx_delay_time", beats);
    if !tap && (left.0 * left.1 - right.0 * right.1).abs() > 0.02 * (left.0 * left.1).max(1e-6) {
        out.note("approximated: the delay's separate right time is not kept");
    }
    out.set("fx_delay_feedback", m.unit("kParamFeedback"));
    out.set("fx_delay_mix", m.unit("kParamWet"));
    Some(DELAY)
}

fn compressor(out: &mut Out, m: &Module) -> Option<usize> {
    let multiband = m.num("kParamMultiband") > 0.5;
    let module = if multiband { MULTIBAND } else { COMPRESSOR };
    let threshold = comp_threshold_db(m.num("kParamThresh"));
    // Serum 2 stores the ratio itself; Limit is a huge number.
    let ratio = m.num("kParamRatio").clamp(1.0, 20.0);
    let attack = m.num("kParamAttack");
    let release = m.num("kParamRelease");
    let makeup = 20.0 * m.num("kParamMakeup").max(1e-3).log10();
    if m.num("kParamWet") < 99.0 {
        out.note("dropped: the compressor's mix (it runs fully wet)");
    }
    if multiband {
        for (n, band) in ["low", "mid", "high"].iter().enumerate() {
            out.set(&format!("fx_multiband_{band}_thresh"), threshold);
            out.set(&format!("fx_multiband_{band}_ratio"), ratio);
            // The band L/M/H knobs (0..200 %) act as band level.
            let level = m.num(&format!("kParamThreshUD{n}")).max(1.0) / 100.0;
            let gain = m.num(&format!("kParamGain{n}")) + 20.0 * level.log10();
            out.set(&format!("fx_multiband_{band}_gain"), gain);
        }
        out.set("fx_multiband_attack", attack);
        out.set("fx_multiband_release", release);
        out.set("fx_multiband_output", makeup);
        out.note("approximated: the multiband compressor is downward only, with Serum's threshold on every band");
    } else {
        out.set("fx_compressor_threshold", threshold);
        out.set("fx_compressor_ratio", ratio);
        out.set("fx_compressor_attack", attack);
        out.set("fx_compressor_release", release);
        out.set("fx_compressor_makeup", makeup);
    }
    Some(module)
}

/// RT60 of a Serum 2 reverb (`fx_common.serum2_reverb_rt60`): size and the
/// decay knob in percent.
fn reverb_rt60(kind: &str, size: f32, decay: f32) -> f32 {
    const PLATE: [(f32, f32); 7] = [
        (0.0, 0.2),
        (10.0, 0.4),
        (20.0, 0.94),
        (35.0, 3.05),
        (50.0, 7.9),
        (65.0, 11.9),
        (100.0, 11.9),
    ];
    const HALL_FLOOR: [(f32, f32); 4] = [(0.0, 3.0), (50.0, 3.2), (80.0, 5.5), (100.0, 7.0)];
    const VINTAGE: [(f32, f32); 5] = [
        (0.0, 0.5),
        (20.0, 0.9),
        (45.0, 1.8),
        (80.0, 6.3),
        (100.0, 9.0),
    ];
    const ABYSS: [(f32, [(f32, f32); 3]); 5] = [
        (0.0, [(0.0, 0.1), (30.0, 1.2), (100.0, 2.5)]),
        (15.0, [(0.0, 0.23), (30.0, 2.5), (100.0, 5.1)]),
        (34.0, [(0.0, 1.66), (30.0, 5.7), (100.0, 7.4)]),
        (65.0, [(0.0, 5.0), (30.0, 8.4), (100.0, 12.1)]),
        (100.0, [(0.0, 7.0), (30.0, 11.0), (100.0, 15.0)]),
    ];
    match kind {
        "kPlate" => interp(size, &PLATE),
        "kVintage" => interp(size, &VINTAGE),
        "kAbyss" => {
            let rows: Vec<(f32, f32)> = ABYSS.iter().map(|(s, r)| (*s, interp(decay, r))).collect();
            interp(size, &rows)
        }
        _ => {
            let grown = 3.0 * ((decay - 30.0) / 35.0).exp();
            interp(size, &HALL_FLOOR).max(grown).min(60.0)
        }
    }
}

fn reverb(out: &mut Out, m: &Module) -> Option<usize> {
    let kind = m.text("kParamType");
    let size = m.num("kParamSize");
    out.set("fx_reverb_size", size / 100.0);
    out.set(
        "fx_reverb_decay",
        reverb_rt60(kind, size, m.num("kParamDelay")),
    );
    out.set("fx_reverb_mix", m.unit("kParamWet"));
    // Serum 2 shows the pre-delay in seconds.
    out.set("fx_reverb_predelay", 1_000.0 * m.num("kParamPreDelay"));
    out.set(
        "fx_reverb_low_cut",
        crate::menus::log_hz(m.unit("kParamFreq"), 20.0, 1_000.0),
    );
    out.set(
        "fx_reverb_damping",
        crate::menus::log_hz(1.0 - m.unit("kParamFreqB"), 500.0, 20_000.0),
    );
    out.set("fx_reverb_width", m.unit("kParamWidth"));
    out.note(format!(
        "approximated: the {} reverb is a different design; its size, decay and tone are matched by their settings",
        kind.trim_start_matches('k')
    ));
    if m.num("kParamFeedback") > 0.0 {
        out.note("dropped: the reverb's feedback");
    }
    Some(REVERB)
}

fn eq(out: &mut Out, m: &Module) -> Option<usize> {
    for (n, side, ours) in [(1, "low", "band1"), (2, "high", "band7")] {
        let kind = m.num(&format!("kParamType{n}")).round() as i32;
        out.set(
            &format!("fx_eq_{ours}_freq"),
            m.num(&format!("kParamFreq{n}")),
        );
        out.set(
            &format!("fx_eq_{ours}_gain"),
            m.num(&format!("kParamGain{n}")),
        );
        out.set(
            &format!("fx_eq_{ours}_q"),
            fx1::eq_q(m.unit(&format!("kParamReso{n}"))),
        );
        if kind != 1 {
            out.note(format!(
                "approximated: EQ band {side} ({}) plays as a peak band",
                if kind == 2 { "pass" } else { "shelf" }
            ));
        }
    }
    Some(EQ)
}

/// A filter cutoff knob (0..1 over MIDI notes 0..135) in Hz.
fn note_hz(knob: f32) -> f32 {
    let note = 135.0 * knob.clamp(0.0, 1.0);
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

fn filter(out: &mut Out, m: &Module) -> Option<usize> {
    let name = m.text("kParamType");
    // Antiserum's effect filter is low-, band- or high-pass.
    let family = filters::serum2(name).map(|(t, _)| t);
    let mode = match family {
        Some(2 | 3) => 2.0,
        Some(4 | 5) => 1.0,
        _ => 0.0,
    };
    let plain = matches!(family, Some(0..=5 | 10 | 11));
    if !plain {
        out.note(format!(
            "approximated: effect filter '{name}' plays as a {} filter",
            ["low-pass", "band-pass", "high-pass"][mode as usize]
        ));
    }
    out.set("fx_filter_mode", mode);
    out.set("fx_filter_cutoff", note_hz(m.num("kParamFreq")));
    out.set("fx_filter_resonance", m.unit("kParamReso"));
    out.set("fx_filter_mix", m.unit("kParamWet"));
    if m.num("kParamDrive") > 1.0 {
        out.note("dropped: the effect filter's drive");
    }
    Some(FILTER)
}

fn hyper(out: &mut Out, m: &Module) -> Option<usize> {
    if m.num("kParamWet") > 1.0 {
        out.note("dropped: Hyper (the unison effect)");
    }
    let mix = m.unit("kParamDimEWet");
    if mix <= 0.0 {
        return None;
    }
    out.set("fx_dimension_size", m.unit("kParamDimESize"));
    out.set("fx_dimension_mix", mix);
    Some(DIMENSION)
}

/// Effect destinations of the mod matrix.
#[must_use]
pub fn destination(module: &str, param: &str) -> Option<(String, f32)> {
    let (ours, scale) = match (module, param) {
        ("FXReverb", "kParamWet") => ("fx_reverb_mix", 1.0),
        ("FXReverb", "kParamSize") => ("fx_reverb_size", 1.0),
        ("FXDelay", "kParamWet") => ("fx_delay_mix", 1.0),
        ("FXDelay", "kParamFeedback") => ("fx_delay_feedback", 1.0),
        ("FXChorus", "kParamWet") => ("fx_chorus_mix", 1.0),
        ("FXFlanger", "kParamWet") => ("fx_flanger_mix", 1.0),
        ("FXPhaser", "kParamWet") => ("fx_phaser_mix", 1.0),
        ("FXDistortion", "kParamDrive") => ("fx_distortion_drive", 1.0),
        ("FXDistortion", "kParamWet") => ("fx_distortion_mix", 1.0),
        ("FXFilter", "kParamFreq") => ("fx_filter_cutoff", CUTOFF_MOD_SCALE),
        ("FXFilter", "kParamReso") => ("fx_filter_resonance", 1.0),
        ("FXFilter", "kParamWet") => ("fx_filter_mix", 1.0),
        _ => return None,
    };
    Some((ours.to_owned(), scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverb_laws() {
        assert!((reverb_rt60("kPlate", 50.0, 0.0) - 7.9).abs() < 1e-4);
        assert!((reverb_rt60("kHall", 0.0, 0.0) - 3.0).abs() < 1e-4);
        assert!((reverb_rt60("kAbyss", 15.0, 30.0) - 2.5).abs() < 1e-4);
    }

    #[test]
    fn filter_cutoff_is_a_note() {
        assert!((note_hz(69.0 / 135.0) - 440.0).abs() < 0.01);
    }

    #[test]
    fn synced_delay_divisions() {
        assert!((synced_delay_beats(0.083) - 1.0).abs() < 1e-6);
        assert!((synced_delay_beats(0.0387) - 0.5).abs() < 1e-6);
        assert!((synced_delay_beats(1.0) - 16.0).abs() < 1e-6);
    }
}

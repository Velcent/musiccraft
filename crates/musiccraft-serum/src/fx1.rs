//! Serum 1's effects onto Antiserum's rack. Unit laws from serum2vital's
//! `serum_tables.py` and `fx_common.py`; the choice of module and control
//! is our own design.

use crate::fxp::Serum1Patch;
use crate::mapping::CUTOFF_MOD_SCALE;
use crate::menus::{self, entry};
use crate::out::Out;

/// Antiserum's effect modules (`fx::MODULES` order); a slot holds index + 1.
pub const DIMENSION: usize = 0;
pub const DISTORTION: usize = 1;
pub const CHORUS: usize = 2;
pub const FLANGER: usize = 3;
pub const PHASER: usize = 4;
pub const DELAY: usize = 5;
pub const COMPRESSOR: usize = 6;
pub const MULTIBAND: usize = 7;
pub const EQ: usize = 8;
pub const FILTER: usize = 9;
pub const REVERB: usize = 10;

/// The tempo unsynced delay times are turned into beats at.
pub const TEMPO: f32 = 120.0;

/// Serum's compressor threshold knob in dB, as serum2vital read it back
/// (`fx_common._COMP_THRESHOLD_TABLE`).
const COMP_THRESHOLD: [(f32, f32); 22] = [
    (0.000, 0.0),
    (0.048, -1.3),
    (0.096, -2.6),
    (0.145, -4.1),
    (0.193, -5.6),
    (0.241, -7.2),
    (0.289, -8.9),
    (0.338, -10.7),
    (0.386, -12.7),
    (0.434, -14.8),
    (0.482, -17.2),
    (0.531, -19.7),
    (0.579, -22.5),
    (0.627, -25.7),
    (0.675, -29.3),
    (0.724, -33.5),
    (0.772, -38.5),
    (0.820, -44.7),
    (0.868, -52.8),
    (0.917, -64.7),
    (0.965, -87.1),
    (1.000, -120.0),
];

/// The reverb tail's floor by size (`fx_common.SERUM1_REVERB_FLOOR`).
const REVERB_FLOOR: [(f32, f32); 7] = [
    (0.1, 0.7),
    (0.2, 0.88),
    (0.35, 1.14),
    (0.5, 1.81),
    (0.65, 3.2),
    (0.8, 5.3),
    (1.0, 7.0),
];
const REVERB_MAX_RT60: f32 = 60.0;

/// Piecewise-linear interpolation through sorted points, flat outside.
#[must_use]
pub fn interp(x: f32, points: &[(f32, f32)]) -> f32 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return 0.0;
    };
    if x <= first.0 {
        return first.1;
    }
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if x <= x1 {
            return if x1 > x0 {
                y0 + (y1 - y0) * (x - x0) / (x1 - x0)
            } else {
                y0
            };
        }
    }
    last.1
}

/// The compressor threshold knob (0..1) in dB.
#[must_use]
pub fn comp_threshold_db(knob: f32) -> f32 {
    interp(knob.clamp(0.0, 1.0), &COMP_THRESHOLD)
}

/// The compressor gain knob in dB: 20·log10(1 + 31·n²).
#[must_use]
pub fn comp_makeup_db(knob: f32) -> f32 {
    20.0 * (1.0 + 31.0 * knob.clamp(0.0, 1.0).powi(2)).log10()
}

/// The reverb's RT60 from its size (0..1) and decay knob (0.8..12 s).
#[must_use]
pub fn reverb_rt60(size: f32, decay: f32) -> f32 {
    if decay >= 12.4 {
        return REVERB_MAX_RT60;
    }
    let floor = interp(size.clamp(0.0, 1.0), &REVERB_FLOOR);
    (2_000.0 / (12.5 - decay).powi(3))
        .max(floor)
        .min(REVERB_MAX_RT60)
}

/// Places the modules in rack order, each once, in Antiserum's slots.
pub fn place(out: &mut Out, modules: &[usize]) {
    let mut placed: Vec<usize> = Vec::new();
    for &m in modules {
        if placed.contains(&m) {
            continue;
        }
        if placed.len() == 10 {
            out.note("dropped: effects beyond Antiserum's ten slots");
            break;
        }
        placed.push(m);
        out.set(&format!("fx_slot{}", placed.len()), (m + 1) as f32);
    }
}

pub fn effects(out: &mut Out, p: &Serum1Patch) {
    let mut rack = Vec::new();
    for effect in p.fx_rack() {
        let enable = match effect {
            "distortion" => "Dist Enable",
            "flanger" => "Flg Enable",
            "phaser" => "Phs Enable",
            "chorus" => "Cho Enable",
            "delay" => "Dly Enable",
            "compressor" => "Comp Enable",
            "reverb" => "Rev Enable",
            "eq" => "EQ Enable",
            "filter" => "FX Fil Enable",
            _ => "Hyp Enable",
        };
        if !p.on(enable) {
            continue;
        }
        let module = match effect {
            "distortion" => distortion(out, p),
            "flanger" => flanger(out, p),
            "phaser" => phaser(out, p),
            "chorus" => chorus(out, p),
            "delay" => delay(out, p),
            "compressor" => compressor(out, p),
            "reverb" => reverb(out, p),
            "eq" => eq(out, p),
            "filter" => filter(out, p),
            _ => hyper(out, p),
        };
        rack.extend(module);
        let level = format!("FX {} Level", level_name(effect));
        if (p.value(&level) - 0.5).abs() > 0.01 {
            out.note(format!("dropped: the {effect}'s output level"));
        }
    }
    place(out, &rack);
}

fn level_name(effect: &str) -> &'static str {
    match effect {
        "distortion" => "Dist",
        "flanger" => "Flg",
        "phaser" => "Phaser",
        "chorus" => "Chorus",
        "delay" => "Delay",
        "compressor" => "Comp",
        "reverb" => "Reverb",
        "filter" => "Filter",
        "hyper" => "Hyper",
        _ => "EQ",
    }
}

fn distortion(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let mode = entry(&menus::DIST_MODE_NAMES, p.value("Dist_Mode"), &[15, 12]).unwrap_or("Tube");
    let hard = matches!(mode, "HardClip" | "Zero-Square");
    if !matches!(mode, "Tube" | "SoftClip" | "HardClip" | "Tape Sat.") {
        out.note(format!(
            "approximated: distortion mode '{mode}' plays as {} clipping",
            if hard { "hard" } else { "soft" }
        ));
    }
    out.set("fx_distortion_mode", f32::from(u8::from(hard)));
    out.set("fx_distortion_drive", 48.0 * p.value("Dist_Drv"));
    out.set("fx_distortion_mix", p.display("Dist_Wet") / 100.0);
    out.set("fx_distortion_output", -6.0);
    if p.value("Dist_PrePost") > 0.25 {
        out.note("dropped: the distortion's pre/post filter");
    }
    Some(DISTORTION)
}

/// A chorus, flanger or phaser rate: free in Hz, or synced (stood in for at
/// the reference tempo).
pub(crate) fn fx_rate(out: &mut Out, what: &str, synced: bool, knob: f32) -> f32 {
    if synced {
        let division = menus::lfo_division(menus::step(knob));
        out.note(format!(
            "approximated: the {what}'s synced rate ({division}) is set in Hz at {TEMPO} BPM"
        ));
        TEMPO / 60.0 / menus::division_beats(division)
    } else {
        menus::fx_rate_hz(knob)
    }
}

fn flanger(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let rate = fx_rate(out, "flanger", p.on("Flg_BPM_Sync"), p.value("Flg_Rate"));
    out.set("fx_flanger_rate", rate);
    out.set("fx_flanger_depth", 5.0 * p.display("Flg_Dep") / 100.0);
    out.set(
        "fx_flanger_feedback",
        2.0 * p.display("Flg_Feed") / 100.0 - 1.0,
    );
    out.set("fx_flanger_mix", p.display("Flg_Wet") / 100.0);
    Some(FLANGER)
}

fn phaser(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let rate = fx_rate(out, "phaser", p.on("Phs_BPM_Sync"), p.value("Phs_Rate"));
    out.set("fx_phaser_rate", rate);
    out.set("fx_phaser_depth", p.display("Phs_Dpth") / 100.0);
    out.set("fx_phaser_feedback", p.display("Phs_Feed") / 100.0);
    out.set("fx_phaser_mix", p.display("Phs_Wet") / 100.0);
    Some(PHASER)
}

fn chorus(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let rate = fx_rate(out, "chorus", p.on("Cho_BPM_Sync"), p.value("Cho_Rate"));
    out.set("fx_chorus_rate", rate);
    // Depth tops out at 26 ms in Serum; delay is 20·n² ms.
    out.set("fx_chorus_depth", 26.0 * p.value("Cho_Dep"));
    out.set("fx_chorus_delay", 20.0 * p.value("Cho_Dly").powi(2));
    out.set("fx_chorus_voices", 2.0);
    out.set("fx_chorus_mix", p.display("Cho_Wet") / 100.0);
    out.set(
        "fx_chorus_tone",
        menus::log_hz(p.value("Cho_Filt"), 50.0, 20_000.0),
    );
    if p.value("Cho_Feed") > 0.05 {
        out.note("dropped: the chorus feedback");
    }
    Some(CHORUS)
}

fn delay(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let mode = entry(&menus::DELAY_MODE_NAMES, p.value("Dly_Mode"), &[2]).unwrap_or("Normal");
    out.flag("fx_delay_ping_pong", mode == "Ping-Pong");
    if mode == "Tap->Delay" {
        out.note("approximated: delay mode Tap->Delay plays as a plain delay");
    }
    let time = p.value("Dly_TimL");
    let multiplier = menus::delay_offset(p.value("Dly_Off L"));
    let beats = if p.on("Dly_BPM_Sync") {
        menus::division_beats(menus::delay_division(menus::step(time))) * multiplier
    } else {
        out.note(format!(
            "approximated: the delay time is set in beats at {TEMPO} BPM"
        ));
        menus::delay_seconds(time) * multiplier * TEMPO / 60.0
    };
    out.set("fx_delay_time", beats);
    if !p.on("Dly_Link")
        && ((p.value("Dly_TimR") - time).abs() > 0.01
            || (p.value("Dly_Off R") - p.value("Dly_Off L")).abs() > 0.01)
    {
        out.note("approximated: the delay's separate right time is not kept");
    }
    out.set("fx_delay_feedback", p.display("Dly_Feed") / 100.0);
    out.set("fx_delay_mix", p.display("Dly_Wet") / 100.0);
    Some(DELAY)
}

fn compressor(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let threshold = comp_threshold_db(p.value("Cmp_Thr"));
    // Serum stores the ratio as 1 − 1/r.
    let ratio = 1.0 / (1.0 - p.value("Cmp_Rat")).max(0.05);
    let attack = menus::comp_ms(p.value("Cmp_Att"));
    let release = menus::comp_ms(p.value("Cmp_Rel"));
    let makeup = comp_makeup_db(p.value("CmpGain"));
    if p.display("Comp_Wet") < 99.0 {
        out.note("dropped: the compressor's mix (it runs fully wet)");
    }
    if p.on("CmpMBnd") {
        for band in ["low", "mid", "high"] {
            out.set(&format!("fx_multiband_{band}_thresh"), threshold);
            out.set(&format!("fx_multiband_{band}_ratio"), ratio);
        }
        out.set("fx_multiband_attack", attack);
        out.set("fx_multiband_release", release);
        out.set("fx_multiband_output", makeup);
        out.note("approximated: the multiband compressor is downward only, with Serum's threshold on every band");
        Some(MULTIBAND)
    } else {
        out.set("fx_compressor_threshold", threshold);
        out.set("fx_compressor_ratio", ratio);
        out.set("fx_compressor_attack", attack);
        out.set("fx_compressor_release", release);
        out.set("fx_compressor_makeup", makeup);
        Some(COMPRESSOR)
    }
}

fn reverb(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let size = p.display("VerbSize") / 100.0;
    out.set("fx_reverb_size", size);
    out.set("fx_reverb_decay", reverb_rt60(size, p.display("VerbDecay")));
    out.set("fx_reverb_mix", p.display("Verb Wet") / 100.0);
    out.set(
        "fx_reverb_low_cut",
        menus::log_hz(p.display("VerbLoCt") / 100.0, 20.0, 1_000.0),
    );
    out.set(
        "fx_reverb_damping",
        menus::log_hz(1.0 - p.display("VerbHiCt") / 100.0, 500.0, 20_000.0),
    );
    out.set("fx_reverb_mod_depth", p.display("VerbSpinDepth") / 100.0);
    out.set(
        "fx_reverb_mod_rate",
        menus::fx_rate_hz(p.value("VerbSpinRate")),
    );
    out.note("approximated: the reverb is a different design; size, decay and tone are matched by their settings");
    if !p.settings.reverb_hall {
        out.note("approximated: the plate reverb plays as Antiserum's hall");
    }
    Some(REVERB)
}

/// Serum's EQ Q knob (0..1) as a filter Q (0.1..10, logarithmic).
pub(crate) fn eq_q(knob: f32) -> f32 {
    0.1 * 100.0_f32.powf(knob.clamp(0.0, 1.0))
}

fn eq(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    for (band, side, ours) in [(1, "L", "band1"), (7, "H", "band7")] {
        let kind = entry(
            &menus::EQ_TYPE_NAMES,
            p.value(&format!("EQ Typ{side}")),
            &[2],
        )
        .unwrap_or("Shelf");
        out.set(
            &format!("fx_eq_{ours}_freq"),
            menus::log_hz(p.value(&format!("EQ Frq{side}")), 22.0, 20_000.0),
        );
        out.set(
            &format!("fx_eq_{ours}_gain"),
            p.display(&format!("EQ Vol{side}")),
        );
        out.set(
            &format!("fx_eq_{ours}_q"),
            eq_q(p.value(&format!("EQ Q {side}"))),
        );
        if kind != "Peak" {
            out.note(format!(
                "approximated: EQ band {} ({kind}) plays as a peak band",
                if band == 1 { "low" } else { "high" }
            ));
        }
    }
    Some(EQ)
}

fn filter(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    let name =
        entry(&menus::FILTER_NAMES, p.value("FX Fil Type"), &[95, 89, 88]).unwrap_or("Low 12");
    let mode = if name.starts_with("High") || name.starts_with('H') {
        2.0
    } else if name.starts_with("Band") || name.starts_with('B') {
        1.0
    } else {
        0.0
    };
    if !(name.starts_with("Low")
        || name.starts_with("MG Low")
        || name.starts_with("High")
        || name.starts_with("Band"))
    {
        out.note(format!(
            "approximated: effect filter '{name}' plays as a {} filter",
            ["low-pass", "band-pass", "high-pass"][mode as usize]
        ));
    }
    out.set("fx_filter_mode", mode);
    out.set("fx_filter_cutoff", menus::cutoff_hz(p.value("FX Fil Freq")));
    out.set("fx_filter_resonance", p.display("FX Fil Reso") / 100.0);
    out.set("fx_filter_mix", p.display("FX Fil Wet") / 100.0);
    if p.display("FX Fil Drive") > 1.0 {
        out.note("dropped: the effect filter's drive");
    }
    Some(FILTER)
}

fn hyper(out: &mut Out, p: &Serum1Patch) -> Option<usize> {
    if p.display("Hyp_Wet") > 1.0 {
        out.note("dropped: Hyper (the unison effect)");
    }
    let mix = p.display("HypDim_Mix") / 100.0;
    if mix <= 0.0 {
        return None;
    }
    out.set("fx_dimension_size", p.display("HypDim_Size") / 100.0);
    out.set("fx_dimension_mix", mix);
    Some(DIMENSION)
}

/// Effect destinations of the mod matrix.
#[must_use]
pub fn destination(name: &str) -> Option<(String, f32)> {
    let (ours, scale) = match name {
        "Dist_Drv" => ("fx_distortion_drive", 1.0),
        "Dist_Wet" => ("fx_distortion_mix", 1.0),
        "Cho_Wet" => ("fx_chorus_mix", 1.0),
        "Cho_Rate" => ("fx_chorus_rate", 1.0),
        "Cho_Dep" => ("fx_chorus_depth", 1.0),
        "Flg_Wet" => ("fx_flanger_mix", 1.0),
        "Flg_Rate" => ("fx_flanger_rate", 1.0),
        "Flg_Dep" => ("fx_flanger_depth", 1.0),
        "Flg_Feed" => ("fx_flanger_feedback", 1.0),
        "Phs_Wet" => ("fx_phaser_mix", 1.0),
        "Phs_Rate" => ("fx_phaser_rate", 1.0),
        "Phs_Dpth" => ("fx_phaser_depth", 1.0),
        "Phs_Feed" => ("fx_phaser_feedback", 1.0),
        "Dly_Wet" => ("fx_delay_mix", 1.0),
        "Dly_Feed" => ("fx_delay_feedback", 1.0),
        "Verb Wet" => ("fx_reverb_mix", 1.0),
        "VerbSize" => ("fx_reverb_size", 1.0),
        "VerbDecay" => ("fx_reverb_decay", 1.0),
        "FX Fil Freq" => ("fx_filter_cutoff", CUTOFF_MOD_SCALE),
        "FX Fil Reso" => ("fx_filter_resonance", 1.0),
        "FX Fil Wet" => ("fx_filter_mix", 1.0),
        "Cmp_Thr" => ("fx_compressor_threshold", -1.0),
        "HypDim_Size" => ("fx_dimension_size", 1.0),
        "HypDim_Mix" => ("fx_dimension_mix", 1.0),
        _ => return None,
    };
    Some((ours.to_owned(), scale))
}

//! Serum 2 onto Antiserum. Module and parameter names, defaults and unit
//! laws from serum2vital's `mapping.py`; which Antiserum control stands in is
//! our own design.

use musiccraft_antiserum::params::{FILTER_TYPES, STACKS};

use crate::assets::Assets;
use crate::cbor::Value;
use crate::filters;
use crate::fx2;
use crate::fxp::LfoSettings;
use crate::mapping::{self, CUTOFF_MOD_SCALE};
use crate::menus;
use crate::out::Out;
use crate::serum2::{Serum2Patch, lookup};

pub fn convert(p: &Serum2Patch, assets: &Assets) -> Out {
    let mut out = Out::new(&p.name);
    out.patch
        .tags
        .extend(p.tags.iter().map(|t| t.to_lowercase()));
    if let Some(category) = mapping::category(&p.name, &p.tags) {
        out.patch.category = category.to_owned();
    }
    global(&mut out, p);
    for osc in 0..3 {
        oscillator(&mut out, p, osc, assets);
    }
    sub(&mut out, p);
    noise(&mut out, p);
    filters_(&mut out, p);
    envelopes(&mut out, p);
    lfos(&mut out, p);
    macros(&mut out, p);
    fx2::effects(&mut out, p);
    modulation(&mut out, p);
    out
}

fn global(out: &mut Out, p: &Serum2Patch) {
    let g = "Global0";
    // The master knob is cubic in amplitude, Antiserum's squared.
    let master = p.param(g, "kParamMasterVolume", 0.7).max(0.0) as f32;
    out.set("volume", master.powf(1.5));
    if p.has(g, "kParamPolyCount") {
        out.set("polyphony", p.param(g, "kParamPolyCount", 8.0) as f32);
    }
    if p.param(g, "kParamMonoToggle", 0.0) > 0.5 {
        let legato = p.param(g, "kParamLegato", 0.0) > 0.5;
        out.set("voice_mode", if legato { 2.0 } else { 1.0 });
    }
    let glide = p.param(g, "kParamPortamentoTime", 0.0) as f32;
    out.set("glide", glide);
    if glide > 5.0 {
        out.note(format!(
            "approximated: portamento of {glide:.1} s is longer than Antiserum's 5 s"
        ));
    }
    if p.has(g, "kParamBendRangeUp") {
        out.set(
            "bend_range",
            p.param(g, "kParamBendRangeUp", 2.0).abs() as f32,
        );
    }
}

/// Where a sound source goes: (to filter 1, level used). Serum 2's routing
/// slot 0 defaults to the filter, the others straight out; a source heard
/// only through an FX bus takes its bus send as its level.
fn routing(out: &mut Out, p: &Serum2Patch, slot: usize, volume: f32, what: &str) -> (bool, f32) {
    let module = format!("RoutingSlot{slot}");
    let dest = p
        .text(&module, "kParamRoutingDest")
        .unwrap_or(if slot == 0 {
            "kRoutingDestFilter"
        } else {
            "kRoutingDestDirect"
        });
    let bus =
        p.param(&module, "kParamFXBus1Level", 0.0)
            .max(p.param(&module, "kParamFXBus2Level", 0.0)) as f32
            / 100.0;
    let mut level = volume.max(0.0);
    if level < 0.05 && bus > 0.0 {
        level = bus;
        out.note(format!(
            "approximated: {what} is heard only through an FX bus; its send is used as its level"
        ));
    }
    (dest == "kRoutingDestFilter", level)
}

fn oscillator(out: &mut Out, p: &Serum2Patch, osc: usize, assets: &Assets) {
    let n = osc + 1;
    let name = |param: &str| format!("osc{n}_{param}");
    let module = format!("Oscillator{osc}");
    let on = p.param(&module, "kParamEnable", if osc == 0 { 1.0 } else { 0.0 }) > 0.5;
    out.flag(&name("on"), on);
    if !on {
        return;
    }
    let kind = p.osc_type(osc);
    if kind != "kOsc_Wavetable" {
        out.note(format!(
            "approximated: oscillator {n} uses Serum 2's {} engine; it plays as a wavetable",
            kind.trim_start_matches("kOsc_")
        ));
    }
    // Levels are quadratic knobs in Serum, linear in Antiserum.
    let volume = p.param(&module, "kParamVolume", 0.75) as f32;
    let (to_filter, level) = routing(out, p, osc, volume, &format!("oscillator {n}"));
    out.set(&name("level"), level.powi(2));
    out.set(&name("filter1"), f32::from(u8::from(to_filter)));
    out.set(&name("filter2"), 0.0);
    out.set(
        &name("pan"),
        p.param(&module, "kParamPan", 0.0) as f32 / 50.0,
    );
    let total = (12.0 * p.param(&module, "kParamOctave", 0.0)
        + p.param(&module, "kParamPitch", 0.0))
    .round() as i32;
    let (octave, semi, clipped) = mapping::split_transpose(total);
    out.set(&name("octave"), octave as f32);
    out.set(&name("semi"), semi as f32);
    if clipped {
        out.note(format!(
            "approximated: oscillator {n} is transposed {total:+} semitones, beyond Antiserum's range"
        ));
    }
    out.set(&name("fine"), p.param(&module, "kParamFine", 0.0) as f32);
    let voices = p
        .param(&module, "kParamUnison", 1.0)
        .round()
        .clamp(1.0, 16.0) as f32;
    out.set(&name("unison"), voices);
    // Serum 2 stores the detune knob squared.
    out.set(
        &name("detune"),
        p.param(&module, "kParamDetune", 0.2).max(0.0).sqrt() as f32,
    );
    out.set(
        &name("width"),
        p.param(&module, "kParamDetuneWid", 100.0) as f32 / 100.0,
    );
    if let Some(stack) = p.text(&module, "kParamUnisonStack") {
        let (index, exact) = match stack {
            "kOctave1" => (1, true),
            "kOctave2" => (2, true),
            "kOctave3" => (2, false),
            "kOctaveFifth1" | "kOctaveFifth2" | "kOctaveFifth3" => (1, false),
            "kCenter12" | "kCenter24" => (3, false),
            _ => (0, false),
        };
        out.set(&name("stack"), index as f32);
        if voices > 1.0 && !exact {
            out.note(format!(
                "approximated: oscillator {n}'s unison stack '{stack}' plays as '{}'",
                STACKS[index]
            ));
        }
    }
    let wt = p
        .module(&module)
        .and_then(|m| m.get(&format!("WTOsc{osc}")));
    let reference = wt
        .and_then(|w| w.get("relativePathToWT"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let found = assets.table(reference);
    out.table(osc, found.table);
    if let Some(note) = found.note {
        out.note(note);
    }
    let params = wt
        .and_then(|w| w.get("plainParams"))
        .and_then(Value::as_map)
        .unwrap_or_default();
    let num = |key: &str, default: f64| {
        lookup(params, key)
            .and_then(Value::as_f64)
            .unwrap_or(default)
    };
    out.set(
        &name("random_phase"),
        num("kParamRandomPhase", 100.0) as f32 / 100.0,
    );
    out.set(
        &name("phase"),
        (num("kParamInitialPhase", 180.0) / 360.0) as f32,
    );
    // The table position is a 1-based frame: normalise by the table's size.
    let frames = wt
        .and_then(|w| w.get("numFrames"))
        .and_then(Value::as_f64)
        .map_or(1.0, |samples| (samples / 2_048.0).floor().max(1.0));
    let position = (num("kParamTablePos", 1.0) - 1.0) / (frames - 1.0).max(1.0);
    out.set(&name("position"), position.clamp(0.0, 1.0) as f32);
    if let Some(menu) = lookup(params, "kParamWarpMenu").and_then(Value::as_str) {
        match mapping::warp2(menu) {
            Some((warp, source, exact)) => {
                out.set(&name("warp1"), warp as f32);
                out.set(&name("warp1_amount"), num("kParamWarp", 0.0) as f32 / 100.0);
                if let Some(source) = source {
                    out.set(&name("mod_source"), source as f32);
                }
                if !exact {
                    out.note(format!(
                        "approximated: oscillator {n}'s warp '{menu}' plays as '{}'",
                        mapping::warp_label(warp)
                    ));
                }
            }
            None => out.note(format!("dropped: oscillator {n}'s warp '{menu}'")),
        }
    }
    if let Some(second) = lookup(params, "kParamWarpMenu2").and_then(Value::as_str) {
        out.note(format!("dropped: oscillator {n}'s second warp '{second}'"));
    }
}

fn sub(out: &mut Out, p: &Serum2Patch) {
    let module = "Oscillator4";
    let on = p.param(module, "kParamEnable", 0.0) > 0.5;
    out.flag("sub_on", on);
    if !on {
        return;
    }
    let (to_filter, level) = routing(
        out,
        p,
        4,
        p.param(module, "kParamVolume", 0.75) as f32,
        "the sub oscillator",
    );
    out.set("sub_level", level.powi(2));
    out.set("sub_filter1", f32::from(u8::from(to_filter)));
    out.set("sub_filter2", 0.0);
    out.set("sub_pan", p.param(module, "kParamPan", 0.0) as f32 / 50.0);
    let octave = p.param(module, "kParamOctave", 0.0).round() as f32;
    out.set("sub_octave", octave);
    if octave > 0.0 {
        out.note("approximated: the sub oscillator is above the note; Antiserum's sub goes no higher than the note");
    }
    let shape = p
        .module(module)
        .and_then(|m| m.get("SubOsc4"))
        .and_then(|s| s.get("plainParams"))
        .and_then(|s| s.get("kParamShape"))
        .and_then(Value::as_str)
        .unwrap_or("kSine");
    let ours = match shape {
        "kTriangle" => 1,
        "kSaw" => 2,
        "kSquare" => 3,
        "kRoundRect" => 1,
        "kPulse" => 3,
        _ => 0,
    };
    if matches!(shape, "kRoundRect" | "kPulse") {
        out.note(format!(
            "approximated: the sub oscillator's '{shape}' shape plays as a {}",
            if ours == 3 { "square" } else { "triangle" }
        ));
    }
    out.set("sub_shape", ours as f32);
}

fn noise(out: &mut Out, p: &Serum2Patch) {
    let module = "Oscillator3";
    let on = p.param(module, "kParamEnable", 0.0) > 0.5;
    out.flag("noise_on", on);
    if !on {
        return;
    }
    let (to_filter, level) = routing(
        out,
        p,
        3,
        p.param(module, "kParamVolume", 0.75) as f32,
        "the noise oscillator",
    );
    out.set("noise_level", level.powi(2));
    out.set("noise_filter1", f32::from(u8::from(to_filter)));
    out.set("noise_filter2", 0.0);
    out.set("noise_pan", p.param(module, "kParamPan", 0.0) as f32 / 50.0);
    let sample = p
        .module(module)
        .and_then(|m| m.get("NoiseOsc3"))
        .and_then(|n| n.get("relativePathToNoiseSample"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let (kind, named) = mapping::noise_type(sample);
    out.set("noise_type", kind as f32);
    if !named {
        out.note(format!(
            "approximated: the noise sample '{sample}' plays as Antiserum's {} noise",
            musiccraft_antiserum::params::NOISE_TYPES[kind]
        ));
    }
}

fn filters_(out: &mut Out, p: &Serum2Patch) {
    for f in 0..2 {
        let n = f + 1;
        let module = format!("VoiceFilter{f}");
        let on = p.param(&module, "kParamEnable", 0.0) > 0.5;
        out.flag(&format!("filter{n}_on"), on);
        if !on {
            continue;
        }
        let kind = p.text(&module, "kParamType").unwrap_or("MgL12");
        match filters::serum2(kind) {
            Some((ours, exact)) => {
                out.set(&format!("filter{n}_type"), ours as f32);
                if !exact {
                    out.note(format!(
                        "approximated: filter {n} '{kind}' plays as '{}'",
                        FILTER_TYPES[ours]
                    ));
                }
            }
            None => {
                out.flag(&format!("filter{n}_on"), false);
                out.note(format!(
                    "dropped: filter {n} '{kind}' has no counterpart; the filter is off"
                ));
                continue;
            }
        }
        let num = |key: &str, default: f64| p.param(&module, key, default) as f32;
        out.set(
            &format!("filter{n}_cutoff"),
            menus::cutoff_hz(num("kParamFreq", 1.0)),
        );
        out.set(
            &format!("filter{n}_resonance"),
            num("kParamReso", 10.0) / 100.0,
        );
        out.set(&format!("filter{n}_drive"), num("kParamDrive", 0.0) / 100.0);
        out.set(&format!("filter{n}_mix"), num("kParamWet", 100.0) / 100.0);
        out.set(
            &format!("filter{n}_key_track"),
            num("kParamKeyTrack", 0.0) / 100.0,
        );
    }
}

fn envelopes(out: &mut Out, p: &Serum2Patch) {
    for e in 0..4 {
        let n = e + 1;
        let module = format!("Env{e}");
        let num = |key: &str, default: f64| p.param(&module, key, default) as f32;
        for (param, key, default) in [
            ("attack", "kParamAttack", 0.005),
            ("hold", "kParamHold", 0.0),
            ("decay", "kParamDecay", 2.0),
            ("release", "kParamRelease", 0.075),
        ] {
            let seconds = num(key, default).max(0.0);
            out.set(&format!("env{n}_{param}"), seconds);
            if seconds > 10.0 {
                out.note(format!(
                    "approximated: envelope {n}'s {param} of {seconds:.1} s is longer than Antiserum's 10 s"
                ));
            }
        }
        // Envelope 1's sustain is quadratic in amplitude.
        let sustain = num("kParamSustain", 1.0).clamp(0.0, 1.0);
        out.set(
            &format!("env{n}_sustain"),
            if e == 0 { sustain.powi(2) } else { sustain },
        );
        for (param, key, default) in [
            ("attack_curve", "kParamCurve1", 50.0),
            ("decay_curve", "kParamCurve2", 66.6),
            ("release_curve", "kParamCurve3", 66.6),
        ] {
            out.set(
                &format!("env{n}_{param}"),
                mapping::env_curve(num(key, default) / 100.0),
            );
        }
    }
}

fn floats(v: Option<&Value>) -> Vec<f64> {
    v.and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn lfos(out: &mut Out, p: &Serum2Patch) {
    let used = used_sources(p);
    for l in 0..8 {
        let n = l + 1;
        let module = format!("LFO{l}");
        let mode = p.text(&module, "kParamMode");
        let settings = LfoSettings {
            hz_mode: p.param(&module, "kParamBeatSync", 1.0) < 0.5,
            dotted: p.param(&module, "kParamDotted", 0.0) > 0.5,
            triplet: p.param(&module, "kParamTriplets", 0.0) > 0.5,
            anchor: p.param(&module, "kParamAnchored", 1.0) > 0.5,
            mode: match mode {
                Some("Envelope") => "env",
                Some("Free") => "off",
                _ => "trig",
            },
            known: true,
        };
        let is_used = used.contains(&(6 + l as i64));
        if let Some(kind) = p.text(&module, "kParamType")
            && kind != "Path"
            && is_used
        {
            out.note(format!(
                "approximated: LFO {n} is a Serum 2 '{kind}' generator; it plays its drawn curve"
            ));
        }
        let curve = p.module(&module).and_then(|m| m.get("curveData"));
        let mut xs = floats(curve.and_then(|c| c.get("xVals")));
        let mut ys = floats(curve.and_then(|c| c.get("yVals")));
        let mut tension = floats(curve.and_then(|c| c.get("curveVals")));
        if xs.is_empty() {
            // Serum 2's default shape.
            xs = vec![0.0, 0.5, 1.0];
            ys = vec![0.0, 1.0, 1.0];
            tension = vec![0.5, 0.5];
        }
        let count = xs
            .iter()
            .position(|&x| x >= 1.0 - 1e-9)
            .map_or(xs.len(), |i| i + 1)
            .min(ys.len());
        if count >= 2 {
            out.set(&format!("lfo{n}_shape"), 7.0);
            out.lfo_shape(
                l,
                mapping::lfo_points(&xs[..count], &ys[..count], &tension, false, false),
            );
        }
        // Both modes store 100·n⁴; the synced knob steps through divisions.
        let rate = p.param(&module, "kParamRate", 6.25).max(0.0);
        let knob = ((rate / 100.0).powf(0.25) as f32).clamp(0.0, 1.0);
        mapping::apply_lfo(out, l, &settings, knob, is_used);
        out.set(
            &format!("lfo{n}_rise"),
            p.param(&module, "kParamRise", 0.0) as f32,
        );
        out.set(
            &format!("lfo{n}_delay"),
            p.param(&module, "kParamDelay", 0.0) as f32,
        );
        out.set(
            &format!("lfo{n}_phase"),
            (p.param(&module, "kParamPhase", 0.0) / 360.0).rem_euclid(1.0) as f32,
        );
    }
    if (8..10).any(|l| !p.plain_params(&format!("LFO{l}")).is_empty()) {
        out.note("dropped: Serum 2's LFOs 9 and 10");
    }
}

fn macros(out: &mut Out, p: &Serum2Patch) {
    for m in 0..8 {
        out.set(
            &format!("macro{}", m + 1),
            p.param(&format!("Macro{m}"), "kParamValue", 0.0) as f32 / 100.0,
        );
    }
}

/// Serum 2's source id to its name in the shared table.
fn source_name(id: i64) -> Option<String> {
    Some(match id {
        1 => "mod_wheel".to_owned(),
        2..=5 => format!("env_{}", id - 1),
        6..=13 => format!("lfo_{}", id - 5),
        16 => "velocity".to_owned(),
        17 => "note".to_owned(),
        18 => "aftertouch".to_owned(),
        19 => "poly_aftertouch".to_owned(),
        21 => "note_random_1".to_owned(),
        22 => "note_random_2".to_owned(),
        25..=32 => format!("macro_{}", id - 24),
        33 => "pitch_bend".to_owned(),
        36 => "mpe_z".to_owned(),
        _ => return None,
    })
}

/// A name for a source with no counterpart, for notes.
fn unsupported_label(id: i64) -> String {
    match id {
        14 | 15 => format!("LFO {}", id - 5),
        20 => "the noise oscillator".to_owned(),
        23 | 24 => "note-on alternate".to_owned(),
        34 | 35 => "MPE".to_owned(),
        37 => "release velocity".to_owned(),
        38 => "fixed".to_owned(),
        49..=54 => "an audio source".to_owned(),
        55..=59 => "a voice source".to_owned(),
        _ => format!("source {id}"),
    }
}

fn mod_slots(p: &Serum2Patch) -> Vec<&Value> {
    let mut slots: Vec<(usize, &Value)> = p
        .state
        .as_map()
        .unwrap_or_default()
        .iter()
        .filter_map(|(k, v)| Some((k.strip_prefix("ModSlot")?.parse().ok()?, v)))
        .collect();
    slots.sort_by_key(|s| s.0);
    slots.into_iter().map(|s| s.1).collect()
}

fn source_ids(slot: &Value) -> (i64, i64) {
    let ids = floats(slot.get("source"));
    (
        ids.first().copied().unwrap_or(0.0) as i64,
        ids.get(1).copied().unwrap_or(0.0) as i64,
    )
}

fn used_sources(p: &Serum2Patch) -> Vec<i64> {
    mod_slots(p).into_iter().map(|s| source_ids(s).0).collect()
}

/// Serum 2 destination to Antiserum parameter, with the ratio of ranges.
fn destination(module: &str, index: usize, param: &str) -> Option<(String, f32)> {
    let n = index + 1;
    let osc = |field: &str| match n {
        1..=3 => Some(format!("osc{n}_{field}")),
        4 => ["level", "pan"]
            .contains(&field)
            .then(|| format!("noise_{field}")),
        5 => ["level", "pan"]
            .contains(&field)
            .then(|| format!("sub_{field}")),
        _ => None,
    };
    let (ours, scale) = match (module, param) {
        ("Oscillator", "kParamVolume") => (osc("level")?, 1.0),
        ("Oscillator", "kParamPan") => (osc("pan")?, 1.0),
        ("Oscillator", "kParamFine") => (osc("fine")?, 1.0),
        ("Oscillator", "kParamCoarsePit") => (osc("semi")?, 128.0 / 24.0),
        ("Oscillator", "kParamPitch") => (osc("semi")?, 1.0),
        ("Oscillator", "kParamOctave") => (osc("octave")?, 1.0),
        ("Oscillator", "kParamDetune") => (osc("detune")?, 1.0),
        ("WTOsc", "kParamTablePos") if n <= 3 => (format!("osc{n}_position"), 1.0),
        ("WTOsc", "kParamWarp") if n <= 3 => (format!("osc{n}_warp1_amount"), 1.0),
        ("VoiceFilter", "kParamFreq") if n <= 2 => (format!("filter{n}_cutoff"), CUTOFF_MOD_SCALE),
        ("VoiceFilter", "kParamReso") if n <= 2 => (format!("filter{n}_resonance"), 1.0),
        ("VoiceFilter", "kParamDrive") if n <= 2 => (format!("filter{n}_drive"), 1.0),
        ("VoiceFilter", "kParamWet") if n <= 2 => (format!("filter{n}_mix"), 1.0),
        ("LFO", "kParamRate") if n <= 8 => (format!("lfo{n}_rate"), 1.0),
        ("Env", "kParamAttack") if n <= 4 => (format!("env{n}_attack"), 1.0),
        ("Env", "kParamDecay") if n <= 4 => (format!("env{n}_decay"), 1.0),
        ("Env", "kParamSustain") if n <= 4 => (format!("env{n}_sustain"), 1.0),
        ("Env", "kParamRelease") if n <= 4 => (format!("env{n}_release"), 1.0),
        ("Macro", "kParamValue") if n <= 8 => (format!("macro{n}"), 1.0),
        ("Global", "kParamMasterVolume") => ("volume".to_owned(), 1.0),
        _ => return fx2::destination(module, param),
    };
    Some((ours, scale))
}

fn modulation(out: &mut Out, p: &Serum2Patch) {
    for slot in mod_slots(p) {
        let (id, aux_id) = source_ids(slot);
        if id == 0 {
            continue;
        }
        let params = slot
            .get("plainParams")
            .and_then(Value::as_map)
            .unwrap_or_default();
        let amount = lookup(params, "kParamAmount")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32
            / 100.0;
        if amount.abs() < 1e-6 {
            continue;
        }
        let Some((ours, exact)) = source_name(id).as_deref().and_then(mapping::source) else {
            out.note(format!(
                "dropped: modulation from {} has no counterpart",
                unsupported_label(id)
            ));
            continue;
        };
        if !exact {
            out.note(format!(
                "approximated: modulation from {} uses Antiserum's {}",
                source_name(id).map_or_else(String::new, |s| mapping::source_label(&s)),
                musiccraft_antiserum::params::SOURCES[ours]
            ));
        }
        let module = slot
            .get("destModuleTypeString")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let index = slot
            .get("destModuleID")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .max(0.0) as usize;
        let param = slot
            .get("destModuleParamName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some((dest, scale)) = destination(module, index, param) else {
            out.note(format!(
                "dropped: modulation of {module} {param} has no counterpart"
            ));
            continue;
        };
        if let Some(n) = dest
            .strip_prefix("lfo")
            .and_then(|d| d.strip_suffix("_rate"))
            && out.get(&format!("lfo{n}_sync")) > 0.5
        {
            out.note(format!(
                "approximated: modulation of LFO {n}'s synced rate does nothing in Antiserum"
            ));
        }
        let aux = if aux_id == 0 {
            0
        } else if let Some((aux, _)) = source_name(aux_id).as_deref().and_then(mapping::source) {
            aux
        } else {
            out.note(format!(
                "approximated: aux source {} has no counterpart; the routing plays without it",
                unsupported_label(aux_id)
            ));
            0
        };
        let bipolar = lookup(params, "kParamBipolar")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            > 0.5;
        out.route(ours, aux, &dest, amount * scale, bipolar);
    }
}

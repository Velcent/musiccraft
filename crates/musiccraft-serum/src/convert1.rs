//! Serum 1 onto Antiserum. Unit laws from serum2vital's `mapping.py` and
//! `serum_tables.py`; which Antiserum control stands in is our own design.

use musiccraft_antiserum::params::{FILTER_TYPES, STACKS, SUB_SHAPES};

use crate::assets::Assets;
use crate::filters;
use crate::fx1;
use crate::fxp::{ModSlot, Serum1Patch, source_name};
use crate::mapping::{self, CUTOFF_MOD_SCALE};
use crate::menus::{self, entry};
use crate::out::Out;
use crate::params::PARAMS;

pub fn convert(p: &Serum1Patch, assets: &Assets) -> Out {
    let mut out = Out::new(&p.name);
    let mut tags = Vec::new();
    if !p.menu.is_empty() {
        tags.push(p.menu.clone());
    }
    if let Some(category) = mapping::category(&p.name, &tags) {
        out.patch.category = category.to_owned();
    }
    global(&mut out, p);
    let used: Vec<&str> = p
        .mod_slots
        .iter()
        .filter(|s| s.active())
        .filter_map(|s| source_name(s.source))
        .collect();
    for (osc, letter) in [(0, "A"), (1, "B")] {
        oscillator(&mut out, p, osc, letter, assets);
    }
    out.flag("osc3_on", false);
    sub(&mut out, p);
    noise(&mut out, p);
    filter(&mut out, p);
    envelopes(&mut out, p);
    lfos(&mut out, p, &used);
    fx1::effects(&mut out, p);
    for slot in p.mod_slots.iter().filter(|s| s.active()) {
        route(&mut out, slot);
    }
    out
}

fn global(out: &mut Out, p: &Serum1Patch) {
    // Serum's master is cubic in amplitude, Antiserum's squared.
    out.set("volume", p.value("MasterVol").powf(1.5));
    let s = &p.settings;
    if !s.known {
        out.note(
            "approximated: this preset predates Serum's voicing and unison switches; defaults assumed",
        );
    }
    out.set("polyphony", f32::from(s.polyphony));
    out.set(
        "voice_mode",
        match (s.mono, s.legato) {
            (false, _) => 0.0,
            (true, false) => 1.0,
            (true, true) => 2.0,
        },
    );
    let glide = menus::portamento_seconds(p.value("PortTime"));
    out.set("glide", glide);
    if glide > 5.0 {
        out.note(format!(
            "approximated: portamento of {glide:.1} s is longer than Antiserum's 5 s"
        ));
    }
    out.set("glide_mode", if s.porta_always { 0.0 } else { 1.0 });
    out.set("glide_curve", p.display("PortCurve") / 100.0);
    if s.porta_scaled {
        out.note("dropped: scaled portamento (glide time by interval)");
    }
    let up = p.display("Bend U").round();
    let down = p.display("Bend D").round();
    out.set("bend_range", up.abs());
    if (down + up).abs() > 0.5 {
        out.note(format!(
            "approximated: pitch bend is +{up}/{down} semitones; Antiserum bends {} both ways",
            up.abs()
        ));
    }
    if (s.a4_hz - 440.0).abs() > 0.01 {
        out.note(format!("dropped: tuning reference A4 = {:.1} Hz", s.a4_hz));
    }
    for m in 1..=4 {
        out.set(
            &format!("macro{m}"),
            p.display(&format!("Macro {m}")) / 100.0,
        );
    }
}

fn oscillator(out: &mut Out, p: &Serum1Patch, osc: usize, letter: &str, assets: &Assets) {
    let n = osc + 1;
    let name = |param: &str| format!("osc{n}_{param}");
    let on = p.on(&format!("Osc {letter} On"));
    out.flag(&name("on"), on);
    if !on {
        return;
    }
    let reference = if osc == 0 {
        &p.wavetable_a
    } else {
        &p.wavetable_b
    };
    let found = assets.table(reference);
    out.table(osc, found.table);
    if let Some(note) = found.note {
        out.note(note);
    }
    // Serum's level knob is quadratic in amplitude, Antiserum's linear.
    out.set(&name("level"), p.value(&format!("{letter} Vol")).powi(2));
    out.set(&name("pan"), p.display(&format!("{letter} Pan")) / 50.0);
    // CoarsePit is a third pitch control, −64..+64 semitones.
    let coarse = ((p.value(&format!("{letter} CoarsePit")) - 0.5) * 128.0).round() as i32;
    let total = p.display(&format!("{letter} Octave")).round() as i32 * 12
        + p.display(&format!("{letter} Semi")).round() as i32
        + coarse;
    let (octave, semi, clipped) = mapping::split_transpose(total);
    out.set(&name("octave"), octave as f32);
    out.set(&name("semi"), semi as f32);
    if clipped {
        out.note(format!(
            "approximated: oscillator {letter} is transposed {total:+} semitones, beyond Antiserum's range"
        ));
    }
    out.set(&name("fine"), p.display(&format!("{letter} Fine")));
    out.set(&name("position"), p.value(&format!("{letter} WTPos")));
    out.set(&name("phase"), p.value(&format!("{letter} Phase")));
    out.set(
        &name("random_phase"),
        p.display(&format!("{letter} RandPhase")) / 100.0,
    );
    let voices = p
        .display(&format!("{letter} Unison"))
        .round()
        .clamp(1.0, 16.0);
    out.set(&name("unison"), voices);
    // Antiserum's detune spans ±100 cents (Serum's default 2-semitone range).
    let range = p.settings.unison_range[osc];
    let detune = p.value(&format!("{letter} UniDet")) * (range / 2.0).sqrt();
    out.set(&name("detune"), detune);
    if voices > 1.0 && detune > 1.0 {
        out.note(format!(
            "approximated: oscillator {letter}'s unison range of {range} semitones is wider than Antiserum's"
        ));
    }
    out.set(
        &name("blend"),
        p.display(&format!("{letter} UniBlend")) / 100.0,
    );
    out.set(
        &name("width"),
        p.display(&format!("{letter} Uni LR")) / 100.0,
    );
    let tuning = p.settings.unison_tuning[osc];
    if voices > 1.0 && tuning != "Linear" {
        out.note(format!(
            "approximated: oscillator {letter}'s unison spacing '{tuning}' plays evenly spaced"
        ));
    }
    let stack = entry(
        &menus::UNISON_STACK_NAMES,
        p.value(&format!("{letter} Uni Stack")),
        &[8],
    )
    .unwrap_or("off");
    let (index, exact) = match stack {
        "off" => (0, true),
        "12 (1x)" => (1, true),
        "12 (2x)" => (2, true),
        "12 (3x)" => (2, false),
        "12+7(1x)" | "12+7(2x)" | "12+7(3x)" => (1, false),
        _ => (3, false),
    };
    out.set(&name("stack"), index as f32);
    if voices > 1.0 && !exact {
        out.note(format!(
            "approximated: oscillator {letter}'s unison stack '{stack}' plays as '{}'",
            STACKS[index]
        ));
    }
    for (param, what) in [("Uni Warp", "warp"), ("Uni WTPos", "table position")] {
        if voices > 1.0 && p.display(&format!("{letter} {param}")).abs() > 0.5 {
            out.note(format!(
                "dropped: oscillator {letter}'s unison {what} spread"
            ));
        }
    }
    if !p.on(&format!("Osc{letter}PitchTrack")) {
        out.note(format!(
            "dropped: oscillator {letter}'s pitch tracking is off in Serum"
        ));
    }
    warp(out, p, n, letter);
    out.set(
        &name("filter1"),
        f32::from(u8::from(p.on(&format!("Osc{letter}>Fil")))),
    );
    out.set(&name("filter2"), 0.0);
}

fn warp(out: &mut Out, p: &Serum1Patch, n: usize, letter: &str) {
    let menu = entry(
        &menus::WARP_NAMES,
        p.value(&format!("WarpOsc{letter}")),
        &[23],
    );
    let amount = p.value(&format!("{letter} Warp"));
    match menu.map(|m| (m, mapping::warp1(m))) {
        Some((_, Some((warp, source, exact)))) => {
            out.set(&format!("osc{n}_warp1"), warp as f32);
            out.set(&format!("osc{n}_warp1_amount"), amount);
            if let Some(source) = source {
                out.set(&format!("osc{n}_mod_source"), source as f32);
            }
            if !exact {
                out.note(format!(
                    "approximated: oscillator {letter}'s warp '{}' plays as '{}'",
                    menu.unwrap_or_default(),
                    mapping::warp_label(warp)
                ));
            }
        }
        Some((m, None)) => out.note(format!("dropped: oscillator {letter}'s warp '{m}'")),
        None => out.note(format!(
            "dropped: oscillator {letter}'s warp mode could not be decoded"
        )),
    }
}

fn sub(out: &mut Out, p: &Serum1Patch) {
    let on = p.on("Osc S On");
    out.flag("sub_on", on);
    if !on {
        return;
    }
    let shape = entry(&menus::SUB_SHAPE_NAMES, p.value("SubOscShape"), &[4]).unwrap_or("Sine");
    let ours = match shape {
        "RoundRect" | "Pulse" => {
            out.note(format!(
                "approximated: the sub oscillator's '{shape}' shape plays as a {}",
                if shape == "Pulse" {
                    "square"
                } else {
                    "triangle"
                }
            ));
            if shape == "Pulse" { 3 } else { 1 }
        }
        "Saw" => 2,
        "Square" => 3,
        _ => 0,
    };
    out.set("sub_shape", ours as f32);
    debug_assert!(ours < SUB_SHAPES.len());
    let octave = p.display("SubOscOctave").round();
    out.set("sub_octave", octave);
    if octave > 0.0 {
        out.note("approximated: the sub oscillator is above the note; Antiserum's sub goes no higher than the note");
    }
    // The sub's level knob is linear in amplitude in both.
    out.set("sub_level", p.value("Sub Osc Level"));
    out.set("sub_pan", p.display("Sub Osc Pan") / 50.0);
    out.set("sub_filter1", f32::from(u8::from(p.on("OscS>Fil"))));
    out.set("sub_filter2", 0.0);
}

fn noise(out: &mut Out, p: &Serum1Patch) {
    let on = p.on("Osc N On");
    out.flag("noise_on", on);
    if !on {
        return;
    }
    let (kind, named) = mapping::noise_type(&p.noise_sample);
    out.set("noise_type", kind as f32);
    if !named {
        out.note(format!(
            "approximated: the noise sample '{}' plays as Antiserum's {} noise",
            p.noise_sample,
            musiccraft_antiserum::params::NOISE_TYPES[kind]
        ));
    }
    out.set("noise_level", p.value("Noise Level").powi(2));
    out.set("noise_pan", p.display("Noise Pan") / 50.0);
    // +48 semitones at the top; below the centre the rate falls faster.
    let pitch = p.value("Noise Pitch");
    let semis = if pitch >= 0.5 {
        (pitch - 0.5) * 96.0
    } else {
        (100.0 * (2.0 * pitch).max(1e-3).log2()).max(-48.0)
    };
    out.set("noise_pitch", semis.round());
    out.flag("noise_track", p.settings.noise_pitch_track);
    out.flag("noise_one_shot", p.settings.noise_one_shot);
    out.set("noise_filter1", f32::from(u8::from(p.on("OscN>Fil"))));
    out.set("noise_filter2", 0.0);
}

fn filter(out: &mut Out, p: &Serum1Patch) {
    out.flag("filter2_on", false);
    let on = p.on("Filter On");
    out.flag("filter1_on", on);
    if !on {
        return;
    }
    let name = entry(&menus::FILTER_NAMES, p.value("Fil Type"), &[95, 89, 88]);
    match name.map(|n| (n, filters::serum1(n))) {
        Some((_, Some((kind, exact)))) => {
            out.set("filter1_type", kind as f32);
            if !exact {
                out.note(format!(
                    "approximated: filter '{}' plays as '{}'",
                    name.unwrap_or_default(),
                    FILTER_TYPES[kind]
                ));
            }
        }
        Some((n, None)) => {
            out.flag("filter1_on", false);
            out.note(format!(
                "dropped: filter '{n}' has no counterpart; the filter is off"
            ));
            return;
        }
        None => {
            out.set("filter1_type", 0.0);
            out.note("approximated: the filter type could not be decoded; low-pass 12 used");
        }
    }
    out.set("filter1_cutoff", menus::cutoff_hz(p.value("Fil Cutoff")));
    out.set("filter1_resonance", p.display("Fil Reso") / 100.0);
    out.set("filter1_drive", p.display("Fil Driv") / 100.0);
    out.set("filter1_mix", p.display("Fil Mix") / 100.0);
    out.set(
        "filter1_key_track",
        f32::from(u8::from(p.settings.filter_keytrack)),
    );
    if (p.display("Fil Stereo") - 50.0).abs() > 0.5 {
        out.note("dropped: the filter's stereo offset");
    }
}

fn envelopes(out: &mut Out, p: &Serum1Patch) {
    let names = [
        ["Env1 Atk", "Env1 Hold", "Env1 Dec", "Env1 Sus", "Env1 Rel"],
        ["Env2 Atk", "Env2 Hld", "Env2 Dec", "Env2 Sus", "Env2 Rel"],
        ["Env3 Atk", "Env3 Hld", "Env3 Dec", "Env3 Sus", "Env3 Rel"],
    ];
    for (e, [atk, hold, dec, sus, rel]) in names.iter().enumerate() {
        let n = e + 1;
        for (param, serum) in [
            ("attack", atk),
            ("hold", hold),
            ("decay", dec),
            ("release", rel),
        ] {
            let seconds = menus::env_seconds(p.value(serum));
            out.set(&format!("env{n}_{param}"), seconds);
            if seconds > 10.0 {
                out.note(format!(
                    "approximated: envelope {n}'s {param} of {seconds:.1} s is longer than Antiserum's 10 s"
                ));
            }
        }
        // Envelope 1's sustain is quadratic in amplitude; the others are percentages.
        let sustain = if e == 0 {
            p.value(sus).powi(2)
        } else {
            p.display(sus) / 100.0
        };
        out.set(&format!("env{n}_sustain"), sustain);
        for (param, stage) in [
            ("attack_curve", "A"),
            ("decay_curve", "D"),
            ("release_curve", "R"),
        ] {
            let knob = p.display(&format!("{stage} curve{n}")) / 100.0;
            out.set(&format!("env{n}_{param}"), mapping::env_curve(knob));
        }
    }
}

fn lfos(out: &mut Out, p: &Serum1Patch, used: &[&str]) {
    for (l, shape) in p.lfo_shapes.iter().enumerate().take(8) {
        let n = l + 1;
        let is_used = used.contains(&format!("lfo_{n}").as_str());
        if shape.xs.len() >= 2 {
            out.set(&format!("lfo{n}_shape"), 7.0);
            out.lfo_shape(
                l,
                mapping::lfo_points(&shape.xs, &shape.ys, &shape.curves, true, true),
            );
        }
        mapping::apply_lfo(
            out,
            l,
            &shape.settings,
            p.value(&format!("LFO{n}Rate")),
            is_used,
        );
        out.set(
            &format!("lfo{n}_rise"),
            4.0 * p.value(&format!("LFO{n} Rise")),
        );
        out.set(
            &format!("lfo{n}_delay"),
            4.0 * p.value(&format!("LFO{n} Delay")),
        );
        if is_used && p.value(&format!("LFO{n} smooth")) > 0.1 {
            out.note(format!("dropped: LFO {n}'s smoothing"));
        }
    }
}

/// Serum 1 destination (parameter name) to Antiserum parameter, with the
/// ratio of the two ranges.
fn destination(name: &str) -> Option<(String, f32)> {
    let osc = |letter: &str, n: usize, param: &str| -> Option<(String, f32)> {
        let rest = param.strip_prefix(letter)?.trim_start();
        let (ours, scale) = match rest {
            "Vol" => ("level", 1.0),
            "Pan" => ("pan", 1.0),
            "Octave" => ("octave", 1.0),
            "Semi" => ("semi", 1.0),
            "CoarsePit" => ("semi", 128.0 / 24.0),
            "Fine" => ("fine", 1.0),
            "UniDet" => ("detune", 1.0),
            "UniBlend" => ("blend", 1.0),
            "Warp" => ("warp1_amount", 1.0),
            "WTPos" => ("position", 1.0),
            "Phase" => ("phase", 1.0),
            "RandPhase" => ("random_phase", 1.0),
            "Uni LR" => ("width", 1.0),
            _ => return None,
        };
        Some((format!("osc{n}_{ours}"), scale))
    };
    if let Some(found) = osc("A ", 1, name).or_else(|| osc("B ", 2, name)) {
        return Some(found);
    }
    if let Some(rest) = name.strip_prefix("Env")
        && let Some((n, stage)) = rest.split_once(' ')
    {
        let ours = match stage {
            "Atk" => "attack",
            "Hold" | "Hld" => "hold",
            "Dec" => "decay",
            "Sus" => "sustain",
            "Rel" => "release",
            _ => return None,
        };
        return Some((format!("env{n}_{ours}"), 1.0));
    }
    if let Some(rest) = name.strip_prefix("LFO") {
        if let Some(n) = rest.strip_suffix("Rate") {
            return Some((format!("lfo{n}_rate"), 1.0));
        }
        if let Some((n, what)) = rest.split_once(' ') {
            return match what {
                "Rise" => Some((format!("lfo{n}_rise"), 1.0)),
                "Delay" => Some((format!("lfo{n}_delay"), 1.0)),
                _ => None,
            };
        }
    }
    if let Some(m) = name.strip_prefix("Macro ") {
        return Some((format!("macro{m}"), 1.0));
    }
    let (ours, scale) = match name {
        "MasterVol" | "Amp." => ("volume", 1.0),
        "Noise Level" => ("noise_level", 1.0),
        "Noise Pan" => ("noise_pan", 1.0),
        "Noise Pitch" => ("noise_pitch", 1.0),
        "Sub Osc Level" => ("sub_level", 1.0),
        "Sub Osc Pan" => ("sub_pan", 1.0),
        "Fil Cutoff" => ("filter1_cutoff", CUTOFF_MOD_SCALE),
        "Fil Reso" => ("filter1_resonance", 1.0),
        "Fil Driv" => ("filter1_drive", 1.0),
        "Fil Mix" => ("filter1_mix", 1.0),
        "PortTime" => ("glide", 1.0),
        "PortCurve" => ("glide_curve", 1.0),
        _ => return fx1::destination(name),
    };
    Some((ours.to_owned(), scale))
}

fn route(out: &mut Out, slot: &ModSlot) {
    let source = source_name(slot.source);
    let Some((ours, exact)) = source.and_then(mapping::source) else {
        out.note(format!(
            "dropped: modulation from {} has no counterpart",
            source.map_or_else(|| format!("source {}", slot.source), mapping::source_label)
        ));
        return;
    };
    if !exact {
        out.note(format!(
            "approximated: modulation from {} uses Antiserum's {}",
            mapping::source_label(source.unwrap_or_default()),
            musiccraft_antiserum::params::SOURCES[ours]
        ));
    }
    let dest_name = PARAMS.get(usize::from(slot.dest)).map(|d| d.0);
    let Some((dest, scale)) = dest_name.and_then(destination) else {
        out.note(format!(
            "dropped: modulation of {} has no counterpart",
            dest_name.unwrap_or("an unknown parameter")
        ));
        return;
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
    if dest.contains("_attack") || dest.contains("_decay") || dest.contains("_release") {
        out.note("approximated: envelope time modulation follows Antiserum's time curve");
    }
    let aux = match source_name(slot.aux).filter(|_| slot.aux != 0) {
        None if slot.aux != 0 => {
            out.note(format!(
                "approximated: aux source {} is unknown; the routing plays without it",
                slot.aux
            ));
            0
        }
        None => 0,
        Some(name) => match mapping::source(name) {
            Some((aux, _)) => aux,
            None => {
                out.note(format!(
                    "approximated: aux source {} has no counterpart; the routing plays without it",
                    mapping::source_label(name)
                ));
                0
            }
        },
    };
    let amount = slot.amount * slot.out_range.clamp(0.0, 1.0) * scale;
    out.route(ours, aux, &dest, amount, slot.bipolar);
}

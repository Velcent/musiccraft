// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Dumps what `musiccraft-flp` reads from an FL Studio file as JSON, in the
//! shape PyFLP's properties take (the differential test compares the two).
//!
//! Usage: `cargo run -p musiccraft-flp --example flp_dump -- FILE > out.json`

use musiccraft_flp::arrangement::{Arrangement, ClipRef, Track};
use musiccraft_flp::channel::{Channel, ChannelKind};
use musiccraft_flp::ids::channel as id;
use musiccraft_flp::mixer::{Insert, Slot};
use musiccraft_flp::pattern::{Pattern, TimeMarker};
use musiccraft_flp::plugin::{Plugin, PluginData, wrapper_flags};
use musiccraft_flp::plugin_data::Native;
use musiccraft_flp::structs::{EnvelopeLfo, NoteItem, Tracking};
use musiccraft_flp::{FlpError, Project, Rgba};
use serde_json::{Map, Value, json};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: flp_dump FILE");
        std::process::exit(2);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{path}: {err}");
            std::process::exit(1);
        }
    };
    let preset = std::env::args().nth(2).is_some();
    let out = match Project::parse(&bytes) {
        Ok(project) => preset
            .then(|| dump_preset(&project, &path))
            .flatten()
            .unwrap_or_else(|| dump(&project)),
        Err(err) => json!({ "parse_error": error_name(&err) }),
    };
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
}

/// PyFLP's exception class for a parse failure.
pub fn error_name(err: &FlpError) -> &'static str {
    match err {
        FlpError::HeaderCorrupted(_) => "HeaderCorrupted",
        FlpError::VersionNotDetected => "VersionNotDetected",
        FlpError::BadVersion(_) => "ValueError",
        FlpError::InvalidEventChunkSize { .. } => "InvalidEventChunkSize",
        FlpError::Event { .. } => "ConstructError",
    }
}

/// Reads a preset as PyFLP's tests do (their `get_model`).
pub fn dump_preset(p: &Project, path: &str) -> Option<Value> {
    let path = std::path::Path::new(path);
    let kind = path.parent()?.file_name()?.to_str()?;
    let ext = path.extension()?.to_str()?;
    Some(match kind {
        "channels" => {
            let kinds = [
                ChannelKind::Sampler,
                ChannelKind::Instrument,
                ChannelKind::Layer,
                ChannelKind::Automation,
            ];
            let mut out = Map::new();
            for kind in kinds {
                let ch = p.preset_channel(kind);
                let mut value = channel(&ch);
                if let Value::Object(map) = &mut value {
                    map.insert("iid".into(), json!(ch.raw_int(id::NEW)));
                }
                out.insert(kind.class().to_owned(), value);
            }
            Value::Object(out)
        }
        "inserts" => json!({ "insert": insert(&p.preset_insert()) }),
        "patterns" if ext == "fsc" => json!({ "pattern": pattern(&p.preset_pattern()) }),
        "plugins" => json!({ "plugin": plugin(p.preset_plugin()) }),
        _ => return None,
    })
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Map<_, _>>(),
    )
}

fn rgba(c: Option<Rgba>) -> Value {
    c.map_or(Value::Null, |c| json!([c.red, c.green, c.blue, c.alpha]))
}

fn bit<T: Into<i64>>(flags: Option<T>, mask: i64) -> Value {
    json!(flags.map(|f| f.into() & mask != 0))
}

/// `str(pathlib.PurePosixPath(text))`.
fn posix_path(text: &str) -> String {
    let rooted = text.starts_with('/');
    let double = text.starts_with("//") && !text.starts_with("///");
    let parts: Vec<&str> = text
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let body = parts.join("/");
    match (double, rooted) {
        (true, _) => format!("//{body}"),
        (false, true) => format!("/{body}"),
        _ if body.is_empty() => ".".to_owned(),
        _ => body,
    }
}

/// Microseconds in `timedelta(days=days)`.
fn micros(days: f64) -> i64 {
    (days * 86_400_000_000.0).round_ties_even() as i64
}

/// `(datetime(1899, 12, 30) + timedelta(days=days)).isoformat()`.
fn delphi_iso(days: f64) -> String {
    let total = micros(days);
    let day = total.div_euclid(86_400_000_000);
    let rest = total.rem_euclid(86_400_000_000);
    // Days from 1970-01-01 (civil-from-days, Howard Hinnant's algorithm).
    let z = day - 25_569 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let secs = rest / 1_000_000;
    let us = rest % 1_000_000;
    let mut out = format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    );
    if us != 0 {
        out.push_str(&format!(".{us:06}"));
    }
    out
}

pub fn dump(p: &Project) -> Value {
    let mut out = Map::new();
    let version = p.version().map_or(Value::Null, |v| {
        let build = v.build.map_or("None".to_owned(), |b| b.to_string());
        json!(format!(
            "FLVersion(major={}, minor={}, patch={}, build={build})",
            v.major, v.minor, v.patch
        ))
    });
    let fields = [
        ("version", version),
        ("ppq", json!(p.ppq())),
        ("format", json!(p.format())),
        ("channel_count", json!(p.channel_count())),
        ("tempo", json!(p.tempo())),
        ("title", json!(p.title())),
        ("artists", json!(p.artists())),
        ("genre", json!(p.genre())),
        ("comments", json!(p.comments())),
        ("url", json!(p.url())),
        ("pan_law", json!(p.pan_law_value())),
        ("main_pitch", json!(p.main_pitch())),
        ("main_volume", json!(p.main_volume())),
        ("looped", json!(p.looped())),
        ("show_info", json!(p.show_info())),
        ("licensed", json!(p.licensed())),
        ("licensee", json!(p.licensee())),
        ("data_path", json!(p.data_path().map(|d| posix_path(&d)))),
        (
            "created_on",
            json!(p.timestamp().map(|(c, _)| delphi_iso(c))),
        ),
        (
            "time_spent",
            json!(p.timestamp().map(|(_, t)| micros(t) as f64 / 1e6)),
        ),
    ];
    for (k, v) in fields {
        out.insert(k.to_owned(), v);
    }
    let rack = p.channels();
    out.insert(
        "rack".into(),
        json!({"swing": rack.swing(), "height": rack.height(), "fit_to_steps": rack.fit_to_steps()}),
    );
    out.insert("groups".into(), json!(rack.groups()));
    out.insert(
        "channels".into(),
        match rack.channels() {
            Ok(list) => Value::Array(list.iter().map(channel).collect()),
            Err(_) => json!({"error": "ModelError"}),
        },
    );
    let patterns = p.patterns();
    out.insert(
        "patterns_info".into(),
        json!({"play_cut_notes": patterns.play_cut_notes(), "current": patterns.current()}),
    );
    out.insert(
        "patterns".into(),
        Value::Array(patterns.patterns().iter().map(pattern).collect()),
    );
    let arrs = p.arrangements();
    let list = arrs.arrangements();
    let current = arrs.current_index().and_then(|i| {
        let list = list.as_ref().ok()?;
        let len = i64::try_from(list.len()).ok()?;
        let i = if i < 0 { i + len } else { i };
        list.get(usize::try_from(i).ok()?)?.iid()
    });
    let (num, beat) = arrs.time_signature();
    out.insert(
        "arrangements_info".into(),
        json!({
            "loop_pos": arrs.loop_pos().map(|(a, b)| json!([a, b])),
            "max_tracks": arrs.max_tracks(),
            "time_signature": {"num": num, "beat": beat},
            "current": current,
        }),
    );
    out.insert(
        "arrangements".into(),
        match &list {
            Ok(list) => Value::Array(list.iter().map(arrangement).collect()),
            Err(_) => json!({"error": "ModelNotFound"}),
        },
    );
    let mixer = p.mixer();
    out.insert(
        "mixer".into(),
        json!({
            "apdc": mixer.apdc(),
            "max_inserts": mixer.max_inserts(),
            "max_slots": mixer.max_slots(),
            "inserts": mixer.inserts().iter().map(insert).collect::<Vec<_>>(),
        }),
    );
    Value::Object(out)
}

fn plugin(p: Option<Plugin<'_>>) -> Value {
    let Some(p) = p else { return Value::Null };
    let mut out = vec![("class", json!(p.class()))];
    if matches!(p.data, PluginData::Vst(_)) {
        out.extend([
            ("name", json!(p.name())),
            ("vendor", json!(p.vendor())),
            ("plugin_path", json!(p.plugin_path())),
            ("fourcc", json!(p.fourcc())),
            ("state", json!(p.state().map(|b| json!({"bytes": b.len()})))),
            ("guid", json!(p.guid().map(|b| json!({"bytes": b.len()})))),
            ("options", vst_options(&p)),
        ]);
    }
    if let PluginData::Native(native) = p.data {
        out.push(("params", native_params(native)));
    }
    let w = |bit| json!(p.wrapper_flag(bit));
    out.extend([
        ("generator", json!(p.generator())),
        ("visible", json!(p.visible())),
        ("width", json!(p.width())),
        ("height", json!(p.height())),
        ("page", json!(p.page())),
        ("compact", w(wrapper_flags::HIDE_SETTINGS)),
        ("demo_mode", w(wrapper_flags::DEMO_MODE)),
        ("detached", w(wrapper_flags::DETACHED)),
        ("disabled", w(wrapper_flags::DISABLED)),
        ("directx", w(wrapper_flags::DIRECTX)),
        ("minimized", w(wrapper_flags::MINIMIZED)),
        ("multithreaded", w(wrapper_flags::THREADED_PROCESSING)),
        ("smart_disable", w(wrapper_flags::SMART_DISABLE)),
    ]);
    obj(out)
}

fn vst_options(p: &Plugin<'_>) -> Value {
    use musiccraft_flp::plugin::vst_flags as f;
    use musiccraft_flp::plugin::vst_flags2 as f2;
    let on = |bit| json!(p.vst_flag(bit));
    let off = |bit| json!(p.vst_flag(bit).map(|b| !b));
    let on2 = |bit| json!(p.vst_flag2(bit));
    json!({
        "automation": {"notify_changes": off(f::DONT_NOTIFY_CHANGES)},
        "compatibility": {
            "buffers_maxsize": on2(f2::USE_MAX_FROM_HOST),
            "fast_idle": p.fast_idle(),
            "fixed_buffers": on(f::FIXED_SIZE_BUFFERS),
            "process_maximum": on2(f2::PROCESS_MAX_SIZE),
            "reset_on_transport": off(f::DONT_RESET_ON_TRANSPORT),
            "send_loop": on(f::SEND_LOOP_POS),
            "use_time_offset": off(f::DONT_USE_TIME_OFFSET),
        },
        "midi": {
            "input": p.midi_input(), "output": p.midi_output(), "pb_range": p.pb_range(),
            "send_modx": on(f::SEND_MOD_X), "send_pb": on(f::SEND_PB_RANGE),
            "send_release": off(f::DONT_SEND_REL_VELO),
        },
        "processing": {
            "allow_sd": on(f::ALLOW_SMART_DISABLE), "bridged": on(f::LOAD_BRIDGED),
            "external": on(f::EXTERNAL_WINDOW), "keep_state": off(f::DONT_KEEP_CPU_STATE),
            "multithreaded": on(f::ALLOW_THREADED), "notify_render": on(f::NOTIFY_RENDER),
            "process_inactive": on(f::PROCESS_INACTIVE),
        },
        "ui": {
            "accept_drop": on(f::ACCEPT_FILE_DROP), "always_update": on(f::UPDATE_WHEN_HIDDEN),
            "dpi_aware": on(f::DPI_AWARE_BRIDGED), "scale_editor": on(f::SCALE_EDITOR),
        },
    })
}

/// A `construct` enum field: its label, or the number when it has none.
fn label(value: u32, labels: &[&str]) -> Value {
    usize::try_from(value)
        .ok()
        .and_then(|i| labels.get(i))
        .map_or(json!(value), |l| json!(l))
}

fn native_params(native: &Native) -> Value {
    match native {
        Native::BooBass { bass, mid, high } => json!({"bass": bass, "mid": mid, "high": high}),
        Native::FruitKick {
            max_freq,
            min_freq,
            freq_decay,
            amp_decay,
            click,
            distortion,
        } => json!({
            "max_freq": max_freq, "min_freq": min_freq, "freq_decay": freq_decay,
            "amp_decay": amp_decay, "click": click, "distortion": distortion,
        }),
        Native::FruityBalance { pan, volume } => json!({"pan": pan, "volume": volume}),
        Native::FruityBloodOverdrive {
            pre_band,
            color,
            pre_amp,
            x100,
            post_filter,
            post_gain,
        } => json!({
            "pre_band": pre_band, "color": color, "pre_amp": pre_amp, "x100": x100,
            "post_filter": post_filter, "post_gain": post_gain,
        }),
        Native::FruityCenter { enabled } => json!({"enabled": enabled}),
        Native::FruityFastDist {
            pre,
            threshold,
            kind,
            mix,
            post,
        } => json!({
            "pre": pre, "threshold": threshold, "kind": label(*kind, &["A", "B"]),
            "mix": mix, "post": post,
        }),
        Native::FruityNotebook2 {
            active_page,
            pages,
            editable,
        } => json!({
            "active_page": active_page,
            "editable": editable,
            "pages": pages.iter().map(|(index, text)| json!({
                "index": index, "length": text.encode_utf16().count(), "value": text,
            })).collect::<Vec<_>>(),
        }),
        Native::FruitySend {
            pan,
            dry,
            volume,
            send_to,
        } => json!({"pan": pan, "dry": dry, "volume": volume, "send_to": send_to}),
        Native::FruitySoftClipper { threshold, post } => {
            json!({"threshold": threshold, "post": post})
        }
        Native::FruityStereoEnhancer {
            pan,
            volume,
            stereo_separation,
            phase_offset,
            effect_position,
            phase_inversion,
        } => json!({
            "pan": pan, "volume": volume, "stereo_separation": stereo_separation,
            "phase_offset": phase_offset,
            "effect_position": label(*effect_position, &["pre", "post"]),
            "phase_inversion": label(*phase_inversion, &["none", "left", "right"]),
        }),
        Native::Plucked {
            decay,
            color,
            normalize,
            gate,
            widen,
        } => json!({
            "decay": decay, "color": color, "normalize": normalize, "gate": gate, "widen": widen,
        }),
        Native::Soundgoodizer { mode, amount } => {
            json!({"mode": label(*mode, &["A", "B", "C", "D"]), "amount": amount})
        }
    }
}

fn tracking(t: Option<Tracking>) -> Value {
    let t = t.unwrap_or_default();
    json!({"middle_value": t.middle_value, "pan": t.pan, "mod_x": t.mod_x, "mod_y": t.mod_y})
}

fn channel(ch: &Channel<'_>) -> Value {
    let mut out = vec![
        ("class", json!(ch.kind.class())),
        ("iid", json!(ch.iid)),
        ("name", json!(ch.name())),
        ("internal_name", json!(ch.internal_name())),
        ("display_name", json!(ch.display_name())),
        ("enabled", json!(ch.enabled())),
        ("volume", json!(ch.volume())),
        ("pan", json!(ch.pan())),
        ("color", rgba(ch.color())),
        ("icon", json!(ch.icon())),
        ("locked", json!(ch.locked())),
        ("zipped", json!(ch.zipped())),
        ("group", json!(ch.group)),
    ];
    let sampler = ch.kind == ChannelKind::Sampler;
    if sampler || ch.kind == ChannelKind::Instrument {
        out.extend(instrument_props(ch));
    }
    if sampler {
        out.extend(sampler_props(ch));
    }
    if ch.kind == ChannelKind::Instrument {
        out.push(("plugin", plugin(ch.plugin())));
    }
    if ch.kind == ChannelKind::Layer {
        let flags = ch.layer_flags();
        out.extend([
            ("children", json!(ch.children())),
            ("crossfade", bit(flags, 2)),
            ("random", bit(flags, 1)),
        ]);
    }
    if ch.kind == ChannelKind::Automation {
        let points: Vec<Value> = ch
            .points()
            .unwrap_or_default()
            .iter()
            .map(|p| json!([p.position, p.value, f64::from(p.tension)]))
            .collect();
        out.extend([
            ("points", json!(points)),
            ("lfo_amount", json!(ch.automation_lfo_amount())),
        ]);
    }
    obj(out)
}

fn instrument_props(ch: &Channel<'_>) -> Vec<(&'static str, Value)> {
    let poly = ch.polyphony().unwrap_or_default();
    let arp = ch.arp().cloned().unwrap_or_default();
    let delay = ch.delay().unwrap_or_default();
    let dmod = ch.delay_mod();
    let dflags = ch.delay_flags();
    let levels = ch.level_adjusts().unwrap_or_default();
    let tracking_value = ch.tracking().map_or(json!({}), |(volume, keyboard)| {
        let mut map = Map::new();
        map.insert("volume".into(), tracking(volume));
        if keyboard.is_some() {
            map.insert("keyboard".into(), tracking(keyboard));
        }
        Value::Object(map)
    });
    vec![
        ("insert", json!(ch.insert())),
        ("pitch_shift", json!(ch.pitch_shift())),
        ("cut_group", json!(ch.cut_group().map(|(a, b)| [a, b]))),
        (
            "keyboard",
            json!({
                "fine_tune": ch.fine_tune(),
                "root_note": ch.root_note(),
                "main_pitch": ch.keyboard_main_pitch(),
                "add_root": ch.keyboard_add_root(),
                "key_region": ch.keyboard_key_region().map(|(a, b)| [a, b]),
            }),
        ),
        (
            "polyphony",
            json!({
                "max": poly.max, "slide": poly.slide,
                "mono": bit(poly.flags, 1), "porta": bit(poly.flags, 2),
            }),
        ),
        (
            "time",
            json!({
                "swing": ch.time_swing(), "gate": ch.time_gate(),
                "shift": ch.time_shift(), "full_porta": ch.time_full_porta(),
            }),
        ),
        (
            "arp",
            json!({
                "direction": arp.arp_direction, "range": arp.arp_range,
                "chord": arp.arp_chord, "time": arp.arp_time.map(f64::from),
                "gate": arp.arp_gate.map(f64::from), "slide": arp.arp_slide,
                "repeat": arp.arp_repeat,
            }),
        ),
        (
            "delay",
            json!({
                "echoes": delay.echoes, "feedback": delay.feedback, "pan": delay.pan,
                "pitch_shift": delay.pitch_shift, "time": delay.time,
                "mod_x": dmod.map(|m| m.0), "mod_y": dmod.map(|m| m.1),
                "fat_mode": bit(dflags, 4), "ping_pong": bit(dflags, 2),
            }),
        ),
        (
            "level_adjusts",
            json!({"pan": levels.pan, "volume": levels.volume, "mod_x": levels.mod_x, "mod_y": levels.mod_y}),
        ),
        ("tracking", tracking_value),
    ]
}

const ENVELOPE_NAMES: [&str; 5] = ["Panning", "Volume", "Mod X", "Mod Y", "Pitch"];

fn envelope(e: &EnvelopeLfo) -> Value {
    json!({
        "enabled": e.envelope_enabled, "predelay": e.envelope_predelay,
        "attack": e.envelope_attack, "hold": e.envelope_hold, "decay": e.envelope_decay,
        "sustain": e.envelope_sustain, "release": e.envelope_release,
        "amount": e.envelope_amount, "synced": bit(e.flags, 1),
        "attack_tension": e.envelope_attack_tension,
        "decay_tension": e.envelope_decay_tension,
        "release_tension": e.envelope_release_tension,
    })
}

fn lfo(e: &EnvelopeLfo) -> Value {
    json!({
        "amount": e.lfo_amount, "attack": e.lfo_attack, "predelay": e.lfo_predelay,
        "speed": e.lfo_speed, "synced": bit(e.flags, 2), "retrig": bit(e.flags, 32),
        "shape": e.lfo_shape,
    })
}

fn named(list: &Option<Vec<EnvelopeLfo>>, each: fn(&EnvelopeLfo) -> Value) -> Value {
    list.as_ref().map_or(json!({}), |list| {
        Value::Object(
            ENVELOPE_NAMES
                .iter()
                .zip(list)
                .map(|(name, e)| ((*name).to_owned(), each(e)))
                .collect(),
        )
    })
}

fn sampler_props(ch: &Channel<'_>) -> Vec<(&'static str, Value)> {
    let params = ch.sampler_parameters().cloned().unwrap_or_default();
    let fx_flags = ch.sampler_int(id::FX_FLAGS);
    let sampler_flags = ch.sampler_int(id::SAMPLER_FLAGS);
    let filter = ch.filter().unwrap_or_default();
    let reverb = ch.reverb();
    let envelopes = ch.envelopes();
    vec![
        (
            "sample_path",
            json!(ch.sample_path().map(|s| posix_path(&s))),
        ),
        ("au_sample_rate", json!(ch.au_sample_rate())),
        (
            "fx",
            json!({
                "boost": ch.sampler_int(id::PREAMP),
                "clip": bit(fx_flags, 4),
                "crossfade": params.fx_crossfade,
                "cutoff": ch.sampler_int(id::CUTOFF),
                "fade_in": ch.sampler_int(id::FADE_IN),
                "fade_out": ch.sampler_int(id::FADE_OUT),
                "fade_stereo": bit(fx_flags, 1),
                "fix_trim": params.fx_fix_trim,
                "freq_tilt": ch.sampler_int(id::FREQ_TILT),
                "inverted": params.fx_inverted,
                "length": params.fx_length,
                "normalize": params.fx_normalize,
                "pogo": ch.sampler_int(id::POGO),
                "remove_dc": params.fx_remove_dc,
                "resonance": ch.sampler_int(id::RESONANCE),
                "reverse": bit(fx_flags, 2),
                "ringmod": ch.ringmod().map(|(a, b)| [a, b]),
                "start": params.fx_start,
                "stereo_delay": ch.sampler_int(id::STEREO_DELAY),
                "swap_stereo": bit(fx_flags, 256),
                "trim": params.fx_trim,
            }),
        ),
        (
            "reverb",
            json!({"type": reverb.map(|r| r.0), "mix": reverb.map(|r| r.1)}),
        ),
        (
            "filter",
            json!({"mod_x": filter.filter_mod_x, "mod_y": filter.filter_mod_y, "type": filter.filter_type}),
        ),
        ("envelopes", named(&envelopes, envelope)),
        ("lfos", named(&envelopes, lfo)),
        (
            "playback",
            json!({
                "ping_pong_loop": ch.ping_pong_loop(),
                "start_offset": params.playback_start_offset,
                "use_loop_points": bit(sampler_flags, 8),
            }),
        ),
        (
            "stretching",
            json!({
                "mode": params.stretching_mode,
                "multiplier": params.stretching_multiplier,
                "pitch": params.stretching_pitch,
                "time": params.stretching_time.map(|t| [t.bars, t.beats, t.ticks]),
            }),
        ),
        (
            "content",
            json!({
                "declick_mode": params.content_declick_mode,
                "keep_on_disk": bit(sampler_flags, 256),
                "load_regions": bit(sampler_flags, 2),
                "load_slices": bit(sampler_flags, 4),
                "resample": bit(sampler_flags, 1),
            }),
        ),
    ]
}

fn marker(m: &TimeMarker<'_>) -> Value {
    let kind = m.is_signature().map(|s| {
        if s {
            musiccraft_flp::pattern::SIGNATURE_MARKER
        } else {
            0
        }
    });
    json!({
        "position": m.position(), "type": kind, "name": m.name(),
        "numerator": m.numerator(), "denominator": m.denominator(),
    })
}

fn note(n: &NoteItem) -> Value {
    json!([
        n.position,
        n.length,
        n.key,
        n.rack_channel,
        n.velocity,
        n.pan,
        n.fine_pitch,
        n.release,
        n.mod_x,
        n.mod_y,
        n.group,
        n.midi_channel,
        n.flags & NoteItem::SLIDE != 0
    ])
}

fn pattern(p: &Pattern<'_>) -> Value {
    json!({
        "iid": p.iid(), "name": p.name(), "length": p.length(), "looped": p.looped(),
        "color": rgba(p.color()),
        "notes": p.notes().iter().map(note).collect::<Vec<_>>(),
        "controllers": p.controllers().iter()
            .map(|c| json!([c.position, c.channel, f64::from(c.value)]))
            .collect::<Vec<_>>(),
        "timemarkers": p.timemarkers().iter().map(marker).collect::<Vec<_>>(),
    })
}

fn track(t: &Track<'_>) -> Value {
    let d = t.data().cloned().unwrap_or_default();
    let items: Vec<Value> = t
        .clips
        .iter()
        .map(|clip| {
            let (kind, iid) = match &clip.target {
                ClipRef::Pattern(p) => ("pattern", p.iid()),
                ClipRef::Channel(c) => ("channel", Some(c.iid)),
            };
            let it = clip.item;
            json!([
                kind,
                iid,
                it.position,
                it.length,
                it.group,
                [f64::from(it.start_offset), f64::from(it.end_offset)]
            ])
        })
        .collect();
    json!({
        "iid": d.iid, "name": t.name(), "enabled": d.enabled, "color": rgba(t.color()),
        "icon": d.icon, "height": t.height_percent(), "locked": d.locked,
        "content_locked": d.content_locked, "grouped": d.grouped, "motion": d.motion,
        "press": d.press, "trigger_sync": d.trigger_sync, "position_sync": d.position_sync,
        "queued": d.queued, "tolerant": d.tolerant, "items": items,
    })
}

fn arrangement(a: &Arrangement<'_>) -> Value {
    json!({
        "iid": a.iid(),
        "name": a.name(),
        "timemarkers": a.timemarkers().iter().map(marker).collect::<Vec<_>>(),
        "tracks": match a.tracks() {
            Ok(tracks) => Value::Array(tracks.iter().map(track).collect()),
            Err(_) => json!({"error": "ModelNotFound"}),
        },
    })
}

fn slot(s: &Slot<'_>) -> Value {
    json!({
        "index": s.index(), "name": s.name(), "internal_name": s.internal_name(),
        "enabled": s.enabled(), "mix": s.mix(), "color": rgba(s.color()), "icon": s.icon(),
        "plugin": plugin(s.plugin()),
    })
}

fn insert(i: &Insert<'_>) -> Value {
    let band = |n| {
        let [gain, freq, reso] = i.eq_band(n);
        json!({"gain": gain, "freq": freq, "reso": reso})
    };
    json!({
        "iid": i.iid, "name": i.name(), "volume": i.volume(), "pan": i.pan(),
        "stereo_separation": i.stereo_separation(), "enabled": i.enabled(),
        "bypassed": i.bypassed(), "channels_swapped": i.channels_swapped(),
        "polarity_reversed": i.polarity_reversed(), "is_solo": i.is_solo(),
        "locked": i.locked(), "separator_shown": i.separator_shown(),
        "color": rgba(i.color()), "icon": i.icon(), "input": i.input(),
        "output": i.output(), "dock": i.dock().map(|d| d as u8),
        "routes": i.routes(),
        "eq": {"low": band(0), "mid": band(1), "high": band(2)},
        "slots": i.slots().map(|s| s.iter().map(slot).collect::<Vec<_>>()),
    })
}

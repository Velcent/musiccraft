use std::sync::Arc;

use musiccraft_dsp::describe;
use musiccraft_params::{EFFECT_KINDS, EffectKind};
use nih_plug::prelude::{Param, Params};

use musiccraft_instruments::Kind;

use crate::antiserum::params::AntiserumParams;
use crate::ids::{
    ANTISERUM_CLAP_ID, CLAP_IDS, INSTRUMENT_CLAP_IDS, INSTRUMENT_KINDS, INSTRUMENT_NAMES, KINDS,
    vst3_class_id, vst3_instrument_class_id,
};
use crate::instrument::data::Shared as InstrumentShared;
use crate::instrument::params::InstrumentParams;
use crate::params::{EffectParams, HostParam, STATE_KEY};
use crate::shared::Shared;

fn params(kind: EffectKind) -> EffectParams {
    EffectParams::new(kind, Arc::new(Shared::new().0))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `clap-id vst3-class-id` per line, in plugin order: the effects, then the
/// instruments and Antiserum.
fn id_table() -> String {
    let effects = CLAP_IDS
        .iter()
        .map(|id| format!("{id} {}\n", hex(&vst3_class_id(id))));
    let instruments = INSTRUMENT_CLAP_IDS
        .iter()
        .chain([&ANTISERUM_CLAP_ID])
        .map(|id| format!("{id} {}\n", hex(&vst3_instrument_class_id(id))));
    effects.chain(instruments).collect()
}

#[test]
fn every_effect_but_the_routed_ones_is_exported() {
    let expected: Vec<EffectKind> = EFFECT_KINDS
        .into_iter()
        .filter(|k| {
            !matches!(
                k,
                EffectKind::Send | EffectKind::BandSplit | EffectKind::Rack
            )
        })
        .collect();
    assert_eq!(KINDS.to_vec(), expected);
    for (kind, id) in KINDS.iter().zip(CLAP_IDS) {
        assert_eq!(id, format!("org.musiccraft.{}", kind.name()));
    }
}

/// Never change an existing line. A new plugin adds one, written with
/// `MUSICCRAFT_BLESS_NAMES=1`.
#[test]
fn frozen_ids() {
    if std::env::var_os("MUSICCRAFT_BLESS_NAMES").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/frozen-ids.txt");
        std::fs::write(path, id_table()).unwrap();
        return;
    }
    let frozen = include_str!("../frozen-ids.txt");
    assert_eq!(
        id_table(),
        frozen,
        "plugin IDs changed; hosts would lose every saved instance"
    );
    let mut vst3: Vec<[u8; 16]> = CLAP_IDS.iter().map(|id| vst3_class_id(id)).collect();
    vst3.extend(
        INSTRUMENT_CLAP_IDS
            .iter()
            .chain([&ANTISERUM_CLAP_ID])
            .map(|id| vst3_instrument_class_id(id)),
    );
    let count = vst3.len();
    vst3.sort_unstable();
    vst3.dedup();
    assert_eq!(vst3.len(), count, "VST3 IDs are unique");
}

#[test]
fn instruments_are_named_like_project_files() {
    for (i, kind) in INSTRUMENT_KINDS.iter().enumerate() {
        assert_eq!(
            INSTRUMENT_CLAP_IDS[i],
            format!("org.musiccraft.{}", INSTRUMENT_NAMES[i])
        );
        // The name is the project file's instrument tag.
        let tag = match kind {
            Kind::Synth => "synth",
            Kind::DrumSynth => "drum_synth",
            Kind::Pluck => "pluck",
            Kind::Fm => "fm",
            Kind::Keys => "keys",
            Kind::Additive => "additive",
            Kind::Sampler => "sampler",
            Kind::Granular => "granular",
            Kind::Resynth => "resynth",
            Kind::Kit => "kit",
            Kind::Zones => "zones",
            Kind::Layer | Kind::External | Kind::MidiOut => "",
        };
        assert_eq!(INSTRUMENT_NAMES[i], tag);
    }
}

#[test]
fn instrument_parameters_are_the_kinds_plus_swing_and_bend() {
    for (kind, name) in INSTRUMENT_KINDS.into_iter().zip(INSTRUMENT_NAMES) {
        let p = InstrumentParams::new(kind, name, Arc::new(InstrumentShared::new(kind).0));
        let ids: Vec<String> = p.param_map().into_iter().map(|(id, _, _)| id).collect();
        let mut expected: Vec<String> = kind.params().map(str::to_owned).collect();
        expected.extend(["swing".to_owned(), "bend_range".to_owned()]);
        assert_eq!(ids, expected, "{name}");
        assert_eq!(
            p.values(),
            musiccraft_instruments::default_params(),
            "{name}"
        );
        // Text reads like the app's knobs and parses back.
        for slot in p.slots.iter().chain(&p.own) {
            let HostParam::Float(param) = &slot.param else {
                continue;
            };
            let n = param.preview_normalized(slot.spec.default);
            let text = param.normalized_value_to_string(n, true);
            assert_eq!(
                text,
                musiccraft_params::describe::display(&slot.spec, slot.spec.default)
            );
            let back = param
                .string_to_normalized_value(&text)
                .map(|n| param.preview_plain(n));
            let back = back.unwrap_or_else(|| panic!("{name} {}: {text}", slot.spec.name));
            let span = (slot.spec.max - slot.spec.min).abs();
            assert!(
                (back - slot.spec.default).abs() <= span * 0.01,
                "{name} {}: {text} -> {back}",
                slot.spec.name
            );
        }
        // The state names the instrument and its samples.
        let fields = p.serialize_fields();
        let json = &fields[STATE_KEY];
        assert!(
            json.contains(&format!(r#""instrument":"{name}""#)),
            "{json}"
        );
        assert_eq!(json.contains(r#""pads""#), kind == Kind::Kit, "{json}");
    }
}

#[test]
fn instrument_state_brings_back_samples() {
    let kind = Kind::Zones;
    let p = InstrumentParams::new(kind, "zones", Arc::new(InstrumentShared::new(kind).0));
    let mut data = p.shared.data();
    data.zones.push(musiccraft_patch::ZoneRef::new(
        "builtin:snare".to_owned(),
        62,
        70,
        64,
    ));
    p.shared.set_data(data.clone());
    let fields = p.serialize_fields();
    let other = InstrumentParams::new(kind, "zones", Arc::new(InstrumentShared::new(kind).0));
    other.deserialize_fields(&fields);
    assert_eq!(other.shared.data(), data);
    // Another instrument's state is ignored.
    let kit = InstrumentParams::new(
        Kind::Kit,
        "kit",
        Arc::new(InstrumentShared::new(Kind::Kit).0),
    );
    let before = kit.shared.data();
    kit.deserialize_fields(&fields);
    assert_eq!(kit.shared.data(), before);
}

#[test]
fn antiserum_parameters_are_its_table_and_its_state_is_a_patch() {
    let (p, _) = AntiserumParams::new();
    let ids: Vec<String> = p.param_map().into_iter().map(|(id, _, _)| id).collect();
    let expected: Vec<&str> = musiccraft_antiserum::params()
        .iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(ids, expected);
    assert_eq!(*p.values(), musiccraft_antiserum::params::defaults());
    // Typed text parses through the app's descriptions.
    let cutoff = musiccraft_antiserum::index_of("filter1_cutoff").unwrap();
    let text = musiccraft_antiserum_presets::text(cutoff, 2_500.0);
    let back = musiccraft_antiserum_presets::parse(cutoff, &text).unwrap();
    assert!((back - 2_500.0).abs() < 1.0, "{text} -> {back}");
    // A factory preset's data comes back through the state.
    let preset = musiccraft_antiserum_presets::find("Glass Reese")
        .unwrap()
        .clone();
    p.shared.set_patch(preset.clone());
    let fields = p.serialize_fields();
    let (other, _) = AntiserumParams::new();
    other.deserialize_fields(&fields);
    let restored = other.shared.patch();
    assert_eq!(restored.tables, preset.tables);
    assert_eq!(restored.lfo_shapes, preset.lfo_shapes);
    assert_eq!(restored.sequence, preset.sequence);
}

#[test]
fn parameters_follow_the_spec_table() {
    for kind in KINDS {
        let p = params(kind);
        let map = p.param_map();
        let ids: Vec<&str> = map.iter().map(|(id, _, _)| id.as_str()).collect();
        let mut expected = vec!["bypass"];
        expected.extend(
            kind.params()
                .iter()
                .filter(|s| s.name != "target" && !s.name.ends_with("_target"))
                .map(|s| s.name),
        );
        assert_eq!(ids, expected, "{kind}");
        // The defaults are the app's.
        assert_eq!(p.values(), kind.defaults(), "{kind}");
    }
}

#[test]
fn routing_becomes_switches() {
    let compressor = params(EffectKind::Compressor);
    let sidechain = compressor.slot("sidechain").unwrap();
    assert_eq!((sidechain.spec.min, sidechain.spec.max), (0.0, 1.0));
    let HostParam::Int(switch) = &sidechain.param else {
        panic!("an integer switch");
    };
    assert_eq!(switch.normalized_value_to_string(1.0, true), "on");
    assert_eq!(switch.string_to_normalized_value("off"), Some(0.0));
    assert!(params(EffectKind::PitchCorrect).slot("follow").is_some());
}

#[test]
fn values_round_trip_and_read_like_the_app() {
    for kind in KINDS {
        let p = params(kind);
        for slot in &p.slots {
            let spec = slot.spec;
            if crate::plugin::keyed(kind) && spec.name == "sidechain" || spec.name == "follow" {
                continue;
            }
            for value in [spec.min, spec.default, spec.max] {
                let (normalized, back, text, parsed) = match &slot.param {
                    HostParam::Float(param) => {
                        let n = param.preview_normalized(value);
                        let text = param.normalized_value_to_string(n, true);
                        (
                            n,
                            param.preview_plain(n),
                            text.clone(),
                            param
                                .string_to_normalized_value(&text)
                                .map(|n| param.normalized_value_to_string(n, true)),
                        )
                    }
                    HostParam::Int(param) => {
                        let v = value.round() as i32;
                        let n = param.preview_normalized(v);
                        let text = param.normalized_value_to_string(n, true);
                        (
                            n,
                            param.preview_plain(n) as f32,
                            text.clone(),
                            param
                                .string_to_normalized_value(&text)
                                .map(|n| param.normalized_value_to_string(n, true)),
                        )
                    }
                };
                let name = format!("{kind}/{} {value}", spec.name);
                assert!((0.0..=1.0).contains(&normalized), "{name}");
                assert!(
                    (back - value).abs() <= 1e-3 * (spec.max - spec.min).abs().max(1.0),
                    "{name}: {back}"
                );
                assert_eq!(text, describe::text(kind, &spec, value), "{name}");
                // Typed text parses back to a value that reads the same.
                let parsed = parsed.unwrap_or_else(|| panic!("{name}: {text:?} parses"));
                assert_eq!(parsed, text, "{name}");
            }
        }
    }
}

#[test]
fn frequencies_and_times_are_skewed_like_the_app() {
    let p = params(EffectKind::Filter);
    let HostParam::Float(cutoff) = &p.slot("cutoff").unwrap().param else {
        panic!("cutoff is continuous");
    };
    let spec = p.slot("cutoff").unwrap().spec;
    let middle = (spec.min * spec.max).sqrt();
    assert!((cutoff.preview_normalized(middle) - 0.5).abs() < 1e-3);
    let p = params(EffectKind::Delay);
    let slot = p
        .slots
        .iter()
        .find(|s| matches!(s.spec.unit, "s" | "ms") && !s.spec.integer)
        .unwrap();
    let HostParam::Float(time) = &slot.param else {
        panic!("a time");
    };
    let quarter = slot.spec.from_unit(0.25);
    assert!((time.preview_normalized(quarter) - 0.25).abs() < 1e-3);
}

#[test]
fn state_names_the_effect_and_restores_the_room() {
    let p = params(EffectKind::Convolution);
    p.shared.set_impulse("cathedral");
    let fields = p.serialize_fields();
    let json = &fields[STATE_KEY];
    assert!(json.contains("\"effect\":\"convolution\""), "{json}");
    assert!(json.contains("\"format\":1"), "{json}");
    assert!(json.contains("\"impulse\":\"cathedral\""), "{json}");

    let other = params(EffectKind::Convolution);
    assert_eq!(other.shared.impulse_name(), "hall");
    other.deserialize_fields(&fields);
    assert_eq!(other.shared.impulse_name(), "cathedral");

    // A state for another effect is ignored.
    let reverb = params(EffectKind::Reverb);
    let state = reverb.state();
    assert!(state.impulse.is_none());
    let mut fields = reverb.serialize_fields();
    fields.insert(
        STATE_KEY.to_owned(),
        fields[STATE_KEY].replace("reverb", "convolution"),
    );
    let fresh = params(EffectKind::Convolution);
    fresh.deserialize_fields(&fields);
    assert_eq!(fresh.shared.impulse_name(), "hall");
}

#[test]
fn editor_sizes_fit_every_effect() {
    for kind in KINDS {
        let (w, h) = crate::editor::size(&params(kind));
        assert!(w >= 400 && (300..=900).contains(&h), "{kind}: {w}×{h}");
    }
}

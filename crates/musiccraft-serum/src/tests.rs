//! Conversion tests on synthetic presets (`fixtures`).

use musiccraft_antiserum::params::{index_of, params};

use crate::cbor::Value;
use crate::fixtures::{self, map, module, n, t};
use crate::{Assets, Conversion, Error, Preset, convert, read};

fn param(c: &Conversion, name: &str) -> f32 {
    c.patch.params.get(name).copied().unwrap_or_else(|| {
        index_of(name)
            .and_then(|i| params().get(i))
            .map_or(f32::NAN, |s| s.default)
    })
}

fn has_note(c: &Conversion, text: &str) -> bool {
    c.notes.iter().any(|n| n.contains(text))
}

fn convert_bytes(data: &[u8]) -> Conversion {
    convert(&read(data, "stem").expect("reads"), &Assets::none())
}

/// Every value the conversion set is inside its parameter's range.
fn in_range(c: &Conversion) {
    for (name, &value) in &c.patch.params {
        let spec = index_of(name)
            .and_then(|i| params().get(i))
            .unwrap_or_else(|| panic!("unknown parameter {name}"));
        assert!(
            (spec.min..=spec.max).contains(&value),
            "{name} = {value} outside {}..{}",
            spec.min,
            spec.max
        );
    }
}

#[test]
fn serum1_reads_name_tables_and_parameters() {
    let data = fixtures::serum1(
        "Test Bass",
        &[("A Vol", 0.5), ("A Octave", 0.75)],
        "Analog/Sine.wav",
        &[],
    );
    let Ok(Preset::Serum1(p)) = read(&data, "file") else {
        panic!("not read as Serum 1");
    };
    assert_eq!(p.name, "Test Bass");
    assert_eq!(p.wavetable_a, "Analog/Sine.wav");
    assert!((p.value("A Vol") - 0.5).abs() < 1e-6);
    assert_eq!(p.fx_rack().first(), Some(&"distortion"));
}

#[test]
fn serum1_converts_oscillators_and_levels() {
    let c = convert_bytes(&fixtures::serum1(
        "Lead",
        &[("A Vol", 0.5)],
        "Analog/Sine.wav",
        &[],
    ));
    assert_eq!(c.patch.name, "Lead");
    assert!(c.patch.tags.contains(&"serum".to_owned()));
    assert!((param(&c, "osc1_on") - 1.0).abs() < 1e-6);
    // Serum's level knob is quadratic in amplitude.
    assert!((param(&c, "osc1_level") - 0.25).abs() < 1e-3);
    assert!((param(&c, "osc3_on")).abs() < 1e-6);
    // A basic shape is generated, so nothing is missing.
    assert!(!has_note(&c, "missing"), "{:?}", c.notes);
    assert!(c.patch.table(0).starts_with("data:"));
    in_range(&c);
}

#[test]
fn serum1_missing_table_is_a_saw_with_a_note() {
    let c = convert_bytes(&fixtures::serum1("Pad", &[], "Some Pack/Glass.wav", &[]));
    assert!(
        has_note(&c, "missing: wavetable Some Pack/Glass.wav"),
        "{:?}",
        c.notes
    );
    assert!(c.patch.table(0).starts_with("data:"));
}

#[test]
fn serum1_modulation_routes() {
    // Source 5 is LFO 1.
    let c = convert_bytes(&fixtures::serum1(
        "Wobble",
        &[],
        "",
        &[(0, 5, "Fil Cutoff", 0.5)],
    ));
    let dest = index_of("filter1_cutoff").expect("parameter") + 1;
    assert!((param(&c, "mod1_dest") - dest as f32).abs() < 1e-6);
    assert!(param(&c, "mod1_amount") > 0.0);
    in_range(&c);
}

#[test]
fn serum1_effects_fill_the_rack_in_order() {
    let c = convert_bytes(&fixtures::serum1(
        "Wet",
        &[
            ("Dly Enable", 1.0),
            ("Rev Enable", 1.0),
            ("Dist Enable", 1.0),
        ],
        "",
        &[],
    ));
    // Rack order: distortion, …, delay, …, reverb.
    let slots: Vec<f32> = (1..=3).map(|s| param(&c, &format!("fx_slot{s}"))).collect();
    assert_eq!(slots, [2.0, 6.0, 11.0]);
    in_range(&c);
}

#[test]
fn another_plugins_fxp_is_refused() {
    let mut data = fixtures::serum1("X", &[], "", &[]);
    data[16..20].copy_from_slice(b"ABCD");
    assert!(matches!(read(&data, "x"), Err(Error::NotSerum(_))));
    assert!(matches!(
        read(b"{\"name\": 1}", "x"),
        Err(Error::NotSerum(_))
    ));
}

#[test]
fn a_cut_off_preset_is_an_error_not_a_panic() {
    let data = fixtures::serum1("X", &[], "", &[]);
    for len in [0, 10, 60, 100, data.len() / 2] {
        assert!(read(&data[..len], "x").is_err(), "length {len}");
    }
    let data = fixtures::serum2("X", &[], &map(vec![]));
    for len in [9, 20, data.len() - 3] {
        assert!(read(&data[..len], "x").is_err(), "length {len}");
    }
}

fn serum2_state() -> Value {
    let reverb = map(vec![
        ("type", n(6.0)),
        (
            "FXReverb",
            module(
                vec![
                    ("kParamType", t("kHall")),
                    ("kParamWet", n(40.0)),
                    ("kParamPreDelay", n(0.02)),
                ],
                vec![],
            ),
        ),
    ]);
    let delay = map(vec![("type", n(4.0)), ("FXDelay", module(vec![], vec![]))]);
    let comp = map(vec![
        ("type", n(5.0)),
        ("FXComp", module(vec![("kParamMultiband", n(1.0))], vec![])),
    ]);
    let bode = map(vec![("type", n(10.0)), ("FXBode", module(vec![], vec![]))]);
    map(vec![
        (
            "Global0",
            module(vec![("kParamMasterVolume", n(0.5))], vec![]),
        ),
        (
            "Oscillator0",
            module(
                vec![("kParamVolume", n(0.5)), ("kParamOctave", n(-1.0))],
                vec![(
                    "WTOsc0",
                    map(vec![
                        ("relativePathToWT", t("S2 Tables/Analog/Saw.wav")),
                        ("plainParams", map(vec![("kParamWarpMenu", t("kSync"))])),
                    ]),
                )],
            ),
        ),
        (
            "VoiceFilter0",
            module(
                vec![
                    ("kParamEnable", n(1.0)),
                    ("kParamType", t("L24")),
                    ("kParamFreq", n(0.5)),
                ],
                vec![],
            ),
        ),
        (
            "LFO0",
            map(vec![(
                "curveData",
                map(vec![
                    ("xVals", Value::Array(vec![n(0.0), n(0.5), n(1.0)])),
                    ("yVals", Value::Array(vec![n(1.0), n(0.0), n(1.0)])),
                    ("curveVals", Value::Array(vec![n(0.5), n(0.5)])),
                ]),
            )]),
        ),
        (
            "FXRack0",
            map(vec![("FX", Value::Array(vec![delay, reverb, comp, bode]))]),
        ),
        (
            "FXRack1",
            map(vec![(
                "FX",
                Value::Array(vec![map(vec![
                    ("type", n(3.0)),
                    ("FXChorus", module(vec![], vec![])),
                ])]),
            )]),
        ),
        (
            "ModSlot0",
            map(vec![
                ("source", Value::Array(vec![n(6.0)])),
                ("destModuleTypeString", t("VoiceFilter")),
                ("destModuleID", n(0.0)),
                ("destModuleParamName", t("kParamFreq")),
                ("plainParams", map(vec![("kParamAmount", n(50.0))])),
            ]),
        ),
        (
            "ModSlot1",
            map(vec![
                ("source", Value::Array(vec![n(37.0)])),
                ("destModuleTypeString", t("Oscillator")),
                ("destModuleID", n(0.0)),
                ("destModuleParamName", t("kParamVolume")),
                ("plainParams", map(vec![("kParamAmount", n(20.0))])),
            ]),
        ),
    ])
}

#[test]
fn serum2_reads_header_and_state() {
    let data = fixtures::serum2("Neon Pluck", &["Pluck", "Bright"], &serum2_state());
    let Ok(Preset::Serum2(p)) = read(&data, "file") else {
        panic!("not read as Serum 2");
    };
    assert_eq!(p.name, "Neon Pluck");
    assert_eq!(p.tags, ["Pluck", "Bright"]);
    assert!((p.param("Oscillator0", "kParamVolume", 0.0) - 0.5).abs() < 1e-9);
    assert_eq!(p.osc_type(0), "kOsc_Wavetable");
}

#[test]
fn serum2_converts_voice_section() {
    let c = convert_bytes(&fixtures::serum2("Neon Pluck", &["Pluck"], &serum2_state()));
    assert_eq!(c.patch.name, "Neon Pluck");
    assert!(c.patch.tags.contains(&"pluck".to_owned()));
    assert!((param(&c, "osc1_level") - 0.25).abs() < 1e-3);
    assert!((param(&c, "osc1_octave") + 1.0).abs() < 1e-6);
    assert!((param(&c, "osc1_warp1") - 1.0).abs() < 1e-6, "sync warp");
    assert!((param(&c, "filter1_on") - 1.0).abs() < 1e-6);
    assert!((param(&c, "filter1_type") - 1.0).abs() < 1e-6, "LP24");
    // Envelope 1's defaults carry over.
    assert!((param(&c, "env1_decay") - 2.0).abs() < 1e-3);
    assert!((param(&c, "lfo1_shape") - 7.0).abs() < 1e-6);
    assert!(c.patch.lfo_shape(0).is_some());
    // The saw is a basic shape: generated, no note.
    assert!(!has_note(&c, "missing"), "{:?}", c.notes);
    in_range(&c);
}

#[test]
fn serum2_converts_effects_and_reports_the_rest() {
    let c = convert_bytes(&fixtures::serum2("Neon Pluck", &[], &serum2_state()));
    let slots: Vec<f32> = (1..=4).map(|s| param(&c, &format!("fx_slot{s}"))).collect();
    // Delay, reverb, multiband; the frequency shifter has no counterpart.
    assert_eq!(slots, [6.0, 11.0, 8.0, 0.0]);
    assert!((param(&c, "fx_reverb_mix") - 0.4).abs() < 1e-4);
    assert!((param(&c, "fx_reverb_predelay") - 20.0).abs() < 1e-3);
    assert!(
        (param(&c, "fx_delay_time") - 1.0).abs() < 1e-4,
        "default 1/4"
    );
    assert!(has_note(&c, "Bode"), "{:?}", c.notes);
    assert!(has_note(&c, "FX bus 1 (Chorus)"), "{:?}", c.notes);
    in_range(&c);
}

#[test]
fn serum2_modulation() {
    let c = convert_bytes(&fixtures::serum2("Neon Pluck", &[], &serum2_state()));
    let dest = index_of("filter1_cutoff").expect("parameter") + 1;
    assert!((param(&c, "mod1_dest") - dest as f32).abs() < 1e-6);
    // Release velocity has no counterpart.
    assert!(has_note(&c, "release velocity"), "{:?}", c.notes);
    assert!(param(&c, "mod2_dest").abs() < 1e-6);
}

#[test]
fn notes_are_not_repeated() {
    let c = convert_bytes(&fixtures::serum2("Neon Pluck", &[], &serum2_state()));
    let mut sorted = c.notes.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), c.notes.len());
}

/// A mono 16-bit WAV of two 2048-sample frames.
fn wav(path: &std::path::Path) {
    let samples: Vec<i16> = (0..4096)
        .map(|i| ((i % 2048) as f32 / 2048.0 * 2.0 - 1.0).mul_add(16_000.0, 0.0) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&44_100_u32.to_le_bytes());
    out.extend_from_slice(&88_200_u32.to_le_bytes());
    out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, out).expect("write wav");
}

#[test]
fn tables_are_found_by_path_then_by_name() {
    let dir = std::env::temp_dir().join(format!("musiccraft-serum-assets-{}", std::process::id()));
    let tables = dir.join("Tables").join("Pack");
    std::fs::create_dir_all(&tables).expect("dirs");
    wav(&tables.join("Glass.wav"));
    let assets = Assets::with_roots(vec![dir.clone()]);
    let exact = assets.table("Pack/Glass.wav");
    assert!(exact.note.is_none());
    // Moved to another folder: found by file name.
    let moved = assets.table("Other/glass.wav");
    assert!(moved.note.is_none());
    assert_eq!(moved.table, exact.table);
    assert_ne!(exact.table, crate::assets::builtin("saw"));
    let missing = assets.table("Other/Nothing.wav");
    assert!(missing.note.is_some());
    // A preset in a pack with a Tables folder above it finds that folder.
    let mut local = Assets::none();
    assert!(local.table("Pack/Glass.wav").note.is_some());
    local.add_local(&dir.join("Presets").join("Bass").join("a.fxp"));
    assert!(local.table("Pack/Glass.wav").note.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_file_names_the_preset_after_the_file_when_unnamed() {
    let dir = std::env::temp_dir().join(format!("musiccraft-serum-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("Unnamed Pad.fxp");
    std::fs::write(&path, fixtures::serum1("", &[], "", &[])).expect("write");
    let c = crate::import_file(&path, &Assets::none()).expect("imports");
    assert_eq!(c.patch.name, "Unnamed Pad");
    assert!(matches!(
        crate::import_file(&dir.join("absent.fxp"), &Assets::none()),
        Err(Error::Io(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

//! The exported instruments, loaded from disk by MusicCraft's own CLAP and
//! VST3 hosts as another DAW would (`docs/spec/plugin-instruments.md`): every
//! instrument is listed, plays notes exactly like the app's own voices (or
//! Antiserum itself), and restores its state. The library is built with
//! nih-plug's `assert_process_allocs` here, so any allocation on the audio
//! thread aborts the test.

mod common;

use std::path::PathBuf;

use musiccraft_antiserum::Antiserum;
use musiccraft_clap::ClapHost;
use musiccraft_engine::{External, ExternalTransport, TimeSig};
use musiccraft_instruments::live::Poly;
use musiccraft_instruments::{Kind, default_params};
use musiccraft_plugin_export::ids::{
    ANTISERUM_CLAP_ID, INSTRUMENT_CLAP_IDS, INSTRUMENT_KINDS, INSTRUMENT_TITLES,
    vst3_instrument_class_id,
};
use musiccraft_plugin_export::instrument::data::Shared;
use musiccraft_render::{PluginFactory, PluginKind, PluginRequest};
use musiccraft_vst3::Vst3Host;

const RATE: u32 = 48_000;
const BLOCK: usize = 256;
const BLOCKS: usize = 40;
const TEMPO: f64 = 120.0;
const BUNDLE: &str = "MusicCraft Instruments";

/// What a test plays: block, frame in the block, key, velocity (0 = off).
const NOTES: [(usize, u32, u8, f32); 4] = [
    (1, 100, 60, 0.8),
    (2, 10, 64, 0.6),
    (12, 0, 60, 0.0),
    (20, 200, 64, 0.0),
];

fn install(tag: &str) -> PathBuf {
    common::install(tag, BUNDLE)
}

fn vst3_id(clap_id: &str) -> String {
    common::vst3_id(vst3_instrument_class_id(clap_id))
}

fn request<'a>(id: &'a str, state: Option<&'a [u8]>) -> PluginRequest<'a> {
    PluginRequest {
        id,
        sample_rate: RATE,
        max_frames: BLOCK as u32,
        state,
    }
}

fn beats(block: usize) -> f64 {
    (block * BLOCK) as f64 / f64::from(RATE) * TEMPO / 60.0
}

fn transport(block: usize) -> ExternalTransport {
    ExternalTransport {
        playing: true,
        tempo_bpm: TEMPO,
        position_beats: beats(block),
        sample_rate: f64::from(RATE),
        time_signature: TimeSig::COMMON,
        bar_start_beats: (beats(block) / 4.0).floor() * 4.0,
        bar_number: (beats(block) / 4.0) as u32,
    }
}

/// [`NOTES`] through a hosted plugin: both channels, block after block.
fn hosted(processor: &mut dyn External) -> Vec<f32> {
    let mut out = Vec::new();
    for block in 0..BLOCKS {
        for &(b, frame, key, velocity) in &NOTES {
            if b == block {
                if velocity > 0.0 {
                    processor.note_on(frame, key, velocity, 0.0, None);
                } else {
                    processor.note_off(frame, key);
                }
            }
        }
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        processor.process(&mut l, &mut r, &transport(block));
        out.extend(l);
        out.extend(r);
    }
    out
}

/// The notes of `block` at or after `from`, in order, and where the next
/// one is.
fn split(block: usize, from: usize) -> (Vec<(u8, f32)>, usize) {
    let now: Vec<(u8, f32)> = NOTES
        .iter()
        .filter(|n| n.0 == block && n.1 as usize == from)
        .map(|n| (n.2, n.3))
        .collect();
    let next = NOTES
        .iter()
        .filter(|n| n.0 == block && n.1 as usize > from)
        .map(|n| n.1 as usize)
        .min()
        .unwrap_or(BLOCK);
    (now, next)
}

/// [`NOTES`] played directly by the app's voices.
fn voices(kind: Kind) -> Vec<f32> {
    let (shared, _) = Shared::new(kind);
    shared.set_sample_rate(RATE as f32);
    let instrument = shared.build();
    let values = default_params();
    let mut poly = Poly::new(RATE as f32);
    let mut out = Vec::new();
    for block in 0..BLOCKS {
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        let mut start = 0;
        while start < BLOCK {
            let (now, next) = split(block, start);
            for (key, velocity) in now {
                if velocity > 0.0 {
                    poly.note_on(key, velocity, &instrument, &values);
                } else {
                    poly.note_off(key);
                }
            }
            poly.render(
                &instrument,
                &values,
                &mut l[start..next],
                &mut r[start..next],
            );
            start = next;
        }
        out.extend(l);
        out.extend(r);
    }
    out
}

/// [`NOTES`] played directly by Antiserum with the default patch.
fn antiserum() -> Vec<f32> {
    let mut synth: Antiserum =
        musiccraft_antiserum_presets::synth(&musiccraft_patch::Patch::default(), RATE as f32);
    let mut out = Vec::new();
    for block in 0..BLOCKS {
        synth.set_tempo(TEMPO as f32);
        synth.set_beat_position(beats(block));
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        let mut start = 0;
        loop {
            let (now, next) = split(block, start);
            for (key, velocity) in now {
                if velocity > 0.0 {
                    synth.note_on(key, velocity);
                } else {
                    synth.note_off(key);
                }
            }
            if start < next {
                synth.process(&mut l[start..next], &mut r[start..next]);
            }
            if next >= BLOCK {
                break;
            }
            start = next;
        }
        out.extend(l);
        out.extend(r);
    }
    out
}

fn compare(name: &str, plugin: &[f32], direct: &[f32]) {
    assert_eq!(plugin.len(), direct.len());
    let peak = direct.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.01, "{name} makes sound ({peak})");
    let worst = plugin
        .iter()
        .zip(direct)
        .fold(0.0_f32, |w, (a, b)| w.max((a - b).abs()));
    assert!(plugin.iter().all(|s| s.is_finite()), "{name}");
    assert!(worst < 1e-4, "{name} differs by {worst} (peak {peak})");
}

fn check_all(host: &dyn PluginFactory, id_of: impl Fn(&str) -> String) {
    let catalog = host.catalog();
    let instruments: Vec<_> = catalog
        .iter()
        .filter(|p| p.kind == PluginKind::Instrument)
        .collect();
    assert_eq!(
        instruments.len(),
        INSTRUMENT_KINDS.len() + 1,
        "{:?}",
        host.scan_errors()
    );
    for (i, kind) in INSTRUMENT_KINDS.into_iter().enumerate() {
        let id = id_of(INSTRUMENT_CLAP_IDS[i]);
        let info = instruments
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("{id} listed"));
        assert_eq!(info.name, INSTRUMENT_TITLES[i]);
        assert_eq!(info.vendor, "MusicCraft");
        let mut live = host.instantiate(&request(&id, None)).unwrap();
        compare(
            INSTRUMENT_TITLES[i],
            &hosted(live.processor.as_mut()),
            &voices(kind),
        );
        live.processor.stop();
    }
    let id = id_of(ANTISERUM_CLAP_ID);
    assert!(
        instruments
            .iter()
            .any(|p| p.id == id && p.name == "Antiserum")
    );
    let mut live = host.instantiate(&request(&id, None)).unwrap();
    compare("Antiserum", &hosted(live.processor.as_mut()), &antiserum());
    live.processor.stop();
}

#[test]
fn clap_host_plays_every_instrument_like_the_app() {
    let dir = install("inst-clap");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    check_all(&host, str::to_owned);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn vst3_host_plays_every_instrument_like_the_app() {
    let dir = install("inst-vst3");
    let host = Vst3Host::with_folders(vec![dir.clone()]);
    check_all(&host, vst3_id);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

/// The host's ID for Antiserum parameter `name` (hosts list parameters in
/// table order).
fn antiserum_param(live: &musiccraft_render::LivePlugin, name: &str) -> u32 {
    let index = musiccraft_antiserum::index_of(name).unwrap();
    live.instance.params()[index].id
}

fn state_round_trip(host: &dyn PluginFactory, id_of: impl Fn(&str) -> String) {
    // A kit keeps its pads.
    let kit = id_of("org.musiccraft.kit");
    let mut live = host.instantiate(&request(&kit, None)).unwrap();
    let state = live.instance.save_state().expect("state");
    let text = String::from_utf8_lossy(&state);
    assert!(text.contains(r#"\"instrument\":\"kit\""#), "{text}");
    assert!(text.contains("builtin:kick"), "{text}");
    drop(live);
    assert!(host.instantiate(&request(&kit, Some(&state))).is_ok());

    // Antiserum keeps its parameters and patch.
    let id = id_of(ANTISERUM_CLAP_ID);
    let mut live = host.instantiate(&request(&id, None)).unwrap();
    let volume = antiserum_param(&live, "volume");
    live.processor.set_param(volume, 0.25);
    let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    live.processor.process(&mut l, &mut r, &transport(0));
    let _ = live.instance.poll();
    let value = live.instance.param_value(volume).unwrap();
    assert!((value - 0.25).abs() < 1e-4, "{value}");
    let state = live.instance.save_state().expect("state");
    let text = String::from_utf8_lossy(&state);
    assert!(text.contains(r#"\"instrument\":\"antiserum\""#), "{text}");
    assert!(
        text.contains(r#"\"tables\""#) || text.contains(r#"\"patch\""#),
        "{text}"
    );
    live.processor.stop();
    drop(live);
    let restored = host.instantiate(&request(&id, Some(&state))).unwrap();
    let value = restored.instance.param_value(volume).unwrap();
    assert!((value - 0.25).abs() < 1e-4, "restored {value}");
}

#[test]
fn state_round_trips_through_clap() {
    let dir = install("inst-clap-state");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    state_round_trip(&host, str::to_owned);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn state_round_trips_through_vst3() {
    let dir = install("inst-vst3-state");
    let host = Vst3Host::with_folders(vec![dir.clone()]);
    state_round_trip(&host, vst3_id);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

/// With an X display (Xvfb in CI), Antiserum's and an instrument's editors
/// open in a host window.
#[test]
fn instrument_editors_open_in_a_host_window() {
    if std::env::var_os("DISPLAY").is_none() {
        eprintln!("skipped: no DISPLAY");
        return;
    }
    let dir = install("inst-editor");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    for id in [
        ANTISERUM_CLAP_ID,
        "org.musiccraft.synth",
        "org.musiccraft.kit",
    ] {
        let mut live = host.instantiate(&request(id, None)).unwrap();
        assert!(live.instance.has_editor());
        live.instance.set_editor_open(true, id).unwrap();
        for _ in 0..10 {
            let _ = live.instance.poll();
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        live.instance.set_editor_open(false, id).unwrap();
        live.processor.stop();
    }
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

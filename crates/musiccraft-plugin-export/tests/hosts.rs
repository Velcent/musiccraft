//! The exported library, loaded from disk by MusicCraft's own CLAP and VST3
//! hosts exactly as another DAW would: every effect is listed, sounds the same
//! as `musiccraft_dsp::Effect` itself, takes parameter changes and restores
//! its state. The library is built with nih-plug's `assert_process_allocs`
//! here, so any allocation on the audio thread aborts the test.

mod common;

use std::path::Path;

use musiccraft_clap::ClapHost;
use musiccraft_dsp::{Effect, ProcessContext};
use musiccraft_engine::{External, ExternalTransport, TimeSig};
use musiccraft_plugin_export::ids::{CLAP_IDS, KINDS, vst3_class_id};
use musiccraft_render::{LivePlugin, PluginFactory, PluginKind, PluginRequest};
use musiccraft_vst3::Vst3Host;

const RATE: u32 = 48_000;
const BLOCK: usize = 256;
const BLOCKS: usize = 24;
const TEMPO: f64 = 120.0;
const BUNDLE: &str = "MusicCraft Effects";

fn install(tag: &str) -> std::path::PathBuf {
    common::install(tag, BUNDLE)
}

fn vst3_id(clap_id: &str) -> String {
    common::vst3_id(vst3_class_id(clap_id))
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

/// A test signal: two tones and a little noise, different on each side.
fn input(block: usize) -> (Vec<f32>, Vec<f32>) {
    let mut seed = 0x1234_5678_u32.wrapping_add(block as u32 * 7919);
    let mut noise = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
    };
    (0..BLOCK)
        .map(|i| {
            let t = (block * BLOCK + i) as f32 / RATE as f32;
            let tau = std::f32::consts::TAU;
            let l = 0.4 * (tau * 220.0 * t).sin() + 0.05 * noise();
            let r = 0.3 * (tau * 330.0 * t).sin() + 0.05 * noise();
            (l, r)
        })
        .unzip()
}

/// Largest difference between the plugin and the effect run directly, over
/// [`BLOCKS`] blocks at default settings.
fn difference(processor: &mut dyn External, kind: musiccraft_params::EffectKind) -> f32 {
    let mut direct = Effect::new(kind, RATE as f32);
    let values = kind.defaults();
    let mut worst: f32 = 0.0;
    for block in 0..BLOCKS {
        let (mut pl, mut pr) = input(block);
        let (mut dl, mut dr) = (pl.clone(), pr.clone());
        processor.process(&mut pl, &mut pr, &transport(block));
        let ctx = ProcessContext {
            sample_rate: RATE as f32,
            tempo_bpm: TEMPO,
            beats: beats(block),
        };
        direct.process(&mut dl, &mut dr, &values, ctx);
        for (a, b) in pl.iter().chain(&pr).zip(dl.iter().chain(&dr)) {
            assert!(a.is_finite(), "{kind}: non-finite output");
            worst = worst.max((a - b).abs());
        }
    }
    worst
}

fn check_all(host: &dyn PluginFactory, id_of: impl Fn(&str) -> String) {
    let catalog = host.catalog();
    let effects = catalog
        .iter()
        .filter(|p| p.kind == PluginKind::Effect)
        .count();
    assert_eq!(effects, KINDS.len(), "{:?}", host.scan_errors());
    for (kind, clap_id) in KINDS.iter().zip(CLAP_IDS) {
        let id = id_of(clap_id);
        let info = catalog
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("{id} listed"));
        assert_eq!(info.name, kind.title());
        assert_eq!(info.vendor, "MusicCraft");
        assert_eq!(info.kind, PluginKind::Effect, "{id}");
        let mut live = host.instantiate(&request(&id, None)).unwrap();
        let diff = difference(live.processor.as_mut(), *kind);
        assert!(
            diff < 1e-4,
            "{kind} differs by {diff} from the mixer effect"
        );
        // After a reset (the host stops and starts processing, as before a
        // render) the plugin sounds like a new effect again: no tails left.
        live.processor.stop();
        let again = difference(live.processor.as_mut(), *kind);
        assert!(
            again < 1e-4,
            "{kind} differs by {again} from a new effect after a reset"
        );
        live.processor.stop();
    }
}

#[test]
fn clap_host_loads_every_effect_and_it_sounds_like_the_mixer() {
    let dir = install("clap");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    check_all(&host, str::to_owned);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn vst3_host_loads_every_effect_and_it_sounds_like_the_mixer() {
    let dir = install("vst3");
    let host = Vst3Host::with_folders(vec![dir.clone()]);
    check_all(&host, vst3_id);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

/// The id the host gives parameter `name`, found by its label.
fn param_id(live: &LivePlugin, label: &str) -> u32 {
    let params = live.instance.params();
    params
        .iter()
        .find(|p| p.name == label)
        .unwrap_or_else(|| panic!("{label} in {params:?}"))
        .id
}

fn peak(data: &[f32]) -> f32 {
    data.iter().fold(0.0, |p, s| p.max(s.abs()))
}

fn parameters_and_state(host: &dyn PluginFactory, id: &str) {
    let mut live = host.instantiate(&request(id, None)).unwrap();
    let params = live.instance.params();
    assert_eq!(
        params.len(),
        1 + musiccraft_params::EffectKind::Filter.params().len()
    );
    let cutoff = param_id(&live, "Cutoff");
    // Hosts see continuous parameters as 0–1 (nih-plug normalizes them).
    let spec = musiccraft_params::EffectKind::Filter.params()[1];
    assert_eq!(spec.name, "cutoff");
    let range = musiccraft_plugin_export::params::float_range(&spec);
    let normalized = |hz: f32| f64::from(range.normalize(hz));
    let text = live.instance.format_param(cutoff, normalized(2_500.0));
    assert_eq!(text.as_deref(), Some("2.50 kHz"));
    // A low cutoff dulls a high tone.
    let low = normalized(60.0);
    live.processor.set_param(cutoff, low);
    let tone = |block: usize| -> (Vec<f32>, Vec<f32>) {
        let tau = std::f32::consts::TAU;
        (0..BLOCK)
            .map(|i| {
                let t = (block * BLOCK + i) as f32 / RATE as f32;
                let s = 0.5 * (tau * 5_000.0 * t).sin();
                (s, s)
            })
            .unzip()
    };
    let mut last = 1.0;
    for block in 0..BLOCKS {
        let (mut l, mut r) = tone(block);
        live.processor.process(&mut l, &mut r, &transport(block));
        last = peak(&l);
    }
    assert!(last < 0.05, "5 kHz through a 60 Hz low-pass: {last}");
    let _ = live.instance.poll();
    let value = live.instance.param_value(cutoff).unwrap();
    assert!((value - low).abs() < 1e-4, "{value} vs {low}");

    // The state brings the value back in a new instance.
    let state = live.instance.save_state().expect("state");
    let text = String::from_utf8_lossy(&state);
    // Our JSON is a string field inside nih-plug's state, so it is escaped.
    assert!(text.contains(r#"\"effect\":\"filter\""#), "{text}");
    live.processor.stop();
    drop(live);
    let restored = host.instantiate(&request(id, Some(&state))).unwrap();
    let value = restored.instance.param_value(cutoff).unwrap();
    assert!(
        (value - low).abs() < 1e-4,
        "restored cutoff {value} vs {low}"
    );
}

#[test]
fn parameters_and_state_round_trip_through_clap() {
    let dir = install("clap-state");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    parameters_and_state(&host, "org.musiccraft.filter");
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn parameters_and_state_round_trip_through_vst3() {
    let dir = install("vst3-state");
    let host = Vst3Host::with_folders(vec![dir.clone()]);
    parameters_and_state(&host, &vst3_id("org.musiccraft.filter"));
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

/// With an X display (Xvfb in CI), the host opens the plugin's own editor in
/// a window, as a DAW does.
#[test]
fn the_editor_opens_in_a_host_window() {
    if std::env::var_os("DISPLAY").is_none() {
        eprintln!("skipped: no DISPLAY");
        return;
    }
    let dir = install("editor");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    let mut live = host
        .instantiate(&request("org.musiccraft.eq", None))
        .unwrap();
    assert!(live.instance.has_editor());
    live.instance.set_editor_open(true, "Equalizer").unwrap();
    assert!(live.instance.editor_open());
    for _ in 0..20 {
        let _ = live.instance.poll();
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    live.instance.set_editor_open(false, "Equalizer").unwrap();
    assert!(!live.instance.editor_open());
    live.processor.stop();
    drop(live);
    drop(host);
    let _ = std::fs::remove_dir_all(Path::new(&dir));
}

/// The host id of the parameter whose label mentions `word`.
fn param_named(live: &LivePlugin, word: &str) -> u32 {
    let params = live.instance.params();
    params
        .iter()
        .find(|p| p.name.to_lowercase().contains(word))
        .unwrap_or_else(|| panic!("{word} in {params:?}"))
        .id
}

/// A loud, pulsing key: a 110 Hz tone on for half of every 4,800 frames.
fn key(block: usize) -> (Vec<f32>, Vec<f32>) {
    (0..BLOCK)
        .map(|i| {
            let n = block * BLOCK + i;
            let t = n as f32 / RATE as f32;
            let on = (n / 2_400) % 2 == 0;
            let s = if on {
                0.9 * (std::f32::consts::TAU * 110.0 * t).sin()
            } else {
                0.0
            };
            (s, 0.8 * s)
        })
        .unzip()
}

/// Largest differences between the plugin and the effect run directly with
/// the key, and between the effect with and without the key.
fn keyed_difference(
    processor: &mut dyn External,
    kind: musiccraft_params::EffectKind,
) -> (f32, f32) {
    let mut keyed = Effect::new(kind, RATE as f32);
    let mut unkeyed = Effect::new(kind, RATE as f32);
    let mut values = kind.defaults();
    if let Some(i) = kind.sidechain_index() {
        values[i] = 1.0;
    }
    let (mut worst, mut effect_of_key) = (0.0f32, 0.0f32);
    for block in 0..BLOCKS * 2 {
        let (mut pl, mut pr) = input(block);
        let (mut kl, mut kr) = (pl.clone(), pr.clone());
        let (mut ul, mut ur) = (pl.clone(), pr.clone());
        let (sl, sr) = key(block);
        processor.sidechain(&sl, &sr);
        processor.process(&mut pl, &mut pr, &transport(block));
        let ctx = ProcessContext {
            sample_rate: RATE as f32,
            tempo_bpm: TEMPO,
            beats: beats(block),
        };
        keyed.process_keyed(&mut kl, &mut kr, &values, ctx, Some((&sl, &sr)));
        unkeyed.process(&mut ul, &mut ur, &values, ctx);
        for ((a, b), c) in pl
            .iter()
            .chain(&pr)
            .zip(kl.iter().chain(&kr))
            .zip(ul.iter().chain(&ur))
        {
            worst = worst.max((a - b).abs());
            effect_of_key = effect_of_key.max((b - c).abs());
        }
    }
    (worst, effect_of_key)
}

/// The compressor and the vocoder hear the host's sidechain input once their
/// `sidechain` switch is on; the pitch corrector follows the notes the host
/// sends once `follow` is on. Both match the mixer's effects fed the same key
/// or note.
fn sidechain_and_follow(host: &dyn PluginFactory, id_of: impl Fn(&str) -> String) {
    use musiccraft_params::EffectKind;
    for kind in [EffectKind::Compressor, EffectKind::Vocoder] {
        let clap_id = KINDS
            .iter()
            .zip(CLAP_IDS)
            .find(|(k, _)| **k == kind)
            .map(|(_, id)| id)
            .unwrap();
        let mut live = host.instantiate(&request(&id_of(clap_id), None)).unwrap();
        let switch = param_named(&live, "sidechain");
        live.processor.set_param(switch, 1.0);
        let (diff, effect_of_key) = keyed_difference(live.processor.as_mut(), kind);
        assert!(effect_of_key > 0.01, "{kind}: the key changes the sound");
        assert!(diff < 1e-4, "{kind} keyed differs by {diff}");
        live.processor.stop();
    }

    let kind = EffectKind::PitchCorrect;
    let mut live = host
        .instantiate(&request(&id_of("org.musiccraft.pitch_correct"), None))
        .unwrap();
    let follow = param_named(&live, "follow");
    live.processor.set_param(follow, 1.0);
    live.processor.note_on(0, 64, 1.0, 0.0, None);
    let mut following = Effect::new(kind, RATE as f32);
    let mut free = Effect::new(kind, RATE as f32);
    if let Effect::PitchCorrect(c) = &mut following {
        c.set_follow(Some(64));
    }
    let mut values = kind.defaults();
    if let Some(i) = kind.channel_ref_index() {
        values[i] = 1.0;
    }
    let (mut worst, mut effect_of_note) = (0.0f32, 0.0f32);
    for block in 0..BLOCKS * 2 {
        let (mut pl, mut pr) = input(block);
        let (mut fl, mut fr) = (pl.clone(), pr.clone());
        let (mut nl, mut nr) = (pl.clone(), pr.clone());
        live.processor.process(&mut pl, &mut pr, &transport(block));
        let ctx = ProcessContext {
            sample_rate: RATE as f32,
            tempo_bpm: TEMPO,
            beats: beats(block),
        };
        following.process(&mut fl, &mut fr, &values, ctx);
        free.process(&mut nl, &mut nr, &values, ctx);
        for ((a, b), c) in pl
            .iter()
            .chain(&pr)
            .zip(fl.iter().chain(&fr))
            .zip(nl.iter().chain(&nr))
        {
            worst = worst.max((a - b).abs());
            effect_of_note = effect_of_note.max((b - c).abs());
        }
    }
    assert!(effect_of_note > 0.01, "following a note changes the sound");
    assert!(worst < 1e-4, "following differs by {worst}");
    live.processor.stop();
}

#[test]
fn sidechain_and_follow_work_through_clap() {
    let dir = install("clap-keyed");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    sidechain_and_follow(&host, str::to_owned);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sidechain_and_follow_work_through_vst3() {
    let dir = install("vst3-keyed");
    let host = Vst3Host::with_folders(vec![dir.clone()]);
    sidechain_and_follow(&host, vst3_id);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

//! The host against real CLAP plugins (the test plugin crate, linked in and
//! called through the C ABI exactly as a loaded `.clap` file would be).

use std::sync::Arc;

use musiccraft_clap_testplugin::{ENTRY, GAIN_ID, PARAM, TONE_ID, TONE_PEAK};
use musiccraft_engine::{External, ExternalTransport};
use musiccraft_render::{PluginEvent, PluginFactory, PluginHost, PluginKind, PluginRequest};

use crate::ClapHost;

const RATE: u32 = 48_000;
const BLOCK: usize = 256;

fn host() -> ClapHost {
    ClapHost::with_linked(ENTRY, "test.clap")
}

fn request<'a>(id: &'a str, state: Option<&'a [u8]>) -> PluginRequest<'a> {
    PluginRequest {
        id,
        sample_rate: RATE,
        max_frames: BLOCK as u32,
        state,
    }
}

fn transport() -> ExternalTransport {
    ExternalTransport {
        playing: true,
        tempo_bpm: 120.0,
        position_beats: 0.0,
        sample_rate: f64::from(RATE),
    }
}

fn block(processor: &mut dyn External, input: f32) -> (Vec<f32>, Vec<f32>) {
    let mut left = vec![input; BLOCK];
    let mut right = vec![input; BLOCK];
    processor.process(&mut left, &mut right, &transport());
    (left, right)
}

fn peak(data: &[f32]) -> f32 {
    data.iter().fold(0.0, |p, s| p.max(s.abs()))
}

#[test]
fn scanning_lists_both_plugins_with_kinds() {
    let catalog = host().catalog();
    let kinds: Vec<_> = catalog.iter().map(|p| (p.id.as_str(), p.kind)).collect();
    assert_eq!(
        kinds,
        [
            (GAIN_ID, PluginKind::Effect),
            (TONE_ID, PluginKind::Instrument)
        ]
    );
    assert_eq!(catalog[0].name, "Test Gain");
    assert_eq!(catalog[0].vendor, "MusicCraft tests");
    let missing = host().instantiate(&request("com.nowhere", None));
    assert!(missing.unwrap_err().contains("not installed"));
}

#[test]
fn effect_processes_audio_and_takes_parameters() {
    let live = host().instantiate(&request(GAIN_ID, None)).unwrap();
    let mut processor = live.processor;
    let params = live.instance.params();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "Gain");
    assert_eq!(
        (params[0].min, params[0].max, params[0].default),
        (0.0, 2.0, 1.0)
    );
    let (l, r) = block(processor.as_mut(), 0.5);
    assert!(l.iter().chain(&r).all(|s| (s - 0.5).abs() < 1e-6));
    processor.set_param(PARAM, 0.5);
    processor.set_param(PARAM, 1.5);
    let (l, _) = block(processor.as_mut(), 0.5);
    assert!((l[0] - 0.75).abs() < 1e-6, "latest value wins: {}", l[0]);
    assert_eq!(live.instance.param_value(PARAM), Some(1.5));
    assert_eq!(
        live.instance.format_param(PARAM, 1.5).as_deref(),
        Some("1.50x")
    );
    processor.stop();
}

#[test]
fn instrument_plays_gated_notes_and_releases_them() {
    let live = host().instantiate(&request(TONE_ID, None)).unwrap();
    let mut processor = live.processor;
    assert!(
        peak(&block(processor.as_mut(), 0.0).0) < 1e-6,
        "silent at rest"
    );
    // A note starting 100 frames in, lasting 300 frames.
    processor.note_on(100, 69, 1.0, 0.0, Some(300));
    let (first, _) = block(processor.as_mut(), 0.0);
    assert!(peak(&first[..100]) < 1e-6);
    let level = 0.5 * TONE_PEAK;
    assert!(
        (peak(&first[100..]) - level).abs() < 0.01,
        "{}",
        peak(&first)
    );
    let (second, _) = block(processor.as_mut(), 0.0);
    assert!(peak(&second[..100]) > 0.1, "still held");
    assert!(peak(&second[200..]) < 1e-6, "released at frame 400");
    // Live notes hold until released; all-notes-off cuts them.
    processor.note_on(0, 60, 1.0, 0.0, None);
    assert!(peak(&block(processor.as_mut(), 0.0).0) > 0.1);
    processor.all_notes_off();
    assert!(peak(&block(processor.as_mut(), 0.0).0) < 1e-6);
}

#[test]
fn state_round_trips_and_restores_into_a_new_instance() {
    let host = host();
    let mut live = host.instantiate(&request(GAIN_ID, None)).unwrap();
    live.processor.set_param(PARAM, 0.25);
    block(live.processor.as_mut(), 1.0);
    let state = live.instance.save_state().unwrap();
    assert_eq!(state, 0.25f64.to_le_bytes());
    let mut restored = host.instantiate(&request(GAIN_ID, Some(&state))).unwrap();
    assert_eq!(restored.instance.param_value(PARAM), Some(0.25));
    let (l, _) = block(restored.processor.as_mut(), 1.0);
    assert!((l[0] - 0.25).abs() < 1e-6);
}

#[test]
fn parameter_changes_from_the_plugin_reach_the_main_thread() {
    let mut live = host().instantiate(&request(TONE_ID, None)).unwrap();
    // Key 127 makes the test plugin report `Level = 0.25` itself.
    live.processor.note_on(0, 127, 1.0, 0.0, Some(10));
    block(live.processor.as_mut(), 0.0);
    let events = live.instance.poll();
    assert!(events.contains(&PluginEvent::Param {
        id: PARAM,
        value: 0.25
    }));
    assert!(!live.instance.has_editor());
}

#[test]
fn renders_a_project_through_the_bridge_and_engine() {
    use musiccraft_commands::Session;
    use musiccraft_project::Project;
    use musiccraft_render::{RenderOptions, full_registry_with, render_with};

    let host: PluginHost = Arc::new(host());
    let mut s = Session::new(
        Project::default(),
        full_registry_with(Some(Arc::clone(&host))),
    );
    for line in [
        "channel.add_plugin org.musiccraft.test.tone Tone",
        "channel.set_volume 1 1",
        "note.add 1 0 4 A4 127",
        "mixer.add_plugin 0 org.musiccraft.test.gain",
        "mixer.set_plugin_param 0 1 1 2",
    ] {
        s.run_line(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    }
    let options = RenderOptions {
        sample_rate: RATE,
        loops: 1,
        max_tail_seconds: 0.0,
    };
    let (left, _, warnings) = render_with(s.project(), options, Some(&host));
    assert!(warnings.is_empty(), "{warnings:?}");
    // Level 0.5 x peak, doubled by the gain plugin.
    let expected = 0.5 * TONE_PEAK * 2.0;
    let level = peak(&left);
    assert!((level - expected).abs() < 0.02, "{level} vs {expected}");
    let quarter = left.len() / 4;
    assert!(
        peak(&left[2 * quarter..]) < 1e-3,
        "note ends after four steps"
    );
}

#[test]
fn loads_a_clap_file_from_a_folder() {
    use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
    // Cargo builds the test plugin's `cdylib` next to this test binary.
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let built = deps.join(format!(
        "{DLL_PREFIX}musiccraft_clap_testplugin{DLL_SUFFIX}"
    ));
    assert!(built.is_file(), "missing {}", built.display());
    let dir = std::env::temp_dir().join(format!("mc-clap-scan-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("vendor")).unwrap();
    std::fs::copy(&built, dir.join("vendor/Test Plugins.clap")).unwrap();
    std::fs::write(dir.join("broken.clap"), b"not a library").unwrap();

    let host = ClapHost::with_folders(vec![dir.clone()]);
    let catalog = host.catalog();
    assert_eq!(catalog.len(), 2, "{catalog:?}");
    assert!(catalog[0].path.ends_with("vendor/Test Plugins.clap"));
    let errors = host.scan_errors();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("broken.clap"));

    let mut live = host.instantiate(&request(GAIN_ID, None)).unwrap();
    live.processor.set_param(PARAM, 2.0);
    let (l, _) = block(live.processor.as_mut(), 0.25);
    assert!((l[0] - 0.5).abs() < 1e-6);
    live.processor.stop();
    drop(live);
    let _ = std::fs::remove_dir_all(dir);
}

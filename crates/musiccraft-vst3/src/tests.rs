//! The host against real VST3 plugins (the test plugin crate, linked in and
//! called through its factory exactly as a loaded module would be).

use std::sync::Arc;

use musiccraft_engine::{External, ExternalTransport};
use musiccraft_render::{PluginEvent, PluginFactory, PluginHost, PluginKind, PluginRequest};
use musiccraft_vst3_testplugin::{
    CONTROLLER_STATE, GAIN_CID, GetPluginFactory, PARAM, TONE_CID, TONE_PEAK,
};
#[cfg(target_os = "linux")]
use musiccraft_vst3_testplugin::{GAIN_VIEW_EDIT, GAIN_VIEW_SIZE};

use crate::module::class_id;
use crate::{GetFactory, Vst3Host};

const RATE: u32 = 48_000;
const BLOCK: usize = 256;

fn host() -> Vst3Host {
    Vst3Host::with_linked(GetPluginFactory as GetFactory, "test.vst3")
}

fn gain_id() -> String {
    class_id(&GAIN_CID)
}

fn tone_id() -> String {
    class_id(&TONE_CID)
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
        time_signature: musiccraft_engine::TimeSig::COMMON,
        bar_start_beats: 0.0,
        bar_number: 0,
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
fn scanning_lists_both_plugins_with_kinds_and_ids() {
    let catalog = host().catalog();
    let kinds: Vec<_> = catalog
        .iter()
        .map(|p| (p.id.clone(), p.name.as_str(), p.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            (gain_id(), "Test Gain", PluginKind::Effect),
            (tone_id(), "Test Tone", PluginKind::Instrument)
        ],
        "controllers are not listed"
    );
    assert!(gain_id().starts_with("vst3:4D435654"), "{}", gain_id());
    assert_eq!(gain_id().len(), 5 + 32);
    assert_eq!(catalog[0].vendor, "MusicCraft tests");
    assert_eq!(catalog[0].version, "1.0.0");
    let missing = host().instantiate(&request("vst3:00", None));
    assert!(missing.unwrap_err().contains("not installed"));
}

#[test]
fn effect_processes_audio_and_takes_parameters() {
    let live = host().instantiate(&request(&gain_id(), None)).unwrap();
    let mut processor = live.processor;
    let params = live.instance.params();
    assert_eq!(params.len(), 1);
    assert_eq!(
        params[0].name, "Gain",
        "the processor's message reached the controller"
    );
    assert_eq!(
        (params[0].min, params[0].max, params[0].default),
        (0.0, 1.0, 0.5)
    );
    assert!(!params[0].stepped);
    let (l, r) = block(processor.as_mut(), 0.5);
    assert!(l.iter().chain(&r).all(|s| (s - 0.5).abs() < 1e-6), "unity");
    processor.set_param(PARAM, 0.25);
    processor.set_param(PARAM, 0.75);
    let (l, _) = block(processor.as_mut(), 0.5);
    assert!((l[0] - 0.75).abs() < 1e-6, "latest value wins: {}", l[0]);
    assert_eq!(
        live.instance.param_value(PARAM),
        Some(0.75),
        "the controller heard it"
    );
    assert_eq!(
        live.instance.format_param(PARAM, 0.75).as_deref(),
        Some("1.50x")
    );
    processor.stop();
}

#[test]
fn instrument_plays_gated_notes_and_releases_them() {
    let live = host().instantiate(&request(&tone_id(), None)).unwrap();
    let mut processor = live.processor;
    assert!(
        peak(&block(processor.as_mut(), 0.0).0) < 1e-6,
        "silent at rest"
    );
    // A note starting 100 frames in, lasting 300 frames.
    processor.note_on(100, 69, 1.0, 0.0, Some(300));
    let (first, _) = block(processor.as_mut(), 0.0);
    assert!(peak(&first[..100]) < 1e-6);
    assert!(
        (peak(&first[100..]) - TONE_PEAK).abs() < 0.01,
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
fn state_round_trips_both_parts_into_a_new_instance() {
    let host = host();
    let mut live = host.instantiate(&request(&gain_id(), None)).unwrap();
    live.processor.set_param(PARAM, 0.125);
    block(live.processor.as_mut(), 1.0);
    let state = live.instance.save_state().unwrap();
    let mut expected = b"MCV3".to_vec();
    expected.extend_from_slice(&1u32.to_le_bytes());
    expected.extend_from_slice(&8u64.to_le_bytes());
    expected.extend_from_slice(&0.125f64.to_le_bytes());
    expected.extend_from_slice(&(CONTROLLER_STATE.len() as u64).to_le_bytes());
    expected.extend_from_slice(CONTROLLER_STATE);
    assert_eq!(state, expected);

    // A different controller part comes back as given.
    let mut custom = state[..state.len() - CONTROLLER_STATE.len() - 8].to_vec();
    custom.extend_from_slice(&4u64.to_le_bytes());
    custom.extend_from_slice(b"ctl2");
    let mut restored = host
        .instantiate(&request(&gain_id(), Some(&custom)))
        .unwrap();
    assert_eq!(
        restored.instance.param_value(PARAM),
        Some(0.125),
        "the controller heard the component's state"
    );
    let (l, _) = block(restored.processor.as_mut(), 1.0);
    assert!((l[0] - 0.25).abs() < 1e-6, "{}", l[0]);
    assert_eq!(restored.instance.save_state().unwrap(), custom);

    // A state from elsewhere is ignored rather than failing the plugin.
    let foreign = host
        .instantiate(&request(&gain_id(), Some(b"junk")))
        .unwrap();
    assert_eq!(foreign.instance.param_value(PARAM), Some(0.5));
}

#[test]
fn parameter_changes_from_the_processor_reach_the_main_thread() {
    let mut live = host().instantiate(&request(&tone_id(), None)).unwrap();
    assert_eq!(live.instance.params()[0].name, "Level");
    // Key 127 makes the test plugin report `Level = 0.25` itself.
    live.processor.note_on(0, 127, 1.0, 0.0, Some(10));
    block(live.processor.as_mut(), 0.0);
    let events = live.instance.poll();
    assert!(
        events.contains(&PluginEvent::Param {
            id: PARAM,
            value: 0.25
        }),
        "{events:?}"
    );
    assert_eq!(live.instance.param_value(PARAM), Some(0.25));
    assert!(!live.instance.has_editor());
}

#[cfg(target_os = "linux")]
#[test]
fn edits_in_the_plugin_window_reach_the_host_and_the_processor() {
    use musiccraft_render::PluginInstance;
    let (mut vst, mut processor) = host().build(&gain_id()).unwrap();
    assert!(vst.has_editor());
    // A stand-in parent: the test plugin never touches the window itself.
    vst.attach_for_test(0x1234).unwrap();
    assert!(vst.editor_open());
    assert_eq!(
        vst.requested_size(),
        Some(GAIN_VIEW_SIZE),
        "resize via the frame"
    );
    std::thread::sleep(std::time::Duration::from_millis(5));
    let events = vst.poll();
    assert!(
        events.contains(&PluginEvent::Param {
            id: PARAM,
            value: GAIN_VIEW_EDIT
        }),
        "the view's timer moved the knob: {events:?}"
    );
    let (l, _) = block(&mut processor, 0.5);
    assert!(
        (l[0] - 1.5 * 0.5).abs() < 1e-6,
        "the edit reached the processor: {}",
        l[0]
    );
    vst.set_editor_open(false, "").unwrap();
    assert!(!vst.editor_open());
    processor.stop();
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
        format!("channel.add_plugin {} Tone", tone_id()),
        "channel.set_volume 1 1".to_owned(),
        "note.add 1 0 4 A4 127".to_owned(),
        format!("mixer.add_plugin 0 {}", gain_id()),
        "mixer.set_plugin_param 0 1 1 1".to_owned(),
    ] {
        s.run_line(&line).unwrap_or_else(|e| panic!("{line}: {e}"));
    }
    let options = RenderOptions {
        sample_rate: RATE,
        loops: 1,
        max_tail_seconds: 0.0,
    };
    let (left, _, warnings) = render_with(s.project(), options, Some(&host));
    assert!(warnings.is_empty(), "{warnings:?}");
    // Full level x peak, -3 dB from the centred channel pan (circular law),
    // doubled by the gain plugin at its top value.
    let expected = TONE_PEAK * std::f32::consts::FRAC_1_SQRT_2 * 2.0;
    let level = peak(&left);
    assert!((level - expected).abs() < 0.02, "{level} vs {expected}");
    let quarter = left.len() / 4;
    assert!(
        peak(&left[2 * quarter..]) < 1e-3,
        "note ends after four steps"
    );
}

/// Copies the test plugin's `cdylib` (built by Cargo next to this test
/// binary) into a bundle at `bundle`.
#[cfg(target_os = "linux")]
fn make_bundle(bundle: &std::path::Path) {
    use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let built = deps.join(format!(
        "{DLL_PREFIX}musiccraft_vst3_testplugin{DLL_SUFFIX}"
    ));
    assert!(built.is_file(), "missing {}", built.display());
    std::fs::create_dir_all(bundle).unwrap();
    let binary = crate::module::binary_path(bundle);
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::copy(&built, &binary).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn loads_a_vst3_bundle_from_a_folder() {
    let dir = std::env::temp_dir().join(format!("mc-vst3-scan-{}", std::process::id()));
    let bundle = dir.join("vendor/Test Plugins.vst3");
    make_bundle(&bundle);
    // A bundle keeps its binary under `Contents/<arch>-linux/`.
    let binary = crate::module::binary_path(&bundle);
    assert!(binary.ends_with("Test Plugins.so"), "{}", binary.display());
    std::fs::write(dir.join("broken.vst3"), b"not a library").unwrap();

    let host = Vst3Host::with_folders(vec![dir.clone()]);
    let catalog = host.catalog();
    assert_eq!(catalog.len(), 2, "{catalog:?}");
    assert!(catalog[0].path.ends_with("vendor/Test Plugins.vst3"));
    let errors = host.scan_errors();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("broken.vst3"));

    let mut live = host.instantiate(&request(&gain_id(), None)).unwrap();
    live.processor.set_param(PARAM, 1.0);
    let (l, _) = block(live.processor.as_mut(), 0.25);
    assert!((l[0] - 0.5).abs() < 1e-6);
    live.processor.stop();
    drop(live);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

/// Not a real test: the child process of a scan. When the scan starts this
/// test binary with `MUSICCRAFT_VST3_PROBE` set, this probes that module and
/// exits; otherwise it does nothing.
#[test]
fn probe_child() {
    if let Some(path) = std::env::var_os(crate::probe::PROBE_ENV) {
        crate::probe::run(std::path::Path::new(&path));
        std::process::exit(0);
    }
}

/// Runs this test binary as a probe child; modules whose path contains
/// "Crashing" abort while loading.
#[cfg(target_os = "linux")]
fn probe_command(path: &std::path::Path) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args([
        "tests::probe_child",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]);
    if path.to_string_lossy().contains("Crashing") {
        command.env(musiccraft_vst3_testplugin::CRASH_ENV, "1");
    }
    command
}

#[cfg(target_os = "linux")]
#[test]
fn a_module_that_crashes_while_scanned_is_skipped() {
    let dir = std::env::temp_dir().join(format!("mc-vst3-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    make_bundle(&dir.join("A Crashing Plugin.vst3"));
    make_bundle(&dir.join("B Test Plugins.vst3"));
    // A Windows-only bundle, as a Windows installer would leave it.
    let windows = dir.join("C Windows Only.vst3/Contents/x86_64-win");
    std::fs::create_dir_all(&windows).unwrap();
    std::fs::write(windows.join("C Windows Only.vst3"), b"MZ").unwrap();

    let host = Vst3Host::with_folders(vec![dir.clone()]).with_probe(probe_command);
    let catalog = host.catalog();
    let names: Vec<&str> = catalog.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Test Gain", "Test Tone"], "{catalog:?}");
    assert!(catalog[0].path.ends_with("B Test Plugins.vst3"));
    let errors = host.scan_errors();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].contains("A Crashing Plugin.vst3")
            && errors[0].contains("crashed while loading (signal 6, SIGABRT)"),
        "{errors:?}"
    );
    assert!(
        errors[1].contains("C Windows Only.vst3")
            && errors[1].contains("no binary for this system"),
        "{errors:?}"
    );
    // Probed modules load into this process only when first used.
    assert!(host.lock().modules.is_empty());

    let mut live = host.instantiate(&request(&gain_id(), None)).unwrap();
    live.processor.set_param(PARAM, 1.0);
    let (l, _) = block(live.processor.as_mut(), 0.25);
    assert!((l[0] - 0.5).abs() < 1e-6);
    live.processor.stop();
    drop(live);
    assert_eq!(host.lock().modules.len(), 1);

    // A rescan asks the loaded module directly and probes the others again.
    host.rescan();
    assert_eq!(host.catalog().len(), 2);
    assert_eq!(host.scan_errors().len(), 2);
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
}

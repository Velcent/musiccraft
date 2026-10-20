//! Offline render round trip (`docs/spec/validation.md`): a project rendered
//! with a built-in instrument or mixer effect sounds the same as the project
//! rendered with that instrument or effect as our exported plugin, hosted by
//! the app through CLAP and through VST3. This is the whole path a user
//! takes: commands, the bridge, the engine, the host and the plugin.

mod common;

use std::sync::Arc;

use musiccraft_clap::ClapHost;
use musiccraft_commands::{Registry, Session};
use musiccraft_plugin_export::ids::{vst3_class_id, vst3_instrument_class_id};
use musiccraft_project::Project;
use musiccraft_render::{PluginHost, RenderOptions, render_with};
use musiccraft_vst3::Vst3Host;

const OPTIONS: RenderOptions = RenderOptions {
    sample_rate: 48_000,
    loops: 1,
    max_tail_seconds: 1.0,
};

/// A chord and a melody note on channel 1, after `setup` adds channel 1 (and
/// whatever else it adds).
fn project(setup: &str) -> Project {
    let mut session = Session::new(Project::default(), Registry::with_builtin());
    let script = format!(
        "project.set_tempo 128
         {setup}
         note.add 1 0 8 C4 115
         note.add 1 0 8 E4 90
         note.add 1 0 12 G3 100
         note.add 1 8 4 B4 80"
    );
    for line in script.lines().map(str::trim).filter(|l| !l.is_empty()) {
        session
            .run_line(line)
            .unwrap_or_else(|e| panic!("`{line}`: {e}"));
    }
    session.project().clone()
}

/// Largest difference between two renders (the shorter length).
fn difference(a: &(Vec<f32>, Vec<f32>), b: &(Vec<f32>, Vec<f32>)) -> f32 {
    a.0.iter()
        .zip(&b.0)
        .chain(a.1.iter().zip(&b.1))
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn peak(render: &(Vec<f32>, Vec<f32>)) -> f32 {
    render
        .0
        .iter()
        .chain(&render.1)
        .fold(0.0, |p, s| p.max(s.abs()))
}

fn render(project: &Project, host: &PluginHost) -> (Vec<f32>, Vec<f32>) {
    let (l, r, warnings) = render_with(project, OPTIONS, Some(host));
    assert!(warnings.is_empty(), "{warnings:?}");
    (l, r)
}

/// Built-in setup lines and the same with plugin `id` (a CLAP id).
const CASES: &[(&str, &str, &str)] = &[
    ("synth", "channel.add_synth", "org.musiccraft.synth"),
    ("fm", "channel.add_fm", "org.musiccraft.fm"),
    ("keys", "channel.add_keys", "org.musiccraft.keys"),
    ("pluck", "channel.add_pluck", "org.musiccraft.pluck"),
    (
        "antiserum",
        "channel.add_antiserum",
        "org.musiccraft.antiserum",
    ),
];

/// Mixer effects on the master, after a synth on channel 1.
const EFFECTS: &[&str] = &["delay", "chorus", "compressor", "reverb"];

fn round_trip(host: &PluginHost, id_of: impl Fn(&str, bool) -> String) {
    for (name, builtin, clap_id) in CASES {
        let ours = render(&project(builtin), host);
        let plugin_line = format!("channel.add_plugin {}", id_of(clap_id, true));
        let hosted = render(&project(&plugin_line), host);
        assert!(peak(&ours) > 0.05, "{name} sounds");
        let diff = difference(&ours, &hosted);
        assert!(diff < 1e-4, "{name}: the plugin differs by {diff}");
    }
    let dry = render(&project("channel.add_synth"), host);
    for effect in EFFECTS {
        let ours = render(
            &project(&format!("channel.add_synth\nmixer.add_effect 0 {effect}")),
            host,
        );
        let line = format!(
            "channel.add_synth\nmixer.add_plugin 0 {}",
            id_of(&format!("org.musiccraft.{effect}"), false)
        );
        let hosted = render(&project(&line), host);
        assert!(difference(&ours, &dry) > 0.01, "{effect} changes the sound");
        let diff = difference(&ours, &hosted);
        assert!(diff < 1e-4, "{effect}: the plugin differs by {diff}");
    }
}

#[test]
fn renders_match_through_clap() {
    let effects = common::install("render-clap-fx", "MusicCraft Effects");
    let instruments = common::install("render-clap-in", "MusicCraft Instruments");
    let host: PluginHost = Arc::new(ClapHost::with_folders(vec![
        effects.clone(),
        instruments.clone(),
    ]));
    round_trip(&host, |id, _| id.to_owned());
    drop(host);
    let _ = std::fs::remove_dir_all(effects);
    let _ = std::fs::remove_dir_all(instruments);
}

#[test]
fn renders_match_through_vst3() {
    let effects = common::install("render-vst3-fx", "MusicCraft Effects");
    let instruments = common::install("render-vst3-in", "MusicCraft Instruments");
    let host: PluginHost = Arc::new(Vst3Host::with_folders(vec![
        effects.clone(),
        instruments.clone(),
    ]));
    round_trip(&host, |id, instrument| {
        common::vst3_id(if instrument {
            vst3_instrument_class_id(id)
        } else {
            vst3_class_id(id)
        })
    });
    drop(host);
    let _ = std::fs::remove_dir_all(effects);
    let _ = std::fs::remove_dir_all(instruments);
}

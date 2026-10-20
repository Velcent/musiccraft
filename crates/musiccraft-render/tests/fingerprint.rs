//! Renders every built-in instrument (and each factory preset) to 32-bit float
//! WAV files and prints a hash of each, so a refactor of the instrument DSP can
//! be checked for bit-identical output on the same machine.
//!
//! Not run by default (hashes depend on the platform's maths library):
//!
//! ```sh
//! MUSICCRAFT_FINGERPRINT_DIR=/tmp/before cargo test -p musiccraft-render \
//!     --test fingerprint -- --ignored --nocapture
//! ```
//!
//! Run it again into another folder after the change and compare the
//! `hashes.txt` files.

use std::fmt::Write as _;
use std::path::PathBuf;

use musiccraft_commands::{Registry, Session};
use musiccraft_params::{INSTRUMENT_PARAMS, index_of, presets};
use musiccraft_project::Project;
use musiccraft_render::{RenderOptions, render};

/// A phrase that exercises chords, velocities, overlaps and a low note.
const PHRASE: &str = "note.add 1 0 4 C5 100
note.add 1 0 4 E5 90
note.add 1 0 4 G5 80
note.add 1 4 3 G5 60
note.add 1 6 4 C6 127
note.add 1 10 2 A4 40
note.add 1 12 4 C3 110";

fn session(lines: &str) -> Session {
    let mut registry = Registry::with_builtin();
    musiccraft_render::register(&mut registry).unwrap();
    let mut s = Session::new(Project::default(), registry);
    for line in lines.lines().map(str::trim).filter(|l| !l.is_empty()) {
        s.run_line(line).unwrap_or_else(|e| panic!("`{line}`: {e}"));
    }
    s
}

fn apply(project: &mut Project, preset: &presets::Preset) {
    for (name, value) in preset.resolve(INSTRUMENT_PARAMS) {
        if let Some(i) = index_of(INSTRUMENT_PARAMS, name) {
            project.channels[0].params.0[i] = value;
        }
    }
}

/// FNV-1a over the bits of every sample.
fn hash(left: &[f32], right: &[f32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in left.iter().chain(right) {
        for byte in s.to_bits().to_le_bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn cases() -> Vec<(String, Project)> {
    let mut out: Vec<(String, Project)> = Vec::new();
    let mut add = |name: &str, setup: &str| {
        out.push((
            name.to_owned(),
            session(&format!("{setup}\n{PHRASE}")).project().clone(),
        ));
    };
    for (name, setup) in [
        ("synth", "channel.add_synth"),
        ("drum-kick", "channel.add_drum kick"),
        ("drum-snare", "channel.add_drum snare"),
        ("drum-hat", "channel.add_drum hat"),
        ("drum-tom", "channel.add_drum tom"),
        ("drum-clap", "channel.add_drum clap"),
        ("pluck", "channel.add_pluck"),
        ("fm", "channel.add_fm"),
        ("keys-piano", "channel.add_keys piano"),
        ("keys-epiano", "channel.add_keys epiano"),
        ("keys-organ", "channel.add_keys organ"),
        ("additive", "channel.add_additive"),
        ("sampler-kick", "channel.add_sampler builtin:kick"),
        ("sampler-open-hat", "channel.add_sampler builtin:open_hat"),
        ("kit", "channel.add_kit"),
        ("granular", "channel.add_granular builtin:crash"),
        ("resynth", "channel.add_resynth builtin:tom"),
        (
            "zones",
            "channel.add_zones\nchannel.add_zone 1 builtin:snare 0 59 60\nchannel.add_zone 1 builtin:rim 60 127 72",
        ),
        (
            "synth-arp-mono-glide-echo",
            "channel.add_synth\nchannel.set_param 1 arp_mode 1\nchannel.set_param 1 mono 1\nchannel.set_param 1 glide 0.1\nchannel.set_param 1 echo_count 3\nproject.set_swing 40",
        ),
        (
            "layer",
            "channel.add_layer\nchannel.add_synth\nchannel.add_pluck\nchannel.set_layer 1 2,3",
        ),
    ] {
        add(name, setup);
    }
    let banks: [(&str, &str, &[presets::Preset]); 6] = [
        ("synth", "channel.add_synth", presets::SYNTH),
        ("drum", "channel.add_drum kick", presets::DRUM),
        ("pluck", "channel.add_pluck", presets::PLUCK),
        ("fm", "channel.add_fm", presets::FM),
        ("keys", "channel.add_keys piano", presets::KEYS),
        ("additive", "channel.add_additive", presets::ADDITIVE),
    ];
    for (bank, setup, list) in banks {
        let base = session(&format!("{setup}\n{PHRASE}")).project().clone();
        for preset in list {
            let mut project = base.clone();
            apply(&mut project, preset);
            out.push((format!("{bank}-preset-{}", preset.name), project));
        }
    }
    out
}

#[test]
#[ignore = "writes renders for a manual before/after comparison"]
fn fingerprint_every_instrument() {
    let dir = PathBuf::from(
        std::env::var_os("MUSICCRAFT_FINGERPRINT_DIR").expect("set MUSICCRAFT_FINGERPRINT_DIR"),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let mut listing = String::new();
    for (name, project) in cases() {
        let (left, right, warnings) = render(&project, RenderOptions::default());
        assert!(warnings.is_empty(), "{name}: {warnings:?}");
        let file: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        musiccraft_samples::write_wav(
            &dir.join(format!("{file}.wav")),
            &left,
            &right,
            48_000,
            musiccraft_samples::WavFormat::Float32,
        )
        .unwrap();
        writeln!(
            listing,
            "{:016x}  {} frames  {name}",
            hash(&left, &right),
            left.len()
        )
        .unwrap();
    }
    std::fs::write(dir.join("hashes.txt"), &listing).unwrap();
    print!("{listing}");
}

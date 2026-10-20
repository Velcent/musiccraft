//! Developer tasks. `cargo xtask parity` checks `docs/parity.md` against the
//! command registry, the default keymap and the spec folder.
//! `cargo xtask bundle [--release]` builds the plugin bundles
//! (`target/bundled/MusicCraft Effects.clap` and `.vst3`, and
//! `MusicCraft Instruments.clap` and `.vst3`) with nih-plug's bundler, one
//! build of the plugin crate per feature. `cargo xtask validate` runs
//! clap-validator and the VST3 validator on those bundles.

mod validate;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use musiccraft_commands::{Invocation, SESSION_COMMANDS};
use musiccraft_ui::{Chord, Keymap};

const USAGE: &str = "usage: cargo xtask parity [--min <percent>]\n       cargo xtask bundle [cargo build options]\n       cargo xtask validate";

/// The package whose library holds the exported plugins.
const PLUGIN_PACKAGE: &str = "musiccraft-plugin-export";

/// The bundles: the plugin crate's feature and the bundle's name.
const BUNDLES: [(&str, &str); 2] = [
    ("instruments", "MusicCraft Instruments"),
    ("effects", "MusicCraft Effects"),
];

/// Puts `bundler.toml` back as it was when dropped.
struct RestoreBundlerToml {
    path: std::path::PathBuf,
    original: String,
}

impl Drop for RestoreBundlerToml {
    fn drop(&mut self) {
        if let Err(error) = std::fs::write(&self.path, &self.original) {
            eprintln!("could not restore {}: {error}", self.path.display());
        }
    }
}

/// Builds and bundles the plugins, one bundle per feature of the plugin
/// crate; `options` go to `cargo build`.
fn bundle(options: &[&str]) -> ExitCode {
    // nih-plug's bundler names a package's bundle from `bundler.toml`, so
    // each build gets the name of its bundle there for the time it runs.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("bundler.toml");
    let original = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let _restore = RestoreBundlerToml {
        path: path.clone(),
        original,
    };
    for (feature, name) in BUNDLES {
        let config = format!(
            "# Written by `cargo xtask bundle` while it runs; see xtask/src/main.rs.\n[{PLUGIN_PACKAGE}]\nname = \"{name}\"\n"
        );
        if let Err(error) = std::fs::write(&path, config) {
            eprintln!("cannot write {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
        let args = [
            "bundle",
            PLUGIN_PACKAGE,
            "--no-default-features",
            "--features",
            feature,
        ]
        .into_iter()
        .chain(options.iter().copied())
        .map(str::to_owned);
        if let Err(error) = nih_plug_xtask::main_with_args("cargo xtask", args) {
            eprintln!("bundle {name} failed: {error:#}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Outcome of checking one feature.
#[derive(Debug, PartialEq, Eq)]
enum Status {
    Done,
    Todo,
    Broken(Vec<String>),
}

#[derive(Debug)]
struct Feature {
    area: String,
    name: String,
    status: Status,
}

struct Checker {
    commands: Vec<String>,
    keymap: Keymap,
    spec_dir: PathBuf,
}

impl Checker {
    fn check(&self, requirement: &str) -> Result<(), String> {
        if let Some(name) = requirement.strip_prefix("cmd:") {
            return if self.commands.iter().any(|c| c == name) {
                Ok(())
            } else {
                Err(format!("command `{name}` is not registered"))
            };
        }
        if let Some(binding) = requirement.strip_prefix("key:") {
            let (chord, action) = binding
                .split_once('=')
                .ok_or_else(|| format!("bad key requirement `{binding}`"))?;
            let chord = Chord::parse(chord)?;
            let expected = Invocation::parse(action)
                .map_err(|e| e.to_string())?
                .to_string();
            return match self.keymap.lookup(&chord) {
                Some(bound) if bound.to_string() == expected => Ok(()),
                Some(bound) => Err(format!("`{chord}` is bound to `{bound}`, not `{expected}`")),
                None => Err(format!("`{chord}` is not bound")),
            };
        }
        if let Some(kind) = requirement.strip_prefix("fx:") {
            return musiccraft_params::EffectKind::parse(kind)
                .map(|_| ())
                .ok_or_else(|| format!("effect `{kind}` does not exist"));
        }
        if let Some(file) = requirement.strip_prefix("spec:") {
            return if self.spec_dir.join(file).is_file() {
                Ok(())
            } else {
                Err(format!("spec `{file}` is missing"))
            };
        }
        Err(format!("unknown requirement `{requirement}`"))
    }
}

/// Splits a requirements cell into tags: the text inside backticks.
fn tags(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(|t| t.replace("\\|", "|"))
        .collect()
}

fn parse(text: &str, checker: &Checker) -> Vec<Feature> {
    let mut features = Vec::new();
    for line in text.lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let [area, name, requires] = cells.as_slice() else {
            continue;
        };
        if *area == "Area" || area.starts_with("---") || !requires.contains('`') {
            continue;
        }
        let tags = tags(requires);
        let status = if tags.iter().any(|t| t == "todo") {
            Status::Todo
        } else {
            let problems: Vec<String> = tags
                .iter()
                .filter(|t| *t != "done")
                .filter_map(|t| checker.check(t).err())
                .collect();
            if problems.is_empty() {
                Status::Done
            } else {
                Status::Broken(problems)
            }
        };
        features.push(Feature {
            area: (*area).to_owned(),
            name: (*name).to_owned(),
            status,
        });
    }
    features
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn parity(min_percent: Option<f64>) -> ExitCode {
    let root = workspace_root();
    let text = match std::fs::read_to_string(root.join("docs/parity.md")) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("cannot read docs/parity.md: {err}");
            return ExitCode::from(2);
        }
    };
    let registry = musiccraft_render::full_registry();
    let mut commands: Vec<String> = registry.iter().map(|c| c.name().to_owned()).collect();
    commands.extend(SESSION_COMMANDS.iter().map(|(n, _, _)| (*n).to_owned()));
    let checker = Checker {
        commands,
        keymap: Keymap::default_keymap(),
        spec_dir: root.join("docs/spec"),
    };
    let features = parse(&text, &checker);
    let mut area = String::new();
    for feature in &features {
        if feature.area != area {
            area.clone_from(&feature.area);
            println!("\n{area}");
        }
        match &feature.status {
            Status::Done => println!("  [x] {}", feature.name),
            Status::Todo => println!("  [ ] {}", feature.name),
            Status::Broken(problems) => {
                println!("  [!] {}", feature.name);
                for problem in problems {
                    println!("        {problem}");
                }
            }
        }
    }
    let done = features.iter().filter(|f| f.status == Status::Done).count();
    let broken = features
        .iter()
        .filter(|f| matches!(f.status, Status::Broken(_)))
        .count();
    let total = features.len().max(1);
    let percent = done as f64 * 100.0 / total as f64;
    println!(
        "\nparity: {done}/{} features ({percent:.0}%), {broken} with failed checks, {} commands registered",
        features.len(),
        checker.commands.len()
    );
    if broken > 0 {
        return ExitCode::FAILURE;
    }
    if min_percent.is_some_and(|min| percent < min) {
        eprintln!(
            "parity is below the required {:.0}%",
            min_percent.unwrap_or(0.0)
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["parity"] => parity(None),
        ["bundle", options @ ..] => bundle(options),
        ["validate"] => validate::validate(&BUNDLES),
        ["parity", "--min", value] => match value.parse() {
            Ok(min) => parity(Some(min)),
            Err(_) => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Checker {
        Checker {
            commands: vec!["transport.toggle".to_owned()],
            keymap: Keymap::parse("Space = transport.toggle\nF9 = view.toggle mixer").unwrap(),
            spec_dir: workspace_root().join("docs/spec"),
        }
    }

    #[test]
    fn requirements_are_checked() {
        let c = checker();
        assert!(c.check("cmd:transport.toggle").is_ok());
        assert!(c.check("cmd:transport.fly").is_err());
        assert!(c.check("key:Space=transport.toggle").is_ok());
        assert!(c.check("key:F9=view.toggle mixer").is_ok());
        assert!(c.check("key:F9=view.toggle playlist").is_err());
        assert!(c.check("spec:transport.md").is_ok());
        assert!(c.check("spec:nope.md").is_err());
        assert!(c.check("fx:reverb").is_ok());
        assert!(c.check("fx:theremin").is_err());
        assert!(c.check("wat").is_err());
    }

    #[test]
    fn table_rows_become_features() {
        let text = "| Area | Feature | Requires |\n| --- | --- | --- |\n\
                    | T | Play | `cmd:transport.toggle` |\n\
                    | T | Fly | `todo` |\n\
                    | T | Broken | `cmd:nope` |\n\
                    | T | Manual | `done` |\n";
        let features = parse(text, &checker());
        let statuses: Vec<_> = features.iter().map(|f| &f.status).collect();
        assert_eq!(statuses.len(), 4);
        assert_eq!(*statuses[0], Status::Done);
        assert_eq!(*statuses[1], Status::Todo);
        assert!(matches!(statuses[2], Status::Broken(_)));
        assert_eq!(*statuses[3], Status::Done);
    }
}

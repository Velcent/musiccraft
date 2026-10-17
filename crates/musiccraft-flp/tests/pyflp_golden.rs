//! Differential test against PyFLP: every file in `tests/assets` (PyFLP's
//! own test assets) is read with `examples/flp_dump.rs` and compared with
//! what PyFLP read from it (`tests/golden`, made by `difftest/run.sh`).
//!
//! Where PyFLP raised, `null` (or an error) agrees. Where PyFLP raised but
//! the port reads a value, the property must be one of the PyFLP bugs the
//! port fixes on purpose (`FIXED`).

#[allow(dead_code)]
#[path = "../examples/flp_dump.rs"]
mod flp_dump;

use std::path::{Path, PathBuf};

use musiccraft_flp::Project;
use serde_json::{Value, json};

/// Properties PyFLP raises on that the port reads (`[]` for any index).
const FIXED: &[&str] = &[
    // PyFLP looks up `arp.*` keys its struct names without the prefix.
    ".arp.chord",
    ".arp.direction",
    ".arp.gate",
    ".arp.range",
    ".arp.repeat",
    ".arp.slide",
    ".arp.time",
    // PyFLP reads a slot's records as if they were its insert's.
    ".slots[].enabled",
    ".slots[].mix",
    // PyFLP drops children listed after their layer.
    ".channels[].children",
];

fn generic(path: &str) -> String {
    let mut out = String::new();
    let mut in_index = false;
    for c in path.chars() {
        match c {
            '[' => {
                in_index = true;
                out.push_str("[]");
            }
            ']' => in_index = false,
            _ if in_index => {}
            _ => out.push(c),
        }
    }
    out
}

fn is_error(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == 1 && o.contains_key("error"))
}

fn compare(py: &Value, rs: &Value, path: &str, problems: &mut Vec<String>) {
    if is_error(py) {
        if !rs.is_null() && !is_error(rs) && !FIXED.iter().any(|f| generic(path).ends_with(f)) {
            problems.push(format!("{path}: PyFLP raises {py}, the port reads {rs}"));
        }
        return;
    }
    match (py, rs) {
        (Value::Number(a), Value::Number(b)) if a.is_f64() || b.is_f64() => {
            let (a, b) = (
                a.as_f64().unwrap_or(f64::NAN),
                b.as_f64().unwrap_or(f64::NAN),
            );
            if (a - b).abs() > 1e-9 * a.abs().max(1.0) {
                problems.push(format!("{path}: {a} != {b}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => compare(x, y, &format!("{path}.{key}"), problems),
                    _ => problems.push(format!("{path}.{key}: missing on one side")),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                problems.push(format!("{path}: length {} != {}", a.len(), b.len()));
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                compare(x, y, &format!("{path}[{i}]"), problems);
            }
        }
        _ if py == rs => {}
        _ => problems.push(format!("{path}: {py} != {rs}")),
    }
}

fn assets() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|e| e == "flp" || e == "fst" || e == "fsc")
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        &mut out,
    );
    out.sort();
    out
}

fn golden(key: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{key}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

#[test]
fn the_port_reads_every_pyflp_asset_as_pyflp_does() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets");
    let files = assets();
    assert_eq!(files.len(), 81, "PyFLP's test assets are missing");
    let mut problems = Vec::new();
    let mut compared = 0;
    for file in &files {
        let rel = file.strip_prefix(&root).unwrap_or(file);
        let key = rel.to_string_lossy().replace(['/', '\\', ' '], "_");
        let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{file:?}: {e}"));
        let parsed = Project::parse(&bytes);
        let whole = match &parsed {
            Ok(project) => flp_dump::dump(project),
            Err(err) => json!({ "parse_error": flp_dump::error_name(err) }),
        };
        compare(&golden(&key), &whole, &key, &mut problems);
        compared += 1;
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if ext == "fst" || ext == "fsc" {
            let preset = parsed
                .as_ref()
                .ok()
                .and_then(|p| flp_dump::dump_preset(p, &file.to_string_lossy()));
            if let Some(preset) = preset {
                compare(
                    &golden(&format!("preset-{key}")),
                    &preset,
                    &key,
                    &mut problems,
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 154);
    assert!(
        problems.is_empty(),
        "{} differences from PyFLP:\n{}",
        problems.len(),
        problems
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_flp_test_project_reads_as_pyflps_tests_expect() {
    // From PyFLP's tests/test_project.py.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/FL 20.8.4.flp");
    let bytes = std::fs::read(path).expect("asset");
    let p = Project::parse(&bytes).expect("parses");
    assert_eq!(p.artists().as_deref(), Some("demberto"));
    assert_eq!(p.channel_count(), 19);
    assert_eq!(p.ppq(), 96);
    assert_eq!(p.tempo(), Some(69.42));
    assert_eq!(p.title().as_deref(), Some("PyFLP Test FLP"));
    assert_eq!(p.licensee().as_deref(), Some("VIKTORKHLEBNIKOV38394416"));
    assert_eq!(
        p.version().map(|v| v.to_string()).as_deref(),
        Some("20.8.4.2576")
    );
    assert_eq!(p.patterns().patterns().len(), 5);
    assert_eq!(p.mixer().inserts().len(), 127);
    assert_eq!(p.arrangements().arrangements().map(|a| a.len()), Ok(2));
}

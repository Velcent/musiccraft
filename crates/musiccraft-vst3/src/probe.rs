//! Reading a module's class list in a child process, so a module that
//! crashes or hangs while it loads only costs its own place in the catalog.
//!
//! The child is the same program started with [`PROBE_ENV`] naming the
//! module. [`serve_probe`] (called first thing in `main`) sees it, loads the
//! module, prints one tab-separated line per class and a final `end` line on
//! stdout, and exits. Lines carry a prefix, so whatever the plugin itself
//! prints is ignored.

use std::ffi::c_char;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::module::{ClassInfo, Module};

/// Names the module a child process should probe.
pub(crate) const PROBE_ENV: &str = "MUSICCRAFT_VST3_PROBE";

/// How long a child may take before it is stopped. Bridged Windows plugins
/// start Wine first, which can be slow the first time.
const TIMEOUT: Duration = Duration::from_secs(120);

const PREFIX: &str = "musiccraft-vst3-probe";

/// Set once this program has called [`serve_probe`], so it can act as the
/// child; until then hosts scan in process.
static SERVED: AtomicBool = AtomicBool::new(false);

/// Call first thing in `main`. In a probe child this probes the module, prints
/// the result and exits; otherwise it returns and lets this program's
/// [`crate::Vst3Host::new`] scan modules in child processes.
pub fn serve_probe() {
    if let Some(path) = std::env::var_os(PROBE_ENV) {
        run(Path::new(&path));
        std::process::exit(0);
    }
    SERVED.store(true, Ordering::SeqCst);
}

/// Whether [`serve_probe`] ran in this process.
pub(crate) fn served() -> bool {
    SERVED.load(Ordering::SeqCst)
}

/// The child's side: loads the module and prints what it offers. The module
/// is unloaded after the `end` line, so a crash while it exits does not hide
/// a good result.
pub(crate) fn run(path: &Path) {
    let module = Module::load(path);
    // Start on a fresh line, after anything the plugin printed.
    let mut text = String::from("\n");
    match &module {
        Ok(module) => {
            for class in module.classes() {
                text.push_str(&class_line(&class));
            }
        }
        Err(err) => text.push_str(&format!("{PREFIX}\terror\t{}\n", clean(err))),
    }
    text.push_str(&format!("{PREFIX}\tend\n"));
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
    drop(out);
    drop(module);
}

/// Tabs and line breaks would break the line format.
fn clean(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

fn class_line(class: &ClassInfo) -> String {
    let cid: String = class
        .cid
        .iter()
        .map(|b| format!("{:02X}", *b as u8))
        .collect();
    format!(
        "{PREFIX}\tclass\t{cid}\t{}\t{}\t{}\t{}\t{}\n",
        clean(&class.category),
        clean(&class.name),
        clean(&class.sub_categories),
        clean(&class.vendor),
        clean(&class.version),
    )
}

fn parse_class(fields: &[&str]) -> Option<ClassInfo> {
    let [cid, category, name, sub_categories, vendor, version] = fields else {
        return None;
    };
    let mut tuid = [0 as c_char; 16];
    if cid.len() != 32 {
        return None;
    }
    for (i, slot) in tuid.iter_mut().enumerate() {
        let byte = u8::from_str_radix(cid.get(2 * i..2 * i + 2)?, 16).ok()?;
        *slot = byte as c_char;
    }
    Some(ClassInfo {
        cid: tuid,
        category: (*category).to_string(),
        name: (*name).to_string(),
        sub_categories: (*sub_categories).to_string(),
        vendor: (*vendor).to_string(),
        version: (*version).to_string(),
    })
}

/// Reads a child's output: the classes, or the error it reported. `None`
/// when the output has no `end` line (the child died first).
pub(crate) fn parse(output: &str) -> Option<Result<Vec<ClassInfo>, String>> {
    let mut classes = Vec::new();
    let mut error = None;
    for line in output.lines() {
        // Whatever the plugin printed without a line break may come first.
        let Some(start) = line.find(PREFIX) else {
            continue;
        };
        let fields: Vec<&str> = line
            .get(start + PREFIX.len()..)
            .unwrap_or_default()
            .split('\t')
            .skip(1)
            .collect();
        match fields.split_first() {
            Some((&"class", rest)) => classes.extend(parse_class(rest)),
            Some((&"error", rest)) => error = Some(rest.join(" ")),
            Some((&"end", _)) => return Some(error.map_or(Ok(classes), Err)),
            _ => {}
        }
    }
    None
}

/// Probes `path` with `command` (the child program). The outer `Err` means
/// the child could not be started.
pub(crate) fn probe(
    mut command: Command,
    path: &Path,
) -> Result<Result<Vec<ClassInfo>, String>, std::io::Error> {
    let mut child = command
        .env(PROBE_ENV, path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdout = child.stdout.take();
    // Read on another thread so a chatty plugin cannot fill the pipe and
    // stall the child.
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stdout) = &mut stdout {
            let _ = stdout.read_to_string(&mut text);
        }
        text
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let output = reader.join().unwrap_or_default();
    let shown = path.display();
    Ok(match (parse(&output), status) {
        (Some(result), _) => result,
        (None, None) => Err(format!(
            "{shown}: skipped, it did not finish loading within {} seconds",
            TIMEOUT.as_secs()
        )),
        (None, Some(status)) => Err(format!("{shown}: skipped, {}", describe(status))),
    })
}

/// How a child that printed no result ended.
fn describe(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            let name = match signal {
                4 => " SIGILL",
                6 => " SIGABRT",
                7 => " SIGBUS",
                8 => " SIGFPE",
                11 => " SIGSEGV",
                _ => "",
            };
            let name = if name.is_empty() {
                String::new()
            } else {
                format!(",{name}")
            };
            return format!("it crashed while loading (signal {signal}{name})");
        }
    }
    match status.code() {
        Some(code) => format!("it crashed while loading (exit code {code})"),
        None => "it crashed while loading".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_lines_round_trip_and_plugin_output_is_ignored() {
        let class = ClassInfo {
            cid: [7; 16],
            category: "Audio Module Class".into(),
            name: "Tab\tName".into(),
            sub_categories: "Fx|Delay".into(),
            vendor: "Vendor".into(),
            version: "1.0".into(),
        };
        let output = format!("plugin says hello{}{PREFIX}\tend\n", class_line(&class));
        let parsed = parse(&output).unwrap().unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].cid, class.cid);
        assert_eq!(parsed[0].name, "Tab Name");
        assert_eq!(parsed[0].sub_categories, "Fx|Delay");
    }

    #[test]
    fn errors_and_missing_end_lines_are_reported() {
        assert_eq!(
            parse(&format!("{PREFIX}\terror\tno luck\n{PREFIX}\tend\n")),
            Some(Err("no luck".to_string()))
        );
        assert_eq!(parse(&format!("{PREFIX}\tclass\t00\n")), None);
    }
}

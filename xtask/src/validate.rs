//! `cargo xtask validate`: runs clap-validator and the VST3 SDK's validator
//! on the bundles in `target/bundled` (`docs/spec/validation.md`). The tools
//! come from `CLAP_VALIDATOR` and `VST3_VALIDATOR`, or from `PATH`.

use std::path::Path;
use std::process::{Command, ExitCode};

/// The program named by environment variable `var`, or `default`.
fn tool(var: &str, default: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| default.to_owned())
}

/// The number before `word` in `line` ("12 failed" → 12).
fn count_before(line: &str, word: &str) -> Option<u32> {
    let words: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();
    words
        .windows(2)
        .find(|pair| pair[1] == word)
        .and_then(|pair| pair[0].parse().ok())
}

/// Text without terminal colour codes.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Runs `program` with `args`; its standard output, or why it did not run.
fn run(program: &str, args: &[&Path]) -> Result<(bool, String), String> {
    let output = Command::new(program)
        .args(args)
        // nih-plug logs every host query in debug builds; keep that out.
        .env("NIH_LOG", "/dev/null")
        .output()
        .map_err(|e| format!("cannot run {program}: {e} (set its path in the environment)"))?;
    Ok((
        output.status.success(),
        plain(&String::from_utf8_lossy(&output.stdout)),
    ))
}

/// Failures clap-validator reports for `bundle`.
fn clap(bundle: &Path) -> Result<u32, String> {
    let program = tool("CLAP_VALIDATOR", "clap-validator");
    let validate = Path::new("validate");
    let only_failed = Path::new("--only-failed");
    let (ok, out) = run(&program, &[validate, only_failed, bundle])?;
    let summary = out
        .lines()
        .rfind(|l| l.contains("tests run"))
        .ok_or_else(|| format!("clap-validator printed no summary:\n{out}"))?;
    println!("  CLAP  {}: {summary}", bundle.display());
    let failed = count_before(summary, "failed").unwrap_or(0)
        + count_before(summary, "crashed").unwrap_or(0);
    if failed > 0 || !ok {
        println!("{out}");
    }
    Ok(failed.max(u32::from(!ok)))
}

/// Failures the VST3 validator reports for `bundle`.
fn vst3(bundle: &Path) -> Result<u32, String> {
    let program = tool("VST3_VALIDATOR", "validator");
    let (ok, out) = run(&program, &[bundle])?;
    let mut failed = 0;
    let mut found = false;
    // "Result: 2068 tests passed, 0 tests failed", once per plugin class.
    for line in out.lines().filter(|l| l.contains("Result:")) {
        found = true;
        let after = line.split_once("passed").map_or("", |(_, rest)| rest);
        failed += count_before(after, "tests").unwrap_or(0);
    }
    if !found {
        return Err(format!("the VST3 validator printed no result:\n{out}"));
    }
    println!("  VST3  {}: {failed} failed", bundle.display());
    if failed > 0 || !ok {
        for line in out
            .lines()
            .filter(|l| l.contains("ERROR") || l.contains("FAILED"))
        {
            println!("    {line}");
        }
    }
    Ok(failed.max(u32::from(!ok)))
}

/// A validator run on one bundle: its failure count, or why it did not run.
type Check = fn(&Path) -> Result<u32, String>;

/// Validates every bundle in `target/bundled`.
pub fn validate(bundles: &[(&str, &str)]) -> ExitCode {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/bundled");
    let mut failed = 0;
    for (_, name) in bundles {
        let checks: [(&str, Check); 2] = [("clap", clap), ("vst3", vst3)];
        for (extension, check) in checks {
            let bundle = dir.join(format!("{name}.{extension}"));
            if !bundle.exists() {
                eprintln!(
                    "{} is missing; run `cargo xtask bundle` first",
                    bundle.display()
                );
                return ExitCode::FAILURE;
            }
            match check(&bundle) {
                Ok(count) => failed += count,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    if failed > 0 {
        eprintln!("validation: {failed} failed");
        return ExitCode::FAILURE;
    }
    println!("validation: every bundle passes");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_come_from_the_summaries() {
        let clap = "2086 tests run, 1690 passed, 3 failed, 25 warnings, 371 skipped";
        assert_eq!(count_before(clap, "failed"), Some(3));
        assert_eq!(count_before(clap, "crashed"), None);
        let vst3 = "Result: 2068 tests passed, 4 tests failed";
        let after = vst3.split_once("passed").map_or("", |(_, rest)| rest);
        assert_eq!(count_before(after, "tests"), Some(4));
        assert_eq!(plain("\u{1b}[1;31m278\u{1b}[0m failed"), "278 failed");
    }
}

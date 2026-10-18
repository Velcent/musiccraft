use std::io::Write;
use std::process::{Command, Stdio};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_musiccraft-cli"))
}

#[test]
fn run_sets_tempo_and_exits_zero() {
    let output = cli()
        .args(["run", "project.set_tempo", "128"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ok: tempo 128 BPM"));
    assert!(stdout.contains("state: tempo=128 bpm, transport=stopped"));
}

#[test]
fn invalid_command_exits_nonzero() {
    let output = cli()
        .args(["run", "project.set_tempo", "9999"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stdout).unwrap().contains("error:"));
}

#[test]
fn script_from_stdin_runs_all_lines() {
    let mut child = cli()
        .args(["script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"project.set_tempo 90\ntransport.toggle\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("state: tempo=90 bpm, transport=playing"));
}

#[test]
fn missing_arguments_print_usage_and_exit_two() {
    let output = cli().output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr).unwrap().contains("usage:"));
}

#[test]
fn quoted_arguments_survive_the_run_subcommand() {
    let output = cli()
        .args(["run", "project.rename", "Two Words"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("ok: project renamed to Two Words")
    );
}

#[test]
fn dump_prints_project_json() {
    let mut child = cli()
        .args(["dump", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"channel.add_sampler builtin:kick\nstep.toggle 1 0\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"format\": \"musiccraft-project\""));
    assert!(stdout.contains("builtin:kick"));
}

#[test]
fn export_from_a_script_writes_audio() {
    let path = std::env::temp_dir().join(format!("musiccraft-cli-{}.wav", std::process::id()));
    let script = format!(
        "channel.add_sampler builtin:kick\nstep.toggle 1 0\nproject.export_wav \"{}\"\n",
        path.display()
    );
    let mut child = cli()
        .args(["script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(std::fs::metadata(&path).unwrap().len() > 44);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn wavetables_list_export_import_and_formula() {
    let dir = std::env::temp_dir().join(format!("mc-wavetable-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = |name: &str| dir.join(name).to_string_lossy().into_owned();

    let list = cli().args(["wavetable", "list"]).output().unwrap();
    let stdout = String::from_utf8(list.stdout).unwrap();
    assert!(stdout.contains("formant\t64 frames"), "{stdout}");

    let out = cli()
        .args(["wavetable", "export", "pwm", &path("pwm.wav")])
        .output()
        .unwrap();
    assert!(out.status.success());
    let imported = cli()
        .args(["wavetable", "import", &path("pwm.wav"), &path("again.wav")])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(imported.stdout)
            .unwrap()
            .contains("wrote 64 frames")
    );
    assert_eq!(
        std::fs::read(path("pwm.wav")).unwrap(),
        std::fs::read(path("again.wav")).unwrap()
    );

    let formula = cli()
        .args([
            "wavetable",
            "formula",
            "sin(x*pi*2)*(1-y)",
            "8",
            &path("f.wav"),
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(formula.stdout)
            .unwrap()
            .contains("wrote 8 frames")
    );
    let bad = cli()
        .args(["wavetable", "formula", "sin(t)", "8", &path("bad.wav")])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

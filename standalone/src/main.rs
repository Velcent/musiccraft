//! `musiccraft-fx-standalone <effect> [options]`: runs one exported effect
//! as a standalone app (nih-plug's standalone wrapper, audio input to output,
//! with the editor). Options after the effect go to the wrapper (`--help`).

use std::process::ExitCode;

use musiccraft_plugin_export::EffectPlugin;
use musiccraft_plugin_export::ids::KINDS;
use nih_plug::wrapper::standalone::nih_export_standalone_with_args;

/// Runs plugin `index` with `args`; `None` for an unknown index.
macro_rules! run {
    ($index:expr, $args:expr; $($k:literal)*) => {
        match $index {
            $($k => Some(nih_export_standalone_with_args::<EffectPlugin<$k>, _>($args)),)*
            _ => None,
        }
    };
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let names: Vec<&str> = KINDS.iter().map(|k| k.name()).collect();
    let Some(index) = args
        .get(1)
        .and_then(|wanted| names.iter().position(|n| n == wanted))
    else {
        eprintln!(
            "usage: musiccraft-fx-standalone <effect> [options]\neffects: {}",
            names.join(", ")
        );
        return ExitCode::FAILURE;
    };
    // The wrapper sees the program name followed by its own options.
    args.remove(1);
    let ran = run!(index, args;
        0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21
        22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43);
    if ran == Some(true) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

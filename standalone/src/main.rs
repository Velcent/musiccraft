//! `musiccraft-fx-standalone <name> [options]`: runs one exported effect or
//! instrument as a standalone app (nih-plug's standalone wrapper, with the
//! editor). Effects take audio input to output; instruments take MIDI input
//! (see the wrapper's `--help`). Options after the name go to the wrapper.

use std::process::ExitCode;

use musiccraft_plugin_export::ids::{INSTRUMENT_NAMES, KINDS};
use musiccraft_plugin_export::{AntiserumPlugin, EffectPlugin, InstrumentPlugin};
use nih_plug::wrapper::standalone::nih_export_standalone_with_args;

/// Runs plugin `index` of wrapper `$plugin` with `args`; `None` for an
/// unknown index.
macro_rules! run {
    ($plugin:ident, $index:expr, $args:expr; $($k:literal)*) => {
        match $index {
            $($k => Some(nih_export_standalone_with_args::<$plugin<$k>, _>($args)),)*
            _ => None,
        }
    };
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let effects: Vec<&str> = KINDS.iter().map(|k| k.name()).collect();
    let Some(wanted) = args.get(1).cloned() else {
        return usage(&effects);
    };
    // The wrapper sees the program name followed by its own options.
    args.remove(1);
    let ran = if let Some(index) = effects.iter().position(|n| *n == wanted) {
        run!(EffectPlugin, index, args;
            0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21
            22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43)
    } else if let Some(index) = INSTRUMENT_NAMES.iter().position(|n| *n == wanted) {
        run!(InstrumentPlugin, index, args; 0 1 2 3 4 5 6 7 8 9 10)
    } else if wanted == "antiserum" {
        Some(nih_export_standalone_with_args::<AntiserumPlugin, _>(args))
    } else {
        return usage(&effects);
    };
    if ran == Some(true) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn usage(effects: &[&str]) -> ExitCode {
    eprintln!(
        "usage: musiccraft-fx-standalone <name> [options]\neffects: {}\ninstruments: {}, antiserum",
        effects.join(", "),
        INSTRUMENT_NAMES.join(", ")
    );
    ExitCode::FAILURE
}

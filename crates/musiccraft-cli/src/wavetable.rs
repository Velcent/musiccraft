//! `musiccraft-cli wavetable …`: Antiserum's wavetable sources without a UI
//! (`docs/spec/antiserum-wavetables.md`).

use std::path::Path;

use musiccraft_wavetables::import::{Slicing, from_wav_file};
use musiccraft_wavetables::{Op, Table, export, factory};

/// Usage of the `wavetable` subcommands.
pub const WAVETABLE_USAGE: &str = "\
  musiccraft-cli wavetable list                         list the factory tables
  musiccraft-cli wavetable export <name> <out.wav>      write a factory table
  musiccraft-cli wavetable import <in.wav> <out.wav> [auto|single|pitch|fixed:<len>|frames:<len>]
  musiccraft-cli wavetable formula <expr> <frames> <out.wav>";

/// Runs a `wavetable` subcommand (`args` after `wavetable`); returns what to
/// print, or an error.
///
/// # Errors
///
/// Bad arguments, an unknown table, a bad formula, or a file that cannot be
/// read or written.
pub fn wavetable(args: &[String]) -> Result<String, String> {
    let arg = |i: usize| args.get(i).map(String::as_str);
    match (arg(0), args.len()) {
        (Some("list"), 1) => Ok(factory::names()
            .iter()
            .map(|name| {
                let frames = factory::table(name).map_or(0, |t| t.frames());
                format!("{name}\t{frames} frames")
            })
            .collect::<Vec<_>>()
            .join("\n")),
        (Some("export"), 3) => {
            let name = arg(1).unwrap_or_default();
            let table = factory::table(name).ok_or_else(|| format!("no factory table `{name}`"))?;
            write(&table, arg(2).unwrap_or_default())
        }
        (Some("import"), 3 | 4) => {
            let slicing = match arg(3) {
                None => Slicing::Auto,
                Some(text) => {
                    Slicing::parse(text).ok_or_else(|| format!("unknown slicing `{text}`"))?
                }
            };
            let table = from_wav_file(Path::new(arg(1).unwrap_or_default()), slicing)?;
            write(&table, arg(2).unwrap_or_default())
        }
        (Some("formula"), 4) => {
            let frames = arg(2)
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (1..=musiccraft_wavetables::MAX_FRAMES).contains(n))
                .ok_or("frames must be 1 to 256")?;
            let mut table = Table::from_fn(frames, |_, _| 0.0);
            Op::Formula {
                text: arg(1).unwrap_or_default().to_owned(),
                first: 0,
                last: frames - 1,
            }
            .apply(&mut table)?;
            write(&table, arg(3).unwrap_or_default())
        }
        _ => Err(format!("usage:\n{WAVETABLE_USAGE}")),
    }
}

fn write(table: &Table, path: &str) -> Result<String, String> {
    export::to_wav_file(table, Path::new(path))?;
    Ok(format!("wrote {} frames to {path}", table.frames()))
}

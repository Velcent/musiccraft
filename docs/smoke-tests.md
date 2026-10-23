# Plugin smoke tests in other hosts

Our plugins checked inside real DAWs before each release
(`docs/spec/validation.md`). Most of it is scripted; what a script cannot do
is checked by hand. Use the bundles from `cargo xtask bundle --release`
(`target/bundled`).

## Scripted: `tools/host-smoke/run.sh`

Runs headless under Xvfb on Linux. It installs the bundles into a temporary
home folder (your own plugin folders and host settings are not touched),
starts JACK's dummy driver so REAPER has a running audio engine, drives each
host with its own scripting, and prints one PASS/FAIL line per check.

```sh
cargo xtask bundle --release
REAPER=/path/to/reaper tools/host-smoke/run.sh   # REAPER for Linux, unpacked
```

Needs `xvfb-run`, `jackd` and `python3`; REAPER for Linux (`REAPER`, or
`reaper` on PATH) and/or Ardour 8 (`ardour8-lua`). A missing host is skipped.
It is not part of CI: REAPER is evaluation software and Ardour is a large
install.

What it checks:

- **REAPER, CLAP and VST3** (`reaper-smoke.lua`, `reaper-sidechain.lua`):
  - REAPER lists all 112 plugins (44 effects and 12 instruments per format),
    and each one loads with its parameters.
  - Synth and Antiserum play a MIDI chord through Delay into a 48 kHz render.
  - A parameter set by the host reaches the plugin and reads back with our
    display text ("1.0 beats").
  - Instrument editors open and close in REAPER windows.
  - Saved, closed and reopened, the project has every plugin and value back,
    and renders bit for bit the same. The CLAP and VST3 synths render the
    same as each other.
  - Sidechain through REAPER's routing: a key track sent to channels 3/4 of a
    compressor's track. The key changes the sound when `sidechain` is on; a
    silent key leaves the sound dry; CLAP and VST3 sound the same.
- **Ardour, VST3** (`ardour-smoke.lua`, through `ardour8-lua`):
  - Ardour's scanner finds all 56 VST3 plugins.
  - In a session, MIDI tracks with Synth and Antiserum plus Delay play live
    MIDI (the master bus is silent before, loud during).
  - A Delay parameter set through Ardour's control takes effect.
  - Saved, closed and reopened, the session has the plugins and the value
    back.

## By hand

Still checked by a person, in hosts the script does not cover (Bitwig Studio
needs an account; Windows and macOS hosts need those systems):

1. Pitch bend from the host's piano roll bends the synth; Antiserum plays a
   factory preset chosen in its editor.
2. Dragging a knob in the plugin's editor records automation in the host,
   and the host's automation moves the knob.
3. The pitch corrector follows a MIDI track when `follow` is on (hosts that
   route MIDI to effects).
4. The convolution reverb loads a WAV impulse chosen with its editor's Load file…, and the
   file comes back when the project is reopened.
5. Open and close each editor several times, resize the host window, and
   close the project with editors open: no crash, no frozen window.
6. Play with the host's smallest buffer size on real audio hardware: no
   dropouts with one Antiserum instance and five effects.

## Results

| Date | Host and version | OS | Format | How | Result |
| --- | --- | --- | --- | --- | --- |
| 2026-10-21 | REAPER 7.82 | Linux x86-64 (Ubuntu 24.04) | CLAP, VST3 | scripted | All checks pass. |
| 2026-10-21 | Ardour 8.4 | Linux x86-64 (Ubuntu 24.04) | VST3 | scripted | All checks pass. Ardour 8.4 does not host CLAP. |
| 2026-10-21 | Carla 2.5.8 (discovery) | Linux x86-64 (Ubuntu 24.04) | VST3 | `carla-discovery-native` | All 56 plugins found with the right audio, MIDI and parameter counts. This Carla build has no CLAP support. |
| — | Bitwig Studio | — | — | by hand | Not run yet: needs a person with an account. |

### Found and fixed by these runs

- The Antiserum plugin's reset (which hosts call before a render) only
  released the notes, so release tails and the effects' tails sounded into
  the next render. It now stops every voice and restarts the random state.
- The built-in instruments' reset left each voice's noise generator and
  filter state where it was, so the drum synth rendered differently each
  time. A reset now returns every voice to new, without allocating.
- The chorus, flanger, pitch shifter and hall reverb kept their LFO or sweep
  phase through a reset. A test now checks every effect against a new one
  after a reset.

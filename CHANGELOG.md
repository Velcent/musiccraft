# Changelog

Notable changes per release. GitHub's generated release notes list every
commit; this file says what they add up to.

## Unreleased (since 0.3.0)

### A more compact interface

- The toolbar is one band in a wide window: menus over the hint bar, then
  the transport, then the window buttons.
- The channel rack, piano roll and mixer put their tools on the title row.
- Mixer strips are half as wide, with names written vertically, so twice as
  many inserts fit.
- Taller step buttons, shaded more clearly by beat; a narrower browser,
  taller playlist tracks and a taller piano roll event lane.

### Pattern colours

- Patterns have no colour until you set one (right-click a pattern →
  Colour, or `pattern.set_color`). Uncoloured clips are drawn neutral, and
  the notes inside every pattern clip show their own colours, as in the
  piano roll, so one pattern can show several.
- The playlist's picker shows each pattern as a miniature of its notes, so
  patterns with default names are easy to tell apart. In previews, a
  step-sequenced pattern (every note on one key, as drums usually are) gets
  one row per channel.

### Fixes

- Rendering a project twice in a DAW gives the same audio. A host reset,
  which DAWs do before a render, now makes every plugin as good as new:
  - Antiserum stops its voices and effect tails at once, where it used to
    only release them, and restarts its random phases.
  - The built-in instruments return each voice to new; the drum synth's
    noise used to differ on every render.
  - The chorus, flanger, pitch shifter and hall reverb restart their LFO and
    sweep phases.

### Checked in other DAWs

- `tools/host-smoke/run.sh` drives REAPER (CLAP and VST3) and Ardour (VST3)
  headless. It loads all 112 plugins, plays and renders, sets parameters
  from the host, opens editors, routes a sidechain, and saves and reopens.
  Results are in `docs/smoke-tests.md`: every check passes in REAPER 7.82
  and Ardour 8.4.

## 0.3.0 (2026-10-20)

### Antiserum, the built-in wavetable synth

- Three oscillators, each a wavetable, a sample (repitch, fixed or stretch,
  with loops), key- and velocity-mapped multisample zones, a granular cloud or
  a spectral resynthesis; stereo samples, unison and warps on every type.
- Sub and noise oscillators, two filters with 18 types, four envelopes, eight
  LFOs with drawn shapes, eight macros and a 32-slot modulation matrix.
- An effects rack (13 modules), an arpeggiator and a 64-step sequencer.
- A wavetable editor (draw, harmonics, formula, morph, import and export with
  frame-size chunks) and 11 generated factory tables.
- About 165 factory presets, plus user presets saved to a folder of their own,
  listed in the browser and shared with the plugin.
- The channel's pitch knob transposes it; its parameters can be automated.

### MusicCraft in other DAWs

- **MusicCraft Effects**: the mixer's 44 effects as CLAP and VST3 plugins.
- **MusicCraft Instruments**: the 11 built-in instruments and Antiserum.
- Every plugin opens the same editor as the app. Presets are shared, and
  automation is sample-accurate. Plugins follow the host's tempo and transport.
- The compressor and vocoder take a sidechain input. The pitch corrector
  follows MIDI notes. The convolution reverb loads WAV impulse files.
- Release downloads include the plugins for Linux, Windows and macOS, with
  install notes.

### Quality

- The plugins pass clap-validator and the VST3 SDK's validator, run on every
  CI build. A project rendered with a plugin sounds the same as with the
  built-in instrument or effect.
- Subnormal input no longer slows effects (up to 18× before). The frequency
  shifter's and DC blocker's tails end in silence.
- Parameter text reads back as typed (no `-0.0`; 999.7 Hz shows `1.00 kHz`).
- Hosting: our CLAP and VST3 hosts feed plugins' sidechain inputs.

### Export

- MP3 (128–320 kbit/s, LAME) and Ogg Vorbis (quality 0–10) export, in the
  export dialog and as `project.export_mp3` / `project.export_ogg`, with the
  project's name as the title tag.

### Fixes

- Exports start at the beginning of the song even with a loop region set;
  empty samplers load silently instead of warning.
- Windows and FL Studio sample paths are found on Linux and macOS.

### Known limitations

- Builds and plugins are not code-signed or notarized (see
  `docs/releasing.md`).
- The plugins had not yet been tried in Ardour, Bitwig or REAPER
  (`docs/smoke-tests.md`); REAPER and Ardour have been since.

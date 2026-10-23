# Spec: MusicCraft's instruments and Antiserum as plugins

Phase 7 of `docs/roadmap-wavetable-and-plugins.md`, after the effects
(`docs/spec/plugin-export.md`). The same wrapper crate, editor adapter, state
rules and tests apply; this spec covers what is different for instruments.

## What is exported

Twelve instruments, in one library, **MusicCraft Instruments**
(`MusicCraft Instruments.clap`, `MusicCraft Instruments.vst3`). A host lists
each as its own instrument plugin.

| Plugin | Name (`org.musiccraft.<name>`) | Data besides parameters |
| --- | --- | --- |
| Synth | `synth` | none |
| Drum synth | `drum_synth` | none |
| Pluck | `pluck` | none |
| FM | `fm` | none |
| Keys | `keys` | none |
| Additive | `additive` | none |
| Sampler | `sampler` | one sample |
| Granular | `granular` | one sample |
| Resynth | `resynth` | one sample (its harmonics are measured on load) |
| Kit | `kit` | sixteen pad samples |
| Zones | `zones` | sample zones (key and velocity ranges, root key) |
| Antiserum | `antiserum` | the patch: tables, drawn LFO shapes, sequencer steps, oscillator samples and zones |

Not exported: **layer** (it routes notes to other channels), **MIDI out**
(it drives external gear) and hosted plugins. A host does both itself.

- **IDs.** The names are the project file's instrument names. CLAP IDs are
  `org.musiccraft.<name>`; VST3 class IDs are `MCin` followed by the first 12
  bytes of the same 128-bit FNV-1a hash the effects use. A test freezes every
  ID, together with the effects' IDs, which never change.
- **Features.** CLAP: `instrument`, `synthesizer` (or `sampler` and
  `drum-machine` for the sample players and the kit), `stereo`. VST3:
  `Instrument` plus `Synth`, `Sampler` or `Drum`.
- **Audio.** No audio input, stereo output.

## Instruments

- **Sound.** Voices are the app's own `musiccraft_instruments::Voice`, so a
  plugin sounds like the same channel in the app.
- **Parameters.** One host parameter per name in `Kind::params()`: the
  envelope (where the instrument has one), its sound parameters, then the note
  functions. IDs are the parameter names, which the `frozen_names` test
  already freezes. Ranges, skew, integer steps, display text and typed text
  are the effects' rules, with the instrument table's descriptions
  (`musiccraft_params::describe`). Two parameters belong to the plugin:
  - `swing` (0–1, default 0): what the project's swing is in the app.
    `swing_mix` scales it, as in the app.
  - `bend_range` (0–24 semitones, default 2): how far MIDI pitch bend moves
    every voice.
- **Notes.** Notes come from the host's MIDI or CLAP note events at their
  exact frame. The block is split at every event. Note-off releases the
  voice; choke and all-notes-off cut it. Velocity is the note's velocity.
  Pitch bend adds `bend × bend_range` semitones to the `pitch` parameter.
- **Voices.** Up to 64 voices per plugin instance. `mono` and `poly_max`
  limit them as in the app (the oldest is cut). Glide follows the newest held
  note, as in the app. Cut groups act inside the instance: when `cut_group` is
  set and equals `cut_by`, a new note cuts the instance's sounding notes, the
  way an open hi-hat is choked by itself in the app.
- **Note functions.** In the app these are applied when a sequence is built.
  A plugin applies the same rules in real time, in the same order: swing,
  then the arpeggiator, then time shift, gate and echo.
  - *Swing.* A note that starts within 1 tick of an off-beat sixteenth of the
    host's position (while the host plays) starts up to half a step later:
    `swing × swing_mix` of half a step. With `swing_truncate` its end stays
    put; otherwise the whole note moves.
  - *Arpeggiator.* Steps fall on multiples of the rate in the host's beat
    position while it plays, and on a free clock at the host's tempo while it
    is stopped. Each step plays the next key, in the app's order, of the keys
    held at that step, spread over `arp_range` octaves. A key pressed while
    nothing was held starts a step at once. Each arpeggio note lasts
    `arp_gate` of a step and has the loudest held velocity. Random order uses
    the app's stable hash of the step number.
  - *Time shift.* A positive shift delays every note by that many ticks at
    the host's tempo. A negative shift cannot play notes before they arrive,
    so the plugin reports that many frames of **latency** and plays notes at
    once. The host then moves the plugin's output earlier by the same amount,
    which is the same result. The latency follows the parameter.
  - *Gate.* When set, every note lasts exactly that many ticks, and the
    host's note-off is ignored.
  - *Echo.* Each note repeats `echo_count` times, `echo_time` ticks apart,
    each repeat `echo_feed` times as loud and `echo_pitch` semitones higher
    (or lower) than the one before, stopping below velocity 1/127 or outside
    the key range. A repeat ends `echo_time × n` after the note's own end.
  - Ticks are 96 per beat, as in the app.
- **Samples.**
  - A sample is a file path (absolute) or a built-in sound (`builtin:kick`).
  - Samples are loaded and analysed off the audio thread, in the editor or
    while state loads. They reach the audio thread through the same lock-free
    hand-over the convolution reverb uses, and the old data is freed off the
    audio thread.
  - A sample that cannot be read leaves the instrument silent for the notes
    that would play it, and the editor shows the error. The path is kept, so
    the state still names it.
- **Presets.** The built-in instruments' factory presets (the app's preset
  bar) are in the editor. Choosing one sets every parameter the preset names
  as one host gesture per parameter.

## Antiserum

- **Sound.** The same `musiccraft_antiserum::Antiserum` the app runs.
- **Parameters.** Every entry of Antiserum's parameter table, in table order,
  with the table's names as IDs (frozen by Antiserum's own `frozen_names`
  test). Display text is the app's (`musiccraft_antiserum_presets::text`).
  Typed text accepts a choice name, a value with or without its unit, or a
  percentage for bipolar amounts.
- **Notes and controllers.**
  - Notes start and end at their frame, as for the instruments.
  - Pitch bend, the mod wheel (CC 1), channel pressure and polyphonic key
    pressure reach the synth's pitch bend, mod wheel, aftertouch and poly
    aftertouch sources.
  - Antiserum's own arpeggiator and sequencer follow the host's tempo and
    position.
  - When the host resets the plugin (processing starts again, for example
    before a render or after a transport jump), every voice stops at once and
    the effects rack's tails are cleared, as for the other instruments. Notes
    released earlier do not keep sounding into the next render, so rendering
    a project twice gives the same audio.
- **Data.**
  - Antiserum keeps data that is not a parameter: wavetables, drawn LFO shapes,
    sequencer steps and oscillator sources.
  - It is built off the audio thread when the editor changes it or state
    loads, and swapped into the running synth in one step through a lock-free
    queue. What the swap replaces comes back to be freed off the audio thread.
    This is the same `Data` swap the app's engine uses.
- **Presets.**
  - The editor's preset bar has the 150 factory presets.
  - **Save** and **Open** write and read patch files (`.json`), the app's own
    format, so a sound moves between the app and any host.
  - Loading a preset or a file sets every parameter through the host (one
    gesture each) and swaps in its data.

## State

The host saves nih-plug's parameter map plus our `musiccraft` field. Values
come back through the parameter map; the field holds the rest.

- Instruments:
  `{"format":1,"instrument":"<name>","values":{…},"sample":"…","pads":[…],"zones":[…]}`.
  The `values` are the app's preset shape. `sample`, `pads` and `zones` are
  present only for the instruments that use them, with the project file's
  zone fields.
- Antiserum: `{"format":1,"instrument":"antiserum","patch":{…}}`. The patch is
  the app's patch format (`musiccraft-patch`), so state and patch files are
  the same thing.
- A field written for another instrument is ignored. An unknown format is
  ignored, and the plugin keeps its defaults.

## Editors

- **Instruments.** The same instrument editor as the app's channel settings,
  moved into `musiccraft-editors` and shared:
  - pages Sound, Envelope (where the instrument has one) and Functions;
  - the preset bar, knobs, choices, the filter, envelope and harmonics graphs.

  The plugin adds its own `swing` and `bend_range` knobs to the Functions
  page, and the `pitch` knob (in the app's channel header) next to its
  title. Sample instruments show the sample's name, its waveform and a
  *Sample…* button; the kit shows its sixteen pads and a *Pad sound…* button
  for the chosen pad, and zones show a zone list with key ranges and *Add
  zone…*. These buttons open the sample slot menu of
  `docs/spec/style/dialogs.md` (browse files with the system's file dialog,
  the built-in sounds, the user's Music, Samples and Music/Samples folders,
  and Clear for a pad).
- **Antiserum.** The app's Antiserum editor (`musiccraft-editors::antiserum`),
  with the same sample slot menus. Browsing for a sample or zone, and saving
  and opening a patch, use the system's file dialog (patches start in the
  user presets folder shared with the app).
- **Gestures and size.** Edits are host gestures, as for the effects. The
  window has a fixed size per plugin and follows the host's scale.

## The app

- The app's channel settings window uses the shared instrument editor. Its
  edits become the same commands as before (`channel.set_param`, a batch for a
  preset). The kit and zones pages gain the playback controls that had no
  knobs before: the kit's pad-1 key and fades, and the zones' fades, glide,
  one-shot and reverse.
- The sample-related parts that need the project stay in the app: dialogs,
  the sample editor, slicing to a pattern, and previews.

## Packaging

- `cargo xtask bundle [--release]` builds the plugin crate twice: with the
  `effects` feature as **MusicCraft Effects**, and with the `instruments`
  feature as **MusicCraft Instruments**.
- Both features are on by default, for tests and the standalone apps.
- `musiccraft-fx-standalone <name>` runs any exported effect or instrument
  standalone. An instrument takes MIDI input from the wrapper's MIDI options.

## Tests

- **IDs.** Every exported instrument's IDs are unique, and frozen alongside
  the effects' IDs.
- **Parameters.**
  - Each instrument's parameter list is `Kind::params()` plus `swing` and
    `bend_range`.
  - Antiserum's is its whole table.
  - Normalized values round-trip, and the text matches the app's.
- **Note functions.**
  - Real time against the app's sequence-time version, for the same notes:
    the arpeggiator (modes, octaves, rates, gate), time shift, gate, echo,
    and combinations of them.
  - Unit tests:
    - swing, against the app's rule (shift and truncate, only while the host
      plays);
    - a key pressed after a silence;
    - negative-shift latency.
- **Voices.** Mono, the polyphony limit and self-choke match the engine's
  rules.
- **Hosts.**
  - Our CLAP and VST3 hosts load the bundled library from disk and list all 12
    instruments.
  - Each instrument, played the same notes through the plugin and directly
    through `Voice` (or `Antiserum`), matches block for block.
  - The library is built with `assert_process_allocs`, so an allocation on the
    audio thread fails the test.
- **State.**
  - State saved by one instance and loaded into another restores parameters,
    samples, pads, zones and Antiserum's patch data, through both hosts.
  - An Antiserum state holds a patch in the app's format, with its tables,
    LFO shapes and steps.
- **Editors.**
  - The shared instrument editor draws every instrument and page headlessly,
    and its edits match the app's previous commands.
  - The plugins' editors draw headlessly.

## Sources

- Own design, on the open CLAP specification and the VST3 SDK under its GPLv3
  option, through nih-plug (ISC; GPLv3 VST3 export).
- The instruments' and note functions' behavior is the app's
  (`docs/spec/channel-rack.md`, `docs/spec/engine.md`); Antiserum's is
  `docs/spec/antiserum*.md`.
- Roadmap decisions D1, D2, D5 and D6.

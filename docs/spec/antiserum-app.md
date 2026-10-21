# Antiserum in the app: patches, presets, editor

Roadmap Phase 6. How Antiserum (`docs/spec/antiserum.md`,
`docs/spec/antiserum-wavetables.md`, `docs/spec/antiserum-fx-arp.md`)
becomes an instrument in MusicCraft: its patch format and factory bank, how a
channel plays it, the commands that edit it, and its editor.

## Patch

A **patch** is everything that makes an Antiserum sound. It is MusicCraft's
own JSON, the same in project files, preset files and (later) the plugin's
state. Crate: `musiccraft-patch` (plain data, serde only).

```json
{
  "name": "Glass Reese",
  "category": "bass",
  "tags": ["reese", "dark"],
  "params": { "osc1_unison": 7, "filter1_cutoff": 900 },
  "tables": ["factory:analog", "factory:basic", "data:AACAPwAA…"],
  "lfo_shapes": { "1": [[0, 0, 0], [0.5, 1, -0.4]] },
  "sequence": [[true, 0, 1, 0.5], …]
}
```

- `params`: values by parameter name. Missing names take their defaults and
  unknown names are ignored, so patches survive new parameters. Values are
  sanitized into range when applied.
- `tables`: one per oscillator. `factory:<name>` names a generated table
  (`musiccraft_wavetables::factory`). `data:<base64>` holds the frames as
  little-endian 32-bit floats, for edited or imported tables. Missing entries
  are `factory:basic`.
- `lfo_shapes`: drawn shapes by LFO number (1–8), each a list of
  `[x, y, curve]` points. Missing LFOs keep the default triangle.
- `sequence`: the sequencer's steps as `[on, transpose, velocity, gate]`
  (missing steps keep the defaults).
- `category` is one of `bass`, `lead`, `pad`, `pluck`, `keys`, `fx`, `drums`
  (or empty for a user patch).

`musiccraft-antiserum-presets` turns a patch into a running synth
(`apply(&patch, &mut Antiserum)`), formats any parameter's value for display
(`text(index, value)`), and holds the factory bank.

## Factory bank

About 165 presets of our own sound design, made in code from a few
parameter recipes per sound family, with these counts:

| Category | Presets |
| --- | --- |
| bass | 25 |
| lead | 25 |
| pad | 29 |
| pluck | 23 |
| keys | 22 |
| fx | 18 |
| drums | 22 |

Fourteen of them play the sample, multisample, granular and spectral
oscillators (`docs/spec/antiserum-oscillators.md`) with the built-in sounds,
so they work on every machine; every oscillator type has at least one.

Every preset has a unique name and at least one tag. Each one sounds (a
held note is not silent), stays finite and peaks below 4. Names are our own
and copy no product's preset names.

## Playing

- A channel with `Instrument::Antiserum { patch }` runs one synth in one of
  the engine's external slots, the same slots hosted plugins use (at most
  128 together).
- Notes, gates, live MIDI, pitch bend and the transport's tempo and position
  reach it like they reach a plugin instrument. Channel volume, pan, mute and
  the mixer track work as for any channel.
- Changing a parameter sends only that parameter. Changing a wavetable, an
  LFO shape or the sequence sends the new data to the running synth, so held
  notes keep sounding. The engine hands the replaced data back to the main
  thread to be freed (`EngineMessage::ExternalData`, `Garbage::Data`).
- Automation and controllers target Antiserum parameters by name
  (`channel:N:<param>`), like other instruments. The engine writes them to
  the synth.

## Commands

| Command | Does | Undo |
| --- | --- | --- |
| `channel.add_antiserum [preset]` | Adds a channel, with a factory preset or the init patch. | `channel.remove` |
| `channel.set_param <ch> <name> <value>` | On an Antiserum channel, names are Antiserum's. | previous value |
| `antiserum.set_table <ch> <osc 1–3> <factory:name\|data:base64>` | Replaces a wavetable. | previous table |
| `antiserum.set_lfo_shape <ch> <lfo 1–8> <x:y:curve,…>` | Replaces a drawn LFO shape. | previous shape |
| `antiserum.set_step <ch> <step 1–64> <on> <transpose> <velocity> <gate>` | Edits one sequencer step. | previous step |
| `antiserum.load_preset <ch> <name>` | Loads a factory preset by name. | `antiserum.set_patch` with the old patch |
| `antiserum.set_patch <ch> <json>` | Replaces the whole patch. | the old patch |
| `antiserum.save_patch <ch> <path>` | Writes the patch as a `.json` file. | (not undoable: a file) |
| `antiserum.open_patch <ch> <path>` | Loads a patch file. | the old patch |
| `antiserum.presets [category]` | Lists factory presets (name, category, tags). | (read only) |

Errors name the problem: an unknown parameter or preset, a channel that is
not Antiserum, an oscillator out of range, a malformed table or shape.

## Editor

The channel settings window's **Sound** tab for an Antiserum channel shows
the editor from `musiccraft-editors::antiserum`. It reports edits, which the
app turns into the commands above. A plugin can turn them into host gestures
later.

- **Preset bar:** the preset's name, previous/next, a menu by category with a
  search box that matches names and tags, plus save and open.
- **Modulation sources strip:** chips for envelopes 1–4, LFOs 1–8, macros
  1–8, velocity, note, mod wheel, aftertouch and random. **Drag a chip onto
  any continuous knob** to route it there: the first free matrix slot gets
  that source and destination with amount 0.5. An existing route from the
  same source to the same knob is left alone.
- **Modulation rings:** a knob with routes shows an arc beside its value arc
  for each route's amount (positive clockwise, negative counter-clockwise).
- **Pages** (tabs):
  - *Oscillators:* the three oscillators and the sub and noise oscillators.
    Each wavetable oscillator shows its table: the current frame at its
    position, a factory table menu and **Edit…**. Edit… opens the wavetable
    editor (Phase 3) on that table; each finished gesture there becomes one
    `antiserum.set_table`.
  - *Mixer:* levels, pans and filter sends of every oscillator, and the
    filter routing.
  - *Filters:* both filters.
  - *Modulation:* the envelopes, and the LFOs with their drawn shapes. Click
    to add a point, drag a point, right-click a point to remove it. When the
    LFO is tempo-synced, the view is marked in beats of its rate.
  - *Matrix:* the 32 slots as rows: source, destination, amount, curve,
    bipolar and auxiliary source.
  - *Effects:* the ten slots in order. Each filled slot shows its module's
    knobs.
  - *Arp/Seq:* the arpeggiator and the sequencer settings, and the 64 steps
    as a grid (on, transpose, velocity, gate) showing the first
    `seq_length` steps.
  - *Global:* voicing, glide, bend range, volume and the eight macros.
- Choice parameters are menus and on/off parameters are switches; the rest
  are knobs. Every knob shows its value text while hovered or dragged,
  resets on double-click or middle-click, and has the automation context
  menu like other channel knobs.

- The channel header (volume, pan, mute, mixer track) and the Functions tab
  (swing, timing, echo, cut groups) work as for other channels; they shape
  the notes before they reach the synth. The Functions tab has no channel
  arpeggiator, because Antiserum has its own (its `arp_*` parameters, on the
  Arp/Seq page).
- The header's **pitch** knob transposes Antiserum: every note, sounding
  ones included, moves by it, like pitch bend. The engine hands it to the
  synth before every block (`External::set_transpose`), so automation of
  `channel:<n>:pitch` moves it too. On an Antiserum channel,
  `channel.set_param` and automation send Antiserum's own names to its patch
  and `pitch` and the note functions to the channel.

## User presets

Patches the user saves as presets live in a folder of their own, shared by
the app and the Antiserum plugin:

- `presets/antiserum` in MusicCraft's per-user data folder
  (`$XDG_DATA_HOME/musiccraft`, or `~/.local/share/musiccraft` on Linux;
  `~/Library/Application Support/MusicCraft` on macOS; `%APPDATA%\MusicCraft`
  on Windows). The `MUSICCRAFT_USER_PRESETS` environment variable names
  another folder instead.
- Each preset is a patch file, `<name>.json`. Its name is the patch's name
  (the file's name if the patch has none); its category is the patch's.
- `antiserum.save_preset <channel> <name>` names the channel's sound and
  saves it there, replacing a user preset of that name. A factory preset's
  name is refused. Undo restores the sound's old name (the file stays).
- `antiserum.delete_preset <name>` removes a user preset's file.
- `antiserum.presets [category]` lists the factory presets, then the user
  presets (tagged `user`). `antiserum.load_preset` and
  `channel.add_antiserum` accept either kind; a factory name wins.
- The editor's preset bar lists the user presets after the factory ones,
  and its **Save…** starts in the user folder, so a saved patch shows up as
  a preset at once. The folder is read again every few seconds.

## Browser

The browser's generators list has **Antiserum**. Its presets are in the
browser's **Presets** category (`browser.md`): an Antiserum folder with the
factory presets grouped by category, followed by a **User** group with the
user presets. A click selects a preset; a double-click, or dragging it onto
the channel rack, adds an Antiserum channel with that preset.

## Sources

- Own design. Drag-and-drop modulation, modulation rings, preset browsers
  with tags and wavetable views are common in software synthesizers, as their
  public manuals describe. No layout, artwork, preset or preset name is taken
  from any of them.
- Factory presets: our own sound design.

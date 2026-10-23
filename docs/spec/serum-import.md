# Spec: importing Serum presets into Antiserum

Antiserum can play presets saved by Xfer Serum 1 (`.fxp`) and Serum 2
(`.SerumPreset`), so preset packs bought for Serum work in MusicCraft. Import
converts a preset into an Antiserum patch (MusicCraft's own JSON) once. Nothing
reads Serum files at play time, and a converted patch is an ordinary patch.

## Commands

- `antiserum.open_patch <channel> <path>` also opens Serum presets. The file's
  first bytes decide (`CcnK` for Serum 1, `XferJson` for Serum 2, otherwise a
  patch JSON), not its name. The editor's **Open…** prompt therefore takes
  them too. Undo restores the old patch, as for any patch.
- `antiserum.import_serum <channel> <path>` does the same and refuses
  anything that is not a Serum preset.
- `antiserum.import_serum_presets <folder>` converts every `.fxp` and
  `.SerumPreset` under the folder (recursively) and saves each as a user preset
  (`antiserum-app.md`, User presets). An existing user preset of the same name
  is not replaced: the new one gets ` (2)`, ` (3)`, … Files that are not Serum
  presets (other plugins' `.fxp` files are common in packs) are skipped and
  counted. Not undoable (it writes files), like `antiserum.save_preset`.

The message of each command says what was converted and lists what was
approximated or dropped, one note per item (see Report).

## Reading

The file formats are read as serum2vital reads them (`docs/clean-room.md`).
`musiccraft-serum` ports serum2vital's `serum1.py`, `serum2.py`,
`serum_params.py` and the menu orders and unit curves in `serum_tables.py`;
changes follow serum2vital, not our own probing.

- **Serum 1**: a VST2 opaque-chunk preset (`CcnK`, `FPCh`, plugin id `XfsX`)
  around a zlib stream. A `.fxp` from another plugin is refused as "not a
  Serum preset". Inside: 299 normalised parameters (two blocks; the second is
  missing or partly stale in files from older builds, which read as Serum's
  defaults, by blob size as serum2vital does), the wavetable and noise names,
  preset name, author, bank and macro names, the 32 modulation slots (found by
  their record marker), the eight LFO curves and switches (classic and new
  layouts), the effect rack order and the global switches block (voicing,
  portamento, noise switches, filter key tracking, unison range and tuning).
- **Serum 2**: `XferJson\0`, a 64-bit JSON header length, the JSON header
  (name, author, description, tags), then a 32-bit size, a 32-bit encoding
  (2: zstd) and a zstd frame holding a CBOR map of module name to module state.
  A module's `plainParams` holds only the values that differ from Serum's
  defaults, so the converter supplies the defaults serum2vital lists. The CBOR
  decoder is our own (RFC 8949); zstd is decoded by `ruzstd`.

## Conversion

Values are converted with the unit laws serum2vital measured (Serum 1 stores
knob positions, Serum 2 stores real units). Antiserum's parameters are named
in `antiserum.md`.

- **Global**: master volume (Serum's knob is cubic in amplitude, Antiserum's
  squared), polyphony, mono and legato, portamento time (8·n⁵ s, at most 5 s),
  "always" portamento, the portamento curve, pitch bend range (the up range).
  Macros 1–4 (Serum 2: 1–8) carry their values.
- **Oscillators** A and B (Serum 2: A, B, C) become oscillators 1–3 with level
  (Serum's level knob is quadratic, Antiserum's linear), pan, octave, semitone
  plus Serum's third coarse pitch control (folded into octave and semitone,
  clamped to ±4 octaves ±12 semitones), fine tune, wavetable position, start
  phase, random phase, unison voices, detune (scaled by Serum's unison range,
  Antiserum's spread being fixed at ±100 cents), blend, stereo width and the
  unison stack (nearest of Antiserum's five), and the warp mode and amount
  (nearest of Antiserum's fourteen; FM, AM and RM keep their source).
  An oscillator Serum routes to the filter sends fully to filter 1, otherwise
  it goes straight to the output. Serum 2's sample, multisample, granular and
  spectral engines are converted as wavetable oscillators (reported).
- **Sub**: shape (sine, triangle for rounded rectangle, saw, square, square
  for pulse), octave (clamped to −4…0), level (linear in both), pan, filter
  routing.
- **Noise**: level, pan, pitch (serum2vital's law), key tracking, one-shot,
  filter routing. Serum's noise is a sample file; Antiserum's noise is
  generated, so the type is chosen from the sample's name (pink, brown,
  crackle or digital when the name says so, white otherwise) and reported.
- **Filter** (Serum 2: both voice filters): on, type (nearest of Antiserum's
  eighteen; table in `musiccraft-serum/src/filters.rs`), cutoff (Serum's
  8 Hz–22.05 kHz knob, clamped to 20 Hz–20 kHz), resonance, drive, mix and key
  tracking. Serum's types with no counterpart (ring modulation, sample and
  hold, reverb, all-pass and similar effects) turn the filter off. The
  variation knob, filter stereo and Serum 2's level out are dropped.
- **Envelopes** 1–3 (Serum 2: 1–4): times (Serum 1: 32·n⁵ s; Serum 2: seconds)
  clamped to Antiserum's 10 s, sustain (envelope 1 is quadratic in Serum),
  and the attack, decay and release curves (50 % straight in Serum, 3·(c − ½)
  in Antiserum, so both bend the same way).
- **LFOs** 1–8: the curve becomes a drawn shape (y flipped, Serum's tension
  turned into Antiserum's curve, the loop closed as Serum closes it), mode
  (trigger, envelope, free), rate (Hz mode: 100·n⁴ Hz; synced: the division
  with dotted and triplet, nearest of Antiserum's seventeen), rise and delay.
  Smoothing, and Serum 2's LFOs 9–10, are reported as dropped.
- **Modulation matrix**: up to 32 routings. Sources map to Antiserum's
  (envelopes, LFOs, velocity, note, aftertouch, mod wheel, pitch bend, note-on
  random, macros); chaos, alternating, MPE, release velocity, fixed and audio
  sources are dropped with a note. Destinations are mapped by name; the amount
  is Serum's amount times its output range, rescaled where the two parameters'
  ranges differ (pitch, cutoff). The bipolar switch and the aux source carry
  over.
- **Effects**: each enabled effect takes a slot in Serum's rack order, using
  Antiserum's module of the same kind (distortion, flanger, phaser, chorus,
  delay, compressor or multiband compressor, reverb, EQ, filter; Serum's
  Hyper/Dimension becomes the dimension module). The main controls carry over
  (mix, drive, rate, depth, feedback, time, size, decay, threshold, ratio,
  gains, frequencies). Serum 2's FX buses and its convolution, Bode, utility
  and split modules are dropped with a note. Antiserum's rack holds each module
  once, so a second Serum 2 module of the same kind is dropped with a note.

## Wavetables

Serum presets name their wavetables; the tables live in the user's Serum data
folder (Serum ships its own and packs add theirs). The importer looks for a
named table:

1. next to the preset: in a `Tables` folder in the preset's folder or any
   folder above it (packs often ship their tables beside their presets);
2. in the folder named by `MUSICCRAFT_SERUM_FOLDER`;
3. in the usual Serum data folders: `Documents/Xfer/Serum Presets` and
   `Documents/Xfer/Serum 2 Presets` in the home folder, macOS's
   `/Library/Audio/Presets/Xfer Records/…`, and on Linux the same folders
   inside Wine prefixes (`~/.wine/drive_c/users/*/Documents/Xfer/…`), where
   Serum runs through yabridge.

In each, the table is looked up by its path under `Tables/` (or
`Serum 2 Presets/Tables/`), then by file name anywhere below. A found table is
read like any imported wavetable (`antiserum-wavetables.md`: the `clm ` chunk
gives the frame size) and stored in the patch as table data, so the patch
plays without the Serum folder. Serum's built-in basic shapes (sine,
triangle, saw, square, pulse, rounded rectangle) are generated. A table that
is not found becomes a single saw frame and is reported.

## Report

Each note says what happened to one Serum feature: approximated, dropped, or
not found. Notes are not repeated. The command message lists them after the
summary line. The folder command's summary counts converted, skipped and
failed files; then each converted preset follows with its notes, and each
failed file with the reason.

## Sources

- The Serum file formats, parameter table, menu orders and unit laws as
  serum2vital reads and measures them: serum2vital
  (https://github.com/btesser/serum2vital, commit `b67d9d3`), © 2026 Ben
  Tesser, GPL-3.0-or-later, ported to Rust by the owner's decision of
  2026-10-23 (`docs/clean-room.md`). serum2vital credits 0xdevalias'
  public notes for the Serum 1 parameter list.
- CBOR: RFC 8949 (open standard). zstd: RFC 8878 (open standard), decoded by
  `ruzstd`. zlib: RFC 1950 (open standard), decoded by `miniz_oxide`. VST2
  `.fxp` container: as serum2vital documents it (`docs/FORMATS.md`).
- The mapping onto Antiserum (which Antiserum control stands in for which
  Serum one, the noise type guess, the wavetable search order) is our own
  design.

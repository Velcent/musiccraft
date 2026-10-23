# Clean-room policy

MusicCraft is an independent implementation of a pattern-based DAW workflow. It is
not affiliated with, endorsed by, or derived from any commercial DAW.

## Why this document exists

Claiming "clean room" is only meaningful if the rules are written down and followed.
A classic clean room uses two separate teams: one studies the original and writes a
specification, the other implements from the specification without ever seeing the
original. This project is built by one developer working with an AI coding agent, so
we cannot claim that separation. Instead we commit to the rules below and record
provenance for every behavior we implement. Nobody has independently verified this
process.

## Allowed sources

1. The publicly published user manual and public tutorials of existing DAWs.
2. Observable behavior of software that a contributor lawfully owns and runs
   normally (what happens when you press a key, what a dialog offers).
3. Open standards and public specifications: MIDI, WAV/RIFF, CLAP, VST3, and any
   standard whose license permits our use.
4. Our own design decisions.
5. Open-source code whose license is compatible with MusicCraft's
   (GPL-3.0-or-later), copied or ported with its copyright notice and license
   kept, and named in the spec's Sources. In particular, the `.flp` importer
   (`musiccraft-flp`) is a Rust port of PyFLP (© 2022 demberto,
   GPL-3.0-or-later). The project owner decided this on 2026-10-16, together
   with licensing MusicCraft under GPL-3.0-or-later. Likewise, the Serum preset
   importer (`musiccraft-serum`) is a Rust port of the reading and mapping parts
   of serum2vital (© 2026 Ben Tesser, GPL-3.0-or-later), by the owner's decision
   of 2026-10-23 (`docs/spec/serum-import.md`).

## Forbidden

- Decompiling, disassembling, debugging, or tracing the binaries of any commercial DAW.
- Any leaked, stolen, or otherwise unauthorized source code, documentation, or assets.
- Reverse-engineering proprietary project/preset formats ourselves (examining
  their files byte by byte, or the program that writes them). The two exceptions
  are the `.flp` import and the Serum preset import above, which reuse published,
  open-source parsers (PyFLP, serum2vital) instead; their authors worked the
  formats out, not us, and we say so. Changes to either importer follow its
  upstream (or are checked against its output), not our own probing. MusicCraft's
  own project and patch formats stay its own.
- Copying icons, skins, bitmaps, fonts, sample packs, factory presets, logos, or names.
- Using another product's trademarks in our name, branding, or marketing.
  The built-in wavetable synth is named **Antiserum**: the owner chose this
  dictionary word (a serum of antibodies) on 2026-10-18. Its artwork,
  wavetables and presets are our own, and it never uses another product's name,
  logo or styling. Wavetable import reads the public WAV `clm ` frame-size
  convention. The only other synth format read is Serum's presets, through the
  serum2vital port above; naming Serum there (the `antiserum.import_serum…`
  commands, the import messages) only says what a file is, and the importer
  ships none of Serum's wavetables, noises or presets: it reads the ones the
  user has.

## Look and feel

Familiar layout and default keyboard shortcuts help users switch, and shortcuts and
general workflow are not themselves something we copy from source. Even so:

- Draw every icon and widget ourselves.
- Keep shortcuts in a keymap data file so they are easy to change.
- Avoid pixel-for-pixel copies of another product's screens. Close imitation of a
  distinctive interface can raise trade dress concerns; when in doubt, differentiate.
- **Owner's decision (2026-10-21): layout plus general feel.** We may compare our
  screens with the screenshots in the public online manual and move closer to them
  in **layout and general feel**:
  - which panels and controls exist and where they sit, and their proportions;
  - the density of a dark, compact UI, control sizes and spacing.

  We keep **our own** colour palette, icons, fonts, knob, button and meter
  drawings, and wording. We never sample colours from, trace or overlay their
  screenshots, and we never match them pixel for pixel. Each change of this kind
  names the manual page it was compared with in its spec's Sources.
- **Owner's decision (2026-10-21): follow the manual.** Where the style guide
  (`docs/spec/style/`) offers a choice between our current layout or behaviour
  and the manual's, the manual's wins. Colours, icons, fonts, drawings and
  wording stay our own as above.

## Process

1. Before implementing a behavior, write or update a spec in `docs/spec/`.
2. Each spec ends with a **Sources** section: public manual section, open standard,
   or "own design".
3. Code follows the spec. If the code needs behavior the spec does not describe,
   update the spec first.
4. Reviewers (human or agent) reject work whose provenance is unclear.

## When in doubt

Stop and ask. Dropping a feature costs less than contaminating the project.

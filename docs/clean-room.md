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
3. Open standards and public specifications: MIDI, WAV/RIFF, CLAP, and any
   standard whose license permits our use.
4. Our own design decisions.

## Forbidden

- Decompiling, disassembling, debugging, or tracing the binaries of any commercial DAW.
- Any leaked, stolen, or otherwise unauthorized source code, documentation, or assets.
- Reading or reverse-engineering proprietary project/preset formats (for example
  `.flp`). Import from such formats may only be added later if the format is publicly
  documented by its owner.
- Copying icons, skins, bitmaps, fonts, sample packs, factory presets, logos, or names.
- Using another product's trademarks in our name, branding, or marketing.

## Look and feel

Familiar layout and default keyboard shortcuts help users switch, and shortcuts and
general workflow are not themselves something we copy from source. Even so:

- Draw every icon and widget ourselves.
- Keep shortcuts in a keymap data file so they are easy to change.
- Avoid pixel-for-pixel copies of another product's screens. Close imitation of a
  distinctive interface can raise trade dress concerns; when in doubt, differentiate.

## Process

1. Before implementing a behavior, write or update a spec in `docs/spec/`.
2. Each spec ends with a **Sources** section: public manual section, open standard,
   or "own design".
3. Code follows the spec. If the code needs behavior the spec does not describe,
   update the spec first.
4. Reviewers (human or agent) reject work whose provenance is unclear.

## When in doubt

Stop and ask. Dropping a feature costs less than contaminating the project.

# Spec: time signatures

## Model

- A **time signature** is written `n/d`: `n` beats per bar (1–32), each a
  `1/d` note, with `d` one of 2, 4, 8 or 16. A beat is `384 / d` ticks (96 per
  quarter note), so a bar is always a whole number of steps (sixteenths).
  Examples: 4/4 is 384 ticks (16 steps), 3/4 is 288 (12), 7/8 is 336 (14),
  6/8 is 288 (12, six eighth-note beats).
- The **project signature** (default 4/4) is the song's signature from its
  start and the signature new patterns get. It is saved as
  `"time_signature": "n/d"`, only when it is not 4/4.
- Every pattern has its own **pattern signature** (piano roll, channel rack,
  pattern-mode playback), saved as `"time_signature"` on the pattern when it is
  not 4/4. Patterns whose signature equals the project's follow it when the
  project signature changes.
- **Signature changes** in the song are playlist markers whose action is
  `time_signature` (saved as `"action": {"time_signature": "7/8"}`, named after
  the signature). A change starts a new bar at its position with the new
  signature; a bar cut short by a change ends where the change is.
- The **meter** of a timeline is its signature plus its changes. Every bar and
  beat position is computed from it (`musiccraft-meter`): bars are numbered
  from the start, each bar is one bar of the signature in force where it
  starts, ending early at a change.
  - The **song meter**: the project signature plus the signature markers.
  - A **pattern meter**: the pattern's signature, no changes.

## Commands

- `project.set_time_signature <n/d>` (undoable) sets the project signature,
  and the signature of every pattern that had the old one.
- `pattern.set_time_signature <pattern> <n/d>` (undoable) sets one pattern's
  signature.
- In both, a pattern whose signature changes and whose length is a whole
  number of bars keeps that number of bars (a one-bar 4/4 pattern becomes one
  bar of 7/8, 14 steps). Other lengths are kept as they are. Undo restores
  every signature and length exactly (`project.restore_meter <json>`).
- `playlist.set_time_signature <bar> <n/d|none>` (undoable) puts a signature
  change at a song position (not the start: that is the project signature), or
  with `none` removes the one there. A new change is a marker named after the
  signature. A marker already at that position (one per position) becomes the
  change and keeps its name, unless the name was its old signature, which
  follows the new one.
- New patterns are one bar of the project signature long.
- Positions and lengths given in bars to any command (playlist clips, loop
  region, markers, insert/delete space, automation and consolidate ranges) are
  bars of the song meter: `2.5` is halfway through the third bar, whatever its
  length. A length in bars counts from the position it starts at. `<n>t` is
  always ticks.
- Lengths in bars inside a pattern (chord lengths) are bars of that pattern's
  signature. Pattern lengths extend by whole bars of the pattern signature to
  fit notes; the song length rounds up to a whole bar of the song meter.

## Playback

- The engine plays with the meter of what is playing: the selected pattern's in
  pattern mode, the song meter in song mode.
- The metronome clicks on every beat of the meter (eighths in 6/8), accented on
  each bar's first beat, including the first beat after a change.
- The count-in clicks one bar of the signature in force where playback starts.
- Hosted plugins (CLAP, VST3) receive the signature in force and the start of
  the current bar with every block.

## Display

- Toolbar: the song position reads `bar.beat.step` in the meter that is
  playing; a signature button shows the project signature and edits it.
- Piano roll and channel rack: the grid is drawn in the pattern meter (beats
  shaded, bars ruled and numbered); the channel rack shades alternate beats.
  Pattern list → right-click → Time signature… edits the pattern signature.
- Playlist: the ruler, the grid and bar numbers follow the song meter;
  signature markers are drawn on the ruler with their signature. Right-click the
  ruler → Time signature here… adds one at the bar under the pointer.
- Automation editor: grid and numbers are bars of the project signature (a
  curve's ticks count from its clip's start, wherever it is placed).
- Snap to Beat and Bar snap to beats and bars of the meter.

## Files and exchange

- MIDI export writes a time-signature meta event (FF 58) at tick 0 and at every
  change (song) or the pattern's signature (pattern export).
- MIDI import into a pattern (`note.import_midi`) gives the pattern the
  file's first time signature when it differs from the one it plays in.
- FL Studio import (`project.import_flp`) takes the project signature, the
  arrangement's signature markers and each pattern's signature markers (the
  pattern takes its first).

## Sources

- Project, pattern and time-marker signatures follow the FL Studio online
  manual: "Project general settings" (time signature), "Patterns" (pattern
  time signature) and "Playlist" (time markers, "Add time signature change").
  Behaviour is described in our own words.
- The FF 58 meta event follows the Standard MIDI File 1.0 specification.
- The FL Studio import of signatures follows PyFLP's time-signature and
  time-marker events (GPL-3.0).
- Denominator range, the bar rule at a change, keeping whole-bar pattern
  lengths, the meter type and the command names are our own design.

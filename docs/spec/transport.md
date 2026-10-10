# Spec: transport, tempo and play mode

## Behavior

- A project has a **tempo** in beats per minute (BPM), a **transport** state that
  is either `stopped` or `playing`, and a **play mode** that is either `pattern`
  or `song`.
- New projects start `stopped`, in `pattern` mode, at 130 BPM.
- Supported tempo range is 20 to 999 BPM inclusive. Values outside the range, and
  non-finite values, are rejected and leave the project unchanged.
- `transport.play` starts playback. If already playing it does nothing and reports so.
- `transport.stop` halts playback and returns the playhead to the start of the
  loop. If already stopped it does nothing and reports so.
- `transport.toggle` plays when stopped and stops when playing. The default keymap
  binds it to the space bar.
- `transport.set_mode pattern|song` chooses what plays: `pattern` loops the
  selected pattern; `song` plays the playlist arrangement and loops at its end.
  `transport.toggle_mode` switches between them (default key: `L`).
- `transport.toggle_metronome` (default key Ctrl+M) turns a click on every beat
  on or off: a short decaying sine, higher on each bar's first beat. The setting
  is saved with the project but the click is never part of an export.
- `project.set_tempo <bpm>` sets the tempo. It is undoable; setting the same value
  records no history entry.
- Transport and mode commands are not part of undo history. The transport state
  is not saved in project files; the play mode is.

## Engine mapping

- The engine tracks position in ticks (96 per beat). While playing, each processed
  block advances position by `frames * bpm / 60 * 96 / sample_rate`. Tempo changes
  apply from the next block; position never jumps.
- While stopped the engine does not advance; held voices finish their release.
- `Stop` releases every voice and rewinds to tick 0. `Pause` (engine only, not yet
  a command) releases voices and keeps the position.
- The loop length is the selected pattern's length in pattern mode, and the song
  length rounded up to whole bars in song mode (one bar when empty). Reaching the
  end wraps to tick 0 within the same audio block, sample-accurately.

## Sources

- Own design. The range and defaults are our own choices.
- Space-bar play/stop, stop returning to the start, and a pattern/song mode switch
  are common conventions described in the public manuals of several pattern-based
  DAWs (FL Studio online manual, "Transport panel" and "Keyboard shortcuts").

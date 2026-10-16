# Spec: transport, tempo and play mode

## Behavior

- A project has a **tempo** in beats per minute (BPM), a **transport** state that
  is either `stopped` or `playing`, and a **play mode** that is either `pattern`
  or `song`.
- New projects start `stopped`, in `pattern` mode, at 130 BPM.
- Supported tempo range is 10 to 999 BPM inclusive. Values outside the range, and
  non-finite values, are rejected and leave the project unchanged.
- `transport.play` starts playback. If already playing it does nothing and reports so.
- `transport.stop` halts playback and returns the playhead to the start of the
  loop. If already stopped it does nothing and reports so.
- `transport.toggle` plays when stopped and stops when playing. The default keymap
  binds it to the space bar.
- `transport.pause` halts playback and keeps the playhead (state `paused`);
  `transport.play_pause` (default key Ctrl+Space) pauses when playing and plays
  from the playhead otherwise. From `paused`, Space (`transport.toggle`) also
  resumes, and `transport.stop` rewinds.
- `transport.jump_to_start` (default key Home) moves the playhead to where
  Stop returns (tick 0, or the loop region's start) without changing the
  transport state: playback continues from there, and a paused transport
  stays paused.
- `transport.panic` (default key Ctrl+H) is "all notes off": every sounding
  note is released at once, including one-shot samples, live keys, plugin
  instruments (all notes off) and MIDI Out channels (a note off for every
  note they started), and the browser preview stops. Playback is not stopped;
  later notes play as usual. Effect tails (reverb, delay) ring out.
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
- Jump-to-start and panic are requests, not state: each command increases a
  counter on the project that is never saved and does not affect project
  equality. The bridge sends `Seek` (to the sequence's Stop position) or
  `AllNotesOff` when a counter rises after its first sync; a counter that
  drops (a new or loaded project) or a fresh bridge only sets the baseline.
- `Stop` releases every voice and rewinds to the sequence's home: tick 0, or the
  loop region's start. `Pause` releases voices and keeps the position; the engine
  also pauses by itself on a pause marker (see `playlist.md`), counting each one so
  the app can put its transport in `paused` to match (`transport.pause`); the
  playhead is already on the marker.
- The loop length is the selected pattern's length in pattern mode, and the song
  length rounded up to whole bars in song mode (one bar when empty). Reaching the
  end wraps to tick 0 (or a song-loop marker) within the same audio block,
  sample-accurately.

## Sources

- Own design. The 10 BPM floor matches the FL Studio online manual's tempo
  range; the 999 BPM ceiling and the defaults are our own choices.
- Space-bar play/stop, stop returning to the start, and a pattern/song mode switch
  are common conventions described in the public manuals of several pattern-based
  DAWs (FL Studio online manual, "Transport panel" and "Keyboard shortcuts").
- Home to jump to the start and Ctrl+H for panic (all notes off) follow the FL
  Studio online manual, "Keyboard shortcuts", global section. What panic
  releases and the request counters are our own design.

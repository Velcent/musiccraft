# Spec: playlist and song mode (M5)

## Behavior

- The playlist has 64 track rows, numbered from 1. A **clip** places a pattern on
  a row at a start time with a length (both in ticks). If a clip is longer than its
  pattern, the pattern repeats; notes are cut at the clip's end.
- Clips on one row may not overlap.
- Rows can be muted; muted rows do not play.
- **Track rows** have a name, a colour, mute and solo (all saved, all undoable):
  `playlist.rename_track <track> [name]` (no name restores "Track n"),
  `playlist.set_track_color <track> <1-16|none>`, `playlist.set_track_solo
  <track> <on|off>` and `playlist.toggle_track_solo <track>`. While any row is
  soloed, only soloed rows play; a muted row stays silent even when soloed.
  Silenced rows skip pattern, audio and automation clips alike. The row header
  shows the colour, name, a mute dot and an S (solo) button; right-click it to
  rename it or pick a colour. A coloured row shows a stripe of its colour at the
  left of its header; silenced rows show their name dimmed.
- Playlist time arguments are in bars from 0 (4/4); fractions are allowed and
  `<n>t` gives exact ticks.
- Commands: `playlist.add_clip <pattern> <track> <start> [length]` (default length:
  the pattern's length), `playlist.remove_clip <track> <at>` and
  `playlist.move_clip`/`playlist.resize_clip` (the clip covering a position),
  `playlist.set_track_mute`, `playlist.toggle_track_mute`, `playlist.clear`,
  `playlist.set_clips` (used by undo). All are undoable.
- In song mode the song length is the end of the last clip rounded up to whole
  bars. Removing a pattern removes its clips (undo restores them).

## Loop region and markers

- `transport.set_loop <start bar> <end bar>` loops that range in song mode:
  playback wraps from the end to the start, and Stop returns to the start.
  `transport.clear_loop` removes it. Exports always render the whole song.
  UI: Shift+drag on the ruler; right-click the ruler → Clear loop.
- `playlist.add_marker <bar> <name>` / `playlist.remove_marker <bar>` manage named
  markers (one per position; re-adding at a position renames the marker and keeps
  its action). UI: right-click the ruler → Add marker here, or Alt+T for a marker
  at the playhead (on the beat); clicking a marker's label jumps the playhead to it.
- Both are saved and undoable.

### Marker actions: making a song stop instead of looping

- Song playback loops by default: at the end of the song it starts again from the
  beginning. Markers change that. `playlist.set_marker_action <bar>
  <none|pause|song_loop>` (undoable) sets what a marker does; UI: right-click a
  marker → No action / Pause / Song loop, or right-click the ruler → Add pause
  marker here (a marker named "Pause", one undo step).
  - `none` (default): only a label, drawn as a flag.
  - `pause`: when song playback reaches the marker it pauses there (transport
    `paused`); the playhead stays on the marker, Play continues from it (it
    does not pause again on the marker it starts from) and Stop rewinds. Put
    one at the end of the song to stop instead of looping. Drawn as two red
    bars.
  - `song_loop`: when playback reaches the end of the song it returns to this
    marker instead of the beginning; Stop still returns to the beginning. Only one
    marker is the song loop: choosing it moves it there (one undo step). A
    song-loop marker at or past the last clip extends the song to it (so exports
    include the silence up to it) and playback then wraps to the beginning. Drawn as
    a down-facing arrow.
- Marker actions apply in song mode while no loop region is set; a loop region
  takes over until it is cleared. Pattern mode always loops the pattern.
- Exports always render the whole song once and ignore pause markers.
- Project files store `"action"` only for markers that have one.
- `playlist.set_markers <json>` replaces every marker; undo uses it to restore
  markers exactly.

## Clip kinds

- **Pattern clips** (`playlist.add_clip`) play a pattern, repeating it.
- **Audio clips** (`playlist.add_audio_clip <channel> <track> <start> <length>`)
  play a sampler channel's sample once from the clip start, unpitched (at the
  channel's root key), cut at the clip end. Dropping a sound from the browser on
  the playlist adds a sampler channel with one-shot off and an audio clip as long
  as the sample. Removing the channel removes its audio clips (undo restores).
- **Automation clips** (`playlist.add_automation_clip <automation> <track>
  <start> [length]`) move one parameter during song playback (see below).
- `playlist.remove_clip`, `move_clip`, `resize_clip` and selection editing work
  the same for every kind. Ctrl+B duplicates any kind.

## Audio recording

- With File → Record audio input on, REC armed and the transport playing in song
  mode, the default input device is captured. When playback stops or REC is
  disarmed, the take is written as a 32-bit float WAV to `recordings/` beside the
  project (the per-user data folder for unsaved projects) and placed as an audio
  clip at the position where recording started, on the first empty track, as one
  undo step. Takes under 0.1 s are discarded. Input arrives through a lock-free
  ring buffer holding four seconds; no latency compensation is applied yet.

## Automation

- An automation has a name, a target and points (tick from clip start, value
  0..1 as a control position). Targets: `channel:<n>:volume|pan|<param>`,
  `mixer:<t>:volume|pan`, `mixer:<t>:<slot>:<param>`. Values map through the
  same curves as the controls (fader curve for volume, log for frequencies,
  cubic for times).
- Commands: `automation.create <target> [name]` (two points at the current value,
  spanning four bars), `automation.remove`, `automation.rename`,
  `automation.add_point`, `automation.remove_point`, `automation.move_point`,
  `automation.set_points`. All undoable.
- During song playback each automation clip writes its interpolated value once
  per audio block (at most 256 frames) while the playhead is inside the clip;
  afterwards the parameter keeps its last value until playback stops, when the
  project's values are restored. Automation does not play in pattern mode.
  Exports include automation.
- A mixer track or effect slot driven by an automation cannot be removed until
  the automation is deleted; moving or removing other effect slots keeps
  automation pointing at the right effect.
- UI: right-click any knob or fader → "Create automation clip" places a new clip
  on the first empty track. The playlist sidebar lists automations; clicking one
  paints it instead of the pattern; double-clicking a clip or list entry opens
  the automation editor (click adds a point, drag moves it, right-click deletes).

## Editor (UI)

- **Tools** as in the piano roll: Draw, Paint (drag to lay the selected
  pattern end to end, skipping places taken by other clips; one undo step),
  Delete (click or drag over clips; one undo step) and Select (box select).

- Left-click on an empty cell places the selected pattern at the snapped bar;
  dragging a clip moves it; dragging its right edge resizes it; right-click
  deletes. The pattern list next to the playlist selects the pattern to paint.
- The playlist is the main workspace. Click or drag on the ruler to move the
  playhead; during song playback the view follows the playhead.
- Selection works as in the piano roll: click, Shift+click, Ctrl+drag rectangle;
  dragging moves or resizes every selected clip; Delete removes; Ctrl+A selects
  all; Ctrl+B duplicates the selection right after its end. One undo step each.
- Ctrl+scroll zooms horizontally.

## Sources

- Pattern clips arranged on a playlist and a song/pattern mode switch follow the
  workflow described in the FL Studio online manual, "Playlist" section. Row
  count, overlap rule and command names are our own.
- Marker actions follow the time-marker actions described in the same manual
  section ("Pause": playback pauses at the marker; "Loop": playback returns to the
  marker at the end of the song, and one placed past the last data extends the
  render; the loop marker drawn with a down-facing arrow), and its "Add time
  marker" shortcut Alt+T ("Keyboard shortcuts"). Glyphs, colours, the one-loop-
  point rule, resuming past a pause and command names are our own.
- Playlist track names, colours, mute and solo follow the FL Studio online
  manual, "Playlist" (track header options). The colour palette and command
  names are our own.

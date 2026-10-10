# Spec: playlist and song mode (M5)

## Behavior

- The playlist has 64 track rows, numbered from 1. A **clip** places a pattern on
  a row at a start time with a length (both in ticks). If a clip is longer than its
  pattern, the pattern repeats; notes are cut at the clip's end.
- Clips on one row may not overlap.
- Rows can be muted; muted rows do not play.
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
  markers (one per position). UI: right-click the ruler → Add marker here;
  clicking a marker's label jumps the playhead to it.
- Both are saved and undoable.

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

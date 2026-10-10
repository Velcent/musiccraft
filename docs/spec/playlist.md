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

## Editor (UI)

- Left-click on an empty cell places the selected pattern at the snapped bar;
  dragging a clip moves it; dragging its right edge resizes it; right-click
  deletes. The pattern list next to the playlist selects the pattern to paint.

## Sources

- Pattern clips arranged on a playlist and a song/pattern mode switch follow the
  workflow described in the FL Studio online manual, "Playlist" section. Row
  count, overlap rule and command names are our own.

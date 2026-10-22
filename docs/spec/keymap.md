# Spec: default keyboard shortcuts

## Behavior

- Shortcuts are data: `crates/musiccraft-ui/keymap/default.keymap` maps key chords
  to invocations. The UI only looks chords up; it has no hard-coded shortcuts.
- A user keymap file can replace the default (`musiccraft --keymap <file>`).
- Shortcuts are ignored while a text field has keyboard focus.
- `view.toggle <window>` and `ui.*` invocations are handled by the UI (they open
  windows and file dialogs and do not change the project); everything else runs
  through the command registry.

## Typing keyboard

- While enabled (Ctrl+T or the ⌨ button), the letter rows play notes on the
  selected channel instead of triggering shortcuts: `Z S X D C V G B H N J M ,`
  play C4 to C5 with black keys on the home row; `Q 2 W 3 E R 5 T 6 Y 7 U I 9 O P`
  continue from C5. Shift is allowed; other modifiers pass through to shortcuts.
  R is a note while it is on, so record with the REC button instead.

## Defaults

| Chord | Action |
| --- | --- |
| Space | Play / stop |
| Ctrl+Space | Play / pause |
| Home | Jump to the start (playback continues) |
| Ctrl+H | Panic: all notes off |
| L | Toggle pattern / song mode |
| Ctrl+M | Metronome on / off |
| R | Arm / disarm recording (`record.toggle`) |
| Ctrl+E | Step edit on / off |
| Ctrl+P | Recording count-in on / off |
| Ctrl+I | Wait for input before recording on / off |
| Ctrl+T | Typing keyboard plays notes |
| F5 / F6 / F7 / F9 | Playlist / channel rack / piano roll / mixer: a closed window opens and comes to the front, an open one behind others comes to the front, the focused one closes |
| Alt+F8 | Browser |
| Enter | Maximize the playlist, or restore it (opens it if closed) |
| Ctrl+Shift+H | Put the windows back in the default layout |
| Alt+T | Add a playlist marker at the playhead |
| F10 | Settings |
| F4 | Next empty pattern (adds one if none), then name it |
| Plus / Minus | Next / previous pattern |
| 1–9 | Select pattern 1–9 (over the channel rack: mute channels instead) |
| P B D E C T Y Z | Tools: draw, paint, delete, select, slice, mute, play, zoom (piano roll and playlist) |
| S / O | New piano-roll notes are slide / portamento notes (toggle) |
| Ctrl+Insert / Ctrl+Delete | Insert / delete the loop region's time in the playlist |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+N | Save new version (`song_2.mcproj`, …) |
| Ctrl+O | Open project |
| Ctrl+S / Ctrl+Shift+S | Save / save as |
| Ctrl+R | Export WAV |
| Ctrl+Shift+M | Export MIDI |

### Over the channel rack

Window-local keys, active while the rack is the focused window (the topmost
one, which a click anywhere in it makes; and the typing keyboard is off); they take precedence over the global keys above.

| Chord | Action |
| --- | --- |
| 1–9, 0 | Mute / unmute channel 1–10 (`channel.toggle_mute`) |
| Ctrl+1–9, Ctrl+0 | Solo channel 1–10 (`channel.solo`) |
| Alt+C / Alt+Delete | Clone / delete the selected channel |
| Alt+Up / Alt+Down | Move the selected channel up / down |
| Ctrl+L | Route the selected channel to a free mixer track (`channel.route_free`) |
| Shift+Ctrl+Left / Right | Shift the selected channel's steps |

## Sources

- Space, L, Ctrl+M, F4, F5, F6, F7, F9, Alt+F8, Alt+T, numeric-keypad plus/minus,
  Ctrl+S, Ctrl+Shift+S, Ctrl+O, Ctrl+N (save new version), Ctrl+R and R
  (record) follow the default shortcuts listed in
  the publicly published FL Studio online manual ("Keyboard shortcuts"). Shortcuts
  are a convenience for users switching tools; the bindings are data and can be
  changed.
- Ctrl+T (typing keyboard): own choice. New project has no default shortcut;
  it is in the File menu.
- Ctrl+E (step edit), Ctrl+P (count-in) and Ctrl+I (wait for input) follow
  the FL Studio online manual, "Keyboard shortcuts" and "Recording".
- The single-key tools (P, B, D, E, C, T, Y, Z, plus S and O for new note
  kinds), 1–9 pattern select, the channel rack keys and Ctrl+Insert /
  Ctrl+Delete follow the FL Studio online manual, "Keyboard shortcuts"
  (global, channel rack, piano roll and playlist sections). Applying one tool
  choice to both editors is our own simplification.
- Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo: common desktop convention (own
  choice; the manual describes a different undo toggle).
- Enter (maximize the playlist) and Ctrl+Shift+H (default window layout)
  follow the FL Studio online manual, "Keyboard shortcuts", "Window
  Navigation". What the default layout restores is our own design.

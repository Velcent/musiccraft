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
| L | Toggle pattern / song mode |
| Ctrl+M | Metronome on / off |
| R | Arm / disarm recording |
| Ctrl+T | Typing keyboard plays notes |
| F5 / F6 / F7 / F9 | Playlist / channel rack / piano roll / mixer |
| Alt+F8 | Browser |
| Alt+T | Add a playlist marker at the playhead |
| F10 | Settings |
| F4 | New pattern |
| Plus / Minus | Next / previous pattern |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+N | Save new version (`song_2.mcproj`, …) |
| Ctrl+O | Open project |
| Ctrl+S / Ctrl+Shift+S | Save / save as |
| Ctrl+R | Export WAV |

## Sources

- Space, L, Ctrl+M, F4, F5, F6, F7, F9, Alt+F8, Alt+T, numeric-keypad plus/minus,
  Ctrl+S, Ctrl+Shift+S, Ctrl+O, Ctrl+N (save new version), Ctrl+R and R
  (record) follow the default shortcuts listed in
  the publicly published FL Studio online manual ("Keyboard shortcuts"). Shortcuts
  are a convenience for users switching tools; the bindings are data and can be
  changed.
- Ctrl+T (typing keyboard): own choice. New project has no default shortcut;
  it is in the File menu. Ctrl+E is left free for step edit mode, as in the
  manual.
- Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo: common desktop convention (own
  choice; the manual describes a different undo toggle).

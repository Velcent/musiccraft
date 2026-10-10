# Spec: default keyboard shortcuts

## Behavior

- Shortcuts are data: `crates/musiccraft-ui/keymap/default.keymap` maps key chords
  to invocations. The UI only looks chords up; it has no hard-coded shortcuts.
- A user keymap file can replace the default (`musiccraft --keymap <file>`).
- Shortcuts are ignored while a text field has keyboard focus.
- `view.toggle <window>` and `ui.*` invocations are handled by the UI (they open
  windows and file dialogs and do not change the project); everything else runs
  through the command registry.

## Defaults

| Chord | Action |
| --- | --- |
| Space | Play / stop |
| L | Toggle pattern / song mode |
| F5 / F6 / F7 / F9 | Playlist / channel rack / piano roll / mixer |
| Alt+F8 | Browser |
| F4 | New pattern |
| Plus / Minus | Next / previous pattern |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+N / Ctrl+O | New / open project |
| Ctrl+S / Ctrl+Shift+S | Save / save as |
| Ctrl+R | Export WAV |

## Sources

- Space, L, F4, F5, F6, F7, F9, Alt+F8, numeric-keypad plus/minus, Ctrl+S,
  Ctrl+Shift+S, Ctrl+O, Ctrl+N and Ctrl+R follow the default shortcuts listed in
  the publicly published FL Studio online manual ("Keyboard shortcuts"). Shortcuts
  are a convenience for users switching tools; the bindings are data and can be
  changed.
- Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo: common desktop convention (own
  choice; the manual describes a different undo toggle).

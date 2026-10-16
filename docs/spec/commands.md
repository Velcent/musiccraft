# Spec: commands, undo and scripting

## Invocations

- Every change is an invocation: a command name (`area.action`) and string
  arguments. Text form separates arguments with whitespace; arguments with spaces,
  quotes or backslashes are written in double quotes with `\"` and `\\` escapes.
  Outside quotes, a token starting with `#` starts a comment.
- Numbering: channels, patterns, playlist tracks and effect slots from 1; mixer
  track 0 is the master. Steps are 0-based. Note times are in steps and playlist
  times in bars, with fractions and `<n>t` ticks allowed.

## Undo

- Undoable commands return an inverse invocation. `edit.undo` runs the inverse,
  `edit.redo` re-runs the original. A new undoable command clears the redo stack.
- Commands that remove something return an inverse that re-inserts it from JSON
  (`channel.insert`, `pattern.insert`, `mixer.insert_track`,
  `mixer.insert_effect`, `note.set_all`, `playlist.set_clips`).
- Redo restores the project's id allocator to where it was when the command first
  ran, so recreated channels and patterns get the same ids that later history
  entries refer to.
- `edit.batch "<command>" "<command>" …` runs commands all-or-nothing as one undo
  step; on failure the commands already run are rolled back.
- Front ends may *coalesce* a continuous gesture (for example dragging a fader):
  consecutive runs of the same command with the same gesture key become a single
  undo step whose inverse restores the value from before the gesture.
- `project.new`, `project.load`, `project.import_flp` and `project.recover`
  clear history.
- Plugin state commands (`channel.set_plugin_state`, `mixer.set_plugin_state`)
  are not undoable: they mirror state the running plugin already holds. A
  plugin parameter with no stored value is restored with
  `… set_plugin_param … unset`.
- A failed command leaves the project and history unchanged.

## Scripts

- `musiccraft-cli script <file>` runs one command per line, stops at the first
  error, and prints a final state line. See `examples/basic-beat.mcs`.

## Sources

- Own design.

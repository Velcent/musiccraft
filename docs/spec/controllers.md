# Spec: controllers

Controllers move one parameter continuously from a live source instead of a
drawn curve: an LFO, an envelope or the key played on a channel, the level of
a mixer insert, a formula, or a knob of their own (a macro). They cover the
jobs of the envelope, keyboard, X/Y, peak and formula controllers in
pattern-based DAWs, with our own design.

## Behaviour

- A project has a list of controllers, numbered from 1. Each has a name, a
  kind, a **target** (the same addresses automation uses:
  `channel:<n>:volume|pan|<param>`, `mixer:<t>:volume|pan`,
  `mixer:<t>:<slot>:<param>`), an output range `min`..`max` in control
  positions (0–1 of the target's range; `min` above `max` inverts) and an
  `enabled` switch.
- Every engine block (at most 256 frames) each enabled controller computes a
  value from 0 to 1, maps it to `min + (max − min) × value` and writes the
  target, after automation, so a controller wins over an automation clip on
  the same target.
- Kinds and their settings:
  - `lfo`: `shape` (sine, triangle, saw, square, random steps), `period`
    (beats per cycle, 1/16 – 64) and `phase` (0–1). It follows the song
    position while playing and a free-running clock at the project tempo
    while stopped, like the tempo-synced effects.
  - `envelope`: `channel` (rack number) and `attack`, `decay`, `release`
    seconds and `sustain` (0–1). The envelope starts when the channel goes
    from no held notes to a held note and releases when its last note ends.
  - `keyboard`: `channel`, `low` and `high` keys: the highest key the
    channel holds maps `low`→0 and `high`→1 (clamped); with nothing held it
    keeps its last value.
  - `peak`: `track` (mixer insert), `gain` (0–8), `attack` and `release` ms:
    follows the insert's peak level from the previous block, times the gain,
    clamped to 1.
  - `formula`: `expr`, an expression of `t` (song position in beats), `bar`
    (`t / 4`) and `pi`, with `+ - * / %`, parentheses, unary minus, numbers
    and the functions `sin cos abs floor frac sqrt min max pow saw tri sqr`
    (`saw`, `tri` and `sqr` take a phase in cycles and return −1..1). The
    result is clamped to 0–1. Expressions are checked when they are set; at
    most 64 tokens.
  - `macro`: `value` (0–1), a knob of its own. Two macros make an X/Y pad.
- Inserting or removing mixer tracks renumbers targets and peak sources;
  removing a track a controller targets or listens to is refused, like
  automation. Removing an effect slot a controller targets is refused too.
  Channels are referred to by identity, so moving channels keeps targets;
  a controller whose channel was removed does nothing.
- Commands (all undoable):
  - `controller.add <kind> <target> [name]` appends a controller with
    default settings (range 0–1).
  - `controller.remove <n>`, `controller.insert <n> <json>` and
    `controller.restore <n> <json>` (the undo forms: insert one, or replace
    one with its earlier settings).
  - `controller.set <n> <setting> <value>`: `name`, `enabled` (on/off),
    `min`, `max`, `target`, and the kind's settings above. Numbers are
    range-checked; `channel` and `track` must exist.
- Right-clicking a knob or fader offers "Add controller" next to "Create
  automation clip", with one entry per kind (envelope and keyboard listen to
  the first channel, peak to insert 1 until changed).
- The browser's project section lists controllers: an on/off box, the name,
  a delete button, the kind's settings as dragged numbers (one undo step per
  drag), a text field for formulas (applied when it loses focus) and the
  output range.

## Sources

- The FL Studio online manual, "Plugins" section, lists envelope, keyboard,
  X/Y, peak and formula controllers that drive parameters from a source;
  only that purpose is used. The design (one target per controller, a
  0–1 output range, the formula language and its functions, the block-rate
  update) is our own.

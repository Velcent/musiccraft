# Spec: window layout and density

How the main editor windows are laid out and how densely they are packed. The
aim is a dark, compact interface where the workspace, not window chrome,
takes the room, with the panels and controls where users of pattern-based
DAWs expect them (see the owner's decision of 2026-10-21 in
`docs/clean-room.md`). Colours, icons, fonts and the drawing of every control
stay our own. The full target look, window by window, is the style guide in
`docs/spec/style/README.md`; its gap lists say what changes here next.

## Window header

- The channel rack, piano roll and mixer have no separate title bar. Their
  first row is a compact **header**: a collapse arrow, the window's title in
  small text at the left, the window's own tools on the same row, and a close
  cross at the right end.
- Drag the header, or any empty part of the window, to move it.
- The arrow collapses the window to its header row and expands it again; the
  window keeps its size for when it is expanded.
- Other windows (channel settings, effect windows, dialogs) keep a standard
  title bar.

## Toolbar

- When the window is at least 1300 points wide, the toolbar is one band: the
  menus over the hint bar in a column at the left, the transport and
  displays beside them, and the window buttons at the right end.
- In a narrower window it falls back to two bands: menus, hint bar and window
  buttons, then the transport below.

## Density

- Tool and transport buttons are 24 × 22 points.
- The browser opens 170 points wide (it can be dragged wider).
- Playlist tracks are 36 points tall by default.
- The piano roll's event lane is 96 points tall.

## Channel rack

- Header, left to right: arrow, title, display filter, the pattern's name,
  swing, steps, Graph, close.
- Step buttons are taller than they are wide (14 × 24 points). Alternate beats
  of the pattern's signature are shaded clearly differently, so steps read in
  groups.
- Channel name buttons are 96 points wide; longer names are cut short with an
  ellipsis. The rack opens just wide enough for a 16-step pattern.
- The selection switch at the end of a row stands apart from the steps: an
  outline when the channel is not selected, filled when it is.
- The button that adds a channel is a "+" centred under the rows.

## Mixer

- Header: arrow, title (`Mixer — <track>`), "+ Insert", the hint, close.
- Strips are 40 points wide, so twice as many inserts fit as before. Each
  strip shows, top to bottom: number, name written vertically, the ON
  (enable) switch, the pan knob, the peak meter beside the fader, the stereo
  separation knob, the level readout (in dB, without the unit), S (solo), the
  effect count and the route arrow.

## Sources

- Compared with the FL Studio online manual's screenshots and descriptions of
  the main window ("Interface overview" in Basics:
  `basics_interface.htm`), the Channel Rack (`channelrack.htm`), Playlist
  (`playlist.htm`), Piano roll (`pianoroll.htm`), Mixer (`mixer.htm`) and
  Toolbar panels (`toolbar_panels.htm`), under the owner's 2026-10-21 "layout
  plus general feel" decision: the window's tools sharing its title row, the
  browser's narrow column, tall step buttons grouped by beat, and narrow
  mixer strips with vertical names and the pan knob above the fader. No
  colours were sampled and nothing was traced; sizes are our own choices.
- Own design: exact sizes, the header's order, the collapse behaviour, and the
  drawing of the arrow and close cross.

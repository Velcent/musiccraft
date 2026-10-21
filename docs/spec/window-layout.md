# Spec: window layout and density

How the main editor windows are laid out and how densely they are packed. The
aim is a dark, compact interface where the workspace, not window chrome,
takes the room, with the panels and controls where users of pattern-based
DAWs expect them (see the owner's decision of 2026-10-21 in
`docs/clean-room.md`). Colours, icons, fonts and the drawing of every control
stay our own. The full target look, window by window, is the style guide in
`docs/spec/style/README.md`; its gap lists say what changes here next.

## Window header

- Every editor window (channel rack, piano roll, mixer) and every secondary
  window (channel settings, effect windows, automation editor, spectrum
  analyzer, keyboard shortcuts) has no separate title bar. Its first row is
  one compact **header**, left to right:
  1. the **window menu** button (three short bars), flush with the top-left
     corner;
  2. the window's own tools;
  3. the **title**: the window's name, an em dash, then its context
     ("Piano roll — Lead"). The mixer puts its title at the right end
     instead, just before the window buttons;
  4. empty space, which is the handle for moving the window;
  5. the **window buttons**, set close together at the right end: collapse,
     maximize (or restore) and close. Fixed-size windows leave out maximize.
- The header never wraps onto a second line. When the window is too narrow
  for its tools, the whole tool group folds into a "more" button (two small
  chevrons) just before the window buttons; clicking it shows the tools in a
  popup. The title is cut short with an ellipsis before anything else goes.
- The window menu lists the window's own items first, then a separator and
  the common items: Maximize (or Restore), Collapse to header (or Expand) and
  Reset position, which puts the window back where it first opened.
- Drag the header, or any empty part of the window, to move it. Double-click
  empty header space to maximize or restore.
- Collapse rolls the window up to its header row and expands it again; the
  window keeps its size for when it is expanded.
- Maximize fills the workspace and covers the other windows; the window keeps
  its header, cannot be moved or resized, and keeps filling the workspace if
  the main window is resized. Restore returns it to its previous rectangle.
- The window on top is the focused one: its header is a shade lighter, its
  title is in the normal text tone (dimmed in other windows) and its outer
  edge is drawn in the dim accent.
- Windows have barely rounded corners and cast no shadow. Menus, popups and
  tooltips cast one faint shadow.
- Modal dialogs (settings, prompts, unsaved changes) keep a header with only
  a title and a close cross.

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

- Header, left to right: window menu, display filter, the pattern's name,
  swing, steps, Graph, the title, then the window buttons.
- Step buttons are taller than they are wide (14 × 24 points). Alternate beats
  of the pattern's signature are shaded clearly differently, so steps read in
  groups.
- Channel name buttons are 96 points wide; longer names are cut short with an
  ellipsis. The rack opens just wide enough for a 16-step pattern.
- The selection switch at the end of a row stands apart from the steps: an
  outline when the channel is not selected, filled when it is.
- The button that adds a channel is a "+" centred under the rows.

## Mixer

- Header: window menu, "+ Insert", then the title (`Mixer — <track>`) at
  the right end before the window buttons. How to use the strips is a hint
  (hint bar), not text in the header.
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
- Own design: exact sizes, the collapse behaviour, the "more" fold, the
  focused-window emphasis, and the drawing of every header glyph.
- Interface style guide (`docs/spec/style/main-window.md`, "The header row,
  left to right"): the header's slot order, maximize and restore, and the
  common window-menu items, following the owner's 2026-10-21 decision to
  follow the manual's layout.

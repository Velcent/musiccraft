# Spec: window layout and density

How the main editor windows are laid out and how densely they are packed. The
aim is a dark, compact interface where the workspace, not window chrome,
takes the room, with the panels and controls where users of pattern-based
DAWs expect them (see the owner's decision of 2026-10-21 in
`docs/clean-room.md`). Colours, icons, fonts and the drawing of every control
stay our own. The full target look, window by window, is the style guide in
`docs/spec/style/README.md`; its gap lists say what changes here next.

## Starting layout

- All four editors (channel rack, mixer, playlist, piano roll) and the browser
  are open when the app starts: the rack top-left, the mixer to its right, the
  playlist across the lower part and the piano roll floating over the right
  half (each section below). That is the first start; afterwards the app
  starts with the arrangement it had when it last closed.
- **Remembered layout.** On closing, the app keeps in the user settings
  (`window_layout`) which windows are open (the editors, the browser, the
  channel window, the spectrum analyser and the shortcut list), where each
  floating window was and its size, and which were folded to their header.
  The next start opens the same windows in the same places, kept inside the
  workspace like any window. `--open` on the command line overrides it, and
  Ctrl+Shift+H still returns to the default layout.
- Ctrl+Shift+H (`view.default_layout`) puts them back: the four editors and
  the browser open, each window unfolded, not maximized and moved to its slot.
- Enter (`view.maximize playlist`) maximizes the playlist over the workspace
  or restores it, opening it first if it was closed. Neither key acts while a
  dialog is open or a text field has focus.

- Right-clicking the empty workspace opens a small menu: a tick item for each
  editor window (show or hide) and "Default layout (Ctrl+Shift+H)".

## Focus

- The focused window is the topmost one; clicking anywhere in a window brings
  it to the front. Its header lifts and its edge takes the accent.
- Window-local keys (piano roll, channel rack, playlist) go to the focused
  window, wherever the pointer is.
- The window keys (F5, F6, F7, F9), the toolbar's window buttons and the View
  menu open a closed window and bring it to the front, bring an open window
  that is behind others to the front, and close the focused window. The
  browser, a docked panel, just shows or hides.
- Esc (`view.close_focused`) closes the focused window. A dialog, an open
  menu, a drag in progress, or a piano-roll or playlist selection takes Esc
  first (Esc then clears the selection).
- F12 (`view.hide_all`) closes every workspace window; Alt+F12
  (`view.hide_effects`) closes only effect windows; Ctrl+F12
  (`view.hide_others`) closes all but the focused one. The browser stays.

## Dragging from the browser

- The drag ghost is a small raised chip beside the pointer: a `RAISED` fill,
  a hairline `ACCENT` outline and the item's name in `TEXT`, so it stays
  quieter than the toolbar's lit buttons.
- While a browser item is dragged, hovering over a toolbar window button
  opens and raises that window, so the drag can continue into it.

## Window header

- Every editor window (playlist, channel rack, piano roll, mixer) and every secondary
  window (channel settings, effect windows, automation editor, spectrum
  analyser, keyboard shortcuts) has no separate title bar. Its first row is
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

- The toolbar is one band across the top of the main window, in the panel
  tone, two button rows tall. At the left a recessed column holds the menu
  bar (File, Edit, Add, Patterns, View, Options, Tools, Help) over the hint
  field (the project name with a small highlight dot while there are unsaved
  changes, then the hint or last message; right-click copies the message).
  Beside it are two
  racks, each packed to the left:
  - **upper rack**: the option toggles (Metronome, Wait for input,
    Count-in, Blend recording, Loop recording, Computer keyboard as piano,
    Step edit), the time display, the output monitor (a scope beside two
    horizontal peak bars, each holding its peak as a thin line and showing a
    clip mark for 3 s after full scale), the performance panel (CPU and
    voices) and the utility shortcuts, none of which latch: Undo (right-click:
    Redo; greyed out with nothing to undo), Sample editor (opens the selected
    channel's window on its Sound page), Record audio (turns on audio-input
    capture and starts recording; lit while armed for audio), Help (the
    keyboard shortcut list, F1), Save (right-click: Save as…, Save new
    version) and Export audio (right-click: Export stems…, Export MIDI…);
  - **lower rack**: the master volume knob, the mode switch (Pattern over
    Song, one button wide; right-click a half to open the channel rack or the
    playlist), play (a pause glyph while playing), stop, record, the tempo
    display, TAP and the time signature, the pattern panel (an options
    chevron with the Patterns menu's items; the pattern field, whose click
    or right-click lists the patterns, wheel or stacked up/down arrows step
    them, double-click renames in place and Ctrl+click shows the pattern's
    first clip in the playlist; and a plus), the window buttons (Playlist,
    Channel rack, Piano roll, Browser, Mixer; right-click on Playlist or
    Piano roll opens that window maximized, `view.open_maximized`) and the
    window layout button, whose menu has Default layout, Maximize or restore
    the playlist, and Piano roll maximized.
- Option toggles all light the same way: an accent fill with a dark glyph.
- The time display shows bar.beat.step, minutes:seconds.hundredths or
  bar.beat.tick, with the last field smaller and the mode (BARS, TIME, TICKS)
  as its caption. A click switches bars and time; right-click lists the three
  modes. While counting in or waiting for the first note it reads COUNT-IN or
  WAITING instead. The tempo display shows whole BPM large and three
  decimals smaller. Each display has a tiny caption in its top-right corner.
- Tempo display: dragging up or down on the whole digits steps 1 BPM every
  6 points (0.1 BPM with Shift); on the decimals it moves 0.01 BPM a point
  (0.001 with Shift). Each scroll notch steps 1 BPM (0.01 with Shift).
  Double-click opens a type-in field (Enter applies, Esc cancels). A pair of
  small chevrons right of the display steps the tempo 1 BPM up or down per
  click, or 0.001 BPM when the decimals were the part last dragged. Right-click
  offers "Enter a tempo…", round tempos (60, 90, 120, 140, 174), "Copy",
  "Paste" (the tempo last copied there; disabled until one is) and "Reset to
  130". The whole gesture, and each chevron click, is one undo step.
- Record pulses while a take is running.
- When the window is narrower than 1000 points it falls back to two bands:
  menus, hint field and window buttons, then the two racks without the
  window buttons.

## Density

- Tool and transport buttons are 24 × 22 points.
- The browser opens 170 points wide (it can be dragged wider).
- Playlist tracks are 60 points tall by default (about three tool buttons, so
  the header has room for two lines); the vertical zoom ranges 18–160 points; a track's own height is a
  percentage of that (`playlist.md`, "Track heights").
- The piano roll's event lane is 96 points tall by default; its splitter
  resizes it from 40 to 320 points (at most 70% of the view).

## Channel rack

- Header, left to right: window menu, group filter, the pattern's name,
  swing, steps, Graph, the title, then the window buttons.
- Step buttons are taller than they are wide (14 × 24 points). Alternate beats
  of the pattern's signature are shaded clearly differently, so steps read in
  groups.
- Channel name buttons are 96 points wide; longer names are cut short with an
  ellipsis. The rack opens just wide enough for a 16-step pattern.
- The selection switch at the end of a row stands apart from the steps: an
  outline when the channel is not selected, filled when it is.
- The button that adds a channel is a "+" centred under the rows.
- When the rows overflow, the rack shows the simplified scroll bars (rail,
  thumb and chevrons, no zoom grips): a vertical bar at the right and a
  horizontal bar underneath, each only while needed. The mixer's strips and
  slot panel and the browser's list use the same simplified bars; the
  browser's vertical bar sits on its outer (left) edge.

## Playlist

- The playlist is an editor window, not the workspace background; the
  background (with its watermark) shows wherever no window covers it. The
  first time it opens it spans the whole width of the workspace in the lower
  part, a little more than half the workspace height, and the channel rack
  sits above it at the top left. F5 shows and hides it.
- Header, left to right: window menu, the snap button (snap glyph and the
  current value), the tools (Draw, Paint, Delete, Mute, Slice, Select, Zoom,
  Play from here), then the title "Playlist — Arrangement" and the window
  buttons.
- The window menu holds Edit (Duplicate, Delete, Insert time, Remove time),
  Clip (the clip edits), Select (All, None), Snap (the snap values), Zoom
  (Zoom in, Zoom out, Fit the song) and Markers (Add marker at the playhead),
  then the common items. There are no zoom buttons or text menus in the
  header; zoom is Ctrl+wheel, the Zoom tool and the window menu.
- Lanes are all one tone, parted by hairline track separators. Step lines
  (only when a step is wide enough), beat lines, bar lines and the first bar
  of every four-bar group get stronger in that order, and alternate four-bar
  groups get a very faint lighter wash.
- A clip is a barely rounded box with a title strip in the clip's colour at
  full strength, holding the kind glyph and the name, over a dark body tinted
  toward the clip's colour. The content (notes, waveform, curve) is the
  brightest thing in the body: waveforms are a light tint of the clip colour,
  and automation curves have a faint fill under them. Selected clips get a
  light strip edge and outline; muted clips are drawn faint.

## Mixer

- The mixer opens at the top of the workspace to the right of the channel
  rack (`mixer.md`, "Layout"). Header: window menu, strip details, slot
  panel, add insert (a plus), then the title (`Mixer — <track>`) at the right
  end before the window buttons. How to use the strips is a hint (hint bar),
  not text in the header.
- Strips are 40 points wide and fill the window's height. Each strip shows,
  top to bottom: the tinted number cell, the name written vertically beside
  the peak meter, the round enable light (S under it), the pan knob, the
  fader, the track switches (phase, swap, separation, effect count) and the
  route arrow. The slot panel is 270 points wide.

## Piano roll

- The piano roll opens over the right half of the workspace, half its width
  and half its height (`piano-roll.md`).
- Header, left to right: window menu, note tools, snap, scale, chord, the
  eight tools, the title and its channel chevron, then the window buttons.
- Under the header: a 14-point scroll/zoom row across the grid, with the
  corner block (as wide as the 52-point keyboard, spanning the row and the
  ruler) at its left and the vertical zoom box at its right; the vertical
  scroll bar is 14 points wide, down the right of the ruler and grid.

## Sources

- Starting layout and drag feedback: the style guide's main-window section
  (`docs/spec/style/main-window.md`, from the public manual's
  `basics_interface.htm` overview and `toolbar_panels.htm`, which says
  dragging content over the window buttons raises that window). Ghost tones
  are our own.
- The piano roll's header pickers, scroll/zoom row and corner block: the
  interface style guide, `docs/spec/style/piano-roll.md`. Glyphs and sizes
  are our own.
- The toolbar's two racks, the option toggles, the stacked mode switch, the
  display captions and the menu and hint column: the interface style guide,
  `docs/spec/style/toolbar.md` (owner's 2026-10-21 decision to follow the
  manual's layout). Glyphs and wording are our own.

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

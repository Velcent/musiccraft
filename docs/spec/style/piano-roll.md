# Style guide: Piano roll

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The piano roll is where melodies and chords are written. A vertical keyboard sits on the left, a wide note grid runs to the right with time going left to right, a ruler sits above the grid, and a lane of note properties sits below it. It should feel like a dense, dark drafting table. The chrome is one thin row of small square tools, and the notes are the brightest and most colourful thing on screen. The grid stays quiet but readable: bar lines are clearly stronger than beat lines, and beat lines are stronger than step lines. Every edit (draw, drag, stretch, delete) happens directly on the grid, with almost no dialogs.

## Placement and proportions

**Where it lives**
- It is an editor window inside the workspace, like the channel rack, mixer and playlist. It has no OS title bar. Its first row is the shared compact header defined in the main-window section ("The header row, left to right"). This section lists only the piano roll's own tools, title and right-hand items. Our current header is described in `docs/spec/window-layout.md`, "Window header".
- Ways to open it:
  - the toolbar's piano-roll window button, or F7 (existing);
  - a channel's right-click menu in the channel rack (existing);
  - **Proposed (needs `piano-roll.md` and `playlist.md` updated first):** double-clicking a pattern clip in the playlist opens that clip's pattern in the piano roll, showing the selected channel.
  - Opening it while it is already open brings it to the front.
- Default placement, as in the main-window section's default layout: floating over the right half of the workspace, overlapping the playlist. It opens about half the workspace's width and about half its height, so the playlist and channel rack stay visible around it. It remembers its last position and size.
- It moves by dragging the header's drag area or any empty chrome, and resizes from any edge or corner. Collapse, maximize/restore, detach and double-click on the drag area all follow the main-window section's shared rules.

**Inner layout, top to bottom (relative sizes only)**

```
+-----------------------------------------------------------------------------------+
| ☰ | tools menu · snap · scale · chord | tools | ▶ | title ▸ |  drag  | ghost | _ □ × |
+--------+---------------------------------------------------------------+----------+
| corner | ‹ [ horizontal scroll / zoom bar with note overview ]       › | zoom box |
| swatch +---------------------------------------------------------------+----------+
|  + porta/slide | ruler: bar numbers along the top,                     |    ^     |
|        |        marker tabs along the bottom edge                     |          |
+--------+---------------------------------------------------------------+    s     |
|keyboard|                                                               |    c     |
| column |                       note grid                               |    r     |
|        |                                                               |    o     |
|        |                                                               |    l     |
|        |                                                               |    l     |
|        |                                                               |    v     |
+--------+---------------------------- splitter grip -------------------+----------+
| lane   | property lane (stems), lane name faint at the top-left       | lane zoom|
| target |                                                               |          |
+--------+---------------------------------------------------------------+----------+
```

- **Header**: one row, as tall as one tool button (see window-layout.md for our button size). It never takes two rows: groups that do not fit fold into the shared "more" chevron (main-window section).
- **Scroll/zoom row**: a thin row, about two thirds of the header's height, across the width of the grid. Its parts are the shared scroll and zoom bar (main-window section, "Scroll and zoom bars").
- **Ruler**: one band, a little taller than the header. Bar numbers run along its top and marker tabs hang along its bottom edge.
- **Corner block**: as wide as the keyboard and as tall as the scroll row and the ruler together.
- **Keyboard column**: narrow, about a tenth of the window width at the default size or a little less. It is wide enough for a short key label plus a visible key body, and it does not grow when the window grows.
- **Right column** (vertical scroll bar and zoom boxes): as wide as the scroll row is tall.
- **Note grid**: all remaining space. At the default zoom about three bars fit across the grid and about two to three octaves fit top to bottom. One key row is a little shorter than a tool button.
- **Splitter**: a thin strip, about a third of the scroll row's height.
- **Property lane**: it keeps a fixed default height, our own value in window-layout.md ("The piano roll's event lane…"), which is roughly a fifth of the body at the default window size. **Proposed (needs `piano-roll.md` and `window-layout.md` updated first):** the splitter changes that height, and the lane can be hidden completely.
- **Density**: everything is compact. Chrome text is small. Labels inside notes are smaller still. Header buttons are close together, with a hairline separator and a slightly wider gap between groups.

## Anatomy

Where this section describes behaviour that `docs/spec/piano-roll.md` does not have yet (a new gesture, a mouse button doing something new, a new shortcut), it is marked **Proposed (needs `piano-roll.md` updated first)**, or `keymap.md` for keys. Until that spec changes, today's behaviour stays the default. Every new key proposed here is window-local: it acts only while the pointer is over the piano roll and no text field has focus, like the existing piano-roll keys in `piano-roll.md`. Each was checked against `crates/musiccraft-ui/keymap/default.keymap` and none is bound globally today. Layout, order and proportions are this guide's target; the specs they change are named under gaps.

### A. Header row (left to right, one row)

Slots follow the main-window section's shared header. Only the piano roll's own contents are listed.

| # | Element | What it shows | Behaviour | States |
|---|---|---|---|---|
| A1 | Window menu button (shared slot 1) | The shared menu glyph | Click opens the window menu (L4), with the common window items after a separator (main-window section). The existing ☰ options menu becomes this button. | As the main-window section |
| — | **Options group** (shared slot 2, first tool group) | — | — | — |
| A2 | Note-tools menu button | Square icon button (own glyph, see Icons) | Click opens the note-tools menu directly: the same list as the window menu's Tools submenu (L4, item 3). A shortcut to that submenu, not a second copy of its logic. | Menu open: pressed look |
| A3 | Snap button | Square icon button (own glyph) | Click opens the list of snap values (None, 1/4 step, 1/2 step, Step, Beat, Bar; existing). The current value is ticked in the list and named in the hint ("Snap: Step"). **Proposed (needs `piano-roll.md` updated first):** right-click sets the default length of new notes to the current snap value; mouse wheel over it steps through the values. | Hover lightens the plate. Snap set to None: glyph dimmed. |
| A4 | Scale button | Square icon button (own glyph) | Click opens the scale menu: "No scale", then the root note (C to B) and scale type side by side (existing choices). A tick item at the end of this menu (existing), "Fit edited notes to the scale": drawn and moved notes land on the nearest key of the highlighted scale; right-click on the button opens the root list directly. | No scale: glyph dimmed. Scale highlighted: glyph in the normal text tone. Fitting on: lit like an active tool. |
| A5 | Chord button | Square icon button (own glyph) | Click opens the chord list ("Note", then the chord names; existing). While a chord is chosen, a click on empty grid draws the whole chord with its root on the clicked row (existing). "Note" returns to single notes. | Chord active: glyph in the highlight tone, so it is clear that clicks draw chords. The hint names the chord. |
| — | Group gap | Hairline separator | — | — |
| A6 | Tool buttons (shared slot 2, second tool group), in this order: **Draw** (P), **Paint** (B), **Delete** (D), **Mute** (T), **Slice** (C), **Select** (E), **Zoom** (Z), **Play** (Y) | Square icon buttons, all the same size, touching or nearly touching | Click picks the tool. Exactly one tool is always active. Keys come from the keymap (existing bindings, unchanged). The same tool choice applies to the playlist (existing). The order is the target; today's order differs (gaps). **Proposed (needs `piano-roll.md` updated first):** holding the right button while rolling the wheel over the tool group steps through the tools. Optional later additions, also proposed (needs `piano-roll.md` and `keymap.md` updated first): **Paint steps** (N, window-local), between Paint and Delete, which lays one-step hits on a fixed grid and toggles mute when clicking an existing note; **Ramp** (I, window-local), between Mute and Slice, which smooths stepped values in the lane into straight ramps. | Active: accent plate, glyph in the dark tone. Idle: raised plate, glyph in the text tone. Hover: plate one step lighter. |
| A7 | Pattern play button (shared slot 3) | Play triangle; pause bars while playing | **Proposed (needs `piano-roll.md` updated first):** click starts or pauses playback of the pattern. Right-click while playing stops and returns to where playback started. Right-click while stopped moves the playhead to the start of the pattern. | Playing: the glyph shows pause and the button is lit |
| A8 | Title and channel picker (shared slot 4) | "Piano roll — ‹channel name›" (existing title) followed by the shared selector chevron | Click opens the list of all channels; each channel that already has notes in this pattern carries a tick. **Proposed (needs `piano-roll.md` updated first):** wheel over the title steps through the channels that have notes, and Ctrl+wheel steps through all channels; right-click opens the channel's instrument window. Until then the channel stays chosen as today (a header drop-down, see gaps). | Hover lightens the text. The chevron points down while the list is open. Unfocused window: dimmed (shared rule). |
| — | Drag area (shared slot 5) | Empty space; stretches | Shared rules | — |
| A9 | Ghost-notes button (shared slot 6, right-hand item) | Square icon button (own glyph) | **Proposed (needs `piano-roll.md` updated first):** click shows or hides the floating ghost-channel panel (L1). Right-click opens a small menu: show every channel as ghosts; show none (Alt+V, window-local, needs `keymap.md`); and a tick item for ghost notes from overlapping clips. | Lit while any channel other than the edited one is ghosted |
| A10 | Collapse, maximize/restore, close (shared slots 7–9) | Shared window buttons | Shared rules | Shared rules |

Notes on the header:
- The options group is four square icon buttons, not text pickers. The current snap value, scale and chord show in each button's hint and as the ticked item in its menu. Own design: when the window is wide enough, a small value text (for example "Step", "C major", "maj7") may follow each of these three buttons; it is the first thing dropped as the window narrows.
- Slide, Porta and the note colour are not in the header. They live on the corner block (B1). The new-note velocity moves into the window menu (L4).
- Too narrow: the shared folding rule applies. The options group folds first, then the tool group. The menu button, title and window buttons never fold.

### B. Scroll/zoom row and corner block

The row is the shared scroll and zoom bar (main-window section, "Scroll and zoom bars"). The piano roll uses all of it: **Proposed (needs `piano-roll.md` updated first)**; today the view uses the toolkit's default scroll bars.

- **B1 Corner block** (above the keyboard; it spans the scroll row and the ruler):
  - It is filled with the current **colour group** of new notes: the channel colour for group 1, otherwise the group's colour. The group number is written small in the lower-left corner.
  - Click opens a 4 × 4 grid of the 16 colour groups. Tiles are numbered 1 to 16, running down the first column and then down the next. The current group has a tick. Clicking a tile picks it and closes the grid. Own design: hovering a tile writes "Colour group N (MIDI channel N for plugins)" in the hint bar.
  - Along its right edge, stacked top to bottom: the **Porta** toggle (O) and the **Slide** toggle (S) (existing toggles and keys), drawn as small glyph-only switches on the swatch. Click toggles them. When on, new notes are portamento or slide notes. Own design, **Proposed (needs `piano-roll.md` updated first):** click-and-hold while adding a note cycles that note through normal → slide → porta.
  - Right-click on the block offers "Apply this colour to the selected notes" (Alt+C, window-local).
- **B2 Left chevron**: scrolls left by one bar; holding it repeats.
- **B3 Horizontal scroll and zoom bar**:
  - The thumb's length is the visible share of the pattern. Drag the body to scroll. Drag either end grip to zoom horizontally, keeping the opposite end fixed. Click the bare rail to page one screen.
  - The rail shows a **miniature overview of the pattern's notes**: one short dash per note at its relative time and pitch, in the channel colour, dimmed. The user sees the whole pattern and can jump to busy parts.
- **B4 Right chevron**: mirror of B2.
- **B5 Vertical zoom box**: the shared zoom box at the right end of the row, above the vertical scroll bar. Dragging up or down on it changes the key-row height (vertical zoom). Double-click resets it.

### C. Ruler (one band, over the grid only)

- **C1 Bar numbers**:
  - Bar numbers (counted from 1) sit along the top of the band, just right of each bar line, small and left-aligned. When bars get too narrow, only every 2nd or 4th number is shown.
  - Short tick marks for beats rise from the bottom edge of the band.
  - Numbers inside the pattern length are in normal text; numbers past the end are dimmed.
- **C2 Markers** (**Proposed (needs `piano-roll.md` updated first)**):
  - Markers are small tabs hanging along the bottom edge of the band, anchored at their time, with a short label to the right. A faint line runs from each tab down the whole grid. Their glyphs match the playlist ruler: a flag for a plain label, a bracket for a time-signature change with the signature as its text (for example "3/4"), and a key marker (new glyph, see Icons) showing a key name (for example "B minor").
  - At the very start of the band, the pattern's own time signature is written a little larger than the other labels.
  - Drag a marker to move it. Right-click a marker for its menu: rename, delete, set time signature…, set key…, and its action (none, pattern length). A marker whose action is "pattern length" sets where the pattern ends.
  - Right-click on empty ruler: add a marker here, add a time signature here, add a key marker here.
- **C3 Playhead marker**: a small downward-pointing triangle at the playhead position. It is shown whenever the playhead is inside this pattern (pattern mode, or song mode inside a clip of this pattern; existing).
- **C4 Ruler gestures** (**Proposed (needs `piano-roll.md` updated first)**; today a click on the ruler does nothing):
  - Click seeks the playhead to the snapped position.
  - Drag along the ruler, or double-click and then drag, makes a **time selection**. The ruler shows it as a band in the highlight role, the grid shows a faint wash over the selected range with two edge lines, and the notes inside are selected.
  - Esc or a click elsewhere on the ruler clears it.
  - Hover writes "Click to seek · drag to select time · right-click for markers" to the hint bar.
- **C5 Pattern end**: everything after the pattern's length is shaded darker in the ruler and the grid, so the loop length is visible at a glance (existing in the grid).

### D. Keyboard column (left)

- Drawn as a real piano turned on its side, low keys at the bottom (existing):
  - White keys run the full column width.
  - Black keys are shorter, reaching a little over half way across from the left edge. They sit on top of the white keys and have slightly rounded outer corners.
  - A thin seam separates neighbouring white keys at E–F and B–C.
- **Labels**: by default only the C keys are labelled (for example "C5"; existing), small and right-aligned near the right edge, in the dark tone on the white key. When rows are tall enough, a keyboard-labels choice (window menu, View, and the key menu) labels all white keys, all keys, scale keys only, or none.
- **Hovered key**: shaded one step darker (white key) or lighter (black key). Its full name appears in the hint bar ("F#4 · click to hear").
- **Pressed key** (click preview, or a note being dragged): filled with the accent role while held.
- **Keys sounding during playback** (own design, optional): lit with the dimmed accent role while the channel's notes sound, so the keyboard shows what plays.
- **Behaviour**:
  - Click and hold plays the key on the current channel (existing). Dragging up and down glides the preview from key to key.
  - Wheel scrolls vertically (existing, through the scroll area).
  - Ctrl+click selects every note of that key and Ctrl+drag selects a range of keys; middle-drag up or down zooms vertically around the clicked key.
  - **Proposed (needs `piano-roll.md` updated first):** right-click opens the key menu (L3).

### E. Note grid

- **E1 Rows**:
  - Rows for black keys are one step darker than rows for white keys, so the keyboard pattern carries across the grid (existing).
  - A line one step stronger than the step lines runs under each C row to mark the octaves.
- **E2 Vertical lines**:
  - Step lines are faint, beat lines one step stronger, and bar lines strongest.
  - Lines follow the pattern's time signature and its signature markers.
  - When zoomed far out, step lines disappear first, then beat lines.
- **E3 Scale highlighting** (when a scale is chosen in A4):
  - Rows of keys in the scale are drawn one step lighter than normal white-key rows, and rows outside the scale are drawn like black-key rows.
  - The root rows get a further, slightly accented tint.
  - The black/white-key shading is replaced, not added on top, so the scale reads cleanly.
  - The keyboard itself is unchanged.
- **E4 Past-end shading**: see C5.
- **E5 Bar-group shading** (own design, optional View item): alternate groups of bars (four by default) are shaded very slightly differently, to help with long patterns.
- **Empty-grid gestures**:
  - Existing: left-click with Draw adds a note at the snapped position with the last-used length; dragging right straight after adding sets its length; Ctrl+drag (any tool) or a drag with Select draws a selection rectangle, Shift adding to the selection; Select's click on empty space clears the selection; wheel scrolls vertically, and Ctrl+wheel zooms horizontally around the pointer.
  - Middle-drag pans in both directions (main-window section).
  - **Proposed (needs `piano-roll.md` updated first):** Shift+drag draws a note by its length; right-click on empty grid clears the selection; Shift+wheel scrolls horizontally; Alt+wheel over a note changes the lane's property for that note (see K).
  - **Zoom keys (existing, window-local):** Page Up and Page Down zoom in and out horizontally around the pointer. Shift+1, Shift+2 and Shift+3 jump to three fixed zoom levels of our own choosing (12, 24 and 48 points per step), Shift+4 fits the whole pattern, and Shift+5 zooms to the time selection. Plain 1–9 keep selecting patterns.

### F. Notes

- **F1 Shape**:
  - A note is a horizontal bar that fills its row, minus a hairline at the top and bottom.
  - Its corners are slightly rounded, and a thin darker outline separates touching notes.
  - It starts exactly at its start time and ends at its end time. Neighbouring notes on the same key show a visible seam.
- **F2 Label**:
  - The note's name ("C5", "F#4") is written inside the bar, left-aligned with a small inset, in the dark text role, smaller than the chrome text.
  - If the bar is too short, the label is cut with an ellipsis ("F…", then just "…"). Below that width there is no label.
  - **Proposed (needs `piano-roll.md` updated first):** when key rows are very tall (maximum vertical zoom), the label moves to the top-left corner and the note's current lane property is drawn inside the note as a thin horizontal level bar. Alt+wheel over the note adjusts it there.
- **F3 Velocity on the note** (own design): the note body is drawn at a strength that follows velocity, so quiet notes blend toward the row colour and loud notes are full colour. The label and outline do not fade. The exact values are shown in the property lane.
- **F4 Colour**:
  - Notes are filled with the channel colour, or with their colour group's colour when the group is not 1 (existing).
  - Every note of one group has one colour, so overlapping groups (and their independent slides) are easy to tell apart.
- **F5 Selected**:
  - The whole body of a selected note changes to a single **selection tone**, with its label in the dark text role. A selection reads at a glance on any channel colour.
  - Unselected notes keep their colour.
  - A thin light outline is not enough on its own (today's look; `piano-roll.md` says "Selected notes are outlined", see gaps).
- **F6 Muted**:
  - The note is hollow: a faint body and an outline in the note colour at reduced strength, with the label kept but dimmed.
  - It still moves, stretches and selects like any note.
- **F7 Slide note**: a small solid ramp mark (our right-angled triangle) at the **right end** of the bar, inside it, in the light text role at partial strength. The name label stays at the left and the note keeps its normal body.
- **F8 Portamento note**: a small rising-curve mark (our glyph) at the right end of the bar, inside it, in the same tone as the slide mark. A note that is both shows the curve to the left of the ramp.
- **F9 Ghost notes**:
  - Notes of other channels in the same pattern (existing): solid, unlabelled blocks in one neutral, dimmed tone. They are not drawn in their channel colours, so they never compete with the edited notes. They sit under the real notes.
  - **Proposed (needs `piano-roll.md` updated first):** double-right-click a ghost note to switch the window to that note's channel.
  - **Proposed (needs `piano-roll.md` updated first):** notes from other pattern clips that overlap this clip in the playlist (optional, View item) are shown as hollow, unlabelled outlines in the same neutral tone.
- **F10 While dragging**:
  - The notes being moved, created, painted or stretched are drawn at their new place at reduced strength; their old place is left empty.
  - The preview sounds the note whenever the pointer crosses to a new key (existing).
  - Notes under a right-drag or a Delete stroke disappear immediately and are removed as one undo step on release (existing).
- **Note gestures** (Draw tool unless noted):
  - Hover over the body shows the move cursor; hover over the last few points of the right edge shows the horizontal resize cursor.
  - Existing: click selects the note; Shift+click toggles it in the selection; drag moves it (and the whole selection if it is selected); dragging the right edge resizes it (and all selected notes by the same amount); right-click deletes it; right-drag deletes every note the pointer crosses.
  - **Proposed (needs `piano-roll.md` updated first):** a click also makes the note's length the default for new notes; own extension: its velocity too.
  - Double-click opens the note-properties window (L5); Alt+wheel changes the current lane property of the note under the pointer (or of all selected notes), and Ctrl+Alt+wheel does it in fine steps (existing).

### G. Playhead

- A thin vertical line in the playhead role runs from the ruler down through the grid while playing (existing). The guide extends it through the property lane. It is hidden when stopped; the ruler marker (C3) stays.
- Own design, **Proposed (needs `piano-roll.md` updated first)**, View option "Follow playhead": the view pages along when the playhead leaves it.

### H. Selection and handles

- **H1 Selection rectangle** (existing gesture): a rectangle with a faint accent-tone fill and a thin accent outline. Notes it touches become selected when the drag ends. Shift adds to the selection.
- **H2 Stretch handle** (**Proposed (needs `piano-roll.md` updated first)**): when two or more notes are selected, a small handle appears just past the right end of the selection, vertically centred on the selected notes. Dragging it right or left stretches or squeezes the whole selection in time, scaling both start positions and lengths around the selection's start. Alt ignores snap. The cursor over it is the horizontal resize cursor.
- **H3 Time selection**: see C4. Edit commands (delete, duplicate, transpose) act on it as well as on the selected notes.

### I. Right column

- **Vertical scroll bar** (shared bar, main-window section): an up chevron at the top, a rail with a thumb whose length is the visible share of the 128 keys, and a down chevron at the bottom. Drag the thumb to scroll, click the rail to page, click the chevrons to step one key (hold to repeat).
- When the view first opens for a channel it is centred on that channel's notes, or on C5 when the channel has none (existing).

### J. Splitter

- A thin strip across the full width between the grid and the property lane, with a small grip in its centre.
- Drag it to move the boundary. Right-click restores the default lane height. Double-click hides or shows the lane (own design). Shift+Enter, window-local, also does this (needs `keymap.md`).
- The cursor over it is the vertical resize cursor.

### K. Property lane (bottom)

- **K1 Target picker** (in the column under the keyboard, at the top): the current target name ("Velocity") followed by a small down chevron (existing drop-down).
  - Click opens the list of targets: note properties (velocity, pan, release, fine pitch, and the two free modulation values, proposed to be called Mod A and Mod B; see Labels) and, later, channel controls (volume, pan, pitch) recorded as automation, as a second group.
  - **Proposed (needs `piano-roll.md` and `keymap.md` updated first):** wheel over it, or Shift+F (window-local), steps through the targets; right-clicking anywhere in that column also opens the list.
- **K2 Lane name**: the target's name is also written in faint, larger text at the top-left of the lane area, like a watermark. It stays readable but is never brighter than the stems.
- **K3 Stems** (note properties): one per note of the current channel, in note colour (existing, as thin stems).
  - A vertical bar rises from the baseline at the note's start time up to its value, with a small round head on top.
  - From the head, a thin tail runs right for the note's length, so stacked chord notes that share a start can be told apart.
  - Stems of selected notes take the selection tone.
  - Stems of ghost channels are drawn faintly behind, in the neutral ghost tone, when ghosts are shown.
- **K4 Centred targets** (pan, fine pitch, Mod A, Mod B, release): a centre line marks the neutral value, and stems grow up or down from it. Dragging close to the centre snaps to it (existing).
- **K5 Grid**: bar, beat and step lines continue through the lane at a step quieter than in the grid, so stems line up visually with their notes.
- **K6 Lane zoom box**: the shared zoom box, in the right column at the top of the lane. Drag it vertically to scale the lane's value view (own design), and double-click to reset.
- **Gestures**:
  - Existing: click or drag vertically on a stem sets that note's value, and the latest-starting note wins when notes overlap; one drag is one undo step.
  - **Proposed (needs `piano-roll.md` updated first):** Shift+drag across the lane draws a straight line, setting every note crossed; Alt+click resets a note to the target's default value.
  - Own design: a small readout in the LCD style follows the pointer while dragging (for example "Velocity 96").

### L. Floating panels, menus and windows

- **L1 Ghost-channel panel** (**Proposed (needs `piano-roll.md` updated first)**): a small floating panel pinned to the upper-right corner of the grid while it is shown. One row per channel: a round on/off dot in the channel's colour, then the channel name. The current channel's row is in bold, and its dot cannot be switched off. Click a row to toggle ghosting for that channel.
- **L2 Colour-group grid**: see B1.
- **L3 Key menu** (right-click on the keyboard; **Proposed (needs `piano-roll.md` updated first)**):
  - The first item is "Name this key…". It opens a small inline field to give the key a custom label, for example drum names. The custom label then replaces the note name on the key and on notes of that key.
  - Next comes a group of radio items for the keyboard's drawing style (piano, flat, or a plain list of names) and a submenu for which keys are labelled.
- **L4 Window menu** (A1), submenus in this order, in our own wording. Items marked *existing* are in today's ☰ menu or Tools submenu; the rest are proposed and each needs its command and `piano-roll.md` first.
  1. **File**: Import MIDI file…, Export as MIDI file….
  2. **Edit**: Cut, Copy, Paste, Duplicate, Delete (existing as keys); Move left / right one snap step; Transpose up / down a semitone; Transpose an octave up / down (*existing*); Mute / Unmute; Apply current colour; Clear the channel's notes (*existing*, in the danger role).
  3. **Tools** (also behind A2): Quantize (*existing*); Strum up / down (*existing*); Humanize (*existing*); Legato (*existing*); Split at grid lines, Join touching notes, Mirror in time / Mirror in pitch (*existing commands*, under proposed new labels, see Labels); Snap to scale (*existing*); Toggle slide (*existing*); then the submenus Generate ▸, Shape ▸, Arpeggiate ▸ (*existing*).
  4. **Chord**: the same list as A5.
  5. **View**: Ghost notes; Ghost notes from overlapping clips; Keyboard labels ▸; Scale highlighting ▸; Note labels; Bar-group shading; Follow playhead; Show property lane.
  6. **Snap**: the same list as A3.
  7. **Select**: All (Ctrl+A, existing key), None (Esc, existing key), Invert, Muted notes, Notes of the current colour.
  8. **Zoom**: Zoom in, Zoom out (*existing* as − / + buttons), Fit the pattern, Zoom to selection.
  9. **Markers**: Add marker at playhead, Add time signature…, Add key marker…, Delete markers in the time selection.
  10. **Channel ▸** (same list as A8) and **Lane ▸** (same list as K1).
  11. The new-note velocity field (existing control, moved here from the header).
  12. After a separator: the common window items (main-window section).

  Disabled items are dimmed and say in the hint bar why they are disabled (for example "Highlight a scale first", existing).
- **L5 Note-properties window** (double-click a note; existing):
  - A small dialog titled with the note name ("Note properties — C5").
  - Top section: a row of our own knobs in our own order, Velocity, Release, Pan, Tune, Mod A, Mod B; then the Slide and Porta toggles and the colour-group swatch.
  - Lower section: start and length readouts in bar.beat.step. These are hidden when several notes are selected.
  - Bottom row: Reset at the left and OK at the right.
  - It uses our own knobs and readouts (controls section).
- **L6 Tool menu at the pointer** (optional, own design, **Proposed (needs `piano-roll.md` updated first)**): pressing the middle and right buttons together anywhere in the window opens a menu with the tools as radio items on the left and the window-menu submenus on the right.

### M. Pointer and wheel summary

"P" marks a **Proposed (needs `piano-roll.md` updated first)** gesture; everything else exists today.

| Where | Left | Right | Middle | Wheel |
|---|---|---|---|---|
| Empty grid | Tool action (Draw: add; Paint: drag lays notes end to end; Select: rectangle; Zoom: click in, Shift+click out; P: Zoom drag a box to zoom to it) | Drag deletes crossed notes; P: click clears the selection | P: drag pans | Scroll; Ctrl: zoom; P: Shift scrolls horizontally |
| Note body | Select / move / preview; P: double-click opens properties | Delete; P: double-right-click on a ghost switches channel | P: pan | P: Alt changes the lane property |
| Note right edge | Resize | Delete | P: pan | — |
| Keyboard | Preview, glide | P: key menu | P: drag zooms vertically | Scroll |
| Ruler | P: seek; drag for a time selection | P: marker menus | — | P: Ctrl zooms |
| Lane | Set value; P: Shift draws a line, Alt resets | P: target list (left column) | P: pan | P: Alt changes the value |
| Splitter (P) | Drag | Reset height | — | — |
| Tool group | Pick tool | — | — | P: with right held, step tools |

### N. Cursor shapes

| Situation | Cursor |
|---|---|
| Empty grid, Draw/Paint | Crosshair |
| Empty grid with Ctrl, or Select tool | Cell / precision cursor |
| Over a note body | Move (four-way) cursor; grabbing while dragging |
| Over a note's right edge, the stretch handle, or between two touching notes | Horizontal resize |
| Delete / Mute / Slice / Play / Zoom tools | Pointer (the active tool button shows the mode) |
| Splitter | Vertical resize |
| Ruler, keyboard | Pointing hand |
| Middle-drag panning | Grabbing hand |

## Icons

All glyphs are simple vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: one colour, line weight matching the existing icons, drawn inside the shared square tool-button frame. None is drawn to resemble another program's artwork.

| Where | What it does | Our glyph |
|---|---|---|
| A1 window menu | Opens the window menu | Shared: three short horizontal bars stacked evenly (main-window section) |
| A2 note-tools menu | Opens the note-tools list | A small wand: a slanted straight line with a four-point sparkle at its upper end |
| A3 snap | Snap value list | A grid corner: two short lines meeting at a right angle, with a filled dot at the joint |
| A4 scale | Scale menu and fit-to-scale | A small staircase: three steps rising left to right, drawn as one stroked line |
| A5 chord | Chord list | Three small filled dots stacked vertically, each a little right of the one below (stacked note heads) |
| A6 Draw | Draw tool | Pencil: a slanted outlined bar ending in a filled point (existing `Icon::Pencil`) |
| A6 Paint | Paint tool | Brush: a thin handle line ending in a filled bristle wedge (existing `Icon::Brush`) |
| A6 Paint steps (optional) | Lay one-step hits | Three small filled squares in a row, the middle one slightly lower |
| A6 Delete | Delete tool | Eraser: a tilted block, half filled, resting on a baseline (existing `Icon::Eraser`) |
| A6 Mute | Mute tool | Speaker with a cross to its right (existing `Icon::Mute`) |
| A6 Ramp (optional) | Smooth lane steps into ramps | Two dots joined by a straight diagonal line |
| A6 Slice | Slice tool | An outlined bar with a blade line crossing it at a slant (existing `Icon::Slice`) |
| A6 Select | Select tool | Dashed rectangle (existing `Icon::Select`) |
| A6 Zoom | Zoom tool | Magnifier: circle with a short heavy handle (existing `Icon::Zoom`) |
| A6 Play tool | Click to hear notes | New: a play triangle with one short sound-wave arc to its right, so it differs from the transport play (today it reuses `Icon::Play`) |
| A7 pattern play | Play / pause | Filled play triangle; two vertical bars while playing |
| A8 channel picker | Opens the channel list | Shared selector chevron after the title; points down while open |
| A9 ghost notes | Ghost panel and menu | Two horizontal rounded bars offset diagonally, the rear one a dashed outline and the front one solid |
| A10 window buttons | Collapse, maximize/restore, close | Shared (main-window section) |
| B1 Porta toggle | New notes glide | A quarter-curve rising from lower left to upper right (the same mark as on porta notes) |
| B1 Slide toggle | New notes slide | A solid right-angled ramp rising to the right (the same mark as on slide notes) |
| B2/B4, I | Scroll arrows | Single chevrons (left, right, up, down) |
| B5, K6 | Zoom boxes | The shared zoom-box glyph (main-window section, "Vertical zoom box") |
| C2 markers | Marker kinds | Same as the playlist ruler: flag (label), bracket (time signature); plus a new key-marker glyph: a small sharp sign (#) |
| C3 | Playhead marker | Downward-pointing filled triangle |
| H2 | Stretch handle | Two outward chevrons ‹ › joined by a short horizontal line, on a small raised plate |
| J | Splitter grip | Two short parallel horizontal lines, centred |
| K1 | Lane target picker | Small down chevron after the name |
| L1 rows | Ghost channel on/off | Filled circle (on) / outlined circle (off) |

## Colour roles

| Element | Token(s) | Relative tone and state changes |
|---|---|---|
| Window body | `theme::PANEL` | Same as other editor windows |
| Header row fill | Proposed `theme::HEADER` / `theme::HEADER_ACTIVE` (main-window section) | Focused window uses `HEADER_ACTIVE` |
| Title text | `theme::TEXT` (focused), `theme::TEXT_DIM` (unfocused) | Hover over the title: text unchanged, chevron goes from `theme::TEXT_DIM` to `theme::TEXT` |
| Window menu, collapse, maximize, close glyphs | As the main-window section: `theme::TEXT_DIM` | Hover `theme::TEXT` on `theme::RAISED_HOVER`; close hover `theme::DANGER` |
| Tool and option buttons, idle | `theme::RAISED` plate, glyph `theme::TEXT` | Hover: plate `theme::RAISED_HOVER`. Pressed: plate `theme::ACCENT_DIM` |
| Tool button, active | Plate `theme::ACCENT`, glyph `theme::BACKGROUND` | Stays lit; hover does not change it |
| Snap button set to None | Glyph `theme::TEXT_DIM` | — |
| Scale button | No scale: glyph `theme::TEXT_DIM`; scale highlighted: as idle above | Fitting on: lit like an active tool |
| Chord button | Idle as above | Chord active: glyph `theme::HIGHLIGHT` |
| Optional value text after A3–A5 | `theme::TEXT_DIM` | Chord name in `theme::HIGHLIGHT` while a chord is active |
| Pattern play button | Idle as above | Playing: plate `theme::ACCENT`, pause glyph `theme::BACKGROUND` (as the toolbar's play) |
| Ghost button | Idle as above | Any channel ghosted: glyph `theme::ACCENT` |
| Corner swatch | Current group colour (`theme::palette(channel)` or `note_color(group)`) | Group number and toggle glyphs in `theme::BACKGROUND`; toggle off: glyph at low strength; on: full strength |
| Scroll row rail, right column rail | `theme::PANEL_DARK` | — |
| Scroll thumbs, chevron buttons | `theme::RAISED`, chevrons `theme::TEXT_DIM` | Hover `theme::RAISED_HOVER`; dragging `theme::ACCENT_DIM` |
| Zoom boxes | As the main-window section: `theme::LCD_BG` fill, `theme::LCD_EDGE` border, lines `theme::TEXT_DIM` | Dragging: lines `theme::ACCENT` |
| Note overview in the scroll rail | Channel colour, dimmed strongly | Notes inside the visible range a step brighter than outside |
| Ruler band | `theme::PANEL_DARK`; bar numbers `theme::TEXT` | Past pattern end: numbers `theme::TEXT_DIM` |
| Marker tabs | Label `theme::AUTOMATION`, time signature `theme::HIGHLIGHT`, key marker `theme::ACCENT`, pattern-length `theme::ACCENT_DIM` | Drop lines: the same token at low strength; hovered tab one step brighter |
| Playhead marker and line | `theme::PLAYHEAD` | Recording: `theme::DANGER` |
| Time selection | Ruler: `theme::HIGHLIGHT` at low strength; grid wash: `theme::HIGHLIGHT` at very low strength; edges `theme::HIGHLIGHT` | — |
| White keys | **New `theme::KEY_WHITE`**: one step lighter than `theme::TEXT`, the lightest surface in the app | Hover: one step darker; pressed: `theme::ACCENT`; sounding (optional): `theme::ACCENT_DIM` |
| Black keys | **New `theme::KEY_BLACK`**: between `theme::PANEL_DARK` and `theme::BACKGROUND` | Hover: one step lighter; pressed: `theme::ACCENT` |
| Key seams (E–F, B–C) | **New `theme::KEY_SEAM`**: `theme::TEXT_DIM` lightened one step | — |
| Key labels | `theme::BACKGROUND` on white keys, `theme::TEXT_DIM` on black keys | Custom-named keys: same tones |
| Grid rows | `theme::WHITE_KEY_ROW`, `theme::BLACK_KEY_ROW` | Octave line under C rows: `theme::GRID_BEAT` |
| Grid vertical lines | `theme::GRID_LINE` (step), `theme::GRID_BEAT` (beat), `theme::GRID_BAR` (bar) | Zoomed out: step lines dropped first |
| Scale rows | **New `theme::SCALE_ROW`**: `theme::WHITE_KEY_ROW` one step lighter; out-of-scale rows `theme::BLACK_KEY_ROW` | Root rows: **new `theme::SCALE_ROOT_ROW`**: `theme::SCALE_ROW` tinted toward `theme::ACCENT_DIM` |
| Past pattern end | **New `theme::PAST_END`**: a wash that darkens whatever is under it by one to two steps | — |
| Bar-group shading (optional) | **New `theme::BAR_GROUP`**: a very faint lightening wash | — |
| Note body | Channel colour `theme::palette(index)` or `note_color(group)` | Strength follows velocity; while dragged: reduced strength |
| Note outline | **New `theme::NOTE_OUTLINE`**: `theme::BACKGROUND` at partial strength | — |
| Note label | `theme::BACKGROUND` | Muted: label at reduced strength |
| Selected note | **New `theme::NOTE_SELECTED`**: `theme::TEXT`, very slightly tinted toward `theme::ACCENT` (a near-white body distinct from every palette colour) | Label stays `theme::BACKGROUND` |
| Muted note | Note colour: body at very low strength, outline at partial strength | Selected and muted: outline in `theme::NOTE_SELECTED` |
| Slide / porta marks | `theme::TEXT` at partial strength | On a selected note: `theme::BACKGROUND` at partial strength |
| Ghost notes | **New `theme::GHOST_NOTE`**: `theme::TEXT_DIM` at low strength (filled for same-pattern ghosts, outline only for overlapping-clip ghosts) | Hover over a ghost: one step stronger |
| Selection rectangle | **New `theme::SELECTION_FILL`**: `theme::ACCENT` at very low strength; outline `theme::ACCENT` | — |
| Stretch handle | Plate `theme::RAISED`, glyph `theme::TEXT` | Hover `theme::RAISED_HOVER`; dragging `theme::ACCENT` |
| Splitter | `theme::PANEL`, top and bottom edges `theme::GRID_BAR` | Grip `theme::TEXT_DIM`; hover `theme::TEXT` |
| Property lane background | `theme::PANEL_DARK` | — |
| Lane grid lines | `theme::GRID_LINE`, `theme::GRID_BEAT`, `theme::GRID_BAR`, each a step quieter than in the grid | Centre line: `theme::GRID_BAR` |
| Lane target picker | Text `theme::ACCENT` (existing) | Hover: plate `theme::RAISED_HOVER` |
| Lane name watermark | `theme::TEXT_DIM` at low strength | — |
| Stems | Note colour (bar, head and tail) | Selected: `theme::NOTE_SELECTED`; ghost channels: `theme::GHOST_NOTE`; muted: reduced strength; dragged stem: one step brighter |
| Drag readout | `theme::LCD_BG` box, `theme::LCD_EDGE` border, `theme::TEXT` value | — |
| Ghost-channel panel | `theme::PANEL`, border `theme::LCD_EDGE` | Dot on: channel colour; off: `theme::RAISED` outline; current channel name `theme::TEXT` bold, others `theme::TEXT_DIM` |
| Menus | Toolkit menus with `theme::apply` visuals | Destructive items `theme::DANGER`; disabled `theme::TEXT_DIM` |

## Labels and wording

- **Window title**: "Piano roll — ‹channel name›" (existing). The chevron after it marks it as the channel picker. The collapsed header shows the same text.
- **Option buttons' hints and lists**:
  - Snap: "Snap: ‹value›", values None, 1/4 step, 1/2 step, Step, Beat, Bar (existing). Proposed new value: "Follow zoom", which snaps to the finest grid line currently drawn.
  - Scale: "No scale" or "‹root› ‹scale›" ("C major", "A natural minor"). Existing hint "Highlight the keys of a scale".
  - Chord: "Note" or the chord name (existing list from `musiccraft_project::music::CHORDS`, for example maj7, min7). Existing hint "Click draws a single note or a chord".
  - Note-tools button: "Note tools: quantize, strum, generate and more".
- **Tool hints** (existing pattern "Name (key): what it does"; texts as in `piano_roll.rs`):
  - "Draw (P): click adds a note, drag moves or resizes, Ctrl+drag selects"
  - "Paint (B): drag to lay down notes one after another"
  - "Delete (D): click or drag over notes to delete them"
  - "Mute (T): click or drag over notes to mute or unmute them"
  - "Slice (C): click a note to cut it in two there"
  - "Select (E): drag a box around notes; click empty space to clear"
  - "Zoom (Z): click to zoom in, Shift+click to zoom out"
  - "Play (Y): click to hear a note or key". Proposed rename: "Audition (Y): click to hear a note or key", so it is not confused with the new pattern play button (A7).
  - Optional: "Paint steps (N): drag to lay one-step hits; click a note to mute it", "Ramp (I): drag across the lane to smooth steps into a ramp".
- **Corner block**: "Colour group N". Slide and Porta keep their existing hints: "(S) New notes are slide notes: they bend the notes they overlap (in their colour group) to their key" and "(O) New notes are portamento notes: they glide from the previous note and retrigger". Proposed: reorder them to the house style, "Slide (S): …", "Porta (O): …".
- **Lane targets**: Velocity, Pan, Release, Fine pitch (existing), and the two free modulation values. Their current labels "Mod X" and "Mod Y" are a proposed rename to **"Mod A"** and **"Mod B"** (our own names; same rename in the channel-rack section). The lane-name watermark uses the same words.
- **Renamed Tools items** (plain actions in our own words; done 2026-10-22, commands unchanged except `note.claw`, now `note.keep_every`): "Chop to grid" → "Split at grid lines", "Glue" → "Join touching notes", "Flip horizontally" → "Mirror in time", "Flip vertically" → "Mirror in pitch". The Shape item "Claw every 2nd" → "Keep every 2nd" (its hint, "Keep every second note or chord", already says this).
- **Other hints**: ruler "Click to seek · drag to select time · right-click for markers"; keyboard "‹key name› · click to hear · right-click to name this key"; splitter "Drag to resize the lane · right-click to reset"; stretch handle "Drag to stretch the selection in time · Alt ignores snap"; ghost button "Ghost notes: click for channels, right-click for options".
- **Hint style**: sentence case, no final full stop. The tool or element name comes first with its key in parentheses, then a colon, then what a click or drag does. Several gestures are separated by " · ". Hints go to the hint bar, or appear as tooltips when the hint bar is hidden (`crates/musiccraft-editors/src/hint.rs`).
- **Number formats**:
  - Bar numbers: plain integers from 1.
  - Key names: letter, optional "#", octave, with C5 = key 60 (as in `docs/spec/piano-roll.md`).
  - Velocity: integer 1–127.
  - Pan: as the controls section ("40% left", "Centre", "40% right").
  - Fine pitch: signed cents, for example "+12 ct".
  - Release, Mod A and Mod B: integer 0–127.
  - Start and length in the note-properties window: bar.beat.step, written the same way as the transport display ("2.1.3").
  - Time signatures: "3/4". Key markers: "B minor".
- **Menu items**: our own verb-first wording as listed in L4. Product-specific tool and menu names from the manual are never used as labels.

## States and feedback

- **Hover**:
  - Buttons go one step lighter (`theme::RAISED_HOVER`).
  - Notes do not change colour on hover, but the cursor changes (N) and the hint bar names the note ("E5 · velocity 100").
  - Keys go one step darker (white) or lighter (black).
  - Marker tabs, ghost notes and the stretch handle brighten one step.
- **Pressed**: buttons show a pressed plate while the mouse is down. Keys take the accent role while held. The preview sound plays while a key or new note is held.
- **Active / on**: the active tool is filled with the accent role. Slide and Porta glyphs on the swatch are at full strength when on and faint when off. The scale button is lit while fitting is on. The ghost button glyph is in the accent role while any channel is ghosted. Chord mode shows the chord glyph (and optional name) in the highlight role.
- **Selected**: notes and their stems take the near-white selection tone (F5, K3). A time selection washes the grid in the highlight role (C4). Once two or more notes are selected, the stretch handle appears.
- **Muted**: hollow notes with dimmed labels (F6). Their stems in the lane are drawn at reduced strength.
- **Disabled**: dimmed text (`theme::TEXT_DIM`) and no hover response. The hint explains why. With no channels in the project, the grid area shows the existing "Add a channel to write notes." in dimmed text, and the tools are hidden (existing).
- **Playing**: the playhead line runs through the grid and lane. The pattern play button shows pause. Optionally, keys light in the dimmed accent role while their notes sound. Own rule: click preview on the keyboard stays silent during playback.
- **Recording**: the playhead marker and line switch to the danger role. Recorded notes appear in the grid as soon as they are written (`docs/spec/piano-roll.md`, recording).
- **Dragging**: moved, stretched or painted notes are drawn at reduced strength at their new place; the original place is empty. The selection rectangle shows a faint accent fill. The lane shows a value readout. Notes swept by right-drag or Delete vanish immediately.
- **Focus**: keyboard shortcuts act on the piano roll while the pointer is over it (existing). The focused window's header uses `theme::HEADER_ACTIVE` and its title `theme::TEXT`; other windows' titles are dimmed (main-window section).

## MusicCraft today: gaps

Ordered by visual impact. References are to the current code and to `docs/screenshots/piano-roll.png`.

1. **Header pickers lack value text.** The pattern play button (A7) is in, after the pickers. Snap, scale and chord are square icon buttons with their value in the hint and ticked in their list, the note-tools button opens the tools list, colour/porta/slide sit on the corner block, velocity is in the window menu and the title has its channel chevron (A2–A5, A8). Still missing: the small value text after each picker when the window is wide, wheel and right-click on the scale and chord pickers and the title (the snap button has both), and the fit-to-scale item. `crates/musiccraft-ui/src/piano_roll.rs` (`pickers`, `channel_picker`).
2. **Scroll/zoom row details.** The row, corner block, vertical bar and zoom box are in (`crates/musiccraft-ui/src/scrollbars.rs`). Missing: the bar-range readout in the hint while dragging a grip, right-drag on the thumb for two-way scrolling, incremental rail paging, the thin-bars setting, and the lane zoom box (K6).
3. **Zoom details.** The zoom box sets the key-row height (6 to 30 points) and double-click resets it, and the horizontal zoom keys are bound. Page Up and Page Down keep the view's left edge instead of zooming around the pointer. Middle-drag pans the view.
4. **Ruler has no markers.** A click seeks, a drag makes a time selection (band, grid wash, notes selected), beat ticks rise from the bottom and numbers thin out when bars are narrow. Still missing: markers, time-signature and key tabs, the pattern's signature at the start, and the right-click menu. `crates/musiccraft-ui/src/piano_roll.rs` (`ruler`, `draw_ruler`).
5. **Property lane details.** Stems have heads and length tails, take the selection tone when selected, show ghost stems behind, and the lane has quieter grid lines and a faint name watermark. The splitter resizes, hides (double-click, Shift+Enter) and resets the lane. Alt+click on a stem resets it to the default. Still missing: the lane zoom box, Shift+drag line drawing and the drag readout. `velocity_lane()`.
6. **Ghost notes have no panel.** Other channels' notes are drawn in one neutral tone (`theme::GHOST_NOTE`), but there are no ghosts from overlapping clips, no ghost-channel panel or header button, and no double-right-click to switch channel.
7. **Keyboard labels and menu.** Keys shade on hover with their name in the hint, and Ctrl+click selects a key's notes. Ctrl+drag selects a range of keys, middle-drag zooms, and the labels choice is in View and on a right-click key menu. Still missing: custom key names and the drawing-style choice in the key menu. `keyboard()`.
8. **No tool switching from the wheel.** The tools follow the target order and Audition (renamed from Play) has its own glyph, but there is no right-hold + wheel tool switching, and the optional Paint steps and Ramp tools are not there.
9. **No stretch handle** for multi-note selections (H2), and no Shift+drag "draw by length".
10. **No tall-row property view.** Note labels are cut with an ellipsis, but the label size is fixed. `draw_note()`.
11. **Window menu is incomplete.** It now has File, Edit, Tools, Chord, Snap, Select and Zoom submenus and the velocity field (`options_menu()`). Edit has Mute, Unmute and Apply the current colour, Select has muted notes and notes of the current colour, and Zoom has Zoom to selection. View has ghost notes, keyboard labels, note labels and the property lane. Still missing: MIDI import, View's other items (ghosts from overlapping clips, scale highlighting, bar-group shading, follow playhead), the Markers submenu, and the Channel and Lane submenus.
12. **Cursors**: the shapes follow the tool and the table in N, but there is no resize cursor between touching notes (the earlier note's right end wins).
13. **Keymap**: Page Up/Down and Shift+1–5 are bound (`keymap.md`, "Over the piano roll"). Alt+C applies the current colour and Shift+Enter hides the lane. Still unbound: Alt+V, Shift+F, N and I, which wait for the features they drive (ghost panel, lane targets, Paint steps, Ramp).

## Sources

**Public online manual** (base `https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/`). Compared for layout, order, proportions, density and behaviour only. No colours were sampled, nothing was traced or measured, and every sentence and label here is our own wording.
- `pianoroll.htm`, sections:
  - "Piano roll Basics": the axes, bars in the ruler, the title as channel picker with ticks and wheel stepping, markers in the ruler, middle-drag panning.
  - "Piano roll Controls": header order (main menu, tools menu, snap, scale, chord, the tools, play, title/channel, ghost-note button at the right), the colour-group swatch with slide/porta toggles, keyboard behaviour, splitter, lane target. These controls are icon buttons in the reference, which is why A3–A5 are icons here.
  - "Snap to grid": right-click on the snap control sets the default length of new notes; clicking a note also sets it.
  - "Scroll and zoom": Page Up/Down and the Shift+1–5 zoom keys.
  - "Piano roll Context Tool-Menu".
  - "Using the Piano roll".
  - "Selection Stretch Handle".
  - "Mouse-wheel Property Note Changes": Alt+wheel, property display at maximum vertical zoom.
  - "Note Properties & Event Editor": stems with round heads and right-going tails, Shift+drag line, Alt+click reset.
  - "Ghost Notes": solid same-pattern ghosts, open ghosts from overlapping clips, double-right-click to switch.
  - "Time Signatures" and "Scales & Keys (Key Signatures)": signature and key tabs in the ruler, lighter rows for in-scale keys.
  - "Waveform Helper View" (noted, not adopted).
  - "Understanding Slides & Portamento" and "Color Groups (Slides, MIDI, Editing)": 16 groups in a 4 × 4 picker, slide marks.
  - "Note Properties Dialog": opened by double-click; level controls, slide/porta, colour, start/duration, reset/accept. Our knob order and captions are our own.
  - "Piano roll Menu".
- `pianoroll_menu.htm`, sections File, Edit, Tools, View (Grid, Content, Time, Helpers, Piano roll), Snap, Select, Group, Zoom, Time markers, and the remaining top-level items. Used for the grouping of our window menu, the scroll-bar note overview, keyboard label and style choices, and the bar-group shading option. Item names are ours.
- Screenshot `img_shot/pianoroll_general.png` (viewed for layout, order and relative proportions only): header order with the menu first and window buttons at the right; the corner swatch spanning the scroll row and ruler with the slide/porta toggles; the scroll/zoom row and vertical zoom box; a single ruler band with numbers along the top and a marker tab on its lower edge; slide and portamento marks at the right end of the note; neutral, unlabelled ghost blocks; the splitter grip; the lane target under the keyboard, the faint lane name, stems with heads and tails, and the lane zoom box. One small icon in its first header group is not described in the manual text and is deliberately left out.
- `basics_interface.htm`: overview screenshot only, for the window's typical size and position among the other panels.

**Repository**:
- `crates/musiccraft-ui/src/piano_roll.rs`
- `crates/musiccraft-ui/src/chrome.rs`
- `crates/musiccraft-ui/src/icons.rs`
- `crates/musiccraft-ui/src/playlist.rs` (ruler and markers)
- `crates/musiccraft-ui/src/geometry.rs` (snap labels)
- `crates/musiccraft-ui/keymap/default.keymap`
- `crates/musiccraft-commands/src/cmds/note_gen.rs`, `crates/musiccraft-commands/src/cmds/tests.rs` (`note.claw`)
- `crates/musiccraft-editors/src/theme.rs`
- `crates/musiccraft-editors/src/hint.rs`
- `docs/spec/piano-roll.md`
- `docs/spec/window-layout.md`
- `docs/spec/keymap.md`
- `docs/clean-room.md`
- `docs/screenshots/piano-roll.png`

**Other guide sections followed for consistency**: main-window (shared header, window menu glyph, window buttons, common window items, scroll and zoom bars, zoom-box glyph and colours, `HEADER` / `HEADER_ACTIVE`, default layout), toolbar (idle buttons with `theme::TEXT` glyphs, the play button's lit state), controls (pan value format, knobs), channel-rack ("Mod A" / "Mod B"), plugin-windows (shared `KEY_WHITE` / `KEY_BLACK`).

**Own design**:
- All colours: roles mapped to `theme.rs` tokens, and the proposed token names (`KEY_WHITE`, `KEY_BLACK`, `KEY_SEAM`, `SCALE_ROW`, `SCALE_ROOT_ROW`, `PAST_END`, `BAR_GROUP`, `NOTE_OUTLINE`, `NOTE_SELECTED`, `GHOST_NOTE`, `SELECTION_FILL`) with their relations.
- All glyphs: wand, grid corner, staircase, stacked dots, sound-wave play tool, ghost bars, paired chevrons, two-line grip, sharp-sign key marker, paint-steps squares, ramp line, and the ramp and curve note marks.
- The ruler's tones and tick marks, the optional value text after the option buttons, and fit-to-scale as a tick item in the scale menu.
- Velocity shown as note-body strength; the near-white selection tone.
- Extending click-to-set-defaults to velocity.
- The sounding-key highlight, the lane value readout and the lane zoom box's value scaling.
- Double-click to hide the lane, click-and-hold to cycle slide/porta, the pointer tool menu's layout, and silent keyboard preview during playback.
- The fixed zoom levels behind Shift+1–3 and all window-local key choices.
- The "Follow zoom" snap value and "Follow playhead".
- All labels, hint texts, menu item names (including "Mod A" / "Mod B", "Split at grid lines", "Join touching notes", "Mirror in time / in pitch", "Keep every 2nd", "Paint steps", "Ramp", "Audition") and number formats.
- Exact sizes (from window-layout.md, not from screenshots).

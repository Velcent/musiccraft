# Style guide: Playlist

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The playlist is where the song is put together. Clips of patterns, sounds and automation sit on numbered tracks that run left to right in bars. It is the biggest window and the one users look at most. It should feel like a dark, quiet timeline: thin chrome at the top and left, then a wide field of lanes where the clips are the only bright, saturated things. Each clip has a strong coloured title strip and a darker body that shows a small picture of its content. The narrow clip list at the left, the track headers and the two-band ruler stay compact and low in contrast, so the arrangement reads at a glance even with dozens of clips.

Behaviour that our specs do not have yet is marked **Proposed (needs `<spec>` updated first)**; the current behaviour stays the default until that spec changes. Layout, order and proportions are this guide's target; the spec changes they need are listed under gaps. Every new key below is window-local (active only while the pointer is over the playlist and no text field has focus, in the way `keymap.md` "Over the channel rack" works), so none clashes with a global binding.

## Placement and proportions

**Where it lives**

- The playlist is an editor window like the channel rack, piano roll and mixer (see the main-window section). It is not the workspace background. Its first row is the shared compact header (main-window section, "The header row, left to right"; `docs/spec/window-layout.md`, "Window header").
- Default placement: it runs across the whole width of the workspace in the lower part, a little more than half the workspace height. The channel rack sits top-left with the mixer to its right along the top, and the piano roll floats over the playlist's right half. The default arrangement is the main-window section's.
- Opening: F5 toggles it (existing binding). The raise-before-hide behaviour of the toggle is the main-window section's proposal. The same toggle is on the toolbar's playlist window button and in the View menu.
- **Proposed (needs `keymap.md` and `window-layout.md` updated first):** Enter toggles the playlist between maximized (filling the workspace) and its previous rectangle, and double-clicking empty header space does the same (main-window section). Enter is unbound today.
- Like every editor window it moves by its header, resizes from all edges and corners, collapses to its header row, maximizes and detaches ("Detach from main window" in its window menu).
- A minimum size keeps the header, the clip list, the track headers and at least two bars of two tracks visible.

**Inner layout, top to bottom and left to right**

```
+-------------------------------------------------------------------------------------+
| ☰ | ⌟• | ✎ ◢ ⌫ ✕ ⇿ ⫽ ⬚ ⌕ ▷) | ▶ | Playlist — Arrangement › | ▤ Pattern 3 ›   …   ▔ ⛶ × |
+--------------+----------------+-------------------------------------------------+----+
| list filters | corner block   | ‹ [ scroll + zoom bar, optional song overview ] › | zb |
| (3 switches) | kind switches  +-------------------------------------------------+----+
+--------------+ kind options   | ruler band 1: bar numbers, playhead marker      | ^  |
|              | (+)            | ruler band 2: markers                           |    |
|  clip list   +--------------+-+-------------------------------------------------+    |
|  (miniature  | track header |▌| lane 1                                          | v  |
|   clips,     +--------------+-+-------------------------------------------------+ e  |
|   own thin   | track header |▌| lane 2                                          | r  |
|   scroll     +--------------+-+-------------------------------------------------+ t  |
|   bar)       | …            | | …                                               |    |
|      +       |              | |                                                 | v  |
+--------------+--------------+-+-------------------------------------------------+----+
                               ^ track selector strip
```

| Part | Relative size | Notes |
| --- | --- | --- |
| Header | One row, as tall as one tool button (button size in window-layout.md) | Never wraps. Tools that do not fit fold into the header's "more" chevron, lowest-priority group first (main-window section). |
| Clip list column | About an eighth of the window width at the default size, a little wider than the track-header column | Full height under the header. **Proposed (needs `playlist.md` updated first):** its width can be dragged at its inner edge, and it can be hidden or moved to the right side of the window (window menu → View). |
| Corner block | As wide as the track-header column plus the selector strip, as tall as the scroll row and the ruler together | Sits above the track headers |
| Track-header column | About a tenth of the window width (our current width is fine) | Fixed width. It does not grow when the window grows. |
| Track selector strip | Very narrow, about a quarter of a tool button | Between each header and its lane |
| Scroll/zoom row | Thin, a little less than the header's height | Over the lanes only, directly under the header (main-window section, F) |
| Ruler | Two stacked bands, each about as tall as the scroll row | Over the lanes only |
| Right column | As wide as the scroll row is tall | Vertical zoom box at its top, vertical scroll bar under it |
| Lanes | All remaining space | One lane per track |

- **Track height**: in the reference a default track is several tool buttons tall (roughly three to four). That leaves room in its header for a name line, an empty middle and a bottom line of small controls. Our default (window-layout.md) is about one and a half tool buttons, which is enough for a clip's title strip and a readable miniature but too short for the two-line header in D.
  - **Proposed (needs `window-layout.md` updated first):** raise the default track height to about three tool buttons.
  - Until then, and for any track made short, the header uses its **compact variant** (D, "Short tracks"): name and mute/solo switch on one line, the resize grip only on the divider.
  - Heights range from about one text line (title strips only, no miniature) up to several times the default. The vertical zoom box scales all tracks together. A track's divider resizes that track alone.
- **Horizontal zoom**: at the default zoom a bar is a few tool buttons wide, so a full-width playlist shows a couple of dozen bars or fewer (own choice; our current default is fine).
- **Density**: chrome text is small, at the same size as the other editors. Clip names are a little smaller than chrome text. Between track headers there is only a hairline gap. Clips fill their lane's height except for a hairline above and below.

## Anatomy

### A. Header row (left to right, one row)

The slots and their order are the main-window section's ("The header row, left to right"): window menu button, tool groups, window play button, title, drag area, right-hand items, then collapse, maximize/restore and close. The window menu button (slot 1), drag area (slot 5) and window buttons (slots 7 to 9) look and behave exactly as defined there. This section lists only the playlist's own tools, title and selector, and the contents of its window menu.

| # | Element | What it shows | Behaviour | States |
| --- | --- | --- | --- | --- |
| A1 | Window menu button (shared slot 1) | Shared three-bar glyph, flush with the top-left corner | Click opens the playlist menu (A12) under the button | As in the main-window section |
| A2 | Snap button (first tool group) | A square icon button with the snap glyph (`Icon::Snap`, shared with the toolbar's global snap), the same size as the tools. When the header has room, the current value ("Bar") follows the glyph in small text (own). | Click opens the snap list: None, 1/4 step, 1/2 step, Step, Beat, Bar (existing values), and two proposed values: "Follow zoom" (the finest grid line drawn) and "Clip edges" (the starts and ends of neighbouring clips on the same track). The current value is ticked and also shown in the hint. **Proposed (needs `playlist.md` updated first):** wheel over it steps through the values; right-click resets to "Bar"; holding Alt during any drag ignores snap for that drag. | Hover: one step lighter. List open: pressed look. |
| — | Group gap | Slightly wider space | — | — |
| A3 | Tool buttons, in this order: **Draw** (P), **Paint** (B), **Delete** (D), **Mute** (T), **Slip** (new), **Slice** (C), **Select** (E), **Zoom** (Z), **Play from here** (Y) | Square icon buttons of equal size, touching or nearly touching | Click picks the tool. Exactly one is always active. The choice is shared with the piano roll (existing). **Proposed (needs `keymap.md` updated first):** Slip takes Shift+S, window-local; plain S stays the global slide-note toggle. **Proposed (needs `playlist.md` updated first):** holding right-click while rolling the wheel over the group steps through the tools. Tool behaviour is listed under F. | Active: `theme::ACCENT`-filled plate with a dark glyph. Inactive: `theme::RAISED` plate with a `theme::TEXT` glyph (the shared button rule of the toolbar and design-language sections). Hover: one step lighter. |
| — | Group gap | — | — | — |
| A4 | Window play button (shared slot 3) | Play triangle; two pause bars while the song is playing | **Proposed (needs `transport.md` updated first):** click starts song playback from the playhead (switching the transport to song mode) or pauses it. Right-click while playing stops and returns to where playback started. Right-click while stopped moves the playhead to the song start. | Lit in the accent while the song is playing |
| A5 | Title (shared slot 4) | "Playlist — Arrangement", followed by a small right chevron once it is a selector | With one arrangement (today) the title is plain text with no chevron. Later, with several arrangements, click opens the arrangement list: switch, add an empty one, clone, rename, delete, and merge another arrangement into this one (later). Wheel over the title steps through them (main-window proposal). | Focused window: normal text. Unfocused: dimmed. Hover over a selector: brighter. |
| A6 | Placing selector (right after the title) | A small kind glyph (pattern, audio or automation, see Icons), the name of what a click on a lane places, and a small right chevron | Click opens a list of every pattern, audio source and automation, grouped by kind with a thin rule between groups. The current one is ticked. Choosing an item selects it in the clip list too. **Proposed (needs `playlist.md` updated first):** wheel over it steps through the current kind's items; Ctrl+wheel steps through all items. | Name in normal text. The glyph takes the item's colour. The chevron points down while the list is open. |

Notes on the header:

- **Proposed (needs `playlist.md` updated first):** the header drops today's text menu "Clip" and the zoom "−" and "+" buttons. Clip editing moves into the window menu (A12) and each clip's own menu (E3). Zoom stays in the scroll bar's grips, the zoom box, Ctrl+wheel and the Zoom tool. Until the spec changes, both stay as they are.
- When the window is narrow the header folds, in order: the snap value text (the glyph stays), then the placing selector's name (its glyph and chevron stay), then the tool group into the "more" chevron (own order).

**A12. Window menu** (our wording; items that exist today are marked ✓; new keys are window-local and proposed):

- **Edit ▸**: Cut, Copy, Paste, Duplicate (Ctrl+B ✓), Delete (Del ✓). Then **Insert time** (Ctrl+Ins ✓) and **Remove time** (Ctrl+Del ✓), both using the time selection; these names are a proposed rename of today's labels (see Labels and wording). Then Nudge left / right one snap step (Shift+Left / Shift+Right) and Move up / down one track (Shift+Up / Shift+Down), all proposed. Then "Allow trimming from the left edge" (a tick item, proposed).
- **Clip ▸** (today's "Clip" menu ✓): Slice at the playhead, **Make independent copy** (proposed rename of today's label), Merge, Consolidate (render to audio), Reverse, Fade in, Fade out. With nothing selected, a dimmed line says "Select clips first (Select tool or Ctrl+A)" (existing).
- **Select ▸**: All (Ctrl+A ✓), None (Esc ✓), Invert, Muted clips, Clips of the same source, Everything inside the time selection.
- **Snap ▸**: the same list as A2.
- **Zoom ▸** (proposed keys): Zoom in (Page Up), Zoom out (Page Down), Zoom presets 1, 2 and 3 (Shift+1, Shift+2, Shift+3), Zoom all the way out and back (Shift+4), Zoom to the time selection (Shift+5, dimmed without one), Fit the song, Centre on the playhead. Shift plus a digit is unbound today; plain digits keep their global pattern-select meaning.
- **Markers ▸**: Add marker at the playhead (Alt+T ✓), Add pause marker at the playhead, Time signature at the playhead…, Add a marker every 4 / 8 bars, Remove all markers.
- **Tracks ▸**: Reset all track heights, Lock all heights, Unmute all tracks, Clear solo.
- **View ▸**: Clip list (shown ✓ by default; Alt+P, proposed), Clip list on the right side, Clip list width ▸ (Narrow, Normal, Wide), Song overview in the scroll bar, Keep clip names in view while scrolling, Bar-group shading, Track separators, Clip shadows, Follow the playhead (on ✓ by default).
- Then a separator and the common window items: Detach from main window, Maximize / Restore, Collapse to header, Reset position (main-window section).

Right-clicking a tick item toggles it without closing the menu (main-window section; proposed there).

### B. Clip list column (left; moves to the right edge when "Clip list on the right side" is on)

Top to bottom:

1. **Filter switches row** (as tall as the scroll row): three small glyph switches side by side, centred in the column: **Patterns**, **Audio**, **Automation** (kind glyphs, see Icons).
   - Click a switch to show only that kind. Click the lit switch again to show all kinds. With all kinds shown, no switch is lit. This replaces today's text tabs (All, Pat, Aud, Aut).
   - **Proposed (needs `playlist.md` updated first):** right-click any switch for a small menu: Show all kinds, then Sort by ▸ (Creation order, Name, Colour, Mixer track — audio only), then Select unused (items with no clip in the song), Delete unused.
   - Hover: "Patterns: show only patterns · click again for all · right-click to sort" (one hint per kind).
2. **The list** (fills the column): every item is a **miniature clip**, the full width of the list less its scroll bar, stacked with a hairline gap. Order: patterns, then audio, then automation when all kinds are shown, each group in the chosen sort order. There are no group headings. The kind glyph in each title strip tells the kinds apart.
   - **Miniature** (the existing pattern miniature, extended to every kind; see `docs/spec/playlist.md`, "Pattern miniatures"):
     - title strip: kind glyph, then the name, in the clip colour;
     - body: one pass of the content (patterns: notes laid out as in a clip; audio: the waveform of the whole sound; automation: the curve over its whole length);
     - a pattern with neither notes nor automation says "empty" in dimmed text (existing).
   - **The current item** (what a click on a lane places) has an accent outline (existing). A small accent notch also sits on the column's inner edge, level with it (own). Hovering an item gives a dimmed outline.
   - **Click** makes the item current (existing).
   - **Double-click** opens the item's editor: automation → automation editor (existing); pattern → piano roll (as double-clicking a pattern clip does today); audio → channel settings for its sampler channel (proposed).
   - **Right-click** opens the item's menu:
     - Patterns: Rename…, Clone, Colour ▸, Time signature…, Clear automation (n lanes), Delete (all existing), plus Show in playlist (scrolls to its first clip; Shift+C, window-local, proposed).
     - Audio: Rename…, Colour ▸, Channel settings…, Show in playlist, Delete (removes the channel and its clips, undoable).
     - Automation: Edit…, Rename…, Delete (existing), plus Show in playlist.
   - **Wheel** scrolls the list.
   - **Proposed (needs `playlist.md` updated first):**
     - **Drag** an item out onto a lane to place a clip of it at the drop point (see H).
     - **Drag within the list** reorders items of the same kind. For patterns this changes pattern numbers, so it asks first (own) and is one undo step. **Shift+wheel** over a selected item moves it up or down.
     - **Ctrl+click** adds items to a multi-selection and **Shift+click** selects a range, for renaming, recolouring or deleting several at once.
     - **Alt+right-click and hold** previews the item: a pattern plays alone, a sound plays once. Release stops it.
3. **List scroll bar**: thin, inside the column's inner edge, taking its own strip so it never covers a miniature (existing rule). At its top is a small **zoom box** (the same drawing as the main vertical zoom box, smaller). Dragging it up or down changes the miniature height, from title strip only up to about twice the default (proposed).
4. **Add button** at the bottom, centred: a "+" that adds a new empty pattern and makes it current. It replaces the "+ New pattern" text button. Hint: "New pattern (F4 picks the next empty one)".
5. **Inner edge**: a hairline divider. **Proposed:** drag to resize the column; double-click toggles between the current width and the default width (own).

When the list has no items of the shown kind, the list area shows one dimmed line: "No audio yet: drop a sound on the playlist" or "No automation yet: right-click a knob → Create automation clip" (adapted from the existing "Drop a sound on the playlist" and the "Create automation clip" knob menu item).

### C. Corner block (above the track headers)

A fixed block in the window's chrome tone. Top to bottom:

1. **Kind switches** (top row, right-aligned over the header column): the same three kind glyphs as the clip list's filters. **Proposed (needs `playlist.md` updated first).** They choose which kind's options appear in row 2 and which kind is **in front**:
   - Click a switch: that kind is in front. Its clips draw at full strength and answer clicks. The other kinds draw at reduced strength and are passed through by the tools (own adaptation; our clips never overlap on a track, so this is a view and edit filter, not a stacking order).
   - Click the lit switch again: no kind is in front, and all kinds are equal (the default, and today's behaviour).
   - Ctrl+click: every kind in front (same as the default).
2. **Kind options row** (small glyph or text toggles under the switches; only for the kind in front, or for patterns when none is):
   - Patterns: "Colours ▾", a compact picker: By pattern (default, today's behaviour), By channel, By note. It sets where pattern clips take their colour.
   - Audio: "Fades" toggle: shows fade and gain handles on audio clips (E5).
   - Automation: "Points" toggle: shows control points on automation clips, which can then be edited in place (E6).
3. **Add-track button** (bottom-left corner of the block): a "+". It is hidden while the track count is fixed at 64. Later it adds a track below the selected one.

Hover hints name each switch and option, for example "Audio in front: other clips are dimmed and ignored by the tools".

### D. Track headers (one per track, top to bottom, in the header column)

Each header is a plate the full width of the column and the height of its track, with a hairline gap under it. Layout inside the plate at the default (target) height:

```
+--------------------------------+
| Name of the track          (▾) |   top line
|                                |
| ═   [icon]   (●arm)  (S)   (◉) |   bottom line
+--------------------------------+
  ^grip                     ^mute/solo switch
```

| # | Element | Where | Behaviour | States |
| --- | --- | --- | --- | --- |
| D1 | Plate | The whole header | Click selects the track (as D7). Right-click opens the track menu (D8, existing). **Proposed (needs `playlist.md` updated first):** double-click renames in place; Shift+wheel over the plate moves the track up or down (later, with track reordering). | Plain tracks: raised plate. Coloured tracks: the whole plate tinted with the track colour, toned well down (replacing today's thin stripe). Selected track: one step lighter. Hover: one step lighter. |
| D2 | Name | Top-left, one line of small text, cut short with an ellipsis before it reaches the right end | Part of the plate | Normal text. Muted, or silenced by another track's solo: dimmed (existing). |
| D3 | Group chevron (later) | Top-right | Folds or unfolds a group of tracks | Only on group parents |
| D4 | Resize grip | Bottom-left: a small two-line grip | **Proposed (needs `playlist.md` updated first):** drag up or down to resize this track alone. The pointer becomes a vertical resize arrow over the grip and along the plate's bottom edge. Alt+click resets this track to the global height. While dragging, the hint bar shows the height as a percentage of the global height. | Hover brightens the grip |
| D5 | Track icon (later, optional) | Bottom line, after the grip | Shows a small glyph the user picked for the track from our own glyph set (Track menu → Icon…). Purely a label; click acts as the plate. | Same tone as the name |
| D6 | Arm dot | Bottom line, left of the switches (existing) | Click arms or disarms the track for audio recording (existing `playlist.toggle_track_arm`). **Proposed:** shown only while the track is armed or while the pointer is over the header. | Armed: filled in the danger tone. Disarmed: outline only. |
| D7 | Solo button "S" | Bottom line, left of the switch (existing) | Click solos or unsolos the track (existing `playlist.toggle_track_solo`) | Soloed: lit in the highlight tone. Off: dim. |
| D8 | Mute/solo switch | Bottom-right corner: a small round light | Click mutes or unmutes the track (existing). **Proposed (needs `playlist.md` updated first):** Ctrl+click or right-click on the light solos or unsolos the track; Shift+click locks its state, so that "Unmute all tracks" and "Clear solo" leave it alone. | Lit (accent): the track plays. Dark with a dim ring: muted. Soloed: lit, with a ring in the highlight tone. A track silenced by another track's solo shows a lit light at reduced strength. Locked: a short bar under the light. |
| D9 | Track selector strip | The narrow strip between the plate and the lane | Click selects the track. The selected track is the target for Insert, Delete, Move and audio drops on headers. **Proposed (needs `keymap.md` updated first):** Ctrl+Up and Ctrl+Down move the selection (window-local; both are unbound today). | Selected: filled in the accent. Others: the chrome's dark tone. |
| D10 | Divider | The plate's bottom edge | Same drag as D4 (proposed) | — |

- **Solo button, target**: **Proposed (needs `playlist.md` updated first):** once solo lives on the light (D8), the separate "S" button becomes a small "S" badge in the highlight tone, shown left of the light only while the track is soloed, so solo can still be read at a glance. Until then the "S" button stays as it is.
- **Short tracks** (compact variant, own design): when the track is shorter than about two text lines, the plate shows one line only: the name at the left, then the arm dot (if armed), the "S" button or badge and the light at the right. The grip is not drawn; the divider (D10) still resizes the track.
- **Track kinds (later, out of scope now)**: the reference can tie a track to one instrument or one mixer insert, and such tracks show a kind glyph and, for audio tracks, a few small input and monitoring controls in the plate. MusicCraft's tracks hold any clip kind and are not tied to a mixer insert, so the plate has none of these today. If track kinds come later, their controls go on the bottom line between the icon and the arm dot.

**Track menu** (D8 on the plate; our wording; ✓ = exists today):

- Rename… ✓, then the colour swatches (16) ✓ and No colour ✓.
- Icon… (later): pick a glyph for D5, or none.
- Name after first clip (own wording): names and colours the track after its first pattern clip.
- Colour clips to match: recolours every clip on the track. For pattern clips this sets the pattern colour, so it asks first (own).
- Reset name, colour and icon.
- Height ▸ (50%, 75%, 100%, 150%, 200%, 300%) ✓, then Lock height (tick).
- Mute all clips, Unmute all clips (clip mutes, separate from the track light).
- Select all clips on this track.
- Join this track's pattern clips into one (the existing merge, applied to all of the track's pattern clips).
- Render this track to audio (the existing consolidate, applied to all of the track's pattern and audio clips).
- Arm for recording ✓ (mirrors D6).
- Later, with a variable track count: Insert track above, Delete track, Clone track, Move up, Move down.

All items except the existing ones are **proposed (needs `playlist.md` updated first)**.

### E. Lanes and clips

**E1. Lanes (grid)**

- One lane per track, all the same tone, separated by hairline track separators. They do not alternate in tone. A coloured track's lane is not tinted (own; one neutral lane tone).
- Vertical lines run through every lane:
  - step lines, the faintest, drawn only when zoomed in far enough that a step is wider than a small gap;
  - beat lines, a step stronger;
  - bar lines, clearly stronger than beats;
  - the first bar line of each bar group, one step stronger again.
- When zoomed out, step lines go first, then beat lines, then every other bar line.
- **Bar-group shading** (View option, on by default): alternate groups of bars get a very faint lighter wash. A group is four bars of the time signature, or one bar when a bar is very wide. Long songs then read in phrases.
- Past the end of the song (the last clip's end rounded up to a bar), the lanes carry on in the same tone. Only the ruler marks the song end (G1, own).
- Empty lane, click with Draw: places the current item (A6) at the snapped position (existing). Ctrl+drag on an empty lane draws a selection box with any tool (existing).
- **Proposed (needs `playlist.md` updated first):** middle-drag scrolls both ways (main-window section, F). Ctrl+right-click on an empty lane zooms to fit the whole song; doing it again returns to the previous zoom and position.

**E2. Clip shape (every kind)**

- A rounded-corner rectangle (small radius) spanning its time on its track, inset by a hairline above and below.
- **Title strip** across the top: about one line of small text tall (our current title height is fine). It is filled with the clip colour at full strength. It holds:
  - the **clip-menu glyph** at its left end (the kind glyph);
  - the **name**, in small text, cut at the clip's right edge.

  With "Keep clip names in view while scrolling" on, when the clip's start is scrolled off the left edge, the glyph and name slide along to stay at the left of the visible part.
- **Body** under the strip: the clip colour toned well down, to a little lighter than the lane. Lane grid lines show through faintly. The miniature of the content is drawn in the body, brighter than the body.
- A hairline outline a step darker than the body keeps neighbouring clips apart.
- When the track is too short for a body (about one text line), only the title strip is drawn.
- **Pattern repeats**: when a pattern clip is longer than its pattern, each repeat start gets a faint short tick hanging from the title strip into the body (own design).

**E3. Clip-menu glyph** (title strip, left)

- Click opens the clip's menu. Hover brightens the glyph.
- Right-click on a clip deletes it with every tool today (existing, `playlist.md`). **Proposed (needs `playlist.md` updated first):** with the Mute, Slip, Slice, Select, Zoom and Play tools, right-click on the title strip opens the clip's menu instead; with Draw, Paint and Delete, right-click keeps deleting.
- Menu (our wording): Preview (play this clip alone), Open in editor (piano roll, automation editor or channel settings), Change source ▸ (the list of the same kind; Shift keeps the clip's start and end), Rename and colour…, Make independent copy ✓ (existing action, proposed label), Select all clips of this source, Mute (tick), then the kind-specific items (patterns: Time signature…; audio: Reverse ✓, Fade in ✓, Fade out ✓, Consolidate ✓; automation: Edit points…), then Delete.

**E4. Pattern clips**

- Title strip colour: the pattern colour. An uncoloured pattern uses the neutral clip fill (existing rule). **Proposed:** with "Colours ▾" set to By channel or By note (C2), the colour comes from the first channel in the pattern or the first note's colour group.
- Body: the notes as short horizontal dashes, each in its own colour (existing rule: the note's colour group, or its channel colour), on the darker body. A thin dark edge keeps them readable (existing). Notes sit by key from the lowest at the bottom. A step-sequenced pattern with several channels gets one row per channel (existing).
- Event automation recorded into the pattern: faint curves under the notes, in the automation role (existing).
- Double-click: selects the pattern and opens the piano roll (existing).

**E5. Audio clips**

- Title strip colour: its sampler channel's colour.
- Body: the waveform, centred on a mid line. Stereo sounds show two halves, left above right (optional). The waveform is drawn in a light tint of the clip colour on the darker body, so it is the brightest thing in the clip. This reverses today's dark-on-colour drawing.
- The waveform is drawn at the sound's real length. A clip shorter than the sound cuts it. A clip longer than the sound shows the body empty after the sound ends (existing).
- **Proposed (needs `playlist.md` updated first)**, with "Fades" on (C2) or while the pointer is over the clip:
  - **fade handles**: one small square grip at each top corner of the body, just under the title strip. Drag a grip inward to set the fade length. A thin fade line runs from the body's bottom corner up to the grip. Right-click a grip to reset it. Hint: "Fade in: 0.25 s · drag to change · right-click to reset". This sets the channel fades (existing `playlist.set_clip_fade`).
  - **gain handle** (later, needs a clip gain parameter): a short horizontal bar at the bottom centre. Drag up or down to set the gain. A small value badge ("−0.5 dB") shows above it while dragging. Alt+click resets it.
- Reversed clips: the waveform is drawn mirrored, and a small left-pointing arrow follows the name.
- Double-click: **proposed:** opens the channel settings for its sampler channel (today it does nothing).

**E6. Automation clips**

- Title strip colour: the automation role (one colour for all automation today; a per-automation colour can come later through Rename and colour…).
- Body: the shaped curve as a line in the automation role, with a translucent fill under it down to the bottom of the body. This makes the level readable at small sizes. Today there is only the line.
- **Proposed (needs `playlist.md` updated first)**, with "Points" on (C2) and the track tall enough:
  - control points are shown as small filled circles on the curve, and each segment has a small diamond at its middle for its tension;
  - drag a point to move it (it snaps; Alt ignores snap; Shift locks it vertically, Ctrl horizontally);
  - right-click and drag on the body adds a point;
  - right-click a point to delete it;
  - drag a diamond up or down to bend the segment, and right-click a diamond to reset it (the same commands as the automation editor);
  - right-click a segment for its shape: **Curve**, **S-curve**, **Hold**, **Steps** (the existing four shapes; these names are a proposed rename of today's labels, shared with the dialogs section);
  - automation clips then resize from the ends of their title strip, not from the body edges, so the body's edges stay free for points.
- Double-click: opens the automation editor (existing).

**E7. Clip edges**

- The right edge resizes (existing).
- **Proposed (needs `playlist.md` updated first):** the left edge trims the start, moving the clip's offset (an Edit menu option "Allow trimming from the left edge", on by default once adopted).
- The resize zone is a narrow band inside each end of the body. The pointer becomes a horizontal resize arrow there.
- When a resize would make a pattern clip longer than its pattern, the repeats appear live (E2 ticks).

### F. Tools on the lanes (what each tool does, behaviour summary)

"Existing" marks today's behaviour. Everything marked "proposed" **needs `playlist.md` updated first**. Today right-click on a clip deletes it with every tool.

| Tool | Left click | Left drag | Right click / drag | Modifiers |
| --- | --- | --- | --- | --- |
| Draw (P) | Empty lane: place the current item. Clip: select it. (existing) | Clip body: move (and change track). Clip edge: resize. (existing) | Click deletes a clip (existing). Right-drag deletes every clip it crosses (proposed). | Ctrl+drag: selection box (existing). Shift+click on a clip: add it to or remove it from the selection (existing). Shift+drag on a clip: copy it (proposed). Alt: no snap (proposed). |
| Paint (B) | As Draw | Empty lane: lay the current item end to end, skipping taken places (existing) | As Draw | As Draw |
| Delete (D) | Delete the clip (existing) | Delete every clip crossed, one undo step (existing) | Delete (existing) | — |
| Mute (T) | Mute or unmute the clip (existing) | Mute or unmute every clip crossed (proposed) | Delete (existing) | — |
| Slip (Shift+S, proposed) | — | Slide the content inside the clip left or right while its start and end stay (changes the offset) | Delete (as today) | Alt: no snap |
| Slice (C) | Cut the clip at the snapped click (existing) | Vertical drag: cut every clip the line crosses at that time (proposed) | Delete (existing) | Alt: no snap (proposed) |
| Select (E) | Select the clip, or clear the selection on empty space (existing) | Selection box (existing) | Delete (existing) | Shift adds to the selection (existing) |
| Zoom (Z) | Zoom in around the click (existing) | Drag a range to zoom to it (proposed) | Delete today; proposed: zoom out | Shift+click zooms out (existing) |
| Play from here (Y) | Move the playhead there and play (existing) | — | Delete (existing) | — |

Wheel and keys over the lanes:

- Ctrl+wheel: zoom horizontally around the pointer (existing).
- Wheel and Shift+wheel: scroll vertically and horizontally (the toolkit's scroll area today; kept).
- Alt+wheel over the vertical zoom box: change the track height (proposed).
- Delete, Ctrl+A, Ctrl+B and Esc as today.

### G. Ruler (two bands, over the lanes only)

The playlist and the piano roll share one ruler design: two bands, bar numbers above and markers below (own decision; see the piano-roll section).

1. **Bar band** (top):
   - Bar numbers start at 1. They are written small, just right of each bar line, left-aligned.
   - The first bar of each bar group (every fourth bar at the default zoom) is written a step larger and brighter than the others.
   - When bars get narrow, only every 2nd, 4th, 8th… number is shown. The existing rule shows every bar or every fourth; add the steps in between.
   - Short beat ticks hang from the band's bottom edge.
   - **Song end**: a thin strip along the band's bottom edge runs from bar 1 to the end of the song (own design).
2. **Marker band** (below the bar band, one step lighter):
   - Each marker is a small tab at its time: its glyph (flag for a label, two bars for a pause, a down arrow for the song loop, a bracket for a time signature, all existing), then its name in small text. The tab has a faint plate in the marker's colour behind it.
   - A faint line in the same colour runs from the tab down through every lane (existing).
   - **Click a tab**: jump the playhead there (existing).
   - **Right-click a tab**: the marker's menu: Rename…, then the action as radio items **Label only** / **Pause here** / **Loop the song here** (the existing three actions; these names are a proposed rename of today's labels), Time signature…, Remove marker (existing); later: move everything after this marker earlier or later by one bar.
   - **Right-click empty band**: Add marker here…, Add pause marker here, Time signature here… (existing), Clear time selection (existing as "Clear loop").
   - **Proposed (needs `playlist.md` updated first):** drag a tab to move the marker (it snaps, and it may pass other markers); double-click a tab to set the time selection from this marker to the next one (or to the song end).
   - At the band's left end, the song's starting time signature is written a little larger than the other labels (own; matches the piano-roll section).
3. **Playhead marker**: a small downward triangle sitting on the top edge of the bar band at the playhead's time. A thin line drops from it through every lane. It is always shown, also when stopped (today the line appears only while the song plays).
   - Clicking or dragging in the bar band moves the playhead there (snapped; existing). Dragging the triangle scrubs the same way.
   - In pattern mode the song playhead stays where it was and is drawn at reduced strength (own).
4. **Time selection** (this is our loop region):
   - Shift+drag across the ruler selects a span of whole snap steps (existing). **Proposed (needs `playlist.md` updated first):** Ctrl+drag on either band does the same, with Shift+drag kept as an alias.
   - It shows as a band in the highlight role across both ruler bands, a very faint wash of the same role over every lane, and a thin edge line at each end through the lanes.
   - Playback loops inside it (existing).
   - Drag either edge line in the ruler to adjust it (proposed).
   - Esc, or a click on empty ruler, does not clear it. Only "Clear time selection" does, so a loop set by hand is not lost by accident (as today).
   - Hover hint for the ruler: see Labels and wording.

### H. Drag-and-drop targets

| Dragged thing | Over | Feedback while hovering | On drop |
| --- | --- | --- | --- |
| Sound from the browser | A lane | A ghost clip as long as the sound, snapped, on the hovered track. The track's header gets an accent outline. (feedback proposed) | Audio clip there (existing `DropAudio`) |
| Sound from the browser | A track header | The header gets an accent outline | **Proposed:** audio clip at the start of the visible range on that track. If the track still has its default name, it takes the sound's name. Today a drop on a header does nothing. |
| Several sounds | A lane or header | Stacked ghosts on consecutive tracks | **Proposed:** one clip per sound on consecutive tracks, one undo step |
| Item from the clip list | A lane | Ghost clip of its length, snapped | **Proposed:** a clip of it there |
| Clip being moved | A lane | The clip at reduced strength at its snapped target (existing) | Move (existing). With Shift held: copy (proposed). |
| Instrument, effect or preset from the browser | Anywhere in the playlist | No-drop cursor, and the hint says "Drop instruments on the channel rack" | Nothing |

While anything is dragged near the left or right edge of the lanes, the view auto-scrolls in that direction. It scrolls faster the closer the pointer is to the edge (own; proposed).

### I. Scroll and zoom (shared bars from the main-window section, F)

**Proposed (needs `playlist.md` updated first)**, as in the main-window section; today the playlist uses the toolkit's default scroll bars.

- **Scroll/zoom row**, under the header and over the lanes: left chevron, rail, thumb with a zoom grip at each end, right chevron, then the **vertical zoom box** at the far right.
  - Dragging a grip zooms horizontally with the opposite end fixed.
  - Dragging the zoom box up or down changes all track heights together. Double-click resets it to the default height.
  - **Song overview** (View option, off by default): the rail draws a miniature of the whole song, with one thin dash per clip in its colour at its track's relative height. Marker positions show as small ticks on the rail's top edge.
- **Vertical scroll bar** down the right column, from the ruler's top edge to the bottom: up chevron, rail, thumb, down chevron. The header, scroll row, ruler and corner block never scroll out of view. Only the lanes and track headers scroll vertically, together.
- Keys: Page Up and Page Down zoom horizontally around the playhead, and Shift+1 to Shift+5 pick the zoom presets (A12; window-local, proposed). Home moves the playhead to the start (existing global key).
- **Following**: during song playback the view jumps a page when the playhead leaves it (existing). View → "Follow the playhead" can turn this off (own; proposed).

## Icons

All glyphs are simple vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`. Each is one colour, with the shared line weight, drawn inside the shared square tool-button frame unless stated otherwise.

| Where | What it does | Our glyph |
| --- | --- | --- |
| A1 | Window menu | Three short horizontal bars stacked evenly (shared with every window) |
| A2 | Snap | New `Icon::Snap` from the toolbar section: two short lines meeting at a right angle (a grid corner) with a filled dot sitting in the corner |
| A3 Draw | Draw tool | Pencil: a slanted outlined bar ending in a filled point (existing `Icon::Pencil`) |
| A3 Paint | Paint tool | Brush: a thin handle line ending in a filled bristle wedge (existing `Icon::Brush`) |
| A3 Delete | Delete tool | Eraser: a tilted block, half filled (existing `Icon::Eraser`) |
| A3 Mute | Mute tool | Speaker with a cross to its right (existing `Icon::Mute`) |
| A3 Slip (new) | Slide content inside a clip | A wide outlined rectangle (the clip) with a short horizontal double-headed arrow inside it, the arrowheads stopping short of the rectangle's sides |
| A3 Slice | Slice tool | An outlined bar with a blade line crossing it at a slant (existing `Icon::Slice`) |
| A3 Select | Select tool | Dashed rectangle (existing `Icon::Select`) |
| A3 Zoom | Zoom tool | Magnifier: a circle with a short heavy handle (existing `Icon::Zoom`) |
| A3 Play from here | Play from the clicked point | Play triangle with one short sound-wave arc to its right. This is the piano roll's audition glyph, which keeps it distinct from the transport play. Today the tool reuses `Icon::Play`. |
| A4 | Play / pause the song | Filled play triangle; two vertical bars while playing |
| A5, A6 | Opens a list | Small right chevron after the text; points down while open |
| Kind glyph: pattern (A6, B1, C1, E3) | Marks patterns | A small square outline with three short horizontal dashes inside it at different heights, like notes on a grid |
| Kind glyph: audio | Marks audio | One smooth wave: a single rounded hump above a horizontal mid line followed by one below it |
| Kind glyph: automation | Marks automation | A smooth curve rising from lower left to upper right, ending in a small filled diamond |
| B4, C3 | Add a pattern / add a track | Plus |
| B3, I | Zoom boxes | Two horizontal lines in a small square whose gap grows with the zoom (shared with the main-window section) |
| D4 | Resize grip | Two short parallel horizontal lines, the same as the piano roll's splitter grip |
| D3 (later) | Fold a track group | Small chevron: pointing down when open, right when folded |
| D5 (later) | Track icon | A glyph the user picks from our own set (the icons in `icons.rs`) |
| D6 | Arm for recording | A small filled circle (record dot) when armed; a ring when not |
| D7 | Solo (today), solo badge (proposed) | The letter "S" in small bold text |
| D8 | Mute / solo light | A small round light: filled when the track plays, a ring when muted. A second, outer ring when soloed. A short bar under it when locked. |
| E5 fade grip | Fade length | A small filled square |
| E5 gain handle (later) | Clip gain | A short thick horizontal bar |
| E5 reversed | Marks a reversed clip | A small left-pointing arrow after the name |
| E6 points | Automation points | Small filled circles (as in the automation editor); tension: a small filled diamond |
| G2 markers | Marker kinds | Flag triangle (label), two vertical bars (pause), down-pointing arrow (song loop), bracket (time signature), all existing |
| G3 | Playhead marker | Downward-pointing filled triangle |
| I | Scroll chevrons, thumb grips | Shared with the main-window section |

## Colour roles

Tokens are in `crates/musiccraft-editors/src/theme.rs`. Names marked "new" are proposals. Some come from other sections, which are cited.

| Element | Token(s) | Relative tone and state changes |
| --- | --- | --- |
| Window body and header | `theme::PANEL`; header `theme::HEADER` / `theme::HEADER_ACTIVE` (new, main-window section) | Focused header lifts one small step |
| Tool buttons, snap button | Inactive: `theme::RAISED` plate, `theme::TEXT` glyph. Active: `theme::ACCENT` plate, `theme::BACKGROUND` glyph | Hover: `theme::RAISED_HOVER` |
| Clip list well | `theme::PANEL_DARK` | Darker than the frame, so the miniatures stand out |
| Clip list filter switches, kind switches | Glyph `theme::TEXT_DIM` | Lit: glyph `theme::ACCENT` on a `theme::RAISED` plate. Hover: glyph `theme::TEXT`. |
| Miniature title strip and body | As clips (below) | Current item: `theme::ACCENT` outline and a notch on the inner edge. Hover: `theme::TEXT_DIM` outline. Multi-selected: a faint `theme::ACCENT` wash. |
| "empty" and empty-list text | `theme::TEXT_DIM` | — |
| Corner block | `theme::PANEL` | Option toggles: `theme::TEXT_DIM` off, `theme::ACCENT` on |
| Track header plate, plain | new `theme::TRACK_HEADER`, the same as `theme::RAISED` | Hover and selected: `theme::RAISED_HOVER` |
| Track header plate, coloured | new `theme::TRACK_TINT`: the track colour (`theme::track_color`) mixed most of the way toward `theme::TRACK_HEADER`, so it stays chrome-quiet | Hover and selected: one step lighter |
| Track name, track icon | `theme::TEXT` | Silenced: `theme::TEXT_DIM` |
| Resize grip | `theme::TEXT_DIM` at partial strength | Hover and drag: `theme::TEXT` |
| Mute/solo light | On: `theme::ACCENT`. Off: `theme::PANEL_DARK` with a `theme::TEXT_DIM` ring | Soloed: outer ring `theme::HIGHLIGHT`. Silenced by another solo: `theme::ACCENT` at reduced strength. Locked: bar `theme::TEXT_DIM`. |
| Solo "S" button / badge | Off: `theme::TEXT_DIM`. Soloed: `theme::HIGHLIGHT` | — |
| Arm dot | Armed: `theme::DANGER`. Not armed: `theme::TEXT_DIM` ring | Recording: `theme::DANGER`, pulsing between full and reduced strength |
| Track selector strip | `theme::PANEL_DARK` | Selected: `theme::ACCENT` |
| Lanes | new `theme::LANE`, the same as `theme::WHITE_KEY_ROW` | Track separators: `theme::GRID_LINE` |
| Bar-group shading | new `theme::BAR_GROUP` (piano-roll section): a very faint lightening wash | — |
| Grid lines | Steps `theme::GRID_LINE`; beats `theme::GRID_BEAT`; bars `theme::GRID_BAR` (the mapping shared with the piano roll and channel rack); bar-group starts new `theme::GRID_GROUP`, one step stronger than `theme::GRID_BAR` | Bars stay clearly stronger than beats at every zoom |
| Ruler bar band | `theme::PANEL_DARK` | Numbers `theme::TEXT_DIM`; group-start numbers `theme::TEXT`; beat ticks `theme::GRID_BEAT` |
| Ruler marker band | `theme::PANEL` (one step lighter than the bar band) | — |
| Song-end strip | `theme::ACCENT_DIM` | — |
| Marker tabs and lines | Label `theme::AUTOMATION`, pause `theme::DANGER`, song loop `theme::ACCENT`, time signature `theme::HIGHLIGHT` (existing) | Tab plate: the same colour at low strength. Line: the same colour at low strength. Hovered tab: one step brighter. |
| Playhead marker and line | `theme::PLAYHEAD` | Recording: `theme::DANGER`. Pattern mode: reduced strength. |
| Time selection | Ruler band `theme::HIGHLIGHT` at low strength; lane wash `theme::HIGHLIGHT` at very low strength; edge lines `theme::HIGHLIGHT` | Edge hover: full strength |
| Clip title strip | Clip colour: the pattern colour (`theme::track_color`), new `theme::CLIP_NEUTRAL` for uncoloured patterns (names today's `UNCOLOURED_CLIP` constant; between `theme::RAISED_HOVER` and `theme::TEXT_DIM` in tone), `theme::palette` for audio, `theme::AUTOMATION` for automation | Hover: one step lighter |
| Clip body | new `theme::CLIP_BODY`: the clip colour mixed most of the way toward `theme::LANE`, slightly translucent so grid lines show through | — |
| Clip outline | The body tone, one step darker | — |
| Clip name text | `theme::TEXT` on dark title strips; `theme::BACKGROUND` on light ones (choose by the strip's lightness) | Muted: reduced strength |
| Pattern notes | Their own colours (existing `note_tint`), with a dark edge | Muted clip: reduced strength |
| Pattern event curves | `theme::AUTOMATION` at partial strength (existing) | — |
| Audio waveform | new `theme::CLIP_CONTENT`: the clip colour mixed toward `theme::TEXT`, the brightest tone in the clip | — |
| Fade grips and lines | `theme::TEXT` | Hover and drag: `theme::ACCENT` |
| Automation curve | `theme::AUTOMATION` line; fill `theme::AUTOMATION` at low strength | Points `theme::TEXT`. Selected point and dragged diamond: `theme::ACCENT`. |
| Selected clip | new `theme::CLIP_SELECTED`: the same tone as the piano roll's proposed `theme::NOTE_SELECTED` (near `theme::TEXT`, faintly toward `theme::ACCENT`). It is used for the title strip and outline. | The body keeps its colour |
| Muted clip | Every part at low strength | Selected and muted: outline `theme::CLIP_SELECTED`, strip faded |
| Clip of a kind not in front | Every part at reduced strength | — |
| Clip being moved or painted | Its normal colours at reduced strength (existing) | — |
| Recording take in progress | Outline `theme::DANGER` | Grows with the take |
| Selection box | new `theme::SELECTION_FILL` (piano-roll section); outline `theme::ACCENT` | Replaces the inline literal |
| Drop target | Ghost clip at reduced strength; header outline `theme::ACCENT` | Only while hovering a valid target |
| Scroll rail, thumb, chevrons, zoom boxes | `theme::SCROLL_RAIL`, `theme::SCROLL_THUMB` (new, main-window section), `theme::LCD_BG` and `theme::LCD_EDGE` for zoom boxes | As in the main-window section |
| Song overview in the rail | Clip colours at reduced strength | Always dimmer than the real clips |
| Menus | egui menus with `theme::apply` visuals | Delete items `theme::DANGER`; disabled `theme::TEXT_DIM` |

## Labels and wording

- **Window title**: "Playlist — Arrangement" (one arrangement today). Later: "Playlist — ‹arrangement name›". The folded header shows the same text. Today's two separate labels, a large bold "Playlist" and a dim "Arrangement", become this one small title.
- **Placing selector**: just the kind glyph and the item's name: the pattern name ("Pattern 3"), the channel name for audio, the automation name. Hint: "What a click places: ‹name› · click to choose · wheel to step".
- **Snap**: values None, 1/4 step, 1/2 step, Step, Beat, Bar (existing), plus the proposed "Follow zoom" and "Clip edges". Hint: "Snap: Bar · click to choose".
- **Tool hints** (existing pattern "Name (key): what it does"; changes to existing text are proposals):
  - "Draw (P): click places the selected item, drag moves, edge resizes" (the existing text says "pattern"; "item" covers audio and automation)
  - "Paint (B): drag to lay the selected item end to end" (same change)
  - "Delete (D): click or drag over clips to delete them" (existing)
  - "Mute (T): click or drag over clips to mute or unmute them" (existing text: "click a clip to mute or unmute it"; change once drag lands)
  - "Slip (Shift+S): drag inside a clip to slide its content; the clip stays put" (new)
  - "Slice (C): click a clip to cut it in two there; drag down to cut several" (second half once drag lands)
  - "Select (E): drag a box around clips; click empty space to clear" (existing)
  - "Zoom (Z): click to zoom in, Shift+click to zoom out, drag a range to fit it" (last part once range drag lands)
  - "Play from here (Y): click to play from there" (existing text names the tool "Play"; proposed rename so it is not confused with the transport)
- **Window play button**: "Play the song from the playhead · right-click: stop and go back".
- **Clip list**: filter hints as in B1. The add button's hint is "New pattern (F4 picks the next empty one)". The empty-list lines are as in B. The menu items are under B2.
- **Track headers**: default names "Track 1" … "Track 64" (existing). Light hint today: "Mute track"; proposed: "Track 3: click to mute · Ctrl+click or right-click to solo · Shift+click to lock". Solo button hint (existing): "Solo track". Arm hint (existing): "Arm for audio recording: takes go to the first armed track with room". Grip hint: "Drag to resize this track · Alt+click to reset". Selector hint: "Select track (Ctrl+Up / Ctrl+Down)".
- **Ruler**: existing hint "Click to seek · Shift+drag to loop · right-click for markers"; once Ctrl+drag lands: "Click to seek · Ctrl+drag to select time (loops) · right-click for markers". Marker tab hint: "‹name› · click to jump · drag to move · double-click to select up to the next marker".
- **Clip hover** (hint bar): "‹name› · bar 5 to bar 9 (4 bars) · double-click to edit · right-click the title for options". Muted clips add " · muted".
- **Proposed label renames** (our own plain wording in place of labels that echo the reference's menus; each needs `playlist.md` updated first):
  - the clip menu's space items → "Insert time" and "Remove time";
  - the clip menu's unique-copy item → "Make independent copy";
  - the marker action radio items → "Label only", "Pause here", "Loop the song here";
  - the automation segment shapes in `automation.rs` → "Curve", "S-curve", "Hold", "Steps" (same list as the dialogs section);
  - the "+ New pattern" text button → the "+" add button with the hint above.
- **Hint style**: sentence case, no final full stop. The element name comes first, then what a click or drag does. Gestures are separated by " · " (shared rule).
- **Number formats**:
  - Bar numbers: plain integers from 1.
  - Positions in hints and readouts: bar.beat.step, as the toolbar shows it ("5.2.1").
  - Lengths: "4 bars", "2 beats", "3 steps" (largest whole unit, own).
  - Time signatures: "3/4".
  - Track height while dragging: "Height 150%".
  - Fades: seconds with two decimals, "0.25 s" (existing " s" suffix).
  - Gain: "−0.5 dB" with a true minus sign and a space before the unit.
- **Menu wording**: our own verb-first phrases as listed in A12, B2, D8, E3 and G2. Never use the reference's coined panel or menu names as labels (see Sources).

## States and feedback

- **Hover**:
  - Header buttons and pickers: one step lighter.
  - Clips: the title strip goes one step lighter. The pointer shows a grab hand over the body, a horizontal resize arrow over the end bands, and a pointing hand over the clip-menu glyph. The hint bar describes the clip.
  - Track headers: the plate goes one step lighter (and, once proposed, the arm dot appears).
  - Marker tabs: one step brighter.
  - Ruler: pointing hand (existing).
  - Clip list items: dimmed outline.
- **Pressed**: buttons show a pressed plate while the mouse is down. The mute/solo light shrinks slightly while held (own).
- **Active / on**:
  - The active tool is filled with the accent.
  - Lit filter and kind switches show their glyph in the accent.
  - The window play button is lit while the song plays.
  - Option toggles in the corner block are accent when on.
- **Selected**:
  - Clips: the title strip and outline take the selection tone.
  - Track: its selector strip is accent-filled and its plate is one step lighter.
  - Clip list items: accent outline for the current item. Multi-selected items also get a faint accent wash.
  - Time selection: highlight band and wash.
- **Muted**:
  - Muted clips are drawn at low strength, and their notes, waveform or curve too (existing).
  - Muted tracks: dark light, dimmed name, and every clip on them drawn at low strength.
  - Tracks silenced by another track's solo look the same, but their light stays lit at reduced strength, so the user can tell "muted" from "not soloed".
- **Disabled**:
  - Menu items that need a selection or a time selection are dimmed. The hint says what is needed (existing pattern: "Select clips first …").
  - The add-track "+" stays hidden while the track count is fixed.
- **Playing**:
  - The playhead line moves through the lanes.
  - The playhead triangle moves along the ruler.
  - The window play button shows pause.
  - The view follows the playhead (existing).
  - Optional: clips under the playhead brighten their title strip one step.
- **Recording**:
  - The playhead marker and line turn to the danger tone.
  - Armed tracks' dots pulse.
  - The take in progress is drawn as a growing clip outline in the danger tone on its target track. It becomes a real audio clip when recording stops (existing behaviour, new drawing).
- **Dragging**:
  - Moved, resized and painted clips are drawn at reduced strength at their snapped target (existing).
  - A copy drag (Shift, proposed) leaves the originals at full strength.
  - Clips swept by Delete (existing) or by a right-drag (proposed) vanish at once.
  - Selection box: faint accent fill and accent outline (existing).
  - Drop targets: as in H.
  - The hint bar shows the position and length in bar.beat.step while dragging.
- **Focus**:
  - Keys act on the playlist while the pointer is over it (existing).
  - The focused window's header lifts slightly and its title uses the normal text tone (main-window section).
  - A track selected by the keyboard shows the accent selector strip.

## MusicCraft today: gaps

Ordered by visual impact. The spec changes (`docs/spec/playlist.md`, `docs/spec/window-layout.md`, `docs/spec/keymap.md`, `docs/spec/transport.md`) come first, per CLAUDE.md.

1. **Header row.** The playlist is a window with the shared header, the snap button, the tools and a window menu (Edit, Clip, Select, Snap, Zoom, Tracks, View, Markers). Still missing: the window play button, the placing selector, the time signature item in the Markers submenu, the Tracks submenu's height items, the rest of the View submenu (clip list options, song overview, clip shadows), and Detach. File: `crates/musiccraft-ui/src/playlist.rs` (`window_menu`, `tools`).
2. **Clip details.** Clips have the strong title strip with the kind glyph, a dark tinted body, a light waveform and a filled automation curve, and the glyph and name stay in view while scrolling. Pattern repeats get a tick and automation points show as dots. Still missing: the glyph does not open the clip's menu. File: `crates/musiccraft-ui/src/playlist.rs` (`draw_clip`).
3. **Track height and track headers.** The default track height (`window-layout.md`, "Playlist tracks"; `PlaylistState::row_height`) is about one and a half tool buttons, much shorter than the reference's default, and too short for the two-line header; the default should rise (proposed) and the compact variant be added. The name is centred vertically. The arm dot, the "S" button and the mute dot sit in a row at the right edge. The colour is a thin stripe at the left, not a tinted plate. The column is flat `theme::PANEL` with only a separator line between rows. Each track has its own height (`playlist.set_track_height`), set by dragging the plate's bottom edge or from the menu's Height ▸, and Tracks ▸ "Reset all track heights" exists; there is no drawn resize grip, no Lock height, no selector strip, and no hairline gap between plates. The right-click menu has only Rename, colours, No colour and Height. Files: `crates/musiccraft-ui/src/playlist.rs` (`headers`, `PlaylistState::row_height`), `docs/spec/playlist.md` (the row-header sentence in "Behavior"), `docs/spec/window-layout.md`.
4. **Scroll and zoom.** The shared scroll/zoom row (chevrons, song overview, zoom grips), the vertical bar and the track-height zoom box are in (`crates/musiccraft-ui/src/scrollbars.rs`). Still missing: the bar-range readout while dragging a grip, right-drag two-way scrolling and the thin-bars setting. The ruler and track headers are painted inside the scrolled area at the clip rect's edges, so they are not real fixed panes. File: `crates/musiccraft-ui/src/playlist.rs` (`canvas`, `bars_around`).
5. **Clip list.** The kind switches are glyphs and the headings are gone. Audio and automation items are text rows ("♪ name", "~ name"), not miniatures. The pattern list has a fixed maximum height (`max_height` in `pattern_list`), so the rest of the column stays empty. "+ New pattern" is a text button under the patterns. Items cannot be dragged onto the lanes or reordered. There is no list zoom, and the column cannot be resized, hidden or moved to the right. Files: `crates/musiccraft-ui/src/playlist.rs` (`pattern_list`, `pattern_section`, `audio_section`, `automation_section`, `LIST_WIDTH`), `docs/spec/playlist.md` ("Picker (UI)").
6. **Ruler.** It is one band, with beat and bar ticks along its bottom edge and, in song mode, a playhead triangle and line shown stopped or playing. Marker glyphs and names sit on top of the bar numbers in the same band, and all numbers are the same size. Markers cannot be dragged, and double-clicking a marker does nothing. The loop is set with Shift+drag only. Files: `crates/musiccraft-ui/src/playlist.rs` (`draw_ruler`, `ruler`).
7. **Drag-and-drop feedback.** A sound dragged from the browser shows a one-bar ghost clip at the snapped spot and outlines the track header, but the ghost is not yet as long as the sound, and dropping on a track header does nothing. File: `crates/musiccraft-ui/src/playlist.rs` (`drop_ghost`, the browser-drop block).
8. **Tools.** There is no Slip tool, and S is taken globally by `ui.toggle_slide` (`crates/musiccraft-ui/keymap/default.keymap`), so Slip needs the window-local Shift+S (proposed, `keymap.md` first). Right-drag delete, Mute-tool drag, the Slice drag line, the Zoom-tool range drag, Alt to ignore snap and Shift to copy are missing. Clips can be resized only from the right edge. Files: `crates/musiccraft-ui/src/playlist.rs` (`tool_stroke`, `edit`), `crates/musiccraft-ui/src/icons.rs`.
9. **No corner block.** There are no kind switches, no "Colours" option for pattern clips, no fade or gain handles on audio clips, and no in-place automation points. Files: `crates/musiccraft-ui/src/playlist.rs`, `crates/musiccraft-ui/src/automation.rs` (the curve editing exists only in the editor window).
10. **Missing window-local keys.** Ctrl+Up / Ctrl+Down (there is no track selection yet) and Alt+P (the clip list cannot be hidden yet) are unbound. The other keys are in `keymap.md`, "Over the playlist".

## Sources

- **FL Studio online manual, `playlist.htm`**, sections compared:
  - "The Playlist" (introduction): clip tracks that hold any clip kind.
  - "Arrangements": the arrangement selector and its menu, including merging one arrangement into another (listed as later).
  - "Working with Playlist Tracks": track selector, Ctrl+Up/Down; track icons and "Track Mode" (instrument and audio tracks with their input, monitoring and arm controls), checked: the icon slot is listed as later, track kinds as out of scope.
  - "Playlist Controls": header order (menu, snap, tools, play, title with arrangement selector, clip source selector, window controls); the snap control as an icon in the header; tool list and their click, drag and right-click behaviour; context tool menu (checked, not adopted); play position marker; timeline selection by Ctrl+drag; horizontal zoom by thumb edges and Ctrl+wheel; the zoom presets on Shift+1 to Shift+5; vertical zoom by the track-height control and dividers; Ctrl+right-click to fit; Clip Focus selector and its option row; mute/solo switch gestures and the lock; Shift+wheel reordering; the divider.
  - "Playlist Track Header Right-Click Menu": rename, colour, icon, auto-name, reset, size and lock size, group, mute or unmute all clips, merge, consolidate, insert, delete, clone, move.
  - "Timeline, Time Markers & Time Signatures": bar numbers, marker kinds, dragging markers, double-click to select up to the next marker, marker menu, song loop marker with a down arrow.
  - "Patterns, Audio Clips and Automation Clips": "Playlist Clip Focus", "Picker Panel" (filter switches, sorting including by mixer track for audio, select unused, click or drag to place, multi-select, scroll bar with zoom at its top, preview by Alt+right-click hold, reorder, show on right side, width), "Project Picker" (checked, not adopted).
  - "Working with Clips" and "Clip Menus": menu glyph at the clip's top-left, menu contents, selecting, moving, copying with Shift, Ctrl+B, trimming from the left as an option, slip editing (Shift+S), names in the title bar, and the option to keep names in view.
  - "Playlist Menu": the Edit, Tools, View, Snap, Select, Zoom and Time markers submenus, used as the outline of A12. The View menu's choice of clip background styles is checked and not adopted (one body style, our own); its clip shadow option is kept as "Clip shadows".
  - "Playlist keyboard shortcuts": tool keys and Page Up / Page Down zoom.
- **`playlist_patterns.htm`**: pattern clips show notes and events together, and the colour can come from note, channel or pattern.
- **`playlist_audioclip.htm`**: fade handles on the upper edge, gain on the lower edge, edge trimming, a header drop naming the track after the file, and double-click to open settings.
- **`playlist_automationclip.htm`**: curve with points and tension handles, editing gestures in the clip, resizing from the title bar, and step and slide options.
- **`playlist_performance.htm`**: checked. Performance mode only adds a zone before the start marker and progress indicators. Not adopted.
- **Manual screenshots looked at** (layout, proportions and density only; no colours sampled, nothing traced or measured): `img_shot/playlist_main.png`, `playlist_trackheaders.png`, `playlist_pickerpanel.png`, `playlist_focus.png`, `playlist_timesignatures.png`, `clips_menus.png`, `basics_interface.png`, `playlist_contextmenu.png`, `playlisttrack_name-color.png`, `playlist_reccontrols.png`, `playlist_pattern.png`, `playlist_automation_edit.png`, `playlist_audioclipenvelopes.png`. `playlist_main.png` and `playlist_trackheaders.png` were re-checked for this final pass (snap as an icon button; a default track several tool buttons tall; the track menu's icon item).
- **Repository**: `crates/musiccraft-ui/src/playlist.rs`, `crates/musiccraft-ui/src/automation.rs`, `crates/musiccraft-ui/src/icons.rs`, `crates/musiccraft-ui/src/lib.rs`, `crates/musiccraft-ui/src/chrome.rs`, `crates/musiccraft-editors/src/theme.rs`, `crates/musiccraft-editors/src/widgets.rs`, `crates/musiccraft-ui/src/geometry.rs`, `crates/musiccraft-ui/keymap/default.keymap`, `docs/spec/playlist.md`, `docs/spec/window-layout.md`, `docs/spec/keymap.md`, `docs/clean-room.md` (owner's decision 2026-10-21), `docs/screenshots/playlist-mixer.png`. The shared header, scroll bars, snap glyph, button colour rule and tokens follow the main-window, toolbar, design-language, piano-roll and dialogs sections.
- **Own design**:
  - every glyph (snap glyph from the toolbar section, kind glyphs, Slip, grip, light, fade grip, diamond tension handle);
  - every token name and tone relation, including `theme::GRID_GROUP`, `theme::TRACK_TINT` and the clip tokens;
  - all wording, including the proposed renames;
  - the "Placing" selector name and layout, and the snap value text beside its glyph;
  - the "in front" adaptation of clip focus for non-overlapping tracks;
  - the "Colours ▾" picker in place of three text toggles;
  - one light for mute and solo, with the solo ring, the lock bar and the "S" badge;
  - the compact track-header variant for short tracks;
  - one neutral lane tone (no per-track lane tint);
  - one two-band ruler shared by the playlist and the piano roll;
  - pattern-repeat ticks, the song-end strip and sticky names;
  - Ctrl+drag for the time selection, with Shift+drag kept;
  - the time selection not clearing on Esc;
  - the reduced-strength playhead in pattern mode;
  - auto-scroll while dragging;
  - double-click on the clip-list divider;
  - the fold order of the header;
  - Shift+S for Slip and every other window-local key placement;
  - default sizes (window-layout.md).

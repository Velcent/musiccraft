# Style guide: Main window and workspace

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The main window is the frame that holds everything else. It has three parts: a toolbar band across the top, a narrow browser column down the left, and a workspace that fills the rest. Editor windows float, overlap, snap together, maximize and detach inside the workspace. Aim for the feel of a dark, compact studio desk. The chrome is quiet, flat and low-contrast and takes as little room as it can. The content inside the windows (clips, notes, steps, meters) is brighter and more saturated than anything around it. Every window shares the same compact chrome: a one-row header with the window menu at the far left, the window's own tools, its title, and small window buttons at the far right. That shared chrome is what makes the different editors read as one application. This section defines that header once; the editor sections list only their own tools, title and right-hand items and refer back here for the rest.

Where this guide describes behaviour our specs do not have yet (new gestures, new shortcuts, a mouse button doing something new), it is marked **Proposed (needs `<spec>.md` updated first)**. Until that spec changes, today's behaviour stays the default. Layout, order and proportions are the guide's target; where they differ from a spec, the gap list names the spec to change first.

## Placement and proportions

**The main window, top to bottom**

| Region | Where | Relative size | Notes |
| --- | --- | --- | --- |
| Toolbar band | Full width, top edge | About two rows of small controls, roughly a twelfth of the window height. Our one-band and two-band rules are in `docs/spec/window-layout.md` | Its contents belong to the toolbar section. Its left end is a block of the menus stacked over the hint bar (project name and the current hint), as `window-layout.md` already says; see the toolbar section. **Proposed (needs `keymap.md` updated first):** Ctrl+F11 hides the band so the workspace takes its height. Ctrl+F11 is unbound today. |
| Browser column | Left edge, from under the toolbar to the bottom edge | A narrow column, about a tenth of the window width. Our opening width is in `window-layout.md` | Resizable from its inner edge. It can move to the right edge, float as its own window, or auto-hide (see D). |
| Workspace | Everything right of the browser and under the toolbar | The rest, roughly nine tenths of the width and eleven twelfths of the height | Editor windows live here. Where no window covers it, the background shows. |

There is no status bar along the bottom. Hints and status messages live in the toolbar's hint bar. The workspace runs to the bottom edge of the main window.

**Default arrangement of the workspace** (the "default layout"; proportions judged by eye against the manual's overview screenshot):

| Window | Position | Share of the workspace |
| --- | --- | --- |
| Channel rack | Top-left corner | About a third of the width. Just under half the height, and its height follows the number of channels. |
| Mixer | Top, to the right of the rack, up to the right edge | The remaining two thirds of the width, about the same height as the rack. This includes the mixer's own effect-slot column at its right end, so the strips take somewhat less than that share. |
| Playlist | Across the whole lower part | Full width, a little more than half the height |
| Piano roll | Floats on top over the right-hand half, covering part of the mixer and the playlist | Roughly two fifths of the width and two fifths of the height |

- The browser is docked as the left column (see D).
- All four editors are open in the default layout.
- Tiled windows sit edge to edge with a narrow gap where the workspace background shows through. They line up because snapping put them there.
- The first time a window opens, it uses its slot in the default layout. After that it reopens where the user left it.

**Window kinds**

1. **Editor windows** (playlist, channel rack, piano roll, mixer). These have the compact header described under Anatomy. Each can move, resize, overlap, collapse to its header, maximize, detach and close. The playlist is a window like the others, not the workspace background.
2. **Secondary windows** (channel settings, effect windows, automation editor, spectrum analyser, keyboard shortcuts). They use the same compact header, with fewer tools, and the same window buttons. Fixed-size ones leave out maximize.
3. **Modal dialogs** (save, export, unsaved changes, settings). These sit centred over the workspace. They have a header with a title and a close cross only, and cannot be detached, collapsed or snapped.
4. **Hosted plugin windows**. These are the plugin's own view in a window we provide. They get our compact header (window menu, preset arrows if any, title, collapse, close) wherever the platform lets us draw one.
5. **Floating hint panel** (optional, later). A larger, floating window that shows the current hint in more room than the hint bar, turned on from the View menu's toolbar group. It uses the same compact header and has no tools.

**Opening, toggling, raising**

- F5 (playlist), F6 (channel rack), F7 (piano roll), F9 (mixer) and Alt+F8 (browser) each toggle their window. The toolbar's window buttons and the View menu do the same. These keys are bound in `default.keymap`, own design for what they do:
  - a hidden window is shown, raised and focused;
  - a window that is visible but not focused is raised and focused;
    - the focused window is hidden.
- Clicking anywhere in a window raises it above the others and focuses it.
- Window keys (all bound except Tab, which is still proposed):
  - Tab moves focus to the next open window in the workspace, in stacking order, and raises it. Tab inside a text field still belongs to the field.
  - Esc closes the focused window, unless a text field, menu or drag inside it takes Esc first.
  - F12 hides every window. Alt+F12 hides only effect and plugin windows (today effect windows; hosted plugin views are not covered yet). Ctrl+F12 hides every window except the focused one. The View menu offers the same three (see Labels; the menu items are still to come).
  - Ctrl+Shift+H puts the windows back in the default layout.
- While something from the browser is being dragged, hovering over a toolbar window button raises that window, so the drag can continue into it.

**Overlap and stacking**: windows overlap freely. The focused window is on top. Detached windows stay above the main window (see Detaching). Dialogs always stay above everything.

**Resizing**

- Every resizable window resizes from all four edges and all four corners. The pointer changes to the matching resize arrow inside a thin band along each edge.
- A window can never be made smaller than its header needs for the menu button, the title and the window buttons.
- The channel rack's height follows its channel count unless the user has set it by hand. The rack's own section has an option for this.
- The mixer stretches in both directions. A taller mixer shows more of each strip, as its own section describes.

**Snapping** (**Proposed (needs `window-layout.md` updated first)**)

- While a window is moved or resized, its edges snap to the nearby edges of other windows when they come close. They also snap to the workspace edges (own extension), so tiles meet exactly.
- Holding Alt while dragging turns snapping off for that drag.
- The snap distance is small, about half the header row's height (own choice).

**Maximize and restore** (**Proposed (needs `window-layout.md` and, for the keys, `keymap.md` updated first)**)

- The maximize button fills the workspace (under the toolbar, beside the browser) and covers the other windows. The button then shows "restore", and pressing it again returns the window to its previous rectangle.
- Enter toggles the playlist between maximized and its previous size. It applies only when no text field, dialog or editor-local action takes it.
- Double-clicking an empty part of a header does the same (own design).
- A maximized window keeps its header, so the window buttons stay reachable.
- When the main window is resized, a maximized window keeps filling the workspace.

**Collapse to header**: the collapse button rolls the window up to its header row and leaves it in place. The window remembers its size and expands again from the same button. Our channel rack, piano roll and mixer already do this from a left-hand arrow (`window-layout.md`, "Window header"); the guide moves the button to the right-hand group and gives it to every window. The manual describes the same behaviour for the browser's minimize button.

**Detaching and multiple screens** (**Proposed (needs `window-layout.md` and `settings.md` updated first)**)

- "Detach from main window" in a window's menu turns it into its own top-level OS window. It keeps the same header and contents, and can be dragged to another monitor.
- Unticking the item puts the window back into the workspace at its last attached rectangle.
- Detached windows stay above the main window. They are not clipped to the workspace or to the main window.
- A setting opens new plugin windows already detached. Another setting makes detached windows open on the same screen as the main window.
- The main window itself can be maximized on one screen, windowed (moved by dragging empty toolbar space and resized from its edges), or spread across all screens. "Bring every window into view" pulls every window back inside the visible workspace, for example after a monitor is unplugged.
- By default, attached windows cannot be dragged fully out of the workspace: at least the header stays inside. An advanced setting removes this limit.

**Touch** (**Proposed (needs `window-layout.md` updated first)**; later, after mouse behaviour is done)

- Two-finger drag on a header moves the window, even over a crowded header: the tools under the fingers are ignored.
- Two-finger tap on a header toggles maximize. Two-finger tap with the fingers spread apart closes the window.
- The header's top-left and top-right corners are enlarged hot spots for tap-and-drag resizing.
- Out of scope for now: a toolbar switch that chooses between touch gestures and touching two controls at once, and a general setting that turns touch support on. List them in the toolbar and settings specs when touch is taken up.

**Remembering size and position**: see H.

**Density and visual hierarchy**

- Each header is a single row about as tall as one tool button. Text is small and the same size throughout the chrome. Padding is minimal, and separators and borders are hairlines.
- Corners are square or barely rounded, and drop shadows are faint or absent, so tiled windows read as one surface and not as a stack of cards.
- From strongest to weakest:
  1. content: clips, notes, steps, meters and waveforms;
  2. things that are on or active, in the accent colour;
  3. labels and titles;
  4. window chrome;
  5. the workspace background, the quietest surface.
- Nothing in the chrome should be brighter than the content it frames. Transient drag feedback (the drag ghost, the snap guide) may use the accent, but stays small and disappears on release.

## Anatomy

### A. Main window, top to bottom

1. **Toolbar band** (contents: see the toolbar section). It spans the full width.
   - Its left end is the menus stacked over the hint bar; the transport, displays and window buttons follow (toolbar section).
   - Empty toolbar space (not on a control) is the handle for moving the main window when it is windowed and has no OS title bar (proposed; platform-dependent).
   - When we draw our own application buttons, they sit at the band's right end: minimize, maximize/restore, close. On macOS they sit at the left end. Right-clicking the application close button opens a small menu: stop playback, minimize, maximize, maximize across all screens, quit.
2. **Body row**, left to right:
   1. **Browser column** (D below).
   2. **Divider**: a hairline between the browser and the workspace, and the browser's resize handle (see D).
   3. **Workspace** (E below), with its editor windows (B, C, F).

### B. Editor window frame

Outside to inside:

1. **Outer edge**: a one-line border all the way round, one step lighter than the window body, so neighbouring windows stay separate across the narrow gap.
2. **Header row** (C below): the full width of the window, at the top. Its fill is the same as the frame's, so header and border read as one piece of chrome.
3. **Body**: the window's content area, inset inside the frame by a hairline margin on the left, right and bottom. Editors lay out their own content here (other sections). Two parts of the body are shared chrome:
   - In time-based editors (playlist, piano roll) the **horizontal scroll and zoom bar** is the first row of the body. It sits directly under the header and above the time ruler, not at the bottom (see F).
   - The **vertical scroll bar** runs down the right edge of the body, under the zoom box (see F).
   - Windows whose content is not on a timeline (channel rack, mixer, browser) put their horizontal scroll bar along the bottom of the scrolling area instead.
4. **Resize band**: a thin invisible band along every edge and corner, where the pointer shows resize arrows. The bottom-right corner may show a small grip mark on hover (own).

### C. The header row, left to right

Every editor window and secondary window has this one row. The order of the slots is fixed. Only the slots' contents change from window to window.

| # | Slot | What it is | Click | Right-click | Other input | States |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Window menu button** | A small square button flush with the top-left corner, smaller than a tool button (see Icons). | Opens the window's menu under the button: the editor's own items (other sections), then a separator, then the common window items (see below). | Same as click | Hover shows "Window menu" in the hint bar. | Idle: dim glyph. Hover: brighter glyph on a raised square. Menu open: pressed look, glyph in the accent. |
| 2 | **Tool groups** | The window's own tools: small square icon buttons in tight groups, a hairline separator between groups. Contents belong to each editor's section. | Per tool | Per tool | Hover → hint bar | Per tool |
| 3 | **Window play button** (only in editors that can play their own content) | A small play glyph just before the title | Plays this window's content | — | Hover → hint | Lit in the accent while that window's content is playing |
| 4 | **Title** | Small text: the window's name, an em dash, then the context (the channel, mixer track or arrangement being shown). It ends in a small right chevron when the title is also a selector. | When it is a selector, opens the list (piano roll: which channel; playlist: which arrangement, once arrangements exist; mixer: none) | — | Proposed: mouse wheel over a selector title steps through the list (own). | Focused window: normal text tone. Unfocused: dimmed. Hover over a selector title: brighter text. |
| 5 | **Drag area** | All empty space left in the row. It stretches. | Click raises and focuses. Drag moves the window (with snapping, once proposed snapping lands). Alt+drag moves it without snapping. | — | Proposed: double-click toggles maximize (own). Touch gestures: see Touch. | — |
| 6 | **Right-hand window items** (optional) | Window-specific items that belong at the far end. Examples: the mixer puts its title here instead of slot 4; the piano roll puts a secondary menu here. | Per item | Per item | — | — |
| 7 | **Collapse** | First of three small window buttons set close together, each smaller than a tool button | Rolls the window up to this header, or expands it again | — | Hover → "Collapse to header" or "Expand" | Glyph flips when collapsed |
| 8 | **Maximize / restore** | Second window button | Fills the workspace, or returns to the previous rectangle | — | Hover → "Maximize" or "Restore" | The glyph shows the opposite of the current state. Left out on fixed-size windows. |
| 9 | **Close** | Third window button, flush with the top-right corner | Hides the window. Its state and position are kept. | — | Hover → "Close (Esc)" once Esc is bound, otherwise "Close" | Hover: the glyph turns the danger tone. |

Placement rules:

- **Title placement**:
  - Playlist, piano roll, channel rack and secondary windows: the title follows the main tool group, a little left of the middle of the header, with any further tools after it.
  - Mixer: its title (`Mixer — <track>`) sits at the right end, just before the window buttons.
  - A window with no tools: the title follows the menu button directly.
- **Too narrow for one row**: the header never wraps onto a second line. Tools that do not fit fold into a "more" chevron, placed just before the right-hand items. The lowest-priority group folds first (own design). The menu button, title and window buttons never fold. The title is cut short with an ellipsis before any of them are dropped.
- **Common window items, appended to every window menu after a separator**:
  - "Detach from main window" (a tick item);
  - "Maximize" or "Restore";
  - "Collapse to header";
  - "Reset position", which returns the window to its slot in the default layout (own).
- Right-clicking a tick item in any menu toggles it without closing the menu, so the user can try settings. **Proposed (needs `controls.md` updated first)**.

### D. Browser column

1. **Header row**, the same chrome as C, left to right:
   - the window menu button;
   - the browser's own small tools: fold all folders, then refresh (contents belong to the browser section);
   - the title "Browser";
   - the drag area, stretching;
   - collapse;
   - close.

   There is no maximize button. **Proposed (needs a browser spec in `window-layout.md` first):** dragging the title moves the browser:
   - drop it on the left or right edge of the main window to dock it there (a docking outline previews the target edge);
   - drop it anywhere else to float it as a detached window.
2. **Body**: the browser's tabs, tree and search (browser section). Its vertical scroll bar is thin and sits along the column's outer edge, the edge away from the workspace.
3. **Divider** on the workspace side.
   - Drag it to change the column width. The workspace and any maximized window follow.
   - Proposed: right-clicking the divider toggles between the current width and a second stored width.
4. **Auto-hide** (a browser menu option, proposed). The column hides, and a thin strip remains at the docked edge. When the pointer reaches the strip, the browser slides out over the workspace without pushing it. When the pointer leaves, the browser slides back.
5. **Collapsed**: when docked, the column shrinks to its header row at the top. When floating, the window rolls up to its header.

### E. Workspace background

- A plain surface, the quietest tone in the interface. It shows only where no window covers it, including the narrow gaps between tiled windows.
- When it is empty, our faint "MusicCraft" mark sits in its bottom-right corner (own; the current watermark). Nothing else is drawn there: no grid and no hints.
- Click: clears focus from all windows. Right-click (own): a small menu listing the editor windows (show/hide) and, for now, the default layout; the full `Windows layout ▸` submenu is still to come. Wheel: nothing.
- An optional user picture or plain colour for this surface may come later. It is user-chosen and stays behind the windows.

### F. Scroll and zoom bars (playlist, piano roll; simplified elsewhere)

**Proposed (needs `playlist.md` and `piano-roll.md` updated first)**; today both use the toolkit's default scroll bars.

Horizontal bar, left to right, as one row the height of a thin header:

1. **Left chevron button**: scrolls left one step per click and repeats while held.
2. **Rail**: the recessed track of the bar.
   - Clicking the rail jumps one page towards the click. With the "incremental scrolling" option it steps there gradually.
   - In the playlist, the rail can draw a miniature overview of the clips (an option).
3. **Thumb** inside the rail. Its length is the visible fraction of the timeline.
   - Left-drag the body to scroll.
   - Right-drag the body to scroll horizontally and vertically together. This applies on the thumb only; right-drag in the piano roll's note area keeps its existing meaning (`piano-roll.md`).
   - Hover brightens it.
4. **Zoom handles**: a small grip at each end of the thumb.
   - Dragging a grip stretches or shrinks the thumb. This zooms the timeline while the opposite end stays put.
   - The pointer becomes a horizontal resize arrow over a grip.
   - While dragging, the hint bar shows the bar range in view.
5. **Right chevron button**: like 1, to the right.
6. **Vertical zoom box**: a small square at the far right end of the row, directly above the vertical scroll bar.
   - Press and drag up or down to change the row height (piano roll keys, playlist tracks).
   - Its glyph shows the current level (see Icons).
   - Hover brightens it. While dragging, the hint bar shows the value.

Vertical bar, down the right edge under the zoom box:

- an up chevron at the top;
- the rail;
- the thumb (proportional to the visible fraction, drag to scroll);
- a down chevron at the bottom.

Other scroll and zoom input:

- Ctrl+wheel zooms horizontally around the pointer (already in `playlist.md` and `piano-roll.md`).
- Proposed: middle-drag anywhere in the content scrolls in both directions. Middle-click on a control keeps its reset meaning (`controls.md`); middle-drag scrolls only on empty content.
- Thickness: a little less than the header row's height. A "thin scroll bars" setting halves it in all editors.

Simplified bars in the channel rack, mixer and browser:

- the rail, the thumb and the end chevrons only;
- no zoom handles and no zoom box;
- at the bottom (horizontal) and right (vertical) of the scrolling area; the browser's vertical bar is on its outer edge (D.2).

### G. Detached windows

- The same header and body as attached windows. Dragging the header moves the OS window.
- A small "return to main window" button appears in slot 6 while the window is detached (own). The window-menu tick item does the same.
- The window has its own OS taskbar entry only if the platform requires it. It closes when the main window closes.

### H. Remembered state

- Each window remembers, per user:
  - its rectangle;
  - collapsed, maximized and detached flags;
  - its screen when detached;
  - whether it is open.

  This survives restarts. On start, windows come back as they were, clamped into the visible screens (own design; the manual documents saved and restored layouts, not per-session memory).
- `Windows layout ▸` (View menu; the same submenu as in the toolbar section):
  - `Default layout (Ctrl+Shift+H)`: the arrangement in "Default arrangement" above;
  - `Second layout`: an alternative built-in arrangement of our own;
  - `My layout`: the user's saved arrangement;
  - `Save as my layout`;
  - `Bring every window into view`.

## Icons

All glyphs are simple vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: one stroke weight, drawn into a rect with an inset margin, coloured by state.

| Where | What it does | Our glyph |
| --- | --- | --- |
| Header slot 1, top-left of every window | Opens the window menu | Three short horizontal bars stacked evenly, the universal menu sign. This is the same glyph as the piano roll's existing options button (☰), so every window matches. |
| Header slot 3 | Plays this window's content | A small filled play triangle pointing right, the same drawing as `Icon::Play` at header size |
| End of a selector title | Shows the title opens a list | A small right-pointing chevron: two short strokes meeting at a point |
| Header slot 7 | Collapse to header / expand | Expanded: an upward chevron over a short horizontal bar ("fold up"). Collapsed: a downward chevron over the bar ("fold down"). |
| Header slot 8 | Maximize / restore | Maximize: four short L-shaped corner brackets at the corners of the glyph box, opening outward. Restore: the same four brackets turned inward, so they point at the centre. |
| Header slot 9 | Close | Two crossing diagonal strokes, the existing `chrome.rs` cross |
| Header "more" fold | Shows tools that did not fit | Two small right chevrons side by side |
| Header slot 6 while detached | Returns the window to the main window | A square outline with a short arrow entering it from the top-right corner, pointing at the centre |
| Scroll bar ends | Step scroll | Small chevrons pointing left / right / up / down, single strokes |
| Thumb ends (zoom grips) | Drag to zoom | Two very short parallel vertical ticks inset at each end of the thumb |
| Vertical zoom box | Drag to change row height | Two horizontal lines inside the small square. The gap between them grows with the zoom: almost touching at the minimum, at the top and bottom edges of the box at the maximum. |
| Resize corner (hover only) | Resize from the corner | Three short diagonal strokes of decreasing length in the bottom-right corner |
| Application buttons (if we draw our own) | Minimize / maximize / close the main window | Minimize: a short horizontal bar low in the box. Maximize: the same four outward corner brackets. Close: the cross. |
| Browser header tools | Fold all / refresh | Fold all: an upward chevron over two short bars. Refresh: an open circle of about three quarters with a small arrowhead on one end. |

## Colour roles

Tokens are in `crates/musiccraft-editors/src/theme.rs`. Names marked "new" are proposals.

| Element | Token(s) | Relative tone and state changes |
| --- | --- | --- |
| Workspace background | `theme::BACKGROUND` | The quietest surface; darker than any window body. It stays the same whether windows are focused or not. |
| Watermark in the empty workspace | new `theme::WATERMARK`: `TEXT` made almost fully transparent (replaces the literal in `theme::watermark`) | Barely visible |
| Toolbar band fill | `theme::PANEL` (see the toolbar section, which owns the band) | The same chrome tone as the window headers; the band's display wells are recessed into it |
| Browser column fill | `theme::PANEL` | Same as window bodies, so the browser reads as part of the desk |
| Browser divider | `theme::PANEL_DARK` hairline | Hover and drag: `theme::ACCENT_DIM` |
| Window outer edge | new `theme::WINDOW_EDGE`, one step lighter than `RAISED` (names the literal in `theme::apply`'s `window_stroke`) | The focused window's edge: `theme::ACCENT_DIM` (own) |
| Header row fill (unfocused) | new `theme::HEADER`, the same as `PANEL` | Part of the frame, matching the edge band |
| Header row fill (focused) | new `theme::HEADER_ACTIVE`: `PANEL` lifted about halfway towards `RAISED` | A just-visible lift; it must not compete with the content |
| Window body behind content | `theme::PANEL`. Content wells (grids, lists) use `PANEL_DARK` or the editors' grid tokens. | Content wells are darker than the frame, and the content on them is brighter than both |
| Title text | `theme::TEXT` (focused window), `theme::TEXT_DIM` (unfocused) | Hover over a selector title: `TEXT` |
| Window menu, collapse, maximize, close glyphs | `theme::TEXT_DIM` | Hover: `TEXT` on a `RAISED_HOVER` square. Menu open: the glyph in `ACCENT` on `RAISED`. |
| Close glyph on hover | `theme::DANGER` | Only while hovered |
| Window play button | `theme::TEXT_DIM` | `ACCENT` while that window's content plays |
| Scroll rail | new `theme::SCROLL_RAIL`, the same as `PANEL_DARK` | Recessed; one step darker than the header |
| Scroll thumb | new `theme::SCROLL_THUMB`, the same as `RAISED` | Hover: `RAISED_HOVER`. Dragging: `ACCENT_DIM`. |
| Zoom grips on the thumb | `theme::TEXT_DIM` | Hover and drag: `ACCENT` |
| Chevron buttons on the scroll bars | `theme::TEXT_DIM` on `SCROLL_RAIL` | Hover: `TEXT` on `RAISED_HOVER`. Held: `ACCENT_DIM` fill. |
| Playlist overview in the rail | Clip colours from `theme::track_color` at reduced opacity | Always dimmer than the real clips |
| Vertical zoom box | `theme::LCD_BG` fill, `theme::LCD_EDGE` border, lines in `TEXT_DIM` | Drag: lines in `ACCENT` |
| Snap guide while dragging | `theme::ACCENT` | A thin line along the edge that will snap. It disappears on release. |
| Docking target (browser drag) | `theme::ACCENT_DIM` translucent fill along the edge, with an `ACCENT` outline | Only while dragging over a valid edge |
| Toolbar window button when a drag hovers it | `theme::ACCENT` outline | Only while something is being dragged |
| Browser drag ghost (label beside the pointer while dragging from the browser) | Today: `theme::ACCENT` fill with `theme::BACKGROUND` text. Proposed: `RAISED` fill, `TEXT` label, `ACCENT` hairline outline | Only while dragging; quieter than the content it is dropped on |
| Inactive windows (optional "translucent inactive windows" setting) | Body and header at reduced opacity | The focused window stays fully opaque |

## Labels and wording

- **Window titles** (the context follows an em dash). Existing labels in our code:
  - `Playlist`
  - `Channel rack`
  - `Piano roll — <channel>`
  - `Mixer — <track>` (`Mixer — Master` when the master is selected)
  - `<channel> — <track>` for channel windows
  - `<effect> — <track>` for effect windows
  - `Automation — <name>`
  - `Spectrum analyser`
  - `Keyboard shortcuts`
  - `Settings`

  Proposed changes (renames of existing labels):
  - `BROWSER` → `Browser`, so every title is sentence case;
  - `Playlist` → `Playlist — <arrangement>`, once arrangements exist.
- **Window button hints** (new):
  - `Window menu`
  - `Collapse to header` / `Expand`
  - `Maximize` / `Restore`
  - `Close (Esc)` (just `Close` until Esc is bound)
  - `Return to main window`
- **Common window-menu items** (new):
  - `Detach from main window`
  - `Maximize` / `Restore`
  - `Collapse to header`
  - `Reset position`
- **View menu, window group**:
  - the existing toggles: `Playlist (F5)`, `Channel rack (F6)`, `Piano roll (F7)`, `Browser (Alt+F8)`, `Mixer (F9)`;
  - then (new) `Hide every window (F12)`, `Hide effect and plugin windows (Alt+F12)`, `Hide all but the focused window (Ctrl+F12)`;
  - `Windows layout ▸` with `Default layout (Ctrl+Shift+H)`, `Second layout`, `My layout`, `Save as my layout`, `Bring every window into view`. The toolbar section uses the same submenu and items.
  - later: `Floating hints` (the optional floating hint panel), under the toolbar group.
- **Browser menu, placement items** (new): `Dock left`, `Dock right`, `Float`, `Auto-hide`.
- **Settings** (new):
  - `Thin scroll bars`
  - `Translucent inactive windows`
  - `Open new plugin windows detached`
  - `Open detached windows on the main window's screen`
  - `Let windows leave the screen`
- **Hint-bar style**, following the existing hints in `playlist.rs`:
  - the name, then the shortcut in parentheses, then a colon and what click, drag and right-click do. Example: `Scroll bar: drag to scroll · drag an end to zoom · right-drag to scroll both ways`;
  - sentence case, no full stop at the end, middle dots between gestures;
  - while dragging, the hint bar shows the live value in place of the description.
- **Number formats while dragging**:
  - horizontal zoom: `Bars 5–12` (1-based bars, en dash);
  - row height: `Row height 140%` (whole percent);
  - window size: not shown;
  - snapping: no text, only the guide line.

## States and feedback

- **Hover**:
  - Chrome buttons get a `RAISED_HOVER` square behind a brightened glyph. Scroll thumbs and zoom grips step up one tone.
  - Edges show the resize pointer only and draw nothing.
  - Every hover writes its hint to the hint bar.
- **Pressed**: the button fill goes to `ACCENT_DIM` while held. Chevrons auto-repeat while held.
- **Menu open**: the window menu button stays pressed with an accent glyph until the menu closes.
- **Focused window**:
  - the header lifts one step (`HEADER_ACTIVE`), the title goes to full text tone, and the edge goes to `ACCENT_DIM`;
  - window-local shortcuts go to this window;
  - only one window is focused at a time. The browser can take focus as well.
- **Unfocused window**: plain header, dimmed title. The content keeps full brightness, because only the chrome dims.
- **Collapsed**: only the header row is drawn and the collapse glyph flips. The window keeps its place in the stack and still snaps.
- **Maximized**: the window covers the workspace exactly, with no gap. The edges no longer resize and the maximize glyph turns to restore. Snapping is off while maximized.
- **Detached**: the same look, plus the "return" button in slot 6. The window menu's tick item is ticked.
- **Dragging a window**: the window follows the pointer. A thin accent snap guide appears on any edge within snap distance, and the window jumps to it on release. Holding Alt suppresses the guide.
- **Resizing**: the opposite edge stays put and the content reflows live.
- **Dragging content over the toolbar's window buttons**: the hovered button gets an accent outline and its window rises to the top.
- **Dragging from the browser**: a small label follows the pointer (the drag ghost, see Colour roles) and disappears on drop.
- **Playing**: the window play button lights in the accent while that window's content plays. The chrome does not change otherwise.
- **Recording**: the chrome does not change. Recording states belong to the toolbar and the editors.
- **Muted, selected**: these belong to the editors' content, not to the chrome.
- **Disabled**: a window button that does not apply (maximize on a fixed-size window) is left out, not greyed out. Menu items that do not apply are greyed out with `TEXT_DIM`.
- **Animation**: collapse, maximize and auto-hide slide quickly, roughly a tenth of a second, or happen at once when the user turns animations off (own).

## MusicCraft today: gaps

Ordered by visual impact. Each needs `docs/spec/window-layout.md` (or the spec named) updated before code.

1. **Scroll bars.**
   - The piano roll and playlist have the shared bars (`crates/musiccraft-ui/src/scrollbars.rs`): the top row with chevrons, a thumb with zoom grips and the vertical zoom box, and the vertical bar. The channel rack, mixer and browser have the simplified bars (`plain_area`).
   - Nowhere is there right-drag two-way scrolling or a thin scroll bar setting.
2. **No window snapping.** Windows neither snap to each other nor to the workspace edges, and there is no Alt override (all `egui::Window` uses).
3. **Window state is forgotten on restart.**
    - eframe's `persistence` feature is off (workspace `Cargo.toml`), so positions, sizes and collapsed flags live only in egui memory for the session (`chrome.rs` stores collapse there).
    - The open-window set is reset by `Windows::default()` in `lib.rs`.
    - There is no `Windows layout ▸` submenu and no saved custom layout.
4. **Missing window shortcuts**: Tab (cycle focus) and Ctrl+F11 (hide toolbar). Enter, Ctrl+Shift+H, Esc, F12, Alt+F12 and Ctrl+F12 are bound. Tab needs care: egui uses it to move keyboard focus between widgets, and a widget holding focus blocks every shortcut, so it has to be taken from the raw input before egui sees it (`crates/musiccraft-app/src/main.rs`). F10 is already bound to `ui.settings`, so no window or project-info shortcut may take F10. `keymap.md` changes first.
5. **Browser chrome and placement** (`crates/musiccraft-ui/src/browser.rs`, `show`):
    - It cannot dock right, float or auto-hide.
    - Its divider has no second stored width.
6. **No detaching or multiple screens.** Editors cannot become their own OS windows (an egui viewport would do it).
    - Hosted plugin views already open in separate OS windows on Linux (`crates/musiccraft-plugin-window`), without our header.
    - There is no "Bring every window into view".
7. **Workspace context.** Right-clicking the empty workspace lists the editor windows and the default layout, but not the rest of the `Windows layout ▸` submenu (second layout, saved layout, bring into view).
8. **No floating hint panel**, and no View-menu toggle for one. Optional; low priority.
9. **Main window chrome.** The app keeps the OS title bar (`crates/musiccraft-app/src/main.rs`, `ViewportBuilder`). It has no own minimize/maximize/close at the toolbar's end, no drag-the-toolbar-to-move, and no "maximize across all screens". This is lowest priority and platform-dependent.
10. **No touch support.** Header gestures and corner hot spots (see Touch) are later work.

## Sources

- **Manual pages compared with** (public online manual, under the owner's 2026-10-21 "layout plus general feel" decision):
  - `basics_interface.htm`:
    - "Controls & Interfaces" (windows snap together; Alt overrides);
    - "Window menus" (a menu button at the top-left of each window);
    - "Menu items" (right-click tick items to try them);
    - "Hint Panel" (the hint bar, and an optional larger floating hint panel turned on from the View menu's toolbar group);
    - "Horizontal Scroll-Zoom Bars & Vertical Zoom" (right-drag two-way scrolling, middle-drag, vertical zoom box, half-width scroll bar option), with its screenshot `img_shot/basics_interface_scrollbars.png`;
    - "Multi-Touch Support" (two-finger drag on the header, two-finger tap to maximize or close, the top-left and top-right resize hot spots, the touch gesture/control switch and the touch setting);
    - "Multi-Screen Support" (detached windows, windowed and multi-monitor main window);
    - the overview screenshot `img_shot/basics_interface.png` (placement and proportions of toolbar, browser, rack, mixer with its effect-slot column, playlist, piano roll; the menus stacked over the hint bar at the band's left end).
  - `menu_view.htm`:
    - "Windows" (window toggles and their keys);
    - "Layout" (hiding all windows, plugin windows, or all but the focused one; arranging windows: default, alternate, custom, save custom, into workspace; background picture or colour). Our menu names and item wording are our own.
  - `basics_shortcuts.htm`: "Window Navigation" (Enter toggles the playlist's size, Esc closes a window, F5–F9, F12, Tab cycles nested windows, Alt+F8, Ctrl+Shift+H arranges windows). What Ctrl+Shift+H restores, and Tab cycling all workspace windows, are our design.
  - `basics_workflow.htm`: "Main Windows" (windows move with overlap, resize, zoom and switch; the rack's height follows its channels).
  - `toolbar_panels.htm`:
    - window controls of the application, including the close button's right-click menu;
    - dragging content over the window buttons raises that window;
    - the toolbar's right-click menu: its hide item (Ctrl+F11) and its hint-panel item.
  - `envsettings_general.htm`:
    - "Display" (transparent inactive windows, animations);
    - "Miscellaneous" (small scroll bars in editors, detach new plugins, open detached plugins with the main window);
    - "Advanced" (windows limited to the screen).
  - `playlist.htm`: menu button and header layout, window controls, detaching, the zoom handle on the scroll bar, the track height control, incremental scrolling, the mini preview in the scroll bar.
  - `pianoroll.htm`: header layout, the horizontal zoom/scroll bar, the vertical zoom box.
  - `channelrack.htm`: header layout, detaching, auto resize, widening from the bottom-right corner.
  - `browser.htm`: "Browser Controls" (menu, fold all, refresh, minimize to the header, close, Alt+F8) and its screenshot `img_shot/browser.png` (header order: menu, the two tools, title, then minimize and close); docking left or right, floating, auto-hide, the divider and its right-click second position.
  - `mixer.htm` and `mixer_mixermenu.htm`: stretching by the edges, detaching, the title at the right end.
  - `plugins/wrapper.htm`: detaching, minimize and close on plugin windows.
- **Repository files**:
  - `docs/clean-room.md`
  - `docs/spec/window-layout.md`
  - `docs/spec/keymap.md`
  - `docs/spec/controls.md` (middle-click reset)
  - `docs/spec/playlist.md`, `docs/spec/piano-roll.md` (Ctrl+scroll zoom; piano roll right-drag)
  - `docs/spec/mixer.md`
  - `docs/spec/settings.md`
  - `crates/musiccraft-ui/src/lib.rs` (`Windows::default()`, `view.toggle`, the drag ghost)
  - `crates/musiccraft-ui/src/chrome.rs`
  - `crates/musiccraft-ui/src/toolbar.rs`
  - `crates/musiccraft-ui/src/icons.rs`
  - `crates/musiccraft-ui/src/browser.rs`
  - `crates/musiccraft-ui/src/playlist.rs`
  - `crates/musiccraft-ui/src/channel_rack.rs`
  - `crates/musiccraft-ui/src/piano_roll.rs`
  - `crates/musiccraft-ui/src/mixer.rs`
  - `crates/musiccraft-ui/src/channel_settings.rs`, `effect_window.rs`, `automation.rs`, `spectrum.rs`, `dialogs.rs`
  - `crates/musiccraft-ui/keymap/default.keymap`
  - `crates/musiccraft-editors/src/theme.rs`
  - `crates/musiccraft-editors/src/hint.rs`
  - `crates/musiccraft-app/src/main.rs`
  - `docs/screenshots/*.png`
- **Own design**:
  - all glyphs: the three-bar menu, the fold chevrons, the corner-bracket maximize and restore, the return-to-main glyph, the zoom grips, the two-line zoom box, the browser tool glyphs;
  - all colour roles and the proposed tokens, including the quieter drag ghost;
  - every label, menu name and hint wording (`Windows layout`, the `Hide …` items, the settings names);
  - focus indication;
  - toggle-raises-before-hiding for F5–F9;
  - double-click to maximize and wheel over a selector title;
  - the header's no-wrap overflow chevron;
  - snapping to workspace edges and the snap guide;
  - right-click menu on the empty workspace;
  - collapse for every window, in the right-hand button group;
  - per-user remembered window state;
  - the "Reset position" item;
  - what the default layout contains and the second built-in layout's contents;
  - animation timing;
  - the watermark.
- **Not used**: no colours were sampled and nothing was traced or measured. The proportions above are rough fractions judged by eye.

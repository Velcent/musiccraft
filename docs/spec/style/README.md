# Spec: interface style guide

How MusicCraft's interface should look and feel, window by window. The aim:
someone who knows the pattern-based DAW described in the public online manual
finds every panel and control where they expect it, at the density they
expect, while everything they see is drawn, coloured and worded by us.

The guide follows the owner's decision of 2026-10-21 in `docs/clean-room.md`
("layout plus general feel").

## What we match and what stays ours

| Matched to the public manual (layout and general feel) | Always our own |
| --- | --- |
| Which panels, windows and controls exist | Every colour: the tokens in `crates/musiccraft-editors/src/theme.rs`, named by role, never by value |
| Where each sits, in what order, and how controls are grouped | Every icon: vector drawings in `crates/musiccraft-ui/src/icons.rs` |
| Relative proportions ("taller than wide", "a narrow column") | Fonts: the faces bundled with egui |
| Density: a dark, compact UI; sizes and spacing relative to each other | The drawing of knobs, faders, buttons, meters and lights |
| What clicking, dragging, right-clicking and scrolling do, and which states exist | Labels, menu wording, hint text, product and plugin names |
| General feel: content brighter than chrome, tools on title rows | Exact sizes: our own choices, kept in `docs/spec/window-layout.md` |

## Contents

Read the first three files before any window file: the others refer to their
tokens, glyphs, control rules and the shared window header.

| File | Covers |
| --- | --- |
| [design-language.md](design-language.md) | Tonal hierarchy and colour roles, type, shape, the icon set and rules for new icons, our names, motion and feedback |
| [controls.md](controls.md) | Knobs, faders, number displays, switches, buttons, menus, text fields, scroll bars, colour picking, automated and linked controls |
| [main-window.md](main-window.md) | Toolbar band, browser column and workspace; the shared window header; default layout; snapping, maximising, detaching |
| [toolbar.md](toolbar.md) | Main menu, hint bar, transport, tempo and time displays, recording toggles, pattern selector, monitors, window buttons |
| [browser.md](browser.md) | The browser column: header, tree, item rows, preview, search, tags, drag and drop |
| [channel-rack.md](channel-rack.md) | The channel rack: header, channel rows, step buttons, graph editor, add button |
| [piano-roll.md](piano-roll.md) | The piano roll: tools, ruler, keyboard, note grid, notes, ghost notes, event lane |
| [playlist.md](playlist.md) | The playlist: tools, clip list, track headers, ruler and markers, clips, zoom |
| [mixer.md](mixer.md) | The mixer: strips, routing, the effect slots panel, EQ, layouts |
| [plugin-windows.md](plugin-windows.md) | Channel settings, instrument and effect windows, hosted plugin windows, presets |
| [dialogs.md](dialogs.md) | Event editor, automation clips, settings, export, project info, message boxes, sample editor frame |

Every window file has the same outline: placement and proportions, anatomy
(every element in order), icons (where, what it does, our glyph), colour
roles, labels and wording, states and feedback, MusicCraft today: gaps, and
Sources.

## How to use it

- **Behaviour specs come first.** Where a guide file describes behaviour, the
  window's own spec (`channel-rack.md`, `piano-roll.md`, `playlist.md`,
  `mixer.md`, `controls.md`, `keymap.md`, …) is authoritative. Anything marked
  **Proposed (needs `<spec>.md` updated first)** is approved as the target
  (owner's decision below): update that spec, then the code
  (`docs/clean-room.md`, Process).
- **Layout is the target.** Where the guide's layout differs from
  `window-layout.md` (for example the shared header: window menu at the far
  left, collapse, maximise and close at the far right), the window file lists
  it under "MusicCraft today: gaps". Change `window-layout.md` in the same
  commit as the code.
- **No numbers.** The guide speaks in proportions and token names. Sizes we
  choose live in `window-layout.md`; colour values live only in `theme.rs`.
  New tokens the guide proposes (by name and relation to existing ones) get
  their values when they are implemented.
- **Gaps.** Each file's gap list was written on 2026-10-21 and is ordered by
  visual impact. Remove a line in the commit that closes it.

## Rules for changing the guide

1. Compare only with the public online manual (its text and screenshots),
   this repository, and our own design.
2. Describe placement, order, grouping, proportions, density, behaviour and
   states.
3. Never write colour values or describe the original's colours; never give
   sizes measured from screenshots; never describe the original's icons,
   logos, skins or fonts; never use its product, plugin or coined names as our
   labels; never copy its wording. Never trace, overlay or colour-sample a
   screenshot.
4. Name the manual page compared with in that file's Sources.
5. When in doubt, differentiate, or ask the owner.

## Owner's decision: follow the manual

On 2026-10-21 the owner decided that wherever the guide offers a choice
between our current layout or behaviour and the manual's, **the manual's
wins**. Where a window file keeps current behaviour "until decided", or marks
something "Proposed", the manual's way is now the target. This settles:

- **Solo** from the mute light (right-click), as the manual does; the
  separate "S" buttons go.
- **Channel rack selection switch** between the name button and the steps.
- **Step buttons**: left-click turns a step on, right-click turns it off.
- **Fine drag** with Ctrl, as the manual does.
- **Window frame**: our own application frame with its own window buttons,
  where the platform allows.
- **Plugin editors** embedded under our window header.
- **Detachable windows** and **native file dialogs**.
- **Toolbar** as the manual's two-row band.
- **Record arm** on mixer strips.
- **Stereo separation** turning right toward mono.

This covers layout and behaviour only. Colours, icons, fonts, the drawing of
every control, labels and names stay our own (`docs/clean-room.md`), so the
proposed label renames that move away from the original's words stand
("Claw every 2nd" → "Keep every 2nd", neutral names for "Mod X / Mod Y", our
own automation curve names, "Pattern / Song", "Clip list"). Each change
still updates its behaviour spec before the code.

## Sources

- Owner's decision of 2026-10-21, `docs/clean-room.md` ("Look and feel").
- The public online manual's pages and screenshots, listed per file in each
  file's Sources; compared for layout, proportions, density and behaviour
  only. No colours were sampled and nothing was traced or measured.
- Owner's decision of 2026-10-21 to follow the manual on every open choice
  of layout and behaviour.
- Own design: the guide's structure, the colour roles, every glyph, every
  label, and the proposed tokens.

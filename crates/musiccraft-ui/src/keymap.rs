//! Keyboard shortcuts as data: a text file mapping key chords to invocations.

use std::collections::BTreeMap;
use std::fmt;

use musiccraft_commands::Invocation;

/// The keymap compiled into the app. Users may override it with their own file.
pub const DEFAULT_KEYMAP: &str = include_str!("../keymap/default.keymap");

/// A key with modifiers, such as `Ctrl+Shift+Z`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Chord {
    /// Ctrl (Cmd on macOS).
    pub ctrl: bool,
    /// Shift.
    pub shift: bool,
    /// Alt / Option.
    pub alt: bool,
    /// egui key name, for example `Space`, `F5`, `Z`.
    pub key: String,
}

impl Chord {
    /// Parses `Ctrl+Shift+Z`-style text. Modifier names are case-insensitive.
    ///
    /// # Errors
    ///
    /// Returns a description of the problem for unknown modifiers or keys.
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        // `Ctrl++` binds the plus key.
        let (mods, key) = match parts.as_slice() {
            [mods @ .., "", ""] => (mods, "Plus"),
            [mods @ .., key] => (mods, *key),
            [] => return Err("empty chord".to_owned()),
        };
        let mut chord = Self {
            ctrl: false,
            shift: false,
            alt: false,
            key: String::new(),
        };
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "command" => chord.ctrl = true,
                "shift" => chord.shift = true,
                "alt" | "option" => chord.alt = true,
                other => return Err(format!("unknown modifier `{other}`")),
            }
        }
        let key = egui::Key::from_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
        chord.key = key.name().to_owned();
        Ok(chord)
    }

    /// Builds a chord from an egui key event.
    #[must_use]
    pub fn from_event(key: egui::Key, modifiers: egui::Modifiers) -> Self {
        Self {
            ctrl: modifiers.command || modifiers.ctrl,
            shift: modifiers.shift,
            alt: modifiers.alt,
            key: key.name().to_owned(),
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        f.write_str(&self.key)
    }
}

/// Chord → invocation bindings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keymap {
    bindings: BTreeMap<Chord, Invocation>,
}

impl Keymap {
    /// Parses keymap text. Blank lines and `#` comments are ignored.
    ///
    /// # Errors
    ///
    /// Returns `line N: problem` for the first malformed line.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut bindings = BTreeMap::new();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fail = |reason: String| format!("line {}: {reason}", number + 1);
            let (chord, action) = line
                .split_once('=')
                .ok_or_else(|| fail("expected `<chord> = <invocation>`".to_owned()))?;
            let chord = Chord::parse(chord.trim()).map_err(fail)?;
            let invocation = Invocation::parse(action).map_err(|e| fail(e.to_string()))?;
            if bindings.insert(chord.clone(), invocation).is_some() {
                return Err(fail(format!("`{chord}` is bound twice")));
            }
        }
        Ok(Self { bindings })
    }

    /// The built-in keymap.
    #[must_use]
    pub fn default_keymap() -> Self {
        Self::parse(DEFAULT_KEYMAP).unwrap_or_default()
    }

    /// The invocation bound to a chord.
    #[must_use]
    pub fn lookup(&self, chord: &Chord) -> Option<&Invocation> {
        self.bindings.get(chord)
    }

    /// The first chord bound to an invocation, for menus and tooltips.
    #[must_use]
    pub fn chord_for(&self, invocation: &str) -> Option<&Chord> {
        self.bindings
            .iter()
            .find(|(_, inv)| inv.to_string() == invocation)
            .map(|(chord, _)| chord)
    }

    /// Every binding in chord order.
    pub fn iter(&self) -> impl Iterator<Item = (&Chord, &Invocation)> {
        self.bindings.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_parses() {
        let keymap = Keymap::parse(DEFAULT_KEYMAP).unwrap();
        let space = Chord::parse("Space").unwrap();
        assert_eq!(keymap.lookup(&space).unwrap().name, "transport.toggle");
        let undo = Chord::parse("ctrl+z").unwrap();
        assert_eq!(keymap.lookup(&undo).unwrap().name, "edit.undo");
        assert_eq!(
            keymap.chord_for("view.toggle mixer").unwrap().to_string(),
            "F9"
        );
        // FL-familiar: R records, Ctrl+N saves a new version.
        let r = Chord::parse("R").unwrap();
        assert_eq!(keymap.lookup(&r).unwrap().name, "record.toggle");
        let ctrl_n = Chord::parse("Ctrl+N").unwrap();
        assert_eq!(keymap.lookup(&ctrl_n).unwrap().name, "ui.save_new_version");
        assert!(keymap.chord_for("ui.new").is_none(), "New is menu-only");
        for (chord, name) in [
            ("Ctrl+E", "record.toggle_step_edit"),
            ("Ctrl+P", "record.toggle_count_in"),
            ("Ctrl+I", "record.toggle_wait"),
        ] {
            let chord = Chord::parse(chord).unwrap();
            assert_eq!(keymap.lookup(&chord).unwrap().name, name);
        }
        // Home jumps to the start, Ctrl+H is panic.
        let home = Chord::parse("Home").unwrap();
        assert_eq!(
            keymap.lookup(&home).unwrap().name,
            "transport.jump_to_start"
        );
        let ctrl_h = Chord::parse("Ctrl+H").unwrap();
        assert_eq!(keymap.lookup(&ctrl_h).unwrap().name, "transport.panic");
    }

    #[test]
    fn chords_normalize() {
        let a = Chord::parse("Shift + Ctrl + s").unwrap();
        let b = Chord::parse("Ctrl+Shift+S").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "Ctrl+Shift+S");
        assert_eq!(Chord::parse("Ctrl++").unwrap().key, "Plus");
        assert_eq!(
            Chord::from_event(egui::Key::Z, egui::Modifiers::COMMAND),
            Chord::parse("Ctrl+Z").unwrap()
        );
    }

    #[test]
    fn errors_name_the_line() {
        assert_eq!(
            Keymap::parse("# ok\nHyper+A = x.y").unwrap_err(),
            "line 2: unknown modifier `hyper`"
        );
        assert!(Keymap::parse("Space transport.toggle").is_err());
        assert!(Keymap::parse("Space = a.b\nSpace = c.d").is_err());
        assert!(Keymap::parse("NotAKey = a.b").is_err());
    }
}

//! The patch being built, with its notes.

use musiccraft_antiserum::params::{SLOTS, index_of, params};
use musiccraft_patch::{Patch, PointData};

use crate::Conversion;

/// A patch under construction.
pub struct Out {
    pub patch: Patch,
    notes: Vec<String>,
    /// Mod matrix slots used so far.
    slots: usize,
}

impl Out {
    pub fn new(name: &str) -> Self {
        Self {
            patch: Patch {
                name: name.to_owned(),
                tags: vec!["serum".to_owned()],
                ..Patch::default()
            },
            notes: Vec::new(),
            slots: 0,
        }
    }

    /// Sets Antiserum parameter `name`, clamped into its range (integers
    /// rounded). An unknown name is a bug in the mapping, so it panics in
    /// debug builds and is skipped otherwise.
    pub fn set(&mut self, name: &str, value: f32) {
        let Some(spec) = index_of(name).and_then(|i| params().get(i)) else {
            debug_assert!(false, "no Antiserum parameter {name}");
            return;
        };
        let value = if value.is_finite() {
            value
        } else {
            spec.default
        };
        let mut value = value.clamp(spec.min, spec.max);
        if spec.integer {
            value = value.round();
        }
        self.patch.params.insert(name.to_owned(), value);
    }

    /// Sets an on/off parameter.
    pub fn flag(&mut self, name: &str, on: bool) {
        self.set(name, f32::from(u8::from(on)));
    }

    /// The value set for `name`, or its default.
    pub fn get(&self, name: &str) -> f32 {
        self.patch.params.get(name).copied().unwrap_or_else(|| {
            index_of(name)
                .and_then(|i| params().get(i))
                .map_or(0.0, |s| s.default)
        })
    }

    /// Adds a note, once.
    pub fn note(&mut self, text: impl Into<String>) {
        let text = text.into();
        if !self.notes.contains(&text) {
            self.notes.push(text);
        }
    }

    /// Sets oscillator `osc`'s (0-based) table.
    pub fn table(&mut self, osc: usize, table: String) {
        self.patch.set_table(osc, table);
    }

    /// Sets LFO `lfo`'s (0-based) drawn shape.
    pub fn lfo_shape(&mut self, lfo: usize, points: Vec<PointData>) {
        self.patch.set_lfo_shape(lfo, points);
    }

    /// Adds a mod matrix routing from source `source` (an index into
    /// Antiserum's sources) to parameter `dest`. `amount` is a fraction of
    /// the destination's range, −1..1 (clamped, with a note).
    pub fn route(&mut self, source: usize, aux: usize, dest: &str, amount: f32, bipolar: bool) {
        let Some(index) = index_of(dest) else {
            debug_assert!(false, "no Antiserum parameter {dest}");
            return;
        };
        if self.slots >= SLOTS {
            self.note(format!(
                "dropped: more than {SLOTS} modulation routings; the rest were left out"
            ));
            return;
        }
        if amount.abs() > 1.0 {
            self.note(format!(
                "approximated: a modulation of {dest} is deeper than Antiserum's range; clamped"
            ));
        }
        self.slots += 1;
        let n = self.slots;
        self.set(&format!("mod{n}_source"), source as f32);
        self.set(&format!("mod{n}_dest"), (index + 1) as f32);
        self.set(&format!("mod{n}_amount"), amount);
        self.flag(&format!("mod{n}_bipolar"), bipolar);
        self.set(&format!("mod{n}_aux"), aux as f32);
    }

    pub fn finish(self) -> Conversion {
        Conversion {
            patch: self.patch,
            notes: self.notes,
        }
    }
}

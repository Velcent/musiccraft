//! Antiserum's patch: everything that makes one of its sounds, as MusicCraft's
//! own JSON (`docs/spec/antiserum-app.md`). Plain data with no knowledge of
//! the synth; `musiccraft-antiserum-presets` applies a patch to a synth.

pub mod base64;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Wavetable oscillators in a patch.
pub const TABLES: usize = 3;
/// Drawn LFO shapes a patch can hold.
pub const LFOS: usize = 8;
/// Sequencer steps a patch can hold.
pub const STEPS: usize = 64;
/// The table an oscillator plays when a patch names none.
pub const DEFAULT_TABLE: &str = "factory:basic";

/// Preset categories, in menu order.
pub const CATEGORIES: [&str; 7] = ["bass", "lead", "pad", "pluck", "keys", "fx", "drums"];

/// One sequencer step: on, transpose (semitones), velocity, gate.
pub type StepData = (bool, i8, f32, f32);

/// One drawn LFO point: x, y, curve.
pub type PointData = [f32; 3];

/// A synth sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// One of [`CATEGORIES`], or empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    /// Search tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Parameter values by name; missing ones take their defaults.
    #[serde(default)]
    pub params: BTreeMap<String, f32>,
    /// One table per oscillator: `factory:<name>` or `data:<base64>`.
    #[serde(default = "default_tables")]
    pub tables: Vec<String>,
    /// Drawn LFO shapes by LFO number ("1"–"8").
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lfo_shapes: BTreeMap<String, Vec<PointData>>,
    /// Sequencer steps; missing ones keep their defaults.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence: Vec<StepData>,
    /// Each oscillator's sample and multisample zones (see
    /// `docs/spec/antiserum-oscillators.md`); missing ones are empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceRef>,
}

/// An oscillator's sample and zones, by file path or `builtin:` name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// The sample for the sample, granular and spectral types (empty: none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sample: String,
    /// The multisample zones, in the order notes look for a match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<ZoneRef>,
}

/// One multisample zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneRef {
    /// File path or `builtin:` name.
    pub sample: String,
    /// Lowest key it plays.
    #[serde(default)]
    pub low: u8,
    /// Highest key it plays.
    #[serde(default = "highest_key")]
    pub high: u8,
    /// The key at which it plays at its own speed.
    #[serde(default = "middle_c")]
    pub root: u8,
    /// Lowest velocity it plays (1–127).
    #[serde(default = "lowest_velocity")]
    pub vel_low: u8,
    /// Highest velocity it plays.
    #[serde(default = "highest_key")]
    pub vel_high: u8,
}

fn highest_key() -> u8 {
    127
}

fn middle_c() -> u8 {
    60
}

fn lowest_velocity() -> u8 {
    1
}

impl ZoneRef {
    /// A zone over keys `low..=high` with root `root`, every velocity.
    #[must_use]
    pub fn new(sample: String, low: u8, high: u8, root: u8) -> Self {
        Self {
            sample,
            low,
            high,
            root,
            vel_low: 1,
            vel_high: 127,
        }
    }
}

fn default_tables() -> Vec<String> {
    vec![DEFAULT_TABLE.to_owned(); TABLES]
}

impl Default for Patch {
    /// The init patch: every parameter at its default, basic tables.
    fn default() -> Self {
        Self {
            name: "Init".to_owned(),
            category: String::new(),
            tags: Vec::new(),
            params: BTreeMap::new(),
            tables: default_tables(),
            lfo_shapes: BTreeMap::new(),
            sequence: Vec::new(),
            sources: Vec::new(),
        }
    }
}

impl Patch {
    /// Table `osc` (0-based), or the default.
    #[must_use]
    pub fn table(&self, osc: usize) -> &str {
        self.tables.get(osc).map_or(DEFAULT_TABLE, String::as_str)
    }

    /// Sets table `osc` (0-based), padding the list with defaults.
    pub fn set_table(&mut self, osc: usize, table: String) {
        while self.tables.len() <= osc {
            self.tables.push(DEFAULT_TABLE.to_owned());
        }
        if let Some(slot) = self.tables.get_mut(osc) {
            *slot = table;
        }
    }

    /// LFO `lfo`'s (0-based) drawn shape, if it has one.
    #[must_use]
    pub fn lfo_shape(&self, lfo: usize) -> Option<&[PointData]> {
        self.lfo_shapes
            .get(&(lfo + 1).to_string())
            .map(Vec::as_slice)
    }

    /// Sets (or with an empty list, clears) LFO `lfo`'s (0-based) shape.
    pub fn set_lfo_shape(&mut self, lfo: usize, points: Vec<PointData>) {
        let key = (lfo + 1).to_string();
        if points.is_empty() {
            self.lfo_shapes.remove(&key);
        } else {
            self.lfo_shapes.insert(key, points);
        }
    }

    /// Step `step` (0-based), if the patch sets it.
    #[must_use]
    pub fn step(&self, step: usize) -> Option<StepData> {
        self.sequence.get(step).copied()
    }

    /// Sets step `step` (0-based), filling earlier steps with `default`.
    /// Trailing default steps are dropped, so a patch stays as short as it
    /// can be and undoing a change restores it exactly.
    pub fn set_step(&mut self, step: usize, data: StepData, default: StepData) {
        if step >= STEPS {
            return;
        }
        while self.sequence.len() <= step {
            self.sequence.push(default);
        }
        if let Some(slot) = self.sequence.get_mut(step) {
            *slot = data;
        }
        while self.sequence.last() == Some(&default) {
            self.sequence.pop();
        }
    }

    /// Oscillator `osc`'s (0-based) source (empty if the patch has none).
    #[must_use]
    pub fn source(&self, osc: usize) -> SourceRef {
        self.sources.get(osc).cloned().unwrap_or_default()
    }

    /// Replaces oscillator `osc`'s source. Trailing empty sources are
    /// dropped, so undoing a change restores the patch exactly.
    pub fn set_source(&mut self, osc: usize, source: SourceRef) {
        if osc >= TABLES {
            return;
        }
        while self.sources.len() <= osc {
            self.sources.push(SourceRef::default());
        }
        if let Some(slot) = self.sources.get_mut(osc) {
            *slot = source;
        }
        while self.sources.last() == Some(&SourceRef::default()) {
            self.sources.pop();
        }
    }

    /// The patch as pretty JSON.
    ///
    /// # Errors
    ///
    /// Only if serialization fails (non-finite numbers are written as null).
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// A patch from JSON.
    ///
    /// # Errors
    ///
    /// The text is not a patch.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("not an Antiserum patch: {e}"))
    }
}

/// A table reference, parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum TableRef {
    /// A generated factory table by name.
    Factory(String),
    /// Frames end to end.
    Data(Vec<f32>),
}

impl TableRef {
    /// Parses `factory:<name>` or `data:<base64>`.
    ///
    /// # Errors
    ///
    /// An unknown prefix or bad base64 (or a byte count that is not a whole
    /// number of floats).
    pub fn parse(text: &str) -> Result<Self, String> {
        if let Some(name) = text.strip_prefix("factory:") {
            return Ok(Self::Factory(name.to_owned()));
        }
        let data = text.strip_prefix("data:").ok_or_else(|| {
            format!("a table is `factory:<name>` or `data:<base64>`, not `{text}`")
        })?;
        let bytes = base64::decode(data).ok_or("the table data is not valid base64")?;
        if bytes.len() % 4 != 0 {
            return Err("the table data is not a list of 32-bit floats".to_owned());
        }
        Ok(Self::Data(
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .map(|x| if x.is_finite() { x } else { 0.0 })
                .collect(),
        ))
    }

    /// The reference as text.
    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Self::Factory(name) => format!("factory:{name}"),
            Self::Data(samples) => {
                let bytes: Vec<u8> = samples.iter().flat_map(|x| x.to_le_bytes()).collect();
                format!("data:{}", base64::encode(&bytes))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_round_trips_through_json() {
        let mut patch = Patch {
            name: "Test".to_owned(),
            category: "bass".to_owned(),
            tags: vec!["dark".to_owned()],
            ..Patch::default()
        };
        patch.params.insert("osc1_unison".to_owned(), 7.0);
        patch.set_table(2, TableRef::Data(vec![0.5, -0.25]).to_text());
        patch.set_lfo_shape(0, vec![[0.0, 0.0, 0.0], [0.5, 1.0, -0.4]]);
        patch.set_step(2, (false, 12, 0.5, 1.0), (true, 0, 1.0, 0.5));
        let json = patch.to_json().unwrap();
        assert_eq!(Patch::from_json(&json).unwrap(), patch);
        assert_eq!(patch.sequence.len(), 3);
        assert_eq!(patch.step(0), Some((true, 0, 1.0, 0.5)));
        assert_eq!(patch.lfo_shape(0).map(<[_]>::len), Some(2));
        assert_eq!(patch.lfo_shape(1), None);
        assert_eq!(
            TableRef::parse(patch.table(2)).unwrap(),
            TableRef::Data(vec![0.5, -0.25])
        );
    }

    #[test]
    fn sources_round_trip_and_stay_short() {
        let mut patch = Patch::default();
        patch.set_source(
            2,
            SourceRef {
                sample: String::new(),
                zones: vec![ZoneRef::new("a.wav".to_owned(), 0, 59, 48)],
            },
        );
        assert_eq!(patch.sources.len(), 3);
        let json = patch.to_json().unwrap();
        assert_eq!(Patch::from_json(&json).unwrap(), patch);
        // A zone written with only its sample takes the defaults.
        let read: SourceRef = serde_json::from_str(r#"{"zones":[{"sample":"b.wav"}]}"#).unwrap();
        assert_eq!(read.zones[0], ZoneRef::new("b.wav".to_owned(), 0, 127, 60));
        patch.set_source(2, SourceRef::default());
        assert!(patch.sources.is_empty());
        assert_eq!(patch.source(1), SourceRef::default());
    }

    #[test]
    fn missing_fields_take_defaults() {
        let patch = Patch::from_json("{}").unwrap();
        assert_eq!(patch.tables, default_tables());
        assert_eq!(patch.table(5), DEFAULT_TABLE);
        assert!(Patch::from_json("[1]").is_err());
    }

    #[test]
    fn table_refs_are_checked() {
        assert_eq!(
            TableRef::parse("factory:pwm").unwrap(),
            TableRef::Factory("pwm".to_owned())
        );
        assert!(TableRef::parse("file:x.wav").is_err());
        assert!(TableRef::parse("data:!!").is_err());
        assert!(TableRef::parse("data:AAA=").is_err(), "two bytes");
    }
}

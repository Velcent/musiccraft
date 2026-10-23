// Ported from serum2vital's `serum1.py`
// (https://github.com/btesser/serum2vital), Copyright (C) 2026 Ben Tesser,
// GPL-3.0-or-later; see `lib.rs`.

//! Serum 1 presets (VST2 `.fxp`): a 60-byte FXP header (`CcnK`, `FPCh`,
//! plugin id `XfsX`) around a zlib-compressed state blob. Offsets inside the
//! blob are serum2vital's (its `docs/FORMATS.md`):
//!
//! ```text
//! 0x0000  modulation slots 1..16 (16 records of 40 bytes)
//! 0x0280  LFO 1-4 shapes (classic layout)     0x1AE0  LFO 1-4 switches
//! 0x1B70  LFO 5-8 shapes (classic layout)     0x6DB8  LFO 5-8 switches
//! 0x3460  parameters 0..227 (f32, 0..1)       0x3BE0  FX rack order
//! 0x3C08  oscillator A table name             0x3E08  oscillator B table name
//! 0x4008  noise sample name                   0x4972  preset name
//! 0x49A0  author   0x49D0  bank               0x4A60  macro names
//! 0x4AE0  parameters 228..298                 varies  global switches
//! 0x84D8  LFO 1-8 blocks (new layout)         varies  modulation slots 17..32
//! ```

use crate::Error;
use crate::params::{LOW_BLOCK, PARAMS, stored_default};

const FXP_HEADER: usize = 60;
const PLUGIN_ID: &[u8; 4] = b"XfsX";

const OFF_PARAMS_LOW: usize = 0x3460;
const OFF_PARAMS_HIGH: usize = 0x4AE0;
const OFF_FX_ORDER: usize = 0x3BE0;
const OFF_WT_A: usize = 0x3C08;
const OFF_WT_B: usize = 0x3E08;
const OFF_NOISE: usize = 0x4008;
const OFF_PRESET_NAME: usize = 0x4972;
const OFF_AUTHOR: usize = 0x49A0;
const OFF_MENU: usize = 0x49D0;
const OFF_MACRO_NAMES: usize = 0x4A60;
const MACRO_NAME_STRIDE: usize = 0x20;

const CLASSIC_BLOCKS: [usize; 2] = [0x0280, 0x1B70];
const CLASSIC_ARRAY_STRIDE: usize = 520;
const CLASSIC_ARRAY_LEN: usize = 65;
const CLASSIC_SETTINGS: [usize; 2] = [0x1AE0, 0x6DB8];
const CLASSIC_SETTINGS_FLAGS: usize = 0x14;
const CLASSIC_SETTINGS_END: usize = 0x2C;

const NEW_LFO_BASE: usize = 0x84D8;
const NEW_LFO_STRIDE: usize = 0x2D28;
const NEW_ARRAY_LEN: usize = 480;
const NEW_OFF_X: usize = 0xF08;
const NEW_OFF_Y: usize = 0x1E08;
const NEW_OFF_FLAGS: usize = 0x2D08;

/// Starts of the global switches block seen in the library, current builds
/// first; the block follows the parameter array, whose length depends on the
/// build that wrote the file.
const SETTINGS_BASES: [usize; 3] = [0x4C48, 0x4C44, 0x4B9C];
const SETTINGS_SIZE: usize = 0x64;
const SET_TUNING_REF: usize = 0x00;
const SET_UNISON_TUNING: [usize; 2] = [0x08, 0x0C];
const SET_MONO: usize = 0x10;
const SET_LEGATO: usize = 0x14;
const SET_PORTA_ALWAYS: usize = 0x18;
const SET_PORTA_SCALED: usize = 0x1C;
const SET_NOISE_ONE_SHOT: usize = 0x24;
const SET_NOISE_PITCH_TRACK: usize = 0x28;
const SET_POLYPHONY: usize = 0x2C;
const SET_FILTER_KEYTRACK: usize = 0x34;
const SET_UNISON_RANGE: [usize; 2] = [0x38, 0x3C];
const SET_CHORUS_MONO: usize = 0x48;
const SET_REVERB_HALL: usize = 0x5C;
/// A copy of the reverb's Plate/Hall byte, used when the block is absent.
const OFF_REVERB_HALL: usize = 0x3B04;

/// Unison tuning menu (switches block, `index / 4`).
pub const UNISON_TUNINGS: [&str; 5] = ["Linear", "Super", "Exp", "Inv", "Random"];

const MOD_RECORD_SIZE: usize = 40;
const MOD_MARKER_OFFSET: usize = 0x21;
const MOD_OFF_AMOUNT: usize = 0x04;
const MOD_OFF_OUT: usize = 0x08;
const MOD_OFF_TYPE: usize = 0x0C;
const MOD_OFF_SOURCE: usize = 0x14;
const MOD_OFF_AUX: usize = 0x16;
const MOD_OFF_DEST: usize = 0x1A;

/// Effects in the order of their enable parameters and of the FX order block.
pub const FX_ORDER_NAMES: [&str; 10] = [
    "distortion",
    "flanger",
    "phaser",
    "chorus",
    "delay",
    "compressor",
    "reverb",
    "eq",
    "filter",
    "hyper",
];

/// Serum's modulation source ids (0: unused slot).
#[must_use]
pub fn source_name(id: u16) -> Option<&'static str> {
    Some(match id {
        1 => "mod_wheel",
        2 => "env_1",
        3 => "env_2",
        4 => "env_3",
        5 => "lfo_1",
        6 => "lfo_2",
        7 => "lfo_3",
        8 => "lfo_4",
        9 => "lfo_5",
        10 => "lfo_6",
        11 => "lfo_7",
        12 => "lfo_8",
        13 => "velocity",
        14 => "note",
        15 => "aftertouch",
        16 => "poly_aftertouch",
        17 => "chaos_1",
        18 => "chaos_2",
        19 => "noise_osc",
        20 => "note_random_1",
        21 => "note_random_2",
        22 => "note_alt_1",
        23 => "note_alt_2",
        24 => "macro_1",
        25 => "macro_2",
        26 => "macro_3",
        27 => "macro_4",
        28 => "pitch_bend",
        29 => "mpe_x",
        30 => "mpe_y",
        31 => "mpe_z",
        32 => "release_velocity",
        33 => "fixed",
        _ => return None,
    })
}

/// One modulation matrix slot.
#[derive(Debug, Clone, PartialEq)]
pub struct ModSlot {
    /// 1-based slot number.
    pub slot: u8,
    /// Source id (see [`source_name`]).
    pub source: u16,
    /// Aux source id (0: none).
    pub aux: u16,
    /// Destination: an index into [`PARAMS`].
    pub dest: u16,
    /// Amount, −1..1.
    pub amount: f32,
    /// Output range (1.0 is 100 %).
    pub out_range: f32,
    /// The matrix type column: the source swings around the knob.
    pub bipolar: bool,
}

impl ModSlot {
    /// Whether the slot does anything.
    #[must_use]
    pub fn active(&self) -> bool {
        self.amount.abs() > 1e-6 && self.source != 0
    }
}

/// An LFO's panel switches that are not VST parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoSettings {
    /// BPM switch off: the rate knob is in Hz.
    pub hz_mode: bool,
    pub dotted: bool,
    pub triplet: bool,
    pub anchor: bool,
    /// `"trig"`, `"env"` or `"off"` (free running).
    pub mode: &'static str,
    /// False when the file does not store the switches.
    pub known: bool,
}

impl Default for LfoSettings {
    fn default() -> Self {
        Self {
            hz_mode: false,
            dotted: false,
            triplet: false,
            anchor: true,
            mode: "off",
            known: true,
        }
    }
}

impl LfoSettings {
    fn unknown() -> Self {
        Self {
            known: false,
            ..Self::default()
        }
    }

    /// From the six flag bytes: anchor, Hz, dotted, triplet, not-off, env.
    fn from_flags(flags: [u8; 6]) -> Self {
        let [anchor, hz, dot, trip, not_off, env] = flags;
        let mode = match (not_off, env) {
            (1, 1) => "env",
            (1, _) => "trig",
            _ => "off",
        };
        Self {
            hz_mode: hz == 1,
            dotted: dot == 1,
            triplet: trip == 1,
            anchor: anchor == 1,
            mode,
            known: true,
        }
    }
}

/// A Serum LFO curve: points in 0..1 with a tension per segment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LfoShape {
    /// 0..1, non-decreasing.
    pub xs: Vec<f64>,
    /// 0..1, 0 at the top of the display.
    pub ys: Vec<f64>,
    /// 0.5 is a straight segment.
    pub curves: Vec<f64>,
    pub settings: LfoSettings,
}

/// The switches that are not VST parameters: voicing, noise, filter, unison.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalSettings {
    pub mono: bool,
    pub legato: bool,
    pub polyphony: u8,
    pub porta_always: bool,
    pub porta_scaled: bool,
    pub noise_one_shot: bool,
    pub noise_pitch_track: bool,
    pub filter_keytrack: bool,
    /// Semitones, oscillator A and B.
    pub unison_range: [f32; 2],
    /// Names from [`UNISON_TUNINGS`].
    pub unison_tuning: [&'static str; 2],
    pub reverb_hall: bool,
    /// The tuning reference, 430..450 Hz.
    pub a4_hz: f32,
    pub chorus_mono: bool,
    /// False when the block could not be located.
    pub known: bool,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            mono: false,
            legato: false,
            polyphony: 8,
            porta_always: false,
            porta_scaled: false,
            noise_one_shot: false,
            noise_pitch_track: false,
            filter_keytrack: false,
            unison_range: [2.0, 2.0],
            unison_tuning: ["Linear", "Linear"],
            reverb_hall: true,
            a4_hz: 440.0,
            chorus_mono: false,
            known: true,
        }
    }
}

/// A parsed Serum 1 preset.
#[derive(Debug, Clone, PartialEq)]
pub struct Serum1Patch {
    pub name: String,
    pub author: String,
    /// The bank (Serum's "menu") text.
    pub menu: String,
    pub macro_names: [String; 4],
    /// 299 values, normalised 0..1.
    pub params: Vec<f32>,
    pub wavetable_a: String,
    pub wavetable_b: String,
    pub noise_sample: String,
    pub mod_slots: Vec<ModSlot>,
    /// Eight entries.
    pub lfo_shapes: Vec<LfoShape>,
    /// Rack position per [`FX_ORDER_NAMES`] entry.
    pub fx_order: Option<[i32; 10]>,
    pub settings: GlobalSettings,
}

impl Serum1Patch {
    /// The stored (0..1) value of the parameter called `name`.
    #[must_use]
    pub fn value(&self, name: &str) -> f32 {
        crate::params::index(name)
            .and_then(|i| self.params.get(i).copied())
            .unwrap_or(0.0)
    }

    /// The parameter called `name` as Serum displays it.
    #[must_use]
    pub fn display(&self, name: &str) -> f32 {
        crate::params::index(name).map_or(0.0, |i| {
            crate::params::display(i, self.params.get(i).copied().unwrap_or(0.0))
        })
    }

    /// Whether the switch parameter called `name` is on.
    #[must_use]
    pub fn on(&self, name: &str) -> bool {
        self.value(name) > 0.5
    }

    /// Effect names in rack order, top first.
    #[must_use]
    pub fn fx_rack(&self) -> Vec<&'static str> {
        match self.fx_order {
            None => vec![
                "hyper",
                "distortion",
                "flanger",
                "phaser",
                "chorus",
                "delay",
                "compressor",
                "reverb",
                "eq",
                "filter",
            ],
            Some(order) => {
                let mut pairs: Vec<(i32, &'static str)> =
                    order.iter().copied().zip(FX_ORDER_NAMES).collect();
                pairs.sort_by_key(|p| p.0);
                pairs.into_iter().map(|p| p.1).collect()
            }
        }
    }
}

fn bytes<const N: usize>(blob: &[u8], offset: usize) -> Option<[u8; N]> {
    blob.get(offset..offset.checked_add(N)?)?.try_into().ok()
}

fn f32_at(blob: &[u8], offset: usize) -> f32 {
    bytes::<4>(blob, offset).map_or(0.0, f32::from_le_bytes)
}

fn u16_at(blob: &[u8], offset: usize) -> u16 {
    bytes::<2>(blob, offset).map_or(0, u16::from_le_bytes)
}

fn doubles(blob: &[u8], offset: usize, count: usize) -> Option<Vec<f64>> {
    let data = blob.get(offset..offset.checked_add(count * 8)?)?;
    Some(
        data.chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap_or([0; 8])))
            .collect(),
    )
}

fn cstring(blob: &[u8], offset: usize, length: usize) -> String {
    let Some(raw) = blob.get(offset..offset + length) else {
        return String::new();
    };
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_owned()
}

/// Trims raw arrays to the points actually used and checks them.
fn build_shape(
    curves: Option<Vec<f64>>,
    xs: Option<Vec<f64>>,
    ys: Option<Vec<f64>>,
    settings: LfoSettings,
) -> LfoShape {
    let (Some(curves), Some(xs), Some(ys)) = (curves, xs, ys) else {
        return LfoShape {
            settings,
            ..LfoShape::default()
        };
    };
    // The shape ends at the first point that reaches the right edge.
    let count = xs
        .iter()
        .position(|&x| x >= 1.0 - 1e-9)
        .map_or(xs.len(), |i| i + 1);
    let in_range = |v: &f64| (-0.001..=1.001).contains(v);
    if count < 2
        || count > ys.len()
        || !xs[..count].iter().all(in_range)
        || !ys[..count].iter().all(in_range)
    {
        return LfoShape {
            settings,
            ..LfoShape::default()
        };
    }
    LfoShape {
        xs: xs[..count].to_vec(),
        ys: ys[..count].to_vec(),
        curves: curves[..(count - 1).max(1).min(curves.len())].to_vec(),
        settings,
    }
}

fn classic_settings(blob: &[u8]) -> Vec<LfoSettings> {
    let mut out = Vec::with_capacity(8);
    for record in CLASSIC_SETTINGS {
        let base = record + CLASSIC_SETTINGS_FLAGS;
        for i in 0..4 {
            if record + CLASSIC_SETTINGS_END > blob.len() {
                out.push(LfoSettings::unknown());
                continue;
            }
            let mut flags = [0_u8; 6];
            for (k, flag) in flags.iter_mut().enumerate() {
                *flag = blob.get(base + 4 * k + i).copied().unwrap_or(2);
            }
            out.push(if flags.iter().all(|&f| f <= 1) {
                LfoSettings::from_flags(flags)
            } else {
                LfoSettings::unknown()
            });
        }
    }
    out
}

fn new_lfo(blob: &[u8], index: usize) -> LfoShape {
    let base = NEW_LFO_BASE + NEW_LFO_STRIDE * index;
    let settings = match bytes::<6>(blob, base + NEW_OFF_FLAGS) {
        Some(flags) if flags.iter().all(|&f| f <= 1) => LfoSettings::from_flags(flags),
        _ => LfoSettings::unknown(),
    };
    // An 8-byte header, then three arrays of 480 f64: curves, x, y.
    build_shape(
        doubles(blob, base + 8, NEW_ARRAY_LEN),
        doubles(blob, base + NEW_OFF_X, NEW_ARRAY_LEN),
        doubles(blob, base + NEW_OFF_Y, NEW_ARRAY_LEN),
        settings,
    )
}

fn uses_new_layout(blob: &[u8]) -> bool {
    blob.len() >= NEW_LFO_BASE + 8 * NEW_LFO_STRIDE
        && blob
            .get(CLASSIC_BLOCKS[0]..CLASSIC_BLOCKS[0] + 1024)
            .is_some_and(|region| region.iter().all(|&b| b == 0))
}

fn lfos(blob: &[u8]) -> Vec<LfoShape> {
    if uses_new_layout(blob) {
        return (0..8).map(|i| new_lfo(blob, i)).collect();
    }
    let settings = classic_settings(blob);
    let mut shapes = Vec::with_capacity(8);
    for (b, block) in CLASSIC_BLOCKS.iter().enumerate() {
        for lfo in 0..4 {
            // Arrays 0-3 are tension, 4-7 x and 8-11 y.
            let array =
                |slot: usize| doubles(blob, block + CLASSIC_ARRAY_STRIDE * slot, CLASSIC_ARRAY_LEN);
            shapes.push(build_shape(
                array(lfo),
                array(4 + lfo),
                array(8 + lfo),
                settings.get(b * 4 + lfo).copied().unwrap_or_default(),
            ));
        }
    }
    shapes
}

fn fx_order(blob: &[u8]) -> Option<[i32; 10]> {
    let mut order = [0_i32; 10];
    for (i, slot) in order.iter_mut().enumerate() {
        *slot = i32::from_le_bytes(bytes::<4>(blob, OFF_FX_ORDER + 4 * i)?);
    }
    let mut sorted = order;
    sorted.sort_unstable();
    (sorted == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).then_some(order)
}

/// Locates the switches block by its invariants: 0.5 at +0x00 and +0x20,
/// 1.0 at +0x30 and a polyphony at +0x2C that is an exact (n − 1)/31. Three
/// of the four are enough.
fn settings_base(blob: &[u8]) -> Option<usize> {
    let mut best: Option<(u8, usize)> = None;
    for base in SETTINGS_BASES {
        if base + SETTINGS_SIZE > blob.len() {
            continue;
        }
        let near = |offset: usize, v: f32| (f32_at(blob, base + offset) - v).abs() < 1e-5;
        let poly = f32_at(blob, base + SET_POLYPHONY);
        let score = u8::from(near(0, 0.5))
            + u8::from(near(0x20, 0.5))
            + u8::from(near(0x30, 1.0))
            + u8::from(
                poly > 0.0 && poly <= 1.0 && (poly * 31.0 - (poly * 31.0).round()).abs() < 1e-3,
            );
        if best.is_none_or(|(s, _)| score > s) {
            best = Some((score, base));
        }
    }
    best.filter(|&(score, _)| score >= 3).map(|(_, base)| base)
}

fn global_settings(blob: &[u8]) -> GlobalSettings {
    let hall = match blob.get(OFF_REVERB_HALL) {
        Some(&b) if b <= 1 => b == 1,
        _ => true,
    };
    let Some(base) = settings_base(blob) else {
        return GlobalSettings {
            reverb_hall: hall,
            known: false,
            ..GlobalSettings::default()
        };
    };
    let value = |offset: usize| f32_at(blob, base + offset);
    let flag = |offset: usize| value(offset) > 0.5;
    let tuning = |offset: usize| {
        let index = (value(offset) * 4.0).round();
        UNISON_TUNINGS
            .get(if index >= 0.0 { index as usize } else { 0 })
            .copied()
            .unwrap_or("Linear")
    };
    let semitones = |offset: usize| (48.0 * value(offset)).clamp(0.0, 48.0);
    let poly = (value(SET_POLYPHONY) * 31.0).round() + 1.0;
    GlobalSettings {
        mono: flag(SET_MONO),
        legato: flag(SET_LEGATO),
        polyphony: poly.clamp(1.0, 32.0) as u8,
        porta_always: flag(SET_PORTA_ALWAYS),
        porta_scaled: flag(SET_PORTA_SCALED),
        noise_one_shot: flag(SET_NOISE_ONE_SHOT),
        noise_pitch_track: flag(SET_NOISE_PITCH_TRACK),
        filter_keytrack: flag(SET_FILTER_KEYTRACK),
        unison_range: SET_UNISON_RANGE.map(semitones),
        unison_tuning: SET_UNISON_TUNING.map(tuning),
        reverb_hall: flag(SET_REVERB_HALL),
        a4_hz: 430.0 + 20.0 * value(SET_TUNING_REF).clamp(0.0, 1.0),
        chorus_mono: flag(SET_CHORUS_MONO),
        known: true,
    }
}

/// Finds every modulation record by its `80 <slot> FF` marker. Slots 1–16
/// sit at fixed offsets; slots 17–32 wherever the marker is.
fn mod_slots(blob: &[u8]) -> Vec<ModSlot> {
    let mut slots: Vec<Option<ModSlot>> = vec![None; 32];
    for start in 0..blob.len().saturating_sub(2) {
        if blob[start] != 0x80 || blob[start + 2] != 0xFF {
            continue;
        }
        let index = usize::from(blob[start + 1]);
        let Some(base) = start.checked_sub(MOD_MARKER_OFFSET) else {
            continue;
        };
        if index > 31 || base + MOD_RECORD_SIZE > blob.len() {
            continue;
        }
        if index < 16 && base != index * MOD_RECORD_SIZE {
            continue;
        }
        if slots[index].is_some() {
            continue;
        }
        let amount = f32_at(blob, base + MOD_OFF_AMOUNT);
        let out_range = f32_at(blob, base + MOD_OFF_OUT);
        if !(-8.0..=8.0).contains(&amount) {
            continue;
        }
        let source = u16_at(blob, base + MOD_OFF_SOURCE);
        let aux = u16_at(blob, base + MOD_OFF_AUX);
        let dest = u16_at(blob, base + MOD_OFF_DEST);
        if source > 64 || aux > 64 || dest >= 1024 {
            continue;
        }
        slots[index] = Some(ModSlot {
            slot: index as u8 + 1,
            source,
            aux,
            dest,
            amount,
            out_range,
            bipolar: blob.get(base + MOD_OFF_TYPE) == Some(&1),
        });
    }
    slots.into_iter().flatten().collect()
}

/// Whether `data` starts like a VST2 preset (`CcnK`).
#[must_use]
pub fn looks_like(data: &[u8]) -> bool {
    data.starts_with(b"CcnK")
}

/// The decompressed Serum state of a `.fxp` file.
///
/// # Errors
///
/// Not an opaque-chunk `.fxp`, another plugin's preset, or a chunk that is
/// not zlib data.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.is_empty() {
        return Err(Error::NotSerum("the file is empty".to_owned()));
    }
    if data.len() < FXP_HEADER || !looks_like(data) {
        return Err(Error::NotSerum(
            "not a VST2 .fxp file (no CcnK header)".to_owned(),
        ));
    }
    if &data[8..12] != b"FPCh" {
        return Err(Error::NotSerum(
            "not an opaque-chunk .fxp preset".to_owned(),
        ));
    }
    if &data[16..20] != PLUGIN_ID {
        return Err(Error::NotSerum(format!(
            "another plugin's preset (plugin id {})",
            String::from_utf8_lossy(&data[16..20])
        )));
    }
    let size = bytes::<4>(data, 56).map_or(0, u32::from_be_bytes) as usize;
    let chunk = data
        .get(FXP_HEADER..FXP_HEADER.saturating_add(size).min(data.len()))
        .unwrap_or_default();
    miniz_oxide::inflate::decompress_to_vec_zlib(chunk)
        .map_err(|e| Error::Bad(format!("the preset data is not zlib data ({e:?})")))
}

/// Parses a Serum 1 `.fxp`; `stem` names the preset when the file does not.
///
/// # Errors
///
/// See [`decompress`]; also a state too small to hold the parameters.
pub fn read(data: &[u8], stem: &str) -> Result<Serum1Patch, Error> {
    let blob = decompress(data)?;
    if blob.len() < OFF_PARAMS_LOW + LOW_BLOCK * 4 {
        return Err(Error::Bad(
            "the preset state is too small to hold its parameters".to_owned(),
        ));
    }
    let high = PARAMS.len() - LOW_BLOCK;
    let mut params: Vec<f32> = (0..LOW_BLOCK)
        .map(|i| f32_at(&blob, OFF_PARAMS_LOW + 4 * i))
        .collect();
    if blob.len() >= OFF_PARAMS_HIGH + high * 4 {
        params.extend((0..high).map(|i| f32_at(&blob, OFF_PARAMS_HIGH + 4 * i)));
    } else {
        // Older builds stop before the second block.
        params.extend((LOW_BLOCK..PARAMS.len()).map(stored_default));
    }
    for v in &mut params {
        *v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    }
    // Older builds wrote the second block before all of its entries existed,
    // leaving memory Serum itself ignores; reset it by blob size.
    let unreliable_from = match blob.len() {
        n if n < 21_000 => LOW_BLOCK,
        n if n < 28_000 => 260,
        n if n < 33_000 => 273,
        _ => PARAMS.len(),
    };
    for (i, v) in params.iter_mut().enumerate().skip(unreliable_from) {
        *v = stored_default(i);
    }
    let name = cstring(&blob, OFF_PRESET_NAME, 32);
    Ok(Serum1Patch {
        name: if name.is_empty() {
            stem.to_owned()
        } else {
            name
        },
        author: cstring(&blob, OFF_AUTHOR, 48),
        menu: cstring(&blob, OFF_MENU, 48),
        macro_names: std::array::from_fn(|i| {
            cstring(
                &blob,
                OFF_MACRO_NAMES + MACRO_NAME_STRIDE * i,
                MACRO_NAME_STRIDE,
            )
        }),
        params,
        wavetable_a: cstring(&blob, OFF_WT_A, 512),
        wavetable_b: cstring(&blob, OFF_WT_B, 512),
        noise_sample: cstring(&blob, OFF_NOISE, 512),
        mod_slots: mod_slots(&blob),
        lfo_shapes: lfos(&blob),
        fx_order: fx_order(&blob),
        settings: global_settings(&blob),
    })
}

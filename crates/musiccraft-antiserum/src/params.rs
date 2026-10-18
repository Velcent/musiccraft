//! Antiserum's parameter table (see `docs/spec/antiserum.md`).
//!
//! Names are stable IDs and so is their order: a mod-matrix destination stores
//! a parameter's index. Parameters are only ever appended (the `frozen_names`
//! test checks `frozen-names.txt`).

use std::sync::OnceLock;

use musiccraft_params::{ARP_MODES, ARP_RATES, ParamSpec};

use crate::fx::{self, FX_PARAMS, FX_SLOTS, MODULES};

/// Global parameters.
pub const VOLUME: usize = 0;
pub const VELOCITY: usize = 1;
pub const POLYPHONY: usize = 2;
pub const VOICE_MODE: usize = 3;
pub const GLIDE: usize = 4;
pub const GLIDE_MODE: usize = 5;
pub const GLIDE_CURVE: usize = 6;
pub const BEND_RANGE: usize = 7;
/// `macro1`; the eight macros follow in order.
pub const MACRO1: usize = 8;
/// Number of macros.
pub const MACROS: usize = 8;

/// Number of wavetable oscillators.
pub const OSCS: usize = 3;
/// First oscillator parameter; each oscillator has [`OSC_STRIDE`].
pub const OSC_BASE: usize = 16;
pub const OSC_STRIDE: usize = 21;
pub mod osc {
    //! Offsets inside one oscillator's parameters.
    pub const ON: usize = 0;
    pub const LEVEL: usize = 1;
    pub const PAN: usize = 2;
    pub const OCTAVE: usize = 3;
    pub const SEMI: usize = 4;
    pub const FINE: usize = 5;
    pub const POSITION: usize = 6;
    pub const PHASE: usize = 7;
    pub const RANDOM_PHASE: usize = 8;
    pub const WARP1: usize = 9;
    pub const WARP1_AMOUNT: usize = 10;
    pub const WARP2: usize = 11;
    pub const WARP2_AMOUNT: usize = 12;
    pub const MOD_SOURCE: usize = 13;
    pub const UNISON: usize = 14;
    pub const DETUNE: usize = 15;
    pub const BLEND: usize = 16;
    pub const WIDTH: usize = 17;
    pub const STACK: usize = 18;
    pub const FILTER1: usize = 19;
    pub const FILTER2: usize = 20;
}

/// Sub oscillator parameters.
pub const SUB_BASE: usize = OSC_BASE + OSCS * OSC_STRIDE;
pub mod sub {
    //! Offsets inside the sub oscillator's parameters.
    pub const ON: usize = 0;
    pub const SHAPE: usize = 1;
    pub const OCTAVE: usize = 2;
    pub const LEVEL: usize = 3;
    pub const PAN: usize = 4;
    pub const FILTER1: usize = 5;
    pub const FILTER2: usize = 6;
}

/// Noise oscillator parameters.
pub const NOISE_BASE: usize = SUB_BASE + 7;
pub mod noise {
    //! Offsets inside the noise oscillator's parameters.
    pub const ON: usize = 0;
    pub const TYPE: usize = 1;
    pub const LEVEL: usize = 2;
    pub const PAN: usize = 3;
    pub const PITCH: usize = 4;
    pub const TRACK: usize = 5;
    pub const ONE_SHOT: usize = 6;
    pub const FILTER1: usize = 7;
    pub const FILTER2: usize = 8;
}

/// Number of filters.
pub const FILTERS: usize = 2;
/// First filter parameter; each filter has [`FILTER_STRIDE`].
pub const FILTER_BASE: usize = NOISE_BASE + 9;
pub const FILTER_STRIDE: usize = 7;
pub mod filter {
    //! Offsets inside one filter's parameters.
    pub const ON: usize = 0;
    pub const TYPE: usize = 1;
    pub const CUTOFF: usize = 2;
    pub const RESONANCE: usize = 3;
    pub const DRIVE: usize = 4;
    pub const KEY_TRACK: usize = 5;
    pub const MIX: usize = 6;
}
/// Serial or parallel filters.
pub const FILTER_ROUTING: usize = FILTER_BASE + FILTERS * FILTER_STRIDE;

/// Number of envelopes (envelope 1 is the amplitude envelope).
pub const ENVS: usize = 4;
pub const ENV_BASE: usize = FILTER_ROUTING + 1;
pub const ENV_STRIDE: usize = 8;
pub mod env {
    //! Offsets inside one envelope's parameters.
    pub const ATTACK: usize = 0;
    pub const HOLD: usize = 1;
    pub const DECAY: usize = 2;
    pub const SUSTAIN: usize = 3;
    pub const RELEASE: usize = 4;
    pub const ATTACK_CURVE: usize = 5;
    pub const DECAY_CURVE: usize = 6;
    pub const RELEASE_CURVE: usize = 7;
}

/// Number of LFOs.
pub const LFOS: usize = 8;
pub const LFO_BASE: usize = ENV_BASE + ENVS * ENV_STRIDE;
pub const LFO_STRIDE: usize = 8;
pub mod lfo {
    //! Offsets inside one LFO's parameters.
    pub const SHAPE: usize = 0;
    pub const MODE: usize = 1;
    pub const SYNC: usize = 2;
    pub const RATE: usize = 3;
    pub const SYNC_RATE: usize = 4;
    pub const DELAY: usize = 5;
    pub const RISE: usize = 6;
    pub const PHASE: usize = 7;
}

/// Number of mod matrix slots.
pub const SLOTS: usize = 32;
pub const MOD_BASE: usize = LFO_BASE + LFOS * LFO_STRIDE;
pub const MOD_STRIDE: usize = 6;
pub mod slot {
    //! Offsets inside one mod matrix slot's parameters.
    pub const SOURCE: usize = 0;
    pub const DEST: usize = 1;
    pub const AMOUNT: usize = 2;
    pub const CURVE: usize = 3;
    pub const BIPOLAR: usize = 4;
    pub const AUX: usize = 5;
}

/// Arpeggiator parameters (`docs/spec/antiserum-fx-arp.md`).
pub const ARP_BASE: usize = MOD_BASE + SLOTS * MOD_STRIDE;
pub mod arp {
    //! Offsets inside the arpeggiator's parameters.
    pub const MODE: usize = 0;
    pub const RATE: usize = 1;
    pub const RANGE: usize = 2;
    pub const GATE: usize = 3;
}

/// Sequencer parameters.
pub const SEQ_BASE: usize = ARP_BASE + 4;
pub mod seq {
    //! Offsets inside the sequencer's parameters.
    pub const ON: usize = 0;
    pub const LENGTH: usize = 1;
    pub const RATE: usize = 2;
}

/// The effect slots, `fx_slot1` … `fx_slot10`.
pub const FX_SLOT_BASE: usize = SEQ_BASE + 3;
/// The effect modules' parameters, module after module.
pub const FX_BASE: usize = FX_SLOT_BASE + FX_SLOTS;

/// Number of parameters.
pub const PARAM_COUNT: usize = FX_BASE + FX_PARAMS;

/// Index of parameter `field` of effect module `m` (0-based).
#[must_use]
pub const fn fx_param(m: usize, field: usize) -> usize {
    FX_BASE + fx::module_offset(m) + field
}

/// Index of field `field` of oscillator `n` (0-based).
#[must_use]
pub const fn osc_param(n: usize, field: usize) -> usize {
    OSC_BASE + n * OSC_STRIDE + field
}
/// Index of field `field` of filter `n` (0-based).
#[must_use]
pub const fn filter_param(n: usize, field: usize) -> usize {
    FILTER_BASE + n * FILTER_STRIDE + field
}
/// Index of field `field` of envelope `n` (0-based).
#[must_use]
pub const fn env_param(n: usize, field: usize) -> usize {
    ENV_BASE + n * ENV_STRIDE + field
}
/// Index of field `field` of LFO `n` (0-based).
#[must_use]
pub const fn lfo_param(n: usize, field: usize) -> usize {
    LFO_BASE + n * LFO_STRIDE + field
}
/// Index of field `field` of mod matrix slot `n` (0-based).
#[must_use]
pub const fn slot_param(n: usize, field: usize) -> usize {
    MOD_BASE + n * MOD_STRIDE + field
}

pub const VOICE_MODES: [&str; 3] = ["poly", "mono", "legato"];
pub const GLIDE_MODES: [&str; 2] = ["always", "legato"];
pub const OFF_ON: [&str; 2] = ["off", "on"];
pub const WARPS: [&str; 14] = [
    "off", "sync", "bend+", "bend-", "PWM", "asym+", "asym-", "mirror", "flip", "quantize",
    "remap", "FM", "AM", "RM",
];
/// Number of warp modes.
pub const WARP_COUNT: usize = WARPS.len();
pub const MOD_SOURCES_OSC: [&str; 4] = ["other A", "other B", "sub", "noise"];
pub const STACKS: [&str; 5] = ["off", "+1 oct", "+2 oct", "-1 oct", "±1 oct"];
pub const SUB_SHAPES: [&str; 4] = ["sine", "triangle", "saw", "square"];
pub const NOISE_TYPES: [&str; 5] = ["white", "pink", "brown", "crackle", "digital"];
pub const FILTER_TYPES: [&str; 18] = [
    "low-pass 12",
    "low-pass 24",
    "high-pass 12",
    "high-pass 24",
    "band-pass 12",
    "band-pass 24",
    "notch 12",
    "notch 24",
    "peak 12",
    "peak 24",
    "ladder",
    "diode",
    "comb+",
    "comb-",
    "formant",
    "vowel",
    "phaser",
    "flanger",
];
pub const ROUTINGS: [&str; 2] = ["serial", "parallel"];
pub const LFO_SHAPES: [&str; 8] = [
    "sine",
    "triangle",
    "saw up",
    "saw down",
    "square",
    "sample & hold",
    "smooth random",
    "custom",
];
pub const LFO_MODES: [&str; 3] = ["trigger", "envelope", "free"];
/// Synced LFO rates: label and length of one cycle in beats.
pub const SYNC_RATES: [(&str, f32); 17] = [
    ("8 bars", 32.0),
    ("4 bars", 16.0),
    ("2 bars", 8.0),
    ("1 bar", 4.0),
    ("1/2", 2.0),
    ("1/2 dotted", 3.0),
    ("1/2 triplet", 4.0 / 3.0),
    ("1/4", 1.0),
    ("1/4 dotted", 1.5),
    ("1/4 triplet", 2.0 / 3.0),
    ("1/8", 0.5),
    ("1/8 dotted", 0.75),
    ("1/8 triplet", 1.0 / 3.0),
    ("1/16", 0.25),
    ("1/16 dotted", 0.375),
    ("1/16 triplet", 1.0 / 6.0),
    ("1/32", 0.125),
];
const SYNC_RATE_LABELS: [&str; 17] = {
    let mut out = [""; 17];
    let mut i = 0;
    while i < 17 {
        out[i] = SYNC_RATES[i].0;
        i += 1;
    }
    out
};

/// Modulation sources, by `modN_source` / `modN_aux` value.
pub const SOURCES: [&str; 27] = [
    "none",
    "env 1",
    "env 2",
    "env 3",
    "env 4",
    "lfo 1",
    "lfo 2",
    "lfo 3",
    "lfo 4",
    "lfo 5",
    "lfo 6",
    "lfo 7",
    "lfo 8",
    "velocity",
    "note",
    "aftertouch",
    "mod wheel",
    "pitch bend",
    "random",
    "macro 1",
    "macro 2",
    "macro 3",
    "macro 4",
    "macro 5",
    "macro 6",
    "macro 7",
    "macro 8",
];
/// Source numbers.
pub mod source {
    pub const NONE: usize = 0;
    pub const ENV1: usize = 1;
    pub const LFO1: usize = 5;
    pub const VELOCITY: usize = 13;
    pub const NOTE: usize = 14;
    pub const AFTERTOUCH: usize = 15;
    pub const MOD_WHEEL: usize = 16;
    pub const PITCH_BEND: usize = 17;
    pub const RANDOM: usize = 18;
    pub const MACRO1: usize = 19;
}

/// One parameter of a repeated group.
struct Field {
    suffix: &'static str,
    min: f32,
    max: f32,
    default: f32,
    unit: &'static str,
    integer: bool,
}

const fn f(suffix: &'static str, min: f32, max: f32, default: f32, unit: &'static str) -> Field {
    Field {
        suffix,
        min,
        max,
        default,
        unit,
        integer: false,
    }
}

const fn int(suffix: &'static str, max: f32, default: f32) -> Field {
    Field {
        suffix,
        min: 0.0,
        max,
        default,
        unit: "",
        integer: true,
    }
}

const fn int_range(suffix: &'static str, min: f32, max: f32, default: f32) -> Field {
    Field {
        suffix,
        min,
        max,
        default,
        unit: "",
        integer: true,
    }
}

const GLOBAL: [Field; 8] = [
    f("volume", 0.0, 1.0, 0.75, ""),
    f("velocity", 0.0, 1.0, 0.6, ""),
    int_range("polyphony", 1.0, 32.0, 8.0),
    int("voice_mode", 2.0, 0.0),
    f("glide", 0.0, 5.0, 0.0, "s"),
    int("glide_mode", 1.0, 0.0),
    f("glide_curve", -1.0, 1.0, 0.0, ""),
    int("bend_range", 48.0, 2.0),
];

const OSC: [Field; OSC_STRIDE] = [
    int("on", 1.0, 0.0),
    f("level", 0.0, 1.0, 0.75, ""),
    f("pan", -1.0, 1.0, 0.0, ""),
    int_range("octave", -4.0, 4.0, 0.0),
    int_range("semi", -12.0, 12.0, 0.0),
    f("fine", -100.0, 100.0, 0.0, "ct"),
    f("position", 0.0, 1.0, 0.0, ""),
    f("phase", 0.0, 1.0, 0.0, ""),
    f("random_phase", 0.0, 1.0, 1.0, ""),
    int("warp1", (WARP_COUNT - 1) as f32, 0.0),
    f("warp1_amount", 0.0, 1.0, 0.0, ""),
    int("warp2", (WARP_COUNT - 1) as f32, 0.0),
    f("warp2_amount", 0.0, 1.0, 0.0, ""),
    int("mod_source", 3.0, 0.0),
    int_range("unison", 1.0, 16.0, 1.0),
    f("detune", 0.0, 1.0, 0.25, ""),
    f("blend", 0.0, 1.0, 0.75, ""),
    f("width", 0.0, 1.0, 1.0, ""),
    int("stack", 4.0, 0.0),
    f("filter1", 0.0, 1.0, 1.0, ""),
    f("filter2", 0.0, 1.0, 0.0, ""),
];

const SUB: [Field; 7] = [
    int("on", 1.0, 0.0),
    int("shape", 3.0, 0.0),
    int_range("octave", -4.0, 0.0, -1.0),
    f("level", 0.0, 1.0, 0.75, ""),
    f("pan", -1.0, 1.0, 0.0, ""),
    f("filter1", 0.0, 1.0, 1.0, ""),
    f("filter2", 0.0, 1.0, 0.0, ""),
];

const NOISE: [Field; 9] = [
    int("on", 1.0, 0.0),
    int("type", 4.0, 0.0),
    f("level", 0.0, 1.0, 0.5, ""),
    f("pan", -1.0, 1.0, 0.0, ""),
    f("pitch", -48.0, 48.0, 0.0, "st"),
    int("track", 1.0, 0.0),
    int("one_shot", 1.0, 0.0),
    f("filter1", 0.0, 1.0, 1.0, ""),
    f("filter2", 0.0, 1.0, 0.0, ""),
];

const FILTER: [Field; FILTER_STRIDE] = [
    int("on", 1.0, 0.0),
    int("type", 17.0, 1.0),
    f("cutoff", 20.0, 20_000.0, 2_000.0, "Hz"),
    f("resonance", 0.0, 1.0, 0.2, ""),
    f("drive", 0.0, 1.0, 0.0, ""),
    f("key_track", 0.0, 1.0, 0.0, ""),
    f("mix", 0.0, 1.0, 1.0, ""),
];

const ENV: [Field; ENV_STRIDE] = [
    f("attack", 0.0, 10.0, 0.002, "s"),
    f("hold", 0.0, 10.0, 0.0, "s"),
    f("decay", 0.0, 10.0, 0.5, "s"),
    f("sustain", 0.0, 1.0, 0.8, ""),
    f("release", 0.0, 10.0, 0.15, "s"),
    f("attack_curve", -1.0, 1.0, 0.0, ""),
    f("decay_curve", -1.0, 1.0, 0.5, ""),
    f("release_curve", -1.0, 1.0, 0.5, ""),
];

const LFO: [Field; LFO_STRIDE] = [
    int("shape", 7.0, 0.0),
    int("mode", 2.0, 0.0),
    int("sync", 1.0, 1.0),
    f("rate", 0.01, 50.0, 2.0, "Hz"),
    int("sync_rate", 16.0, 7.0),
    f("delay", 0.0, 10.0, 0.0, "s"),
    f("rise", 0.0, 10.0, 0.0, "s"),
    f("phase", 0.0, 1.0, 0.0, ""),
];

const SLOT: [Field; MOD_STRIDE] = [
    int("source", (SOURCES.len() - 1) as f32, 0.0),
    int("dest", PARAM_COUNT as f32, 0.0),
    f("amount", -1.0, 1.0, 0.0, ""),
    f("curve", -1.0, 1.0, 0.0, ""),
    int("bipolar", 1.0, 0.0),
    int("aux", (SOURCES.len() - 1) as f32, 0.0),
];

const ARP: [Field; 4] = [
    int("mode", (ARP_MODES.len() - 1) as f32, 0.0),
    int("rate", (ARP_RATES.len() - 1) as f32, 1.0),
    int_range("range", 1.0, 4.0, 1.0),
    f("gate", 0.05, 1.0, 0.5, ""),
];

const SEQ: [Field; 3] = [
    int("on", 1.0, 0.0),
    int_range("length", 1.0, 64.0, 16.0),
    int("rate", (ARP_RATES.len() - 1) as f32, 1.0),
];

fn leak(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}

fn push_group(table: &mut Vec<ParamSpec>, prefix: &str, fields: &[Field]) {
    for field in fields {
        let name = if prefix.is_empty() {
            field.suffix
        } else {
            leak(format!("{prefix}_{}", field.suffix))
        };
        table.push(ParamSpec {
            name,
            min: field.min,
            max: field.max,
            default: field.default,
            unit: field.unit,
            integer: field.integer,
        });
    }
}

fn build() -> Vec<ParamSpec> {
    let mut table = Vec::with_capacity(PARAM_COUNT);
    push_group(&mut table, "", &GLOBAL);
    for n in 1..=MACROS {
        table.push(ParamSpec {
            name: leak(format!("macro{n}")),
            min: 0.0,
            max: 1.0,
            default: 0.0,
            unit: "",
            integer: false,
        });
    }
    for n in 1..=OSCS {
        push_group(&mut table, &format!("osc{n}"), &OSC);
    }
    push_group(&mut table, "sub", &SUB);
    push_group(&mut table, "noise", &NOISE);
    for n in 1..=FILTERS {
        push_group(&mut table, &format!("filter{n}"), &FILTER);
    }
    push_group(&mut table, "", &[int("filter_routing", 1.0, 0.0)]);
    for n in 1..=ENVS {
        push_group(&mut table, &format!("env{n}"), &ENV);
    }
    for n in 1..=LFOS {
        push_group(&mut table, &format!("lfo{n}"), &LFO);
    }
    for n in 1..=SLOTS {
        push_group(&mut table, &format!("mod{n}"), &SLOT);
    }
    push_group(&mut table, "arp", &ARP);
    push_group(&mut table, "seq", &SEQ);
    for n in 1..=FX_SLOTS {
        push_group(
            &mut table,
            "fx",
            &[Field {
                suffix: leak(format!("slot{n}")),
                ..int("", MODULES.len() as f32, 0.0)
            }],
        );
    }
    for module in &MODULES {
        for spec in fx::module_specs(module) {
            table.push(ParamSpec {
                name: leak(format!("fx_{}_{}", module.name, spec.name)),
                ..spec
            });
        }
    }
    // Defaults that differ between instances of a group.
    if let Some(on) = table.get_mut(osc_param(0, osc::ON)) {
        on.default = 1.0;
    }
    if let Some(on) = table.get_mut(filter_param(0, filter::ON)) {
        on.default = 1.0;
    }
    if let Some(cutoff) = table.get_mut(filter_param(0, filter::CUTOFF)) {
        cutoff.default = 20_000.0;
    }
    table
}

/// The parameter table, in index order.
#[must_use]
pub fn params() -> &'static [ParamSpec] {
    static TABLE: OnceLock<Vec<ParamSpec>> = OnceLock::new();
    TABLE.get_or_init(build)
}

/// Every parameter at its default.
#[must_use]
pub fn defaults() -> [f32; PARAM_COUNT] {
    let mut out = [0.0; PARAM_COUNT];
    for (value, spec) in out.iter_mut().zip(params()) {
        *value = spec.default;
    }
    out
}

/// The index of the parameter called `name`.
#[must_use]
pub fn index_of(name: &str) -> Option<usize> {
    params().iter().position(|spec| spec.name == name)
}

/// The names of a choice parameter's values (index = value), if it is one.
#[must_use]
pub fn choices(name: &str) -> Option<&'static [&'static str]> {
    let suffix = name.rsplit_once('_').map_or(name, |(_, s)| s);
    let group = name.split('_').next().unwrap_or(name);
    if let Some(rest) = name.strip_prefix("fx_") {
        return fx_choices(rest);
    }
    let list: &'static [&'static str] = match name {
        "arp_mode" => &ARP_MODES,
        "arp_rate" | "seq_rate" => arp_rate_labels(),
        "voice_mode" => &VOICE_MODES,
        "glide_mode" => &GLIDE_MODES,
        "filter_routing" => &ROUTINGS,
        "sub_shape" => &SUB_SHAPES,
        "noise_type" => &NOISE_TYPES,
        "noise_track" | "noise_one_shot" => &OFF_ON,
        _ if name.ends_with("_mod_source") => &MOD_SOURCES_OSC,
        _ if group.starts_with("osc") && (suffix == "warp1" || suffix == "warp2") => &WARPS,
        _ if group.starts_with("osc") && suffix == "stack" => &STACKS,
        _ if group.starts_with("filter") && suffix == "type" => &FILTER_TYPES,
        _ if group.starts_with("lfo") && suffix == "shape" => &LFO_SHAPES,
        _ if group.starts_with("lfo") && suffix == "mode" => &LFO_MODES,
        _ if group.starts_with("lfo") && suffix == "sync" => &OFF_ON,
        _ if group.starts_with("lfo") && name.ends_with("sync_rate") => &SYNC_RATE_LABELS,
        _ if group.starts_with("mod") && (suffix == "source" || suffix == "aux") => &SOURCES,
        _ if group.starts_with("mod") && suffix == "bipolar" => &OFF_ON,
        _ if group.starts_with("mod") && suffix == "dest" => destinations(),
        _ if suffix == "on" => &OFF_ON,
        _ => return None,
    };
    Some(list)
}

/// The arpeggiator and sequencer rate names.
fn arp_rate_labels() -> &'static [&'static str] {
    static LIST: OnceLock<Vec<&'static str>> = OnceLock::new();
    LIST.get_or_init(|| ARP_RATES.iter().map(|(label, _)| *label).collect())
}

/// Choices of `fx_<rest>`: the slot menus, or a module parameter's choices
/// as its mixer effect has them.
fn fx_choices(rest: &str) -> Option<&'static [&'static str]> {
    static SLOT: OnceLock<Vec<&'static str>> = OnceLock::new();
    static MODULE: OnceLock<Vec<(&'static str, Vec<&'static str>)>> = OnceLock::new();
    if rest.starts_with("slot") {
        return Some(SLOT.get_or_init(|| {
            std::iter::once("none")
                .chain(MODULES.iter().map(|m| m.name))
                .collect()
        }));
    }
    let lists = MODULE.get_or_init(|| {
        let mut lists = Vec::new();
        for module in &MODULES {
            let Some(kind) = module.kind else {
                continue;
            };
            for spec in fx::module_specs(module) {
                if let Some(list) = musiccraft_dsp::describe::choices(kind, spec.name) {
                    lists.push((leak(format!("{}_{}", module.name, spec.name)), list));
                }
            }
        }
        lists
    });
    lists
        .iter()
        .find(|(name, _)| *name == rest)
        .map(|(_, list)| list.as_slice())
}

/// Mod matrix destinations by `dest` value: "none", then every parameter's
/// name (continuous parameters only are modulated; others are ignored).
fn destinations() -> &'static [&'static str] {
    static LIST: OnceLock<Vec<&'static str>> = OnceLock::new();
    LIST.get_or_init(|| {
        std::iter::once("none")
            .chain(params().iter().map(|spec| spec.name))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_the_index_constants() {
        let table = params();
        assert_eq!(table.len(), PARAM_COUNT);
        let name = |i: usize| table[i].name;
        assert_eq!(name(VOLUME), "volume");
        assert_eq!(name(BEND_RANGE), "bend_range");
        assert_eq!(name(MACRO1 + 7), "macro8");
        assert_eq!(name(osc_param(0, osc::ON)), "osc1_on");
        assert_eq!(name(osc_param(2, osc::FILTER2)), "osc3_filter2");
        assert_eq!(name(SUB_BASE + sub::FILTER2), "sub_filter2");
        assert_eq!(name(NOISE_BASE + noise::FILTER2), "noise_filter2");
        assert_eq!(name(filter_param(1, filter::MIX)), "filter2_mix");
        assert_eq!(name(FILTER_ROUTING), "filter_routing");
        assert_eq!(name(env_param(3, env::RELEASE_CURVE)), "env4_release_curve");
        assert_eq!(name(lfo_param(7, lfo::PHASE)), "lfo8_phase");
        assert_eq!(name(slot_param(31, slot::AUX)), "mod32_aux");
        assert_eq!(SOURCES[source::MACRO1], "macro 1");
        assert_eq!(SOURCES[source::RANDOM], "random");
        assert_eq!(SOURCES[source::LFO1], "lfo 1");
    }

    #[test]
    fn defaults_are_in_range_and_choices_fit() {
        for spec in params() {
            assert!(spec.accepts(spec.default), "{}", spec.name);
            if let Some(list) = choices(spec.name) {
                assert!(spec.integer, "{}", spec.name);
                assert_eq!(
                    (spec.max - spec.min + 1.0) as usize,
                    list.len(),
                    "{}",
                    spec.name
                );
            }
        }
        assert_eq!(choices("osc2_warp1").map(<[_]>::len), Some(WARP_COUNT));
        assert_eq!(choices("osc1_level"), None);
        assert_eq!(choices("mod5_dest").and_then(|d| d.get(1)), Some(&"volume"));
    }

    /// Names and their order are stable IDs; parameters are only appended.
    /// `MUSICCRAFT_BLESS_NAMES=1` rewrites the file after adding some.
    #[test]
    fn frozen_names() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/frozen-names.txt");
        let current: String = params().iter().map(|s| format!("{}\n", s.name)).collect();
        if std::env::var_os("MUSICCRAFT_BLESS_NAMES").is_some() {
            std::fs::write(path, &current).unwrap();
        }
        let frozen = std::fs::read_to_string(path).unwrap();
        assert!(
            current.starts_with(&frozen),
            "parameters were renamed, removed or reordered; only appending is allowed"
        );
        assert_eq!(
            current, frozen,
            "new parameters: add them to frozen-names.txt"
        );
    }
}

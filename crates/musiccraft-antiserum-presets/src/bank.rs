//! The factory bank: our own sound design, written as a few parameter recipes
//! per sound family. Every preset lists only what differs from the init patch.

use std::sync::OnceLock;

use musiccraft_antiserum::fx::MODULES;
use musiccraft_antiserum::index_of;
use musiccraft_antiserum::params::source;
use musiccraft_patch::Patch;

/// Filter types by name (see `musiccraft_antiserum::params::FILTER_TYPES`).
mod ft {
    pub const LP12: f32 = 0.0;
    pub const LP24: f32 = 1.0;
    pub const HP12: f32 = 2.0;
    pub const BP12: f32 = 4.0;
    pub const LADDER: f32 = 10.0;
    pub const DIODE: f32 = 11.0;
    pub const COMB: f32 = 12.0;
    pub const FORMANT: f32 = 14.0;
    pub const VOWEL: f32 = 15.0;
    pub const PHASER: f32 = 16.0;
}

/// Warps by name (see `musiccraft_antiserum::params::WARPS`).
mod warp {
    pub const SYNC: f32 = 1.0;
    pub const BEND_UP: f32 = 2.0;
    pub const MIRROR: f32 = 7.0;
    pub const QUANTIZE: f32 = 9.0;
    pub const FM: f32 = 11.0;
    pub const RM: f32 = 13.0;
}

/// LFO sync rates by name (see `SYNC_RATES`).
mod rate {
    pub const BARS_4: f32 = 1.0;
    pub const HALF: f32 = 4.0;
    pub const QUARTER: f32 = 7.0;
    pub const EIGHTH: f32 = 10.0;
    pub const EIGHTH_TRIPLET: f32 = 12.0;
    pub const SIXTEENTH: f32 = 13.0;
}

const ENV2: usize = source::ENV1 + 1;
const LFO1: usize = source::LFO1;
const LFO2: usize = source::LFO1 + 1;

/// A preset under construction.
struct B {
    patch: Patch,
    routes: usize,
    slots: usize,
}

impl B {
    fn new(name: &str, category: &str, tags: &[&str]) -> Self {
        Self {
            patch: Patch {
                name: name.to_owned(),
                category: category.to_owned(),
                tags: tags.iter().map(|t| (*t).to_owned()).collect(),
                ..Patch::default()
            },
            routes: 0,
            slots: 0,
        }
    }

    fn p(mut self, name: &str, value: f32) -> Self {
        self.patch.params.insert(name.to_owned(), value);
        self
    }

    fn ps(mut self, values: &[(&str, f32)]) -> Self {
        for (name, value) in values {
            self = self.p(name, *value);
        }
        self
    }

    fn table(mut self, osc: usize, name: &str) -> Self {
        self.patch.set_table(osc, format!("factory:{name}"));
        self
    }

    /// Wavetable oscillator `n` (1-based) on, with a table and position.
    fn osc(self, n: usize, table: &str, position: f32) -> Self {
        self.table(n - 1, table)
            .p(&format!("osc{n}_on"), 1.0)
            .p(&format!("osc{n}_position"), position)
    }

    fn unison(self, n: usize, voices: f32, detune: f32) -> Self {
        self.p(&format!("osc{n}_unison"), voices)
            .p(&format!("osc{n}_detune"), detune)
    }

    fn amp(self, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        self.env(1, attack, decay, sustain, release)
    }

    fn env(self, n: usize, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        self.p(&format!("env{n}_attack"), attack)
            .p(&format!("env{n}_decay"), decay)
            .p(&format!("env{n}_sustain"), sustain)
            .p(&format!("env{n}_release"), release)
    }

    fn filter(self, n: usize, kind: f32, cutoff: f32, resonance: f32) -> Self {
        self.p(&format!("filter{n}_on"), 1.0)
            .p(&format!("filter{n}_type"), kind)
            .p(&format!("filter{n}_cutoff"), cutoff)
            .p(&format!("filter{n}_resonance"), resonance)
    }

    /// A synced LFO `n` at `rate` with `shape`.
    fn lfo(self, n: usize, shape: f32, sync_rate: f32) -> Self {
        self.p(&format!("lfo{n}_shape"), shape)
            .p(&format!("lfo{n}_sync"), 1.0)
            .p(&format!("lfo{n}_sync_rate"), sync_rate)
    }

    /// A free LFO `n` at `hz`.
    fn lfo_hz(self, n: usize, shape: f32, hz: f32) -> Self {
        self.p(&format!("lfo{n}_shape"), shape)
            .p(&format!("lfo{n}_sync"), 0.0)
            .p(&format!("lfo{n}_rate"), hz)
    }

    /// The next matrix slot: `source` to parameter `dest` by `amount`.
    fn route(mut self, source: usize, dest: &str, amount: f32) -> Self {
        self.routes += 1;
        let n = self.routes;
        let dest = index_of(dest).map_or(0.0, |i| (i + 1) as f32);
        self.p(&format!("mod{n}_source"), source as f32)
            .p(&format!("mod{n}_dest"), dest)
            .p(&format!("mod{n}_amount"), amount)
    }

    /// The next effect slot: `module` with some of its parameters.
    fn fx(mut self, module: &str, values: &[(&str, f32)]) -> Self {
        self.slots += 1;
        let number = MODULES
            .iter()
            .position(|m| m.name == module)
            .map_or(0, |m| m + 1);
        let slot = format!("fx_slot{}", self.slots);
        self = self.p(&slot, number as f32);
        for (name, value) in values {
            self = self.p(&format!("fx_{module}_{name}"), *value);
        }
        self
    }

    fn reverb(self, mix: f32, decay: f32) -> Self {
        self.fx("reverb", &[("mix", mix), ("decay", decay)])
    }

    fn delay(self, mix: f32, feedback: f32) -> Self {
        self.fx("delay", &[("mix", mix), ("feedback", feedback)])
    }

    /// Oscillator `n` (1-based) on, of type `kind` (1 sample, 2 multisample,
    /// 3 granular, 4 spectral), playing the built-in sound `sample`.
    fn sampled(mut self, n: usize, kind: f32, sample: &str) -> Self {
        let mut source = self.patch.source(n - 1);
        source.sample = format!("builtin:{sample}");
        self.patch.set_source(n - 1, source);
        self.p(&format!("osc{n}_on"), 1.0)
            .p(&format!("osc{n}_type"), kind)
    }

    /// A multisample zone on oscillator `n` (1-based).
    fn zone(mut self, n: usize, sample: &str, low: u8, high: u8, root: u8) -> Self {
        let mut source = self.patch.source(n - 1);
        source.zones.push(musiccraft_patch::ZoneRef::new(
            format!("builtin:{sample}"),
            low,
            high,
            root,
        ));
        self.patch.set_source(n - 1, source);
        self.p(&format!("osc{n}_on"), 1.0)
            .p(&format!("osc{n}_type"), 2.0)
    }

    /// Oscillator `n`'s grains: size (seconds), density, spread and scan.
    fn grains(self, n: usize, size: f32, density: f32, spread: f32, scan: f32) -> Self {
        self.p(&format!("osc{n}_grain_size"), size)
            .p(&format!("osc{n}_grain_density"), density)
            .p(&format!("osc{n}_grain_spread"), spread)
            .p(&format!("osc{n}_grain_scan"), scan)
    }

    /// Oscillator `n`'s spectral shift (Hz), stretch, tilt and speed.
    fn spectral(self, n: usize, shift: f32, stretch: f32, tilt: f32, speed: f32) -> Self {
        self.p(&format!("osc{n}_spectral_shift"), shift)
            .p(&format!("osc{n}_spectral_stretch"), stretch)
            .p(&format!("osc{n}_spectral_tilt"), tilt)
            .p(&format!("osc{n}_spectral_speed"), speed)
    }

    fn mono(self, glide: f32) -> Self {
        self.p("voice_mode", 2.0).p("glide", glide)
    }

    fn done(self) -> Patch {
        self.patch
    }
}

/// Every factory preset, in category order.
#[must_use]
pub fn factory() -> &'static [Patch] {
    static BANK: OnceLock<Vec<Patch>> = OnceLock::new();
    BANK.get_or_init(|| {
        let mut bank = Vec::new();
        bank.extend(bass());
        bank.extend(lead());
        bank.extend(pad());
        bank.extend(pluck());
        bank.extend(keys());
        bank.extend(fx());
        bank.extend(drums());
        bank.extend(sampled());
        bank
    })
}

/// Presets built on the sample, multisample, granular and spectral
/// oscillators, playing the built-in sounds (so they work everywhere).
fn sampled() -> Vec<Patch> {
    const SAMPLE: f32 = 1.0;
    const GRAINS: f32 = 3.0;
    const SPECTRAL: f32 = 4.0;
    vec![
        // Pads.
        B::new("Crash Cloud", "pad", &["pad", "granular", "evolving"])
            .sampled(1, GRAINS, "crash")
            .grains(1, 0.15, 40.0, 0.4, 0.3)
            .unison(1, 3.0, 0.2)
            .p("osc1_width", 0.8)
            .p("osc1_level", 1.0)
            .amp(0.8, 1.0, 0.9, 2.5)
            .reverb(0.4, 0.8)
            .done(),
        B::new("Spectral Choir", "pad", &["pad", "spectral", "warm"])
            .sampled(1, SPECTRAL, "tom")
            .spectral(1, 0.0, 0.0, -0.3, 0.0)
            .unison(1, 5.0, 0.25)
            .p("osc1_width", 0.9)
            .amp(0.9, 1.0, 0.85, 2.0)
            .reverb(0.35, 0.7)
            .done(),
        B::new("Frozen Metal", "pad", &["pad", "spectral", "metallic"])
            .sampled(1, SPECTRAL, "crash")
            .spectral(1, 0.0, 0.35, 0.1, 0.1)
            .p("osc1_loop", 1.0)
            .unison(1, 3.0, 0.15)
            .lfo_hz(1, 0.0, 0.2)
            .route(LFO1, "osc1_spectral_stretch", 0.2)
            .amp(0.7, 1.0, 0.8, 2.5)
            .reverb(0.4, 0.8)
            .done(),
        B::new("Grain Drift", "pad", &["pad", "granular", "texture"])
            .sampled(1, GRAINS, "tom")
            .grains(1, 0.2, 24.0, 0.8, 0.05)
            .p("osc1_grain_pitch", 0.3)
            .p("osc1_root", 52.0)
            .p("osc1_level", 1.0)
            .unison(1, 2.0, 0.3)
            .amp(0.9, 1.0, 0.9, 3.0)
            .delay(0.25, 0.4)
            .done(),
        // Plucks.
        B::new("Tom Pluck", "pluck", &["pluck", "sample", "percussive"])
            .sampled(1, SAMPLE, "tom")
            .p("osc1_root", 52.0)
            .filter(1, ft::LP24, 2_200.0, 0.25)
            .env(2, 0.0, 0.25, 0.0, 0.2)
            .route(ENV2, "filter1_cutoff", 0.5)
            .amp(0.0, 0.5, 0.0, 0.3)
            .done(),
        B::new("Rim Mallet", "pluck", &["pluck", "sample", "fm"])
            .sampled(1, SAMPLE, "rim")
            .p("osc1_root", 72.0)
            .osc(2, "basic", 0.0)
            .p("osc2_level", 0.0)
            .p("osc1_warp1", warp::FM)
            .p("osc1_warp1_amount", 0.15)
            .amp(0.0, 0.35, 0.0, 0.25)
            .reverb(0.2, 0.4)
            .done(),
        B::new("Spectral Harp", "pluck", &["pluck", "spectral", "bright"])
            .sampled(1, SPECTRAL, "tom")
            .spectral(1, 0.0, 0.0, 0.2, 2.0)
            .amp(0.0, 0.8, 0.0, 0.6)
            .reverb(0.3, 0.6)
            .done(),
        // Keys.
        B::new("Tuned Toms", "keys", &["keys", "multisample", "percussive"])
            .zone(1, "kick", 0, 47, 36)
            .zone(1, "tom", 48, 71, 60)
            .zone(1, "rim", 72, 127, 84)
            .amp(0.0, 0.6, 0.3, 0.4)
            .done(),
        B::new("Sampled Bells", "keys", &["keys", "spectral", "bell"])
            .sampled(1, SPECTRAL, "crash")
            .spectral(1, 0.0, 0.5, -0.1, 0.5)
            .amp(0.0, 1.5, 0.0, 1.5)
            .reverb(0.35, 0.7)
            .done(),
        // Effects.
        B::new("Crash Stretch", "fx", &["fx", "sample", "stretch"])
            .sampled(1, SAMPLE, "crash")
            .p("osc1_sample_mode", 2.0)
            .p("osc1_grain_size", 0.08)
            .amp(0.0, 2.0, 0.0, 2.0)
            .done(),
        B::new("Granular Rain", "fx", &["fx", "granular", "noise"])
            .sampled(1, GRAINS, "hat")
            .grains(1, 0.03, 80.0, 1.0, 0.5)
            .p("osc1_grain_pitch", 12.0)
            .unison(1, 2.0, 0.4)
            .p("osc1_width", 1.0)
            .amp(0.5, 1.0, 1.0, 1.5)
            .reverb(0.5, 0.9)
            .done(),
        B::new("Shift Drone", "fx", &["fx", "spectral", "drone"])
            .sampled(1, SPECTRAL, "open_hat")
            .spectral(1, 180.0, 0.0, 0.0, 0.2)
            .p("osc1_loop", 1.0)
            .unison(1, 3.0, 0.2)
            .amp(1.0, 1.0, 1.0, 2.0)
            .delay(0.3, 0.5)
            .done(),
        // Drums.
        B::new("Kit Zones", "drums", &["drums", "multisample", "kit"])
            .zone(1, "kick", 0, 40, 36)
            .zone(1, "snare", 41, 50, 38)
            .zone(1, "hat", 51, 60, 42)
            .zone(1, "clap", 61, 127, 39)
            .amp(0.0, 0.5, 0.0, 0.3)
            .done(),
        B::new("Crunch Kick", "drums", &["drums", "sample", "lo-fi"])
            .sampled(1, SAMPLE, "kick")
            .p("osc1_sample_mode", 1.0)
            .p("osc1_warp1", warp::QUANTIZE)
            .p("osc1_warp1_amount", 0.6)
            .amp(0.0, 0.4, 0.0, 0.2)
            .done(),
    ]
}

/// The factory preset called `name` (case-insensitive).
#[must_use]
pub fn find(name: &str) -> Option<&'static Patch> {
    factory().iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

fn bass() -> Vec<Patch> {
    let c = "bass";
    let mut out = Vec::new();
    for (i, (name, shape, octave_level)) in [
        ("Deep Sub", 0.0, 0.9),
        ("Round Sub", 1.0, 0.8),
        ("Sub Thump", 0.0, 1.0),
        ("Warm Sub", 1.0, 0.6),
        ("Sub Growl", 2.0, 0.7),
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["sub", "clean"])
                .osc(1, "basic", 0.0 + 0.1 * fi)
                .p("osc1_level", 0.5)
                .ps(&[
                    ("sub_on", 1.0),
                    ("sub_shape", shape),
                    ("sub_level", octave_level),
                ])
                .amp(0.002, 0.3 + 0.1 * fi, 0.7, 0.08)
                .filter(1, ft::LP24, 400.0 + 150.0 * fi, 0.15)
                .mono(0.03 * fi)
                .done(),
        );
    }
    for (i, name) in [
        "Glass Reese",
        "Night Reese",
        "Wide Reese",
        "Rusty Reese",
        "Slow Reese",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["reese", "dark", "detuned"])
                .osc(1, "analog", 0.0)
                .unison(1, 5.0 + fi, 0.2 + 0.05 * fi)
                .osc(2, "analog", 0.1)
                .p("osc2_octave", -1.0)
                .p("osc2_fine", 7.0)
                .amp(0.005, 0.5, 0.9, 0.15)
                .filter(1, ft::LP24, 500.0 + 200.0 * fi, 0.25)
                .p("osc2_filter1", 1.0)
                .lfo_hz(1, 0.0, 0.1 + 0.08 * fi)
                .route(LFO1, "filter1_cutoff", 0.08)
                .fx("distortion", &[("drive", 4.0 + fi * 2.0), ("mix", 0.3)])
                .fx("dimension", &[("size", 0.6), ("mix", 0.3)])
                .mono(0.04)
                .done(),
        );
    }
    for (i, (name, sync)) in [
        ("Wobble Eighths", rate::EIGHTH),
        ("Wobble Sixteenths", rate::SIXTEENTH),
        ("Wobble Triplets", rate::EIGHTH_TRIPLET),
        ("Wobble Halves", rate::HALF),
        ("Wobble Vowel", rate::QUARTER),
    ]
    .into_iter()
    .enumerate()
    {
        let vowel = i == 4;
        out.push(
            B::new(name, c, &["wobble", "dubstep", "lfo"])
                .osc(1, "analog", 0.2)
                .unison(1, 3.0, 0.15)
                .ps(&[("sub_on", 1.0), ("sub_level", 0.6)])
                .amp(0.003, 0.4, 1.0, 0.1)
                .filter(1, if vowel { ft::VOWEL } else { ft::LADDER }, 600.0, 0.5)
                .lfo(1, if i % 2 == 0 { 0.0 } else { 1.0 }, sync)
                .route(LFO1, "filter1_cutoff", 0.45)
                .fx("distortion", &[("drive", 6.0), ("mix", 0.4)])
                .mono(0.0)
                .done(),
        );
    }
    for (i, (name, kind, table)) in [
        ("Acid Line", ft::LADDER, "analog"),
        ("Acid Squelch", ft::DIODE, "analog"),
        ("Acid Glide", ft::LADDER, "pwm"),
        ("Diode Acid", ft::DIODE, "pwm"),
        ("Acid Square", ft::LADDER, "basic"),
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["acid", "303", "resonant"])
                .osc(1, table, if table == "basic" { 1.0 } else { 0.0 })
                .amp(0.002, 0.3, 0.6, 0.05)
                .filter(1, kind, 300.0 + 80.0 * fi, 0.75)
                .p("filter1_drive", 0.4)
                .env(2, 0.001, 0.18 + 0.04 * fi, 0.0, 0.1)
                .route(ENV2, "filter1_cutoff", 0.5)
                .route(source::VELOCITY, "filter1_resonance", 0.2)
                .mono(0.06 + 0.02 * fi)
                .delay(0.15, 0.35)
                .done(),
        );
    }
    for (i, (name, table, warp_kind)) in [
        ("FM Punch", "fm", warp::FM),
        ("Rubber Bass", "basic", warp::BEND_UP),
        ("Digital Bass", "harmonic_morph", warp::QUANTIZE),
        ("Sync Bass", "sync", warp::SYNC),
        ("Fold Bass", "basic", warp::MIRROR),
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["punchy", "modern"])
                .osc(1, table, 0.4)
                .p("osc1_warp1", warp_kind)
                .p("osc1_warp1_amount", 0.3)
                .ps(&[("sub_on", 1.0), ("sub_level", 0.5)])
                .amp(0.001, 0.25 + 0.05 * fi, 0.5, 0.08)
                .env(2, 0.001, 0.15, 0.0, 0.1)
                .route(ENV2, "osc1_warp1_amount", 0.5)
                .route(ENV2, "osc1_position", 0.3)
                .filter(1, ft::LP12, 2_000.0, 0.2)
                .fx("compressor", &[])
                .mono(0.0)
                .done(),
        );
    }
    out
}

fn lead() -> Vec<Patch> {
    let c = "lead";
    let mut out = Vec::new();
    for (i, name) in [
        "Hyper Saw",
        "Stadium Saw",
        "Bright Saw",
        "Soft Saw",
        "Octave Saw",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["supersaw", "trance", "wide"])
                .osc(1, "analog", 0.0)
                .unison(1, 7.0 + fi * 2.0, 0.25 + 0.04 * fi)
                .p("osc1_width", 1.0)
                .p("osc1_stack", if i == 4 { 1.0 } else { 0.0 })
                .amp(0.01, 0.4, 0.85, 0.3)
                .filter(1, ft::LP12, 6_000.0 - 800.0 * fi, 0.1)
                .fx("dimension", &[("mix", 0.25)])
                .delay(0.2, 0.4)
                .reverb(0.2, 2.5)
                .done(),
        );
    }
    for (i, name) in [
        "Sync Scream",
        "Sync Bite",
        "Sync Sweep",
        "Sync Pulse",
        "Sync Wire",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["sync", "aggressive"])
                .osc(1, if i == 3 { "pwm" } else { "basic" }, 0.66)
                .p("osc1_warp1", warp::SYNC)
                .p("osc1_warp1_amount", 0.2 + 0.1 * fi)
                .env(2, 0.01, 0.4 + 0.2 * fi, 0.2, 0.3)
                .route(ENV2, "osc1_warp1_amount", 0.4)
                .route(source::MOD_WHEEL, "osc1_warp1_amount", 0.4)
                .amp(0.005, 0.3, 0.9, 0.2)
                .mono(0.05)
                .fx("distortion", &[("mix", 0.2)])
                .delay(0.18, 0.3)
                .done(),
        );
    }
    for (i, name) in ["FM Lead", "Bell Lead", "Metal Lead", "Fluty FM", "Growl FM"]
        .into_iter()
        .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["fm", "digital"])
                .osc(1, "fm", 0.2 + 0.15 * fi)
                .osc(2, "basic", 0.0)
                .p("osc2_semi", [12.0, 7.0, 19.0, 12.0, -12.0][i])
                .p("osc1_warp1", warp::FM)
                .p("osc1_warp1_amount", 0.15 + 0.05 * fi)
                .amp(0.005, 0.6, 0.7, 0.3)
                .lfo_hz(1, 0.0, 5.0)
                .p("lfo1_delay", 0.3)
                .p("lfo1_rise", 0.4)
                .route(LFO1, "osc1_fine", 0.08)
                .mono(0.04)
                .reverb(0.18, 2.0)
                .done(),
        );
    }
    for (i, name) in [
        "Square Lead",
        "Pulse Lead",
        "Chip Lead",
        "Hollow Lead",
        "Wide Pulse",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["square", "pwm", "retro"])
                .osc(1, "pwm", 0.1 + 0.2 * fi)
                .lfo_hz(1, 1.0, 0.4 + 0.3 * fi)
                .route(LFO1, "osc1_position", 0.2)
                .unison(1, if i == 4 { 4.0 } else { 1.0 }, 0.1)
                .amp(0.002, 0.3, 0.8, 0.15)
                .filter(1, ft::LP12, 4_000.0, 0.2)
                .p("arp_mode", if i == 2 { 1.0 } else { 0.0 })
                .p("arp_rate", 0.0)
                .delay(0.12, 0.25)
                .done(),
        );
    }
    for (i, name) in ["Vox Lead", "Ah Lead", "Oh Lead", "Talk Lead", "Choir Lead"]
        .into_iter()
        .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["vocal", "formant"])
                .osc(1, "formant", 0.2 * fi)
                .unison(1, 3.0, 0.1)
                .lfo_hz(1, 0.0, 0.3 + 0.1 * fi)
                .route(LFO1, "osc1_position", 0.15)
                .route(source::MOD_WHEEL, "osc1_position", 0.5)
                .filter(1, ft::FORMANT, 1_200.0, 0.4)
                .amp(0.02, 0.5, 0.85, 0.35)
                .mono(0.06)
                .reverb(0.25, 3.0)
                .done(),
        );
    }
    out
}

fn pad() -> Vec<Patch> {
    let c = "pad";
    let mut out = Vec::new();
    for (i, name) in [
        "Warm Blanket",
        "Velvet Pad",
        "Analog Dream",
        "Soft Haze",
        "Golden Hour",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["warm", "analog", "lush"])
                .osc(1, "analog", 0.3 + 0.1 * fi)
                .unison(1, 6.0, 0.2)
                .osc(2, "analog", 0.6)
                .p("osc2_octave", 1.0)
                .p("osc2_level", 0.4)
                .amp(0.8 + 0.2 * fi, 1.0, 0.9, 1.5)
                .filter(1, ft::LP24, 1_500.0 + 300.0 * fi, 0.2)
                .p("osc2_filter1", 1.0)
                .lfo_hz(1, 0.0, 0.15)
                .route(LFO1, "filter1_cutoff", 0.1)
                .fx("chorus", &[("mix", 0.4)])
                .reverb(0.4, 4.0)
                .done(),
        );
    }
    for (i, name) in [
        "Glass Air",
        "Crystal Field",
        "Ice Cathedral",
        "Prism",
        "Clear Sky",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["glass", "bright", "additive"])
                .osc(1, "additive", 0.3 + 0.1 * fi)
                .osc(2, "harmonic_morph", 0.2)
                .p("osc2_octave", 1.0)
                .p("osc2_level", 0.3)
                .unison(1, 4.0, 0.12)
                .amp(1.2, 1.0, 0.8, 2.0)
                .lfo_hz(1, 1.0, 0.07 + 0.02 * fi)
                .route(LFO1, "osc1_position", 0.3)
                .fx("eq", &[])
                .reverb(0.5, 6.0)
                .done(),
        );
    }
    for (i, name) in [
        "Slow Tide",
        "Drifting",
        "Shifting Sands",
        "Aurora",
        "Undertow",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        let table = ["noise", "harmonic_morph", "fm", "formant", "sync"][i];
        out.push(
            B::new(name, c, &["evolving", "motion", "cinematic"])
                .osc(1, table, 0.0)
                .unison(1, 5.0, 0.18)
                .lfo(1, 2.0, rate::BARS_4)
                .route(LFO1, "osc1_position", 0.9)
                .lfo_hz(2, 6.0, 0.2 + 0.05 * fi)
                .route(LFO2, "filter1_cutoff", 0.15)
                .amp(1.5, 1.0, 1.0, 2.5)
                .filter(1, ft::LP12, 2_500.0, 0.3)
                .fx("phaser", &[("mix", 0.3)])
                .delay(0.25, 0.5)
                .reverb(0.45, 7.0)
                .done(),
        );
    }
    for (i, name) in [
        "Choir Pad",
        "Distant Voices",
        "Hum",
        "Vowel Drift",
        "Monastery",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["choir", "vocal", "formant"])
                .osc(1, "formant", 0.25 * fi.min(4.0))
                .unison(1, 8.0, 0.15)
                .lfo_hz(1, 0.0, 0.05 + 0.03 * fi)
                .route(LFO1, "osc1_position", 0.4)
                .filter(1, ft::VOWEL, 900.0, 0.3)
                .amp(0.9, 1.0, 0.9, 2.0)
                .fx("chorus", &[("mix", 0.3)])
                .reverb(0.55, 8.0)
                .done(),
        );
    }
    for (i, name) in ["Dark Matter", "Abyss", "Coal Smoke", "Low Fog", "Black Ice"]
        .into_iter()
        .enumerate()
    {
        let fi = i as f32;
        out.push(
            B::new(name, c, &["dark", "noise", "ambient"])
                .osc(1, "analog", 0.8)
                .p("osc1_octave", -1.0)
                .unison(1, 4.0, 0.3)
                .ps(&[
                    ("noise_on", 1.0),
                    ("noise_type", 1.0 + (i % 2) as f32),
                    ("noise_level", 0.25),
                ])
                .filter(1, ft::LP24, 600.0 + 100.0 * fi, 0.35)
                .p("noise_filter1", 1.0)
                .lfo_hz(1, 6.0, 0.1)
                .route(LFO1, "filter1_cutoff", 0.2)
                .amp(2.0, 1.0, 1.0, 3.0)
                .fx("dimension", &[("mix", 0.4)])
                .reverb(0.5, 9.0)
                .done(),
        );
    }
    out
}

fn pluck() -> Vec<Patch> {
    let c = "pluck";
    let mut out = Vec::new();
    let tables = ["analog", "basic", "pwm", "fm", "additive"];
    for (i, name) in [
        "Pop Pluck",
        "Glass Pluck",
        "Pulse Pluck",
        "FM Pluck",
        "Harp Pluck",
        "Wide Pluck",
        "Tight Pluck",
        "Muted Pluck",
        "Bell Pluck",
        "Soft Pluck",
        "Dub Pluck",
        "Kalimba",
        "Marimba Tone",
        "Koto Line",
        "Arp Pluck",
        "Sixteenth Pluck",
        "Echo Pluck",
        "Short Stab",
        "Chord Stab",
        "Laser Pluck",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        let mut b = B::new(name, c, &["pluck", "short"])
            .osc(1, tables[i % tables.len()], (0.13 * fi) % 1.0)
            .unison(1, if i % 3 == 0 { 5.0 } else { 1.0 }, 0.15)
            .amp(
                0.001,
                0.15 + 0.03 * (i % 7) as f32,
                0.0,
                0.15 + 0.02 * (i % 5) as f32,
            )
            .filter(1, ft::LP12, 900.0 + 120.0 * (i % 6) as f32, 0.3)
            .env(2, 0.001, 0.12 + 0.02 * (i % 4) as f32, 0.0, 0.1)
            .route(ENV2, "filter1_cutoff", 0.45)
            .route(source::VELOCITY, "filter1_cutoff", 0.2);
        if (10..14).contains(&i) {
            b = b
                .osc(2, "basic", 0.0)
                .p("osc2_semi", 12.0)
                .p("osc2_level", 0.4);
        }
        if (14..16).contains(&i) {
            b = b.p("arp_mode", 1.0 + (i % 2) as f32).p("arp_range", 2.0);
        }
        if i == 19 {
            b = b
                .route(ENV2, "osc1_fine", 0.8)
                .p("osc1_warp1", warp::BEND_UP);
        }
        out.push(
            b.delay(0.2 + 0.02 * (i % 4) as f32, 0.3)
                .reverb(0.15 + 0.01 * fi, 1.5)
                .done(),
        );
    }
    out
}

fn keys() -> Vec<Patch> {
    let c = "keys";
    let mut out = Vec::new();
    for (i, name) in [
        "Electric Tines",
        "Soft Tines",
        "Bright Tines",
        "Dyno Keys",
        "Suitcase",
        "Drawbar One",
        "Drawbar Full",
        "Gospel Bars",
        "Jazz Bars",
        "Perc Organ",
        "Toy Piano",
        "Felt Keys",
        "Glass Keys",
        "Clav Snap",
        "Wurly Bark",
        "Bell Keys",
        "Celesta",
        "Music Box",
        "Vibes",
        "Steel Drum",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        let b = B::new(name, c, &["keys", "playable"]);
        let b = match i / 5 {
            // Electric pianos: FM tines over a soft sine body.
            0 => b
                .osc(1, "fm", 0.2 + 0.05 * fi)
                .osc(2, "basic", 0.0)
                .p("osc2_level", 0.6)
                .amp(0.002, 1.2 + 0.2 * fi, 0.2, 0.4)
                .env(2, 0.001, 0.3, 0.0, 0.2)
                .route(ENV2, "osc1_position", 0.3)
                .route(source::VELOCITY, "osc1_position", 0.3)
                .fx("chorus", &[("mix", 0.25)]),
            // Organs: additive drawbars, no decay.
            1 => b
                .osc(1, "additive", 0.05 + 0.08 * (fi - 5.0))
                .osc(2, "basic", 0.0)
                .p("osc2_octave", 1.0)
                .p("osc2_level", 0.3)
                .amp(0.005, 0.1, 1.0, 0.05)
                .lfo_hz(1, 0.0, 6.0)
                .route(LFO1, "osc1_pan", 0.2)
                .fx("distortion", &[("mix", 0.15)]),
            // Struck and plucked keys.
            2 => b
                .osc(
                    1,
                    ["basic", "additive", "harmonic_morph", "pwm", "analog"][i - 10],
                    0.4,
                )
                .amp(0.001, 0.6 + 0.1 * (fi - 10.0), 0.1, 0.3)
                .filter(1, ft::LP12, 2_500.0, 0.2)
                .env(2, 0.001, 0.25, 0.0, 0.2)
                .route(ENV2, "filter1_cutoff", 0.3),
            // Bells and mallets: inharmonic FM and ring modulation.
            _ => b
                .osc(1, "fm", 0.5 + 0.1 * (fi - 15.0))
                .p("osc1_warp1", warp::RM)
                .p("osc1_warp1_amount", 0.2)
                .osc(2, "basic", 0.0)
                .p("osc2_semi", [7.0, 12.0, 24.0, 19.0, 5.0][i - 15])
                .p("osc2_level", 0.5)
                .amp(0.001, 1.5, 0.0, 1.2),
        };
        out.push(b.reverb(0.2, 2.5).done());
    }
    out
}

fn fx() -> Vec<Patch> {
    let c = "fx";
    let mut out = Vec::new();
    for (i, name) in [
        "Riser",
        "Long Riser",
        "Downlifter",
        "White Sweep",
        "Laser Zap",
        "Siren",
        "Alarm Pulse",
        "Robot Talk",
        "Radio Static",
        "Space Drone",
        "Wind Tunnel",
        "Glitch Seq",
        "Pulse Seq",
        "Bubble Seq",
        "Impact Boom",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        let b = B::new(name, c, &["fx", "sound design"]);
        let b = match i {
            0..=3 => b
                .ps(&[
                    ("noise_on", 1.0),
                    ("noise_level", 0.6),
                    ("noise_filter1", 1.0),
                ])
                .osc(1, "sync", 0.0)
                .filter(1, ft::BP12, 400.0, 0.5)
                .env(2, 3.0 + 3.0 * fi, 0.1, 1.0, 1.0)
                .route(ENV2, "filter1_cutoff", if i == 2 { -0.6 } else { 0.7 })
                .route(ENV2, "osc1_position", 0.9)
                .amp(0.5, 0.5, 1.0, 1.5),
            4..=6 => b
                .osc(1, "basic", 0.66)
                .lfo(1, if i == 6 { 4.0 } else { 2.0 }, rate::SIXTEENTH)
                .route(LFO1, "osc1_fine", 1.0)
                .p("osc1_warp1", warp::FM)
                .p("osc1_warp1_amount", 0.4)
                .amp(0.001, 0.4, 0.7, 0.3),
            7..=10 => b
                .osc(
                    1,
                    ["formant", "noise", "harmonic_morph", "noise"][i - 7],
                    0.3,
                )
                .unison(1, 6.0, 0.4)
                .lfo_hz(1, 6.0, 2.0 + fi)
                .route(LFO1, "osc1_position", 0.6)
                .filter(1, ft::COMB, 800.0, 0.6)
                .amp(0.4, 1.0, 1.0, 2.0)
                .fx("flanger", &[("mix", 0.4)]),
            11..=13 => {
                let mut b = b
                    .osc(1, ["pwm", "fm", "basic"][i - 11], 0.5)
                    .p("seq_on", 1.0)
                    .p("seq_length", 16.0)
                    .amp(0.001, 0.12, 0.0, 0.08)
                    .filter(1, ft::LADDER, 1_200.0, 0.6)
                    .env(2, 0.001, 0.1, 0.0, 0.05)
                    .route(ENV2, "filter1_cutoff", 0.4);
                for step in 0..16_usize {
                    let transpose = [0, 12, 7, 0, 3, 15, 10, 5][(step * (i - 9)) % 8];
                    let on = (step * 5 + i) % 7 != 0;
                    b.patch.set_step(
                        step,
                        (
                            on,
                            transpose,
                            0.6 + 0.4 * ((step % 4 == 0) as u8 as f32),
                            0.4,
                        ),
                        (true, 0, 1.0, 0.5),
                    );
                }
                b
            }
            _ => b
                .osc(1, "basic", 0.0)
                .p("osc1_octave", -2.0)
                .ps(&[("noise_on", 1.0), ("noise_type", 2.0), ("noise_level", 0.5)])
                .amp(0.001, 2.0, 0.0, 2.0)
                .env(2, 0.001, 0.4, 0.0, 0.2)
                .route(ENV2, "osc1_fine", 0.6),
        };
        out.push(b.delay(0.2, 0.45).reverb(0.35, 5.0).done());
    }
    out
}

fn drums() -> Vec<Patch> {
    let c = "drums";
    let mut out = Vec::new();
    for (i, name) in [
        "Round Kick",
        "Punch Kick",
        "Long Kick",
        "Click Kick",
        "Dirty Kick",
        "Tight Snare",
        "Fat Snare",
        "Rim Shot",
        "Noise Snare",
        "Clap",
        "Closed Hat",
        "Open Hat",
        "Shaker",
        "Ride",
        "Crash Wash",
        "Low Tom",
        "High Tom",
        "Conga",
        "Cowbell",
        "Zap Perc",
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        let b = B::new(name, c, &["drums", "one-shot"]).p("polyphony", 4.0);
        let b = match i / 5 {
            // Kicks: a low sine with a pitch and FM burst at the start.
            0 => b
                .osc(1, "basic", 0.0)
                .p("osc1_octave", -2.0)
                .p("osc1_warp1", warp::FM)
                .env(2, 0.001, 0.03 + 0.01 * fi, 0.0, 0.02)
                .route(ENV2, "osc1_warp1_amount", 0.4 + 0.1 * fi)
                .route(ENV2, "osc1_fine", 1.0)
                .amp(0.001, 0.3 + 0.1 * fi, 0.0, 0.1)
                .fx("distortion", &[("mix", 0.1 * fi)]),
            // Snares and claps: noise through a band-pass over a short body.
            1 => b
                .osc(1, "basic", 0.33)
                .p("osc1_level", 0.4)
                .ps(&[
                    ("noise_on", 1.0),
                    ("noise_level", 0.8),
                    ("noise_filter1", 1.0),
                ])
                .filter(1, ft::BP12, 1_800.0 + 300.0 * (fi - 5.0), 0.3)
                .amp(0.001, 0.15 + 0.03 * (fi - 5.0), 0.0, 0.1),
            // Cymbals: bright noise and ring-modulated metal through a high-pass.
            2 => b
                .osc(1, "fm", 0.9)
                .p("osc1_warp1", warp::RM)
                .p("osc1_warp1_amount", 0.6)
                .p("osc1_level", 0.3)
                .ps(&[
                    ("noise_on", 1.0),
                    ("noise_level", 0.7),
                    ("noise_filter1", 1.0),
                ])
                .filter(1, ft::HP12, 6_000.0, 0.2)
                .amp(0.001, [0.05, 0.4, 0.08, 0.9, 1.6][i - 10], 0.0, 0.2),
            // Toms and percussion.
            _ => b
                .osc(
                    1,
                    if i == 18 { "pwm" } else { "basic" },
                    if i == 18 { 0.3 } else { 0.0 },
                )
                .p("osc1_octave", [-1.0, 0.0, 0.0, 1.0, 1.0][i - 15])
                .env(2, 0.001, 0.08, 0.0, 0.05)
                .route(ENV2, "osc1_fine", 0.7)
                .filter(1, ft::PHASER, 2_000.0, 0.2)
                .amp(0.001, 0.25, 0.0, 0.1),
        };
        out.push(b.fx("compressor", &[]).done());
    }
    out
}

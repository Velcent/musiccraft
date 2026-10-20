//! Which effects are exported and what hosts know them by. Hosts store these
//! IDs in their projects, so they never change (see the `frozen_ids` test).

use musiccraft_instruments::Kind;
use musiccraft_params::EffectKind;
use nih_plug::prelude::{ClapFeature, Vst3SubCategory};

/// Exported effects: every built-in effect except the ones the mixer routes
/// (`send`, `band_split`, `rack`), in menu order. Plugin `K` wraps `KINDS[K]`.
pub const KINDS: [EffectKind; 44] = [
    EffectKind::Filter,
    EffectKind::AutoFilter,
    EffectKind::Formant,
    EffectKind::Eq,
    EffectKind::GraphicEq,
    EffectKind::ParametricEq,
    EffectKind::Delay,
    EffectKind::MultiTap,
    EffectKind::Stutter,
    EffectKind::BeatShaper,
    EffectKind::Scratch,
    EffectKind::MultiFx,
    EffectKind::FxChain,
    EffectKind::Reverb,
    EffectKind::HallReverb,
    EffectKind::Convolution,
    EffectKind::Chorus,
    EffectKind::Phaser,
    EffectKind::Flanger,
    EffectKind::RingMod,
    EffectKind::FreqShift,
    EffectKind::PitchShift,
    EffectKind::PitchCorrect,
    EffectKind::Tuner,
    EffectKind::Vocoder,
    EffectKind::Compressor,
    EffectKind::Multiband,
    EffectKind::MultibandLimiter,
    EffectKind::Limiter,
    EffectKind::Dynamics,
    EffectKind::NoiseGate,
    EffectKind::Transient,
    EffectKind::SoftClipper,
    EffectKind::Gate,
    EffectKind::Autopan,
    EffectKind::Stereo,
    EffectKind::StereoShaper,
    EffectKind::DcBlock,
    EffectKind::Distortion,
    EffectKind::Overdrive,
    EffectKind::Waveshaper,
    EffectKind::GuitarChain,
    EffectKind::BassBoost,
    EffectKind::Bitcrusher,
];

/// CLAP IDs, by plugin index: `org.musiccraft.<effect name>`.
pub const CLAP_IDS: [&str; 44] = [
    "org.musiccraft.filter",
    "org.musiccraft.auto_filter",
    "org.musiccraft.formant",
    "org.musiccraft.eq",
    "org.musiccraft.graphic_eq",
    "org.musiccraft.parametric_eq",
    "org.musiccraft.delay",
    "org.musiccraft.multi_tap",
    "org.musiccraft.stutter",
    "org.musiccraft.beat_shaper",
    "org.musiccraft.scratch",
    "org.musiccraft.multi_fx",
    "org.musiccraft.fx_chain",
    "org.musiccraft.reverb",
    "org.musiccraft.hall_reverb",
    "org.musiccraft.convolution",
    "org.musiccraft.chorus",
    "org.musiccraft.phaser",
    "org.musiccraft.flanger",
    "org.musiccraft.ring_mod",
    "org.musiccraft.freq_shift",
    "org.musiccraft.pitch_shift",
    "org.musiccraft.pitch_correct",
    "org.musiccraft.tuner",
    "org.musiccraft.vocoder",
    "org.musiccraft.compressor",
    "org.musiccraft.multiband",
    "org.musiccraft.multiband_limiter",
    "org.musiccraft.limiter",
    "org.musiccraft.dynamics",
    "org.musiccraft.noise_gate",
    "org.musiccraft.transient",
    "org.musiccraft.soft_clipper",
    "org.musiccraft.gate",
    "org.musiccraft.autopan",
    "org.musiccraft.stereo",
    "org.musiccraft.stereo_shaper",
    "org.musiccraft.dc_block",
    "org.musiccraft.distortion",
    "org.musiccraft.overdrive",
    "org.musiccraft.waveshaper",
    "org.musiccraft.guitar_chain",
    "org.musiccraft.bass_boost",
    "org.musiccraft.bitcrusher",
];

/// Exported instruments besides Antiserum, in menu order
/// (`docs/spec/plugin-instruments.md`). Instrument plugin `K` plays
/// `INSTRUMENT_KINDS[K]`.
pub const INSTRUMENT_KINDS: [Kind; 11] = [
    Kind::Synth,
    Kind::DrumSynth,
    Kind::Pluck,
    Kind::Fm,
    Kind::Keys,
    Kind::Additive,
    Kind::Sampler,
    Kind::Granular,
    Kind::Resynth,
    Kind::Kit,
    Kind::Zones,
];

/// Instrument names as project files write them, by instrument plugin.
pub const INSTRUMENT_NAMES: [&str; 11] = [
    "synth",
    "drum_synth",
    "pluck",
    "fm",
    "keys",
    "additive",
    "sampler",
    "granular",
    "resynth",
    "kit",
    "zones",
];

/// Plugin names, by instrument plugin.
pub const INSTRUMENT_TITLES: [&str; 11] = [
    "Synth",
    "Drum synth",
    "Pluck",
    "FM",
    "Keys",
    "Additive",
    "Sampler",
    "Granular",
    "Resynth",
    "Kit",
    "Zones",
];

/// CLAP IDs, by instrument plugin: `org.musiccraft.<instrument name>`.
pub const INSTRUMENT_CLAP_IDS: [&str; 11] = [
    "org.musiccraft.synth",
    "org.musiccraft.drum_synth",
    "org.musiccraft.pluck",
    "org.musiccraft.fm",
    "org.musiccraft.keys",
    "org.musiccraft.additive",
    "org.musiccraft.sampler",
    "org.musiccraft.granular",
    "org.musiccraft.resynth",
    "org.musiccraft.kit",
    "org.musiccraft.zones",
];

/// Antiserum's CLAP ID.
pub const ANTISERUM_CLAP_ID: &str = "org.musiccraft.antiserum";

/// The VST3 class ID for an effect's CLAP ID: `MCfx` and the first 12 bytes
/// of the ID's 128-bit FNV-1a hash.
pub const fn vst3_class_id(clap_id: &str) -> [u8; 16] {
    class_id(*b"MCfx", clap_id)
}

/// The VST3 class ID for an instrument's CLAP ID: `MCin` and the same hash.
pub const fn vst3_instrument_class_id(clap_id: &str) -> [u8; 16] {
    class_id(*b"MCin", clap_id)
}

/// How an instrument is listed: a synthesizer, a sampler or a drum machine.
pub const fn instrument_features(kind: Kind) -> &'static [ClapFeature] {
    use ClapFeature::{Instrument, Stereo};
    match kind {
        Kind::Sampler | Kind::Granular | Kind::Resynth | Kind::Zones => {
            &[Instrument, ClapFeature::Sampler, Stereo]
        }
        Kind::Kit | Kind::DrumSynth => &[Instrument, ClapFeature::DrumMachine, Stereo],
        _ => &[Instrument, ClapFeature::Synthesizer, Stereo],
    }
}

/// VST3 subcategories of an instrument.
pub const fn instrument_subcategories(kind: Kind) -> &'static [Vst3SubCategory] {
    use Vst3SubCategory as V;
    match kind {
        Kind::Sampler | Kind::Granular | Kind::Resynth | Kind::Zones => {
            &[V::Instrument, V::Sampler]
        }
        Kind::Kit | Kind::DrumSynth => &[V::Instrument, V::Drum],
        _ => &[V::Instrument, V::Synth],
    }
}

const fn class_id(prefix: [u8; 4], clap_id: &str) -> [u8; 16] {
    const OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
    let bytes = clap_id.as_bytes();
    let mut hash = OFFSET;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u128;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }
    let digest = hash.to_be_bytes();
    let mut id = [0; 16];
    id[0] = prefix[0];
    id[1] = prefix[1];
    id[2] = prefix[2];
    id[3] = prefix[3];
    let mut j = 0;
    while j < 12 {
        id[4 + j] = digest[j];
        j += 1;
    }
    id
}

/// What kind of effect a host lists it as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Filter,
    Equalizer,
    Delay,
    Reverb,
    Chorus,
    Phaser,
    Flanger,
    FrequencyShifter,
    PitchShifter,
    PitchCorrection,
    Analyzer,
    Compressor,
    TransientShaper,
    Limiter,
    Gate,
    Tremolo,
    Glitch,
    MultiEffects,
    Distortion,
    Utility,
}

const fn category(kind: EffectKind) -> Category {
    use EffectKind as K;
    match kind {
        K::Filter | K::AutoFilter | K::Formant | K::Vocoder => Category::Filter,
        K::Eq | K::GraphicEq | K::ParametricEq | K::BassBoost => Category::Equalizer,
        K::Delay | K::MultiTap => Category::Delay,
        K::Reverb | K::HallReverb | K::Convolution => Category::Reverb,
        K::Chorus => Category::Chorus,
        K::Phaser => Category::Phaser,
        K::Flanger => Category::Flanger,
        K::FreqShift | K::RingMod => Category::FrequencyShifter,
        K::PitchShift => Category::PitchShifter,
        K::PitchCorrect => Category::PitchCorrection,
        K::Tuner => Category::Analyzer,
        K::Compressor | K::Multiband | K::Dynamics => Category::Compressor,
        K::Transient => Category::TransientShaper,
        K::Limiter | K::MultibandLimiter | K::SoftClipper => Category::Limiter,
        K::NoiseGate => Category::Gate,
        K::Gate | K::Autopan => Category::Tremolo,
        K::Stutter | K::BeatShaper | K::Scratch => Category::Glitch,
        K::MultiFx | K::FxChain | K::GuitarChain => Category::MultiEffects,
        K::Distortion | K::Overdrive | K::Waveshaper | K::Bitcrusher => Category::Distortion,
        K::Stereo | K::StereoShaper | K::DcBlock => Category::Utility,
        K::Send | K::BandSplit | K::Rack | K::Plugin => Category::Utility,
    }
}

/// CLAP features: an audio effect, stereo, and its category.
pub const fn clap_features(kind: EffectKind) -> &'static [ClapFeature] {
    use ClapFeature::{AudioEffect as Fx, Stereo};
    match category(kind) {
        Category::Filter => &[Fx, Stereo, ClapFeature::Filter],
        Category::Equalizer => &[Fx, Stereo, ClapFeature::Equalizer],
        Category::Delay => &[Fx, Stereo, ClapFeature::Delay],
        Category::Reverb => &[Fx, Stereo, ClapFeature::Reverb],
        Category::Chorus => &[Fx, Stereo, ClapFeature::Chorus],
        Category::Phaser => &[Fx, Stereo, ClapFeature::Phaser],
        Category::Flanger => &[Fx, Stereo, ClapFeature::Flanger],
        Category::FrequencyShifter => &[Fx, Stereo, ClapFeature::FrequencyShifter],
        Category::PitchShifter => &[Fx, Stereo, ClapFeature::PitchShifter],
        Category::PitchCorrection => &[Fx, Stereo, ClapFeature::PitchCorrection],
        Category::Analyzer => &[Fx, Stereo, ClapFeature::Analyzer],
        Category::Compressor => &[Fx, Stereo, ClapFeature::Compressor],
        Category::TransientShaper => &[Fx, Stereo, ClapFeature::TransientShaper],
        Category::Limiter => &[Fx, Stereo, ClapFeature::Limiter],
        Category::Gate => &[Fx, Stereo, ClapFeature::Gate],
        Category::Tremolo => &[Fx, Stereo, ClapFeature::Tremolo],
        Category::Glitch => &[Fx, Stereo, ClapFeature::Glitch],
        Category::MultiEffects => &[Fx, Stereo, ClapFeature::MultiEffects],
        Category::Distortion => &[Fx, Stereo, ClapFeature::Distortion],
        Category::Utility => &[Fx, Stereo, ClapFeature::Utility],
    }
}

/// VST3 subcategories: `Fx` and the closest category.
pub const fn vst3_subcategories(kind: EffectKind) -> &'static [Vst3SubCategory] {
    use Vst3SubCategory as V;
    match category(kind) {
        Category::Filter => &[V::Fx, V::Filter],
        Category::Equalizer => &[V::Fx, V::Eq],
        Category::Delay => &[V::Fx, V::Delay],
        Category::Reverb => &[V::Fx, V::Reverb],
        Category::Chorus | Category::Phaser | Category::Flanger | Category::Tremolo => {
            &[V::Fx, V::Modulation]
        }
        Category::FrequencyShifter | Category::PitchShifter | Category::PitchCorrection => {
            &[V::Fx, V::PitchShift]
        }
        Category::Analyzer => &[V::Fx, V::Analyzer],
        Category::Compressor | Category::TransientShaper | Category::Limiter | Category::Gate => {
            &[V::Fx, V::Dynamics]
        }
        Category::Glitch | Category::MultiEffects => &[V::Fx],
        Category::Distortion => &[V::Fx, V::Distortion],
        Category::Utility => &[V::Fx, V::Tools],
    }
}

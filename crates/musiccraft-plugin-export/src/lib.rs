//! MusicCraft's built-in effects and instruments as CLAP and VST3 plugins
//! (and standalone apps), built on nih-plug. See `docs/spec/plugin-export.md`
//! and `docs/spec/plugin-instruments.md`.
//!
//! Every exported effect is `EffectPlugin<K>`, a thin wrapper around the same
//! `musiccraft_dsp::Effect` the mixer runs; every built-in instrument is
//! `InstrumentPlugin<K>` around the app's voices; Antiserum is
//! `AntiserumPlugin`. Host parameters come from the same `ParamSpec` tables
//! as the app's, and the editors are the app's own.
//!
//! The `effects` feature exports the effects (the **MusicCraft Effects**
//! bundle) and `instruments` the instruments (**MusicCraft Instruments**);
//! both are on by default, for tests and the standalone apps.
//!
//! This crate may use `unsafe` (roadmap decision D1): nih-plug's export
//! macros emit the C entry points, and `Params` is an unsafe trait.

pub mod antiserum;
mod editor;
pub mod handoff;
pub mod ids;
pub mod instrument;
pub mod params;
pub mod plugin;
pub mod shared;

pub use antiserum::AntiserumPlugin;
pub use instrument::InstrumentPlugin;
pub use plugin::EffectPlugin;

/// Exports the plugin types listed, as CLAP and VST3.
#[allow(unused_macros)]
macro_rules! export {
    ($($plugin:ty),+ $(,)?) => {
        /// The C entry points. nih-plug's VST3 export expands vst3-sys's
        /// `vtable!` macro, whose trailing semicolons newer compilers warn
        /// about; that code is not ours, so the lint is allowed here only.
        #[allow(semicolon_in_expressions_from_non_local_macros)]
        mod entry_points {
            use super::*;
            nih_plug::nih_export_clap!($($plugin),+);
            nih_plug::nih_export_vst3!($($plugin),+);
        }
    };
}

/// Exports every effect, followed by the plugin types listed.
#[allow(unused_macros)]
macro_rules! effects_and {
    ($($tail:ty),*) => {
        export!(
            EffectPlugin<0>,
            EffectPlugin<1>,
            EffectPlugin<2>,
            EffectPlugin<3>,
            EffectPlugin<4>,
            EffectPlugin<5>,
            EffectPlugin<6>,
            EffectPlugin<7>,
            EffectPlugin<8>,
            EffectPlugin<9>,
            EffectPlugin<10>,
            EffectPlugin<11>,
            EffectPlugin<12>,
            EffectPlugin<13>,
            EffectPlugin<14>,
            EffectPlugin<15>,
            EffectPlugin<16>,
            EffectPlugin<17>,
            EffectPlugin<18>,
            EffectPlugin<19>,
            EffectPlugin<20>,
            EffectPlugin<21>,
            EffectPlugin<22>,
            EffectPlugin<23>,
            EffectPlugin<24>,
            EffectPlugin<25>,
            EffectPlugin<26>,
            EffectPlugin<27>,
            EffectPlugin<28>,
            EffectPlugin<29>,
            EffectPlugin<30>,
            EffectPlugin<31>,
            EffectPlugin<32>,
            EffectPlugin<33>,
            EffectPlugin<34>,
            EffectPlugin<35>,
            EffectPlugin<36>,
            EffectPlugin<37>,
            EffectPlugin<38>,
            EffectPlugin<39>,
            EffectPlugin<40>,
            EffectPlugin<41>,
            EffectPlugin<42>,
            EffectPlugin<43>
            $(, $tail)*
        );
    };
}

// Plugin `K` of each kind is its wrapper with index `K` in `ids`.
#[cfg(all(feature = "effects", feature = "instruments"))]
effects_and!(
    InstrumentPlugin<0>,
    InstrumentPlugin<1>,
    InstrumentPlugin<2>,
    InstrumentPlugin<3>,
    InstrumentPlugin<4>,
    InstrumentPlugin<5>,
    InstrumentPlugin<6>,
    InstrumentPlugin<7>,
    InstrumentPlugin<8>,
    InstrumentPlugin<9>,
    InstrumentPlugin<10>,
    AntiserumPlugin
);

#[cfg(all(feature = "effects", not(feature = "instruments")))]
effects_and!();

#[cfg(all(feature = "instruments", not(feature = "effects")))]
export!(
    InstrumentPlugin<0>,
    InstrumentPlugin<1>,
    InstrumentPlugin<2>,
    InstrumentPlugin<3>,
    InstrumentPlugin<4>,
    InstrumentPlugin<5>,
    InstrumentPlugin<6>,
    InstrumentPlugin<7>,
    InstrumentPlugin<8>,
    InstrumentPlugin<9>,
    InstrumentPlugin<10>,
    AntiserumPlugin
);

#[cfg(test)]
mod tests;

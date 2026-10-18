//! MusicCraft's built-in effects as CLAP and VST3 plugins (and standalone
//! apps), built on nih-plug. See `docs/spec/plugin-export.md`.
//!
//! Every exported effect is `EffectPlugin<K>`, a thin wrapper around the same
//! `musiccraft_dsp::Effect` the mixer runs, with host parameters derived
//! from the effect's `ParamSpec` table and the shared egui editor. One
//! library exports all of them.
//!
//! This crate may use `unsafe` (roadmap decision D1): nih-plug's export
//! macros emit the C entry points, and `Params` is an unsafe trait.

mod editor;
pub mod ids;
pub mod params;
pub mod plugin;
pub mod shared;

pub use plugin::EffectPlugin;

// Plugin `K` is `EffectPlugin<K>` for every index of `ids::KINDS`.
nih_plug::nih_export_clap!(
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
);

nih_plug::nih_export_vst3!(
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
);

#[cfg(test)]
mod tests;

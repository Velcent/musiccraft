// Event ids, ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto, GPL-3.0-or-later. Generated from PyFLP's own
// `EventEnum` subclasses, in the order its parser consults them; members
// whose name starts with `_` there are deprecated in recent FL versions.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.

//! Event ids by model, as PyFLP's `EventEnum` subclasses define them.

#![allow(missing_docs)]

use crate::events::Declared;

/// PyFLP `PluginID`.
pub mod plugin {
    pub const COLOR: u8 = 128;
    pub const ICON: u8 = 155;
    pub const INTERNAL_NAME: u8 = 201;
    pub const NAME: u8 = 203;
    pub const WRAPPER: u8 = 212;
    pub const DATA: u8 = 213;
    /// Every id in this group.
    pub const ALL: &[u8] = &[COLOR, ICON, INTERNAL_NAME, NAME, WRAPPER, DATA];
}

/// PyFLP `ChannelID`.
pub mod channel {
    pub const IS_ENABLED: u8 = 0;
    pub const VOL_BYTE: u8 = 2;
    pub const PAN_BYTE: u8 = 3;
    pub const ZIPPED: u8 = 15;
    pub const PING_PONG_LOOP: u8 = 20;
    pub const TYPE: u8 = 21;
    pub const ROUTED_TO: u8 = 22;
    pub const IS_LOCKED: u8 = 32;
    pub const NEW: u8 = 64;
    pub const FREQ_TILT: u8 = 69;
    pub const FX_FLAGS: u8 = 70;
    pub const CUTOFF: u8 = 71;
    pub const VOL_WORD: u8 = 72;
    pub const PAN_WORD: u8 = 73;
    pub const PREAMP: u8 = 74;
    pub const FADE_OUT: u8 = 75;
    pub const FADE_IN: u8 = 76;
    pub const RESONANCE: u8 = 83;
    pub const STEREO_DELAY: u8 = 85;
    pub const POGO: u8 = 86;
    pub const TIME_SHIFT: u8 = 89;
    pub const CHILDREN: u8 = 94;
    pub const SWING: u8 = 97;
    pub const RING_MOD: u8 = 131;
    pub const CUT_GROUP: u8 = 132;
    pub const ROOT_NOTE: u8 = 135;
    pub const DELAY_MOD_XY: u8 = 138;
    pub const REVERB: u8 = 139;
    pub const STRETCH_TIME: u8 = 140;
    pub const FINE_TUNE: u8 = 142;
    pub const SAMPLER_FLAGS: u8 = 143;
    pub const LAYER_FLAGS: u8 = 144;
    pub const GROUP_NUM: u8 = 145;
    pub const AU_SAMPLE_RATE: u8 = 153;
    pub const NAME: u8 = 192;
    pub const SAMPLE_PATH: u8 = 196;
    pub const DELAY: u8 = 209;
    pub const PARAMETERS: u8 = 215;
    pub const ENVELOPE_LFO: u8 = 218;
    pub const LEVELS: u8 = 219;
    pub const POLYPHONY: u8 = 221;
    pub const TRACKING: u8 = 228;
    pub const LEVEL_ADJUSTS: u8 = 229;
    pub const AUTOMATION: u8 = 234;
    /// Every id in this group.
    pub const ALL: &[u8] = &[
        IS_ENABLED,
        VOL_BYTE,
        PAN_BYTE,
        ZIPPED,
        PING_PONG_LOOP,
        TYPE,
        ROUTED_TO,
        IS_LOCKED,
        NEW,
        FREQ_TILT,
        FX_FLAGS,
        CUTOFF,
        VOL_WORD,
        PAN_WORD,
        PREAMP,
        FADE_OUT,
        FADE_IN,
        RESONANCE,
        STEREO_DELAY,
        POGO,
        TIME_SHIFT,
        CHILDREN,
        SWING,
        RING_MOD,
        CUT_GROUP,
        ROOT_NOTE,
        DELAY_MOD_XY,
        REVERB,
        STRETCH_TIME,
        FINE_TUNE,
        SAMPLER_FLAGS,
        LAYER_FLAGS,
        GROUP_NUM,
        AU_SAMPLE_RATE,
        NAME,
        SAMPLE_PATH,
        DELAY,
        PARAMETERS,
        ENVELOPE_LFO,
        LEVELS,
        POLYPHONY,
        TRACKING,
        LEVEL_ADJUSTS,
        AUTOMATION,
    ];
}

/// PyFLP `DisplayGroupID`.
pub mod display_group {
    pub const NAME: u8 = 231;
    /// Every id in this group.
    pub const ALL: &[u8] = &[NAME];
}

/// PyFLP `RackID`.
pub mod rack {
    pub const SWING: u8 = 11;
    pub const FIT_TO_STEPS: u8 = 13;
    pub const WINDOW_HEIGHT: u8 = 133;
    /// Every id in this group.
    pub const ALL: &[u8] = &[SWING, FIT_TO_STEPS, WINDOW_HEIGHT];
}

/// PyFLP `TimeMarkerID`.
pub mod time_marker {
    pub const NUMERATOR: u8 = 33;
    pub const DENOMINATOR: u8 = 34;
    pub const POSITION: u8 = 148;
    pub const NAME: u8 = 205;
    /// Every id in this group.
    pub const ALL: &[u8] = &[NUMERATOR, DENOMINATOR, POSITION, NAME];
}

/// PyFLP `PatternsID`.
pub mod patterns {
    pub const PLAY_TRUNCATED_NOTES: u8 = 30;
    pub const CURRENTLY_SELECTED: u8 = 67;
    /// Every id in this group.
    pub const ALL: &[u8] = &[PLAY_TRUNCATED_NOTES, CURRENTLY_SELECTED];
}

/// PyFLP `PatternID`.
pub mod pattern {
    pub const LOOPED: u8 = 26;
    pub const NEW: u8 = 65;
    pub const COLOR: u8 = 150;
    pub const NAME: u8 = 193;
    pub const CHANNEL_IID: u8 = 160;
    pub const N161: u8 = 161;
    pub const N162: u8 = 162;
    pub const LENGTH: u8 = 164;
    pub const CONTROLLERS: u8 = 223;
    pub const NOTES: u8 = 224;
    /// Every id in this group.
    pub const ALL: &[u8] = &[
        LOOPED,
        NEW,
        COLOR,
        NAME,
        CHANNEL_IID,
        N161,
        N162,
        LENGTH,
        CONTROLLERS,
        NOTES,
    ];
}

/// PyFLP `ArrangementsID`.
pub mod arrangements {
    pub const TIME_SIG_NUM: u8 = 17;
    pub const TIME_SIG_BEAT: u8 = 18;
    pub const CURRENT: u8 = 100;
    pub const LOOP_POS: u8 = 152;
    pub const PL_SELECTION: u8 = 217;
    /// Every id in this group.
    pub const ALL: &[u8] = &[TIME_SIG_NUM, TIME_SIG_BEAT, CURRENT, LOOP_POS, PL_SELECTION];
}

/// PyFLP `ArrangementID`.
pub mod arrangement {
    pub const NEW: u8 = 99;
    pub const NAME: u8 = 241;
    pub const PLAYLIST: u8 = 233;
    /// Every id in this group.
    pub const ALL: &[u8] = &[NEW, NAME, PLAYLIST];
}

/// PyFLP `TrackID`.
pub mod track {
    pub const NAME: u8 = 239;
    pub const DATA: u8 = 238;
    /// Every id in this group.
    pub const ALL: &[u8] = &[NAME, DATA];
}

/// PyFLP `InsertID`.
pub mod insert {
    pub const ICON: u8 = 95;
    pub const OUTPUT: u8 = 147;
    pub const COLOR: u8 = 149;
    pub const INPUT: u8 = 154;
    pub const NAME: u8 = 204;
    pub const ROUTING: u8 = 235;
    pub const FLAGS: u8 = 236;
    /// Every id in this group.
    pub const ALL: &[u8] = &[ICON, OUTPUT, COLOR, INPUT, NAME, ROUTING, FLAGS];
}

/// PyFLP `MixerID`.
pub mod mixer {
    pub const APDC: u8 = 29;
    pub const PARAMS: u8 = 225;
    /// Every id in this group.
    pub const ALL: &[u8] = &[APDC, PARAMS];
}

/// PyFLP `SlotID`.
pub mod slot {
    pub const INDEX: u8 = 98;
    /// Every id in this group.
    pub const ALL: &[u8] = &[INDEX];
}

/// PyFLP `ProjectID`.
pub mod project {
    pub const LOOP_ACTIVE: u8 = 9;
    pub const SHOW_INFO: u8 = 10;
    pub const VOLUME: u8 = 12;
    pub const PAN_LAW: u8 = 23;
    pub const LICENSED: u8 = 28;
    pub const TEMPO_COARSE: u8 = 66;
    pub const PITCH: u8 = 80;
    pub const TEMPO_FINE: u8 = 93;
    pub const CUR_GROUP_ID: u8 = 146;
    pub const TEMPO: u8 = 156;
    pub const FL_BUILD: u8 = 159;
    pub const TITLE: u8 = 194;
    pub const COMMENTS: u8 = 195;
    pub const URL: u8 = 197;
    pub const RTF_COMMENTS: u8 = 198;
    pub const FL_VERSION: u8 = 199;
    pub const LICENSEE: u8 = 200;
    pub const DATA_PATH: u8 = 202;
    pub const GENRE: u8 = 206;
    pub const ARTISTS: u8 = 207;
    pub const TIMESTAMP: u8 = 237;
    /// Every id in this group.
    pub const ALL: &[u8] = &[
        LOOP_ACTIVE,
        SHOW_INFO,
        VOLUME,
        PAN_LAW,
        LICENSED,
        TEMPO_COARSE,
        PITCH,
        TEMPO_FINE,
        CUR_GROUP_ID,
        TEMPO,
        FL_BUILD,
        TITLE,
        COMMENTS,
        URL,
        RTF_COMMENTS,
        FL_VERSION,
        LICENSEE,
        DATA_PATH,
        GENRE,
        ARTISTS,
        TIMESTAMP,
    ];
}

/// Every declared id with the value type PyFLP gives it (`None`: chosen by
/// the id's range, as for undeclared ids), in PyFLP's lookup order.
pub const DECLARED: &[(u8, Option<Declared>)] = &[
    (plugin::COLOR, Some(Declared::Color)),
    (plugin::ICON, Some(Declared::U32)),
    (plugin::INTERNAL_NAME, None),
    (plugin::NAME, None),
    (
        plugin::WRAPPER,
        Some(Declared::Struct(crate::events::StructKind::Wrapper)),
    ),
    (plugin::DATA, None),
    (channel::IS_ENABLED, Some(Declared::Bool)),
    (channel::VOL_BYTE, Some(Declared::U8)),
    (channel::PAN_BYTE, Some(Declared::U8)),
    (channel::ZIPPED, Some(Declared::Bool)),
    (channel::PING_PONG_LOOP, Some(Declared::Bool)),
    (channel::TYPE, Some(Declared::U8)),
    (channel::ROUTED_TO, Some(Declared::I8)),
    (channel::IS_LOCKED, Some(Declared::Bool)),
    (channel::NEW, Some(Declared::U16)),
    (channel::FREQ_TILT, Some(Declared::U16)),
    (channel::FX_FLAGS, Some(Declared::U16)),
    (channel::CUTOFF, Some(Declared::U16)),
    (channel::VOL_WORD, Some(Declared::U16)),
    (channel::PAN_WORD, Some(Declared::U16)),
    (channel::PREAMP, Some(Declared::U16)),
    (channel::FADE_OUT, Some(Declared::U16)),
    (channel::FADE_IN, Some(Declared::U16)),
    (channel::RESONANCE, Some(Declared::U16)),
    (channel::STEREO_DELAY, Some(Declared::U16)),
    (channel::POGO, Some(Declared::U16)),
    (channel::TIME_SHIFT, Some(Declared::U16)),
    (channel::CHILDREN, Some(Declared::U16)),
    (channel::SWING, Some(Declared::U16)),
    (channel::RING_MOD, Some(Declared::U16Pair)),
    (channel::CUT_GROUP, Some(Declared::U16Pair)),
    (channel::ROOT_NOTE, Some(Declared::U32)),
    (channel::DELAY_MOD_XY, Some(Declared::U16Pair)),
    (channel::REVERB, Some(Declared::U32)),
    (channel::STRETCH_TIME, Some(Declared::F32)),
    (channel::FINE_TUNE, Some(Declared::I32)),
    (channel::SAMPLER_FLAGS, Some(Declared::U32)),
    (channel::LAYER_FLAGS, Some(Declared::U32)),
    (channel::GROUP_NUM, Some(Declared::I32)),
    (channel::AU_SAMPLE_RATE, Some(Declared::U32)),
    (channel::NAME, None),
    (channel::SAMPLE_PATH, None),
    (
        channel::DELAY,
        Some(Declared::Struct(crate::events::StructKind::Delay)),
    ),
    (
        channel::PARAMETERS,
        Some(Declared::Struct(crate::events::StructKind::Parameters)),
    ),
    (
        channel::ENVELOPE_LFO,
        Some(Declared::Struct(crate::events::StructKind::EnvelopeLFO)),
    ),
    (
        channel::LEVELS,
        Some(Declared::Struct(crate::events::StructKind::Levels)),
    ),
    (
        channel::POLYPHONY,
        Some(Declared::Struct(crate::events::StructKind::Polyphony)),
    ),
    (
        channel::TRACKING,
        Some(Declared::Struct(crate::events::StructKind::Tracking)),
    ),
    (
        channel::LEVEL_ADJUSTS,
        Some(Declared::Struct(crate::events::StructKind::LevelAdjusts)),
    ),
    (
        channel::AUTOMATION,
        Some(Declared::Struct(crate::events::StructKind::Automation)),
    ),
    (display_group::NAME, None),
    (rack::SWING, Some(Declared::U8)),
    (rack::FIT_TO_STEPS, Some(Declared::U8)),
    (rack::WINDOW_HEIGHT, Some(Declared::U32)),
    (time_marker::NUMERATOR, Some(Declared::U8)),
    (time_marker::DENOMINATOR, Some(Declared::U8)),
    (time_marker::POSITION, Some(Declared::U32)),
    (time_marker::NAME, None),
    (patterns::PLAY_TRUNCATED_NOTES, Some(Declared::Bool)),
    (patterns::CURRENTLY_SELECTED, Some(Declared::U16)),
    (pattern::LOOPED, Some(Declared::Bool)),
    (pattern::NEW, Some(Declared::U16)),
    (pattern::COLOR, Some(Declared::Color)),
    (pattern::NAME, None),
    (pattern::CHANNEL_IID, Some(Declared::U32)),
    (pattern::N161, Some(Declared::I32)),
    (pattern::N162, Some(Declared::U32)),
    (pattern::LENGTH, Some(Declared::U32)),
    (
        pattern::CONTROLLERS,
        Some(Declared::Struct(crate::events::StructKind::Controller)),
    ),
    (
        pattern::NOTES,
        Some(Declared::Struct(crate::events::StructKind::Notes)),
    ),
    (arrangements::TIME_SIG_NUM, Some(Declared::U8)),
    (arrangements::TIME_SIG_BEAT, Some(Declared::U8)),
    (arrangements::CURRENT, Some(Declared::U16)),
    (arrangements::LOOP_POS, Some(Declared::U16Pair)),
    (
        arrangements::PL_SELECTION,
        Some(Declared::Struct(crate::events::StructKind::PLSelection)),
    ),
    (arrangement::NEW, Some(Declared::U16)),
    (arrangement::NAME, None),
    (
        arrangement::PLAYLIST,
        Some(Declared::Struct(crate::events::StructKind::Playlist)),
    ),
    (track::NAME, None),
    (
        track::DATA,
        Some(Declared::Struct(crate::events::StructKind::Track)),
    ),
    (insert::ICON, Some(Declared::I16)),
    (insert::OUTPUT, Some(Declared::I32)),
    (insert::COLOR, Some(Declared::Color)),
    (insert::INPUT, Some(Declared::I32)),
    (insert::NAME, None),
    (
        insert::ROUTING,
        Some(Declared::Struct(crate::events::StructKind::InsertRouting)),
    ),
    (
        insert::FLAGS,
        Some(Declared::Struct(crate::events::StructKind::InsertFlags)),
    ),
    (mixer::APDC, None),
    (
        mixer::PARAMS,
        Some(Declared::Struct(crate::events::StructKind::MixerParams)),
    ),
    (slot::INDEX, Some(Declared::U16)),
    (project::LOOP_ACTIVE, Some(Declared::Bool)),
    (project::SHOW_INFO, Some(Declared::Bool)),
    (project::VOLUME, Some(Declared::U8)),
    (project::PAN_LAW, Some(Declared::U8)),
    (project::LICENSED, Some(Declared::Bool)),
    (project::TEMPO_COARSE, None),
    (project::PITCH, Some(Declared::I16)),
    (project::TEMPO_FINE, None),
    (project::CUR_GROUP_ID, Some(Declared::I32)),
    (project::TEMPO, Some(Declared::U32)),
    (project::FL_BUILD, Some(Declared::U32)),
    (project::TITLE, None),
    (project::COMMENTS, None),
    (project::URL, None),
    (project::RTF_COMMENTS, None),
    (project::FL_VERSION, Some(Declared::Ascii)),
    (project::LICENSEE, None),
    (project::DATA_PATH, None),
    (project::GENRE, None),
    (project::ARTISTS, None),
    (
        project::TIMESTAMP,
        Some(Declared::Struct(crate::events::StructKind::Timestamp)),
    ),
];

// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! The plugin in a channel or an effect slot (`pyflp.plugin`'s models).

use crate::events::Value;
use crate::ids::plugin;
use crate::plugin_data::{Native, VstPlugin, vst_id};
use crate::tree::EventTree;

/// `_WrapperFlags`.
pub mod wrapper_flags {
    pub const VISIBLE: i64 = 1 << 0;
    pub const DISABLED: i64 = 1 << 1;
    pub const DETACHED: i64 = 1 << 2;
    pub const GENERATOR: i64 = 1 << 4;
    pub const SMART_DISABLE: i64 = 1 << 5;
    pub const THREADED_PROCESSING: i64 = 1 << 6;
    pub const DEMO_MODE: i64 = 1 << 7;
    pub const HIDE_SETTINGS: i64 = 1 << 8;
    pub const MINIMIZED: i64 = 1 << 9;
    pub const DIRECTX: i64 = 1 << 16;
}

/// `_VSTFlags`: the VST wrapper's options.
pub mod vst_flags {
    pub const SEND_PB_RANGE: u32 = 1 << 0;
    pub const FIXED_SIZE_BUFFERS: u32 = 1 << 1;
    pub const NOTIFY_RENDER: u32 = 1 << 2;
    pub const PROCESS_INACTIVE: u32 = 1 << 3;
    pub const DONT_SEND_REL_VELO: u32 = 1 << 5;
    pub const DONT_NOTIFY_CHANGES: u32 = 1 << 6;
    pub const SEND_LOOP_POS: u32 = 1 << 11;
    pub const ALLOW_THREADED: u32 = 1 << 12;
    pub const KEEP_FOCUS: u32 = 1 << 15;
    pub const DONT_KEEP_CPU_STATE: u32 = 1 << 16;
    pub const SEND_MOD_X: u32 = 1 << 17;
    pub const LOAD_BRIDGED: u32 = 1 << 18;
    pub const EXTERNAL_WINDOW: u32 = 1 << 21;
    pub const UPDATE_WHEN_HIDDEN: u32 = 1 << 23;
    pub const DONT_RESET_ON_TRANSPORT: u32 = 1 << 25;
    pub const DPI_AWARE_BRIDGED: u32 = 1 << 26;
    pub const ACCEPT_FILE_DROP: u32 = 1 << 28;
    pub const ALLOW_SMART_DISABLE: u32 = 1 << 29;
    pub const SCALE_EDITOR: u32 = 1 << 30;
    pub const DONT_USE_TIME_OFFSET: u32 = 1 << 31;
}

/// `_VSTFlags2`.
pub mod vst_flags2 {
    pub const PROCESS_MAX_SIZE: u32 = 1 << 0;
    pub const USE_MAX_FROM_HOST: u32 = 1 << 1;
}

/// What a plugin's data is.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginData<'a> {
    /// A VST2 or VST3 plugin (`VSTPlugin`).
    Vst(&'a VstPlugin),
    /// A native plugin PyFLP decodes.
    Native(&'a Native),
    /// Data PyFLP does not decode (`_PluginBase`).
    Unknown,
}

/// A plugin and its wrapper settings.
#[derive(Debug, Clone)]
pub struct Plugin<'a> {
    events: EventTree<'a>,
    pub data: PluginData<'a>,
}

/// Which plugin data a model accepts (PyFLP's `PluginProp` types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Accepts {
    /// Channels: VSTs, BooBass, Fruit Kick, Plucked!.
    Channel,
    /// Effect slots: VSTs and the native effects.
    Slot,
    /// Any plugin (presets, where PyFLP's caller names the type).
    Any,
}

impl<'a> Plugin<'a> {
    /// The plugin in `events` (a channel's or slot's), if its data is of a
    /// kind the model accepts.
    pub(crate) fn of(events: &EventTree<'a>, accepts: Accepts) -> Option<Self> {
        let data = events.first(plugin::DATA)?;
        let kind = match &data.value {
            Value::Vst(vst) => PluginData::Vst(vst),
            Value::Native(native) => {
                let fits = match accepts {
                    Accepts::Channel => matches!(
                        native,
                        Native::BooBass { .. } | Native::FruitKick { .. } | Native::Plucked { .. }
                    ),
                    Accepts::Slot => !matches!(
                        native,
                        Native::BooBass { .. } | Native::FruitKick { .. } | Native::Plucked { .. }
                    ),
                    Accepts::Any => true,
                };
                if !fits {
                    return None;
                }
                PluginData::Native(native)
            }
            Value::Unknown(_) => PluginData::Unknown,
            _ => return None,
        };
        let tree = events.subtree(|e| e.id == plugin::WRAPPER || e.id == plugin::DATA);
        Some(Self {
            events: tree,
            data: kind,
        })
    }

    /// PyFLP's class name for the plugin.
    #[must_use]
    pub fn class(&self) -> &'static str {
        match &self.data {
            PluginData::Vst(_) => "VSTPlugin",
            PluginData::Unknown => "_PluginBase",
            PluginData::Native(native) => match native {
                Native::BooBass { .. } => "BooBass",
                Native::FruitKick { .. } => "FruitKick",
                Native::FruityBalance { .. } => "FruityBalance",
                Native::FruityBloodOverdrive { .. } => "FruityBloodOverdrive",
                Native::FruityCenter { .. } => "FruityCenter",
                Native::FruityFastDist { .. } => "FruityFastDist",
                Native::FruityNotebook2 { .. } => "FruityNotebook2",
                Native::FruitySend { .. } => "FruitySend",
                Native::FruitySoftClipper { .. } => "FruitySoftClipper",
                Native::FruityStereoEnhancer { .. } => "FruityStereoEnhancer",
                Native::Plucked { .. } => "Plucked",
                Native::Soundgoodizer { .. } => "Soundgoodizer",
            },
        }
    }

    fn wrapper(&self) -> Option<crate::structs::Wrapper> {
        match self.events.first(plugin::WRAPPER)?.value {
            Value::Wrapper(w) => Some(w),
            _ => None,
        }
    }

    /// Whether a [`wrapper_flags`] bit is set.
    #[must_use]
    pub fn wrapper_flag(&self, bit: i64) -> Option<bool> {
        crate::flag(self.wrapper()?.flags.map(i64::from), bit)
    }

    /// Whether the plugin is a generator (instrument) rather than an effect.
    #[must_use]
    pub fn generator(&self) -> Option<bool> {
        self.wrapper_flag(wrapper_flags::GENERATOR)
    }

    /// Whether its editor is open.
    #[must_use]
    pub fn visible(&self) -> Option<bool> {
        self.wrapper_flag(wrapper_flags::VISIBLE)
    }

    #[must_use]
    pub fn width(&self) -> Option<u32> {
        self.wrapper()?.width
    }

    #[must_use]
    pub fn height(&self) -> Option<u32> {
        self.wrapper()?.height
    }

    /// Selected wrapper page.
    #[must_use]
    pub fn page(&self) -> Option<u8> {
        self.wrapper()?.page
    }

    fn vst(&self) -> Option<&'a VstPlugin> {
        match self.data {
            PluginData::Vst(vst) => Some(vst),
            _ => None,
        }
    }

    /// A VST's factory name.
    #[must_use]
    pub fn name(&self) -> Option<&'a str> {
        self.vst()?.text(vst_id::NAME)
    }

    /// A VST's maker.
    #[must_use]
    pub fn vendor(&self) -> Option<&'a str> {
        self.vst()?.text(vst_id::VENDOR)
    }

    /// The VST binary's path on the machine that saved the project.
    #[must_use]
    pub fn plugin_path(&self) -> Option<&'a str> {
        self.vst()?.text(vst_id::PLUGIN_PATH)
    }

    /// A VST2's four-character code.
    #[must_use]
    pub fn fourcc(&self) -> Option<&'a str> {
        self.vst()?.text(vst_id::FOURCC)
    }

    /// A VST3's class id bytes.
    #[must_use]
    pub fn guid(&self) -> Option<&'a [u8]> {
        self.vst()?.bytes(vst_id::GUID)
    }

    /// The plugin's own preset data.
    #[must_use]
    pub fn state(&self) -> Option<&'a [u8]> {
        self.vst()?.bytes(vst_id::STATE)
    }

    /// Whether a [`vst_flags`] bit is set in a VST's options.
    #[must_use]
    pub fn vst_flag(&self, bit: u32) -> Option<bool> {
        Some(self.vst()?.options()?.0? & bit != 0)
    }

    /// Whether a [`vst_flags2`] bit is set in a VST's options.
    #[must_use]
    pub fn vst_flag2(&self, bit: u32) -> Option<bool> {
        Some(self.vst()?.options()?.1? & bit != 0)
    }

    /// A VST's "fast idle" option.
    #[must_use]
    pub fn fast_idle(&self) -> Option<bool> {
        self.vst()?.options()?.2
    }

    /// A VST's MIDI input port.
    #[must_use]
    pub fn midi_input(&self) -> Option<i32> {
        self.vst()?.midi()?.0
    }

    /// A VST's MIDI output port.
    #[must_use]
    pub fn midi_output(&self) -> Option<i32> {
        self.vst()?.midi()?.1
    }

    /// A VST's pitch-bend range in semitones.
    #[must_use]
    pub fn pb_range(&self) -> Option<u32> {
        self.vst()?.midi()?.2
    }
}

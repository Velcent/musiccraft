// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Plugin data events (`PluginID.Data`), whose layout depends on the plugin
//! named by the `PluginID.InternalName` event before them: the VST wrapper's
//! nested records, and the native plugins PyFLP decodes.

use crate::reader::Reader;
use crate::structs::Failed;

/// `_VSTPluginEventID`: records inside the VST wrapper's data.
pub mod vst_id {
    pub const MIDI: u32 = 1;
    pub const FLAGS: u32 = 2;
    pub const IO: u32 = 30;
    pub const INPUTS: u32 = 31;
    pub const OUTPUTS: u32 = 32;
    pub const PLUGIN_INFO: u32 = 50;
    pub const FOURCC: u32 = 51;
    pub const GUID: u32 = 52;
    pub const STATE: u32 = 53;
    pub const NAME: u32 = 54;
    pub const PLUGIN_PATH: u32 = 55;
    pub const VENDOR: u32 = 56;
}

/// One record of the VST wrapper's data.
#[derive(Debug, Clone, PartialEq)]
pub enum VstRecord {
    /// Text records (FourCC, name, vendor, plugin path), UTF-8.
    Text(u32, String),
    /// MIDI ports and pitch-bend range.
    Midi {
        input: Option<i32>,
        output: Option<i32>,
        pb_range: Option<u32>,
    },
    /// Wrapper options.
    Flags {
        flags: Option<u32>,
        flags2: Option<u32>,
        fast_idle: Option<bool>,
    },
    /// Anything else, kept as bytes (the GUID, the plugin's state…).
    Bytes(u32, Vec<u8>),
}

/// `VSTPluginEvent`: a VST2 or VST3 plugin's data.
#[derive(Debug, Clone, PartialEq)]
pub struct VstPlugin {
    /// 8 or 10 for VSTs.
    pub kind: u32,
    pub records: Vec<VstRecord>,
}

impl VstPlugin {
    pub(crate) fn read(data: &[u8]) -> Result<Self, Failed> {
        let mut r = Reader::new(data);
        let kind = r.u32().ok_or("VST data too short")?;
        let records = r.greedy(|r| {
            let id = r.u32()?;
            let len = usize::try_from(r.u64()?).ok()?;
            let bytes = r.bytes(len)?;
            // Invalid UTF-8 in a text record ends the list, as in PyFLP.
            Self::record(id, bytes)
        });
        Ok(Self { kind, records })
    }

    fn record(id: u32, bytes: &[u8]) -> Option<VstRecord> {
        let mut r = Reader::new(bytes);
        Some(match id {
            vst_id::MIDI => VstRecord::Midi {
                input: r.attempt(Reader::i32),
                output: r.attempt(Reader::i32),
                pb_range: r.attempt(Reader::u32),
            },
            vst_id::FLAGS => {
                let _u1 = r.attempt(|r| r.bytes(9));
                let flags = r.attempt(Reader::u32);
                let flags2 = r.attempt(Reader::u32);
                let _u2 = r.attempt(|r| r.bytes(5));
                VstRecord::Flags {
                    flags,
                    flags2,
                    fast_idle: r.attempt(Reader::flag),
                }
            }
            vst_id::FOURCC | vst_id::NAME | vst_id::VENDOR | vst_id::PLUGIN_PATH => {
                VstRecord::Text(id, std::str::from_utf8(bytes).ok()?.to_owned())
            }
            _ => VstRecord::Bytes(id, bytes.to_vec()),
        })
    }

    /// The text of record `id`.
    #[must_use]
    pub fn text(&self, id: u32) -> Option<&str> {
        self.records.iter().find_map(|r| match r {
            VstRecord::Text(i, text) if *i == id => Some(text.as_str()),
            _ => None,
        })
    }

    /// The MIDI record: (input port, output port, pitch-bend range).
    #[must_use]
    pub fn midi(&self) -> Option<(Option<i32>, Option<i32>, Option<u32>)> {
        self.records.iter().find_map(|r| match *r {
            VstRecord::Midi {
                input,
                output,
                pb_range,
            } => Some((input, output, pb_range)),
            _ => None,
        })
    }

    /// The options record: (`_VSTFlags`, `_VSTFlags2`, fast idle).
    #[must_use]
    pub fn options(&self) -> Option<(Option<u32>, Option<u32>, Option<bool>)> {
        self.records.iter().find_map(|r| match *r {
            VstRecord::Flags {
                flags,
                flags2,
                fast_idle,
            } => Some((flags, flags2, fast_idle)),
            _ => None,
        })
    }

    /// The bytes of record `id`.
    #[must_use]
    pub fn bytes(&self, id: u32) -> Option<&[u8]> {
        self.records.iter().find_map(|r| match r {
            VstRecord::Bytes(i, bytes) if *i == id => Some(bytes.as_slice()),
            _ => None,
        })
    }
}

/// A native plugin PyFLP decodes, with its fields in PyFLP's order.
#[derive(Debug, Clone, PartialEq)]
pub enum Native {
    BooBass {
        bass: u32,
        mid: u32,
        high: u32,
    },
    FruitKick {
        max_freq: i32,
        min_freq: i32,
        freq_decay: u32,
        amp_decay: u32,
        click: u32,
        distortion: u32,
    },
    FruityBalance {
        pan: u32,
        volume: u32,
    },
    FruityBloodOverdrive {
        pre_band: u32,
        color: u32,
        pre_amp: u32,
        x100: bool,
        post_filter: u32,
        post_gain: u32,
    },
    FruityCenter {
        enabled: bool,
    },
    FruityFastDist {
        pre: u32,
        threshold: u32,
        kind: u32,
        mix: u32,
        post: u32,
    },
    FruityNotebook2 {
        active_page: u32,
        pages: Vec<(i32, String)>,
        editable: bool,
    },
    FruitySend {
        pan: i32,
        dry: u32,
        volume: u32,
        send_to: i32,
    },
    FruitySoftClipper {
        threshold: u32,
        post: u32,
    },
    FruityStereoEnhancer {
        pan: i32,
        volume: u32,
        stereo_separation: u32,
        phase_offset: u32,
        effect_position: u32,
        phase_inversion: u32,
    },
    Plucked {
        decay: u32,
        color: u32,
        normalize: bool,
        gate: bool,
        widen: bool,
    },
    Soundgoodizer {
        mode: u32,
        amount: u32,
    },
}

/// The internal name PyFLP uses for the VST wrapper.
pub const VST_INTERNAL_NAME: &str = "Fruity Wrapper";

/// Internal names of the native plugins [`Native`] covers.
pub const NATIVE_NAMES: [&str; 12] = [
    "BooBass",
    "Fruit Kick",
    "Fruity Balance",
    "Fruity Blood Overdrive",
    "Fruity Center",
    "Fruity Fast Dist",
    "Fruity NoteBook 2",
    "Fruity Send",
    "Fruity Soft Clipper",
    "Fruity Stereo Enhancer",
    "Plucked!",
    "Soundgoodizer",
];

impl Native {
    /// Decodes the data of the native plugin called `name`; `None` when PyFLP
    /// does not decode that plugin.
    pub(crate) fn read(name: &str, data: &[u8]) -> Option<Result<Self, Failed>> {
        const SHORT: Failed = "plugin data too short";
        let len = data.len();
        let mut r = Reader::new(data);
        let r = &mut r;
        let u = |r: &mut Reader<'_>| r.u32().ok_or(SHORT);
        let i = |r: &mut Reader<'_>| r.i32().ok_or(SHORT);
        let b = |r: &mut Reader<'_>| r.bool32().ok_or(SHORT);
        let skip_if = |r: &mut Reader<'_>, cond: bool| -> Result<(), Failed> {
            if cond {
                r.bytes(4).ok_or(SHORT)?;
            }
            Ok(())
        };
        let read = || -> Result<Self, Failed> {
            Ok(match name {
                "BooBass" => {
                    skip_if(r, len == 16)?;
                    Self::BooBass {
                        bass: u(r)?,
                        mid: u(r)?,
                        high: u(r)?,
                    }
                }
                "Fruit Kick" => {
                    r.bytes(4).ok_or(SHORT)?;
                    let kick = Self::FruitKick {
                        max_freq: i(r)?,
                        min_freq: i(r)?,
                        freq_decay: u(r)?,
                        amp_decay: u(r)?,
                        click: u(r)?,
                        distortion: u(r)?,
                    };
                    r.bytes(4).ok_or(SHORT)?;
                    kick
                }
                "Fruity Balance" => Self::FruityBalance {
                    pan: u(r)?,
                    volume: u(r)?,
                },
                "Fruity Blood Overdrive" => {
                    skip_if(r, len == 36)?;
                    let overdrive = Self::FruityBloodOverdrive {
                        pre_band: u(r)?,
                        color: u(r)?,
                        pre_amp: u(r)?,
                        x100: b(r)?,
                        post_filter: u(r)?,
                        post_gain: u(r)?,
                    };
                    r.bytes(8).ok_or(SHORT)?;
                    overdrive
                }
                "Fruity Center" => {
                    skip_if(r, len == 8)?;
                    Self::FruityCenter { enabled: b(r)? }
                }
                "Fruity Fast Dist" => Self::FruityFastDist {
                    pre: u(r)?,
                    threshold: u(r)?,
                    kind: u(r)?,
                    mix: u(r)?,
                    post: u(r)?,
                },
                "Fruity NoteBook 2" => {
                    r.bytes(4).ok_or(SHORT)?;
                    let active_page = u(r)?;
                    let mut pages = Vec::new();
                    // A `GreedyRange` of pages. A page index of -1 is an
                    // empty entry (`StopIf`), not the end of the range;
                    // it is left out of `pages`.
                    loop {
                        let page = r.attempt(|r| {
                            let index = r.i32()?;
                            if index == -1 {
                                return Some(None);
                            }
                            let chars = usize::try_from(r.varint()?).ok()?;
                            let bytes = r.bytes(chars.checked_mul(2)?)?;
                            let units: Vec<u16> = bytes
                                .chunks_exact(2)
                                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                                .collect();
                            let text = String::from_utf16(&units).ok()?;
                            Some(Some((index, text.trim_end_matches('\0').to_owned())))
                        });
                        match page {
                            Some(Some(page)) => pages.push(page),
                            Some(None) => {}
                            None => break,
                        }
                    }
                    Self::FruityNotebook2 {
                        active_page,
                        pages,
                        editable: r.flag().ok_or(SHORT)?,
                    }
                }
                "Fruity Send" => Self::FruitySend {
                    pan: i(r)?,
                    dry: u(r)?,
                    volume: u(r)?,
                    send_to: i(r)?,
                },
                "Fruity Soft Clipper" => Self::FruitySoftClipper {
                    threshold: u(r)?,
                    post: u(r)?,
                },
                "Fruity Stereo Enhancer" => Self::FruityStereoEnhancer {
                    pan: i(r)?,
                    volume: u(r)?,
                    stereo_separation: u(r)?,
                    phase_offset: u(r)?,
                    effect_position: u(r)?,
                    phase_inversion: u(r)?,
                },
                "Plucked!" => Self::Plucked {
                    decay: u(r)?,
                    color: u(r)?,
                    normalize: b(r)?,
                    gate: b(r)?,
                    widen: b(r)?,
                },
                "Soundgoodizer" => {
                    skip_if(r, len == 12)?;
                    Self::Soundgoodizer {
                        mode: u(r)?,
                        amount: u(r)?,
                    }
                }
                _ => return Err("not decoded"),
            })
        };
        NATIVE_NAMES.contains(&name).then(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vst_records_split_into_text_and_bytes() {
        let mut data = 10u32.to_le_bytes().to_vec();
        for (id, bytes) in [(vst_id::NAME, &b"Synth"[..]), (vst_id::STATE, &[1, 2, 3])] {
            data.extend_from_slice(&id.to_le_bytes());
            data.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            data.extend_from_slice(bytes);
        }
        let vst = VstPlugin::read(&data).unwrap();
        assert_eq!(vst.kind, 10);
        assert_eq!(vst.text(vst_id::NAME), Some("Synth"));
        assert_eq!(vst.bytes(vst_id::STATE), Some(&[1, 2, 3][..]));
        assert_eq!(vst.text(vst_id::VENDOR), None);
    }

    #[test]
    fn native_layouts_depend_on_the_data_size() {
        let words = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        assert_eq!(
            Native::read("BooBass", &words(&[1, 2, 3])),
            Some(Ok(Native::BooBass {
                bass: 1,
                mid: 2,
                high: 3
            }))
        );
        assert_eq!(
            Native::read("BooBass", &words(&[9, 1, 2, 3])),
            Some(Ok(Native::BooBass {
                bass: 1,
                mid: 2,
                high: 3
            })),
            "a 16-byte BooBass starts with a marker"
        );
        assert_eq!(Native::read("Something else", &[]), None);
        assert!(Native::read("Fruity Balance", &[1]).unwrap().is_err());
    }
}

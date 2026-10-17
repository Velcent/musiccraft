//! Converts an FL Studio project, as `musiccraft-flp` (a port of PyFLP)
//! reads it, into a MusicCraft project. See `docs/spec/flp-import.md`.

use std::collections::BTreeMap;

use musiccraft_flp::arrangement::{Arrangement, ClipRef};
use musiccraft_flp::channel::{Channel as FlChannel, ChannelKind};
use musiccraft_flp::mixer::Insert;
use musiccraft_flp::plugin::{Plugin, PluginData};
use musiccraft_flp::{PanLaw as FlPanLaw, Project as FlProject};
use musiccraft_project::params::{MAX_EFFECT_SLOTS, MAX_MIXER_TRACKS, PPQ, TICKS_PER_STEP};
use musiccraft_project::{
    Channel, ChannelId, Clip, ClipSource, DEFAULT_INSERTS, EffectSlot, Instrument,
    MAX_PATTERN_STEPS, MAX_SENDS, Marker, MarkerAction, MixerSend, MixerTrack, Note, NoteProps,
    PLAYLIST_TRACKS, PanLaw, Pattern, PatternId, PluginRef, Project, TimeSig,
};

/// What could not be brought over, for the command's message.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub replaced: Vec<String>,
    pub dropped: Vec<String>,
}

impl Report {
    fn drop(&mut self, what: impl Into<String>) {
        self.dropped.push(what.into());
    }
}

/// Converts the bytes of an `.flp` file; `fallback_name` names a project
/// without a title.
pub(crate) fn convert(bytes: &[u8], fallback_name: &str) -> Result<(Project, Report), String> {
    let fl = FlProject::parse(bytes).map_err(|e| e.to_string())?;
    let mut report = Report::default();
    let name = fl
        .title()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| fallback_name.to_owned());
    let mut project = Project::new(name);
    let scale = Scale(u32::from(fl.ppq()).max(1));
    if let Some(bpm) = fl.tempo() {
        project
            .set_tempo_bpm(bpm.clamp(10.0, 999.0))
            .map_err(|e| e.to_string())?;
    }
    if let Some(cents) = fl.main_pitch() {
        project.master_pitch = (cents.clamp(-1200, 1200)) as f32;
    }
    if fl.pan_law() == Some(FlPanLaw::Triangular) {
        project.pan_law = PanLaw::Triangular;
    }
    if let (Some(num), Some(den)) = fl.arrangements().time_signature()
        && let Some(sig) = time_sig(num, den)
    {
        project.time_signature = sig;
    }
    let inserts = fl.mixer().inserts();
    let channels = convert_channels(&fl, &mut project, &mut report);
    let patterns = convert_patterns(&fl, &mut project, &channels, scale, &mut report);
    let maps = (&channels, &patterns);
    convert_playlist(&fl, &mut project, maps, scale, &mut report);
    convert_mixer(&inserts, &mut project, &mut report);
    // A round trip through the file format applies the same checks and
    // repairs as loading a saved project.
    let json = project.to_json().map_err(|e| e.to_string())?;
    let project = Project::from_json(&json).map_err(|e| e.to_string())?;
    Ok((project, report))
}

/// A signature from FL's numerator and beat value, if MusicCraft has it.
fn time_sig(num: i64, den: i64) -> Option<TimeSig> {
    TimeSig::new(u32::try_from(num).ok()?, u32::try_from(den).ok()?)
}

/// The signature of a time marker that changes it.
fn marker_sig(marker: &musiccraft_flp::pattern::TimeMarker<'_>) -> Option<TimeSig> {
    if marker.is_signature() != Some(true) {
        return None;
    }
    time_sig(marker.numerator()?, marker.denominator()?)
}

/// Rescales ticks from the file's PPQ to MusicCraft's.
#[derive(Debug, Clone, Copy)]
struct Scale(u32);

impl Scale {
    fn ticks(self, value: f64) -> u32 {
        let scaled = (value.max(0.0) * f64::from(PPQ) / f64::from(self.0)).round();
        if scaled >= f64::from(u32::MAX) {
            u32::MAX
        } else {
            scaled as u32
        }
    }
}

/// An imported channel: its MusicCraft id and FL kind.
#[derive(Debug, Clone, Copy)]
struct Imported {
    id: ChannelId,
    kind: ChannelKind,
}

type ChannelMap = BTreeMap<i64, Imported>;

fn convert_channels(fl: &FlProject, project: &mut Project, report: &mut Report) -> ChannelMap {
    let list = match fl.channels().channels() {
        Ok(list) => list,
        Err(reason) => {
            report.drop(format!("the channel rack ({reason})"));
            return ChannelMap::new();
        }
    };
    let mut map = ChannelMap::new();
    let mut layers = Vec::new();
    for ch in &list {
        let name = ch
            .display_name()
            .unwrap_or_else(|| format!("Channel {}", ch.iid + 1));
        let instrument = match ch.kind {
            ChannelKind::Automation => {
                report.drop(format!("automation clip “{name}”"));
                continue;
            }
            ChannelKind::Layer => {
                layers.push((project.channels.len(), ch.children().unwrap_or_default()));
                Instrument::Layer {
                    channels: Vec::new(),
                }
            }
            ChannelKind::Sampler => Instrument::Sampler {
                sample: ch.sample_path().unwrap_or_default(),
            },
            ChannelKind::Instrument | ChannelKind::Channel => {
                match ch.plugin().and_then(|p| vst3_ref(&p, &name, project)) {
                    Some(plugin) => Instrument::Plugin { plugin },
                    None => {
                        report.replaced.push(format!("“{name}” (now a synth)"));
                        Instrument::Synth
                    }
                }
            }
        };
        let id = ChannelId(project.alloc_id());
        project
            .channels
            .push(convert_channel(ch, id, name, instrument));
        map.insert(ch.iid, Imported { id, kind: ch.kind });
    }
    for (index, children) in layers {
        let ids = children
            .iter()
            .filter_map(|iid| map.get(iid).map(|c| c.id))
            .collect();
        if let Some(channel) = project.channels.get_mut(index) {
            channel.instrument = Instrument::Layer { channels: ids };
        }
    }
    map
}

fn convert_channel(
    ch: &FlChannel<'_>,
    id: ChannelId,
    name: String,
    instrument: Instrument,
) -> Channel {
    let mut channel = Channel::new(id, name, instrument);
    if let Some(volume) = ch.volume() {
        channel.volume = (volume as f32 / 12800.0).clamp(0.0, 1.0);
    }
    if let Some(pan) = ch.pan() {
        channel.pan = ((pan - 6400) as f32 / 6400.0).clamp(-1.0, 1.0);
    }
    channel.mute = ch.enabled() == Some(false);
    channel.mixer_track = ch
        .insert()
        .and_then(|i| usize::try_from(i).ok())
        .unwrap_or(0);
    channel.group = ch
        .group
        .clone()
        .filter(|g| g != "Unsorted")
        .unwrap_or_default();
    channel
}

/// A VST3 plugin as a MusicCraft plugin reference (no state: FL's is in its
/// wrapper's own format).
fn vst3_ref(plugin: &Plugin<'_>, fallback: &str, project: &mut Project) -> Option<PluginRef> {
    if !matches!(plugin.data, PluginData::Vst(_)) || plugin.fourcc().is_some() {
        return None;
    }
    let guid: [u8; 16] = plugin.guid()?.try_into().ok()?;
    let saved_on_windows = plugin
        .plugin_path()
        .is_some_and(|p| p.contains('\\') || p.get(1..2) == Some(":"));
    let id = vst3_id(guid, saved_on_windows, cfg!(windows));
    let name = plugin.name().unwrap_or(fallback).to_owned();
    Some(PluginRef::new(id, name, project.alloc_id()))
}

/// MusicCraft's id for a VST3 class: `vst3:` and the TUID's bytes in the
/// order of the platform it runs on. A TUID is in COM (GUID) byte order on
/// Windows and in plain order elsewhere (the VST3 SDK's `INLINE_UID`).
fn vst3_id(mut guid: [u8; 16], com_order: bool, want_com_order: bool) -> String {
    if com_order != want_com_order {
        guid[0..4].reverse();
        guid[4..6].reverse();
        guid[6..8].reverse();
    }
    let hex: String = guid.iter().map(|b| format!("{b:02X}")).collect();
    format!("vst3:{hex}")
}

fn convert_patterns(
    fl: &FlProject,
    project: &mut Project,
    channels: &ChannelMap,
    scale: Scale,
    report: &mut Report,
) -> PatternMap {
    let mut patterns = Vec::new();
    let mut dropped_notes = 0;
    for (index, p) in fl.patterns().patterns().iter().enumerate() {
        let id = PatternId(project.alloc_id());
        let name = p
            .name()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("Pattern {}", index + 1));
        // A pattern takes its first signature marker, else the project's.
        let sig = p
            .timemarkers()
            .iter()
            .find_map(marker_sig)
            .unwrap_or(project.time_signature);
        let mut pattern = Pattern::with_signature(id, name, sig);
        if let Some(length) = p.length().filter(|l| *l > 0) {
            let ticks = scale.ticks(length as f64);
            pattern.length_steps = ticks
                .div_ceil(TICKS_PER_STEP)
                .clamp(sig.bar_steps(), MAX_PATTERN_STEPS);
        }
        for n in p.notes() {
            let Some(channel) = channels.get(&i64::from(n.rack_channel)) else {
                dropped_notes += 1;
                continue;
            };
            let length = match scale.ticks(f64::from(n.length)) {
                0 => TICKS_PER_STEP,
                ticks => ticks,
            };
            pattern.notes.push(Note {
                channel: channel.id,
                start: scale.ticks(f64::from(n.position)),
                length,
                key: n.key.min(127) as u8,
                velocity: n.velocity.clamp(1, 127),
                pan: ((i32::from(n.pan) - 64) * 100 / 64).clamp(-100, 100) as i8,
                slide: n.flags & musiccraft_flp::structs::NoteItem::SLIDE != 0,
                props: NoteProps {
                    release: n.release.min(127),
                    fine: ((i32::from(n.fine_pitch) - 120) * 10).clamp(-100, 100) as i8,
                    mod_x: n.mod_x / 2,
                    mod_y: n.mod_y / 2,
                    color: n.midi_channel & 0x0F,
                    ..NoteProps::default()
                },
            });
        }
        pattern.sort_notes();
        pattern.notes.dedup_by_key(|n| (n.channel, n.start, n.key));
        patterns.push((p.iid(), pattern));
    }
    if dropped_notes > 0 {
        report.drop(format!("{dropped_notes} notes of channels not imported"));
    }
    let map = patterns
        .iter()
        .filter_map(|(iid, p)| Some(((*iid)?, p.id)))
        .collect();
    if !patterns.is_empty() {
        project.patterns = patterns.into_iter().map(|(_, p)| p).collect();
        project.current_pattern = 0;
    }
    map
}

/// FL pattern iid to MusicCraft pattern id.
type PatternMap = BTreeMap<i64, PatternId>;

fn chosen_arrangement<'a>(fl: &'a FlProject) -> Result<Option<Arrangement<'a>>, &'static str> {
    let arrangements = fl.arrangements();
    let list = arrangements.arrangements()?;
    let len = i64::try_from(list.len()).unwrap_or(i64::MAX);
    let index = arrangements
        .current_index()
        .map(|i| if i < 0 { i + len } else { i })
        .and_then(|i| usize::try_from(i).ok())
        .filter(|i| *i < list.len())
        .unwrap_or(0);
    Ok(list.into_iter().nth(index))
}

fn convert_playlist(
    fl: &FlProject,
    project: &mut Project,
    (channels, patterns): (&ChannelMap, &PatternMap),
    scale: Scale,
    report: &mut Report,
) {
    let arrangement = match chosen_arrangement(fl) {
        Ok(Some(a)) => a,
        Ok(None) => return,
        Err(reason) => {
            report.drop(format!("the playlist ({reason})"));
            return;
        }
    };
    let tracks = match arrangement.tracks() {
        Ok(tracks) => tracks,
        Err(reason) => {
            report.drop(format!("the playlist ({reason})"));
            return;
        }
    };
    let playlist = &mut project.playlist;
    let (mut far_clips, mut other_clips) = (0, 0);
    for (row, track) in tracks.iter().enumerate() {
        let row = u32::try_from(row).unwrap_or(u32::MAX);
        if row >= PLAYLIST_TRACKS {
            far_clips += track.clips.len();
            continue;
        }
        if let Some(name) = track.name().filter(|n| !n.is_empty()) {
            playlist.track_names.insert(row, name);
        }
        if track.data().and_then(|d| d.enabled) == Some(false) {
            playlist.muted_tracks.insert(row);
        }
        for clip in &track.clips {
            let item = clip.item;
            let source = match &clip.target {
                ClipRef::Pattern(p) => p
                    .iid()
                    .and_then(|iid| patterns.get(&iid))
                    .map(|id| ClipSource::Pattern(*id)),
                ClipRef::Channel(c) => channels
                    .get(&c.iid)
                    .filter(|c| c.kind == ChannelKind::Sampler)
                    .map(|c| ClipSource::Audio(c.id)),
            };
            let Some(source) = source else {
                other_clips += 1;
                continue;
            };
            let offset = match source {
                ClipSource::Pattern(_) if item.start_offset > 0.0 => {
                    scale.ticks(f64::from(item.start_offset))
                }
                _ => 0,
            };
            playlist.clips.push(Clip {
                source,
                track: row,
                start: scale.ticks(f64::from(item.position)),
                length: scale.ticks(f64::from(item.length)).max(1),
                offset,
                muted: false,
            });
        }
    }
    playlist.sort_clips();
    let mut start_sig = None;
    for marker in arrangement.timemarkers() {
        let Some(position) = marker.position() else {
            continue;
        };
        let tick = scale.ticks(position as f64);
        let action = match (marker.is_signature(), marker_sig(&marker)) {
            (Some(false), _) => MarkerAction::None,
            // The song's signature at its start is the project's.
            (Some(true), Some(sig)) if tick == 0 => {
                start_sig = Some(sig);
                continue;
            }
            (Some(true), Some(sig)) => MarkerAction::TimeSignature(sig),
            _ => continue,
        };
        // One marker per position.
        if playlist.markers.iter().any(|m| m.tick == tick) {
            continue;
        }
        let name = marker.name().filter(|n| !n.is_empty());
        playlist.markers.push(Marker {
            tick,
            name: match action {
                MarkerAction::TimeSignature(sig) => name.unwrap_or_else(|| sig.to_string()),
                _ => name.unwrap_or_default(),
            },
            action,
        });
    }
    playlist.markers.sort_by_key(|m| m.tick);
    if let Some(sig) = start_sig {
        project.time_signature = sig;
    }
    if let Some((Some(start), Some(end))) = fl.arrangements().loop_pos() {
        let (start, end) = (scale.ticks(f64::from(start)), scale.ticks(f64::from(end)));
        if end > start {
            playlist.loop_range = Some((start, end));
        }
    }
    if far_clips > 0 {
        report.drop(format!(
            "{far_clips} clips on playlist rows past {PLAYLIST_TRACKS}"
        ));
    }
    if other_clips > 0 {
        report.drop(format!("{other_clips} automation and other channel clips"));
    }
}

/// FL inserts by number: their position in the file, master first. (PyFLP's
/// `iid` is one less; channels' insert numbers count by position.)
fn by_number<'b, 'a>(inserts: &'b [Insert<'a>]) -> BTreeMap<usize, &'b Insert<'a>> {
    inserts.iter().enumerate().collect()
}

fn has_effects(insert: &Insert<'_>) -> bool {
    insert
        .slots()
        .unwrap_or_default()
        .iter()
        .any(|s| s.plugin().is_some())
}

/// How many mixer tracks to import (master included).
fn mixer_size(inserts: &BTreeMap<usize, &Insert<'_>>, channels: &[Channel]) -> usize {
    let mut last = DEFAULT_INSERTS;
    for (number, insert) in inserts {
        if insert.name().is_some_and(|n| !n.is_empty()) || has_effects(insert) {
            last = last.max(*number);
        }
    }
    for channel in channels {
        last = last.max(channel.mixer_track);
    }
    (last + 1).min(MAX_MIXER_TRACKS).min(inserts.len().max(1))
}

fn convert_mixer(inserts: &[Insert<'_>], project: &mut Project, report: &mut Report) {
    let inserts = by_number(inserts);
    if inserts.is_empty() {
        return;
    }
    let size = mixer_size(&inserts, &project.channels);
    let mut tracks = Vec::with_capacity(size);
    let mut dropped_effects = Vec::new();
    for number in 0..size {
        let default_name = if number == 0 {
            "Master".to_owned()
        } else {
            format!("Insert {number}")
        };
        let mut track = MixerTrack::new(default_name);
        if let Some(insert) = inserts.get(&number) {
            convert_insert(insert, number, size, &mut track);
            for slot in insert.slots().unwrap_or_default() {
                let Some(plugin) = slot.plugin() else {
                    continue;
                };
                let label = slot
                    .name()
                    .or_else(|| slot.internal_name())
                    .unwrap_or_else(|| plugin.class().to_owned());
                match vst3_ref(&plugin, &label, project) {
                    Some(plugin) if track.effects.len() < MAX_EFFECT_SLOTS => {
                        let mut effect = EffectSlot::plugin(plugin);
                        effect.enabled = slot.enabled().unwrap_or(true);
                        track.effects.push(effect);
                    }
                    _ => dropped_effects.push(format!("“{label}” on {}", track.name)),
                }
            }
        }
        tracks.push(track);
    }
    project.mixer = tracks;
    for channel in &mut project.channels {
        if channel.mixer_track >= size {
            report.drop(format!(
                "the routing of “{}” to a missing insert",
                channel.name
            ));
            channel.mixer_track = 0;
        }
    }
    if !dropped_effects.is_empty() {
        report.drop(format!("effects {}", dropped_effects.join(", ")));
    }
}

fn convert_insert(insert: &Insert<'_>, number: usize, size: usize, track: &mut MixerTrack) {
    if let Some(name) = insert.name().filter(|n| !n.is_empty()) {
        track.name = name;
    }
    if let Some(volume) = insert.volume() {
        track.volume = (volume as f32 / 12800.0).clamp(0.0, 1.25);
    }
    if let Some(pan) = insert.pan() {
        track.pan = (pan as f32 / 6400.0).clamp(-1.0, 1.0);
    }
    if let Some(separation) = insert.stereo_separation() {
        track.separation = (-separation as f32 / 64.0).clamp(-1.0, 1.0);
    }
    track.phase_invert = insert.polarity_reversed() == Some(true);
    track.swap_sides = insert.channels_swapped() == Some(true);
    track.mute = insert.enabled() == Some(false);
    track.solo = insert.is_solo() == Some(true);
    if number == 0 {
        return;
    }
    let Some(flags) = insert.routing() else {
        return;
    };
    track.to_master = flags.first() == Some(&true);
    for target in number + 1..size.min(flags.len()) {
        if flags.get(target) != Some(&true) || track.sends.len() >= MAX_SENDS {
            continue;
        }
        let level = insert
            .route_level(target)
            .map_or(1.0, |l| (l as f32 / 12800.0).clamp(0.0, 1.25));
        // A route at level 0 sends nothing.
        if level > 0.0 {
            track.sends.push(MixerSend { target, level });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vst3_ids_follow_the_running_platforms_byte_order() {
        let guid = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 9, 10, 11, 12, 13, 14, 15, 16,
        ];
        assert_eq!(
            vst3_id(guid, true, true),
            "vst3:0102030405060708090A0B0C0D0E0F10"
        );
        assert_eq!(
            vst3_id(guid, true, false),
            "vst3:0403020106050807090A0B0C0D0E0F10"
        );
        assert_eq!(vst3_id(guid, false, true), vst3_id(guid, true, false));
    }

    #[test]
    fn ticks_rescale_to_96_ppq() {
        assert_eq!(Scale(96).ticks(384.0), 384);
        assert_eq!(Scale(192).ticks(384.0), 192);
        assert_eq!(Scale(960).ticks(1000.0), 100);
        assert_eq!(Scale(96).ticks(-1.0), 0);
    }
}

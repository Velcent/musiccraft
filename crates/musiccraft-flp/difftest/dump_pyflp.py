"""Dumps what PyFLP reads from an FL Studio file as JSON, for comparing with
MusicCraft's Rust port (musiccraft-flp, example `flp_dump`).

Usage: python dump_pyflp.py FILE > out.json
Every property is read inside a guard: one PyFLP raises on becomes
{"error": "<exception name>"}.
"""

import enum
import json
import pathlib
import sys

import pyflp
from pyflp.arrangement import ChannelPLItem, PatternPLItem
from pyflp.channel import Automation, Instrument, Layer, Sampler
from pyflp.plugin import VSTPlugin


def norm(v):
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, enum.Enum):
        return int(v.value) if isinstance(v.value, int) else str(v.value)
    if isinstance(v, int):
        return int(v)
    if isinstance(v, float):
        return v
    if isinstance(v, pathlib.PurePath):
        return str(v)
    if isinstance(v, bytes):
        return {"bytes": len(v)}
    if hasattr(v, "_asdict"):  # RGBA, MusicalTime
        return [norm(x) for x in v]
    if isinstance(v, (list, tuple)):
        return [norm(x) for x in v]
    if isinstance(v, dict):
        return {str(k): norm(x) for k, x in v.items()}
    return repr(v)


def get(obj, name):
    try:
        return norm(getattr(obj, name))
    except Exception as exc:  # noqa: BLE001
        return {"error": type(exc).__name__}


def props(obj, names):
    return {n: get(obj, n) for n in names}


def guard(fn):
    try:
        return fn()
    except Exception as exc:  # noqa: BLE001
        return {"error": type(exc).__name__}


NATIVE_PROPS = {
    "BooBass": ["bass", "mid", "high"],
    "FruitKick": ["max_freq", "min_freq", "freq_decay", "amp_decay", "click", "distortion"],
    "FruityBalance": ["pan", "volume"],
    "FruityBloodOverdrive": ["pre_band", "color", "pre_amp", "x100", "post_filter", "post_gain"],
    "FruityCenter": ["enabled"],
    "FruityFastDist": ["pre", "threshold", "kind", "mix", "post"],
    "FruityNotebook2": ["active_page", "editable", "pages"],
    "FruitySend": ["pan", "dry", "volume", "send_to"],
    "FruitySoftClipper": ["threshold", "post"],
    "FruityStereoEnhancer": ["pan", "volume", "stereo_separation", "phase_offset",
                             "effect_position", "phase_inversion"],
    "Plucked": ["decay", "color", "normalize", "gate", "widen"],
    "Soundgoodizer": ["mode", "amount"],
}

VST_OPTIONS = {
    "automation": ["notify_changes"],
    "compatibility": ["buffers_maxsize", "fast_idle", "fixed_buffers", "process_maximum",
                      "reset_on_transport", "send_loop", "use_time_offset"],
    "midi": ["input", "output", "pb_range", "send_modx", "send_pb", "send_release"],
    "processing": ["allow_sd", "bridged", "external", "keep_state", "multithreaded",
                   "notify_render", "process_inactive"],
    "ui": ["accept_drop", "always_update", "dpi_aware", "scale_editor"],
}


def plugin(p):
    if p is None:
        return None
    out = {"class": type(p).__name__}
    if isinstance(p, VSTPlugin):
        out.update(props(p, ["name", "vendor", "plugin_path", "fourcc"]))
        out["state"] = get(p, "state")
        out["guid"] = get(p, "guid")
        out["options"] = {k: props(getattr(p, k), v) for k, v in VST_OPTIONS.items()}
    if type(p).__name__ in NATIVE_PROPS:
        out["params"] = props(p, NATIVE_PROPS[type(p).__name__])
        pages = out["params"].get("pages")
        if isinstance(pages, list):
            # The Rust port leaves out the empty (index -1) entries and
            # construct's `_io` bookkeeping.
            out["params"]["pages"] = [
                {k: v for k, v in pg.items() if k != "_io"} for pg in pages if "value" in pg
            ]
    out.update(props(p, ["generator", "visible", "width", "height", "page", "compact",
                         "demo_mode", "detached", "disabled", "directx", "minimized",
                         "multithreaded", "smart_disable"]))
    return out


def channel(ch):
    out = {"class": type(ch).__name__}
    out.update(
        props(
            ch,
            ["iid", "name", "internal_name", "display_name", "enabled", "volume",
             "pan", "color", "icon", "locked", "zipped"],
        )
    )
    out["group"] = guard(lambda: ch.group.name)
    if isinstance(ch, (Instrument, Sampler)):
        out.update(props(ch, ["insert", "pitch_shift", "cut_group"]))
        out["keyboard"] = guard(lambda: props(ch.keyboard, ["fine_tune", "root_note", "main_pitch", "add_root", "key_region"]))
        out["polyphony"] = guard(lambda: props(ch.polyphony, ["max", "slide", "mono", "porta"]))
        out["time"] = guard(lambda: props(ch.time, ["swing", "gate", "shift", "full_porta"]))
        out["arp"] = guard(lambda: props(ch.arp, ["direction", "range", "chord", "time", "gate", "slide", "repeat"]))
        out["delay"] = guard(lambda: props(ch.delay, ["echoes", "feedback", "pan", "pitch_shift", "time", "mod_x", "mod_y", "fat_mode", "ping_pong"]))
        out["level_adjusts"] = guard(lambda: props(ch.level_adjusts, ["pan", "volume", "mod_x", "mod_y"]))
        out["tracking"] = guard(lambda: {k: props(t, ["middle_value", "pan", "mod_x", "mod_y"]) for k, t in (ch.tracking or {}).items()})
    if isinstance(ch, Sampler):
        out["sample_path"] = get(ch, "sample_path")
        out["au_sample_rate"] = get(ch, "au_sample_rate")
        out["fx"] = guard(lambda: props(ch.fx, ["boost", "clip", "crossfade", "cutoff", "fade_in", "fade_out", "fade_stereo", "fix_trim", "freq_tilt", "inverted", "length", "normalize", "pogo", "remove_dc", "resonance", "reverse", "ringmod", "start", "stereo_delay", "swap_stereo", "trim"]))
        out["reverb"] = guard(lambda: props(ch.fx.reverb, ["type", "mix"]))
        out["filter"] = guard(lambda: props(ch.filter, ["mod_x", "mod_y", "type"]))
        out["envelopes"] = guard(lambda: {k: props(e, ["enabled", "predelay", "attack", "hold", "decay", "sustain", "release", "amount", "synced", "attack_tension", "decay_tension", "release_tension"]) for k, e in (ch.envelopes or {}).items()})
        out["lfos"] = guard(lambda: {k: props(e, ["amount", "attack", "predelay", "speed", "synced", "retrig", "shape"]) for k, e in (ch.lfos or {}).items()})
        out["playback"] = guard(lambda: props(ch.playback, ["ping_pong_loop", "start_offset", "use_loop_points"]))
        out["stretching"] = guard(lambda: props(ch.stretching, ["mode", "multiplier", "pitch", "time"]))
        out["content"] = guard(lambda: props(ch.content, ["declick_mode", "keep_on_disk", "load_regions", "load_slices", "resample"]))
    if isinstance(ch, Instrument):
        out["plugin"] = guard(lambda: plugin(ch.plugin))
    if isinstance(ch, Layer):
        out["children"] = guard(lambda: [c.iid for c in ch])
        out.update(props(ch, ["crossfade", "random"]))
    if isinstance(ch, Automation):
        out["points"] = guard(lambda: [[norm(p.position), norm(p.value), norm(p.tension)] for p in ch])
        out["lfo_amount"] = guard(lambda: norm(ch.lfo.amount))
    return out


def marker(m):
    return props(m, ["position", "type", "name", "numerator", "denominator"])


def pattern(p):
    out = props(p, ["iid", "name", "length", "looped", "color"])
    out["notes"] = guard(lambda: [
        [n.position, n.length, n["key"], n.rack_channel, n.velocity, n.pan,
         n.fine_pitch, n.release, n.mod_x, n.mod_y, n.group, n.midi_channel, n.slide]
        for n in p.notes])
    out["controllers"] = guard(lambda: [[c.position, c.channel, c.value] for c in p.controllers])
    out["timemarkers"] = guard(lambda: [marker(m) for m in p.timemarkers])
    return out


def item(it):
    if isinstance(it, PatternPLItem):
        ref = ["pattern", guard(lambda: it.pattern.iid)]
    elif isinstance(it, ChannelPLItem):
        ref = ["channel", guard(lambda: it.channel.iid)]
    else:
        ref = [type(it).__name__, None]
    return ref + [get(it, "position"), get(it, "length"), get(it, "group"), get(it, "offsets")]


def track(t):
    out = props(t, ["iid", "name", "enabled", "color", "icon", "height", "locked",
                    "content_locked", "grouped", "motion", "press", "trigger_sync",
                    "position_sync", "queued", "tolerant"])
    out["items"] = guard(lambda: [item(i) for i in t])
    return out


def arrangement(a):
    out = props(a, ["iid", "name"])
    out["timemarkers"] = guard(lambda: [marker(m) for m in a.timemarkers])
    out["tracks"] = guard(lambda: [track(t) for t in a.tracks])
    return out


def slot(s):
    out = props(s, ["index", "name", "internal_name", "enabled", "mix", "color", "icon"])
    out["plugin"] = guard(lambda: plugin(s.plugin))
    return out


def insert(i):
    out = props(i, ["iid", "name", "volume", "pan", "stereo_separation", "enabled",
                    "bypassed", "channels_swapped", "polarity_reversed", "is_solo",
                    "locked", "separator_shown", "color", "icon", "input", "output", "dock"])
    out["routes"] = guard(lambda: list(i.routes))
    out["eq"] = guard(lambda: {b: props(getattr(i.eq, b), ["gain", "freq", "reso"]) for b in ("low", "mid", "high")})
    out["slots"] = guard(lambda: [slot(s) for s in i])
    return out


def dump(path):
    p = pyflp.parse(path)
    out = props(p, ["version", "ppq", "format", "channel_count", "tempo", "title",
                    "artists", "genre", "comments", "url", "pan_law", "main_pitch",
                    "main_volume", "looped", "show_info", "licensed", "licensee",
                    "data_path"])
    out["created_on"] = guard(lambda: p.created_on.isoformat() if p.created_on else None)
    out["time_spent"] = guard(lambda: p.time_spent.total_seconds() if p.time_spent else None)
    out["rack"] = guard(lambda: props(p.channels, ["swing", "height", "fit_to_steps"]))
    out["groups"] = guard(lambda: [g.name for g in p.channels.groups])
    out["channels"] = guard(lambda: [channel(c) for c in p.channels])
    out["patterns_info"] = guard(lambda: {"play_cut_notes": norm(p.patterns.play_cut_notes),
                                          "current": norm(p.patterns.current.iid) if p.patterns.current else None})
    out["patterns"] = guard(lambda: [pattern(x) for x in p.patterns])
    out["arrangements_info"] = guard(lambda: {"loop_pos": norm(p.arrangements.loop_pos),
                                              "max_tracks": norm(p.arrangements.max_tracks),
                                              "time_signature": props(p.arrangements.time_signature, ["num", "beat"]),
                                              "current": norm(p.arrangements.current.iid) if p.arrangements.current else None})
    out["arrangements"] = guard(lambda: [arrangement(a) for a in p.arrangements])
    out["mixer"] = guard(lambda: {"apdc": get(p.mixer, "apdc"),
                                  "max_inserts": get(p.mixer, "max_inserts"),
                                  "max_slots": get(p.mixer, "max_slots"),
                                  "inserts": [insert(i) for i in p.mixer]})
    return out


def preset_plugin_type(events):
    """The model PyFLP's tests would pick for a plugin preset's data."""
    import pyflp.plugin as pp

    try:
        data = events.first(pp.PluginID.Data)
    except KeyError:
        return None
    if isinstance(data, pp.UnknownDataEvent):
        return pp._PluginBase
    return getattr(pp, type(data).__name__[: -len("Event")])


def dump_preset(path):
    """Reads a preset as PyFLP's tests do (tests/conftest.py `get_model`)."""
    from pyflp.channel import Automation, Instrument, Layer, Sampler
    from pyflp.mixer import Insert, Mixer, MixerID
    from pyflp.pattern import Pattern, PatternID
    from pyflp.plugin import PluginID

    p = pyflp.parse(path)
    kind = pathlib.Path(path).parent.name
    if kind == "channels":
        return {cls.__name__: guard(lambda cls=cls: channel(cls(p.events)))
                for cls in (Sampler, Instrument, Layer, Automation)}
    if kind == "inserts":
        def one():
            mixer = Mixer(p.events)
            params = mixer.events.first(MixerID.Params)
            items = tuple(params.items_.values())[0]
            return insert(Insert(mixer.events, iid=0, max_slots=10, params=items))
        return {"insert": guard(one)}
    if kind == "patterns" and path.endswith(".fsc"):
        return {"pattern": guard(lambda: pattern(Pattern(p.events.subtree(lambda e: e.id in PatternID))))}
    if kind == "plugins":
        def one():
            ptype = preset_plugin_type(p.events)
            if ptype is None:
                return None
            return plugin(ptype(p.events.subtree(lambda e: e.id in (PluginID.Data, PluginID.Wrapper))))
        return {"plugin": guard(one)}
    return None


if __name__ == "__main__":
    try:
        preset = dump_preset(sys.argv[1]) if len(sys.argv) > 2 else None
        result = preset if preset is not None else dump(sys.argv[1])
    except Exception as exc:  # noqa: BLE001
        result = {"parse_error": type(exc).__name__}
    # JSON has no NaN; write it as null, as serde_json does.
    def no_nan(v):
        if isinstance(v, float) and v != v:
            return None
        if isinstance(v, list):
            return [no_nan(x) for x in v]
        if isinstance(v, dict):
            return {k: no_nan(x) for k, x in v.items()}
        return v

    json.dump(no_nan(result), sys.stdout, sort_keys=True, separators=(",", ":"), allow_nan=False)
    print()

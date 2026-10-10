# Spec: mixer and effects (M3)

## Behavior

- Mixer track 0 is the **master**; tracks 1… are **inserts**. New projects have
  eight inserts. At most 128 tracks.
- Each track has a name, volume (linear 0–1.25, default 1.0), pan (-1–1), mute,
  solo and up to eight insert effects processed in order.
- Channels feed exactly one track. Each insert feeds the master unless
  `mixer.route_to_master <track> off`, and may also have up to eight post-fader
  **sends** to higher-numbered inserts (`mixer.set_send <track> <target>
  <level>`; level 0 removes it). Sends only go "up", so routing can never form a
  feedback loop and tracks are processed in number order. This builds buses (for
  example drums 1–3 into insert 10) and shared effect returns.
- Removing a track removes sends into it and renumbers sends past it; undo
  restores them.
- Solo: when any insert is soloed, every insert that is not soloed, not feeding
  a soloed insert, and not fed by one is silenced. The master is never silenced
  by solo.
- Commands: `mixer.add_track`, `mixer.remove_track` (channels routed to it move to
  the master; undo restores them), `mixer.rename`, `mixer.set_volume`,
  `mixer.set_pan`, `mixer.set_mute`, `mixer.toggle_mute`, `mixer.set_solo`,
  `mixer.toggle_solo`, `mixer.set_send`, `mixer.route_to_master`, `mixer.add_effect`, `mixer.remove_effect`,
  `mixer.move_effect`, `mixer.set_effect_param`, `mixer.set_effect_enabled`,
  `mixer.toggle_effect`. Effect slots are numbered from 1. All are undoable.
- Changing parameters never interrupts audio. Adding, removing or reordering
  effects rebuilds the track's processing and resets effect tails.

## Effects

| Kind | Parameters |
| --- | --- |
| `filter` | mode (0 low-pass, 1 band-pass, 2 high-pass), cutoff Hz, resonance, mix |
| `eq` | low gain/freq (shelf), mid gain/freq/q (peak), high gain/freq (shelf) |
| `delay` | time in beats (tempo-synced), feedback, mix, ping_pong, damping |
| `reverb` | size, damping, width, mix |
| `chorus` | rate Hz, depth ms, delay ms, mix |
| `compressor` | threshold dB, ratio, attack ms, release ms, makeup dB, sidechain (0 = off, else the insert whose post-fader output drives detection) |
| `limiter` | input gain dB, ceiling dB, release ms (instant attack; output never exceeds the ceiling) |
| `distortion` | drive dB, mix, output dB |

Ranges and defaults are defined in `musiccraft-params`.

## Sidechain

- A compressor's `sidechain` parameter names an insert whose post-fader output
  drives its level detection (classic "kick ducks the bass"). The source must be
  a lower-numbered insert, or any insert for a compressor on the master, so it is
  always processed first. A source can be taken off the master
  (`mixer.route_to_master <t> off`) to key without being heard.
- Removing a track used as a sidechain source is refused; removing or inserting
  other tracks renumbers sources.

## Metering

- The engine publishes a peak level per track; the UI shows it decaying.

## Sources

- Mixer layout (master plus numbered inserts, channel routing, per-insert effect
  slots) follows the workflow described in the FL Studio online manual, "Mixer"
  section. Effects are our own implementations of textbook designs: RBJ "Audio EQ
  Cookbook" biquads, Freeverb-style reverb (public domain algorithm by Jezar),
  feed-forward compression, peak limiting, a modulated-delay chorus, tanh soft
  clipping.

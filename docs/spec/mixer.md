# Spec: mixer and effects (M3)

## Behavior

- Mixer track 0 is the **master**; tracks 1… are **inserts**. New projects have
  eight inserts. At most 128 tracks.
- Each track has a name, volume (linear 0–1.25, default 1.0), pan (-1–1), mute,
  solo and up to eight insert effects processed in order.
- Channels feed exactly one track. Every insert feeds the master.
- Solo: when any insert is soloed, every non-soloed insert is silenced. The master
  is never silenced by solo.
- Commands: `mixer.add_track`, `mixer.remove_track` (channels routed to it move to
  the master; undo restores them), `mixer.rename`, `mixer.set_volume`,
  `mixer.set_pan`, `mixer.set_mute`, `mixer.toggle_mute`, `mixer.set_solo`,
  `mixer.toggle_solo`, `mixer.add_effect`, `mixer.remove_effect`,
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
| `compressor` | threshold dB, ratio, attack ms, release ms, makeup dB |
| `distortion` | drive dB, mix, output dB |

Ranges and defaults are defined in `musiccraft-params`.

## Metering

- The engine publishes a peak level per track; the UI shows it decaying.

## Sources

- Mixer layout (master plus numbered inserts, channel routing, per-insert effect
  slots) follows the workflow described in the FL Studio online manual, "Mixer"
  section. Effects are our own implementations of textbook designs: RBJ "Audio EQ
  Cookbook" biquads, Freeverb-style reverb (public domain algorithm by Jezar),
  feed-forward compression, tanh soft clipping.

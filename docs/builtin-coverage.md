# Built-in plugin coverage

How the stock generators and effects listed in the FL Studio online manual
("Plugins" section) map onto MusicCraft's own built-ins. Product names appear
here only as references for comparison; MusicCraft's instruments and effects
are independent implementations of textbook designs with their own names,
artwork and presets (see `docs/clean-room.md`).

Status:
- **covered**: the musical job is done by a MusicCraft built-in.
- **partial**: the main use is covered, but notable features are missing
  (listed).
- **not planned**: outside MusicCraft's scope, with the reason given.
- **todo**: wanted, not yet built.

## Generators

| Reference | MusicCraft | Status | Notes |
| --- | --- | --- | --- |
| Channel sampler / Audio clip | Sampler channel, audio clips | covered | |
| 3x Osc | Synth (three oscillators, sub, unison) | covered | |
| SimSynth, MiniSynth, Sawer, PoiZone, Wasp / Wasp XT, Kepler, GMS | Synth with presets | partial | One filter and one LFO per voice; no per-oscillator filters or mod matrix |
| Transistor Bass, BooBass | Synth "Acid bass"/"Sub bass" presets with slide notes | covered | |
| Fruity DX10 | FM synth (4 operators, 6 algorithms) | covered | |
| Sytrus, Toxic Biohazard | FM synth | partial | 4 operators instead of 6, no per-operator filters or waveform drawing |
| Morphine, Ogun | Additive synth (harmonic drawbars) | partial | No morphing between spectra, no resynthesis |
| Harmor, Harmless | Additive synth, synth | partial | No image or audio resynthesis, no blur/prism |
| FL Keys | Keys: piano | covered | |
| (electric piano, organ presets) | Keys: e-piano, organ (9 drawbars) | covered | |
| Plucked!, Sakura | Pluck (Karplus–Strong string) | partial | Sakura's bowing and multi-string model are not implemented |
| Drumaxx, Fruity Drum Synth Live, Fruity Kick, Fruit Kick | Drum synth (kick, snare, hat, tom, clap) | covered | |
| FPC, Drumpad | Drum kit (16 pads) | covered | |
| DirectWave | Sampler, drum kit | partial | No multi-sample key or velocity zones |
| Slicex, Fruity Slicer | `channel.slice` (sample slicing into steps) | covered | |
| Fruity Granulizer | Granular instrument | covered | |
| Layer | Layer channel | covered | |
| Fruity Wrapper | CLAP plugin hosting | partial | CLAP only. VST3 is a `todo` (extending the `unsafe` exception needs a decision); VST2 and AU are not planned |
| FLEX, Autogun, BeepMap | — | not planned | Preset-player or image-to-sound tools; no equivalent design planned |
| MIDI Out | MIDI Out channel (`channel.add_midi_out`) | partial | Notes on a MIDI channel to one output port; no controller knobs, program changes or MIDI clock |
| Patcher | — | not planned | Modular plugin graphs; mixer routing and sends cover the common cases |
| Fruity Envelope / Keyboard / X-Y controllers, Formula Controller, Peak Controller | Automation clips, synth LFO | partial | No audio-driven (peak) or formula controllers |
| Dashboard, Fruity Video Player, ZGameEditor Visualizer | — | not planned | Video and custom-panel tools are outside an audio workstation's core |

## Effects

| Reference | MusicCraft | Status | Notes |
| --- | --- | --- | --- |
| Fruity Filter, Fast LP, Free Filter | `filter` | covered | |
| Fruity Love Philter | `auto_filter` | partial | One LFO and an envelope follower; no drawn envelope shapes per target |
| Fruity 7 Band EQ | `graphic_eq` | covered | |
| Fruity Parametric EQ / EQ 2 | `parametric_eq` (7 bands), `eq` (3 bands) | covered | |
| Fruity Bass Boost | `bass_boost` | covered | |
| Fruity Compressor | `compressor` (with sidechain) | covered | |
| Fruity Multiband Compressor | `multiband` | covered | |
| Fruity Limiter | `dynamics` (gate, compressor and limiter in one insert); also `limiter`, `compressor`, `noise_gate` | covered | |
| Maximus | `multiband` + `limiter` | partial | No combined multiband limiter in one effect |
| Soundgoodizer | `multiband` "Upfront" preset | partial | |
| Transient Processor | `transient` | covered | |
| Fruity Soft Clipper | `soft_clipper` | covered | |
| Fruity WaveShaper | `waveshaper` | partial | Six fixed curves; the curve cannot be drawn |
| Fruity Blood Overdrive, Fast Dist | `overdrive`, `distortion` | covered | |
| Fruity Squeeze | `bitcrusher`, `waveshaper` | covered | |
| Hardcore | `overdrive`, `waveshaper`, `auto_filter`, `chorus` | partial | No single guitar-pedal chain effect |
| Effector | Separate effects | partial | No all-in-one effect |
| Fruity Delay 2 / 3 | `delay` | covered | |
| Fruity Delay Bank, Multiband Delay | `multi_tap` (per-tap pan, low cut and high cut) | covered | Four taps |
| Fruity Reeverb 2, Luxeverb | `reverb` | covered | |
| Fruity Convolver | `convolution` (files or generated rooms) | covered | |
| Fruity Chorus, Vintage Chorus | `chorus` | covered | |
| Fruity Flanger, Flangus | `flanger` | covered | |
| Fruity Phaser, Vintage Phaser | `phaser` | covered | |
| Fruity Pan O Matic | `autopan` | covered | |
| Fruity Stereo Enhancer, Balance, Center, Phase Inverter | `stereo`, `dc_block` | covered | |
| Fruity Stereo Shaper | `stereo_shaper` (left/right mixing matrix, inter-channel delay) | covered | |
| Fruity Vocoder, Vocodex | `vocoder` (sidechain modulator) | covered | 16 fixed bands |
| Frequency Shifter | `freq_shift` | covered | Ring-mod mode is the separate `ring_mod` |
| Pitch Shifter | `pitch_shift` | covered | |
| Pitcher, Newtone | `pitch_correct` | partial | Real-time correction to a key and scale; no MIDI-controlled target notes, harmonies or offline note editing |
| Gross Beat | `gate`, `stutter` | partial | No drawn time/volume envelopes |
| Fruity Scratcher | — | not planned | Turntable simulation |
| Frequency Splitter | — | not planned | Splitting into separate mixer tracks; `multiband` covers band processing |
| Fruity Send, Fruity Mute 2 | Mixer sends and mute | covered | |
| Fruity Spectroman, Wave Candy | Spectrum analyzer, toolbar oscilloscope | covered | |
| Fruity dB Meter | Mixer meters | covered | |
| Tuner | `tuner` | covered | |
| Edison | Sampler slicing, recording to audio clips | not planned | A full audio editor is out of scope |
| Newtime | — | not planned | Time-stretch editor |
| Fruity Big Clock, Notebook, HTML Notebook, Control Surface | — | not planned | Not audio processing |

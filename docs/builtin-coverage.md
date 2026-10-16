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
| SimSynth, MiniSynth, Sawer, PoiZone, Wasp / Wasp XT, Kepler, GMS | Synth with presets (two filters with series/parallel/split routing, two LFOs, mod envelope, three-slot mod matrix) | covered | |
| Transistor Bass, BooBass | Synth bass presets ("Acid bass", "Sub bass", "Round bass", "Picked bass", "Fretless") with slide notes | covered | |
| Fruity DX10 | FM synth (4 operators, 6 algorithms) | covered | |
| Sytrus, Toxic Biohazard | FM synth (6 operators, 12 algorithms, per-operator waveforms, output filter) | partial | One filter on the output rather than per operator; waveforms are picked, not drawn |
| Morphine, Ogun | Additive synth (two spectra with a timed and LFO morph), resynth | covered | |
| Harmor, Harmless | Resynth (`channel.add_resynth`: a sample's harmonics over time, with speed, blur and stretch), additive synth, synth | partial | No image resynthesis; prism is the harmonic stretch |
| FL Keys | Keys: piano | covered | |
| (electric piano, organ presets) | Keys: e-piano, organ (9 drawbars) | covered | |
| Plucked!, Sakura | Pluck (Karplus–Strong strings with bowing, up to three detuned strings) | covered | |
| Drumaxx, Fruity Drum Synth Live, Fruity Kick, Fruit Kick | Drum synth (kick, snare, hat, tom, clap) | covered | |
| FPC, Drumpad | Drum kit (16 pads) | covered | |
| DirectWave | Zone sampler (`channel.add_zones`: key and velocity zones with their own root keys), sampler, drum kit | covered | |
| Slicex, Fruity Slicer | `channel.slice` (sample slicing into steps) | covered | |
| Fruity Granulizer | Granular instrument | covered | |
| Layer | Layer channel | covered | |
| Fruity Wrapper | CLAP and VST3 plugin hosting | partial | CLAP and VST3 (VST3 windows on Linux only so far; VST3 on macOS not yet); VST2 and AU are not planned |
| FLEX, Autogun, BeepMap | — | not planned | Preset-player or image-to-sound tools; no equivalent design planned |
| MIDI Out | MIDI Out channel (`channel.add_midi_out`) | covered | Notes, eight control-change knobs, program change and MIDI clock to one output port |
| Patcher | `rack` (four parallel chains of up to six built-in effects, branch level and pan, dry mix) | partial | Effect graphs only, as parallel chains; instruments combine through layer channels, and plugins cannot go inside a rack |
| Fruity Envelope / Keyboard / X-Y controllers, Formula Controller, Peak Controller | Controllers (`controller.add`: lfo, envelope, keyboard, peak, formula, macro), automation clips | covered | One target per controller; an X/Y pad is two macros |
| Dashboard, Fruity Video Player, ZGameEditor Visualizer | — | not planned | Video and custom-panel tools are outside an audio workstation's core |

## Effects

| Reference | MusicCraft | Status | Notes |
| --- | --- | --- | --- |
| Fruity Filter, Fast LP, Free Filter | `filter` | covered | |
| Fruity Love Philter | `auto_filter` | covered | One LFO (five shapes or drawn) driving cutoff, volume and pan, plus an envelope follower |
| Fruity 7 Band EQ | `graphic_eq` | covered | |
| Fruity Parametric EQ / EQ 2 | `parametric_eq` (7 bands), `eq` (3 bands) | covered | |
| Fruity Bass Boost | `bass_boost` | covered | |
| Fruity Compressor | `compressor` (with sidechain) | covered | |
| Fruity Multiband Compressor | `multiband` | covered | |
| Fruity Limiter | `dynamics` (gate, compressor and limiter in one insert); also `limiter`, `compressor`, `noise_gate` | covered | |
| Maximus | `multiband_limiter` | covered | Three bands with a master ceiling |
| Soundgoodizer | `multiband_limiter` "Enhance" presets with the one-knob `amount` | covered | |
| Transient Processor | `transient` | covered | |
| Fruity Soft Clipper | `soft_clipper` | covered | |
| Fruity WaveShaper | `waveshaper` | covered | Six fixed curves plus a nine-point drawn curve |
| Fruity Blood Overdrive, Fast Dist | `overdrive`, `distortion` (soft or hard-threshold mode) | covered | |
| Fruity Squeeze | `bitcrusher` (with squeeze), `waveshaper` | covered | |
| Hardcore | `guitar_chain` (compressor, drive, tone stack, chorus, cabinet) | covered | Fixed stage order |
| Effector | `multi_fx` (13 blocks on an X/Y pad) | covered | |
| Distructor | `fx_chain` (four X/Y blocks in series) | covered | |
| Fruity Delay 2 / 3 | `delay` (offset, modulation, feedback low cut, diffusion, saturation) | covered | |
| Fruity Delay Bank, Multiband Delay | `multi_tap` (per-tap pan, low cut and high cut) | covered | Four taps |
| Fruity Reeverb 2, Luxeverb | `reverb`, `hall_reverb` (dense FDN with early reflections and modulation) | covered | |
| Fruity Convolver | `convolution` (files or generated rooms) | covered | |
| Fruity Chorus, Vintage Chorus | `chorus` (up to three voices, darkening tone) | covered | |
| Fruity Flanger, Flangus | `flanger` | covered | |
| Fruity Phaser, Vintage Phaser | `phaser` (with warm feedback saturation) | covered | |
| Fruity Pan O Matic | `autopan` | covered | |
| Fruity Stereo Enhancer, Balance, Center, Phase Inverter | `stereo`, `dc_block` | covered | |
| Fruity Stereo Shaper | `stereo_shaper` (left/right mixing matrix, inter-channel delay) | covered | |
| Fruity Vocoder, Vocodex | `vocoder` (sidechain modulator) | covered | 4–64 bands over a chosen range |
| Frequency Shifter | `freq_shift` | covered | Ring-mod mode is the separate `ring_mod` |
| Pitch Shifter | `pitch_shift` | covered | |
| Pitcher | `pitch_correct` (scale or a followed channel's notes, two harmony voices) | covered | |
| Newtone | `sample.edit … tune` and `pitch`, `pitch_correct` | partial | Offline correction to a key and scale; no per-note editing |
| Gross Beat | `beat_shaper` (drawn time and volume envelopes), `gate`, `stutter` | covered | Eight points per envelope |
| Fruity Scratcher | `scratch` (held span, automatable position, scratch patterns) | covered | Scratches the insert's own audio rather than a loaded sample |
| Frequency Splitter | `band_split` (three bands to separate inserts) | covered | |
| Fruity Send, Fruity Mute 2 | `send` effect (mid-chain send with dry control), mixer sends and mute | covered | |
| Fruity Spectroman, Wave Candy | Spectrum analyzer, toolbar oscilloscope | covered | |
| Fruity dB Meter | Mixer meters | covered | |
| Tuner | `tuner` | covered | |
| Edison | Sample editor (`sample.edit`: trim, cut, silence, fades, gain, normalize, reverse, stretch, pitch, tune), recording to audio clips | partial | Edits write new files; no spectral view or noise reduction |
| Newtime | `sample.edit … stretch` (WSOLA time-stretch) | partial | Whole-sample stretch; no warp markers |
| Fruity Big Clock, Notebook, HTML Notebook, Control Surface | — | not planned | Not audio processing |

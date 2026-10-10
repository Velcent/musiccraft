use std::path::Path;

use musiccraft_commands::{Command, CommandError, Outcome, Registry};
use musiccraft_engine::{EngineMessage, MAX_BLOCK, Sequence, engine_pair};
use musiccraft_params::PPQ;
use musiccraft_project::{Project, Transport};
use musiccraft_samples::{WavFormat, write_wav};

use crate::{Bridge, build_sequence};

/// Settings for an offline render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    /// Output sample rate in hertz.
    pub sample_rate: u32,
    /// How many times to play the loop (pattern or song).
    pub loops: u32,
    /// Longest tail to keep after the last loop, in seconds. Rendering stops
    /// earlier once the output has been silent for half a second.
    pub max_tail_seconds: f64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            sample_rate: 44_100,
            loops: 1,
            max_tail_seconds: 10.0,
        }
    }
}

/// Level below which the tail counts as silent (about -90 dBFS).
const SILENCE: f32 = 3.2e-5;

/// Renders the project's loop (current pattern or song, by play mode) to stereo
/// audio, followed by the decaying tail of notes and effects.
///
/// Returns the left and right channels and any warnings (for example samples
/// that failed to load).
#[must_use]
pub fn render(project: &Project, options: RenderOptions) -> (Vec<f32>, Vec<f32>, Vec<String>) {
    let sample_rate = options.sample_rate.max(8_000);
    let (mut engine, mut handle) = engine_pair(sample_rate);
    let mut bridge = Bridge::new(sample_rate);
    let mut stopped = project.clone();
    stopped.set_transport(Transport::Stopped);
    let report = bridge.sync(&stopped, &mut handle);

    // Repeat the loop `loops` times, then leave room for the tail without
    // wrapping back to the start.
    let base = build_sequence(project);
    let loop_ticks = base.length().max(1);
    let loops = u64::from(options.loops.max(1));
    let mut events = Vec::new();
    for i in 0..loops {
        events.extend(base.events().iter().map(|e| musiccraft_engine::NoteEvent {
            start: e.start + i * loop_ticks,
            ..*e
        }));
    }
    let ticks_per_second = project.tempo_bpm() / 60.0 * f64::from(PPQ);
    let tail_ticks = (options.max_tail_seconds.max(0.0) * ticks_per_second).ceil() as u64 + 1;
    let total_ticks = loop_ticks * loops;
    let sequence = Sequence::new(events, total_ticks + tail_ticks);
    let _ = handle.send(EngineMessage::SetSequence(Box::new(sequence)));
    let _ = handle.send(EngineMessage::Play);

    let body_frames =
        (total_ticks as f64 / ticks_per_second * f64::from(sample_rate)).ceil() as usize;
    let max_tail_frames = (options.max_tail_seconds.max(0.0) * f64::from(sample_rate)) as usize;
    let silence_frames = sample_rate as usize / 2;

    let mut left = Vec::with_capacity(body_frames);
    let mut right = Vec::with_capacity(body_frames);
    let mut block = vec![0.0_f32; MAX_BLOCK * 2];
    let mut quiet = 0;
    while left.len() < body_frames + max_tail_frames {
        let remaining = body_frames + max_tail_frames - left.len();
        let frames = remaining.min(MAX_BLOCK);
        let out = &mut block[..frames * 2];
        engine.process(out, 2);
        handle.collect_garbage();
        for frame in out.chunks_exact(2) {
            left.push(frame[0]);
            right.push(frame[1]);
        }
        if left.len() > body_frames {
            let loud = out.iter().any(|s| s.abs() > SILENCE);
            quiet = if loud { 0 } else { quiet + frames };
            if quiet >= silence_frames {
                break;
            }
        }
    }
    (left, right, report.warnings)
}

/// Renders the project and writes it as a stereo WAV file.
///
/// # Errors
///
/// Returns a message if the file cannot be written.
pub fn export_wav(
    project: &Project,
    path: &Path,
    format: WavFormat,
    options: RenderOptions,
) -> Result<(f64, Vec<String>), String> {
    let (left, right, warnings) = render(project, options);
    write_wav(path, &left, &right, options.sample_rate, format).map_err(|e| e.to_string())?;
    Ok((left.len() as f64 / f64::from(options.sample_rate), warnings))
}

/// `project.export_wav <path> [16|24|32f] [loops] [sample rate]`.
pub struct ExportCommand;

impl Command for ExportCommand {
    fn name(&self) -> &'static str {
        "project.export_wav"
    }

    fn summary(&self) -> &'static str {
        "Render the pattern or song (by play mode) to a WAV file."
    }

    fn usage(&self) -> &'static str {
        "project.export_wav <path> [16|24|32f] [loops] [sample rate]"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let invalid = |reason: String| CommandError::InvalidArgument {
            command: self.name().to_owned(),
            reason,
        };
        let [path, rest @ ..] = args else {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        };
        if rest.len() > 3 {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        }
        let format = match rest.first() {
            Some(raw) => WavFormat::parse(raw)
                .ok_or_else(|| invalid(format!("`{raw}` is not 16, 24 or 32f")))?,
            None => WavFormat::Int16,
        };
        let loops = match rest.get(1) {
            Some(raw) => raw
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=64).contains(n))
                .ok_or_else(|| invalid(format!("`{raw}` is not a loop count 1-64")))?,
            None => 1,
        };
        let sample_rate = match rest.get(2) {
            Some(raw) => raw
                .parse::<u32>()
                .ok()
                .filter(|n| (8_000..=192_000).contains(n))
                .ok_or_else(|| invalid(format!("`{raw}` is not a sample rate 8000-192000")))?,
            None => 44_100,
        };
        let options = RenderOptions {
            sample_rate,
            loops,
            ..RenderOptions::default()
        };
        let (seconds, warnings) =
            export_wav(project, Path::new(path), format, options).map_err(CommandError::Io)?;
        let mut message = format!("exported {seconds:.2} s to {path}");
        for warning in warnings {
            message.push_str(&format!(" (warning: {warning})"));
        }
        Ok(Outcome::done(message))
    }
}

/// Adds the render crate's commands to a registry.
///
/// # Errors
///
/// Returns [`CommandError::Duplicate`] if they are already registered.
pub fn register(registry: &mut Registry) -> Result<(), CommandError> {
    registry.register(Box::new(ExportCommand))
}

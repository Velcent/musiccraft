//! Project data model: the single source of truth for what a MusicCraft project
//! contains. Nothing here knows about audio devices or user interfaces.

use std::error::Error;
use std::fmt;

/// Lowest tempo a project accepts, in beats per minute.
pub const MIN_TEMPO_BPM: f64 = 20.0;
/// Highest tempo a project accepts, in beats per minute.
pub const MAX_TEMPO_BPM: f64 = 999.0;
/// Tempo of a newly created project, in beats per minute.
pub const DEFAULT_TEMPO_BPM: f64 = 130.0;

/// Whether the project is currently playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transport {
    /// Playback is halted.
    #[default]
    Stopped,
    /// Playback is running.
    Playing,
}

impl Transport {
    /// Returns `true` while playback is running.
    #[must_use]
    pub fn is_playing(self) -> bool {
        self == Self::Playing
    }
}

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Stopped => "stopped",
            Self::Playing => "playing",
        })
    }
}

/// Errors produced when a change is rejected by the project model.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectError {
    /// The requested tempo is not finite or is outside the supported range.
    TempoOutOfRange(f64),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TempoOutOfRange(bpm) => write!(
                f,
                "tempo {bpm} is outside the supported range {MIN_TEMPO_BPM}-{MAX_TEMPO_BPM} BPM"
            ),
        }
    }
}

impl Error for ProjectError {}

/// A MusicCraft project.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    name: String,
    tempo_bpm: f64,
    transport: Transport,
}

impl Project {
    /// Creates an empty project with default settings.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tempo_bpm: DEFAULT_TEMPO_BPM,
            transport: Transport::Stopped,
        }
    }

    /// The project's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current tempo in beats per minute.
    #[must_use]
    pub fn tempo_bpm(&self) -> f64 {
        self.tempo_bpm
    }

    /// Current transport state.
    #[must_use]
    pub fn transport(&self) -> Transport {
        self.transport
    }

    /// Sets the tempo and returns the previous value.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError::TempoOutOfRange`] if `bpm` is not finite or lies
    /// outside [`MIN_TEMPO_BPM`]..=[`MAX_TEMPO_BPM`]. The project is unchanged.
    pub fn set_tempo_bpm(&mut self, bpm: f64) -> Result<f64, ProjectError> {
        if !bpm.is_finite() || !(MIN_TEMPO_BPM..=MAX_TEMPO_BPM).contains(&bpm) {
            return Err(ProjectError::TempoOutOfRange(bpm));
        }
        Ok(std::mem::replace(&mut self.tempo_bpm, bpm))
    }

    /// Sets the transport state and returns the previous state.
    pub fn set_transport(&mut self, transport: Transport) -> Transport {
        std::mem::replace(&mut self.transport, transport)
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_defaults() {
        let project = Project::default();
        assert_eq!(project.name(), "Untitled");
        assert!((project.tempo_bpm() - DEFAULT_TEMPO_BPM).abs() < f64::EPSILON);
        assert_eq!(project.transport(), Transport::Stopped);
    }

    #[test]
    fn set_tempo_returns_previous_value() {
        let mut project = Project::default();
        let previous = project.set_tempo_bpm(140.0).unwrap();
        assert!((previous - DEFAULT_TEMPO_BPM).abs() < f64::EPSILON);
        assert!((project.tempo_bpm() - 140.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tempo_range_edges_are_inclusive() {
        let mut project = Project::default();
        assert!(project.set_tempo_bpm(MIN_TEMPO_BPM).is_ok());
        assert!(project.set_tempo_bpm(MAX_TEMPO_BPM).is_ok());
    }

    #[test]
    fn invalid_tempo_is_rejected_and_project_unchanged() {
        let mut project = Project::default();
        for bad in [0.0, 19.9, 999.1, f64::NAN, f64::INFINITY, -120.0] {
            assert!(matches!(
                project.set_tempo_bpm(bad),
                Err(ProjectError::TempoOutOfRange(_))
            ));
        }
        assert!((project.tempo_bpm() - DEFAULT_TEMPO_BPM).abs() < f64::EPSILON);
    }

    #[test]
    fn set_transport_returns_previous_state() {
        let mut project = Project::default();
        assert_eq!(
            project.set_transport(Transport::Playing),
            Transport::Stopped
        );
        assert_eq!(
            project.set_transport(Transport::Stopped),
            Transport::Playing
        );
    }
}

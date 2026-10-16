// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! Small value types shared by the models (`pyflp.types`).

use std::cmp::Ordering;

/// The FL Studio version a file was saved with: `major.minor.patch[.build]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: Option<u32>,
}

impl FlVersion {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            build: None,
        }
    }

    /// The version as a tuple, as Python compares it (`dataclasses.astuple`
    /// without a missing build).
    fn parts(&self) -> Vec<u32> {
        let mut parts = vec![self.major, self.minor, self.patch];
        parts.extend(self.build);
        parts
    }

    /// Compares with a plain `(major, minor, patch)` tuple the way Python
    /// compares tuples (a longer tuple with an equal prefix is greater).
    #[must_use]
    pub fn cmp_tuple(&self, other: &[u32]) -> Ordering {
        self.parts().as_slice().cmp(other)
    }
}

impl PartialOrd for FlVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FlVersion {
    /// Major, minor and patch first; a missing build counts as lower than any
    /// build (PyFLP would raise when only the builds differ that way).
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch, self.build).cmp(&(
            other.major,
            other.minor,
            other.patch,
            other.build,
        ))
    }
}

impl std::fmt::Display for FlVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(build) = self.build {
            write!(f, ".{build}")?;
        }
        Ok(())
    }
}

/// A colour, each component 0–1 (`RGBA`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

/// A musical length: 1 bar = 16 beats, 1 beat = 240 ticks (`MusicalTime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicalTime {
    pub bars: u32,
    pub beats: u32,
    pub ticks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_like_python_tuples() {
        let v = FlVersion {
            build: Some(2576),
            ..FlVersion::new(20, 8, 4)
        };
        assert_eq!(v.cmp_tuple(&[20, 8, 4]), Ordering::Greater);
        assert_eq!(v.cmp_tuple(&[20, 9, 0]), Ordering::Less);
        assert!(v >= FlVersion::new(12, 9, 1));
        assert_eq!(v.to_string(), "20.8.4.2576");
    }
}

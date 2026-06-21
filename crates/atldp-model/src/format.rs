//! The open ATLDP project file format (`.atldp`) — phase G6.
//!
//! The format is **versioned, human-readable JSON**: a single [`Project`] object
//! serialized with `serde_json`. JSON is chosen deliberately so the format is
//! open and trivially parseable by third-party tools (the anti-lock-in goal of
//! the plan's stage 6) without committing readers to a bespoke parser. The
//! top-level `schema_version` gates forward/backward compatibility.

use std::path::Path;

use crate::{Project, SCHEMA_VERSION};

/// Errors from reading or writing an `.atldp` file.
#[derive(Debug)]
pub enum FormatError {
    /// Underlying I/O failure (path not found, permissions, …).
    Io(std::io::Error),
    /// The bytes are not valid project JSON.
    Parse(serde_json::Error),
    /// The file's schema version is newer than this build understands.
    UnsupportedVersion { found: u32, supported: u32 },
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Io(e) => write!(f, "I/O error: {e}"),
            FormatError::Parse(e) => write!(f, "not a valid .atldp project: {e}"),
            FormatError::UnsupportedVersion { found, supported } => write!(
                f,
                "project schema version {found} is newer than this build supports ({supported}); please update ATLDP"
            ),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<std::io::Error> for FormatError {
    fn from(e: std::io::Error) -> Self {
        FormatError::Io(e)
    }
}

/// Serialize a project to the `.atldp` JSON text (pretty-printed for diffability).
pub fn to_atldp_string(project: &Project) -> Result<String, FormatError> {
    serde_json::to_string_pretty(project).map_err(FormatError::Parse)
}

/// Parse `.atldp` JSON text into a [`Project`], rejecting a future schema version
/// and migrating older ones forward (none yet — v1 is the first).
pub fn from_atldp_str(text: &str) -> Result<Project, FormatError> {
    let project: Project = serde_json::from_str(text).map_err(FormatError::Parse)?;
    if project.schema_version > SCHEMA_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: project.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    Ok(migrate(project))
}

/// Migrate an older-schema project up to [`SCHEMA_VERSION`]. v1 is the first
/// schema, so this is currently the identity; it exists as the seam future
/// versions hook into.
fn migrate(project: Project) -> Project {
    project
}

/// Write a project to `path` as `.atldp` JSON.
pub fn save(project: &Project, path: impl AsRef<Path>) -> Result<(), FormatError> {
    let text = to_atldp_string(project)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Read an `.atldp` project from `path`.
pub fn load(path: impl AsRef<Path>) -> Result<Project, FormatError> {
    let text = std::fs::read_to_string(path)?;
    from_atldp_str(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileSample, Tower};

    fn sample_project() -> Project {
        let mut p = Project::new("Test line 138 kV");
        p.metadata.notes = "round-trip fixture".to_string();
        p.parameters.horizontal_tension_n = 31_500.0;
        p.ground_profile = vec![
            ProfileSample {
                distance_m: 0.0,
                elevation_m: 100.0,
            },
            ProfileSample {
                distance_m: 500.0,
                elevation_m: 80.0,
            },
            ProfileSample {
                distance_m: 1000.0,
                elevation_m: 120.0,
            },
        ];
        p.towers = vec![
            Tower {
                distance_m: 0.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 15.0,
            },
            Tower {
                distance_m: 400.0,
                ground_elevation_m: 85.0,
                attachment_height_m: 18.0,
            },
            Tower {
                distance_m: 1000.0,
                ground_elevation_m: 120.0,
                attachment_height_m: 15.0,
            },
        ];
        p
    }

    #[test]
    fn round_trips_through_string() {
        let p = sample_project();
        let text = to_atldp_string(&p).unwrap();
        let back = from_atldp_str(&text).unwrap();
        assert_eq!(back.metadata.name, p.metadata.name);
        assert_eq!(back.metadata.notes, p.metadata.notes);
        assert_eq!(back.towers.len(), 3);
        assert_eq!(back.ground_profile.len(), 3);
        assert!((back.parameters.horizontal_tension_n - 31_500.0).abs() < 1e-9);
        assert!((back.towers[1].attachment_height_m - 18.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_future_schema_version() {
        let mut p = sample_project();
        p.schema_version = SCHEMA_VERSION + 1;
        let text = to_atldp_string(&p).unwrap();
        match from_atldp_str(&text) {
            Err(FormatError::UnsupportedVersion { found, supported }) => {
                assert_eq!(found, SCHEMA_VERSION + 1);
                assert_eq!(supported, SCHEMA_VERSION);
            }
            other => panic!("expected UnsupportedVersion, got {other:?}"),
        }
    }

    #[test]
    fn ground_at_interpolates() {
        let p = sample_project();
        assert!((p.ground_at(250.0).unwrap() - 90.0).abs() < 1e-9);
        // Clamps outside the sampled range.
        assert!((p.ground_at(-10.0).unwrap() - 100.0).abs() < 1e-9);
        assert!((p.ground_at(5000.0).unwrap() - 120.0).abs() < 1e-9);
    }
}

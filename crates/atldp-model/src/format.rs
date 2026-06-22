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
/// and migrating older ones forward.
///
/// Migration is performed on the raw JSON *before* it is deserialized into the
/// current [`Project`] shape, because a breaking schema change (e.g. v1's single
/// `conductor` → v2's `wires`) renames or drops fields the typed struct no longer
/// carries.
pub fn from_atldp_str(text: &str) -> Result<Project, FormatError> {
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(FormatError::Parse)?;
    let found = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    if found > SCHEMA_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found,
            supported: SCHEMA_VERSION,
        });
    }
    migrate_value(&mut value, found);
    serde_json::from_value(value).map_err(FormatError::Parse)
}

/// Migrate raw `.atldp` JSON from `from_version` up to [`SCHEMA_VERSION`], in
/// place, one schema step at a time. Each step is the seam a future schema bump
/// hooks into; the steps chain (a v1 file runs v1→v2 then v2→v3).
fn migrate_value(value: &mut serde_json::Value, from_version: u32) {
    if from_version < 2 {
        migrate_v1_to_v2(value);
    }
    if from_version < 3 {
        migrate_v2_to_v3(value);
    }
}

/// v1 → v2 (G7/G8): wrap the single `conductor` strung at
/// `parameters.horizontal_tension_n` as a one-element `wires` set (a phase wire
/// at zero offset), and stamp the new `schema_version`. Untyped v1 towers default
/// to suspension, so the whole line forms a single tension section — reproducing
/// the v1 single-conductor results exactly.
fn migrate_v1_to_v2(value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    if obj.get("wires").is_none() {
        let tension = obj
            .get("parameters")
            .and_then(|p| p.get("horizontal_tension_n"))
            .and_then(|t| t.as_f64())
            .unwrap_or(30_000.0);
        if let Some(conductor) = obj.remove("conductor") {
            obj.insert(
                "wires".to_string(),
                serde_json::json!([{
                    "name": "Phase",
                    "role": "phase",
                    "conductor": conductor,
                    "vertical_offset_m": 0.0,
                    "lateral_offset_m": 0.0,
                    "tension_n": tension,
                }]),
            );
        }
    }
    obj.insert(
        "schema_version".to_string(),
        serde_json::json!(SCHEMA_VERSION),
    );
}

/// v2 → v3 (G9/G10): synthesise a trivial two-POI route (terminal → terminal)
/// from the existing ground-profile endpoints so the now-derived profile has a
/// route to belong to, and stamp the new `schema_version`. A v2 profile is just
/// (distance, elevation) with no georeference, so the synthetic terminals borrow
/// the terrain tile's south-west corner (or `0,0`) for lat/lon — the route is a
/// straight terminal-to-terminal line that reproduces the v2 profile exactly.
/// G10's `geometry` field defaults in via serde, so no structure-geometry work is
/// needed here.
fn migrate_v2_to_v3(value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    let needs_route = obj.get("route").map(|r| r.is_null()).unwrap_or(true);
    if needs_route {
        let samples = obj
            .get("ground_profile")
            .and_then(|p| p.as_array())
            .filter(|a| a.len() >= 2);
        if let Some(samples) = samples {
            let sample_at = |v: &serde_json::Value, key: &str| -> f64 {
                v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
            };
            let first = &samples[0];
            let last = &samples[samples.len() - 1];
            let (lat, lon) = obj
                .get("terrain")
                .filter(|t| !t.is_null())
                .map(|t| {
                    (
                        t.get("sw_lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        t.get("sw_lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    )
                })
                .unwrap_or((0.0, 0.0));
            obj.insert(
                "route".to_string(),
                serde_json::json!({
                    "pois": [
                        {
                            "kind": "terminal",
                            "lat": lat,
                            "lon": lon,
                            "distance_m": sample_at(first, "distance_m"),
                            "ground_elevation_m": sample_at(first, "elevation_m"),
                            "deviation_angle_deg": 0.0,
                            "name": "Terminal A",
                        },
                        {
                            "kind": "terminal",
                            "lat": lat,
                            "lon": lon,
                            "distance_m": sample_at(last, "distance_m"),
                            "ground_elevation_m": sample_at(last, "elevation_m"),
                            "deviation_angle_deg": 0.0,
                            "name": "Terminal B",
                        }
                    ]
                }),
            );
        }
    }
    obj.insert(
        "schema_version".to_string(),
        serde_json::json!(SCHEMA_VERSION),
    );
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
                ..Default::default()
            },
            Tower {
                distance_m: 400.0,
                ground_elevation_m: 85.0,
                attachment_height_m: 18.0,
                ..Default::default()
            },
            Tower {
                distance_m: 1000.0,
                ground_elevation_m: 120.0,
                attachment_height_m: 15.0,
                ..Default::default()
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

    /// A v1 project (single `conductor`, untyped towers) loads as a one-wire,
    /// single-section v2 project that reproduces its inputs (ADR-0015 migration).
    #[test]
    fn migrates_v1_single_conductor_to_wire_set() {
        let v1 = serde_json::json!({
            "schema_version": 1,
            "metadata": { "name": "Legacy 1-wire", "notes": "" },
            "conductor": {
                "name": "ACSR Drake 26/7",
                "unit_weight_n_per_m": 15.97,
                "diameter_m": 0.0281,
                "rated_strength_n": 140_100.0
            },
            "parameters": {
                "horizontal_tension_n": 31_500.0,
                "attachment_height_m": 15.0,
                "min_clearance_m": 8.0,
                "wind_pressure_pa": 700.0
            },
            "terrain": null,
            "towers": [
                { "distance_m": 0.0, "ground_elevation_m": 100.0, "attachment_height_m": 15.0 },
                { "distance_m": 400.0, "ground_elevation_m": 100.0, "attachment_height_m": 15.0 }
            ]
        })
        .to_string();

        let p = from_atldp_str(&v1).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        assert_eq!(p.wires.len(), 1);
        let w = &p.wires[0];
        assert_eq!(w.role, crate::WireRole::Phase);
        assert_eq!(w.conductor.name, "ACSR Drake 26/7");
        assert!((w.tension_n - 31_500.0).abs() < 1e-9);
        assert_eq!(w.vertical_offset_m, 0.0);
        // Untyped towers ⇒ all suspension ⇒ exactly one tension section.
        assert!(p.towers.iter().all(|t| !t.function.is_anchor()));
        assert_eq!(crate::tension_sections(&p.towers).len(), 1);
    }

    /// A v2 project (wires + typed towers, no `route`) loads as v3 with a trivial
    /// terminal→terminal route synthesised from its profile endpoints, and its
    /// stored profile is preserved unchanged (ADR-0019 migration).
    #[test]
    fn migrates_v2_to_v3_synthesises_route() {
        let v2 = serde_json::json!({
            "schema_version": 2,
            "metadata": { "name": "Legacy v2", "notes": "" },
            "wires": [{
                "name": "Phase", "role": "phase",
                "conductor": {
                    "name": "ACSR Drake 26/7", "unit_weight_n_per_m": 15.97,
                    "diameter_m": 0.0281, "rated_strength_n": 140_100.0
                },
                "vertical_offset_m": 0.0, "lateral_offset_m": 0.0, "tension_n": 30_000.0
            }],
            "parameters": {
                "horizontal_tension_n": 30_000.0, "attachment_height_m": 15.0,
                "min_clearance_m": 8.0, "wind_pressure_pa": 700.0
            },
            "terrain": { "source_path": "tile.hgt", "sw_lat": -23, "sw_lon": -43 },
            "ground_profile": [
                { "distance_m": 0.0, "elevation_m": 100.0 },
                { "distance_m": 1000.0, "elevation_m": 120.0 }
            ],
            "towers": []
        })
        .to_string();

        let p = from_atldp_str(&v2).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        let route = p.route.expect("route synthesised");
        assert_eq!(route.pois.len(), 2);
        assert_eq!(route.pois[0].kind, crate::PoiKind::Terminal);
        assert_eq!(route.pois[1].kind, crate::PoiKind::Terminal);
        assert!((route.length_m() - 1000.0).abs() < 1e-9);
        // Borrowed the terrain SW corner for the synthetic georeference.
        assert!((route.pois[0].lat - (-23.0)).abs() < 1e-9);
        assert!((route.pois[1].ground_elevation_m - 120.0).abs() < 1e-9);
        // The stored profile is untouched.
        assert_eq!(p.ground_profile.len(), 2);
        // Each terminal pins an anchor.
        assert_eq!(
            route.pois[0].kind.pinned_function(),
            Some(crate::StructureFunction::Anchor)
        );
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

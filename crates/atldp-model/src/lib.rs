//! ATLDP project model — the staged-pipeline integration contract (ADR-0009) and
//! the open **ATLDP project file format** (phase G6).
//!
//! A single serializable [`Project`] that each pipeline stage reads, augments, and
//! writes back. It is the unit of serialization for the open ATLDP format
//! ([`format`]); the drafting and reporting stage (stage 6) consumes it through
//! [`analysis`] to produce calculation [`report`]s and plan-&-profile [`sheet`]s.
//!
//! Modules:
//! - [`format`]   — read/write the open `.atldp` file (versioned JSON)
//! - [`analysis`] — derive spans, sags, clearances, and structure loads
//! - [`report`]   — Markdown calculation report (stage 6)
//! - [`sheet`]    — SVG plan-&-profile drawing (stage 6)

use serde::{Deserialize, Serialize};

pub mod analysis;
pub mod format;
pub mod report;
pub mod sheet;

pub use format::FormatError;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current `.atldp` schema version. Bumped on any breaking change to [`Project`];
/// [`format::from_atldp_str`] refuses a newer one and migrates an older one.
pub const SCHEMA_VERSION: u32 = 1;

// ── project model ───────────────────────────────────────────────────────────────

/// A complete ATLDP project: everything needed to reproduce the views, the
/// engineering checks, and the drafting output, in one serializable document.
///
/// The terrain itself (the DEM tile) is *referenced*, not embedded — but the
/// extracted [`ground_profile`](Project::ground_profile) along the route **is**
/// stored so that clearance checks, reports, and sheets are reproducible without
/// re-reading the raster (which may be unavailable on another machine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// `.atldp` schema version (see [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// Human-facing project metadata.
    pub metadata: Metadata,
    /// The conductor strung along the line.
    pub conductor: ConductorSpec,
    /// Stringing / design parameters shared by every span.
    pub parameters: Parameters,
    /// Provenance of the terrain the route was spotted on (optional).
    pub terrain: Option<TerrainRef>,
    /// Ground elevation sampled along the route, ordered by distance.
    #[serde(default)]
    pub ground_profile: Vec<ProfileSample>,
    /// Spotted structures, ordered by distance along the route.
    #[serde(default)]
    pub towers: Vec<Tower>,
}

impl Project {
    /// An empty project carrying the built-in ACSR Drake conductor and the
    /// default stringing parameters — the starting point for a new line.
    pub fn new(name: impl Into<String>) -> Self {
        Project {
            schema_version: SCHEMA_VERSION,
            metadata: Metadata {
                name: name.into(),
                notes: String::new(),
            },
            conductor: ConductorSpec::drake(),
            parameters: Parameters::default(),
            terrain: None,
            ground_profile: Vec::new(),
            towers: Vec::new(),
        }
    }

    /// Ground elevation at a route distance by linear interpolation over
    /// [`ground_profile`](Project::ground_profile); `None` if there is no profile.
    pub fn ground_at(&self, distance_m: f64) -> Option<f64> {
        let p = &self.ground_profile;
        if p.is_empty() {
            return None;
        }
        if distance_m <= p[0].distance_m {
            return Some(p[0].elevation_m);
        }
        let last = p.last().unwrap();
        if distance_m >= last.distance_m {
            return Some(last.elevation_m);
        }
        let idx = p.partition_point(|s| s.distance_m < distance_m);
        let a = &p[idx - 1];
        let b = &p[idx];
        let t = (distance_m - a.distance_m) / (b.distance_m - a.distance_m);
        Some(a.elevation_m + t * (b.elevation_m - a.elevation_m))
    }
}

/// Human-facing project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Project / line name.
    pub name: String,
    /// Free-form notes carried with the project.
    #[serde(default)]
    pub notes: String,
}

/// The conductor's mechanical/geometric properties used by the drafting stage.
///
/// A self-contained snapshot (not a reference into `atldp-core`'s library) so the
/// file stays valid even as the built-in library evolves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorSpec {
    /// Conductor designation, e.g. `"ACSR Drake 26/7"`.
    pub name: String,
    /// Unit weight, newtons per metre.
    pub unit_weight_n_per_m: f64,
    /// Overall diameter, metres (projected area for wind load).
    pub diameter_m: f64,
    /// Rated tensile strength, newtons.
    pub rated_strength_n: f64,
}

impl ConductorSpec {
    /// The built-in ACSR Drake 26/7, mirroring [`atldp_core::conductor::drake_acsr`].
    pub fn drake() -> Self {
        let c = atldp_core::conductor::drake_acsr();
        ConductorSpec {
            name: c.name,
            unit_weight_n_per_m: c.unit_weight,
            diameter_m: c.diameter,
            rated_strength_n: c.rated_strength,
        }
    }
}

/// Stringing / design parameters shared across the line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    /// Horizontal tension the conductor is strung to, newtons.
    pub horizontal_tension_n: f64,
    /// Default conductor attachment height above ground for new towers, metres.
    pub attachment_height_m: f64,
    /// Minimum permissible conductor-to-ground clearance, metres.
    pub min_clearance_m: f64,
    /// Design transverse wind pressure on the conductor, pascals (stage 5).
    pub wind_pressure_pa: f64,
}

impl Default for Parameters {
    fn default() -> Self {
        Parameters {
            horizontal_tension_n: 30_000.0,
            attachment_height_m: 15.0,
            min_clearance_m: 8.0,
            wind_pressure_pa: 700.0,
        }
    }
}

/// Provenance of the terrain source — enough to re-locate and re-load the DEM,
/// but the elevations themselves live in [`Project::ground_profile`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainRef {
    /// Path to the DEM tile as known when the project was saved.
    pub source_path: String,
    /// South-west corner latitude of the tile, degrees (HGT convention).
    pub sw_lat: i32,
    /// South-west corner longitude of the tile, degrees.
    pub sw_lon: i32,
}

/// One sampled ground elevation along the route.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProfileSample {
    /// Cumulative distance from the route start, metres.
    pub distance_m: f64,
    /// Terrain elevation, metres (MSL).
    pub elevation_m: f64,
}

// ── spotting model (G5) ─────────────────────────────────────────────────────────

/// One structure placement along a transmission-line route (G5 manual spotting).
///
/// The tower is characterised by its position along the route profile
/// (`distance_m`), the terrain elevation at its base, and the height at which
/// the conductor attaches. Phase 5 (automatic spotting) will extend this with
/// structure type, cost, and load data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tower {
    /// Cumulative distance from the profile start, metres.
    pub distance_m: f64,
    /// Terrain elevation at the tower base, metres (MSL).
    pub ground_elevation_m: f64,
    /// Conductor attachment height above ground, metres.
    pub attachment_height_m: f64,
}

impl Tower {
    /// Absolute elevation of the conductor attachment point, metres (MSL).
    #[inline]
    pub fn attachment_elevation_m(&self) -> f64 {
        self.ground_elevation_m + self.attachment_height_m
    }
}

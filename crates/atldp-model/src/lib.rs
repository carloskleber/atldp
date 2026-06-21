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
///
/// - **v1** (G6): single [`ConductorSpec`] at one global tension, untyped towers.
/// - **v2** (G7/G8): a [`Wire`] set, structure-function typing + tension sections,
///   and a [`StructureFamily`] library with application charts.
pub const SCHEMA_VERSION: u32 = 2;

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
    /// The ordered set of wires strung along the line — phase conductors plus
    /// shield/ground wire(s) (G7). Each is an independent catenary.
    #[serde(default)]
    pub wires: Vec<Wire>,
    /// Stringing / design parameters shared by every span.
    pub parameters: Parameters,
    /// Structure-family library the spotted towers select from (G8).
    #[serde(default)]
    pub families: Vec<StructureFamily>,
    /// Per-(wire, section) stringing-tension overrides — the "different tractions
    /// per stretch" mechanism (G7). Empty ⇒ every section uses the wire's own
    /// [`tension_n`](Wire::tension_n).
    #[serde(default)]
    pub section_tensions: Vec<SectionTension>,
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
    /// An empty project carrying a single ACSR Drake phase wire and the default
    /// stringing parameters — the starting point for a new line. Use
    /// [`Project::with_three_phase`] for the full multi-wire set.
    pub fn new(name: impl Into<String>) -> Self {
        Project {
            schema_version: SCHEMA_VERSION,
            metadata: Metadata {
                name: name.into(),
                notes: String::new(),
            },
            wires: vec![Wire::phase("Phase", ConductorSpec::drake(), 0.0, 0.0)],
            parameters: Parameters::default(),
            families: Vec::new(),
            section_tensions: Vec::new(),
            terrain: None,
            ground_profile: Vec::new(),
            towers: Vec::new(),
        }
    }

    /// A project pre-loaded with a single circuit: three ACSR Drake phases plus
    /// one EHS shield wire, strung at the default tensions (G7).
    pub fn with_three_phase(name: impl Into<String>) -> Self {
        let mut p = Self::new(name);
        let h = p.parameters.horizontal_tension_n;
        p.wires = vec![
            Wire::phase("Phase A", ConductorSpec::drake(), 0.0, -4.0).strung(h),
            Wire::phase("Phase B", ConductorSpec::drake(), -2.5, 0.0).strung(h),
            Wire::phase("Phase C", ConductorSpec::drake(), -5.0, 4.0).strung(h),
            // Shield wire sits above the phases and is strung tighter.
            Wire::shield("Shield", ConductorSpec::ehs_shield(), 4.0, 0.0).strung(h * 1.2),
        ];
        p
    }

    /// The wire that governs ground clearance: the one whose attachment sits
    /// lowest (most negative vertical offset). `None` if there are no wires.
    pub fn lowest_wire(&self) -> Option<usize> {
        self.wires
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.vertical_offset_m
                    .partial_cmp(&b.vertical_offset_m)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Stringing tension for `wire` in tension `section`, applying any override in
    /// [`section_tensions`](Project::section_tensions) over the wire default.
    pub fn wire_tension(&self, wire: usize, section: usize) -> f64 {
        self.section_tensions
            .iter()
            .find(|s| s.wire == wire && s.section == section)
            .map(|s| s.tension_n)
            .or_else(|| self.wires.get(wire).map(|w| w.tension_n))
            .unwrap_or(self.parameters.horizontal_tension_n)
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

    /// A representative 3/8" EHS steel shield/ground wire — lighter and thinner
    /// than the phase conductor and strung tighter (G7 default wire set). Values
    /// are nominal manufacturer figures, not a redistributed standard table.
    pub fn ehs_shield() -> Self {
        ConductorSpec {
            name: "EHS 3/8\" shield".to_string(),
            unit_weight_n_per_m: 4.69,
            diameter_m: 0.00953,
            rated_strength_n: 68_900.0,
        }
    }
}

/// The mechanical role a wire plays on the structure (G7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WireRole {
    /// A phase conductor (carries current).
    #[default]
    Phase,
    /// A shield / overhead ground / OPGW wire.
    Shield,
}

/// One wire strung along the line — a phase conductor or a shield wire (G7).
///
/// Each wire is an independent catenary with its own conductor spec, attachment
/// geometry (offset from the structure reference attachment point), and stringing
/// tension. Clearance is checked wire-by-wire; the [`lowest`](Project::lowest_wire)
/// wire governs ground clearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire {
    /// Label, e.g. `"Phase A"` or `"Shield"`.
    pub name: String,
    /// Mechanical role.
    #[serde(default)]
    pub role: WireRole,
    /// Conductor strung as this wire.
    pub conductor: ConductorSpec,
    /// Vertical attachment offset from the tower reference attachment, metres
    /// (signed: phases hang below ⇒ negative, the shield sits above ⇒ positive).
    #[serde(default)]
    pub vertical_offset_m: f64,
    /// Lateral offset from the route centreline, metres (plan view only).
    #[serde(default)]
    pub lateral_offset_m: f64,
    /// Stringing horizontal tension, newtons (the section default before any
    /// per-section override in [`Project::section_tensions`]).
    pub tension_n: f64,
}

impl Wire {
    /// A phase wire at the given vertical/lateral offsets, strung at the default
    /// 30 kN until [`strung`](Wire::strung) overrides it.
    pub fn phase(
        name: impl Into<String>,
        conductor: ConductorSpec,
        vertical_offset_m: f64,
        lateral_offset_m: f64,
    ) -> Self {
        Wire {
            name: name.into(),
            role: WireRole::Phase,
            conductor,
            vertical_offset_m,
            lateral_offset_m,
            tension_n: 30_000.0,
        }
    }

    /// A shield wire at the given vertical/lateral offsets.
    pub fn shield(
        name: impl Into<String>,
        conductor: ConductorSpec,
        vertical_offset_m: f64,
        lateral_offset_m: f64,
    ) -> Self {
        Wire {
            role: WireRole::Shield,
            ..Wire::phase(name, conductor, vertical_offset_m, lateral_offset_m)
        }
    }

    /// Set the stringing tension (builder style).
    pub fn strung(mut self, tension_n: f64) -> Self {
        self.tension_n = tension_n;
        self
    }
}

/// A per-(wire, section) stringing-tension override (G7).
///
/// `wire` indexes [`Project::wires`]; `section` is the ordinal of the tension
/// section (see [`tension_sections`]). Lets the engineer string different
/// stretches of one wire to different tensions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SectionTension {
    /// Index into [`Project::wires`].
    pub wire: usize,
    /// Tension-section ordinal (see [`tension_sections`]).
    pub section: usize,
    /// Stringing horizontal tension for this stretch, newtons.
    pub tension_n: f64,
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

// ── structure typing & family library (G7/G8) ───────────────────────────────────

/// The mechanical function of a spotted structure (G7).
///
/// This typing is what partitions the line into **tension sections**: a section
/// runs from one [`Anchor`](StructureFunction::Anchor) to the next (see
/// [`tension_sections`]). The first and last structures are anchors implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StructureFunction {
    /// A tangent suspension structure (insulators swing freely, equalise tension).
    #[default]
    Suspension,
    /// An angle (running-angle suspension) structure.
    Angle,
    /// An anchor / strain / dead-end structure — terminates the tension section.
    Anchor,
}

impl StructureFunction {
    /// Whether this structure terminates a tension section.
    #[inline]
    pub fn is_anchor(self) -> bool {
        matches!(self, StructureFunction::Anchor)
    }

    /// Short label for tables and tooltips.
    pub fn label(self) -> &'static str {
        match self {
            StructureFunction::Suspension => "suspension",
            StructureFunction::Angle => "angle",
            StructureFunction::Anchor => "anchor",
        }
    }
}

/// The allowable wind-span × weight-span (× line-angle) envelope of a structure
/// family at a height — its **application / usage chart** (G8).
///
/// The chart is a piecewise envelope: for each [`ChartPoint`] (ordered by
/// ascending wind span) it gives the allowable weight-span band. Between points
/// the band is linearly interpolated; outside the wind-span range the placement
/// is rejected. A single deviation-angle cap covers the (separately rated) line
/// angle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationChart {
    /// Envelope vertices, ascending by `wind_span_m`.
    pub points: Vec<ChartPoint>,
    /// Maximum line (deviation) angle the family carries, degrees.
    pub max_line_angle_deg: f64,
}

/// One vertex of an [`ApplicationChart`]: the allowable weight-span band at a
/// wind span.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChartPoint {
    /// Wind (horizontal) span, metres.
    pub wind_span_m: f64,
    /// Smallest allowable weight span (uplift limit, may be negative), metres.
    pub weight_span_min_m: f64,
    /// Largest allowable weight span, metres.
    pub weight_span_max_m: f64,
}

impl ApplicationChart {
    /// A rectangular envelope: a constant weight-span band over `[0, max_wind]`.
    pub fn rectangular(
        max_wind_span_m: f64,
        weight_span_min_m: f64,
        weight_span_max_m: f64,
        max_line_angle_deg: f64,
    ) -> Self {
        ApplicationChart {
            points: vec![
                ChartPoint {
                    wind_span_m: 0.0,
                    weight_span_min_m,
                    weight_span_max_m,
                },
                ChartPoint {
                    wind_span_m: max_wind_span_m,
                    weight_span_min_m,
                    weight_span_max_m,
                },
            ],
            max_line_angle_deg,
        }
    }

    /// Whether a placement's spans and line angle fall inside the envelope.
    pub fn allows(&self, wind_span_m: f64, weight_span_m: f64, line_angle_deg: f64) -> bool {
        if line_angle_deg.abs() > self.max_line_angle_deg + 1e-9 {
            return false;
        }
        let Some((lo, hi)) = self.weight_span_band(wind_span_m) else {
            return false; // outside the wind-span range
        };
        weight_span_m >= lo - 1e-9 && weight_span_m <= hi + 1e-9
    }

    /// Interpolated `(min, max)` allowable weight span at `wind_span_m`, or `None`
    /// if the wind span is outside the charted range.
    pub fn weight_span_band(&self, wind_span_m: f64) -> Option<(f64, f64)> {
        let p = &self.points;
        if p.is_empty() {
            return None;
        }
        if wind_span_m < p[0].wind_span_m - 1e-9 {
            return None;
        }
        let last = p.last().unwrap();
        if wind_span_m > last.wind_span_m + 1e-9 {
            return None;
        }
        let idx = p.partition_point(|c| c.wind_span_m < wind_span_m);
        if idx == 0 {
            return Some((p[0].weight_span_min_m, p[0].weight_span_max_m));
        }
        let a = &p[idx - 1];
        let b = &p[idx.min(p.len() - 1)];
        if (b.wind_span_m - a.wind_span_m).abs() < 1e-9 {
            return Some((a.weight_span_min_m, a.weight_span_max_m));
        }
        let t = (wind_span_m - a.wind_span_m) / (b.wind_span_m - a.wind_span_m);
        Some((
            a.weight_span_min_m + t * (b.weight_span_min_m - a.weight_span_min_m),
            a.weight_span_max_m + t * (b.weight_span_max_m - a.weight_span_max_m),
        ))
    }
}

/// A standardised structure design available across a height range, rated by an
/// [`ApplicationChart`] (G8).
///
/// Spotting a tower means choosing a family + height whose chart envelopes the
/// wind/weight spans and line angle at that location; the optimizer (Phase 5)
/// makes the same choice by search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureFamily {
    /// Family name, e.g. `"Tangent suspension T1"`.
    pub name: String,
    /// Mechanical function the family is built for.
    pub function: StructureFunction,
    /// Minimum conductor-attachment (effective) height, metres.
    pub min_height_m: f64,
    /// Maximum conductor-attachment (effective) height, metres.
    pub max_height_m: f64,
    /// Default effective height when a tower first references the family, metres.
    pub default_height_m: f64,
    /// Allowable-loads envelope.
    pub chart: ApplicationChart,
}

impl StructureFamily {
    /// A small built-in library: a tangent, a light-angle, and a heavy
    /// angle/dead-end family. Chart values are illustrative (not a standard
    /// table — ADR-0004); a project can edit or replace them.
    pub fn built_in_library() -> Vec<StructureFamily> {
        vec![
            StructureFamily {
                name: "Tangent suspension".to_string(),
                function: StructureFunction::Suspension,
                min_height_m: 18.0,
                max_height_m: 36.0,
                default_height_m: 24.0,
                chart: ApplicationChart::rectangular(450.0, -150.0, 600.0, 2.0),
            },
            StructureFamily {
                name: "Light angle".to_string(),
                function: StructureFunction::Angle,
                min_height_m: 18.0,
                max_height_m: 33.0,
                default_height_m: 24.0,
                chart: ApplicationChart::rectangular(400.0, -100.0, 550.0, 20.0),
            },
            StructureFamily {
                name: "Heavy angle / dead-end".to_string(),
                function: StructureFunction::Anchor,
                min_height_m: 18.0,
                max_height_m: 30.0,
                default_height_m: 21.0,
                chart: ApplicationChart::rectangular(350.0, -50.0, 500.0, 90.0),
            },
        ]
    }
}

/// A spotted tower's reference into the [`StructureFamily`] library, with
/// optional per-structure overrides (G8) — "edit the structure after spotting".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerFamily {
    /// Index into [`Project::families`].
    pub family: usize,
    /// Chosen height within the family's range, metres.
    pub height_m: f64,
    /// Per-structure effective-height override (else `height_m` is used), metres.
    #[serde(default)]
    pub effective_height_override_m: Option<f64>,
    /// Per-structure chart override (else the family's chart is used).
    #[serde(default)]
    pub chart_override: Option<ApplicationChart>,
}

// ── spotting model (G5, extended in G7/G8) ───────────────────────────────────────

/// One structure placement along a transmission-line route (G5 manual spotting,
/// extended with G7 typing and the G8 family reference).
///
/// The tower is characterised by its position along the route profile
/// (`distance_m`), the terrain elevation at its base, and the height at which the
/// conductor reference point attaches. Its [`function`](Tower::function) types it
/// for tension-section partitioning, and an optional [`family`](Tower::family)
/// reference ties it to a rated structure design.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tower {
    /// Cumulative distance from the profile start, metres.
    pub distance_m: f64,
    /// Terrain elevation at the tower base, metres (MSL).
    pub ground_elevation_m: f64,
    /// Reference-conductor attachment height above ground, metres.
    pub attachment_height_m: f64,
    /// Mechanical function (G7) — drives tension-section partitioning.
    #[serde(default)]
    pub function: StructureFunction,
    /// Line (deviation) angle at this structure, degrees (0 ⇒ tangent). Checked
    /// against the family chart (G8).
    #[serde(default)]
    pub line_angle_deg: f64,
    /// Structure-family reference + overrides (G8); `None` ⇒ unassigned.
    #[serde(default)]
    pub family: Option<TowerFamily>,
}

impl Tower {
    /// Absolute elevation of the reference conductor attachment point, metres (MSL).
    #[inline]
    pub fn attachment_elevation_m(&self) -> f64 {
        self.ground_elevation_m + self.attachment_height_m
    }
}

/// Partition the ordered towers into **tension sections** at every anchor (G7).
///
/// Returns one `(from_tower, to_tower)` index pair per section, where both
/// endpoints are anchors (the first and last towers are anchors implicitly).
/// Fewer than two towers ⇒ no sections.
pub fn tension_sections(towers: &[Tower]) -> Vec<(usize, usize)> {
    if towers.len() < 2 {
        return Vec::new();
    }
    let mut anchors = vec![0usize];
    for (i, t) in towers.iter().enumerate().take(towers.len() - 1).skip(1) {
        if t.function.is_anchor() {
            anchors.push(i);
        }
    }
    anchors.push(towers.len() - 1);
    anchors.windows(2).map(|w| (w[0], w[1])).collect()
}

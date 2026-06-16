//! ATLDP project model — the staged-pipeline integration contract (ADR-0009).
//!
//! A single serializable model that each pipeline stage reads, augments, and
//! writes back, tracking which downstream results are stale. It is the unit of
//! serialization for the open ATLDP project format (defined in phase G6).

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── G5: spotting model ────────────────────────────────────────────────────────

/// One structure placement along a transmission-line route (G5 manual spotting).
///
/// The tower is characterised by its position along the route profile
/// (`distance_m`), the terrain elevation at its base, and the height at which
/// the conductor attaches. Phase 3 (automatic spotting) will extend this with
/// structure type, cost, and load data.
#[derive(Debug, Clone)]
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

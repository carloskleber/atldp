//! ATLDP geospatial layer — G3: DEM ingest, CRS, ground profile (ADR-0013).
//!
//! Local DEM as source of truth (ADR-0005), CRS/datum tracked explicitly.
//! Pure-Rust stack — no GDAL/PROJ dependency for the < 30 MB binary target.
//!
//! | Module | Purpose |
//! |--------|---------|
//! | `dem`  | HGT/SRTM parser, bilinear elevation query, `LocalGrid` |
//! | `crs`  | Equirectangular local tangent plane (full proj4rs in later pass) |
//! | `profile` | Ground profile extraction along a line segment |

pub mod crs;
pub mod dem;
pub mod profile;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

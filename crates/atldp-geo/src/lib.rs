//! ATLDP geospatial layer (ADR-0011, ADR-0013).
//!
//! Local DEM as source of truth (ADR-0005), CRS/datum tracked explicitly, LiDAR
//! point clouds later — all on a pure-Rust stack to protect the binary footprint.
//! Implemented in phases G3 (DEM, CRS, ground profile) and G4 (LAS/LAZ).
//!
//! Skeleton only in phase G0.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

//! ATLDP project model — the staged-pipeline integration contract (ADR-0009).
//!
//! A single serializable model that each pipeline stage reads, augments, and
//! writes back, tracking which downstream results are stale. It is the unit of
//! serialization for the open ATLDP project format (defined in phase G6).
//!
//! Skeleton only in phase G0.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

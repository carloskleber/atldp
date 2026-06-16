//! ATLDP engineering core — pure sag-tension numerics (ADR-0011).
//!
//! Headless, deterministic, no I/O and no GPU. This is the Rust reimplementation
//! of the Python `core/` engine (`core/src/atldp/core/`), and must reproduce its
//! results before that oracle is retired (ADR-0014).
//!
//! Modules land in phase G1, one per Python source module:
//! - `geometry`        — 3D span geometry (horizontal distance + rise, bearing)
//! - `catenary`        — inclined/level catenary + parabolic approximation
//! - `change_of_state` — equilibrium across temperature/load states
//! - `ruling_span`     — equivalent-span section model
//! - `conductor`       — linear-elastic + thermal constitutive model
//!
//! Until then this is an intentionally empty skeleton (phase G0).

/// Crate version, surfaced for the CLI/app "about" and validation provenance.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_exposed() {
        assert!(!VERSION.is_empty());
    }
}

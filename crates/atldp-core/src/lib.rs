//! ATLDP engineering core — pure sag-tension numerics (ADR-0011).
//!
//! Headless, deterministic, no I/O and no GPU. This is the Rust reimplementation
//! of the Python `core/` engine (`core/src/atldp/core/`), and reproduces its
//! results within explicit tolerances before that oracle is retired (ADR-0014).
//!
//! Modules, one per Python source module:
//! - [`geometry`]        — 3D span geometry (horizontal distance + rise, bearing)
//! - [`catenary`]        — inclined/level catenary + parabolic approximation
//! - [`change_of_state`] — equilibrium across temperature/load states
//! - [`ruling_span`]     — equivalent-span section model
//! - [`conductor`]       — linear-elastic + thermal constitutive model
//!
//! The crate has no external dependencies: it is the validation target for the
//! Python oracle, so its numerics are self-contained and auditable.

pub mod catenary;
pub mod change_of_state;
pub mod conductor;
pub mod geometry;
pub mod ruling_span;

/// Standard gravity (CODATA / ISO 80000), m/s^2.
pub const G: f64 = 9.80665;

/// Crate version, surfaced for the CLI/app "about" and validation provenance.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors raised by the sag-tension core. Mirrors the `ValueError`s and bracket
/// failures the Python oracle raises, as a typed, recoverable error instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    /// Horizontal distance `S` must be positive.
    NonPositiveSpan,
    /// Load `w` and horizontal tension `H` must be positive.
    NonPositiveLoadOrTension,
    /// The change-of-state root is not bracketed by `[lo, hi]`.
    RootNotBracketed,
    /// An unrecognised solver method name.
    UnknownMethod(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::NonPositiveSpan => write!(f, "horizontal distance S must be positive"),
            CoreError::NonPositiveLoadOrTension => write!(f, "w and H must be positive"),
            CoreError::RootNotBracketed => {
                write!(f, "change-of-state root not bracketed by [lo, hi]")
            }
            CoreError::UnknownMethod(m) => write!(f, "unknown method: {m:?}"),
        }
    }
}

impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_exposed() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn gravity_matches_oracle() {
        assert_eq!(G, 9.80665);
    }
}

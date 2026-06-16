//! Change-of-state equation.
//!
//! Given a conductor at a known reference state (horizontal tension `H1` at
//! temperature `T1` under load `w1`), find the horizontal tension `H2` at a new
//! state `(T2, w2)` by enforcing that the *unstrained* conductor length is
//! conserved between the two states.
//!
//! The unstrained length `L0` satisfies `L(state) = L0 * (1 + strain(state))`.
//! Eliminating `L0 = L1 / (1 + strain1)` between the two states gives the
//! equilibrium condition solved here:
//!
//! ```text
//! L_geom(H2; S, h, w2) = L1 * (1 + strain2) / (1 + strain1)
//! ```
//!
//! where `L1 = L_geom(H1; S, h, w1)` is the conductor arc length at the reference
//! state and `strain = sigma/E + alpha*(T - T_ref)` (see
//! [`crate::conductor::Conductor`]). The left side decreases with `H2` (tighter
//! conductor) while the right side increases (more elastic stretch), so the root
//! is unique. The Python oracle uses `scipy.optimize.brentq`; this port uses a
//! bisection on the same bracket — the residual is monotone, so both converge to
//! the same root well within tolerance.
//!
//! Because `L_geom` is the *inclined* catenary length, this handles uneven spans
//! directly — no level-span assumption.
//!
//! Mirror of the Python `atldp.core.change_of_state` oracle (ADR-0014).

use crate::catenary::{solve_span, CatenarySolution, Method};
use crate::conductor::Conductor;
use crate::CoreError;

/// A weather/loading state for one span (or ruling span).
#[derive(Clone, Debug, PartialEq)]
pub struct StateCase {
    /// Human-readable label.
    pub name: String,
    /// Temperature, degC.
    pub temperature: f64,
    /// Resultant load per unit length, N/m (>= conductor self-weight).
    pub w: f64,
    /// Additional permanent strain in this state.
    pub creep_strain: f64,
}

impl StateCase {
    /// A state with no creep strain.
    pub fn new(name: impl Into<String>, temperature: f64, w: f64) -> Self {
        Self {
            name: name.into(),
            temperature,
            w,
            creep_strain: 0.0,
        }
    }
}

/// Bisection root finder for a monotone-decreasing residual on `[lo, hi]`.
///
/// Matches the bracketed root that the oracle's `brentq` converges to; the
/// interval is shrunk to `1e-9` N (far tighter than the oracle's `xtol = 1e-6`).
fn bisect(
    mut lo: f64,
    mut hi: f64,
    residual: impl Fn(f64) -> Result<f64, CoreError>,
) -> Result<f64, CoreError> {
    let f_lo = residual(lo)?;
    let f_hi = residual(hi)?;
    if f_lo == 0.0 {
        return Ok(lo);
    }
    if f_hi == 0.0 {
        return Ok(hi);
    }
    if f_lo.signum() == f_hi.signum() {
        return Err(CoreError::RootNotBracketed);
    }
    // Residual decreases with H2: positive at lo, negative at hi.
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let f_mid = residual(mid)?;
        if f_mid > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if (hi - lo) <= 1e-9 || f_mid == 0.0 {
            break;
        }
    }
    Ok(0.5 * (lo + hi))
}

/// Solve for the span state at `target` given the `reference` state.
///
/// Returns the full [`CatenarySolution`] of the target state.
pub fn change_of_state(
    conductor: &Conductor,
    s: f64,
    h: f64,
    reference_h: f64,
    reference: &StateCase,
    target: &StateCase,
    method: Method,
) -> Result<CatenarySolution, CoreError> {
    let ref_solution = solve_span(s, h, reference.w, reference_h, method)?;
    let l1 = ref_solution.conductor_length;

    let strain1 = conductor.strain(reference_h, reference.temperature, reference.creep_strain);
    // Unstrained reference length (eliminated exactly, so the relation is
    // round-trip invertible).
    let l0 = l1 / (1.0 + strain1);

    let residual = |h2: f64| -> Result<f64, CoreError> {
        let strain2 = conductor.strain(h2, target.temperature, target.creep_strain);
        let target_length = l0 * (1.0 + strain2);
        let geom_length = solve_span(s, h, target.w, h2, method)?.conductor_length;
        Ok(geom_length - target_length)
    };

    // Bracket H2 between a very slack conductor and the rated strength. The lower
    // bound keeps the catenary constant c >= S/20 (sag finite, no cosh/sinh
    // overflow); the true root is always far tighter than that.
    let lo = (target.w * s / 20.0).max(1.0);
    let hi = conductor.rated_strength;
    let h2 = bisect(lo, hi, residual)?;

    solve_span(s, h, target.w, h2, method)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::drake_acsr;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
    }

    #[test]
    fn round_trip_recovers_reference_tension() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("cold", 0.0, w);
        let hot = StateCase::new("hot", 75.0, w);
        let forward =
            change_of_state(&drake, 400.0, 0.0, 31500.0, &reference, &hot, Method::Auto).unwrap();
        let back = change_of_state(
            &drake,
            400.0,
            0.0,
            forward.h_tension,
            &hot,
            &reference,
            Method::Auto,
        )
        .unwrap();
        assert!(close(back.h_tension, 31500.0, 1e-4));
    }

    #[test]
    fn higher_temperature_lowers_tension() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("ref", 15.0, w);
        let cold = change_of_state(
            &drake,
            400.0,
            0.0,
            31500.0,
            &reference,
            &StateCase::new("c", -5.0, w),
            Method::Auto,
        )
        .unwrap();
        let hot = change_of_state(
            &drake,
            400.0,
            0.0,
            31500.0,
            &reference,
            &StateCase::new("h", 80.0, w),
            Method::Auto,
        )
        .unwrap();
        assert!(cold.h_tension > 31500.0 && 31500.0 > hot.h_tension);
        assert!(cold.sag < hot.sag);
    }

    #[test]
    fn heavier_load_raises_tension_and_sag() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("ref", 15.0, w);
        let bare = change_of_state(
            &drake,
            400.0,
            0.0,
            31500.0,
            &reference,
            &StateCase::new("bare", 15.0, w),
            Method::Auto,
        )
        .unwrap();
        let iced = change_of_state(
            &drake,
            400.0,
            0.0,
            31500.0,
            &reference,
            &StateCase::new("iced", 15.0, 2.0 * w),
            Method::Auto,
        )
        .unwrap();
        assert!(iced.h_tension > bare.h_tension);
        assert!(iced.sag > bare.sag);
    }

    #[test]
    fn handles_inclined_span() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("ref", 15.0, w);
        let hot = StateCase::new("hot", 75.0, w);
        let sol = change_of_state(
            &drake,
            450.0,
            70.0,
            31500.0,
            &reference,
            &hot,
            Method::Catenary,
        )
        .unwrap();
        assert_eq!(sol.method, Method::Catenary);
        assert_eq!(sol.max_tension(), sol.tension_end);
        assert!(sol.h_tension < 31500.0);
    }
}

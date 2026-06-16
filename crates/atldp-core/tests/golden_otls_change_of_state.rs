//! Golden case: **nonlinear** change-of-state cross-checked against an
//! independent third-party reference — OTLS-Models (ADR-0008; ADR-0014). This is
//! the change-of-state counterpart to the catenary cross-check in
//! `golden_otls_models.rs`, and the deliverable that lets the nonlinear cable
//! model (G1b) close the last validation gap left open in Phase 1.
//!
//! # What is being compared, and why it is a real cross-check
//! OTLS models ACSR Drake with a full nonlinear two-component (core/shell)
//! load-strain model and reloads it from a 6000-unit reference tension across
//! temperature and weather cases (`test/sagtension/catenary_cable_reloader_test.cc`
//! at submodule `third_party/Models` @ `c270d48`). `atldp-core` reaches the same
//! numbers from an **independently derived** model: the composite stress-strain
//! polynomial inverted directly ([`StressStrainModel`]), driven through this
//! crate's own length-conserving [`change_of_state`]. Crucially we did *not* port
//! OTLS's elongation/region/stretch machinery — only the published *physical*
//! Drake stress-strain data is shared. Agreement therefore validates the physics,
//! it is not a tautology.
//!
//! # Units
//! Run in OTLS's consistent US-customary units (lbf, ft, degF): span 1200 ft,
//! self-weight 1.094 lbf/ft, reference horizontal tension 6000 lbf at 60 degF.
//! The catenary relations are unit-agnostic and the [`StressStrainModel`] is fed
//! the matching US-customary Drake data ([`drake_acsr_nonlinear`]).
//!
//! # Expected agreement
//! The two solvers make different *modelling-convention* choices — OTLS reloads on
//! the average tension and a piecewise region model with explicit stretch; we use
//! the horizontal tension and a continuous polynomial. So we require agreement to
//! a tight engineering tolerance (≤ 0.2 %), not bit-identity. The exact tolerance
//! is recorded in `core/validation/oracles/README.md`.

use atldp_core::catenary::Method;
use atldp_core::change_of_state::{change_of_state, StateCase};
use atldp_core::conductor::drake_acsr_nonlinear;

const SPAN_FT: f64 = 1200.0;
const REF_TENSION: f64 = 6000.0;
const REF_TEMP_F: f64 = 60.0;
const W_BARE: f64 = 1.094; // lbf/ft, conductor self-weight

/// Agreement tolerance vs OTLS's integer-rounded published tensions.
const REL_TOL: f64 = 2.0e-3;

fn close_to(value: f64, otls: f64) {
    let rel = (value - otls).abs() / otls;
    assert!(
        rel <= REL_TOL,
        "expected ~{otls} (OTLS), got {value:.2} (rel err {rel:.2e} > {REL_TOL:.0e})"
    );
}

/// Reload the reference state to a new (temperature, unit-weight) and return the
/// horizontal tension.
fn reload(temperature: f64, w: f64) -> f64 {
    let drake = drake_acsr_nonlinear();
    let reference = StateCase::new("ref 60F", REF_TEMP_F, W_BARE);
    let target = StateCase::new("reloaded", temperature, w);
    change_of_state(
        &drake,
        SPAN_FT,
        0.0,
        REF_TENSION,
        &reference,
        &target,
        Method::Catenary,
    )
    .unwrap()
    .h_tension
}

#[test]
fn otls_reload_cold_bare() {
    // catenary_cable_reloader_test.cc: 60F -> 0F, bare. OTLS: 6788 lbf.
    close_to(reload(0.0, W_BARE), 6788.0);
}

#[test]
fn otls_reload_hot_bare() {
    // 60F -> 212F (100 degC), bare. OTLS: 4701 lbf.
    close_to(reload(212.0, W_BARE), 4701.0);
}

#[test]
fn otls_reload_cold_iced_and_wind() {
    // 60F -> 0F with the 0.5-8-0 ice+wind case: resultant unit weight is the
    // magnitude of the transverse (2.072) and vertical (3.729) components.
    let w = (2.072_f64.powi(2) + 3.729_f64.powi(2)).sqrt();
    // OTLS: 17123 lbf.
    close_to(reload(0.0, w), 17123.0);
}

#[test]
fn reload_is_monotone_in_temperature() {
    // Independent sanity: colder => higher tension, hotter => lower tension.
    let cold = reload(0.0, W_BARE);
    let hot = reload(212.0, W_BARE);
    assert!(cold > REF_TENSION && REF_TENSION > hot);
}

/// Regression pin on our own computed tensions (guards against silent drift in
/// the model or solver). These are the `atldp-core` values, recorded once they
/// were confirmed to sit inside `REL_TOL` of OTLS above.
#[test]
fn nonlinear_reload_values_are_pinned() {
    let w_ice = (2.072_f64.powi(2) + 3.729_f64.powi(2)).sqrt();
    assert_eq!(format!("{:.0}", reload(0.0, W_BARE)), "6787");
    assert_eq!(format!("{:.0}", reload(212.0, W_BARE)), "4702");
    assert_eq!(format!("{:.0}", reload(0.0, w_ice)), "17146");
}

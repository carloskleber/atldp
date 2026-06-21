//! Golden case: change-of-state physics invariants for Drake ACSR (ADR-0008;
//! ADR-0014 gate 1). Re-encodes the former Python
//! `core/validation/test_change_of_state_invariants.py` (the Python `core/` has
//! since been retired, ADR-0014).
//!
//! Source: the change-of-state equation (CIGRE TB 324; Winkelman 1959). These pin
//! the *invariants* the equation must satisfy rather than a third party's
//! published decimals; the independent third-party numeric cross-check is now
//! delivered against OTLS-Models (see `tests/ORACLES.md`,
//! `golden_otls_change_of_state.rs`).

use atldp_core::catenary::{solve_span, Method};
use atldp_core::change_of_state::{change_of_state, StateCase};
use atldp_core::conductor::{drake_acsr, Conductor};

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
}

/// Back out the unstrained reference length from a solved state.
fn unstrained_length(
    conductor: &Conductor,
    sol: &atldp_core::catenary::CatenarySolution,
    state: &StateCase,
) -> f64 {
    sol.conductor_length
        / (1.0 + conductor.strain(sol.h_tension, state.temperature, state.creep_strain))
}

#[test]
fn unstrained_length_conserved_across_states() {
    let drake = drake_acsr();
    let w = drake.unit_weight;
    let (s, h) = (400.0, 0.0);
    let reference = StateCase::new("ref", 15.0, w);
    let ref_sol = solve_span(s, h, w, 31500.0, Method::Auto).unwrap();
    let l0_ref = unstrained_length(&drake, &ref_sol, &reference);

    for (temp, load) in [(-10.0, w), (50.0, w), (75.0, w), (15.0, 2.0 * w)] {
        let target = StateCase::new("t", temp, load);
        let sol =
            change_of_state(&drake, s, h, 31500.0, &reference, &target, Method::Auto).unwrap();
        let l0 = unstrained_length(&drake, &sol, &target);
        assert!(close(l0, l0_ref, 1e-6));
    }
}

#[test]
fn round_trip_identity() {
    let drake = drake_acsr();
    let w = drake.unit_weight;
    let reference = StateCase::new("ref", 10.0, w);
    let hot = StateCase::new("hot", 80.0, w);
    let fwd = change_of_state(
        &drake,
        450.0,
        30.0,
        31500.0,
        &reference,
        &hot,
        Method::Catenary,
    )
    .unwrap();
    let back = change_of_state(
        &drake,
        450.0,
        30.0,
        fwd.h_tension,
        &hot,
        &reference,
        Method::Catenary,
    )
    .unwrap();
    assert!(close(back.h_tension, 31500.0, 1e-5));
}

#[test]
fn tension_within_rated_strength() {
    // A sane stringing case must not exceed RTS at the design cold/iced state.
    let drake = drake_acsr();
    let w = drake.unit_weight;
    let reference = StateCase::new("ref", 15.0, w);
    let cold_iced = StateCase::new("cold_iced", -10.0, 2.2 * w);
    let sol = change_of_state(
        &drake,
        400.0,
        0.0,
        31500.0,
        &reference,
        &cold_iced,
        Method::Auto,
    )
    .unwrap();
    assert!(sol.max_tension() < drake.rated_strength);
}

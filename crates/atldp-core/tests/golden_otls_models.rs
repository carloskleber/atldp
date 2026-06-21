//! Golden case: exact catenary against an **independent third-party numeric
//! reference** (ADR-0008; ADR-0014 gate 3).
//!
//! Source: OTLS-Models (Overhead Transmission Line Software, "Models"), vendored
//! as the git submodule `third_party/Models` pinned at commit `c270d48`
//! (tag 4.0.0-5), file `test/transmissionline/catenary_test.cc`. The values below
//! are that suite's own `EXPECT_EQ` expectations, reproduced verbatim. Provenance
//! and the full table live in `tests/ORACLES.md`.
//!
//! The catenary relations depend only on the dimensionless group `S/(2c)` with
//! `c = H/w`, so they are independent of the conductor constitutive model *and*
//! of the unit system: OTLS's consistent (ft, lbf) numbers are used directly as
//! SI (m, N). This is what makes it a genuine cross-implementation oracle rather
//! than a tautology. Closing this gate retires the Python `core/` (ADR-0014).
//!
//! Reference object: H = 1000, w = 0.5  ->  c = H/w = 2000.

use atldp_core::catenary::solve_catenary;

/// OTLS rounds its published expectations to 2 decimals; agreement well inside
/// that proves the cross-implementation match without over-claiming precision.
const REL_TOL: f64 = 1e-4;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= REL_TOL * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
}

#[test]
fn otls_catenary_level_span() {
    // catenary_test.cc Catenary2dTest: spacing (1000, 0), H = 1000, w = 0.5.
    let sol = solve_catenary(1000.0, 0.0, 0.5, 1000.0).unwrap();

    assert!(close(sol.catenary_constant(), 2000.00)); // Constant
    assert!(close(sol.conductor_length, 1010.45)); // Length
    assert!(close(sol.sag, 62.83)); // Sag (max)
    assert!(close(sol.max_tension(), 1031.41)); // TensionMax / Tension(0)
    assert!(close(sol.h_tension, 1000.00)); // Tension(0.5), low point

    // Pinned decimals matching OTLS's rounding (regression guard).
    assert_eq!(format!("{:.2}", sol.conductor_length), "1010.45");
    assert_eq!(format!("{:.2}", sol.sag), "62.83");
    assert_eq!(format!("{:.2}", sol.max_tension()), "1031.41");
}

#[test]
fn otls_catenary_inclined_span() {
    // catenary_test.cc Length / TensionMax: spacing (1000, 500), H = 1000, w = 0.5.
    let sol = solve_catenary(1000.0, 500.0, 0.5, 1000.0).unwrap();

    assert!(close(sol.conductor_length, 1127.39)); // Length (inclined)
    assert!(close(sol.max_tension(), 1275.78)); // TensionMax (inclined)

    assert_eq!(format!("{:.2}", sol.conductor_length), "1127.39");
    assert_eq!(format!("{:.2}", sol.max_tension()), "1275.78");
}

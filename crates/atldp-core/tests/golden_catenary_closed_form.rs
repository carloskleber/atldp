//! Golden case: exact catenary against closed-form identities (ADR-0008,
//! ADR-0014 gate 1). Re-encodes the former Python
//! `core/validation/test_catenary_closed_form.py` (the Python `core/` is retired,
//! ADR-0014).
//!
//! Source: the catenary closed-form relations (Irvine, *Cable Structures*, MIT
//! Press, 1981; sketched in `docs/theory.md`). These are independent of the
//! solver's implementation, so they are a genuine oracle.
//!
//! Level span: S = 300 m, w = 10 N/m, H = 15000 N -> c = H/w = 1500 m.
//!     sag    = c (cosh(S/2c) - 1)        = 7.506260... m
//!     length = 2 c sinh(S/2c)            = 300.500418... m
//!     T_max  = H cosh(S/2c) = H + w*sag  = 15075.062... N

use atldp_core::catenary::solve_catenary;

const TOL: f64 = 1e-9;

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
}

#[test]
fn level_span_closed_form() {
    let (s, w, h) = (300.0, 10.0, 15000.0);
    let c = h / w;
    let sol = solve_catenary(s, 0.0, w, h).unwrap();

    let expected_sag = c * ((s / (2.0 * c)).cosh() - 1.0);
    let expected_len = 2.0 * c * (s / (2.0 * c)).sinh();
    let expected_tmax = h * (s / (2.0 * c)).cosh();

    assert!(close(sol.sag, expected_sag, TOL));
    assert!(close(sol.conductor_length, expected_len, TOL));
    assert!(close(sol.max_tension(), expected_tmax, TOL));

    // Pinned decimals (regression guard), matching the Python golden.
    assert_eq!(format!("{:.4}", sol.sag), "7.5063");
    assert_eq!(format!("{:.4}", sol.conductor_length), "300.5003");
    assert_eq!(format!("{:.2}", sol.max_tension()), "15075.06");
}

#[test]
fn inclined_span_length_identity() {
    // Inclined catenary length identity: L = sqrt(h^2 + (2c sinh(S/2c))^2).
    let (s, h, w, h_tension) = (300.0, 40.0, 10.0, 15000.0);
    let c = h_tension / w;
    let sol = solve_catenary(s, h, w, h_tension).unwrap();
    let expected_len = (h * h + (2.0 * c * (s / (2.0 * c)).sinh()).powi(2)).sqrt();
    assert!(close(sol.conductor_length, expected_len, TOL));
    assert_eq!(format!("{:.4}", sol.conductor_length), "303.1508");
}

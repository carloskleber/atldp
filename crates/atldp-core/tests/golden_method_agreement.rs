//! Golden case: the parabola must agree with the exact catenary in the shallow
//! regime where it is admissible (ADR-0003; ADR-0014 gate 1). Re-encodes the
//! former Python `core/validation/test_method_agreement.py` (the Python `core/`
//! is retired, ADR-0014).
//!
//! For a level span at sag/span well below 1/8, the truncated parabola and the
//! exact catenary agree to better than 0.2% on sag and max tension, and to ~1e-5
//! on length. This both validates the parabola and exercises the regime switch.

use atldp_core::catenary::{solve_catenary, solve_parabola};

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
}

#[test]
fn shallow_level_span_agreement() {
    let (s, w) = (300.0, 15.97);
    for h in [25000.0, 35000.0, 50000.0, 70000.0] {
        let cat = solve_catenary(s, 0.0, w, h).unwrap();
        let par = solve_parabola(s, 0.0, w, h).unwrap();

        assert!((cat.sag / s) < 1.0 / 8.0); // regime precondition
        assert!(close(cat.sag, par.sag, 2e-3));
        assert!(close(cat.conductor_length, par.conductor_length, 1e-5));
        assert!(close(cat.max_tension(), par.max_tension(), 2e-3));
    }
}

#[test]
fn disagreement_grows_in_deep_regime() {
    // Slack, deep span: the parabola is no longer admissible; the methods part.
    let (s, w, h) = (600.0, 30.0, 12000.0);
    let cat = solve_catenary(s, 0.0, w, h).unwrap();
    let par = solve_parabola(s, 0.0, w, h).unwrap();
    assert!((cat.sag / s) > 1.0 / 8.0);
    assert!(!close(cat.sag, par.sag, 2e-3));
}

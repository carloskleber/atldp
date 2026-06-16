//! Single-span sag-tension: exact catenary and parabolic approximation.
//!
//! Both supports may be at different elevations (inclined / uneven span), which
//! is the normal case on real terrain. The span is described by its horizontal
//! distance `S` and elevation difference `h` (see [`crate::geometry`]), and
//! loaded by a resultant load per unit length `w` (N/m). For Phase 1 `w` is the
//! conductor self-weight; Phase 2 will pass the wind+weight resultant.
//!
//! # Conventions
//! * Horizontal axis `x` runs from the lower-numbered support (`x=0`) to the
//!   other (`x=S`); `y` is vertical (up). Support 1 is at `(0, 0)`, support 2 at
//!   `(S, h)`.
//! * The horizontal component of tension `H` is constant along the span. The
//!   total tension at a point is `T(x) = sqrt(H^2 + V(x)^2)`; it is largest at
//!   the higher support.
//! * "Sag" is the maximum *vertical* distance from the straight chord between the
//!   supports down to the conductor.
//!
//! Regime selection follows `docs/theory.md`: the exact catenary is required when
//! the sag-to-span ratio exceeds 1/8 or the supports are inclined; the parabola
//! is admissible only for shallow, near-level spans. [`solve_span`] applies this
//! rule when `method == Method::Auto`.
//!
//! Mirror of the Python `atldp.core.catenary` oracle (ADR-0014). The parabolic
//! arc length, integrated numerically by `scipy.quad` in the oracle, is evaluated
//! here in closed form (the integrand `sqrt(1 + y'^2)` has a linear `y'`), which
//! agrees with the quadrature to ~1e-9.

use crate::CoreError;

/// Above this sag/span ratio the parabola is no longer trustworthy and the exact
/// catenary must be used (`docs/theory.md`).
pub const SAG_SPAN_RATIO_LIMIT: f64 = 1.0 / 8.0;

/// Above this support inclination the parabola is no longer trustworthy and the
/// exact catenary must be used (`docs/theory.md`).
pub const INCLINATION_LIMIT_RAD: f64 = 0.261_799_387_799_149_44; // 15 degrees

/// Which solver to use for a span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Pick catenary or parabola by the regime rule (`docs/theory.md`).
    Auto,
    /// Force the exact inclined catenary.
    Catenary,
    /// Force the parabolic approximation.
    Parabola,
}

impl Method {
    /// Parse a CLI-style method name (`"auto"`, `"catenary"`, `"parabola"`).
    pub fn parse(name: &str) -> Result<Method, CoreError> {
        match name {
            "auto" => Ok(Method::Auto),
            "catenary" => Ok(Method::Catenary),
            "parabola" => Ok(Method::Parabola),
            other => Err(CoreError::UnknownMethod(other.to_string())),
        }
    }

    /// Lower-case name, matching the Python `CatenarySolution.method` strings.
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Auto => "auto",
            Method::Catenary => "catenary",
            Method::Parabola => "parabola",
        }
    }
}

/// Result of solving one span. Lengths in metres, tensions in newtons, `w` in
/// N/m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatenarySolution {
    /// Solver that produced this result ([`Method::Catenary`] or
    /// [`Method::Parabola`], never [`Method::Auto`]).
    pub method: Method,
    /// Horizontal distance `S`, m.
    pub horizontal_distance: f64,
    /// Elevation difference `h`, m.
    pub elevation_difference: f64,
    /// Resultant load per unit length, N/m.
    pub w: f64,
    /// Horizontal tension, N.
    pub h_tension: f64,
    /// Arc length of conductor between supports, m.
    pub conductor_length: f64,
    /// Max vertical sag below the chord, m.
    pub sag: f64,
    /// `x` of the max-sag point, m.
    pub sag_position: f64,
    /// `x` of the lowest point (may be outside `[0, S]`), m.
    pub low_point_x: f64,
    /// Total tension at support 1, N.
    pub tension_start: f64,
    /// Total tension at support 2, N.
    pub tension_end: f64,
}

impl CatenarySolution {
    /// Largest support tension, N.
    pub fn max_tension(&self) -> f64 {
        self.tension_start.max(self.tension_end)
    }

    /// `c = H / w` (metres) — only meaningful for the exact catenary solution.
    pub fn catenary_constant(&self) -> f64 {
        self.h_tension / self.w
    }
}

/// Return the abscissa `a` of the catenary low point.
///
/// The conductor curve is `y(x) = c*cosh((x-a)/c) - c*cosh(a/c)` (so that
/// `y(0)=0`); the second support condition `y(S)=h` inverts in closed form to
/// `a = S/2 - c * asinh(h / (2 c sinh(S/2c)))`. `a` is `S/2` for a level span and
/// moves outside `[0, S]` for steep inclines.
fn solve_low_point(s: f64, h: f64, c: f64) -> f64 {
    s / 2.0 - c * (h / (2.0 * c * (s / (2.0 * c)).sinh())).asinh()
}

fn validate(s: f64, w: f64, h_tension: f64) -> Result<(), CoreError> {
    if s <= 0.0 {
        return Err(CoreError::NonPositiveSpan);
    }
    if w <= 0.0 || h_tension <= 0.0 {
        return Err(CoreError::NonPositiveLoadOrTension);
    }
    Ok(())
}

/// Exact inclined catenary for given horizontal tension `H`.
pub fn solve_catenary(
    s: f64,
    h: f64,
    w: f64,
    h_tension: f64,
) -> Result<CatenarySolution, CoreError> {
    validate(s, w, h_tension)?;

    let c = h_tension / w;
    let a = solve_low_point(s, h, c);
    let b = -c * (a / c).cosh(); // vertical offset so that y(0) = 0
    let y = |x: f64| c * ((x - a) / c).cosh() + b;

    // Closed-form arc length (independent of a): a useful internal cross-check.
    let length = (h * h + (2.0 * c * (s / (2.0 * c)).sinh()).powi(2)).sqrt();

    // Tension: T(x) = H*cosh((x-a)/c). Largest at the support farther from the
    // low point.
    let t_start = h_tension * (a / c).cosh();
    let t_end = h_tension * ((s - a) / c).cosh();

    // The max-sag abscissa is where the conductor slope equals the chord slope:
    // y'(x) = sinh((x-a)/c) = h/S.
    let mut xs = a + c * (h / s).asinh();
    xs = xs.clamp(0.0, s);
    let sag = (h / s) * xs - y(xs);

    Ok(CatenarySolution {
        method: Method::Catenary,
        horizontal_distance: s,
        elevation_difference: h,
        w,
        h_tension,
        conductor_length: length,
        sag,
        sag_position: xs,
        low_point_x: a,
        tension_start: t_start,
        tension_end: t_end,
    })
}

/// Antiderivative of `sqrt(1 + u^2)` w.r.t. `u`, used for the parabola arc length.
fn arc_primitive(u: f64) -> f64 {
    (u * (1.0 + u * u).sqrt() + u.asinh()) / 2.0
}

/// Parabolic approximation for given horizontal tension `H`.
///
/// The conductor is taken as `y(x) = (h/S)x - (w/2H)x(S-x)` (chord minus a
/// symmetric downward parabola). Valid only for shallow, near-level spans. The
/// arc length is the exact integral of `sqrt(1 + y'^2)`; since `y'` is linear,
/// substituting `u = y'(x)` (so `dx = (H/w) du`) gives the closed form below.
pub fn solve_parabola(
    s: f64,
    h: f64,
    w: f64,
    h_tension: f64,
) -> Result<CatenarySolution, CoreError> {
    validate(s, w, h_tension)?;

    let yprime = |x: f64| h / s - (w / (2.0 * h_tension)) * (s - 2.0 * x);

    // length = integral_0^S sqrt(1 + y'(x)^2) dx, with u = y'(x), dx = (H/w) du.
    let u0 = yprime(0.0);
    let us = yprime(s);
    let length = (h_tension / w) * (arc_primitive(us) - arc_primitive(u0));

    // Vertical sag below the chord is (w/2H) x (S-x), max at mid-span.
    let sag = w * s * s / (8.0 * h_tension);
    let xs = s / 2.0;

    let t_start = h_tension.hypot(h_tension * yprime(0.0));
    let t_end = h_tension.hypot(h_tension * yprime(s));

    // Abscissa of the lowest point: where y'(x) = 0.
    let low_x = s / 2.0 - (h_tension * h) / (w * s);

    Ok(CatenarySolution {
        method: Method::Parabola,
        horizontal_distance: s,
        elevation_difference: h,
        w,
        h_tension,
        conductor_length: length,
        sag,
        sag_position: xs,
        low_point_x: low_x,
        tension_start: t_start,
        tension_end: t_end,
    })
}

/// Solve one span for a given horizontal tension `H`.
///
/// [`Method::Auto`] uses the exact catenary whenever the supports are inclined
/// beyond [`INCLINATION_LIMIT_RAD`] or the sag/span ratio exceeds
/// [`SAG_SPAN_RATIO_LIMIT`], and the parabola otherwise (`docs/theory.md`).
pub fn solve_span(
    s: f64,
    h: f64,
    w: f64,
    h_tension: f64,
    method: Method,
) -> Result<CatenarySolution, CoreError> {
    match method {
        Method::Catenary => solve_catenary(s, h, w, h_tension),
        Method::Parabola => solve_parabola(s, h, w, h_tension),
        Method::Auto => {
            let inclined = h.atan2(s).abs() > INCLINATION_LIMIT_RAD;
            // Cheap parabolic sag estimate to test the ratio before committing.
            let approx_sag = w * s * s / (8.0 * h_tension);
            let deep = (approx_sag / s) > SAG_SPAN_RATIO_LIMIT;
            if inclined || deep {
                solve_catenary(s, h, w, h_tension)
            } else {
                solve_parabola(s, h, w, h_tension)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
    }

    /// Arc length of the *solved* catenary curve, integrated independently by
    /// composite Simpson — the analogue of the oracle's `scipy.quad` cross-check.
    fn numeric_arc_length(sol: &CatenarySolution) -> f64 {
        let c = sol.catenary_constant();
        let a = sol.low_point_x;
        let f = |x: f64| ((x - a) / c).cosh();
        let n = 200_000usize;
        let s = sol.horizontal_distance;
        let dx = s / n as f64;
        let mut acc = f(0.0) + f(s);
        for i in 1..n {
            let x = i as f64 * dx;
            acc += if i % 2 == 1 { 4.0 } else { 2.0 } * f(x);
        }
        acc * dx / 3.0
    }

    #[test]
    fn level_catenary_matches_closed_form() {
        let (s, w, h_tension) = (400.0, 15.97, 30000.0);
        let sol = solve_catenary(s, 0.0, w, h_tension).unwrap();
        let c = h_tension / w;
        assert!(close(sol.sag, c * ((s / (2.0 * c)).cosh() - 1.0), 1e-10));
        assert!(close(
            sol.conductor_length,
            2.0 * c * (s / (2.0 * c)).sinh(),
            1e-10
        ));
        assert!(close(sol.low_point_x, s / 2.0, 1e-9));
        assert!(close(sol.sag_position, s / 2.0, 1e-9));
    }

    #[test]
    fn max_tension_identity_level_span() {
        let sol = solve_catenary(400.0, 0.0, 15.97, 30000.0).unwrap();
        assert!(close(
            sol.max_tension(),
            sol.h_tension + sol.w * sol.sag,
            1e-10
        ));
        assert!(close(sol.tension_start, sol.tension_end, 1e-10));
    }

    #[test]
    fn closed_form_length_equals_numeric_integral() {
        for h in [0.0, 40.0, -60.0] {
            let sol = solve_catenary(300.0, h, 12.0, 18000.0).unwrap();
            assert!(close(sol.conductor_length, numeric_arc_length(&sol), 1e-9));
        }
    }

    #[test]
    fn inclined_max_tension_at_higher_support() {
        let sol = solve_catenary(350.0, 60.0, 15.97, 28000.0).unwrap();
        assert!(sol.tension_end > sol.tension_start);
        assert_eq!(sol.max_tension(), sol.tension_end);
    }

    #[test]
    fn auto_selects_parabola_for_shallow_level_span() {
        let sol = solve_span(300.0, 0.0, 15.97, 40000.0, Method::Auto).unwrap();
        assert_eq!(sol.method, Method::Parabola);
    }

    #[test]
    fn auto_selects_catenary_for_inclined_span() {
        let sol = solve_span(300.0, 120.0, 15.97, 40000.0, Method::Auto).unwrap();
        assert_eq!(sol.method, Method::Catenary);
    }

    #[test]
    fn parabola_agrees_with_catenary_in_shallow_regime() {
        let (s, w) = (300.0, 15.97);
        for h_tension in [25000.0, 35000.0, 50000.0] {
            let cat = solve_catenary(s, 0.0, w, h_tension).unwrap();
            let par = solve_parabola(s, 0.0, w, h_tension).unwrap();
            assert!(close(cat.sag, par.sag, 2e-3));
            assert!(close(cat.conductor_length, par.conductor_length, 1e-5));
            assert!(close(cat.max_tension(), par.max_tension(), 2e-3));
        }
    }

    #[test]
    fn rejects_invalid_inputs() {
        assert_eq!(
            solve_catenary(0.0, 0.0, 1.0, 1.0),
            Err(CoreError::NonPositiveSpan)
        );
        assert_eq!(
            solve_catenary(100.0, 0.0, 0.0, 1.0),
            Err(CoreError::NonPositiveLoadOrTension)
        );
    }
}

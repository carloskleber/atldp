//! Structure loading — wind span, weight span, and the conductor loads they
//! impose on a support (pipeline stage 5, phase G6).
//!
//! A transmission structure is loaded by the two spans adjacent to it. Two
//! span lengths drive the design:
//!
//! - **Wind span** (a.k.a. *horizontal span*) — the length of conductor whose
//!   transverse wind load is carried by the structure. It is the half-sum of the
//!   two adjacent horizontal spans (just the one half at a line terminal).
//! - **Weight span** (a.k.a. *vertical span*) — the horizontal distance between
//!   the catenary **low points** of the two adjacent spans. It is the length of
//!   conductor whose weight bears on the structure; it can exceed the wind span
//!   in a valley and go *negative* (uplift) on a summit, which this model carries
//!   through as a signed quantity rather than clamping.
//!
//! From these and the conductor's unit weight and diameter, the per-phase
//! transverse (wind) and vertical (weight) loads follow directly. This is the
//! "simple lattice-model representation" load input of ADR-0010 / the plan's
//! stage 5 — the structural check itself (member forces, guying) is a later
//! refinement; here we deliver the spans and the resulting conductor loads.

/// Geometry of one span as seen from a support, in span-local coordinates.
///
/// `low_point_x_m` is the abscissa of the catenary vertex measured from the
/// **back** (start) support of the span — i.e. [`crate::catenary::CatenarySolution::low_point_x`].
/// It lies in `[0, horizontal_m]` for a level-ish span and outside it for steep
/// inclines, which is exactly what makes the weight span signed.
#[derive(Clone, Copy, Debug)]
pub struct SpanGeom {
    /// Horizontal span length, metres.
    pub horizontal_m: f64,
    /// Catenary low-point abscissa from the back support, metres.
    pub low_point_x_m: f64,
}

impl SpanGeom {
    /// Build from a solved catenary span.
    pub fn from_solution(sol: &crate::catenary::CatenarySolution) -> Self {
        SpanGeom {
            horizontal_m: sol.horizontal_distance,
            low_point_x_m: sol.low_point_x,
        }
    }
}

/// Wind/weight spans and the conductor loads they impose on one support.
#[derive(Clone, Copy, Debug)]
pub struct StructureLoads {
    /// Wind (horizontal) span, metres.
    pub wind_span_m: f64,
    /// Weight (vertical) span, metres; signed (negative ⇒ uplift).
    pub weight_span_m: f64,
    /// Transverse load from wind on the conductor, newtons.
    pub transverse_load_n: f64,
    /// Vertical load from conductor weight, newtons; signed with the weight span.
    pub vertical_load_n: f64,
}

/// Compute the wind/weight spans and conductor loads at one support.
///
/// `back` is the span behind the support (the support is its *end*); `ahead` is
/// the span in front (the support is its *start*). A line terminal passes `None`
/// for the missing side. `unit_weight` (N/m) and `diameter` (m) are the
/// conductor's; `wind_pressure` (Pa) is the design transverse wind pressure on
/// the projected conductor area.
///
/// Wind span = ½·back + ½·ahead. Weight span = the distance from the back span's
/// low point up to the support, plus the distance from the support out to the
/// ahead span's low point: `(back.horizontal − back.low_point_x) + ahead.low_point_x`.
pub fn structure_loads(
    back: Option<SpanGeom>,
    ahead: Option<SpanGeom>,
    unit_weight: f64,
    diameter: f64,
    wind_pressure: f64,
) -> StructureLoads {
    let mut wind_span = 0.0;
    let mut weight_span = 0.0;
    if let Some(b) = back {
        wind_span += 0.5 * b.horizontal_m;
        weight_span += b.horizontal_m - b.low_point_x_m;
    }
    if let Some(a) = ahead {
        wind_span += 0.5 * a.horizontal_m;
        weight_span += a.low_point_x_m;
    }
    StructureLoads {
        wind_span_m: wind_span,
        weight_span_m: weight_span,
        transverse_load_n: wind_pressure * diameter * wind_span,
        vertical_load_n: unit_weight * weight_span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catenary::solve_catenary;

    /// A level span's vertex sits at mid-span, so an interior support between two
    /// equal level spans sees wind span = weight span = the common span length.
    #[test]
    fn equal_level_spans_give_matching_spans() {
        let s = solve_catenary(400.0, 0.0, 15.97, 30_000.0).unwrap();
        let g = SpanGeom::from_solution(&s);
        let loads = structure_loads(Some(g), Some(g), 15.97, 0.0281, 700.0);
        assert!((loads.wind_span_m - 400.0).abs() < 1e-9);
        assert!((loads.weight_span_m - 400.0).abs() < 1e-6);
        // Vertical load = w * weight span.
        assert!((loads.vertical_load_n - 15.97 * loads.weight_span_m).abs() < 1e-9);
        // Transverse load = pressure * diameter * wind span.
        assert!((loads.transverse_load_n - 700.0 * 0.0281 * 400.0).abs() < 1e-9);
    }

    /// A line terminal carries half the wind span and only the near-side weight.
    #[test]
    fn terminal_support_carries_one_side() {
        let s = solve_catenary(300.0, 0.0, 15.97, 30_000.0).unwrap();
        let g = SpanGeom::from_solution(&s);
        let loads = structure_loads(None, Some(g), 15.97, 0.0281, 0.0);
        assert!((loads.wind_span_m - 150.0).abs() < 1e-9);
        // Vertex at mid-span ⇒ weight span ≈ 150 m for a level span.
        assert!((loads.weight_span_m - 150.0).abs() < 1e-6);
    }

    /// On a summit (low support flanked by higher ones) the vertex of each span
    /// lies *outside* the span toward the support, driving the weight span — and
    /// hence the vertical load — negative: uplift.
    #[test]
    fn summit_support_sees_uplift() {
        // Both adjacent spans rise away from the centre support: from the centre
        // support the far ends are higher, so the elevation difference of the
        // back span (start high → end low) is negative and the ahead span
        // (start low → end high) is positive, pushing both vertices past the
        // centre support.
        let back = solve_catenary(250.0, -60.0, 15.97, 12_000.0).unwrap();
        let ahead = solve_catenary(250.0, 60.0, 15.97, 12_000.0).unwrap();
        let loads = structure_loads(
            Some(SpanGeom::from_solution(&back)),
            Some(SpanGeom::from_solution(&ahead)),
            15.97,
            0.0281,
            0.0,
        );
        assert!(
            loads.weight_span_m < 0.0,
            "expected uplift, got weight span {}",
            loads.weight_span_m
        );
        assert!(loads.vertical_load_n < 0.0);
        // Wind span is geometric and stays positive.
        assert!((loads.wind_span_m - 250.0).abs() < 1e-9);
    }
}

//! Derived engineering results for a [`Project`] — the bridge between the stored
//! model and the drafting stage (reports, sheets, and the app's data panels).
//!
//! Everything here is a pure function of the project: each span is solved with
//! the exact catenary ([`atldp_core::catenary`]), clearances are checked against
//! the stored [`ground_profile`](Project::ground_profile), and the per-structure
//! wind/weight spans and loads come from [`atldp_core::structure`]. The same
//! [`Analysis`] feeds [`crate::report`], [`crate::sheet`], and the GUI panels, so
//! they cannot disagree.

use atldp_core::catenary::solve_catenary;
use atldp_core::structure::{structure_loads, SpanGeom, StructureLoads};

use crate::{Project, Tower};

/// Number of points sampled along each catenary for clearance and drawing.
pub const SAMPLES_PER_SPAN: usize = 64;

/// Solved results for one span between two consecutive towers.
#[derive(Debug, Clone)]
pub struct SpanResult {
    /// Index of the back (start) tower.
    pub from_tower: usize,
    /// Index of the ahead (end) tower.
    pub to_tower: usize,
    /// Horizontal span length, metres.
    pub horizontal_m: f64,
    /// Attachment elevation difference (ahead − back), metres.
    pub elevation_diff_m: f64,
    /// Maximum sag below the chord, metres.
    pub sag_m: f64,
    /// Conductor arc length, metres.
    pub conductor_length_m: f64,
    /// Largest support tension, newtons.
    pub max_tension_n: f64,
    /// Largest support tension as a percentage of rated strength.
    pub tension_pct_rts: f64,
    /// Catenary low-point abscissa from the back support, metres.
    pub low_point_x_m: f64,
    /// Minimum conductor-to-ground clearance, metres; `None` without a profile.
    pub min_clearance_m: Option<f64>,
    /// Whether the span meets the project's minimum-clearance criterion.
    pub clearance_ok: bool,
}

/// Wind/weight span and conductor loads at one support.
#[derive(Debug, Clone)]
pub struct StructureResult {
    /// Index of the tower in [`Project::towers`].
    pub tower: usize,
    /// Wind/weight spans and the resulting conductor loads.
    pub loads: StructureLoads,
}

/// All derived results for a project.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    /// One entry per span, in route order.
    pub spans: Vec<SpanResult>,
    /// One entry per tower, in route order.
    pub structures: Vec<StructureResult>,
    /// Smallest clearance across all spans, metres; `None` if not computable.
    pub worst_clearance_m: Option<f64>,
    /// Largest support-tension utilisation across all spans, percent of RTS.
    pub max_tension_pct_rts: Option<f64>,
}

impl Analysis {
    /// Whether every span with a computed clearance meets the criterion.
    pub fn all_clearances_ok(&self) -> bool {
        self.spans
            .iter()
            .all(|s| s.min_clearance_m.is_none() || s.clearance_ok)
    }
}

/// Sample the catenary of one span as `(distance_m, elevation_m)` in route
/// coordinates. Returns an empty vector for a degenerate (non-positive) span.
pub fn span_catenary_points(
    project: &Project,
    t1: &Tower,
    t2: &Tower,
    samples: usize,
) -> Vec<(f64, f64)> {
    let horiz = t2.distance_m - t1.distance_m;
    if horiz <= 0.0 {
        return Vec::new();
    }
    let w = project.conductor.unit_weight_n_per_m;
    let h = project.parameters.horizontal_tension_n;
    let elev_diff = t2.attachment_elevation_m() - t1.attachment_elevation_m();
    let Ok(sol) = solve_catenary(horiz, elev_diff, w, h) else {
        return Vec::new();
    };
    let c = sol.catenary_constant();
    let a = sol.low_point_x;
    let b = -c * (a / c).cosh();
    let e1 = t1.attachment_elevation_m();
    (0..=samples)
        .map(|i| {
            let x = i as f64 / samples as f64 * horiz;
            let y_rel = c * ((x - a) / c).cosh() + b;
            (t1.distance_m + x, e1 + y_rel)
        })
        .collect()
}

/// Compute every derived result for `project`.
pub fn analyze(project: &Project) -> Analysis {
    let w = project.conductor.unit_weight_n_per_m;
    let h = project.parameters.horizontal_tension_n;
    let rts = project.conductor.rated_strength_n;
    let min_clear = project.parameters.min_clearance_m;

    let mut spans = Vec::new();
    let mut span_geoms: Vec<Option<SpanGeom>> = Vec::new();

    for i in 0..project.towers.len().saturating_sub(1) {
        let t1 = &project.towers[i];
        let t2 = &project.towers[i + 1];
        let horiz = t2.distance_m - t1.distance_m;
        let elev_diff = t2.attachment_elevation_m() - t1.attachment_elevation_m();
        let Ok(sol) = solve_catenary(horiz, elev_diff, w, h) else {
            span_geoms.push(None);
            continue;
        };

        // Clearance: sample the conductor and compare with the stored ground.
        let pts = span_catenary_points(project, t1, t2, SAMPLES_PER_SPAN);
        let min_clearance_m = if project.ground_profile.is_empty() {
            None
        } else {
            pts.iter()
                .filter_map(|&(d, y)| project.ground_at(d).map(|g| y - g))
                .fold(None, |acc: Option<f64>, c| {
                    Some(acc.map_or(c, |a| a.min(c)))
                })
        };

        let max_tension_n = sol.max_tension();
        let tension_pct_rts = if rts > 0.0 {
            100.0 * max_tension_n / rts
        } else {
            f64::NAN
        };

        span_geoms.push(Some(SpanGeom::from_solution(&sol)));
        spans.push(SpanResult {
            from_tower: i,
            to_tower: i + 1,
            horizontal_m: horiz,
            elevation_diff_m: elev_diff,
            sag_m: sol.sag,
            conductor_length_m: sol.conductor_length,
            max_tension_n,
            tension_pct_rts,
            low_point_x_m: sol.low_point_x,
            min_clearance_m,
            clearance_ok: min_clearance_m.map(|c| c >= min_clear).unwrap_or(true),
        });
    }

    // Structure loads: each tower sees the span behind it and the span ahead.
    let mut structures = Vec::new();
    for i in 0..project.towers.len() {
        let back = if i > 0 {
            span_geoms.get(i - 1).copied().flatten()
        } else {
            None
        };
        let ahead = span_geoms.get(i).copied().flatten();
        if back.is_none() && ahead.is_none() {
            continue;
        }
        let loads = structure_loads(
            back,
            ahead,
            w,
            project.conductor.diameter_m,
            project.parameters.wind_pressure_pa,
        );
        structures.push(StructureResult { tower: i, loads });
    }

    let worst_clearance_m = spans
        .iter()
        .filter_map(|s| s.min_clearance_m)
        .fold(None, |acc: Option<f64>, c| {
            Some(acc.map_or(c, |a| a.min(c)))
        });
    let max_tension_pct_rts = spans
        .iter()
        .map(|s| s.tension_pct_rts)
        .filter(|p| p.is_finite())
        .fold(None, |acc: Option<f64>, p| {
            Some(acc.map_or(p, |a| a.max(p)))
        });

    Analysis {
        spans,
        structures,
        worst_clearance_m,
        max_tension_pct_rts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileSample, Tower};

    fn three_tower_project() -> Project {
        let mut p = Project::new("analysis fixture");
        p.ground_profile = vec![
            ProfileSample {
                distance_m: 0.0,
                elevation_m: 100.0,
            },
            ProfileSample {
                distance_m: 400.0,
                elevation_m: 100.0,
            },
            ProfileSample {
                distance_m: 800.0,
                elevation_m: 100.0,
            },
        ];
        p.towers = vec![
            Tower {
                distance_m: 0.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 20.0,
            },
            Tower {
                distance_m: 400.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 20.0,
            },
            Tower {
                distance_m: 800.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 20.0,
            },
        ];
        p
    }

    #[test]
    fn produces_span_and_structure_results() {
        let p = three_tower_project();
        let a = analyze(&p);
        assert_eq!(a.spans.len(), 2);
        assert_eq!(a.structures.len(), 3);
        // Two equal level spans ⇒ interior tower wind span = 400 m.
        assert!((a.structures[1].loads.wind_span_m - 400.0).abs() < 1e-6);
        // Clearance computed and positive (20 m attach, modest sag).
        assert!(a.worst_clearance_m.unwrap() > 0.0);
        assert!(a.all_clearances_ok());
    }

    #[test]
    fn flags_clearance_violation() {
        let mut p = three_tower_project();
        // Drop the attachment so the sagged conductor dips below min clearance.
        for t in &mut p.towers {
            t.attachment_height_m = 9.0;
        }
        p.parameters.min_clearance_m = 8.0;
        p.parameters.horizontal_tension_n = 5_000.0; // big sag
        let a = analyze(&p);
        assert!(!a.all_clearances_ok());
    }
}

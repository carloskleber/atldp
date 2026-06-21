//! Derived engineering results for a [`Project`] — the bridge between the stored
//! model and the drafting stage (reports, sheets, and the app's data panels).
//!
//! Everything here is a pure function of the project. Since G7 a project carries a
//! **wire set** ([`Wire`]) and is partitioned into **tension sections** by its
//! anchor structures ([`tension_sections`]): each wire is solved span-by-span as an
//! independent catenary at its per-section stringing tension, clearance is checked
//! **wire-by-wire** against the stored [`ground_profile`](Project::ground_profile),
//! and each structure's wind/weight spans are checked against its G8 family chart.
//! The same [`Analysis`] feeds [`crate::report`], [`crate::sheet`], and the GUI
//! panels, so they cannot disagree.

use atldp_core::catenary::solve_catenary;
use atldp_core::structure::{structure_loads, SpanGeom};

use crate::{tension_sections, Project, Tower, Wire};

/// Number of points sampled along each catenary for clearance and drawing.
pub const SAMPLES_PER_SPAN: usize = 64;

/// Solved results for one wire across one span between two consecutive towers.
#[derive(Debug, Clone)]
pub struct SpanResult {
    /// Index of the back (start) tower.
    pub from_tower: usize,
    /// Index of the ahead (end) tower.
    pub to_tower: usize,
    /// Index of the wire in [`Project::wires`].
    pub wire: usize,
    /// Index of the tension section (see [`tension_sections`]) this span belongs to.
    pub section: usize,
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

/// Wind/weight span and conductor loads at one support (summed over all wires),
/// plus the G8 family-chart verdict.
#[derive(Debug, Clone)]
pub struct StructureResult {
    /// Index of the tower in [`Project::towers`].
    pub tower: usize,
    /// Mechanical function of the structure (G7).
    pub function: crate::StructureFunction,
    /// Tension section the structure belongs to (its back span's section).
    pub section: usize,
    /// Wind (horizontal) span, metres — geometric, common to all wires.
    pub wind_span_m: f64,
    /// Weight (vertical) span, metres; signed (negative ⇒ uplift). Representative
    /// (governing-wire) geometry.
    pub weight_span_m: f64,
    /// Line (deviation) angle at the structure, degrees.
    pub line_angle_deg: f64,
    /// Transverse load from wind, newtons — summed over every wire.
    pub transverse_load_n: f64,
    /// Vertical load from wire weight, newtons — summed over every wire.
    pub vertical_load_n: f64,
    /// Name of the referenced structure family, if any (G8).
    pub family: Option<String>,
    /// Whether the loads fall inside the family chart; `None` if unassigned.
    pub chart_ok: Option<bool>,
}

/// The equivalent (ruling) span of one tension section (G7).
#[derive(Debug, Clone)]
pub struct SectionResult {
    /// Section ordinal (its index in [`tension_sections`]).
    pub index: usize,
    /// Back anchor tower index.
    pub from_tower: usize,
    /// Ahead anchor tower index.
    pub to_tower: usize,
    /// Number of real spans in the section.
    pub span_count: usize,
    /// Equivalent (ruling) span length, metres: `sqrt(Σ Sᵢ³ / Σ Sᵢ)`.
    pub ruling_span_m: f64,
}

/// All derived results for a project.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    /// Tension sections, in route order.
    pub sections: Vec<SectionResult>,
    /// One entry per (wire × span), in wire-then-route order.
    pub spans: Vec<SpanResult>,
    /// One entry per tower, in route order.
    pub structures: Vec<StructureResult>,
    /// Smallest clearance across all wires and spans, metres; `None` if not computable.
    pub worst_clearance_m: Option<f64>,
    /// Largest support-tension utilisation across all wires, percent of RTS.
    pub max_tension_pct_rts: Option<f64>,
}

impl Analysis {
    /// Whether every span with a computed clearance meets the criterion.
    pub fn all_clearances_ok(&self) -> bool {
        self.spans
            .iter()
            .all(|s| s.min_clearance_m.is_none() || s.clearance_ok)
    }

    /// Whether every family-assigned structure stays within its chart (G8).
    pub fn all_charts_ok(&self) -> bool {
        self.structures.iter().all(|s| s.chart_ok != Some(false))
    }

    /// Spans belonging to one wire, in route order.
    pub fn spans_of_wire(&self, wire: usize) -> impl Iterator<Item = &SpanResult> {
        self.spans.iter().filter(move |s| s.wire == wire)
    }
}

/// Sample one wire's catenary across a span as `(distance_m, elevation_m)` in
/// route coordinates. Returns an empty vector for a degenerate (non-positive)
/// span. `tension_n` is the wire's stringing tension in this section.
pub fn span_catenary_points(
    t1: &Tower,
    t2: &Tower,
    wire: &Wire,
    tension_n: f64,
    samples: usize,
) -> Vec<(f64, f64)> {
    let horiz = t2.distance_m - t1.distance_m;
    if horiz <= 0.0 {
        return Vec::new();
    }
    let w = wire.conductor.unit_weight_n_per_m;
    let e1 = t1.attachment_elevation_m() + wire.vertical_offset_m;
    let e2 = t2.attachment_elevation_m() + wire.vertical_offset_m;
    let Ok(sol) = solve_catenary(horiz, e2 - e1, w, tension_n) else {
        return Vec::new();
    };
    let c = sol.catenary_constant();
    let a = sol.low_point_x;
    let b = -c * (a / c).cosh();
    (0..=samples)
        .map(|i| {
            let x = i as f64 / samples as f64 * horiz;
            let y_rel = c * ((x - a) / c).cosh() + b;
            (t1.distance_m + x, e1 + y_rel)
        })
        .collect()
}

/// Which tension section a span (between tower `i` and `i+1`) belongs to.
fn section_of_span(sections: &[(usize, usize)], span: usize) -> usize {
    sections
        .iter()
        .position(|&(a, b)| a <= span && span < b)
        .unwrap_or(0)
}

/// Compute every derived result for `project`.
pub fn analyze(project: &Project) -> Analysis {
    let towers = &project.towers;
    let n_span = towers.len().saturating_sub(1);
    let sections = tension_sections(towers);
    let min_clear = project.parameters.min_clearance_m;
    let wind_pressure = project.parameters.wind_pressure_pa;

    // ── per-(wire, span) catenary solve ──
    // span_geoms[wire][span] feeds the structure-load pass below.
    let mut span_geoms: Vec<Vec<Option<SpanGeom>>> = vec![vec![None; n_span]; project.wires.len()];
    let mut spans = Vec::new();

    for (wi, wire) in project.wires.iter().enumerate() {
        let w = wire.conductor.unit_weight_n_per_m;
        let rts = wire.conductor.rated_strength_n;
        for (i, pair) in towers.windows(2).enumerate() {
            let t1 = &pair[0];
            let t2 = &pair[1];
            let sec = section_of_span(&sections, i);
            let h = project.wire_tension(wi, sec);
            let horiz = t2.distance_m - t1.distance_m;
            let e1 = t1.attachment_elevation_m() + wire.vertical_offset_m;
            let e2 = t2.attachment_elevation_m() + wire.vertical_offset_m;
            let elev_diff = e2 - e1;
            let Ok(sol) = solve_catenary(horiz, elev_diff, w, h) else {
                continue;
            };

            // Clearance: sample this wire and compare with the stored ground.
            let pts = span_catenary_points(t1, t2, wire, h, SAMPLES_PER_SPAN);
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

            span_geoms[wi][i] = Some(SpanGeom::from_solution(&sol));
            spans.push(SpanResult {
                from_tower: i,
                to_tower: i + 1,
                wire: wi,
                section: sec,
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
    }

    // ── structure loads (summed over wires) + family-chart check ──
    let mut structures = Vec::new();
    for (ti, tower) in towers.iter().enumerate() {
        let mut transverse = 0.0;
        let mut vertical = 0.0;
        let mut rep_wind = 0.0;
        let mut rep_weight = 0.0;
        let mut any = false;
        for (wi, wire) in project.wires.iter().enumerate() {
            let back = if ti > 0 {
                span_geoms[wi].get(ti - 1).copied().flatten()
            } else {
                None
            };
            let ahead = span_geoms[wi].get(ti).copied().flatten();
            if back.is_none() && ahead.is_none() {
                continue;
            }
            let loads = structure_loads(
                back,
                ahead,
                wire.conductor.unit_weight_n_per_m,
                wire.conductor.diameter_m,
                wind_pressure,
            );
            transverse += loads.transverse_load_n;
            vertical += loads.vertical_load_n;
            // The first contributing wire is the representative geometry.
            if !any {
                rep_wind = loads.wind_span_m;
                rep_weight = loads.weight_span_m;
                any = true;
            }
        }
        if !any {
            continue;
        }

        let section = section_of_span(&sections, ti.min(n_span.saturating_sub(1)));
        let (family, chart_ok) = match &tower.family {
            Some(tf) => {
                let chart = tf
                    .chart_override
                    .as_ref()
                    .or_else(|| project.families.get(tf.family).map(|f| &f.chart));
                let name = project.families.get(tf.family).map(|f| f.name.clone());
                let ok = chart.map(|c| c.allows(rep_wind, rep_weight, tower.line_angle_deg));
                (name, ok)
            }
            None => (None, None),
        };

        structures.push(StructureResult {
            tower: ti,
            function: tower.function,
            section,
            wind_span_m: rep_wind,
            weight_span_m: rep_weight,
            line_angle_deg: tower.line_angle_deg,
            transverse_load_n: transverse,
            vertical_load_n: vertical,
            family,
            chart_ok,
        });
    }

    // ── tension sections (ruling spans) ──
    let section_results = sections
        .iter()
        .enumerate()
        .map(|(index, &(a, b))| {
            let lengths: Vec<f64> = (a..b)
                .map(|i| towers[i + 1].distance_m - towers[i].distance_m)
                .collect();
            let num: f64 = lengths.iter().map(|l| l.powi(3)).sum();
            let den: f64 = lengths.iter().sum();
            let ruling_span_m = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
            SectionResult {
                index,
                from_tower: a,
                to_tower: b,
                span_count: b - a,
                ruling_span_m,
            }
        })
        .collect();

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
        sections: section_results,
        spans,
        structures,
        worst_clearance_m,
        max_tension_pct_rts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApplicationChart, ProfileSample, StructureFamily, StructureFunction, Tower, TowerFamily,
    };

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
                ..Default::default()
            },
            Tower {
                distance_m: 400.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 20.0,
                ..Default::default()
            },
            Tower {
                distance_m: 800.0,
                ground_elevation_m: 100.0,
                attachment_height_m: 20.0,
                ..Default::default()
            },
        ];
        p
    }

    #[test]
    fn produces_span_and_structure_results() {
        let p = three_tower_project();
        let a = analyze(&p);
        // One wire × two spans.
        assert_eq!(a.spans.len(), 2);
        assert_eq!(a.structures.len(), 3);
        // Two equal level spans ⇒ interior tower wind span = 400 m.
        assert!((a.structures[1].wind_span_m - 400.0).abs() < 1e-6);
        // Clearance computed and positive (20 m attach, modest sag).
        assert!(a.worst_clearance_m.unwrap() > 0.0);
        assert!(a.all_clearances_ok());
        // A single tension section spanning all three towers.
        assert_eq!(a.sections.len(), 1);
        assert_eq!(a.sections[0].span_count, 2);
        assert!((a.sections[0].ruling_span_m - 400.0).abs() < 1e-6);
    }

    #[test]
    fn anchor_splits_into_two_sections() {
        let mut p = three_tower_project();
        p.towers[1].function = StructureFunction::Anchor;
        let a = analyze(&p);
        assert_eq!(a.sections.len(), 2);
        assert_eq!(a.sections[0].from_tower, 0);
        assert_eq!(a.sections[0].to_tower, 1);
        assert_eq!(a.sections[1].from_tower, 1);
        assert_eq!(a.sections[1].to_tower, 2);
    }

    #[test]
    fn multi_wire_solves_each_wire() {
        let mut p = three_tower_project();
        p.wires = vec![
            Wire::phase("A", crate::ConductorSpec::drake(), 0.0, 0.0),
            Wire::shield("S", crate::ConductorSpec::ehs_shield(), 5.0, 0.0).strung(36_000.0),
        ];
        let a = analyze(&p);
        // Two wires × two spans.
        assert_eq!(a.spans.len(), 4);
        // Structure loads sum both wires' vertical contributions.
        assert!(a.structures[1].vertical_load_n > 0.0);
    }

    #[test]
    fn flags_clearance_violation() {
        let mut p = three_tower_project();
        for t in &mut p.towers {
            t.attachment_height_m = 9.0;
        }
        p.parameters.min_clearance_m = 8.0;
        p.wires[0].tension_n = 5_000.0; // big sag
        let a = analyze(&p);
        assert!(!a.all_clearances_ok());
    }

    #[test]
    fn family_chart_flags_overloaded_structure() {
        let mut p = three_tower_project();
        p.families = vec![StructureFamily {
            name: "Tiny".to_string(),
            function: StructureFunction::Suspension,
            min_height_m: 10.0,
            max_height_m: 30.0,
            default_height_m: 20.0,
            // Allow only up to a 100 m wind span — the 400 m spans bust it.
            chart: ApplicationChart::rectangular(100.0, -200.0, 200.0, 5.0),
        }];
        p.towers[1].family = Some(TowerFamily {
            family: 0,
            height_m: 20.0,
            effective_height_override_m: None,
            chart_override: None,
        });
        let a = analyze(&p);
        let st = a.structures.iter().find(|s| s.tower == 1).unwrap();
        assert_eq!(st.chart_ok, Some(false));
        assert!(!a.all_charts_ok());
    }
}

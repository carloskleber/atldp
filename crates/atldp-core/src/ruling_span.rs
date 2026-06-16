//! Ruling-span (equivalent-span) section model.
//!
//! A run of spans between two strain (dead-end) structures shares a single
//! horizontal tension, because the suspension insulators swing freely and
//! equalise `H` along the section. The classic ruling-span method replaces the
//! section with one fictitious level span
//!
//! ```text
//! RS = sqrt( sum(S_i^3) / sum(S_i) )
//! ```
//!
//! solves the change-of-state on that single span to get the common horizontal
//! tension for each weather state, and then applies that tension back to every
//! real span (each with its own length and elevation difference) to get per-span
//! sag and tension.
//!
//! This is exact only under the usual ruling-span assumptions (free-swinging
//! suspensions, similar spans); its limits at high temperature are documented by
//! Motlis et al. 1999, and the FEM track (ADR-0003, Phase 6) is the escape hatch
//! when those assumptions break.
//!
//! Mirror of the Python `atldp.core.ruling_span` oracle (ADR-0014).

use crate::catenary::{solve_span, CatenarySolution, Method};
use crate::change_of_state::{change_of_state, StateCase};
use crate::conductor::Conductor;
use crate::geometry::Span;
use crate::CoreError;

/// Per-section result: the equivalent span, common tension, and per-span states.
#[derive(Clone, Debug, PartialEq)]
pub struct RulingSpanResult {
    /// Equivalent span length, m.
    pub ruling_span: f64,
    /// Common horizontal tension at the target state, N.
    pub h_tension: f64,
    /// Solution of the fictitious ruling span.
    pub ruling_solution: CatenarySolution,
    /// Per-span solutions at the common tension.
    pub spans: Vec<CatenarySolution>,
}

/// A tension section: a conductor and the ordered list of its spans.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Conductor strung across the section.
    pub conductor: Conductor,
    /// Ordered spans between the two strain structures.
    pub spans: Vec<Span>,
}

impl Section {
    /// Build a section from a conductor and its spans.
    pub fn new(conductor: Conductor, spans: Vec<Span>) -> Self {
        Self { conductor, spans }
    }

    /// Equivalent (ruling) span length, m.
    pub fn ruling_span(&self) -> f64 {
        let lengths: Vec<f64> = self.spans.iter().map(|s| s.horizontal_distance()).collect();
        let num: f64 = lengths.iter().map(|l| l.powi(3)).sum();
        let den: f64 = lengths.iter().sum();
        (num / den).sqrt()
    }

    /// Solve the section at `target` given a `reference` state.
    ///
    /// `reference_h` is the common horizontal tension at the reference state
    /// (e.g. the stringing tension). The change-of-state runs on the ruling span;
    /// the resulting tension is applied to every real span.
    pub fn solve(
        &self,
        reference_h: f64,
        reference: &StateCase,
        target: &StateCase,
        method: Method,
    ) -> Result<RulingSpanResult, CoreError> {
        let rs = self.ruling_span();
        let ruling_solution = change_of_state(
            &self.conductor,
            rs,
            0.0,
            reference_h,
            reference,
            target,
            method,
        )?;
        let h_tension = ruling_solution.h_tension;
        let mut per_span = Vec::with_capacity(self.spans.len());
        for s in &self.spans {
            per_span.push(solve_span(
                s.horizontal_distance(),
                s.elevation_difference(),
                target.w,
                h_tension,
                method,
            )?);
        }
        Ok(RulingSpanResult {
            ruling_span: rs,
            h_tension,
            ruling_solution,
            spans: per_span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor::drake_acsr;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
    }

    #[test]
    fn ruling_span_of_equal_spans_equals_span_length() {
        let section = Section::new(drake_acsr(), vec![Span::level(350.0, 0.0); 4]);
        assert!(close(section.ruling_span(), 350.0, 1e-12));
    }

    #[test]
    fn ruling_span_between_min_and_max() {
        let section = Section::new(
            drake_acsr(),
            vec![
                Span::level(200.0, 0.0),
                Span::level(400.0, 0.0),
                Span::level(600.0, 0.0),
            ],
        );
        let rs = section.ruling_span();
        assert!(rs > 200.0 && rs < 600.0);
        assert!(rs > 400.0); // weighted toward the longer (cubed) spans
    }

    #[test]
    fn equal_spans_match_single_span_change_of_state() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("ref", 15.0, w);
        let hot = StateCase::new("hot", 75.0, w);

        let section = Section::new(drake.clone(), vec![Span::level(350.0, 0.0); 3]);
        let result = section
            .solve(31500.0, &reference, &hot, Method::Auto)
            .unwrap();

        let single =
            change_of_state(&drake, 350.0, 0.0, 31500.0, &reference, &hot, Method::Auto).unwrap();

        assert!(close(result.h_tension, single.h_tension, 1e-9));
        for span_sol in &result.spans {
            assert!(close(span_sol.sag, single.sag, 1e-9));
            assert!(close(span_sol.h_tension, result.h_tension, 1e-12));
        }
    }

    #[test]
    fn common_tension_applied_to_uneven_spans() {
        let drake = drake_acsr();
        let w = drake.unit_weight;
        let reference = StateCase::new("ref", 15.0, w);
        let hot = StateCase::new("hot", 75.0, w);

        let spans = vec![
            Span::level(250.0, 0.0),
            Span::level(500.0, 60.0),
            Span::level(400.0, -30.0),
        ];
        let result = Section::new(drake, spans)
            .solve(31500.0, &reference, &hot, Method::Auto)
            .unwrap();

        assert!(result
            .spans
            .iter()
            .all(|s| close(s.h_tension, result.h_tension, 1e-12)));
        assert!(result.spans[1].sag > result.spans[2].sag);
        assert!(result.spans[2].sag > result.spans[0].sag);
    }
}

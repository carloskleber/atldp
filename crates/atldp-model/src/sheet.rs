//! Plan-&-profile drawing (SVG) for the drafting stage — phase G6 / stage 6.
//!
//! Produces a self-contained SVG sheet — the classic transmission-line drawing:
//! a **profile** panel (ground line, sagged conductors, structures, against
//! distance and elevation axes with a vertical exaggeration) above a **plan**
//! strip (the route stationing seen from above). SVG is an open, text vector
//! format, so the output diffs, scales, and prints without a binary viewer,
//! matching the open-format intent of stage 6.
//!
//! The drawing is built by string concatenation — no SVG dependency — so it stays
//! auditable and adds nothing to the binary. SVG attributes are single-quoted
//! (valid XML) to keep the Rust raw string literals simple.

use std::fmt::Write as _;

use crate::analysis::{analyze, span_catenary_points, Analysis, SAMPLES_PER_SPAN};
use crate::Project;

/// Default vertical exaggeration of the profile panel.
pub const DEFAULT_VERTICAL_EXAGGERATION: f64 = 5.0;

const WIDTH: f64 = 1100.0;
const MARGIN_L: f64 = 70.0;
const MARGIN_R: f64 = 30.0;
const PROFILE_TOP: f64 = 60.0;
const PLAN_HEIGHT: f64 = 70.0;
const GAP: f64 = 50.0;

/// Render a plan-&-profile SVG sheet for `project` at the default exaggeration.
pub fn plan_profile_svg(project: &Project) -> String {
    plan_profile_svg_with(project, &analyze(project), DEFAULT_VERTICAL_EXAGGERATION)
}

/// Render the sheet from a pre-computed [`Analysis`] at a given vertical
/// exaggeration (`>= 1.0`).
pub fn plan_profile_svg_with(project: &Project, analysis: &Analysis, vertical_exag: f64) -> String {
    let plot_w = WIDTH - MARGIN_L - MARGIN_R;

    // ── world bounds ──
    let mut d_min = f64::INFINITY;
    let mut d_max = f64::NEG_INFINITY;
    let mut e_min = f64::INFINITY;
    let mut e_max = f64::NEG_INFINITY;
    let mut note = |d: f64, e: f64| {
        d_min = d_min.min(d);
        d_max = d_max.max(d);
        e_min = e_min.min(e);
        e_max = e_max.max(e);
    };
    for s in &project.ground_profile {
        if s.elevation_m.is_finite() {
            note(s.distance_m, s.elevation_m);
        }
    }
    for t in &project.towers {
        note(t.distance_m, t.ground_elevation_m);
        note(t.distance_m, t.attachment_elevation_m());
    }
    if !d_min.is_finite() {
        // Nothing to draw — emit a minimal valid placeholder sheet.
        return placeholder_svg(&project.metadata.name);
    }
    if (d_max - d_min).abs() < 1e-6 {
        d_max = d_min + 1.0;
    }
    if (e_max - e_min).abs() < 1e-6 {
        e_max = e_min + 1.0;
    }
    let pad_e = (e_max - e_min) * 0.1;
    e_min -= pad_e;
    e_max += pad_e;

    let scale_x = plot_w / (d_max - d_min);
    let vexag = vertical_exag.max(1.0);
    let scale_y = scale_x * vexag;
    let profile_h = (e_max - e_min) * scale_y;

    let plan_top = PROFILE_TOP + profile_h + GAP;
    let height = plan_top + PLAN_HEIGHT + 40.0;

    // World → SVG mappers.
    let px = |d: f64| MARGIN_L + (d - d_min) * scale_x;
    let py = |e: f64| PROFILE_TOP + (e_max - e) * scale_y; // SVG y grows downward

    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns='http://www.w3.org/2000/svg' width='{WIDTH:.0}' height='{height:.0}' viewBox='0 0 {WIDTH:.0} {height:.0}' font-family='sans-serif'>"#
    );
    let _ = writeln!(
        s,
        r#"<rect width='{WIDTH:.0}' height='{height:.0}' fill='#ffffff'/>"#
    );

    // Title.
    let _ = writeln!(
        s,
        r#"<text x='{MARGIN_L:.0}' y='32' font-size='20' font-weight='bold'>{}</text>"#,
        escape(&project.metadata.name)
    );
    let _ = writeln!(
        s,
        r#"<text x='{:.0}' y='32' font-size='11' text-anchor='end' fill='#555'>plan &amp; profile · V.E. {vexag:.0}× · ATLDP {}</text>"#,
        WIDTH - MARGIN_R,
        crate::VERSION
    );

    // ── profile panel frame ──
    frame(&mut s, MARGIN_L, PROFILE_TOP, plot_w, profile_h);

    // Elevation gridlines + labels at a "nice" step.
    let e_step = nice_step((e_max - e_min) / 6.0);
    let mut e = (e_min / e_step).ceil() * e_step;
    while e <= e_max {
        let y = py(e);
        let _ = writeln!(
            s,
            r#"<line x1='{MARGIN_L:.1}' y1='{y:.1}' x2='{:.1}' y2='{y:.1}' stroke='#eeeeee'/>"#,
            MARGIN_L + plot_w
        );
        let _ = writeln!(
            s,
            r#"<text x='{:.1}' y='{:.1}' font-size='10' text-anchor='end' fill='#555'>{e:.0}</text>"#,
            MARGIN_L - 6.0,
            y + 3.0
        );
        e += e_step;
    }

    // ── terrain polyline + fill ──
    let prof: Vec<(f64, f64)> = project
        .ground_profile
        .iter()
        .filter(|s| s.elevation_m.is_finite())
        .map(|s| (px(s.distance_m), py(s.elevation_m)))
        .collect();
    if prof.len() >= 2 {
        let base_y = PROFILE_TOP + profile_h;
        let mut d = format!("M {:.1} {:.1}", prof[0].0, base_y);
        for &(x, y) in &prof {
            let _ = write!(d, " L {x:.1} {y:.1}");
        }
        let _ = write!(d, " L {:.1} {base_y:.1} Z", prof.last().unwrap().0);
        let _ = writeln!(s, r#"<path d='{d}' fill='#e8efe0' stroke='none'/>"#);
        let line: String = prof
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| format!("{} {x:.1} {y:.1}", if i == 0 { "M" } else { "L" }))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(
            s,
            r#"<path d='{line}' fill='none' stroke='#5a7d2a' stroke-width='1.5'/>"#
        );
    }

    // ── conductors (clearance-coloured) ──
    for sp in &analysis.spans {
        let t1 = &project.towers[sp.from_tower];
        let t2 = &project.towers[sp.to_tower];
        let pts = span_catenary_points(project, t1, t2, SAMPLES_PER_SPAN);
        if pts.len() < 2 {
            continue;
        }
        let colour = if sp.min_clearance_m.is_some() && !sp.clearance_ok {
            "#d62828"
        } else {
            "#1f8fbf"
        };
        let path: String = pts
            .iter()
            .enumerate()
            .map(|(i, &(d, y))| {
                format!(
                    "{} {:.1} {:.1}",
                    if i == 0 { "M" } else { "L" },
                    px(d),
                    py(y)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(
            s,
            r#"<path d='{path}' fill='none' stroke='{colour}' stroke-width='1.6'/>"#
        );
    }

    // ── towers ──
    for (i, t) in project.towers.iter().enumerate() {
        let x = px(t.distance_m);
        let yg = py(t.ground_elevation_m);
        let ya = py(t.attachment_elevation_m());
        let _ = writeln!(
            s,
            r#"<line x1='{x:.1}' y1='{yg:.1}' x2='{x:.1}' y2='{ya:.1}' stroke='#b8860b' stroke-width='2'/>"#
        );
        let _ = writeln!(
            s,
            r#"<line x1='{:.1}' y1='{ya:.1}' x2='{:.1}' y2='{ya:.1}' stroke='#b8860b' stroke-width='2'/>"#,
            x - 5.0,
            x + 5.0
        );
        let _ = writeln!(
            s,
            r#"<text x='{x:.1}' y='{:.1}' font-size='10' text-anchor='middle' fill='#7a5b00'>T{}</text>"#,
            ya - 6.0,
            i + 1
        );
    }

    // ── distance axis labels under the profile ──
    let d_step = nice_step((d_max - d_min) / 8.0);
    let mut d = (d_min / d_step).ceil() * d_step;
    let axis_y = PROFILE_TOP + profile_h;
    while d <= d_max {
        let x = px(d);
        let _ = writeln!(
            s,
            r#"<line x1='{x:.1}' y1='{axis_y:.1}' x2='{x:.1}' y2='{:.1}' stroke='#888'/>"#,
            axis_y + 5.0
        );
        let _ = writeln!(
            s,
            r#"<text x='{x:.1}' y='{:.1}' font-size='10' text-anchor='middle' fill='#555'>{:.0}</text>"#,
            axis_y + 17.0,
            d
        );
        d += d_step;
    }
    let _ = writeln!(
        s,
        r#"<text x='{:.1}' y='{:.1}' font-size='10' fill='#555'>distance (m) →</text>"#,
        MARGIN_L,
        axis_y + 32.0
    );

    // ── plan strip ──
    let _ = writeln!(
        s,
        r#"<text x='{MARGIN_L:.0}' y='{:.1}' font-size='11' font-weight='bold' fill='#444'>Plan</text>"#,
        plan_top - 8.0
    );
    frame(&mut s, MARGIN_L, plan_top, plot_w, PLAN_HEIGHT);
    let centre = plan_top + PLAN_HEIGHT * 0.5;
    let _ = writeln!(
        s,
        r#"<line x1='{MARGIN_L:.1}' y1='{centre:.1}' x2='{:.1}' y2='{centre:.1}' stroke='#1f8fbf' stroke-width='2'/>"#,
        MARGIN_L + plot_w
    );
    for (i, t) in project.towers.iter().enumerate() {
        let x = px(t.distance_m);
        let _ = writeln!(
            s,
            r#"<rect x='{:.1}' y='{:.1}' width='6' height='6' fill='#b8860b'/>"#,
            x - 3.0,
            centre - 3.0
        );
        let _ = writeln!(
            s,
            r#"<text x='{x:.1}' y='{:.1}' font-size='9' text-anchor='middle' fill='#7a5b00'>T{}</text>"#,
            centre - 8.0,
            i + 1
        );
    }

    let _ = writeln!(s, "</svg>");
    s
}

/// Draw a thin rectangular panel frame.
fn frame(s: &mut String, x: f64, y: f64, w: f64, h: f64) {
    let _ = writeln!(
        s,
        r#"<rect x='{x:.1}' y='{y:.1}' width='{w:.1}' height='{h:.1}' fill='none' stroke='#888' stroke-width='1'/>"#
    );
}

/// A "nice" axis step (1, 2, 5 × 10ⁿ) at least as large as `raw`.
fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f64.powf(exp);
    for m in [1.0, 2.0, 5.0, 10.0] {
        if m * base >= raw {
            return m * base;
        }
    }
    10.0 * base
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn placeholder_svg(name: &str) -> String {
    format!(
        r#"<svg xmlns='http://www.w3.org/2000/svg' width='{WIDTH:.0}' height='200' viewBox='0 0 {WIDTH:.0} 200' font-family='sans-serif'><rect width='{WIDTH:.0}' height='200' fill='#ffffff'/><text x='{MARGIN_L:.0}' y='40' font-size='18' font-weight='bold'>{}</text><text x='{MARGIN_L:.0}' y='70' font-size='12' fill='#777'>Nothing to draw — spot towers and load terrain first.</text></svg>"#,
        escape(name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileSample, Tower};

    fn project() -> Project {
        let mut p = Project::new("Sheet test");
        p.ground_profile = vec![
            ProfileSample {
                distance_m: 0.0,
                elevation_m: 100.0,
            },
            ProfileSample {
                distance_m: 400.0,
                elevation_m: 60.0,
            },
            ProfileSample {
                distance_m: 800.0,
                elevation_m: 110.0,
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
                ground_elevation_m: 60.0,
                attachment_height_m: 20.0,
            },
            Tower {
                distance_m: 800.0,
                ground_elevation_m: 110.0,
                attachment_height_m: 20.0,
            },
        ];
        p
    }

    #[test]
    fn emits_well_formed_svg() {
        let svg = plan_profile_svg(&project());
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Sheet test"));
        assert!(svg.contains("Plan"));
        // A conductor path and tower labels are present.
        assert!(svg.contains(">T1</text>"));
        assert!(svg.contains("stroke='#1f8fbf'"));
    }

    #[test]
    fn empty_project_yields_placeholder() {
        let p = Project::new("Empty");
        let svg = plan_profile_svg(&p);
        assert!(svg.contains("Nothing to draw"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn escapes_title_markup() {
        let mut p = project();
        p.metadata.name = "A & B <line>".to_string();
        let svg = plan_profile_svg(&p);
        assert!(svg.contains("A &amp; B &lt;line&gt;"));
    }
}

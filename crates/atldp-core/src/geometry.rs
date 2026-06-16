//! 3D geometry for transmission-line spans.
//!
//! Sag-tension is usually *presented* in 2D, but the real problem is 3D: support
//! attachment points sit at arbitrary positions, consecutive spans have different
//! lengths and elevations, and the line changes plan direction at angle towers.
//!
//! We model attachment points as 3D coordinates `(east, north, up)` in metres. A
//! [`Span`] is the segment between two such points. The mechanics of a single
//! span (catenary / parabola) act in the **vertical plane through the two
//! attachment points**, so the only span quantities the analytic core needs are
//! the *horizontal distance* between the ends and their *elevation difference*;
//! these are derived here from the full 3D coordinates.
//!
//! This keeps the door open for Phase 2 without rework:
//!
//! * **Angle towers** turn the route in plan. The plan bearing of each span is
//!   available ([`Span::plan_bearing_rad`]); the change in bearing at a tower
//!   feeds the structure's transverse load later. It does *not* change the
//!   within-span catenary, which still lives in the vertical plane through the
//!   chord.
//! * **Wind blow-out / swing** tilts that load plane out of vertical. The catenary
//!   solver already takes the load per unit length as a free parameter, so a
//!   tilted resultant load slots in as a Phase-2 concern without touching the
//!   geometry.
//!
//! Mirror of the Python `atldp.core.geometry` oracle (ADR-0014).

use std::ops::Sub;

/// An attachment point in metres. `z` is the vertical (up) axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3D {
    /// East.
    pub x: f64,
    /// North.
    pub y: f64,
    /// Up (elevation).
    pub z: f64,
}

impl Point3D {
    /// Construct a point from its `(east, north, up)` coordinates.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl Sub for Point3D {
    type Output = Point3D;

    fn sub(self, other: Point3D) -> Point3D {
        Point3D::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

/// A single span between two attachment points.
///
/// `start` and `end` are the conductor attachment points (typically the
/// suspension/strain insulator clamp positions). The order matters only for the
/// sign of [`Span::elevation_difference`]; results are otherwise symmetric.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    /// First (lower-numbered) attachment point.
    pub start: Point3D,
    /// Second attachment point.
    pub end: Point3D,
}

impl Span {
    /// Construct a span from its two attachment points.
    pub fn new(start: Point3D, end: Point3D) -> Self {
        Self { start, end }
    }

    /// Horizontal (plan) distance between the two ends, metres.
    pub fn horizontal_distance(&self) -> f64 {
        let d = self.end - self.start;
        d.x.hypot(d.y)
    }

    /// `end.z - start.z` — positive when the end is higher, metres.
    pub fn elevation_difference(&self) -> f64 {
        self.end.z - self.start.z
    }

    /// Straight-line distance between the two ends, metres.
    pub fn chord_length(&self) -> f64 {
        let d = self.end - self.start;
        (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
    }

    /// Angle of the chord above the horizontal, radians.
    pub fn inclination_rad(&self) -> f64 {
        self.elevation_difference()
            .atan2(self.horizontal_distance())
    }

    /// Plan bearing of the span, radians, measured from the +north axis
    /// clockwise toward +east (compass convention). Used at angle towers to
    /// derive the change in direction; it does not affect the catenary.
    pub fn plan_bearing_rad(&self) -> f64 {
        let d = self.end - self.start;
        d.x.atan2(d.y)
    }

    /// Convenience constructor for a span described only by its horizontal
    /// distance and elevation difference (the 2D view of the 3D span).
    pub fn level(horizontal_distance: f64, elevation_difference: f64) -> Self {
        Self::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(horizontal_distance, 0.0, elevation_difference),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn level_span_helpers() {
        let span = Span::level(400.0, 25.0);
        assert_eq!(span.horizontal_distance(), 400.0);
        assert_eq!(span.elevation_difference(), 25.0);
        assert_eq!(span.chord_length(), 400.0_f64.hypot(25.0));
    }

    #[test]
    fn span_projects_to_horizontal_and_elevation() {
        // A span running NE and uphill.
        let span = Span::new(
            Point3D::new(0.0, 0.0, 100.0),
            Point3D::new(300.0, 400.0, 150.0),
        );
        assert!(close(span.horizontal_distance(), 500.0)); // 3-4-5
        assert!(close(span.elevation_difference(), 50.0));
        assert!(close(
            span.chord_length(),
            (500.0_f64.powi(2) + 50.0_f64.powi(2)).sqrt()
        ));
    }

    #[test]
    fn plan_bearing_is_independent_of_elevation() {
        let flat = Span::new(Point3D::new(0.0, 0.0, 0.0), Point3D::new(100.0, 100.0, 0.0));
        let sloped = Span::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(100.0, 100.0, 80.0),
        );
        assert!(close(flat.plan_bearing_rad(), sloped.plan_bearing_rad()));
        assert!(close(flat.plan_bearing_rad(), 45.0_f64.to_radians()));
    }
}

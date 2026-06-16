"""3D geometry for transmission-line spans.

Sag-tension is usually *presented* in 2D, but the real problem is 3D: support
attachment points sit at arbitrary positions, consecutive spans have different
lengths and elevations, and the line changes plan direction at angle towers.

We model attachment points as 3D coordinates ``(east, north, up)`` in metres. A
:class:`Span` is the segment between two such points. The mechanics of a single
span (catenary / parabola) act in the **vertical plane through the two
attachment points**, so the only span quantities the analytic core needs are the
*horizontal distance* between the ends and their *elevation difference*; these
are derived here from the full 3D coordinates.

This keeps the door open for Phase 2 without rework:

* **Angle towers** turn the route in plan. The plan bearing of each span is
  available (:attr:`Span.plan_bearing_rad`); the change in bearing at a tower
  feeds the structure's transverse load later. It does *not* change the
  within-span catenary, which still lives in the vertical plane through the
  chord.
* **Wind blow-out / swing** tilts that load plane out of vertical. The catenary
  solver already takes the load per unit length as a free parameter, so a tilted
  resultant load slots in as a Phase-2 concern without touching the geometry.
"""

from __future__ import annotations

import math
from dataclasses import dataclass


@dataclass(frozen=True)
class Point3D:
    """An attachment point in metres. ``z`` is the vertical (up) axis."""

    x: float  # east
    y: float  # north
    z: float  # up (elevation)

    def __sub__(self, other: "Point3D") -> "Point3D":
        return Point3D(self.x - other.x, self.y - other.y, self.z - other.z)


@dataclass(frozen=True)
class Span:
    """A single span between two attachment points.

    ``start`` and ``end`` are the conductor attachment points (typically the
    suspension/strain insulator clamp positions). The order matters only for the
    sign of :attr:`elevation_difference`; results are otherwise symmetric.
    """

    start: Point3D
    end: Point3D

    @property
    def horizontal_distance(self) -> float:
        """Horizontal (plan) distance between the two ends, metres."""
        d = self.end - self.start
        return math.hypot(d.x, d.y)

    @property
    def elevation_difference(self) -> float:
        """``end.z - start.z`` — positive when the end is higher, metres."""
        return self.end.z - self.start.z

    @property
    def chord_length(self) -> float:
        """Straight-line distance between the two ends, metres."""
        d = self.end - self.start
        return math.sqrt(d.x * d.x + d.y * d.y + d.z * d.z)

    @property
    def inclination_rad(self) -> float:
        """Angle of the chord above the horizontal, radians."""
        return math.atan2(self.elevation_difference, self.horizontal_distance)

    @property
    def plan_bearing_rad(self) -> float:
        """Plan bearing of the span, radians, measured from the +north axis
        clockwise toward +east (compass convention). Used at angle towers to
        derive the change in direction; it does not affect the catenary."""
        d = self.end - self.start
        return math.atan2(d.x, d.y)

    @classmethod
    def level(cls, horizontal_distance: float, elevation_difference: float = 0.0) -> "Span":
        """Convenience constructor for a span described only by its horizontal
        distance and elevation difference (the 2D view of the 3D span)."""
        return cls(
            Point3D(0.0, 0.0, 0.0),
            Point3D(horizontal_distance, 0.0, elevation_difference),
        )

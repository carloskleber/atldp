"""ATLDP core — validated, headless sag-tension engine (Phase 1).

This package is the pure engineering core mandated by ADR-0002: no GUI, no
network, no geospatial dependencies. It implements the analytic sag-tension
baseline of ADR-0003 (exact catenary + parabolic approximation, change-of-state,
ruling span) for the design pipeline's stage 4 (ADR-0009).

Although sag-tension is classically drawn in 2D, a transmission line is a 3D
problem: supports sit at arbitrary positions, spans are uneven (the two ends are
at different elevations), and the route turns at angle towers. The geometry here
is therefore expressed in 3D from the start (see :mod:`atldp.core.geometry`),
and the within-span mechanics solve the *inclined* catenary rather than assuming
level supports. Out-of-plane effects (wind blow-out / swing) and angle-tower
transverse loads are deliberately left to Phase 2 and the FEM track (ADR-0003),
but the geometry is laid out so they slot in without reshaping the core.
"""

from atldp.core.conductor import Conductor
from atldp.core.geometry import Point3D, Span
from atldp.core.catenary import CatenarySolution, solve_span
from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.ruling_span import RulingSpanResult, Section

__all__ = [
    "Conductor",
    "Point3D",
    "Span",
    "CatenarySolution",
    "solve_span",
    "StateCase",
    "change_of_state",
    "RulingSpanResult",
    "Section",
]

__version__ = "0.1.0"

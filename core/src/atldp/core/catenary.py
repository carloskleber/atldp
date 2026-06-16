"""Single-span sag-tension: exact catenary and parabolic approximation.

Both supports may be at different elevations (inclined / uneven span), which is
the normal case on real terrain. The span is described by its horizontal
distance ``S`` and elevation difference ``h`` (see :mod:`atldp.core.geometry`),
and loaded by a resultant load per unit length ``w`` (N/m). For Phase 1 ``w`` is
the conductor self-weight; Phase 2 will pass the wind+weight resultant.

Conventions
-----------
* Horizontal axis ``x`` runs from the lower-numbered support (``x=0``) to the
  other (``x=S``); ``y`` is vertical (up). Support 1 is at ``(0, 0)``, support 2
  at ``(S, h)``.
* The horizontal component of tension ``H`` is constant along the span. The
  total tension at a point is ``T(x) = sqrt(H^2 + V(x)^2)``; it is largest at the
  higher support.
* "Sag" is the maximum *vertical* distance from the straight chord between the
  supports down to the conductor.

Regime selection follows ``docs/theory.md``: the exact catenary is required when
the sag-to-span ratio exceeds 1/8 or the supports are inclined; the parabola is
admissible only for shallow, near-level spans. :func:`solve_span` applies this
rule when ``method="auto"``.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from scipy.integrate import quad

# Above this sag/span ratio, or this support inclination, the parabola is no
# longer trustworthy and the exact catenary must be used (docs/theory.md).
SAG_SPAN_RATIO_LIMIT = 1.0 / 8.0
INCLINATION_LIMIT_RAD = math.radians(15.0)


@dataclass(frozen=True)
class CatenarySolution:
    """Result of solving one span.

    Lengths in metres, tensions in newtons, ``w`` in N/m.
    """

    method: str  # "catenary" or "parabola"
    horizontal_distance: float  # S
    elevation_difference: float  # h
    w: float  # resultant load per unit length, N/m
    H: float  # horizontal tension, N
    conductor_length: float  # arc length of conductor between supports, m
    sag: float  # max vertical sag below the chord, m
    sag_position: float  # x of the max-sag point, m
    low_point_x: float  # x of the lowest point (may be outside [0, S])
    tension_start: float  # total tension at support 1, N
    tension_end: float  # total tension at support 2, N

    @property
    def max_tension(self) -> float:
        return max(self.tension_start, self.tension_end)

    @property
    def catenary_constant(self) -> float:
        """``c = H / w`` (metres) — only meaningful for the exact solution."""
        return self.H / self.w


def _solve_low_point(S: float, h: float, c: float) -> float:
    """Return the abscissa ``a`` of the catenary low point.

    The conductor curve is ``y(x) = c*cosh((x-a)/c) - c*cosh(a/c)`` (so that
    ``y(0)=0``); the second support condition ``y(S)=h`` gives
    ``h = c*(cosh((S-a)/c) - cosh(a/c))``. Using
    ``cosh u - cosh v = 2 sinh((u+v)/2) sinh((u-v)/2)`` this inverts in closed
    form:

        a = S/2 - c * asinh( h / (2 c sinh(S/2c)) )

    ``a`` is ``S/2`` for a level span and moves outside ``[0, S]`` for steep
    inclines, where the lowest point lies beyond a support.
    """
    return S / 2.0 - c * math.asinh(h / (2.0 * c * math.sinh(S / (2.0 * c))))


def solve_catenary(S: float, h: float, w: float, H: float) -> CatenarySolution:
    """Exact inclined catenary for given horizontal tension ``H``."""
    if S <= 0:
        raise ValueError("horizontal distance S must be positive")
    if w <= 0 or H <= 0:
        raise ValueError("w and H must be positive")

    c = H / w
    a = _solve_low_point(S, h, c)
    b = -c * math.cosh(a / c)  # vertical offset so that y(0) = 0

    def y(x: float) -> float:
        return c * math.cosh((x - a) / c) + b

    # Closed-form arc length (independent of a): a useful internal cross-check.
    length = math.sqrt(h * h + (2.0 * c * math.sinh(S / (2.0 * c))) ** 2)

    # Tension: T(x) = H*cosh((x-a)/c) = w*(y(x) - b). Largest at the support
    # farther from the low point.
    t_start = H * math.cosh(a / c)
    t_end = H * math.cosh((S - a) / c)

    # Max vertical sag below the straight chord.
    def neg_gap(x: float) -> float:
        chord = (h / S) * x
        return -(chord - y(x))  # minimise -> most negative gap = max sag

    # The max-sag abscissa is where the conductor slope equals the chord slope.
    # y'(x) = sinh((x-a)/c); set equal to h/S.
    xs = a + c * math.asinh(h / S)
    xs = min(max(xs, 0.0), S)
    sag = (h / S) * xs - y(xs)

    return CatenarySolution(
        method="catenary",
        horizontal_distance=S,
        elevation_difference=h,
        w=w,
        H=H,
        conductor_length=length,
        sag=sag,
        sag_position=xs,
        low_point_x=a,
        tension_start=t_start,
        tension_end=t_end,
    )


def solve_parabola(S: float, h: float, w: float, H: float) -> CatenarySolution:
    """Parabolic approximation for given horizontal tension ``H``.

    The conductor is taken as ``y(x) = (h/S)x - (w/2H)x(S-x)`` (chord minus a
    symmetric downward parabola). Valid only for shallow, near-level spans; the
    arc length is integrated numerically from this approximate shape.
    """
    if S <= 0:
        raise ValueError("horizontal distance S must be positive")
    if w <= 0 or H <= 0:
        raise ValueError("w and H must be positive")

    def yprime(x: float) -> float:
        return h / S - (w / (2.0 * H)) * (S - 2.0 * x)

    length, _ = quad(lambda x: math.sqrt(1.0 + yprime(x) ** 2), 0.0, S)

    # Vertical sag below the chord is (w/2H) x (S-x), max at mid-span.
    sag = w * S * S / (8.0 * H)
    xs = S / 2.0

    # Tension components: H constant; vertical reaction from each half plus the
    # chord slope. Total tension at the supports.
    t_start = math.hypot(H, H * yprime(0.0))
    t_end = math.hypot(H, H * yprime(S))

    # Abscissa of the lowest point: where y'(x) = 0.
    low_x = S / 2.0 - (H * h) / (w * S)

    return CatenarySolution(
        method="parabola",
        horizontal_distance=S,
        elevation_difference=h,
        w=w,
        H=H,
        conductor_length=length,
        sag=sag,
        sag_position=xs,
        low_point_x=low_x,
        tension_start=t_start,
        tension_end=t_end,
    )


def solve_span(S: float, h: float, w: float, H: float, method: str = "auto") -> CatenarySolution:
    """Solve one span for a given horizontal tension ``H``.

    ``method`` is ``"catenary"``, ``"parabola"`` or ``"auto"``. ``"auto"`` uses
    the exact catenary whenever the supports are inclined beyond
    :data:`INCLINATION_LIMIT_RAD` or the sag/span ratio exceeds
    :data:`SAG_SPAN_RATIO_LIMIT`, and the parabola otherwise (docs/theory.md).
    """
    if method == "catenary":
        return solve_catenary(S, h, w, H)
    if method == "parabola":
        return solve_parabola(S, h, w, H)
    if method != "auto":
        raise ValueError(f"unknown method: {method!r}")

    inclined = abs(math.atan2(h, S)) > INCLINATION_LIMIT_RAD
    # Cheap parabolic sag estimate to test the ratio before committing.
    approx_sag = w * S * S / (8.0 * H)
    deep = (approx_sag / S) > SAG_SPAN_RATIO_LIMIT
    if inclined or deep:
        return solve_catenary(S, h, w, H)
    return solve_parabola(S, h, w, H)

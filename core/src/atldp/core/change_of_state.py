"""Change-of-state equation.

Given a conductor at a known reference state (horizontal tension ``H1`` at
temperature ``T1`` under load ``w1``), find the horizontal tension ``H2`` at a
new state ``(T2, w2)`` by enforcing that the *unstrained* conductor length is
conserved between the two states.

The unstrained length ``L0`` satisfies ``L(state) = L0 * (1 + strain(state))``.
Eliminating ``L0 = L1 / (1 + strain1)`` between the two states gives the
equilibrium condition solved here:

    L_geom(H2; S, h, w2) = L1 * (1 + strain2) / (1 + strain1)

where ``L1 = L_geom(H1; S, h, w1)`` is the conductor arc length at the reference
state and ``strain = sigma/E + alpha*(T - T_ref)`` (see
:class:`atldp.core.conductor.Conductor`). The left side decreases with ``H2``
(tighter conductor) while the right side increases (more elastic stretch), so the
root is unique and found by bisection.

Because ``L_geom`` is the *inclined* catenary length, this handles uneven spans
directly — no level-span assumption.
"""

from __future__ import annotations

from dataclasses import dataclass

from scipy.optimize import brentq

from atldp.core.catenary import CatenarySolution, solve_span
from atldp.core.conductor import Conductor


@dataclass(frozen=True)
class StateCase:
    """A weather/loading state for one span (or ruling span)."""

    name: str
    temperature: float  # degC
    w: float  # resultant load per unit length, N/m (>= conductor self-weight)
    creep_strain: float = 0.0  # additional permanent strain in this state


def change_of_state(
    conductor: Conductor,
    S: float,
    h: float,
    reference_H: float,
    reference: StateCase,
    target: StateCase,
    method: str = "auto",
) -> CatenarySolution:
    """Solve for the span state at ``target`` given the ``reference`` state.

    Returns the full :class:`CatenarySolution` of the target state.
    """
    ref_solution = solve_span(S, h, reference.w, reference_H, method=method)
    L1 = ref_solution.conductor_length

    strain1 = conductor.strain(reference_H, reference.temperature, reference.creep_strain)
    # Unstrained reference length (eliminated exactly, so the relation is
    # round-trip invertible).
    L0 = L1 / (1.0 + strain1)

    def residual(H2: float) -> float:
        strain2 = conductor.strain(H2, target.temperature, target.creep_strain)
        target_length = L0 * (1.0 + strain2)
        geom_length = solve_span(S, h, target.w, H2, method=method).conductor_length
        return geom_length - target_length

    # Bracket H2 between a very slack conductor and the rated strength. The
    # lower bound is set so the catenary constant stays c >= S/20 (sag finite,
    # no cosh/sinh overflow); the true root is always far tighter than that.
    lo = max(1.0, target.w * S / 20.0)
    hi = conductor.rated_strength
    H2 = brentq(residual, lo, hi, xtol=1e-6, rtol=1e-12)

    return solve_span(S, h, target.w, H2, method=method)

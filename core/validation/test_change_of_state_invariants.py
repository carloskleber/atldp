"""Golden case: change-of-state physics invariants for Drake ACSR.

Source: the change-of-state equation (CIGRE TB 324; Winkelman 1959). These pin
the *invariants* the equation must satisfy rather than a third party's published
decimals (see validation/README.md for why, and the open TODO to add a numeric
cross-check against OnSag/SSTC).

Conductor: built-in ACSR Drake 26/7, nominal published constants.
"""

import math

from atldp.core.catenary import solve_span
from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import DRAKE_ACSR


def _unstrained_length(conductor, sol, state):
    """Back out the unstrained reference length from a solved state."""
    return sol.conductor_length / (1.0 + conductor.strain(sol.H, state.temperature, state.creep_strain))


def test_unstrained_length_conserved_across_states():
    w = DRAKE_ACSR.unit_weight
    S, h = 400.0, 0.0
    ref = StateCase("ref", 15.0, w)
    ref_sol = solve_span(S, h, w, 31500.0)
    L0_ref = _unstrained_length(DRAKE_ACSR, ref_sol, ref)

    for temp, load in [(-10.0, w), (50.0, w), (75.0, w), (15.0, 2.0 * w)]:
        target = StateCase("t", temp, load)
        sol = change_of_state(DRAKE_ACSR, S, h, 31500.0, ref, target)
        L0 = _unstrained_length(DRAKE_ACSR, sol, target)
        # Same physical conductor: unstrained length is conserved.
        assert math.isclose(L0, L0_ref, rel_tol=1e-6)


def test_round_trip_identity():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 10.0, w)
    hot = StateCase("hot", 80.0, w)
    fwd = change_of_state(DRAKE_ACSR, 450.0, 30.0, 31500.0, ref, hot, method="catenary")
    back = change_of_state(DRAKE_ACSR, 450.0, 30.0, fwd.H, hot, ref, method="catenary")
    assert math.isclose(back.H, 31500.0, rel_tol=1e-5)


def test_tension_within_rated_strength():
    # A sane stringing case must not exceed RTS at the design cold/iced state.
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    cold_iced = StateCase("cold_iced", -10.0, 2.2 * w)
    sol = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, cold_iced)
    assert sol.max_tension < DRAKE_ACSR.rated_strength

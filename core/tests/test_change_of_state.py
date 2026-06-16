import math

from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import DRAKE_ACSR


def test_round_trip_recovers_reference_tension():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("cold", 0.0, w)
    hot = StateCase("hot", 75.0, w)
    forward = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, hot)
    back = change_of_state(DRAKE_ACSR, 400.0, 0.0, forward.H, hot, ref)
    assert math.isclose(back.H, 31500.0, rel_tol=1e-4)


def test_higher_temperature_lowers_tension():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    cold = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, StateCase("c", -5.0, w))
    hot = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, StateCase("h", 80.0, w))
    assert cold.H > 31500.0 > hot.H
    assert cold.sag < hot.sag  # hotter -> longer conductor -> more sag


def test_heavier_load_raises_tension_and_sag():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    bare = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, StateCase("bare", 15.0, w))
    iced = change_of_state(DRAKE_ACSR, 400.0, 0.0, 31500.0, ref, StateCase("iced", 15.0, 2.0 * w))
    assert iced.H > bare.H
    assert iced.sag > bare.sag


def test_change_of_state_handles_inclined_span():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    hot = StateCase("hot", 75.0, w)
    sol = change_of_state(DRAKE_ACSR, 450.0, 70.0, 31500.0, ref, hot, method="catenary")
    assert sol.method == "catenary"
    assert sol.max_tension == sol.tension_end  # higher support carries more
    assert sol.H < 31500.0

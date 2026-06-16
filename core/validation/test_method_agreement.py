"""Golden case: the parabola must agree with the exact catenary in the shallow
regime where it is admissible (ADR-0003: methods that overlap must agree).

For a level span at sag/span well below 1/8, the truncated parabola and the
exact catenary agree to better than 0.2% on sag and max tension, and to ~1e-5 on
length. This both validates the parabola and exercises the regime switch.
"""

import math

import pytest

from atldp.core.catenary import solve_catenary, solve_parabola


@pytest.mark.parametrize("H", [25000.0, 35000.0, 50000.0, 70000.0])
def test_shallow_level_span_agreement(H):
    S, w = 300.0, 15.97
    cat = solve_catenary(S, 0.0, w, H)
    par = solve_parabola(S, 0.0, w, H)

    assert (cat.sag / S) < 1.0 / 8.0  # regime precondition
    assert math.isclose(cat.sag, par.sag, rel_tol=2e-3)
    assert math.isclose(cat.conductor_length, par.conductor_length, rel_tol=1e-5)
    assert math.isclose(cat.max_tension, par.max_tension, rel_tol=2e-3)


def test_disagreement_grows_in_deep_regime():
    # Slack, deep span: the parabola is no longer admissible; the methods part.
    S, w, H = 600.0, 30.0, 12000.0
    cat = solve_catenary(S, 0.0, w, H)
    par = solve_parabola(S, 0.0, w, H)
    assert (cat.sag / S) > 1.0 / 8.0
    assert not math.isclose(cat.sag, par.sag, rel_tol=2e-3)

import math

import pytest
from scipy.integrate import quad

from atldp.core.catenary import solve_catenary, solve_parabola, solve_span


def _numeric_arc_length(sol):
    """Arc length of the *solved* catenary curve, integrated independently."""
    c = sol.catenary_constant
    a = sol.low_point_x
    return quad(lambda x: math.cosh((x - a) / c), 0.0, sol.horizontal_distance)[0]


def test_level_catenary_matches_closed_form():
    S, w, H = 400.0, 15.97, 30000.0
    sol = solve_catenary(S, 0.0, w, H)
    c = H / w
    assert math.isclose(sol.sag, c * (math.cosh(S / (2 * c)) - 1.0), rel_tol=1e-10)
    assert math.isclose(sol.conductor_length, 2 * c * math.sinh(S / (2 * c)), rel_tol=1e-10)
    # Low point is mid-span; sag sits there too.
    assert math.isclose(sol.low_point_x, S / 2, rel_tol=1e-9)
    assert math.isclose(sol.sag_position, S / 2, rel_tol=1e-9)


def test_max_tension_identity_level_span():
    # For a level catenary, max tension = H + w * sag.
    sol = solve_catenary(400.0, 0.0, 15.97, 30000.0)
    assert math.isclose(sol.max_tension, sol.H + sol.w * sol.sag, rel_tol=1e-10)
    assert math.isclose(sol.tension_start, sol.tension_end, rel_tol=1e-10)


def test_closed_form_length_equals_numeric_integral():
    for h in (0.0, 40.0, -60.0):
        sol = solve_catenary(300.0, h, 12.0, 18000.0)
        assert math.isclose(sol.conductor_length, _numeric_arc_length(sol), rel_tol=1e-9)


def test_inclined_max_tension_at_higher_support():
    sol = solve_catenary(350.0, 60.0, 15.97, 28000.0)
    # End is higher (h>0), so it carries the larger tension.
    assert sol.tension_end > sol.tension_start
    assert sol.max_tension == sol.tension_end


def test_inclined_reduces_to_level_when_h_zero():
    cat = solve_catenary(400.0, 0.0, 15.97, 30000.0)
    par = solve_parabola(400.0, 0.0, 15.97, 30000.0)
    # symmetric: equal end tensions, mid-span sag.
    assert math.isclose(cat.tension_start, cat.tension_end, rel_tol=1e-10)
    assert math.isclose(par.tension_start, par.tension_end, rel_tol=1e-10)
    assert math.isclose(cat.sag_position, 200.0, rel_tol=1e-9)


def test_auto_selects_parabola_for_shallow_level_span():
    sol = solve_span(300.0, 0.0, 15.97, 40000.0, method="auto")
    assert sol.method == "parabola"


def test_auto_selects_catenary_for_inclined_span():
    sol = solve_span(300.0, 120.0, 15.97, 40000.0, method="auto")
    assert sol.method == "catenary"


@pytest.mark.parametrize("H", [25000.0, 35000.0, 50000.0])
def test_parabola_agrees_with_catenary_in_shallow_regime(H):
    # Shallow, level span: the two methods must agree closely (ADR-0003).
    S, w = 300.0, 15.97
    cat = solve_catenary(S, 0.0, w, H)
    par = solve_parabola(S, 0.0, w, H)
    assert math.isclose(cat.sag, par.sag, rel_tol=2e-3)
    assert math.isclose(cat.conductor_length, par.conductor_length, rel_tol=1e-5)
    assert math.isclose(cat.max_tension, par.max_tension, rel_tol=2e-3)

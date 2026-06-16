"""Golden case: exact catenary against closed-form identities.

Source: the catenary closed-form relations (Irvine, *Cable Structures*, MIT
Press, 1981; sketched in docs/theory.md). These are independent of the solver's
implementation, so they are a genuine oracle.

Level span: S = 300 m, w = 10 N/m, H = 15000 N -> c = H/w = 1500 m.
    sag    = c (cosh(S/2c) - 1)        = 7.506260... m
    length = 2 c sinh(S/2c)            = 300.500418... m
    T_max  = H cosh(S/2c) = H + w*sag  = 15075.062... N
"""

import math

from atldp.core.catenary import solve_catenary

TOL = 1e-9


def test_level_span_closed_form():
    S, w, H = 300.0, 10.0, 15000.0
    c = H / w
    sol = solve_catenary(S, 0.0, w, H)

    expected_sag = c * (math.cosh(S / (2 * c)) - 1.0)
    expected_len = 2 * c * math.sinh(S / (2 * c))
    expected_tmax = H * math.cosh(S / (2 * c))

    assert math.isclose(sol.sag, expected_sag, rel_tol=TOL)
    assert math.isclose(sol.conductor_length, expected_len, rel_tol=TOL)
    assert math.isclose(sol.max_tension, expected_tmax, rel_tol=TOL)
    # Pinned decimals (regression guard).
    assert round(sol.sag, 4) == 7.5063
    assert round(sol.conductor_length, 4) == 300.5003
    assert round(sol.max_tension, 2) == 15075.06


def test_inclined_span_length_identity():
    # Inclined catenary length identity: L = sqrt(h^2 + (2c sinh(S/2c))^2).
    S, h, w, H = 300.0, 40.0, 10.0, 15000.0
    c = H / w
    sol = solve_catenary(S, h, w, H)
    expected_len = math.sqrt(h * h + (2 * c * math.sinh(S / (2 * c))) ** 2)
    assert math.isclose(sol.conductor_length, expected_len, rel_tol=TOL)
    assert round(sol.conductor_length, 4) == 303.1508

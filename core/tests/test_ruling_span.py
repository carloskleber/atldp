import math

from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import DRAKE_ACSR
from atldp.core.geometry import Span
from atldp.core.ruling_span import Section


def test_ruling_span_of_equal_spans_equals_span_length():
    section = Section(DRAKE_ACSR, [Span.level(350.0) for _ in range(4)])
    assert math.isclose(section.ruling_span, 350.0, rel_tol=1e-12)


def test_ruling_span_between_min_and_max():
    section = Section(DRAKE_ACSR, [Span.level(200.0), Span.level(400.0), Span.level(600.0)])
    rs = section.ruling_span
    assert 200.0 < rs < 600.0
    # RS is weighted toward the longer spans (cubed), so above the mean (400).
    assert rs > 400.0


def test_equal_spans_match_single_span_change_of_state():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    hot = StateCase("hot", 75.0, w)

    section = Section(DRAKE_ACSR, [Span.level(350.0) for _ in range(3)])
    result = section.solve(31500.0, ref, hot)

    single = change_of_state(DRAKE_ACSR, 350.0, 0.0, 31500.0, ref, hot)

    assert math.isclose(result.H, single.H, rel_tol=1e-9)
    for span_sol in result.spans:
        assert math.isclose(span_sol.sag, single.sag, rel_tol=1e-9)
        assert math.isclose(span_sol.H, result.H, rel_tol=1e-12)


def test_common_tension_applied_to_uneven_spans():
    w = DRAKE_ACSR.unit_weight
    ref = StateCase("ref", 15.0, w)
    hot = StateCase("hot", 75.0, w)

    spans = [Span.level(250.0, 0.0), Span.level(500.0, 60.0), Span.level(400.0, -30.0)]
    result = Section(DRAKE_ACSR, spans).solve(31500.0, ref, hot)

    # Every span shares the same horizontal tension; longer spans sag more.
    assert all(math.isclose(s.H, result.H, rel_tol=1e-12) for s in result.spans)
    assert result.spans[1].sag > result.spans[2].sag > result.spans[0].sag

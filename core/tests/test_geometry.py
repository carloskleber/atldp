import math

from atldp.core.geometry import Point3D, Span


def test_level_span_helpers():
    span = Span.level(400.0, 25.0)
    assert span.horizontal_distance == 400.0
    assert span.elevation_difference == 25.0
    assert span.chord_length == math.hypot(400.0, 25.0)


def test_3d_span_projects_to_horizontal_and_elevation():
    # A span running NE and uphill.
    span = Span(Point3D(0, 0, 100), Point3D(300, 400, 150))
    assert math.isclose(span.horizontal_distance, 500.0)  # 3-4-5
    assert math.isclose(span.elevation_difference, 50.0)
    assert math.isclose(span.chord_length, math.sqrt(500.0**2 + 50.0**2))


def test_plan_bearing_is_independent_of_elevation():
    flat = Span(Point3D(0, 0, 0), Point3D(100, 100, 0))
    sloped = Span(Point3D(0, 0, 0), Point3D(100, 100, 80))
    assert math.isclose(flat.plan_bearing_rad, sloped.plan_bearing_rad)
    assert math.isclose(flat.plan_bearing_rad, math.radians(45.0))

# ADR-0002 — Python computational core with separated layers

- Status: Accepted (implemented in Phase 1, 2026-06-15)
- Date: 2026-06-15

## Context

The prototypes so far are Python (Plotly, pandas, requests). The domain is
numerically heavy (root-finding, nonlinear equilibrium, eventually FEM) and
geospatial (DEM rasters, CRS transforms). The team needs fast iteration during the
study phase, a rich scientific/geospatial ecosystem, and the ability to optimize
hot paths later without rewriting everything.

## Decision

Adopt **Python 3.12+** with `numpy`/`scipy` as the default language for the
reference implementation, and structure the code into clearly separated layers:

1. **Core** — pure, headless engineering logic (sag-tension, loads, ampacity).
   No GUI, no I/O, no network, no geospatial dependencies. Fully unit-tested.
2. **Geospatial** — DEM ingestion, CRS handling, profile extraction.
3. **I/O & reporting** — file formats, reports.
4. **Presentation** — CLI now, GUI/web later (see ADR-0006).

Performance-critical kernels may later be moved to a compiled backend
(Cython / Numba / Rust via PyO3) *behind the core's interface*, without changing
callers. This is deferred until a profile proves it necessary.

## Consequences

- Fast iteration and access to `scipy`, `rasterio`, `pyproj`, `pandas`, `plotly`.
- The pure core is portable and testable in isolation, and could be reimplemented
  in another language later if needed.
- Python's numerical performance may require a compiled kernel for large FEM/
  Monte-Carlo runs; the layering keeps that contained.

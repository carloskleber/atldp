# ADR-0005 — Local DEM as geospatial source of truth

- Status: Proposed (tooling refined by [ADR-0013](0013-pure-rust-geospatial-stack.md), 2026-06-15)
- Date: 2026-06-15

> **Note.** The principle below stands. The named tooling (`rasterio`/GDAL,
> `pyproj`) reflected the Python stack; under ADR-0011 the implementation moves to
> a pure-Rust geospatial stack — see ADR-0013.

## Context

The terrain prototype ([tests/terrain/terrain_navigation.py](../../tests/terrain/terrain_navigation.py))
fetches elevations from the **Open-Elevation** public API per point. That is
convenient for a demo but unsuitable for engineering: it depends on a third-party
service, rate limits, has unclear vertical accuracy/datum, and is slow for the
dense profiles a real line needs. A `dem.tif` raster already sits next to the
prototype, hinting at the better path.

## Decision

Use **local DEM rasters** (GeoTIFF) as the geospatial source of truth, read via
`rasterio`/GDAL, with coordinate-reference-system handling via `pyproj`. Ground
profiles along the line are sampled from the DEM. Vertical datum and CRS are
tracked explicitly and recorded with each project.

Online elevation APIs (Open-Elevation and similar) are allowed **only in
throwaway prototypes**, never in the engineering core or reports.

## Consequences

- Reproducible, offline, fast, accuracy-controlled elevation data.
- Requires the user to supply a DEM and requires correct CRS/datum bookkeeping.
- DEM files are large binaries — they must be git-ignored (see ADR-0007).

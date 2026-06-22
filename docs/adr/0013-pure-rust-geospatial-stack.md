# ADR-0013 — Pure-Rust geospatial stack

- Status: Accepted
- Date: 2026-06-15
- Refines tooling of: [ADR-0005](0005-local-dem-as-geospatial-source-of-truth.md)

## Context

ADR-0005 makes **local DEM rasters the geospatial source of truth** (with LiDAR
point clouds later, ADR-0009), and named `rasterio`/GDAL + `pyproj` as the
tooling — appropriate while the stack was Python. ADR-0011 moves the production
app to a native Rust binary with a **< 30 MB** footprint target.

GDAL and PDAL are large C/C++ libraries: hundreds of megabytes of format drivers,
awkward static linking, and a painful Windows build. Linking them would defeat the
footprint target and complicate the cross-platform story.

## Decision

Implement `atldp-geo` on a **pure-Rust geospatial stack**:

- **DEM (GeoTIFF):** the `tiff` / `geotiff` crates for raster + geo-tags.
- **CRS / datum:** `proj4rs` (pure-Rust PROJ) for transforms; CRS and vertical
  datum are tracked explicitly and stored with each project (as ADR-0005 requires).
- **LiDAR LAS/LAZ:** `las` + `laz-rs` for point-cloud ingestion.

A **`gdal` Cargo feature flag** is kept as a documented fallback: if pure-Rust CRS
coverage or format support proves insufficient for a real project, GDAL can be
enabled at the cost of footprint, off by default.

ADR-0005's **principle is unchanged** — local DEM as source of truth, online
elevation APIs only in throwaway prototypes; only the *tooling* moves to Rust.

**Amendment (2026-06-21, ADR-0022):** the no-network posture is relaxed in **one**
place — an **online basemap for the stage-1 endpoint-picking map** (the user picks the
project endpoints on a world map before any DEM exists). This is display-only tiling
for endpoint selection; the geospatial *numerics* — DEM ingest, CRS transforms,
profile sampling — stay fully offline and file-based, and online DEM tile download
remains out of scope.

## Consequences

- Static, self-contained builds that keep the < 30 MB target and cross-compile
  cleanly to Windows.
- Dependence on the maturity of `proj4rs` (CRS coverage) and `laz-rs` (decode
  speed); both are validated early (phases G3/G4), with the GDAL flag as a
  pressure valve.
- Geospatial code is reimplemented from the Python terrain prototype rather than
  reused; bounded, since the prototype is small and throwaway (ADR-0007).

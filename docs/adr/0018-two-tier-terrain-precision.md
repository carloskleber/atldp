# ADR-0018 — Two-tier terrain precision (interpolated ~1 m right-of-way corridor)

- Status: Proposed
- Date: 2026-06-20
- Refines: [ADR-0005](0005-local-dem-as-geospatial-source-of-truth.md),
  [ADR-0013](0013-pure-rust-geospatial-stack.md)

## Context

Clearance verification is only as trustworthy as the ground profile under the wires.
Public DEMs are coarse for this purpose — SRTM is ~30 m posting, and the current
`atldp_geo::profile::extract_profile` samples the DEM along the route at N points,
reading the **nearest cell** (`dem.elevation_at`); `Project::ground_at` then linearly
interpolates between those stored samples. Over a wide reconnaissance map that crude
profile is fine, but once the route is fixed the **right-of-way profile must be
precise — on the order of 1 m** — because a metre of ground error is a metre of
clearance error directly against the normative minimum.

Two different fidelities are therefore wanted at two stages: **coarse over the wide
area** for routing, and **fine along the committed corridor** for verification.

## Decision

Adopt a **two-tier terrain model**:

- **Wide-area tier (routing):** the coarse public DEM as today — cheap to load and
  pan over the whole study area (ADR-0005's local-DEM principle is unchanged).
- **Right-of-way tier (verification):** once the route is committed, **densify the
  corridor** to a configurable fine spacing (target ~1 m) and sample the DEM with
  **sub-cell interpolation** (bilinear, with a bicubic option) rather than
  nearest-cell, so the profile is smooth and as accurate as the source allows. Where
  the public DEM cannot support 1 m truth, interpolation makes the profile *continuous
  and consistent* and leaves a **clear hook to substitute a better source** — surveyed
  **LiDAR** (already planned, ADR-0009/0013 G4) or imported **contour/total-station
  survey** — for the corridor without changing downstream consumers.

Concretely this refines `atldp_geo`: `extract_profile` (and `dem.elevation_at`) gain
sub-cell interpolation and a configurable corridor spacing, with explicit
**no-data/NaN handling** so a void cell is flagged, not silently zero-filled. The
stored `Project.ground_profile` becomes the fine corridor profile.

## Consequences

- Clearance checks, reports, and sheets run against a profile whose precision matches
  the engineering tolerance, removing a class of false pass/fail from DEM quantisation.
- Interpolation cannot *create* accuracy the source lacks — it bounds the error and
  documents it; the LiDAR/survey hook is where real 1 m truth ultimately comes from.
- Pure-Rust stack is preserved (ADR-0013): bilinear/bicubic sampling and densification
  are local arithmetic, no new heavy dependency; the `gdal` fallback flag remains.
- A finer corridor profile means more stored samples — bounded, since it covers only
  the route corridor, not the whole tile.

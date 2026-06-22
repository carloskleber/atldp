# ADR-0022 — SRTM area selection and the plan-view route editor

- Status: Proposed
- Date: 2026-06-21
- Builds on: [ADR-0012](0012-desktop-gui-wgpu-egui.md) (GUI),
  [ADR-0013](0013-pure-rust-geospatial-stack.md) (geospatial stack),
  [ADR-0018](0018-two-tier-terrain-precision.md) (two-tier terrain),
  [ADR-0019](0019-route-poi-model-and-mandatory-angle-structures.md) (route / POI model)

## Context

ADR-0019 made the **route the primary plan artefact**: an ordered list of kinded
[`Poi`](../../crates/atldp-model/src/lib.rs) vertices from which the 2-D ground
profile is *derived* (`geo::extract_profile_polyline`). The model is in place, but the
app cannot yet author it. Two gaps remain, both explicitly deferred at delivery:

- **The terrain area is not chosen.** `atldp-app` loads one whole HGT tile from
  `ATLDP_TERRAIN` (or a baked-in default path) and uses it as-is
  ([`TerrainData::load`](../../crates/atldp-app/src/main.rs)). There is no way, in the
  app, to pick *which* SRTM data to import or *what part* of it the project covers — a
  1°×1° tile is ~110 km on a side, far larger than most lines.
- **The route is hard-coded, not drawn.** The app synthesises a single straight
  corridor across the tile interior (`lat_min + 0.1 … lat_max − 0.1`) and wraps it in a
  trivial terminal→terminal `Route`. ADR-0019's own delivery note calls the "plan-view
  POI editor (angle points etc.)" *later GUI work*. So the very first step of the real
  workflow — **define the route on the terrain** — is the one step the GUI omits.

The engineer's actual first move (workflow stages 1–4, ADR-0009/0019) is: pick the
**project endpoints** on an overall basemap, import terrain *for the area those
endpoints span*, then **pick the intermediate POIs on a plan (top-down) view** — angle
points, crossings — and let the profile fall out. Today they get a fixed diagonal
instead. This ADR closes that, realising the deferred authoring GUI on top of the
existing route model and DEM ingest, and giving ADR-0018's "commit the route, then
densify the corridor" step a route to act on.

## Decision

Add an **import-area step** and a **plan-view route editor** to `atldp-app`, with the
supporting geometry in `atldp-geo`/`atldp-model`. No new numerics; the route still feeds
the existing `extract_profile_polyline` → spotting → analysis pipeline.

- **Choose the SRTM area.** Replace the env-var-only tile load with an explicit import
  seeded by the project endpoints (workflow stage 1): the **area of interest** — a
  rectangular lat/lon window — defaults to the endpoints' bounding box plus a routing
  buffer, and is adjustable on the basemap. From that window the app resolves which HGT
  tiles are needed; the user selects them (file picker, falling back to `ATLDP_TERRAIN`
  for headless/dev use). The DEM is cropped/mosaicked to the window and becomes the
  project's working extent. `atldp-geo` gains a tile **mosaic** (stitch adjacent `Dem`s
  sharing an edge) and a `Dem::crop(bounds)` so a route may cross a tile seam and so the
  working set stays small. `TerrainRef` grows from a single tile to the **set of source
  tiles + the chosen bounds** (schema bump, below).
- **A plan (top-down) view.** Add a **plan-view tab** beside the 3-D and profile views
  (ADR-0012's dock): the DEM rendered as a shaded/hypsometric raster or contour image in
  geographic/local-plane coordinates, with pan/zoom. This is the map the route is drawn
  on — the line-frame profile (ADR-0019) and the structure-frame elevation (ADR-0020)
  gain their missing **plan-frame** sibling.
- **Draw the route by placing POIs.** In a route-edit mode, clicking the plan view
  appends/inserts a `Poi`; a per-vertex kind selector sets `Terminal` / `Angle` /
  `Crossing` / `Obstacle` / `Constraint`. Dragging a vertex moves it; the **deviation
  angle of an `Angle` POI is computed from the adjacent legs**, not typed. On every edit
  the app re-runs `extract_profile_polyline` over the route to recompute
  `ground_profile`, re-stations the POIs, and **pins the obligatory structures**
  (`Route::pinned_structures`, ADR-0019) — so the profile and the editable tower table
  (ADR-0019) update live, exactly as a manual function change does today.
- **One route, one derived profile.** The hard-coded diagonal is removed; an empty
  project starts with no route, and the profile/3-D corridor are whatever the drawn route
  yields. The straight-corridor path stays only as the v3→v4 migration's behaviour for
  legacy projects (a two-POI route from the stored endpoints).

The `.atldp` `SCHEMA_VERSION` bumps to **4**: `TerrainRef` carries the tile set and the
chosen area bounds (was a single tile). The migration seam carries a v3 project forward
by treating its single tile as a one-element set with bounds = the full tile, so existing
projects reproduce their results (round-trip tested, per the ADR-0015/0019 pattern).

## Consequences

- The app finally starts where the workflow does: **import area → draw route → derived
  profile → spotting**, instead of editing towers on a fixed diagonal. The route model
  (ADR-0019) gets the authoring surface it was built for.
- The plan view is the natural home for the future plan-&-profile *sheet*'s top half and
  for the automatic-spotting search domain (it shows the obligatory structures the
  optimiser must keep fixed). It is also where ADR-0018's corridor densification is
  triggered ("commit this route, refine its corridor to ~1 m").
- `atldp-geo` gains tile **mosaic + crop**; `proj4rs`/CRS coverage (ADR-0013) is
  exercised for the first time on a user-chosen, possibly seam-crossing window — a place
  the documented `gdal` fallback may surface.
- A schema migration (v3 → v4) must be written and round-trip tested; `TerrainRef`
  consumers (`build_project`/`apply_project`) move from one tile to a tile set + bounds.
- Scope guard: this ADR is **area selection + route authoring only**. Multi-resolution
  corridor densification stays ADR-0018; GPX/KML route import and online DEM tile
  download remain out of scope (DEM ingest stays a file-based, offline-first import).
  An **online basemap is in scope** for the stage-1 endpoint-picking map only — the one
  documented relaxation of ADR-0013's no-network posture (amended there); the terrain
  numerics stay fully offline.

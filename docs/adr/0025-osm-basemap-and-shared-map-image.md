# ADR-0025 — OpenStreetMap basemap and a shared map raster for the plan & 3D views

- Status: Accepted, **fully delivered** (plan-view basemap + camera, G10d 2026-06-23;
  the 3-D textured-terrain drape — a new wgpu surface pipeline — landed G10e, 2026-06-24;
  north-up orientation, 3-D pan and work-area selection, G10f 2026-06-24)
- Date: 2026-06-23
- Builds on: [ADR-0012](0012-desktop-gui-wgpu-egui.md) (GUI / rendering),
  [ADR-0013](0013-pure-rust-geospatial-stack.md) (pure-Rust, offline-first geospatial
  stack), [ADR-0018](0018-two-tier-terrain-precision.md) (two-tier terrain),
  [ADR-0022](0022-srtm-area-selection-and-plan-view-route-editor.md) (plan-view editor
  + the online-basemap relaxation)

## Context

The G10c plan view (ADR-0022) draws the working DEM as **hypsometric shading** and lets
the engineer draw the route on it. That relief is abstract: a route is sited against
roads, rivers, settlements, parcels and existing lines — the content of an
**OpenStreetMap basemap**, not bare elevation. ADR-0022 already admitted that "an online
basemap is in scope for the stage-1 endpoint-picking map … the one documented relaxation
of ADR-0013's no-network posture."

Two related gaps:

- **The plan view has no recognizable map.** It needs a real basemap under the route,
  and — to be usable as a map — **pan, zoom and a scale bar** (today it re-fits the
  whole window every frame).
- **The 3-D view does not reflect the plan.** It shows a bare wireframe/relief; the
  engineer cannot see the same map draped on the terrain, so the plan and 3-D views
  read as unrelated. They should show **one shared image**.

ADR-0013 keeps the terrain **numerics** offline and reproducible (local DEM as the
source of truth). A basemap is *imagery only* — it changes nothing in the elevation,
profile, sag-tension or clearance results — so it can be an optional, cached online
layer without touching that posture.

## Decision

Add an **optional, cached OSM raster basemap** and make the plan and 3-D views share one
georeferenced **map image**.

- **Fetch & cache tiles.** For the working bounds (`TerrainRef`), compute the slippy-map
  tile set (`z/x/y`, zoom chosen from the window's on-screen size), fetch the PNG tiles
  over HTTPS on a **background thread** (a pure-Rust blocking client — `ureq` + rustls —
  decoded with `image`), and **cache** them on disk keyed by `z/x/y`. Composite the
  tiles into a single RGBA **map image** clipped to the working bounds, carrying its own
  geo bounds.
- **Plan view: basemap + camera.** Render the map image as the plan background, under
  the route; keep the **hypsometric shading as the offline fallback** when tiles are
  missing. Give the plan view a **persistent pan/zoom camera** (centre in local-plane
  metres + pixels-per-metre, initialised to fit) and an adaptive **scale bar**. Route
  markers and the raster transform through the one camera.
- **3-D view: the same drape.** Texture the 3-D terrain mesh with that map image —
  `atldp-render::terrain_mesh` gains per-vertex **UVs** (each grid vertex's geographic
  position → image UV) and a sampled texture — so the 3-D view reflects the plan image
  and the two stay registered through the shared `LocalPlane`.
- **Network posture (ADR-0013 amended, narrowly).** Imagery only; **all terrain numerics
  and elevations stay offline and reproducible**. The app runs fully **without network**
  — it falls back to hypsometric shading with no error. Requests are low-volume (one
  window's tiles, then cache), send a descriptive **User-Agent**, and respect the tile
  provider's usage policy; the tile source is configurable so a self-hosted or
  permissively-licensed provider can be substituted.

`TerrainData` grows a `basemap: Option<MapImage>` (RGBA + geo bounds); the basemap is a
**cache, not project state** — the `TerrainRef` tile set + bounds already pin what to
re-fetch, so the `.atldp` schema is unchanged. Any new preference (e.g. chosen provider)
rides a `#[serde(default)]` field, no schema bump.

## Consequences

- **New dependencies:** an HTTP client (`ureq`, rustls — pure-Rust, no OpenSSL) and
  `image` (PNG decode); optionally `resvg` if the sheet preview (G10d) rasterizes SVG.
  All weighed against the **< 30 MB** binary gate (ADR-0011); each is modest but the sum
  is watched.
- **Progressive, non-blocking:** tiles arrive on a worker thread; the plan/3-D views draw
  whatever has loaded and refine as more land. Offline degrades to shading, never an
  error dialog.
- **Registration:** plan transform and terrain UVs both derive from the working
  `LocalPlane`, so the route, the basemap and the 3-D drape line up by construction.
- **Depth buffer (3-D drape, G10e).** The drape is a *filled, opaque* surface, so the
  3-D pass gained a shared depth buffer (`atldp_render::DEPTH_FORMAT`): the drape writes
  and tests depth (`Less`) so the terrain occludes itself correctly. The existing line
  pipelines (wireframe, spotting, catenary) and egui itself stay depth-`Always`/no-write,
  so enabling the buffer leaves their painter-order behaviour unchanged — when no basemap
  is loaded the drape is simply not drawn and the 3-D view is identical to before. The
  texture is uploaded **once** when a basemap loads (`TerrainDrapeResources::set_texture`),
  not per frame; no new dependencies (the binary stays ~18 MB stripped, < 30 MB gate).
- **Orientation (3-D drape, G10f).** With the right-handed orbit camera and world axes
  (x=east, z=north), a south-facing view puts north into the distance but flips east↔west.
  The 3-D projection therefore **mirrors clip-space X** (and the orbit/pan horizontal input
  signs follow), so the drape reads **north-up / east-right** like the plan view. The
  basemap texture is glued to the geometry, so it mirrors with it and stays registered; the
  pipelines use no back-face culling, so the winding flip is harmless. The 3-D view also
  gained **right/middle-drag pan**, and the basemap can be fetched for any **work area**
  cropped via the new *Set work area…* dialog (smaller window ⇒ finer grid + higher zoom).
- Realizes ADR-0022's deferred **online basemap** relaxation; the future stage-1
  endpoint-picking map (ADR-0022) reuses this same tile layer.
- **Operational:** tile-usage-policy compliance and provider choice are documented
  concerns, not code guarantees; abusive use is a deployment risk mitigated by caching,
  low volume, and a configurable source.
- Scope guard: **raster basemap + shared drape only.** Vector basemaps, offline tile
  bundles, and route import from map features stay out of scope.

# ATLDP — Implementation Plan

This translates the project's goals (see [README](../README.md)) into a phased,
verifiable plan, and points to the architectural decisions that back it (see
[adr/](adr/)). It is a living document: phases **G0–G10f are delivered** — the workflow
was **reprioritised on 2026-06-21** (ADR-0019–0021) around the real design workflow:
an explicit route/POI model with obligatory angle structures (**G9**, ADR-0019), the
tower-elevation view with real attachment geometry (**G10**, ADR-0020), and the
**plan-view route editor** that finally lets the engineer *draw* the route on the
terrain — with a tile-set/working-bounds terrain model and the angle correction
(**G10c**, ADR-0022/0023). That authoring surface was then made usable and recognizable —
pan/zoom, a shared **OpenStreetMap basemap** for the plan and 3-D views, interactive
drafting dialogs, route-editor safety fixes, a north-up basemap drape on the 3-D terrain,
a work-area selector and a menu-driven UI (**G10d–G10f**, ADR-0025). The immediate
next step brings the FEM section
solver forward for uneven spans (**G11**, ADR-0021), ahead of the previously-planned
standards load cases, ~1 m terrain, and stringing tables, which move to **G12–G14**
(ADR-0017–0018), and the **production plan-&-profile sheets** that turn the G6 plot into
paginated A1/A0 drawings (**G15**, ADR-0024).

> **Substrate (2026-06-15).** The product is a desktop CAD application — interactive
> 2D/3D views (terrain, route, towers, conductors, LiDAR point clouds), high
> rendering performance, and a small optimized binary (< 30 MB), Linux-first then
> Windows. The production stack is a **native Rust** Cargo workspace (ADR-0011) with
> a winit + wgpu + egui shell (ADR-0012) and a pure-Rust geospatial stack
> (ADR-0013). The staged pipeline and domain modules below are unchanged from the
> original Python design; only the implementation language moved. The Python `core/`
> was a transitional port scaffold — self-consistent but never an independent oracle
> — and has been **removed** (ADR-0014); its golden cases and cross-check fixtures
> live on in the Rust test tree (`crates/atldp-core/tests/`). The **de facto external
> numerical reference is OTLS-Models** (`third_party/Models`, ADR-0008).

## Guiding principles

1. **Validate before building.** Every numerical model must reproduce a published or
   independently-computed reference before it is trusted.
2. **Separate the core from everything else.** A pure, headless, well-tested
   computational engine — no GUI, no I/O, no geospatial dependencies in it.
3. **Standards are first-class.** Criteria from IEC / IEEE / CIGRE / ABNT are encoded
   explicitly and cited, not hard-coded as magic numbers (ADR-0004).
4. **Throwaway prototypes are fine.** `tests/` stays a sandbox; promotion to the core
   requires tests and validation.

## The design pipeline (product workflow)

ATLDP executes a transmission-line project as a staged pipeline; each stage consumes
the previous stage's output and adds to a shared project model (ADR-0009). This is
the *runtime* order of a project — distinct from the *build* order of the phases.

Stages 1–3 are order-tolerant **setup**; stages 4–9 are the **computational pipeline**
(sequence reordered 2026-06-21, ADR-0009/0022).

| # | Stage | Input → Output | Notes |
| --- | --- | --- | --- |
| 1 | **Define the project** *(setup)* | — → endpoints, capacity, voltage | initial/final points picked on an overall basemap (ADR-0013/0022); frame the area downstream |
| 2 | **Standards & criteria** *(setup)* | voltage/region → load criteria | applicable standards fix load cases, wind/ice, RoW widths, clearances (ADR-0017) |
| 3 | **Tower families** *(setup)* | voltage/circuits → candidate families | structures selected from a library/database by their application chart (ADR-0016) |
| 4 | **Terrain model** | endpoints → 3D ground+feature model | over the area the endpoints span; coarse wide-area tier + interpolated ~1 m right-of-way corridor (ADR-0018, G13) |
| 5 | **Line route** | terrain + **POIs** → georeferenced route, **profile derived from it** | route is an explicit POI polyline (angle points, crossings, obstacles, constraints); the 2-D profile is *sampled along the route*, not an independent input (ADR-0019, G9) |
| 6 | **Tower placement (spotting)** | route + families → spotted, typed structures | **angle (deflection) POIs oblige a structure** (ADR-0019, G9); suspension vs anchor typing → **tension sections** (ADR-0015), angle a deflection-derived property gated by the chart (ADR-0023); structures from **families**, each a **drawn shape with real attachment points** (ADR-0016/0020, G10); **editable after spotting**; manual first, auto later |
| 7 | **Sag-tension** | sections + **wire set** + load cases → sags, tensions, clearances | per-section solve — analytic **ruling span** where its assumptions hold, **FEM for uneven / multi-attachment spans** (ADR-0021, G11); **every wire** (3 phases/circuit + shield) at its own tension; swing/blowout; wire-ground & phase-phase clearances |
| 8 | **Structure modeling** | spotted structures + spans + load cases → load checks | wind/weight/longitudinal span loads vs the family's rating; **IEC 60826** cases (ADR-0017) |
| 9 | **Plan & profile drafting** | full project model → sheets & reports | **paginated A1/A0 plan-&-profile sheets** with structure/section/span/wire labels, title & notes blocks (ADR-0024, G15); calculation reports; field **stringing table** (G14) |

## Domain decomposition

The pipeline rests on these largely independent computational modules:

| Module | Problem | Serves stage | Key references |
| --- | --- | --- | --- |
| **Conductor model** | thermal/elastic constitutive behavior, stress-strain (initial/final, creep) | 7 | Aluminum Association, CIGRE TB 324 |
| **Sag-tension (single span)** | catenary / parabola, level & inclined supports | 7 | Irvine & Caughey 1974; [theory.md](theory.md) |
| **Change-of-state** | equilibrium across temperature/load/creep states | 7 | CIGRE TB 601 |
| **Ruling span & tension sections** | multi-span section between anchors; per-section traction | 6, 7 | IEEE/CIGRE practice; ADR-0015 |
| **Multi-wire set** | phases (3·circuits) + shield wires, each own tension/load; lowest-wire clearance | 7 | ADR-0015 |
| **Uneven / multi-attachment spans (static FEM)** | cable-element section solve when ruling-span equal-tension assumptions break (different attachment points per structure) — **brought forward to G11** | 7 | Bertrand 2020; Sugiyama 2003; ADR-0021 |
| **Dynamic spans / vibration (FEM + ROM)** | aeolian vibration, galloping, gust response — later research track behind the static core | 7 | Bertrand 2022; Irvine 1974; ADR-0003 |
| **Weather, loads & load cases** | wind/ice cases, swing/blowout, IEC 60826 case set (extreme wind, construction, broken wire) | 2, 7, 8 | IEC 60826; ABNT NBR 5422; ADR-0017 |
| **Thermal rating** | ampacity vs. conductor temperature | 7 | IEEE 738; CIGRE TB 601 |
| **Geospatial** | DEM/LiDAR ingestion; coarse map + interpolated ~1 m corridor profile; CRS | 4, 5 | tiff/geotiff, proj4rs, las/laz; ADR-0018 |
| **Routing & POIs** | explicit POI polyline over terrain; profile **derived** from it; angle POIs pin structures | 5, 6 | ADR-0019 |
| **Structure library** | families, effective height, application (wind/weight-span) chart, **attachment/silhouette geometry** for the tower-elevation view | 3, 6, 8 | ADR-0016/0020 |
| **Spotting** | structure placement & family selection; cost-minimizing optimization | 6 | ADR-0010/0016; NBR 5422 clearances |
| **Clearances** | ground/RoW and phase-phase clearance checks | 6, 7 | NBR 5422 |
| **Structure model** | wind/weight/longitudinal span loads, lattice & guyed-tower checks | 8 | IEC 60826; NBR 5422 |
| **Drafting, reporting & stringing table** | **paginated A1/A0 plan & profile sheets** (configurable structure/section/span/wire labels, title/notes/legend blocks), calculation reports, field stringing chart | 9 | IEEE 524; ADR-0009/0024 |

## Delivered — native build track (Phases G0–G10c, complete)

The production app is built natively in Rust (ADR-0011/0012/0013). The sag-tension
core (stage 7) was built first — highest risk, highest value, and the validation
oracle for everything else — with the surrounding pipeline stages wrapped around it.
Full
blow-by-blow validation lineage lives in
[`crates/atldp-core/tests/ORACLES.md`](../crates/atldp-core/tests/ORACLES.md); the
summary:

| Phase | Delivered | Date |
| --- | --- | --- |
| **G0** | ADRs 0011–0014 + Cargo workspace skeleton (`crates/atldp-{core,geo,model,render,app,cli}`), optimized release profile, Linux+Windows CI; fmt/clippy/test green. | 2026-06-15 |
| **G1** | Dependency-free `atldp-core` (stage 7): geometry, inclined catenary + parabola, change-of-state, ruling span, conductor; thin `atldp` CLI. Cross-checked against the Python oracle (≤1e-7 over 882 cases) **and** the independent **OTLS-Models** reference (`golden_otls_models`). | 2026-06-16 |
| **G1b** | Independent nonlinear bimetallic stress-strain/creep conductor model (`conductor::StressStrainModel`, Aluminum Assoc. / CIGRE TB 324). Reproduces OTLS reloader tensions to ≤0.2 % (`golden_otls_change_of_state`) — closes the change-of-state validation gap. | 2026-06-16 |
| **G2** | Render foundation (ADR-0012): winit + egui docked shell, wgpu 3D orbit viewport with live catenary, 2D ortho viewport. **12 MB stripped** (< 30 MB gate ✅). | 2026-06-16 |
| **G3** | Terrain & route (stages 4–5): `atldp-geo` DEM ingest + CRS + ground profile (pure-Rust); 3D terrain wireframe, 2D profile, camera auto-fit. | 2026-06-16 |
| **G4** | LiDAR point-cloud engine (stage 4, advanced) — **⏸ postponed** until a surveyed dataset is available to validate against. | — |
| **G5** | Manual spotting + in-GUI sag-tension (stages 6–7): click-to-place towers, live catenary, colour-coded ground-clearance check, tower/span tables. | 2026-06-16 |
| **G6** | Structure loads (`core::structure`: signed wind/weight spans + conductor loads), the open versioned-JSON **`.atldp`** format (`atldp-model`), and drafting (Markdown `report` + SVG plan-&-profile `sheet`) from one shared `analysis` pass. | 2026-06-20 |
| **G7** | Multi-**wire** set (`model::Wire`: phase + shield wires, per-wire conductor/offset/tension) and **tension sections** from structure-function typing (`tension_sections`, per-section ruling span). `analysis` fans out over (wire × section); `report`/`sheet` plot **every wire** with a **lowest-wire** clearance mode. `.atldp` `SCHEMA_VERSION` → **2** with a round-trip-tested v1→v2 migration. | 2026-06-21 |
| **G8** | **Structure-family library** (`model::StructureFamily`: function, height range, **application chart** = piecewise wind-span × weight-span × angle envelope); a `Tower` references a family + height with per-structure overrides. `analysis` checks each placement's loads against its chart and flags violations — the constraint oracle the Phase-5 optimizer will select against. | 2026-06-21 |
| **G9** | **Route / POI model & mandatory angle structures** (ADR-0019): an explicit `model::Route` of kinded `Poi` vertices (terminal / angle / crossing / obstacle / constraint), each with the structure function it obliges (`PoiKind::pinned_function`); the ground profile is **derived** from the route polyline (`geo::extract_profile_polyline`, continuous stationing across bends). `Tower` gains an `origin_poi` link; the GUI right panel is now an **editable tower table** (function/family/height per row) that **re-partitions tension sections live**. `.atldp` `SCHEMA_VERSION` → **3** with a round-trip-tested v2→v3 migration (synthesise a terminal→terminal route from the legacy profile endpoints). | 2026-06-21 |
| **G10** | **Structure geometry & tower-elevation view** (ADR-0020): `StructureFamily` gains a drawable `AttachmentGeometry` — per-conductor attachment points (role, lateral/vertical offset) plus a body/crossarm silhouette — the single source of truth for where each wire attaches (`Project::reconcile_wire_offsets`). A new **tower-elevation tab** in `atldp-app` draws the selected structure's silhouette with every attachment point labelled by wire and its resulting conductor elevation. "Choosing a structure" becomes inspecting a real shape. | 2026-06-21 |
| **G10d (basemap)** | **Plan-view camera & OSM basemap** (ADR-0025). The plan view gets a persistent **pan/zoom camera** (centre + pixels-per-metre, fit on load; right-drag pan, scroll zoom) and an adaptive **scale bar**, replacing the fit-every-frame transform. A **user-triggered OpenStreetMap basemap** (toolbar 🌍) fetches/caches slippy tiles on a **background thread** (`ureq` + rustls, `image` PNG decode, disk cache, identifying User-Agent), composites them into one georeferenced image, and **textures the plan DEM mesh** (per-vertex UV; Web-Mercator-correct V) — with **hypsometric shading as the offline fallback** (a failed/empty fetch degrades silently). Imagery only; terrain numerics stay offline (ADR-0013). No `.atldp` change (the basemap is a cache). **The 3-D drape remains** (the 3-D terrain is still a wireframe — see the remainder below). | 2026-06-23 |
| **G10d (fixes)** | **Route-editor correctness & interactive drafting** (ADR-0006/0022 follow-ups). The 2-D profile draws as the **ground line only** (the axis-anchored `convex_polygon` fan is gone; the SVG sheet already used a true polygon). Entering **route-edit mode with towers present raises a confirmation** that clears the now-orphaned spotting before re-stationing. Report/sheet exports stop being silent: each opens an **in-app preview** (report as text; sheet rasterized via `resvg`) with a **native `rfd` "save as…" dialog**; project **save/load** use native dialogs too (`ATLDP_*` env paths stay the headless fallback). The plan-view pan/zoom, OSM basemap, and 3-D drape remain (ADR-0025, below). | 2026-06-23 |
| **G10e** | **3-D basemap drape** (ADR-0025, the G10d remainder). The 3-D terrain — until now a `LINE_LIST` wireframe only — gains an **additive textured-surface pipeline** (`atldp-render::terrain_drape`): a filled triangle mesh of the working grid with per-vertex **UVs** (the same Web-Mercator-correct lon/lat→UV map the plan view uses, via the shared `LocalPlane`), sampling the composited OSM `MapImage` the plan view already produces — so the plan and 3-D views show **one shared raster**, registered by construction. The 3-D pass gains a **shared depth buffer** (`DEPTH_FORMAT`) so the opaque drape occludes itself (`Less`); the wireframe/spotting/catenary line pipelines and egui stay depth-`Always`/no-write, so the view is **byte-identical when no basemap is loaded** (the drape is drawn *under* the wireframe and only when a basemap texture has been uploaded — once, on load, not per frame). No new dependencies; release binary **18 MB stripped** (< 30 MB gate ✅). No `.atldp` change (the basemap stays a cache). | 2026-06-24 |
| **G10f** | **3-D view & authoring UX** (ADR-0025/0022 follow-ups). **3-D orientation:** the default orbit looks from the **south** and the 3-D projection mirrors clip-X, so the draped basemap reads **north-up / east-right** — matching the plan view — instead of north-toward-the-viewer (which read as "inverted"); the 3-D view also gains **right/middle-drag pan** (the camera was rotate-and-zoom only). **Area selection (#3):** a **Terrain ▸ Set work area…** dialog crops the working window to chosen lat/lon bounds (mosaicked from the current tile set, route preserved) — a **smaller area yields finer grid spacing and a higher-zoom basemap** — and **Terrain ▸ Load terrain tile…** opens an SRTM `.hgt` for another region. **Toolbar → menu (#4):** all named commands move to a **menu bar** (File / Terrain / Tools / Parameters); the toolbar keeps only **icon toggles** (spot, route, basemap) and live status (extent, tower count, worst clearance). No `.atldp` change. | 2026-06-24 |
| **G10c** | **Plan-view route editor & terrain set** (ADR-0022/0023). `atldp-geo` gains tile **`mosaic`** (stitch grid-aligned tiles across a seam) + **`Dem::crop`** (cell-aligned window). `TerrainRef` becomes a **set of source tiles + chosen working `AreaBounds`** (was a single tile); `.atldp` `SCHEMA_VERSION` → **4** with a round-trip-tested v3→v4 migration (single tile → one-element set over the full tile; any stored `Angle` function → running-angle `Suspension` keeping its deflection). A new **plan-view tab** renders the cropped DEM as a hypsometric raster with the **editable route** drawn on it: in route-edit mode, clicking empty map inserts an angle `Poi` on the nearest leg, dragging moves a vertex, the selected vertex's kind is editable, and every edit re-runs `extract_profile_polyline` (re-stationing, ground-sampling, deflection angles) so the profile and tower table update live. The hard-coded diagonal is gone — `TerrainData` carries the real route and project save/load resolves the tile set (mosaic + crop). **ADR-0023:** `StructureFunction` drops `Angle` (now `Suspension`/`Anchor`); "angle" is a deflection-derived property of the location (`Tower::is_angle`, gated by the family chart's `max_line_angle_deg`); `PoiKind::pinned_function` splits into `requires_structure` + an optional fixed function (only terminals fix `Anchor`). | 2026-06-23 |

These phases deliver a working **multi-wire, multi-section** line tool with a
structure-family library, an explicit route the profile derives from, structures
seen as real shapes, a **plan view the route is drawn on** (G10c), and a shared **OSM
basemap** under both the plan and the 3-D terrain (G10d–G10e) — ahead of the FEM section
solver (G11) and the standards/terrain/field-output work that follows.

## Delivered — the 3-D basemap drape (Phase G10e) — ADR-0025

G10c made the route *drawable*; G10d/G10e make the authoring surface **usable and
recognizable**. The G10d work landed 2026-06-23 (authoring fixes — profile-line,
route-edit confirmation, interactive `rfd` drafting — and the **plan-view camera + OSM
basemap**: pan/zoom, scale bar, cached tiles textured onto the plan DEM with hypsometric
fallback). The last piece — **the 3-D view reflecting the same basemap** — landed as
**G10e** (2026-06-24, see the delivered row above):

- **3-D reflects the plan image.** The same composited `MapImage` now **textures the 3-D
  terrain**. The 3-D terrain was a `LINE_LIST` **wireframe** (`atldp-render::terrain_mesh`),
  so the drape is a **new textured-surface pipeline** (`atldp-render::terrain_drape`) — a
  filled triangle mesh with per-vertex UVs (reusing the plan view's Web-Mercator-correct
  lon/lat→UV map via the shared `LocalPlane`), a texture + sampler bind group, and a WGSL
  shader — added *additively* under the wireframe, and a **shared depth buffer** so the
  opaque surface occludes itself while the line pipelines and egui stay depth-`Always`/
  no-write (the 3-D view is unchanged when no basemap is loaded). The texture uploads once
  on basemap load, not per frame; no `.atldp` change (the basemap stays a cache).
- **North-up orientation (G10f).** The default orbit looks from the south and the 3-D
  projection mirrors clip-X, so the drape reads **north-up / east-right** like the plan
  (with the right-handed camera and world axes x=east/z=north, a south-facing view puts
  north into the distance but flips east↔west; the mirror restores it — the basemap is
  glued to the geometry, so it stays consistent). The 3-D view also gained **right-drag
  pan** and a **menu-bar UI** with a **work-area selector** (crop for finer detail) — see
  the G10f row.

### Obligatory-structure signalling — route → spotting (ADR-0019/0023)

The route already encodes which stations **oblige** a structure: each **terminal pins
an anchor** (the line begins and ends anchored), and a **deflection (`Angle`) POI
requires a tower** (a conductor cannot turn in mid-span), the suspension-vs-anchor
choice gated by the family chart's `max_line_angle_deg` (ADR-0023). G10d surfaces these
obligations rather than auto-spotting them:

- The plan and profile **flag every obligatory POI with no structure** — a terminal
  without an anchor, a deflection without a tower — with a visible "needs a structure"
  marker and a checklist entry, instead of silently dropping in a default tower.
- The engineer **complies and chooses** the structure (function + family) at each
  flagged station; a terminal is gated to an **anchor**, a deflection to a family whose
  `max_line_angle_deg` covers it.
- Rationale: structure choice is an engineering decision (running vs strain angle,
  family rating); the tool should **require and guide** it, not pre-empt it.
  Cost-minimizing auto-spotting against the chart stays the future optimiser (ADR-0010).

## Next — uneven-span mechanics (Phase G11)

G7–G10 gave the line many wires, tension sections, a rated and **drawable** structure
library, and a route the profile derives from. What is still assumed rather than solved
is the horizontal tension across spans whose **attachment points are uneven** — exactly
the unevenness the per-family geometry (G10) now makes explicit. G11 closes that.

### G11 — Uneven-span FEM section solver — *stage 7* — ADR-0021

- An **uneven-span cable-FEM section kernel** behind the existing `ruling_span::Section`
  interface in `analysis`, modelling the section across its real, **unequal attachment
  points** (G10) with insulator swing, so horizontal tension is **solved**, not assumed
  equal. Reuses `core::conductor` for the change of state — the new code is the
  structural solve, not new material physics.
- **The analytic ruling span stays** as the validated oracle; the FEM kernel must agree
  with it in the level/equal-span limit to an explicit tolerance (ADR-0008), and the
  per-section kernel choice (analytic vs FEM) is reported. Static uneven-span solve only;
  the dynamic FEM/ROM track stays later (ADR-0003).

## Then — standards, precise terrain & field outputs (Phases G12–G15)

These reuse the validated core (`change_of_state`, the G11 section solve,
`structure::structure_loads`) — representational and orchestration work over numerics
that are already cross-checked.

### G12 — Load cases & standards (IEC 60826) — *stages 2, 7–8* — ADR-0017

- A **criteria-set / load-case engine**: each case (everyday, **extreme wind**,
  **construction**, **broken-wire / unbalanced longitudinal**, min-temp) is a
  change-of-state target on the section solve plus a structure-load combination,
  compared to the conductor tension limits and the structure family's rating.
- IEC 60826 first; NBR 5422 and others slot in as additional criteria sets,
  selectable per project (extends ADR-0004). Generalises the current single
  `wind_pressure_pa`. Reuses the cable solver untouched (ADR-0008 suite still gates).

### G13 — High-precision right-of-way terrain (~1 m) — *stage 4* — ADR-0018

- **Two-tier terrain:** coarse public DEM for wide-area routing; once the route is
  committed, **densify the corridor** to ~1 m and sample the DEM with **bilinear /
  bicubic interpolation** (refining `geo::extract_profile` / `dem.elevation_at`),
  with explicit no-data handling. Leaves a clear hook to substitute surveyed
  **LiDAR** (G4) or imported survey for the corridor without disturbing consumers.

### G14 — Stringing tables & field outputs — *stage 9*

- Per tension section, a **sag-tension table vs temperature** at the stringing
  tension (and per-span clipping/pulley offsets), emitted alongside the report and
  sheet. No new numerics — it tabulates the per-section solve through
  `change_of_state` for the field crew (IEEE 524 stringing practice).

### G15 — Production plan & profile sheets — *stage 9* — ADR-0024

G6 delivered the plan-&-profile *seam* — one shared `analysis` feeding a Markdown report
and an SVG plot. G15 turns that plot into the **document a line engineer recognises**:

- **Paginated, physically-sized sheets.** `sheet.rs` grows a `PageSize` (**A1**
  594×841 mm default, **A0** 841×1189 mm, or custom), authored in `mm` so it prints 1:1
  and imports to CAD at true scale. The line is drawn at a **fixed horizontal/vertical
  scale** (e.g. 1:2400 / 1:480, V.E. 5×) and **split into sheets by station** with a
  **match line** between them; `plan_profile_sheets(project, &layout)` returns an ordered
  `Vec` of SVGs. The existing `plan_profile_svg` stays as the fit-to-one preview / GUI
  live view.
- **Configurable label sets** via a `SheetLayout` mirroring the reference tool's tabs:
  **structure labels** (number, station on by default; X/Y, height & elevation,
  height/leg adjustment, orientation, offset, embedded length, attachment height/elevation,
  line angle, comments optional), **section labels** (structure range, cable file, ruling
  span, design tension on by default; voltage, phases/wires, weather case, legend
  optional), **span-length** and **wire** labels, and the **plan/inset** view content
  (centerline, structures, RoW edges, stationing, crossings; ortho raster when present).
  Each field maps to a value already in `Tower`/`Wire`/section/`Poi` or `Analysis`.
- **Title, notes & legend blocks.** A lower-right **title block** from `Project::metadata`
  (owner/agency, line name, voltage, drawing/sheet `n of N`, dates, revision/supersedes —
  metadata extended via a round-trip-tested `.atldp` bump, defaulting empty for legacy
  projects), a **notes block** (loading code/weather case, conductor & OPGW specs, ruling
  span, tension at a reference condition, ground clearance, max operating temp,
  terrain/imagery provenance, ATLDP version), and a **legend**.
- **No new numerics** — every value is read from the model or `Analysis`; tests assert
  sheet *structure* (well-formed SVG, page count for a station range, presence of selected
  labels and title-block fields), as the G6 sheet tests already do. SVG-only,
  string-built, zero new deps. Phasing/leg-extension diagrams and full ortho-raster
  compositing are designed in as optional blocks and may land incrementally; the schematic
  plan strip is the fallback until the corridor raster (ADR-0022) exists.

### Then — automatic spotting & dynamics

- **Automatic spotting** (the original Phase 5): a cost-minimizing optimizer that
  **chooses structure families** (G8/G10) and respects load cases (G12) and per-section
  tractions (G7), layered on the *same* placement-evaluation functions as manual
  spotting (ADR-0010) — and keeping the obligatory angle structures (G9) fixed — so both
  paths agree by construction.
- **Dynamics / ROM** (research track): FEM + reduced-order models for aeolian
  vibration, galloping and gust response (Bertrand 2022; Irvine), behind the static
  core and validated against it in the static limit (ADR-0003).

## Validation strategy

- **OTLS-Models is the de facto external reference** for the sag-tension core
  (`third_party/Models`, ADR-0008): both the catenary (`golden_otls_models`) and the
  nonlinear change-of-state (`golden_otls_change_of_state`, ≤0.2 %) are cross-checked
  against it. The independent nonlinear conductor model (G1b) makes the
  change-of-state comparison a validation rather than a transcription.
- A suite of **golden cases** (`crates/atldp-core/tests/golden_*.rs`), each citing its
  source; the closed-form catenary identities (Irvine) are a second independent
  oracle. The retired Python `core/` lives on as committed cross-check fixtures
  (`crates/atldp-core/tests/fixtures/`).
- Most G7–G15 work is **additive over these validated numerics** — multi-wire,
  sections, the route/POI model, load cases, stringing tables, and the production sheets
  orchestrate or draw the same solver output, so the existing suite keeps gating
  correctness and new tests cover the orchestration and rendering (and the schema
  migrations). The delivered G7–G10 orchestration is
  covered in `atldp-model` (section partitioning, v1→v2 **and v2→v3** schema migration,
  chart envelopes, multi-wire fan-out, the route/POI pinning rules, and the family
  attachment geometry / wire-offset reconciliation) and `atldp-geo` (the polyline
  profile extractor).
- **G11 is the exception**: the uneven-span FEM section solver introduces *new*
  numerics into `atldp-core` and must earn its place under ADR-0008. It is gated by
  **agreement with the analytic ruling-span core in their overlapping domain** (level,
  equal spans, free-swinging insulators) to an explicit tolerance, plus external
  FEM/reference cases (e.g. OTLS-Models uneven configurations) where available. Track
  tolerances explicitly; treat a tolerance regression as a build failure. The
  later dynamics/ROM track is likewise validated against the static core in its limit.

## Open questions (to resolve via ADRs / discussion)

- ~~Final language & packaging.~~ **Resolved:** native Rust workspace (ADR-0011).
- ~~GUI/desktop vs. web.~~ **Resolved:** native desktop CAD app (ADR-0012).
- ~~An open project file format.~~ **Resolved:** versioned JSON `.atldp` (G6).
  Interoperability with a *specific commercial* format — and its legal footing —
  remains open.
- ~~**Multi-wire schema migration (G7):** the v1 → v2 upgrade path.~~ **Resolved:**
  `format::from_atldp_str` migrates the raw JSON before typing it — v1's single
  `conductor` at `horizontal_tension_n` becomes a one-element phase `wires` set, and
  untyped towers default to suspension (one tension section), reproducing v1 results
  (round-trip tested).
- ~~**Application-chart data (G8):** representation of the wind/weight-span envelope.~~
  **Resolved:** a piecewise `ApplicationChart` (weight-span band interpolated over
  wind span, plus a deviation-angle cap); charts are the family's own data, shipping
  as a small built-in library (`StructureFamily::built_in_library`) and editable per
  project (no standard tables redistributed — ADR-0004).
- ~~**Route schema (G9):** the `Route`/`Poi` representation and the v2→v3 migration —
  how POI kinds map to pinned structures, and how derived `ground_profile` staleness is
  tracked (ADR-0019).~~ **Resolved:** a `Route` is an ordered `Vec<Poi>` (kind +
  lat/lon + station + sampled elevation + deviation angle); `PoiKind::pinned_function`
  maps terminal→anchor and angle→(≥)angle, others pin nothing; the profile is **derived**
  via `geo::extract_profile_polyline` and stored, with `Tower::origin_poi` linking a
  structure to the POI that pinned it. The v2→v3 migration synthesises a
  terminal→terminal route from the legacy profile endpoints (round-trip tested).
  **Revised (ADR-0023):** the `angle→angle` mapping was wrong — angle is **not** a
  structure function. A `StructureFunction` is `Suspension` or `Anchor` only; an angle
  POI obliges *a* structure (not a specific function), and "angle" is derived from the
  POI's deviation, gated by the family chart's `max_line_angle_deg`. This correction is
  folded into the G10c v3→v4 schema bump (see below).
- ~~**Area selection & route authoring (G10c, ADR-0022):** how the import area maps to a
  cropped/mosaicked DEM and the v3→v4 `TerrainRef` (single tile → tile set + bounds), and
  how plan-view POI edits re-derive the profile and re-pin structures live.~~
  **Resolved (route editor):** `atldp-geo` `mosaic` + `Dem::crop` produce the working
  DEM; `TerrainRef` is a `Vec<TerrainTile>` + `AreaBounds`; `TerrainData` holds the
  editable `Vec<Poi>` and `recompute()` re-stations, ground-samples, and re-derives the
  profile (`extract_profile_polyline`) on every plan-view edit. **Area selection landed
  (G10f):** a *Set work area…* dialog crops the working window to typed lat/lon bounds
  (route preserved) for finer grid/basemap detail, and *Load terrain tile…* opens an
  `.hgt` for another region. **Still open:** a graphical (drag-a-box) area picker and the
  stage-1 online-basemap endpoint picker, and multi-tile auto-discovery for a span that
  crosses a seam not yet loaded.
- **FEM kernel selection (G11):** the precise geometric criterion that switches a
  section from the analytic ruling span to the FEM solver (span-unevenness ratio,
  inclination, operating temperature), and the validated agreement tolerance between
  them (ADR-0021/0008).
- ~~**Basemap source & policy (G10d–G10e, ADR-0025):** which tile provider/style and the
  zoom-selection rule for a window; OSM tile-usage-policy compliance (User-Agent,
  cache, request volume) vs. a self-hosted/permissive alternative; texture-memory
  budget for the 3-D drape; and the HTTP/decoder/SVG-raster dependencies (`ureq`,
  `image`, `resvg`) against the < 30 MB binary gate.~~ **Resolved:** OSM tiles via a
  configurable `TILE_URL` (single point of change for a self-hosted/permissive source),
  identifying `User-Agent`, on-disk cache and a ≤64-tile-per-window cap; zoom picked so
  the window stays within ~6 tiles. The 3-D drape (G10e) reuses the one composited
  `MapImage` as its texture (uploaded once, ≤ 8×8×256² RGBA), adding **no new
  dependencies**; the release binary is **18 MB stripped** (< 30 MB gate).
- ~~**Structure geometry data (G10):** the representation of a family's attachment /
  silhouette geometry, and how it reconciles with the per-`Wire` offsets introduced in
  G7 (ADR-0020).~~ **Resolved:** an `AttachmentGeometry` on `StructureFamily` — an
  ordered `Vec<AttachmentPoint>` (role + lateral/vertical offset + label) plus a
  silhouette polyline in the structure elevation frame. The family geometry is the
  source of truth; `Project::reconcile_wire_offsets` copies each attachment point's
  offsets onto the matching wire by index, leaving conductor spec and tension on the
  `Wire`.
- **Production sheet layout (G15, ADR-0024):** the `PageSize`/`SheetLayout` representation,
  the station-pagination + match-line rule at a fixed drawing scale, the title-block fields
  added to `Project::metadata` (and their `.atldp` migration), and how dense label stacks
  avoid collision at close structure spacing. Proposed in ADR-0024; resolution lands with
  the phase.
- **1 m corridor source (G13):** how far DEM interpolation is trusted before a
  surveyed LiDAR/contour source is required, and the ingestion path for it.
- Maturity of the pure-Rust geospatial stack (`proj4rs` CRS coverage, `laz-rs` decode
  speed) — validated in G3/G4, with a documented `gdal` fallback (ADR-0013).

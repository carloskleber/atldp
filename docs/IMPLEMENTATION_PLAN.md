# ATLDP — Implementation Plan

This is a proposal. Nothing here is built yet. It translates the project's goals
(see [README](../README.md)) into a phased, verifiable plan, and points to the
architectural decisions that back it (see [adr/](adr/)).

> **Substrate change (2026-06-15).** The product is a desktop CAD application with
> interactive 2D/3D views (terrain, route, towers, conductors, **LiDAR point
> clouds**), high rendering performance, and a small optimized binary (< 30 MB),
> Linux-first then Windows. To meet that, the production stack moves from Python to
> a **native Rust** Cargo workspace (ADR-0011), with a winit + wgpu + egui desktop
> shell (ADR-0012) and a pure-Rust geospatial stack (ADR-0013). The **product
> design is unchanged** — the staged pipeline and domain modules below still hold;
> only the implementation language changes. The Python `core/` (Phase 1) is kept as
> a **transitional validation oracle** and retired once the Rust core reproduces it
> and an independent reference (ADR-0014). The build phases below are renamed to a
> native track (Phase **G0–G6**); the original phase numbering is preserved in the
> per-phase headings for traceability.

## Guiding principles

1. **Validate before building.** Every numerical model must reproduce a published
   or independently-computed reference result before it is trusted.
2. **Separate the core from everything else.** A pure, headless, well-tested
   computational engine — no GUI, no I/O, no geospatial dependencies in it.
3. **Standards are first-class.** Criteria from IEEE / CIGRE / ABNT are encoded
   explicitly and cited, not hard-coded as magic numbers.
4. **Throwaway prototypes are fine.** `tests/` stays a sandbox; promotion to the
   core requires tests and validation.

## The design pipeline (product workflow)

ATLDP executes a transmission-line project as a staged pipeline; each stage
consumes the previous stage's output and adds to a shared project model
(ADR-0009). This is the *runtime* order of a project — distinct from the *build*
order of the phases below.

| # | Stage | Input → Output | Notes |
| --- | --- | --- | --- |
| 1 | **Terrain model** | public DEM / LiDAR → 3D ground+feature model | DEM first, professional LiDAR point clouds later |
| 2 | **Line route** | terrain + POIs → georeferenced route | angle points, crossings, obstacles, constraints |
| 3 | **Tower placement (spotting)** | route + structure library → spotted structures | manual first; automatic cost-minimizing optimizer later |
| 4 | **Sag-tension** | spans + conductors + weather → sags, tensions, clearances | swing/blowout; tension limits; wire-ground & phase-phase clearances |
| 5 | **Structure modeling** | spotted structures + spans → load checks | wind/weight span, guyed towers, simple lattice model |
| 6 | **Plan & profile drafting** | full project model → sheets & reports | drawings + calculation reports |

## Domain decomposition

The pipeline rests on these largely independent computational modules:

| Module | Problem | Serves stage | Key references |
| --- | --- | --- | --- |
| **Conductor model** | thermal/elastic constitutive behavior, stress-strain (initial/final, creep) | 4 | Aluminum Association, CIGRE TB 324 |
| **Sag-tension (single span)** | catenary / parabola, level & inclined supports | 4 | Irvine & Caughey 1974; theory.md |
| **Change-of-state** | equilibrium across temperature/load/creep states | 4 | CIGRE TB 601 |
| **Ruling span** | multi-span section under varying conditions | 4 | IEEE/CIGRE practice |
| **Uneven / inclined / dynamic spans** | FEM when ruling-span assumptions break | 4 | Bertrand 2020/2022; Sugiyama 2003 |
| **Weather & loads** | wind/ice load cases, conductor swing/blowout | 4, 5 | ABNT NBR 5422; IEC 60826 |
| **Thermal rating** | ampacity vs. conductor temperature | 4 | IEEE 738; CIGRE TB 601 |
| **Geospatial** | DEM/LiDAR ingestion, ground profile, CRS handling | 1, 2 | rasterio/GDAL, pyproj, PDAL |
| **Routing & POIs** | route geometry over terrain | 2 | — |
| **Spotting** | structure placement; cost-minimizing optimization | 3 | NBR 5422 clearances |
| **Clearances** | ground/RoW and phase-phase clearance checks | 3, 4 | NBR 5422 |
| **Structure model** | wind/weight span, lattice & guyed-tower load checks | 5 | NBR 5422; IEC 60826 |
| **Drafting & reporting** | plan & profile sheets, calculation reports | 6 | — |

## Phases

The build order differs from the runtime pipeline order: the sag-tension core
(stage 4) is built first because it is the highest-risk, highest-value piece and
the validation oracle for everything else. Stages 1–3 and 5–6 wrap around it.

### Native build track (Phases G0–G6)

Per the substrate change above, the production app is built natively in Rust
(ADR-0011/0012/0013). These phases carry the project from the validated Python
prototype to the shipping desktop CAD application; the original Python phases
(0–6) below remain as the model/spec the native track reproduces.

- **G0 — ADRs + workspace skeleton — ✅ done (2026-06-15).** ADR-0011–0014
  written; ADR-0002 superseded, ADR-0006 resolved, ADR-0005 tooling refined. A
  Cargo workspace (`Cargo.toml`, `crates/atldp-{core,geo,model,render,app,cli}`)
  with the optimized release profile and Linux+Windows CI
  ([.github/workflows/ci.yml](../.github/workflows/ci.yml)); `cargo fmt`, `clippy
  -D warnings`, and `cargo test` are green on the skeleton.
- **G1 — Port & validate `atldp-core` (stage 4). 🚧 in progress (2026-06-15).**
  Geometry, catenary (inclined + parabola, regime switch), change-of-state,
  ruling span, and conductor are reimplemented in Rust as a **dependency-free**
  `atldp-core`, with the thin `atldp` CLI (`catenary`, `cos`) ported alongside.
  ADR-0014's first two retirement gates are met: every `core/validation/` golden
  case is re-encoded as an `atldp-core` test, and a **cross-check harness**
  (`core/validation/export_reference.py` → committed CSV fixtures →
  `crates/atldp-core/tests/cross_check_python_oracle.rs`) agrees with the Python
  oracle to ≤1e-7 rel over an 882-case sweep. The **third gate — an independent
  third-party numeric reference — is still open** (the same Phase-1 item in
  `core/validation/README.md`), so the Python `core/` is **kept**, not yet
  retired. Closing that reference completes the ADR-0014 gate and retires `core/`.
- **G2 — Render foundation (ADR-0012).** winit + egui docked shell; wgpu 3D
  viewport with an orbit camera and a live catenary from the core; 2D ortho
  viewport (pan/zoom/grid/snap). **Prove the < 30 MB optimized build on Linux and
  Windows here.**
- **G3 — Terrain & route (stages 1–2).** `atldp-geo` DEM ingest + CRS + ground
  profile (pure-Rust, ADR-0013); terrain mesh in 3D; route + POIs in 2D and 3D.
- **G4 — LiDAR point-cloud engine (stage 1, advanced).** LAS/LAZ load, octree
  LOD, GPU point renderer, picking — the highest-risk component, isolated.
- **G5 — Manual spotting + sag-tension (stages 3–4 in-GUI).** Place towers on the
  profile with live clearance/tension checks; conductors + swing in 3D.
- **G6 — Structure modeling, drafting & file format (stages 5–6).** 2D plan &
  profile sheets, reports, and `atldp-model` serialization as the open ATLDP
  project format.

The original (Python) phases below remain the validated specification the native
track must reproduce.

### Phase 0 — Repository hygiene (prerequisite) — ✅ done (2026-06-15)

- ✅ Added a `.gitignore` that excludes virtualenvs (`**/.venv/`), large binaries,
  DEM/LiDAR rasters (`*.tif`, `*.las`, …), prototype-generated HTML, and LaTeX
  build artifacts. Verified that the 676 MB-class `tests/terrain/.venv` and
  `dem.tif` are ignored while real source is not.
- ✅ Adopted per-prototype environments (ADR-0007): the terrain prototype now has
  a local `tests/terrain/requirements.txt` and a `README.md` documenting its
  isolated setup.
- ✅ Established the ADR log ([adr/](adr/)) and this plan as living documents;
  ADR-0007 is now **Accepted** as the implemented hygiene baseline.

### Phase 1 — Validated sag-tension core — *pipeline stage 4* — 🚧 in progress (2026-06-15)

The headless core lives in [`core/`](../core/) (package `atldp`, ADR-0002 layout).

- ✅ **3D-aware geometry** (`atldp.core.geometry`): attachment points in 3D,
  reduced to horizontal distance + elevation difference. Sag-tension is usually
  drawn in 2D but is really 3D — uneven spans and angle towers are the normal
  case. Plan bearing is carried for angle-tower handling, and load-per-length is
  a parameter so wind blow-out (Phase 2) slots in without rework. See the
  expanded `theory.md`.
- ✅ **Single-span catenary** (exact, **inclined/uneven** supports) and
  **parabolic** approximation, with the span-to-depth-ratio / inclination switch
  documented in `theory.md` (`atldp.core.catenary`).
- ✅ **Change-of-state equation** (conserves the unstrained length; works on
  inclined spans) and a **ruling-span** section model (`atldp.core.change_of_state`,
  `atldp.core.ruling_span`).
- ✅ **Conductor library** with ACSR Drake 26/7 — linear-elastic + thermal model
  (`atldp.core.conductor`). ⏳ Full nonlinear stress-strain (initial/final) + creep
  per CIGRE TB 324 is a documented later refinement of `Conductor.strain`.
- ✅ Headless library with a thin **CLI** (`atldp`), no GUI (ADR-0006).
- ✅ **Validation** (`core/validation/`, ADR-0008): closed-form catenary
  identities and parabola↔catenary cross-method agreement are fully independent
  oracles; change-of-state pins physics invariants (length conservation,
  monotonicity, round-trip). ⏳ **Open item:** a third-party numeric cross-check
  against `OnSag`/`SSTC` or a digitised textbook/IEEE table — the `mpewsey`
  reference turned out to be algorithm-only (no numbers). Tracked in
  `core/validation/README.md`.

Remaining for Phase 1 close-out: the third-party numeric golden case and the
nonlinear conductor stress-strain/creep refinement.

### Phase 2 — Loads, swing, clearances, ampacity — *pipeline stage 4*

- Weather load cases and conductor **swing/blowout** (real 3D wind + normative).
- **Clearance checks**: wire-to-ground and **phase-to-phase** (including swing).
- **Ampacity** via IEEE 738 (steady-state first, transient later).
- Encode ABNT NBR 5422 load/clearance criteria as a pluggable "criteria set".

### Phase 3 — Terrain, route & manual spotting — *pipeline stages 1–3*

- **Terrain:** promote the terrain prototype — ingest **local DEMs** (`rasterio`)
  as the source of truth; CRS/datum handling via `pyproj`; ground-profile
  extraction along the line. Online elevation APIs stay prototype-only (ADR-0005).
- **LiDAR:** add a path to ingest professionally surveyed **point clouds**
  (`PDAL`/LAS), classified to ground + features (later than DEM).
- **Route:** lay out the route over the terrain with **points of interest**
  (angle points, crossings, obstacles, constraints).
- **Manual spotting:** place structures along the profile by hand, running the
  Phase 1/2 clearance and tension checks at each candidate position (ADR-0010).

### Phase 4 — Structure modeling & drafting — *pipeline stages 5–6*

- **Structure model:** wind span / weight span and structure **load checks**,
  including **guyed towers**, using a **simple lattice-model representation**.
- **Drafting:** generate **plan & profile sheets** and calculation reports per
  national standards.
- Define and document ATLDP's own **open project file format**; investigate
  import/export with existing formats to address bid-process lock-in.

### Phase 5 — Automatic spotting — *pipeline stage 3, optimized*

- Replace manual placement with an optimizer that **minimizes overall material
  cost** subject to clearance, tension, and structure-loading constraints
  (ADR-0010).

### Phase 6 — Advanced mechanics (optional / research track)

- FEM for inclined/uneven spans and dynamics (Bertrand, Sugiyama); reduced-order
  models for vibration. Validated against the analytic core where they overlap.

## Validation strategy

- A `validation/` suite of **golden cases**, each citing its source.
- Cross-check the analytic core against the FEM track in their common domain.
- Track tolerances explicitly; treat a tolerance regression as a build failure.

## Open questions (to resolve via ADRs / discussion)

- ~~Final language & packaging for the production core.~~ **Resolved:** native Rust
  workspace, single optimized binary (ADR-0011).
- ~~GUI/desktop vs. web delivery.~~ **Resolved:** native desktop CAD app, winit +
  wgpu + egui (ADR-0012).
- Whether to target interoperability with a specific commercial format, and the
  legal footing for doing so (addressed by the open ATLDP format in G6).
- Maturity of the pure-Rust geospatial stack (`proj4rs` CRS coverage, `laz-rs`
  decode speed) — validated in G3/G4, with a documented `gdal` fallback (ADR-0013).

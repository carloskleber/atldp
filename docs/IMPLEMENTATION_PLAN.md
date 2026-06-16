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
> only the implementation language changes. The Python `core/` (Phase 1) was a
> **transitional port scaffold, not an independently validated oracle**: its
> closed-form catenary checks are genuine, but its change-of-state engineering
> results were only pinned to self-consistency invariants, never to an outside
> reference. **OTLS-Models is the de facto external validation reference**
> (`third_party/Models`, ADR-0008/0014); gates 1–2 establish only Rust≡Python
> *port fidelity*, so external trust rests on the OTLS cross-check (gate 3) plus
> the closed-form identities. The Python `core/` is retired once that holds —
> which it now does (ADR-0014). The build phases below are renamed to a
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
| **Sag-tension (single span)** | catenary / parabola, level & inclined supports | 4 | Irvine & Caughey 1974; [theory.md](theory.md) |
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
(ADR-0011/0012/0013). These phases carry the project from the Python prototype to
the shipping desktop CAD application; the original Python phases (0–6) below remain
as the model/spec the native track reproduces — but the **de facto numerical
reference is OTLS-Models**, not the Python core (which is self-consistent, never
independently validated; see G1 and the validation strategy).

- **G0 — ADRs + workspace skeleton — ✅ done (2026-06-15).** ADR-0011–0014
  written; ADR-0002 superseded, ADR-0006 resolved, ADR-0005 tooling refined. A
  Cargo workspace (`Cargo.toml`, `crates/atldp-{core,geo,model,render,app,cli}`)
  with the optimized release profile and Linux+Windows CI
  ([.github/workflows/ci.yml](../.github/workflows/ci.yml)); `cargo fmt`, `clippy
  -D warnings`, and `cargo test` are green on the skeleton.
- **G1 — Port & validate `atldp-core` (stage 4). ✅ done (2026-06-16).**
  Geometry, catenary (inclined + parabola, regime switch), change-of-state,
  ruling span, and conductor are reimplemented in Rust as a **dependency-free**
  `atldp-core`, with the thin `atldp` CLI (`catenary`, `cos`) ported alongside.
  **All three of ADR-0014's retirement gates are now met:** every
  `core/validation/` golden case is re-encoded as an `atldp-core` test (gate 1); a
  **cross-check harness** (`core/validation/export_reference.py` → committed CSV
  fixtures → `crates/atldp-core/tests/cross_check_python_oracle.rs`) agrees with
  the Python oracle to ≤1e-7 rel over an 882-case sweep (gate 2); and the Rust
  catenary now reproduces an **independent third-party reference** —
  **OTLS-Models** (`third_party/Models` submodule @ `c270d48`, its own
  `catenary_test.cc` expectations) via `crates/atldp-core/tests/golden_otls_models.rs`
  (gate 3, see `core/validation/oracles/README.md`). The Python `core/` is
  therefore **eligible for retirement** per ADR-0014 (removable in a single
  ADR-citing commit). `OnSag` was evaluated but is a wxWidgets GUI consuming a
  precomputed tension table; OTLS-Models is its headless numeric engine and the
  cleaner oracle. The *change-of-state* third-party pin that was outstanding here
  is now delivered by G1b below.
- **G1b — Nonlinear cable (conductor) model. ✅ done (2026-06-16).** Added an
  **independent nonlinear bimetallic stress-strain model**
  (`atldp_core::conductor::StressStrainModel`): the composite load-strain curve is
  the sum of the steel-core and aluminium-shell **load-strain (and creep)
  polynomials** (Aluminum Association handbook / CIGRE TB 324), inverted directly,
  with **per-material thermal strains** — the bimetallic effect a single modulus
  can't capture. It attaches to a `Conductor` (`stress_strain` field) and
  `Conductor::strain` then uses it with **no change to callers** (the
  change-of-state equation is untouched; ADR-0003). **Crucially it is derived from
  the stress-strain physics, not transcribed from OTLS's elongation/region/stretch
  code** — only the published physical Drake polynomial data is shared, so the
  cross-check is a validation, not a tautology. Driven through this crate's own
  length-conserving `change_of_state`, it reproduces OTLS-Models'
  `catenary_cable_reloader` reload tensions (6788 / 4701 / 17123 lbf reference
  cases) to **≤ 0.2 %** — the bounded gap traces to legitimate convention
  differences (horizontal vs. average tension; continuous polynomial vs. piecewise
  regions), recorded in `core/validation/oracles/README.md` and pinned by
  `crates/atldp-core/tests/golden_otls_change_of_state.rs`. This **closes the
  change-of-state validation gap** Phase 1 left open. (Creep polynomials and the
  initial/final distinction are in the model; the after-creep and prior-stretch
  reload cases are a later refinement, not a blocker.)
- **G2 — Render foundation (ADR-0012). ✅ done (2026-06-16).** winit + egui docked
  shell; wgpu 3D viewport with orbit camera (left-drag rotate, scroll zoom) and a
  live LINE_STRIP catenary from `atldp-core`; 2D ortho viewport (right-drag pan,
  scroll zoom, adaptive grid). Stack: winit 0.30 (`ApplicationHandler`), egui
  0.34.3, egui_dock 0.19, wgpu 29.0.3 (Vulkan/DX12/Metal), WGSL shader embedded in
  binary. Binary size: **12 MB stripped on Linux** (< 30 MB gate ✅). Toolbar
  DragValues for span and horizontal tension drive the catenary in real time; sag is
  displayed live. wgpu 29 API migration: `InstanceDescriptor::new_without_display_handle`,
  `RenderPass::forget_lifetime`, `CurrentSurfaceTexture` enum (replacing
  `Result<SurfaceTexture, SurfaceError>`), `immediate_size` / `multiview_mask` /
  `depth_slice` fields.
- **G3 — Terrain & route (stages 1–2). ✅ done (2026-06-16).** `atldp-geo` DEM
  ingest + CRS + ground profile (pure-Rust, ADR-0013); terrain wireframe in the 3D
  orbit viewport; terrain profile + elevation annotations in the 2D panel; camera
  auto-fit to terrain extents on load; `ATLDP_TERRAIN` env-var or default tile path.
- **G4 — LiDAR point-cloud engine (stage 1, advanced). ⏸ postponed.** LAS/LAZ load,
  octree LOD, GPU point renderer, picking — the highest-risk component, isolated.
  Deferred until a surveyed LiDAR dataset is available to validate against.
- **G5 — Manual spotting + sag-tension (stages 3–4 in-GUI). ✅ done (2026-06-16).**
  Click-to-place tower spotting in the 2D terrain profile; live catenary between
  consecutive towers (`atldp-core` catenary, ACSR Drake weight); ground-clearance
  check with colour-coded violation highlights (cyan = OK, red = violation) in both
  2D and 3D; right-side panel with tower table and span table (horizontal span, sag,
  min clearance per span); toolbar controls for attachment height, minimum clearance,
  horizontal tension, vertical exaggeration, Undo / Clear; worst-clearance indicator
  in the toolbar; tower + conductor geometry in the 3D viewport via the
  `SpottingLines` LINE_LIST wgpu renderer (`atldp-render::spotting_lines`).
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
  (`atldp.core.conductor`), plus the ✅ **nonlinear bimetallic stress-strain
  model** (initial/final + creep polynomials, per-material thermal; **G1b** above)
  that gates and now delivers the OTLS change-of-state cross-check.
- ✅ Headless library with a thin **CLI** (`atldp`), no GUI (ADR-0006).
- ✅ **Validation** (`core/validation/`, ADR-0008): closed-form catenary
  identities and parabola↔catenary cross-method agreement are fully independent
  oracles; change-of-state pins physics invariants (length conservation,
  monotonicity, round-trip). ✅ **Third-party numeric cross-check (closed
  2026-06-16):** the Rust catenary matches **OTLS-Models** (`third_party/Models`
  @ `c270d48`) to its 2-dp rounding (`golden_otls_models`), and the **nonlinear**
  change-of-state matches its reloader tensions to ≤ 0.2 %
  (`golden_otls_change_of_state`, G1b) — provenance in
  `core/validation/oracles/README.md`. The `mpewsey` reference was algorithm-only
  (no numbers); `OnSag` is a GUI consuming a tension table, and OTLS-Models is its
  headless engine.

Phase 1 is closed out: the **nonlinear conductor stress-strain/creep model (G1b)**
landed, and with it the *change-of-state* third-party pin against OTLS-Models — the
catenary and the change-of-state are both now cross-checked against an independent
reference (`golden_otls_models`, `golden_otls_change_of_state`).

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

- **OTLS-Models is the de facto external reference** for the sag-tension core
  (`third_party/Models` submodule, ADR-0008/0014): both the catenary
  (`golden_otls_models`) and the nonlinear change-of-state
  (`golden_otls_change_of_state`, ≤ 0.2 %) are cross-checked against it. The
  independent nonlinear conductor model (G1b) is what makes the change-of-state
  comparison a validation rather than a transcription. The Python `core/` is
  **not** an independent oracle
  — it was only self-consistent (closed-form identities + invariants), so the
  Rust↔Python gates prove port fidelity, not correctness.
- A `validation/` suite of **golden cases**, each citing its source; the
  closed-form catenary identities (Irvine) are a second independent oracle.
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

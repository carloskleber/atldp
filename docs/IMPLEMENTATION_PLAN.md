# ATLDP — Implementation Plan

This translates the project's goals (see [README](../README.md)) into a phased,
verifiable plan, and points to the architectural decisions that back it (see
[adr/](adr/)). It is a living document: phases **G0–G6 are delivered**, phases
**G7–G11** are the next traced steps (ADR-0015–0018).

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

| # | Stage | Input → Output | Notes |
| --- | --- | --- | --- |
| 1 | **Terrain model** | public DEM / LiDAR → 3D ground+feature model | coarse wide-area tier + interpolated ~1 m right-of-way corridor (ADR-0018, G10) |
| 2 | **Line route** | terrain + POIs → georeferenced route | angle points, crossings, obstacles, constraints |
| 3 | **Tower placement (spotting)** | route + structure library → spotted, typed structures | suspension vs anchor typing → **tension sections** (ADR-0015); structures chosen from **families** by application chart (ADR-0016); manual first, auto later |
| 4 | **Sag-tension** | sections + **wire set** + load cases → sags, tensions, clearances | per-section ruling span; **every wire** (3 phases/circuit + shield) at its own tension; swing/blowout; wire-ground & phase-phase clearances |
| 5 | **Structure modeling** | spotted structures + spans + load cases → load checks | wind/weight/longitudinal span loads vs the family's rating; **IEC 60826** cases (ADR-0017) |
| 6 | **Plan & profile drafting** | full project model → sheets & reports | drawings, calculation reports, and the field **stringing table** (G11) |

## Domain decomposition

The pipeline rests on these largely independent computational modules:

| Module | Problem | Serves stage | Key references |
| --- | --- | --- | --- |
| **Conductor model** | thermal/elastic constitutive behavior, stress-strain (initial/final, creep) | 4 | Aluminum Association, CIGRE TB 324 |
| **Sag-tension (single span)** | catenary / parabola, level & inclined supports | 4 | Irvine & Caughey 1974; [theory.md](theory.md) |
| **Change-of-state** | equilibrium across temperature/load/creep states | 4 | CIGRE TB 601 |
| **Ruling span & tension sections** | multi-span section between anchors; per-section traction | 3, 4 | IEEE/CIGRE practice; ADR-0015 |
| **Multi-wire set** | phases (3·circuits) + shield wires, each own tension/load; lowest-wire clearance | 4 | ADR-0015 |
| **Uneven / inclined / dynamic spans** | FEM when ruling-span assumptions break | 4 | Bertrand 2020/2022; Sugiyama 2003 |
| **Weather, loads & load cases** | wind/ice cases, swing/blowout, IEC 60826 case set (extreme wind, construction, broken wire) | 4, 5 | IEC 60826; ABNT NBR 5422; ADR-0017 |
| **Thermal rating** | ampacity vs. conductor temperature | 4 | IEEE 738; CIGRE TB 601 |
| **Geospatial** | DEM/LiDAR ingestion; coarse map + interpolated ~1 m corridor profile; CRS | 1, 2 | tiff/geotiff, proj4rs, las/laz; ADR-0018 |
| **Routing & POIs** | route geometry over terrain | 2 | — |
| **Structure library** | families, effective height, application (wind/weight-span) chart | 3, 5 | ADR-0016 |
| **Spotting** | structure placement & family selection; cost-minimizing optimization | 3 | ADR-0010/0016; NBR 5422 clearances |
| **Clearances** | ground/RoW and phase-phase clearance checks | 3, 4 | NBR 5422 |
| **Structure model** | wind/weight/longitudinal span loads, lattice & guyed-tower checks | 5 | IEC 60826; NBR 5422 |
| **Drafting, reporting & stringing table** | plan & profile sheets, calculation reports, field stringing chart | 6 | IEEE 524; ADR-0009 |

## Delivered — native build track (Phases G0–G6, complete)

The production app is built natively in Rust (ADR-0011/0012/0013). The sag-tension
core (stage 4) was built first — highest risk, highest value, and the validation
oracle for everything else — with stages 1–3 and 5–6 wrapped around it. Full
blow-by-blow validation lineage lives in
[`crates/atldp-core/tests/ORACLES.md`](../crates/atldp-core/tests/ORACLES.md); the
summary:

| Phase | Delivered | Date |
| --- | --- | --- |
| **G0** | ADRs 0011–0014 + Cargo workspace skeleton (`crates/atldp-{core,geo,model,render,app,cli}`), optimized release profile, Linux+Windows CI; fmt/clippy/test green. | 2026-06-15 |
| **G1** | Dependency-free `atldp-core` (stage 4): geometry, inclined catenary + parabola, change-of-state, ruling span, conductor; thin `atldp` CLI. Cross-checked against the Python oracle (≤1e-7 over 882 cases) **and** the independent **OTLS-Models** reference (`golden_otls_models`). | 2026-06-16 |
| **G1b** | Independent nonlinear bimetallic stress-strain/creep conductor model (`conductor::StressStrainModel`, Aluminum Assoc. / CIGRE TB 324). Reproduces OTLS reloader tensions to ≤0.2 % (`golden_otls_change_of_state`) — closes the change-of-state validation gap. | 2026-06-16 |
| **G2** | Render foundation (ADR-0012): winit + egui docked shell, wgpu 3D orbit viewport with live catenary, 2D ortho viewport. **12 MB stripped** (< 30 MB gate ✅). | 2026-06-16 |
| **G3** | Terrain & route (stages 1–2): `atldp-geo` DEM ingest + CRS + ground profile (pure-Rust); 3D terrain wireframe, 2D profile, camera auto-fit. | 2026-06-16 |
| **G4** | LiDAR point-cloud engine (stage 1, advanced) — **⏸ postponed** until a surveyed dataset is available to validate against. | — |
| **G5** | Manual spotting + in-GUI sag-tension (stages 3–4): click-to-place towers, live catenary, colour-coded ground-clearance check, tower/span tables. | 2026-06-16 |
| **G6** | Structure loads (`core::structure`: signed wind/weight spans + conductor loads), the open versioned-JSON **`.atldp`** format (`atldp-model`), and drafting (Markdown `report` + SVG plan-&-profile `sheet`) from one shared `analysis` pass. | 2026-06-20 |

These phases deliver a working **single-conductor, single-tension** line tool. The
phases below grow it into a real multi-wire, multi-section project tool.

## Next — from a single conductor to a real line (Phases G7–G11)

The G6 model strings one conductor at one global tension. A real line has many wires
at different tensions, is split into tension sections by anchor structures, is built
from a library of structure families, must satisfy a standard's load cases, needs a
precise right-of-way profile, and ends in a field stringing table. These phases add
exactly that, **reusing the validated core** (`change_of_state`,
`ruling_span::Section`, `structure::structure_loads`) — they are representational and
orchestration work over numerics that are already cross-checked.

### G7 — Tension sections + multi-wire conductor set — *stages 3–4* — ADR-0015

- **Structure typing** (suspension / angle / anchor-dead-end) on each spotted
  structure; group the ordered structures into **tension sections** at every anchor.
- Feed each section's spans to `atldp_core::ruling_span::Section` (already built) for
  a **per-section ruling span and stringing tension** — the home for the ruling span
  in the product, and the mechanism for **different tractions per stretch**.
- Replace the single `Project.conductor` + `parameters.horizontal_tension_n` with a
  **wire set**: N phase conductors (3 · circuits) plus shield/ground wire(s), each
  with its own conductor spec, attachment geometry, and per-section tension. **Plot
  every wire**; add a **"lowest wire only"** mode for reading ground clearance.
- `.atldp` `SCHEMA_VERSION` → 2; migrate a v1 project as a one-wire, single-section
  case (round-trip tested) via the existing `format::from_atldp_str` seam.

### G8 — Structure family library & editable structures — *stage 3* — ADR-0016

- A `StructureFamily`: name, function, **effective height** (over a height range),
  per-wire attachment offsets, and an **application/usage chart** (allowable
  wind-span × weight-span × angle envelope).
- A spotted `Tower` **references a family + height** and may **override** the
  effective height / chart — edit a structure after spotting.
- Manual spotting checks each placement's wind/weight spans
  (`structure::structure_loads`) against the family chart and flags violations; the
  later optimizer (Phase 5) **selects** the family from the same check.

### G9 — Load cases & standards (IEC 60826) — *stages 4–5* — ADR-0017

- A **criteria-set / load-case engine**: each case (everyday, **extreme wind**,
  **construction**, **broken-wire / unbalanced longitudinal**, min-temp) is a
  change-of-state target on the section ruling span plus a structure-load
  combination, compared to the conductor tension limits and the structure family's
  rating.
- IEC 60826 first; NBR 5422 and others slot in as additional criteria sets,
  selectable per project (extends ADR-0004). Generalises the current single
  `wind_pressure_pa`. Reuses the cable solver untouched (ADR-0008 suite still gates).

### G10 — High-precision right-of-way terrain (~1 m) — *stages 1–2* — ADR-0018

- **Two-tier terrain:** coarse public DEM for wide-area routing; once the route is
  committed, **densify the corridor** to ~1 m and sample the DEM with **bilinear /
  bicubic interpolation** (refining `geo::extract_profile` / `dem.elevation_at`),
  with explicit no-data handling. Leaves a clear hook to substitute surveyed
  **LiDAR** (G4) or imported survey for the corridor without disturbing consumers.

### G11 — Stringing tables & field outputs — *stage 6*

- Per tension section, a **sag-tension table vs temperature** at the stringing
  tension (and per-span clipping/pulley offsets), emitted alongside the report and
  sheet. No new numerics — it tabulates the G7 per-section solve through
  `change_of_state` for the field crew (IEEE 524 stringing practice).

### Then — automatic spotting & advanced mechanics

- **Automatic spotting** (the original Phase 5): a cost-minimizing optimizer that
  **chooses structure families** (G8) and respects load cases (G9) and per-section
  tractions (G7), layered on the *same* placement-evaluation functions as manual
  spotting (ADR-0010) so both paths agree by construction.
- **Advanced mechanics / FEM** (the original Phase 6, research track): FEM for
  inclined/uneven spans and dynamics (Bertrand, Sugiyama); reduced-order vibration
  models. Validated against the analytic core where they overlap (ADR-0003).

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
- The G7–G11 work is **additive over these validated numerics** — multi-wire,
  sections, load cases, and stringing tables orchestrate the same solver, so the
  existing suite keeps gating correctness; new tests cover the orchestration
  (section partitioning, schema migration, chart envelopes, table generation).
- Cross-check the analytic core against the FEM track in their common domain. Track
  tolerances explicitly; treat a tolerance regression as a build failure.

## Open questions (to resolve via ADRs / discussion)

- ~~Final language & packaging.~~ **Resolved:** native Rust workspace (ADR-0011).
- ~~GUI/desktop vs. web.~~ **Resolved:** native desktop CAD app (ADR-0012).
- ~~An open project file format.~~ **Resolved:** versioned JSON `.atldp` (G6).
  Interoperability with a *specific commercial* format — and its legal footing —
  remains open.
- **Multi-wire schema migration (G7):** the v1 → v2 upgrade path and how to default
  per-section, per-wire tractions for an imported single-conductor project.
- **Application-chart data (G8):** representation of the wind/weight-span envelope and
  where a project's family charts come from (built-in set vs. user/utility import).
- **1 m corridor source (G10):** how far DEM interpolation is trusted before a
  surveyed LiDAR/contour source is required, and the ingestion path for it.
- Maturity of the pure-Rust geospatial stack (`proj4rs` CRS coverage, `laz-rs` decode
  speed) — validated in G3/G4, with a documented `gdal` fallback (ADR-0013).

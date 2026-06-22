# Architecture Decision Records

This log captures the significant architectural decisions for ATLDP, with their
context and consequences. Format: lightweight [MADR](https://adr.github.io/madr/).

Most ADRs below are **proposed** — recommendations for discussion, not yet
ratified. ADR-0007 was **accepted** and implemented in Phase 0 (repository
hygiene); ADR-0003 and ADR-0008 were **accepted** and implemented in Phase 1 (the
sag-tension core). ADR-0011–0014 (**accepted**, 2026-06-15) commit the production
app to a native Rust desktop CAD stack: ADR-0011 supersedes ADR-0002, ADR-0012
resolves ADR-0006, and ADR-0013 refines the geospatial tooling of ADR-0005.
ADR-0015–0018 (2026-06-20) trace the product step from the single-conductor G6
model to a real multi-wire, multi-section line design (tension sections, structure
families, standards load cases, and a precise right-of-way profile). **ADR-0015 and
ADR-0016 are accepted and implemented** (G7/G8, 2026-06-21); ADR-0017–0018 remain
proposed. ADR-0019–0021 (2026-06-21) reprioritise the near-term roadmap around the
real design workflow: an explicit route/POI model with obligatory angle structures
(0019), the tower-elevation view with real attachment geometry (0020), and bringing
the FEM section solver forward for uneven spans (0021, amending the *timing* of
ADR-0003). **ADR-0019 and ADR-0020 are accepted and implemented** (G9/G10,
2026-06-21); ADR-0021 remains proposed. ADR-0022 (2026-06-21) proposes the GUI
authoring step ADR-0019 deferred — SRTM **area selection** and a **plan-view route
editor** so the route is drawn on the terrain rather than hard-coded — and **reorders
the workflow** so project-endpoint definition and standards selection precede the
terrain (see ADR-0009). ADR-0023 (2026-06-21) corrects ADR-0019's structure typing:
**angle is a deflection-derived property, not a structure function** (a structure is
suspension or anchor). ADR-0024 (2026-06-22) grows the G6 plan-&-profile *plot* into
**production sheets** — paginated to **A1/A0** at a fixed drawing scale, with the full
configurable structure / section / span / wire label sets, a title block, and a
notes/legend block — the stage-9 drafting deliverable. See phases G7–G15 in the
[implementation plan](../IMPLEMENTATION_PLAN.md).

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Proposed |
| [0002](0002-python-core-with-separated-layers.md) | Python computational core with separated layers | Superseded by 0011 |
| [0003](0003-analytic-sag-tension-baseline-before-fem.md) | Analytic sag-tension baseline before FEM | Accepted (timing amended by 0021) |
| [0004](0004-standards-baseline.md) | Standards baseline (IEEE / CIGRE / ABNT) | Proposed |
| [0005](0005-local-dem-as-geospatial-source-of-truth.md) | Local DEM as geospatial source of truth | Proposed (tooling refined by 0013) |
| [0006](0006-defer-gui-headless-first.md) | Headless-first; defer the GUI decision | Resolved by 0012 |
| [0007](0007-prototype-isolation-and-repo-hygiene.md) | Prototype isolation and repository hygiene | Accepted |
| [0008](0008-validation-against-references.md) | Validate against published references | Accepted |
| [0009](0009-staged-design-pipeline-and-project-model.md) | Staged design pipeline and shared project model | Proposed |
| [0010](0010-manual-spotting-before-cost-optimization.md) | Manual spotting before cost-minimizing optimization | Proposed |
| [0011](0011-rust-native-production-stack.md) | Rust native production stack | Accepted |
| [0012](0012-desktop-gui-wgpu-egui.md) | Desktop GUI and rendering: winit + wgpu + egui | Accepted |
| [0013](0013-pure-rust-geospatial-stack.md) | Pure-Rust geospatial stack | Accepted |
| [0014](0014-python-core-retirement-criteria.md) | Python core retirement criteria | Accepted |
| [0015](0015-multi-wire-conductor-set-and-tension-sections.md) | Multi-wire conductor set and tension sections | Accepted (G7) |
| [0016](0016-structure-family-library-and-application-chart.md) | Structure family library and application chart | Accepted (G8) |
| [0017](0017-load-case-criteria-set-engine.md) | Load-case / criteria-set engine (IEC 60826) | Proposed |
| [0018](0018-two-tier-terrain-precision.md) | Two-tier terrain precision (interpolated ~1 m RoW) | Proposed |
| [0019](0019-route-poi-model-and-mandatory-angle-structures.md) | Route / POI model and mandatory structures at angle points | Accepted (G9; angle typing superseded by 0023) |
| [0020](0020-structure-geometry-and-tower-elevation-view.md) | Structure geometry and the tower-elevation view | Accepted (G10) |
| [0021](0021-bring-fem-forward-for-uneven-spans.md) | Bring FEM forward as the uneven-span section solver | Proposed |
| [0022](0022-srtm-area-selection-and-plan-view-route-editor.md) | SRTM area selection and the plan-view route editor | Proposed |
| [0023](0023-angle-as-deflection-derived-property.md) | Angle as a deflection-derived property, not a structure function | Proposed |
| [0024](0024-production-plan-and-profile-sheets.md) | Production plan & profile sheets (paginated, A1/A0, full label sets) | Proposed |

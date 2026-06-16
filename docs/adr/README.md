# Architecture Decision Records

This log captures the significant architectural decisions for ATLDP, with their
context and consequences. Format: lightweight [MADR](https://adr.github.io/madr/).

Most ADRs below are **proposed** — recommendations for discussion, not yet
ratified. ADR-0007 was **accepted** and implemented in Phase 0 (repository
hygiene); ADR-0003 and ADR-0008 were **accepted** and implemented in Phase 1 (the
sag-tension core). ADR-0011–0014 (**accepted**, 2026-06-15) commit the production
app to a native Rust desktop CAD stack: ADR-0011 supersedes ADR-0002, ADR-0012
resolves ADR-0006, and ADR-0013 refines the geospatial tooling of ADR-0005. See
the [implementation plan](../IMPLEMENTATION_PLAN.md).

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Proposed |
| [0002](0002-python-core-with-separated-layers.md) | Python computational core with separated layers | Superseded by 0011 |
| [0003](0003-analytic-sag-tension-baseline-before-fem.md) | Analytic sag-tension baseline before FEM | Accepted |
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

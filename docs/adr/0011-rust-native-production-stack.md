# ADR-0011 — Rust native production stack (supersedes ADR-0002)

- Status: Accepted
- Date: 2026-06-15
- Supersedes: [ADR-0002](0002-python-core-with-separated-layers.md)

## Context

ADR-0002 chose a **Python** computational core for fast iteration during the
study phase, deferring any compiled backend until a profile proved it necessary,
and deferring the GUI entirely (ADR-0006). That served Phase 1: the sag-tension
core is now built and validated (`core/`).

The product, however, is a **desktop CAD application** — interactive 2D and 3D
views over terrain, routes, structures, conductors, and **LiDAR point clouds of
millions of points** — with three hard requirements that Python/Qt bundling does
not meet well:

- **High, sustained rendering performance** (GPU, large point clouds, 60fps).
- **A small, optimized, self-contained binary** (target **< 30 MB**). A frozen
  Python + Qt + VTK distribution is ~80–150 MB and starts slowly.
- **Linux-first with a near-term Windows port** from a single, simple toolchain.

These are presentation- *and* runtime-level constraints, not a hot-path profile
finding. They argue for committing the production application to a compiled
language now rather than wrapping the Python core.

## Decision

Build the production application as a **single native binary in Rust**, organised
as a Cargo workspace that preserves the layered separation ADR-0002 intended:

1. **`atldp-core`** — pure numerics (geometry, catenary, change-of-state, ruling
   span, conductor). No I/O, no GPU. Fully unit-tested.
2. **`atldp-geo`** — geospatial ingestion (DEM, CRS, ground profile, LiDAR), see
   [ADR-0013](0013-pure-rust-geospatial-stack.md).
3. **`atldp-model`** — the serializable project model (ADR-0009).
4. **`atldp-render`** + **`atldp-app`** — the GPU rendering layer and the desktop
   shell, see [ADR-0012](0012-desktop-gui-wgpu-egui.md). A thin **`atldp-cli`**
   keeps today's scripting parity.

Rust is chosen over C++ for: cross-platform GPU via `wgpu` (Vulkan/DX12/Metal),
small statically-linked LTO binaries that realistically hit the < 30 MB target, a
trivial Windows cross-compile, and memory safety for a long-lived interactive
editor — without a heavyweight C++ CAD framework that would blow the footprint.

The existing **Python `core/` is retained as a transitional validation oracle**,
not the production engine; the Rust core must reproduce its results before it is
retired (see [ADR-0014](0014-python-core-retirement-criteria.md)). This keeps the
validate-first principle (ADR-0003) intact rather than discarding it.

## Consequences

- The product is a fast, small, portable native binary instead of a Python bundle.
- The numerically heavy and geospatial work is reimplemented in Rust (`nalgebra`,
  `roots`/`argmin`, pure-Rust geo per ADR-0013) — a real cost, bounded because the
  Phase 1 models are small and well specified, and validated against the oracle.
- Iteration is slower than Python during the port; mitigated by keeping `core/` as
  a fast prototyping and cross-check surface through the transition.
- ADR-0002's layered separation is preserved; only the substrate changes.

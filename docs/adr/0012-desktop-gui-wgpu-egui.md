# ADR-0012 — Desktop GUI and rendering: winit + wgpu + egui (resolves ADR-0006)

- Status: Accepted
- Date: 2026-06-15
- Resolves: [ADR-0006](0006-defer-gui-headless-first.md)

## Context

ADR-0006 deferred the GUI choice until the core was proven, to be made later in a
separate ADR. The core is now validated, and ADR-0011 commits the production app
to a native Rust binary. This ADR fixes the GUI and rendering stack.

The application is **CAD-like**: dockable panels around interactive viewports, a
**3D scene** (terrain meshes, conductors, towers, and **million-point LiDAR
clouds** with level-of-detail, plus orbit/pan/zoom, picking, measuring) and a
**2D plan-&-profile** drafting view (infinite canvas, grid, snapping, layers,
dimensions). It must render at interactive frame rates and fit the < 30 MB,
Linux-first / Windows-next constraints of ADR-0011.

## Decision

- **Windowing:** `winit` (Linux + Windows + macOS later).
- **GPU:** `wgpu` — one renderer over Vulkan (Linux), DX12 (Windows), Metal
  (macOS later). Shaders are WGSL embedded in the binary; no external runtime.
- **UI:** `egui` (immediate-mode) integrated via `egui-wgpu` / `egui-winit`, with
  `egui_dock` for dockable panels. Viewports are custom `wgpu` render surfaces
  that egui panels composite around. Prior art followed: **`rerun.io`**, a Rust +
  egui + wgpu app that renders massive point clouds and 3D scenes performantly in
  a compact binary.
- **3D engine** (`atldp-render`): orbit/pan/zoom CAD camera; DEM → indexed
  triangle mesh with quadtree LOD; conductors as catenary polylines/tubes from
  `atldp-core`; towers as instanced glyph/line geometry; picking via GPU id-buffer
  readback or CPU ray-cast against a spatial index.
- **LiDAR point cloud:** LAS/LAZ → an octree (potree-style chunking) streamed to
  the GPU with view-dependent LOD. This is the highest-risk component and is built
  in its own phase (G4).
- **2D CAD:** the same `atldp-render` crate under an orthographic camera; vector
  paths tessellated with `lyon` → wgpu for entity-heavy drawings, egui's painter
  for light overlays.
- **App architecture** (`atldp-app`): a **retained document/scene model** with a
  **command pattern + undo/redo** for modal tools (place tower, measure, edit
  route), layered on top of immediate-mode egui. Interaction state lives in the
  app, never in `atldp-render`.

## Consequences

- A single, portable, GPU-accelerated CAD shell that meets the footprint target.
- One rendering path serves both 2D and 3D, reducing duplication.
- Immediate-mode UI requires a deliberate retained model + command stack for CAD
  interactions; this is designed up front (G2), not retrofitted.
- The LiDAR LOD renderer is genuine engineering risk; isolated to phase G4 and
  guided by potree/rerun prior art.

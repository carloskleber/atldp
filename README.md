# ATLDP — Alternative Transmission Line Design Program

> An open-source alternative for the electromechanical design of overhead
> transmission lines (sag-tension, tower spotting, clearances, georeferencing).

**Status:** the production app is a **native Rust desktop CAD application** with
interactive 2D/3D views and a small optimized binary (ADR-0011/0012/0013). The
engineering models were first validated in a Python prototype; the Rust core now
reproduces it and is additionally cross-checked against the OTLS-Models reference,
so the **Python `core/` has been retired** (ADR-0014). The native track has
reached G8 — manual spotting, sag-tension, structure loads, calculation reports,
plan-&-profile sheets, and the open `.atldp` project format, now over a real
**multi-wire set** (phases + shield wires) split into **tension sections** at the
anchor structures, with a **structure-family library** whose application charts gate
each placement (G7/G8, ADR-0015/0016; `.atldp` schema v2 with a v1→v2 migration).
Phases **G9–G11** (ADR-0017–0018) are the next traced steps: standards load cases
(IEC 60826), a precise ~1 m right-of-way profile, and field stringing tables.

---

## Motivation

Provide a *free* (as in freedom) alternative for the electromechanical design of
transmission lines.

The application area is a niche, but a strategic one:

- Commercial design software is effectively a monopoly.
- Transmission-line auctions/bids in Brazil often mandate the use of that
  software's proprietary file formats, reinforcing lock-in.

An open model enables collaboration, R&D, independent testing/validation, and
auditability of the engineering criteria — while honoring intellectual property
through proper attribution and a copyleft license (GPLv3).

*Open source is not the same as freeware:* the value is in open code,
patent-free collaboration, and open file formats — commercial use of the
**service** remains possible under the license.

## The design workflow

A transmission-line project flows through ATLDP as a pipeline of stages, each one
consuming the previous stage's output (see ADR-0009):

1. **Terrain model** — load a 3D terrain, initially from public sources (DEMs,
   elevation APIs) and ultimately from professionally surveyed **LiDAR point
   clouds**. A coarse tier covers the wide area for routing; the committed
   right-of-way corridor is refined to a **precise ~1 m profile** by interpolation
   (ADR-0018).
2. **Line route** — lay out the route over the terrain, marking points of
   interest (angle points, crossings, obstacles, constraints).
3. **Tower placement (spotting)** — place structures along the route, each typed as
   **suspension or anchor (strain/dead-end)**; anchors divide the line into **tension
   sections**, each with its own ruling span and stringing traction. Structures are
   chosen from a **family library** by their application chart (ADR-0015/0016).
   Manual first; the target is **automatic spotting** that minimizes material cost
   subject to clearance and loading constraints.
4. **Sag-tension** — for **every wire** (three phase conductors per circuit plus
   shield / ground wires, each at its own tension and load), compute sag and tension
   across weather and **load cases** (temperature, extreme wind, construction,
   broken-wire — IEC 60826), including conductor **swing / blowout**, and verify
   tension limits and clearances **wire-to-ground** (governed by the lowest wire) and
   **between phases**.
5. **Structure modeling** — wind span / weight span / longitudinal span loads and
   structure load checks against the family's rating, including **guyed towers**,
   using a simple lattice-model representation.
6. **Plan & profile drafting** — produce the plan-and-profile sheets, calculation
   reports, and the field **stringing table** (sag/tension vs temperature).

Criteria are based on **IEC, IEEE, CIGRE, and ABNT (NBR 5422)** standards (ADR-0004),
encoded as a pluggable criteria set (ADR-0017), with calculation reports aligned to
national standards.

## Roadmap (high level)

1. **Study & prototype** — survey the state of the art (IEEE, CIGRE, ABNT) and
   prototype isolated pieces (terrain, catenary, change-of-state).
2. **Validated sag-tension core** — first built as a headless Python engine for
   stage 4 (the technical heart): inclined catenary, change-of-state, ruling span,
   a thin CLI, and a golden-case validation suite. *Done*, and it served as the
   **validation oracle** for the native port; now **retired** (ADR-0014) — its
   golden cases and cross-check fixtures live on in the Rust test tree.
3. **Native application** (done to G6) — reimplement the stack as a **native Rust**
   desktop CAD app: validated Rust core, then a wgpu/egui 2D+3D shell, terrain,
   LiDAR, spotting, and drafting (phases **G0–G6**; ADR-0011/0012/0013). Maps onto
   the same pipeline stages 1–6 for a single conductor.
4. **Real line model** (current) — grow the single-conductor tool into a real
   project: tension sections + multi-wire set (**G7, done**), structure-family
   library (**G8, done**), standards load cases (G9), a ~1 m right-of-way profile
   (G10), and field stringing tables (G11) — ADR-0015–0018.
5. **Automatic spotting** — the cost-minimizing optimizer for stage 3, selecting
   structure families and respecting load cases and section tractions.

The product workflow (pipeline stages) is unchanged; only the implementation
language moves from Python to Rust. See
[docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for the native build
track (G0–G8).

See [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for the detailed,
phased plan, and [docs/adr/](docs/adr/) for the architectural decisions and their
rationale.

## Project structure

```text
.
├── README.md
├── references.md           # Bibliography and related open-source projects
├── Cargo.toml              # Rust workspace — the native production app (ADR-0011)
├── crates/                 # Native crates (layered per ADR-0011)
│   ├── atldp-core/          #   pure sag-tension numerics, ruling-span sections + structure loads (no I/O, no GPU)
│   ├── atldp-geo/           #   DEM / CRS / LiDAR ingestion; coarse map + ~1 m corridor (pure-Rust, ADR-0013/0018)
│   ├── atldp-model/         #   project model (wire set, sections, structure library), .atldp format, reports, sheets & stringing table (ADR-0009/0015–0017)
│   ├── atldp-render/        #   wgpu 2D + 3D rendering (ADR-0012)
│   ├── atldp-app/           #   winit/egui desktop CAD shell (the binary)
│   └── atldp-cli/           #   thin CLI for scripting parity
├── docs/
│   ├── theory.md            # The sag-tension problem (math sketch, 3D notes)
│   ├── IMPLEMENTATION_PLAN.md
│   └── adr/                 # Architecture Decision Records
└── tests/                   # Throwaway prototypes, one folder per experiment
    └── terrain/             # 3D terrain navigation prototype (Plotly + DEM/API)
```

The native production app lives in [`crates/`](crates/) (ADR-0011). The Python
engine that was the original validation oracle has been retired (ADR-0014); its
golden cases and cross-check fixtures now live in the Rust test tree
([`crates/atldp-core/tests/`](crates/atldp-core/tests/), provenance in
[`ORACLES.md`](crates/atldp-core/tests/ORACLES.md)). `tests/` holds throwaway
prototype routines used to evaluate candidate models.

> **Note:** each prototype owns its own throwaway virtual environment, which must
> **not** be committed. See ADR-0007 and the `.gitignore`.

## Building and running

**Prerequisites:** Rust stable toolchain (`rustup`), a GPU with Vulkan drivers (Mesa on Linux, stock drivers on Windows).

```bash
# Desktop app (G2+ — 3D/2D CAD shell)
cargo run --release -p atldp-app

# Release binary only (~12 MB stripped on Linux)
cargo build --release -p atldp-app
strip target/release/atldp-app

# Headless CLI (catenary & change-of-state)
cargo run -p atldp-cli -- catenary --span 300 --weight 15.97 --tension 30000
cargo run -p atldp-cli -- cos --span 300 --ref-H 30000 --ref-temp 25 --target-temp 75

# Full test suite (Rust core golden cases + cross-check fixtures + model/format tests)
cargo test --workspace
```

The desktop app opens a docked shell with a live 3D orbit viewport (wgpu) on the left and a 2D profile viewport (egui) on the right. Spot towers in the profile, and the toolbar edits attachment height, clearance, tension, wind pressure, and vertical exaggeration; both viewports update in real time. The **Save / Load / Report / Sheet** toolbar buttons write the open `.atldp` project, a Markdown calculation report, and an SVG plan-&-profile sheet (G6).

|Control|Action|
|---|---|
|Left-drag (3D)|Orbit camera|
|Scroll (3D)|Zoom|
|Right-drag (2D)|Pan|
|Scroll (2D)|Zoom|
|🗼 Spot towers + left-click (2D)|Place a structure|
|💾 Save / 📂 Load|Write / read the `.atldp` project (`ATLDP_PROJECT`)|
|📄 Report / 🖼 Sheet|Export `atldp_report.md` / `atldp_profile.svg`|

## References

See [references.md](references.md) for theory, methods (analytic and FEM),
commercial software, and related open-source repositories.

## License

GPLv3 — see [LICENSE](LICENSE).

## Author

Carlos Kleber C. Arruda

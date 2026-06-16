# ATLDP — Alternative Transmission Line Design Program

> An open-source alternative for the electromechanical design of overhead
> transmission lines (sag-tension, tower spotting, clearances, georeferencing).

**Status:** transitioning from a validated Python prototype to the production app.
The engineering models are validated (Python `core/`, Phase 1); the production
target is now fixed: a **native Rust desktop CAD application** with interactive
2D/3D views and a small optimized binary (ADR-0011/0012/0013). The Python `core/`
is kept as a validation oracle until the Rust port reproduces it (ADR-0014).

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
   clouds** processed into a ground/feature model.
2. **Line route** — lay out the route over the terrain, marking points of
   interest (angle points, crossings, obstacles, constraints).
3. **Tower placement (spotting)** — place structures along the route. The first
   version is **manual**; the target is **automatic spotting** that minimizes
   overall material cost subject to clearance and loading constraints.
4. **Sag-tension** — compute sag and tension across weather cases (temperature,
   wind), including conductor **swing / blowout**, and verify cable tension limits
   and clearances **wire-to-ground** and **between phases**.
5. **Structure modeling** — wind span / weight span and structure load checks,
   including **guyed towers**, using a simple lattice-model representation.
6. **Plan & profile drafting** — produce the plan-and-profile sheets and reports.

Criteria are based on **IEEE, CIGRE, and ABNT (NBR 5422)** standards (ADR-0004),
with calculation reports aligned to national standards.

## Roadmap (high level)

1. **Study & prototype** — survey the state of the art (IEEE, CIGRE, ABNT) and
   prototype isolated pieces (terrain, catenary, change-of-state).
2. **Validated sag-tension core** — a headless Python engine for stage 4 (the
   technical heart). *Done* — see [`core/`](core/): inclined catenary,
   change-of-state, ruling span, a thin CLI, and a golden-case validation suite.
   Now the **validation oracle** for the native port (ADR-0014).
3. **Native application** (current) — reimplement the stack as a **native Rust**
   desktop CAD app: validated Rust core, then a wgpu/egui 2D+3D shell, terrain,
   LiDAR, spotting, and drafting (phases **G0–G6**; ADR-0011/0012/0013). Maps onto
   the same pipeline stages 1–6.
4. **Automatic spotting** — the cost-minimizing optimizer for stage 3.

The product workflow (pipeline stages) is unchanged; only the implementation
language moves from Python to Rust. See
[docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for the native build
track (G0–G6).

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
│   ├── atldp-core/          #   pure sag-tension numerics (no I/O, no GPU)
│   ├── atldp-geo/           #   DEM / CRS / LiDAR ingestion (pure-Rust, ADR-0013)
│   ├── atldp-model/         #   serializable project model (ADR-0009)
│   ├── atldp-render/        #   wgpu 2D + 3D rendering (ADR-0012)
│   ├── atldp-app/           #   winit/egui desktop CAD shell (the binary)
│   └── atldp-cli/           #   thin CLI for scripting parity
├── docs/
│   ├── theory.md            # The sag-tension problem (math sketch, 3D notes)
│   ├── IMPLEMENTATION_PLAN.md
│   └── adr/                 # Architecture Decision Records
├── core/                    # Validated Python engine — now the validation oracle
│   ├── src/atldp/           #   pure headless core + thin CLI (ADR-0002, retired per ADR-0014)
│   ├── tests/               #   unit tests
│   └── validation/          #   golden cases citing their sources (ADR-0008)
└── tests/                   # Throwaway prototypes, one folder per experiment
    └── terrain/             # 3D terrain navigation prototype (Plotly + DEM/API)
```

The native production app lives in [`crates/`](crates/) (ADR-0011). The validated
Python engine in [`core/`](core/) — see its [README](core/README.md) — is the
**oracle** the Rust core is checked against, and is retired once the ADR-0014 gate
is met. `tests/` holds throwaway prototype routines used to evaluate candidate
models.

> **Note:** each prototype owns its own throwaway virtual environment, which must
> **not** be committed. See ADR-0007 and the `.gitignore`.

## References

See [references.md](references.md) for theory, methods (analytic and FEM),
commercial software, and related open-source repositories.

## License

GPLv3 — see [LICENSE](LICENSE).

## Author

Carlos Kleber C. Arruda

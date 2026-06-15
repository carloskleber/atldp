# ATLDP — Alternative Transmission Line Design Program

> An open-source alternative for the electromechanical design of overhead
> transmission lines (sag-tension, tower spotting, clearances, georeferencing).

**Status:** early exploration / prototyping. No platform or language is fixed yet.
The goal of this phase is to study the state of the art and validate models before
committing to an implementation.

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

1. **Study & prototype** (current) — survey the state of the art (IEEE, CIGRE,
   ABNT), prototype isolated pieces (terrain, catenary, change-of-state) in any
   convenient language, and validate against published results and existing
   open repositories.
2. **Validated sag-tension core** — a headless engine for stage 4 (the technical
   heart of the project).
3. **Terrain, route & manual spotting** — stages 1–3, manual placement first.
4. **Structure modeling & drafting** — stages 5–6.
5. **Automatic spotting** — the cost-minimizing optimizer for stage 3.

See [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for the detailed,
phased plan, and [docs/adr/](docs/adr/) for the architectural decisions and their
rationale.

## Project structure

```text
.
├── README.md
├── references.md           # Bibliography and related open-source projects
├── docs/
│   ├── theory.tex/.pdf      # The sag-tension problem (math sketch)
│   ├── IMPLEMENTATION_PLAN.md
│   └── adr/                 # Architecture Decision Records
└── tests/                   # Throwaway prototypes, one folder per experiment
    └── terrain/             # 3D terrain navigation prototype (Plotly + DEM/API)
```

During this phase, `tests/` holds prototype routines, in no particular language,
used to evaluate the candidate models. Based on the results — and possibly
external contributions — a proper software model will be chosen for
implementation.

> **Note:** each prototype owns its own throwaway virtual environment, which must
> **not** be committed. See ADR-0007 and the `.gitignore`.

## References

See [references.md](references.md) for theory, methods (analytic and FEM),
commercial software, and related open-source repositories.

## License

GPLv3 — see [LICENSE](LICENSE).

## Author

Carlos Kleber C. Arruda

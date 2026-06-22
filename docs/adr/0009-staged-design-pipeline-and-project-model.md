# ADR-0009 — Staged design pipeline and shared project model

- Status: Proposed (stage sequence revised 2026-06-21, ADR-0022)
- Date: 2026-06-15

## Context

A transmission-line project is produced as an ordered set of engineering stages.
Stages 1–3 **set up** the project — independent definition/selection steps — and
stages 4–9 form a **computational pipeline** in which each stage consumes the previous
stage's output (sequence revised 2026-06-21; the original list ran
terrain → route → spotting → sag-tension → structure → drafting):

1. **Define the project** *(setup)* — endpoints (initial/final points), transmission
   capacity, and voltage level; endpoints are picked on an overall basemap
   (ADR-0013/0022) and frame every downstream stage.
2. **Standards & criteria** *(setup)* — the applicable standards (by voltage/region)
   that fix the load cases, wind/ice parameters, right-of-way widths, and clearances
   (ADR-0017).
3. **Tower families** *(setup)* — the candidate structure families (by voltage,
   circuits, and application chart) drawn from a library/database (ADR-0016).
4. **Terrain model** — over the area the endpoints span, public DEM first,
   professionally surveyed **XYZ / LiDAR point clouds** later (ADR-0018).
5. **Line route** — an explicit polyline of **points of interest (POIs)** over the
   terrain; the **2-D profile is derived from it** by sampling the terrain along the
   polyline (ADR-0019). The profile is an *output* of this stage, not an input to it.
6. **Tower placement (spotting)** — manual first, automatic later (ADR-0010); a
   structure is **obligatory at every angle (deflection) POI** (ADR-0019), typed
   suspension or anchor with angle a deflection-derived property (ADR-0023).
7. **Sag-tension** — temperature/wind cases, swing/blowout, tension limits, and
   wire-to-ground and phase-to-phase clearances.
8. **Structure modeling** — wind/weight span and load checks, including guyed
   towers, with a simple lattice-model representation.
9. **Plan & profile drafting** — sheets and reports.

This sequence is the product's backbone. It needs an architecture where stages
are decoupled (so each can be prototyped, validated, and replaced independently)
yet share a consistent representation of the project. The *setup* stages are
order-tolerant definition steps; the *pipeline* stages have the strict input→output
dependency above.

## Decision

Model the application as a **pipeline of stages over a single, serializable
project model**. Each stage:

- reads the project model, writes its own results back into it, and declares the
  upstream stages it depends on;
- is independently testable with a fixed input project model;
- never reaches around the model to depend on another stage's internals.

The project model (terrain, route, structures, conductors, weather cases,
results) is the integration contract and the unit of serialization (ADR-0006
keeps it headless; the file format is defined in Phase 4). The *build* order
(start with the sag-tension core, runtime stage 7) is deliberately different from this
*runtime* order, because that core is the highest-risk component and the validation
oracle for the rest (ADR-0003, ADR-0008).

## Consequences

- Stages can be developed, validated, and swapped in isolation (e.g. DEM → LiDAR;
  manual → automatic spotting) without disturbing the rest.
- A re-run after an upstream change can invalidate downstream results; the model
  must track which results are stale.
- Requires investing early in a clear project-model schema, before the file
  format is finalized.

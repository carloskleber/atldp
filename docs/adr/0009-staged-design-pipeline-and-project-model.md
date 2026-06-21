# ADR-0009 — Staged design pipeline and shared project model

- Status: Proposed
- Date: 2026-06-15

## Context

A transmission-line project is produced as an ordered sequence of engineering
stages, each consuming the previous stage's output:

1. **Terrain model** — public DEM first, professionally surveyed **LiDAR point
   clouds** later.
2. **Line route** — an explicit polyline of **points of interest (POIs)** over the
   terrain; the **2-D profile is derived from it** by sampling the terrain along the
   polyline (ADR-0019). The profile is an *output* of this stage, not an input to it.
3. **Tower placement (spotting)** — manual first, automatic later (ADR-0010); **angle
   POIs oblige a structure** at their station (ADR-0019).
4. **Sag-tension** — temperature/wind cases, swing/blowout, tension limits, and
   wire-to-ground and phase-to-phase clearances.
5. **Structure modeling** — wind/weight span and load checks, including guyed
   towers, with a simple lattice-model representation.
6. **Plan & profile drafting** — sheets and reports.

This sequence is the product's backbone. It needs an architecture where stages
are decoupled (so each can be prototyped, validated, and replaced independently)
yet share a consistent representation of the project.

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
(start with stage 4, the sag-tension core) is deliberately different from this
*runtime* order, because stage 4 is the highest-risk component and the validation
oracle for the rest (ADR-0003, ADR-0008).

## Consequences

- Stages can be developed, validated, and swapped in isolation (e.g. DEM → LiDAR;
  manual → automatic spotting) without disturbing the rest.
- A re-run after an upstream change can invalidate downstream results; the model
  must track which results are stale.
- Requires investing early in a clear project-model schema, before the file
  format is finalized.

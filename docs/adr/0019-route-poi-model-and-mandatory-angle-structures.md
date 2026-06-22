# ADR-0019 — Explicit route / POI model and mandatory structures at angle points

- Status: Accepted (delivered G9, 2026-06-21)
- Date: 2026-06-21
- Superseded in part by: [ADR-0023](0023-angle-as-deflection-derived-property.md)
  (angle is a deflection-derived property, not a structure function)
- Builds on: [ADR-0009](0009-staged-design-pipeline-and-project-model.md) (pipeline),
  [ADR-0010](0010-manual-spotting-before-cost-optimization.md) (spotting),
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md) (structure typing)

## Context

The pipeline (ADR-0009) is **terrain → route → spotting → sag-tension → …**, but the
G6–G8 project model skipped straight from terrain to spotting: `Project` holds a
`ground_profile` (a 1-D list of sampled elevations) and a flat `towers: Vec<Tower>`
placed by `distance_m`. **There is no route object.** The profile is treated as the
primary artefact and towers are dropped onto it directly.

That inverts the engineer's actual first move. On a blank terrain the work is, *in
order*:

1. **Define the route** — pick the georeferenced **points of interest (POIs)** that
   the line must pass through: the two terminals, the **angle points** where the line
   changes plan direction, and crossings / obstacles / constraints. The route is the
   polyline through those POIs.
2. **Derive the 2-D profile** — sample the terrain *along that polyline* (cumulative
   ground distance vs. elevation). The profile is an **output of the route**, not an
   independent input. It only exists once a route exists.
3. **Spot structures** on the derived profile.

Two facts fall out of this that the flat model cannot express:

- A POI where the line **changes direction** is not optional scenery: a conductor
  cannot turn in mid-span, so an **angle point obliges a structure at that exact
  station**. The deviation angle there is a real load (the bisector pull) and forces
  the structure to be at least an **angle** type — frequently an **anchor**.
- The route distance is measured **along the plan polyline**, so the profile's
  horizontal axis and every tower `distance_m` are defined by the route geometry, not
  by an arbitrary baseline.

## Decision

Introduce an explicit **route model** upstream of the profile and spotting, leaving
`atldp-core` untouched:

- A **`Route`** is an ordered list of **`Poi`** vertices, each georeferenced
  (CRS-projected x/y, with terrain elevation sampled at G10/ADR-0018 precision) and
  **kinded**: `Terminal` (start/end), `Angle` (deviation point), `Crossing`,
  `Obstacle`, `Constraint`. An `Angle` POI carries its **deviation angle**.
- The **ground profile is derived** from the route: `geo::extract_profile` samples the
  DEM along the route polyline, so `Project.ground_profile` becomes a *computed*
  product of `Project.route` and the terrain, re-extracted whenever either changes
  (the staleness contract of ADR-0009).
- **Mandatory structures at angle POIs.** Each `Angle` (and `Terminal`) POI
  **pins a structure** at its station; the engineer may not delete it and may not set
  its function below what the deviation demands (an `Angle` POI's structure is at least
  [`StructureFunction::Angle`](../../crates/atldp-model/src/lib.rs); terminals are
  [`Anchor`](../../crates/atldp-model/src/lib.rs)). Its `line_angle_deg` is driven by
  the POI, not edited free-hand. Tangent (suspension) towers between POIs remain freely
  placeable and movable, as today.
- **Editing after spotting** (the ADR-0010/0016 requirement, made concrete here): the
  app presents an **editable tower table**. Selecting a row edits a structure's
  function (suspension ↔ angle ↔ anchor), family, and height; changing the function
  **re-partitions the tension sections live** (`tension_sections`,
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md)) and re-runs the
  analysis. Pinned (POI-derived) structures expose only the edits their POI permits.

The `.atldp` `SCHEMA_VERSION` bumps to **3**: the format gains `route` and the
per-tower link to its originating POI (if any). The migration seam in
`format::from_atldp_str` carries a v2 project forward by synthesising a trivial
two-POI route (terminal → terminal) from the existing profile endpoints, so v2
projects keep reproducing their results (round-trip tested, per ADR-0015's pattern).

## Consequences

- The product finally matches the pipeline order: **blank terrain → route/POIs →
  profile → spotting**, with the profile visibly derived from the route.
- Angle points can no longer be left un-structured — a whole class of invalid designs
  becomes unrepresentable, and the deviation load has a definite home for the
  structure-load check (ADR-0017's transverse/bisector case).
- The route is the natural anchor for the plan (top) view of the plan-&-profile sheet
  and for the future automatic-spotting search domain (it fixes the obligatory
  structures the optimiser must keep).
- A schema migration (v2 → v3) must be written and round-trip tested; the
  `ground_profile` becomes derived state that the model must recompute and not trust
  blindly when loaded.
- Cross-references: the editable-table / type-change behaviour realises the "edit the
  structure after spotting" consequence of ADR-0016; the tower-elevation view that the
  table opens is ADR-0020.

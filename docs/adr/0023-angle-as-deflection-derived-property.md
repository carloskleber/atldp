# ADR-0023 — Angle is a deflection-derived property, not a structure function

- Status: Proposed
- Date: 2026-06-21
- Builds on: [ADR-0016](0016-structure-family-library-and-application-chart.md) (application chart),
  [ADR-0019](0019-route-poi-model-and-mandatory-angle-structures.md) (route / POI model)
- Supersedes (in part): ADR-0019's treatment of "angle" as a structure function

## Context

G9 (ADR-0019) shipped a [`StructureFunction`](../../crates/atldp-model/src/lib.rs)
with **three** variants — `Suspension`, `Angle`, `Anchor` — and made an `Angle` POI pin
an `Angle` structure (`PoiKind::pinned_function`). That conflates two independent
properties of a structure:

- its **mechanical function** — does it terminate a tension section? Only two answers:
  **suspension** (insulators swing, tension continues through) or **anchor**
  (strain / dead-end, tension is terminated). This is what partitions the line into
  tension sections (ADR-0015).
- whether it sits at a **plan deflection** — an "angle structure". An angle structure
  is **not a third function**: it can be either a *suspension* (a running / light angle)
  or an *anchor* (a strain / heavy-angle dead-end). Which one is an engineering choice,
  bounded by what the family's application chart allows.

The chart already carries this: `ApplicationChart::max_line_angle_deg` (ADR-0016) is the
deviation a family tolerates, and `Poi::deviation_angle_deg` (ADR-0019) is the deflection
at the POI. "Angle" is therefore *derivable* (the POI has a deflection) and *gated* (the
chosen family's `max_line_angle_deg` ≥ that deflection) — it never needed to be a
function variant. The three-variant enum also misleads tension-sectioning: an `Angle`
was treated as non-section-breaking, but a strain angle **does** break the section.

## Decision

Make **angle a property of the location, not a function of the structure.**

- **`StructureFunction` becomes two-valued:** `Suspension` and `Anchor`. The `Angle`
  variant is removed.
- **An "angle structure" is derived:** a structure whose origin POI has a non-zero
  `deviation_angle_deg`. It may be a suspension (running angle) or an anchor (strain
  angle); the descriptor is for display/reporting, not a stored function.
- **Angle POIs still oblige a structure, but not a function.** `PoiKind::Angle` requires
  *a* structure at its station (a conductor cannot turn in mid-span) without fixing
  suspension-vs-anchor; the engineer (later, the optimiser) chooses, defaulting to the
  lightest function whose family chart covers the deflection. `PoiKind::Terminal` still
  pins an `Anchor`. (`pinned_function` splits into "is a structure required here?" and an
  optional fixed function — only terminals fix one.)
- **The chart is the gate.** A placement at a deflection POI is valid only if the chosen
  family's `max_line_angle_deg` ≥ the POI's deviation, checked alongside the existing
  wind-span × weight-span envelope (ADR-0016). No separate "angle rating" path.
- **Tension sections fall out correctly:** a deflection carried by a *suspension* stays
  inside its section; a deflection carried by an *anchor* terminates it — exactly the
  `is_anchor` rule already used, now with no special-case for a removed `Angle`.

This corrects delivered G9 code, so it carries a schema step: `.atldp`
`SCHEMA_VERSION` bumps and the migration maps any stored `Angle` function to a
`Suspension` (the running-angle default) at a POI that keeps its deviation, round-trip
tested per the ADR-0015/0019 pattern.

## Consequences

- The model matches practice: "suspension vs anchor" is the function; "angle" is where
  the structure sits. Reporting can still label angle structures by reading the POI.
- Touch points: `StructureFunction` (drop `Angle`, fix `label`), `PoiKind::pinned_function`
  (required-vs-fixed split), `tension_sections` (no `Angle` case), the editable tower
  table (function selector is two-valued; deflection shown from the POI), the application
  chart violation check (deviation gate), tests, and a schema migration.
- The automatic-spotting optimiser (future) selects function + family against the chart's
  deviation gate rather than against a phantom `Angle` function — a cleaner search space.
- ADR-0019 stays accepted for the route/POI model; only its "angle is a function" detail
  is superseded here.

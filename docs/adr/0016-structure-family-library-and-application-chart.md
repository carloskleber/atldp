# ADR-0016 — Structure family library and application (usage) chart

- Status: Proposed
- Date: 2026-06-20
- Builds on: [ADR-0010](0010-manual-spotting-before-cost-optimization.md) (spotting),
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md) (structure typing)

## Context

A spotted `Tower` in the G6 model is just a point with a base elevation and a single
attachment height. A real designer works from a **library of structure families**:
standardised tower designs (e.g. a tangent suspension family, a small-angle family,
a heavy-angle/dead-end family), each available in a range of **heights (body +
extensions)** and each rated by an **application chart** — the manufacturer/utility
curve that says, for that family at that height, the **allowable combination of wind
span and weight span** (and angle). Spotting a structure means *choosing a family and
height whose application chart envelopes the spans and loads at that location*; the
automatic optimizer (Phase 5) does the same choice by search.

`atldp_core::structure::structure_loads` already returns the **wind span** and the
(signed) **weight span** at a support — exactly the two axes of the application
chart. So the gap is again representational: there is no family, no chart, and a
`Tower` cannot reference one or be edited after placement.

## Decision

Introduce a **structure family library** in the project model:

- A **`StructureFamily`** carries: a name, its mechanical **function** (suspension /
  angle / anchor, per ADR-0015), an **effective height** (the conductor attachment
  height the catenary sees) selectable across the family's height range, per-wire
  **attachment offsets**, and an **application/usage chart** — the allowable
  **wind-span × weight-span (× line-angle)** envelope.
- A spotted **`Tower` references a family** (+ chosen height) and may **override** the
  effective height and the application chart for that individual structure — the
  "user can edit the structure after spotting" requirement.
- **Manual spotting** evaluates each placement's wind/weight spans
  (`structure_loads`) against the referenced family's chart and flags a structure
  whose loads fall outside its envelope, alongside the existing clearance/tension
  checks (ADR-0010).
- **Automatic spotting** (Phase 5) treats the library as its selection set: it
  *chooses* the cheapest family+height whose chart still envelopes the spans — the
  manual evaluator is its constraint oracle, so both paths agree by construction.

The library ships with a small built-in set and is extensible per project; chart data
is the family's own (no standard tables are redistributed — ADR-0004).

## Consequences

- Spotting becomes a real engineering decision (pick a structure that *fits*), not
  just placing a point at a height; violations of a structure's own rating are caught.
- The same `structure_loads` output feeds both the load report and the chart check —
  one source of truth.
- Automatic spotting (Phase 5) has a well-defined discrete decision variable (family +
  height) and a checkable feasibility test, which is what makes the optimization
  tractable.
- Requires a data model for application charts (piecewise envelope) and a UI to edit a
  structure's family/height/overrides after placement; the format gains the library
  and the per-tower reference (schema bump, coordinated with ADR-0015's v2).

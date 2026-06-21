# ADR-0015 — Multi-wire conductor set and tension sections

- Status: Proposed
- Date: 2026-06-20
- Builds on: [ADR-0009](0009-staged-design-pipeline-and-project-model.md) (project
  model), [ADR-0010](0010-manual-spotting-before-cost-optimization.md) (spotting)

## Context

The G6 model strings **one** conductor along the line at **one** global horizontal
tension (`Project.conductor`, `Parameters.horizontal_tension_n`). A real
transmission line is not one wire at one tension:

- It carries **multiple wires**: three phase conductors per circuit (often two
  circuits) plus one or two **shield / ground (OPGW) wires**. Each wire has its own
  conductor type, its own attachment point on the structure, and — because shield
  wires are strung tighter and phases looser — its own **tension and load**.
- The line is divided into **tension sections** by **anchor (strain / dead-end)
  structures**. Within a section the suspension insulators swing freely and equalise
  a single horizontal tension; at an anchor the tension is terminated and a new
  section — possibly with a **different stringing tension (traction)** — begins.

The mechanics for this already exist in the validated core: `atldp_core::ruling_span`
computes a `Section`'s ruling span (`RS = sqrt(Σ Sᵢ³ / Σ Sᵢ)`) and applies the
common tension back to each real span. What is missing is the **project-model
representation** that lets the app *build* those sections and carry per-wire data.
Today's flat `towers: Vec<Tower>` has no structure type, and there is exactly one
conductor and one tension, so no section can be formed.

## Decision

Extend the project model (and the views/reports that consume it) with two coupled
concepts, leaving `atldp-core` numerics unchanged:

1. **Structure function typing.** Each spotted structure is typed by its mechanical
   function — **suspension**, **angle (suspension)**, or **anchor (strain /
   dead-end)**. A `terminal` is an anchor. This typing is the only thing needed to
   partition the ordered structures into **tension sections**: a section runs from
   one anchor to the next, and its spans feed `atldp_core::ruling_span::Section`.

2. **Wire set.** Replace the single `Project.conductor` with an ordered set of
   **wires** — N phase conductors (3 · circuits) plus the shield/ground wire(s).
   Each wire carries its own conductor spec, attachment geometry (offset from the
   structure reference point), and **per-section stringing tension**. Each wire is an
   independent catenary; clearance is checked **wire-by-wire**, and the view offers a
   **"lowest wire only"** mode (the governing wire for ground clearance) alongside
   "all wires".

The `.atldp` `SCHEMA_VERSION` bumps to **2**. The existing migration seam in
`atldp_model::format::from_atldp_str` (which already rejects a *newer* schema and is
the documented hook for upgrading an *older* one) carries a v1 single-conductor
project forward by wrapping its conductor as a one-wire set and treating every
structure as an anchor (a single trivial section), preserving its results.

Per-section **traction** (stringing tension) is a property of the (wire, section)
pair, defaulting from the criteria set (ADR-0017) but user-overridable — this is the
"different tractions per stretch" the engineer needs.

## Consequences

- The ruling span gains a concrete home in the product: sections are visible,
  editable, and each reports its own ruling span and tension — closing the "where is
  the ruling span defined?" gap.
- Sag-tension, clearance, drafting (sheet/report), and the stringing table (G11) all
  iterate over (wire × section) instead of a single conductor; the analysis pass in
  `atldp_model::analysis` becomes the place that fans out and is the single source of
  truth for every consumer.
- Structure typing is the prerequisite for structure families (ADR-0016) and for the
  load-case combinations (ADR-0017, e.g. broken-wire longitudinal load at anchors).
- A schema migration must be written and round-trip tested; v1 projects must keep
  reproducing their prior results.

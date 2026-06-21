# ADR-0017 — Load-case / criteria-set engine (IEC 60826 first)

- Status: Proposed
- Date: 2026-06-20
- Extends: [ADR-0004](0004-standards-baseline.md) (standards baseline)
- Builds on: [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md)

## Context

ADR-0004 committed to **standards-based, auditable criteria** implemented as a
*pluggable "criteria set"*, but that hook is not yet realised: the G6 app applies a
single static `wind_pressure_pa` and one everyday tension. A line design must instead
be checked against a **set of load cases** drawn from a governing standard. For
**IEC 60826** the core mechanical cases are:

- **Everyday (EDS)** — the long-term tension limit (creep / aeolian fatigue).
- **Extreme wind** — the high-wind reliability case (transverse load, conductor
  swing/blowout, structure transverse load).
- **Construction / maintenance loads** — stringing and installation conditions,
  including lifting and operator loads (a *safety/construction* limit state).
- **Broken-wire / unbalanced longitudinal** — a failure-containment case: one or more
  conductors broken, producing an unbalanced longitudinal load at the structure
  (governs anchors / dead-ends especially).
- **Minimum temperature** — the low-temperature high-tension case.

These differ in two independent ways: the **weather/temperature state** (a
change-of-state target on the section ruling span — already handled by
`atldp_core::change_of_state`) and the **structure load combination** they produce
(transverse + vertical + longitudinal, with per-case factors).

## Decision

Add a **load-case / criteria-set engine** layered on the validated core:

- A **criteria set** names a standard (IEC 60826 first; NBR 5422 and others added as
  further sets) and enumerates its **load cases**. Each case is `{ weather/temperature
  state, load factors, structure-load combination, governing limit }`.
- Each case's mechanical state is **one change-of-state target sharing the section
  ruling span** (ADR-0015), so the engine *reuses* the existing solver — the engine
  adds the case catalogue and the load-combination/limit bookkeeping, not new cable
  numerics.
- The structure-load combination assembles the per-wire transverse, vertical, and
  (for broken-wire) **longitudinal** loads (extending
  `atldp_core::structure::structure_loads`, which today returns transverse + vertical)
  and compares them to the structure family's rating (ADR-0016) and the conductor
  tension limits (% RTS).
- Criteria are **cited at the point of use** and selectable per project (ADR-0004), so
  results trace to a named clause and a different jurisdiction is a different set, not
  a code change.

## Consequences

- Design verification moves from "one wind pressure" to "passes every case of the
  governing standard", which is what a real project must demonstrate.
- The broken-wire/longitudinal case gives anchors (ADR-0015) a load they uniquely
  carry, justifying the suspension/anchor distinction structurally as well as
  mechanically.
- The engine is additive over validated numerics — the cable solver is untouched, so
  the existing golden/cross-check suite still gates correctness (ADR-0008).
- Encoding standard *methods and the user's parameters* (not copyrighted text/tables)
  must be respected per ADR-0004; multiple criteria sets are a maintenance surface.

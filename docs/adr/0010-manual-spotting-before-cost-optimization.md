# ADR-0010 — Manual spotting before cost-minimizing optimization

- Status: Proposed
- Date: 2026-06-15

## Context

Tower placement ("spotting") is stage 6 of the pipeline (ADR-0009). The ideal is
**automatic spotting**: place structures to **minimize overall material cost**
subject to constraints — ground and phase clearances, conductor tension limits,
allowable structure loading (wind/weight span), and structure-type availability.

That is a constrained combinatorial optimization problem that *depends on* every
downstream check (sag-tension, clearances, structure loading) being correct and
fast. Building the optimizer before those checks exist and are validated would
optimize against an unreliable objective.

## Decision

Implement **manual spotting first**: the engineer places structures along the
profile, and the system evaluates each placement against the validated stage 7/8
checks (clearances, tension, structure loading), flagging violations.

Spotting runs on the **profile derived from the route** (ADR-0019), not on a
free-standing profile: the route's **angle POIs pin obligatory structures**
(a conductor cannot change plan direction in mid-span), and the engineer places the
tangent structures between them. Placed structures are **editable after spotting** — a
table row changes a structure's function (suspension ↔ angle ↔ anchor), family, and
height, which re-partitions the tension sections and re-runs the checks live
(ADR-0015/0016/0019).

Treat **automatic, cost-minimizing spotting as a later phase** (Phase 5) layered
on top of the *same* placement-evaluation functions used by manual spotting. The
manual evaluator becomes the optimizer's objective/constraint oracle, so both
paths agree by construction.

## Consequences

- A usable, trustworthy spotting workflow early, with the engineer in control.
- The hard optimization problem is deferred until its building blocks are
  validated, and reuses them rather than reimplementing checks.
- Manual spotting on long lines is laborious until the optimizer arrives — an
  accepted interim cost.

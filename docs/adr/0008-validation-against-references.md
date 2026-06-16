# ADR-0008 — Validate against published references

- Status: Accepted (policy adopted; suite seeded in Phase 1, 2026-06-15)
- Date: 2026-06-15

## Context

A free design tool is only credible if its numbers are demonstrably correct.
`references.md` collects theory papers, a worked sag-tension example
(`mpewsey`), and several open-source implementations (`OnSag`, `SSTC`,
`Transmission_Line_Simulation_MATLAB`, and others) that can serve as independent
oracles. Auditability is a stated project goal.

## Decision

Every numerical model must reproduce at least one independent reference result
before it is trusted. Maintain a `validation/` suite of **golden cases**, each
one:

- citing its source (paper, standard example, or named open repo),
- pinning expected outputs and an explicit numerical tolerance,
- runnable in CI as part of the normal test run.

Where two ATLDP methods overlap (e.g. analytic core vs. FEM — ADR-0003), they
must agree within tolerance. A tolerance regression is treated as a build failure.

## Consequences

- Results are defensible and auditable; regressions are caught automatically.
- Up-front effort to curate reference cases and digitize expected values.
- Some third-party results carry their own assumptions; each golden case
  documents the assumptions it was validated under.

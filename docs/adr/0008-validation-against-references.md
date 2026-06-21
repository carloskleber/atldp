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

## Status (2026-06-16)

The independent third-party reference is **OTLS-Models** (Overhead Transmission
Line Software), vendored as the `third_party/Models` git submodule @ `c270d48` and
digitised in `crates/atldp-core/tests/ORACLES.md`. The Rust catenary reproduces its
`catenary_test.cc` numbers (`crates/atldp-core/tests/golden_otls_models.rs`). The
sibling `OnSag` project (a wxWidgets GUI consuming a precomputed tension table) was
not used directly because OTLS-Models *is* its headless numeric engine and the
directly comparable oracle. This closes ADR-0014 gate 3.

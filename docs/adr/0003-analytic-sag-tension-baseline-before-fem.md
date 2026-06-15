# ADR-0003 — Analytic sag-tension baseline before FEM

- Status: Proposed
- Date: 2026-06-15

## Context

`theory.tex` lays out two modeling regimes: the **exact catenary** (required for
span-to-depth ratio > 1/8 or inclined supports) and the **parabolic
approximation** (valid for shallow sags with level supports). The references also
point to a more general **finite-element** route (Bertrand 2020/2022; Sugiyama
2003) that handles inclined/uneven spans and dynamics, at much higher
implementation and validation cost.

We need trustworthy sag-tension numbers as early as possible, and a fixed yardstick
to validate anything more elaborate.

## Decision

Build the **analytic core first**: exact catenary + parabolic approximation, the
**change-of-state equation**, and a **ruling-span** section model. Select catenary
vs. parabola by the documented span-to-depth-ratio / support-inclination rule.

Treat **FEM as a later, optional track** (Phase 4), introduced only where the
ruling-span assumptions break down (strongly uneven/inclined spans, dynamics).
When implemented, FEM must agree with the analytic core in their overlapping
domain (see ADR-0008).

## Consequences

- Usable, auditable results early, against which everything else is checked.
- The analytic core is the canonical reference and the validation oracle.
- Some real configurations (very uneven spans, dynamics) wait for Phase 4.

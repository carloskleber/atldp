# ADR-0003 — Analytic sag-tension baseline before FEM

- Status: Accepted (implemented in Phase 1, 2026-06-15); **FEM *timing* amended by
  [ADR-0021](0021-bring-fem-forward-for-uneven-spans.md)** (2026-06-21)
- Date: 2026-06-15

> **Amendment (ADR-0021, 2026-06-21).** This ADR's *principle* stands — the analytic
> core is built first and remains the validation oracle. Its *sequencing* is revised:
> because each structure can have a different attachment point, uneven spans are the
> **common** case and the ruling-span assumption breaks down early, so the FEM track is
> **brought forward** (phase G11) for the static uneven-span section solve, validated
> against this analytic core in their overlap. The dynamic FEM/ROM track remains later.
> See ADR-0021.

## Context

`theory.md` lays out two modeling regimes: the **exact catenary** (required for
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
- Some real configurations wait for the FEM track: **uneven spans are brought forward
  to G11** (ADR-0021); dynamics remain a later research track.

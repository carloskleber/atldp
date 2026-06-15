# ADR-0001 — Record architecture decisions

- Status: Proposed
- Date: 2026-06-15

## Context

ATLDP is in an exploratory phase with no fixed platform or language. Decisions
made now (modeling approach, standards, geospatial strategy) will be expensive to
reverse later and need to be auditable — auditability of engineering criteria is
itself a stated project goal.

## Decision

Record every architecturally significant decision as an ADR in `docs/adr/`, using
the lightweight MADR format (context → decision → consequences). ADRs are
immutable once accepted; a superseded decision gets a new ADR that references it.

## Consequences

- New contributors can reconstruct *why* the project is the way it is.
- Engineering criteria and their normative basis are traceable.
- Small ongoing cost: each significant change comes with an ADR.

# ADR-0004 — Standards baseline (IEEE / CIGRE / ABNT)

- Status: Proposed
- Date: 2026-06-15

## Context

A core selling point of ATLDP is *auditable, standards-based* engineering
criteria. Brazilian transmission-line projects are governed by **ABNT NBR 5422**;
international practice draws on **IEEE** and **CIGRE**. Criteria must be explicit
and citable, not buried as magic numbers.

## Decision

Adopt this normative baseline and cite it at the point of use in code/docs:

- **ABNT NBR 5422** — design loads, wind, electrical clearances, right-of-way
  (the Brazilian baseline; primary target).
- **IEEE Std 738** — conductor thermal rating / ampacity.
- **CIGRE TB 601** — sag-tension and conductor high-temperature behavior;
  **CIGRE TB 324** — conductor stress-strain / creep.
- **IEC 60826** / Aluminum Association methods — supporting load and conductor
  models where useful.

Criteria are implemented as a **pluggable "criteria set"** so that a project can
select the governing standard rather than having it hard-coded.

## Consequences

- Results are traceable to a named clause of a named standard — supports audit.
- Multiple jurisdictions can be supported by adding criteria sets.
- The standards themselves are copyrighted: ATLDP encodes *methods and the
  user's own parameters*, and references clauses, but does not redistribute
  standard text or proprietary tables.

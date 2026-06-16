# ADR-0014 — Python core retirement criteria

- Status: Accepted
- Date: 2026-06-15
- Relates to: [ADR-0011](0011-rust-native-production-stack.md),
  [ADR-0003](0003-analytic-sag-tension-baseline-before-fem.md),
  [ADR-0008](0008-validation-against-references.md)

## Context

ADR-0011 reimplements the engineering engine in Rust (`atldp-core`) and keeps the
validated Python `core/` as a **transitional oracle** rather than discarding it.
This preserves validate-first (ADR-0003): the new implementation must be proven
against the old one and against independent references before the old one goes.
The open question is the explicit, auditable bar for **when `core/` is retired**.

## Decision

Retire the Python `core/` only when **all** of the following hold:

1. **Golden-case parity.** Every validation golden case in `core/validation/`
   (ADR-0008) is re-encoded as an `atldp-core` Rust test and passes within its
   stated tolerance.
2. **Cross-implementation agreement.** A cross-check harness compares Rust vs.
   Python outputs over a parameter sweep (spans, inclinations, tensions,
   temperatures, conductor states) and agrees within explicit tolerances; a
   tolerance regression is a build failure (ADR-0008).
3. **Independent reference.** The Rust core also matches at least one
   **third-party numeric reference** (the still-open OnSag/SSTC or a digitised
   IEEE/textbook table tracked in `core/validation/README.md`), closing the open
   Phase 1 validation item — so retirement does not rest on the Python core alone.

When all three are met, `core/` and its Python tooling are removed in a single
commit that cites this ADR; the Rust golden cases and cross-reference fixtures
remain as the permanent oracle.

## Consequences

- Retirement is a verifiable gate, not a judgement call, and is auditable.
- The Python prototyping/cross-check surface stays available through the port
  (phase G1) and no longer after — bounding maintenance of two engines.
- Forces the long-open third-party numeric cross-check to be resolved as part of
  closing out the engine, which strengthens the project regardless of language.

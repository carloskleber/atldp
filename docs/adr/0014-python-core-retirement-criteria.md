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

### Status update (2026-06-16) — all gates met

Gate 3 is closed by **OTLS-Models**, vendored as the `third_party/Models`
submodule (@ `c270d48`): the Rust catenary reproduces that library's own
`catenary_test.cc` expectations (`crates/atldp-core/tests/golden_otls_models.rs`;
provenance in `core/validation/oracles/README.md`). `OnSag` was the original
candidate but is a wxWidgets GUI in imperial units that *consumes* a precomputed
tension table — its actual numeric engine is OTLS-Models, which is the directly
comparable, headless oracle. The reference pinned is the **model-independent
catenary**; a change-of-state pin awaits the nonlinear conductor model (ADR-0003),
but that is a refinement, not a retirement gate. With gates 1–3 met, the Python
`core/` may now be removed in a commit citing this ADR.

### Status update (2026-06-20) — Python `core/` retired

With gates 1–3 met, the Python `core/` (engine, CLI, tests, and `validation/`
tooling) was **removed** in the G6 commit citing this ADR. The permanent oracle
that remains is entirely in the Rust tree:

- the golden re-encodings (`crates/atldp-core/tests/golden_*.rs`),
- the committed cross-check fixtures (`crates/atldp-core/tests/fixtures/`, the
  frozen sweep output of the former Python oracle — gate 2), and
- the third-party provenance, relocated from `core/validation/oracles/README.md`
  to [`crates/atldp-core/tests/ORACLES.md`](../../crates/atldp-core/tests/ORACLES.md).

No Python remains in the engine path; the only Python left in the repository is
the sanctioned terrain *prototype* under `tests/terrain/` (ADR-0007), which is
not part of the validated core.

## Consequences

- Retirement is a verifiable gate, not a judgement call, and is auditable.
- The Python prototyping/cross-check surface stays available through the port
  (phase G1) and no longer after — bounding maintenance of two engines.
- Forces the long-open third-party numeric cross-check to be resolved as part of
  closing out the engine, which strengthens the project regardless of language.

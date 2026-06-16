# Validation suite

Golden cases for the sag-tension core (ADR-0008). Each case pins expected outputs
with an explicit tolerance and cites its source. A tolerance regression is a
build failure.

| Case | What it pins | Source | Tolerance |
| --- | --- | --- | --- |
| `test_catenary_closed_form` | Exact catenary sag, length, max tension for level and inclined spans | Closed-form catenary identities (Irvine, *Cable Structures*, 1981; `docs/theory.md`) | 1e-9 rel |
| `test_method_agreement` | Parabola ≡ exact catenary in the shallow-sag regime | ADR-0003 cross-method agreement | 2e-3 rel |
| `test_change_of_state_invariants` | Change-of-state conserves the unstrained length; thermal/load monotonicity | Change-of-state equation (CIGRE TB 324; Winkelman 1959) | 1e-6 |
| `golden_otls_models` (Rust) | Catenary arc length, sag, support tension (level + inclined) match an independent third-party implementation | OTLS-Models `catenary_test.cc` @ `c270d48` ([`oracles/`](oracles/README.md)) | 1e-4 rel (its 2-dp rounding) |

## Rust port cross-check (ADR-0014)

The Rust `atldp-core` is validated against this Python oracle during the G1 port:

- **Golden-case parity (ADR-0014 gate 1):** each case above is re-encoded as an
  `atldp-core` integration test under `crates/atldp-core/tests/golden_*.rs` and
  passes within the same tolerance.
- **Cross-implementation agreement (ADR-0014 gate 2):** `export_reference.py`
  dumps oracle outputs over an 882-case parameter sweep (spans, inclinations,
  loads, tensions, temperatures, both solver methods) to the committed CSV
  fixtures in `crates/atldp-core/tests/fixtures/`;
  `crates/atldp-core/tests/cross_check_python_oracle.rs` recomputes every row in
  Rust and asserts agreement to ≤1e-7 rel (observed ~1e-9). The CSVs are committed
  so CI needs no Python — regenerate them with
  `python core/validation/export_reference.py` whenever the oracle changes.

## Status / open items

The closed-form and cross-method cases are fully independent oracles (they check
the code against mathematics it does not itself define). The change-of-state case
currently pins **physics invariants** (length conservation, monotonic response,
round-trip identity) rather than a third party's published decimals, because the
`mpewsey` reference is algorithm-only (no numbers).

## Third-party numeric oracle (ADR-0014 gate 3) — ✅ closed (2026-06-16)

The **independent third-party numeric reference** is **OTLS-Models**, vendored as
the `third_party/Models` git submodule (pinned at `c270d48`). The Rust
`atldp-core` catenary reproduces that library's own `EXPECT_EQ` catenary numbers
(arc length, sag, support tension, for a level and an inclined span) to its
2-decimal rounding. Provenance and the digitised table are in
[`oracles/README.md`](oracles/README.md); the check is
`crates/atldp-core/tests/golden_otls_models.rs`. This closes the last ADR-0014
gate (gates 1 — golden parity — and 2 — Rust↔Python sweep — were already met), so
the Python `core/` is now eligible for retirement.

`OnSag` itself was not used: it is a wxWidgets GUI app in imperial units that
consumes a precomputed tension table, and its numeric engine *is* OTLS-Models
(see `oracles/README.md`).

**Remaining (separate Phase-1 refinement, not a gate):** a *change-of-state*
third-party pin is still open because OTLS uses a full nonlinear core/shell
load-strain model while the Phase-1 conductor is linear-elastic + thermal
(ADR-0003); their reload tensions differ by that constitutive gap, not a bug. Once
the deferred nonlinear conductor model lands, a change-of-state case can be pinned
against OTLS too. Until then the change-of-state golden stays anchored to its
physics invariants, and the `DRAKE_ACSR` constants remain nominal published values.

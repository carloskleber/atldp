# Validation suite

Golden cases for the sag-tension core (ADR-0008). Each case pins expected outputs
with an explicit tolerance and cites its source. A tolerance regression is a
build failure.

| Case | What it pins | Source | Tolerance |
| --- | --- | --- | --- |
| `test_catenary_closed_form` | Exact catenary sag, length, max tension for level and inclined spans | Closed-form catenary identities (Irvine, *Cable Structures*, 1981; `docs/theory.md`) | 1e-9 rel |
| `test_method_agreement` | Parabola ≡ exact catenary in the shallow-sag regime | ADR-0003 cross-method agreement | 2e-3 rel |
| `test_change_of_state_invariants` | Change-of-state conserves the unstrained length; thermal/load monotonicity | Change-of-state equation (CIGRE TB 324; Winkelman 1959) | 1e-6 |

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
`mpewsey` reference is algorithm-only (no numbers) and the other open repos
(`OnSag`, `SSTC`) require digitising their fixtures.

**TODO (tracked for Phase 1 close-out / ADR-0014 gate 3):** add a numeric
cross-check of a Drake ACSR change-of-state against `OnSag`/`SSTC` or a digitised
textbook/IEEE table, pinning H2 and sag to a stated tolerance. Until then the
conductor constants in `atldp.core.conductor.DRAKE_ACSR` are nominal published
values and the change-of-state golden is anchored to the model's own invariants.
This is the **only remaining ADR-0014 gate**: the Rust port already meets gates 1
(golden parity) and 2 (cross-implementation sweep), so closing this third-party
reference is what finally retires the Python `core/`.

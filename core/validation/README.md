# Validation suite

Golden cases for the sag-tension core (ADR-0008). Each case pins expected outputs
with an explicit tolerance and cites its source. A tolerance regression is a
build failure.

| Case | What it pins | Source | Tolerance |
| --- | --- | --- | --- |
| `test_catenary_closed_form` | Exact catenary sag, length, max tension for level and inclined spans | Closed-form catenary identities (Irvine, *Cable Structures*, 1981; `docs/theory.md`) | 1e-9 rel |
| `test_method_agreement` | Parabola ≡ exact catenary in the shallow-sag regime | ADR-0003 cross-method agreement | 2e-3 rel |
| `test_change_of_state_invariants` | Change-of-state conserves the unstrained length; thermal/load monotonicity | Change-of-state equation (CIGRE TB 324; Winkelman 1959) | 1e-6 |

## Status / open items

The closed-form and cross-method cases are fully independent oracles (they check
the code against mathematics it does not itself define). The change-of-state case
currently pins **physics invariants** (length conservation, monotonic response,
round-trip identity) rather than a third party's published decimals, because the
`mpewsey` reference is algorithm-only (no numbers) and the other open repos
(`OnSag`, `SSTC`) require digitising their fixtures.

**TODO (tracked for Phase 1 close-out):** add a numeric cross-check of a Drake
ACSR change-of-state against `OnSag`/`SSTC` or a digitised textbook/IEEE table,
pinning H2 and sag to a stated tolerance. Until then the conductor constants in
`atldp.core.conductor.DRAKE_ACSR` are nominal published values and the
change-of-state golden is anchored to the model's own invariants.

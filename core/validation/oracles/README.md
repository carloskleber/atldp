# Third-party numeric oracle — OTLS-Models

This is the **independent third-party numeric reference** required by
[ADR-0014](../../../docs/adr/0014-python-core-retirement-criteria.md) gate 3 and
the long-open Phase-1 validation item in [`../README.md`](../README.md). It closes
the last gate that was blocking retirement of the Python `core/`.

## Source

- **Library:** OTLS-Models (Overhead Transmission Line Software, "Models"),
  vendored as the git submodule [`third_party/Models`](../../../third_party/Models),
  pinned at commit **`c270d48`** (tag `4.0.0-5`). Public domain (Unlicense).
- **File:** `test/transmissionline/catenary_test.cc` — the `Catenary2d` / `Catenary3d`
  unit tests. The numbers below are that suite's own asserted `EXPECT_EQ`
  expectations at the pinned commit; they are reproduced here verbatim.

### Why OTLS-Models and not OnSag

The original target was `OnSag`. On inspection `OnSag` is a **wxWidgets GUI** app
in **imperial units** that *consumes* a precomputed tension–temperature table and
computes stringing / transit / dynamic-wave sag — it is not itself a
change-of-state solver, and building it pulls all of wxWidgets. Its actual numeric
engine is this same organisation's headless **`Models`** library, which depends
only on googletest (no wxWidgets) and ships unit tests with embedded expected
numbers. `Models` is therefore the cleaner, directly-comparable oracle.

## What is pinned — the catenary (model-independent)

The catenary relations (horizontal tension `H`, unit weight `w`, span geometry →
sag, arc length, support tension) are **independent of the conductor constitutive
model and of the unit system**: every quantity is a function of the
dimensionless group `S/(2c)` with `c = H/w`. OTLS's tests are written in its
internal consistent units (feet, lbf); because the relations are dimensionless we
use the same numbers directly as SI (metres, newtons) — the equations are
identical. Our `atldp_core::catenary::solve_catenary` must reproduce them.

Reference object: `H = 1000`, `w = 0.5`  →  `c = H/w = 2000`.

| Case | Inputs (S, Δh) | Quantity | OTLS expected | atldp-core field |
| --- | --- | --- | --- | --- |
| Level    | (1000, 0)   | catenary constant `c` | 2000.00 | `catenary_constant()` |
| Level    | (1000, 0)   | arc length            | 1010.45 | `conductor_length` |
| Level    | (1000, 0)   | max sag               | 62.83   | `sag` |
| Level    | (1000, 0)   | tension at support    | 1031.41 | `max_tension()` |
| Level    | (1000, 0)   | tension at low point  | 1000.00 | `h_tension` |
| Inclined | (1000, 500) | arc length            | 1127.39 | `conductor_length` |
| Inclined | (1000, 500) | max support tension   | 1275.78 | `max_tension()` |

These are encoded as the Rust golden test
[`crates/atldp-core/tests/golden_otls_models.rs`](../../../crates/atldp-core/tests/golden_otls_models.rs),
which asserts agreement to OTLS's 2-decimal rounding. A regression there fails the
build (ADR-0008). No C++ is built in CI — the numbers above are the committed,
auditable digitised reference, and they are independently re-derivable from the
closed-form catenary identities in [`docs/theory.md`](../../../docs/theory.md).

## What is *not* pinned — change-of-state

OTLS models the ACSR Drake conductor with a **full nonlinear two-component
(core/shell) load-strain + creep polynomial** model (`test/factory.cc`), whereas
the Phase-1 `atldp-core` conductor is **linear-elastic + thermal** (single
modulus, ADR-0003). Their change-of-state reload tensions (e.g.
`line_cable_reloader_test.cc`: H = 6000 → 5561 / 4701 / 17123 lbf across weather
cases) therefore differ from ours by the constitutive-model gap, not by a bug, so
pinning them would be misleading. The change-of-state machinery stays validated by
its physics invariants and the Rust↔Python sweep (gates 1–2). A tight third-party
change-of-state pin is deferred to the nonlinear conductor refinement already
tracked in [`../README.md`](../README.md) — this oracle sharpens its priority.

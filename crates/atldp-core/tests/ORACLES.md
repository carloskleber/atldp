# Third-party numeric oracle — OTLS-Models

This is the **independent third-party numeric reference** required by
[ADR-0014](../../../docs/adr/0014-python-core-retirement-criteria.md) gate 3 and
the Phase-1 validation item in the
[implementation plan](../../../docs/IMPLEMENTATION_PLAN.md). It closed the last
gate that was blocking retirement of the Python `core/`.

> **Provenance note.** The Python `core/` has since been **retired** (ADR-0014):
> the golden tests in this directory and the committed cross-reference fixtures
> in [`fixtures/`](fixtures/) are the permanent oracle. This document moved here
> from the former `core/validation/oracles/README.md` when `core/` was removed.

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
[`golden_otls_models.rs`](golden_otls_models.rs), which asserts agreement to
OTLS's 2-decimal rounding. A regression there fails the build (ADR-0008). No C++
is built in CI — the numbers above are the committed, auditable digitised
reference, and they are independently re-derivable from the closed-form catenary
identities in [`docs/theory.md`](../../../docs/theory.md).

## Change-of-state — pinned via the nonlinear conductor (G1b)

OTLS models the ACSR Drake conductor with a **full nonlinear two-component
(core/shell) load-strain + creep polynomial** model (`test/factory.cc`). The
Phase-1 `atldp-core` conductor was **linear-elastic + thermal** (single modulus,
ADR-0003), so its reload tensions could not match OTLS — only the
model-independent catenary was pinned.

G1b adds an **independent nonlinear bimetallic stress-strain model**
(`atldp_core::conductor::StressStrainModel`): the composite curve is the sum of
the steel-core and aluminium-shell load-strain polynomials, inverted directly,
with per-material thermal strains (the bimetallic effect). It is **derived from
the stress-strain physics, not transcribed from OTLS's elongation/region/stretch
classes** — only the published *physical* Drake polynomial data is shared. That
independence is what makes the comparison below a validation rather than a copy.

Run through this crate's own length-conserving `change_of_state` (horizontal
tension, exact catenary) in OTLS's consistent US units (span 1200 ft, w = 1.094
lbf/ft, reference H = 6000 lbf at 60 °F):

| Reload case | OTLS expected | `atldp-core` | rel. err |
| --- | --- | --- | --- |
| 60 °F → 0 °F, bare | 6788 | 6787 | ~1e-4 |
| 60 °F → 212 °F, bare | 4701 | 4702 | ~2e-4 |
| 60 °F → 0 °F, 0.5″ ice + 8 psf wind (resultant w = √(2.072²+3.729²)) | 17123 | 17146 | ~1.3e-3 |

Source: `test/sagtension/catenary_cable_reloader_test.cc` at the pinned commit
(the no-stretch reference→reload cases). Agreement is to ≤ 0.2 %, not bit-identity:
OTLS reloads on the **average** tension with a piecewise region model and explicit
stretch, while we use the **horizontal** tension and a continuous polynomial, so a
small bounded difference is expected and is itself evidence of independence.

Encoded as the Rust golden test
[`golden_otls_change_of_state.rs`](golden_otls_change_of_state.rs), which asserts
the ≤ 0.2 % agreement and additionally regression-pins our own computed tensions.
A drift there fails the build (ADR-0008). This closes the change-of-state
validation gap that Phase 1 left open.

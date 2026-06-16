# ATLDP core — sag-tension engine (Phase 1)

The pure, headless engineering core mandated by [ADR-0002](../docs/adr/0002-python-core-with-separated-layers.md):
no GUI, no network, no geospatial dependencies. It implements the analytic
sag-tension baseline of [ADR-0003](../docs/adr/0003-analytic-sag-tension-baseline-before-fem.md)
for stage 4 of the design pipeline ([ADR-0009](../docs/adr/0009-staged-design-pipeline-and-project-model.md)).

## What it does

- **3D span geometry** (`atldp.core.geometry`) — attachment points in 3D; spans
  reduced to horizontal distance + elevation difference. Sag-tension is usually
  drawn in 2D, but a real line is 3D: uneven spans (ends at different
  elevations) and angle towers (plan-direction changes) are the normal case, not
  the exception. The geometry carries the plan bearing for angle-tower handling
  and takes the load per unit length as a parameter so wind blow-out slots in
  later (Phase 2).
- **Single-span mechanics** (`atldp.core.catenary`) — exact **inclined catenary**
  (closed-form, no level-support assumption) plus the **parabolic approximation**,
  with the regime switch from `docs/theory.md` (exact when sag/span > 1/8 or
  supports inclined).
- **Conductor model** (`atldp.core.conductor`) — linear-elastic + thermal
  constitutive law and a built-in **ACSR Drake 26/7**. Full nonlinear
  stress-strain + creep (CIGRE TB 324) is a documented later refinement.
- **Change-of-state** (`atldp.core.change_of_state`) — solves the new horizontal
  tension across temperature/load states by conserving the unstrained length;
  works directly on inclined spans.
- **Ruling span** (`atldp.core.ruling_span`) — equivalent-span section model that
  shares one horizontal tension across uneven spans.
- **CLI** (`atldp …`) — a thin presentation layer over the above.

## Setup

This prototype owns its own throwaway virtual environment
([ADR-0007](../docs/adr/0007-prototype-isolation-and-repo-hygiene.md)); it must
not be committed.

```bash
python3 -m venv .venv
.venv/bin/pip install -e ".[test]"
.venv/bin/pytest          # unit tests + validation/ golden cases
```

## CLI examples

```bash
# Solve one span at a fixed horizontal tension:
atldp catenary --span 400 --rise 30 --weight 15.97 --tension 30000

# Change of state for the built-in Drake ACSR (15 degC -> 75 degC):
atldp cos --span 400 --rise 0 --ref-H 31500 --ref-temp 15 --target-temp 75
```

## Validation

`validation/` holds golden cases, each citing its source with an explicit
tolerance (ADR-0008). See [validation/README.md](validation/README.md), including
the open item to add a third-party numeric cross-check (OnSag/SSTC).

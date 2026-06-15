# ADR-0006 — Headless-first; defer the GUI decision

- Status: Proposed
- Date: 2026-06-15

## Context

It is tempting to pick a GUI/desktop/web stack early. But the project's value and
its hardest risk both live in the **numerical engine**, not the interface. Picking
a presentation framework now would bias the architecture and waste effort before
the core is proven. Prototypes already render fine with Plotly (HTML output).

## Decision

Build **headless-first**: the engine is driven by a library API and a thin CLI,
producing data and Plotly/HTML artifacts. The choice of a full GUI (desktop vs.
web) is **deferred** until the core is validated (post Phase 1–3). When made, it
will be a separate ADR, and the GUI will sit strictly in the presentation layer
(ADR-0002) consuming the same core API.

## Consequences

- Effort concentrates on the engine and its validation.
- Everything is scriptable and testable from day one.
- No interactive end-user GUI until later; intermediate users work via CLI/notebooks.

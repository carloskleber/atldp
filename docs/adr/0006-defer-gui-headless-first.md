# ADR-0006 — Headless-first; defer the GUI decision

- Status: Resolved by [ADR-0012](0012-desktop-gui-wgpu-egui.md) (2026-06-15)
- Date: 2026-06-15

> **Resolved.** The deferred GUI decision is now made in ADR-0012 (native Rust
> desktop app: winit + wgpu + egui), under the runtime stack of ADR-0011. The
> headless-first principle held: the GUI sits in the presentation layer over a
> separately validated core.

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

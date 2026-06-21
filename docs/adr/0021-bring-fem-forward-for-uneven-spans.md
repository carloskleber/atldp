# ADR-0021 — Bring FEM forward as the uneven-span section solver

- Status: Proposed
- Date: 2026-06-21
- Amends (timing of): [ADR-0003](0003-analytic-sag-tension-baseline-before-fem.md)
  (analytic baseline before FEM)
- Builds on: [ADR-0008](0008-validation-against-references.md) (validation),
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md) (sections),
  [ADR-0020](0020-structure-geometry-and-tower-elevation-view.md) (attachment geometry)

## Context

ADR-0003 (accepted, implemented in Phase 1) built the **analytic core** — exact
inclined catenary + change-of-state + **ruling-span** section model — and deferred the
**finite-element method** to a "later, optional track (Phase 4)", to be introduced
"only where the ruling-span assumptions break down". That sequencing was right for
*building a validated yardstick first*. It is now too conservative about *when* FEM is
needed, for a reason that surfaced as the model grew real (ADR-0015/0016/0020):

- Every structure can have a **different attachment point** — different family,
  different height/extensions, different crossarm geometry (ADR-0020). The suspension
  insulators in a section therefore hang from points at **different elevations and
  offsets**, and the spans between them are **genuinely uneven** rather than the
  "geometrically similar spans" the ruling span assumes.
- The **ruling-span reduction** (`Sᵣ = √(ΣSᵢ³ / ΣSᵢ)`, one common `H` applied back to
  every span) is *exact only* under free-swinging insulators that equalise `H` across
  similar spans. Its accuracy is known to **degrade for strongly unequal spans and at
  high operating temperature** ([theory.md](../theory.md) ref. Motlis 1999). Uneven
  spans are not an edge case on real terrain with mixed structures — they are the
  **common** case. Solving them on a single equalised tension can mis-state tension and
  clearance precisely where the design is tightest.

So the trigger ADR-0003 named ("where ruling-span assumptions break down") is reached
**early**, not late. We bring FEM forward — without discarding the analytic core, which
remains the validation oracle.

## Decision

Implement an **uneven-span FEM section solver** as a near-term phase (G11), as an
**alternative section kernel** behind the same interface the analytic ruling span uses
today (`ruling_span::Section` consumers in `atldp_model::analysis`):

- The solver models a **tension section** as a cable finite element across its real,
  uneven supports (the per-structure attachment points of ADR-0020), with the
  suspension insulators represented so that the horizontal tension is **not assumed
  equal** across spans but **solved for** from equilibrium and the insulator swing. It
  reuses the validated **conductor constitutive law** (`atldp_core::conductor`,
  stress-strain/creep) for the change of state — the new code is the *structural* solve,
  not new material physics. References: the robust cable element of Bertrand 2020 and
  the non-incremental formulation of Sugiyama 2003 ([theory.md](../theory.md) refs.
  10, 12).
- **The analytic ruling-span core stays.** It remains the canonical reference and is
  retained for the level / equal-span limit and as the oracle. Per ADR-0008 and
  ADR-0003, **the FEM solver must agree with the analytic core in their overlapping
  domain** (level, equal spans, free-swinging insulators) to within an explicit
  tolerance — a tolerance regression is a build failure. Where they *disagree* (uneven
  spans), the difference is the value FEM adds, and is itself regression-tracked.
- **Selection.** The section solver is chosen automatically by the section's geometry:
  the analytic ruling span where its assumptions hold (similar spans, modest
  inclination), the FEM solver where they do not (uneven attachment points / spans,
  high-temperature cases). The choice is reported, so a result always states which
  kernel produced it.
- **Scope now vs. later.** This phase delivers the **static** uneven-span solve — the
  sag-tension and clearance numbers the design depends on. The **dynamic** FEM/ROM
  track (aeolian vibration, galloping; [theory.md](../theory.md) *Dynamics*, refs. 9,
  11) stays a later research track behind the static core, still validated against it
  in the static limit.

## Consequences

- Sag-tension and clearance are computed on the **real, uneven geometry** of a mixed
  structure line, instead of an equalised-tension approximation that is weakest exactly
  where clearance is tightest — the correctness motive the author raised.
- ADR-0003 is **not reversed**: its principle (analytic baseline first, FEM validated
  against it) stands; only the *timing* of the FEM track moves earlier, and its first
  use is the static uneven-span section rather than dynamics.
- New numerics enter the validated `atldp-core` and must earn their place under
  ADR-0008: golden cases against the analytic core in the overlap, and against an
  external FEM/reference (e.g. OTLS-Models uneven configurations) where available.
- The `analysis` fan-out over (wire × section) (ADR-0015) gains a per-section kernel
  choice; downstream consumers (report, sheet, stringing table) are unchanged because
  the kernel is behind the section interface.
- Cost and risk rise versus pure orchestration work: FEM is real engineering, isolated
  to its phase and gated by the agreement tolerance, mirroring how ADR-0012 isolated the
  LiDAR LOD risk.

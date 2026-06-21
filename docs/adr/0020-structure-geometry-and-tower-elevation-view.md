# ADR-0020 — Structure geometry and the tower-elevation view

- Status: Proposed
- Date: 2026-06-21
- Builds on: [ADR-0012](0012-desktop-gui-wgpu-egui.md) (GUI),
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md) (wire set),
  [ADR-0016](0016-structure-family-library-and-application-chart.md) (family library)

## Context

In the G8 model a structure is, geometrically, **one number**: an
`attachment_height_m` (plus per-wire vertical/lateral offsets on the `Wire`). The GUI
lets the engineer "choose a height". But a real structure is a **shape** — body,
crossarms, and the discrete **attachment points** at which each phase and shield wire
connects. The designer needs to *see that shape*: where each of the (3 · circuits)
phases and the shield wire(s) actually attach, the crossarm reach (which sets phase
spacing and the swing/clearance envelope), and how a chosen family/height positions
them above ground.

Two requirements make this concrete:

- A structure must be presented as a **drawn elevation in its own window**, not picked
  as a scalar height. Selecting a tower (in the profile or the editable table of
  ADR-0019) opens a **tower-elevation view** showing the silhouette and the labelled
  per-wire attachment points.
- Because each wire attaches at its **own point**, the per-wire attachment geometry is
  first-class design data — and it is exactly what makes spans *uneven* between
  structures of different families/heights, which is the motivation for FEM
  (ADR-0021). The geometry the view draws is the same geometry the section solver
  consumes.

## Decision

Give a **`StructureFamily`** (ADR-0016) a **drawable geometry**, and add a dedicated
view that renders it:

- The family gains an **attachment geometry**: per **conductor position** (each phase
  of each circuit, each shield wire) a point in the structure's own 2-D elevation frame
  — height above the structure reference and lateral offset from the centreline — plus
  the body/crossarm polyline needed to draw the silhouette. The per-wire offsets that
  G7 put on `Wire` are reconciled to *come from* the family's geometry at the structure
  (the family is the source of truth for where a wire attaches; the `Wire` carries the
  conductor spec and tension).
- A **tower-elevation view** (a new egui panel / detachable window in `atldp-app`,
  ADR-0012) draws the selected structure's silhouette with every attachment point
  labelled by wire, the chosen height/extensions, and the resulting attachment
  elevations. It is the structure-scale counterpart of the line-scale profile view and
  is opened from a tower selection.
- The view is **editable in the structure frame** where the family allows it: pick the
  family, set the height/extension within the family range, and apply per-structure
  overrides (ADR-0016's `effective_height_override_m` / `chart_override`), with the
  silhouette and attachment elevations updating live and feeding back into the
  profile/clearance analysis.

No new numerics: the view consumes existing model data; the only model addition is the
family's attachment/silhouette geometry, which is also what ADR-0021's solver reads.

## Consequences

- "Choosing a structure" becomes inspecting and placing a **real shape with real
  attachment points**, not entering a height — closing the gap the user flagged.
- Per-wire attachment points get a single authoritative source (the family geometry),
  which both the renderer and the uneven-span solver (ADR-0021) read, so the picture
  and the mechanics cannot diverge.
- `atldp-render` / `atldp-app` gain a second drawing context (structure-frame
  elevation) beside the line-frame profile; both are orthographic 2-D, so the existing
  2-D path (ADR-0012) is reused under a different transform.
- The family library (ADR-0016) and the `.atldp` format gain the attachment/silhouette
  geometry; coordinated with the ADR-0019 schema bump.

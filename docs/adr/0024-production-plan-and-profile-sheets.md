# ADR-0024 — Production plan & profile sheets (paginated, A1/A0, full label sets)

- Status: Proposed
- Date: 2026-06-22
- Builds on: [ADR-0009](0009-staged-design-pipeline-and-project-model.md) (drafting is
  stage 9 over the shared project model),
  [ADR-0015](0015-multi-wire-conductor-set-and-tension-sections.md) (wires + tension
  sections — the section/wire labels),
  [ADR-0016](0016-structure-family-library-and-application-chart.md) /
  [ADR-0020](0020-structure-geometry-and-tower-elevation-view.md) (structures as
  families with geometry — the structure labels),
  [ADR-0019](0019-route-poi-model-and-mandatory-angle-structures.md) (route/POI model —
  stationing, coordinates, crossings),
  [ADR-0022](0022-srtm-area-selection-and-plan-view-route-editor.md) (the plan-frame
  view that feeds the sheet's plan strip)

## Context

Phase G6 delivered a **plan-&-profile SVG** ([`sheet.rs`](../../crates/atldp-model/src/sheet.rs)):
one fixed-width (1100 px) panel with a terrain fill, per-wire catenaries, plain `T1…Tn`
tower ticks, a thin schematic plan strip, and axis gridlines. It proved the drafting
seam — one shared `analysis` pass feeds both the Markdown `report` and the SVG — but it
is a *plot*, not a **sheet**. A delivered transmission-line plan & profile (the WAPA
*Bismarck–Glenham* and *Trinity–Weaverville* drawings are the reference exemplars) is a
formal engineering document with a strict, information-dense anatomy:

- A **title block** (lower-right): owner/agency, line name, voltage, "PLAN & PROFILE",
  the station range, drawing/sheet number, designed/approved signatures, date,
  "supersedes DWG …".
- **Per-structure label columns** — vertical text stacks at each structure carrying its
  *number*, *station*, and (selectably) X/Y coordinates, height & elevation, height/leg
  adjustment, orientation angle, offset, embedded length, conductor attachment
  height/elevation, and line angle.
- **Per-section label blocks** — structure range, cable (conductor) file, **ruling
  span**, **design tension**, and selectably voltage, phases/wires, and the displayed
  weather case/condition/tension.
- **Span-length labels** along each span and **wire labels** on each cable.
- A **plan strip** below the profile: the route on an **ortho/aerial raster** with
  stationing, structure markers, the RoW edges, and **crossing call-outs** (roads,
  fences, other lines), i.e. the plan-frame view (ADR-0022) composited onto the sheet.
- A **notes block**: loading code (e.g. NESC Heavy), conductor & OPGW/shield specs,
  ruling span, tension at a reference temperature/condition, minimum ground clearance,
  maximum operating temperature, survey/imagery dates, software version; plus a
  **legend** (line styles, phasing diagram, leg-extension key).
- **Dual elevation axes** (left and right) over a fixed-scale grid.

Two further facts the current plot ignores: a real line **does not fit on one sheet** —
it is paginated across **A1 or A0** sheets at a fixed horizontal scale (commonly
1″=200′ horizontal, 1″=40′ vertical ⇒ V.E. ≈ 5×) with a **match line** between sheets;
and the **fields shown are a configurable set** (the PLS-CADD *Structure Labels /
Section Labels / Wire Labels / Span Length / Inset Views* tabs), not a fixed list.

## Decision

Grow `sheet.rs` from a single plot into a **paginated production sheet generator** over
the existing validated `analysis`, with a declarative, configurable layout. No new
numerics; everything drawn is already in the project model or `Analysis`.

### Sheet geometry & pagination

- **Physical page sizes.** A sheet is authored at a real paper size — **A1
  (594 × 841 mm)** by default, **A0 (841 × 1189 mm)** optional — **landscape**, with SVG
  `width`/`height` in `mm` and a `viewBox` in millimetres so it prints 1:1 and imports
  into CAD at true scale. A `PageSize` enum (`A1`, `A0`, and a custom `mm × mm`) plus
  margins and a title-block reserve define the drawable frame.
- **Fixed drawing scale + pagination.** The sheet carries an explicit **horizontal
  scale** and **vertical scale** (e.g. 1:2400 / 1:480, V.E. = 5) rather than fitting the
  whole line into one panel. The line is **split into sheets** by station: each sheet
  covers the station interval that fits its frame at the chosen scale, and adjacent
  sheets share a **match line** at the boundary. `plan_profile_sheets(project, &layout)`
  returns an **ordered `Vec` of SVG sheets**; the single-panel
  `plan_profile_svg` stays as the "fit-to-one" convenience/preview (and for the GUI live
  view).

### Configurable label sets

A `SheetLayout` struct makes the drawn fields a **selectable set**, mirroring the
reference tool's tabs, with sensible defaults (only the always-useful fields on):

- **Structure labels** (vertical stack at each structure): `number`, `station`
  *(default on)*; `xy_coords`, `height_and_elevation`, `height_adjustment`,
  `orientation_angle`, `offset_adjustment`, `embedded_length`,
  `conductor_attachment_height`, `conductor_attachment_elevation`, `line_angle`,
  `comments` *(default off)*. Each maps to an existing field
  ([`Tower`](../../crates/atldp-model/src/lib.rs): `distance_m`, `ground_elevation_m`,
  `attachment_height_m`, `line_angle_deg`, `family`, `origin_poi`; X/Y from the linked
  [`Poi`](../../crates/atldp-model/src/lib.rs) `lat`/`lon` via CRS).
- **Section labels** (block per tension section, ADR-0015): `structure_range`,
  `cable_file` (conductor name), `ruling_span`, `design_tension` *(default on)*;
  `voltage`, `cable_description`, `phases_and_wires`, `weather_case`, `legend`
  *(default off)*.
- **Span-length labels** (per span) and **wire labels** (per cable) — on/off with a
  placement option.
- **Inset / plan views**: the plan strip's content — `centerline`, `structures`,
  `row_edges`, `stationing`, `crossings`, and whether an **ortho raster** backs it
  (ADR-0022's plan-frame raster) or it stays a schematic strip when no imagery is loaded.

### Title block, notes & legend

- A **title block** in the reserved lower-right region, populated from
  `Project::metadata` (extended as needed: owner/agency, line name, voltage, drawing
  number, sheet `n of N`, dates, revision/"supersedes"). Missing fields render blank
  rather than fabricated.
- A **notes block** and **legend** built from the project's criteria and wire set:
  loading code and design weather case (ADR-0017 when present, else the current single
  wind pressure), each wire's conductor/OPGW spec and tension, ruling span, minimum
  ground clearance, max operating temperature, terrain/imagery provenance (ADR-0018/0022),
  and the ATLDP version. A small **legend** keys the line styles and (later) the phasing
  and leg-extension diagrams.

### Scope guards

- **SVG only, string-built, zero new deps** — the G6 posture (auditable, diffable,
  adds nothing to the binary) is kept; PDF is left to the consumer (browser/CAD print).
- **No new numerics.** Every value is read from the project model or `Analysis`; this
  is layout/typography work. The validation suite (ADR-0008) is untouched; new tests
  assert sheet *structure* (well-formed SVG, correct page count for a station range,
  presence of selected labels/title-block fields), as the G6 tests already do.
- **Phasing & leg-extension diagrams** and full **ortho-raster compositing** depend on
  data ATLDP does not yet carry (per-phase bundle geometry; a georeferenced corridor
  image). They are designed into the layout as optional blocks but may land
  incrementally; the schematic plan strip remains the fallback until the raster exists.

## Consequences

- The drafting deliverable becomes a **document a line engineer recognises**: correctly
  sized for A1/A0 plotting, paginated with match lines, and carrying the structure,
  section, span, and wire labels and the title/notes/legend blocks of a real plan &
  profile — closing the gap between the G6 proof-of-seam plot and stage-9 field output.
- `SheetLayout` makes the field set **explicit and configurable** and gives the GUI a
  natural settings surface (the same tabs as the reference tool) without hard-coding
  choices into the renderer.
- The plan strip and the plan-view editor (ADR-0022) **share** the plan-frame
  projection, and the sheet's notes block consumes the load-case engine (ADR-0017) and
  terrain provenance (ADR-0018) as those land — the sheet becomes the project's single
  printable summary.
- `Project::metadata` grows title-block fields (owner, line name, voltage, drawing/sheet
  identity, dates, revision); if these cross the persisted schema they ride a normal
  `.atldp` version bump with a round-trip-tested migration (ADR-0015 pattern), defaulting
  empty for legacy projects.
- Risk: per-sheet pagination and dense label placement invite **overlap/collision** at
  close structure spacing. Mitigation: deterministic label stacking with a documented
  fixed scale (engineers expect a fixed scale, not auto-fit), and the single-panel
  fit-to-one view stays available for quick on-screen review.

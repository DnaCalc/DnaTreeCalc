# SHEET_SPEC — the grid, done properly

Status: v0.2 draft · 2026-07-13 · hardens to ratification at S3 kickoff.
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md) ·
functional lineage: route K in `docs/ux/FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` (verb map,
`EnterGridCell` single entry verb, error table remain normative).

## 1. Purpose

*"I want the grid — and my file back, intact."* Excel-strict profile's home stage: behaves like
Excel where behavior counts (keys, entry, names, tables, spills, F-keys), looks like Strand.
Canvas-rendered from day one; the huge-grid path (GPU, engine-priority feedback) is a renderer
upgrade behind the same contracts, never a redesign.

## 2. Renderer architecture (D1: canvas + DOM overlay)

- **Canvas viewport**: Canvas2D tile cache keyed by (tile rect, projection_epoch,
  overlay_epoch, zoom tier); draws cells, gridlines, overlays (tables/spills/merges from
  `GridOverlayBundle`, clip flags honored), selection, provenance typography.
- **DOM overlay**: exactly one — the active-cell editor (Bridge grammar, degrade per
  SHELL_SPEC §6 until G1) positioned over the canvas; plus transient peek cards.
- **RenderPlan IR**: the renderer emits a deterministic display list; geometry + hit-test
  invariants are asserted in unit tests (Foundation doctrine) — no screenshot assertions.
- **Interest discipline**: scroll/zoom coalesce to ≤1 `SetGridInterest` per frame (RAF
  coalescer, estate pattern); render from the delta mirror only; never re-read a grid on a
  keystroke. **G4** makes interest real on the workbook dispatcher (today a no-op) and adds
  multi-rect + prefetch + intra-window diffs; until then the stage works honestly at
  bounded-sheet scale.
- Future GPU/engine-feedback: WebGL tile renderer + viewport-driven recalc priority are
  explicitly deferred; the tile/LOD contract in G4 is written so they slot in without touching
  this stage's logic (deferral, not rewrite).

## 3. Semantic zoom (mechanisms 01, 04)

| Tier | Range | Renders |
|---|---|---|
| Detail | ≥ 60% | full cell text, gridlines, provenance typography |
| Structure | 15–60% | values fade to blocks; **named ranges, tables, spill regions render as labeled Strand blocks**; text never below legibility floor — labels swap in, not shrink |
| District | < 15% | the sheet as a map: used-range districts + labels — visually continuous with Atlas |

## 4. Interaction (Excel-familiar core)

- Entry: type-to-replace, F2 edit-in-place, Enter/Tab commit+move, Esc exact revert — all via
  `EnterGridCell` three-way outcome; unresolved names surface per the route-K error table.
- Navigation: arrows, PgUp/PgDn, Home/Ctrl+Home; Ctrl+arrow edge-jump requires a host query
  (G4 model-query; degrade: window-local jump with atlas note).
- Selection: single cell + table cell today; **ranges, row/col ops, fill grips, grid clipboard,
  merged authoring, workbook undo/redo all arrive with G3** — affordances render disabled-with-
  reason until then (no fake handles).
- Sheet tabs: `SheetProjection` + add/rename/delete/move intents; keyboard Alt+PgUp/PgDn
  (browser-safe primary; Ctrl+PgUp/PgDn added on desktop per SHELL_SPEC §5.1).
- Name box: goto (name/A1) + names manager (`DefinedNamesProjection`, static + dynamic;
  dynamic names open the manager, per engine-ask ledger).
- Spills (mechanism 03): origin badge + extent veil from `GridSpillOverlayDescriptor`;
  `SpillDisplay` members read-only with anchor jump; blocked spills point at the blocker.
- Provenance typography (mechanism 04): `GridAuthoredKindProjection` × `ValueProvenance
  Projection` → constant/formula/spill-member/external text treatments (structural, not
  decorative).
- Formats: display through **G2** (until then: number-format-code at name/node level only;
  cells render canonical text — stated honestly in the stage, no guessed styling).

## 5. Gaps this stage consumes

**G3** (interaction pack — the defining ask) · **G4** (viewport/LOD; makes scale honest) ·
G2 (formats/CF) · G9 (grid dependency projection → X-Ray + error triage here) · G1 (rich
in-cell editing) · G5 (typed errors for triage grouping).

## 6. Acceptance sketch (hardens at S3)

1. Open a real .xlsx (R6 lane): scroll a 100k-row sheet at 60 fps with interest windows; edit
   a literal; dependent recalc renders from delta; save round-trips (fidelity ledger clean).
2. Zoom out: named regions and tables become labeled blocks (tier 2) and districts (tier 3);
   zoom back preserves focus (legibility floor holds throughout).
3. Spill: origin badge, veil, blocked-spill blocker jump all correct on the corpus spill cases.
4. Range select → fill grip with live delta readout (post-G3); workbook undo/redo through the
   Timeline.
5. RenderPlan geometry/hit-test invariant suite green on both targets; no canvas work on the
   UI thread beyond paint (worker-hosted session default at this stage's scale).

## 7. Open questions

- Frozen panes v1 or post-G3 (leaning post-G3 with the row/col pack).
- In-cell rich text (never for v1; G2 scope question for later).
- District-tier label density heuristics (tie to Atlas's treemap rules or keep per-stage?).

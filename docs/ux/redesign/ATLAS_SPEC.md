# ATLAS_SPEC — the probe stage

Status: v0.2 draft · 2026-07-13 · skeleton lands with S2, grid depth with S3 (G9), tree depth
with S4. Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md).
Absorbs: Flow lens (its vision doc `docs/ux/flow-skin/FLOW_VISION.md` remains the north star
for traversal), plus the structure-map ambition. Read-mostly by design.

## 1. Purpose

*"I was handed 40 sheets and 120 000 formulas — show me what this thing is."* Comprehension
before editing: the model as a map (structure), as a flow (dependencies), and as a health
report (calc, errors, feeds). Also the first-run experience for any huge foreign workbook.

## 2. The three lenses of one stage (not three stages)

| Lens | Shows | Fed by |
|---|---|---|
| **Map** | sheets/top-subtrees as districts sized by content; blocks = tables, named regions, spill fields, formula fields vs constant fields | `SheetProjection` + `GridOverlayBundle` + `DefinedNamesProjection` (grid) · root subtrees + tables (tree) |
| **Flow** | dependency sentence: topological columns, wires, reading-head replay of the last run along real `evaluation_order`; `]`/`[` trace, `E` explain | `DependencyGraphProjection` + `CalcRunProjection` (tree today; **G9** for grid) |
| **Health** | calc HUD (phase timings, run state, dirty), error triage groups (root cause + blast radius), feed instruments | `CalcRunProjection` + invalidation reasons + cycle groups (+ G7 feeds, G5 typed errors) |

One selection, three lenses: picking a district scopes Flow and Health to it (cross-lens
continuity inside the stage mirrors cross-stage continuity).

Health additionally reserves a **document parity-report section** (coverage, divergences,
out-of-scope — ladder rung 3 in [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md)); the section id
exists in the lens layout, renders nothing this phase.

## 3. Interaction

- **Fly-in**: district → region → cell/node, double-click or Enter descends, Backspace
  ascends; each level swaps LOD (labels-over-blocks law); "open in Sheet/Model" jumps stages
  with continuity halo (mech 09).
- **Reading head** (Flow): scrub or play the last calc run; the head moves along
  `evaluation_order`, lighting values as they publish — recalc made visible (mech 16).
- **Error triage** (mech 17): groups by root cause via reverse edges; fixing the origin (jump
  to owning stage) collapses the group on next delta.
- **Path questions**: "what feeds this / what does this feed" as readable columns, not
  hairballs; cycle groups render as explicit rings with member lists.
- Rendering: Map/Flow on canvas (same RenderPlan discipline as Sheet); Health is DOM.

## 4. Scale honesty

The stage renders what the IR windows give it. Pre-G4/G9 ceilings: tree Flow at guardrail
scale (n ≤ ~1k interactive today), grid Map from overlay + names data (cheap) while per-cell
dependency flow waits for G9; the stage states its coverage ("mapping names + tables;
cell-level flow arrives with the grid dependency projection") rather than sampling silently
— no-silent-caps law (MECHANISMS preamble).

## 5. Keyboard (beyond universal)

Enter/Backspace descend/ascend · `]`/`[` trace forward/back · `E` explain from selection ·
Space play/pause reading head · 1/2/3 switch lens (Map/Flow/Health) within the stage.

## 6. Gaps this stage consumes

**G9** (grid dependency projection — the defining ask for Excel-strict depth) · G4
(model-query + windowed projections for big-model maps) · G5 (typed errors for triage) ·
G7 (feed instruments in Health) · G8 (deck entries; agent addressability of map objects).

## 7. Acceptance sketch

1. Open a 40-sheet corpus workbook: Map renders districts with honest coverage statement;
   fly-in to a table; "open in Sheet" lands on it selected.
2. Tree model: Flow replays the last run with the reading head over real evaluation order;
   `E` on a node explains its derivation recursively (absorbed Flow behavior preserved).
3. Break a corpus case: Health groups the resulting errors under one root cause with correct
   blast-radius count; jump-to-origin lands in the owning stage.
4. Post-G9: cell-level precedent/dependent traversal on the grid matches engine truth on the
   corpus dependency cases.

## 8. Open questions

- Whether Map districts encode calc-time (treemap-by-milliseconds toggle) in v1 Health or later.
- Persisted "tour" bookmarks (guided walkthrough of a model) — candidate for the published
  story, post-S4.
- Name collision note: the historical ATLAS lens-suite docs remain under `docs/ux/skin-suite/`;
  new references to "Atlas" mean this stage (program §6 rule).

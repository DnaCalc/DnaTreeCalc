# MODEL_SPEC — home of the Rich Tree profile

Status: **v1.0 ratified · 2026-07-23** (owner) · was v0.2 draft 2026-07-13. Ratification
decisions recorded in §9. Prerequisite still gating the M-track UI beads: RichTree session
extraction into `dnacalc-host-core` (S4 P-track).
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md) · model
semantics: `docs/model/CORE_MODEL_SPEC.md` + `docs/model/META_NODES.md` (normative).
Absorbs: Capture, Tree, Ledger, Sheet-on-tree lenses (consolidation map §4 of the program).

## 1. Purpose

*"I want to build and shape this model."* The premium modelling surface: a tree of named
formulas with nodes that scale from a chip to a full table, references that explain
themselves, and structure you can type into existence. Top-level nodes carry workbook-like
scope; the second level acts sheet-like; deeper levels extend naturally.

## 2. Node cards (progressive disclosure — four densities)

chip (name + value + classification tint) → summary (fold shows the node's OWN value + an
"N children collapsed" indicator, Excel-parity, mech 02; §9.1) → open card (formula via Bridge +
value/array region + children) → focused (table entry full-width; array editing degrades to the
open-card Bridge overlay in S4 — no distinct array layout, §9.1). One component, four densities; `NodeView` + `TableProjection` feed
all four. Meta subtrees render ghosted behind the reveal toggle (`is_effective_meta`), default
hidden (REQUIREMENTS dial).

## 3. Core interactions

- **Capture-by-typing** (absorbed Capture lens): a path entry line scaffolds structure —
  `Products.Alpha.Rev = 1440` creates intermediate nodes in one transaction (candidate lane,
  one undo step); paste-block dry-runs via the preview seam before commit (mech 12).
- **Structure ops**: add/rename/move/reorder/delete via tree intents with `MutationImpact
  Projection` ghosts (blast radius, orphan risk, cycle risk) before commit. **Rename** is
  initiated by an explicit command (distinct from value editing) and, when the name is
  referenced elsewhere, REQUIRES a confirmation card showing the reference-rewrite extent
  (mech 15); an unreferenced-name rename commits directly (§9.2).
- **Cleave bar** (absorbed Ledger): shared `CleavePredicate` filter/sort chips over the visible
  tree; cohort selection feeds bulk edit through existing multi-select + scoped-content
  intents.
- **Tables-in-node**: full table system per `TableProjection` — headers, totals row, column
  formulas, natural row insert at the boundary, per-column number format (`SetNumberFormat` on
  scope; richer per-column format = G2); structured references highlight their region on hover.
- **Reference X-Ray** (mech 07, richest here): caret on `^^Prices.Base` replays resolution —
  walk-up steps, anchor hop, selector — as numbered hops over the visible tree. **Engine-true
  steps via ask G11** (owner decision §9.3; the IR carries no step list today); the pre-G11
  honest degrade is presentational origin→target hops with a visible "engine-true steps arrive
  with G11" note. `ReferenceResolutionProjection` + collection families cover `.*`, `**`,
  `@`-meta, reference arrays. This is how the tree grammar teaches itself.
- **Node styling**: classification/calc-state/provenance tints now; author-chosen type/color/
  emphasis arrives with **G2** (per-node effective format) — until then no decorative styling
  (law: structure from blocks and type, not paint).

## 4. Layouts (one model, three projections; layout data never touches calc)

| Layout | v1 scope | Position store |
|---|---|---|
| **Outline** | S4 core (this spec's default) | derived (no positions) |
| **Canvas** (free-form) | S4 late: place/pin cards, ghost modular grid, alignment guides on drag only | skin-local `SkinState` keyed by NodeKey; **G10** graduates positions to a document overlay |
| **Diagram** (connection view) | S4 late, read-mostly: cards + dependency wires (auto-laid) | derived + pin overrides |

Semantic zoom (mech 01/02): collapsed subtrees render as summary blocks; far zoom shows the
tree as nested districts (continuous with Atlas) — **deferred in S4** with an honest "not built"
note, the Model twin of Sheet's deferred District tier; tracked as a V2 follow-up (§9.4).

## 5. Keyboard (beyond universal)

←/→ fold/unfold · Alt+↑/↓ reorder among siblings · Alt+←/→ outdent/indent (move) ·
Ctrl+Shift+A add child, Ctrl+Enter add sibling · `.` in the capture line descends ·
table cells: Excel-familiar (Enter/Tab/F2/Esc), Ctrl+= totals toggle. Atlas-tagged, rebindable.

## 6. Gaps & prerequisites this stage consumes

**RichTree extraction into host-core** (engineering prerequisite; the seam is empty today) ·
G1 (Bridge fidelity in node/table contexts) · G2 (node styling + per-column formats) · G5
(windowed arrays in node cards) · **G11 (engine-true reference resolution-step projection for
X-Ray — filed 2026-07-23)** · G10 (shareable canvas positions, optional) · G4's windowed
tree projection for very large trees (degrade: full-snapshot at tree scale ≤ perf guardrails).

## 7. Acceptance sketch (hardens at S4)

1. Capture-by-typing builds a three-level model in one line per node; undo collapses each
   line's transaction to one step.
2. Table node: add column formula, totals row, insert row at boundary; structured refs
   highlight regions; export/round-trip per tree→xlsx subset rules.
3. X-Ray replays `^^` and `.*` resolutions as visible hops on the corpus reference cases
   (engine-true steps once G11 lands; the pre-G11 degrade shows origin→target hops + the note).
4. Move a subtree with dependents: ghost shows blast radius; commit rebinds; Timeline entry
   carries the transaction summary.
5. Fold any subtree → summary chip shows the node's OWN value + child-count (Excel-parity, no
   invented aggregate); cleave to `HasError` cohort and bulk-edit a constant across it.

## 8. Open questions (remaining after ratification)

- Canvas ghost-grid spacing (tie to block gap scale) and pin/unpin affordance.
- Hybrid groundwork: which S5 proof — grid-region card (Sheet surface embedded) vs chart-node
  stub — earns the slot.

(The per-node aggregate-picker question is CLOSED — dropped in favor of Excel-parity own-value
folds; see §9.1.)

## 9. Ratification decisions (owner, 2026-07-23 — v0.2 → v1.0)

1. **Folded-row aggregate = Excel-parity.** A folded node shows its OWN `computed_value`
   verbatim (the authored rollup / SUBTOTAL-equivalent), never a model-invented aggregate, plus
   a structural "N children collapsed" indicator. Aggregation is the user's own rollup formulas
   (like Excel outline SUBTOTAL rows; don't-double-count is the engine's job). The per-node
   aggregate picker is DROPPED — Excel has no such control. Array editing (the Focused density's
   array half) degrades to the open-card Bridge overlay; no distinct array-editing layout ships
   in S4. (Beads F3, M11.)
2. **Rename** is initiated by an explicit command (not click-the-name), clearly distinct from
   value editing; a confirmation card showing the reference-rewrite extent is REQUIRED only when
   the name is referenced elsewhere (`reverse_references` non-empty), else it commits directly.
   (Beads M3 trigger, M5 confirmation.)
3. **Reference X-Ray = engine-true** via new ask **G11** (filed in SKIN_IR_GAP_REGISTER.md) —
   presentational origin→target hops are only the honest pre-G11 degrade. (Beads F6, M8.)
4. **Far-zoom district tier deferred** with an honest "not built" note (parity with Sheet's
   deferred District tier); V2 follow-up.
5. **Profile entry = a real in-app profile switcher** (NOT a URL flag) — swaps the mounted
   composition between the Excel-strict workbook and the Rich Tree profiles. (Bead P9.)
6. **Undo/Redo lives in the host engine** (`dnacalc-host-core`), not the UI — capture's
   one-undo-step is a model contract, not a UI convenience. (Bead P5.)
7. **Mech-16 citation fixed** — §2's densities are the four-density progressive-disclosure
   ladder, not "mechanism 16" (that is the Calc HUD).

Still gating the M-track UI beads: the RichTree session extraction (P-track), independent of
this spec's ratification.

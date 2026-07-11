# MODEL_SPEC — home of the Rich Tree profile

Status: v0.2 draft · 2026-07-13 · hardens to ratification at S4 kickoff (prerequisite:
RichTree session extraction into `dnacalc-host-core`).
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md) · model
semantics: `docs/model/CORE_MODEL_SPEC.md` + `docs/model/META_NODES.md` (normative).
Absorbs: Capture, Tree, Ledger, Sheet-on-tree lenses (consolidation map §4 of the program).

## 1. Purpose

*"I want to build and shape this model."* The premium modelling surface: a tree of named
formulas with nodes that scale from a chip to a full table, references that explain
themselves, and structure you can type into existence. Top-level nodes carry workbook-like
scope; the second level acts sheet-like; deeper levels extend naturally.

## 2. Node cards (progressive disclosure — mechanism 16's four states)

chip (name + value + classification tint) → summary (fold with aggregate/sparkline, mech 02)
→ open card (formula via Bridge + value/array region + children) → focused (table entry /
array editing full-width). One component, four densities; `NodeView` + `TableProjection` feed
all four. Meta subtrees render ghosted behind the reveal toggle (`is_effective_meta`), default
hidden (REQUIREMENTS dial).

## 3. Core interactions

- **Capture-by-typing** (absorbed Capture lens): a path entry line scaffolds structure —
  `Products.Alpha.Rev = 1440` creates intermediate nodes in one transaction (candidate lane,
  one undo step); paste-block dry-runs via the preview seam before commit (mech 12).
- **Structure ops**: add/rename/move/reorder/delete via tree intents with `MutationImpact
  Projection` ghosts (blast radius, orphan risk, cycle risk) before commit; rename shows the
  reference-rewrite diff (mech 15).
- **Cleave bar** (absorbed Ledger): shared `CleavePredicate` filter/sort chips over the visible
  tree; cohort selection feeds bulk edit through existing multi-select + scoped-content
  intents.
- **Tables-in-node**: full table system per `TableProjection` — headers, totals row, column
  formulas, natural row insert at the boundary, per-column number format (`SetNumberFormat` on
  scope; richer per-column format = G2); structured references highlight their region on hover.
- **Reference X-Ray** (mech 07, richest here): caret on `^^Prices.Base` replays resolution —
  walk-up steps, anchor hop, selector — as numbered hops over the visible tree;
  `ReferenceResolutionProjection` + collection families cover `.*`, `**`, `@`-meta, reference
  arrays. This is how the tree grammar teaches itself.
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
tree as nested districts (continuous with Atlas).

## 5. Keyboard (beyond universal)

←/→ fold/unfold · Alt+↑/↓ reorder among siblings · Alt+←/→ outdent/indent (move) ·
Ctrl+Shift+A add child, Ctrl+Enter add sibling · `.` in the capture line descends ·
table cells: Excel-familiar (Enter/Tab/F2/Esc), Ctrl+= totals toggle. Atlas-tagged, rebindable.

## 6. Gaps & prerequisites this stage consumes

**RichTree extraction into host-core** (engineering prerequisite; the seam is empty today) ·
G1 (Bridge fidelity in node/table contexts) · G2 (node styling + per-column formats) · G5
(windowed arrays in node cards) · G10 (shareable canvas positions, optional) · G4's windowed
tree projection for very large trees (degrade: full-snapshot at tree scale ≤ perf guardrails).

## 7. Acceptance sketch (hardens at S4)

1. Capture-by-typing builds a three-level model in one line per node; undo collapses each
   line's transaction to one step.
2. Table node: add column formula, totals row, insert row at boundary; structured refs
   highlight regions; export/round-trip per tree→xlsx subset rules.
3. X-Ray replays `^^` and `.*` resolutions as visible hops on the corpus reference cases.
4. Move a subtree with dependents: ghost shows blast radius; commit rebinds; Timeline entry
   carries the transaction summary.
5. Fold any subtree → summary chip shows the chosen aggregate; cleave to `HasError` cohort and
   bulk-edit a constant across it.

## 8. Open questions

- Summary-rule authoring UX (per-node aggregate choice) — overlay data; picker design pending.
- Canvas ghost-grid spacing (tie to block gap scale) and pin/unpin affordance.
- Hybrid groundwork: which S5 proof — grid-region card (Sheet surface embedded) vs chart-node
  stub — earns the slot.

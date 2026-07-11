# NOTEBOOK_SPEC — the story stage

Status: v0.3 draft · 2026-07-13 · hardens to ratification at S2 kickoff.
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md) ·
functional lineage: `docs/ux/FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §B.1 (route N), whose verb
map and error-presentation table remain normative.

## 1. Purpose

*"I want to tell the model as a train of thought."* A linear, literate view over live model
objects — the Pluto/Jupyter virtue (linear story, rich local results) without their vice
(copies drifting from truth). Blocks are views, never copies. Works over both profiles;
workbook profile ships first (S2), tree follows with S4.

## 2. Block model

| Block kind | Backing (workbook profile) | Notes |
|---|---|---|
| **Name entry** (primary idiom) | defined name (`DefinedNamesProjection`, `CreateNamedValue`, `SetDefinedName`) | value or formula; the literate spine |
| **Cell entry** (escape hatch) | grid cell via `EnterGridCell` three-way outcome | shown with its address chip |
| **Table entry** | `TableProjection` + table intents | header/totals-aware card |
| **Prose** | manifest layer (annotation; never model cells) | serif, Reading density, ~68ch |
| **Scenario chips** | `ScenarioManifestProjection` + `ActivateScenario` / `SetScenarioOverride` | per-notebook bar, mech "fork lightly" |

Ordering + prose live in the document manifest (xlsx + embedded manifest, ratified F.2/F.4;
`_names` hidden backing sheet). **G6** turns this from host convention into an IR projection
(blocks, order, cursor); pre-G6 the host-core owns manifest read/write and the stage renders
its ad-hoc projection — degrade documented, no skin-side manifest parsing.

## 3. Layout & interaction

- Single column, Reading density for prose, Working density inside entry blocks; block gutter
  carries kind glyph + classification tint (structural, per style law).
- An entry block = name row (name · classification chip · liveness dot) + Bridge-grammar editor
  (shared `dnacalc-bridge`, degrade rules per SHELL_SPEC §6) + result region (value, bounded
  array window — needs **G5** for windows outside OneFormula; degrade: shape summary + "open in
  Sheet/Inspector" affordance instead of a fake full render).
- Block operations: add (name / cell / table / prose), reorder (drag or Alt+↑/↓), collapse to
  summary line, delete (ghost-preview + Esc revert via preview seam where present).
- Scenario chips row: switching chips re-renders affected blocks with the scenario tint
  (provenance law); `override_values` render as editable inputs inside the scenario.
- Reference X-Ray: caret in a block's formula highlights other blocks it reads (block-to-block
  arrows on the gutter), mechanism 07 at notebook granularity.
- Published/locked mode: persona Reviewer renders the same stage read-only — this *is* the
  report artifact (no separate export surface in v1). (Future trace, deferred: a
  "verified against evidence" footer badge for readers — [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md) §5.)

## 4. Keyboard (beyond universal)

Enter opens the focused block's editor · Shift+Enter commit-and-next-block (notebook muscle
memory) · Alt+↑/↓ reorder · Ctrl+Shift+M new prose block · Ctrl+Shift+N new name block.
All rebindable; atlas-tagged.

## 5. Gaps this stage consumes

**G6** (blocks projection — the defining ask) · **G5** (array windows in workspace) · G1
(rich editing in blocks; pre-G1 the Bridge degrade applies) · G8 (deck entries for block ops).

## 6. Acceptance sketch (hardens at S2)

1. Open the W011 fixture workbook: names render as blocks; edit `A1`-backed input block →
   dependent name block re-renders; save; reload; order and prose intact (manifest round-trip).
2. Create name block `GrowthRate = 12%`; reference it from a second block; rename it in the
   Registry → notebook text updates (rename-is-refactor path).
3. Two scenario chips with one override each; switching swaps rendered values with scenario
   tint; no model mutation outside the scenario.
4. Reviewer persona: fully readable, zero enabled mutations, prints/reads as the report.

## 7. Open questions

- Prose richness (markdown subset) — v2 per route map; v1 is plain paragraphs.
- Whether cell-entry blocks show a mini grid-context strip (3×3 neighborhood) or address only.
- Block-level "explain" (auto-derived dependency sentence) — candidate mechanism, not committed.

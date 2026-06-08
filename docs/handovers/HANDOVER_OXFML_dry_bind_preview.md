# HANDOVER_OXFML_dry_bind_preview

Status: Partial
Target: OxFml
Ask: Add an OxFml-owned dry-bind entrypoint for uncommitted formula edits so DnaTreeCalc can surface profile legality and bind diagnostics in pre-commit previews without mutating or evaluating.
Context: DnaTreeCalc W2 legality-impact preview is a thin host join over OxFml dry-bind plus OxCalc committed-graph invalidation planning. OxCalc now exposes the recalc-plan half, but there is no current OxFml dry-bind API for an unexecuted TreeCalc edit.
Evidence: `DnaTreeCalc/docs/ux/stack-requirements/ENGINE_REQUIREMENTS.md` `engine-dry-bind`; `DnaTreeCalc/docs/ux/stack-requirements/ROADMAP.md` W2; OxCalc `OxCalcTreeContext::plan_invalidation`.

## Needed Shape

TreeCalc needs a parse/bind-only API along these lines:

```rust
dry_bind(edit: &PreviewMutation, profile: CapabilityProfile) -> BindVerdict
```

Minimum result fields:

- `profile_violations`: typed profile feature rejects.
- `bind_diagnostics`: typed diagnostics with source spans, matching the committed bind diagnostic vocabulary.
- `would_rebind`: stable target identities or handles when OxFml can identify formula text/reference recomposition consequences.

## Boundary

OxFml owns grammar, parse, bind, profile gates, diagnostics, and reference-text composition. OxCalc owns committed-graph invalidation, rebind pressure, scheduling impact, and runtime publication. DnaTreeCalc host should only join the OxFml verdict with OxCalc's `plan_invalidation` result plus host-owned collision/scope facts.

## Current State

OxCalc now provides a committed-graph recalc preview:

- input: `OxCalcTreePreviewMutation`;
- output: invalidated nodes, conservative formula evaluation order, rebind-required nodes, estimated count, and cycle-risk groups;
- no candidate, evaluation, publication, or workspace mutation.

The missing W2 piece is the OxFml dry-bind verdict for the same uncommitted authoring intent.

## 2026-06-09 Update

First node-formula dry-bind slice is now available across the stack:

- OxFml `RuntimeEnvironment::dry_bind_authored_input(...)` returns parse/bind verdicts without
  evaluation or publication.
- OxCalc `OxCalcTreeContext::dry_bind_node_formula_text(...)` runs that verdict through the TreeCalc
  host-reference syntax, host-name resolver, and table context owned by OxCalc.
- DnaTreeCalc `TreeWorkspaceSession::preview_formula_bind(...)` projects the verdict into Skin IR.
- DnaTreeCalc `TreeWorkspaceSession::preview_content_edit_impact(...)` joins the node-content dry-bind
  verdict with OxCalc committed-graph invalidation planning for the first legality-impact preview
  slice.

Still open before this handover can be closed:

- typed profile-violation taxonomy and non-empty profile-gating evidence;
- scoped authoring subjects and table new-column preflight subjects beyond existing body/totals
  formula edits;
- broader legality-impact preview coverage for structural, table, scoped, and collision/orphan cases.

## 2026-06-09 Table Subject Update

Existing table body and totals formula edit previews now run through the same dry-bind path:

- OxCalc exposes `OxCalcTreeContext::dry_bind_table_column_formula_text(...)` and
  `OxCalcTreeContext::dry_bind_table_totals_formula_text(...)`.
- Those methods reuse the OxCalc table formula context: table descriptor, enclosing table,
  caller table region, primary locus, function registry, host formula context, and OxFml dry-bind.
- DnaTreeCalc projects the result as `TableFormulaBindPreviewProjection`, with table node, stable
  key, engine table id, column id, body/totals region, input kind, typed diagnostics, and profile
  violations.
- Programmable Skin IR tests exercise valid body current-row syntax, valid totals syntax, syntax
  diagnostics, and no workspace mutation.

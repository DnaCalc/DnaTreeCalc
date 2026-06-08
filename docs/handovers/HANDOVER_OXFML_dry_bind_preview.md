# HANDOVER_OXFML_dry_bind_preview

Status: Open
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

# OxReplay OneCalc Consumption Path

Status: `active_consumer_note`
Date: `2026-04-06`

## 1. Purpose
This note records how `DnaOneCalc` consumes `OxReplay` diff and explain outputs for retained discrepancy import, `Workbench`, and `Inspect`.

It is a local consumer note.
It does not redefine `OxReplay` mismatch semantics.

Primary upstream inputs:
1. [HANDOFF_OXREPLAY_XML_VERIFICATION_REQUEST.md](HANDOFF_OXREPLAY_XML_VERIFICATION_REQUEST.md)
2. [HANDOFF_XML_VERIFICATION_SCOPE.md](HANDOFF_XML_VERIFICATION_SCOPE.md)
3. `C:\Work\DnaCalc\OxReplay\docs\handoffs\HANDOFF_DNAONECALC_001_XML_VIEW_FAMILY_COMPARISON_RESPONSE.md`
4. `C:\Work\DnaCalc\OxReplay\docs\spec\OXREPLAY_DNA_ONECALC_CONSUMPTION_MODEL.md`

## 2. Consumer Rule
`DnaOneCalc` is a consumer of `OxReplay`.

It must:
1. preserve richer per-record diff and explain outputs when present,
2. keep `view_family`-typed divergence visible in retained artifacts and UI,
3. treat `projection_coverage_gap` as a real comparison constraint,
4. keep fallback compatibility with older coarse outputs such as `view_value`,
5. avoid synthesizing missing comparison families from raw replay events.

It must not:
1. reinterpret a missing comparison family as a semantic value mismatch,
2. backfill formatting or conditional-formatting comparison locally,
3. infer comparison coverage that upstream artifacts did not publish.

## 3. Accepted OxReplay Shapes
### 3.1 Diff
`DnaOneCalc` accepts `mismatches` records with:
1. `mismatch_kind`
2. `severity`
3. `view_family`
4. `left_value`
5. `right_value`
6. `detail`

Current XML-family expectations:
1. `visible_value`
2. `effective_display_text`
3. `formatting_view`
4. `conditional_formatting_view`

### 3.2 Explain
`DnaOneCalc` accepts `records` entries with:
1. `query_id`
2. `summary`
3. `mismatch_kind`
4. `severity`
5. `view_family`
6. `left_value`
7. `right_value`
8. `detail`

## 4. Local Mapping Rule
The local mapping is:
1. `effective_display_text` divergence -> display divergence summary
2. `visible_value` divergence -> visible-value divergence summary
3. `projection_coverage_gap` -> per-family coverage-gap summary
4. older coarse `view_value` without `view_family` -> fallback visible-value divergence

When richer per-family records are present:
1. `Workbench` and `Inspect` should prefer them over coarse upstream-gap summaries,
2. the family id should remain visible in the user-facing summary,
3. retained artifacts should preserve the full typed records.

When richer per-family records are absent:
1. `DnaOneCalc` may fall back to coarse mismatch-kind handling,
2. the fallback should remain visibly coarse,
3. the UI should not pretend that missing view-family detail exists.

## 5. Retained Artifact Rule
Imported retained discrepancy artifacts preserve:
1. `replay_mismatch_kinds`
2. `replay_mismatch_records`
3. `replay_explain_records`
4. source XML context where present
5. upstream gap reports where present

This lets `Workbench` and `Inspect` show:
1. display divergence as display divergence,
2. formatting or conditional-formatting absence as coverage gaps,
3. older coarse replay mismatches without inventing per-family detail.

## 6. Current Honest Limitation
Even with the richer `OxReplay` consumer path in place, `DnaOneCalc` still depends on upstream publication of `comparison_views`.

That means:
1. `OxReplay` can now classify those families honestly,
2. but `DnaOneCalc` can only show family-specific divergence when upstream artifacts actually publish those families,
3. missing publication from `OxFml` or `OxXlPlay` remains an upstream projection constraint, not a local UI bug.

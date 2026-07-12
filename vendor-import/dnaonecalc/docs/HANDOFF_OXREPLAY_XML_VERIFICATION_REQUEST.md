# OxReplay Handoff: Display and Formatting Comparison Views

## Why this handoff exists
OneCalc now produces verification bundles that contain:
- OxFml replay projection
- OxXlPlay capture
- XML extraction facts
- required observation scope
- upstream gap report

The first real XML-backed run exposed a real comparison seam:
- visible numeric result matches semantically
- display/value presentation diverges
- current diff classification lands as `view_value`

That is too coarse.

Representative evidence:
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\comparison-summary.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/comparison-summary.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\upstream-gap-report.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/upstream-gap-report.json)

## Required OxReplay views
OneCalc now needs comparison views for:

- `visible_value`
- `effective_display_text`
- `formatting_view`
- `conditional_formatting_view`

Current bundle views exposed to OneCalc are effectively:
- `visible_value`
- `replay_normalized_events`

## Concrete ask
Expand OxReplay’s normalization/diff/explain pipeline so it can:

1. Normalize comparison-ready view families for both sides:
- visible value
- effective display text
- formatting view
- conditional-formatting view

2. Diff them independently and coherently:
- visible value equivalence
- display equivalence
- formatting equivalence
- conditional-formatting equivalence

3. Improve classification:
- if visible values match but display or formatting diverges, classify that explicitly
- if one side lacks a required view family, classify that as a projection/coverage gap rather than a plain semantic mismatch

4. Explain output:
- explanation artifacts should state which view family diverged or was missing
- explanation should be suitable for OneCalc Workbench/Inspect import without local reinterpretation

## Constraints
- Do not require OneCalc to post-process raw replay event streams into higher-level formatting views.
- Prefer typed or stable machine-readable outputs over human-oriented console text.

## Downstream consumer
These richer view families are intended to flow directly into:
- verification bundle summaries
- retained discrepancy records
- Workbench outcome/evidence panels
- Inspect retained-artifact context

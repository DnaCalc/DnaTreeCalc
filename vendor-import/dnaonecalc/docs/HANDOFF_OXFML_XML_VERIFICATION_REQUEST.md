# OxFml Handoff: XML Verification Publication Scope

## Why this handoff exists
OneCalc now runs a real XML-backed verification bundle path for SpreadsheetML 2003 cell cases. The OneCalc side can extract workbook/cell/style facts and can evaluate through live OxFml, but the published OxFml replay/result surfaces are still too thin for display-faithful comparison against Excel-observed output.

Representative evidence:
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\comparison-summary.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/comparison-summary.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\required-observation-scope.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/required-observation-scope.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\upstream-gap-report.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/upstream-gap-report.json)

Observed seam:
- OxFml effective display summary: `6`
- Excel observed display: `$6.00`
- Replay diff currently lands as `view_value`

That is a publication/comparison seam, not a OneCalc-local formatting bug.

## Required OxFml outputs
For an XML-backed verification case, OneCalc now needs OxFml to publish enough result and presentation context for replay and twin-oracle comparison. The required scope is:

- `entered_cell_text`
- `returned_value_surface`
- `effective_display_text`
- `format_profile`
- `locale_format_context`
- `date1904`
- `number_format_code`
- `style_id`
- `style_hierarchy`
- `format_dependency_facts`
- `format_delta`
- `display_delta`
- `presentation_hint`
- `font_color`
- `fill_color`
- `conditional_formatting_rules`
- `conditional_formatting_target_ranges`
- `conditional_formatting_rule_kind`
- `conditional_formatting_operator`
- `conditional_formatting_thresholds`
- `conditional_formatting_effective_display`

## Concrete ask
Add an OxFml-owned publication surface for verification/replay export that includes:

1. Formula evaluation result:
- typed value
- error/value kind
- effective display text

2. Formatting-relevant publication context:
- number format code used for display
- date base / `Date1904`
- style id and any style lineage/fallback summary
- color surfaces already supported by OxFml presentation logic

3. Conditional-formatting-relevant publication context:
- rules that apply to the target cell
- evaluation-relevant thresholds/operators
- effective display/style consequences visible to the host

4. Replay export shape:
- replay projection should carry the above in a stable comparison-friendly structure
- OneCalc should not need to infer display/format semantics locally from raw formula results

## Constraints
- Do not push this interpretation burden into OneCalc.
- Keep OxFml responsible for its own value/display/publication semantics.
- Prefer a stable typed replay/result contract over ad hoc JSON fragments.

## Downstream consumer
The immediate downstream consumer is the OneCalc verification bundle CLI and retained-artifact flow. The medium-term consumer is Workbench/Inspect for discrepancy triage.

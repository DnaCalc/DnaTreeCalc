# SpreadsheetML Verification Scope Handoff

## Purpose
Record the current OneCalc-owned verification scope for SpreadsheetML 2003 workbook cases, and the exact upstream observation gaps now exposed by a real CLI run.

## Current local path
OneCalc now supports:
- `verify-xml-cell --case-id <id> --workbook-xml <path> --locator <Sheet!Cell>`
- SpreadsheetML 2003 cell extraction for:
  - entered cell text
  - formula text
  - worksheet/cell locator
  - style id
  - number format code
  - font color
  - fill color
  - conditional-formatting rules affecting the target cell
  - workbook `Date1904`
- retained verification bundle output with:
  - copied workbook XML
  - extracted XML facts
  - required observation scope
  - upstream gap report
  - OxFml replay projection
  - OxXlPlay capture
  - OxReplay diff/explain outputs

## Representative run
Executed from OneCalc:

- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\verification-bundle-report.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/verification-bundle-report.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\comparison-summary.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/comparison-summary.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\required-observation-scope.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/required-observation-scope.json)
- [C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-1\cases\xml-case-1\upstream-gap-report.json](/C:/Work/DnaCalc/DnaOneCalc/target/onecalc-verification/manual-xml-case-1/cases/xml-case-1/upstream-gap-report.json)

Observed result:
- OxFml effective display summary: `6`
- Excel observed surface: `$6.00`
- replay diff mismatch kind: `view_value`

This is a real display/format seam, not just a theoretical requirement.

## Required OxFml scope
OneCalc now records this required OxFml scope for XML-backed verification:
- `entered_cell_text`
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
- `returned_value_surface`
- `font_color`
- `fill_color`
- `conditional_formatting_rules`
- `conditional_formatting_target_ranges`
- `conditional_formatting_rule_kind`
- `conditional_formatting_operator`
- `conditional_formatting_thresholds`
- `conditional_formatting_effective_display`

## Required OxXlPlay surfaces
OneCalc currently needs these Excel-observed surfaces for XML-backed verification:
- `formula_text`
- `cell_value`
- `effective_display_text`
- `number_format_code`
- `style_id`
- `font_color`
- `fill_color`
- `conditional_formatting_rules`
- `conditional_formatting_effective_style`

Current active OxXlPlay support used by OneCalc:
- `cell_value`
- `formula_text`

## Required OxReplay views
OneCalc currently needs these comparable replay views:
- `visible_value`
- `effective_display_text`
- `formatting_view`
- `conditional_formatting_view`

Current active bundle views used by OneCalc:
- `visible_value`
- `replay_normalized_events`

## Upstream follow-up targets
Detailed repo-specific requests:
- [OxFml XML verification request](C:/Work/DnaCalc/DnaOneCalc/docs/HANDOFF_OXFML_XML_VERIFICATION_REQUEST.md)
- [OxXlPlay XML verification request](C:/Work/DnaCalc/DnaOneCalc/docs/HANDOFF_OXXLPLAY_XML_VERIFICATION_REQUEST.md)
- [OxReplay XML verification request](C:/Work/DnaCalc/DnaOneCalc/docs/HANDOFF_OXREPLAY_XML_VERIFICATION_REQUEST.md)

### OxXlPlay
Add observation support for:
- effective display text
- number format code
- style id / style lineage if applicable
- font color
- fill color
- conditional-formatting rule capture
- conditional-formatting effective style capture

### OxReplay
Add comparison support for:
- effective display text
- formatting view
- conditional-formatting view

Also improve classification where visible values may match while format/display families diverge.

### OxFml
Strengthen bundle/replay publication so OneCalc can compare:
- visible value
- effective display
- format-significant publication consequences
- conditional-formatting-significant publication consequences

without forcing OneCalc to invent local interpretation.

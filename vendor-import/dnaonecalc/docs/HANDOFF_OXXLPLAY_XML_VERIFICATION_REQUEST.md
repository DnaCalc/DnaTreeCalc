# OxXlPlay Handoff: SpreadsheetML 2003 Observation Expansion

## Why this handoff exists
OneCalc now feeds SpreadsheetML 2003 workbook cases into the verification bundle path and uses the original XML workbook as the Excel-readable source artifact for OxXlPlay scenarios.

Current OneCalc extraction already records:
- formula text
- entered cell text
- style id
- number format code
- font color
- fill color
- conditional-formatting rules
- workbook `Date1904`

Current OxXlPlay-observed surfaces, from OneCalc’s perspective, are still effectively:
- `formula_text`
- `cell_value`

That is not enough for the twin-oracle comparison we want.

## Required OxXlPlay-observed surfaces
For a target workbook/cell case, OneCalc now needs OxXlPlay to capture:

- `formula_text`
- `cell_value`
- `effective_display_text` such as `Range.Text`
- `number_format_code`
- `style_id`
- `font_color`
- `fill_color`
- `conditional_formatting_rules`
- `conditional_formatting_effective_style`

## Concrete ask
Extend OxXlPlay’s scenario/capture flow so a SpreadsheetML 2003 workbook case can yield:

1. Basic observation:
- workbook path
- worksheet
- cell locator
- formula text
- value surface
- effective display text

2. Formatting observation:
- number format code
- style id
- relevant font/fill color surfaces

3. Conditional-formatting observation:
- rules affecting the target cell
- effective formatting/display consequence after Excel applies CF

4. Output packaging:
- emit these as stable scenario/capture outputs that OneCalc can retain directly
- avoid a shape that requires OneCalc to scrape ad hoc logs or console text

## SpreadsheetML note
The input workbook format here is XML Spreadsheet 2003 rather than `.xlsx`. That is intentional. If OxXlPlay has gaps in directly opening this format or preserving the relevant style/CF surfaces, those should be fixed upstream rather than worked around inside OneCalc.

## Downstream consumer
OneCalc will carry these captures into:
- verification bundle comparison
- retained discrepancy artifacts
- Workbench triage
- Inspect context

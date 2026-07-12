*Posted by Codex agent on behalf of @govert*

# OxFml Prompt: Fix XML Verification Effective Display / Formatting Publication

Use this prompt in the `OxFml` repo.

## Prompt

Process the DnaOneCalc XML verification mismatch and fix OxFml’s verification publication / replay projection so the XML-backed single-cell verification lane matches Excel more closely.

Read these first:
1. `C:\Work\DnaCalc\DnaOneCalc\docs\HANDOFF_XML_VERIFICATION_SCOPE.md`
2. `C:\Work\DnaCalc\DnaOneCalc\docs\HANDOFF_OXFML_XML_VERIFICATION_REQUEST.md`
3. `C:\Work\DnaCalc\DnaOneCalc\docs\HANDOFF_OXFML_EFFECTIVE_DISPLAY_XML_PROMPT.md`
4. `C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-live-verify2\cases\xml-case-1\xml-cell-extract.json`
5. `C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-live-verify2\cases\xml-case-1\oxfml-v1-replay-projection.json`
6. `C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-live-verify2\cases\xml-case-1\comparison-summary.json`
7. `C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-live-verify2\cases\xml-case-1\oxreplay\diff.report.json`
8. `C:\Work\DnaCalc\OxReplay\docs\spec\OXREPLAY_DNA_ONECALC_CONSUMPTION_MODEL.md`

Goal:
Fix OxFml’s XML verification publication so the replay/publication lane emits correct effective display and formatting-family comparison views for a SpreadsheetML 2003 workbook cell under the default Windows Excel host profile.

Concrete reproduced case:
- Workbook: `C:\Work\DnaCalc\OxXlPlay\docs\test-corpus\excel\xlplay_capture_spreadsheetml_formatting_001\workbook.xml`
- Locator: `Input!A1`
- Formula: `=SUM(1,2,3)`
- Number format code from XML: `$#,##0.00`
- Style hierarchy from XML: `calcBase -> calc`
- Excel effective display: `$6.00`
- OxFml effective display currently emitted: `$$6.00`

What DnaOneCalc now does correctly:
- passes SpreadsheetML-derived verification publication context into OxFml runtime
- includes number format, style id, style hierarchy, font color, fill color, conditional-formatting rules, and workbook `Date1904`
- preserves OxFml `comparison_views` and `verification_publication_surface` in the retained replay projection
- compares those views through OxReplay without synthesizing missing families locally

Current observed mismatch:
- `effective_display_text` diverges:
  - OxFml: `$$6.00`
  - Excel: `$6.00`
- `formatting_view` and `conditional_formatting_view` are also still diverging at the compared-view level
- the remaining gap is no longer in DnaOneCalc transport; it is in OxFml publication / formatting semantics

Required OxFml work:
1. Trace why the XML-backed locale/formatting path produces `$$6.00` instead of `$6.00`.
2. Fix the formatting/publication path so a currency format code such as `$#,##0.00` does not double-emit currency symbols in `effective_display_text`.
3. Validate the replay projection’s `comparison_views.formatting_view` against the SpreadsheetML-derived style context being provided by DnaOneCalc.
4. Validate the `conditional_formatting_view` publication path against the source-declared SpreadsheetML expression rule and effective style.
5. Keep the publication model consumer-facing and honest:
   - no DnaOneCalc-side synthesis
   - no weakening to coarse visible-value-only comparison
6. Preserve backward compatibility for non-XML callers.

Constraints:
- Treat DnaOneCalc as a consumer, not the place to reinterpret formatting semantics.
- Use the retained DnaOneCalc verification bundle as the concrete failing fixture.
- Do not remove `comparison_views`; fix the semantics they publish.
- Keep the verification lane aligned with the broader XML verification plan, not just this single symptom.

Please:
1. Implement the OxFml fix.
2. Add or update OxFml tests covering this exact XML-backed currency-format case.
3. Report:
   - what was wrong
   - what changed
   - whether `effective_display_text`, `formatting_view`, and `conditional_formatting_view` now align better with Excel for this case
4. Provide the exact DnaOneCalc command that should be rerun to verify the fix:
   - `cargo run -p dnaonecalc-host -- verify-xml-cell --case-id xml-case-1 --workbook-xml C:\Work\DnaCalc\OxXlPlay\docs\test-corpus\excel\xlplay_capture_spreadsheetml_formatting_001\workbook.xml --locator Input!A1 --output-root C:\Work\DnaCalc\DnaOneCalc\target\onecalc-verification\manual-xml-case-live-verify2`

## Expected outcome

The next DnaOneCalc verification run should still compare through OxReplay, but the primary family mismatch should no longer be the doubled-currency effective display.

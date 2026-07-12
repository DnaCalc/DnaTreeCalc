*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Per-Cell Conditional-Formatting Evaluation on Array Values

Status: closed (OxFml landing acknowledged 2026-05-04 as W071)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Closed date: 2026-05-04
Related:
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-008_W071_CF_ARRAY_PER_CELL_LANDING.md`,
  `OxFml/crates/oxfml_core/src/publication/mod.rs`,
  `OxFml/crates/oxfml_core/tests/conditional_formatting_array_tests.rs`,
  `docs/HANDOFF_OXFML_CF_PREDICATE_AND_RELATIVE_DATE_RULES.md` (W070 closed),
  `docs/HANDOFF_OXFML_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md` (W069 closed),
  `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md` (W072 closed)

## Closure note (2026-05-04)

OxFml's W071 landing introduced the per-cell carrier
`VerificationPublicationSurface.array_cell_format: Option<ArrayCellFormatGrid>`
with row-major `ArrayCellFormat` entries carrying `effective_display_text`
/ `effective_font_color` / `effective_fill_color` / `data_bar` /
`icon`. Operator and predicate rules (cell_value, text, dates,
blanks/noBlanks, errors/noErrors) evaluate per-cell. 1×1 arrays
agree with the whole-cell CF fields. The `data_bar` / `icon`
slots stayed `None` until W072 visualization work landed —
covered by the sibling handoff.

DnaOneCalc absorbed the carrier the same day:

- Mirrored the per-cell types in
  `src/dnaonecalc-host/src/adapters/oxfml/types.rs`.
- Lifted `array_cell_format` from the publication surface in
  `live_bridge.rs::map_value_presentation`.
- Added `cell_format` to `ResultView::Array`.
- Renderer applies per-cell font / fill via inline style on
  each `.onecalc-array-browser__cell`.

## OneCalc-side mental model

A DnaOneCalc result hero showing an array result behaves *as if*
the user had selected the whole spilled result area in Excel and
applied the formatting / CF rules to that selection. Excel then
does per-cell formatting — the colour, shading, bar, and icon
fall out of evaluating each cell against the rules with the
selected range as the aggregate context. CF rules attached to a
formula in OneCalc need to flow through OxFml the same way:

- For *operator-driven* and *predicate* rules (`cell_value`,
  `text`, `dates`, `blanks`, `errors`, `expression`), each cell
  evaluates independently against the rule's thresholds.
- For *aggregate-context* rules (`colorScale`, `dataBar`,
  `iconSet`, `aboveAverage`, `belowAverage`, `top`, `bottom`,
  `uniqueValues`, `duplicateValues`), each cell evaluates with
  the *array as the implicit range* — min / max / mean / sorted
  order / unique-set computed once, then per-cell colour, fill,
  bar fill ratio, or icon falls out.

This handoff covers **the operator / predicate side**; the
aggregate-context visualization kinds are detailed in
`docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`. They
share the per-cell publication-surface shape this handoff
introduces — a single `array_cell_format` carrier that the
visualization handoff extends with bar / icon fields.

## Symptom

DnaOneCalc renders array results in a scrollable result-hero
browser. The user attaches a CF rule "highlight green when value
> 3" to a formula that returns `=SEQUENCE(2,3)` (cells `1..6`).
Excel's expectation, observed against an authored CF rule on a
spill range: cells `4`, `5`, `6` highlight green; `1`, `2`, `3`
do not.

DnaOneCalc against the same rule today: nothing highlights at
all. Tracing through OxFml's `compare_threshold`:

```rust
match value {
    EvalValue::Number(number) => {
        let threshold_value = parse_threshold_number(threshold, locale_ctx)?;
        number.partial_cmp(&threshold_value)
    }
    // ... Text / Logical / Error branches
    _ => Some(visible_value_text.cmp(strip_threshold_quotes(threshold))),
}
```

For `EvalValue::Array(_)`, control falls through to the `_ =>`
arm, which compares `visible_value_text` (the whole array
stringified, e.g. `"{1,2,3;4,5,6}"`) against the threshold
string `"3"`. The lexicographic compare returns nonsense; the
rule's `applies` ends up `Some(false)` (or `None`), the
publication surface emits a single `effective_font_color` /
`effective_fill_color` for the whole array (today: `None` /
`None`), and the host's array-browser cells render uncoloured.

## Architectural ask

CF rules on an array result evaluate **per-cell**. The
publication surface for an array carries per-cell applied
formatting alongside the whole-array fields:

```rust
pub struct VerificationPublicationSurface {
    // ... existing fields (whole-cell summary)
    pub effective_display_text: String,
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,

    /// Per-cell CF outcomes when `published_value` is an Array.
    /// `None` for non-array values. Outer index = row, inner = col.
    /// Each entry carries the CF outcome for that single cell:
    /// font / fill colour overrides and any CF-rule-supplied
    /// display-text override.
    pub array_cell_format: Option<ArrayCellFormatGrid>,
}

pub struct ArrayCellFormatGrid {
    pub rows: Vec<Vec<ArrayCellFormat>>,
}

pub struct ArrayCellFormat {
    pub effective_display_text: String,
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,
}
```

Inside the publication-surface builder, the per-cell evaluation
runs `evaluate_conditional_formatting_rule` once per cell with
that cell's `EvalValue` (extracted from the
`EvalValue::Array(...)` shape). Each rule's applicability is
decided independently per cell; the *rule list itself* is the
same for every cell (cell-level rules are scalar; the per-cell
expansion is purely about the value being matched).

The whole-array fields stay so the existing single-cell consumer
path is unaffected. For a 1×1 array (the OxFunc 1×1-publication
seam), `array_cell_format` is populated alongside the whole-cell
fields and they agree.

## Behavioural notes

- **Cell extraction order** — row-major to match the host's array
  browser; `ArrayCellFormatGrid.rows[row][col]` is the cell at
  worksheet position `(row, col)` relative to the array's
  top-left.
- **Effective display text per cell** — when a CF rule's
  `effective_display_text` fires (e.g. a custom-format rule like
  `"[POS] $#,##0.00"` against a positive cell), the per-cell
  override replaces only that cell's display text. Other cells
  show their normal formatted display.
- **Default colour fallback** — when no rule applies for a given
  cell, both `effective_font_color` and `effective_fill_color`
  are `None` (host renders the cell in default chrome).
- **Locale / now_serial** — the per-cell evaluation reuses the
  same `LocaleFormatContext` and `now_serial` as the whole-cell
  path. Relative-date predicates (W070) work per-cell against
  the same wall clock.
- **Empty cells inside Array** — `EvalValue::Array` carries
  `ArrayCellValue` per slot, which can include "empty". The CF
  predicate dispatch for `blanks` should fire for empty array
  cells, matching its single-cell behaviour.

## Test coverage

In `oxfml_core/tests/conditional_formatting_predicate_tests.rs`
(or a sibling `conditional_formatting_array_tests.rs`):

1. `=SEQUENCE(2,3)` (values `1..6`) with rule `cell_value`,
   `greaterThan`, threshold `"3"`, fill `"#E6F2D9"` → cells
   `(0,0)..(1,2)` map to applies `[false,false,false; true,true,true]`,
   only the latter three carry the green fill.
2. `={1, "x", #DIV/0!; 2, "y", #N/A}` with rule `errors` →
   `effective_fill_color` populated only on the `#DIV/0!` and
   `#N/A` cells.
3. `=SEQUENCE(3)` with rule `dates`, `today`, `now_serial =
   Some(46045.0)` against an integer-valued array — cells equal
   to today's serial get the highlight; the rest don't.
4. 1×1 array result: `array_cell_format.rows[0][0]` matches the
   whole-cell `effective_*` fields exactly.

## Out-of-scope here (covered separately)

- **Aggregate-context visualization kinds** (`colorScale`,
  `dataBar`, `iconSet`, `aboveAverage`, `belowAverage`, `top` /
  `bottom`, `uniqueValues`, `duplicateValues`) are detailed in
  `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`.
  Same per-cell carrier introduced here — that handoff just
  grows it with bar / icon fields and pre-computes the array's
  aggregate context once before the per-cell pass.
- **Worksheet-range CF semantics** (rules attached to a real
  worksheet range, with the range carrier separate from the
  formula). DnaOneCalc has no notion of "range-attached CF"
  today; if and when a worksheet-range surface lands, separate
  handoff.

## DnaOneCalc-side work after this lands

1. Bridge surface grows
   `array_cell_format: Option<ArrayCellFormatGrid>` (or
   equivalent) on `FormulaValuePresentation`.
2. The array browser reads per-cell `effective_font_color` /
   `effective_fill_color` and applies them to the cell `<div>`s
   via inline styles or data-attribute-driven CSS.
3. Per-cell `effective_display_text` overrides the existing
   `format_array_cell_value` rendering when present.
4. Visual regression: take a screenshot of `=SEQUENCE(2,3)` with
   a `> 3` green-fill rule and pin that the bottom row is green.

## Closure conditions

- `VerificationPublicationSurface` carries per-cell CF
  outcomes for array results (or equivalent shape).
- Single-cell consumers see no change.
- Tests above pass.
- DnaOneCalc consumes the per-cell carrier and renders array CF
  in the result-hero browser.

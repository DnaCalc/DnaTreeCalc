*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Aggregate-Context CF — Color Scales, Data Bars, Icon Sets, Rank, Average, Unique

Status: closed (OxFml landing acknowledged 2026-05-04 as W072)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Closed date: 2026-05-04
Related:
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-009_W072_CF_AGGREGATE_VISUALIZATION_INTAKE.md`,
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-010_W072_AGGREGATE_PREDICATE_SLICE.md`,
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-011_W072_VISUALIZATION_RULE_SLICE.md`,
  `docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md` (W071 closed — per-cell carrier),
  `OxFml/crates/oxfml_core/src/publication/mod.rs`,
  `OxFml/crates/oxfml_core/tests/conditional_formatting_array_tests.rs`

## Closure note (2026-05-04)

OxFml's W072 work landed across two slices:

**Aggregate predicates** (`HANDOFF-DNAONECALC-010`):
`aboveAverage`, `belowAverage`, `top` (count + percent),
`bottom` (count + percent), `uniqueValues`, `duplicateValues`.
Aggregate context — array mean, sorted numeric values,
visible-value counts — computed once per evaluation.

**Visualization rules** (`HANDOFF-DNAONECALC-011`): `colorScale`,
`dataBar`, `iconSet`, with bounded-payload conventions over the
existing `VerificationConditionalFormattingRule` shape until a
richer typed payload lands:

- `colorScale` stops in `thresholds`: `"min:#F8696B"`,
  `"mid:#FFEB84"`, `"max:#63BE7B"`, `"percent:50:#FFEB84"`,
  `"num:42:#63BE7B"`. Sets per-cell `effective_fill_color` via
  gradient interpolation.
- `dataBar` colour from `fill_color`; optional `thresholds`
  entries `"direction:right"` and `"showBarOnly"`. Sets per-cell
  `data_bar`.
- `iconSet` kind from `thresholds[0]` (default `3Arrows`),
  equal-width min / max numeric bins. Sets per-cell `icon`.

Mixed visualization + scalar rules preserve per-field priority:
a later scalar rule can set a font colour without erasing an
earlier color-scale fill.

DnaOneCalc absorbed both slices the same day:

- `cf_seam_id_for_kind` no longer marks the nine aggregate +
  visualization rule kinds — all live.
- `seed_visualization_rule_defaults` auto-fills the bounded-
  payload conventions when the user picks a visualization kind
  in the UI dropdown (3-stop red-yellow-green for colorScale,
  blue bar for dataBar, `3Arrows` default for iconSet, count of
  10 for top / bottom).
- Array browser renders per-cell fill colour, data-bar overlay
  (proportional inline-block sized via `fill_ratio`), and icon
  glyph (Unicode mapping for the 3 / 4 / 5-icon sets).

## Remaining open lanes (per OxFml HANDOFF-011)

### Resolved 2026-05-04 by W073 (`HANDOFF-DNAONECALC-012`)

OxFml shipped the typed payload on
`VerificationConditionalFormattingRule.typed_rule` and, in the
2026-05-04 update of the handoff, **removed the W072 bounded-
string fallback for the seven typed families**:

- `colorScale`, `dataBar`, `iconSet`, `top`, `bottom`,
  `aboveAverage`, `belowAverage` — `typed_rule` is the only
  accepted metadata source. Bounded `thresholds` are
  intentionally ignored for these kinds.
- Typed sub-options for `colorScale` (ordered stops with
  `min` / `mid` / `max` / `percent` / `percentile` / numeric
  positions), `dataBar` (typed min / max bounds, bar colour,
  direction, show-bar-only), `iconSet` (kind + explicit
  threshold sequence), `top` / `bottom` (typed count or
  percent), and `aboveAverage` / `belowAverage` (typed
  equal-average and stddev multiplier).
- `thresholds` remains the rule input for scalar / operator /
  expression families (`cell_value`, `text`, `dates`,
  `expression`).

Host followed: every kind switch into a W073 family seeds a
sensible `typed_rule` and clears any stale bounded thresholds;
the persistence loader drops stale entries when reading older
files. Implementation captured in
`docs/WS14_DESIGN_BACKLOG_2026-05-04.md` §2 and
`docs/FORMATTING_CF_TOPIC_STATE_2026-05-04.md` §5.

### Still open

- Explicit `formula` stop type for color-scale rules
  (currently positions are limited to min/mid/max/percent/
  percentile/number — no `formula` stop yet).
- Negative-axis / gradient-fill options for data bars.
- Excel's full library of icon-set thresholds beyond the
  ordered numeric sequence (e.g. mixed-kind thresholds).

## OneCalc-side mental model

A single-formula array result on the DnaOneCalc result hero is
the user's spilled selection in Excel. CF rules attached to that
formula apply *as if* the user had selected the whole spilled
range in Excel and applied the rule to the selection. Excel
then evaluates the rule **with the selected range as the
aggregate context**: min / max / mean / sorted order / unique
set computed once, then per-cell colour / fill / bar fill /
icon kind falls out.

This handoff asks OxFml to support that aggregate-context
evaluation for the seven CF rule families that need it. They
share the per-cell publication-surface carrier introduced in
`docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md`; this handoff
extends the per-cell shape with bar / icon fields and
documents the aggregate computations.

## Affected rule kinds

### 1. `colorScale`

A 2-stop or 3-stop colour gradient. `thresholds` carries the
stop positions (`min`, `mid`, `max`, or absolute values), each
with an associated colour. The cell's value's position in the
range determines its interpolated colour.

Excel canonical shape:

```xml
<colorScale>
  <cfvo type="min"/>
  <cfvo type="percentile" val="50"/>
  <cfvo type="max"/>
  <color rgb="FFF8696B"/>
  <color rgb="FFFFEB84"/>
  <color rgb="FF63BE7B"/>
</colorScale>
```

OxFml-facing input shape (via `VerificationConditionalFormattingRule`
or a richer kind-specific shape):

- `rule_kind = "colorScale"`
- `thresholds`: ordered list of stops, each `(stop_kind, value, color)`.

Per-cell output: `effective_fill_color` interpolated on the
gradient based on the cell's value's position in the array's
range.

### 2. `dataBar`

Proportional horizontal bar fill, with bar colour, fill
direction, and an optional gradient.

Per-cell output (new shape on the per-cell carrier):

```rust
pub struct DataBarFill {
    /// 0.0 — 1.0; the cell's value's position scaled between
    /// the rule's lower and upper bounds (typically array min /
    /// max, but Excel allows explicit min/max thresholds).
    pub fill_ratio: f64,
    /// Hex `#RRGGBB`.
    pub bar_color: String,
    /// `Left` (positive bar grows right) or `Right` (negative
    /// bar grows left). Excel default is `Left`.
    pub direction: DataBarDirection,
    /// True when Excel's "show bar only" was set (no number
    /// rendered alongside).
    pub show_bar_only: bool,
}
```

Aggregate computation: array min / max once; per-cell
`fill_ratio = (cell - min) / (max - min)`.

### 3. `iconSet`

Categorical icons (3-arrows, 5-quartiles, traffic-lights, etc.)
based on the cell's value's quantile / threshold position.

Per-cell output (new shape on the per-cell carrier):

```rust
pub struct CfIcon {
    /// Icon-set identifier — Excel's published kind, e.g.
    /// `3Arrows`, `3TrafficLights1`, `4Rating`, `5Quarters`.
    pub set_kind: String,
    /// 0-based index into the set's icon array.
    pub icon_index: usize,
}
```

Aggregate computation: sort or quantile-bin the array's values;
per-cell index falls out of where the value lands in the bins.

### 4. `aboveAverage` / `belowAverage`

Predicate fires for cells whose value is above (or below) the
array's mean. `thresholds` may carry stddev offset (e.g.
"above 1 stddev"); `equalAverage` flag controls whether the
mean itself is included.

Per-cell output: `effective_font_color` / `effective_fill_color`
when applies.

Aggregate computation: array mean (and optional stddev) once;
per-cell `applies = cell > mean + k*stddev` (or analogous).

### 5. `top` / `bottom`

Highlight the top-N (or bottom-N) values in the array, or the
top-N% / bottom-N%. `thresholds[0]` carries the count or
percentage; a `bottom: bool` flag inverts.

Per-cell output: `effective_font_color` / `effective_fill_color`
when applies.

Aggregate computation: sort the array's numeric values; the
top-N cut-off threshold falls out; per-cell `applies = cell ≥
cutoff`.

### 6. `uniqueValues` / `duplicateValues`

Predicate fires for cells whose value appears once in the array
(`uniqueValues`) or appears more than once (`duplicateValues`).

Per-cell output: `effective_font_color` / `effective_fill_color`
when applies.

Aggregate computation: count occurrences of each distinct value
once; per-cell `applies = (count == 1)` (or `> 1` for
duplicates).

## API shape — extending the per-cell carrier

The per-cell handoff introduces:

```rust
pub struct ArrayCellFormat {
    pub effective_display_text: String,
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,
}
```

This handoff extends it with optional visualization fields:

```rust
pub struct ArrayCellFormat {
    pub effective_display_text: String,
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,
    pub data_bar: Option<DataBarFill>,
    pub icon: Option<CfIcon>,
}
```

When no visualization rule fires for a cell, both new fields
are `None` and the host renders the cell with just its colour.

For a *single-cell* (scalar) result with a visualization rule
attached, the same carrier is populated with the array degenerated
to a 1×1 — the colour scale picks the midpoint, the data bar's
fill ratio is `1.0`, the icon set picks the middle bin. Excel's
own behaviour for one-cell selection.

## Behavioural notes

- **Aggregate computation runs once per evaluation.** OxFml
  computes the array's min / max / mean / stddev / sorted-order
  / unique-set up front, then iterates the per-cell pass against
  that pre-computed context.
- **Non-numeric cells inside an array** — for `colorScale` /
  `dataBar` / `aboveAverage` / `top`, non-numeric cells are
  excluded from the aggregate computation and rendered with no
  visualization. For `uniqueValues` / `duplicateValues`,
  non-numeric values are compared by their visible-value-text.
- **Mixing rules** — one formula can have multiple CF rules.
  Order matters: when both a `colorScale` and a `cell_value`
  rule fire on the same cell, Excel's rule-priority order
  decides (later-listed rules override). The per-cell carrier
  reflects the post-priority outcome.
- **Locale / now_serial** — same as the per-cell handoff: reuse
  the same `LocaleFormatContext` and `now_serial` for any
  rules that need them.

## Test coverage

Suggested in `oxfml_core` (a new
`conditional_formatting_aggregate_tests.rs` or sibling):

1. `colorScale` 2-stop (min red → max green) over `=SEQUENCE(5)`
   (values 1..5) → cell 1 fully red, cell 5 fully green, cell 3
   midpoint orange.
2. `dataBar` with implicit min/max over `[10, 20, 30, 40]` →
   per-cell `fill_ratio` = `[0.0, 0.333, 0.667, 1.0]`.
3. `iconSet` `3Arrows` over `=SEQUENCE(6)` → bottom two cells
   icon 0, middle two icon 1, top two icon 2.
4. `aboveAverage` over `[1, 2, 3, 4, 5]` (mean 3) → cells 4, 5
   apply.
5. `top` count-5 over `[1..10]` → cells 6..10 apply.
6. `uniqueValues` over `[1, 2, 1, 3]` → cells 1 and 3 of the
   array don't apply (both values 1 are duplicates), cell 2
   (value 2) and cell 4 (value 3) apply.
7. Single-cell scalar (`=42`) with a `colorScale` 2-stop rule →
   cell carries the midpoint colour (degenerate 1×1 case).
8. Mixed: `colorScale` + `cell_value > 5` (font_color red)
   ordered such that the cell-value rule wins on cells > 5 →
   per-cell carrier shows red font on those cells, scaled fill
   on the rest.

## DnaOneCalc-side state in the meantime

DnaOneCalc's CF panel exposes the visualization rule kinds in
the rule-kind dropdown with `<NOT IMPL>` SEAM badges pointing
here. The user can author a `colorScale` rule and the host
persists it; OxFml ignores the rule kind today (the
visualization computation isn't wired) so nothing visible
changes on the result hero.

When this handoff lands:

1. Bridge surface grows
   `data_bar: Option<DataBarFill>` and `icon: Option<CfIcon>`
   on the per-cell carrier.
2. Array browser renders bars / icons per cell from those
   fields.
3. Host removes the SEAM markers from the visualization rule
   kinds.

For scalar results, the host renders the visualization output
on the result hero in the same way (a single cell with a
proportional bar overlay or an icon glyph alongside the value).

## Closure conditions

- Aggregate computations land in OxFml for the seven rule
  families above.
- The per-cell carrier carries `data_bar` and `icon` fields.
- Tests above pass.
- DnaOneCalc removes `<NOT IMPL>` markers from the visualization
  rule kinds.

*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: CF Rules — Relative Dates, Blanks, Errors, and Rule-Kind Dispatch

Status: closed (OxFml landing acknowledged 2026-05-04 as W070)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Closed date: 2026-05-04
Related:
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-006_W070_CF_PREDICATE_RULES_LANDING.md`,
  `OxFml/crates/oxfml_core/src/publication/mod.rs::evaluate_conditional_formatting_rule`,
  `OxFml/crates/oxfml_core/tests/conditional_formatting_predicate_tests.rs`,
  `docs/HANDOFF_OXFML_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md` (W069 closed),
  `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md`

## Closure note (2026-05-04)

OxFml's W070 landing covers the rule-kind predicates this handoff
asked for:

- `blanks` / `noBlanks` — blank-value predicates.
- `errors` / `noErrors` — error-value predicates.
- `dates` — relative-date predicates (`today`, `yesterday`,
  `tomorrow`, `last7Days`, `thisWeek`, `lastWeek`, `nextWeek`,
  `thisMonth`, `lastMonth`, `nextMonth`).

Relative-date predicates use `TypedContextQueryBundle.now_serial`
when publication is produced by `SingleFormulaHost` — the path
DnaOneCalc's runtime takes — so the host's existing
`scenario_seeds` plumbing already feeds the right value. No
host-side bridge change was needed.

Unknown predicates and relative-date predicates without
`now_serial` continue to surface `applies: null`, which
DnaOneCalc treats as "rule did not fire" (matches Excel's
default).

DnaOneCalc removed the `SEAM-OXFML-CF-PREDICATES-AND-RELATIVE-DATES`
marker from `cf_seam_id_for_kind` for the now-live kinds, added
`noBlanks` and `noErrors` to the rule-kind dropdown, swapped the
threshold input for a relative-date dropdown when `rule_kind ==
"dates"`, and fixed the operator-dropdown values to use OxFml's
canonical names (`greaterThan`, `lessThan`, `equal`, …) — the
abbreviated names (`gt`, `lt`, `eq`) the host previously emitted
silently never matched OxFml's normalised dispatch table, so
operator-driven CF rules had been quietly no-ops upstream. Fixed
the same day.

## Summary

OxFml's `evaluate_conditional_formatting_rule` today handles the
operator-driven `cell_value` and `expression` rule kinds — full
comparison set (`gt` / `gte` / `lt` / `lte` / `eq` / `notequal` /
`between` / `notBetween`) plus the text predicates
(`containsText` / `notContainsText` / `beginsWith` / `endsWith`)
and AND / OR-combined expression formulas. Three Excel rule
families that DnaOneCalc's CF panel exposes are **not** evaluated
today and currently return `None` (treated as "rule did not
match" by the publication surface):

1. **Relative-date rules** (`dates` rule kind) — predicates like
   today / yesterday / tomorrow / last 7 days / last week / this
   week / next week / last month / this month / next month.
2. **Blanks** (`blanks` rule kind) — predicate fires when the
   value is blank / empty.
3. **Errors** (`errors` rule kind) — predicate fires when the
   value is an `EvalValue::Error(_)`.

These are operator-less Excel CF kinds: they fire based on the
*kind* alone, not on a threshold comparison. OxFml's current
dispatch in `evaluate_conditional_formatting_rule` only
dispatches off `rule.operator` and `rule.rule_kind == "expression"`
— anything that needs `rule_kind`-specific evaluation falls through.

DnaOneCalc has the UI surface for these rule kinds in place
(rule-kind dropdown in the formatting panel). They render with
`<NOT IMPL>` SEAM badges pointing here.

## What needs to change

`evaluate_conditional_formatting_rule` grows a third arm of
dispatch — **rule-kind-driven evaluation** — alongside the
existing operator and expression arms:

```rust
let applies = if let Some(operator) = rule.operator.as_deref() {
    evaluate_operator_rule(...)
} else if rule.rule_kind.eq_ignore_ascii_case("expression") {
    rule.thresholds.first().and_then(|formula| {
        evaluate_expression_rule(...)
    })
} else {
    // NEW: rule-kind predicate dispatch
    evaluate_predicate_rule(rule, value, locale_ctx, now_serial)
};
```

with `evaluate_predicate_rule` matching on `rule.rule_kind`
(case-insensitive) and `rule.thresholds` carrying any
sub-classification:

```rust
fn evaluate_predicate_rule(
    rule: &VerificationConditionalFormattingRule,
    value: &EvalValue,
    locale_ctx: Option<&LocaleFormatContext<'_>>,
    now_serial: Option<f64>,
) -> Option<bool> {
    match rule.rule_kind.to_ascii_lowercase().as_str() {
        "blanks"        => Some(is_blank(value)),
        "noblanks"      => Some(!is_blank(value)),
        "errors"        => Some(matches!(value, EvalValue::Error(_))),
        "noerrors"      => Some(!matches!(value, EvalValue::Error(_))),
        "dates"         => evaluate_dates_rule(rule, value, now_serial),
        _               => None,
    }
}
```

### `dates` — relative-date predicates

Excel's `dates` CF rule encodes a relative-date kind in
`thresholds[0]` (or in a parallel field — pick whichever fits
OxFml's existing serialisation). The canonical Excel kinds:

```
today
yesterday
tomorrow
last7Days
lastWeek
thisWeek
nextWeek
lastMonth
thisMonth
nextMonth
```

Each predicate compares the cell's date serial against a window
derived from the runtime's `now_serial`. OxFml already plumbs
`now_serial` through `TypedContextQueryBundle` to the runtime —
the same value should reach the CF evaluator (either by passing
it through the publication-surface call, or by stashing it on the
locale context for the duration of one render pass).

Date arithmetic notes (Excel-faithful):

- `today` — `floor(value) == floor(now_serial)`.
- `yesterday` — `floor(value) + 1 == floor(now_serial)`.
- `tomorrow` — `floor(value) - 1 == floor(now_serial)`.
- `last7Days` — `floor(now_serial) - 6 ≤ floor(value) ≤ floor(now_serial)`.
- `thisWeek` — Sunday-anchored week containing `now_serial`.
- `lastWeek`, `nextWeek` — adjacent Sunday-anchored weeks.
- `thisMonth` / `lastMonth` / `nextMonth` — calendar months
  derived from `ymd_from_excel_serial(now_serial)`.

### `blanks` / `noBlanks`

OxFml currently has no `EvalValue::Empty` (blank cell) variant
that I can see — blanks come through as text or empty-string in
the formula's value. Define `is_blank` to match:

- `EvalValue::Empty` if such a variant exists,
- `EvalValue::Text(t)` where `t.is_empty()`,
- otherwise `false`.

(If OxFml wants to surface a clean `EvalValue::Empty` as part of
this work, that's worthwhile but separable.)

### `errors` / `noErrors`

```rust
matches!(value, EvalValue::Error(_))
```

Trivially.

## DnaOneCalc-side UI implications

DnaOneCalc's CF rule-kind dropdown today lists ten options:
`cell_value`, `colorscale`, `databar`, `iconset`, `text`, `dates`,
`blanks`, `errors` — plus rank / average / unique aspirationally.
The single-cell result-hero has no "range" to compute color
scales / data bars / icon sets / rank / average / unique against,
so those entries are leaving the dropdown in a follow-on host
slice (they'll come back if and when DnaOneCalc grows a worksheet-
range CF surface).

The kinds that **stay** in the host UI and target this handoff:

- `cell_value` — operator-driven (already works).
- `text` — operator-driven, with the host adding `containsText`
  / `beginsWith` / `endsWith` to its operator dropdown (host
  work, no upstream dependency since OxFml already handles
  these).
- `dates` — needs this handoff.
- `blanks` — needs this handoff.
- `errors` — needs this handoff.
- `expression` — already works (host UI exposure of free-form
  formula entry is separate host-side work).

## Test coverage

In `oxfml_core` against `evaluate_conditional_formatting_rule`:

1. `rule_kind="blanks"`, value `EvalValue::Text("")` → `applies = Some(true)`.
2. `rule_kind="blanks"`, value `EvalValue::Number(0.0)` → `applies = Some(false)`.
3. `rule_kind="errors"`, value `EvalValue::Error(WorksheetErrorCode::DivZero)` → `applies = Some(true)`.
4. `rule_kind="errors"`, value `EvalValue::Number(0.0)` → `applies = Some(false)`.
5. `rule_kind="dates"`, threshold `["today"]`, `now_serial = Some(46045.5)` (a fractional time-of-day),
   value `EvalValue::Number(46045.0)` (the same date, midnight) → `applies = Some(true)`.
6. `rule_kind="dates"`, threshold `["yesterday"]`, `now_serial = Some(46045.5)`,
   value `EvalValue::Number(46044.0)` → `applies = Some(true)`.
7. `rule_kind="dates"`, threshold `["last7Days"]`, value within 6 days back from now → `applies = Some(true)`.
8. `rule_kind` unknown → `applies = None` (current behaviour, regression-pinned).

## Closure conditions

- `evaluate_conditional_formatting_rule` dispatches on
  `rule_kind` for blanks / noBlanks / errors / noErrors / dates
  (in addition to operator and expression).
- `now_serial` reaches the predicate evaluator (same value the
  runtime already receives via `TypedContextQueryBundle`).
- Tests above pass.
- DnaOneCalc removes the `SEAM-OXFUNC-CF-DATES` /
  `SEAM-OXFUNC-CF-BLANKS` / `SEAM-OXFUNC-CF-ERRORS` markers
  from `services::home_shell_view_model::cf_seam_id_for_kind`
  the same week the OxFml change lands.

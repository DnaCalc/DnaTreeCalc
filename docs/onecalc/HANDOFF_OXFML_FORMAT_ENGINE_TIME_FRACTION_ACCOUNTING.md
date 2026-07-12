*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Format Engine Coverage — Time, Datetime, Fraction, Accounting

Status: closed (OxFml landing acknowledged 2026-05-04 as W069)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting expansion
Filed date: 2026-05-04
Closed date: 2026-05-04
Related:
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-005_W069_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md`,
  `OxFml/crates/oxfml_core/src/format/{datetime.rs,number.rs,engine.rs}`,
  `OxFml/crates/oxfml_core/tests/format_time_fraction_accounting_tests.rs`,
  `OxFml/crates/oxfml_core/tests/ftc_0654_fraction_format_engine_tests.rs`

## Closure note (2026-05-04)

OxFml's W069 landing covers the bounded format families this
handoff asked for: time tokens (`h`/`hh`/`m`/`mm`/`s`/`ss`),
`AM/PM` 12-hour rendering, datetime composites, elapsed-time
tokens (`[h]`/`[m]`/`[s]`), simple fractions (`?/?`, `??/??`,
`# ?/?`, `# ??/??`, `0/0`), and common accounting parentheses
patterns. The publication surface honours user-supplied codes
ahead of the presentation-hint fallback.

The remaining custom-format grammar (multi-section colour
tokens, locale prefix, text section) is not part of W069's
bounded scope and now lives under
`docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md`.

DnaOneCalc removed the `SEAM-OXFML-FORMAT-ENGINE-TIME-FRACTION-ACCOUNTING`
markers from the Time / Time(12h) / Datetime / Fraction /
Accounting preset chips the same day.

## Symptom seen from DnaOneCalc

User typed `=NOW()`, then opened the formatting panel and entered the
custom number-format code `HH:mm:ss`. The result hero rendered
**`2025-12-09`** instead of a wall-clock time. Reproduces against the
en-US locale on the 1900 calendar with the host passing `HH:mm:ss`
verbatim through `VerificationPublicationContext.number_format_code`.

The render is wrong by two layers:

1. `render_with_code(profile, date_system, value, "HH:mm:ss")` returns
   `Err(FormatFailure::UnsupportedCode(...))` — the format engine has
   no time-token path.
2. The publication-surface fallback in `render_effective_display_text`
   then drops to `NOW`'s `PresentationHint::DateLike` default
   (`"yyyy-mm-dd"`), which is the date string the user actually saw.
   The fallback masking makes the gap easy to miss in casual testing
   (a wrong format code looks like a working "default" instead of an
   `Err`).

## Diagnosis inside `oxfml_core`

`crates/oxfml_core/src/format/datetime.rs` line 44:

```rust
fn contains_unsupported_time_tokens(section: &str) -> bool {
    let lower = section.to_ascii_lowercase();
    lower.contains("am/pm") || lower.contains('h') || lower.contains(':')
}
```

This is consulted both from `looks_like_date_format` (which then
returns `false` for any code carrying time tokens) and from
`render_with_date_tokens` (which short-circuits to `None` before
reaching the substitution table).

`crates/oxfml_core/src/format/number.rs::render_with_number_format_code`
then walks through:

1. `datetime::looks_like_date_format` → false (because of the time
   token).
2. `is_two_digit_integer_code` → false.
3. `contains_fraction_placeholder_pattern` → false.
4. `parse_numeric_section` → `None` (no `#`/`0`/`?` placeholders in
   `HH:mm:ss`).
5. Returns `Err(FormatFailure::UnsupportedCode("HH:mm:ss"))`.

Net: `OXFML_FORMAT_CODE_ENGINE.render_with_code` cannot render any
of the standard Excel time / datetime codes:

```
h:mm                hh:mm                hh:mm:ss
HH:mm               HH:mm:ss             h:mm AM/PM
m/d/yyyy h:mm       yyyy-mm-dd hh:mm:ss  [h]:mm:ss
```

Same gap also exists for **fraction** (`# ?/?`, `# ??/??`, `0/0`)
and **accounting** (`($#,##0.00);($#,##0.00)` parens-on-negative
patterns) — both are commonly authored Excel formats; both are
listed in the existing six-family code-picker plan
(`docs/APP_UX_REVIEW_GAP_MATRIX.md`).

## What the host needs from OxFml

A `render_with_code` that handles, against the same en-US locale
profile:

1. **Time tokens.** `h` (1-23), `hh` (00-23), `m` (1-59 in time
   context — disambiguated by adjacency to `h`/`s`), `mm` (00-59 in
   time context), `s` (0-59), `ss` (00-59), `AM/PM` and `am/pm`
   (12-hour rendering with 12-hour mod), `[h]` / `[m]` / `[s]`
   (elapsed forms). The disambiguation rule that distinguishes
   minute-`m` from month-`m` is "if the `m` group is adjacent to an
   `h` or `s` group, it's minutes." Excel's parser is the spec.
2. **Datetime composites.** A single section containing both date
   tokens and time tokens (e.g. `yyyy-mm-dd hh:mm:ss`) renders both
   parts together. The current `looks_like_date_format` shortcut
   needs to grow into "section contains date *or* time tokens" with
   one shared renderer.
3. **Fraction.** `?/?`, `??/??`, `# ?/?`, `# ??/??`, `0/0`. Today
   `contains_fraction_placeholder_pattern` exists but the matched
   path returns `Err(UnsupportedCode)` — the placeholder detection
   was wired in anticipation of a renderer that wasn't built.
4. **Accounting.** Currency display where negatives render in
   parentheses with the symbol aligned (`($1,234.50)` rather than
   `-$1,234.50`). The numeric-section parser already supports
   per-section format strings; what's missing is the canonical
   accounting code recognised as "Accounting" rather than just
   "Currency with parens".

For each, the upstream test suite should grow tests against the
en-US profile that pin:

- Each token's output for a known serial (e.g. `h:mm` against
  `0.625` → `"15:00"`).
- 12-hour mode with AM/PM correctly switching at noon and midnight.
- Datetime composite rendering both parts in one pass.
- Negative-zero, NaN, and very-large serials don't panic.
- Accounting alignment matches the published Excel reference for
  the canonical accounting codes.

## Why this matters more than it looks

Two compounding effects make the current behaviour confusing for
end users:

1. The fallback masks the error. A user who types a perfectly
   reasonable `HH:mm:ss` doesn't see a "format unsupported" message
   — they see the default DateLike rendering. The format engine is
   silently rejecting their input.
2. `=NOW()`'s presentation hint drives the default, which means
   *every* `=NOW()` in the absence of an explicit format code looks
   like a date-only function. The user assumes NOW is broken
   ("where's the time?"); the actual gap is in the format engine,
   not in NOW.

DnaOneCalc is going to add an autoformat-from-presentation-hint
path that picks `"yyyy-mm-dd HH:mm:ss"` for `DateLike` so a fresh
`=NOW()` shows date and time. That code will start *working as
soon as OxFml grows time-token support*; until then, every NOW /
TODAY result looks date-only.

## Closure conditions

- `oxfml_core::format::engine::render_with_code` returns `Ok` for
  every Excel-canonical time, datetime, fraction, and accounting
  format code.
- `oxfml_core::format::datetime` (or its successor module) renders
  date+time composites in one pass.
- The fallback in `oxfml_core::publication::render_effective_display_text`
  no longer masks rejected codes for the day-to-day formats above
  — when the user supplies a recognised time code, that code is
  what renders.
- Tests pin each token, the AM/PM transition, datetime composites,
  fraction renderings, and accounting alignment against the
  published Excel reference.

## DnaOneCalc-side state in the meantime

DnaOneCalc will:

- Surface the OxFunc presentation hint (currently dropped on the
  floor) so `=NOW()` / `=TODAY()` produce correctly-formatted
  defaults *as soon as the OxFml engine supports the corresponding
  codes*. This work is host-side and proceeds independently.
- Mark Time / Datetime / Fraction / Accounting families in the
  format-picker UI with explicit `<NOT IMPLEMENTED>` SEAM markers
  citing this handoff, so users see the gap explicitly rather than
  through a silent fallback.
- Once OxFml lands the engine work, the host removes the SEAM
  markers and the families become live without a rebuild — they're
  just custom-code presets all the way down.

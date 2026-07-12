*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Full Custom-Format Grammar — Sections, Colour Tokens, Locale Prefix, Text

Status: triaged (split across three lanes — see "Triage" below)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Triage acknowledged: 2026-05-04 in
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-007_W070_LOCALE_AND_CUSTOM_FORMAT_TRIAGE.md`
Related:
  `OxFml/crates/oxfml_core/src/format/number.rs` (multi-section parser today),
  `docs/HANDOFF_OXFML_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md` (W069 landed),
  `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md` (locale tables)

## Triage outcome (2026-05-04, OxFml W070)

OxFml split the four-piece request into three lanes:

1. **Already covered** by W069's bounded engine — multi-section
   conditional selection (`[>=N]section1;[<0]section2;default`)
   is in place. The handoff item still standing is *test
   coverage* in OxFml's own suite (not new code).
2. **OxFml-local follow-up** (no upstream blocker; OxFml will
   land in a future slice):
   - **Text fourth-section behaviour** — selecting the 4th
     section for `EvalValue::Text(_)` and rendering `@` as the
     verbatim placeholder.
   - **Surfacing selected format-section colour information**
     onto `VerificationPublicationSurface` so the host can
     render the user-authored colour token (`[Red]` /
     `[Color3]` / etc.) on the result hero.
3. **Locale-blocked follow-up** — locale-prefix grammar
   (`[$-040C]…`) and locale-specific month / day rendering
   inside the prefix's effective locale. Blocked on
   `BLK-FML-005` (the same OxFunc dependency as the
   locale-expansion handoff).

DnaOneCalc keeps no SEAM marker for these grammar pieces today
because the host doesn't surface them in the formatting panel —
the user can type them by hand into the custom number-format
input and OxFml renders what it understands. The visible-on-
result-hero gap is just the colour-token one (custom codes like
`[Red]#,##0` render numerically correct but in default ink).
That gap closes when lane (2) lands; until then the host accepts
the codes verbatim and the colour silently drops on the floor.

## Summary

W069 unblocked the day-to-day format families (time, datetime,
fraction, common accounting parens). It explicitly excluded "full
Excel custom-format grammar, text sections, and UI-specific
accounting alignment". This handoff lines up that remaining
custom-format work so the formatting topic between OneCalc and
OxFml can close.

The four pieces below are the parts of Excel's number-format
grammar a power user *will* type, that don't yet flow through
OxFml's `render_with_code`:

1. **Multi-section formats with conditional headers** —
   `[>0]"+"#,##0;[<0]"-"#,##0;0` selects a section per condition.
2. **Colour tokens** — `[Red]#,##0.00;[Blue](#,##0.00)` colours
   the rendered text. Excel-supported colour names: Black, Blue,
   Cyan, Green, Magenta, Red, White, Yellow, plus `[Color1]`
   through `[Color56]` for the indexed palette. (Today OxFml
   strips colour tokens during section-selection per
   `strip_condition_and_color_tokens`; what's missing is
   surfacing the *applied colour* on the publication surface so
   the host can render it.)
4. **Locale prefix** — `[$-040C]dddd, d mmmm yyyy` overrides the
   active locale for *that code*. Cross-references
   `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md` (the prefix is dead
   without locale tables to switch to).
5. **Text section** — the 4th `;`-separated section
   (`positive;negative;zero;text`) renders a non-numeric value
   like `EvalValue::Text(t)`. Today OxFml's
   `select_number_format_section` picks one of the first three
   numeric sections per the value's sign; a text section is
   ignored.

Each piece is small individually. Together they cover what an
Excel power user expects when they paste a custom format code
across.

## Symptom from DnaOneCalc

The user pastes the format code
`[Green][>=1000]#,##0;[Red][<0](#,##0);0` into the formatting
panel, expecting "green for ≥1000, red parenthesised for negative,
plain otherwise". W069 makes the *numeric* rendering correct on
each branch, but:

- The colour tokens are stripped during section selection and
  never surfaced to the publication surface, so the result hero
  renders all values in the default ink colour.
- The conditional `[>=1000]` / `[<0]` headers select sections
  correctly today (they're handled inside
  `select_number_format_section`); confirm and pin.

For locale-prefixed codes:

- `[$-0407]dddd, d. mmmm yyyy` (German prefix) renders English
  weekday / month names because the locale-prefix is parsed but
  the underlying locale tables don't exist (see locale-expansion
  handoff).

For text sections:

- `0.00;-0.00;"-";@` against `EvalValue::Text("hello")` renders
  empty (no numeric placeholder matches a Text value), instead of
  rendering `hello` per the trailing `@` text section.

## What needs to change

### 1. Multi-section conditional selection

Confirm `select_number_format_section` already handles
`[>=N]` / `[<=N]` / `[=N]` / `[<N]` / `[>N]` / `[<>N]` headers in
the first two sections (Excel allows two of them; the third is
the default). If gaps remain, close them. If complete, the only
visible part is to pin them with tests against the published
behaviour.

### 2. Colour tokens — surface the applied colour

Today `strip_condition_and_color_tokens` removes `[Red]` /
`[Color3]` / etc. before rendering. The information is dropped on
the floor — the host receives no signal that the value should
render in a particular colour.

Add a return shape on the rendering path that captures
`(rendered_text, applied_color: Option<Color>)`. Surface that on
`VerificationPublicationSurface` so the host can read it
alongside `effective_display_text` and apply the colour to the
result hero. Sketch:

```rust
pub struct AppliedFormatStyle {
    pub font_color: Option<String>, // hex "#RRGGBB"
}

pub struct VerificationPublicationSurface {
    // ... existing fields
    pub effective_display_text: String,
    pub applied_format_style: AppliedFormatStyle,
}
```

The host's `render_effective_display_summary` reads
`applied_format_style.font_color` and applies it via the existing
formatting-state pipeline (which already knows how to render font
/ fill colour).

Excel's colour-token grammar:

- Named: `[Black]`, `[Blue]`, `[Cyan]`, `[Green]`, `[Magenta]`,
  `[Red]`, `[White]`, `[Yellow]`.
- Indexed: `[Color1]` through `[Color56]` (legacy palette indexes;
  resolve via the workbook's palette or a stable default table —
  Excel ships its own).
- Token position: any of `[Color]` and `[<condition>]` may appear
  in either order at the *start* of the section, before the
  numeric body.

### 3. Locale prefix `[$-040C]` and friends

Excel's locale prefix is a 4-hex code at the start of the format
code. Examples: `[$-0409]` (en-US), `[$-040C]` (fr-FR),
`[$-0407]` (de-DE), `[$-0411]` (ja-JP). Excel resolves the prefix
to a `LocaleProfileId` and renders the rest of the code under
that profile.

Implementation depends on
`docs/HANDOFF_OXFML_LOCALE_EXPANSION.md` — the prefix wires the
existing parsing path to the new tables. The hex → profile-id map
is Excel's published `LCID` table.

### 4. Text section

Excel's number format has up to four sections separated by `;`:

```
positive_section ; negative_section ; zero_section ; text_section
```

When the value is a text (non-numeric, non-error), Excel selects
the **fourth** section. Inside that section, `@` is the placeholder
that emits the input text verbatim, and any other tokens render
as literal characters around it.

Today `select_number_format_section` only chooses among the first
three numeric sections. Grow it to:

- Detect text values (`EvalValue::Text(_)`).
- Select the 4th section if it exists.
- If the 4th section is absent, fall back to General-rendering
  the text (i.e., emit it verbatim).
- Inside the section, replace `@` with the text and emit the
  surrounding literals.

## Test coverage

In `oxfml_core::format::number::tests` (or a new
`format_custom_grammar_tests.rs`):

1. `[Red]#,##0;[Blue]#,##0` against `42` →
   `(rendered = "42", color = Some("#FF0000"))`.
2. Same against `-42` → `(rendered = "-42", color = Some("#0000FF"))`.
3. `[Color3]0.00` against `1.5` → `color = Some(<colour 3 hex>)`.
4. `[>=1000]#,##0;[<0](#,##0);0` against `5000` selects the
   first section and renders `5,000`.
5. `[$-0407]dddd, d. mmmm yyyy` (depends on locale-expansion
   handoff) against a known Sunday-serial renders German output
   regardless of the active locale.
6. `0.00;-0.00;"-";@` against `EvalValue::Text("hello")` renders
   `hello`.
7. `0.00;-0.00;"-";"prefix-"@"-suffix"` against
   `EvalValue::Text("x")` renders `prefix-x-suffix`.
8. Three-section format `0.00;-0.00;"-"` against
   `EvalValue::Text("hello")` falls back to a verbatim `hello`
   (no fourth section, text fall-through).

## Scope clarifications

This handoff explicitly **does not** cover:

- The visualization-CF kinds (color scales, data bars, icon sets,
  rank, average, unique). Those are worksheet-range concepts and
  are leaving the DnaOneCalc CF UI in this slice; if and when
  DnaOneCalc grows a worksheet-range CF surface they come back
  with their own handoff.
- Negative-format alignment for accounting. W069 covered the
  parens patterns; pixel-perfect column alignment is OOXML
  worksheet rendering, not single-cell display.
- Asterisk-fill (`_($* #,##0.00_)`) repeat-character semantics.
  W069 covered the parens; the `*` repeat is a different feature.
  Add as a follow-up if it's needed.

## Closure conditions

- Multi-section conditional selection covers
  `[>=N]` / `[<=N]` / `[<>N]` / `[=N]` / `[>N]` / `[<N]` headers.
- Colour tokens are recognised and the applied colour flows out
  via the publication surface for the host to apply.
- Locale prefix `[$-XXXX]` resolves to a `LocaleProfileId` and
  switches the rendering profile (depends on
  `HANDOFF_OXFML_LOCALE_EXPANSION.md`).
- Text section (`@` placeholder) renders text values per the
  4th section.
- Tests above (or OxFml-side equivalents) are green.
- DnaOneCalc removes any custom-format-grammar SEAM markers and
  documents the closeout under a final formatting-topic-closed
  note in `docs/`.

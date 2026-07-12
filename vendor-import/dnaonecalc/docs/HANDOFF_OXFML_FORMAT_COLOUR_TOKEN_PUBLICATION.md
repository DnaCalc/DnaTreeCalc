*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Surface Custom-Format Colour Tokens on the Publication Surface

Status: filed
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Related:
  `OxFml/crates/oxfml_core/src/format/number.rs::strip_condition_and_color_tokens`,
  `OxFml/crates/oxfml_core/src/publication/mod.rs::VerificationPublicationSurface`,
  `docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md` (parent — three-lane triage; this is lane 2's colour-token piece broken out as a focused request)

## Summary

Excel custom format codes can prefix any `;`-separated section
with a colour token: `[Red]`, `[Blue]`, `[Color3]`, etc. Common
patterns:

```
[Red]#,##0.00;[Blue](#,##0.00)
[Green][>=1000]#,##0;[Red][<0](#,##0);0
[Color3]0.00
```

The selected section's colour applies to the rendered text. After
W069 OxFml renders the *numeric body* of these codes correctly,
but the colour token itself is **dropped on the floor**: the
strip step at `oxfml_core::format::number::strip_condition_and_color_tokens`
removes it during section selection and no downstream surface
captures it.

Result: a user typing `[Red]#,##0` into DnaOneCalc's formatting
panel sees the right number formatting but not the red colour.

## What needs to change

`build_verification_publication_surface` learns to capture the
colour token of the selected section and surface it on
`VerificationPublicationSurface` in a position consumers can
read.

The natural home for it is **the existing
`effective_font_color: Option<String>` field** — same shape, same
consumers, same downstream code path. The order of precedence
is: CF rule's `font_color` (already populated) wins, then the
section's colour token (this handoff), then the workspace
default (None).

```rust
let effective_font_color =
    // existing: CF rule override
    conditional_formatting_rules
        .iter()
        .rev()
        .find(|rule| rule.applies == Some(true))
        .and_then(|rule| rule.effective_font_color.clone())
    // new: format-section colour token
    .or_else(|| selected_section_colour_token(format_code, value))
    // existing: nothing → None
    ;
```

`selected_section_colour_token` runs the same section-selection
logic the format engine already uses, captures any `[Color]` or
`[<Named>]` token at the start of the selected section, and
resolves it to a hex `#RRGGBB`.

### Excel colour-token grammar to support

- **Named** (case-insensitive): `[Black]`, `[Blue]`, `[Cyan]`,
  `[Green]`, `[Magenta]`, `[Red]`, `[White]`, `[Yellow]`. Each
  resolves to a fixed hex (Excel's published table — the standard
  8-colour palette).
- **Indexed**: `[Color1]` through `[Color56]` map to Excel's
  legacy indexed palette. If the workbook has a custom palette
  it overrides; absent a custom palette, ship a fixed default
  table (also Excel-published).
- **Position rule**: a colour token may appear at the *start* of
  a section, optionally combined with a condition token like
  `[>=1000]`. Tokens may appear in either order: `[Red][>=1000]…`
  and `[>=1000][Red]…` are equivalent.

### Section-selection interaction

For a multi-section format like `[Red]#,##0;[Blue](#,##0);0`:

- Positive value → first section selected → font colour `Red`.
- Negative value → second section selected → font colour `Blue`.
- Zero → third section selected → no colour (no token).

The same section-selection function the format engine already
uses to pick the rendering body should yield the `(body, colour)`
pair so the surface can read both.

## Test coverage

In `oxfml_core` (publication or format integration suite):

1. `[Red]#,##0` against `42` → `effective_display_text == "42"`,
   `effective_font_color == Some("#FF0000")`.
2. `[Red]#,##0;[Blue]#,##0` against `-42` →
   `effective_display_text == "-42"`,
   `effective_font_color == Some("#0000FF")`.
3. `[Color3]0.00` against `1.5` →
   `effective_font_color == Some(<colour-3 hex from Excel's
   palette>)`.
4. `[Green][>=1000]#,##0;0` against `5000` →
   first section selected, `effective_font_color ==
   Some("#00FF00")`.
5. CF rule precedence: a `cell_value > 0` rule with
   `font_color = "#000000"` against a value that fires the rule,
   format `[Red]#,##0` → `effective_font_color == Some("#000000")`
   (CF wins over colour token).
6. No colour token in selected section →
   `effective_font_color` retains its existing behaviour
   (None unless a CF rule fired).

## DnaOneCalc-side state

The host already reads
`verification_publication_surface.effective_font_color` and
`effective_fill_color` and applies them to the result hero
(landed alongside this handoff for the CF-rule path). The moment
OxFml routes colour tokens through the same fields, they render
without any host-side change — this is a transparent upstream
upgrade.

## Out of scope for this handoff

- **Fill colour** for format sections. Excel custom formats can
  encode background fill via the workbook style id, not directly
  in the format code grammar. Cell-level fill from format codes
  isn't a pattern Excel exposes through `[Color]` tokens — only
  font colour is. This handoff stays font-only.
- **Locale-prefix grammar** (`[$-040C]…`) — different lane in the
  parent triage; depends on `BLK-FML-005`.
- **Text fourth section** — different lane in the parent triage;
  no upstream blocker but separable.
- **Per-cell colour-token expansion for arrays** — once
  `docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md` lands the per-cell
  surface, colour tokens flow through the same per-cell carrier.

## Closure conditions

- `VerificationPublicationSurface.effective_font_color` carries
  the resolved colour for any selected format section that
  begins with a `[Color]` / `[Named]` token.
- CF rule `font_color` continues to win when both a CF rule and
  a colour token would apply.
- Tests above pass.
- The custom-format-grammar parent handoff
  (`docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md`) marks this
  lane (2a) closed.

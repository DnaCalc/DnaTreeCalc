# HANDOVER_OXFML_constant_input

Status: Open
Target: OxFml
Ask: Make the §2.1A cell-entry classification reachable from a TreeCalc host channel — leading `=` = formula, `'` = forced text, otherwise a typed constant — with the formula branch parsing tree-path references under `treecalc-v1` instead of `WorksheetA1`.
Context: DNA TreeCalc normalizes node content to a single `formula` field. An entry without a leading `=` is a literal constant that the engine must parse and resolve to a typed `EvalValue` during bind — not via a host-side fallback evaluator.
Evidence: OxFml `docs/spec/OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md` §2.1A ("WorksheetA1 Cell-Entry Classification", rules 1–6); DnaTreeCalc `docs/model/CORE_MODEL_SPEC.md` §2, §5, §6 (item 11).

## Background

TreeCalc has dropped the "formula vs. literal value" duality from its model. A node now carries one
content string. We adopt Excel's cell-entry convention to decide what that string means, which is
exactly what OxFml already specifies for the single-cell host path:

> §2.1A — entries whose first char is `=` are formulas; first char `'` forces text (escape, not part
> of the value); unprefixed finite-number entries are number literals; `TRUE`/`FALSE` (case-insensitive)
> are logical literals; quoted string literals decode through the string path; every other unprefixed
> entry is a text literal preserving the entered text exactly. "This classification is OxFml-owned …
> so DNA OneCalc does not need a host-side fallback evaluator for ordinary literal cell entries."

TreeCalc wants the same guarantee, so that `123.4`, `TRUE`, `'007`, and `=A.B+1` are all just `formula`
strings and the engine resolves type + value with no host-side constant parser.

## What TreeCalc needs

1. **Channel reach.** §2.1A is defined for `FormulaChannelKind::WorksheetA1`. TreeCalc references are
   tree paths, not A1. Either (a) make the entry-classification step channel-agnostic — classify the
   entry text first, then dispatch the *formula* branch to the active channel's reference grammar — or
   (b) expose a tree-reference channel that reuses the same classification with the `treecalc-v1`
   grammar on the formula branch. Please confirm which, and the channel id TreeCalc should pass.
2. **Discriminator + escapes unchanged.** Rules 1–6 above are adopted verbatim; TreeCalc adds nothing
   to the constant branch.
3. **Excel-aligned implicit number-format inference.** A number constant entered as `5%`, `$5`,
   `12/31/2025`, or `1,000` should resolve a value *and* an Excel-aligned implicit number format, on
   the same basis OxFml/OxFunc already handle Excel value entry. Confirm scope (value-only vs.
   value+format) and where the implicit format surfaces.
4. **Profile interaction.** The constant branch is profile-agnostic (a constant is a constant under
   both `strict-excel` and `treecalc-v1`); only the formula branch's reference grammar is profile-gated.

## Expected disposition

Likely **confirm**, not **add**: §2.1A already owns the classification and removes the host fallback
for OneCalc. The TreeCalc ask is to make that same behavior available on the tree-reference channel.
If it is already channel-agnostic, the response is a confirmation plus the channel id and a note on
the implicit-number-format scope (item 3).

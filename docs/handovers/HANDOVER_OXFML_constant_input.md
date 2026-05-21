# HANDOVER_OXFML_constant_input

Status: Open
Target: OxFml
Ask: Make the §2.1A cell-entry classification reachable from a TreeCalc host channel — empty string = `Empty`, leading `=` = formula, `'` = forced text, otherwise a typed constant — with the formula branch parsing tree-path references under `treecalc-v1` instead of `WorksheetA1`.
Context: DNA TreeCalc normalizes node content to a single `formula` field. The empty string is the node-level `Empty` value; a non-empty entry without a leading `=` is a literal constant that the engine must parse and resolve to a typed `EvalValue` during bind — not via a host-side fallback evaluator.
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

TreeCalc wants the same guarantee, so that `""`, `123.4`, `TRUE`, `'007`, and `=A.B+1` are all just
`formula` strings and the engine resolves type + value with no host-side constant parser. The empty
string is the one TreeCalc-specific addition to the classification: it produces the node's `Empty`
value. A formula can produce `""` as a text value, but cannot produce top-level `Empty`.

## What TreeCalc needs

1. **Channel reach.** §2.1A is defined for `FormulaChannelKind::WorksheetA1`. TreeCalc references are
   tree paths, not A1. Either (a) make the entry-classification step channel-agnostic — classify the
   entry text first, then dispatch the *formula* branch to the active channel's reference grammar — or
   (b) expose a tree-reference channel that reuses the same classification with the `treecalc-v1`
   grammar on the formula branch. Please confirm which, and the channel id TreeCalc should pass.
2. **Discriminator + escapes unchanged for non-empty entries.** Rules 1–6 above are adopted for all
   non-empty entries; TreeCalc adds only the explicit empty-string branch before those rules.
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

## TreeCalc W002 integration note (2026-05-21)

TreeCalc now has a local Rust host skeleton and a first OxCalc bridge smoke path:

- local crate: `DnaTreeCalc/src/dnatreecalc-host`;
- bridge boundary: `adapters::oxcalc::LiveOxCalcTreeBridge`;
- local smoke fixture: `Root.A = 2`, prepared `Root.B = A + 3`, submitted through OxCalc's
  `OxCalcTreeRuntimeFacade`;
- local quarantine: the smoke path uses a TreeCalc-local `PreparedFormulaCatalog` to carry the
  engine-ready expression. It deliberately does **not** claim that TreeCalc formula text is bound yet.

That means the remaining OxFml unblocker is precise:

1. **Entry classification API.** TreeCalc needs the same Excel-style entry classifier used for
   `WorksheetA1`, callable for a TreeCalc formula channel. The host can handle the empty-string
   `Empty` branch locally, but every non-empty constant/formula classification should be OxFml-owned.
2. **Tree formula channel id.** Please confirm the channel id TreeCalc should pass for formula entries
   whose reference grammar is `treecalc-v1`, and whether this is an existing `FormulaChannelKind` or
   a new one.
3. **Bind artifact into OxCalc.** Please confirm the output TreeCalc should hand to OxCalc for
   formula entries: direct `TreeFormula` / reference carriers, a bind packet that OxCalc lowers into
   `TreeFormulaCatalog`, or another consumer-facing object. The local `PreparedFormulaCatalog` is only
   a temporary smoke-test carrier.
4. **Minimum W002 activation.** TreeCalc will keep `docs/test-corpus/constants/entry-classification.json`
   pending until this response identifies the API and value/format result surface. Once answered, the
   first active slice will be ordinary constants plus the leading-`=` formula discriminator.

Sibling-review checklist:

- confirm whether §2.1A is already channel-agnostic internally;
- name the TreeCalc channel id / enum variant;
- name the constant-classification result type and implicit-format fields;
- name the formula-bind output TreeCalc should pass toward OxCalc;
- identify any OxFml workset/bead needed before TreeCalc should remove the temporary prepared-formula
  carrier from the W002 smoke path.

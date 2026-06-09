# HANDOVER_OXFML_paste_special_authoring

Status: Open
Target: OxFml
Ask: Add OxFml-owned paste-special authoring operations for computed value literalization and formula rebind so DnaTreeCalc can complete W3 `paste-special` without converting displayed values or rewriting formula text in the host.
Context: DNA TreeCalc has landed the ownership-safe host slice for constant-source value paste: a value clipboard carrier records the source content kind plus authored constant input text, and `PasteClipboardValues` routes that exact input through the OxCalc-backed scoped content transaction path. The remaining paste-special modes need engine-owned semantics. OxFml owns formula text composition, grammar legality, value literal syntax, profile gating, and reference-binding rewrite rules; DnaTreeCalc must not synthesize formula text from rendered values or mutate references by string manipulation.
Evidence: `docs/ux/stack-requirements/ROADMAP.md` W3 includes `paste-special` after the W3 formula authoring verbs, and `docs/ux/stack-requirements/ENGINE_REQUIREMENTS.md` describes paste values/formula/format with formula paste reusing `replicate-by-id` rebind machinery. Current DnaTreeCalc commit `f6da5f6` implements only constant-source `PasteClipboardValues`; current OxFml public consumer/editor search still finds authored-input/dry-bind and editor completion/application surfaces, but no computed-value-to-authored-input literalization API or handle/id-based formula rebind API.

Update 2026-06-09: The first computed-value literalization slice is now implemented for scalar cell
values. OxFml exposes scalar `CalcValue` to authored-input literalization for blank, finite number,
text, logical, and worksheet-error values, with typed unsupported verdicts for arrays, references,
missing/non-finite values, and rich/callable values. DnaTreeCalc projects that authored input through
`NodeView.literalized_value_input` and `PasteClipboardValues` consumes it through the existing
scoped content transaction path without using rendered display text. This handoff remains open for
array literalization policy, formula rebind/paste, formula-and-format paste, subtree internal
reference rebind, and formula/subtree cut source deletion semantics.

## Required Shape

The DnaTreeCalc host needs an OxFml API family along these lines:

| Operation | Host supplies | OxFml returns |
|---|---|---|
| Literalize computed value for paste | typed `CalcValue`/array value, target capability/profile context, locale/format policy where relevant | Scalar cell values are partially satisfied by OxFml-authored input literalization. Array literalization policy, target profile/locale policy, and richer diagnostics remain open. |
| Rebind formula paste | source formula text or formula document, source caller context, target caller context(s), reference-resolution/bind handles where available, profile context | one recomposed formula text per target, rewrite provenance, dry-bind diagnostics/profile violations |
| Formula-and-format paste | same formula rebind plus format-code validation inputs | recomposed formula text plus format validation verdicts; OxCalc/DnaTreeCalc still store/apply host-owned format metadata |
| Subtree internal-reference rebind support | source formula text per cloned node, source-to-clone node mapping, external-reference preservation policy, target caller contexts, profile context | recomposed formula text per cloned node, diagnostics, and stable provenance for which references rebound internally versus stayed external |

## Boundary

- OxFml owns value literal syntax, array literalization policy, formula text composition, reference binding rewrite, grammar/profile diagnostics, and parse/bind legality.
- OxCalc owns applying the resulting node edits in transactions, rebind/dependency descriptors, invalidation, scheduling, publication, and runtime reference-provider descriptors.
- DnaTreeCalc host owns clipboard carrier storage, authoring scope expansion, edit-buffer/caret state, structural clone orchestration, and dispatch receipts.
- Skins render and dispatch only.

Until the remaining APIs exist, DnaTreeCalc should keep array paste, formula paste, formula-and-format paste, duplicate-subtree formula rebind, and cut source deletion that depends on a successful rebind out of the supported Skin IR surface. Constant-source value paste remains supported because it uses authored source input text, and scalar computed-value paste is supported only through OxFml-authored value literalization, not rendered computed value text.

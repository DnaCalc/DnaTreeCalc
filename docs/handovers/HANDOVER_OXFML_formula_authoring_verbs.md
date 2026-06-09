# HANDOVER_OXFML_formula_authoring_verbs

Status: Open
Target: OxFml
Ask: Add OxFml-owned formula authoring operations for W3 TreeCalc skin intents: replicate/fill by id, F4 binding toggle, and point-mode reference insertion by handle.
Context: DNA TreeCalc W3 requires reference/content authoring verbs that carry ids, handles, scopes, and caller context. Skins must never synthesize formula text, and the DnaTreeCalc host must not implement formula rewrite semantics with string manipulation. OxFml owns formula parse, bind, reference text composition, absolute/relative binding syntax, and profile gating; OxCalc owns the rebind/dependency/scheduling consequence after the recomposed formula text is produced.
Evidence: `docs/ux/stack-requirements/ROADMAP.md` W3 lists `replicate-by-id`, `f4-toggle-binding`, and `reference-insertion` before broader paste/duplicate work. Current OxFml public editor facade (`crates/oxfml_core/src/consumer/editor`) exposes immutable edit/apply/completion flows over caller-provided formula text and completion insert text, but not handle/id-based formula rewriting operations for these verbs.

## Required Shape

The DnaTreeCalc host needs an OxFml API family along these lines:

| Operation | Host supplies | OxFml returns |
|---|---|---|
| Replicate/fill by id | source formula text or formula document, source node/caller context, ordered target node ids/caller contexts, reference-resolution/bind handles where available, profile/capability context | one recomposed formula text per target, text-change/rewrite provenance, dry-bind diagnostics/profile violations |
| F4 binding toggle | formula text or formula document, cursor/span/reference handle, caller context, profile context | recomposed formula text with the next absolute/relative binding mode, changed span/provenance, diagnostics |
| Point-mode reference insertion | formula text/edit buffer, host-provided target reference descriptor/handle, insertion or replacement span, caller context, profile context | insert text or full recomposed formula text, applied span, diagnostics/profile violations |

The operation result should be suitable for OxCalc to rebind and schedule without DnaTreeCalc
interpreting formula syntax. It should preserve source spans or stable handles when possible so the
Skin IR can explain what changed without parsing formula text.

## Boundary

- OxFml owns formula text composition, grammar legality, binding legality, and profile diagnostics.
- OxCalc owns dependency descriptors, invalidation, transactions, publication, and runtime reference-provider descriptors after the new text is bound.
- DnaTreeCalc host owns caret/edit-buffer state, authoring scope expansion, dispatch receipts, and projection.
- Skins render and dispatch only.

Until this API exists, DnaTreeCalc should treat `replicate-by-id`, `f4-toggle-binding`, and
`reference-insertion` as blocked for ownership-correct implementation. The current W3 implementation
can proceed with non-rewrite authoring slices such as number-format write, notes, or metadata where
the host owns the structural storage and OxFml/OxCalc still own semantic consequences.

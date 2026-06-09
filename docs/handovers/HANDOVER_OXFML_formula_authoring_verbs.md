# HANDOVER_OXFML_formula_authoring_verbs

Status: Open
Target: OxFml
Ask: Add OxFml-owned formula authoring operations for W3 TreeCalc skin intents: replicate/fill by id, F4 binding toggle, and formula/subtree rebind. Point-mode reference insertion by typed host target is landed for the current TreeCalc selector profile.
Context: DNA TreeCalc W3 requires reference/content authoring verbs that carry ids, handles, scopes, and caller context. Skins must never synthesize formula text, and the DnaTreeCalc host must not implement formula rewrite semantics with string manipulation. OxFml owns formula parse, bind, reference text composition, absolute/relative binding syntax, and profile gating; OxCalc owns the rebind/dependency/scheduling consequence after the recomposed formula text is produced.
Evidence: `docs/ux/stack-requirements/ROADMAP.md` W3 lists `replicate-by-id`, `f4-toggle-binding`, and `reference-insertion` before broader paste/duplicate work. Current OxFml public editor facade (`crates/oxfml_core/src/consumer/editor`) now exposes typed host-reference insertion for host names, host-reference collections, and host structural selectors, including TreeCalc profile selector spelling and bracket escaping. It still does not expose handle/id-based formula rewriting operations for F4 binding toggles, replicate/fill, formula paste/rebind, or subtree internal-reference rebind.

Update 2026-06-09: A fresh code inspection found `AddressMode` and source spans in OxFml binding internals, but no public TreeCalc host-reference rewrite entrypoint that can toggle binding modes or recompose formulas by source/target node context. DnaTreeCalc therefore landed only a formula-free `duplicate-subtree` slice and rejects formula-bearing subtrees before mutation rather than copying stale formula text.

## Required Shape

The DnaTreeCalc host needs an OxFml API family along these lines:

| Operation | Host supplies | OxFml returns |
|---|---|---|
| Replicate/fill by id | source formula text or formula document, source node/caller context, ordered target node ids/caller contexts, reference-resolution/bind handles where available, profile/capability context | one recomposed formula text per target, text-change/rewrite provenance, dry-bind diagnostics/profile violations |
| F4 binding toggle | formula text or formula document, cursor/span/reference handle, caller context, profile context | recomposed formula text with the next absolute/relative binding mode, changed span/provenance, diagnostics |
| Point-mode reference insertion | formula text/edit buffer, host-provided target reference descriptor/handle, insertion or replacement span, caller context, profile context | Landed for typed host names, host-reference collections, and host structural selectors; richer selector-choice UX may still widen the target surface. |
| Subtree internal-reference rebind support | source formula text per cloned node, source-to-clone node mapping, external-reference preservation policy, target caller contexts, profile context | recomposed formula text per cloned node, diagnostics, and stable provenance for which references rebound internally versus stayed external |

The operation result should be suitable for OxCalc to rebind and schedule without DnaTreeCalc
interpreting formula syntax. It should preserve source spans or stable handles when possible so the
Skin IR can explain what changed without parsing formula text.

## Boundary

- OxFml owns formula text composition, grammar legality, binding legality, and profile diagnostics.
- OxCalc owns dependency descriptors, invalidation, transactions, publication, and runtime reference-provider descriptors after the new text is bound.
- DnaTreeCalc host owns caret/edit-buffer state, authoring scope expansion, dispatch receipts, and projection.
- Skins render and dispatch only.

Until the remaining rewrite APIs exist, DnaTreeCalc should treat `replicate-by-id`,
`f4-toggle-binding`, formula paste/rebind, and formula-bearing subtree duplication as blocked for
ownership-correct implementation. Formula-free subtree duplication may proceed as host structural
orchestration because it does not rewrite formula text or reinterpret formula semantics.

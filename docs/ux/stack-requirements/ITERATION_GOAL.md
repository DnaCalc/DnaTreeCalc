# Stack Improvement Iteration Goal

## Goal Statement

Drive DNA TreeCalc skinning-stack improvements in roadmap order by pushing durable truth to the
layer that owns it, then exposing that truth through the host projection and Skin IR without
skin-side semantics.

Each iteration should advance the earliest unmet requirement in `ROADMAP.md` that is not blocked by
an unbuilt substrate. For W0/W1, that usually means exposing or extending facts OxCalc/OxFml already
compute: stable node identity, typed invalidation and dependency records, typed values, reference
resolution, run state, phase timing, runtime effects, overlays, derivation traces, formats, and
binding diagnostics. For W2 and later, it means first proving the required engine substrate exists
or deliberately spiking it before adding host or skin affordances.

The iteration is successful only when the capability is visible from outside the engine through the
published `WorkspaceState` or closed `WorkspaceIntent` seam, covered by Skin test plumbing, and
reported against the roadmap with concrete evidence and explicit remaining gaps.

## Iteration Rule

1. Start from the earliest unmet roadmap wave, not from incidental UI polish.
2. Verify the readiness claim against code before treating an item as `expose`, `extend`, or `new`.
3. Implement the capability where its truth lives:
   - OxFml owns grammar, bind, single-node evaluation, reference text composition, and format parsing/rendering.
   - OxCalc owns multi-node scheduling, dependency graph, invalidation, epochs, publication, overlays,
     cycles, candidates, and revisions.
   - DnaTreeCalc host owns projection, closed intents, structural editing, selection, and workspace dispatch.
   - Skins render and dispatch only; they do not parse, bind, recompute, or invent semantic facts.
4. Thread the result through the host projection or intent receipt before calling it useful.
5. Exercise it through programmable Skin IR tests or a real skin, not only engine-local tests.
6. Report product scope, evidence, known exclusions, and the next blocked or unblocked roadmap item.

## Working Checklist

### W0 / W1 Exposure Tranche

- [x] Carry stable `NodeKey` beside display path in Skin IR transition window.
- [x] Project typed dependency kinds.
- [x] Project typed invalidation reasons.
- [x] Project typed run state and node calc state.
- [x] Project typed phase-timing keys.
- [x] Surface runtime effects and runtime overlays in active skin detail.
- [x] Surface current derivation trace records in active skin detail.
- [ ] Complete host cutover from path-keyed maps to `NodeKey`-keyed maps.
- [ ] Preserve richer scalar/error/reference value variants through Skin IR, not only arrays and display text.
- [x] Add the reference-resolution map: token/source handle to target plus reverse index.
- [ ] Audit derivation trace payloads against the full prepared-call tree and hole-binding requirement.
- [ ] Add typed binding diagnostics intake where OxFml exposes them.
- [ ] Add per-node effective format and OxFml-backed render entrypoint plumbing.
- [ ] Confirm or implement per-node published-value epochs; keep delta work decoupled until then.

### W2 Structural Authoring Tranche

- [ ] Replace remaining `Rejected(String)` receipt paths with typed `IntentError` variants where the
      host or engine already has typed truth.
- [ ] Define `AuthoringScope` use for multi-node, subtree, and collection subjects.
- [ ] Add edit transaction ids with real semantics once OxCalc transaction scope exists.
- [ ] Spike OxCalc `transaction-scope`: batch edit boundary, rollback, schedule once, publish once.
- [ ] Add dry-bind and recalc-plan preview only after OxFml/OxCalc readiness is confirmed.

### Gating Engine Workstreams

- [ ] `transaction-scope`: design spike, then engine implementation if go.
- [ ] `revision-graph-retention`: retained parent-linked revision store and cursor; no inverse replay.
- [ ] `candidate-overlay-handle`: addressable, layerable, non-publishing candidate contexts.
- [ ] `value-epoch-keying`: per-node published-value epoch distinct from input epoch.

## Status Template

Use this shape at the end of each iteration:

```text
Product status: <roadmap item and exact supported scope>
Evidence: <tests, checks, screenshots, or code path exercised>
Still open: <concrete remaining gaps or blocked dependents>
Formal status: <spec/proof/model status if relevant, otherwise "not applicable">
Next roadmap item: <earliest unblocked item>
```

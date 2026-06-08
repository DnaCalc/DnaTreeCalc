# Stack Improvement Iteration Goal

## Goal Statement

Execute the DNA TreeCalc stack-requirements roadmap in dependency order, starting from the earliest
unmet wave item and moving durable semantic truth into the repo that owns it: OxFml for formula
language, binding, rendering, and single-node semantics; OxCalc for multi-node calculation,
dependency, invalidation, publication, overlays, revisions, and candidate state; DnaTreeCalc host for
closed intents and projection; skins for rendering and dispatch only.

The working loop is: verify the roadmap readiness claim against real code, implement or expose the
capability at its owning layer, thread it through the host projection or closed intent seam, exercise
it from outside the engine through Skin IR tests or a real skin, then record product scope, evidence,
known gaps, and the next roadmap item. W0/W1 should burn down high-leverage `expose`/`extend` work:
stable node identity, typed invalidation/dependency/run/value facts, reference resolution, binding
diagnostics, derivation/runtime/overlay detail, effective formatting, and per-node value epochs. W2+
must not fake missing engine substrates: transaction scope, revision graph retention, candidate
overlay handles, and value-epoch keying require an explicit spike or implementation before dependent
host or skin affordances are scheduled.

An iteration is complete only when downstream consumers can observe the capability through
`WorkspaceState`, `WorkspaceIntent`, or the published Skin IR surface, and the checklist below marks
the exact roadmap item advanced. Incidental UI polish is allowed only when it supports the active
roadmap item; it is not the default next move.

The current working target is the W2 structural-authoring tranche. W0/W1 exposure work has already
established the stable identity spine, typed engine facts, reference-resolution visibility,
binding diagnostics, effective formatting, derivation/runtime detail, and per-node published-value
epochs. The next iterations should therefore advance first-class authoring subjects and typed
intent receipts, then stop at the `transaction-scope` gate unless OxCalc has a verified spike path
for atomic accumulate-and-publish-once semantics. Larger time/speculation features stay queued
behind the explicit engine workstreams: transaction scope, retained revision graph, and candidate
overlay handles.

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
- [x] Add a `NodeKey` to display-path lookup index on `WorkspaceState` for the cutover transition.
- [x] Complete host cutover from path-keyed semantic maps to `NodeKey`-keyed node and dependency
      maps; retain path maps only as transition/display compatibility.
- [x] Preserve richer scalar/error value variants through Skin IR, not only arrays and display text.
- [x] Prove Skin IR projection for reference-valued `CalcValue`; ordinary TreeCalc node-result
      producer remains upstream in OxCalc/OxFml, so no host-side producer is fabricated.
- [x] Add the reference-resolution map: token/source handle to target plus reverse index.
- [x] Audit and complete published-run derivation trace payloads for prepared-call tree, hole
      bindings, typed root result, typed child-call results, and typed prepared argument values.
- [x] Add typed binding diagnostics intake from OxFml `BindDiagnostic` through OxCalc outcome,
      `NodeView`, `CalcRunProjection`, and active-node detail.
- [x] Add per-node effective number-format projection from `Format.NumberFormat` meta nodes and
      OxFml-backed numeric display rendering through `NodeView` and active-node detail.
- [x] Implement OxCalc per-node published-value epochs distinct from input epochs and project them
      through `NodeView.value_epoch` and active-node detail; keep delta work decoupled.

### W2 Structural Authoring Tranche

- [x] Replace legacy `Rejected(String)` receipt paths with typed `IntentError` variants for
      host/session structural and table errors; keep named `EngineRejected` / `HostFailure`
      fallbacks for genuinely untyped failures.
- [x] Define `AuthoringScope` as the typed subject model for node, ordered multi-node, subtree, and
      reference-collection subjects, with host-owned projection expansion and Skin IR tests; mutating
      multi-target verbs remain gated on transaction scope.
- [x] Add edit transaction ids with real semantics for node edit/rename/move/reorder/delete receipts
      by threading OxCalc transaction outcomes through host `IntentReceipt`; add-node, table, and
      scoped multi-target transaction ids remain gated on broader OxCalc transaction operation
      coverage.
- [x] Spike OxCalc `transaction-scope`: go for an OxCalc-owned Stage 1 batch edit API with rollback
      and optional recalc/publish-once; first node-edit engine slice implemented upstream.
- [ ] Add dry-bind and recalc-plan preview only after OxFml/OxCalc readiness is confirmed.

### Gating Engine Workstreams

- [ ] `transaction-scope`: first OxCalc node-edit transaction slice implemented and routed through
      DnaTreeCalc receipts for edit/rename/move/reorder/delete; add-node results, table/scoped
      verbs, and broader operation families still required.
- [ ] `revision-graph-retention`: retained parent-linked revision store and cursor; no inverse replay.
- [ ] `candidate-overlay-handle`: addressable, layerable, non-publishing candidate contexts.
- [x] `value-epoch-keying`: per-node published-value epoch distinct from input epoch.

## Status Template

Use this shape at the end of each iteration:

```text
Product status: <roadmap item and exact supported scope>
Evidence: <tests, checks, screenshots, or code path exercised>
Still open: <concrete remaining gaps or blocked dependents>
Formal status: <spec/proof/model status if relevant, otherwise "not applicable">
Next roadmap item: <earliest unblocked item>
```

# Stack Improvement Iteration Goal

## Goal Statement

Execute the DNA TreeCalc stack-requirements roadmap as an engine-first product program: each
iteration takes the earliest unclosed roadmap capability, verifies where the truth lives in the
current code, implements the smallest useful tranche in that owning layer, and proves the result
through the DnaTreeCalc host projection, closed intent seam, or Skin IR test plumbing.

The purpose is not to polish skins opportunistically. The purpose is to make richer skins possible
without giving them semantic ownership. OxFml must own formula parsing, binding, rewriting, and
format rendering; OxCalc must own dependency facts, invalidation, transactions, publication,
overlays, epochs, revisions, and scheduling; the DnaTreeCalc host must project typed facts and
dispatch closed intents; skins must only render state and send typed commands.

The iteration loop is:

1. choose the earliest unmet `ROADMAP.md` wave item,
2. confirm the readiness tag against live OxFml/OxCalc/DnaTreeCalc code,
3. classify the work as `expose`, `extend`, or `new substrate`,
4. implement in the repo that owns the truth,
5. thread the capability through the host projection or intent receipt,
6. prove it from outside the engine with programmable Skin IR tests or a real skin,
7. record exact supported scope, evidence, exclusions, and the next cursor, and
8. commit the affected repos before starting the next tranche.

Current cursor: **W3 - Reference and content authoring verbs**. W2 safe structural authoring is
closed for the current Skin IR surface: receipts carry typed errors and real OxCalc transaction ids,
previews use OxFml dry-bind plus OxCalc invalidation planning, and the closure review found no
skin-side formula parsing, semantic value computation, or transaction-id fabrication.

## Roadmap Position

W0/W1 established the identity spine and typed published facts. W2 established safe structural
authoring, typed errors, legality/impact previews, and transaction receipts for the current Skin IR
surface. W3 now tests the next ownership boundary: authoring commands that carry ids, handles, and
scopes while OxFml composes formula text and OxCalc rebinds and schedules.

When a requirement depends on a real engine gate (`transaction-scope`,
`revision-graph-retention`, `candidate-overlay-handle`, or remaining value-epoch shape work), the
iteration stops being host/Skin IR projection work and becomes an OxCalc spike or implementation
until that substrate is genuinely available.

For a compact execution checklist, use
[`ROADMAP_EXECUTION_CHECKLIST.md`](ROADMAP_EXECUTION_CHECKLIST.md).

The roadmap alignment rule is:

| Roadmap slice | Iteration focus | Owning-layer test |
|---|---|---|
| W0 | Stable identity and typed engine facts | The host can correlate nodes and read typed dependency, invalidation, run, timing, and value facts without parsing prose. |
| W1 | Value-faithful display and deep read | Skins can read formatted values, reference-resolution facts, binding diagnostics, runtime effects, derivation traces, cycle facts, and value epochs from published state. |
| W2 | Safe structural authoring | A skin can ask whether an edit is legal, what it will invalidate, and why it may fail, using typed receipts and previews rather than committing speculative semantics host-side. |
| W3 | Reference/content authoring verbs | Authoring commands carry ids, handles, scopes, and profile-aware requests; OxFml recomposes formula text and OxCalc rebinds and schedules. |
| W4a/W4b/W4c | Revision history and speculation | Dependent UI work waits for OxCalc-owned revision graph retention and addressable candidate overlays; skins never fake undo, time travel, or what-if state. |
| W5+ | Platform and frontier capabilities | Delta channels, worker hosting, composition, table operations, import/export, sweeps, RTD, and onboarding ride only on the earlier engine substrate that makes them truthful. |

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
7. After each implemented tranche, review the changed repos with fresh eyes for ownership drift,
   host-side semantic fabrication, missing tests, and roadmap-order mistakes before updating this
   checklist or committing.

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
- [x] Add edit transaction ids with real semantics for node add/edit/rename/move/reorder/delete
      receipts by threading OxCalc transaction outcomes through host `IntentReceipt`; table and
      scoped multi-target transaction ids remain gated on broader OxCalc transaction operation
      coverage.
- [x] Spike OxCalc `transaction-scope`: go for an OxCalc-owned Stage 1 batch edit API with rollback
      and optional recalc/publish-once; first node-edit engine slice implemented upstream.
- [x] Add OxCalc committed-graph recalc-plan preview for node-level preview mutations, and project it
      through host/Skin IR tests without evaluation, candidate creation, publication, or mutation.
- [x] Add OxFml dry-bind verdicts for uncommitted formula edits; first node-formula edit slice now
      flows OxFml parse/bind verdicts through OxCalc TreeCalc host context into Skin IR without
      mutation or evaluation. First joined node-content legality-impact preview now combines that
      dry-bind verdict with OxCalc committed-graph invalidation planning in Skin IR. Table body and
      totals formula edit previews now dry-bind through OxCalc's table formula context and project as
      typed table subjects in Skin IR. Profile violations now have a typed `FunctionUnavailable`
      taxonomy from OxFml capability overlays and are threaded through OxCalc and Skin IR. New table
      formula-column preflight now dry-binds through an OxCalc-owned preview table context and
      projects through Skin IR without mutating the table shape. OxCalc table snapshot preview
      planning now classifies formula-column insertion through its structured-table update impact
      taxonomy and carries typed table invalidation/dependent seeds into Skin IR legality-impact
      preview. Scoped content-edit legality-impact preview now expands `AuthoringScope` through
      projected host state, dry-binds each target through OxCalc/OxFml, and plans the combined
      invalidation through OxCalc without mutating state. Rename legality-impact preview now joins
      host-owned same-parent name collision detection with OxCalc structural invalidation planning
      and projects typed `NameCollision` blockers through Skin IR. Move/drop legality-impact preview
      now joins host-owned drop validity and destination collision checks with OxCalc structural
      invalidation planning. Delete/orphan structural preview now reports outside dependents from
      engine-published reference-resolution maps and OxCalc delete invalidation planning without
      mutating state. Add-node preview now carries typed initial-content policy and `is_meta`,
      reports name collisions before mutation, accepts empty/literal policies, and returns typed
      unsupported-policy blockers for inherited column formulas and template-bound content until
      those later substrates exist. Table row/column structural previews now cover add, delete,
      rename, and reorder operations with typed table-collision and duplicate-input blockers and
      OxCalc table-snapshot invalidation planning without mutating table state. Table snapshot
      authoring receipts now use OxCalc transaction outcomes for table row delete/rename/reorder,
      formula-column add/edit/delete, totals/header visibility and formula edits, and table column
      delete/rename/reorder. Remaining W2 closure is the final ownership review.
      Scoped existing-node content edits now carry `AuthoringScope` through Skin IR and are expanded
      by the host into one OxCalc batch edit transaction with one receipt transaction id. OxCalc now
      exposes engine-owned reserved node ids for transaction builders, and DnaTreeCalc uses them to
      route `AddTableRow` and constant `AddTableColumn` through real OxCalc transactions without
      host-predicted node ids.
- [x] W2 closure review: scan confirmed current skins render/project and dispatch typed intents
      without parsing formula syntax, computing semantic values, or fabricating transaction ids.

### W3 Reference / Content Authoring Tranche

- [ ] Assess the first W3 authoring verb slice against live OxFml/OxCalc APIs:
      `replicate-by-id`, `f4-toggle-binding`, `reference-insertion`, `clipboard-transfer-model`,
      `paste-special`, `duplicate-subtree`, `set-membership-write`, `meta-and-attribute-write`,
      `note-write`, `format-write`, and `add-node-content-policy` widening. Pick the earliest slice
      that preserves ownership: OxFml composes or rewrites formula text; OxCalc rebinds and
      schedules; DnaTreeCalc host carries ids, handles, and scopes through closed intents; skins
      dispatch only.

### Gating Engine Workstreams

- [x] `transaction-scope`: first OxCalc node-edit transaction slice implemented and routed through
      DnaTreeCalc receipts for add/edit/rename/move/reorder/delete; table snapshot operations now
      route through OxCalc `SetNodeTable` transactions for row delete/rename/reorder,
      formula-column add/edit/delete, totals/header visibility/formula edits, and column
      delete/rename/reorder. Scoped existing-node content edits route through one OxCalc batch edit
      transaction. Generated-node table operations (`AddTableRow`, constant `AddTableColumn`) now use
      OxCalc reserved node ids and carry real transaction ids for the current Skin IR table-add
      surface.
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

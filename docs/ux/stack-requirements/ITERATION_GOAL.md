# Stack Improvement Iteration Goal

## Operational Goal

Drive the stack-requirements roadmap as an engine-first execution program for DNA TreeCalc: each
iteration must move the earliest unclosed roadmap capability from verified engine/language truth,
through the DnaTreeCalc host projection or closed intent seam, into Skin IR evidence that a skin can
consume without owning semantics.

The iteration cursor is therefore not "make the skins nicer." It is:

1. pick the earliest unmet item in `ROADMAP.md`,
2. confirm whether it is an exposure, an extension, or genuinely new substrate in the live code,
3. implement the smallest useful tranche in the owning repo,
4. thread it through the host/Skin IR seam,
5. prove it from outside the engine with programmable Skin IR or real-skin tests,
6. update this goal/checklist with exact supported scope and remaining gaps,
7. commit the affected repos before taking the next tranche.

Current cursor: finish **W2 - Subjects, transactions, typed errors, and safe structural authoring**.
The next useful work is not W3 authoring verbs or W5 platform polish until W2 transaction coverage
and the W2 closure review are honest. The main known W2 gap is that several table and scoped
multi-target operations still need real OxCalc transaction coverage, especially where generated
node ids make a single snapshot transaction insufficient.

## Goal Statement

Execute the DNA TreeCalc stack-requirements roadmap in dependency order, with each iteration tied to
the earliest unmet wave item rather than to incidental UI polish. The purpose of the loop is to move
semantic truth upward from the owning engine or language layer into the host projection and Skin IR,
so skins can become richer without parsing formulas, inventing values, reinterpreting dependency
facts, or owning calculation behavior.

Each iteration must do four things:

1. identify the active roadmap slice and the exact requirement it advances,
2. verify the readiness tag against the current OxCalc, OxFml, and DnaTreeCalc code,
3. implement or expose the capability in the repo that owns the truth, and
4. prove the downstream behavior through host receipts/projection, programmable Skin IR tests, or a
   real skin.

The current cursor is W2 safe structural authoring. W0/W1 established the identity spine and typed
published facts; the next work should continue closing W2 legality, dry-bind, transaction, and
impact-preview scope before advancing to W3 authoring verbs or W4/W5 substrate consumers. When a
requirement depends on one of the real engine gates (`transaction-scope`,
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
- [ ] Add OxFml dry-bind verdicts for uncommitted formula edits; first node-formula edit slice now
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
      delete/rename/reorder. Remaining W2 closure is the final ownership review plus transaction
      substrate expansion for generated-node table operations such as adding constant rows/columns.
      Scoped existing-node content edits now carry `AuthoringScope` through Skin IR and are expanded
      by the host into one OxCalc batch edit transaction with one receipt transaction id.

### Gating Engine Workstreams

- [ ] `transaction-scope`: first OxCalc node-edit transaction slice implemented and routed through
      DnaTreeCalc receipts for add/edit/rename/move/reorder/delete; table snapshot operations now
      route through OxCalc `SetNodeTable` transactions for row delete/rename/reorder,
      formula-column add/edit/delete, totals/header visibility/formula edits, and column
      delete/rename/reorder. Scoped existing-node content edits route through one OxCalc batch edit
      transaction. Generated-node table operations (`AddTableRow`, constant `AddTableColumn`) still
      require broader OxCalc transaction substrate.
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

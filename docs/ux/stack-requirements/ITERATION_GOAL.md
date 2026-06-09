# Stack Improvement Iteration Goal

## Goal Statement

Drive the DNA TreeCalc stack-requirements roadmap in dependency order by turning each iteration into
one ownership-correct capability push: take the earliest unclosed roadmap item, verify the current
OxFml/OxCalc/DnaTreeCalc code against the readiness claim, implement the smallest useful tranche in
the layer that owns the truth, and prove the result from the outside through the host projection,
closed intent seam, programmable Skin IR tests, or a real skin.

Each iteration must leave the stack more truthful, not just more decorated. The work should expose
or strengthen engine facts that skins can consume, route authoring through closed typed intents, and
avoid any host or skin workaround that parses formulas, recomputes values, fabricates transaction
ids, or invents semantic state that OxFml or OxCalc should own.

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

The running success test is: a future skin can become richer because it reads more typed truth or
sends a better typed command, while the semantic boundary remains obvious in code and tests.

Current cursor: **W3 - Reference and content authoring verbs**. W2 safe structural authoring is
closed for the current Skin IR surface: receipts carry typed errors and real OxCalc transaction ids,
previews use OxFml dry-bind plus OxCalc invalidation planning, and the closure review found no
skin-side formula parsing, semantic value computation, or transaction-id fabrication. The first W3
assessment found that formula rewrite verbs (`replicate-by-id`, `f4-toggle-binding`, and
`reference-insertion`) require an OxFml-owned authoring API; DnaTreeCalc filed a handoff and now has
the first formula-authoring slice landed for point-mode node reference insertion. OxFml composes
host-reference text from typed targets and the TreeCalc host-reference syntax profile, including
bracket escaping and profile-selected selector spellings, and the editor path now parses incremental
edits with the same host-reference profile it binds with. DnaTreeCalc exposes a closed
`InsertFormulaReference` Skin IR intent that carries the edited node key, current edit-buffer text,
replacement span, and typed target; the host maps keys to projected node facts, calls OxFml, dry-binds
the recomposed formula through OxCalc, and commits it through a real content transaction.
The second reference-insertion tranche is also landed: programmable Skin IR tests now exercise
host-reference collection targets such as `Base.@CHILDREN` and structural selector targets such as
`A.@NEXT` from outside the engine, proving recomposed formula text, OxCalc dependency resolution, and
computed values. The third reference-insertion tranche is landed: successful receipts now carry a
typed `FormulaReferenceInserted` delta with the inserted text, updated formula text, applied span,
target, and edited node key, so skins can observe the OxFml-authored text without reconstructing it.
`replicate-by-id`, `f4-toggle-binding`, formula paste/rebind, formula-and-format paste, subtree
internal-reference rebind, and richer editor UX around selector choice remain open. DnaTreeCalc has
also landed the first ownership-correct W3 slice: number-format write through host-owned meta nodes and
real OxCalc transactions. The next W3 slice, `note-write`, is also landed through canonical
host-owned `Note` meta nodes and Skin IR note projection. `meta-and-attribute-write` is now landed
for the current Skin IR surface: `SetMeta` is an OxCalc-owned revisioned meta-membership edit, and
`SetNodeAttributes` patches a host-owned string attribute bag through revisioned meta nodes. The
first `add-node-content-policy` widening is landed for literal formula initial content: OxCalc
dry-binds the prospective node without mutating, DnaTreeCalc previews syntax/bind/profile blockers,
and commit rejects invalid literal formulas before model mutation. The next `add-node-content-policy`
slice is now implemented for `InheritColumnFormula { table, column_id }`: the host reads formula
text from table column metadata, OxCalc dry-binds it in the prospective new-node context, and
row-context/table-only formulas are rejected rather than faking table context. `TemplateBound` still
waits for the template subsystem. The first `clipboard-transfer-model` tranche is now landed:
`CopyToClipboard` populates a host-owned typed clipboard carrier for values, formula source,
formats, and subtrees from `AuthoringScope`, projects it through `WorkspaceState.clipboard`, and
emits a typed clipboard delta without involving the OS clipboard or fabricating paste semantics. The
first paste-special slice is also landed for format payloads only: `PasteClipboardFormat` consumes a
single copied format carrier and routes the write through the existing canonical
`SetNumberFormat`/meta-node transaction path. The next `clipboard-transfer-model` slice is also
landed: `CutToClipboard` populates the same typed carrier with `operation = Cut` while leaving the
model untouched until a later paste/commit verb owns the mutation. The next paste-special slice is
now landed for constant-source values only: value clipboard carriers record source content kind plus
an optional authored constant input string, and `PasteClipboardValues` applies a single constant
source through the scoped content transaction path without converting rendered values into input
text. Constant-value cut/paste commit is also now landed: a successful `CutToClipboard(Values)`
followed by `PasteClipboardValues` applies the target write and clears the source in one OxCalc
transaction, then clears the host clipboard. Rejected cut-paste attempts leave the source and
clipboard intact. The next constant-value paste-special slice is landed for multi-source authored
constants: multiple constant value carriers paste one-to-one over an explicitly ordered node target
scope in a single OxCalc transaction, and multi-source cut paste clears copied sources not included
in the target scope. The first computed-value paste-special slice is now landed for scalar cell
values and deterministic array constants: OxFml literalizes typed `CalcValue` payloads into authored
input text, including supported arrays as OxFml-authored array constant formulas, with explicit
unsupported verdicts for references, missing/non-finite values, rich/callable values, nested arrays,
empty array cells, and quote-containing array text in this slice. DnaTreeCalc projects that
literalization and `PasteClipboardValues` consumes it without using rendered display text. Formula
paste, formula/subtree source deletion, formula-and-format paste, and subtree rebind remain open. A
focused OxFml handoff records the remaining paste-special APIs for formula rebind,
formula-and-format paste, and subtree internal-reference rebind support. The W3
`set-membership-write` assessment is
also complete:
current OxCalc publishes `TreeReferenceCollectionDependency` facts and DnaTreeCalc can expand
`AuthoringScope::Collection` for read/scoped operations. OxCalc now has a first
`SetReferenceCollectionMembership` edit slice that validates owner/source-handle/member ids and
returns typed unknown/non-editable errors for current derived collections, but it still has no
positive authored membership/order store, version bump, invalidation, or descriptor republication.
A focused OxCalc handoff records the remaining substrate. A follow-up OxCalc spike ruled out a
descriptor-only positive edit for `ReferenceLiteralArrayV1`: edited descriptors can be produced, but
runtime evaluation remains bound to the original OxFml-authored formula source, so descriptor truth
and computed value truth diverge. Positive set-membership-write stays blocked until the owning seam
can update both dependency descriptors and evaluated reference collection values together. The first system-clipboard interchange slice is
now landed without giving the host OS clipboard authority: typed clipboard carriers project optional
plain text for platform export, and `PasteExternalClipboardText` accepts text supplied by the
skin/platform clipboard layer and routes it through the existing authored-content transaction path.
The next system-clipboard interchange slice is also landed for multi-item plain text: TSV/newline
text from the platform clipboard is flattened row-major and assigned one item per explicitly ordered
target node in a single OxCalc transaction when the item count exactly matches the expanded target
scope. Single-node paste preserves raw text, including newlines, and a single pasted text item still
broadcasts to the whole target scope. Count mismatches reject before mutation.

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

- [x] Assess the first W3 authoring verb slice against live OxFml/OxCalc APIs:
      `replicate-by-id`, `f4-toggle-binding`, `reference-insertion`, `clipboard-transfer-model`,
      `paste-special`, `duplicate-subtree`, `set-membership-write`, `meta-and-attribute-write`,
      `note-write`, `format-write`, and `add-node-content-policy` widening. Pick the earliest slice
      that preserves ownership: OxFml composes or rewrites formula text; OxCalc rebinds and
      schedules; DnaTreeCalc host carries ids, handles, and scopes through closed intents; skins
      dispatch only.
- [x] File the OxFml W3 formula-authoring handoff for handle/id-based formula rewrite verbs.
- [x] Land the first `reference-insertion` formula-authoring tranche:
      OxFml exposes `EditorEditService::insert_host_reference`, composes host-name,
      host-reference-collection, and host-structural-selector text from typed targets and the
      host-reference syntax profile, bracket-escapes non-identifier host names, and reparses editor
      edits with the same host-reference syntax profile used by binding. DnaTreeCalc exposes
      `WorkspaceIntent::InsertFormulaReference` over node keys, edit-buffer text, replacement span,
      and typed target; the host calls OxFml, dry-binds the recomposed formula through OxCalc, and
      commits it through the existing transaction path. The current Skin IR test covers node
      point-mode insertion into a formula and proves the resulting dependency/value after recalc.
      Full replicate/fill by id, F4 binding toggle, formula paste/rebind, formula-and-format paste,
      subtree internal-reference rebind, and richer multi-selector insertion UX remain open.
- [x] Widen `reference-insertion` Skin IR evidence to collection and structural selector targets:
      programmable Skin IR tests insert a typed `HostReferenceCollection` target and verify
      `=SUM(Base.@CHILDREN)` resolves to child value dependencies/results, and insert a typed
      `HostStructuralSelector` target and verify `=SUM(A.@NEXT)` resolves through OxCalc sibling
      navigation. This widens proof for the already OxFml-owned insertion API without adding
      host-side formula spelling.
- [x] Add typed authored output for `reference-insertion` receipts:
      successful `InsertFormulaReference` receipts now append
      `WorkspaceDeltaChange::FormulaReferenceInserted`, carrying edited node key, typed target,
      OxFml-composed inserted text, updated formula text, and applied span. Programmable Skin IR
      tests assert the delta for node, collection, and structural selector insertion.
- [x] Implement first `format-write` slice: `WorkspaceIntent::SetNumberFormat` over
      `AuthoringScope`, storing authored number-format codes in canonical `Format.NumberFormat`
      meta nodes, rejecting non-meta reserved-path collisions with typed errors, and carrying real
      OxCalc transaction ids through Skin IR receipts.
- [x] Implement `note-write`: `WorkspaceIntent::SetNote { node, note }` creates, updates, or
      clears a canonical `Note` meta node, projects `NodeView.note` and active-node note detail,
      round-trips through the OxCalc-backed workspace document, and rejects non-meta reserved-path
      collisions with typed errors.
- [x] Implement the `SetMeta` half of `meta-and-attribute-write`: OxCalc owns
      `OxCalcTreeEdit::SetNodeMeta`, meta membership participates in namespace/workspace revision
      identity, DnaTreeCalc routes `WorkspaceIntent::SetMeta` through a real transaction, and Skin
      IR tests prove revision movement, projected `is_meta`, and formula invisibility.
- [x] Implement the `SetNodeAttributes` half of `meta-and-attribute-write` for the current Skin IR
      surface: `WorkspaceIntent::SetNodeAttributes { node, attrs }` patches path-safe string
      attributes, stores them in canonical `Attributes.<key>` meta nodes through real OxCalc
      transactions, projects `NodeView.attributes` and active-node attributes, rejects invalid keys
      and non-meta reserved paths with typed errors, and keeps attributes formula-invisible. Richer
      typed/styling/template attributes remain separate future surfaces rather than hidden semantics.
- [x] Implement the first `add-node-content-policy` widening: `Literal(content)` initial formulas are
      dry-bound by OxCalc in a prospective new-node context without mutating the workspace; add-node
      preview carries typed syntax/bind/profile blockers; and add-node commit rejects invalid literal
      formulas before creating the node. `Empty`, literal constants, and `is_meta` remain supported.
      `InheritColumnFormula { table, column_id }` is now supported for table-column formula metadata
      that dry-binds as an ordinary node formula in the prospective target context; row-context or
      table-only formulas are rejected with bind diagnostics before mutation, and constant columns are
      rejected with typed table-column errors. `TemplateBound` remains blocked on the template
      subsystem.
- [x] Implement the first `clipboard-transfer-model` tranche:
      `WorkspaceIntent::CopyToClipboard { scope, payload }` expands host-owned `AuthoringScope` and
      projects a typed `WorkspaceState.clipboard` carrier for `Values`, `Formula`, `Format`, and
      `Subtree` payloads. Values use projected typed `NodeValueProjection`; formula copy carries one
      source `NodeKey` plus authored content text without rewriting it; format copy carries projected
      effective format; subtree copy carries the expanded key set. This tranche does not implement
      paste, cut, OS clipboard integration, formula rewrite, or subtree rebind.
- [x] Implement the first `paste-special` slice:
      `WorkspaceIntent::PasteClipboardFormat { target }` consumes a single `Format` clipboard
      payload and applies its `number_format_code` through the existing `set_number_format_transaction`
      path. It can also paste an unformatted source as a clear. This slice deliberately does not
      implement value paste, formula paste, OS clipboard integration, or subtree paste/rebind.
- [x] Implement the second `clipboard-transfer-model` tranche:
      `WorkspaceIntent::CutToClipboard { scope, payload }` populates the host-owned clipboard carrier
      with `ClipboardOperationProjection::Cut` using the same typed payload construction as copy.
      Cut does not delete nodes, advance revisions, or fabricate paste behavior; it only records the
      pending transfer operation for later paste semantics.
- [x] Implement the second `paste-special` slice:
      `WorkspaceIntent::PasteClipboardValues { target }` consumes a single value clipboard carrier
      only when the source was an authored constant. The carrier includes `content_kind` plus
      `constant_input_text`, and paste routes that authored input through the existing scoped content
      transaction path with a real OxCalc transaction id. It deliberately rejects computed formula
      values, arrays, formula paste, OS clipboard transfer, and subtree rebind until the owning
      OxFml/OxCalc literalization and rebind machinery exists.
- [x] Implement the first cut/paste commit slice for constant values:
      successful `CutToClipboard { payload: Values }` plus `PasteClipboardValues` applies the target
      constant write and clears the cut source in one OxCalc transaction when the target does not
      include the source, then clears the host clipboard. Failed paste attempts preserve both source
      content and clipboard. Formula/subtree source deletion remains open until the owning rebind
      machinery exists.
- [x] Implement the next constant-value paste-special slice:
      multi-source value clipboard carriers paste only when every source has authored constant input
      text and the target is an explicitly ordered node scope of the same length. Matching sources
      apply one-to-one in one OxCalc transaction. Multi-source cut paste clears copied sources not
      included in the target scope and clears the host clipboard after success. Computed value
      literalization is covered by the following tranche; formula paste, formula/subtree source
      deletion, and subtree rebind remain open.
- [x] Implement the first computed-value paste-special slice:
      OxFml exposes `CalcValue` authoring literalization for blank, finite number, text, logical,
      worksheet-error, and supported array values. Arrays are emitted as OxFml-authored array
      constant formulas, while references, missing/non-finite values, rich/callable values, nested
      arrays, empty array cells, and quote-containing array text remain explicit unsupported cases in
      this slice. DnaTreeCalc projects the authored literalization on
      `NodeView.literalized_value_input`, includes it in value clipboard carriers, and
      `PasteClipboardValues` consumes it through the existing scoped content transaction path.
      Authored constants still win over computed literalization. Formula paste/rebind,
      formula-and-format paste, subtree rebind, and formula/subtree cut source deletion remain open.
- [x] Fix typed array value retention after recalculation:
      OxCalc edge-cache hits now recover the retained typed published `CalcValue` when the cached
      display payload matches the same node's published display text, rather than reparsing display
      strings such as `Array(2x2)` as scalar text. DnaTreeCalc projection now prefers
      `last_outcome.published_calc_values` over literal-input node-view values, so explicit recalc
      keeps dynamic array values as `NodeValueProjection::Array` and leaves scalar literalization
      unsupported for arrays.
- [x] File the OxFml W3 paste-special handoff for computed value literalization, formula rebind,
      formula-and-format paste, and subtree internal-reference rebind support. The scalar
      literalization portion is now partially satisfied; the handoff remains open for arrays and
      formula/subtree rebind.
- [x] Assess `set-membership-write` against live OxCalc/DnaTreeCalc code and file the OxCalc handoff:
      current collection facts are published read/dependency descriptors only, and
      `AuthoringScope::Collection` is a projection expansion surface, not a membership mutation
      substrate. OxCalc now exposes a first validation/rejection edit slice for owner/source-handle
      checks and typed non-editable derived collection errors. `SetCollectionMembership` remains
      unsupported in Skin IR until OxCalc owns positive authored membership/order storage,
      invalidation, and descriptor republication.
- [x] Spike positive `set-membership-write` for the first plausible editable family:
      `ReferenceLiteralArrayV1` descriptor overrides alone are a no-go. A direct OxCalc test showed
      the descriptor membership/order can be changed independently, but the runtime result still
      evaluates the original OxFml-authored source (`=SUM({A,C,A})` remains `11` after a descriptor
      override to `C,A`). Positive membership writes therefore require an OxFml formula-rewrite or
      bound/evaluated-collection replacement seam that keeps dependency descriptors and computed
      values aligned. No Skin IR `SetCollectionMembership` support is exposed yet.
- [x] Implement the first system-clipboard interchange slice:
      `ClipboardProjection.plain_text` exports deterministic text for supported typed clipboard
      payloads (`Values` as scalar text or array TSV, `Formula` as authored formula text), while
      `PasteExternalClipboardText { target, text }` lets the platform supply OS clipboard text and
      routes it as authored content through the existing scoped content transaction path. The host
      still does not call browser or desktop clipboard APIs; rich OS clipboard formats remain open.
- [x] Implement the next system-clipboard interchange slice:
      multi-item plain-text paste splits platform-supplied TSV/newline text into row-major authored
      content items and applies them one-to-one over an explicitly ordered `AuthoringScope::Nodes`
      expansion in one OxCalc transaction. Single-node paste preserves raw text, including newlines,
      and a single pasted text item still broadcasts through the existing scoped-content path.
      Item/target count mismatches reject before mutation. Rich OS clipboard formats, formula
      rewrite/rebind paste, and subtree paste remain open.
- [ ] Continue W3 with the next ownership-correct slice. Candidate order: implement the next
      OxFml-backed formula authoring verb (`f4-toggle-binding`, `replicate-by-id`, formula
      paste/rebind) when its rewrite semantics are available, or record a focused blocker if the
      current OxFml editor surface cannot support it. Complete remaining `add-node-content-policy`
      only when template substrate exists.

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

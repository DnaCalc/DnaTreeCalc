# Stack Improvement Iteration Goal

## Goal Statement

Drive the DNA TreeCalc stack-requirements roadmap in dependency order by making each iteration a
small, ownership-correct capability push that moves semantic truth upward from the engine/language
layers into the host projection and Skin IR.

The work is not "skin polish" unless the roadmap item says the platform/skin layer owns it. The
default move is to expose or strengthen typed truth at the layer that owns it: OxFml owns formula
grammar, binding, rewriting, single-node evaluation, and format rendering; OxCalc owns dependency
facts, invalidation, transactions, publication, overlays, epochs, revisions, scheduling, and
candidate state; the DnaTreeCalc host owns projection, workspace dispatch, structural editing, and
closed intents; skins render typed state and send typed commands.

Each iteration should leave one of the roadmap waves more real:

| Roadmap band | Iteration meaning |
|---|---|
| W0/W1 | Expose typed identity, value, dependency, invalidation, formatting, trace, and runtime facts already owned by OxCalc/OxFml. |
| W2 | Make structural authoring safe through typed subjects, previews, transaction ids, and typed rejection. |
| W3 | Add content/reference authoring verbs without host-side formula rewriting or semantic reconstruction. |
| W4a/W4b/W4c | Build the real revision and candidate-overlay substrates before any undo, history, or what-if skin claim. |
| W5+ | Add platform/frontier features only on top of the typed engine substrate they require. |

The standard iteration loop is:

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

Current cursor: **W4b - Candidate substrate**, with scoped W4a revision graph work complete and the
W4b OxCalc spike answered: `candidate-overlay-handle` is new OxCalc substrate, not an exposure of
the current candidate/publication lane or the published-basis `RuntimeOverlaySet`. The first OxCalc
build slice is now landed: an opaque non-publishing candidate handle over a retained revision can
take a private node edit, evaluate private values, and discard without publishing live workspace
state. The first commit bridge is also landed: a candidate can commit into the live workspace only
when the live workspace revision still equals the candidate basis revision; stale-basis commit is a
typed engine rejection. The first host/Skin IR projection slice is also landed: skins can open a
candidate, apply a candidate-local content edit, evaluate private candidate values, discard the
candidate, or commit it when the basis is current. Candidate projections live beside published
workspace values, so candidate evaluation does not rewrite published node state. The first parented
copy-layer slice is now landed: a child candidate can open over a parent candidate's private state,
project its parent handle, and commit the stacked private state while parent lifecycle is guarded.
The candidate-private revision-history slice is now landed: candidate projections carry private
revision graph entries, apply-only private edit revisions carry real engine transaction ids without
fabricated invalidation summaries, and candidate commit receipts use the promoted revision's real
transaction id when one exists. The candidate basis-retention pin slice is also landed: OxCalc keeps
host-visible candidate basis revisions retained under bounded revision retention while candidate
handles are live, including shared-basis candidates and sibling candidates that survive another
candidate's commit. The candidate-private structural projection slice is now landed: OxCalc
candidate views carry private node structure, and DnaTreeCalc projects it through
`CandidateProjection.nodes` without rewriting published node state. The candidate run projection
slice is now landed for candidate-only nodes: DnaTreeCalc resolves candidate-private tree ids through
the candidate view when projecting candidate calculation runs, so evaluation order and invalidation
records can name candidate-added nodes without publishing them. The candidate inherited table-column
formula initial-content slice is now landed: candidate adds read formula text from candidate-private
table metadata, dry-bind it in the candidate prospective-node context, and reject row-context table
formulas before private mutation. The first candidate speculation-budget/GC slice is now landed:
OxCalc computes typed candidate pressure from live candidate handles, reaps unprotected candidates
to a requested budget, and DnaTreeCalc projects the pressure plus a closed `ReapCandidates` Skin IR
intent without host-side lifecycle fabrication. The stale-candidate rebase and flattened
parent-layer slice is now landed: OxCalc retains a private candidate edit log, replays it onto the
current workspace revision without publishing, flattens parented candidates by replaying their
captured layered private edits and dropping the parent handle, and DnaTreeCalc exposes a closed
`RebaseCandidate` intent that projects the rebased candidate without stale values until it is
explicitly evaluated. The live layering and first conservative merge/rebase conflict-policy slices
are also landed: parented candidates refresh from parent-private edits made after child open, and
stale candidate rebase returns a typed conflict report when live and candidate edits overlap on the
same stable node while non-overlapping edits still rebase. Candidate add-node parent/order conflict
classification is also landed: candidate adds mark their parent lane so a live sibling/ordering
change rejects stale rebase instead of silently merging. Old-parent/delete-descendant classification
is now landed too: candidate moves mark their source and destination parent lanes, candidate deletes
mark their removed subtree, and explicit candidate reorder marks the parent lane. The first
lane-aware merge slice is also landed: OxCalc now separates content-node touches from structural
parent/order lanes during candidate rebase, so candidate structural adds can rebase over live
content edits on the same parent and candidate rename/move can rebase over live content edits on
the affected node without publishing candidate-only structure. The compatible multi-edit rebase
slice is also landed: one stale candidate can combine private rename, move, and add edits over live
content edits on the affected nodes/parent, then commit the merged structure and values through the
normal candidate bridge. The first same-node structural facet merge slice is also landed: OxCalc
allows candidate rename over live move and candidate move over live rename for the same stable node
when replay validation succeeds, while competing same-node rename-vs-rename still rejects with a
typed rebase conflict. The first same-parent namespace merge slice is now landed too: candidate
rename can rebase over a live sibling add when the final namespace is legal, while a duplicate-name
collision discovered during replay validation is surfaced as a typed candidate rebase conflict.
The first same-parent order/name merge slice is now landed as well: candidate rename can rebase over
live sibling reorder and candidate reorder can rebase over live sibling rename. Skin IR now exposes
a closed `ReorderCandidateNode` intent so candidate-private order edits are testable without skin
semantics. The first sibling add/delete merge slice is now landed too: candidate add over live
sibling delete and candidate delete over live sibling add rebase and commit when the touched nodes
are distinct, while deleted-subtree overlap remains rejected. The first sibling add/reorder and
delete/reorder merge slice is now landed as well: candidate add/reorder and delete/reorder
combinations rebase and commit when touched/deleted nodes do not overlap and replay validation
succeeds, while competing reorder/order edits remain rejected. OxCalc node views now publish ordered
parent/child ids and DnaTreeCalc consumes those ids for published and candidate tree projections, so
skins no longer reconstruct child order from paths. Candidate add-node template policy, competing
structural name merge algebra beyond these rename/move, rename/add, rename/reorder, sibling
add/delete, sibling add/reorder, and sibling delete/reorder facets, and full scenario/what-if UX
remain open. The
first richer host-pin retention slice
is also landed: OxCalc exposes explicit candidate retention pins that protect active candidates from
budget reaping, DnaTreeCalc projects pin counts and pressure reason counts, and Skin IR exposes
closed pin/unpin intents without owning lifecycle semantics. The first W4c scenario-projection slice
is also landed: DnaTreeCalc projects a host-owned scenario manifest over existing OxCalc candidate
handles, creates/activates/deletes scenario labels through closed Skin IR intents, and pins the
backing candidate while the scenario exists so budget reaping does not erase the scenario rail.
The first W4c scenario-substrate override slice is also landed: Skin IR exposes closed
`SetScenarioOverride` / `ClearScenarioOverride` intents by stable `NodeKey`, the host literalizes
supported typed scalar and array `NodeValueProjection` payloads through OxFml into authored input
text, applies them through OxCalc candidate-private edit transactions, and clears overrides by
restoring the candidate-private input captured on first override. The next W4c scenario projection
freshness slice is also landed: scenario entries carry a host-owned scenario value epoch that
advances on scenario creation, override set/clear, candidate-private edits, and candidate
evaluation; active scenarios project per-node `NodeView.scenario_override` without changing
published `computed_value`. The first W4c comparative multi-overlay projection slice is also
landed: `WorkspaceState.comparison` projects a published basis column plus scenario-backed columns
whose values merge typed scenario overrides with the scenario backing candidate's typed
`values_by_key`, leaving unevaluated non-overridden values empty rather than fabricating values. The
first W4c `series-projection` slice is
also landed: `WorkspaceState.series` derives chart/feed series from the published basis and
scenario-backed comparison columns, keeps point values as typed `NodeValueProjection`, orders points
by workspace `key_order`, and leaves unevaluated scenario series empty rather than fabricating
values. The scoped/unit W4c series slice is also landed: `WorkspaceState::series_for_scope`
expands the existing `AuthoringScope` model into explicit chart/feed series and publishes unit
metadata from host-owned `series_unit` / `unit` attributes when every selected point agrees. These
slices deliberately do not implement direct sweep/goal-seek comparison columns, formula/rich-value
scenario override authoring, or engine-published scenario revision history. Remaining W3
formula-rewrite/rebind verbs stay parked until their owning
OxFml/OxCalc substrates are available. W2 safe structural authoring is
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
row-context/table-only formulas are rejected rather than faking table context. `TemplateBound` now
has a minimal built-in initial-content policy (`starter`, `input-zero`) that resolves to ordinary
content before OxCalc dry-bind/recalc; the full template subsystem remains future work. The first
`clipboard-transfer-model` tranche is now landed:
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
The first `duplicate-subtree` slice is now landed for formula-free ordinary subtrees:
`DuplicateSubtree` carries a source `NodeKey`, destination parent, and new symbol; the host expands
the projected subtree shape, rejects formula-bearing or table-backed subtrees before mutation, and
applies the clone as one OxCalc transaction with reserved engine node ids. Formula rebind,
formula/subtree source deletion, table subtree cloning, and meta-subtree breadth remain open.
The second `duplicate-subtree` slice preserves host-authored local notes, number formats, and
attributes by cloning the canonical metadata nodes in the same OxCalc transaction. Inherited
effective formats are not converted into local authored metadata. The third `duplicate-subtree`
slice is now landed: hidden non-canonical formula-free meta descendants are cloned
through the same OxCalc transaction without projecting those hidden nodes into ordinary
`WorkspaceState` views. Formula-bearing meta descendants, table-backed meta descendants, table
subtree cloning, formula rebind, and formula/subtree source deletion remain open. The fourth
`duplicate-subtree` slice is also landed for constant-only table snapshots: the host clones the
table node, assigns a fresh table id/name/path identity, recreates generated table cell meta nodes,
and points the cloned OxCalc table snapshot at those cloned generated nodes in one transaction.
Formula-backed table columns, totals formulas, formula-bearing table cell nodes, and
formula-visible table-name collisions still reject before mutation until OxFml/OxCalc table formula
rebind support exists.

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
      unsupported-policy blockers for invalid inherited column formulas and unknown template-bound
      content. Table row/column structural previews now cover add, delete,
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
      rejected with typed table-column errors. `TemplateBound` now supports a minimal built-in
      initial-content registry (`starter`, `input-zero`) that resolves to ordinary content before
      OxCalc dry-bind/recalc; unknown template ids remain typed unsupported-policy blockers. The full
      template subsystem remains future work.
- [x] Implement minimal `TemplateBound` add-node initial content:
      DnaTreeCalc resolves built-in template ids (`starter`, `input-zero`) to ordinary initial
      content before calling OxCalc for preview, published add-node, and candidate add-node paths.
      Formula templates are dry-bound in the prospective target context and evaluated by OxCalc
      after add/evaluate; unknown template ids remain typed unsupported initial content. Programmable
      Skin IR tests cover preview, published add, candidate add, computed formula result, and unknown
      template rejection. Full template definition/edit/instantiate/sync remains future template
      subsystem work.
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
- [x] `revision-graph-retention`: first OxCalc substrate slice landed in
      `0735d9c Retain workspace revision lineage`: `OxCalcTreeContext` retains an in-memory
      parent-linked revision graph, `workspace_view` exposes the current parent plus retained
      entries, and `OxCalcTreeEditTransaction` returns predecessor/successor revision ids with one
      public lineage edge per successful transaction. Follow-on OxCalc slice
      `8bc6283 Navigate retained workspace revisions` adds in-memory navigation to retained
      revisions and restores OxCalc-owned structural/input/namespace, table, publication,
      runtime-overlay, value-epoch, and diagnostic state. Current DnaTreeCalc slice projects the
      retained revision history into `WorkspaceState` and routes `NavigateRevision` through the
      host dispatcher to OxCalc without inverse replay. Current DnaTreeCalc follow-on adds
      `WorkspaceIntent::Undo` and `WorkspaceIntent::Redo` as host-owned cursor commands over the
      retained OxCalc revision graph: normal edit transactions record the previous revision as an
      undo boundary, redo is cleared on branch edits, successful undo/redo republishes the restored
      OxCalc snapshot, and selection is restored from the host cursor entry. The transaction-summary
      slice is also landed: OxCalc retains transaction id, invalidated node ids, rebind flags, typed
      invalidation reasons, and estimated invalidated-node count on successor revision entries, and
      DnaTreeCalc projects those facts through `RevisionHistoryEntryProjection.transaction_summary`
      keyed by `NodeKey`. Bounded retention is now OxCalc-owned and deterministic: retained
      in-memory revisions use oldest-first eviction through `OxCalcTreeRevisionRetentionPolicy`
      while preserving the current revision. Persistence policy is explicit: workspace snapshots
      persist the active revision/layer state, not the navigable retained history DAG. This closes
      scoped W4a for in-memory undo/history/time-scrub substrate; durable cross-session history is a
      future product/storage layer rather than hidden engine state.
- [x] `candidate-overlay-handle` spike: live OxCalc code inspection confirms the current
      `recalculate` / `AcceptedCandidateResult` / `RuntimeOverlaySet` path is one synchronous
      publish-or-reject lane, not addressable non-publishing speculation. OxCalc recorded the go
      decision and first build slice in
      `docs/spec/core-engine/CORE_ENGINE_CANDIDATE_OVERLAY_HANDLE_SPIKE.md` under bead `calc-etez`.
- [x] `candidate-overlay-handle` first OxCalc substrate slice: OxCalc now exposes
      `CandidateOverlayHandle`, opens candidates on retained revisions, applies private candidate
      edit transactions, evaluates private candidate values, and discards handles. The focused test
      `treecalc_context_candidate_evaluation_does_not_publish_workspace_state` asserts the live
      workspace revision, publication snapshot, runtime overlay set, visible value, and published
      value epoch remain unchanged. Follow-up commit/layering/projection work is tracked in OxCalc
      bead `calc-4ipg`.
- [x] `candidate-overlay-handle` first commit bridge slice: OxCalc
      `commit_candidate` promotes a candidate's private evaluated state into the live workspace only
      when the live revision still matches the candidate basis. Focused tests cover successful
      commit and typed stale-basis rejection while retaining the candidate for later discard or
      future rebase semantics.
- [x] `candidate-overlay-handle` first DnaTreeCalc host/Skin IR projection slice: Skin IR exposes
      closed candidate lifecycle intents for open, candidate content edit, evaluate, discard, and
      commit. `WorkspaceState.candidates` projects candidate values separately from published node
      values, and programmable Skin IR tests prove candidate evaluation does not publish until
      commit. Commit produces a new workspace revision; this slice does not fabricate a transaction
      id when the promoted candidate revision has no retained transaction summary.
- [x] `candidate-overlay-handle` first parented copy-layer slice: OxCalc child candidates open from
      a retained parent candidate's private state at child-open time, parent handles are projected
      through `CandidateProjection.parent_handle`, parent discard/commit is rejected while a retained
      child depends on it, and programmable Skin IR tests prove layered child values stay separate
      from published node values until commit.
- [x] `candidate-overlay-handle` candidate-private revision history slice: OxCalc revision graph
      entries now carry real transaction identity separately from optional invalidation summaries,
      candidate views/commit outcomes expose their private revision graph entries, and DnaTreeCalc
      projects that history through `CandidateProjection.revision_history`. Candidate edit/evaluate
      receipts remain non-publishing and transactionless; candidate commit receipts use the promoted
      private revision's real transaction id when present. Focused OxCalc and programmable Skin IR
      tests prove the transaction id is real and no invalidation summary is fabricated for
      apply-only candidate edits.
- [x] `candidate-overlay-handle` candidate basis-retention pin slice: OxCalc pins retained workspace
      revisions that are the basis of live candidates, reference-counts shared-basis candidates, and
      preserves sibling candidate basis pins when another candidate commits and replaces the live
      workspace state. Focused OxCalc tests prove the basis stays navigable while pinned and becomes
      evictable again under bounded-retention pressure after the last candidate releases it.
- [x] `candidate-overlay-handle` candidate structural projection/read slice: OxCalc candidate views
      project private node structure from the candidate workspace state, and DnaTreeCalc projects it
      through `CandidateProjection.nodes` separately from published `WorkspaceState.nodes`.
      Engine and programmable Skin IR tests prove a candidate-private structural shape can be read
      without rewriting the live workspace projection.
- [x] `candidate-overlay-handle` first closed structural candidate mutation intent: Skin IR exposes
      `RenameCandidateNode { handle, node: NodeKey, new_symbol }`, the host sends it through
      OxCalc's private candidate edit transaction, and programmable Skin IR tests prove the rename
      changes only candidate-private node structure until commit while preserving the real promoted
      transaction id.
- [x] `candidate-overlay-handle` move/delete structural candidate mutation intents: Skin IR exposes
      `MoveCandidateNode` and `DeleteCandidateNode` by stable `NodeKey`, the host sends them through
      OxCalc's private candidate edit transaction, and programmable Skin IR tests prove candidate
      move/delete update only candidate-private structure until commit while preserving real
      promoted transaction ids.
- [x] `candidate-overlay-handle` constant/empty candidate add-node intent: Skin IR exposes
      `AddCandidateNode` by stable parent `NodeKey`, the host reserves the node id and sends the add
      through OxCalc's private candidate edit transaction, and programmable Skin IR tests prove the
      added node appears only in candidate-private structure until commit.
- [x] `candidate-overlay-handle` formula-literal candidate add-node dry-bind: OxCalc exposes
      candidate-context dry-bind for prospective new nodes, DnaTreeCalc uses it for formula literal
      initial content, and programmable Skin IR tests prove a candidate-added formula can bind
      against candidate-private structure. Template initial content remains open.
- [x] `candidate-overlay-handle` candidate run projection for candidate-only nodes: DnaTreeCalc
      resolves candidate-private tree ids from `OxCalcTreeCandidateView.nodes` while projecting
      candidate calculation runs, and programmable Skin IR tests prove a candidate-added formula node
      appears in candidate `run.evaluation_order` without publishing to `WorkspaceState.nodes`.
- [x] `candidate-overlay-handle` inherited table-column formula candidate add-node policy:
      DnaTreeCalc reads candidate-private table metadata from `OxCalcTreeCandidateView.nodes`, asks
      OxCalc to dry-bind the inherited formula in the candidate prospective-node context, and
      programmable Skin IR tests prove successful inherited formula projection plus row-context
      rejection without private mutation.
- [x] `candidate-overlay-handle` candidate private-edit invalidation summaries:
      OxCalc attaches planning-derived transaction summaries to candidate apply-only revision entries
      when every private edit maps to an existing invalidation preview mutation, and DnaTreeCalc
      projects those summaries through candidate revision history. Programmable Skin IR tests prove a
      private candidate content edit carries invalidated-node counts and rebind facts without
      publishing live state. Unsupported candidate edit families remain summary-less rather than
      fabricated.
- [x] `candidate-overlay-handle` first speculation-budget/GC slice:
      OxCalc computes typed candidate pressure, including retained, protected, reclaimable, and
      over-budget candidate counts for a retention policy, and reaps unprotected candidates to a
      requested budget with deterministic handle order. DnaTreeCalc projects those pressure facts
      through `WorkspaceState.speculation_pressure` and exposes a closed `ReapCandidates` Skin IR
      intent. Programmable Skin IR tests prove candidate removal deltas and pressure updates are
      observed from outside the engine without publishing workspace node state.
- [x] `candidate-overlay-handle` host-retention pin slice:
      OxCalc exposes explicit candidate retention pins, reports child-protected and host-pinned
      pressure reason counts, and protects host-pinned candidates from budget reaping. DnaTreeCalc
      projects `CandidateProjection.retention_pin_count`, reason-specific speculation pressure, and
      closed `PinCandidateRetention` / `UnpinCandidateRetention` Skin IR intents. Programmable Skin
      IR tests prove a pinned candidate survives reaping while an unpinned candidate is reclaimed,
      pin/unpin emits candidate-change deltas, and an unbalanced unpin is rejected.
- [x] `candidate-overlay-handle` stale-candidate rebase and flattened parent-layer slice:
      OxCalc retains successful private candidate edit transactions and exposes
      `rebase_candidate_to_current_revision` to replay a candidate's private edit log onto the
      current workspace revision without publishing. Parented candidates flatten their captured
      layered private edits during rebase and drop the parent handle; child-retained rebase still
      rejects with a typed engine error. DnaTreeCalc exposes the closed `RebaseCandidate` Skin IR
      intent and projects the rebased candidate without stale values until explicit candidate
      evaluation. Focused OxCalc tests prove stale commit rejection, successful unparented rebase,
      flattened parented rebase, non-publishing evaluation, commit from the rebased basis, and parent
      lifecycle release after flattening; programmable Skin IR tests prove the same paths from
      outside the engine.
- [x] `candidate-overlay-handle` live-layering and conservative conflict-policy slice:
      OxCalc refreshes parented candidates from parent-private edits made after a child candidate was
      opened, preserving non-publishing semantics while keeping child values live with the parent
      layer. OxCalc also reports typed `CandidateRebaseConflict` details when stale candidate rebase
      finds overlapping live/candidate node edits against the candidate basis, while non-overlapping
      edits still rebase. DnaTreeCalc maps the report to a typed Skin IR `IntentError` with stable
      `NodeKey` overlaps. Focused OxCalc candidate tests prove conflict rejection, clean unparented
      rebase, clean parented flattening, and live child refresh; programmable Skin IR tests prove the
      same conflict/success paths from outside the engine. Richer structural merge algebra, template
      initial content, and scenario/what-if UX remain open.
- [x] `candidate-overlay-handle` add-node parent/order conflict slice:
      OxCalc now classifies candidate `AddNode` edits as touching their parent lane, so stale rebase
      rejects with typed `CandidateRebaseConflict` when the live workspace changes the same parent
      structure/order before rebase. Candidate `MoveNode` edits also mark the destination parent
      lane. DnaTreeCalc projects the same conflict as stable `NodeKey` overlaps through Skin IR.
      Focused OxCalc tests prove candidate-add versus live sibling-add conflict on the parent node;
      programmable Skin IR tests prove the same rebase rejection from outside the engine. Richer
      multi-edit merge algebra, template initial content, and scenario/what-if UX remain open.
- [x] `candidate-overlay-handle` old-parent/delete-descendant conflict slice:
      OxCalc now derives candidate rebase touch sets from the retained basis structural snapshot
      rather than only the node ids named directly in each edit. Candidate moves mark both source
      and destination parent lanes, candidate deletes mark the removed subtree plus parent lane, and
      explicit candidate reorder marks its parent lane. DnaTreeCalc continues to project the typed
      `CandidateRebaseConflict` as stable `NodeKey` overlaps without host-side structural merge
      semantics. Focused OxCalc tests prove old-parent move conflict, delete-descendant conflict,
      and explicit reorder parent-lane conflict; programmable Skin IR tests prove old-parent move
      and delete-descendant rebase rejection from outside the engine. Richer structural merge
      algebra, template initial content, and scenario/what-if UX remain open.
- [x] `candidate-overlay-handle` lane-aware rebase merge slice:
      OxCalc now classifies candidate/live rebase touches into content nodes, structural parent/order
      lanes, structural node edits, and deleted nodes rather than using one coarse node set for every
      edit kind. This preserves same-node content conflicts and structural lane conflicts while
      allowing candidate structural adds over live parent content edits and candidate rename/move
      edits over live content edits on the affected node. Focused OxCalc tests prove the positive
      merge cases and the existing conflict cases; programmable Skin IR tests prove accepted
      rebase/commit paths through the host projection. Broader multi-edit merge algebra, template
      initial content, and scenario/what-if UX remain open.
- [x] `candidate-overlay-handle` multi-edit structural/content rebase slice:
      The lane-aware rebase policy is now exercised across a candidate with multiple private
      structural edits in one stale overlay: rename, move, and add replay together over live content
      edits on the renamed node, moved node, and add parent without publishing candidate-only
      structure before commit. Focused OxCalc and programmable Skin IR tests prove the full
      rebase/commit path from engine and host seams. Full structural merge algebra for competing
      structural order/name/delete combinations, template initial content, and broader what-if UX
      remain open.
- [x] `candidate-overlay-handle` same-node rename/move structural facet merge slice:
      OxCalc now records structural rebase lanes as typed touches, so same-node candidate rename
      over live move and same-node candidate move over live rename can merge when replay validation
      succeeds. Competing same-node rename-vs-rename remains rejected as a typed
      `CandidateRebaseConflict`. Focused OxCalc and programmable Skin IR tests prove both accepted
      rebase/commit paths plus the rejection from outside the skin layer. Structural order/delete
      and broader name-collision merge algebra, template initial content, and broader what-if UX
      remain open.
- [x] `candidate-overlay-handle` same-parent rename/add namespace merge slice:
      OxCalc now treats candidate rename versus live sibling add as a compatible structural-lane
      pair and lets normal replay validation own final namespace legality. Non-colliding rename/add
      rebases and commits; duplicate-name replay rejection is converted to typed
      `CandidateRebaseConflict` instead of leaking as a generic structural failure. Focused OxCalc
      and programmable Skin IR tests prove the accepted and rejected paths. Structural order/delete
      merge algebra, template initial content, and broader what-if UX remain open.
- [x] `candidate-overlay-handle` same-parent rename/reorder structural facet merge slice:
      OxCalc now tracks candidate-private reordered nodes as a typed facet and treats rename versus
      reorder as compatible structural-lane touches when replay validation succeeds. Candidate rename
      over live sibling reorder and candidate reorder over live sibling rename both rebase and commit.
      Skin IR now exposes the closed `ReorderCandidateNode` intent by stable `NodeKey`, with
      programmable Skin IR tests proving both directions from outside the engine. Reorder/add index
      semantics, order/delete merge algebra, template initial content, and broader what-if UX remain
      open.
- [x] `candidate-overlay-handle` sibling add/delete structural merge slice:
      OxCalc now treats same-parent add/delete lane touches as compatible when the deleted/touched
      node sets do not overlap and replay validation succeeds. Candidate add over live sibling
      delete and candidate delete over live sibling add both rebase and commit, while the existing
      delete-descendant overlap test keeps destructive overlaps rejected. Programmable Skin IR tests
      prove both directions from outside the engine. Reorder/add index semantics, deeper delete
      merge algebra, template initial content, and broader what-if UX remain open.
- [x] `candidate-overlay-handle` sibling add/reorder and delete/reorder structural merge slice:
      OxCalc now treats same-parent add/reorder and delete/reorder lane touches as compatible when
      touched/deleted node sets do not overlap and replay validation succeeds. Candidate add over
      live sibling reorder, candidate reorder over live sibling add, candidate delete over live
      sibling reorder, and candidate reorder over live sibling delete all rebase and commit, while
      competing reorder/order edits remain rejected. OxCalc projects ordered parent/child ids on
      node views and DnaTreeCalc uses them for published and candidate child projections, so Skin IR
      tests can verify speculative tree order without host-side reconstruction. Template initial
      content and broader name-collision/what-if UX remain open.
- [x] W4c `scenario-projection` first candidate-backed scenario rail slice:
      DnaTreeCalc projects `WorkspaceState.scenarios` as a host-owned manifest over existing OxCalc
      candidate handles, with closed `CreateScenarioFromCandidate`, `ActivateScenario`, and
      `DeleteScenario` intents. Creating a scenario pins its backing candidate, budget reaping
      preserves pinned scenario candidates, deleting a scenario releases the pin, and programmable
      Skin IR tests prove manifest deltas plus candidate lifecycle interaction from outside the
      skin layer. Scenario override values, scenario-local value epochs, comparative overlays, and
      series projection remain open.
- [x] W4c `scenario-substrate` first typed value override slice:
      Skin IR exposes closed `SetScenarioOverride` and `ClearScenarioOverride` intents over stable
      `NodeKey` targets on candidate-backed scenarios. The host converts supported typed scalar and
      array `NodeValueProjection` payloads to `CalcValue`, asks OxFml to literalize them as authored
      input text, applies the edit through OxCalc candidate-private transactions, and clears by
      restoring the original candidate-private input captured on first override. Programmable Skin
      IR tests prove scalar dependency recalculation, repeated override preserving the original
      clear target, typed array override projection, unsupported value rejection, and override
      cleanup when an overridden candidate-private node is deleted. Comparative overlays,
      formula/rich-value override authoring, and series projection remain open.
- [x] W4c `scenario-projection` freshness and active-node override slice:
      Scenario entries now carry a host-owned `value_epoch` that advances on scenario creation,
      scenario override set/clear, candidate-private edits, and candidate evaluation for the backing
      candidate. Active scenarios project per-node `NodeView.scenario_override` from the stored typed
      override payload without rewriting published `computed_value`. Programmable Skin IR tests
      prove epoch progression, active/inactive override visibility, and array override visibility.
      Direct sweep/goal-seek comparison columns, chart/feed series projection, formula/rich-value
      override authoring, and engine-published scenario revision history remain open.
- [x] W4c `comparative-multi-overlay-projection` first scenario-backed slice:
      `WorkspaceState.comparison` now projects a published basis column and scenario-backed
      comparison columns. Basis values come from published `NodeView.computed_value`; scenario column
      values merge typed scenario override values with the matching candidate projection's typed
      `values_by_key`; unevaluated non-overridden scenario values remain empty. Programmable Skin IR
      tests prove basis/scenario separation, scenario labels/sources, evaluated scenario values, and
      column removal when a scenario is deleted. Direct sweep/goal-seek comparison columns, richer
      value provenance, and engine-published scenario revision history remain open.
- [x] W4c `series-projection` first comparison-backed slice:
      `WorkspaceState.series` now projects chart/feed series for the published basis and
      scenario-backed comparison columns. Points are ordered by workspace `key_order`, labels come
      from current display paths, and values remain typed `NodeValueProjection` payloads. Unevaluated
      scenario series remain empty instead of fabricating values. Programmable Skin IR tests prove
      published basis series, unevaluated scenario series, evaluated scenario series, basis/scenario
      separation, and scenario deletion cleanup. Direct sweep/goal-seek series, richer value
      provenance, and engine-published scenario revision history remain open.
- [x] W4c `series-projection` scoped/unit slice:
      `WorkspaceState::series_for_scope` now expands the existing Skin IR `AuthoringScope` model
      and returns chart/feed series for just that explicit selection. Unit metadata is projected from
      host-owned `series_unit` / `unit` node attributes only when every selected point has the same
      non-empty unit; mixed or missing units remain untyped instead of fabricating a label.
      Programmable Skin IR tests prove selected published series, mixed-unit suppression, selected
      scenario-backed series, and preservation of typed `NodeValueProjection` values.
- [x] W5 early `projection-delta-channel` / `projection-version-stamp` synchronous projection slice:
      `SkinContext` and `ErasedSkinContext` now expose a required `latest_delta:
      ReadSignal<WorkspaceDelta>` beside the full `WorkspaceState` read signal. The host dispatcher
      owns projection publication, stamps each published full snapshot with a monotonic
      `projection_seq`, emits the matching `WorkspaceDelta { from_seq, to_seq, changes }` through
      both the intent receipt and latest-delta signal, and routes accepted no-op plus rejected
      live-host intents through an unchanged delta at the current sequence. Programmable Skin IR and
      walking-skeleton tests prove structural/value deltas, full resets, selection no-ops, rejected
      no-ops, workspace switching, and shell/registry context wiring from outside the skin layer.
      Delta-only resync/replay, worker calculation, virtualization, telemetry, and gap-recovery UI
      policy remain later W5 work.
- [x] W5 early `skinstate-persistence-exercised` slice:
      Skin IR now persists each typed `SkinState` by `(skin_id, slot, workspace_id)`, loads and
      migrates records by `SkinState::schema_version`, runs `gc(live_node_keys)` over stable
      `NodeKey` identities at mount, and saves state after typed handle updates. The framework
      exposes an in-memory store for tests and a native local-file store for desktop hosts; the wasm
      web entrypoint wires a browser `localStorage` backend. Framework tests prove roundtrip,
      migration, slot/workspace isolation, NodeKey GC, and native local-file storage. Walking
      skeleton and programmable Skin IR tests prove the required persisted-state store is threaded
      through real shell/skin mounts without recalculating or adding skin-side semantics. Shared-state
      audit, scenario metadata persistence policy, a11y helpers, and multi-slot composition remain
      later W5 work.
- [x] W5 early `workspace-document-persistence` slice:
      The host now owns a `WorkspaceDocumentStore` seam for `.dnatree` workspace documents plus a
      persisted workspace catalog/active-workspace pointer. `HostDispatcher` autosaves accepted
      intents through that seam, preserving host-owned OxCalc snapshots and selected node state
      without giving skins serialization responsibility. The host exposes in-memory and native
      local-file stores for tests/desktop hosts, and the wasm web entrypoint restores/saves through
      browser `localStorage`. Walking-skeleton tests prove dispatcher autosave, restore through the
      store, and native local-file document storage. Candidate overlays, scenarios, and richer
      workspace metadata persistence remain explicit later-policy work.
- [x] W5 early `design-token-layer` slice:
      Skin IR now carries required `ThemeTokens` on both `SkinContext` and `ErasedSkinContext`,
      with typed `ThemeMode { Light, Dark, HighContrast }` and CSS custom property emission. The
      shell accepts the active token set, injects the corresponding `.dtc-shell` variables, and
      passes the same tokens through every mounted skin context. Built-in shell/skin CSS now consumes
      `var(--dtc-...)` presentation tokens rather than raw stylesheet colors. Framework tests prove
      token emission for light/dark/high-contrast modes, while walking-skeleton and programmable Skin
      IR tests prove token context wiring through real mounts. Runtime theme selection, per-skin
      overrides, locale tokens, and a11y helpers remain later W5 work.
- [x] W5 early `a11y-primitives` first selection-surface slice:
      Skin IR now exposes framework-owned `tree_a11y`, `listbox_a11y`, `table_a11y`,
      `stable_node_dom_id`, selectable item/row ARIA attribute carriers, and roving-tabindex helpers.
      Tree/list/table active descendants are derived from stable `NodeKey` ids rather than display
      paths. TripleEditor and FormulaTree node rails now render tree/treeitem semantics with
      `aria-level`, `aria-posinset`, `aria-setsize`, `aria-selected`, and selection-bound roving
      focus. DependencyInspector uses listbox/option semantics, and OutlineTable publishes row
      selection plus an active descendant over stable node ids. Framework tests prove the helper
      contract; walking-skeleton and programmable Skin IR tests prove the updated real skins still
      mount and dispatch through the host without adding skin-side semantics. Table-cell-grid-specific
      a11y helpers for ValueBoard's `TableCellSelection`, focus-boundary helpers for future
      multi-slot composition, and broader screen-reader/browser audits remain later W5 work.
- [ ] `candidate-overlay-handle`: continue toward fully addressable, layerable, non-publishing
      candidate contexts with broader structural name-collision merge algebra beyond same-node
      rename/move, same-parent rename/add, same-parent rename/reorder, sibling add/delete, sibling
      add/reorder, and sibling delete/reorder facet merging, direct sweep/goal-seek comparison
      columns/series, and broader what-if UX.
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

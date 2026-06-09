# Stack Roadmap Execution Checklist

This checklist keeps iteration work aligned with `ROADMAP.md`. It is not a second roadmap; it is the
short operational cursor used to decide what to do next, what repo owns it, and what proof is needed
before moving on.

## Iteration Contract

Every tranche should answer these before it is committed. The checklist is deliberately about the
roadmap boundary, not general task hygiene: it proves that the work advanced the sequenced stack
requirements and did not move semantics into the host or skin layer.

- [ ] Which `ROADMAP.md` wave and requirement did this advance?
- [ ] Was the readiness tag checked against live code (`expose`, `extend`, or `new`)?
- [ ] Is the implementation in the owning layer?
- [ ] Does the host only project, dispatch, or join typed facts rather than reinterpret semantics?
- [ ] Is there a programmable Skin IR test or real-skin exercise from outside the engine?
- [ ] Were changed specs, handovers, or checklist notes updated?
- [ ] Did the final status name product scope, evidence, still-open gaps, and next roadmap item?

Use this decision rule when picking the next item:

1. Stay on the earliest roadmap wave with open product behavior.
2. Prefer a small engine/language exposure over a skin workaround.
3. If the needed substrate is absent, write a spike or handoff before dependent UI work.
4. Move to later-wave polish only when the earlier open item is either landed or explicitly blocked
   by an owning-layer substrate.

## Active Cursor

Active wave: **W4b - Candidate substrate**.

Feature checkpoint: **usable speculative tree editing** is now landed as a coherent slice. Skins can
open retained non-publishing candidates, edit/add/rename/move/reorder/delete candidate nodes by
stable key, evaluate candidate-only values, inspect ordered candidate structure, rebase common
non-overlapping live/candidate structural edits, receive typed conflicts for ambiguous edits,
commit/discard/reap/pin candidates, project scenario rails and comparison/series values over
candidates, use minimal template-bound initial content, and persist workspace/skin state through
browser `localStorage` or desktop/test stores. Skins can also read a framework-owned command catalog
with selection-, clipboard-, candidate-, scenario-, sweep-, table-, and revision-sensitive
enablement metadata, titles, shortcuts, effective bindings, and disabled reasons. Remaining work
after this checkpoint is larger-scope: richer name-collision merge algebra, goal-seek
columns/series, full template definition/instantiate/sync, formula rewrite/rebind authoring APIs,
and broader real-skin UX polish.

Current objective: continue the addressable, layerable, non-publishing candidate overlay substrate
without letting skins or the host fabricate what-if semantics. The OxCalc spike is complete and the
first non-publishing handle slice is implemented under bead `calc-etez`: candidates open on retained
revisions, accept private node edits, evaluate private values, and discard without advancing live
publication. The first commit bridge is also implemented: candidate state can promote only when the
live revision still equals the candidate basis. The first host/Skin IR projection slice is now
implemented for content-only candidate preview/evaluate, discard, and commit; candidate values
project separately from published workspace values. The first parented copy-layer slice is also
implemented: child candidates open from a parent candidate's private state, project the parent
handle, and keep parent lifecycle guarded while children are retained. Candidate-private revision
history is now projected with real transaction ids and optional invalidation summaries, so commit
receipts can carry the promoted private revision's real transaction id without host fabrication.
Candidate basis revisions are now retained under bounded revision retention while candidate handles
are live, including shared-basis candidates and sibling candidates that survive another candidate's
commit. Candidate-private node structure is now projected through `CandidateProjection.nodes`
separately from published workspace nodes. Stale candidates can now rebase by replaying their
OxCalc-owned private edit log onto the current workspace revision through the closed
`RebaseCandidate` intent; parented candidates flatten their captured layered private edits during
rebase, and rebased candidates do not project stale values before explicit candidate evaluation.
Optimized live layering/merge semantics remain open. The first W2 `command-palette-metadata`
slice is now also landed in the Skin IR/framework layer: `WorkspaceState::command_catalog` derives
typed command affordances from projected host truth rather than skin-side legality checks.
OxCalc owns candidate state, publication/discard, overlay provenance, and value epochs; the
DnaTreeCalc host exposes only typed handles, projections, and closed intents.

Context: first W3 assessment is complete. The formula rewrite verbs
(`replicate-by-id`, `f4-toggle-binding`, and broader `reference-insertion`) require OxFml-owned
authoring APIs and are recorded in `../../handovers/HANDOVER_OXFML_formula_authoring_verbs.md`.
The first formula-authoring slice is now landed for point-mode node reference insertion:
OxFml composes host reference text from typed targets and the TreeCalc host-reference syntax profile,
including bracket escaping and profile-selected selector tokens, while DnaTreeCalc exposes a closed
`InsertFormulaReference` Skin IR intent that carries node keys, edit-buffer text, replacement span,
and typed target. The host calls OxFml, dry-binds through OxCalc, and commits through a real content
transaction. The second insertion tranche is now also landed for typed collection and structural
selector targets: programmable Skin IR tests insert `Base.@CHILDREN` and `A.@NEXT` without host-side
formula spelling and verify OxCalc dependencies and values. The third insertion tranche is also
landed: successful receipts append a typed `FormulaReferenceInserted` delta with OxFml-composed
inserted text, updated formula text, applied span, target, and edited node key. Full replicate/fill
by id, F4 binding toggle, formula paste/rebind, formula-and-format paste, subtree internal-reference
rebind, and richer editor UX around selector choice remain open.
The first landed W3 write slice is `format-write` for authored number formats via canonical meta nodes and
real OxCalc transactions. The second landed W3 slice is `note-write` via canonical `Note` meta nodes
and `NodeView.note` projection. The `SetMeta` half of `meta-and-attribute-write` is landed through
an OxCalc-owned revisioned meta-membership edit. The `SetNodeAttributes` half is landed for the
current Skin IR surface as a revisioned host-owned string attribute bag stored in canonical meta
nodes and projected through `NodeView.attributes`. The first `add-node-content-policy` widening is
landed for literal formula initial content: OxCalc dry-binds prospective new-node formulas without
mutation, and DnaTreeCalc previews/rejects invalid literal formulas before add-node commit.
The second `add-node-content-policy` widening is landed for
`InheritColumnFormula { table, column_id }`: table-column formula metadata can seed a new node when
OxCalc dry-binds it in the prospective node context; row-context/table-only formulas reject before
mutation; constant columns reject with typed table-column errors.
The first `clipboard-transfer-model` tranche is landed: `CopyToClipboard` populates a host-owned,
typed `WorkspaceState.clipboard` carrier for values, formula source, formats, and subtrees from
`AuthoringScope`, with a `ClipboardChanged` projection delta. Paste/cut, OS clipboard integration,
formula rewrite, and subtree rebind remain separate work.
The first `paste-special` slice is landed for format payloads: `PasteClipboardFormat` consumes one
copied format carrier and applies it through the existing canonical number-format transaction path.
Value paste, formula paste, OS clipboard integration, and subtree paste/rebind remain open.
The second `clipboard-transfer-model` tranche is landed: `CutToClipboard` records
`ClipboardOperationProjection::Cut` on the host-owned clipboard carrier without deleting nodes or
advancing model revisions.
The second `paste-special` slice is landed for constant-source values:
`PasteClipboardValues` consumes a single value clipboard carrier only when it has authored constant
input text, applies it through the existing scoped content transaction path, and rejects computed
formula results, arrays, formula paste, OS clipboard transfer, formula/subtree source deletion, and
subtree rebind. The first cut/paste commit slice is also landed for constant values:
successful `CutToClipboard(Values)` followed by `PasteClipboardValues` applies the target write and
source clear in one OxCalc transaction, then clears the host clipboard; rejected paste attempts leave
source and clipboard intact. The remaining paste-special semantics are recorded in
`../../handovers/HANDOVER_OXFML_paste_special_authoring.md`.
The next constant-value paste-special slice is also landed: multiple authored-constant value carriers
paste one-to-one over an explicitly ordered node target scope in one OxCalc transaction, and
multi-source cut paste clears copied sources not included in the target scope before clearing the
host clipboard. The first computed-value paste-special slice is also landed for scalar cell values
and supported array constants: OxFml literalizes typed `CalcValue` payloads into authored input
text, DnaTreeCalc projects that literalization through the Skin IR, and `PasteClipboardValues`
consumes it through the existing scoped content transaction path. Formula paste,
formula/subtree source deletion, formula-and-format paste, and subtree rebind remain open.
The W3 `set-membership-write` assessment is complete and recorded in
`../../handovers/HANDOVER_OXCALC_set_membership_write.md`: current collection membership/order is an
OxCalc-published dependency fact. OxCalc now has a first transaction edit slice for typed
owner/source-handle/member validation and derived-collection rejection, but not yet positive
authored membership/order mutation. A follow-up positive-substrate spike ruled out
descriptor-only overrides for `ReferenceLiteralArrayV1`, because runtime evaluation remains tied to
the original OxFml-authored source and would diverge from edited descriptor truth.
The first system-clipboard interchange slice is landed without giving the host OS clipboard
authority: typed clipboard carriers project optional plain text for platform export, and
`PasteExternalClipboardText` accepts platform-supplied clipboard text as authored content.
The second system-clipboard interchange slice is also landed for multi-item plain text:
TSV/newline text supplied by platform clipboard code is flattened row-major and applied one-to-one
over an explicitly ordered multi-node target scope in one OxCalc transaction when counts match;
single-node paste preserves raw text, including newlines; single text items keep the existing
broadcast behavior; and count mismatches reject before mutation.
The first `duplicate-subtree` slice is now landed for formula-free ordinary subtrees:
`DuplicateSubtree { source, destination_parent, new_symbol }` creates cloned nodes through one
OxCalc edit transaction with reserved engine node ids, projects the normal structural delta, and
rejects formula-bearing subtrees before mutation because internal-reference rebind remains
OxFml-owned and unavailable. The second slice preserves host-authored local notes, number formats,
and attributes by creating the same canonical meta nodes under cloned nodes in that transaction.
The third slice is now landed for hidden non-canonical formula-free meta descendants: duplicate
subtree preserves those custom hidden branches through the same OxCalc transaction while keeping
them out of ordinary projected node lists. Table subtree cloning, table-backed meta
descendants, formula-bearing meta descendants, and formula rebind remain open. The fourth slice is
also landed for constant-only table snapshots: duplicate subtree creates a fresh table id/name/path
identity, clones generated table cell meta nodes, and sets the cloned OxCalc table snapshot to those
new generated node ids in one transaction. Formula-backed table columns, totals formulas,
formula-bearing table cell nodes, and formula-visible table-name collisions remain rejected before
mutation until table formula rebind support exists.

Do not advance to W4b/W4c speculation or W5 platform polish as the default next step while the W4a
revision substrate still lacks bounded retention/persistence and invalidation summaries.

## Per-Iteration Gate

Before implementation, write the tranche in this form:

| Field | Required answer |
|---|---|
| Roadmap wave | The exact `ROADMAP.md` wave and requirement id. |
| User-visible capability | What a skin or host can now do that it could not do before. |
| Owning truth | Which layer owns the semantic fact or mutation: OxFml, OxCalc, DnaTreeCalc host, or skin. |
| Readiness result | `expose`, `extend`, or `new substrate`, corrected against live code. |
| Seam change | Projection field, closed intent, receipt shape, Skin IR shape, or handoff doc. |
| Evidence | Engine test plus programmable Skin IR or real-skin exercise, unless the tranche is a spike only. |
| Exclusions | Concrete unsupported cases and the next blocked or unblocked requirement. |

After implementation, close the tranche only when the evidence row is true and the owning-truth row
has not drifted. If the owning layer lacks the needed API, record a handoff and move only to the next
roadmap item that can be implemented without fabricating semantics host-side.

## Iteration-To-Roadmap Checklist

Use this as the per-tranche goal statement before implementation:

- [ ] `Roadmap item`: name the exact `ROADMAP.md` requirement and wave.
- [ ] `Readiness`: verify live code confirms `expose`, `extend`, or `new`; correct the roadmap note
      if reality differs.
- [ ] `Owning repo`: implement in OxFml, OxCalc, DnaTreeCalc host, or skin layer according to the
      ownership boundary.
- [ ] `Seam`: thread the result through projection, intent receipt, or Skin IR without skin-side
      semantic reconstruction.
- [ ] `Evidence`: add or update programmable Skin IR tests or real-skin checks that exercise the
      capability from outside the engine.
- [ ] `Scope`: record exactly what now works, what remains open, and which gated workstream blocks
      the next dependent feature.
- [ ] `Commit`: commit affected repos at the end of the tranche so the next iteration starts from a
      clean boundary.

## W0 / W1 Baseline Already Available

- [x] Stable `NodeKey` transition spine carried through projection.
- [x] Typed invalidation reasons, dependency kinds, run state, calc state, phase timings, and richer
      value projection.
- [x] Reference-resolution map plus reverse lookup.
- [x] Binding diagnostics, effective formatting, runtime effects, overlay detail, derivation trace
      payloads, and active-node detail.
- [x] Per-node published-value epochs.

## W2 Execution Order

- [x] Typed intent errors replace stringly structural rejection receipts for the implemented
      host/session paths.
- [x] `AuthoringScope` models node, ordered multi-node, subtree, and reference-collection subjects
      with host projection expansion.
- [x] Edit transaction ids flow through node structural receipts for current node-level operations.
- [x] OxCalc `transaction-scope` spike has a go decision and first node-edit slice.
- [x] OxCalc committed-graph recalc-plan preview supports node-level preview mutations.
- [x] Node formula edit preview joins OxFml dry-bind with OxCalc invalidation planning.
- [x] Table body/totals formula previews dry-bind through OxCalc table formula context.
- [x] New table formula-column preflight dry-binds and plans table snapshot invalidation.
- [x] Scoped content edit preview expands `AuthoringScope`, dry-binds each target, and plans combined
      invalidation.
- [x] Rename preview joins collision legality with OxCalc structural invalidation planning.
- [x] Move/drop preview joins drop validity, collision legality, and OxCalc structural invalidation
      planning.
- [x] Delete/orphan structural preview reports outside dependents and invalidation impact without
      mutation.
- [x] Add/default-content policy preview is typed and tested for empty/literal content, meta-node
      flagging, name-collision blocking, and typed unsupported blockers for inherited/template
      policies.
- [x] Broader table row/column structural preview breadth is typed and tested for add, delete,
      rename, and reorder operations with table-collision / duplicate-input blockers and OxCalc
      table-snapshot invalidation planning.
- [x] Remaining multi-target/table transaction ids are backed by OxCalc transaction operation
      coverage rather than host batching.
  - [x] Table snapshot operations with existing node ids route through OxCalc transaction outcomes:
        row delete/rename/reorder, formula-column add/edit/delete, totals/header
        visibility/formula edits, and column delete/rename/reorder.
  - [x] Generated-node table operations (`AddTableRow`, constant `AddTableColumn`) use OxCalc
        reserved node ids so one transaction can add generated cell nodes and publish the table
        snapshot that references them.
  - [x] Scoped existing-node content edits carry `AuthoringScope` through Skin IR and route through
        one OxCalc batch edit transaction after host-owned projection expansion.
  - [x] Other scoped multi-target authoring verbs remain parked with their owning W3 authoring verbs.
- [x] W2 closure review confirms no skin parses formulas, computes semantic values, or fabricates
      engine facts.

## W3 Execution Order

- [x] Assess W3 authoring verbs against live OxFml/OxCalc APIs:
      `replicate-by-id`, `f4-toggle-binding`, `reference-insertion`, `clipboard-transfer-model`,
      `paste-special`, `duplicate-subtree`, `set-membership-write`, `meta-and-attribute-write`,
      `note-write`, `format-write`, and `add-node-content-policy` widening.
- [x] File OxFml handoff for W3 formula rewrite/composition verbs that cannot be implemented
      ownership-correctly in DnaTreeCalc with the current editor facade.
- [x] Land first `reference-insertion` formula-authoring tranche:
      OxFml exposes a typed editor host-reference insertion operation for host names,
      host-reference collections, and host structural selectors, composes source text from the
      host-reference syntax profile, bracket-escapes host names, fixes editor incremental parsing to
      use the same host-reference syntax profile as binding, and tests `Base.@PARENT`,
      `Base.@CHILDREN`, `Base.*`, and bracketed names. DnaTreeCalc exposes
      `WorkspaceIntent::InsertFormulaReference` with node key, current formula text, replacement
      span, and typed target; the host maps keys to projected facts, calls OxFml, dry-binds through
      OxCalc, and commits through a real transaction. Programmable Skin IR tests prove point-mode
      node insertion updates formula text, dependencies, and calculated value from outside the
      engine.
- [x] Widen `reference-insertion` Skin IR proof to collection and structural selector targets:
      programmable Skin IR tests insert typed `HostReferenceCollection` and `HostStructuralSelector`
      targets, producing `=SUM(Base.@CHILDREN)` and `=SUM(A.@NEXT)` through OxFml composition and
      proving OxCalc dependency/value resolution from outside the engine.
- [x] Add typed authored-output receipt projection for `reference-insertion`:
      `WorkspaceDeltaChange::FormulaReferenceInserted` carries the edited node key, typed target,
      inserted text, updated formula text, and applied span returned by the OxFml authoring path.
      Programmable Skin IR tests assert this receipt delta for node, collection, and structural
      selector insertion.
- [x] Land first ownership-correct `format-write` tranche:
      `WorkspaceIntent::SetNumberFormat { scope, number_format_code }` creates, updates, or clears
      `Format.NumberFormat` meta nodes through OxCalc transactions; receipts carry transaction ids;
      Skin IR tests assert set, clear, multi-node scope, and reserved-path rejection.
- [x] Land `note-write`:
      `WorkspaceIntent::SetNote { node, note }` creates, updates, or clears a `Note` meta node
      through OxCalc transactions; `NodeView.note` and active-node detail project it; tests assert
      set, clear, document round-trip, and reserved-path rejection.
- [x] Land the `SetMeta` half of `meta-and-attribute-write`:
      OxCalc exposes `OxCalcTreeEdit::SetNodeMeta`, meta membership enters namespace/workspace
      revision identity, DnaTreeCalc exposes `WorkspaceIntent::SetMeta`, and tests assert
      transaction receipts, revision movement, projected `is_meta`, retained addressability, and
      formula invisibility.
- [x] Land the `SetNodeAttributes` half of `meta-and-attribute-write` for the current Skin IR
      surface:
      `WorkspaceIntent::SetNodeAttributes { node, attrs }` patches path-safe string attributes via
      canonical `Attributes.<key>` meta nodes, projects `NodeView.attributes` and active-node
      attributes, carries transaction receipts, and rejects invalid keys / reserved non-meta paths
      with typed errors.
- [x] Land first `add-node-content-policy` widening:
      `InitialNodeContentProjection::Literal { content }` formula text is dry-bound by OxCalc in a
      prospective new-node context without workspace mutation; add-node preview carries typed
      syntax/bind/profile blockers; add-node commit rejects invalid literal formulas before
      mutation. Empty, literal constants, and `is_meta` remain supported.
- [x] Land second `add-node-content-policy` widening:
      `InitialNodeContentProjection::InheritColumnFormula { table, column_id }` reads the source
      formula from host-owned table column metadata, asks OxCalc to dry-bind that formula in the
      prospective new-node context, and commits only formulas that bind as ordinary node formulas.
      Row-context/table-only formulas reject with bind diagnostics before mutation; constant columns
      reject with typed table-column errors.
- [x] Land third `add-node-content-policy` widening:
      `InitialNodeContentProjection::TemplateBound { template_id }` resolves built-in host template
      ids (`starter`, `input-zero`) to ordinary initial content before OxCalc dry-bind/recalc for
      preview, published add-node, and candidate add-node paths. Unknown template ids remain typed
      unsupported initial content. `WorkspaceState.templates` now projects the same built-in host
      template catalog with names, descriptions, preview content, and the exact typed initial
      content payload skins should dispatch. Full template definition/edit/instantiate/sync remains
      future template subsystem work.
- [x] Land first `clipboard-transfer-model` tranche:
      `WorkspaceIntent::CopyToClipboard { scope, payload }` builds a typed host-owned carrier for
      `Values`, `Formula`, `Format`, and `Subtree` payloads from projected state and emits
      `WorkspaceDeltaChange::ClipboardChanged`. This is a transfer artifact only; paste-special,
      cut/delete coupling, OS clipboard export/import, formula rewrite, and subtree rebind remain
      open.
- [x] Land first `paste-special` tranche:
      `WorkspaceIntent::PasteClipboardFormat { target }` accepts a single copied `Format` carrier and
      applies its `number_format_code` to a target `AuthoringScope` through the existing
      `set_number_format_transaction` path. Pasting an unformatted source clears the target format.
      Value paste, formula paste, OS clipboard integration, and subtree paste/rebind remain open.
- [x] Land second `clipboard-transfer-model` tranche:
      `WorkspaceIntent::CutToClipboard { scope, payload }` records a `Cut` operation on the same typed
      host-owned clipboard carrier as copy. It intentionally does not delete source nodes or advance
      the workspace revision; later paste/commit semantics own any model mutation.
- [x] Land second `paste-special` tranche:
      `WorkspaceIntent::PasteClipboardValues { target }` extends the value clipboard carrier with
      source `content_kind` and optional `constant_input_text`, then pastes exactly one authored
      constant source through the scoped content transaction path with a real transaction id.
      Computed formula values, arrays, formula paste, OS clipboard transfer, and subtree rebind
      remain open.
- [x] Land first cut/paste commit tranche for constant values:
      a successful cut-value paste clears the source in the same OxCalc transaction as the target
      write and clears `WorkspaceState.clipboard`; rejected paste attempts preserve both source and
      clipboard. Formula/subtree source deletion remains open until formula rebind support exists.
- [x] Land next constant-value paste-special tranche:
      multi-source `Values` carriers paste one-to-one only when every source has authored constant
      input text and the target is an explicitly ordered node scope of the same length. Copy paste
      preserves the host clipboard; cut paste clears copied sources outside the target scope and then
      clears the host clipboard. Computed value literalization is covered by the following tranche;
      formula paste, formula/subtree source deletion, and subtree rebind remain open.
- [x] Land first computed-value paste-special tranche:
      OxFml now exposes `CalcValue` authoring literalization for blank, finite number, text,
      logical, worksheet-error, and supported array values, with typed unsupported verdicts for
      references, missing/non-finite values, rich/callable values, nested arrays, empty array cells,
      and quote-containing array text in this slice. DnaTreeCalc projects the literalized authored
      input through `NodeView.literalized_value_input`, value clipboard carriers include it, and
      `PasteClipboardValues` consumes it without using rendered display text. Authored constants
      remain preferred over computed literalization. Formula rebind/paste, formula-and-format paste,
      subtree rebind, and formula/subtree cut source deletion remain open.
- [x] Fix typed array value retention after recalculation:
      OxCalc edge-cache hits now reuse the retained typed published `CalcValue` when the cached
      display payload matches the same node's published display text, avoiding display-string
      reparsing for array values. DnaTreeCalc projection now prefers `last_outcome.published_calc_values`
      over literal-input node-view values, so explicit recalc retains array values as typed Skin IR
      arrays and can now project OxFml-authored array constant literal input for supported arrays.
- [x] File OxFml handoff for the remaining W3 paste-special APIs:
      computed value literalization, formula rebind, formula-and-format paste, and subtree
      internal-reference rebind support. Computed value literalization is now satisfied for scalar
      cells and supported array constants; formula and subtree rebind operations remain open.
- [x] Assess `set-membership-write` and file OxCalc handoff:
      `TreeReferenceCollectionDependency` facts carry handles, members, and membership/order versions
      for projection and `AuthoringScope::Collection` expansion. OxCalc now exposes a first
      `SetReferenceCollectionMembership` validation/rejection edit surface, but current derived
      collections remain non-editable and there is no authored collection-membership/order store.
      `SetCollectionMembership` remains unsupported until OxCalc provides that positive transaction
      substrate.
- [x] Spike the first positive `set-membership-write` substrate:
      A descriptor-only `ReferenceLiteralArrayV1` override is rejected as insufficient. It can change
      projected membership/order descriptors, but not the value computed from the OxFml-authored
      formula source. The next implementation path needs OxFml rewrite/bound invocation support or
      an equivalent evaluated-collection replacement seam owned by the engine boundary.
- [x] Land first system-clipboard interchange tranche:
      `ClipboardProjection.plain_text` exports deterministic text for supported typed clipboard
      payloads (`Values` as scalar text or array TSV, `Formula` as authored formula text), and
      `PasteExternalClipboardText { target, text }` lets platform clipboard code provide text for the
      existing authored-content transaction path. Browser/desktop clipboard APIs stay outside the
      host; rich OS clipboard formats remain open.
- [x] Land second system-clipboard interchange tranche:
      `PasteExternalClipboardText { target, text }` now treats TSV/newline platform text with
      multiple cells as row-major authored content for an explicitly ordered node target scope,
      applies matching cells in one OxCalc transaction, preserves single-node raw-text paste and
      single-cell broadcast behavior, and rejects item-count mismatches before mutation. Rich OS
      clipboard formats, formula rewrite paste, and subtree paste/rebind remain open.
- [x] Land first `duplicate-subtree` tranche:
      `WorkspaceIntent::DuplicateSubtree { source, destination_parent, new_symbol }` duplicates
      formula-free ordinary subtrees through one OxCalc transaction with reserved node ids and a
      normal structural delta. Formula-bearing subtrees reject with typed
      `DuplicateSubtreeUnsupported` rather than copying stale formula text; formula rebind,
      formula/subtree cut source deletion, table subtree cloning, and meta-subtree breadth remain
      open.
- [x] Land second `duplicate-subtree` tranche:
      formula-free subtree duplication now preserves host-authored local `Note`, `Format.NumberFormat`,
      and `Attributes.<key>` metadata by creating canonical meta nodes under each cloned node in
      the same OxCalc transaction. Inherited/effective format is not converted into local authored
      metadata; arbitrary meta-subtree breadth, table subtree cloning, and formula rebind remain
      open.
- [x] Land third `duplicate-subtree` tranche:
      formula-free subtree duplication also preserves hidden non-canonical formula-free meta
      descendants without projecting those hidden nodes into ordinary `WorkspaceState` lists.
      Formula-bearing meta descendants, table-backed meta descendants, table subtree cloning, and
      formula rebind remain open.
- [x] Land fourth `duplicate-subtree` tranche:
      constant-only table snapshots are cloned through the same transaction as the duplicated table
      node. The clone receives a fresh table id/name/path identity, generated table cell meta nodes
      are recreated under the cloned table node, and the cloned OxCalc table snapshot points at the
      cloned generated node ids. Formula-backed table columns, totals formulas, formula-bearing
      table cell nodes, formula-visible table-name collisions, and formula rebind remain open.
- [ ] Continue W3 with the next feasible tranche: move to the next OxFml-backed formula authoring
      verb (`f4-toggle-binding`, `replicate-by-id`, formula paste/rebind) when its rewrite semantics
      are available, or record a focused blocker if the current editor surface cannot support it.
      Complete remaining `add-node-content-policy` only when template substrate exists.

## Gated Workstreams

- [x] `transaction-scope`: current W2 node, table snapshot, generated-node table add, and
      existing-node scoped content receipts carry real OxCalc transaction ids. Remaining scoped
      multi-target authoring verbs belong to W3 command expansion rather than W2 closure.
- [x] `revision-graph-retention`: first OxCalc substrate slice landed in
      `0735d9c Retain workspace revision lineage`: in-memory parent-linked revision graph,
      workspace-view retained entries/current parent, and transaction predecessor/successor ids.
      Follow-on OxCalc slice `8bc6283 Navigate retained workspace revisions` adds in-memory
      navigation to retained revisions and restores OxCalc-owned structural/input/namespace,
      table, publication, runtime-overlay, value-epoch, and diagnostic state. Current DnaTreeCalc
      slice projects retained revision history into `WorkspaceState` and routes `NavigateRevision`
      through the host dispatcher to OxCalc without inverse replay. Follow-on DnaTreeCalc command
      routing adds `WorkspaceIntent::Undo` and `WorkspaceIntent::Redo` over host-owned cursor stacks:
      successful edit transactions record previous revisions as undo boundaries, redo clears on
      branch edits, and undo/redo republish the OxCalc-restored snapshot while restoring host
      selection. Current OxCalc/DnaTreeCalc transaction-summary slice retains transaction id,
      invalidated node ids, rebind flags, typed invalidation reasons, and estimated invalidated-node
      count on successor revision entries, then projects those facts through Skin IR revision
      history keyed by `NodeKey`. Bounded retention is now OxCalc-owned and deterministic:
      `OxCalcTreeRevisionRetentionPolicy` caps retained in-memory revisions with oldest-first
      eviction while preserving the current revision. Persistence policy is explicit: workspace
      snapshots preserve the active revision/layer state, not the navigable retained history DAG.
      Scoped W4a is closed for the in-memory undo/history/time-scrub substrate.
- [x] `candidate-overlay-handle` spike: OxCalc recorded
      `CORE_ENGINE_CANDIDATE_OVERLAY_HANDLE_SPIKE.md`, confirming W4b is schedulable only as a new
      addressable candidate-context substrate. Current runtime overlays are published-basis state
      and current candidate results are one-run publish/reject packets, not scenario handles.
- [x] `candidate-overlay-handle` first OxCalc substrate slice: opaque candidate handles can be
      opened on retained revisions, privately edited, evaluated, and discarded while the live
      workspace revision, publication snapshot, runtime overlay set, visible value, and published
      value epoch remain unchanged. Follow-up commit/layering/projection work is tracked in OxCalc
      bead `calc-4ipg`.
- [x] `candidate-overlay-handle` first commit bridge slice: OxCalc commits candidate-private
      evaluated state into the live workspace only when the candidate basis revision is still
      current, and returns typed stale-basis rejection otherwise.
- [x] `candidate-overlay-handle` first DnaTreeCalc host/Skin IR projection slice: closed candidate
      lifecycle intents project candidate values separately from published node state and commit
      through OxCalc. Programmable Skin IR tests cover private candidate evaluation, commit, discard,
      and stale-basis rejection.
- [x] `candidate-overlay-handle` first parented copy-layer slice: OxCalc child candidates open from a
      parent candidate's private state at child-open time; DnaTreeCalc projects `parent_handle` and
      programmable Skin IR tests cover layered child values, parent retained-child rejection, and
      child commit.
- [x] `candidate-overlay-handle` candidate-private revision history slice: OxCalc revision graph
      entries carry real transaction ids separately from optional invalidation summaries, candidate
      views/commit outcomes expose private graph entries, and DnaTreeCalc projects
      `CandidateProjection.revision_history`. Programmable Skin IR tests prove candidate edit/evaluate
      remain non-publishing while commit receipts use a real promoted-revision transaction id.
- [x] `candidate-overlay-handle` candidate basis-retention pin slice: OxCalc keeps candidate basis
      revisions retained while candidate handles are live, counts shared-basis pins, and preserves
      sibling candidate pins when another candidate commits. Focused OxCalc tests prove pinned bases
      remain navigable under bounded retention and become evictable after the last candidate releases
      them.
- [x] `candidate-overlay-handle` candidate structural projection/read slice: OxCalc candidate views
      expose candidate-private node views after private structural edits, and DnaTreeCalc projects
      them through `CandidateProjection.nodes` without rewriting published workspace nodes.
      Focused OxCalc and programmable Skin IR tests prove the projection remains non-publishing
      until commit.
- [x] `candidate-overlay-handle` first closed structural candidate mutation intent: Skin IR exposes
      candidate rename by stable `NodeKey`, the host dispatches it through OxCalc's private candidate
      edit transaction, and programmable Skin IR tests prove candidate-private structure changes
      without publishing until commit.
- [x] `candidate-overlay-handle` move/delete structural candidate mutation intents: Skin IR exposes
      candidate move and delete by stable `NodeKey`, the host dispatches them through OxCalc's
      private candidate edit transaction, and programmable Skin IR tests prove candidate-private
      structure changes without publishing until commit.
- [x] `candidate-overlay-handle` constant/empty candidate add-node intent: Skin IR exposes candidate
      add by stable parent `NodeKey`, the host reserves the node id and dispatches the add through
      OxCalc's private candidate edit transaction, and programmable Skin IR tests prove
      candidate-private structure changes without publishing until commit.
- [x] `candidate-overlay-handle` formula-literal candidate add-node dry-bind: OxCalc exposes
      candidate-context dry-bind for prospective new nodes, DnaTreeCalc uses it for formula literal
      initial content, and programmable Skin IR tests prove a candidate-added formula can bind
      against candidate-private structure.
- [x] `candidate-overlay-handle` candidate run projection for candidate-only nodes:
      DnaTreeCalc resolves candidate-private tree ids from `OxCalcTreeCandidateView.nodes` while
      projecting candidate calculation runs, and programmable Skin IR tests prove a candidate-added
      formula node appears in candidate `run.evaluation_order` without publishing to
      `WorkspaceState.nodes`.
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
      publishing live state.
- [x] `candidate-overlay-handle` first speculation-budget/GC slice:
      OxCalc computes typed candidate pressure for a retention policy, including retained,
      protected, reclaimable, and over-budget candidate counts, and reaps unprotected candidates to
      a requested budget. DnaTreeCalc projects the pressure through `WorkspaceState` and exposes a
      closed `ReapCandidates` Skin IR intent; programmable Skin IR tests prove pressure changes and
      candidate removal deltas from outside the engine.
- [x] `candidate-overlay-handle` host-retention pin slice:
      OxCalc exposes explicit candidate retention pins, reports child-protected and host-pinned
      pressure reason counts, and protects host-pinned candidates from budget reaping. DnaTreeCalc
      projects candidate pin counts and exposes closed pin/unpin intents; programmable Skin IR tests
      prove pinned candidates survive reaping, pin/unpin updates project through candidate-change
      deltas, and unbalanced unpin rejects.
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
      OxCalc refreshes parented candidates from parent-private edits made after child open, preserving
      non-publishing semantics while exposing updated candidate values through the existing
      projection. OxCalc also rejects stale candidate rebase with a typed conflict report when live
      and candidate private edits overlap on the same stable tree node; non-overlapping stale
      candidates still rebase. DnaTreeCalc maps the engine report to typed Skin IR
      `CandidateRebaseConflict` with stable `NodeKey` overlaps. Focused OxCalc candidate tests and
      programmable Skin IR tests prove same-node conflict rejection, non-overlap rebase, parented
      flattening, and live child refresh.
- [x] `candidate-overlay-handle` add-node parent/order conflict slice:
      OxCalc marks candidate `AddNode` edits as touching their parent lane and marks candidate
      `MoveNode` edits as touching their destination parent lane. Stale candidate rebase now rejects
      with typed `CandidateRebaseConflict` when the live workspace changes the same parent
      structure/order before rebase. DnaTreeCalc maps the parent overlap to stable Skin IR `NodeKey`
      conflict payloads. Focused OxCalc candidate tests and programmable Skin IR tests prove
      candidate-add versus live sibling-add conflict without publishing the candidate node.
- [x] `candidate-overlay-handle` old-parent/delete-descendant conflict slice:
      OxCalc derives candidate rebase touch sets against the retained basis structural snapshot,
      marking move source and destination parent lanes, delete subtrees plus parent lanes, and
      explicit reorder parent lanes. DnaTreeCalc continues to map typed conflict reports to stable
      Skin IR `NodeKey` overlaps without owning merge semantics. Focused OxCalc candidate tests prove
      old-parent move conflict, delete-descendant conflict, and explicit reorder parent-lane
      conflict; programmable Skin IR tests prove old-parent move and delete-descendant conflict from
      outside the engine.
- [x] `candidate-overlay-handle` lane-aware rebase merge slice:
      OxCalc classifies rebase touches into content nodes, structural parent/order lanes,
      structural node edits, and deleted nodes instead of using one coarse overlap set. Candidate
      structural add now rebases over a live content edit on the same parent, and candidate
      rename/move now rebase over live content edits on the affected node, while same-node content,
      parent/order, move, and delete conflicts remain rejected. Focused OxCalc tests prove the
      positive merge and existing conflict cases; programmable Skin IR tests prove accepted
      rebase/commit paths through the host projection.
- [x] `candidate-overlay-handle` multi-edit structural/content rebase slice:
      A stale candidate containing multiple private structural edits now has direct evidence for
      compatible rebase: candidate rename, move, and add replay together over live content edits on
      the affected nodes/parent, keep candidate-only structure unpublished until commit, and then
      promote the merged structure and values through the normal commit bridge. Focused OxCalc and
      programmable Skin IR tests prove this from the engine and host seams. Full competing
      structural merge algebra remains open.
- [x] `candidate-overlay-handle` same-node rename/move structural facet merge slice:
      Candidate rename over live move and candidate move over live rename on the same stable node
      now rebase through OxCalc typed structural lane touches when replay validation succeeds.
      Competing same-node rename-vs-rename remains rejected as a typed conflict. Focused OxCalc and
      programmable Skin IR tests prove both accepted rebase/commit paths and the rejected competing
      rename case.
- [x] `candidate-overlay-handle` same-parent rename/add namespace merge slice:
      Candidate rename over live sibling add now rebases through OxCalc typed structural lane touches
      when the final namespace is legal. Duplicate-name replay validation is reported as typed
      `CandidateRebaseConflict`, not generic structural failure. Focused OxCalc and programmable
      Skin IR tests prove the accepted non-collision and rejected collision paths.
- [x] `candidate-overlay-handle` same-parent rename/reorder structural facet merge slice:
      Candidate rename over live sibling reorder and candidate reorder over live sibling rename now
      rebase through OxCalc typed structural lane touches when replay validation succeeds. Skin IR
      exposes a closed `ReorderCandidateNode` intent by stable `NodeKey`. Focused OxCalc and
      programmable Skin IR tests prove both accepted rebase/commit paths.
- [x] `candidate-overlay-handle` sibling add/delete structural merge slice:
      Candidate add over live sibling delete and candidate delete over live sibling add now rebase
      through OxCalc typed structural lane touches when deleted/touched nodes do not overlap and
      replay validation succeeds. Existing delete-descendant conflict coverage keeps destructive
      overlap conservative. Focused OxCalc and programmable Skin IR tests prove both accepted paths.
- [x] `candidate-overlay-handle` sibling add/reorder and delete/reorder structural merge slice:
      Candidate add over live sibling reorder, candidate reorder over live sibling add, candidate
      delete over live sibling reorder, and candidate reorder over live sibling delete now rebase
      through OxCalc typed structural lane touches when deleted/touched nodes do not overlap and
      replay validation succeeds. Competing reorder/order edits remain conservative conflicts.
      OxCalc node views publish ordered parent/child ids and DnaTreeCalc consumes them for published
      and candidate child projections, so programmable Skin IR tests verify speculative tree order
      from outside the engine.
- [x] W4c `scenario-projection` first candidate-backed scenario rail slice:
      DnaTreeCalc projects `WorkspaceState.scenarios` as a host-owned manifest over existing OxCalc
      candidate handles and exposes closed create/activate/delete scenario intents. Creating a
      scenario pins its backing candidate, budget reaping preserves pinned scenario candidates,
      deleting a scenario releases the pin, and programmable Skin IR tests prove manifest deltas and
      candidate lifecycle interaction from outside the skin layer. Scenario override values,
      scenario-local value epochs, comparative overlays, and series projection remain open.
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
      Goal-seek comparison columns, chart/feed series projection, formula/rich-value
      override authoring, and engine-published scenario revision history remain open.
- [x] W4c `comparative-multi-overlay-projection` first scenario-backed slice:
      `WorkspaceState.comparison` now projects a published basis column and scenario-backed
      comparison columns. Basis values come from published `NodeView.computed_value`; scenario column
      values merge typed scenario override values with the matching candidate projection's typed
      `values_by_key`; unevaluated non-overridden scenario values remain empty. Programmable Skin IR
      tests prove basis/scenario separation, scenario labels/sources, evaluated scenario values, and
      column removal when a scenario is deleted. Goal-seek comparison columns, richer
      value provenance, and engine-published scenario revision history remain open.
- [x] W4c `series-projection` first comparison-backed slice:
      `WorkspaceState.series` now projects chart/feed series for the published basis and
      scenario-backed comparison columns. Points are ordered by workspace `key_order`, labels come
      from current display paths, and values remain typed `NodeValueProjection` payloads. Unevaluated
      scenario series remain empty instead of fabricating values. Programmable Skin IR tests prove
      published basis series, unevaluated scenario series, evaluated scenario series, basis/scenario
      separation, and scenario deletion cleanup. Goal-seek series, richer value
      provenance, and engine-published scenario revision history remain open.
- [x] W4c `series-projection` scoped/unit slice:
      `WorkspaceState::series_for_scope` now expands the existing Skin IR `AuthoringScope` model
      and returns chart/feed series for just that selection. Unit metadata is projected from
      host-owned `series_unit` / `unit` node attributes only when every selected point has the same
      non-empty unit; mixed or missing units remain untyped. Programmable Skin IR tests prove
      selected published series, mixed-unit suppression, selected scenario-backed series, and typed
      `NodeValueProjection` value preservation.
- [x] W4c `direct-sensitivity-sweep` first scenario-backed slice:
      Skin IR now exposes closed `CreateScenarioSweep`, `ActivateSweep`, and `DeleteSweep` intents.
      The host owns the sweep manifest, materializes each point as an evaluated OxCalc
      candidate-backed internal scenario, keeps backing scenarios hidden from the ordinary scenario
      rail, and projects typed sweep point columns/series through `WorkspaceState.comparison` and
      `WorkspaceState.series`. Programmable Skin IR tests prove published-base sweeps,
      scenario-layered sweeps, typed input values, evaluated dependent formula values, active sweep
      projection, deletion cleanup, and hidden backing-scenario behavior from outside the skin layer.
      Goal-seek solving, richer sweep provenance, and durable arbitrary-candidate scenario
      snapshots remain open.
- [x] W5 early / W6 `scenario-persist-migrate` first managed document slice:
      Skin IR now exposes a host-managed `CreateScenario` intent that opens the backing OxCalc
      candidate itself, making the scenario reconstructable from typed overrides. `.dnatree`
      workspace documents persist managed scenario names, active scenario id, typed override
      payloads, managed sweep specs, and active sweep id; reload re-materializes candidates through
      OxCalc and re-evaluates values instead of storing rendered results. Arbitrary
      `CreateScenarioFromCandidate` scenarios remain transient because their candidate-private
      structural/content edits are not yet durable engine scenario snapshots. Programmable Skin IR
      and walking-skeleton autosave tests prove document export/import plus browser/desktop store
      autosave paths from outside the skin layer.
- [x] W5 early `projection-delta-channel` / `projection-version-stamp` synchronous projection slice:
      Skin IR now carries a required `latest_delta: ReadSignal<WorkspaceDelta>` beside the full
      `WorkspaceState` signal, and the host dispatcher owns a single publication path that stamps
      full snapshots with monotonic `projection_seq` values while emitting the matching receipt and
      latest-delta payload. Accepted no-op selection and rejected live-host intents publish unchanged
      deltas at the current sequence instead of falling back to sequence zero. Programmable Skin IR
      and walking-skeleton tests prove mutation deltas, full-reset deltas, no-op deltas, rejected
      no-op deltas, workspace switching, and shell/registry context wiring. Delta-only replay/resync,
      gap-recovery UI, worker calc, virtualization, frame telemetry, and later W5 platform hardening
      remain open.
- [x] W5 early `skinstate-persistence-exercised` slice:
      Skin IR now persists each typed `SkinState` by `(skin_id, slot, workspace_id)`, with load,
      schema migration, stable-`NodeKey` GC, and save-on-update owned by the framework handle rather
      than by skins. The framework exposes an in-memory store for tests and a native local-file store
      for desktop hosts; the wasm web entrypoint wires browser `localStorage`. Framework tests prove
      persisted roundtrip, migration, slot/workspace isolation, identity-keyed GC, and local-file
      storage. Walking-skeleton and programmable Skin IR tests prove the store is threaded through
      real shell/skin mounts without recalc or skin-side semantics.
- [x] W5 early `workspace-document-persistence` slice:
      Host-owned `.dnatree` workspace documents now persist through a `WorkspaceDocumentStore`
      seam with a workspace catalog and active-workspace pointer. `HostDispatcher` autosaves
      accepted intents, native hosts can use the local-file store, and the wasm web entrypoint
      restores/saves through browser `localStorage`. Walking-skeleton tests prove dispatcher
      autosave/restore, desktop local-file document storage, and managed what-if document autosave.
      Arbitrary candidate overlays and freeform candidate-backed scenarios remain later-policy work.
- [x] W5 early `design-token-layer` slice:
      Skin IR now exposes required `ThemeTokens` with typed light, dark, and high-contrast modes.
      The shell injects those tokens as `.dtc-shell` CSS custom properties and passes the same
      object through each mounted `SkinContext`. Built-in shell/skin CSS consumes `var(--dtc-...)`
      presentation tokens instead of literal colors. Framework tests prove token CSS emission;
      walking-skeleton and programmable Skin IR tests prove the token context reaches real mounts.
- [x] W5 early `a11y-primitives` first selection-surface slice:
      Skin IR now exposes framework-owned tree/listbox/table ARIA helpers, stable NodeKey-derived
      DOM ids, selectable item/row attribute carriers, and roving-tabindex policy. TripleEditor,
      FormulaTree, DependencyInspector, and OutlineTable consume those helpers so selected node
      surfaces publish `aria-selected`, stable active descendants, and one focusable visible item
      when no visible selection exists. Framework tests prove helper output; walking-skeleton and
      programmable Skin IR tests prove the real skins still mount and route host selection/dispatch.
- [x] W2 `command-palette-metadata` first Skin IR slice:
      Skin IR now exposes `WorkspaceState::command_catalog(&SelectionState)` with stable command
      kind ids, titles, shortcuts, effective bindings, enablement, and disabled reasons for the
      current closed workspace, node, clipboard, candidate, scenario, sweep, revision, and table
      command surface. The catalog derives from projected host truth only; it does not execute
      commands, fabricate target payloads, or parse formula semantics. Framework tests prove
      selection, clipboard, candidate, scenario, sweep, and revision-state enablement; a
      programmable Skin IR host test proves real dispatcher activity updates the catalog from the
      published `WorkspaceState`.
- [x] Real-skin command/template consumption checkpoint:
      The shared node-management panel used by FormulaTree and TripleEditor now reads
      `WorkspaceState.templates` to offer projected built-in initial-content templates, dispatches
      the selected template's typed `InitialNodeContentProjection` through the existing `AddNode`
      intent, and renders shortcut/enablement hints from `WorkspaceState::command_catalog` instead
      of local skin rules. Skins crate tests prove the helper behavior; the programmable Skin IR
      suite proves the projected templates still add/evaluate through the host and OxCalc paths.
- [ ] `candidate-overlay-handle`: continue W4b with broader structural order/delete/name-collision
      merge algebra beyond same-node rename/move, same-parent rename/add, and same-parent
      rename/reorder facet merging plus sibling add/delete, sibling add/reorder, and sibling
      delete/reorder merging, goal-seek comparison columns/series, and broader what-if UX.
- [x] `value-epoch-keying`: per-node published-value epoch is available for projection consumers.

## Next-Wave Parking Lot

Only pull these forward when their prerequisites above are met:

- W3: reference/content authoring verbs (`replicate-by-id`, F4 binding toggle, point-mode
  insertion, paste special, duplicate subtree, set membership, notes, formats).
- W4a/W4b/W4c: revision navigation, candidate overlays, speculation, scenarios, and comparative
  projections.
- W5 early remaining subset: delta-only resync/gap-recovery policy on top of the landed projection
  delta/version stream, plus table-cell-grid-specific a11y helpers and runtime theme switching as
  follow-through.
- W5+ later platform: worker calc, multi-slot composition, keybinding registry, virtualization,
  capability negotiation, error isolation, telemetry.
- W6: templates, table structural authoring, import/export, external feeds, sensitivity/goal seek,
  onboarding.

# Engine Requirements (OxCalc & OxFml) — push-ready handover

Full detail for every engine-owned ask. This is the document to roll out to the **OxCalc** and
**OxFml** lanes (companion to their `docs/handovers/` flow). Index and sequencing in
[`FUNCTIONALITY_MATRIX.md`](FUNCTIONALITY_MATRIX.md) / [`ROADMAP.md`](ROADMAP.md); tenets and the
ownership boundary in [`README.md`](README.md).

**Readiness:** `expose` = the engine already derives it internally, surface it on the contract ·
`extend` = bounded extension of existing machinery · `new` = genuine new capability needing a spike.

Shapes are illustrative Rust to fix intent and ownership; exact signatures are the lane's to finalise.
None of these ask the engine to take on host concerns (caret/edit-buffer, view-state, naming policy).

---

## OxCalc

### Gating workstreams (sequence first — each needs a spike)

#### `transaction-scope` — atomic batch with single publication · `extend` · L · foundational
> **Shape.** `WorkspaceIntent::EditNodes(Vec<ScopedEdit>)` *or* host `begin_transaction() ->
> TransactionScope { push(intent); commit() -> IntentReceipt }`. OxCalc accumulates the edits,
> schedules **once**, republishes **one** `WorkspaceState`; rolls back fully on any rejection.
- **Unlocks:** a 50-target fill, multi-node format change, or import bulk-edit commits as one revision
  and one update — no intermediate flicker, no half-applied model on failure. Prerequisite for every
  multi-target verb and the speculation commit path.
- **Note / risk:** the engine today publishes **per node** (`produce_candidate → PublishReady →
  publish_and_clear` keyed by `TreeNodeId`). Accumulate-many-edits-then-publish-once is genuine
  scheduler rework — the highest-risk foundational item even at effort L.

#### `revision-graph-retention` — retained, addressable, immutable revision DAG · `new` · L · foundational
> **Shape.** Retain published revisions in an addressable store: `RevisionId -> RetainedRevision {
> parent: Option<RevisionId>, value_epoch, edit_transaction_id, snapshot_refs }`; a navigable cursor;
> `fn revision_at(id)`, `fn navigate(id)`. A **DAG** because edit-after-undo branches.
- **Unlocks:** the substrate all of undo-navigation, version-history, time-scrubbing, and
  collaboration depend on.
- **Note / risk:** the engine today holds snapshot-*identity hashes* (content hashes for change
  detection), **not** a retained navigable store with parent links (`formula.rs` notes "retained
  non-table replay missing"). Undo-by-navigation has no substrate today (HARD CONSTRAINT 6 forbids
  inverse replay). Confirm whether this is a bounded extension of the snapshot machinery or a
  from-scratch store with its own memory/GC budget (open question #4).

#### `candidate-overlay-handle` — handle-addressed, layerable, non-publishing candidate substrate · `new` · L · foundational
> **Shape.** `fn open_candidate(basis: RevisionId) -> CandidateHandle`; `fn apply_overlay(h,
> OverlaySet)`; `fn evaluate_candidate(h) -> CandidateRun`; `fn close_candidate(h)`. Candidates
> **layer** (a candidate may declare a parent) and **never** enter the publication queue.
- **Unlocks:** multiple simultaneous what-ifs / scenario comparisons / sweep points; PreviewEdit,
  scenarios, goal-seek, and sensitivity all share **one** substrate.
- **Note / risk:** the engine today has a per-node publish lifecycle plus a **single**
  `RuntimeOverlaySet` pinned to the publication snapshot — no `open_candidate`, no parented/layered
  candidates, no isolation from the publication queue. Going from one publish path to many isolated
  parented overlay contexts is scheduler rework = **new**, not extend. *The single largest risk; the
  whole speculation band gates on it* (spike — open question #3).

### Read exposure (mostly cheap — surface facts already derived)

#### `typed-run-and-calc-state` — close run/calc_state enums & retire prose control-flow · `extend` · M · foundational
> **Shape.** Keep `calc_state: CalcState` and `last_run.run_state: RunState` as closed
> `#[non_exhaustive]` enums; designate `diagnostics: Vec<String>` the **only** free-text channel,
> advisory/non-load-bearing. **Engine cleanup:** retire sites that branch on diagnostic prose (e.g.
> matching `"cycle.excel_match_iterative"`, parsing `"oxfml_reject:{code}"`) — promote those facts to
> typed enum fields.
- **Unlocks:** skins safely switch on lifecycle state; "diagnostics are advisory" becomes true rather
  than aspirational.
- **Note:** the live engine today leaks control-relevant facts into diagnostics and branches on them
  downstream — this is engine debt to retire, else tenet 2 ships violated on day one.

#### `value-epoch-keying` — per-node published-value epoch · `extend` · M · foundational
> **Shape.** Stamp each node's *published* value with a `value_epoch` distinct from its *input* epoch;
> `NodeView.computed_value` pairs with `value_epoch: u64`. If true per-node value epochs prove too
> costly, the delta channel falls back to workspace `value_epoch` + the existing invalidated-node set.
- **Unlocks:** precondition for shape-diff, history scrubbing, delta value carriage, "is this stale
  relative to epoch N", and skin memoisation that skips re-formatting unchanged cells at scale.
- **Note:** today there is a **single** workspace-scope `value_epoch` + a per-node **input** epoch.
  Threading a true per-node value epoch is engine bookkeeping (open question #2).

#### `phase-timings-typed` — typed-KEY phase-timing map · `extend` · S · enriching
> **Shape.** `last_run.phase_timings: BTreeMap<PhaseKey, u64>` where `enum PhaseKey { Invalidate,
> Schedule, Evaluate, Publish, Overlay, .. Other(String) }` is **OxCalc-owned**.
- **Unlocks:** labelled perf lane; unknown ad-hoc phases survive via `Other(String)` instead of being
  dropped or host-re-classified.
- **Note:** must be an *engine-owned* enum with an `Other` escape — a fixed host-side struct would be
  re-classification of engine prose (the anti-pattern) or would drop phases.

#### `typed-cycle-diagnostics` — convergence/terminal/reject detail · `extend` · M · enriching
> **Shape.** `cycle_groups[].diagnostics: CycleDiagnostic { members: Vec<NodeKey>, profile:
> IterationProfile, iteration_trace: Vec<IterStep{ epoch, max_change }>, terminal: CycleTerminalState,
> reject_kind: Option<RejectKind> }`.
- **Unlocks:** cycle panel shows the convergence curve, terminal state, reject reason — not an opaque
  `CycleBlocked` badge.
- **Note:** the iterative path is **fixture-surfaced** today; general per-iteration data for arbitrary
  cycles is not yet derived (open question #7).

#### `reference-resolution-map` — token→target map + reverse index · `expose` · M · high-leverage
> **Shape.** `dependencies.resolution_map: BTreeMap<SourceReferenceHandle, ResolvedRef { token_span,
> target: RefTarget(Node|Collection|External|Unresolved), collection_members: Option<Vec<NodeKey>> }>`
> plus a reverse index for find-references.
- **Unlocks:** jump-to-definition / find-references; hover-token→highlight-target; powers
  `legality-impact-preview` rebind impact and `f4-toggle-binding`.
- **Note:** OxFml binds tokens to handles, OxCalc resolves handles to targets — both engine-owned;
  descriptors already carry `source_reference_handle`.

### Structural authoring & integrity

#### `rename-move-ref-integrity` — reference-preserving rename/move · `extend` · M · foundational
> **Shape.** `RenameNode`/`MoveNode` trigger OxCalc reference rebind so dependents follow the node by
> stable `NodeKey`; `descriptors_by_owner[].requires_rebind_on_structural_change` drives it; receipt
> reports rebound dependents.
- **Unlocks:** renaming/moving updates all name-based references automatically (Name-Manager parity);
  reorganising the tree never silently breaks formulas.
- **Note:** the intents exist; the gap is *guaranteed* integrity. Host must not rewrite referencing
  text (constraint 1).

#### `recalc-plan-preview` — dry-run invalidation over the committed graph · `extend` · M · high-leverage
> **Shape.** `fn plan_invalidation(&self, edits: &[PreviewMutation]) -> InvalidationPlan {
> invalidated_nodes, evaluation_order, requires_rebind, estimated_node_count, cycle_risk }` — walks the
> **existing committed** dependency graph **without** evaluating kernels, publishing, or opening a
> candidate.
- **Unlocks:** "this edit touches 14 nodes, forces 2 rebinds, risks a cycle" as a pre-commit hover;
  the blast-radius highlight.
- **Note:** uses only the committed graph (reverse edges + rebind flags the engine computes every
  tick) — needs **no** speculation substrate, so W2 stands on W0/W1 alone.

#### `set-membership-write` — authored reference collections · `extend` · M · high-leverage
> **Shape.** `WorkspaceIntent::SetCollectionMembership { owner: NodeKey, source_reference_handle,
> members: Vec<NodeKey>, order: MemberOrder }`; bumps `membership_version`/`order_version`.
- **Unlocks:** authoring a multi-value input as a reference *collection* (the sanctioned grid
  alternative): add/remove/reorder the nodes a SUM-over-set references.
- **Note:** multi-value is a node `Array` or a reference collection, never a grid (constraint 3).

#### `batch-repair-write` — RebindReferences / AcceptRepair · `extend` · M · enriching
> **Shape.** `WorkspaceIntent::RebindReferences { scope }`, `::AcceptRepair { scope }`; drives OxCalc
> to re-bind/clear `RejectedPendingRepair` and `requires_rebind` in one transaction; receipt reports
> repaired nodes.
- **Unlocks:** batch-repair nodes stuck after structural churn or import — "fix all broken references".

#### `table-structural-ops` — table row/column structure + column formula · `new` · L · frontier
> **Shape.** `::InsertTableRow/DeleteTableRow/InsertTableColumn/DeleteTableColumn/SetColumnFormula`;
> `TableProjection` gains column metadata (still **no** grid coords).
- **Unlocks:** authoring structured tables; a calculated-column formula that fills by reference.
- **Note:** confirm the engine's table notion reconciles rows + columns without 2D coordinate
  addressing (open question #9).

#### `cross-workspace-alias` — RegisterAlias + alias manifest · `new` · L · frontier
> **Shape.** `::RegisterAlias { local_name, target_workspace, target_node: NodeKey }`,
> `::UnregisterAlias { local_name }`; `WorkspaceState` gains `alias_manifest` + per-descriptor
> `external_available: bool`.
- **Unlocks:** reference another workspace's node by a stable local alias; engine tracks external
  availability/staleness.
- **Note:** cross-workspace resolution + external availability are OxCalc dependency-graph truth.

### Speculation & time (ride the two gating substrates)

#### `undo-redo-revision-nav` — undo/redo as revision navigation · `new` · M · high-leverage
> **Shape.** `WorkspaceIntent::Undo`, `::Redo`, `::NavigateRevision { target: RevisionId }`; the
> dispatcher moves a revision **cursor** over the retained DAG and republishes the snapshot for that
> revision. Never synthesises inverse edits.
- **Unlocks:** correct, deterministic undo/redo across runtime overlays and spill; the time-scrubber.
- **Depends:** `revision-graph-retention`, `revision-history-projection`, `edit-transaction-id`.

#### `value-shape-diff` — dims + changed-cell mask between epochs · `extend` · L · high-leverage
> **Shape.** `fn value_delta(from: ValueEpoch, to: ValueEpoch) -> ValueDelta { per_node:
> BTreeMap<NodeKey, NodeValueDelta { prior_dims, new_dims, changed: ChangeMask, kind_changed }> }`;
> works between any two epochs incl. a candidate's.
- **Unlocks:** FLOW's delta ledger / change-pulse and array unfurl (only changed cells).
- **Note:** inherits `value-epoch-keying`'s risk; mask is logical cells, not grid coords.

#### `derivation-trace-for-candidate` — per-candidate "why did this change" · `extend` · M · frontier
> **Shape.** `fn derivation_trace(h: CandidateHandle, node: NodeKey) -> DerivationTrace`; extends
> `full-derivation-trace` addressability from the published run to any candidate.

#### `goal-seek-substrate` — iterate a candidate to a target · `new` · L · enriching
> **Shape.** `fn goal_seek(basis, GoalSeekSpec { target_node, target_value, by_node, tolerance,
> max_iters }) -> GoalSeekResult { converged, solved_value, iterations, terminal }`. Runs entirely
> inside candidate overlays.
- **Note:** do **not** cite cycle/iteration as readiness — that path is fixture-surfaced. A
  Newton/secant loop driving candidate evals is genuinely new scheduler work.

#### `sensitivity-sweep-substrate` — N parallel candidate points · `new` · L · frontier
> **Shape.** `fn sweep(basis, SweepSpec { vary: Vec<SweepAxis{node, points}>, observe: Vec<NodeKey> })
> -> SweepResult { grid: Vec<SweepCell { inputs, outputs, run_summary }> }`. Each grid point is an
> independent candidate; none publishes.
- **Unlocks:** sensitivity sweeps no Excel data-table can do — vary tree-shaped inputs, observe many
  outputs, tornado/spider charts (via `series-projection`).

#### `speculation-budget-gc` — lease / budget / reclaim candidates · `new` · M · enriching
> **Shape.** Handles carry a lease; `fn reap_candidates(ReapPolicy)` closes un-pinned/un-referenced
> candidates past a budget; projection exposes `speculation_pressure`. Pinned views (host) are GC roots.

### Frontier — external feeds & collaboration (research)

#### `external-rtd-value-motion` — external/RTD source intake · `new` · L · frontier
> **Shape.** `::UpdateExternalValue { handle, value }` / `::InvalidateExternal { handle }` feed an
> engine external-value source; affected nodes go `Pending` then resolve when the **host** pumps the
> resolving tick on its worker; `NodeValueProjection` gains `External` provenance.
- **Note:** external sources + invalidation are engine truth, but the engine stays **synchronous** —
  the host pumps the resolving tick. "Passive engine" stays literally true. The source intake is the
  new-capability part.

#### `intent-conflict-policy` — typed conflict resolution over the revision DAG · `new` · L · frontier *(research)*
> **Shape.** `enum ConflictResolution { Rebase, Reject, Merge }`; OxCalc reports a typed
> `ConflictReport` when two peers' intents target overlapping nodes against the same basis revision.

#### `intent-rebase` — rebase a remote intent onto local head · `new` · L · frontier *(research)*
> **Shape.** `fn rebase(intent, from_basis, onto_head) -> RebasedIntent | Conflict`. NodeKey-keyed
> structural rebase is tractable; content/formula/collection rebase may conflict — genuinely
> research-grade.

---

## OxFml

#### `engine-dry-bind` — profile/bind legality of an unexecuted edit · `new` · M · high-leverage
> **Shape.** `fn dry_bind(edit: &PreviewMutation, profile: CapabilityProfile) -> BindVerdict {
> profile_violations: Vec<ProfileFeature>, bind_diagnostics: Vec<BindingDiagnostic>, would_rebind:
> Vec<NodeKey> }` — parse/bind only, no evaluation, no publish.
- **Unlocks:** the semantic half of `legality-impact-preview` — profile rejection and bind diagnostics
  for an intent that has not been applied. Profile gating happens at parse/bind (constraint 5), which
  is OxFml's.

#### `binding-diagnostics-typed` — typed binding/parse diagnostics (incl. profile rejection) · `expose` · M · high-leverage
> **Shape.** `enum BindingDiagnostic { UnresolvedRef{token_span}, ProfileRejected{feature:
> ProfileFeature}, AmbiguousName{name}, TypeMismatch, RebindPending }` carried on `NodeView` /
> `ActiveNodeDetail`. `ProfileFeature` is a readable enum so the UI can explain *why* input is rejected.
- **Unlocks:** inline red-squiggle authoring feedback at a token span; profile-gating UI.

#### `replicate-by-id` — fill carrying source + target IDs · `new` · L · high-leverage
> **Shape.** `WorkspaceIntent::ReplicateContent { source: NodeKey, targets: AuthoringScope, mode:
> ReplicateMode(RelativeRefs|AbsoluteRefs|ValuesOnly) }`; host forwards to OxFml which **recomposes
> raw text** per target by reference-relative rebind, then OxCalc schedules within one transaction.
- **Unlocks:** Excel-style fill-down/right **by name** — the engine rebinds each clone under the active
  profile; survives reorder. The hard rebind machinery `paste-special` and `duplicate-subtree` reuse.
- **Note:** HARD CONSTRAINT 1 — skins never rewrite formula text; relative/absolute adjustment is OxFml.

#### `f4-toggle-binding` — toggle reference binding mode by handle · `extend` · M · high-leverage
> **Shape.** `::ToggleReferenceBinding { node: NodeKey, source_reference_handle, cycle: BindingCycle
> (Rel→ColAbs→RowAbs→Abs) }`; OxFml mutates the binding mode of that one reference token, re-emits raw
> text; OxCalc rebinds.
- **Unlocks:** Excel F4 cycling — an intent carrying the handle, never a character edit.

#### `reference-insertion` — compose profile-correct reference text · `new` · M · high-leverage
> **Shape.** `fn compose_reference(editing_node: NodeKey, target: NodeKey, binding: BindingMode) ->
> RawText`; the **host** owns the edit buffer + caret and splices the returned text. The intent carries
> `(editing_node, target, binding)` only — **no caret**.
- **Unlocks:** point-mode reference insertion (click a node while editing to insert a ref to it); the
  skin never synthesises `A1`/name text.
- **Note:** composition of reference grammar/naming is OxFml's; caret is the host's (no ownership leak).

#### `paste-special` — paste Values | Formula | Format · `extend` · L · high-leverage
> **Shape.** `::PasteContent { source: NodeKey | Clipboard, targets: AuthoringScope, mode:
> PasteMode(Values|Formula|Format|FormulaAndFormat) }`. Values resolves source `computed_value` to
> constants; Formula rebinds like Replicate; Format copies effective format.
- **Note:** reuses the same hard rebind machinery as `replicate-by-id` (its dependency) — hence L, not
  cheaper.

#### `format-write` — author per-node effective format · `extend` · M · high-leverage
> **Shape.** `::SetFormatProperty { scope: AuthoringScope, property: FormatProperty(NumberFormatCode|
> Font|Fill|Border|Alignment) }`; OxFml parses/validates the number-format code; host stores the
> binding; projection surfaces `effective_format`.
- **Note:** format-code parsing is OxFml; rendering is engine-faithful (constraints 4/7).

#### `conditional-format-write` — author CF rules · `new` · L · frontier *(gated on a concrete use)*
> **Shape.** `::AddCfRule { scope, rule: CfRuleSpec }`, `::RemoveCfRule { scope, rule_id }`,
> `::ReorderCfRules { scope, order }`; OxFml owns rule semantics, OxCalc schedules per-cell evaluation.
- **Note:** the CF **result** read already ships in W1 (`per-node-effective-format`); the **authoring**
  surface is large net-new OxFml work, deferred until a concrete FLOW use beyond "highlight over
  threshold". Rule order is significant Excel semantics — hence explicit reorder.

#### `data-validation-write` — author validation rules · `new` · M · frontier *(gated on a concrete use)*
> **Shape.** `::SetDataValidation { scope, rule: Option<ValidationRule(List|NumberRange|CustomFormula)>
> }`; OxFml binds/evaluates; `NodeView.validation_state` is surfaced (read) regardless.
- **Note:** the validation **state** read ships in W1; **authoring** is deferred frontier.
  Custom-formula constraints are formula text the engine binds (constraint 1).

---

## Engine-side obligations behind host-projection reads

Several host-projection requirements are honest only if the engine exposes the underlying record. They
are owned by the host *contract* but carry an engine obligation worth tracking in the lanes:

| Host requirement | Engine obligation | Lane | Readiness |
|---|---|---|---|
| `full-derivation-trace` | expose the ordered prepared-call tree + hole bindings (not a count) | OxFml/OxCalc | expose |
| `runtime-effects-list` | expose the typed `RuntimeEffect` records (not just a count) | OxCalc | expose |
| `overlay-resize-deltas` | expose spill/region overlay resize deltas | OxCalc | expose |
| `per-node-effective-format` | expose evaluated number-format + per-cell CF result | OxFml | expose |
| `per-edge-cache-evidence` | expose per-edge `Hit/Miss/Bypassed` (not just a basis fingerprint) | OxCalc | extend (open q #6) |
| `format-resolver-on-context` | expose a `render(value, format_code, locale) -> RenderedCell` entrypoint | OxFml | extend |
| `workbook-export` | formula/format serialisation for round-trip out | OxFml | new |

---

*Sequence the three gating workstreams (`transaction-scope`, `revision-graph-retention`,
`candidate-overlay-handle`) with spikes first — see [`ROADMAP.md`](ROADMAP.md) open questions #2–#7.*

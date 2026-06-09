# Host & Skin-IR Requirements — implemented here

Full detail for every ask owned by the **host** (`WorkspaceState` projection + `WorkspaceIntent`
dispatcher) and the **Skin-IR contract** (`SkinContext`, `SkinState`, the projection/intent types).
These land in *this* repo. Engine-owned asks are in [`ENGINE_REQUIREMENTS.md`](ENGINE_REQUIREMENTS.md);
index/sequencing in [`FUNCTIONALITY_MATRIX.md`](FUNCTIONALITY_MATRIX.md) / [`ROADMAP.md`](ROADMAP.md).

Foundational items get a full block; high-leverage get shape + unlock; enriching/frontier get a
one-line shape. Every field is verbatim in
[`reference/REQUIREMENTS_SYNTHESIS.raw.json`](reference/REQUIREMENTS_SYNTHESIS.raw.json).

---

## Skin-IR contract

### Foundational

#### `stable-node-identity` — re-key the IR off `NodeKey` · expose · L
> `NodeView { key: NodeKey, display_path: NodeId, .. }` where `NodeKey` wraps the engine's existing
> `TreeNodeId(u64)`. `WorkspaceState` keys `node_order`, `dependencies`, selection, scope, references,
> history, `SkinState` by `NodeKey`. **Transition window:** carry both key + `display_path`; intents
> accept either during migration, then cut over.
- **Unlocks:** selection, scroll anchor, pins, expansion, persisted `SkinState`, delta addressing,
  reference maps, multi-select, value-diff, and history all survive rename/move. The keystone.
- **Note:** engine side is `expose` (the stable `TreeNodeId` already exists). The L/schedule-risk is
  the breaking IR/host cutover — every projection field, intent payload, `SkinState`, and the
  `walking_skeleton` tests. De-risk via the transition window (open question #1).

#### `richer-typed-value` — typed `NodeValueProjection` variants · expose · L
> `enum NodeValueProjection { Unevaluated, Pending, Empty, Number{raw:f64, unit:Option<UnitHint>},
> Text(String), Logical(bool), ErrorV(ExcelError), Array{dims, cells}, Reference(RefValue),
> Lambda{arity, params}, Rich(RichValueRef), Image(ImageRef) }`.
- **Unlocks:** numbers right-aligned, logicals as checkboxes, typed error glyphs, lambda arity badges,
  references as chips, `Empty` distinct from `Unevaluated` — without parsing a debug string.
- **Note:** must **not** add display formatting here (that is the resolver's job); `unit` is `expose`
  only if `EvalValue` already carries a hint, else omitted.

#### `typed-invalidation-reasons` · expose · M
> `enum InvalidationReason { UpstreamValueChanged{source:NodeKey}, StructuralRebindRequired,
> RefTargetMoved{handle}, CollectionMembershipChanged{family}, OverlayApplied, ManualDirty,
> CycleParticipant, FormatRuleChanged, .. #[non_exhaustive] }`; `invalidated_nodes[].reasons:
> Vec<InvalidationReason>` replaces `Vec<String>`.
- **Unlocks:** reason chips per node; "show only structurally-rebound nodes"; i18n + compiler-checked
  completeness. Independent, parallelizable, low-risk.

#### `typed-dependency-kinds` · expose · M
> `enum DependencyKind { DirectRef, QualifiedSibling, CollectionMember, RangeCarrier, NameRef,
> ExternalAlias, RuntimeSpill, .. }`; `enum CarrierDetail { SingleCell, Collection{family,members},
> Region{rows,cols}, .. }` replace the `String` fields on descriptors.
- **Unlocks:** group/filter/colour dependency edges by typed kind; find-references scoped by family.

#### `skinstate-persistence-exercised` — exercise persist + migrate + identity-keyed gc · n/a · M
> Persist each slot's `SkinState` keyed by `(skin_id, slot_id, workspace_id)`; on load run
> `migrate(from schema_version)`; run `gc(live_node_keys)` to drop view-state for deleted nodes.
> Round-trip tested.
- **Unlocks:** expansion/pins/columns/scenarios survive reload and skin upgrades; renamed nodes keep
  view-state (gc keys off `NodeKey`). Makes the currently-dead contract real.

### High-leverage
- **`value-published-pending-flag`** · extend · S — `ValueProvenance { origin: Published | Pending |
  Speculative{handle} | Scenario{id} | External{source}, basis_epoch }` alongside `computed_value`,
  plus `is_stale: bool`. *Ghost vs real vs scenario vs external, unambiguously styled.*
- **`scenario-projection`** · extend · S — `WorkspaceState.scenarios: ScenarioManifest { entries[{id,
  name, override_count, overridden_nodes, value_epoch}], active }`; `NodeView.scenario_override`. *The
  scenario rail.*
- **`revision-history-projection`** · new · M — `WorkspaceState.history: VersionHistoryProjection {
  entries[{revision_id, value_epoch, edit_transaction_id, label, parent, is_current, summary}],
  current }`. A **DAG**, not a stack (edit-after-undo forks). *Time-scrubber + history panel.*
- **`projection-version-stamp`** · n/a · S — `WorkspaceState.projection_seq: u64`; each delta carries
  `from_seq → to_seq`; a gap triggers full resync. *Makes the incremental path trustworthy.*
- **`design-token-layer`** · n/a · M — `SkinContext.tokens: ThemeTokens` → CSS custom properties;
  `ThemeMode { Light, Dark, HighContrast }`; skins reference `var(--surface)`, never literal hex.
  *Dark/high-contrast/re-skin across all skins at once; removes hardcoded hex coupling.*
- **`a11y-primitives`** · n/a · M — framework helpers `tree_a11y(...) -> AriaAttrs`; roving-tabindex
  bound to the selection signal; `aria-activedescendant` wired to NodeKey-derived stable DOM ids.
  *Keyboard nav + screen-reader for every tree/table skin without re-implementation.*
- **`drag-gesture-model`** · extend · M — `drop_legality(dragged: AuthoringScope, over: NodeKey,
  position: DropPosition) -> DropVerdict { legal, would_rebind, would_orphan, collision }` (reuses
  `legality-impact-preview`); `DropPosition`/`DropVerdict` types. Drag *state* stays skin-local; only
  the verdict crosses. *Live drop-target validity during a drag.*

### Enriching / frontier
- `rich-image-value-handles` · extend · M — `NodeValueProjection::Rich(RichValueRef)` / `::Image(ImageRef)`, opaque handles.
- `comparative-multi-overlay-projection` · extend · M — `ComparativeProjection { basis, columns:[OverlayColumn{label, source, values}] }` — side-by-side scenario/sweep columns.
- `frame-telemetry-hooks` · n/a · M — `FrameMetric { intent_seq, dispatch_to_delta_us, delta_apply_us, render_us, dropped }` into the replay sink. *Makes the 60fps goal falsifiable.*
- `locale-presentation-layer` · n/a · M — `SkinContext.locale: LocaleTokens { dir: Ltr|Rtl, ui_strings }` — chrome strings + direction only (value formatting stays engine-owned).

---

## Host-projection

### Foundational

#### `format-resolver-on-context` — thin OxFml forward + profile on `SkinContext` · extend · M
> `SkinContext` gains `profile: CapabilityProfile` and `resolver: &FormatResolver`, where `resolve()`
> **forwards** to OxFml `render(value, format_code, locale) -> RenderedCell` and the host attaches only
> frame concerns (align/colour → tokens). The profile's gated feature set is readable so the UI can
> explain rejections.
- **Unlocks:** every skin renders Excel-faithful display text via `resolve()` instead of debug
  `Scalar` text (closes the raw-debug-text gap); profile-aware input gating.
- **Note:** the resolver **must not** parse number-format codes host-side (that would re-implement
  OxFml semantics and risk divergence — constraint 7). `extend` because OxFml must expose `render`.

#### `projection-delta-channel` — incremental delta alongside full snapshot · extend · L
> `enum WorkspaceDelta { NodesChanged(Vec<NodeKey>), ValuesChanged(Vec<(NodeKey,NodeValueProjection)>),
> DepsChanged(..), CalcRun(..), Structural(..), FullReset }`; receipt carries `delta: WorkspaceDelta`
> and the workspace signal exposes a delta stream alongside the full `ReadSignal`. **Keys on the
> engine's existing invalidated-node set** (not blocked by per-node value epochs).
- **Unlocks:** skins re-render only changed rows instead of diffing a full clone — the single biggest
  responsiveness win; makes spill/region-resize and value-shape-diff cheap; a virtualised tree updates
  O(changed) not O(model).
- **Note:** the host maps engine invalidation into a delta rather than discarding it via full
  re-snapshot. Republish stays synchronous and complete; the delta is additive (open question #8 about
  a future delta-only mode).

#### `multi-slot-composition` — wire the inspector/split slots · n/a · M
> Shell mounts `SlotLayout { main, right_inspector, split_left, split_right }`; each slot gets its own
> `SkinContext` sharing the **same** workspace + selection + shared signals but a distinct typed
> `SkinState`; `SlotId` on context.
- **Unlocks:** the FLOW multi-pane vision — tree in Main, active-node detail in RightInspector,
  what-if/scenario in a split pane — simultaneously, by independent skins.
- **Note:** composition is a host/shell concern; skins remain frame-only. Shared truth via one signal
  set keeps panes coherent without skins coordinating.

### High-leverage
- **`per-node-effective-format`** · expose · M — `NodeView.effective_format: EffectiveFormat`;
  `NodeView.cf_results: Vec<CfResult{rule_id, matched, applied}>` (per-cell for arrays); also
  `validation_state`. *READ of evaluated results only — never re-evaluate rules host-side.*
- **`full-derivation-trace`** · expose · L — `last_run.derivation_traces: Vec<DerivationTrace{node,
  template, hole_bindings, child_calls:[PreparedCall{kernel, arg_values, result}]}>` + read-only
  `EvaluateFragment(node, sub_expr_id)`. *FLOW's explain-stack.*
- **`runtime-effects-list`** · expose · M — `last_run.runtime_effects: Vec<RuntimeEffect{Spill|
  DynamicRef|OverlayWrite|RegionResize}>` replaces the bare count.
- **`overlay-resize-deltas`** · expose · M — `last_run.overlay_deltas: Vec<OverlayDelta{node, kind,
  prior_dims, new_dims, grew}>`. *Array unfurl animates only the changed region.*
- **`active-node-detail`** · extend · M — `WorkspaceState.active_detail: Option<ActiveNodeDetail{node,
  content_text, value, effective_format, binding_diagnostics, reference_map}>`. *One struct for the
  formula bar/inspector.*
- **`naming-collision-policy`** · new · S — `RenameStrategy { on_collision: Reject|AutoSuffix|Replace }`;
  host owns the name-uniqueness algebra for Add/Rename/Duplicate/Instantiate; collisions reported via
  legality preview. *Keyed on `NodeKey`, `display_path` is the contended namespace.*
- **`legality-impact-preview`** · extend · M — `preview_intent(&WorkspaceIntent) -> MutationImpact {
  legal, profile_violations /*from engine dry_bind*/, requires_rebind /*from plan_invalidation*/,
  affected_refs, orphaned_dependents, collisions, blocked_reason }`; pure, no mutation. *A thin
  ORCHESTRATOR — host computes only collisions/orphans/scope; profile + rebind are delegated and
  surfaced verbatim.*
- **`model-query-projection`** · extend · M — `query(QuerySpec{text_match, calc_state_filter,
  has_error, references, is_meta}) -> QueryResult{matches, total}`. *Find a node in a 100k tree; pairs
  with virtualization.*
- **`host-worker-calc`** · n/a · L — the **host** runs the synchronous engine calc on a worker, pumping
  bounded slices per tick; `run_state` gains `Pending{token, started_value_epoch}`; dispatch returns
  immediately with `completion: Option<CompletionToken>`; host republishes on completion. *The engine
  stays single-threaded/passive; concurrency is a host concern* (open question #5).
- **`capability-manifest-negotiation`** · n/a · M — `SkinManifest { required_read_fields,
  required_intents:[IntentId], required_slots, min_profile, schema_version }`; host `negotiate(manifest)
  -> Result<MountGrant, CapabilityError>` checked before mount; mismatches fail loudly. *3rd-party
  skins rejected cleanly instead of panicking at render.*

### Enriching / frontier
- `per-edge-cache-evidence` · extend · M — `last_run.scheduling: SchedulingReport { mode, edges:[EdgeEval{owner, cache:Hit|Miss|Bypassed, reused}], reuse_ratio }` (open question #6).
- `table-cell-readback` · extend · L — `TableProjection.cells_view(table_id) -> TableCells { columns:[ColumnView{name, formula, effective_format}], rows:[[NodeValueProjection]] }` — read-only.
- `series-projection` · extend · M — `series(scope) -> Series { points:[(label, NodeValueProjection)], unit }` — plottable feed for chart skins.
- `virtualization-window-projection` · extend · L — `request_window(WindowSpec{anchor, before, after, expanded_set})` → only in-view `NodeView`s + values; off-window reference-only.
- `keybinding-registry` · n/a · M — host resolves `KeyChord -> IntentId` per focused slot with a conflict policy; user overrides persist in audited SharedState; surfaced in command catalog.
- `skin-error-isolation` · n/a · M — shell wraps each slot mount in an error boundary; a panicking skin shows a fallback in *its* slot without taking down others.
- `empty-state-onboarding` · n/a · S — `NewWorkspace` seeds from a named `StarterTemplate`; command catalog exposes onboarding metadata.
- `pinned-speculative-view` · extend · M — audited shared `pinned_speculations:[PinnedView{handle, label, basis_epoch}]` keeps a candidate alive (GC root) while exploring others.
- `scenario-persist-migrate` · n/a · M — scenario *names/metadata* persist via `SkinState`; override **values** round-trip through the engine model, never a parallel store.
- `collab-presence-markers` · new · M — shared `peers:[PresenceMarker{peer_id, slot, selection, editing}]` + advisory `ClaimNode`/`ReleaseNode` soft-locks *(research)*.

---

## Host-intent

### Foundational

#### `selection-subject-model` — first-class multi-selection · n/a · S
> `struct SelectionState { primary: Option<NodeKey>, anchor: Option<NodeKey>, selected: Vec<NodeKey>
> /*ordered, deduped*/ }`; `WorkspaceIntent::SelectNodes(Vec<NodeKey>)`, `::SelectRange{anchor, focus}`,
> `::ToggleNodeInSelection(NodeKey)`.
- **Unlocks:** shift/ctrl-select a set and apply one fill/delete/format verb to all. The capability
  flag `supports_multi_select` already exists, unbacked.
- **Note:** host-owned; range resolution uses `node_order`/depth; routes to the selection signal only
  (no recalc); keyed on `NodeKey` to survive structural edits.

#### `scope-value` — typed `AuthoringScope` on mutating intents · n/a · M
> `enum AuthoringScope { Node(NodeKey), Nodes(Vec<NodeKey>), Subtree(NodeKey), Collection{owner,
> source_reference_handle} }`; mutating intents take `scope: AuthoringScope` instead of a bare id.
- **Unlocks:** one verb spans single node / multi-select / subtree / reference collection —
  Delete/Replicate/Paste/SetFormat work over any subject without verb explosion. Keeps the closed enum
  small while making subjects rich. A skin cannot safely expand a `Subtree` scope (membership +
  meta-invisibility are host/engine truth).

#### `typed-intent-error` — replace `Rejected(String)` · extend · S
> `enum IntentError { ProfileRejected{feature}, NameCollision{existing}, WouldFormCycle{members},
> Forbidden{persona}, Orphans{refs}, EngineRejected{revision, detail:RejectKind}, Unsupported }`.
> Shares its vocabulary with `legality-impact-preview` so pre/post-commit errors match.
- **Unlocks:** a recoverable, explainable rejection instead of grepping a string. **Closes a tenet-2
  violation hiding inside the receipt** — sequenced in W2, not deferred.

#### `edit-transaction-id` — receipt carries txn id + revision + completion + per-edit failure · extend · S
> `struct IntentReceipt { accepted, error:Option<IntentError>, transaction_id:TxnId,
> produced_revision:Option<RevisionId>, completed_signal:Option<CompletionToken>, delta:WorkspaceDelta,
> failed_edit_index:Option<usize> }`.
- **Unlocks:** correlate an edit with the revision it produced (undo nav, optimistic UI, scrubber
  labels), await completion once async lands, tie a delta-ledger entry to its cause, know which scoped
  edit failed in a batch. `completed_signal` `Option` keeps it synchronous today, async-ready later.

#### `speculation-discard-commit` — terminal actions for a speculation handle · extend · M
> `WorkspaceIntent::CommitSpeculation{handle}` applies the candidate's edits as a normal published
> edit-transaction (advances `value_epoch`, becomes a retained revision); `::DiscardSpeculation{handle}`
> closes the candidate, no revision created.
- **Unlocks:** the only two terminal actions on a ghost what-if — bless it into history or drop it for
  free. The core "explore freely, commit deliberately" loop. *Commit is the SOLE bridge from
  speculation to the publish path.*

### High-leverage
- **`command-palette-metadata`** · new · M — `command_catalog(&WorkspaceState, &SelectionState) ->
  Vec<CommandMeta{intent_kind, title, shortcut, effective_binding, enabled, disabled_reason}>`.
  *Palette/menus auto-populate with enablement; greyed "Delete" when nothing selected.*
- **`duplicate-subtree`** · new · L — `::DuplicateNode{source, new_parent, new_index, rename:
  RenameStrategy}`; clones structure + content; internal refs rebind to the clone, external preserved.
  *Clone a sub-model in one intent (reuses `replicate-by-id` rebind machinery).*
- **`meta-and-attribute-write`** · extend · M — `::SetMeta{node, is_meta}`, `::SetNodeAttributes{node,
  attrs}`. *Mark a subtree meta (excluded from calc) as a revisioned edit — toggling meta changes the
  dependency graph, so it MUST be an engine-visible intent, not a shared-state flag.*
- **`workbook-export`** · new · L — `::ExportWorkbook{scope, mode:ValuesOnly|FormulasAndFormat|
  FullModel} -> ExportArtifact`; plus a lighter `value_snapshot(scope) -> Csv|Json`. *Closes the
  verify-against-Excel loop; a platform that imports but can't export traps user work.*
- **`preview-edit-intent`** · extend · L — `::PreviewEdit{edits:[PreviewMutation], handle}`; receipt
  carries `PreviewProjection { handle, run, overlay_values, value_epoch_basis }`. **Never** advances
  published `value_epoch`. *FLOW's ghost what-if.*
- **`scenario-substrate`** · extend · L — `::SetScenarioOverride{scenario_id, node, value}`,
  `ClearScenarioOverride`, `CreateScenario`, `DeleteScenario`, `ActivateScenario`. *Override VALUES
  live as an OxCalc overlay; naming/active-selection is host state.*
- **`audited-shared-state`** · n/a · M — replace direct-mutation shared fields with a `SharedStore
  set(key, value)` that validates and records `(SlotId, key, prev, next, ts)` into a bounded audit
  ring; typed key enum. **The `recalc_mode` switch (calc-affecting) routes through the dispatcher / is
  read as dispatcher policy, NOT a raw shared write.** *Closes the side-door mutation gap.*
- **`intent-log-replay`** · extend · M — `IntentRecorder` taps the dispatcher: append `(seq, intent,
  receipt, delta, value_epoch, persona, origin)`; `replay(log, fresh_workspace) -> WorkspaceState`.
  *Deterministic skin tests, repro bug reports, audit, the collab wire format. Replay re-issues intents
  through the same dispatcher — not inverse-undo.*

### Enriching / frontier
- `clipboard-transfer-model` · new · M — `WorkspaceState.clipboard: Clipboard { operation: Copy|Cut, payload: Values{content_kind,constant_input_text?,value} | Formula{source} | Format | Subtree{root}, plain_text? }`; `CopyToClipboard`/`CutToClipboard` populate, `PasteClipboardFormat` and constant-source `PasteClipboardValues` are the first paste consumers, successful constant-value cut paste clears the source plus host clipboard in one transaction, and `PasteExternalClipboardText` imports platform-supplied clipboard text as authored content. Paste/PasteSubtree consume in the full model. Host-owned, distinct from OS clipboard access.
- `add-node-content-policy` · extend · S — `AddNode` gains `initial: Empty|InheritColumnFormula{table,column_id}|TemplateBound{id}|Literal` and `is_meta: bool`.
- `note-write` · extend · S — `::SetNote{node, note:Option<NoteContent>}`; `NodeView.note`. *Authored notes that round-trip to Excel comments; may stay allowed for Reviewer.*
- `template-subsystem` · new · L — `::PromoteToTemplate`, `::InstantiateTemplate{template_id, parent, bindings}`, `::EditTemplate`, `::SyncInstance`, `::DetachInstance`; `template_index` + `NodeView.instance_of/drift`.
- `import-workbook` · new · L — `::ImportWorkbook{source, mode:DryRun|Commit}`; DryRun → `ImportManifestProjection{proposed_nodes, binding_diagnostics, unsupported_features}` without mutating.
- `shared-focus-arbitration` · n/a · M — shared `focused_slot`, `hovered:Option<NodeKey>`, `recent_selections:VecDeque<NodeKey>` + arbitration so one slot's selection drives others' highlight without focus theft.
- `readonly-reviewer-persona` · extend · M — `SkinContext.permissions{persona:Author|Reviewer|ReadOnly, can_mutate, allowed_intents}`; dispatcher rejects disallowed intents **per intent ORIGIN** (local or peer) with `Forbidden{persona}`.
- `backpressure-coalescing` · extend · M — dispatcher coalesces superseded `EditContentDeferred` on the same `NodeKey` and drops/queues recalc while a run is `Pending`; receipt may return `Coalesced{into_seq}`.
- `add-node-content-policy`, `note-write`, `template-subsystem` reuse the transaction + rebind machinery already specified above.

---

## ATLAS-surfaced additions

Surfaced while designing the [ATLAS suite](../skin-suite/). All host/skin-layer.

#### `cleave-predicate-shared` — the filter/cleave predicate as shared continuity · extend · S · **foundational**
> Promote the active `QuerySpec` **predicate** (filter only) from a lens's local `SkinState` into
> host-owned shared state: `SharedSkinState.cleave_predicate: Option<QuerySpecPredicate>`. On a lens
> switch each lens **re-runs** `model-query` locally against the current model.
- **Unlocks:** "cleave to all errors in Ledger, switch to Flow, Flow lights the matching error cone";
  the cleave is part of continuity, so a filtered focus follows you across lenses.
- **Note (honest scope):** the *predicate* carries, **not** a frozen `NodeKey` match-set (re-applying
  is correct and avoids re-materialising a set past the virtualization window); the resulting set may
  differ if the model changed between switches. **Sort key and group-by stay lens-local presentation.**
  Depends on `model-query-projection` (W2) + `audited-shared-state`. Ships with the early shared-state
  layer because it is sold as continuity.

#### `shared-focus-set` — optional cross-lens focus highlight (NO unified zoom intent) · n/a · S · enriching
> Optional `SharedSkinState.focus_set: Vec<NodeKey>` for degree-of-interest / fade-by-distance
> highlight across panes. **Explicit non-goal:** there is **no** `Zoom`/degree-of-interest *intent* —
> viewport/camera zoom is skin-local `SkinState`, never dispatched, never shared. Only the **collapse**
> and **pin** portions of Focus/Zoom are shared, and they already exist (`tree_collapsed`/`pinned`).
- **Unlocks:** a consistent "dim the periphery around the selection" cue across composed panes without
  conflating a cosmetic camera with model state.
- **Note:** guards the frame-only boundary — a camera-zoom must never become a dispatcher intent.

#### `cockpit-preset-registry` — named, persisted SlotLayout presets · n/a · M · enriching
> Persisted `Vec<CockpitPreset { name, slot_layout: SlotLayout, default_persona: Persona,
> restore_on_open: bool }>`, selectable as first-class objects; `multi-slot-composition` provides the
> mechanism, this provides the curated defaults.
- **Unlocks:** the six flagship ATLAS cockpits (Modeling / Author / Audit / Sheet / Map / Story) as
  one-gesture presets, with restore-on-workspace-open.
- **Depends:** `multi-slot-composition`, `skinstate-persistence-exercised`. (Default compositions-per-
  modality are open question #13.)

#### `facade-position-persistence` — skin-owned geometry keyed on NodeKey · n/a · M · enriching
> A persisted `CanvasSkinState { positions: HashMap<NodeKey,(f32,f32)>, groups: Vec<GroupRect> }`,
> workspace-scoped, that survives reparent/rename/undo (positions are cosmetic, never the model) and is
> **preserved** across lens switches by `skinstate-persistence`, **not re-projected** from shared truth.
- **Unlocks:** Canvas's spatial map survives structural undo and edits; the concrete reason "zero
  re-orientation" is scoped to *shared truth*, with intrinsic geometry *preserved* rather than
  reconstructed.
- **Note:** keyed on `NodeKey` so positions survive rename/move; gc on node delete. Depends on
  `stable-node-identity` + `skinstate-persistence-exercised`.

#### `replay-authored-artifact` — recorded exploration as a shippable, editable asset · extend · M · frontier
> On top of W5 `intent-log-replay`: a recorded `(intent, receipt, delta, revision)` exploration that
> can be **saved, named, edited** (trim/reorder steps) and shipped with the workspace as a presentation
> asset, plus a self-advancing timeline transport bound to a narrative block sequence.
- **Unlocks:** Story's "Play" walkthrough — a model that ships a replayable narrative of its own
  reasoning (onboarding / audit / teaching).
- **Depends:** `intent-log-replay`, `revision-history-projection`. Authoring/curation layered over W5
  raw capture/playback.

#### `narrative-projection` — block-backed model-card projection · new · M · frontier
> A new live projection type: `NarrativeProjection { blocks: Vec<NarrativeBlock { id, backing:
> NodeKey|Scope|Series, kind }>, cursor }` — an ordered list of curated blocks each backed by a node /
> scope / series, with stable block identity across edits and a read-only block-cursor selection axis.
> Distinct from the node / grid / graph projections.
- **Unlocks:** Story's document spine as a **live** projection over engine truth (never frozen text);
  every figure stays faithful and re-projects under scenario/revision.
- **Note:** a model-card is a curated *view*; blocks reference nodes/scopes/series, never copy values.
  Depends on `richer-typed-value`, `series-projection`, `stable-node-identity`.

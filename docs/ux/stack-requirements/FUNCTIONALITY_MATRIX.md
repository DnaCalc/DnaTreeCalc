# Functionality Matrix

The capability-domain × stack-layer grid, then the complete catalog index of every requirement.
Full per-requirement detail (Rust shape, justification, doctrinal note) lives in
[`ENGINE_REQUIREMENTS.md`](ENGINE_REQUIREMENTS.md) (OxCalc/OxFml) and
[`HOST_AND_SKIN_IR_REQUIREMENTS.md`](HOST_AND_SKIN_IR_REQUIREMENTS.md) (Host/Skin-IR), and verbatim in
[`reference/REQUIREMENTS_SYNTHESIS.raw.json`](reference/REQUIREMENTS_SYNTHESIS.raw.json). Tenets are
defined in the [README](README.md).

Legend — **Readiness**: `expose` = the engine already derives it, just surface it · `extend` =
bounded extension of existing machinery · `new` = genuine new engine capability · `n/a` = pure
host/skin concern. **Effort**: S/M/L. **Tier**: foundational / high-leverage / enriching / frontier.

---

## Capability domain × stack layer

For each domain, what each layer must provide (`—` = nothing at that layer).

### Stable identity & provenance
- **Skin IR** — `NodeView { key: NodeKey, display_path }` (transition window carries both); all maps/selection/state/`ValueProvenance` key on `NodeKey`.
- **Host** — derive `display_path` from key; map engine invalidation into deltas; identity-keyed `SkinState` gc; selection/scope/clipboard/log key on `NodeKey`; receipt joins intent→revision via txn id.
- **OxCalc** — expose the existing stable `TreeNodeId`; per-node value epoch (extend); resolution map handle→`NodeKey`.
- **OxFml** — —

### Typed read fidelity (the calc X-ray)
- **Skin IR** — `InvalidationReason` / `DependencyKind` / `CarrierDetail` / `BindingDiagnostic` enums; `PhaseKey`-keyed timing map with `Other(String)`; richer `NodeValueProjection`; closed run/calc_state.
- **Host** — runtime-effects list, overlay deltas, full derivation traces, active-node detail; per-edge cache evidence (contingent on engine exposing per-edge outcomes).
- **OxCalc** — expose existing invalidation/scheduling/overlay records verbatim; commit to `PhaseKey` enum; **retire control-flow-via-diagnostic-string**; cycle iteration is only fixture-surfaced today (extend).
- **OxFml** — expose prepared-call tree + hole bindings; emit typed bind diagnostics; expose evaluated CF results.

### Value display & formatting
- **Skin IR** — typed value variants incl. `Rich`/`Image` + `UnitHint`; `EffectiveFormat`/`FormattedValue` shapes.
- **Host** — per-node `effective_format` + per-cell `cf_results` (READ); resolver + profile on `SkinContext` as a thin **forward**; table cell readback; series projection.
- **OxCalc** — expose `EvalValue` universe as typed variants; spill/array dims.
- **OxFml** — **own** number/format-code rendering behind the resolver (host must not parse codes); CF rule semantics; rich-value field structure.

### Selection, scope & subjects
- **Skin IR** — `SelectionState { primary, anchor, selected }`; `AuthoringScope` enum; `DropPosition`/`DropVerdict`.
- **Host** — `node_order`/depth range resolution; collision/naming policy; drop-legality; `recent_selections`/hover in shared; `SelectNodes`/`SelectRange`/`ToggleNodeInSelection`; scope-bearing intents; command-catalog enablement.
- **OxCalc** — collection membership/order versioning.
- **OxFml** — —

### Structural authoring & references
- **Skin IR** — intents carry `NodeKey`/`RefHandle`/scope only — never formula text, never caret; `ResolvedRef` map shape.
- **Host** — reference-map projection; legality-impact JOIN over engine dry-bind + plan-invalidation; default-content policy; clipboard payload; Add/Rename/Move/Delete/Duplicate/Reorder; SetMeta/Attributes/Note; template subsystem; Copy/Cut; **host owns caret + splice**.
- **OxCalc** — rename/move reference rebind by `NodeKey`; `plan_invalidation` over committed graph; batch repair; one-transaction publish (engine rework).
- **OxFml** — `dry_bind` (profile legality); recompose raw text on replicate/paste/F4; compose profile-correct reference text (host splices); column-formula.

### Format & rule authoring
- **Skin IR** — `FormatProperty` spec typed; `CfRuleSpec`/`ValidationRule` specs typed (write deferred; results read now).
- **Host** — store where format binds; surface `effective_format` + `cf_results` + `validation_state` back.
- **OxCalc** — schedule per-cell CF + validation evaluation; version stamps.
- **OxFml** — parse/validate number-format codes; own CF rule semantics; bind data-validation formulas — never host-computed.

### Atomicity, errors, undo & history
- **Skin IR** — `IntentReceipt { txn_id, produced_revision, completion, failed_edit_index }`; typed `IntentError`; `VersionHistoryProjection` (DAG).
- **Host** — project history entries + touched-node summaries; `begin_transaction`/`EditNodes`; `Undo`/`Redo`/`NavigateRevision`; receipt + typed-error minting.
- **OxCalc** — accumulate-schedule-publish-once (**engine rework**); **retained immutable revision graph** + cursor (**new**); never inverse-replay.
- **OxFml** — —

### Speculation & scenarios
- **Skin IR** — `ValueProvenance` discriminator; `ScenarioManifest`; `ComparativeProjection`; `PreviewProjection`.
- **Host** — pinned speculative views (audited); scenario metadata persistence (values stay in engine); speculation facade on `SkinContext`; `PreviewEdit`/`Commit`/`Discard`; scenario `Create`/`Set`/`Clear`/`Activate`.
- **OxCalc** — **handle-addressed layerable candidate overlays** (**new** keystone, gates the band); scenario overlay; goal-seek/sweep (new); value-shape-diff; per-candidate trace; lease/GC.
- **OxFml** — bind scenario/preview override raw text under the active profile.

### Performance, async & freshness
- **Skin IR** — `projection_seq` + `from_seq→to_seq` sequencing; `is_stale` flag; `FrameMetric`; `(NodeKey, value_epoch)` memo key.
- **Host** — `WorkspaceDelta` channel (keys on engine invalidated-node set); windowed/range projection; model-query; **host-owned worker calc**; intent coalescing/backpressure; `receipt.delta`.
- **OxCalc** — engine stays **synchronous/passive** (no engine threading); external/RTD value source intake (new); maps invalidation into the host delta.
- **OxFml** — —

### Platform: composition, persona, observability, collab
- **Skin IR** — `SkinManifest`; `ThemeTokens`/`LocaleTokens`/`AriaAttrs`; `SkinState` serde + schema_version + migrate + gc.
- **Host** — wire `RightInspector`/`SplitLeft`/`SplitRight`; per-slot context + fault isolation; capability `negotiate()`; design-token/theme; keybinding registry; persist+migrate `SkinState`; export; persona/permission gate at dispatcher **per origin**; audited typed `SharedStore` (recalc_mode is dispatcher policy); cross-slot focus; intent log + replay; edit-claims.
- **OxCalc** — immutable revision ordering + conflict policy + intent rebase as the collaboration sync substrate (**research-grade**).
- **OxFml** — profile feature set readable so the host can gate/explain rejected input; export serialization.

---

## Complete catalog index

Grouped by tenet. `→` = depends on.

### Tenet 1 — Identity is permanent, path is cosmetic
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `stable-node-identity` | Skin IR | expose | L | **foundational** | — | view-state/selection/refs/history survive rename/move; the keystone |
| `value-epoch-keying` | OxCalc | extend | M | **foundational** | stable-node-identity | per-node staleness, shape-diff, delta value carriage, memo keys |
| `reference-resolution-map` | OxCalc | expose | M | high-leverage | typed-dependency-kinds | jump-to-definition / find-references; powers legality preview & F4 |

### Tenet 2 — Typed and lossless over stringly and flattened
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `typed-invalidation-reasons` | Skin IR | expose | M | **foundational** | — | reason chips / filters; compiler-checked, i18n-able |
| `typed-dependency-kinds` | Skin IR | expose | M | **foundational** | — | group/filter/colour edges by kind |
| `richer-typed-value` | Skin IR | expose | L | **foundational** | — | numbers/logicals/errors/lambdas/refs rendered without parsing debug text |
| `typed-run-and-calc-state` | OxCalc | extend | M | **foundational** | — | safe lifecycle switching; engine stops branching on its own prose |
| `phase-timings-typed` | OxCalc | extend | S | enriching | — | labelled perf lane; unknown phases survive via `Other` |
| `runtime-effects-list` | Host | expose | M | high-leverage | richer-typed-value | list what spilled/resolved this tick (not just a count) |
| `overlay-resize-deltas` | Host | expose | M | high-leverage | runtime-effects-list | animate only the changed spill region |
| `typed-cycle-diagnostics` | OxCalc | extend | M | enriching | typed-invalidation-reasons | convergence curve / terminal / reject instead of opaque badge |
| `binding-diagnostics-typed` | OxFml | expose | M | high-leverage | reference-resolution-map | inline red-squiggle; explain profile rejection |
| `active-node-detail` | Host | extend | M | high-leverage | reference-resolution-map, per-node-effective-format, richer-typed-value, binding-diagnostics-typed | one struct for the formula bar/inspector |
| `rich-image-value-handles` | Skin IR | extend | M | frontier | richer-typed-value | render linked-data rich values & in-cell images |
| `table-cell-readback` | Host | extend | L | enriching | richer-typed-value, per-node-effective-format | render real table cells/column formulas |
| `typed-intent-error` | Host | extend | S | **foundational** | — | recoverable/explainable rejections (profile/collision/cycle/forbidden) |

### Tenet 3 — Engine is the sole source of derivation, value, format
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `per-edge-cache-evidence` | Host | extend | M | high-leverage | typed-invalidation-reasons | which subtree recomputed vs reused |
| `full-derivation-trace` | Host | expose | L | high-leverage | richer-typed-value | the explain-stack: ordered prepared-call tree w/ kernel in/out |
| `per-node-effective-format` | Host | expose | M | high-leverage | richer-typed-value | resolved format + which CF rule fired per cell |
| `format-resolver-on-context` | Host | extend | M | **foundational** | per-node-effective-format, richer-typed-value | Excel-faithful display text via engine forward (closes raw-debug-text) |
| `series-projection` | Host | extend | M | enriching | richer-typed-value, scope-value | plottable series feed for chart/value-board skins |
| `derivation-trace-for-candidate` | OxCalc | extend | M | frontier | candidate-overlay-handle, value-shape-diff, full-derivation-trace | "why is this different under Bear?" |
| `format-write` | OxFml | extend | M | high-leverage | scope-value, per-node-effective-format | author number formats/styling as engine-validated intents |
| `conditional-format-write` | OxFml | new | L | frontier | format-write, per-node-effective-format | author CF rules (results already read in W1) |
| `data-validation-write` | OxFml | new | M | frontier | format-write, reference-resolution-map | author validation rules (state already read) |

### Tenet 4 — Conceivable is one intent; subjects first-class, text the engine's
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `selection-subject-model` | Host | n/a | S | **foundational** | stable-node-identity | multi-select set as one subject for verbs |
| `scope-value` | Host | n/a | M | **foundational** | selection-subject-model | one verb spans node/multi/subtree/collection |
| `replicate-by-id` | OxFml | new | L | high-leverage | scope-value, transaction-scope | fill-by-name; engine rebinds, survives reorder |
| `f4-toggle-binding` | OxFml | extend | M | high-leverage | reference-resolution-map | F4 abs/rel cycle by reference handle |
| `reference-insertion` | OxFml | new | M | high-leverage | reference-resolution-map | point-mode insert; OxFml composes text, host splices |
| `clipboard-transfer-model` | Host | new | M | enriching | scope-value, richer-typed-value | one typed carrier for copy/cut/paste/duplicate |
| `paste-special` | OxFml | extend | L | high-leverage | scope-value, replicate-by-id, format-write, clipboard-transfer-model | paste values / formula / format |
| `duplicate-subtree` | Host | new | L | high-leverage | transaction-scope, replicate-by-id, naming-collision-policy | clone a sub-model, internal refs rebind |
| `add-node-content-policy` | Host | extend | S | enriching | scope-value | new node inherits column formula / template binding |
| `set-membership-write` | OxCalc | extend | M | high-leverage | selection-subject-model | edit a reference collection's members directly |
| `meta-and-attribute-write` | Host | extend | M | high-leverage | transaction-scope | mark a subtree meta (excluded from calc) as a revisioned edit |
| `note-write` | Host | extend | S | enriching | transaction-scope | authored notes/comments that round-trip to Excel |
| `batch-repair-write` | OxCalc | extend | M | enriching | transaction-scope, reference-resolution-map, rename-move-ref-integrity | "fix all broken references" |
| `rename-move-ref-integrity` | OxCalc | extend | M | **foundational** | stable-node-identity, legality-impact-preview, edit-transaction-id | rename/move never silently breaks formulas |
| `template-subsystem` | Host | new | L | enriching | duplicate-subtree, transaction-scope, add-node-content-policy | reusable templates w/ managed instances + drift |
| `table-structural-ops` | OxCalc | new | L | frontier | transaction-scope, replicate-by-id, table-cell-readback | insert/delete row/column, calculated columns |
| `cross-workspace-alias` | OxCalc | new | L | frontier | reference-resolution-map | reference another workspace by stable alias |
| `workbook-export` | Host | new | L | high-leverage | transaction-scope, per-node-effective-format | export to Excel (closes verify-against-Excel loop) |
| `command-palette-metadata` | Host | new | M | enriching | selection-subject-model, legality-impact-preview | palette/menus w/ titles, shortcuts, enablement |

### Tenet 5 — All-or-nothing, reversible only by navigation
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `transaction-scope` | OxCalc | extend | L | **foundational** | — | atomic multi-edit → one revision, one republish *(highest-risk foundational)* |
| `edit-transaction-id` | Host | extend | S | **foundational** | transaction-scope, typed-intent-error | correlate edit→revision; await completion; per-edit failure |
| `revision-graph-retention` | OxCalc | **new** | L | **foundational** | transaction-scope | the retained navigable revision DAG; gates all undo/history/time |
| `undo-redo-revision-nav` | OxCalc | new | M | high-leverage | revision-graph-retention, revision-history-projection, edit-transaction-id | correct undo/redo over overlays/spill; time-scrubber |
| `revision-history-projection` | Skin IR | new | M | high-leverage | revision-graph-retention, edit-transaction-id | history rail / time-scrubber; branch points after undo-then-edit |
| `intent-rebase` | OxCalc | new | L | frontier | intent-conflict-policy, stable-node-identity | collab: re-apply a remote intent onto local head |

### Tenet 6 — Speculation is consequence-free, composable, provenanced
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `candidate-overlay-handle` | OxCalc | **new** | L | **foundational** | transaction-scope, revision-graph-retention | N addressable layerable non-publishing overlays; gates the whole band *(largest risk)* |
| `value-published-pending-flag` | Skin IR | extend | S | high-leverage | candidate-overlay-handle | ghost/real/scenario/external provenance per value |
| `speculation-discard-commit` | Host | extend | M | **foundational** | candidate-overlay-handle, edit-transaction-id | the only two terminal actions: bless to history or drop for free |
| `preview-edit-intent` | Host | extend | L | high-leverage | candidate-overlay-handle, speculation-discard-commit | ghost what-if: downstream ripple w/o committing |
| `scenario-substrate` | Host | extend | L | high-leverage | candidate-overlay-handle | Base/Bull/Bear override sets |
| `scenario-projection` | Skin IR | extend | S | high-leverage | scenario-substrate, value-published-pending-flag | scenario rail tabs + override badges |
| `comparative-multi-overlay-projection` | Skin IR | extend | M | enriching | candidate-overlay-handle, value-published-pending-flag | side-by-side scenario/sweep columns per node |
| `goal-seek-substrate` | OxCalc | new | L | enriching | candidate-overlay-handle | set output→target by varying input, w/ convergence trace |
| `sensitivity-sweep-substrate` | OxCalc | new | L | frontier | candidate-overlay-handle | vary tree-shaped inputs over N points; tornado/spider |
| `speculation-budget-gc` | OxCalc | new | M | enriching | candidate-overlay-handle, pinned-speculative-view | bounded memory for hundreds of transient candidates |
| `pinned-speculative-view` | Host | extend | M | enriching | candidate-overlay-handle, audited-shared-state | keep one what-if on screen while trying another |
| `scenario-persist-migrate` | Host | n/a | M | enriching | scenario-substrate, scenario-projection, skinstate-persistence-exercised | scenarios survive reload (names in SkinState, values in engine) |

### Tenet 7 — Predict before you pay
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `legality-impact-preview` | Host | extend | M | high-leverage | scope-value, recalc-plan-preview, reference-resolution-map, naming-collision-policy, engine-dry-bind | disable illegal ops; "renaming rebinds 7"; pre-commit |
| `engine-dry-bind` | OxFml | new | M | high-leverage | binding-diagnostics-typed | profile/bind legality of an unexecuted edit |
| `recalc-plan-preview` | OxCalc | extend | M | high-leverage | stable-node-identity | "this edit touches 14 nodes, 2 rebinds, 1 cycle risk" over committed graph |
| `drag-gesture-model` | Skin IR | extend | M | high-leverage | legality-impact-preview, selection-subject-model | live drop-target validity during a drag |
| `naming-collision-policy` | Host | new | S | high-leverage | stable-node-identity | predictable auto-suffix / reject / pre-commit collision warnings |
| `value-shape-diff` | OxCalc | extend | L | high-leverage | richer-typed-value, value-epoch-keying, candidate-overlay-handle | which nodes/cells changed between epochs (delta ledger) |
| `import-workbook` | Host | new | L | frontier | transaction-scope, legality-impact-preview | import Excel w/ a dry-run mapping preview |

### Tenet 8 — The frame is composable, negotiated, presentation-tokened
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `multi-slot-composition` | Host | n/a | M | **foundational** | skinstate-persistence-exercised | tree + inspector + what-if panes simultaneously |
| `skinstate-persistence-exercised` | Skin IR | n/a | M | **foundational** | stable-node-identity | view-state survives reload + skin upgrades; gc on NodeKey |
| `design-token-layer` | Skin IR | n/a | M | high-leverage | — | dark / high-contrast / re-skin across all skins at once |
| `a11y-primitives` | Skin IR | n/a | M | high-leverage | stable-node-identity | roving tabindex / ARIA tree / active-descendant for free |
| `capability-manifest-negotiation` | Host | n/a | M | high-leverage | projection-delta-channel | 3rd-party skins fail cleanly when host/profile can't satisfy |
| `shared-focus-arbitration` | Host | n/a | M | enriching | multi-slot-composition, audited-shared-state | select-here-highlights-there; back/forward; breadcrumb |
| `keybinding-registry` | Host | n/a | M | enriching | multi-slot-composition, shared-focus-arbitration, command-palette-metadata | arbitrate/ rebind chords per focused slot |
| `locale-presentation-layer` | Skin IR | n/a | M | enriching | design-token-layer | RTL + translated chrome (not value formatting) |
| `skin-error-isolation` | Host | n/a | M | enriching | multi-slot-composition, capability-manifest-negotiation | a buggy pane can't crash the workspace |
| `empty-state-onboarding` | Host | n/a | S | frontier | template-subsystem, command-palette-metadata | guided first-run / starter templates |
| `collab-presence-markers` | Host | new | M | frontier | intent-conflict-policy, shared-focus-arbitration | presence cursors + advisory edit-claims |

### Tenet 9 — Every interaction is observable, governed, replayable
| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `projection-delta-channel` | Host | extend | L | **foundational** | stable-node-identity | re-render only changed rows; the #1 responsiveness win |
| `projection-version-stamp` | Skin IR | n/a | S | high-leverage | projection-delta-channel | gap-detecting delta sequencing → resync on drop |
| `host-worker-calc` | Host | n/a | L | high-leverage | projection-delta-channel | slow recalc never freezes the frame (engine stays passive) |
| `audited-shared-state` | Host | n/a | M | high-leverage | — | debuggable cross-skin writes; recalc_mode as dispatcher policy |
| `intent-log-replay` | Host | extend | M | high-leverage | projection-delta-channel, edit-transaction-id | deterministic tests, repro bug reports, audit, collab wire format |
| `model-query-projection` | Host | extend | M | high-leverage | stable-node-identity, reference-resolution-map | find a node in 100k: search/filter/refs |
| `readonly-reviewer-persona` | Host | extend | M | enriching | capability-manifest-negotiation, edit-transaction-id, typed-intent-error | reviewer mode enforced per intent origin |
| `backpressure-coalescing` | Host | extend | M | enriching | host-worker-calc | typing/slider-drag doesn't queue 50 recalcs |
| `frame-telemetry-hooks` | Skin IR | n/a | M | enriching | intent-log-replay, projection-version-stamp | attribute a stall to calc vs delta-apply vs render |
| `virtualization-window-projection` | Host | extend | L | enriching | projection-delta-channel, stable-node-identity, model-query-projection | materialise only ~100 visible rows of a 100k tree |
| `external-rtd-value-motion` | OxCalc | new | L | frontier | host-worker-calc, value-published-pending-flag | live external/RTD feeds drive recalc w/o blocking |
| `intent-conflict-policy` | OxCalc | new | L | frontier | revision-graph-retention, intent-log-replay | collab: typed conflict detection/resolution over the revision DAG |

---

## Suite-surfaced additions (ATLAS)

Six additional requirements surfaced while designing the **ATLAS** multi-perspective skin suite
(see [`../skin-suite/`](../skin-suite/)). They are host/skin-layer, mostly enriching/frontier, and
slot into the existing waves (see [`ROADMAP.md`](ROADMAP.md)). Full detail in
[`HOST_AND_SKIN_IR_REQUIREMENTS.md`](HOST_AND_SKIN_IR_REQUIREMENTS.md).

| ID | Owner | Ready | Eff | Tier | Depends | Unlocks |
|---|---|---|---|---|---|---|
| `cleave-predicate-shared` | Host-projection (shared) | extend | S | **foundational** | model-query-projection, audited-shared-state | the filter/cleave **predicate** carries across lens switches — cleave in Ledger, switch to Flow, Flow re-runs it. *Predicate only* (re-applied per lens; set may differ if the model changed); sort/group stay lens-local |
| `shared-focus-set` | Skin IR (shared) | n/a | S | enriching | shared-focus-arbitration | optional cross-lens fade-by-distance focus highlight. **NON-GOAL:** no `Zoom` intent — viewport/camera zoom is skin-local SkinState, never dispatched, never shared (only collapse/pin are shared) |
| `cockpit-preset-registry` | Host-projection | n/a | M | enriching | multi-slot-composition, skinstate-persistence-exercised | named, persisted `SlotLayout` presets (the six flagship cockpits) with per-preset persona + restore-on-open |
| `facade-position-persistence` | Skin IR | n/a | M | enriching | stable-node-identity, skinstate-persistence-exercised | Canvas (x,y)/group geometry keyed on `NodeKey`, surviving reparent/rename/undo; **preserved, not re-projected** (the model never had a position) |
| `replay-authored-artifact` | Host-intent | extend | M | frontier | intent-log-replay, revision-history-projection | save/name/edit a recorded exploration as a shippable walkthrough — Story's "Play" |
| `narrative-projection` | Host-projection | new | M | frontier | richer-typed-value, series-projection, stable-node-identity | block-backed **model-card** projection (a live view over engine truth, never frozen text) |

> **Not new:** ATLAS's "table-authoring reach" is already covered by `table-structural-ops` +
> `table-cell-readback` — the suite confirms **Sheet** as the consumer and ties them to open
> question #9 (a no-grid engine table reconciling rows + columns).

---

*~92 core requirements + 6 ATLAS-surfaced additions, across 9 tenets and 5 layers. The
dependency-sequenced rollout is in [`ROADMAP.md`](ROADMAP.md).*

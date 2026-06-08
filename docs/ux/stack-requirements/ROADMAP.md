# Rollout Roadmap & Waves

Dependency-sequenced waves for pushing the [requirements](FUNCTIONALITY_MATRIX.md) up the stack.
Each wave depends only on earlier ones. Foundational enablers land first; the two big engine
substrates (revision graph, candidate overlays) are isolated into their own gating sub-waves so
their risk is visible and never buried among IR shapes.

---

## The minimum foundational set (push these first)

Twelve requirements that unlock the most. Land them and FLOW plus the existing skins all level up:

`stable-node-identity` · `typed-invalidation-reasons` · `typed-dependency-kinds` · `richer-typed-value`
· `reference-resolution-map` · `per-node-effective-format` · `format-resolver-on-context` ·
`scope-value` · `transaction-scope` · `typed-intent-error` · `edit-transaction-id` ·
`projection-delta-channel`.

## The four gating engine workstreams

These are **new engine capability**, not exposure of existing facts. Each needs an engine spike
before its dependents are schedulable:

1. **`transaction-scope`** (OxCalc) — accumulate-then-publish-once. The engine publishes *per node*
   today (`produce_candidate → PublishReady → publish_and_clear`). Highest-risk foundational item;
   prerequisite for every multi-target verb and the speculation commit path.
2. **`revision-graph-retention`** (OxCalc) — a retained navigable parent-linked revision store with a
   cursor. Only snapshot-*identity hashes* exist now. Gates all undo / history / time-travel /
   collaboration. **(W4a)**
3. **`candidate-overlay-handle`** (OxCalc) — N addressable, layerable, non-publishing overlays. One
   overlay set pinned to publication exists now. Gates the entire speculation band. *The single
   largest risk.* **(W4b)**
4. **`value-epoch-keying`** (OxCalc) — a per-node *published-value* epoch distinct from the per-node
   *input* epoch. Gates shape-diff + per-node staleness. (The delta channel is deliberately decoupled
   from this — it keys on the engine's existing invalidated-node set.)

**The async/passive resolution.** `host-worker-calc` keeps the engine single-threaded and passive
and makes concurrency a *host* worker concern — so "host drives every tick" stays literally true even
with off-main-thread calc. There is no engine-threading workstream.

---

## Waves

### W0 — Identity keystone + free typed-vs-stringly conversions
**Theme:** `NodeKey` first and alone; then the independent zero-risk typing conversions.
**Requirements:** `stable-node-identity`, `typed-invalidation-reasons`, `typed-dependency-kinds`,
`richer-typed-value`, `typed-run-and-calc-state`, `phase-timings-typed`.
**Rationale:** only `stable-node-identity` is a true universal precondition — it gates delta
addressing, persistence gc, selection survival, history correlation. Land it via a **transition
window** (carry both key + path), not a big-bang cutover. The typed conversions are independent,
parallelizable, low-risk `expose`/`extend` work that can land in any order. `value-epoch-keying` is
deliberately moved *out* of W0 (it is engine bookkeeping, not a free thread-through).
**Unlocks:** skins correlate facts across recalcs and structural edits; reason chips, dependency-kind
filters, typed value rendering; the IR stops lying about what it knows *and* the engine stops
branching on its own prose.

### W1 — Value-faithful display & deep read
**Theme:** render values as Excel would; surface the derivation the engine already holds.
**Requirements:** `per-node-effective-format`, `format-resolver-on-context`, `reference-resolution-map`,
`binding-diagnostics-typed`, `runtime-effects-list`, `overlay-resize-deltas`, `per-edge-cache-evidence`,
`full-derivation-trace`, `typed-cycle-diagnostics`, `active-node-detail`, `value-epoch-keying`.
**Rationale:** with typed values present, the resolver (a thin OxFml forward, not a host
reimplementation) + effective format close the raw-debug-text gap; reference-resolution + binding
diagnostics power navigation and inline feedback; runtime/derivation/cycle surfaces turn the calc
X-ray on. `value-epoch-keying` lands here as honest engine bookkeeping. `per-edge-cache-evidence` and
`typed-cycle-diagnostics` are honestly `extend` pending engine confirmation.
**Unlocks:** Excel-faithful display everywhere; FLOW's explain-stack, calc X-ray (effects/cycle
lanes), formula bar with live binding diagnostics, jump-to-definition / find-references; per-node
value epochs for downstream memo/diff.

### W2 — Subjects, transactions, typed errors & safe structural authoring
**Theme:** first-class subjects + atomic publish-once + typed failure + foresight, standing on W0/W1.
**Requirements:** `selection-subject-model`, `scope-value`, `transaction-scope`, `typed-intent-error`,
`edit-transaction-id`, `naming-collision-policy`, `engine-dry-bind`, `recalc-plan-preview`,
`legality-impact-preview`, `drag-gesture-model`, `model-query-projection`, `rename-move-ref-integrity`,
`command-palette-metadata`.
**Rationale:** `transaction-scope` is the highest-risk foundational item (per-node publish today →
accumulate-publish-once is genuine rework). `typed-intent-error` closes a tenet-2 violation hiding in
the receipt and is sequenced *here*, not deferred. The legality/plan preview is a thin host JOIN over
engine `dry_bind` + `plan_invalidation`; `recalc-plan-preview` reads the **committed** graph and does
**not** back-edge into the candidate substrate, so this wave stands on W0/W1 alone.
**Unlocks:** confident structural editing — batch select-and-act, atomic multi-edit, recoverable
typed rejections, pre-commit "this rebinds 7 dependents", live drop-validity, model search,
command-palette enablement, reorganise the tree without breaking formulas.

### W3 — Reference & content authoring verbs
**Theme:** fill, F4, point-mode insert, paste, duplicate, set-membership, meta/notes — all
id/handle-carrying.
**Requirements:** `replicate-by-id`, `f4-toggle-binding`, `reference-insertion`,
`clipboard-transfer-model`, `paste-special`, `duplicate-subtree`, `set-membership-write`,
`meta-and-attribute-write`, `note-write`, `format-write`, `add-node-content-policy`.
**Rationale:** the authoring verbs that compose raw text in OxFml and rebind in OxCalc — each carries
ids/handles/scope, never skin-synthesised text; `reference-insertion` composes text the *host*
splices (caret stays host-side). They depend on scope + transaction (W2) and reference-resolution
(W1). `paste-special` and `duplicate-subtree` are L (they reuse the same hard rebind machinery as
`replicate-by-id`, not cheaper). `format-write` feeds the resolver already in place.
**Unlocks:** Excel-parity fill-down/right, F4 cycling, point-mode reference insertion, paste-special,
subtree duplication, authored reference collections, meta subtrees, notes, number formats.

### W4a — Revision graph *(gating engine workstream)*
**Theme:** build the retained navigable revision DAG the entire time axis depends on.
**Requirements:** `revision-graph-retention`.
**Rationale:** the engine holds snapshot-*identity hashes*, not a retained navigable revision store.
Undo-by-navigation has no substrate today. Isolated into its own sub-wave so its risk is visible.
**Unlocks:** the substrate for undo-navigation, version-history, time-scrubbing, and (later)
collaboration — nothing in the time axis is schedulable before this lands.

### W4b — Candidate substrate *(gating engine workstream)* + time axis
**Theme:** build the addressable candidate-overlay substrate, then provenance, undo & history on top.
**Requirements:** `candidate-overlay-handle`, `value-published-pending-flag`,
`speculation-discard-commit`, `undo-redo-revision-nav`, `revision-history-projection`,
`value-shape-diff`.
**Rationale:** `candidate-overlay-handle` is the single largest engine risk — going from one per-node
publish path + single overlay set to N addressable, layerable, non-publishing contexts is
new-capability scheduler rework, and gates the whole speculation band. Undo/history ride on W4a's
revision graph. `value-shape-diff` inherits `value-epoch-keying`'s risk.
**Unlocks:** consequence-free what-if substrate; ghost-vs-real provenance; commit-to-history bridge;
deterministic undo by navigation; branchable DAG history; the delta-ledger change-pulse.

### W4c — Scenarios & preview UX
**Theme:** the author-facing speculation features riding the proven candidate substrate.
**Requirements:** `preview-edit-intent`, `scenario-substrate`, `scenario-projection`,
`comparative-multi-overlay-projection`, `series-projection`.
**Rationale:** with the candidate substrate proven in W4b, the scenario rail, ghost preview,
comparison columns, and chart-feed series-projection are buildable. Kept distinct from W4b so they
don't appear schedulable before the substrate exists.
**Unlocks:** FLOW's ghost what-if, scenario rail (Base/Bull/Bear comparison columns), comparative
projection, and chart-ready series feeds — explore freely, commit deliberately.

### W5 — Platform robustness
**Theme:** incremental updates, host-side concurrency, multi-pane, theming/a11y/keybinding,
negotiation, observability.
**Requirements:** `projection-delta-channel`, `projection-version-stamp`, `host-worker-calc`,
`backpressure-coalescing`, `frame-telemetry-hooks`, `skinstate-persistence-exercised`,
`multi-slot-composition`, `shared-focus-arbitration`, `keybinding-registry`, `audited-shared-state`,
`design-token-layer`, `a11y-primitives`, `capability-manifest-negotiation`, `readonly-reviewer-persona`,
`intent-log-replay`, `virtualization-window-projection`, `locale-presentation-layer`,
`skin-error-isolation`.
**Rationale:** `host-worker-calc` resolves async-vs-passive by keeping the engine synchronous and
making concurrency a host concern. The delta channel keys on the engine's existing invalidated-node
set (not blocked by per-node value epochs). Multi-slot + focus + keybinding + isolation + tokens +
a11y + manifest deliver the composable frame; persona gates per origin; frame-telemetry makes the
perf goal falsifiable; `audited-shared-state` treats recalc_mode as dispatcher policy.
> **Note — sequencing nuance:** `projection-delta-channel` is in the *minimum foundational set* and
> several of its consumers want it early. In practice land the delta channel + version stamp +
> persistence + design tokens + a11y as soon as W0's `stable-node-identity` exists (they only depend
> on it); the rest of W5 (multi-slot, worker, negotiation, replay) is the later platform hardening.
**Unlocks:** 60fps on 100k-node models with a measurable frame budget; the full FLOW multi-pane
layout; dark/high-contrast/RTL themable accessible skins; arbitrated keybindings; third-party +
reviewer extensibility; deterministic replayable tests; persistent view-state.

### W6 — Frontier: structural reuse, export, external feeds, sweeps, onboarding
**Theme:** templates, tables, aliases, import/export, RTD, sensitivity, rich values — on proven
substrates.
**Requirements:** `template-subsystem`, `table-structural-ops`, `table-cell-readback`,
`cross-workspace-alias`, `import-workbook`, `workbook-export`, `batch-repair-write`,
`rich-image-value-handles`, `scenario-persist-migrate`, `pinned-speculative-view`,
`derivation-trace-for-candidate`, `goal-seek-substrate`, `sensitivity-sweep-substrate`,
`external-rtd-value-motion`, `empty-state-onboarding`, `speculation-budget-gc`.
**Rationale:** reach-extending capabilities on every prior wave. Templates/tables/aliases/import/export
are structural-reuse and the canonical-verification round-trip; `batch-repair` completes the rejection
lifecycle; goal-seek/sweep + RTD are gated behind the proven candidate (W4b) and host-worker (W5)
milestones; onboarding seeds from templates.
**Unlocks:** reusable templates with drift, structured tables, cross-model links, Excel import **and**
export (closing the verify-against-Excel loop), live RTD dashboards, sensitivity/tornado analysis over
a tree, guided onboarding.

### W7 — Research bucket *(not on the sequenced roadmap)*
**Theme:** collaboration conflict semantics and deferred rule-authoring — explicitly off the critical
path.
**Requirements:** `intent-conflict-policy`, `intent-rebase`, `collab-presence-markers`,
`conditional-format-write`, `data-validation-write`.
**Rationale:** collaboration's hard core (conflict policy + intent rebase over the revision DAG) is
research-grade new-capability, not a deferred feature. CF-write and data-validation-write are large
net-new OxFml rule-authoring surfaces with no cited near-term FLOW use (their *result*/*state* reads
already ship in W1). Kept as candidates pending a concrete authoring use, **not** sequenced — this is
what keeps the matrix from diluting into a wishlist.
**Unlocks:** multi-user co-editing with presence (research); full conditional-format and
data-validation authoring (pending demand) — none gating the planned UX.

---

## Open questions / spikes needed

Mostly engine-readiness confirmations — run each spike before committing the dependent requirement.

1. **NodeKey cutover.** The transition-window strategy is committed (carry both key + `display_path`;
   intents accept either) — but the window's exit criteria and how long dual-addressing is maintained
   across projection / intent payloads / `SkinState` / `walking_skeleton` tests need a concrete
   migration plan.
2. **Per-node published-value epoch.** Confirm the engine can stamp each node's *published* value with
   an epoch distinct from the per-node *input* epoch. If only a workspace `value_epoch` + per-node
   input epoch exist, `value-epoch-keying` and `value-shape-diff` are materially larger than tagged
   (the delta channel is already decoupled).
3. **Candidate-overlay reach.** Size the scheduler rework to go from one per-node publish path +
   single overlay set to N addressable, layerable (parented), non-publishing contexts. Single largest
   risk gating the speculation band — needs an engine spike before W4b is schedulable.
4. **Retained revision graph.** Confirm whether a navigable parent-linked revision store with a cursor
   is a bounded extension of the existing snapshot machinery or a from-scratch store with its own
   memory/GC budget.
5. **Passivity model.** Confirm the engine's synchronous calc can be sliced/resumed cooperatively (so
   the host worker can pump it without blocking a frame), or whether bounded-slice pumping itself
   requires engine reentrancy work.
6. **Per-edge cache outcomes.** Confirm the scheduler exposes per-edge `Hit/Miss/Bypassed`, or only an
   edge-value-cache *basis* fingerprint. `per-edge-cache-evidence` is `extend` pending this.
7. **General iterative convergence.** Confirm whether OxCalc has a general iterative solver producing
   per-iteration `max_change` for arbitrary cycles, or only Excel-match cycle *fixtures*.
   `typed-cycle-diagnostics` and `goal-seek-substrate` both depend on the answer.
8. **Delta-only mode.** The delta is additive (alongside the full snapshot) today. At 100k nodes skins
   will want delta-*only*. Is the synchronous full-republish guarantee mandatory forever, or is there
   a sanctioned resync-on-gap mode (triggered by `projection-version-stamp`)?
9. **Table without grid coords.** `table-structural-ops`/`table-cell-readback` walk close to the
   no-grid constraint. Is there an engine notion of "table" reconciling rows *and* columns without
   reintroducing 2D coordinate addressing?
10. **Durable scenario store.** Does the engine model have a durable, Excel-round-tripping place for
    named scenario overrides (scenario-manager parity), or do override values live only in transient
    runtime overlays — in which case durability is itself new engine capability?
11. **Collaboration conflict model.** Which model (rebase over the revision DAG vs reject vs merge)
    does the immutable-revision substrate naturally support, and does NodeKey-keyed structural rebase
    compose with content/formula and collection-membership rebase without a separate merge algebra?
12. **Persona source of truth.** Does persona come from host config, capability-manifest negotiation,
    or an external auth layer — and for remote peer intents, how is the originating peer's persona
    authenticated at the single dispatcher chokepoint?
13. **Cockpit preset defaults.** `multi-slot-composition` gives the mechanism, but the *curated
    default compositions-per-modality* (which lens + companions per task) are an open design question —
    captured as `cockpit-preset-registry`.

---

## Suite-surfaced additions (ATLAS) — where they land

The six [ATLAS-surfaced additions](FUNCTIONALITY_MATRIX.md#suite-surfaced-additions-atlas) slot into
the existing waves:

- **W2** — `cleave-predicate-shared` (it is *sold as continuity*, so it ships with the early
  shared-state/spine layer; it depends on `model-query-projection`, which is W2).
- **W5** — `shared-focus-set`, `cockpit-preset-registry`, `facade-position-persistence` (platform /
  composition-era state; ride `multi-slot-composition` + `skinstate-persistence-exercised`).
- **W6** — `replay-authored-artifact`, `narrative-projection` (Story's narrative + authored-replay
  surfaces, on top of W5 `intent-log-replay`).

This maps onto the ATLAS rollout phases: **Phase A** (mono-lens core, W0–W3 + the early-W5 subset)
carries `cleave-predicate-shared`; **Phase B** (the W5 cockpit) carries the three composition-era
fields; **Phase D** (narrative + tables, W6) carries the two Story fields. The ATLAS suite ships its
single-slot mono-lens core *first* and the cockpit/companions/scenarios/time-travel/Story arrive only
as their gating waves land — see [`../skin-suite/`](../skin-suite/).

# DNA TreeCalc — Skinning Stack Requirements

**Purpose.** A principled, layer-assigned requirement set that pushes *truth up the stack* so the
skinning layer gains deep, robust reach **without ever owning semantics**. It exists to be rolled
out to the upstream lanes (OxCalc, OxFml) and implemented in the host/Skin-IR here, in a
dependency-sequenced order. It is the requirements companion to the UX work in
[`../flow-skin/`](../flow-skin/) and the architecture in [`../SKINS.md`](../SKINS.md).

This set was synthesised from the gap inventory surfaced while designing the **FLOW** skin
(see [`../flow-skin/`](../flow-skin/)) plus a fresh four-band architecture pass, then run through
doctrine-ownership, completeness, and over-reach critics. The raw multi-agent synthesis is
preserved verbatim at [`reference/REQUIREMENTS_SYNTHESIS.raw.json`](reference/REQUIREMENTS_SYNTHESIS.raw.json)
— every requirement field (shape, justification, dependencies, readiness) is there as provenance;
these docs are the curated, push-ready view.

---

## North star

> A passive, Excel-faithful calc engine (OxFunc / OxFml / OxCalc) feeds a host projection
> (DnaTreeCalc) consumed by **frame-only** skins through a closed intent dispatcher. This set pushes
> truth up the stack so skins gain reach without owning semantics: a stable identity spine, lossless
> typed projections of facts the engine already derives, atomic id-carrying authoring verbs, a
> consequence-free speculation/time axis, and a composable, governed platform.

The ordering principle is **push-up-the-stack-first**: land the keystone identity and the *free*
typed-vs-stringly conversions, then fan out — and honestly distinguish "expose existing engine
truth" (cheap) from the handful of genuine engine workstreams the speculation/time/async bands
actually require.

---

## The nine tenets (the principled spine)

Every requirement maps to one of these. They are the *why*; the matrix and roadmap are the *what*
and *when*.

1. **Identity is permanent, path is cosmetic.** Every node is addressed by a stable opaque `NodeKey`
   (the engine's existing `TreeNodeId`), invariant across rename/move; the dotted `display_path` is a
   derived label. All projections, deltas, selection, scope, references, view-state, persisted
   `SkinState`, history, and collaboration key off `NodeKey`. *The keystone* — without it, every
   rename/move silently orphans selection, pins, persisted state, deltas, and history correlation.

2. **Typed and lossless over stringly and flattened.** Every fact the engine already derives crosses
   the IR boundary intact, as closed exhaustive Rust enums/structs keyed by stable identity — never
   `String` prose to grep, a bare count, or a collapsed `Scalar(String)` — **including the
   write-failure channel** (`IntentError`). Where the engine emits an open taxonomy (e.g. phase-timing
   keys), the IR uses a typed-key map with an explicit `Other(String)` escape, never host-side
   re-classification.

3. **Engine is the sole source of derivation, value, and format.** Explanations, traces, caches,
   schedules, overlays, cycle terminals, conditional-format results, number-format meaning, and
   rendered value text are *read* from the engine that produced them, never reconstructed host- or
   skin-side. The format/value resolver on `SkinContext` is a **thin forward** into OxFml's renderer
   (it MUST NOT parse number-format codes host-side). *Formats describe the value; skins describe the
   frame.*

4. **Conceivable is one intent; subjects are first-class, text is the engine's.** Every articulable
   change is exactly one entry in the closed `WorkspaceIntent` enum, applied to a typed host-owned
   **subject** (one node, an ordered multi-selection, a subtree, a reference collection) carried in
   the intent — never a skin-stitched sequence and never synthesised formula text. Fill, F4,
   paste-formula, column-formula, and reference building all travel as ids/handles; OxFml recomposes
   raw text and OxCalc rebinds. Caret/edit-buffer concerns stay host-side.

5. **All-or-nothing, reversible only by navigation.** A single conceptual edit — even spanning many
   nodes — commits atomically as one transaction producing one published revision and one projection
   update, or not at all; its undo is achieved by navigating OxCalc's retained immutable revision
   **graph**, never by replaying inverse intents. Every receipt carries the transaction id and the
   revision it produced.

6. **Speculation is consequence-free, composable, and provenanced.** Every what-if, scenario,
   goal-seek, and sweep runs on handle-addressed candidate overlays that are *structurally incapable*
   of touching the published revision stream; overlays compose and stack; published `value_epoch`
   advances only on an explicit `Commit`. Every projected value carries provenance
   (`Published | Pending | Speculative{handle} | Scenario{id} | External{source}`) so a ghost can
   never be mistaken for real.

7. **Predict before you pay.** Any consequential intent is answerable in the interrogative mood
   before the imperative: the same dispatcher that mutates can report — without mutating — what an
   intent *would* invalidate, collide with, rebind, orphan, cost, and reject. The host computes only
   its own fields (collisions, orphans, scope expansion); profile-legality and rebind impact are
   *delegated* to engine dry-bind / plan-invalidation and surfaced verbatim.

8. **The frame is composable, negotiated, and presentation-tokened.** Multiple skins mount
   simultaneously into distinct slots over one workspace/selection/shared-state source of truth with
   defined focus, keybinding arbitration, and per-slot fault isolation; each skin declares a
   capability manifest the host negotiates at mount; all visual/locale/accessibility presentation
   flows through typed tokens on `SkinContext`.

9. **Every interaction is observable, governed, and replayable.** Mutation funnels through one
   dispatcher chokepoint that enforces persona/permission per intent origin, records the
   `(intent, receipt, delta, revision)` stream as a deterministic replayable log, and emits minimal
   addressable projection deltas alongside the full snapshot. Long calc moves to a **host-owned**
   worker (the engine stays single-threaded and passive; the host pumps bounded slices and observes a
   `Pending` run-state) — the frame never blocks, polls, or owns scheduling.

---

## The ownership boundary (non-negotiable)

A requirement must land at the layer that **owns its truth**:

| Layer | Owns |
|---|---|
| **OxFunc** | function/operator semantics, the `EvalValue` value universe, coercion, array lift, error algebra |
| **OxFml** | grammar, parse, **bind**, single-node eval, LAMBDA/LET, number/format-code parsing, CF rule semantics |
| **OxCalc** | multi-node scheduling, dependency graph + invalidation, candidate/accept/reject + publication, epochs, runtime overlays, cycles/iteration, **immutable revisions** |
| **Host (DnaTreeCalc)** | the `WorkspaceState` projection, the closed `WorkspaceIntent` dispatcher, structural editing, templates, meta-nodes, model↔Excel mapping. Owns **no** engine/semantics |
| **Skin** | frame only — render the projection, hold typed view-state, dispatch intents. Owns **no** truth |

Hard rules every requirement respects: skins never parse/bind/rewrite formula text; mutation only via
the closed intent enum; no grid/coords/A1-ranges/spill-block (multi-value = one node's `Array` or a
reference *collection*); values/formats/dates/errors/LAMBDA are engine-owned and rendered as-is;
the capability profile gates the input surface at parse/bind; undo = revision navigation, never
inverse replay; `is_meta` subtrees are invisible to formulas.

---

## Reading order

1. **This README** — north star, the nine tenets, the ownership boundary.
2. [`FUNCTIONALITY_MATRIX.md`](FUNCTIONALITY_MATRIX.md) — the capability-domain × stack-layer grid and
   the complete catalog index of every requirement (id, layer, readiness, effort, tier, dependencies).
3. [`ROADMAP.md`](ROADMAP.md) — the dependency-sequenced waves (W0–W7), the **minimum foundational
   set**, the **four gating engine workstreams**, and the open questions/spikes.
4. [`ENGINE_REQUIREMENTS.md`](ENGINE_REQUIREMENTS.md) — full detail (Rust shape + justification +
   readiness + deps) for every **OxCalc** and **OxFml** ask. *This is the doc to roll out to the
   sibling lanes.*
5. [`HOST_AND_SKIN_IR_REQUIREMENTS.md`](HOST_AND_SKIN_IR_REQUIREMENTS.md) — full detail for every
   **Host-projection**, **Host-intent**, and **Skin-IR-contract** ask. *Implemented here.*
6. [`reference/REQUIREMENTS_SYNTHESIS.raw.json`](reference/REQUIREMENTS_SYNTHESIS.raw.json) — the raw
   multi-agent synthesis, verbatim, for provenance.

---

## The two headlines

**Push these first — the minimum foundational set (12).** Land them and FLOW + the existing skins all
level up:
`stable-node-identity` · `typed-invalidation-reasons` · `typed-dependency-kinds` · `richer-typed-value`
· `reference-resolution-map` · `per-node-effective-format` · `format-resolver-on-context` ·
`scope-value` · `transaction-scope` · `typed-intent-error` · `edit-transaction-id` ·
`projection-delta-channel`.

**The four gating engine workstreams** (genuinely *new* engine capability, not mere exposure — each
needs a spike before its dependents are schedulable):

1. **`transaction-scope`** — accumulate-then-publish-once (the engine publishes *per node* today).
   Highest-risk foundational item.
2. **`revision-graph-retention`** — a navigable parent-linked revision store (only snapshot-*identity
   hashes* exist now). Gates **all** undo / history / time-travel.
3. **`candidate-overlay-handle`** — N addressable, layerable, non-publishing overlays (one overlay set
   pinned to publication exists now). Gates **all** what-if / scenario / goal-seek. *The single
   largest risk.*
4. **`value-epoch-keying`** — a per-node *published-value* epoch (only a workspace epoch + per-node
   *input* epoch exist). Gates shape-diff + per-node staleness.

Plus the framing fix that dissolves the apparent async-vs-passive contradiction: **the engine stays
single-threaded and passive; concurrency is a *host* worker concern** (`host-worker-calc`) — so
"host drives every tick" stays literally true even with off-main-thread calc.

---

## Downstream consumer: the ATLAS skin suite

This requirement set is consumed by two UX efforts in [`../`](..): the **FLOW** lens
([`../flow-skin/`](../flow-skin/)) and the **ATLAS** multi-perspective skin suite
([`../skin-suite/`](../skin-suite/)). ATLAS surfaced **six additional requirements** — folded in as the
[*Suite-surfaced additions*](FUNCTIONALITY_MATRIX.md#suite-surfaced-additions-atlas) and sequenced in
[`ROADMAP.md`](ROADMAP.md). The most important is `cleave-predicate-shared` (the filter/cleave
predicate as shared continuity, W2/spine); the rest are W5 composition state and W6 narrative surfaces.
ATLAS's rollout maps onto these waves: its single-slot mono-lens core ships on W0–W3, the cockpit on
W5, time-travel/speculation on W4, and the narrative (Story) lens on W6.

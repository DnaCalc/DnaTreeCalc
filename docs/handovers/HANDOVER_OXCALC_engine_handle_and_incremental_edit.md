# HANDOVER_OXCALC_engine_handle_and_incremental_edit

Status: Open
Target: OxCalc
Ask: Commit the **host-driven engine-handle + incremental-edit interaction shape** as the consumer-contract direction (not a deferred "session" widening lane), confirm the **sans-executor / host-as-executor** stance, and confirm the forward-compat properties (steppable, cancellable, executor-model-agnostic) so the future async / parallel / GPU engine is not foreclosed by the V1 surface.
Context: The consumer contract packages V1 as one-shot `Document`+`Request` execution and parks "session, incremental, or driven-host packaging" as a later widening lane (§6.5, §9). For DNA TreeCalc the host↔engine *interaction shape* is a central API-design decision, not a versioned rollout — it underpins incremental editing, undo, RTD, and the long-term parallel/async engine. OxCalc already states it owns custody of the tree-model structure and that the host loads/creates/updates it through the contract (consumer §3; W051 §2.1, §3) — which is exactly a handle / open-then-update model. We want that made explicit and its V1 surface shaped as the thin first slice of the handle model rather than a path that gets replaced.
Evidence: OxCalc `CORE_ENGINE_OXCALCTREE_CONSUMER_INTERFACE_AND_HOST_CONTRACT_V1.md` §3 (boundaries), §4–§6 (facade), §6.1 (no ambient mutable state / no smuggled scheduler), §6.5 (one-shot; later widening), §9 (V1 scope); OxCalc `W051_SPARSE_RANGE_READERS_AND_DEFINED_ENTRY_SEMANTICS.md` §2.1/§3 (tree-model custody). DNA TreeCalc `CORE_MODEL_SPEC.md` §1 (model & formula authority rule), §7 (recalc/calc-state), §8a (undo), §6; `HANDOVER_OXCALC_undo_versioning.md` (pinned-version substrate); `REQUIREMENTS.md` §1.3 + `SKINS.md` Trace E (host-side RTD).

## 1. The interaction model

OxCalc is a **passive, synchronous, sans-executor library**, not a service:

- It owns no thread, no event loop; nothing ticks or progresses between calls; it never calls out (no engine→host callbacks).
- A **handle** holds everything — tree model, published values, dependency graph, pinned versions. The **host owns the handle's lifetime; OxCalc owns the handle's internal shape and is the only thing that mutates it.**
- Every advance is a synchronous host call against the handle: open, edit, recalc (F9), external-value update (RTD), async completion. The host drives; the engine yields and resumes.
- This is the sans-I/O / poll-based-state-machine pattern — explicit state in, result out, no ambient runtime — and is consistent with consumer §6.1 ("must not smuggle scheduler", "must not hide behind ambient mutable state"): the handle is explicit, not ambient.

## 2. Lifecycle / call surface (the ask)

A handle + incremental-edit surface, all synchronous:

- `open(document) -> handle` — pins initial structural truth (the existing `OxCalcTreeDocument` is the natural seed).
- typed edit calls against the handle — `update_node` / `set_formula` / structural edits / `set_external_value` — each producing a **new pinned version** (candidate → publication / epoch).
- `recalc` / step against the handle (see §4).
- `close(handle)`.

We are **not** asking for the full surface at once. We are asking that V1's one-shot facade be explicitly the **first slice of this handle model** (open + one edit-batch + recalc), so the host does not build against a shape that is later replaced.

## 3. Completion mechanism for async work (anticipate; build later)

When a recalc cannot complete synchronously (an async function, later), OxCalc must not block, spawn, or `.await`. It **suspends and surfaces a completion descriptor as return-value data** — which node awaits which token — and returns. The host owns the async work and **resumes** with a synchronous call. So:

- the result gains a **`Pending` / `AwaitingCompletion`** run-state alongside `Published` / `VerifiedClean` / `Rejected`, carrying completion tokens;
- **F9, RTD updates, external streaming values, and async-function completions are one mechanism** — host-driven completion of pending engine work; F9 is the degenerate already-complete case;
- no engine→host callbacks: pending tokens, invalidation, and results all return as data; the host re-enters.

Forward-compat only — async functions are not on the v1 path — but the contract shape should leave the door open.

## 4. Forward-compat: parallel / GPU and an async interface (do not foreclose)

Sans-executor must **not** be read as "single-threaded forever." It constrains *ownership of the runtime*, not *use of parallelism*:

- OxCalc may fan a recalc across all cores and the GPU **within a host-driven call/step**, provided the **executor is host-supplied or call-scoped** (structured concurrency, joined before return) — never an ambient runtime that ticks between calls. An engine-held worker pool is acceptable only if injectable and idle between calls, so the WASM host can substitute Web Workers / WebGPU while the native host supplies rayon / wgpu.
- Long recalcs are **steppable + cancellable**: `step(handle, budget, cancel) -> Progress { published_so_far, pending, done? }`; progress returns as data; **cancellation = abandon the in-flight candidate, keep the last publication** (the pinned-version model makes this safe).
- Determinism comes from the dependency frontier + atomic publication; safe concurrent read-during-recalc comes from immutable published versions.
- **An async interface is then a free surface:** because the core is a steppable, cancellable, executor-agnostic state machine, the interface can be surfaced synchronously (`step`) or as `async fn` (host's executor drives, `.await` re-enters) with no semantic change — `async`/`await` *is* the formalized sans-executor pattern, and future-cancel-by-drop maps onto candidate-abandonment. Async is also the WASM-portable surface.

The single discipline that buys all of this: keep recalc **steppable, cancellable, and executor-agnostic** (no internal `block_on`, no owned runtime), and keep published versions immutable / snapshot-able.

## 5. System-of-record split

Per consumer §3 and W051 §3, **OxCalc owns custody of the canonical calc tree-model**; DNA TreeCalc owns the **product workspace** (meta-nodes, formats, skin state), **persistence**, and **edit orchestration**. Pin the sync contract:

- host edit → handle update call → host reads results;
- how the host obtains engine-held values / calc-state / pinned-version identity for rendering and persistence;
- whether the host keeps a structural mirror or projects from the handle (and, if a mirror, the reconciliation rule).

## 6. Implications for the current V1 framing

- Reframe one-shot `Document`+`Request` execution as the **first slice of the handle model**, not a separate path.
- The **"driven-host packaging" widening lane is mis-scoped**: driving is the host's job. OxCalc needs synchronous edit / update / resume entry points and steppable recalc — not a push / scheduler / callback mechanism.
- Confirm **no engine→host callbacks** anywhere in the contract (this also retires any `subscribe_invalidation`-style idea on the DNA TreeCalc bridge).

## 7. What we're asking for

1. Confirm the **engine-handle + incremental-edit interaction shape** as the contract direction, with V1's one-shot facade as its first slice.
2. Confirm the **sans-executor / host-as-executor / no-callbacks / no-ambient-runtime** stance against the coordinator/publication model.
3. Confirm the **pinned-version semantics** underpin both incremental edit and undo (tie to `HANDOVER_OXCALC_undo_versioning.md`) and make cancellation safe.
4. Confirm the **forward-compat properties** — steppable, cancellable, executor-supplied/scoped, executor-model-agnostic (sync or `async fn` surface) — so the parallel / GPU / async future is not foreclosed by V1.
5. Accept (or counter) the **`Pending` run-state + completion-token** shape for async work as anticipated contract, even if unimplemented.
6. Pin the **system-of-record / host↔engine sync contract** (§5).

## Expected disposition

Part **confirm** (OxCalc's ownership statements and §6.1 constraints already imply the passive handle model; cancellation/versioning align with the undo handover), part **coordinate / re-sequence** (lifting the handle + incremental-edit surface from "later widening" to the agreed contract direction, and shaping the V1 facade as its first slice), part **design** (the `Pending` / completion-token shape and the steppable / cancellable recalc entry). No request to build the parallel / async engine now — only to not foreclose it.

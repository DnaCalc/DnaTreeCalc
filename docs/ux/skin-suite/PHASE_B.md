# Phase B — The Cockpit Era: plan of record

The detailed Phase-B plan for the ATLAS suite, refreshed after B.1 shipped and
the OxCalc passivity spike landed. Companion to the rollout table in
[`README.md`](README.md), the spine contract in [`SPINE.md`](SPINE.md), and the
stack requirements in [`../stack-requirements/`](../stack-requirements/).

> Sequencing headline: the passivity spike
> (OxCalc `docs/spec/core-engine/CORE_ENGINE_HOST_WORKER_PASSIVITY_SPIKE.md`)
> answered the worker question (run-to-completion, **zero engine changes**) but
> discovered that **engine recalc cost — not threading — is the scale
> blocker** (quadratic+ cold, warm no-op runs 10–80× slower than cold).
> Phase B therefore runs *governance-and-reach work in parallel with the
> engine lane's performance workstream*, and lands the worker only when
> time-to-result is worth moving off-thread.

---

## B.1 — Composition core ✅ (shipped 2026-06-10)

| Requirement | What shipped |
|---|---|
| `audited-shared-state` | `SharedSkinStateHandle::apply(SharedStateChange, SharedStateOrigin)` — typed, attributed (Shell / Host / Lens{skin, slot}), 256-entry audit ring; all suite mutation sites migrated; `update()` kept as a documented host-internal escape hatch |
| `multi-slot-composition` | Main + RightInspector + BottomConsole slots; per-slot `ErasedSkinContext` (and `SkinContext.slot`); per-workspace persisted `CockpitLayout` with a non-reactive *owner* guarding saves across workspace switches |
| `capability-manifest-negotiation` (slot subset) | `SkinCapabilities.allowed_slots` (+`None` = Main-only), `RegisteredSkin::negotiate_slot`, fail-loud fallback cards; `companion_slots_active` publishes **validated** occupancy only |
| `cockpit-preset-registry` (built-ins) | Solo / Modeling / Author / Audit presets with graceful degradation to registered skins |
| `shared-focus-arbitration` (first slice) | `focused_slot` + `hovered` in shared state; click-to-focus with ring; focus clamps to Main when a slot closes |
| Lens ↔ companion re-projection | `NodeInspector`/`ConsoleBar` gate **themselves** on companion occupancy (`standalone=true` for the companions) — every lens stands embedded copies down for free |
| `skin-error-isolation` (honest slice) | Unknown-id / refused-slot fallbacks; **wasm panics still abort** — full isolation is not promised (see B.3) |

**Known limitations carried forward:** Ctrl+1..9 still indexes the full
registry order while the switcher tabs filter to Main-capable skins (slots 8/9
reach legacy skins; companions are not keyboard-reachable — acceptable, they
compose via presets/toggles); `SplitLeft`/`SplitRight` slots exist in the enum
but have no shell chrome yet.

---

## B.2 — Time-to-result + flow control

**Goal:** edits feel instant and recalculation never blocks the frame, at
thousands of nodes. **Gate:** the OxCalc performance workstream (bead
`calc-ekq3`) — worker plumbing without it just moves a 4-minute calc
off-thread.

### B.2.0 — Engine performance workstream *(OxCalc lane; gates the rest)*

**Round 1 landed on OxCalc main 2026-06-11** (bead `calc-ekq3`, merge
`ab86126`, results in the spike doc's "Performance workstream round 1"):
cache-basis + snapshot-id digesting, a per-run name-resolution index,
prepared-formula retention across runs, and Arc-shared revision retention.
All four original targets below were addressed; the warm-recalc OOM is gone.

**Round 2 landed on OxCalc main 2026-06-12** (merge `009a157`): the w056
`host_name_bind_results` O(N²) cartesian was bounded (owner-signed-off
diagnostic-content change — each formula now carries bind results only for
names present in its source text) plus byte-identical identity-string
streaming. 10–30× on every harness leg; n=5000 — unmeasurable in round 1 —
now runs (cold 64.6 s, warm 8.2 s, incremental 7.1 s; n=1000: 2.80 s /
0.54 s / **0.48 s incremental**).

**Acceptance against the criteria: still 1 of 3 — the gate stays closed,
but the margins collapsed.**
- ✅ Warm strictly cheaper than cold (5.2× at n=1000, 7.9× at n=5000).
- ❌ chain n=5k cold ≤ ~1 s: 64.6 s, ~60× over (down from ~3 orders).
  Residual: per-formula `OxfmlFormulaEvaluation` cost still grows ~linearly
  with N (1.8 → 8.8 ms/formula from n=1000 to n=5000).
- ❌ Incremental ∝ dirty set: re-evaluation itself is proportional (3 ms @
  n=1000) but total wall scales ~N^1.7 on untimed per-node overhead inside
  `EvaluationLoopTotal`.

Round-3 targets: the cold-path per-formula evaluation quadratic, phase-timer
coverage for the untimed evaluation-loop overhead, build-session wall
(per-add snapshot clones). Note for the host: at n≤1000 the engine is now in
interactive territory (sub-second warm/incremental) — B.2.2's payoff case is
models above that.

Original targets (for the record): `EdgeValueCacheLookup` warm cost,
`DiagnosticSeedCollection` scaling, per-run `OxfmlPrepareFormulas`
re-preparation, consumer `recalculate` prelude/postlude cloning.

**Acceptance (re-run the retained harness,
`OxCalc tests/host_worker_passivity_spike.rs`, now sized n=[1000, 2000]):**
chain n=5k cold ≤ ~1 s release; warm strictly cheaper than cold; incremental
cost proportional to the dirty set, not N. *DnaTreeCalc's role: re-run the
harness as the gate check; raise observations via the handover convention.*

### B.2.1 — Serializable IR seam ✅ *(shipped 2026-06-11)*
`Serialize`/`Deserialize` across the whole Skin-IR surface (~100 projection
types + every intent/receipt/delta type). `profile` became `String`;
`PhaseKeyProjection` serializes as its stable-id string (manual impls — JSON
map keys cannot carry `Other(String)`); 11 round-trip tests pin the wire
shapes. Format remains `serde_json` first; `postcard` if profiling demands.

### B.2.2 — Worker session *(host; gated on B.2.0)*
- The wasm web worker **owns** the `TreeWorkspaceSession`; the main thread
  keeps a projection mirror and a dispatcher proxy (intent → postMessage →
  receipt + delta back). Native/desktop later uses a thread (the context has
  no `Rc`/interior mutability — movable).
- `run_state` gains `Pending { token, started_value_epoch }`; dispatch returns
  immediately; superseded runs are **versioned and discarded on arrival**
  (this is the cancellation story — no engine hook needed).
- Trunk worker build wiring (`data-type="worker"`).

### B.2.3 — Delta-only + resync *(host; pairs with the worker)*
ROADMAP open question 8: shipping full snapshots over `postMessage` per publish
is the real boundary cost.
- UI side applies `WorkspaceDelta` onto a retained snapshot; `projection_seq`
  gap or `FullReset` ⇒ request a fresh snapshot (resync-on-gap).
- The delta channel + version stamp already exist; this adds the apply/resync
  protocol and a delta-coverage audit (every projection field a lens reads must
  be representable in a delta or trigger a reset).

### B.2.4 — Flow control + telemetry *(host)*
- `backpressure-coalescing`: dispatcher queue; coalesce superseded
  `EditContentDeferred` per `NodeKey`; drop/queue recalc while `Pending`;
  `Coalesced{into_seq}` receipts.
- `frame-telemetry-hooks`: `FrameMetric { intent_seq, dispatch_to_delta_us,
  delta_apply_us, render_us, dropped }` into the replay sink — makes the
  60fps goal falsifiable.

**B.2 exit criteria:** on a 5k-node model — type an edit, see `Pending`
provenance immediately, published values arrive without main-thread jank;
boundary traffic is delta-only in steady state; telemetry graphs the frame
budget; the spike harness numbers meet the B.2.0 acceptance.

---

## B.3 — Governance + reach *(host/framework; parallel with B.2.0, no engine gate)*

In rough priority order:

1. **Preview seam on `SkinContext`** ✅ *(shipped 2026-06-11)* —
   `PreviewService` (dry-bind + mutation impact) carried optionally on
   `SkinContext`; the live dispatcher forwards to the session's `preview_*`
   methods under the session mutex; the shared `NodeInspector` shows live
   per-keystroke legality — now threaded through ALL seven lens inspectors.
   *Remaining:* adopt mutation-impact previews in Tree/Capture/Sheet
   affordances (pre-commit collision/orphan warnings).
2. **`readonly-reviewer-persona`** ✅ *(first slice shipped 2026-06-11)* —
   `Persona { Author, Reviewer, ReadOnly }` with a closed policy over the
   closed intent enum; the dispatcher gates every intent and rejects with
   typed `Forbidden{persona}`; switching travels as the audited `SetPersona`
   intent (shell selector + Console chip). *Remaining:* per-origin policies
   (remote peers), `allowed_intents` surfacing in the command catalog so
   lenses pre-disable affordances.
3. **`intent-log-replay`** ✅ *(shipped 2026-06-11)* — every dispatch recorded
   as a typed `IntentRecord` (outcome, transaction id, revision, value epoch,
   persona); serde-exportable; `replay()` re-dispatches with divergence
   tracking. Proven: a recorded session replays onto a fresh fixture with zero
   mismatches and identical published values. *Remaining:* per-slot intent
   origins once slot-scoped dispatch proxies exist (B.2.2 territory).
4. **Multi-select promotion** ✅ *(shipped 2026-06-11)* —
   `WorkspaceIntent::SelectNodes { keys, anchor }`: host-validated (stale key
   = typed rejection), mirrored into shared state, allowed for all personas;
   Ledger dispatches instead of writing view-state.
5. **Keybinding registry v2** — per-user remapping persisted in audited shared
   state, per-slot overrides, surfaced in the command catalog (the registry
   API was built for this).
6. **Cockpit reach** — user-defined presets (persisted, editable), split slots
   (`SplitLeft`/`SplitRight`) chrome for side-by-side lenses, keyboard slot
   cycling, and the remaining ATLAS W5 fields (`shared-focus-set` cross-lens
   hover highlight is half-wired: `hovered` exists, no lens renders it yet).
7. **Hardening + polish** — `skin-error-isolation` beyond fallback cards
   (documented wasm panic limits; defensive Result paths; per-slot remount
   affordance), `locale-presentation-layer` (chrome strings + direction only),
   a11y audit of the cockpit chrome, `virtualization-window-projection`
   (only meaningful after B.2.0 fixes time-to-result).

---

## Phase C/D pointers (unchanged, for orientation)

Deeper speculation UX (ghost what-if inline in Flow, goal-seek, value-shape
change-pulse) stays gated on goal-seek/value-shape-diff substrates and scenario
durability (ROADMAP Q10); Story/Canvas and Sheet table-authoring depth remain
Phase D on W6. The corpus-failure fixes (one real `@PREV/@NEXT` engine
regression, one transaction-semantics conflict, stale artifacts) run in the
engine lane independently of this plan.

## Standing verification floor (every B tranche)

`cargo test -p dnatreecalc-skin-framework -p dnatreecalc-skins -p
dnatreecalc-shell` + host `walking_skeleton` + `programmable_skin_ir`
(`-j 1 --no-fail-fast`; 8 corpus failures are known-pre-existing) + wasm build
+ clippy-clean on suite crates + a preview-browser smoke of the touched
surfaces (`.claude/launch.json` → `trunk serve`, port 8421).

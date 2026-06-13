# DESIGN PROPOSAL — Performance-Proving Discipline (perf lens)

## 1. Structure: paired implementations + the Perf Register

**Every optimization ships as a permanent pair.** The reference twin is product code, not scaffolding: GridCalc-Ref (tracecalc extension, designs-verification §3), the scalarizer lowering (designs-calcgraph invariant 3), eager-renumber insert/delete (designs-storage §Mutation), per-cell reverse edges (vs interval index), per-cell prepare (vs template prepare-once), `BTreeMap` cell storage (vs adaptive blocks). Selection is a runtime enum on the harness (`--engine reference|optimized|both`), both compiled into the same release binary so a differential run is one process. **Merge rule: no optimization lands without (a) an expand-and-compare oracle against its twin (the Invariant Register row, designs-verification §7) and (b) a Perf Register row.**

**The Perf Register** — `OxCalc/docs/spec/core-engine/CORE_ENGINE_GRID_PERF_REGISTER.md`, companion table to the Invariant Register, one row per optimization:

| field | content |
|---|---|
| id / invariant link | `P-xx` ↔ `I-xx` (cross-link mandatory: every perf claim has an equivalence oracle, every compressed structure has a perf claim) |
| claim | complexity + constant, falsifiable: "invalidation O(dirty-rects·log n + k); ≤17 B/cell on boring-1Mx10" |
| ref twin | which paired implementation expands/checks it |
| workload(s) | named corpus ids (§4) |
| measured | ref number, optimized number, ratio, retained run-id |
| regression budget | a **counted** bound, never wall-clock |
| status | claimed → measured → **bound** (counter gate live) → retired |

The move that reconciles per-PR gating with TECHNICAL.md §7.6's "no clock-time gates" (`docs/ux/TECHNICAL.md:558`) and the documented ±25% wall variance (`OxCalc/docs/spec/core-engine/CORE_ENGINE_HOST_WORKER_PASSIVITY_SPIKE.md:184`): **gates are deterministic counters** (bytes/cell, prepare count, cells evaluated, edges visited, bytes/frame); wall-clock is recorded evidence in retained baselines only. Counters can't flake; clocks can.

**Retro-seed the register with calc-perf round 1.** The four merged fixes (`1955c8d`, `6a2cca0`, `aa8eb26`, `64e144f`; spike doc :117-132) already have claim/workload/number shape — make them rows P-01..P-04 so the register starts with proven exemplars, and the round-2 residuals (spike doc :166-173) become open rows. One register spans tree and grid lanes.

## 2. Harness: extend, don't fork

What exists: the closed-form scale runner (`OxCalc/src/oxcalc-core/src/treecalc_scale.rs`, CLI `oxcalc-tracecalc-cli -- treecalc-scale`, profiles in `docs/test-runs/core-engine/treecalc-scale/README.md`) emitting `run_summary/phase_timings/validation_summary/model_profile.json` with retained runs + `BASELINE_2026-05-04.md`; the `#[ignore]`d spike harness (`OxCalc/src/oxcalc-core/tests/host_worker_passivity_spike.rs`) whose phase timers drove round 1; the §7.6 doctrine; `docs/test-corpus/perf/` is *named in doctrine but does not exist yet* (verified — no `perf/` under `DnaTreeCalc/docs/test-corpus/`).

Grid perf work = **new profiles in the same runner** (generalize the name to `calc-scale`). Additions:

- Two new artifacts per run: `counter_summary.json` (cells evaluated, prepares, edges visited, rects propagated, bytes by layer, publication entries) and `register_assertions.json` (per touched P-row: bound, measured, pass/fail).
- `--engine both` runs ref + optimized and emits the sampled-readout equivalence diff in the same run — perf evidence and equivalence evidence from one artifact set.
- Phase vocabulary extends the existing seven (README.md:102-110): `block_build`, `template_prepare`, `rect_propagation`, `tile_publication`.
- Engine side: a `CalcRunCounters` struct on the run outcome (the phase-timer seam at `treecalc.rs:788` proves the hook exists). Counters are the gateable surface; ship them unconditionally — they're increments.
- Host side: intent-log replay (`src/dnatreecalc-skin-framework/src/intent.rs:683,712`) as the session-workload driver, per designs-verification §6; Doom and viewport workloads run here through the tile protocol.

## 3. The catalogue (claim → workload → measurement)

| id | claim | workload | counter measured (vs ref twin) |
|---|---|---|---|
| P-10 blocks | ≤17 B/cell dense, ≤85 B/cell adversarial, blank = 0 bytes (designs-storage table) | boring-1Mx10, zig-zag-1M, full-column | bytes-by-layer vs `BTreeMap<coord,CalcValue>` (>200 B/cell) |
| P-11 template prepare-once | prepare_count == templates, not cells (kills `treecalc.rs:798-811` per-node cost) | fill-down-1M (1 template), enron-mix | prepare counter vs per-cell prepare |
| P-12 rect propagation + interval index | invalidation O(dirty-rects·log n + k) | edit-storm on boring-1Mx10; sum-pyramid | edges visited + seeds vs per-cell reverse-edge expansion |
| P-13 FAP/TACO compressed reverse edges | support bytes O(regions); queries ≡ expanded graph | fill-down R[-1]C-1M; sum-pyramid (Sestoft §3.3 O(N²) blowup case) | graph bytes + expand-and-compare |
| P-14 plan-cache hit rate | hits ≥ (cells−templates)/cells steady-state | `--recalc-rounds` amplification (README.md:18) | hit/miss counters |
| P-15 tile streaming | bytes/frame ≤ k·subscribed-cells, independent of model size (designs-display Doom bound) | doom-320x200 @30Hz over 1M-cell backing | bytes/frame counter |
| P-16 visible-first | cells evaluated before P0-complete ≤ \|upstream closure of visible rects\| | viewport-64k-of-1M, deep deps | evaluation counter; time-to-visible-clean vs full recalc recorded |
| P-17 insert/delete | O(log n) positional mapping; blocks touched ≤ boundary + log n | insert-storm-1M | blocks-touched vs eager renumber O(n); guards against re-importing the 337s rebind pathology (`BASELINE_2026-05-04.md:23`) |
| P-18 partition witness | same-level regions have disjoint read/write rects (witness only; execution deferred) | boring, zig-zag | witness validity + max-parallelism bound recorded now |
| **P-19 (own) warm no-op O(dirty)** | no-edit verify visits 0 cells on non-volatile sheets — generalizes round 1's hardest-won fix; tripwire for the 10–80× warm pathology | all profiles, warm pass | cells-visited == 0 |
| **P-20 (own) occupancy-proportional aggregates** | `SUM(A:A)` slots visited == occupied cells, never 2^20 (storage I4) | full-column, zig-zag | reader slots-visited counter |
| **P-21 (own) COW retention ∝ delta** | revision retention bytes ∝ touched blocks (extends `aa8edd26`/`aa8eb26` Arc-share to grid layers) | edit-storm with retention on | retained-bytes growth counter |
| P-22 incremental publication ∝ delta | grid computed-layer publication never full-N — the round-2 `CandidatePublication` residual, solved structurally by the storage lens's publication bypass | edit-storm | publication-entries counter |

## 4. Workloads

Create `DnaTreeCalc/docs/test-corpus/perf/` (host/session workloads: doom, viewport, intent-replays) and `OxCalc/docs/test-corpus/grid-perf/` (engine workloads). **Descriptors retained, fixtures generated** — follow the treecalc-scale precedent ("does not check in million-node fixtures", README.md:9): each named workload is a small JSON of generator params + closed-form expectations. Names: `boring-1Mx10` (Excel's ~37 B/cell + 0.57 s comparison point), `zig-zag-1M`, `sum-pyramid-N`, `deep-chain-N` (same shape as the calc-ekq3 spike chain — keeps numbers comparable across rounds), `fill-down-1M`, `flash-fill-region` (host-declared never-materialized region — covers owner decision 4's second authorship mode), `doom-320x200`, `insert-storm-1M`, `viewport-64k-of-1M`, `enron-mix` (4.5% unique-formula ratio as a generator parameter). Excel timing comparison (§7.6 third bullet) applies to the exportable subset (boring, sum-pyramid) via the EXCEL_EXPORT path + OxXlPlay `RecalcTrigger` once construction exists.

## 5. Cadence and gates

- **Per-PR:** counter gates only, at `perf_smoke_*` scale (precedent dirs exist; seconds to run) for register rows the PR touches, plus the ≤1e5-cell differential. Runs in OxCalc CI for engine rows, DnaTreeCalc for host rows.
- **Per round (calc-ekq3 model):** retained full-scale acceptance run → `docs/test-runs/core-engine/grid-scale/BASELINE_<date>.md` in the 2026-05-04 format (table, phase split, observations, semantic-binding note), recording commit, machine, variance caveat.
- **Round-2 interaction:** grid work must not *wait* on the residuals — P-22 sidesteps `CandidatePublication` structurally; the w056 O(n²) diagnostics sign-off (spike doc :168) blocks tree-lane rows only. The one true shared dependency is persistent incremental graph maintenance (designs-calcgraph open Q4): assign a single owner before either lane builds it.

## 6. Hardest problems + derisking

**(a) Keeping the reference fast enough to be an oracle.** Strategy: (1) **naive in structure, not in constants** — ref shares the leaf evaluation stack (OxFml eval, OxFunc) with the optimized engine; only the *machinery* (storage, graph, scheduling) differs. That holds ref within ~10–100× instead of 10⁴×, and is safe because leaf semantics are separately oracled by TraceCalc/Excel. (2) **The ref gets its own register row**: cells evaluated == occupied cells exactly once (Sestoft visited/uptodate discipline) as a counter gate — accidental superlinearity in the ref silently shrinks the differential tier, so gate it like a product. (3) **Cone-sampled differential above 1e5**: pick N probe cells in a big workload, materialize only their upstream cone in the ref, compare. Circularity risk (cone derived from the optimized engine's own graph) is mitigated by over-approximating cones to whole regions/rows and validating cone extraction itself on small cases. (4) Closed-form expectations (existing `validation_summary.json` pattern) remain the full-scale third leg. (5) Hard independence rule: ref may share only spec vocabulary + leaf eval — any "speedup" to the ref that imports optimized structure correlates failures and is rejected.

**(b) Counter fidelity drift** — counters measuring the wrong thing as code moves. Derisk: each counter is asserted against a closed-form expectation on at least one workload (e.g., edges-visited on sum-pyramid has an exact formula), so a silently broken counter fails validation, not just gating.

**(c) Wall-clock evidence credibility** at ±25% variance. Derisk: retained baselines always report ref/optimized *ratios* alongside absolutes; rounds re-run the previous round's headline rows on the same machine before claiming improvement (the round-1 doc already models this, spike doc :145-147).

## 7. Build first / defer

**First:** (1) `CalcRunCounters` + `counter_summary.json` in the existing runner; (2) GridCalc-Ref + its own register row + ref scaling curve as the *first* retained baseline (defines the differential tier honestly); (3) Perf Register doc seeded with round-1 rows + P-10/P-11/P-19 (storage bytes, prepare-once, warm no-op); (4) `boring/zig-zag/deep-chain/fill-down` generators; (5) `--engine both` differential mode. **Defer:** Excel timing comparison (needs export path), Doom/tile rows until the tile protocol exists (but reserve P-15 now), partition-witness execution, cone-sampling (until >1e5 workloads exist).

## 8. Open questions

1. **One register or per-lane?** Recommend one (`CORE_ENGINE_GRID_PERF_REGISTER.md` absorbing calc-ekq3 evidence currently living in the spike doc) — confirm, since it moves calc-perf's record-keeping.
2. **Reference machine policy:** sanction a single named baseline machine + pinned toolchain for retained wall-clock runs, or accept variance with ratio-only claims?
3. **`CalcRunCounters` in release builds permanently** (recommended — increments are free) or feature-gated?
4. **Gate ownership:** does the per-PR counter gate for host-side rows (tiles, replay) run in DnaTreeCalc CI even when the row's code is in OxCalc?
5. **Excel timing tier:** wait for the file-I/O repo's export path, or hand-author the `boring-1Mx10` comparison workbook now to get §7.6's Excel-timing leg early?
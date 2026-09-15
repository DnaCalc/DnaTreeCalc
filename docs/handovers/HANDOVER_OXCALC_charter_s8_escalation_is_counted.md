# Handover — OxCalc: CHARTER §8 "a conservative full rebuild is legal only when it is counted" + the lane outcome and `rebuild_from_input` as public, countable facts

Status: Open
Target: OxCalc (`CHARTER.md` §8; `src/oxcalc-core/src/consumer.rs`)
Ask: (1) amend CHARTER §8 by one sentence so a conservative full rebuild is legal only when it is counted, reported, and owned by a bead naming its incremental successor; (2) expose the seeded lane's `GridSeededLaneOutcome` and every `rebuild_from_input` as public, countable facts on the verbs' outcomes (the accessor `calc-1q03.15` already tracks, plus the rebuild event).
Context: DnaTreeCalc W012 bead `dtc-5zet.14` (escalation counters on the live path, `Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1 "Make escalation cost something" and §4 "Escalation is never free"). Host-core now keeps a per-session `RecalcAccounting` (`src/dnacalc-host-core/src/recalc_accounting.rs`) and `dnacalc-conform` prints `escalation_rate` beside coverage on every `diff` and `bench` run. The accounting counts exactly what the host can observe today, and the gap it cannot close is this handover's ask.

## The one-sentence charter amendment

CHARTER §8 today (the sentence in force):

> Conservative full rebuild remains legal when compatibility cannot be proven, but the architecture must leave room for local structural identities, dependency components, publication shards, and subtree hashes to prove that unaffected regions can be retained safely.

Proposed, from the Foundation note (§1, move A; §4 the standing rule), as an added sentence directly after it:

> A conservative full rebuild on the live path is legal only when it is **counted** (a typed outcome the consumer can read per recalc — `GridSeededLaneOutcome`, and a `rebuild_from_input` event), **reported** beside coverage by the instrument that measures the live path, and **owned** by a bead naming its incremental successor.

Why the sentence and not a guideline: the survey found five escalation reasons typed and honest, every uncertain path escalating, nothing failing for escalating, and no published number anywhere getting worse — a licence with no accounting. The charter is where the licence is granted, so the charter is where the accounting has to be attached to it.

## What the host counts today, and what it cannot

Measured through `WorkbookSession` (the product path) on the OxCalc HEAD at `5ea74767` (calc-1q03.10 and calc-1q03.2 closed; calc-1q03.15 open; consumer.rs line numbers below are from `eadb8c55` and approximate):

| fact | host-observable? | how it is counted |
|---|---|---|
| a recalc happened (entry/clear verb under Automatic; F9 that drained; the load's open-recalc) | yes | verb outcomes, `WorkbookRecalcOutcome::drained`, `WorkbookLoadReport::engine_recalcs_at_load` |
| the F9's seeded-pass `cells_evaluated` per drained sheet | yes | `WorkbookRecalcOutcome` |
| the rebuild's own mark-all before the seeded pass (F9, Automatic Delete) | **no** — counted **by verb contract** (`recalculate_workbook` ~8670: "rebuilt as a pure function of its authored input"; `clear_grid_cell` ~5787) as `contract_rebuilds` | a lower bound: correct as long as the two verbs keep that contract; the canary is `dnacalc-conform`'s `bench_tests.rs` (warm F9 `cells_evaluated == 2` on the fill-down) |
| `GridSeededLaneOutcome` per recalc (`Incremental` / `NoRetainedValuation` / `BasisMismatch` / `NotFullCoverage` / `GraphNotInstalled` / `TopologyGrowth`) | **no** — private `GridDerivedState::last_lane_outcome` (~1611) | every recalc lands in `lane_outcomes.unobserved`; `escalation_rate` is `null` |
| `cells_evaluated` of an Automatic inline edit | **no** — the verb returns a view | not counted |
| sheets recalculated by cross-sheet propagation inside one transaction (and their outcomes) | **no** | not counted; the host diffs peer projections to find them (`peer_grid_projections`) |
| a same-address overwrite riding `Incremental`; a new-address write escalating `TopologyGrowth`; the propagation re-entering the origin with `topology_grew_since_recalc = true` (~11051) | **no** | stated as code-reading expectations in the host-core tests, never asserted as observations |

So the number the instrument prints is honest and blunt: `escalation_rate UNOBSERVABLE (lane outcomes engine-reported 0 / N)`, with `escalation_rate_lower_bound = contract_rebuilds / recalcs`. On the in-tree Foundation corpus (109 workbooks, all `Automatic`) `diff` issues no recalc after the open — the forced F9 is a no-op on every file — so both rates are undefined there and the 89 open-recalcs are reported apart; on a `Manual` load the forced F9 is one drain = one contract rebuild (lower bound 1.0 by construction, saying nothing about the incremental lane). The bench's Automatic edits are where the rate would move, and there it reads `unobserved` (run-wide lower bound 0.3243 = 12 F9 drains / 37 recalcs on the default shape, 2026-09-15).

## Observed while counting: a failed F9 drops the sheet's seeds

`recalculate_workbook` takes the accumulated seeds (`std::mem::take`) before it rebuilds the drained grid; when the rebuild's own recalc fails typed (`OxfmlEvaluationFailed` under the F9 path's `Abort` policy — `SCN-EK011-FUNCTION-RECOGNITION.xlsx`, `D1 = COPILOT(1)`, which the open-recalc degrades per calc-1q03.10 but the F9 path does not, calc-1q03.19), the `?` returns before `grid.derived = rebuilt`, so the OLD derived state survives with its seeds already taken. Observed from the host (2026-09-15, host-core test `a_failed_f9_is_counted_apart_and_drops_the_seed` on an in-memory `=COPILOT(1)` under Manual, and on the corpus file): `has_undrained_edits` flips to `false`, the next F9 is a no-op, and the edited cell keeps its `Stale` publication with no way to drain it. Host-core counts these F9s as `f9_failures`, apart from drains and no-ops. Filed in DnaTreeCalc as a sibling finding beside this handover; the fix belongs with calc-1q03.19 (per-cell degradation on the F9/edit path) or as its own bead: put the seeds back (or leave them in place until the rebuild succeeds) when the drain aborts.

## The ask, concretely

1. **Land `calc-1q03.15`** as specified there (`RecalcFacts { tick_id, lane_outcome, cells_evaluated, rebuilt_from_input, phase_timings, differential_mismatches }` per grid, and a workbook-level aggregate per drained sheet), or the outcome-carried form the earlier handover asked for (`HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`, ask 2): the lane outcome and `rebuilt_from_input` ON `GridCellEntryOutcome` (the edited sheet plus one entry per propagated sheet) and ON `WorkbookRecalcOutcome` (per drained sheet: the rebuild's mark-all `cells_evaluated` and the seeded pass's, separately). Host-core's `LaneOutcomeCounters` has one typed field per variant waiting for it; when it lands, `unobserved` goes to zero and `escalation_rate` becomes exact (DnaTreeCalc `dtc-5zet.22` wires it).
2. **Make `rebuild_from_input` an event, not a contract.** Today the host counts it from the verb's documented behaviour; a change to `recalculate_workbook` or `clear_grid_cell` (W063-7, "Delete/Manual incremental") would silently make `contract_rebuilds` wrong. A `rebuilt_from_input: bool` (or a count) on the outcomes removes the inference.
3. **The charter sentence above**, in §8, so that the licence and the accounting are in the same paragraph.

Not asked here: changing any escalation (W5 / W063-7 own that). This handover is about making them visible; the earlier handover (`HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`) remains the technical spec for the accessor and is not duplicated.

Evidence: `DnaTreeCalc/src/dnacalc-host-core/src/recalc_accounting.rs` (the counters and the acceptance tests on the W011 fixture, the two-sheet demo and a Delete — each recording `unobserved`); `DnaTreeCalc/src/dnacalc-conform/src/escalation.rs` (the summary printed beside coverage); `DnaTreeCalc/docs/test-runs/w012/ESCALATION_2026-09-15.md` (the numbers observed on the Foundation corpus and on the bench shape).

# W012 escalation-counter run — 2026-09-15 (dtc-5zet.14)

The first `dnacalc-conform` runs that print `escalation_rate` beside coverage, from host-core's new per-session `RecalcAccounting` (`src/dnacalc-host-core/src/recalc_accounting.rs`; the same counters ride the Skin IR as `WorkbookCalcProjection::recalc_accounting`). Every number below is copied from the retained artifacts `target/w012/run-5zet14/divergence.v1.json` and `target/w012/bench-5zet14/bench.v1.json` (uncommitted; the two commands reproduce them; their stdout is retained beside them as `*.stdout.txt`). Nothing is estimated, and — the point of this run — nothing is inferred: the engine exposes no `GridSeededLaneOutcome` to the host, so **the escalation rate is unobservable on every recalc**, and the artifacts say so in a typed field rather than print a guess.

```
cargo build --release -p dnacalc-conform --offline
./target/release/dnacalc-conform.exe diff  --corpus ../Foundation/research/runs --out target/w012/run-5zet14     # exit 0
./target/release/dnacalc-conform.exe bench --corpus ../Foundation/research/runs --out target/w012/bench-5zet14   # exit 0, 87.1 s
```

## 1. Sibling state (what the numbers are pinned to)

| | |
|---|---|
| DnaTreeCalc | `e5335f5` plus this bead's commit |
| OxCalc | `5ea74767`, tree clean at build start (calc-1q03.10 "open never aborts" and calc-1q03.2 "LocaleFormatContext reaches both grid lanes" committed; calc-1q03.15 — the lane-outcome accessor — open). `optimized_sheet.rs` started being edited again by its agent at 03:48:53, after `oxcalc-core`'s release rlib was compiled (03:38:10) from the clean tree |
| OxFml | `1fc6da8`, one untracked test file only |
| OxFunc | `c5571ac` **plus 48–50 uncommitted working-tree files** (oxf-xvt5.13, the lazy-argument profile axis on `FunctionMeta`, in flight) — compiled as found |
| OxDoc | `c879b08`, tree clean at build start (`baf05f9` by the end of the bench, a beads-only commit) |
| rustc | 1.94.1 (e408947bf 2026-03-25), `--release`, `debug_assertions false`; AMD Ryzen 7 4800H |
| Corpus | `../Foundation/research/runs`, 109 workbooks, set sha256 `d33530acd8c7e4d552aa671116c0be430c3e44dbe8b84839427940e8b9f741d6` — the same set as run-001 (dtc-5zet.8) and run-5zet12 |

**Method note (learned the hard way this bead).** The path dependencies compile whatever is on disk *at the moment cargo reads it*. An earlier release build of this bead (03:25) compiled `oxcalc-core` while its agent was rewriting `optimized_sheet.rs` / `calc_ref_sheet.rs` (mtimes 03:25:37–03:26:20, `git status` clean before and after), and that binary reported a reference-twin VALUE mismatch on `SCN-EK011 E17 = DOLLAR(1)` (reference `"R1.00"`, optimized `"$1.00"`) and an F9 failure `locale_format_context unavailable` on `SCN-EB025 B8`. A rebuild from the committed `5ea74767` with every source mtime before the build start reproduces **neither**. Both were discarded as build artefacts, not findings; the rule from here: check sibling `git status` AND source mtimes against the build start before retaining a number.

## 2. `diff` — the corpus, beside coverage

```
COVERAGE 0.9622  = engine_calculated 407 / formula_cells 423
ESCALATION escalation_rate n/a (0 recalcs); lower bound n/a; verbs: automatic_entries 0 automatic_clears 0
  f9_drains 0 (sheets 0, cells_evaluated 0) f9_noops 109 f9_failures 0 manual_edits 0 clear_noops 0;
  open_recalcs 89 (sheets 89, not in the rate)
  escalation per workbook (retained in the artifact): 0 of 109 opened workbooks issued a recalc after the open
```

| | run-5zet12 (2026-09-15, dtc-5zet.12) | this run |
|---|---|---|
| OxCalc | `7b7e0a1b` + uncommitted calc-1q03.10 | `5ea74767` (calc-1q03.10 + calc-1q03.2 committed) |
| Workbooks opened / open_failed / unreadable | 109 / 0 / 0 | 109 / 0 / 0 |
| **Coverage** | 0.9338 = 395 / 423 | **0.9622 = 407 / 423** |
| not_evaluated | 28: file_cached 21, external_reference_pinned 7 | 16: file_cached 9, external_reference_pinned 7 |
| excluded | 26 | 30: ca=1 17, TODAY 3, no_file_cache 3, NOW 2, RANDARRAY 1, oracle_unknown_error_code 4 |
| match / mismatch / unclassified | 312 / 57 / 0 | 321 / 56 / 0 |
| reference_twin rows | 1 (`H18 = =_xlfn.LAMBDA(1)`, RichOnly) | 1 (the same row; dtc-5zet.26) |
| **recalcs after the open** | not counted | **0** — every one of the 109 files is `calcMode` Automatic; the forced F9 finds nothing to drain (109 `f9_noops`) |
| **open-recalcs** | not counted | **89** (one `recalculate_workbook` per Automatic load with a calc-bearing sheet, 89 sheets drained); the other 20 files bound no formula (13 have no formula cells; 7 carry only external-reference pins) |
| escalation_rate / lower bound | — | **n/a / n/a** (0 recalcs) |

The coverage move (0.9338 → 0.9622; `file_cached` 21 → 9) is calc-1q03.2 landing through the path dependency mid-bead — recorded as dtc-5zet.30, feeding the dtc-5zet.19 re-baseline; not this bead's doing and not a baseline (OxFunc's tree was mid-edit).

What the `diff` accounting says about the campaign's question: **nothing yet**, and honestly so. On this corpus the only host verb after the open is the forced F9, which is a no-op on an Automatic load, so `recalcs == 0` and both rates are undefined; the 89 open-recalcs are cold by definition (a fresh backing has no retained valuation to fall back from) and are reported apart. The 62 `Manual` files the Foundation census counted live outside this in-tree set; on those the forced F9 is one drain = one contract rebuild per file (lower bound 1.0 by construction — a fact about the F9 verb, not about the incremental lane). The bench is where the rate would move.

## 3. `bench` — the default shape, per subject

Subject selection changed since the baseline (`BASELINE_2026-09-15.md`): the three largest formula-cell files now **open** on the product path (calc-1q03.10), so the subjects are `SCN-EK011-FUNCTION-RECOGNITION` (212 formula cells), `reason_code_wave1` (14) and `SCN-EB025-FUNCTION-CORE` (13) instead of the three 9–10-cell probes; `skipped_open_failed` is empty. That is a campaign effect on the bench's subject set (dtc-5zet.30 records it with the coverage move).

| subject | recalcs | contract_rebuilds | lower bound | lane outcomes observed | verbs |
|---|---|---|---|---|---|
| corpus `SCN-EK011-FUNCTION-RECOGNITION` | 5 | 0 | 0.0000 | 0 / 5 | 5 Automatic entries; **3 F9s FAILED** (`f9_failures 3`, §4); 3 Manual edits; open-recalc 1 |
| corpus `reason_code_wave1` | 8 | 3 | 0.3750 | 0 / 8 | 5 entries; 3 F9 drains (3 sheets, seeded pass 8 cells each); open-recalc 1 |
| corpus `SCN-EB025-FUNCTION-CORE` | 8 | 3 | 0.3750 | 0 / 8 | 5 entries (a new literal — topology growth by construction, unobserved); 3 F9 drains (1 cell each); open-recalc 1 |
| synthetic fill-down 2,000 | 8 | 3 | 0.3750 | 0 / 8 | 5 entries; 3 F9 drains (cold 4,000 cells, warm 2 + 2); 4,002 Manual edits (the setup) |
| synthetic cross-sheet 500 | 8 | 3 | 0.3750 | 0 / 8 | 5 entries; 3 F9 drains over 4 sheets (cold 1,000, warm 500 + 500 — the origin's mark-all, as the baseline recorded); 1,002 Manual edits |
| **run-wide** | **37** | **12** | **0.3243** | **0 / 37** | 25 entries, 12 F9 drains (13 sheets, 6,031 seeded-pass cells), 3 F9 failures, 5,013 Manual edits, 3 open-recalcs |

```
ESCALATION (run-wide) escalation_rate UNOBSERVABLE (lane outcomes engine-reported 0 / 37; OxCalc exposes no
  GridSeededLaneOutcome — calc-1q03.15); lower bound 0.3243 = contract_rebuilds 12 / recalcs 37
```

Reading the lower bound: 12 of the 37 recalcs the bench issued are full rebuilds **by the F9 verb's contract** (every drained sheet is rebuilt from authored input and recalculated in full before the seeded pass whose `cells_evaluated` the outcome reports). The other 25 are Automatic single-cell edits whose lane outcome — `Incremental` for the same-address overwrites, `TopologyGrowth` for `SCN-EB025`'s new literal, and the cross-sheet subject's propagation re-entering the origin with a mark-all — is exactly what the campaign wants counted and exactly what the host cannot see. The true rate is somewhere in `[0.3243, 1.0]`; the instrument prints the bound and the gap, never a point inside it.

Wall-clock (this run's mins vs the baseline's Run A bands, for the record — this bead is not the wall-clock bead): fill-down cold F9 6,952.9 ms (band 3,970–6,616), warm F9 4,680.6 (2,301–3,834), Automatic edit 177.4 (110–184, inside); cross-sheet cold F9 2,395.3 (1,208–2,014), warm F9 856.3 (406–676), Automatic edit 715.0 (401–669). Every counter exact (4,000 / 2 / 1,000 / 500). Five of six numbers sit above their band, in a session where three sibling agents were running cargo builds on the same machine throughout (the baseline's rule forbids that for a regression call; an earlier run at 03:39, same binary, had the fill-down inside every band). Not called a regression here; dtc-5zet.19 / .22 / .24 own the re-cut. Peak working set 442.7 MiB (band ≤ 551).

## 4. Observed on the live path while counting

1. **A failed F9 drops the drained sheet's seeds** (dtc-5zet.29, sibling OxCalc). `SCN-EK011`'s `D1 = COPILOT(1)` is degraded to `FileCached` by the open-recalc (calc-1q03.10) but the F9 path still runs under `Abort` (calc-1q03.19 open), so every `recalculate()` after a Manual edit fails typed `OxfmlEvaluationFailed`. Probed from the host: before the F9 the sheet reads dirty; after the failure it reads clean, a second F9 is a no-op, and the edited `A1` keeps its `Stale` publication. Cause by code reading: `recalculate_workbook` takes the seeds before the rebuild and the rebuild's `?` returns before `grid.derived = rebuilt`. Pinned in-tree by host-core's `a_failed_f9_is_counted_apart_and_drops_the_seed` on `=COPILOT(1)` under Manual; counted as `f9_failures`, apart from drains and no-ops.
2. **The three F9 cycles on that subject measure nothing** — 21–24 ms each is the cost of failing, not of recalculating — and the bench records them with their error, as designed (dtc-5zet.13).
3. **Everything the bead asked to count per reason is `unobserved`**: `LaneOutcomeCounters` has a typed field per `GridSeededLaneOutcome` variant and every one is 0 with `unobserved == recalcs` (37 / 37 on the bench). The code-reading expectations the bead named — edit `A1` on the W011 fixture → `Incremental`; the two-sheet demo → `TopologyGrowth` on every authoring write; a Delete → `NoRetainedValuation` on the rebuilt sheet — are stated in the host-core tests' doc comments as expectations and asserted nowhere. The ask is `docs/handovers/HANDOVER_OXCALC_charter_s8_escalation_is_counted.md` (with the one-sentence CHARTER §8 amendment) on top of `HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md` (OxCalc `calc-1q03.15`); dtc-5zet.22 wires the accessor when it lands.

## 5. What this run does and does not prove

- Proves: every recalc the live host path causes is now counted, typed, at the session (`RecalcAccounting`), on the wire (`WorkbookCalcProjection::recalc_accounting`, carried by every `CalcStateChanged` — and since this bead by every accepted entry/clear receipt, which closed dtc-j7n8.20's mirror lag), and in both instrument artifacts beside coverage, with the identities `recalcs == automatic_entries + automatic_clears + f9_drains`, `contract_rebuilds == automatic_clears + f9_drains`, `lane_outcomes total == recalcs` checked before either artifact is written.
- Proves: the lower bound is real work — 12 full rebuilds in 37 recalcs on the default bench shape — and that the F9 verb rebuilds every drained sheet by contract is the fact the campaign's "escalation is never free" rule needs made countable rather than read from a doc comment.
- Does not prove: any per-reason escalation count. `escalation_rate` is `null` everywhere, by design, until OxCalc reports the lane outcome.
- Does not claim: a wall-clock regression or a coverage baseline (§1's OxFunc state; §3's concurrent builds).

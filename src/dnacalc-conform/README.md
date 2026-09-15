# dnacalc-conform

The W012 measured-Excel conformance instrument (`Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §3 W1, Appendix A; bead dtc-5zet.8). A TC-tier lib + bin that runs a **provenance-first, cell-by-cell Excel-vs-DNA differential** over a directory of Excel-authored `.xlsx` files, offline, through the path the product runs.

```
cargo run -p dnacalc-conform --offline -- diff --corpus <dir> --out <dir>
```

Per workbook: `std::fs::read` → `dnacalc_host_core::WorkbookSession::open_xlsx_bytes_with_validation(.., GridEngineValidationMode::DualValidated)` (OxDoc `LoadProfile::full()` + OxCalc ingest; the same open path the product runs, with the engine's reference twin on every recalc instead of the interactive host's 1-in-16 sample — see "The reference twin" below) → `session.recalculate()` **unconditionally**, whatever the file's `calcMode` (62 of the 174 Excel-authored corpus files are `manual` and would otherwise contribute zero coverage) → for every formula cell the file carries, the verdict ladder → `<out>/divergence.v1.json`.

## The primary number

**coverage = engine_calculated / formula_cells**, corpus-wide and per workbook, printed first on stdout and written first in the artifact. "Green" must always mean *we evaluated X% of the corpus and matched*, never *we matched what we chose to evaluate*. A class count is never printed without its denominator.

- `formula_cells` (the denominator) counts every `CellPayload::Formula` in OxDoc's event stream — **never** the engine's authored view, where an OxFml-rejected formula reads back as `Empty` (dtc-5zet.6). A workbook the engine refuses to open still contributes its formula cells, counted from an engine-free OxDoc open of the same bytes (dtc-5zet.5).
- `engine_calculated` (the numerator) counts formula cells whose published provenance is `PublishedValueProvenance::Calculated { tick_id }`.
- Identities the runner checks before writing the artifact: `engine_calculated == match + mismatch + excluded` and `formula_cells == engine_calculated + not_evaluated`.

## The verdict ladder — provenance first, always

```
0. the workbook did not open                     -> not_evaluated:open_failed:<kind>
1. provenance of the published cell:
     nothing published at the address            -> not_evaluated:not_published
     FileCached, refined by the load report      -> not_evaluated:bind_degraded |
                                                    external_reference_pinned |
                                                    not_calc_modeled | manual_mode |
                                                    file_cached
     Degraded                                    -> not_evaluated:degraded
     Stale                                       -> not_evaluated:stale
1b. Calculated, and the engine's reference twin disputes
    the cell (differential_mismatches names it)   -> mismatch:ReferenceTwin |
                                                    mismatch:ReferenceTwin:RichOnly
2. Calculated, formula calls NOW/TODAY/RAND/RANDBETWEEN/RANDARRAY
   or the file marks the cell ca="1"             -> excluded:nondeterministic:<trigger>
3. Calculated, no scorable cache                 -> excluded:no_file_cache |
                                                    oracle_unknown_error_code |
                                                    oracle_unsupported_payload:<detail>
4. Calculated with a cache                        -> match | mismatch:<class>
```

Only rung 4 produces `match`. A `FileCached` value that equals Excel's cache is the false green (`dnacalc-host-core/src/xlsx_false_green.rs`) and stops at rung 1 by construction — the fixture test `false_green_rejected_cell_is_not_evaluated_bind_degraded_and_control_matches` pins that.

Provenance is read at the OxCalc level (`WorkbookSession::grid_view`, the engine's `PublishedValueProvenance`), not through the skin-IR mirror, which folds `Degraded` into `FileCached`.

### The reference twin (rung 1b, W012-1.5 / dtc-5zet.12)

OxCalc ships a second, brute-force **reference** grid engine beside the optimized one and compares their readouts after a recalc (`grid/machine/differential.rs`, `compare_grid_engine_readouts`); the result is `OxCalcTreeGridView::differential_mismatches`. The interactive host samples it (`DualValidatedSampled { one_in: 16 }`) and, until this bead, collapsed the list to a boolean and discarded the rows. The instrument runs every session under **`DualValidated`** — the twin on **every** recalc — because under a sampled mode an empty list can mean "not checked this recalc" (the engine keeps the last checked result and exposes no per-recalc "lane ran" fact to the host), so only the every-recalc mode makes an empty list mean *checked and clean*. The published values the ladder scores come from the optimized lane in both modes; the mode changes only the oracle spend.

What the instrument does with the engine's rows — it never re-compares the lanes itself:

- every row is a `reference_twin.mismatches[]` entry on the workbook (sheet, A1, both rendered values, `core_equal`, both rich-payload kinds, `formula_cell`), whether or not the file carries a formula cell at the address, so nothing the engine reported is dropped;
- a row on a **`Calculated`** formula cell is that cell's verdict: `mismatch:ReferenceTwin` when the two `CoreValue`s differ, `mismatch:ReferenceTwin:RichOnly` when the cores are equal and only the rich payload differs (see the class table); the verdict row carries the oracle lane's value as `twin_reference`; no Excel comparison is made — the engine disagrees with itself;
- a row on a cell the ladder stopped at rung 1 (a pin) leaves that verdict alone — provenance decides before value does, and a pinned value is not the engine's — but the row is still listed and counted;
- `totals.reference_twin_mismatches` / `totals.reference_twin_workbooks` sum the rows; with **`--fail-on-twin`** a non-zero total makes the binary exit **`3`** after writing the artifact and the summary.

The host-side proof (`fixture_tests.rs`, `injected_leaf_arithmetic_twin_mismatch_scores_reference_twin_and_fails_on_twin` and its siblings) injects a mismatch at the engine→instrument seam (`engine_sheet_views` → `score_session_views`) on the real W011 fixture. It proves the host no longer discards a row the engine reports; it does **not** prove that the engine's differential catches a wrong value at the three blind sites the Foundation note names (the >10k mark-all fall-through, the cross-sheet escalation, a FileCached pin) — those need a test-only seam inside OxCalc: `docs/handovers/HANDOVER_OXCALC_twin_injection_hooks.md`.

Observed on the first every-recalc run over the Foundation corpus (2026-09-15, `docs/test-runs/w012/TWIN_2026-09-15.md`): one row, `SCN-EK011-FUNCTION-RECOGNITION.xlsx` `H18 = =_xlfn.LAMBDA(1)`, both lanes `#CALC!`, cores equal, `Callable` payload on both sides — OxFunc's `CallableValue::eq` is handle identity (`Shared::ptr_eq`) and each lane minted its own handle, so OxCalc's whole-`CalcValue` comparison reports a mismatch that is not a value divergence. Typed `ReferenceTwin:RichOnly`, kept red, sibling-owned (dtc-5zet.26).

### `not_evaluated` reasons

| key | meaning |
|---|---|
| `open_failed:<kind>` | `open_xlsx_bytes` failed; `kind` is the typed error's variant path (e.g. `oxcalc:GridEngine:InvalidDefinedName`). |
| `sheet_unmatched` | the file's sheet has no engine sheet of the same display name. |
| `engine_read_failed:<kind>` | the session refused a read the ladder needs (`sheets()` / `grid_view()`): an internal invariant, surfaced as a row rather than a panic. |
| `not_published` | the engine published nothing at the address (a cache-less `NotCalcModeled` or unbacked shared-region cell). |
| `bind_degraded` | `FileCached`, and `WorkbookLoadReport::bind_degradations` names the address: OxFml rejected the text; the engine pinned the file's cache. |
| `external_reference_pinned` | `FileCached`, and `external_reference_pins` names the address: the formula references another workbook. |
| `not_calc_modeled` | `FileCached`, and the file's topology record is `DataTable` or an unknown `t=`: the ingest never binds it. |
| `manual_mode` | `FileCached` on a `calcMode="manual"` load that no forced recalc drained: none was forced (test-only `--no-force-recalc`), it failed, or it had nothing to drain. |
| `file_cached` | `FileCached` with no more specific explanation from the load report. |
| `degraded` | `Degraded`: a rejected formula with no file cache; the engine published a substitute `#NAME?`. |
| `stale` | `Stale`: calculated once, then invalidated by an undrained edit. |

The refinement order for `FileCached` is the table order (most specific first). Note: OxCalc's `BindDegradation::address` is `R{row}C{col}` with no sheet token, so on a multi-sheet workbook the `bind_degraded` / `external_reference_pinned` refinement matches by sheet-local address only (the current corpus is single-sheet throughout; a follow-up bead names the sibling change).

### `excluded` reasons (engine-`Calculated`, not scored)

| key | meaning |
|---|---|
| `nondeterministic:<FUNCTION>` | the formula calls one of `NOW`, `TODAY`, `RAND`, `RANDBETWEEN`, `RANDARRAY` (outside string literals; `_xlfn.` prefix stripped). |
| `nondeterministic:ca=1` | no such call, but the file marks the cell `ca="1"` — Excel's own always-calculate flag, which it sets on every cell of a volatile closure, so `INDIRECT`/`OFFSET` users and cells that depend on any volatile land here too (observed on the 2026-09-14 baseline: `=B8-B9` under `B8 = TODAY()+1`, `=INDIRECT("B1")`, `=SUM(OFFSET(#REF!,0,0,1,1))`). The function list alone would score those; the bead's rule is that the file's marker excludes. |
| `no_file_cache` | the file carries no `<v>` for the cell — which is also how OxDoc reports an empty-string result (`<v></v>` decodes to no cache). |
| `oracle_unknown_error_code` | the cache is an error literal OxDoc does not decode (`#SPILL!`, `#CALC!`, `#GETTING_DATA`, ... arrive as byte 255, which OxCalc's ingest maps to `#VALUE!` flagged unknown); comparing would launder a modern error into `#VALUE!`. |
| `oracle_unsupported_payload:<detail>` | the cache payload is not a value (`Empty`, nested `Formula`, `RichStub`, or a shared-string index outside the table). |

### `mismatch` classes (engine-`Calculated`, compared with Excel's cache)

Comparison is on the typed value: numbers as `f64`, booleans, text as UTF-16 code units, errors **by code** (OxDoc's byte through the same `map_biff_error_code` table the ingest uses), never by display text.

| key | one-line definition |
|---|---|
| `Ulp` | both numbers, not equal, at most 4 ULP apart (the per-cell row carries the distance). |
| `SignificantDigits15` | both numbers, more than 4 ULP apart, equal when both are rounded to 15 significant digits (Excel's cache precision vs a full `f64`). |
| `Number` | both numbers, differing beyond 15 significant digits, or an engine non-finite value. |
| `Text` | both text, differing in at least one UTF-16 code unit. |
| `Logical` | both booleans, differing. |
| `ErrorCode:<engine>-><excel>` | both errors, differing in code. |
| `EngineError:<code>` | the engine produced an error where Excel cached a non-error (observed in dtc-5zet.7 for `_xlfn.SINGLE`, `XLOOKUP`, `LET`, `LAMBDA`, whole-column refs, union, Table refs: these bind, evaluate to an error, and are `Calculated` — so they score here, never under `not_evaluated`). |
| `OracleError:<code>` | Excel cached an error where the engine produced a non-error value. |
| `Type:<engine>-><excel>` | different scalar kinds, neither an error (e.g. engine `Empty`, Excel `Number`). |
| `ReferenceTwin` | the engine's reference (oracle) lane and its optimized lane published different `CoreValue`s for this cell on the last recalc (`differential_mismatches`); Excel's cache is recorded but not compared — the engine disagrees with itself. |
| `ReferenceTwin:RichOnly` | the two lanes' `CoreValue`s are equal and their `CalcValue`s still compare unequal: only the rich payload differs (a `Callable` compares by handle identity; `Presentation` / `ErrorMetadata` / `Object` by value). The engine's verdict, typed apart so a payload-identity disagreement never shares a count with a value disagreement. |
| `Unclassified:<engine>` | an engine value shape the classifier does not model (array, reference, missing). **Must be 0.** Widening any class to reach zero requires a named bead. |

`match`: exact — equal `f64` bits (or both zero: Excel has no negative zero), equal booleans, equal UTF-16 text, equal error codes.

## Escalation beside coverage (W012-1.7 / dtc-5zet.14)

The Foundation note's standing rule: every conservative fallback on the live path is counted and reported beside coverage, so escalating to a full mark-all stops being free. `diff` and `bench` both print an `ESCALATION` line right after the primary number and retain the block in their artifacts (`totals.escalation` / `workbooks[*].escalation`; `escalation` / `subjects[*].escalation`), read from host-core's per-session `RecalcAccounting` (`dnacalc-host-core/src/recalc_accounting.rs`; the same counters ride the Skin IR as `WorkbookCalcProjection::recalc_accounting`).

What the block says, and what it cannot say yet:

- `recalcs` = the recalc transactions the host issued after the open: `automatic_entries + automatic_clears + f9_drains`. The load's Automatic open-recalc is `open_recalcs` / `open_recalc_sheet_drains`, kept apart — a fresh backing has no retained valuation, so it is a full evaluation by definition, not a fallback.
- `contract_rebuilds` = `automatic_clears + f9_drains`: the transactions whose verb rebuilds every drained sheet from authored input and recalculates it in full **by OxCalc's documented contract** (`clear_grid_cell` under Automatic, `recalculate_workbook` per drained sheet — the rebuild + mark-all before the seeded pass whose `cells_evaluated` the outcome reports). `escalation_rate_lower_bound = contract_rebuilds / recalcs`. It is a lower bound because an Automatic entry may escalate too (`TopologyGrowth` on a write to an empty address; the propagation's re-entry of the origin sheet), and the host cannot see it.
- `escalation_rate` = engine-reported escalations / `recalcs`, **`null` today**: OxCalc exposes no `GridSeededLaneOutcome` to the host (private `GridDerivedState::last_lane_outcome`; `calc-1q03.15` open), so every recalc lands in `lane_outcomes.unobserved` and `lane_outcomes_exposed` is `false`. The rate is printed as `UNOBSERVABLE (lane outcomes engine-reported 0 / N)`; nothing is inferred in its place. `docs/handovers/HANDOVER_OXCALC_charter_s8_escalation_is_counted.md` carries the ask; `dtc-5zet.22` wires the accessor when it lands, at which point the typed per-variant fields fill and the rate becomes exact.
- `f9_failures` counts the F9s the engine aborted with a typed error (a cell it cannot evaluate under the F9 path's `Abort` policy — the open-recalc degrades such a cell since calc-1q03.10, the F9 path does not yet, OxCalc calc-1q03.19). Apart from drains and no-ops, in no rate. Observed on the live path (2026-09-15, `SCN-EK011-FUNCTION-RECOGNITION.xlsx`, `D1 = COPILOT(1)`): the failed F9 also consumes the sheet's seeds, so the next F9 is a no-op and the edited cell stays `Stale` — pinned by host-core's `a_failed_f9_is_counted_apart_and_drops_the_seed`, filed as a sibling finding.
- Identities checked before either artifact is written: `recalcs == automatic_entries + automatic_clears + f9_drains`, `contract_rebuilds == automatic_clears + f9_drains`, `lane_outcomes total == recalcs`.

Read the numbers with the run shape in mind. On a `diff` run the only host verb after the open is the forced F9: a no-op on an `Automatic` load (0 recalcs, rates undefined) and one drain on a `Manual` load (1 recalc, a contract rebuild, lower bound 1.0) — so on a Manual-heavy corpus the lower bound is 1.0 by construction and says nothing about the incremental lane, and on the in-tree Foundation corpus (109 workbooks, all `Automatic`) there are 0 recalcs after the open and both rates are undefined (`docs/test-runs/w012/ESCALATION_2026-09-15.md`). The `bench` subjects are where the rate would move: `edit_repeats` Automatic entries (unobserved) plus the F9 cycles (contract rebuilds).

## `divergence.v1.json`

Field order is reading order: `schema`, **`coverage`** (corpus-wide), `totals` (workbooks opened / open_failed / unreadable, sheets, cells, formula_cells, engine_calculated, coverage, `escalation` (W012-1.7, the corpus fold), `not_evaluated` by reason, `excluded` by reason, `match`, `mismatch` by class, `unclassified`, `reference_twin_mismatches`, `reference_twin_workbooks`), `worst_covered` (the ten lowest-coverage formula-bearing workbooks), `corpus` (every workbook's sha256, sorted, and the sha256 of that list), `run` (corpus dir, out dir, `forced_recalc`, `validation_mode`, `fail_on_twin`), then `workbooks`: per workbook the same counts plus `open` status, `calc_mode`, the engine's `WorkbookLoadReport` counts, the forced recalc's tick / drained sheets / cells evaluated, the `reference_twin` block (validation mode, `checked_every_recalc`, every disputed cell), the `escalation` block (W012-1.7), and a `verdicts` row per formula cell (sheet, A1 address, formula text as stored, verdict key, provenance, `tick_id`, engine value, Excel value, and `twin_reference` when the twin disputes the cell) so every `match` can be audited back to its `Calculated { tick_id }`. The W012-1.5 fields are all additive (`#[serde(default)]`): an artifact written before them still reads back.

`unreadable` workbooks (OxDoc itself rejects the bytes) have no event stream and therefore no known formula-cell count; they are counted and named but contribute nothing to the denominator — the honest gap, stated rather than hidden.

## `bench` — the grid-engine wall-clock baseline (dtc-5zet.13)

```
cargo run --release -p dnacalc-conform --offline -- bench --corpus <dir> --out <dir>
```

Measures the lane that runs `.xlsx`, through the same `WorkbookSession` verbs the product runs (under the interactive host's `DualValidatedSampled { one_in: 16 }` on purpose — the baseline was measured under it; `diff`'s every-recalc twin is not the bench's), and writes `<out>/bench.v1.json` (`bench_report.rs`; machine identity first, then what the host can and cannot observe, the subject selection, and one block per subject with every sample retained). The committed baseline and its tolerance bands are `docs/test-runs/w012/BASELINE_2026-09-15.md`.

Subjects, in order:

- the `--corpus-subjects` (3) largest-formula-cell corpus workbooks that open on the product path — every readable `*.xlsx` under `--corpus` is ranked Excel-authored first (`docProps/app.xml` carries `<Application>Microsoft Excel</Application>`, read through OxDoc's package snapshot), then formula cells descending, then path; higher-ranked files the engine refuses are listed under `skipped_open_failed` with their typed error kind;
- `synthetic:filldown-<rows>` — one sheet, `A{r}` = `r`, `B{r}` = `=A{r}*2`, authored under `Manual` through `enter_grid_cell` (default 2,000 rows; `--filldown-rows 10000` is the bead's long run — every verb is O(authored cells), so it takes tens of minutes);
- `synthetic:cross-sheet-<rows>` — `Inputs!A{r}` = `r`, `Calc!A{r}` = `=Inputs!A{r}*2` (default 500), so the cross-sheet propagation's escalation is in the numbers.

Per subject: corpus opens (`oxdoc_only` = engine-free `open_host_owned_xlsx_source`; `product_open` = `open_xlsx_bytes`, which for an `Automatic` file includes the cold open-recalc) or synthetic setup plus the cold F9; then `--edit-repeats` (5) Automatic-mode `enter_grid_cell` edits of an input cell (an existing literal a formula references, found by an address-token scan of the file's formulas, cross-sheet qualifiers included; a workbook with no literal gets a new literal in column A, recorded as such), each proven drained by the `Calculated { tick_id }` the edited cell carries and by the dependent formula's value moving; then `--full-recalc-repeats` (3, the cold one included for synthetic subjects) cycles of `set_calc_mode(Manual)` → edit → `recalculate()`.

What the counters mean — read before quoting a number:

- `full_recalc[i].cells_evaluated` is `WorkbookRecalcOutcome::total_cells_evaluated()`. The F9 verb rebuilds each seeded sheet's derived state from authored input (a whole-sheet mark-all, tick 0, so the reference twin runs under the host's `DualValidatedSampled { one_in: 16 }`) and THEN runs a second, seeded recalc; the counter is that second pass only. The rebuild's mark-all and its oracle sweep are in `wall_ms` and absent from the counter (a warm F9 on the 2,000-row fill-down reports `2`).
- On a workbook with cross-sheet edges the origin sheet's counter is every one of its cells, not the cone: `propagate_cross_sheet_edit` re-enters the origin with a mark-all. Sheets recalculated only by propagation are not in the outcome.
- `edit.lane_outcome` is `null`: OxCalc has no public accessor for `GridSeededLaneOutcome`, `cells_evaluated` on an inline edit, or the grid phase timers (`docs/handovers/HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`). The artifact's `observability` block says so once; nothing is estimated in their place.
- `peak_working_set_bytes` is `Get-Process PeakWorkingSet64` of the bench process at the end of the run (Windows; `null` with the reason elsewhere) — process-wide, not per subject. There is no allocation counter: the workspace forbids `unsafe_code`, which a counting `GlobalAlloc` needs.
- `subjects[*].escalation` / `escalation` (W012-1.7) are the session's recalc accounting per subject and folded for the run — see "Escalation beside coverage" above. Per subject on the default shape: `edit_repeats` (5) Automatic entries, `full_recalc_repeats` (3) F9 drains, so `recalcs 8`, `contract_rebuilds 3`, lower bound `0.375`, exact rate `null`.

Wall-clock samples are milliseconds with every sample retained plus min / median / max; the summary quotes the median. Run-to-run spread on the same machine was up to 2.8× on some numbers (see the baseline doc), so bands are set per number, not as one percentage.

## Exit codes and flags

`0`: the run completed and the artifact was written — RED coverage or mismatches are results, not failures (floors are a later bead). `1`: the run could not complete (I/O, no workbooks under the corpus dir, an internal identity violation, a bench subject that could not be measured — the artifact is then not written). `2`: bad arguments. `3`: `diff --fail-on-twin` only — the run completed, the artifact was written, and `totals.reference_twin_mismatches > 0`.

`diff --fail-on-twin` is the one flag that changes the exit code; it changes nothing else (the twin runs, and its rows are reported, on every `diff`).

`bench` knobs (`--corpus-subjects`, `--filldown-rows`, `--cross-sheet-rows`, `--open-repeats`, `--edit-repeats`, `--full-recalc-repeats`) take positive integers; the defaults are the committed baseline shape and are printed by `--help`.

`--no-force-recalc` is a test-only flag, absent from the usage text: it skips the forced recalc so the Manual-mode coverage trap can be proven (`manual_mode_fixture_has_coverage_only_because_the_recalc_is_forced`). It is not a way to run the instrument.

## Tier

TC: depends on `dnacalc-host-core`, `dnacalc-skin-ir` (T0; the `CalcModeProjection` that `set_calc_mode` takes), `oxcalc_core`, `oxfunc_core`, `oxdoc_model`, `oxdoc_xlsx` (`oxdoc_conformance` dev-only, for zipping the parts-only W012 fixtures in tests), plus `sha2` (already in the workspace lock) for the corpus identity. Never a dependency of any other workspace crate; `dnacalc-arch-gates` carries the positive control `tc_control_conform_contains_oxcalc_and_oxdoc`. No zip crate, no absolute machine paths: corpus and out dirs are arguments; tests reach `../../fixtures` by `CARGO_MANIFEST_DIR` arithmetic.

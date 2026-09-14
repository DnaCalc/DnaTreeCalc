# dnacalc-conform

The W012 measured-Excel conformance instrument (`Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §3 W1, Appendix A; bead dtc-5zet.8). A TC-tier lib + bin that runs a **provenance-first, cell-by-cell Excel-vs-DNA differential** over a directory of Excel-authored `.xlsx` files, offline, through the path the product runs.

```
cargo run -p dnacalc-conform --offline -- diff --corpus <dir> --out <dir>
```

Per workbook: `std::fs::read` → `dnacalc_host_core::WorkbookSession::open_xlsx_bytes` (OxDoc `LoadProfile::full()` + OxCalc ingest) → `session.recalculate()` **unconditionally**, whatever the file's `calcMode` (62 of the 174 Excel-authored corpus files are `manual` and would otherwise contribute zero coverage) → for every formula cell the file carries, the verdict ladder → `<out>/divergence.v1.json`.

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
2. Calculated, formula calls NOW/TODAY/RAND/RANDBETWEEN/RANDARRAY
   or the file marks the cell ca="1"             -> excluded:nondeterministic:<trigger>
3. Calculated, no scorable cache                 -> excluded:no_file_cache |
                                                    oracle_unknown_error_code |
                                                    oracle_unsupported_payload:<detail>
4. Calculated with a cache                        -> match | mismatch:<class>
```

Only rung 4 produces `match`. A `FileCached` value that equals Excel's cache is the false green (`dnacalc-host-core/src/xlsx_false_green.rs`) and stops at rung 1 by construction — the fixture test `false_green_rejected_cell_is_not_evaluated_bind_degraded_and_control_matches` pins that.

Provenance is read at the OxCalc level (`WorkbookSession::grid_view`, the engine's `PublishedValueProvenance`), not through the skin-IR mirror, which folds `Degraded` into `FileCached`.

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
| `Unclassified:<engine>` | an engine value shape the classifier does not model (array, reference, missing). **Must be 0.** Widening any class to reach zero requires a named bead. |

`match`: exact — equal `f64` bits (or both zero: Excel has no negative zero), equal booleans, equal UTF-16 text, equal error codes.

## `divergence.v1.json`

Field order is reading order: `schema`, **`coverage`** (corpus-wide), `totals` (workbooks opened / open_failed / unreadable, sheets, cells, formula_cells, engine_calculated, coverage, `not_evaluated` by reason, `excluded` by reason, `match`, `mismatch` by class, `unclassified`), `worst_covered` (the ten lowest-coverage formula-bearing workbooks), `corpus` (every workbook's sha256, sorted, and the sha256 of that list), `run` (corpus dir, out dir, `forced_recalc`), then `workbooks`: per workbook the same counts plus `open` status, `calc_mode`, the engine's `WorkbookLoadReport` counts, the forced recalc's tick / drained sheets / cells evaluated, and a `verdicts` row per formula cell (sheet, A1 address, formula text as stored, verdict key, provenance, `tick_id`, engine value, Excel value) so every `match` can be audited back to its `Calculated { tick_id }`.

`unreadable` workbooks (OxDoc itself rejects the bytes) have no event stream and therefore no known formula-cell count; they are counted and named but contribute nothing to the denominator — the honest gap, stated rather than hidden.

## Exit codes and flags

`0`: the run completed and the artifact was written — RED coverage or mismatches are results, not failures (floors are a later bead). `1`: the run could not complete (I/O, no workbooks under the corpus dir, an internal identity violation — the artifact is then not written). `2`: bad arguments.

`--no-force-recalc` is a test-only flag, absent from the usage text: it skips the forced recalc so the Manual-mode coverage trap can be proven (`manual_mode_fixture_has_coverage_only_because_the_recalc_is_forced`). It is not a way to run the instrument.

## Tier

TC: depends on `dnacalc-host-core`, `oxcalc_core`, `oxfunc_core`, `oxdoc_model`, `oxdoc_xlsx` (`oxdoc_conformance` dev-only, for zipping the parts-only W012 fixtures in tests), plus `sha2` (already in the workspace lock) for the corpus identity. Never a dependency of any other workspace crate; `dnacalc-arch-gates` carries the positive control `tc_control_conform_contains_oxcalc_and_oxdoc`. No zip crate, no absolute machine paths: corpus and out dirs are arguments; tests reach `../../fixtures` by `CARGO_MANIFEST_DIR` arithmetic.

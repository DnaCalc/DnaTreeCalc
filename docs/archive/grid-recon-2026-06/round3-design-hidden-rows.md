# DESIGN PROPOSAL — Hidden/Summary Rows as Calc-Affecting State (visibility lens)

## 1. Three-layer visibility model (doctrine, stated once)

| Layer | Meaning | Lives in | Calc effect |
|---|---|---|---|
| **V1 Viewport** | scroll/panes/`ViewRegion` | read-side channel, never a revision (`designs-display.md:20`) | **Schedule-only.** Order/timing, never values (`treecalc.rs:8392-8395` equivalence) |
| **V2 Document hidden state** | manual hide, filter hide, outline collapse, row-height-0 | **`AxisState`** — calc-owned, revisioned sheet-facet document state | **Calc input.** Feeds SUBTOTAL/AGGREGATE via host-info seam; produces invalidation seeds |
| **V3 Styling visibility** | white-on-white, `;;;` formats | style layer (`designs-storage.md:25`) | None, ever. Calc never reads StyleId |

**One store, one derivation direction.** Amend `designs-storage.md:27`'s single `hidden: bool` to a provenance-typed `AxisState`: per-axis runs `Vec<(start, len, AxisProps)>` with `AxisProps { size, hidden_manual: bool, hidden_filter: bool, outline_level: u8, collapsed: bool }`. `hidden_outline` is **derived** (collapse level → hidden spans, pure function). Effective-hidden = `manual || filter || outline`. `AxisLayout` (`designs-display.md:27-34`) becomes a **compiled view of AxisState** (Fenwick over effective sizes); it never stores independent hidden truth — calc and display read the same source, display reads the compilation. Provenance is mandatory: SUBTOTAL 1-11 vs 101-111 split on manual-vs-filter (`subtotal_rules`, `OxFunc/crates/oxfunc_core/src/functions/subtotal_aggregate_family.rs:174-210`), and AGGREGATE's split is the open COM question. AxisState is COW alongside blocks → hide/unhide is an undoable, replayable revision (it must be: it changes values). Columns: store the same props for symmetry, but calc **hard-exempts** column hiding (well-sourced: never affects results, never triggers recalc).

## 2. Function seam

**Declaration already exists — don't invent a second one.** SUBTOTAL/AGGREGATE declare hidden-sensitivity via `HostInteractionClass::WorkbookState` + `ArgPreparationProfile::RefsVisibleInAdapter` (`subtotal_aggregate_family.rs:38-64`) and read it through `HostInfoProvider::query_aggregate_reference_context` (`host_info.rs:177-182`), provider-mandatory (`:579,611`). The grid work is **one implementation, zero OxFunc API change in v1**: `GridHostInfoProvider` beside `TreeCalcHostInfoProvider` (`OxCalc/src/oxcalc-core/src/treecalc.rs:5830-5857`), built per-evaluation like the reference provider (`treecalc.rs:4774-4787`):

- Resolve `ReferenceLike` → sheet + rect (same resolution the value lane uses).
- `row_hidden_manual` / `row_filtered_out`: intersect rect's row span with AxisState runs — O(runs ∩ span), expanded to the shape-aligned per-cell `Vec<AggregateCellContext>` the seam demands. Per-cell expansion is symmetric with the value materialization the function already does (`materialize_ref_filtered_arg`, `:347-368`), so it adds no asymptotic cost; a run-compressed seam variant is a *registered future optimization* (open Q5), not v1.
- `nested_subtotal_or_aggregate`: a **template-level fact**. Add `contains_subtotal_or_aggregate: bool` to `GridFormulaTemplate` (derived once at bind from the bound artifact's function-id set; loners and punch-through overrides carry it on their own binding). Provider answers by querying the region map over the rect — O(regions ∩ rect) via the same interval machinery, expanded at the seam. This closes recon gap #2 with no per-cell formula scanning.

Decision: **keep `HostInfoProvider` as the single host-state seam** — no parallel "visibility reader" beside `ReferenceSystemProvider`. The trait is already mandatory for these functions and will later serve `CELL` width/protect queries from the same AxisState.

## 3. Dependency + invalidation

**New descriptor kind**: `GridVisibilityRange { sheet, row_span }` extending `dependency.rs:13-33` alongside `GridRangeRegion` — emitted at template lowering when the bound artifact shows aggregate-context dependence (the WorkbookState/host-info fact OxFml's `ExecutionProfileSummary` already carries). It is a **1-D row-interval edge** (columns exempt), stored in a per-sheet, per-axis interval index — the row-only sibling of the rect interval tree (`designs-calcgraph.md:18`).

**Regions stay whole.** A filtered-table footer row of SUBTOTALs is one region with one `GridVisibilityRange` edge claiming the referenced row span. The running-probe pattern — fill-down `=SUBTOTAL(103, B$2:B2)` — is an **affine span edge** (span = rows 2..caller_row), the visibility analogue of `GridTemplateRelative`: toggle of rows r₁..r₂ → affected caller rows = those whose span intersects = `caller_row ≥ r₁`, a closed form. Fallback if affine spans slip: claim the whole-region union span — sound under the rect over-approximation invariant (`designs-calcgraph.md:21`), recompute extra, never wrong. Single-cell `SUBTOTAL(103, cell)` probes (recon gap #8) are the degenerate affine case and fall out for free.

**Typed edits**: `GridAxisEdit { sheet, axis, span, op }` with `op ∈ { SetHidden { hidden, provenance: Manual|Filter|Outline }, SetOutlineLevel, SetCollapsed, SetSize }`, entering the extended `OxCalcTreeEdit` transaction surface (`consumer.rs:297-338` per `designs-calcgraph.md:27`). Each effective-hidden delta emits `InvalidationSeed::AxisVisibility { sheet, axis, span }` → 1-D index query → dirty exactly the VisibilitySensitive consumers ∩ span → normal rect propagation. The hidden rows' own cells are **not** dirtied (their values don't change).

**Excel staleness: be correct, reserve compat.** Excel's freshness mechanism is event-driven row-flagging ("hide/unhide flags the selected rows as uncalculated" — Decision Models; MS Learn trigger list), which both over-dirties (non-SUBTOTAL formulas in hidden rows) and possibly under-dirties (SUBTOTAL over constants-only hidden rows, cross-sheet SUBTOTALs — unverified). **Recommend: default `visibility_staleness = Exact`** — precise dependency-driven invalidation, spec'd in GRID_MODEL as "fresher-than-Excel is conforming; we never publish a value a conforming full recalc would not." Reserve an `ExcelCompat` profile flag but **do not build it** unless OxReplay conformance diffs actually trip on observable staleness (witnessable only in manual-calc mode or constants-only-hidden cases — COM matrix below decides). Invariant: Exact's recompute set ⊇ every value Excel recalcs.

## 4. Excel anchoring

**OxXlPlay first (Wave 1 long pole — recon gap #7).** New scenario ops: `set_row_hidden`, `set_row_height` (incl. 0), `apply_autofilter{criteria}` / `clear_autofilter`, `outline_group` + `Outline.ShowLevels`, VBA `EntireRow.Hidden`. New **row-visibility view** (per-row `Hidden`, `Height`, outline level, filter membership) joining the existing view family (`OXXLPLAY_CLI_CONTRACT.md:21-46`). Recalc-witnessing via the existing VBA-UDF workbook lane: eval-counter UDF cells record which cells Excel actually recalculates on each toggle, under both auto and manual calc.

**COM matrix** (priority order): (1) AGGREGATE options 0-7 × {manual, filter} — resolves the MS-doc vs Exceljet option-4 conflict; OxFunc's `aggregate_rules` (`:212-290`) encodes the MS reading and is a data-table one-liner to flip; (2) SUBTOTAL 1-11/101-111 × {manual, filter, outline-collapse, height-0, VBA-hidden}; (3) dirtying scope: constants-only hidden rows, cross-sheet SUBTOTAL, manual-calc observability; (4) outline ShowLevels ≡ manual-hidden bit; (5) column-hide non-effect (confirmation).

**Spec placement**: GRID_MODEL gains §"Axis visibility as calc input" (AxisState model, provenance, seeds, the Exact-freshness claim + profile flag); the **function rule tables stay in the OxFunc function-lane contract** (`FUNCTION_SLICE_SUBTOTAL_AGGREGATE_CONTRACT_PRELIM.md`), amended with COM evidence pointers — GRID_MODEL references, never restates. The invariant register row lands in `CORE_ENGINE_GRID_REFINEMENT_AND_EQUIVALENCE.md`.

**GridCalc-Ref**: per-row `BTreeMap<u32, RowProps>`, naive `query_aggregate_reference_context` by direct lookup, and — since the ref is mark-all-dirty per recalc (`designs-verification.md §3`) — visibility freshness is correct by construction. Generator gains hide/unhide/filter/collapse script steps; the invalidation-closure observation surface (refinement surface 2) compares recompute sets.

## 5. Filter/outline v1 scope

**v1 = hidden state as input.** AutoFilter the *feature* (criteria evaluation, dropdown UI, `ShowAllData`) is deferred ENGINE work; the `Filter` provenance bit ships now so hosts/file-IO can set it. xlsx ingest heuristic (file-boundary lens): `hidden="1"` rows inside an active AutoFilter range → provenance Filter, else Manual; ledgered `Derived` (`round2-design-fileBoundary.md §3`). Outline: `outline_level`/`collapsed` stored now; collapse→hidden-runs derivation ships in v1 (it's how summary rows hide); outline UI stays deferred per `designs-display.md:75`.

## 6. Register rows + tile interplay

- **I-VIS-1**: hidden-toggle over span S dirties exactly `{VisibilitySensitive consumers with declared span ∩ S ≠ ∅}`; oracle: expand affine visibility edges per-cell, compare vs ref recompute set.
- **I-VIS-2** (metamorphic): random hide/unhide/filter storm changes **no** non-visibility-sensitive value.
- **I-VIS-3**: provenance separation — manual vs filter toggles flip SUBTOTAL 1-11 vs 101-111 exactly per the (COM-confirmed) rule table.
- **P-VIS-1**: toggle cost — `cells_evaluated == |affected consumers|`, `edges_visited = O(log intervals + k)`, **independent of sheet size**; workload `hide-storm` on boring-1Mx10 + k SUBTOTAL footers; counter-gated per `round2-design-perf.md §1`.
- **P-VIS-2**: aggregate-context query slots-visited ∝ runs ∩ span (provider side), recorded now; the per-cell seam expansion bounded by range extent, with the run-compressed seam as an open row.

**One edit, two paths, coherent.** A `GridAxisEdit` fans out to (a) calc seeds and (b) AxisLayout run-splice + tile re-render. Tiles are **model-indexed** (`TileCoord`), so hiding rows changes pixel mapping and visible-tile membership, not tile identity; only cells whose values change get new tile epochs. Coherence rule: both paths derive from the same AxisState revision; AxisLayout generation and `TilePatch` epochs both carry the revision epoch — layout updates immediately (hide is layout truth), value patches follow under stale shading. Invariant: renderer never observes a layout generation older than the newest tile epoch it holds.

## Build order (vs agreed waves)

- **Wave 0**: nothing visibility-specific; ensure `GridAxisEdit` is in the typed-edit enum sketch and AxisState provenance amends designs-storage.
- **Wave 1**: OxXlPlay ops + row-visibility view + COM matrix (start immediately — semantic long pole); GRID_MODEL §; GridCalc-Ref visibility + ~25 hand corpus cases; flip `aggregate_rules` if COM contradicts MS docs. OxFml: no work (no syntax); confirm `ExecutionProfileSummary` exposes function-id presence for the nested fact.
- **Wave 2**: AxisState runs + COW; `GridHostInfoProvider`; `GridVisibilityRange` + 1-D interval index + `InvalidationSeed::AxisVisibility`; `GridAxisEdit`; I-VIS/P-VIS rows live.
- **Wave 3**: AxisLayout derived from AxisState; epoch/generation coherence; outline collapse derivation wired to display.

## Hardest problems + derisking

1. **Semantic uncertainty (AGGREGATE option-4, outline≡manual, dirtying scope).** Derisk: COM matrix first; rules are data tables, cheap to flip; only dirtying-scope answers gate Wave 2 design (filter-rule answers are independent of invalidation machinery).
2. **Affine visibility spans for running probes.** Risk O(n) per-row edges; derisk: property-test closed-form span intersection vs scalarized ref; sound whole-region fallback.
3. **ExcelCompat pressure from replay diffs.** Derisk: COM-witness whether staleness is observable before building anything; Exact-by-default ratified as spec text.
4. **Nested-fact maintenance under punch-through.** Metamorphic test: punch SUBTOTAL into a constant region referenced by another SUBTOTAL; bit must update transactionally with the override.

## Open questions

1. Ratify AxisState (heights + hidden + provenance) as revisioned **document state** — hide/unhide is an undoable intent (extends designs-display's axis-geometry ownership amendment).
2. Ratify `visibility_staleness = Exact` default with "fresher-than-Excel is conforming" spec language; ExcelCompat reserved, unbuilt.
3. Single `HostInfoProvider` seam for all grid host-state queries (recommended) vs separate visibility reader?
4. Accept the xlsx Filter-provenance ingest heuristic as `Derived` in the fidelity ledger?
5. Accept per-cell `AggregateReferenceContext` at 1M-row scale for v1, with the run-compressed OxFunc seam as a registered deferral?
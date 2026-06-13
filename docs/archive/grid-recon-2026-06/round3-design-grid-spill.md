# DESIGN PROPOSAL — Dynamic-array spill on the grid (lens: SPILL/CTRO)

## Stance

Spill is **engine-owned placement arbitration over the computed layer**, never an OxFml concern and never authored state. OxFml already computes extent truthfully (`build_candidate_result`, `OxFml/crates/oxfml_core/src/session/mod.rs:1006-1125`); everything downstream — placement, blockage, `A1#` dereference — is OxCalc grid-profile work riding the shipped CTRO lane (`OxCalc/src/oxcalc-core/src/treecalc.rs:2294-2322`). DnaVisiCalc's Option A (spill = derived overlay on a scalar engine, `DnaVisiCalc/docs/DYNAMIC_ARRAYS_DESIGN.md`) is the proven shape; this design upgrades it to first-class engine state with rect propagation.

## 1. Representation

**SpillLedger** — per-sheet engine state in the grid facet, sibling of the computed layer (`grid/spill.rs` next to `grid/store.rs`, round2-design-structure §1):

```rust
SpillRecord { anchor: SparseCellCoord, intended: Extent, status: SpillStatus, epoch: SpillEpoch }
SpillStatus = Established { extent: Extent } | Blocked { blocking: Vec<SparseCellCoord>, reason: BlockReason }
```

keyed by anchor. Reverse lookup (cell → owning anchor; blockage watches) goes into the **same per-sheet interval index** the calc-graph lens specifies for listening rects (designs-calcgraph §3), with a rect-class tag `{ListeningRect, SpillBody, BlockageWatch}` — one structure, three uses. **Wave-0 note:** the interval-index design must admit tagged rect classes; that is the only shared-refactor addition this lens needs.

**Body cells materialize into the computed layer** as ordinary scalar `CompactCell`s written by the anchor's evaluation; the array value itself is stored once in the rich-value side table (designs-storage `CompactCell` codec). Rejected alternative: read-through redirection from body coords to anchor-array slices — it puts a branch on every hot read and makes `SUM`, tiles, and the oracle all spill-aware. Materialization keeps every reader uniform: a spill body cell IS a computed value at its coordinate; ghost-ness is purely the authored layer being `Empty` there (formula bar grey, edit semantics). Cost is ~16 B/cell ∝ extent — bounded by P-23 below. This preserves storage **I3** (blank authored cells cost zero authored bytes) and Excel observables (`ISBLANK(body)=FALSE`, ranges spanning spill bodies just work).

Spec hook: designs-verification §2 already reserves `CellState::SpillTarget(anchor)` — define it as *derived* state with the invariant ledger ⇔ computed-support (I-SP2 below).

## 2. Calc-time resolution via CTRO

**Extent is a calc output; placement commits before dependents read.**

- **`A1#` consumers** hold a *static* edge to the anchor cell (anchor identity never changes) plus a CTRO-published rect edge to the last-known extent. At eval, the grid `ReferenceSystemProvider` implements `ReferenceKind::SpillAnchor` (`OxFunc/crates/oxfunc_core/src/resolver.rs:586` capability; today zero implementors) by a ledger lookup — O(1), no extent scan (P-24). Each dereference records a resolution fact exactly like `TreeCalcRuntimeReferenceTextResolution` (`tree_reference_system.rs:214-269`), harvested post-invoke, lowered to `ctro_spill_extent` dynamic descriptors, merged into the effective graph and persisted via positive publication (`treecalc.rs:2294-2322`). This is W047's anticipated "region/spill resize" frontier trigger (`W047_EFFECTIVE_GRAPH_OVERLAY_AND_FRONTIER_REPAIR_SEMANTICS.md:144-148`) made real: CTRO generalizes from value-dependent target *identity* (INDIRECT) to value-dependent target *extent*. A blocked anchor dereferences to `#REF!` per Excel (pin via COM).
- **Refs into the middle of a spill** (`B3` where B3 is body) need **no new edge kind**. They are ordinary rect-listening edges; the key rule is that an anchor's evaluation emits a **dirty rect = old extent ∪ new extent** (contraction included) into the interval index, as if it were an edit. Rect propagation does the rest. This is the load-bearing simplification: only `#`-refs are CTRO; everything else is rect propagation.
- **Ordering within a run:** placement arbitration runs immediately after the anchor's candidate is built, committing ledger + working body values before later-scheduled consumers read (working-values discipline, `treecalc.rs:1186-1191`). Static anchor edges put `A1#` consumers after the anchor topologically. The hazard is *growth into coords no prior-run edge covered*: shipped CTRO has no within-run re-entry (convergence is run-over-run, `tree_reference_system.rs:168-197`). **v1 decision: bounded run-level repair passes** — after the main loop, if any `ShapeDelta` extent differs from the prior ledger, seed the symmetric-difference rects and run another pass; cap at k (propose 4); residual instability ⇒ circular-spill `#SPILL!` (reason `circular`). This is iterate-to-fixpoint at run granularity, not statement-level re-entry — consistent with shipped CTRO discipline and with W047's repair semantics (doc :191-201) without building mid-run interruption.
- **Implicit intersection:** grid eval naturally supplies `CallerContext { row, col }` (`resolver.rs:16-20`); machinery is otherwise complete (gap-list item 6).

## 3. Blockage semantics

Conflict set for `intended` rect minus anchor: authored non-Empty cells (literals, formula markers, punch-through overrides), other ledger rects (anchor or body), table-overlay claimed rects (`TableBacking` grid backing, round2-design-structure §3), merged-cell rects (blocked even when empty — COM-pin). Whole-anchor failure: no partial spill; anchor publishes `WorksheetErrorCode::Spill` (`oxfunc_value_types/src/lib.rs:28`) as a 1×1, ledger goes `Blocked{blocking, reason}` with Excel-style first-blocker diagnostics.

The occupancy probe must be **occupancy-proportional**: test `intended ∩ occupied blocks` via block keys, never iterate empty slots of a 1M-row intent (P-25, storage I4 analog).

**Both directions are invalidation seeds through the interval index:** (a) blocked anchor registers a `BlockageWatch` rect = intended extent; any clearing edit intersecting it dirties the anchor (re-spill on clearance); (b) an authored edit landing inside an `Established` rect hits the `SpillBody` rect query, dirties the owning anchor, next eval arbitrates → `#SPILL!`. Excel allows typing into a spill range — the edit succeeds in the authored layer; the *anchor* errors.

**Seam graduation:** the engine is the arbiter; OxFml's always-`Established`/empty-`blocking_loci` placeholders (`session/mod.rs:1113`) are **not** extended into a veto/re-entry seam. The engine rewrites the anchor's published value post-candidate and emits truthful `SpillEvent`s/`SpillFact`s (`seam/mod.rs:74-81,146-156`) from arbitration results. OxFml stays declarative; `SpillBlocked`/`SpillClearance` finally get a producer.

## 4. Interactions

- **Template regions:** spill anchors may be template members; ledger entries are per-instance; arbitration per-instance in deterministic order. Adjacent spilling instances blocking each other is *correct Excel behavior*, reproducible only if order is spec'd — propose row-major anchor order, **COM-verify** what Excel actually does. Flash-fill of a spilling formula = host-declared region whose template carries `uses_spill_reference`/spill execution profile; no new machinery.
- **Punch-through overrides:** an override inside another spill's rect is an authored cell → blocks it. An override that itself spills → loner anchor + ledger entry. Region edges still claim full extent (calc-graph punch-through invariant holds).
- **Insert/delete:** ledger rects are derived — never shift-adjusted. Anchors shift with the authored layer; every anchor whose intended rect intersects the shift band is invalidated and re-arbitrated. Simple-correct; cost ∝ affected anchors.
- **Tiles/stale-visible:** body values are ordinary computed-layer tile patches; contraction rects are part of the dirty rect so ghost-clearing patches and tile-epoch bumps fall out free. Optional display-lane `SpillFact` for intended-extent shading on `#SPILL!` — defer to Wave 3.
- **Hidden rows:** no semantic interaction — spill places into hidden rows normally (cheap COM confirmation alongside the hidden-row lens's new OxXlPlay scenario ops; joint ask).
- **GridCalc-Ref:** naive spill = after each full evaluation pass, recompute all extents, arbitrate in spec order over a plain map, iterate to fixpoint with the same cap k. Refinement surface 3 (designs-verification §3) already names `#SPILL!` placement + extents; α maps ledger → ref's assignment. Equality: published values at all coords, anchor error placement, per-anchor extent. Order-sensitivity is exactly why arbitration order must be in the spec, not emergent.
- **TreeCalc:** untouched. `CORE_MODEL_SPEC.md:50-52` ("no inter-node spilling") stays verbatim.

## 5. Register rows

Invariants: **I-SP1** spill bodies never appear in the authored layer; **I-SP2** ledger ⇔ computed-layer support (every Established rect fully materialized; no body value outside a ledger rect) — expand-and-compare oracle: naive all-anchor scan vs interval-index queries; **I-SP3** arbitration determinism under spec order; **I-SP4** quiesced no-edit run changes no extent (extends P-19 warm no-op); **I-SP5** scalarizer equivalence — GridCalc-Ref fixpoint = optimized engine on values/errors/extents.

Perf: **P-23** re-spill cost ∝ |old ∪ new extent| (cells written + rects propagated counters; workload `filter-spill-1M`: FILTER whose result size changes per edit); **P-24** `A1#` dereference = O(1) ledger probes counter; **P-25** blockage check ∝ occupied-blocks-intersecting-intent (workload: spill intent over a 1M-row empty column); **P-26** spill-extent epoch — `A1#` consumers re-evaluate on extent-epoch or value change only, never on unrelated anchor-sheet churn (workload `spill-storm`). Counter gates per round2-design-perf doctrine; no wall-clock.

## 6. Profile angle

Strict-excel-grid only. Gate at bind: `BindProfile` admits capability `"spill_reference"` (`OxFml semantics/mod.rs:439-445`) only under the grid profile; TreeCalc keeps the carrier-gating diagnostic path (`carrier.rs:188-194`). One value substrate: the anchor's array is the same `CalcValue` array TreeCalc nodes publish — the grid adds *placement*, nothing else. This design is the answer to the parked `DynamicArraySpillPolicy` requirement (`structured_table.rs:2002,2219-2227`): grid profile admits `OP_SPILL_REF` over tables, tree profile keeps deny.

## Hardest problems + derisking

1. **Fixpoint/repair semantics** (growth into uncovered coords; mutual-blockage oscillation). Derisk Wave 1: build ref-machine fixpoint first, property-generate mutual/circular blockage, COM-pin Excel's order and circular behavior, then pick k against the oracle.
2. **Engine-rewrites-candidate seam** (published `#SPILL!` ≠ candidate value). Derisk: keep OxFml unchanged except truthful fact pass-through; characterization tests that TreeCalc seam consumers see no behavior change.
3. **Contraction ghosts × revisions/undo**: I-SP2 must survive COW snapshot restore; metamorphic spill-edit-undo storms vs ref.
4. **OxXlPlay can't capture spill scenarios yet** (no dynamic-array scenario ops verified) — scenario-op work shared with the hidden-row lens; sequence it first in Wave 1.

## Build order (vs agreed waves)

- **Wave 0:** tagged rect classes in the interval-index design (tiny); nothing else.
- **Wave 1:** GRID_MODEL spill section (derived `SpillTarget`, arbitration order, conflict set, repair cap); GridCalc-Ref fixpoint spill (~300 LOC over the BTreeMap ref); ~25 pinned corpus cases (blockage, clearance, mutual, circular, contraction, `A1#`-of-constant, merged, hidden-row spill); OxXlPlay spill scenario ops + COM pins; OxFml: truthful-fact plumbing only.
- **Wave 2:** SpillLedger + arbitration + computed-layer body writes; interval-index SpillBody/BlockageWatch rects; provider `SpillAnchor` dereference + `ctro_spill_extent` publication; run-level repair passes; register rows P-23..P-26, I-SP1..5.
- **Wave 3:** contraction-clearing tile patches, intended-extent shading, stale-visible interaction.

## Open questions for the owner

1. **Repair-pass cap vs pure run-over-run convergence**: I recommend bounded in-run repair (k≈4) so a single recalc quiesces like Excel; pure next-run convergence is simpler but user-visible (stale `A1#` after one calc). Confirm.
2. **Arbitration order**: row-major proposed; spec whatever COM shows Excel does. Sanction COM evidence as the order-of-record?
3. **Body materialization** (recommended) vs read-through overlay: accept ~16 B/cell × extent computed-layer cost for huge spills in exchange for uniform readers?
4. **`#`-operator in TreeCalc profile** (node-array spill refs): recommend never — spill vocabulary stays grid-only. Ratify as CORE_MODEL_SPEC text.
5. **Blocked-anchor `A1#` result**: `#REF!` (my reading of Excel) — COM-pin before spec'ing.
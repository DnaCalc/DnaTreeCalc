# Design proposal — Calc graph integration & virtual nodes (lens: dependency graph / templates / recalc)

## Recommended architecture

**1. Grid is a node facet; cells are virtual; the schedulable unit is the *region*, not the cell.**
Mirror the table precedent exactly: a `Sheet` node carries a grid facet the way `StructuralEdit::SetTableShape` attaches `StructuralTableShape` (OxCalc `src/oxcalc-core/src/structural.rs:144`), and like `TreeCalcTableNodeSnapshot` keeps cells virtual with selective materialization (`structured_table.rs:132–151`). Do **not** mint a `TreeNodeId` per cell. Introduce a typed calc-unit identity:

```rust
enum CalcTarget { Node(TreeNodeId), GridRect { sheet: TreeNodeId, extent: SparseRangeExtent }, GridCell { sheet: TreeNodeId, coord: SparseCellCoord } }
```

reusing `SparseCellCoord`/`SparseRangeExtent` verbatim (`sparse_reader.rs:17–66`). `DependencyDescriptor.target_node_id: Option<TreeNodeId>` (`dependency.rs:40`) generalizes to this enum; owner side likewise. The dependency graph, invalidation closure, and recalc tracker operate on **regions** (maximal rects of R1C1-identical formulas, or constant blocks) plus per-cell "loner" units for unique formulas. This is the LibreOffice formula-group / TACO compressed-graph model, and the Enron stat (4.5% unique formulas by relative-R1C1) says region granularity captures real sheets.

**2. One bound template, per-cell closure.**
A `GridFormulaTemplate` = R1C1-canonical text + one OxFml bound artifact with *symbolic* (offset-form) references, instantiated per cell as `(caller_row, caller_col)` closures over one cached `CompiledFormulaPlan`. The in-repo precedent is the table column formula: one formula, per-row evaluation (`structured_table.rs:3501`). This requires the OxFml change the OxFml recon scoped (symbolic bound-ref form; drop caller anchor from `bind_context_fingerprint`, `OxFml binding/mod.rs:302–315`; offset resolution in `CompiledReferenceExpr::Atom`) — it is the single most load-bearing cross-repo dependency; file the handover first. Catalog side: `TreeFormulaCatalog`'s one-`TreeFormulaBinding`-per-node (`formula.rs:2697–2716`) gains a sibling `GridTemplateCatalog: BTreeMap<TemplateId, GridFormulaTemplate>` with regions holding `(template_id, extent)`. This also kills the per-node-per-run `prepare_oxfml_formula` cost (`treecalc.rs:798–811`) for grid content: prepare once per template.

**3. Edges are affine rect relations; reverse lookup is an interval index.**
New `DependencyDescriptorKind` variants (extending `dependency.rs:13–33`): `GridTemplateRelative` (region→region with an affine offset, e.g. fill-down of `=R[-1]C+1` is one edge `(region, region, Δrow=-1)`), `GridRangeRegion` (consumer→rect, for `SUM(A1:A1000000)` from a cell or a tree node), `GridCellDirect` (loners). Dirty propagation maps **rectangles through affine relations**: a dirty rect in the source becomes a dirty rect in the target — Sestoft's FAP-set compression specialized to the common case. Replace per-target `reverse_edges: BTreeMap<TreeNodeId, Vec<DependencyEdge>>` (`dependency.rs:246–254`) for grid targets with a per-sheet **interval tree of listening rects** (Excel range-listener / HyperFormula range-node model): edit at (r,c) → O(log n + k) query for affected consumers. `derive_invalidation_closure` (`dependency.rs:450`) gains `InvalidationSeed::GridRect` and produces per-region records carrying dirty-rect sets — `Stage1RecalcTracker::new`'s entry-per-node materialization (`recalc.rs:80–92`) must become per-region + rect, never per-cell.

**Key invariants.**
- *Rect over-approximation soundness*: propagation may mark clean cells dirty (rect unions), never the reverse; evaluation idempotence makes this perf-only.
- *Punch-through layering*: region evaluation consults overrides before template (below); a region's edges always claim its full extent, so override edits never require edge surgery on the template.
- *Scalar-expansion equivalence*: every `GridSnapshot` has a defined lowering to one `Calculation` node per occupied cell with `StaticDirect` edges; optimized engine ≡ scalarized engine on published values, per run.

## Integration with what exists

Reuse as-is: `SparseRangeReader` as the value surface for ranges and the OxFml `ReferenceSystemProvider` backing (`sparse_reader.rs`; `OxFml eval/mod.rs:1823–1899`); `TreeCalcTableVirtualAnchor` coordinate vocabulary (`structured_table.rs:51`) and its host projection `TableAnchorProjection` (DnaTreeCalc `workspace.rs:1946`); `PushVisibilityBounded` scheduling plan (`treecalc.rs:454–460`, planner at `:2654–2750`) generalized from observer node-ids to observer **rects** — its declared semantic-equivalence + starvation framing (`treecalc.rs:8392–8395`) transfers directly; `OxCalcTreeEdit` transaction surface (`consumer.rs:297–338`) extended with grid edit variants; TraceCalc as oracle (`src/oxcalc-tracecalc`). Bypass `EdgeValueCache` for grid cells entirely — it's string-keyed and already the warm-run pathology (`value_cache.rs:173`; DnaTreeCalc `docs/ux/stack-requirements/ROADMAP.md:206–210`); region-level value epochs replace it.

Generalize: descriptor/seed/record target typing (above); the per-run full graph rebuild (`treecalc.rs:845`) is untenable at grid scale — the grid graph must be **persisted and incrementally maintained** across runs. That overlaps the in-flight calc-perf workstream (calc-ekq3); sequence explicitly (open Q4).

## Hard cases

**Overridden cell in a template region.** Keep the region intact; overrides live in a sparse punch-through map on the region: value override → constant cell; formula override → loner calc unit with its own edges. Do **not** split rectangles eagerly (splitting causes O(edits) region churn; DataSpread's hybrid-region lesson). Coalesce/split lazily past a density threshold. The scalarizer treats overrides uniformly, so the oracle covers this for free. Derisk with a metamorphic test family: random fill → random punch-throughs → random edits, optimized vs scalarized.

**Volatile functions.** Volatility is already a template-level fact via OxFml's `ExecutionProfileSummary` (`OxFml semantics/mod.rs:176–198`). A volatile template puts its region on a per-sheet always-dirty list (Sestoft §3.3) — cheap to represent, catastrophic to evaluate at 1M rows. Under rect-bounded visibility scheduling, only the visible intersection evaluates per cycle; off-screen volatile cells sit `DirtyPending` with stale epochs. That is a deliberate Excel deviation and must be a spec'd profile flag (open Q2).

**Grid↔tree cross-references.** Tree node → grid range: a `GridRangeRegion` descriptor with `owner_node_id` as today; the graph is unified, not parallel. Grid template → tree name: binds through `HostNameResolver` (`OxFml binding/mod.rs:101–117`) exactly as current formulas do (`consumer.rs:36–39`), and templating makes it *one* region→node edge for the whole fill — strictly better than today. The heterogeneous-target precedent is `workspace_reverse_edges` (`dependency.rs:246–254`).

**Cycles / iterative calc.** Honest finding from code: iterative calc today is **fixture-only** — `excel_match_iterative_fixture_surface` returns hard-coded values keyed on `runtime_policy_id` substrings (`treecalc.rs:1615–1660`); there is no general iterative evaluator to integrate with. Region-level Tarjan (`dependency.rs:579–660`) over-approximates: fill-down `=R[-1]C+1` is a region self-edge but cell-acyclic. Required: an SCC **refinement pass** — a self/intra-SCC edge whose affine offsets are strictly monotone in one axis is acyclic and evaluates as a scan (the `SiblingOffset`/@PREV precedent, `formula.rs:86–144`); only zero/mixed-offset SCCs become true cycle groups, then fall into the W048/W055 cycle-profile lane. Derisk the monotonicity check first with property tests vs the scalarizer; defer real iterative semantics.

**Threading path (defer, but shape now).** Engine is single-threaded and passive (zero sync primitives in `src/`). The region condensation DAG gives Excel-MTR-style level scheduling; within a region with no intra-region offsets, rows are embarrassingly parallel (LibreOffice formula groups). Build only the **partition witness** now: emit per-run a trace proving same-level regions have disjoint read/write rects, checkable by TraceCalc — that's the Stage-2 seam.

## Build order

1. **Scalarizer + conformance harness** (the simple-correct machine is the existing engine over materialized nodes, capped ~50k cells) — the equivalence oracle exists before any optimization.
2. **Region/typed-target plumbing**: constants + loner formulas only; generalize descriptor/seed/tracker; no templates yet.
3. **OxFml symbolic-ref handover + template instantiation loop** (modeled on `structured_table.rs:3501`) + plan caching.
4. **Rect propagation + interval index**; metamorphic edit storms vs oracle (W033 style).
5. **Punch-through overrides, volatile region list, SCC refinement.**
Defer: iterative calc, actual threading, lazy/streaming aggregates, automatic region inference.

## Open questions for the owner

1. **Unified graph vs parallel graph**: I recommend generalizing `CalcTarget` inside the one `DependencyGraph`; the alternative (separate grid graph joined at boundary edges) doubles planner/closure code. Confirm appetite for touching core identity types.
2. **Volatile + viewport semantics**: is "off-screen volatile cells stale until visible" acceptable under a documented capability profile, or must volatility force full evaluation (killing viewport prioritization for volatile sheets)?
3. **Region authorship**: host-declared regions (fill/stamp intents create them, engine verifies R1C1-identity) vs engine-inferred coalescing of identical neighbors. I recommend host-declared, engine-verified — it matches the intent/dispatch discipline and avoids inference heuristics in the engine.
4. **Sequencing vs calc-perf (calc-ekq3)**: persistent incremental graph maintenance is needed by both; who lands it, and does grid work assume per-run rebuild in slices 1–3?
5. **Spec placement**: TreeCalc charters "not a grid" (`CHARTER.md:44`); confirm the grid spec lands as a lane-owned OxCalc core-engine spec (`OxCalc/docs/spec/core-engine/`) targeting the PreCalc host, leaving TreeCalc's non-goal intact.
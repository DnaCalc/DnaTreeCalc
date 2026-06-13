# DESIGN PROPOSAL — Specification & Verification Strategy for OxCalc Grid Support

## 1. Recommendation in one paragraph

Write one lane-owned semantic spec (`CORE_ENGINE_GRID_MODEL.md`) defining a sheet as a total function from bounded coordinates to cell-state with finite support, with formula identity given by an **R1C1-relative normal form**. Pair it with a grid extension of the existing TraceCalc reference machine (`OxCalc/src/oxcalc-tracecalc/src/machine.rs`, `oracle_matrix.rs`) as the simple-correct implementation, and define correctness of the optimized engine as **observational refinement at three read surfaces** under an explicit abstraction function — never trace equality. "Traceably correct" = a maintained **invariant register**: every optimization gets a named invariant, its abstraction-function clause, a targeted expand-and-compare oracle, and pinned corpus cases. This reuses machinery that already exists: TraceCalc conformance mismatches (`oxcalc-tracecalc/src/contracts.rs:355`, `assertions.rs:120`), the closed-form scale runner (`oxcalc-core` `treecalc_scale.rs:32`), DnaTreeCalc intent-log replay (`src/dnatreecalc-skin-framework/src/intent.rs:683,712`), OxReplay diff, and OxXlPlay Excel captures.

## 2. Semantic model (the spec)

State space, in `CORE_MODEL_SPEC.md` style (numbered sections, ownership tables, "what this spec does NOT specify"):

- `Grid : SheetId → (Row × Col) → CellState`, **total** over `Row=[1,1048576] × Col=[1,16384]`, with **finite support** (all but finitely many cells are `Empty`). Out-of-bounds is not a coordinate; references that shift past bounds become `#REF!` (closing OxFml gap: unbounded `column_to_index`, `binding/mod.rs:3049`).
- `CellState = Empty | Literal(CalcValue) | FormulaCell(TemplateId, anchor) | SpillTarget(anchor)`. `TemplateId` keys a **canonical R1C1-relative formula** — the spec's formula identity. A1 text (`$`-fidelity included) is a presentation channel; two cells are spec-identical formulas iff their R1C1 normal forms are equal. This makes template sharing a *representation choice the spec never mentions* (matching the "mandate behavior not representation" convention, `DnaTreeCalc/docs/model/META_NODES.md:61`), and is backed empirically (Enron: 20.3M formulas, 4.5% unique by relative-R1C1).
- Evaluation semantics: a recalc relation `Eval(Grid, dirty) → Values` defined as the least fixpoint over occupied cells, with calc order **existentially quantified** (any dependency-consistent order is conforming — Excel's self-optimizing chain and our viewport scheduling are then trivially in-spec). Blank reads are `SparseCellRead::Blank` (`oxcalc-core/src/sparse_reader.rs:17–110`) with coercion owned by OxFunc — state that in the ownership table, don't respecify.
- Spec lives in **OxCalc**, not DnaTreeCalc: TreeCalc is chartered non-grid (`DnaTreeCalc/CHARTER.md:44`, `docs/SCOPE.md:39`); Foundation doctrine puts canonical specs in the owning lane. DnaTreeCalc gets only a handover + an eventual `CORE_MODEL_SPEC.md` § noting the `strict-excel` profile interaction (`CORE_MODEL_SPEC.md:293`).

## 3. Reference vs optimized, and the refinement relation

**GridCalc-Ref (simple-correct):** extend `oxcalc-tracecalc` — per-sheet `BTreeMap<(u32,u32), CellState>`, every occupied formula cell bound and evaluated independently (no template sharing), recalc = mark-all-dirty + naive topo evaluation, no caches, no virtual anything. ~2–3k LOC. It is the executable form of the spec (the Calc.ts/Funcalc pattern; we already have the precedent in `CORE_ENGINE_TRACECALC_REFERENCE_MACHINE.md`).

**Refinement relation:** define `α : OptimizedState → SpecState` clause by clause (sparse blocks flatten to the finite-support map; a template region expands to per-cell `FormulaCell(t, anchor)`; virtual cells are spec-cells with no `TreeNodeId`). Conformance = equality after `α` at three observation surfaces:

1. **Value readout** through `SparseRangeReader` — both engines answer the same coordinate probes;
2. **Invalidation closure as a set** (compare against ref's recomputed-cell set; order excluded);
3. **Committed effects/errors** (`#REF!`/`#SPILL!` placement, spill extents).

Diagnostics, timings, epochs-as-numbers are explicitly outside the relation.

## 4. Equivalence testing machinery

- **Differential harness:** extend `oxcalc-tracecalc/src/runner.rs` + `independent_conformance.rs`: one scenario → both engines → mismatch vector (`contracts.rs:366`). Readout is **sampled**, not exhaustive: all occupied cells + boundary probes (row 1048576, col XFD/16384, block edges) + N random blank probes per sheet.
- **Generator (property-based), biased to the nasty cases:** template region with per-cell overrides punched into it; insert/delete rows/cols crossing region and block boundaries; references shifting off-grid (→`#REF!`); spill anchors whose extents collide with occupied cells and with template regions; cross-sheet refs; volatile cells; whole-row/col refs over sparse occupancy. Seeded, shrinking, scenario serialized to the declarative JSON corpus format (`DnaTreeCalc/docs/SPEC.md:65` pending→active discipline).
- **Scale tier:** differential capped at ~10⁵ occupied cells (ref engine budget). Above that, use the `TreeCalcScaleProfile` pattern — closed-form-checkable workloads (SUM pyramids, prefix chains over template regions) where expected values are computed analytically, plus metamorphic properties (§5) on the optimized engine alone.
- **Excel oracle tier:** OxXlPlay COM captures (`OxXlPlay/scripts/invoke-excel-observation.ps1`, retained `states/excel/` families) pin Excel-semantics cases — insert/delete ref adjustment, `#REF!` text forms, spill collisions. The grid is precisely where the Excel oracle becomes directly usable (unlike the tree model). Gaps logged in the Foundation `KNOWN_GAPS_AND_UNCERTAINTIES.md` pattern.

## 5. Metamorphic properties (cheap at scale, no oracle needed)

1. **Translation invariance:** translate the entire occupied region by (Δr,Δc) within bounds → values equal modulo translation. Directly exercises R1C1 normal-form correctness. (Precedent: `OxCalc/docs/.../W033_METAMORPHIC_SCALE_SEMANTIC_BINDING.md`.)
2. **Materialization invariance** — the key new one: forcing any virtual cell to a real `TreeNodeId` binding (the `body_cell_nodes` move, `structured_table.rs:119–123`) changes no value anywhere.
3. **Recalc idempotence:** a second no-edit recalc is a value-level no-op and strictly cheaper — also a regression tripwire for the warm-10–80×-slower pathology (`docs/ux/skin-suite/PHASE_B.md:12`).
4. **Insert-then-delete identity** (row/col), including across template regions.
5. **Edit-order independence** for dependency-independent edit pairs.
6. **Viewport-schedule invariance:** any `PushVisibilityBounded` schedule (`treecalc.rs:454–460`), once quiesced, equals full recalc — turning the equivalence claim at `treecalc.rs:8392–8395` into a tested property.

## 6. Replay integration

The DnaTreeCalc intent log (`IntentRecord`, `intent.rs:683`; `replay()`, `:712`) is the workload format: replay one recorded session into ref-backed and optimized-backed engine builds, diff via OxReplay (its chartered role: "compares + governs"). Retained runs under `docs/test-runs/<grid-workset>-NNN/`. This makes every real UX session a free differential test once grid intents exist as enum variants.

## 7. "Traceably correct," operationally

The **Invariant Register** (one table in `CORE_ENGINE_GRID_REFINEMENT_AND_EQUIVALENCE.md`): per optimization — invariant statement, `α`-clause, targeted oracle (a debug-assert or test that *expands the compressed form and compares to naive*, e.g., expand FAP/TACO-style compressed dependents vs per-cell reverse edges; expand block index vs BTreeMap), corpus case ids, and the metamorphic properties that cover it. PR rule: no optimization merges without a register row. Full formal proof is explicitly out of scope; the Lean/TLA lanes (w033–w037) remain available for the recalc relation later.

## 8. Document set

| Doc | Location |
|---|---|
| `CORE_ENGINE_GRID_MODEL.md` (semantic model, bounds, R1C1 normal form, eval relation) | `OxCalc/docs/spec/core-engine/` |
| `CORE_ENGINE_GRID_REFINEMENT_AND_EQUIVALENCE.md` (α, surfaces, Invariant Register) | same |
| `CORE_ENGINE_GRID_REFERENCE_MACHINE.md` (extends TraceCalc ref doc) | same |
| Grid corpus JSON + generator scenarios | `OxCalc/docs/test-corpus/grid/` (TreeCalc corpus-format reuse) |
| `HANDOVER_OXFML_r1c1_normal_form.md` (symbolic bound refs, `$`-fidelity — cross-lane ask) | `DnaTreeCalc/docs/handovers/` per convention |
| Workset `W0xx_GRID_*` + paired `calc-*` beads | `DnaTreeCalc/docs/WORKSET_REGISTER.md` |

## 9. Build order

**First:** (1) `GRID_MODEL` §1–3 + bounds/#REF!; (2) GridCalc-Ref + ~40 hand-written corpus cases (boundary, overrides, blanks); (3) differential harness + generator v1 (templates, overrides, insert/delete); (4) Invariant Register seeded with block-storage and template-interning invariants. **Defer:** Lean/TLA formalization; full spill/dynamic-array semantics (pin corpus cases as `pending` now); automated Excel-COM in CI (manual capture refresh suffices); viewport metamorphic at 10⁷ scale.

## 10. Hardest problems + derisking

1. **Ref engine can't reach 10⁷ cells** → three-tier strategy (§4): differential ≤10⁵, closed-form profiles + metamorphic above, sampled readout always.
2. **Insert/delete shift semantics** (ref adjustment, region splitting, `#REF!` creation) is the largest semantic surface and a known pathology (337 s rebind churn baseline) → spec it *first* from OxXlPlay Excel captures; make it generator bias #1; Sestoft §2.8 adjustment-memo as the design reference for the ref engine.
3. **Cross-lane dependency on OxFml** for the R1C1 normal form (A1 `$` is dropped today, `OxFml binding/mod.rs:2926–2956`; bind pre-resolves coords) → the spec can define the normal form textually and GridCalc-Ref can implement adjustment over bound refs before OxFml ships symbolic refs, but file the handover immediately.

## 11. Open questions for the owner

1. Confirm **R1C1-relative normal form as canonical formula identity** (forces the OxFml symbolic-bound-ref work onto the critical path) — recommended yes.
2. Equivalence scope: values+errors+dirty-sets only (my recommendation), or also calc-state machine parity (`recalc.rs:12` states)?
3. Is Excel-the-binary the oracle of record for grid edge semantics, and which pinned build via OxXlPlay?
4. Does the grid land as a PreCalc-era host or as a TreeCalc workspace facet? Determines where intent-log grid workloads and the corpus *runner* live (OxCalc-only until a grid host exists?).
5. Do iterative-calc cycle profiles (`excel_match_iterative`, CORE_MODEL_SPEC §7a) apply to grid v1, or is cycle = error initially?
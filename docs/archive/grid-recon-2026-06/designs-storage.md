# DESIGN PROPOSAL — Sparse Grid Data Model & Memory (lens 1 of 4)

## Headline recommendation

Add a per-sheet **block-chunked, copy-on-write `SheetStore`** that lives *outside* the structural tree, attached as a node facet (the `SetTableShape` precedent, `OxCalc/src/oxcalc-core/src/structural.rs:144`, shape carrier at `:65-78`). One `StructuralNode` per sheet; zero tree nodes per cell. This is non-negotiable: `StructuralSnapshot` clones its whole `BTreeMap` per edit (`structural.rs:631`) and rebuilds a string `path_index` (`structural.rs:559`) — any design that puts cells in the tree dies at 10^5 cells, let alone 10^10 addressable. Cells get `TreeNodeId`s only on demand, exactly like `body_cell_nodes` (`structured_table.rs:119-123`).

## Core data structures

**Addressing.** Reuse `SparseCellCoord { row: u32, column: u32 }` (`sparse_reader.rs:17-28`) as the public coordinate; internally `PackedCellAddr(u64)` = `row(20b) | col(14b)` (1,048,576 = 2^20, 16,384 = 2^14 — 34 bits). Bounds are clamped constants `MAX_ROW/MAX_COL`; out-of-bounds is `#REF!` at the OxFml seam (which today has no clamping — `OxFml/crates/oxfml_core/src/binding/mod.rs:3049`). Addressability is purely arithmetic; nothing is materialized for blank space (**Invariant I3: a blank cell costs zero bytes; memory ∝ occupancy, never extent**).

**Block grid.** Sheet → `BTreeMap<BlockKey, Arc<Block>>`, block = **256 rows × 32 cols (8,192 slots)**, `BlockKey = (row_band: u16, col_band: u16)`. Each block is adaptive:

```rust
enum BlockRepr {
    Dense(Box<[CompactCell; 8192]>),            // occupancy > ~12%
    Sparse(Vec<(u16 /*intra*/, CompactCell)>),  // sorted, binary search
    Run(Vec<(u16, u16, CompactCell)>),          // RLE for constant fills
}
```

**`CompactCell` — 16 bytes, fixed.** 1-byte tag + payload: `f64` inline; `bool` inline; error as `u8` code; text as `u32` id into a workbook-level interned string table (xlsx sharedStrings precedent); rich/array values as `u32` index into a per-sheet side table holding real `CalcValue`s. Crucially, do **not** store `CalcValue` per cell: it is `{ core: CoreValue, rich: Option<Rc<RichValue>> }` (`OxFunc/crates/oxfunc_value_types/src/lib.rs:374-389`) — ~40+ bytes, heap text, and `Rc` makes it `!Send`, which poisons Stage 2 partitioned-parallel plans. A lossless `CompactCell ↔ CalcValue` codec sits at the reader boundary (**Invariant I6: round-trip preserves `PartialEq`**).

**Two value layers, same geometry.** An *authored* layer (literals + formula-cell markers carrying a `u32` template-region id — the template lens owns what that id means) and a *computed* layer for published results. They churn at different rates (edit vs recalc) and the computed layer replaces `BTreeMap<TreeNodeId, CalcValue>` publication (`consumer.rs:803-809`) for grid cells — grid results must never pass through per-node BTreeMaps.

**Formatting layer — separate, never coupled to value blocks.** `StyleId(u32)` interned into a dedup'd style table (xlsx `cellXfs` precedent), with cascade sheet-default → col-default → row-default → runs → cell override. Storage is per-row-band column-run RLE (`Vec<(col_start, len, StyleId)>`), falling back to the same adaptive block repr (4 B/slot dense) for pathological regions. Formatting-only sheets allocate zero value blocks.

**Row/col structural metadata.** Heights, hidden, outline level: run-length `Vec<(start, len, Props)>` with implicit defaults. Never per-row arrays — 1M rows of alternating-hidden stripes stays KB-scale.

**Immutability/COW.** A published `SheetSnapshot` is immutable (**I2**); an edit clones the block map (thousands of `Arc` pointers even for huge sheets) and COWs only touched blocks (≤ 128 KB dense block clone). This slots directly into `WorkspaceRevision` retention (`treecalc.rs:160-179`), candidate overlays (`consumer.rs:789-801`), and gives version-based undo (CORE_MODEL §6 #13, `revision-graph-retention`) at O(touched blocks) per step — retained revisions share all untouched blocks structurally.

**Read surface.** Everything reads through `SparseRangeReader` (`sparse_reader.rs:17-110`), implemented by a `SheetGridSparseReader` exactly as `TreeCalcTableSparseReader` does (`structured_table.rs:2973`). `defined_iter` walks occupied blocks only — whole-column `SUM(A:A)` touches occupancy, not 2^20 slots (**I4: no API returns dense rectangles of unbounded extent**). This same iterator, block-keyed, is the streaming feed for the rendering pass and the windowed projection the DnaTreeCalc B.2.3 protocol needs (`docs/ux/skin-suite/PHASE_B.md:68-98`), using `TableAnchorProjection` coordinates (`DnaTreeCalc/src/dnatreecalc-skin-framework/src/workspace.rs:1946`).

## Memory quantification (16 B/cell values, 4 B/cell styles, headers amortized)

| Case | Layout | Cost |
|---|---|---|
| 1M rows × 10 numeric cols (the "boring spreadsheet") | Dense blocks | ~160 MB values + ~0 style (runs) ≈ **~17 B/cell** vs Excel's observed ~37 B, vs OxCalc today ~11 KB/node |
| Full column, 1M numbers | Sparse repr in 4,096 blocks (1 col of 32 occupied) | ~20 B/cell ≈ **20 MB** |
| Adversarial zig-zag, 1M isolated singletons (worst block fragmentation) | 1 cell/block × 1M sparse blocks (hdr ~64 B + entry) | ≈ **~85 B/cell, 85 MB** — bounded, no cliff |
| Doom (cell=pixel formatting, ~320×200 viewport region) | Dense style blocks | 64 K cells × 4 B ≈ **256 KB** |
| Whole-sheet per-cell-unique formatting (17.2e9 cells) | unrepresentable at 4 B/cell (~69 GB) — **explicitly out of scope**; cost is ∝ formatting *complexity* via runs, same practical cap as Excel |

## Mutation patterns

- **Single edit:** O(log blocks) locate + COW one block. Microseconds; emits one region-granular `InvalidationSeed`.
- **Large paste:** build blocks directly from the source iterator, block-aligned; promote Sparse→Dense on threshold. O(pasted cells), no per-cell tree edits, one transaction (needs CORE_MODEL §6 #8 `transaction-scope`).
- **Insert/delete rows/cols — the killer.** Recommendation: **logical→physical translation layer** per axis, an order-statistic/counted-B-tree mapping (DataSpread's hierarchical positional mapping, arXiv 1708.06712). Block keys are *physical* and immortal (**I5**); insert = O(log n) splice plus splitting at most boundary blocks. Eager renumbering (shift every block below the insertion) is the *simple-correct reference implementation* — ship it first, keep it forever as the oracle. This pairs perfectly with the project's simple-vs-optimized equivalence mandate: same edit script into both, compare logical reads via the TraceCalc conformance pattern (`src/oxcalc-tracecalc`). Note OxCalc already has a measured rebind-churn pathology (337 s soft-reference update, `docs/test-runs/core-engine/treecalc-scale/BASELINE_2026-05-04.md`) — physical-stable keys are how the grid avoids importing it.

## Three hardest problems & derisking

1. **Insert/delete vs stable block keys and region maps.** Risk: translation layer leaks into every read (double indirection on hot paths) or drifts from the eager-renumber reference. Derisk: property-test harness replaying random edit scripts into both implementations, comparing full logical reads; benchmark read-path overhead of translation (target <10%); only adopt if the eager version's measured insert cost actually hurts (it will at 1M rows, but prove it).
2. **`CompactCell ↔ CalcValue` fidelity.** Rich values, `ExcelText` semantics, arrays-in-cells must survive the codec; `Rc` in `CalcValue` (`oxfunc_value_types/src/lib.rs:388`) cannot enter block storage. Derisk first: write the codec before any block code, round-trip fuzz against `PartialEq`, microbench bytes/cell on representative corpora.
3. **Layer independence under degenerate loads.** Formatting-dense/value-empty, value-dense/format-empty, and constant-fill (Run repr) must each cost only their own layer. Derisk: three named stress corpora under `DnaTreeCalc/docs/test-corpus/perf/` (full-column, zig-zag, dense-format) with **byte-budget assertions** via the closed-form-checked scale-runner pattern (`treecalc_scale.rs:32`), per TECHNICAL.md §7.6 measurement doctrine.

## Build order

**First:** `PackedCellAddr` + bounds/#REF! constants; block store with Dense+Sparse only; `CompactCell` codec + fuzz; Arc-COW `SheetSnapshot`; `SheetGridSparseReader`; single-edit + paste; the four memory benchmarks. **Second:** style table + run layer; row/col metadata runs; eager-renumber insert/delete (reference). **Defer:** translation-tree insert/delete, Run value repr, workbook string interning, column-strip block specialization, computed-layer eviction for off-viewport regions.

## Open questions for the owner

1. **Crate placement:** new module inside `oxcalc-core` behind the sheet facet (my recommendation — it must interleave with revisions/overlays) vs a new `oxcalc-grid` crate?
2. **Publication fork:** grid computed values bypass `PublishedRuntimeLayerPayload` (`consumer.rs:803`) into the computed block layer. This forks the engine's publication model — acceptable, or must grids publish through a unified seam?
3. **Charter sequencing:** CHARTER.md:44 says the grid arrives with PreCalc. Is this work an OxCalc-lane substrate spec'd now and consumed by a future host, or does TreeCalc's `atlas` line grow a grid surface? Determines where the spec doc lands (`OxCalc/docs/spec/core-engine/CORE_ENGINE_<TOPIC>.md` naming, per WORKSET_REGISTER conventions).
4. **`Rc` in `CalcValue` vs Stage 2 threading:** may I assume a future `Arc`/`Send` migration in OxFunc, or must the codec guarantee `Send` storage forever (side tables thread-confined)?
5. **Row heights / column widths ownership:** TRACEABILITY.md doctrine says viewport state is skin-local, but Excel persists heights/widths in the model and layout math needs them engine-adjacent for windowed projection. Sanction model-side storage?
6. **Number-format display strings:** cache formatted text per cell (memory) or compute in the render pass (CPU)? Determines whether the streaming protocol carries raw `CompactCell`s (my recommendation) or formatted runs.
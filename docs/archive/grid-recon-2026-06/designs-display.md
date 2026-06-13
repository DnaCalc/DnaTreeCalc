# Lens proposal — Display pipeline, viewport & recalc prioritization

## Stance

The engine owns *layout truth* (axis geometry, viewport declarations, visibility priority) and a *tile-streamed value surface*; the rendering pass owns pixels. The pivotal doctrinal move: **visibility never changes dirtiness — it only changes evaluation order and publication timing.** That keeps `TRACEABILITY.md:191-198` (F10 "resize never recalcs") honest in spirit while enabling visible-first scheduling, and it is exactly the contract `PushVisibilityBounded` already declares (`OxCalc/src/oxcalc-core/src/treecalc.rs:454-460`, equivalence statement + starvation caveat at `treecalc.rs:8392-8395`).

## Architecture: three new engine-adjacent components

### 1. `SheetViewModel` — visible-region model (engine-owned)

```rust
GridRect { start: SparseCellCoord, row_count: u32, column_count: u32 }  // = SparseRangeExtent, sparse_reader.rs:31
ViewRegion { sheet: SheetId, rect: GridRect, class: ViewRegionClass }
ViewRegionClass = Visible | FrozenPane | PrefetchHalo | Camera | Watch
ViewportDecl { view_id: ViewId, regions: Vec<ViewRegion>, generation: u64 }
```

Everything — frozen panes (up to 4 quadrant rects per pane split), split views (2–4 independent scroll rects over one sheet), camera/watch ranges (rects possibly on *other* sheets) — is a uniform list of prioritized rects. No special cases downstream: the scheduler and the streaming layer consume `Vec<ViewRegion>` only. Reuse `SparseRangeExtent`/`SparseCellCoord` (`sparse_reader.rs:17-66`) as the coordinate vocabulary; the host-side mirror is `TableAnchorProjection`-shaped (`DnaTreeCalc/src/dnatreecalc-skin-framework/src/workspace.rs:1946`).

`ViewportDecl` updates arrive as a **new read-side channel, not a `WorkspaceIntent`**: they are not document edits, must not enter the replay log (`intent.rs:683`) as authored history, and must never produce a revision. Add a `declare_viewport()` method beside the preview seam (`skin-framework/src/preview.rs:34`) — same pure-observer family.

### 2. `AxisLayout` — row/col layout solving (engine-owned)

Per sheet, per axis:

```rust
AxisLayout {
  default_size_px: u16,                       // logical px at zoom 1.0
  runs: Vec<AxisRun { start: u32, count: u32, size_px: u16, hidden: bool, outline_level: u8 }>,
  prefix: FenwickOverRuns,                    // pixel prefix sums over runs
}
```

Invariants: runs are sorted, non-overlapping, coalesced (adjacent identical runs merge); unrepresented indices have `default_size_px`; hidden = size contribution 0 (height preserved for unhide); outline collapse compiles to hidden runs. Operations: `index→pixel_offset` and `pixel→index` in O(log r) where r = run count (typically hundreds, worst case 1M — still 20 probes); insert/delete rows = run splice. Sizes are integer logical pixels at zoom 1; **zoom is a renderer transform**, so layout solving never touches floats and two views at different zooms share one `AxisLayout`. The renderer holds a mirrored copy (it's small and delta-able as run splices) so hit-testing, scrollbar geometry, and pixel→cell are synchronous local queries — no round-trip on scroll.

Why engine-owned: viewport rect → occupied-cell resolution must happen where prioritization happens, and `GridRect` from pixels requires the same prefix structure. This amends the skin-suite ownership table (`docs/ux/TRACEABILITY.md:19-26` puts column widths in skin meta) — for the grid host, axis geometry is document state (as in xlsx), not skin state. Skin-local remains: zoom, scroll position, selection chrome.

### 3. Tile streaming protocol (engine → rendering pass)

**Unit: fixed power-of-two tiles, recommended 64 rows × 16 cols (1,024 cells), aligned to (an integer divisor of) the storage lens's block size.** Tiles beat row bands — a 16,384-col row band is unboundedly wide; tiles bound every message by viewport area. Tile coords: `TileCoord { sheet, tile_row: u32, tile_col: u32 }`.

Subscription model over the B.2.2 worker boundary (`docs/ux/skin-suite/PHASE_B.md:68-98`):

- Renderer → worker: `GridSubscribe { view_id, tiles: Vec<TileCoord>, epoch_basis: Vec<(TileCoord, TileEpoch)> }` — re-sent on scroll (cheap; it's a tile-set diff).
- Worker → renderer: `TilePatch { tile, tile_epoch, calc_summary: AllClean|HasDirty|HasError, payload }`. `payload` is **columnar with two lanes**: (a) *display lane* — run-length-encoded `(formatted_text, style_id, align)` runs with blanks compressed away (the worker does number formatting; the rendering pass never needs OxFml/format logic); (b) *raw lane* — packed typed scalars (`f64`/`u32`) for tiles the renderer marks `raw=true`. Per-tile monotone epochs; an epoch gap ⇒ per-tile resync request — this is exactly the B.2.3 `projection_seq`-gap/`FullReset` protocol (`PHASE_B.md:78-86`) recursed down to tile granularity.
- Backpressure (B.2.4 pattern, `PHASE_B.md:88-94`): worker keeps **at most one pending patch per tile, latest epoch wins**; renderer acks frames; superseded patches are dropped, never queued. This is the grid analogue of `Coalesced{into_seq}` receipts.

This channel **bypasses `WorkspaceState` entirely**. The full-snapshot pipeline (`session.rs:5077` re-projection → `dispatcher.rs:1415` publish → `BTreeMap<NodeId, NodeView>`, `workspace.rs:19`) stays for tree lenses; the grid lens is a normal `WorkspaceSkin` (`skin.rs:92`) that owns a `<canvas>` and consumes tile patches via the anticipated per-skin event seam (`skin.rs:62-66`). Canvas 2D with per-tile offscreen caches and damage-rect repaints (Glide/Google Sheets precedent) is the floor; WebGL deferred. The protocol's renderer-facing guarantees: monotone tile epochs, render-ready runs, mirrored `AxisLayout`, per-tile calc summary for stale shading.

## Visibility-driven recalc prioritization

Extend the scheduling policy (`treecalc.rs:454`) with a region form:

```rust
PushRegionBounded { regions: Vec<(ViewRegion, PriorityBand)>, budget: PumpBudget }
// Bands: P0 Visible+Frozen → P1 PrefetchHalo (≥1 viewport ahead in scroll direction) → P2 Camera/Watch → P3 background completion
```

Rects resolve to observer cells via the storage lens's occupancy index; from there the existing observer-upstream-closure plan (`treecalc.rs:2654-2750`) applies. Each host pump tick (sans-executor doctrine, `TECHNICAL.md:281-287`) drains bands in order under a budget; each band completion publishes tile patches incrementally. Starvation handled per the engine's own caveat (`treecalc.rs:8395`): P3 aging + periodic full-closure sweep.

**Invariant (spec-level):** for any scheduling policy and budget, a cell's published value at epoch E equals the `PullFullClosure` value — visibility affects *when*, never *what*. A dirty cell scrolled into view renders its last-published value with a stale marker; scrolling never blocks on calc.

## The Doom case (cell=pixel) as protocol acceptance test

320×200 px region, 1 cell = 1 px, every cell recomputed per tick at 30–60 Hz: 64,000 cells changing per frame. This breaks every per-cell path: per-key delta lists (`intent.rs:771`-style `ValuesChanged` would be 64k entries), formatted-string payloads, and any queue that buffers superseded frames. It survives the proposed design iff: (a) the **raw lane** ships ~63 tiles × packed `u32` ≈ 256 KB/frame via postMessage transferables; (b) **latest-epoch-wins** drops frames when calc exceeds budget; (c) one coalesced publish per pump, not per cell. Make it a retained perf workload (`docs/test-corpus/perf/` style) with the falsifiable bound: **bytes/frame ≤ k × subscribed-cell-count, independent of model size and change count.** This single test motivates the typed columnar lane from day 1.

## Hardest problems & derisking

1. **Resumable, budget-sliced scheduling.** Today `execute` is one-shot and rebuilds the graph per run (`treecalc.rs:788, 845`); budget slicing needs persistent scheduler state across pumps. Derisk: prototype band-draining on the existing `PushVisibilityBounded` with a synthetic deep-chain + wide-SUM workload *before* committing the API; state the equality-under-any-schedule property as a metamorphic spec checked against TraceCalc (`src/oxcalc-tracecalc` oracle). Dependency: this rides on B.2.0/`calc-perf` persistent-graph work — coordinate, don't fork.
2. **Visible cell with near-global upstream closure** (one visible `SUM(A:A)` over 1M rows ⇒ P0 ≈ whole graph). Derisk: per-observer closure-size estimate from the compressed dependency layer; observers exceeding budget degrade to stale-render + background completion with progress, instead of starving the rest of P0.
3. **Doctrine conflict.** Viewport-driven recalc is "mildly counter-doctrinal" (skins own viewport; F10). Derisk: a short spec amendment in `CORE_MODEL_SPEC.md` style separating *dirty-truth* (viewport-independent, engine) from *schedule-order* (viewport-driven) and reassigning axis geometry to document state — get owner sign-off before any code.

## Build order

**First:** (1) `AxisLayout` + Fenwick prefix (pure, small, testable); (2) tile-protocol serde types + per-tile epoch/resync in the skin-framework IR, exercised over today's in-process dispatcher before the worker exists; (3) `PushRegionBounded` resolving rects via occupancy; (4) canvas grid lens with tile mirror + stale shading; (5) Doom-case bench. **Defer:** WebGL, scroll-direction prefetch heuristics, autofit measurement loop, outline UI, split-view scroll linking (model the rects now, implement later).

## Open questions for the owner

1. **Stale-visible UX:** render last-published value + marker (recommended), or Excel-style block-until-clean for visible cells?
2. **Autofit:** text measurement lives in the renderer — does it report measured heights back as an intent the engine persists, or does the engine ship font metrics and compute autofit itself?
3. **Are viewport declarations document-persisted** (xlsx persists panes/selection) or session-only? Determines whether `ViewportDecl` ever touches the revision model.
4. **Tile size:** fixed protocol constant vs negotiated to match the storage lens's block size — needs a cross-lens decision with the storage proposal.
5. **Zoom/DPR:** confirm engine stays at logical-pixel zoom-1 and zoom is purely a renderer transform (Excel's autofit-by-zoom interaction is the edge case).
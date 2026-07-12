# WS14_DESIGN_LARGE_ARRAY_RESULTS — Targeted area design

> **Document role.** Targeted design for the WS-14 array-result rendering
> path. Array results (Excel spill, dynamic-array functions, FILTER,
> SEQUENCE, …) can be very large — thousands of rows, millions of cells. The
> default rendering path must stay honest, fast, and scannable across four
> sizing contexts (result hero, formula drill leaf, compare view, full
> inspector). This document carries the design from
> **requirements → test design → phased implementation**.
>
> **Read alongside:**
> - [APP_UX_REALIZATION.md](APP_UX_REALIZATION.md) — overall WS-14 realization map
> - [APP_UX_REALIZATION.md §3.4](APP_UX_REALIZATION.md#34-result-section) — result hero surface map
> - [WS14_DESIGN_FORMULA_EDITOR.md](WS14_DESIGN_FORMULA_EDITOR.md) — sibling area design
> - [`adapters/oxfml/types.rs`](../src/dnaonecalc-host/src/adapters/oxfml/types.rs) `FormulaArrayPreview` — current preview type
> - [`services/value_panel_model.rs`](../src/dnaonecalc-host/src/services/value_panel_model.rs) `ValuePanelValue::Array` — current value-panel shape
>
> **Status.** `area_design_v1` · 2026-04-26 · authoritative for the
> array-rendering phase of WS-14 implementation.

---

## 0. Reading guide

| § | Content | When to read |
|---|---|---|
| [§1](#1-why-this-is-high-risk) | Why this is high-risk | Onboarding |
| [§2](#2-design-principles) | Design principles | Always |
| [§3](#3-requirements) | **Requirements (FR / NFR / HON)** | Spec input |
| [§4](#4-architectural-decisions) | Architectural decisions | First impl pass |
| [§5](#5-sizing-modes) | Sizing modes (hero / drill / compare / inspector) | Layout work |
| [§6](#6-per-cell-rendering) | Per-cell rendering rules | Cell renderer impl |
| [§7](#7-truncation-and-honesty) | Truncation + honesty contract | Always (regression-prone) |
| [§8](#8-virtualization) | Virtualization strategy | Large-array work |
| [§9](#9-compare-view-array-handling) | Compare view side-by-side | Compare phase |
| [§10](#10-copy-semantics) | Copy semantics | Power-user phase |
| [§11](#11-bridge-and-state-contract) | Bridge contract changes + state | Bridge work |
| [§12](#12-performance-budgets) | Performance budgets | Tuning |
| [§13](#13-test-design) | **Test design + traceability matrix** | Test gates |
| [§14](#14-implementation-phases) | **Phased implementation** | Each phase |
| [§15](#15-risk-register) | Risk register | Always |
| [§16](#16-open-questions) | Open questions | Roadmap input |

---

## 1. Why this is high-risk

Arrays are the single result shape that can be **truly large**. A scalar
fits anywhere; a 1000×50 spill from `FILTER(big_table, criterion)` does
not. Without care:

1. **Naive DOM rendering breaks.** 50 000 `<div>` cells = ~150 ms first
   paint, hundreds of MB of DOM nodes, scroll jank. With four sizing
   contexts, one of which is a side-by-side compare, costs multiply.
2. **Bridge payload balloons.** Today `FormulaArrayPreview.rows: Vec<Vec<String>>`
   ships every cell as a string. A 100k-cell array becomes a ~2 MB JSON
   payload per evaluation; auto-proof on every keystroke would saturate
   the bridge.
3. **Truncation lies.** If the UI silently truncates without showing the
   full shape, the user thinks they see the whole array. CHARTER §7
   "Evidence Rule" forbids this.
4. **Compare view doubles everything.** Two arrays side-by-side, possibly
   with different shapes, with cell-by-cell mismatch highlighting. Without
   a clear strategy this is the worst-perf surface in WS-14.
5. **Mixed types within an array.** A column may hold numbers, errors,
   and `#N/A`s mixed. Per-cell renderer must dispatch on `EvalValue`
   discriminator, which means the bridge must carry typed cells, not
   stringified previews.
6. **Drill into a cell.** A user wants to ask "why is row 17 col 3 wrong?"
   That is partial-evaluation territory (`SEAM-OXFML-PARTIAL-EVAL`). The
   array surface must be the entry point for that flow, but cannot wait
   on the SEAM to land before shipping the basic rendering.

The combination — large data, four contexts, honesty constraint, and a
partial-eval seam — makes arrays the second-highest-risk WS-14 surface.

---

## 2. Design principles

| ID | Principle | Why |
|---|---|---|
| P-1 | **Default preview is small and bounded.** First paint never blocks on the full array. | Performance + honest "scan first" UX. |
| P-2 | **Truncation is always visible.** Every truncated render shows total shape and the count of hidden rows/cols. | CHARTER §7 honesty. |
| P-3 | **Cells carry their type.** The bridge sends typed `EvalValue` (or a stable enumerated repr) per cell, not stringified previews. | Per-cell renderer; mixed-types support; format propagation. |
| P-4 | **Virtualization where it matters, not everywhere.** ≤ 50 cells visible: render fully. > 50 cells: virtualize. | Avoid layout cost where the array is small. |
| P-5 | **Same renderer in all four contexts, scaled by mode.** Hero / drill leaf / compare cell / full inspector share one implementation. | One implementation surface; one set of tests. |
| P-6 | **Full array fetch is on demand.** Default preview ships the visible window only; user must explicitly request the full set. | Bridge payload bound; explicit user intent for expensive fetches. |
| P-7 | **Compare view scrolls in lockstep.** Two arrays side by side share scroll state. | Cell-by-cell visual comparison. |
| P-8 | **Copy semantics are explicit.** "Copy what's visible" vs "copy entire array" must be distinct user actions, with a confirm step on the latter when the array is large. | Avoid accidental megabyte clipboard copies. |

---

## 3. Requirements

### 3.1 Functional requirements (FR-)

#### Result hero rendering

| ID | Requirement |
|---|---|
| FR-RENDER-001 | Result hero for `EvalValue::Array` shows a header `Array[R × C]` where R, C are total rows and cols. |
| FR-RENDER-002 | Default visible window: max 6 × 6 cells. |
| FR-RENDER-003 | Cells beyond the window are NOT rendered to DOM by default. |
| FR-RENDER-004 | Truncation chip below the grid: `… +{R-6} more rows · +{C-6} more cols` when truncated. |
| FR-RENDER-005 | Spill-shape outline rendered in 1 px teal around the visible cells (visual signal of "this is one logical result"). |
| FR-RENDER-006 | Header row: column letters (A, B, C, …) muted, optional. |
| FR-RENDER-007 | Header column: row numbers (1, 2, 3, …) muted, optional. |
| FR-RENDER-008 | Both header row and column suppressible via a small `Headers` toggle in the result-foot context chip area. |

#### Per-cell rendering

| ID | Requirement |
|---|---|
| FR-CELL-009 | `EvalValue::Number` cell: right-aligned, format applied per cell's `PresentationHint` (or scenario-context format code). |
| FR-CELL-010 | `EvalValue::Text` cell: left-aligned, truncated at 32 chars with `…`, native tooltip with full text. |
| FR-CELL-011 | `EvalValue::Logical(true)` → `TRUE`; `Logical(false)` → `FALSE`. Sage soft pill background. |
| FR-CELL-012 | `EvalValue::Error(code)` cell: terracotta error code (`#DIV/0!`, etc.), centered. |
| FR-CELL-013 | `EvalValue::Reference` cell: opaque "ref" pill, target text tooltip. |
| FR-CELL-014 | `EvalValue::Lambda` cell: opaque "λ" pill. |
| FR-CELL-015 | Empty cell (lattice gap, e.g. partial-fill): muted dot or em-dash. |
| FR-CELL-016 | Mixed types within one array: each cell renders per its own discriminator (no coercion). |

#### Scrolling and virtualization

| ID | Requirement |
|---|---|
| FR-SCROLL-017 | Within an array preview, vertical scroll wheel events scroll the preview, not the page. |
| FR-SCROLL-018 | Within an array preview, horizontal scroll (Shift+wheel or trackpad) scrolls the preview, not the page. |
| FR-SCROLL-019 | Vertical scroll perf: 60 fps for arrays up to 100 000 cells. |
| FR-SCROLL-020 | When user scrolls to the bottom, render an end-of-array indicator. |
| FR-VIRT-021 | When `total_rows > 50` and the preview is in expanded mode, virtualize rows: render only visible + 1 tile above/below. |
| FR-VIRT-022 | When `total_cols > 50` and the preview is in expanded mode, virtualize cols similarly. |
| FR-VIRT-023 | Tile size: 50 rows × 50 cols. |
| FR-VIRT-024 | Tile visibility tracked via IntersectionObserver. |

#### Truncation and full-fetch

| ID | Requirement |
|---|---|
| FR-FULL-025 | Truncation chip is interactive: click → expand the preview to its expanded mode (still virtualized if very large). |
| FR-FULL-026 | Expanded mode shows total `R × C` cells, virtualized. The result hero panel grows to fit but caps at 70 vh; further scroll is internal. |
| FR-FULL-027 | "Open in inspector…" button at the top-right of the expanded preview opens a full-screen array inspector overlay. |
| FR-FULL-028 | The inspector loads cells in 50×50 tile slices on demand from the bridge. |

#### Drill leaf preview

| ID | Requirement |
|---|---|
| FR-DRILL-029 | A `FormulaWalkNode` whose value is an array shows `Array[R × C]` in its `value_preview` slot. |
| FR-DRILL-030 | Hovering the row reveals a small inline mini-grid (max 3 × 3) anchored next to the value preview. |
| FR-DRILL-031 | Click the mini-grid → opens the full-screen array inspector for that node's value. |

#### Compare view side-by-side

| ID | Requirement |
|---|---|
| FR-COMPARE-032 | Compare view's two-column body, when both sides are arrays, renders synchronized side-by-side previews. |
| FR-COMPARE-033 | Synchronized scroll: scrolling either preview scrolls the other to the same row/col. |
| FR-COMPARE-034 | Per-cell mismatch indicator: cells whose left ≠ right get a 2 px terracotta border. |
| FR-COMPARE-035 | Mismatch summary above the previews: `M of N cells differ; worst severity: <semantic|informational|coverage>`. |
| FR-COMPARE-036 | Click a mismatched cell → focus the matching `OxReplayMismatchRecord` in the mismatch list. |
| FR-COMPARE-037 | When shapes differ (e.g. `Array[6×3]` vs `Array[6×4]`): render both at their actual size; out-of-shape cells in the smaller side render with a hatch background and `<empty>` text. |

#### Cell drill-through

| ID | Requirement |
|---|---|
| FR-CELL-DRILL-038 | Click a cell → focus that cell (single-cell selection in the preview); subline below shows row/col index and full cell value. |
| FR-CELL-DRILL-039 | Right-click cell → context menu: Copy value, Copy expression for this cell, Evaluate cell as subexpression (SEAM). |
| FR-CELL-DRILL-040 | "Evaluate cell as subexpression" emits a `SEAM-OXFML-PARTIAL-EVAL` request with `(formula, row_index, col_index)`. |

#### Copy semantics

| ID | Requirement |
|---|---|
| FR-COPY-041 | Selection within preview (drag, shift+click) → highlighted cells. |
| FR-COPY-042 | Ctrl+C / Cmd+C with selection → copy selected cells as TSV (Excel-pasteable). |
| FR-COPY-043 | Ctrl+C / Cmd+C with no selection → copy visible cells as TSV. |
| FR-COPY-044 | Ctrl+Shift+C / Cmd+Shift+C → copy entire array as TSV. **If `total_cells > 10 000`, show confirm dialog**: "Copy {N} cells (~{K} KB)?" |
| FR-COPY-045 | Right-click → menu offers: Copy as TSV, Copy as CSV, Copy as JSON, Copy as Markdown table. |
| FR-COPY-046 | Copy-as-JSON shape: `{ shape: [R, C], cells: [[...], [...]] }` with typed cell values. |

### 3.2 Non-functional requirements (NFR-)

| ID | Requirement |
|---|---|
| NFR-PERF-001 | First paint of result hero with array (≤ 6 × 6 visible) ≤ 80 ms after bridge response. |
| NFR-PERF-002 | First paint of expanded preview (1000 rows, virtualized) ≤ 220 ms after click. |
| NFR-PERF-003 | Scroll throughput: 60 fps for arrays up to 100 000 cells. |
| NFR-PERF-004 | Full-fetch of a 100 000-cell array ≤ 800 ms (bridge round-trip). |
| NFR-PERF-005 | Tile fetch (50 × 50 cells) ≤ 80 ms. |
| NFR-MEM-006 | DOM memory ≤ 4 MB for an expanded preview with 10 000 cells in viewport. |
| NFR-MEM-007 | State memory: default preview window ≤ 200 KB per formula space. |
| NFR-MEM-008 | State memory: full-fetched arrays GC'd when formula space switches. |
| NFR-PAYLOAD-009 | Default bridge response (preview only) ≤ 200 KB JSON for any array. |
| NFR-PAYLOAD-010 | Full-fetch response chunked into 50×50 tiles; each tile ≤ 100 KB. |

### 3.3 Honesty requirements (HON-)

| ID | Requirement |
|---|---|
| HON-001 | Total array shape (`R × C`) is **always** visible when an array result is shown, regardless of context (hero, drill, compare, inspector). |
| HON-002 | Truncation indicator (`+N more rows · +M more cols`) is **always** visible when the preview is windowed. Never silently truncated. |
| HON-003 | Per-cell error codes (`#DIV/0!`, `#NUM!`) are rendered in error styling — never replaced by `<empty>` or hidden. |
| HON-004 | Mixed-type cells render per their own discriminator. No coercion to a "common" type. |
| HON-005 | Compare view: shape disagreement is always visible. The smaller side is not silently extended. |
| HON-006 | Bridge fetch failures (network / timeout) for tiles render the tile cell-area as a `BLOCKED` overlay with the SEAM id `SEAM-OXFML-BRIDGE-TILE-FETCH-FAILURE`, not as empty cells. |

### 3.4 Out of scope

- **In-place cell editing.** Arrays are read-only; the formula is the
  source of truth.
- **Sort / filter inside preview.** Not a WS-14 surface; users edit the
  formula instead.
- **Conditional formatting per cell within an array preview.** Today
  CF rules are evaluated at the worksheet/cell level (`VerificationConditionalFormattingRule`),
  not array-element level. CF on individual array cells gates on
  upstream design.
- **Pivot / aggregate views.** Arrays render as is.
- **Charting from an array preview.** No.

---

## 4. Architectural decisions

| ID | Decision | Reasoning |
|---|---|---|
| AD-1 | **Single `array_preview.rs` component** rendered in four sizing modes via a `mode: ArrayPreviewMode` prop. | P-5; one component, four flavors. |
| AD-2 | **Bridge ships a typed-cell preview**, not stringified rows. New types `ArrayPreviewCell` (carries `EvalValue` discriminator + small repr). | P-3, NFR-PAYLOAD-009. |
| AD-3 | **Default bridge preview window: 50 × 50** (not 6 × 6 — the UI window is a sub-set of the preview window). The bridge ships 50×50; the hero shows 6×6 of that without an extra fetch. | NFR-PAYLOAD-009; explicit "fast scroll within typical preview" without round-trip. |
| AD-4 | **Tile-based virtualization for expanded mode**, 50 × 50 tiles, on-demand fetch via a new bridge call `fetch_array_tile(formula_id, row_start, col_start)`. | P-4, FR-VIRT-021..024. |
| AD-5 | **IntersectionObserver drives tile visibility.** When a tile enters the viewport, render its cells (already-fetched) or kick a fetch; when it leaves, retain DOM nodes only for the immediate neighbours. | NFR-PERF-003, NFR-MEM-006. |
| AD-6 | **Compare view uses synchronized scroll signals.** Both previews bind to the same `RwSignal<(row_offset, col_offset)>`; either side's scroll updates the signal, both re-render. | P-7, FR-COMPARE-033. |
| AD-7 | **Selection is local UI state**, not in `OneCalcHostState`. | Unsaved selection should not survive scenario switch; local is correct. |
| AD-8 | **Copy uses ClipboardItem with multiple MIME types**: `text/plain` (TSV), `text/html` (Excel-styled HTML table), `application/json`. | FR-COPY-042..046; Excel pastes from `text/html` perfectly. |
| AD-9 | **Inspector overlay is a separate route**, not a modal. URL changes; back button works. | UX brief — "compare-with-Excel takes over the screen" pattern. |
| AD-10 | **Cell-as-subexpression drill-through gates on `SEAM-OXFML-PARTIAL-EVAL`**; UI ships with the menu item disabled and a tooltip pointing at the SEAM. | FR-CELL-DRILL-040. |

---

## 5. Sizing modes

The `array_preview.rs` component takes a `mode: ArrayPreviewMode` prop:

```rust
pub enum ArrayPreviewMode {
    Hero,         // result hero, max 6×6, no virtualization
    DrillLeaf,    // formula drill row, max 3×3 inline, hover-revealed
    CompareCell,  // compare view column, max 4 cols × 8 rows visible, virtualized vertically
    Inspector,    // full-screen overlay, virtualized in both axes
    Expanded,     // result hero in-place expansion (intermediate between Hero and Inspector)
}
```

### 5.1 Hero (default)

- **Visible window:** 6 rows × 6 cols.
- **Virtualization:** none (always renders the full 6×6).
- **Headers:** column letters + row numbers, muted, suppressible.
- **Spill outline:** 1 px teal around the grid.
- **Truncation chip:** when `R > 6` or `C > 6`, chip text:
  `… +{R-6} more rows · +{C-6} more cols`. Click → switch to `Expanded` mode.
- **Open inspector button:** top-right, only shown when `R > 50 || C > 50`.
- **Footer:** `total {R} × {C}` on the right, near the truncation chip.

### 5.2 DrillLeaf

- **Closed (default):** value preview reads `Array[R × C]` only.
- **Hover row:** floating mini-grid 3 × 3 cells positioned next to the row,
  with a "open inspector…" link at the bottom.
- **Click mini-grid:** opens Inspector for that walk node's value.
- **No virtualization** (3 × 3 fits).

### 5.3 CompareCell

- **Layout:** part of the compare view's two-column body. Each side renders
  `array_preview.rs` in CompareCell mode.
- **Visible window:** 4 cols × 8 rows initially, vertically scrollable.
- **Virtualization:** rows virtualized (always); cols virtualized if `C > 8`.
- **Synchronized scroll:** both sides bind to the same scroll signal.
- **Mismatch indicator:** cells whose left ≠ right get a 2 px terracotta
  border; mismatched-cell list summary above.
- **Shape disagreement:** smaller side's "missing" cells get a hatch
  background + `<empty>` text.

### 5.4 Inspector

- **Layout:** full-screen overlay reachable from a "Open in inspector…" link
  in any other mode.
- **Header:** scenario name, formula, total shape, "← back" button.
- **Body:** virtualized grid, both axes, with sticky headers (col letters
  on top, row numbers on left).
- **Footer:** copy-as menu, "fetch all" button (with confirm if > 10k cells).
- **Address bar:** small `R{n} C{n}` indicator showing currently focused cell.

### 5.5 Expanded

An intermediate state between Hero and Inspector. The result hero panel
grows in-place to ~70 vh; virtualization activates; user gets a richer view
without leaving the home screen. Triggered by clicking the truncation chip.

---

## 6. Per-cell rendering

### 6.1 Per-EvalValue rules

| `EvalValue` | Render | Alignment | Special |
|---|---|---|---|
| `Number(f)` | Format applied per cell `PresentationHint` (or fallback to scenario-context format code) | Right | Locale-aware separators |
| `Text(s)` | `s` truncated at 32 chars with `…`; full text in tooltip | Left | `whitespace: pre` for short multi-line |
| `Logical(true)` | `TRUE` | Center | Sage-soft pill background |
| `Logical(false)` | `FALSE` | Center | Muted-soft pill background |
| `Error(code)` | `code` (e.g. `#DIV/0!`) | Center | Terracotta text + soft terracotta background |
| `Reference(_)` | Opaque "ref" pill, target tooltip | Center | Muted background |
| `Lambda(_)` | Opaque "λ" pill | Center | Muted background |

### 6.2 ExtendedValue per cell

For a cell whose value is `ExtendedValue::ValueWithPresentation { value, hint }`,
the **per-cell hint takes precedence over the scenario-context format code
for that cell**. (The scenario-context format code is the fallback when the
function-emitted hint is absent.) This mirrors the OxFml hint integration
rule for scalars; see [APP_UX_REALIZATION §2A.2](APP_UX_REALIZATION.md#2a2-what-the-diagram-is-telling-you).

For a cell whose value is `ExtendedValue::RichValue(_)`, render the rich
value's type name + first-two-key preview (gates on
`SEAM-ONECALC-EXTENDED-VALUE-ROUTING`).

### 6.3 Mixed-type cells within one array

Each cell dispatches independently per HON-004. No coercion. A column may
hold numbers and `#N/A`s mixed; each cell renders per its own type.

---

## 7. Truncation and honesty

### 7.1 Truncation states

The preview is in one of:

| State | Visible | Indicator |
|---|---|---|
| `WithinPreview` | All cells render | none |
| `TruncatedWindow` | Subset; bridge sent ≥ visible window | `… +N more rows · +M more cols` chip |
| `TruncatedFetch` | Subset; bridge sent partial preview, full-fetch needed | chip + "fetch full" link |

### 7.2 Counts shown

- Top-of-grid header: `Array[R × C] · showing {visible_r} × {visible_c} of {R} × {C}`
- Truncation chip text: `… +{R - visible_r} more rows · +{C - visible_c} more cols`
- Inspector footer: `viewing tile R{tile_r0}–R{tile_r1} × C{tile_c0}–C{tile_c1}`

### 7.3 Bridge-fetch failure

When a tile fetch fails:
- The tile area renders a `BLOCKED` overlay with terracotta hatching.
- Tooltip: `Failed to fetch tile (rows {r0}–{r1}, cols {c0}–{c1}): {reason}`.
- SEAM `SEAM-OXFML-BRIDGE-TILE-FETCH-FAILURE` shown in the overlay.

### 7.4 Honesty contract (regression-prone)

Three things **must always** be true and are explicit invariants:

1. The total shape is visible. Tested by T-HON-001 (every array context
   asserts presence of `[R × C]` text).
2. The truncation indicator is visible whenever a preview window is shown.
   Tested by T-HON-002.
3. Errors are not replaced by empty. Tested by T-HON-003 with a fixture
   array containing `#DIV/0!`.

Removing any of these without simultaneously removing the test breaks the
build.

---

## 8. Virtualization

### 8.1 Tile model

The expanded preview is a virtual grid of 50×50 tiles. For an array of
`R × C`, there are `ceil(R/50) × ceil(C/50)` tiles.

```rust
struct Tile {
    tile_row: usize,    // tile coord, not cell coord
    tile_col: usize,
    state: TileState,
}

enum TileState {
    Unfetched,
    Fetching,
    Fetched(TileCells),
    FetchFailed(String),
}

struct TileCells {
    rows: Vec<Vec<ArrayPreviewCell>>,    // up to 50 × 50
}
```

### 8.2 IntersectionObserver

A sentinel `<div>` per tile, sized to `50 × cell_height` × `50 × cell_width`.
The IntersectionObserver fires when a tile's sentinel enters or leaves the
viewport ± 1 tile margin.

- **Enter:** if `Unfetched`, dispatch `fetch_array_tile(...)`. State →
  `Fetching`. On response, state → `Fetched`; cells render.
- **Leave (with margin):** state stays — fetched tiles keep their data.
  DOM nodes for rendered cells are kept only for the visible + 1-tile-margin
  set; tiles outside that margin have their cell DOM nodes removed but
  cell data is retained in state for fast re-render on scroll-back.

### 8.3 Memory budget

- **State (cell data):** ~50 cells × 50 cells × 80 bytes = 200 KB per tile.
  100 tiles fetched = 20 MB. Budget: cap fetched tiles at 200 (40 MB);
  beyond that, evict oldest (LRU) per NFR-MEM-006.
- **DOM:** only viewport ± 1 tile of cells are DOM-rendered. With
  ~16 cells visible on screen for a typical inspector window, peak DOM
  cells = ~3 × 16 + buffer ≈ 100 cells. Well under NFR-MEM-006.

### 8.4 Sticky headers

In Inspector and Expanded modes:

- Top row (column letters): `position: sticky; top: 0` within the scroll
  container.
- Left column (row numbers): `position: sticky; left: 0`.
- Top-left corner cell: both sticky, z-index above row.

Sticky positioning is a CSS feature, not virtualization; it works against
both the rendered DOM and the virtual scroll height.

---

## 9. Compare view array handling

### 9.1 Layout

Two-column compare body. When both sides are arrays, each side renders
`array_preview.rs` in `CompareCell` mode. The two columns share a parent
that:

- Reserves equal width per column.
- Owns the synchronized scroll signal.
- Renders the mismatch summary above the columns.

### 9.2 Synchronized scroll

```rust
let scroll_offset: RwSignal<(usize /*row*/, usize /*col*/)> = RwSignal::new((0, 0));
// Each <ArrayPreview mode=CompareCell scroll_signal=scroll_offset />
// reads scroll_offset for placement and writes to it on user scroll.
```

The two previews are scrolled by the same signal: scrolling either side
updates the signal, both sides re-position. Wheel/key events on either
preview are intercepted and update the shared signal.

### 9.3 Cell-level mismatch markers

For the cell at `(r, c)`:

```
left_cell  = left_array.get(r, c)
right_cell = right_array.get(r, c)
mismatch   = !cells_equivalent(left_cell, right_cell, equivalence_policy)
```

Where `cells_equivalent` consults `OxReplay`'s equivalence policy (the same
one used at the case level). When `mismatch == true`:
- Both cells render with a 2 px terracotta border.
- The cell carries `data-mismatch-id={id}` for cross-link to the mismatch
  list.

### 9.4 Mismatch summary

Above the two-column body:

```
12 of 600 cells differ; worst severity: semantic
[show only mismatching ▾]
```

Click `show only mismatching` → filter view shows only mismatch rows
(virtualization preserves shape addresses, so row numbers are preserved
even when filtered).

### 9.5 Shape disagreement

When `left.shape != right.shape`:

- Both render at their actual size.
- Out-of-shape cells in the smaller side render with a hatched background
  and `<empty>` text.
- Mismatch summary header includes:
  `shapes disagree: left {Rl×Cl}, right {Rr×Cr}`.
- Per HON-005, no silent extension.

---

## 10. Copy semantics

### 10.1 Copy actions

| Action | Trigger | Scope | Format |
|---|---|---|---|
| Copy visible | `Ctrl+C` no selection | Currently rendered cells | TSV (default) + HTML + JSON multi-MIME |
| Copy selection | `Ctrl+C` with selected cells | Selected cells | TSV + HTML + JSON |
| Copy all (small) | `Ctrl+Shift+C`, `total ≤ 10 000` | Whole array | TSV + HTML + JSON |
| Copy all (large) | `Ctrl+Shift+C`, `total > 10 000` | Whole array | After confirm popover |
| Copy as CSV | Right-click → menu | per scope above | CSV |
| Copy as JSON | Right-click → menu | per scope above | JSON `{ shape, cells }` |
| Copy as Markdown | Right-click → menu | per scope above | GFM table |

### 10.2 Confirm popover for large arrays

When user invokes "copy all" with `total > 10 000`:

```
┌─────────────────────────────────────┐
│ Copy 12 600 cells (~480 KB)?        │
│ [ Cancel ]   [ Copy ]               │
└─────────────────────────────────────┘
```

The estimate uses an upper bound on cell repr length (e.g. 32 chars per
cell × 12 600 cells × 1.2 overhead ≈ 480 KB).

### 10.3 ClipboardItem multi-MIME

Per AD-8, the clipboard write uses `ClipboardItem` with three MIME types:

```js
const item = new ClipboardItem({
  'text/plain': new Blob([tsv], { type: 'text/plain' }),
  'text/html':  new Blob([html_table], { type: 'text/html' }),
  'application/json': new Blob([json_repr], { type: 'application/json' }),
});
navigator.clipboard.write([item]);
```

Excel pastes from `text/html` (preserves formatting like color); plain
editors paste from `text/plain` (TSV); programmatic consumers can read
JSON.

---

## 11. Bridge and state contract

### 11.1 Bridge mirror types (new)

```rust
/// Replaces the existing FormulaArrayPreview's stringified rows with
/// typed-cell content.
pub struct ArrayPreviewBundle {
    pub label: String,                          // e.g. "Array[1000 × 50]"
    pub total_shape: (usize, usize),            // (rows, cols)
    pub preview_window: ArrayWindow,            // what's shipped in this response
    pub cells: Vec<Vec<ArrayPreviewCell>>,      // preview_window.rows × preview_window.cols
    pub truncated: bool,                        // true iff preview_window != total_shape
}

pub struct ArrayWindow {
    pub row_start: usize,
    pub row_end: usize,    // exclusive
    pub col_start: usize,
    pub col_end: usize,    // exclusive
}

pub struct ArrayPreviewCell {
    pub kind: ArrayPreviewCellKind,
    pub display_repr: String,                   // ready-to-render string (already format-applied)
    pub raw_repr: String,                       // un-formatted (for copy)
    pub presentation_hint: Option<PresentationHint>,  // per-cell
    pub error_code: Option<WorksheetErrorCode>, // when kind == Error
}

pub enum ArrayPreviewCellKind {
    Number,
    Text,
    Logical,
    Error,
    Reference,
    Lambda,
    RichValue,
    Empty,
}
```

### 11.2 New bridge calls

```rust
pub trait OxfmlEditorBridge {
    // (existing) apply_formula_edit → returns EditorDocument
    //   EditorDocument.value_presentation now carries Option<ArrayPreviewBundle>
    //   for the default 50×50 preview window, replacing FormulaArrayPreview.

    /// Fetch an arbitrary tile of an array result. Used by Inspector mode.
    fn fetch_array_tile(
        &self,
        request: ArrayTileRequest,
    ) -> Result<ArrayTileResponse, OxfmlEditorBridgeError>;
}

pub struct ArrayTileRequest {
    pub formula_stable_id: String,
    pub green_tree_key: String,                 // pinned at request time for cache validity
    pub row_start: usize,
    pub row_end: usize,
    pub col_start: usize,
    pub col_end: usize,
}

pub struct ArrayTileResponse {
    pub cells: Vec<Vec<ArrayPreviewCell>>,
    pub green_tree_key: String,
}
```

When the formula's `green_tree_key` changes between request and response,
the response is **discarded** (the array changed underneath; the tile is
no longer valid).

### 11.3 New SEAMs

| SEAM id | Owner | What |
|---|---|---|
| `SEAM-BRIDGE-ARRAY-PREVIEW-TYPED-CELLS` | OneCalc | Replace `FormulaArrayPreview.rows: Vec<Vec<String>>` with `ArrayPreviewBundle` carrying typed cells. |
| `SEAM-BRIDGE-ARRAY-TILE-FETCH` | OneCalc | New `fetch_array_tile` bridge call. |
| `SEAM-OXFML-BRIDGE-TILE-FETCH-FAILURE` | OneCalc + bridge | Per HON-006: render failed-tile overlay with this id. |
| `SEAM-ONECALC-ARRAY-CELL-SUBEXPR-DRILL` | OneCalc + `OxFml` | Cell-as-subexpression drill (FR-CELL-DRILL-040). Wraps `SEAM-OXFML-PARTIAL-EVAL`. |

### 11.4 State extension

```rust
pub struct FormulaSpaceState {
    // ... existing fields ...
    pub array_preview: Option<ArrayPreviewState>,    // replaces today's FormulaArrayPreviewState
}

pub struct ArrayPreviewState {
    pub bundle: ArrayPreviewBundle,
    pub fetched_tiles: BTreeMap<(usize, usize), TileState>,
    pub last_tile_evict: Vec<(usize, usize)>,        // LRU for memory cap
    pub local_ui: ArrayPreviewLocalUi,               // selection, scroll offset
}

pub struct ArrayPreviewLocalUi {
    pub mode: ArrayPreviewMode,
    pub scroll_offset: (usize, usize),
    pub selection: Option<ArraySelection>,
    pub focused_cell: Option<(usize, usize)>,
    pub headers_visible: bool,
}
```

The local UI state is preserved across formula-space switch (within the
session) but reset on reload.

---

## 12. Performance budgets

| Path | Budget | Measured how |
|---|---|---|
| Bridge response (preview) → first paint | ≤ 80 ms | Browser perf timer |
| Click truncation chip → expanded mode first paint | ≤ 220 ms | Browser perf timer |
| Tile fetch round-trip | ≤ 80 ms p50, ≤ 200 ms p99 | Bridge timing |
| Scroll throughput (1k rows) | ≥ 60 fps | rAF-based test |
| Scroll throughput (100k cells, virtualized) | ≥ 60 fps | rAF-based test |
| DOM memory peak (10k cells in viewport) | ≤ 4 MB | Heap snapshot |
| Default bridge payload | ≤ 200 KB | Network sniffer in test |
| Tile bridge payload | ≤ 100 KB | Network sniffer |
| Copy-all (100k cells) | ≤ 800 ms wall | Test timer |

Budget violations fail the relevant test (T-PERF-*).

---

## 13. Test design

### 13.1 Test layers

| Layer | Where | What |
|---|---|---|
| **Unit** | in-tree `#[cfg(test)]` | Pure functions: cell renderer dispatch, tile coordinate math, copy formatting (TSV/CSV/JSON/HTML), truncation chip text |
| **Integration** | `tests/array_preview.rs` (new) | `array_preview.rs` view-model build with stub bundles |
| **Browser invariant** | `tests/browser/array_preview.rs` (new) | DOM-visible invariants in headless Chromium |
| **Browser perf** | `tests/browser/array_perf.rs` | Scroll fps, fetch budgets |
| **Visual regression** | `tests/browser/array_visual.rs` | Snapshot in each mode at typical sizes |
| **A11y** | `tests/browser/array_a11y.rs` | Sticky headers and selection are keyboard-accessible |

### 13.2 Browser invariant test catalogue

#### T-RENDER — basic rendering

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-RENDER-001 | Bridge returns `Array[2×2]` with 4 numbers | render hero | 4 cells rendered; header shows `Array[2 × 2]` |
| T-RENDER-002 | Bridge returns `Array[10×10]` | render hero | 6×6 visible; truncation chip says `… +4 more rows · +4 more cols` |
| T-RENDER-003 | Bridge returns `Array[1×3]` | render hero | 1 row × 3 cols rendered; spill outline visible |
| T-RENDER-004 | Bridge returns `Array[6×6]` (exactly) | render hero | full grid; no truncation chip |

#### T-CELL — per-cell rendering

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-CELL-005 | Cell value `Number(123.45)` | render | right-aligned; locale-formatted |
| T-CELL-006 | Cell value `Text("hello world long...")` (40 chars) | render | truncated at 32 chars + `…`; native tooltip has full text |
| T-CELL-007 | Cell value `Logical(true)` | render | `TRUE` pill, sage |
| T-CELL-008 | Cell value `Error(Div0)` | render | `#DIV/0!` terracotta; cell background terracotta-soft |
| T-CELL-009 | Mixed cells: Number, Error, Text in same array | render | each cell's type rendering applied independently |

#### T-SCROLL — scroll behavior

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-SCROLL-010 | Hero with 6×6 visible | wheel inside preview | preview scrolls; page does NOT scroll |
| T-SCROLL-011 | Expanded with 1000 rows | scroll to row 500 | only viewport ± 1 tile rendered (DOM cells ≤ 200) |
| T-SCROLL-012 | Expanded with 1000 rows | rapid scroll | 60 fps maintained (rAF-based assertion) |

#### T-VIRT — virtualization

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-VIRT-013 | Inspector with 100×100 array | scroll to (50, 50) | tile (1, 1) fetch dispatched (assert via spy) |
| T-VIRT-014 | Tile fetched | scroll away then back | tile re-renders without re-fetch |
| T-VIRT-015 | 200 tiles fetched | fetch tile #201 | LRU eviction: oldest tile data dropped |

#### T-TRUNC — truncation indicators

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-TRUNC-016 | Hero `Array[10×10]` | DOM check | truncation chip text matches `… +4 more rows · +4 more cols` |
| T-TRUNC-017 | Hero `Array[6×7]` (cols only truncated) | DOM check | chip text matches `… +1 more cols` only |
| T-TRUNC-018 | Click truncation chip | mode change | hero grows to expanded; virtualization active |

#### T-FULL — full-fetch and inspector

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-FULL-019 | Hero with 100×100 array | click "Open in inspector" | inspector overlay visible; URL updated |
| T-FULL-020 | Inspector | click "← back" | hero visible; scroll preserved |
| T-FULL-021 | Inspector | click "Fetch all" with > 10k cells | confirm popover appears |

#### T-DRILL — drill leaf preview

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-DRILL-022 | Walk node with array value | render row | `Array[R×C]` text shown; no mini-grid |
| T-DRILL-023 | Walk node with array value | hover row | mini-grid (3×3) appears next to row |
| T-DRILL-024 | Click mini-grid | mode change | inspector opens for that node's array |

#### T-COMPARE — compare-view arrays

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-COMPARE-025 | Compare with two `Array[6×3]` (one differing cell) | render | both arrays rendered side-by-side; one cell has terracotta border |
| T-COMPARE-026 | T-COMPARE-025 | scroll left side | right side scrolls in lockstep (same row offset) |
| T-COMPARE-027 | Compare with `Array[6×3]` vs `Array[6×4]` | render | both at actual size; right side col 4 cells have hatch background |
| T-COMPARE-028 | Click mismatched cell | focus change | mismatch list scrolls to matching record; record is highlighted |
| T-COMPARE-029 | Click "show only mismatching" | filter mode | only mismatch rows shown; row numbers preserved |

#### T-COPY — copy semantics

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-COPY-030 | Hero with 6×6 visible | Ctrl+C no selection | clipboard contains 6 lines TSV |
| T-COPY-031 | Hero, drag-select 2×2 | Ctrl+C | clipboard contains 2 lines TSV with 2 cols |
| T-COPY-032 | Hero `Array[100×100]` | Ctrl+Shift+C | confirm popover appears |
| T-COPY-033 | Confirm copy-all | check clipboard | TSV with 100 lines × 100 cols |
| T-COPY-034 | Right-click → Copy as JSON | check clipboard | JSON with `{ shape: [R,C], cells: [...] }` |
| T-COPY-035 | Paste into Excel | end-to-end (manual or fixture) | formatting preserved (HTML mime path) |

#### T-CELL-DRILL — cell drill-through

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-CELL-DRILL-036 | Hero | click cell (3, 2) | cell focused; subline `R3 C2: <value>` |
| T-CELL-DRILL-037 | Hero | right-click cell | menu appears with "Evaluate cell as subexpression" disabled (SEAM tooltip) |
| T-CELL-DRILL-038 | When SEAM lands | invoke menu item | bridge call dispatched with `(formula, 3, 2)` |

#### T-HON — honesty

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-HON-039 | Any array context (hero, drill, compare, inspector) | render | total shape `[R × C]` text present |
| T-HON-040 | Any windowed render | render | truncation indicator present |
| T-HON-041 | Array with `#DIV/0!` cells | render | cells render error styling, not empty |
| T-HON-042 | Compare with shape disagreement | render | both shapes shown in summary header |
| T-HON-043 | Tile fetch fails | mock failure | tile area shows BLOCKED overlay with SEAM id |

#### T-PERF — performance

| ID | Setup | Action | Assertion |
|---|---|---|---|
| T-PERF-044 | Hero with `Array[6×6]` | mount + bridge response | first paint ≤ 80 ms |
| T-PERF-045 | Click truncation chip on `Array[1000×100]` | expand | first paint ≤ 220 ms |
| T-PERF-046 | Inspector with 100×100, scrolling | rAF measure 1 s | ≥ 60 fps |
| T-PERF-047 | Tile fetch | mock 10 ms server | round-trip ≤ 80 ms p50 |
| T-PERF-048 | Bridge payload (default preview, 1000×50) | network sniffer | ≤ 200 KB |

### 13.3 Traceability matrix (FR/NFR/HON → Tests)

| Requirement | Tests |
|---|---|
| FR-RENDER-001..008 | T-RENDER-001..004 |
| FR-CELL-009..016 | T-CELL-005..009 |
| FR-SCROLL-017..020 | T-SCROLL-010 |
| FR-VIRT-021..024 | T-VIRT-013..015, T-SCROLL-011..012 |
| FR-FULL-025..028 | T-TRUNC-018, T-FULL-019..021 |
| FR-DRILL-029..031 | T-DRILL-022..024 |
| FR-COMPARE-032..037 | T-COMPARE-025..029 |
| FR-CELL-DRILL-038..040 | T-CELL-DRILL-036..038 |
| FR-COPY-041..046 | T-COPY-030..035 |
| NFR-PERF-001..005 | T-PERF-044..047 |
| NFR-MEM-006..008 | T-VIRT-015 + heap snapshot in T-PERF-046 |
| NFR-PAYLOAD-009..010 | T-PERF-048 + tile sniffer |
| HON-001..006 | T-HON-039..043 |

Every requirement closes through at least one test.

---

## 14. Implementation phases

| # | Phase name | Scope (FR / NFR / HON) | Exit gate (test IDs) | Why this order |
|---|---|---|---|---|
| **P1** | Bridge typed-cell preview | `SEAM-BRIDGE-ARRAY-PREVIEW-TYPED-CELLS` (the new bundle), AD-2, AD-3 | unit + T-RENDER-001..004 | Foundation: without typed cells, per-cell renderer can't dispatch. |
| **P2** | Hero mode (≤ 6×6, no virtualization) | FR-RENDER-001..008, FR-CELL-009..016 (basic), HON-001..004 | T-RENDER-001..004, T-CELL-005..009, T-HON-039..041 | Smallest, fastest, validates renderer dispatch. |
| **P3** | Truncation indicator + click-to-expand | FR-RENDER-002, FR-RENDER-004, FR-FULL-025..026 | T-TRUNC-016..018 | Honesty before scale. |
| **P4** | Internal scroll + non-virtualized expanded | FR-SCROLL-017..018, FR-FULL-026 | T-SCROLL-010 | Local scroll plumbing without page hijack. |
| **P5** | Virtualization (tile model) | FR-VIRT-021..024, AD-4, AD-5, NFR-MEM-006 | T-VIRT-013..015, T-SCROLL-011..012 | The hard part; tile-based with IntersectionObserver. |
| **P6** | Inspector overlay | FR-FULL-027..028, AD-9 | T-FULL-019..021 | Reuses virtualized renderer; new shell. |
| **P7** | DrillLeaf mode | FR-DRILL-029..031 | T-DRILL-022..024 | Reuses cell renderer; tiny mode. |
| **P8** | CompareCell mode + synchronized scroll | FR-COMPARE-032..037, AD-6, HON-005 | T-COMPARE-025..029, T-HON-042 | Compare-view dependency; needs P5 first. |
| **P9** | Copy semantics | FR-COPY-041..046, AD-8 | T-COPY-030..035 | Power-user; doesn't block compare or other work. |
| **P10** | Cell selection + drill-through (UI without SEAM) | FR-CELL-DRILL-038..039 | T-CELL-DRILL-036..037 | Right-click menu + selection; partial-eval menu item disabled. |
| **P11** | Tile fetch failure honesty | HON-006, `SEAM-OXFML-BRIDGE-TILE-FETCH-FAILURE` | T-HON-043 | Tested with mocked failure. |
| **P12** | Performance + a11y polish | NFR-PERF-001..005, NFR-PAYLOAD-009..010, sticky headers a11y | T-PERF-044..048 | Final pass; budget enforcement. |
| **P13** | (later) Cell-as-subexpression drill-through | FR-CELL-DRILL-040 | T-CELL-DRILL-038 | Gates on `SEAM-OXFML-PARTIAL-EVAL`; keep menu item disabled until lit. |

**Parallel-safe:** P7 (DrillLeaf) and P9 (Copy) can run in parallel with
P5 (Virtualization). P11 can run alongside any later phase. P13 is post-WS-14.

---

## 15. Risk register

| ID | Risk | Probability | Impact | Mitigation |
|---|---|---|---|---|
| R-1 | Virtualization tile boundaries flicker on scroll. | medium | medium | Sentinel margin = 1 tile; test scroll-fps invariants. |
| R-2 | IntersectionObserver fires too often (every pixel). | low | medium | Use `threshold: 0` and `rootMargin` to coalesce events. |
| R-3 | Tile fetch rate-limited by upstream `OxFml`. | medium | medium | Coalesce concurrent requests for adjacent tiles into a single batched call (future bridge optimization; today, serial is fine). |
| R-4 | Sticky headers misalign on Safari. | medium | low | Visual regression in BC matrix; manual verify on Safari for the inspector. |
| R-5 | Synchronized scroll feedback loop (each side fires events that update the other ad infinitum). | medium | high | Scroll signal write debounced; only the side that initiated the scroll writes; other side reads only. |
| R-6 | Mismatch detection on huge arrays slow. | medium | medium | Compute lazily per visible tile (don't pre-compute all 100k mismatches). |
| R-7 | Copy-large operations block the main thread. | low | high | Build TSV/HTML/JSON in chunks via `requestIdleCallback`; show progress. |
| R-8 | Tile cache memory bloats over long sessions. | medium | medium | LRU eviction at 200 fetched tiles; on formula-space switch, drop all. |
| R-9 | Per-cell `PresentationHint` not always set; fallback ambiguous. | medium | low | Falling back to scenario-context format code is the documented behavior (see [§6.2](#62-extendedvalue-per-cell)); test fixtures cover both. |
| R-10 | Excel paste of HTML clipboard format breaks on certain locales. | low | low | Strip locale-specific formatting from HTML; pure values + minimal style. |
| R-11 | Naïve scroll restore after re-render misaligns (jitter). | medium | low | Save scroll offset in `local_ui`; restore explicitly after re-render. |
| R-12 | Heat-map / CF on cells not landing during WS-14 confuses users. | low | low | Out-of-scope per [§3.4](#34-out-of-scope); the cascade in result drill makes scenario-level CF visible, not per-cell. |

---

## 16. Open questions

1. **Should the hero default mode show 6×6 or auto-fit?** A 1000-row array
   with 6×6 visible feels small. Alternatives: auto-fit to available
   vertical room (up to ~12 rows). Recommendation: stay 6×6 to keep the
   home screen consistent across array shapes; the truncation chip plus
   one click takes the user to expanded mode.
2. **Cell drill-through gating.** Should the menu item appear disabled
   (with SEAM tooltip) or be entirely hidden until the SEAM lands?
   Recommendation: disabled with tooltip. Discoverability beats noise.
3. **Compare-view shape disagreement: should rows count from 0 or shift?**
   When `left.shape == (6, 3)` and `right.shape == (8, 3)`, do row numbers
   align (rows 1..6 on both, rows 7..8 only on right) or do the empty rows
   appear top of left? Recommendation: align from row 1; empty rows visible
   on the larger side at the bottom.
4. **Sort/filter inside compare's mismatch view.** "Show only mismatching"
   is the only filter we plan. Should there be a "mismatched-rows-only"
   *sort* (move mismatching rows to the top)? Recommendation: no — preserve
   row addresses for trust; filter is enough.
5. **Inspector route.** A separate URL? An overlay only? Recommendation:
   overlay with a routed identifier so the back button + reload work,
   parallel to compare-view.
6. **Copy as Excel-native (BIFF / XLSX) format?** The HTML clipboard format
   already pastes into Excel cleanly, but a true XLSX clipboard would
   carry types. Defer until an actual user need surfaces.
7. **Per-cell partial-eval cost.** When `SEAM-OXFML-PARTIAL-EVAL` lands,
   evaluating a single cell of a `FILTER` result requires re-running
   FILTER's predicate over the source range. Cost may be similar to the
   whole-formula eval; UX should set the expectation. Recommendation: show
   a small "evaluating…" indicator on the cell during partial-eval.

---

## Appendix A — Reviewer's 60-second checklist

1. **Every requirement in [§3](#3-requirements) has a test ID** in
   the traceability matrix [§13.3](#133-traceability-matrix-frnfrhon--tests).
   No row is empty.
2. **Every honesty requirement [§3.3](#33-honesty-requirements-hon-) has
   an explicit test in T-HON-***.
3. **The four large-array failure modes are explicitly tested:** payload
   bloat (T-PERF-048), DOM bloat (T-VIRT-015), silent truncation
   (T-HON-040), shape disagreement hiding (T-HON-042).
4. **Bridge contract changes are SEAM'd**, not silently introduced.

If those four pass, the array result surface is safe to ship.

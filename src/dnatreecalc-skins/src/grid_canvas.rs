//! GRID CANVAS — the shared windowed-grid surface.
//!
//! This is the SheetLens grid machinery promoted into a shared component
//! (route-map §C.2, §E.4 K1a/K1b): a scroll container + virtual canvas sized
//! from `max_rows`/`max_cols`, absolutely-positioned windowed cells, and a
//! read-only overlay layer (tables, merged regions, spills). Only the
//! interest-window cells exist in the DOM; scrollbars are sized by bounds —
//! windowing *is* the virtualization, no virtual-list library.
//!
//! K1a was a **pure extraction** of the pre-extraction `grid_surface`
//! (`sheet.rs:907–1054`) and its two pure helpers, moved unchanged. K1b
//! (§C.2's two mandatory upgrades) adds, on top of that extraction:
//!
//! 1. **Interest coalescing** — [`GridInterestCoalescer`], a pure scheduler
//!    that collapses any number of scroll-driven window recomputations
//!    within one animation frame into a single pending window, flushed by
//!    one `requestAnimationFrame` callback (so a scroll storm dispatches at
//!    most one `SetGridInterest` per frame instead of one per scroll event).
//! 2. **Authored-aware cell rendering** — [`render_grid_cell`] renders the
//!    classifier's read-only affordance when `authored.editability !=
//!    Editable`, and (in show-formulas mode) `authored.source_text` in place
//!    of the computed value.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, GridAuthoredCellProjection, GridCellProjection, GridEditabilityProjection,
    GridOverlayRect, NodeId, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use leptos::wasm_bindgen::{self, JsCast};

use crate::value_render::render_value;

/// A pure per-frame coalescing scheduler for `SetGridInterest` windows
/// (route-map §C.2 K1b: "scroll events debounce/coalesce into one
/// `SetGridInterest` per animation frame"). Any number of `note` calls
/// between two `drain` calls collapse to the single most recent window; a
/// `drain` with nothing pending returns `None` so the caller dispatches
/// nothing. No DOM/RAF dependency — the frame boundary is owned by the
/// caller (one `drain` per `requestAnimationFrame` tick in the live
/// component), so this is unit-testable as a pure function of calls.
#[derive(Debug, Default)]
pub struct GridInterestCoalescer {
    pending: Option<(u32, u32, u32, u32)>,
}

impl GridInterestCoalescer {
    #[must_use]
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Record a newly computed interest window, overwriting any window
    /// already pending for this frame (the whole point of coalescing: only
    /// the latest scroll position matters, not the intermediate ones).
    pub fn note(&mut self, window: (u32, u32, u32, u32)) {
        self.pending = Some(window);
    }

    /// Take the pending window, if any, clearing it. Call once per frame
    /// boundary; returns `None` when no `note` happened since the last
    /// `drain` (so the frame dispatches nothing).
    pub fn drain(&mut self) -> Option<(u32, u32, u32, u32)> {
        self.pending.take()
    }
}

/// Fixed grid cell metrics for the windowed grid surface. Per-cell sizing and
/// text layout are Phase 5; the read path renders a uniform cell box.
pub const GRID_ROW_HEIGHT_PX: f64 = 22.0;
pub const GRID_COL_WIDTH_PX: f64 = 84.0;
/// Cap the virtual scroll canvas so a full Excel-extent sheet does not blow past
/// the browser's max element height; the window math still clamps to real bounds.
pub const GRID_VIRTUAL_CELL_CAP: u32 = 100_000;

/// The 1-based inclusive interest window a scroll offset exposes over a uniform
/// grid, clamped to the grid bounds with a one-cell trailing overscan. Pure, so
/// the scroll-to-window mapping is unit-testable without a DOM.
pub fn grid_interest_window(
    scroll_top: f64,
    scroll_left: f64,
    viewport_height: f64,
    viewport_width: f64,
    max_rows: u32,
    max_cols: u32,
) -> (u32, u32, u32, u32) {
    let axis = |offset: f64, viewport: f64, cell: f64, max: u32| -> (u32, u32) {
        let max = max.max(1);
        let first = (offset / cell).floor().max(0.0) as u32 + 1;
        let visible = (viewport / cell).ceil() as u32 + 1;
        let top = first.min(max);
        let bottom = top.saturating_add(visible).min(max).max(top);
        (top, bottom)
    };
    let (top_row, bottom_row) = axis(scroll_top, viewport_height, GRID_ROW_HEIGHT_PX, max_rows);
    let (left_col, right_col) = axis(scroll_left, viewport_width, GRID_COL_WIDTH_PX, max_cols);
    (top_row, left_col, bottom_row, right_col)
}

/// Absolute-position style for an overlay box over a window-clipped rect, in the
/// same coordinate space as the cells. Each edge is drawn `solid` when the rect
/// ends within the window and `clipped` (dashed) when the window cut it, so a
/// clipped border reads as "continues beyond the window".
fn overlay_box_style(rect: &GridOverlayRect, solid: &str, clipped: &str) -> String {
    let top = f64::from(rect.top_row.saturating_sub(1)) * GRID_ROW_HEIGHT_PX;
    let left = f64::from(rect.left_col.saturating_sub(1)) * GRID_COL_WIDTH_PX;
    let width = f64::from(rect.right_col.saturating_sub(rect.left_col) + 1) * GRID_COL_WIDTH_PX;
    let height = f64::from(rect.bottom_row.saturating_sub(rect.top_row) + 1) * GRID_ROW_HEIGHT_PX;
    let border = |is_clipped: bool| if is_clipped { clipped } else { solid };
    format!(
        "position:absolute;top:{top}px;left:{left}px;width:{width}px;height:{height}px;\
         box-sizing:border-box;pointer-events:none;\
         border-top:{};border-left:{};border-bottom:{};border-right:{};",
        border(rect.clipped_top),
        border(rect.clipped_left),
        border(rect.clipped_bottom),
        border(rect.clipped_right),
    )
}

/// The read-only affordance class for a non-`Editable` classification (route-map
/// §C.3's affordance table), or `None` for `Editable` (which renders as a normal
/// cell — no affordance class, no title hint).
fn editability_affordance_class(editability: &GridEditabilityProjection) -> Option<&'static str> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::SpillDisplay { .. } => Some("dtc-grid__cell--spill-display"),
        GridEditabilityProjection::RepeatedRegionMember { .. } => {
            Some("dtc-grid__cell--repeated-member")
        }
        GridEditabilityProjection::MergedFollower { .. } => Some("dtc-grid__cell--merged-follower"),
        GridEditabilityProjection::TableStructural { .. } => {
            Some("dtc-grid__cell--table-structural")
        }
    }
}

/// The `title` hint for a non-`Editable` classification (§C.3 "On edit attempt"
/// column, surfaced here as a hover affordance since K1b ships no edit loop
/// yet — K2 adds the click-time flash/jump behavior on top of this same
/// classification).
fn editability_hint(editability: &GridEditabilityProjection) -> Option<String> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::SpillDisplay { anchor } => Some(format!(
            "Spilled from {} — edit the anchor",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::RepeatedRegionMember { anchor } => Some(format!(
            "Part of a filled region — anchor {}",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::MergedFollower { anchor } => Some(format!(
            "Part of a merged cell — anchor {}",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::TableStructural { table_id } => Some(format!(
            "Table header — rename via the table's header row ({table_id})"
        )),
    }
}

/// A minimal `A1`-style label for a projected `(row, col)` ref, used only for
/// the hover-hint text (never for engine addressing — the projection never
/// carries engine addresses, §A.2).
fn grid_cell_ref_label(row: u32, col: u32) -> String {
    let mut col_label = String::new();
    let mut n = col;
    while n > 0 {
        let rem = (n - 1) % 26;
        col_label.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    format!("{col_label}{row}")
}

/// Render one windowed grid cell, authored-aware (route-map §C.2 K1b): a
/// plain `Editable` cell renders its computed value (or, in show-formulas
/// mode, its `authored.source_text`); a non-`Editable` cell additionally
/// carries the classifier's read-only affordance class + hover hint so K2's
/// edit loop has a rendering contract to key off of. Cells with no authored
/// projection at all (pre-H3 mirror, or a projection that never carried
/// authored fields) render exactly as K1a did — value-only, no affordance —
/// so the upgrade is purely additive.
fn render_grid_cell(cell: &GridCellProjection, show_formulas: bool) -> AnyView {
    let top = f64::from(cell.row.saturating_sub(1)) * GRID_ROW_HEIGHT_PX;
    let left = f64::from(cell.col.saturating_sub(1)) * GRID_COL_WIDTH_PX;
    let style = format!(
        "position:absolute;top:{top}px;left:{left}px;width:{GRID_COL_WIDTH_PX}px;height:{GRID_ROW_HEIGHT_PX}px;"
    );

    let authored: Option<&GridAuthoredCellProjection> = cell.authored.as_ref();
    let affordance_class = authored.and_then(|a| editability_affordance_class(&a.editability));
    let hint = authored.and_then(|a| editability_hint(&a.editability));
    let class = match affordance_class {
        Some(affordance) => format!("dtc-grid__cell {affordance}"),
        None => "dtc-grid__cell".to_string(),
    };

    // Show-formulas mode renders the authored source text in place of the
    // computed value (route-map §C.2 K1b) whenever the cell has one; a
    // literal or empty cell has no `source_text` and still renders its value
    // (there is no "formula" to show).
    let body = if show_formulas {
        match authored.and_then(|a| a.source_text.as_ref()) {
            Some(source_text) => {
                view! { <div class="dtc-value-display dtc-value-display--formula">{source_text.clone()}</div> }
                    .into_any()
            }
            None => render_value(&cell.value),
        }
    } else {
        render_value(&cell.value)
    };

    match hint {
        Some(hint) => view! { <div class=class style=style title=hint>{body}</div> }.into_any(),
        None => view! { <div class=class style=style>{body}</div> }.into_any(),
    }
}

/// A windowed grid surface for a grid-backed sheet node. The scroll container and
/// virtual canvas are built once (stable across projection updates); only the
/// positioned cells re-render reactively, so a `SetGridInterest` re-scope swaps
/// the windowed cells without resetting the scroll position. Scrolling dispatches
/// [`WorkspaceIntent::SetGridInterest`] so OxCalc re-scopes the projection to the
/// newly visible window ("viewing is subscribing").
pub fn grid_surface(
    grid_id: NodeId,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    show_formulas: Signal<bool>,
) -> AnyView {
    // The sheet extent is fixed, so the canvas size is read once (untracked); the
    // scroll handler clamps windows to it.
    let (max_rows, max_cols) = workspace
        .get_untracked()
        .grids
        .get(&grid_id)
        .map_or((1, 1), |grid| (grid.max_rows, grid.max_cols));
    let canvas_height = f64::from(max_rows.min(GRID_VIRTUAL_CELL_CAP)) * GRID_ROW_HEIGHT_PX;
    let canvas_width = f64::from(max_cols.min(GRID_VIRTUAL_CELL_CAP)) * GRID_COL_WIDTH_PX;

    // Reactive: only the windowed cells re-render when the projection changes; the
    // surrounding scroll box is created once, so scrollTop survives a re-scope.
    let cells_id = grid_id.clone();
    let cells = move || {
        let ws = workspace.get();
        let Some(grid) = ws.grids.get(&cells_id) else {
            return Vec::new();
        };
        let show = show_formulas.get();
        grid.cells
            .iter()
            .map(|cell| render_grid_cell(cell, show))
            .collect::<Vec<_>>()
    };

    // Reactive read-only overlay layer (tables, merged regions, spills) drawn over
    // the cells in the same coordinate space. Each box is `pointer-events:none` so
    // it never intercepts cell interaction; a dashed edge marks a window-clipped
    // boundary ("continues beyond the window").
    let overlays_id = grid_id.clone();
    let overlays = move || {
        let ws = workspace.get();
        let Some(grid) = ws.grids.get(&overlays_id) else {
            return Vec::new();
        };
        let mut boxes: Vec<AnyView> = Vec::new();
        for table in &grid.overlays.tables {
            if let Some(header) = &table.header_rect {
                let style = format!(
                    "{}background:rgba(59,125,216,0.12);",
                    overlay_box_style(header, "1px solid #3b7dd8", "1px dashed #3b7dd8")
                );
                boxes.push(
                    view! {
                        <div class="dtc-grid__overlay dtc-grid__overlay--table-header" style=style></div>
                    }
                    .into_any(),
                );
            }
            let style = overlay_box_style(
                &table.table_range,
                "2px solid #3b7dd8",
                "1px dashed #3b7dd8",
            );
            let label = table.table_name.clone();
            boxes.push(
                view! {
                    <div class="dtc-grid__overlay dtc-grid__overlay--table" style=style title=label></div>
                }
                .into_any(),
            );
        }
        for region in &grid.overlays.merged {
            let style = overlay_box_style(&region.rect, "1px solid #9a6b2f", "1px dashed #9a6b2f");
            boxes.push(
                view! { <div class="dtc-grid__overlay dtc-grid__overlay--merged" style=style></div> }
                    .into_any(),
            );
        }
        for spill in &grid.overlays.spills {
            let (style, class) = if spill.blocked {
                (
                    format!(
                        "{}background:rgba(192,57,43,0.10);",
                        overlay_box_style(&spill.extent, "2px solid #c0392b", "1px dashed #c0392b")
                    ),
                    "dtc-grid__overlay dtc-grid__overlay--spill-blocked",
                )
            } else {
                (
                    format!(
                        "{}background:rgba(58,138,58,0.08);",
                        overlay_box_style(
                            &spill.extent,
                            "1px dashed #3a8a3a",
                            "1px dashed #3a8a3a"
                        )
                    ),
                    "dtc-grid__overlay dtc-grid__overlay--spill",
                )
            };
            boxes.push(view! { <div class=class style=style></div> }.into_any());
        }
        boxes
    };

    // Interest coalescing (route-map §C.2 K1b): every scroll tick recomputes
    // the window and hands it to the coalescer, but only the *first* `note`
    // since the last flush schedules a `requestAnimationFrame` callback; any
    // further scroll ticks before that frame fires just overwrite the
    // pending window (`GridInterestCoalescer::note`). The RAF callback
    // drains the coalescer and dispatches at most once per frame, clearing
    // the "scheduled" flag so the next scroll tick schedules a fresh frame.
    let coalescer = Rc::new(RefCell::new(GridInterestCoalescer::new()));
    let frame_scheduled = Rc::new(RefCell::new(false));
    let scroll_id = grid_id.clone();
    let on_scroll = move |ev: leptos::ev::Event| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
            return;
        };
        let window = grid_interest_window(
            f64::from(element.scroll_top()),
            f64::from(element.scroll_left()),
            f64::from(element.client_height()),
            f64::from(element.client_width()),
            max_rows,
            max_cols,
        );
        coalescer.borrow_mut().note(window);

        if *frame_scheduled.borrow() {
            return;
        }
        *frame_scheduled.borrow_mut() = true;

        let coalescer = coalescer.clone();
        let frame_scheduled = frame_scheduled.clone();
        let dispatch = dispatch.clone();
        let scroll_id = scroll_id.clone();
        let flush: wasm_bindgen::closure::Closure<dyn FnMut()> =
            wasm_bindgen::closure::Closure::once(move || {
                *frame_scheduled.borrow_mut() = false;
                if let Some((top_row, left_col, bottom_row, right_col)) =
                    coalescer.borrow_mut().drain()
                {
                    dispatch.dispatch(WorkspaceIntent::SetGridInterest {
                        grid: scroll_id.clone(),
                        top_row,
                        left_col,
                        bottom_row,
                        right_col,
                    });
                }
            });
        let Some(window) = leptos::web_sys::window() else {
            return;
        };
        let _ = window.request_animation_frame(flush.as_ref().unchecked_ref());
        // The RAF callback fires at most once (`Closure::once`); leaking it
        // here is the standard wasm-bindgen pattern for a one-shot callback
        // whose owning closure must outlive the JS call that invokes it.
        flush.forget();
    };

    view! {
        <div class="dtc-grid" aria-label="Grid surface" on:scroll=on_scroll>
            <div
                class="dtc-grid__canvas"
                style=format!("position:relative;height:{canvas_height}px;width:{canvas_width}px;")
            >
                {cells}
                {overlays}
            </div>
        </div>
    }
    .into_any()
}

/// The `.dtc-grid` surface styles, extracted verbatim from `SHEET_CSS` so the
/// shared grid component owns its own paint. Consumers concatenate this into
/// their lens stylesheet.
pub const GRID_CANVAS_CSS: &str = r#"
.dtc-grid {
  position: relative; overflow: auto; height: 320px; margin: 8px 0;
  border: 1px solid var(--dtc-border, #ccc); background: var(--dtc-surface);
}
.dtc-grid__cell {
  box-sizing: border-box; padding: 2px 4px; overflow: hidden;
  white-space: nowrap; text-overflow: ellipsis;
  border-right: 1px solid var(--dtc-border, #eee);
  border-bottom: 1px solid var(--dtc-border, #eee);
  font-variant-numeric: tabular-nums;
}
.dtc-grid__cell--spill-display { background: rgba(58,138,58,0.06); cursor: default; }
.dtc-grid__cell--repeated-member { background: rgba(59,125,216,0.05); cursor: default; }
.dtc-grid__cell--merged-follower { cursor: default; }
.dtc-grid__cell--table-structural { background: rgba(59,125,216,0.10); font-weight: 600; cursor: default; }
.dtc-value-display--formula { font-family: var(--dtc-mono-font, monospace); }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{GridAuthoredKindProjection, GridCellRefProjection};

    #[test]
    fn grid_interest_window_maps_scroll_to_clamped_bounds() {
        // Top-left, a 220x420 viewport over uniform 22x84 cells: ~10 rows and
        // ~5 cols visible, plus a one-cell trailing overscan.
        assert_eq!(
            grid_interest_window(0.0, 0.0, 220.0, 420.0, 1000, 1000),
            (1, 1, 12, 7)
        );
        // Scrolling down one viewport advances the first row to 11.
        let (top, _, _, _) = grid_interest_window(220.0, 0.0, 220.0, 420.0, 1000, 1000);
        assert_eq!(top, 11);
        // The window clamps to a small grid's real bounds.
        assert_eq!(
            grid_interest_window(0.0, 0.0, 220.0, 420.0, 4, 3),
            (1, 1, 4, 3)
        );
    }

    // ---- Acceptance (1): coalescing unit test -----------------------------
    // N scroll events "in one frame" (i.e. N `note` calls before any `drain`)
    // must collapse to exactly one dispatch — asserted here as "one `drain`
    // returns the latest window, and only the latest".

    #[test]
    fn coalescer_collapses_n_notes_into_one_pending_window() {
        let mut coalescer = GridInterestCoalescer::new();
        // Nothing pending before any scroll tick.
        assert_eq!(coalescer.drain(), None);

        // A storm of scroll-driven window recomputations within one frame.
        for row in 1..=50u32 {
            coalescer.note((row, 1, row + 10, 7));
        }

        // Exactly one dispatch's worth of data: the LATEST window, not the
        // first or an average of the fifty.
        assert_eq!(coalescer.drain(), Some((50, 1, 60, 7)));
        // The frame's pending window is consumed — draining again (as the
        // next animation frame would, with no scroll in between) dispatches
        // nothing, proving this frame's storm produced exactly one dispatch.
        assert_eq!(coalescer.drain(), None);
    }

    #[test]
    fn coalescer_reports_a_fresh_window_after_each_drain() {
        let mut coalescer = GridInterestCoalescer::new();
        coalescer.note((1, 1, 10, 5));
        assert_eq!(coalescer.drain(), Some((1, 1, 10, 5)));

        // A later frame's scroll ticks are independent of the drained one.
        coalescer.note((11, 1, 20, 5));
        coalescer.note((12, 1, 21, 5));
        assert_eq!(coalescer.drain(), Some((12, 1, 21, 5)));
        assert_eq!(coalescer.drain(), None);
    }

    // ---- Acceptance (2): authored-aware affordance ------------------------

    fn anchor(row: u32, col: u32) -> GridCellRefProjection {
        GridCellRefProjection { row, col }
    }

    /// A cell whose authored classification is `SpillDisplay { anchor: B3 }`,
    /// showing a spilled value.
    fn spill_display_cell() -> GridCellProjection {
        GridCellProjection {
            row: 4,
            col: 2,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "30".to_string(),
                display: "30".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 4,
                col: 2,
                kind: GridAuthoredKindProjection::Empty,
                literal_text: None,
                source_text: None,
                editability: GridEditabilityProjection::SpillDisplay {
                    anchor: anchor(3, 2),
                },
            }),
            provenance: None,
        }
    }

    #[test]
    fn spill_display_cell_renders_read_only_affordance_from_projection() {
        let editability = GridEditabilityProjection::SpillDisplay {
            anchor: anchor(3, 2),
        };
        assert_eq!(
            editability_affordance_class(&editability),
            Some("dtc-grid__cell--spill-display")
        );
        assert_eq!(
            editability_hint(&editability).as_deref(),
            Some("Spilled from B3 — edit the anchor")
        );

        // And through the cell-render fn itself: the rendered markup carries
        // the read-only affordance class, the anchor hint, and still shows
        // the spilled value.
        let html = render_grid_cell(&spill_display_cell(), false).to_html();
        assert!(
            html.contains("dtc-grid__cell--spill-display"),
            "spill-display markup must carry the affordance class: {html}"
        );
        assert!(
            html.contains("Spilled from B3"),
            "spill-display markup must carry the anchor hint: {html}"
        );
        assert!(
            html.contains("30"),
            "spill-display markup still renders the spilled value: {html}"
        );

        // An Editable cell rendered by the same fn has neither class nor hint.
        let editable_html = render_grid_cell(&formula_cell("=A1*3"), false).to_html();
        assert!(
            !editable_html.contains("dtc-grid__cell--"),
            "an Editable cell must render no affordance modifier class: {editable_html}"
        );
        assert!(
            !editable_html.contains("title="),
            "an Editable cell must render no hover hint: {editable_html}"
        );
    }

    #[test]
    fn editable_cell_has_no_affordance_or_hint() {
        assert_eq!(
            editability_affordance_class(&GridEditabilityProjection::Editable),
            None
        );
        assert_eq!(editability_hint(&GridEditabilityProjection::Editable), None);
    }

    #[test]
    fn every_non_editable_variant_maps_to_a_distinct_affordance_class() {
        let repeated = GridEditabilityProjection::RepeatedRegionMember {
            anchor: anchor(1, 1),
        };
        let merged = GridEditabilityProjection::MergedFollower {
            anchor: anchor(1, 1),
        };
        let table = GridEditabilityProjection::TableStructural {
            table_id: "tbl:Scenarios".to_string(),
        };

        assert_eq!(
            editability_affordance_class(&repeated),
            Some("dtc-grid__cell--repeated-member")
        );
        assert_eq!(
            editability_affordance_class(&merged),
            Some("dtc-grid__cell--merged-follower")
        );
        assert_eq!(
            editability_affordance_class(&table),
            Some("dtc-grid__cell--table-structural")
        );
        assert!(editability_hint(&table).unwrap().contains("tbl:Scenarios"));
    }

    #[test]
    fn grid_cell_ref_label_formats_a1_style_addresses() {
        assert_eq!(grid_cell_ref_label(3, 2), "B3");
        assert_eq!(grid_cell_ref_label(1, 1), "A1");
        assert_eq!(grid_cell_ref_label(1, 27), "AA1");
    }

    // ---- Acceptance (3): show-formulas mode --------------------------------

    fn formula_cell(source_text: &str) -> GridCellProjection {
        GridCellProjection {
            row: 1,
            col: 2,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "3".to_string(),
                display: "3".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 2,
                kind: GridAuthoredKindProjection::Formula,
                literal_text: None,
                source_text: Some(source_text.to_string()),
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        }
    }

    #[test]
    fn show_formulas_mode_renders_source_text_instead_of_the_value() {
        let cell = formula_cell("=A1*3");

        // Mode off: the computed value renders, never the source text.
        let value_html = render_grid_cell(&cell, false).to_html();
        assert!(
            value_html.contains('3') && !value_html.contains("=A1*3"),
            "with show-formulas off the computed value must render: {value_html}"
        );

        // Mode on: the authored source text renders in place of the value.
        let formula_html = render_grid_cell(&cell, true).to_html();
        assert!(
            formula_html.contains("=A1*3"),
            "with show-formulas on the authored source text must render: {formula_html}"
        );
        assert!(
            formula_html.contains("dtc-value-display--formula"),
            "the source text renders in the formula-text style: {formula_html}"
        );
    }

    #[test]
    fn show_formulas_mode_keeps_the_value_for_a_literal_cell() {
        let cell = GridCellProjection {
            row: 1,
            col: 1,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "7".to_string(),
                display: "7".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 1,
                kind: GridAuthoredKindProjection::Literal,
                literal_text: Some("7".to_string()),
                source_text: None,
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        };
        assert!(cell.authored.as_ref().unwrap().source_text.is_none());

        // A literal has no formula to show — show-formulas mode still renders
        // its value (never a blank).
        let html = render_grid_cell(&cell, true).to_html();
        assert!(
            html.contains('7'),
            "a literal cell renders its value even in show-formulas mode: {html}"
        );
    }

    #[test]
    fn cell_without_authored_metadata_renders_value_only_as_before_k1b() {
        // Pre-H3 mirror payloads carry no authored fields; the K1b upgrades
        // must be purely additive for them.
        let cell = GridCellProjection {
            row: 2,
            col: 3,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "42".to_string(),
                display: "42".to_string(),
            },
            value_epoch: 1,
            authored: None,
            provenance: None,
        };
        let html = render_grid_cell(&cell, true).to_html();
        assert!(html.contains("42"), "value renders: {html}");
        assert!(
            !html.contains("dtc-grid__cell--") && !html.contains("title="),
            "no affordance or hint without authored metadata: {html}"
        );
    }
}

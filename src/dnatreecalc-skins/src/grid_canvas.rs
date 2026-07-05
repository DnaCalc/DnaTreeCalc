//! GRID CANVAS — the shared windowed-grid surface.
//!
//! This is the SheetLens grid machinery promoted, verbatim, into a shared
//! component (route-map §C.2, §E.4 K1a): a scroll container + virtual canvas
//! sized from `max_rows`/`max_cols`, absolutely-positioned windowed cells, and
//! a read-only overlay layer (tables, merged regions, spills). Only the
//! interest-window cells exist in the DOM; scrollbars are sized by bounds —
//! windowing *is* the virtualization, no virtual-list library.
//!
//! K1a is a **pure extraction** — this is the pre-extraction `grid_surface`
//! (`sheet.rs:907–1054`) and its two pure helpers, moved unchanged. Interest
//! coalescing and authored-aware cell rendering are the K1b upgrades and are
//! deliberately NOT here.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, GridOverlayRect, NodeId, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use crate::value_render::render_value;

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
        grid.cells
            .iter()
            .map(|cell| {
                let top = f64::from(cell.row.saturating_sub(1)) * GRID_ROW_HEIGHT_PX;
                let left = f64::from(cell.col.saturating_sub(1)) * GRID_COL_WIDTH_PX;
                let style = format!(
                    "position:absolute;top:{top}px;left:{left}px;width:{GRID_COL_WIDTH_PX}px;height:{GRID_ROW_HEIGHT_PX}px;"
                );
                let value = render_value(&cell.value);
                view! { <div class="dtc-grid__cell" style=style>{value}</div> }
            })
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

    // Each scroll tick re-scopes the window. Coalescing a scroll storm into the
    // latest window (like EditContentDeferred in the worker proxy) is the
    // documented refinement; the read path dispatches per event.
    let scroll_id = grid_id.clone();
    let on_scroll = move |ev: leptos::ev::Event| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
            return;
        };
        let (top_row, left_col, bottom_row, right_col) = grid_interest_window(
            f64::from(element.scroll_top()),
            f64::from(element.scroll_left()),
            f64::from(element.client_height()),
            f64::from(element.client_width()),
            max_rows,
            max_cols,
        );
        dispatch.dispatch(WorkspaceIntent::SetGridInterest {
            grid: scroll_id.clone(),
            top_row,
            left_col,
            bottom_row,
            right_col,
        });
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
"#;

#[cfg(test)]
mod tests {
    use super::*;

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
}

//! RENDER_PLAN — the Sheet stage's deterministic display list (S3.3).
//!
//! A pure, framework-free projection of a windowed [`GridProjection`] into a
//! [`RenderPlan`]: the flat, ordered list of what the Canvas2D renderer (S3.5)
//! draws for the current [`Viewport`] — the on-screen cells (with their honest
//! display text), the visible column and row header bands, and the grid extent.
//! No Leptos, no DOM, no `web_sys`; the whole plan is data, so its two doctrines
//! are asserted directly in unit tests:
//!
//! - **Determinism**: equal inputs yield an *equal* plan ([`RenderPlan`] derives
//!   `PartialEq`). Cells are sorted by `(row, col)`, so the plan never depends on
//!   the order the host happened to list `grid.cells` in.
//! - **Honesty**: the plan contains EXACTLY the projection's windowed cells that
//!   intersect the viewport — never a cell the host did not provide (the window
//!   is "viewing is subscribing"; fabricating cells would invent values), and
//!   never dropping one that is on screen. Each cell's text is a faithful
//!   rendering of its [`NodeValueProjection`] ([`value_text`]) — an array is a
//!   `R×C` shape summary, never a fabricated full grid of cells.
//!
//! Geometry (rects, visible bands) is delegated wholesale to [`crate::geometry`],
//! so a planned cell's rect round-trips through
//! [`hit_test`](crate::geometry::hit_test) to its own `(row, col)` by
//! construction — the property the tests pin over the real demo workbook.
//!
//! Gridlines are intentionally *not* materialized here: they are a pure function
//! of the metrics and the visible bands, so the renderer derives them from
//! [`RenderPlan::extent_rows`]/[`extent_cols`](RenderPlan::extent_cols) and the
//! header bands rather than the plan carrying a redundant, must-stay-in-sync copy.

use dnacalc_skin_ir::{GridProjection, NodeValueProjection};

use crate::geometry::{
    CellRect, GridMetrics, Viewport, cell_rect, visible_col_range, visible_row_range,
};

/// One cell in a [`RenderPlan`]: its 1-based address, its viewport-local rect
/// ([`cell_rect`]), and the honest display text of its value ([`value_text`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedCell {
    pub row: u32,
    pub col: u32,
    pub rect: CellRect,
    pub text: String,
}

/// One visible column-header band: the 1-based column, its header rect (in the
/// top strip), and its label.
///
/// The `label` is the column's 1-based index as text, deliberately **not** an
/// A1 column letter (`A`, `B`, … `AA`): letter formatting is a display/format
/// concern the (later) formatter owns, and this module declines to stand up a
/// second address-format authority — mirroring the Notebook model's choice to
/// label cells `R{row}C{col}` rather than mint an A1 authority. The renderer maps
/// the index to a letter when the format layer lands.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedColHeader {
    pub col: u32,
    pub rect: CellRect,
    pub label: String,
}

/// One visible row-header band: the 1-based row, its header rect (in the left
/// strip), and its label (the row's 1-based index as text — the row axis is
/// already numeric in A1, so no format authority is implied here).
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedRowHeader {
    pub row: u32,
    pub rect: CellRect,
    pub label: String,
}

/// The deterministic display list for one [`Viewport`] over a [`GridProjection`].
///
/// Derives `PartialEq` so determinism is directly assertable: `build_render_plan`
/// of equal inputs yields an equal `RenderPlan`. `extent_rows`/`extent_cols`
/// carry the grid's full bounds (`max_rows`/`max_cols`) so a renderer can size
/// scrollbars and draw gridlines without re-reading the projection.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderPlan {
    /// On-screen cells, sorted by `(row, col)` — exactly the projection's
    /// windowed cells whose rect intersects the viewport.
    pub cells: Vec<PlannedCell>,
    /// Visible column-header bands, left-to-right.
    pub col_headers: Vec<PlannedColHeader>,
    /// Visible row-header bands, top-to-bottom.
    pub row_headers: Vec<PlannedRowHeader>,
    /// The grid's full row extent (`GridProjection::max_rows`).
    pub extent_rows: u32,
    /// The grid's full column extent (`GridProjection::max_cols`).
    pub extent_cols: u32,
}

/// Build the deterministic [`RenderPlan`] for `grid` under `m`/`v`.
///
/// Plans a cell iff it (a) exists in `grid.cells` — the host's window, never
/// fabricated — AND (b) is inside the grid's `[1, max_rows] × [1, max_cols]`
/// extent AND (c) intersects the viewport ([`rect_intersects_viewport`]). The
/// surviving cells are sorted by `(row, col)`, so the plan is independent of the
/// projection's cell ordering (determinism). Header bands cover the visible
/// row/column ranges ([`visible_col_range`]/[`visible_row_range`]) clamped to
/// `grid.max_cols`/`grid.max_rows`. The extents are copied through verbatim.
///
/// EXTENT COUPLING (load-bearing invariant): a planned cell is always a
/// *hittable* cell — [`hit_test`] must be called with THIS plan's
/// `extent_rows`/`extent_cols` (i.e. the same `grid.max_rows`/`max_cols`), so
/// that every `PlannedCell` round-trips through `hit_test` back to its own
/// `(row, col)`. The in-extent guard (b) is what makes that hold even for a
/// malformed projection cell reported outside the grid extent: such a cell is
/// skipped rather than rendered-but-unhittable.
#[must_use]
pub fn build_render_plan(grid: &GridProjection, m: &GridMetrics, v: &Viewport) -> RenderPlan {
    // (a) exists in the host window, (b) in-extent, (c) intersects the viewport.
    let mut cells: Vec<PlannedCell> = grid
        .cells
        .iter()
        .filter_map(|cell| {
            // (b) In-extent guard: a projection cell outside [1, max_rows] ×
            // [1, max_cols] is malformed for this grid (rows/cols are 1-based).
            // Skipping it keeps the plan↔hit_test round-trip exact — `hit_test`
            // clamps to the same extent, so a planned cell is always hittable.
            if cell.row < 1
                || cell.row > grid.max_rows
                || cell.col < 1
                || cell.col > grid.max_cols
            {
                return None;
            }
            let rect = cell_rect(m, v, cell.row, cell.col);
            if rect_intersects_viewport(&rect, v) {
                Some(PlannedCell {
                    row: cell.row,
                    col: cell.col,
                    rect,
                    text: value_text(&cell.value),
                })
            } else {
                None
            }
        })
        .collect();
    // Deterministic order regardless of the projection's cell ordering.
    cells.sort_by_key(|cell| (cell.row, cell.col));

    let col_headers = visible_col_range(m, v, grid.max_cols)
        .map(|(first, last)| {
            (first..=last)
                .map(|col| PlannedColHeader {
                    col,
                    rect: col_header_rect(m, v, col),
                    label: col_label(col),
                })
                .collect()
        })
        .unwrap_or_default();

    let row_headers = visible_row_range(m, v, grid.max_rows)
        .map(|(first, last)| {
            (first..=last)
                .map(|row| PlannedRowHeader {
                    row,
                    rect: row_header_rect(m, v, row),
                    label: row.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    RenderPlan {
        cells,
        col_headers,
        row_headers,
        extent_rows: grid.max_rows,
        extent_cols: grid.max_cols,
    }
}

/// The Excel A1 column label for a 1-based column index: 1→A, 26→Z, 27→AA,
/// 702→ZZ, 703→AAA. A pure, total presentation function (bijective base-26) —
/// it FORMATS an index for the Excel-strict header row; it neither parses nor
/// resolves addresses, so it is a skin-layer display concern, not a second
/// address authority (SHEET_SPEC: the grid "behaves like Excel where behavior
/// counts, looks like Strand"). Rows keep their numeric labels (Excel's own
/// convention). `col == 0` (never produced by the 1-based visible range) yields
/// an empty string.
#[must_use]
pub fn col_label(col: u32) -> String {
    let mut n = col;
    let mut label = String::new();
    while n > 0 {
        let rem = ((n - 1) % 26) as u8;
        label.insert(0, (b'A' + rem) as char);
        n = (n - 1) / 26;
    }
    label
}

/// The header rect for `col`: the column's data band, but spanning the top
/// header strip (`y = 0`, `h = header_h`) instead of a data row.
fn col_header_rect(m: &GridMetrics, v: &Viewport, col: u32) -> CellRect {
    let data = cell_rect(m, v, 1, col);
    CellRect {
        x: data.x,
        y: 0.0,
        w: m.col_width,
        h: m.header_h,
    }
}

/// The header rect for `row`: the row's data band, but spanning the left header
/// strip (`x = 0`, `w = header_w`) instead of a data column.
fn row_header_rect(m: &GridMetrics, v: &Viewport, row: u32) -> CellRect {
    let data = cell_rect(m, v, row, 1);
    CellRect {
        x: 0.0,
        y: data.y,
        w: m.header_w,
        h: m.row_height,
    }
}

/// Whether a cell rect overlaps the viewport rectangle `[0, width) × [0, height)`
/// (standard half-open AABB overlap). A cell partly scrolled under a header strip
/// still counts as on screen — the header simply overdraws it — which is the
/// honest "falls in the viewport" test the RenderPlan uses.
fn rect_intersects_viewport(rect: &CellRect, v: &Viewport) -> bool {
    rect.x < v.width && rect.x + rect.w > 0.0 && rect.y < v.height && rect.y + rect.h > 0.0
}

/// The honest display text for a grid cell's value.
///
/// - `Number`/`Logical`: the engine's own `display` text (never a re-formatted guess).
/// - `Text`/`Scalar`: the string verbatim.
/// - `Error`: its diagnostic message (surfaced, not swallowed).
/// - `Reference`: the engine target string verbatim.
/// - `Empty`/`Missing`/`Unevaluated`/`Pending`: a parenthesized marker, so an
///   absent/in-flight value reads as a state, never as a blank cell that might be
///   mistaken for real content.
/// - `Array`: a `R×C` shape summary — the array's shape, **never** a fabricated
///   render of its cells (a grid cell shows a spilled array's anchor summary; the
///   spilled cells are their own projected cells).
#[must_use]
pub fn value_text(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Number { display, .. }
        | NodeValueProjection::Logical { display, .. } => display.clone(),
        NodeValueProjection::Text(text) | NodeValueProjection::Scalar(text) => text.clone(),
        NodeValueProjection::Error(message) => message.clone(),
        NodeValueProjection::Reference { target } => target.clone(),
        NodeValueProjection::Empty => "(empty)".to_string(),
        NodeValueProjection::Missing => "(missing)".to_string(),
        NodeValueProjection::Unevaluated => "(unevaluated)".to_string(),
        NodeValueProjection::Pending => "(pending)".to_string(),
        // Shape summary only — the cells are deliberately not unrolled here.
        NodeValueProjection::Array { rows, cols, .. } => format!("{rows}\u{00d7}{cols} array"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{HitTarget, hit_test};
    use dnacalc_skin_ir::{
        GridCellProjection, GridOverlayBundle, GridProjection, NodeId, NodeKey,
    };

    fn cell(row: u32, col: u32, value: NodeValueProjection) -> GridCellProjection {
        GridCellProjection {
            row,
            col,
            value,
            value_epoch: 1,
            authored: None,
            provenance: None,
        }
    }

    fn number(display: &str) -> NodeValueProjection {
        NodeValueProjection::Number {
            raw: display.to_string(),
            display: display.to_string(),
        }
    }

    fn grid_with(cells: Vec<GridCellProjection>, max_rows: u32, max_cols: u32) -> GridProjection {
        GridProjection {
            grid_node_key: NodeKey::new("k:Sheet1"),
            grid_node_id: NodeId::new("Sheet1"),
            grid_id: "grid:Sheet1".to_string(),
            max_rows,
            max_cols,
            cells,
            projection_epoch: 1,
            overlays: GridOverlayBundle::default(),
            overlay_epoch: 0,
            differential_clean: true,
            authored_epoch: 0,
        }
    }

    fn viewport(width: f64, height: f64) -> Viewport {
        Viewport {
            scroll_x: 0.0,
            scroll_y: 0.0,
            width,
            height,
        }
    }

    /// `build_render_plan` is deterministic: equal inputs yield an equal plan,
    /// AND the plan is independent of the projection's cell ordering (a shuffled
    /// input produces the same plan).
    #[test]
    fn build_render_plan_is_deterministic_and_order_independent() {
        let m = GridMetrics::default();
        let v = viewport(800.0, 600.0);

        let ordered = grid_with(
            vec![
                cell(1, 1, number("1")),
                cell(1, 2, number("2")),
                cell(2, 1, number("3")),
                cell(3, 5, number("4")),
            ],
            100,
            100,
        );
        let shuffled = grid_with(
            vec![
                cell(3, 5, number("4")),
                cell(2, 1, number("3")),
                cell(1, 2, number("2")),
                cell(1, 1, number("1")),
            ],
            100,
            100,
        );

        let plan_a = build_render_plan(&ordered, &m, &v);
        // Equal inputs → equal plan.
        assert_eq!(plan_a, build_render_plan(&ordered, &m, &v));
        // Cell ordering does not change the plan.
        assert_eq!(plan_a, build_render_plan(&shuffled, &m, &v));
        // ...and the cells came out sorted by (row, col).
        let addrs: Vec<(u32, u32)> = plan_a.cells.iter().map(|c| (c.row, c.col)).collect();
        assert_eq!(addrs, vec![(1, 1), (1, 2), (2, 1), (3, 5)]);
    }

    /// The plan contains EXACTLY the windowed cells whose rect intersects the
    /// viewport — none fabricated, none on-screen dropped — cross-checked
    /// against an independent per-cell intersection test. The viewport is sized
    /// to clip some cells so the filter is genuinely exercised.
    #[test]
    fn build_render_plan_plans_exactly_the_on_screen_windowed_cells() {
        let m = GridMetrics::default();
        // A viewport that shows the first ~3 columns and ~5 rows of data.
        let v = viewport(m.header_w + 3.0 * m.col_width, m.header_h + 5.0 * m.row_height);

        // Cells across a wide address span: some on screen, some off to the
        // right / below.
        let source = vec![
            cell(1, 1, number("a")),
            cell(2, 2, number("b")),
            cell(5, 3, number("c")),
            cell(1, 20, number("off-right")),
            cell(40, 1, number("off-bottom")),
            cell(3, 3, number("d")),
        ];
        let grid = grid_with(source.clone(), 1000, 1000);
        let plan = build_render_plan(&grid, &m, &v);

        // Independent expectation: the same intersection test, applied directly.
        let expected: std::collections::BTreeSet<(u32, u32)> = source
            .iter()
            .filter(|c| {
                let r = cell_rect(&m, &v, c.row, c.col);
                r.x < v.width && r.x + r.w > 0.0 && r.y < v.height && r.y + r.h > 0.0
            })
            .map(|c| (c.row, c.col))
            .collect();
        let planned: std::collections::BTreeSet<(u32, u32)> =
            plan.cells.iter().map(|c| (c.row, c.col)).collect();

        assert_eq!(planned, expected, "plan must equal the on-screen windowed cells");
        // Every planned address is one the host actually provided (no fabrication).
        let provided: std::collections::BTreeSet<(u32, u32)> =
            source.iter().map(|c| (c.row, c.col)).collect();
        assert!(
            planned.is_subset(&provided),
            "the plan never invents a cell the host did not window"
        );
    }

    /// Every planned cell's rect round-trips through `hit_test` back to its own
    /// `(row, col)` — the geometry↔hit-test inverse holds by construction across
    /// the whole plan.
    #[test]
    fn planned_cell_rects_round_trip_through_hit_test() {
        let m = GridMetrics::default();
        let v = viewport(1200.0, 900.0);
        let source: Vec<GridCellProjection> = (1..=8u32)
            .flat_map(|row| (1..=6u32).map(move |col| cell(row, col, number("x"))))
            .collect();
        let grid = grid_with(source, 100, 100);
        let plan = build_render_plan(&grid, &m, &v);
        assert!(!plan.cells.is_empty());
        for planned in &plan.cells {
            let cx = planned.rect.x + planned.rect.w / 2.0;
            let cy = planned.rect.y + planned.rect.h / 2.0;
            assert_eq!(
                hit_test(&m, &v, plan.extent_rows, plan.extent_cols, cx, cy),
                HitTarget::Cell {
                    row: planned.row,
                    col: planned.col,
                },
                "planned R{}C{} must hit-test back to itself",
                planned.row,
                planned.col,
            );
        }
    }

    /// `value_text` is honest across every projection variant: `display` for
    /// number/logical, verbatim text/scalar, the error message, parenthesized
    /// markers for absent/in-flight states, and a shape summary — never the
    /// cells — for an array.
    #[test]
    fn value_text_is_honest_per_variant() {
        assert_eq!(value_text(&number("3.14")), "3.14");
        assert_eq!(
            value_text(&NodeValueProjection::Logical {
                value: true,
                display: "TRUE".to_string(),
            }),
            "TRUE"
        );
        assert_eq!(
            value_text(&NodeValueProjection::Text("hello".to_string())),
            "hello"
        );
        assert_eq!(
            value_text(&NodeValueProjection::Scalar("legacy".to_string())),
            "legacy"
        );
        assert_eq!(
            value_text(&NodeValueProjection::Error("#DIV/0!".to_string())),
            "#DIV/0!"
        );
        assert_eq!(
            value_text(&NodeValueProjection::Reference {
                target: "Sheet1!A1".to_string(),
            }),
            "Sheet1!A1"
        );
        assert_eq!(value_text(&NodeValueProjection::Empty), "(empty)");
        assert_eq!(value_text(&NodeValueProjection::Missing), "(missing)");
        assert_eq!(
            value_text(&NodeValueProjection::Unevaluated),
            "(unevaluated)"
        );
        assert_eq!(value_text(&NodeValueProjection::Pending), "(pending)");

        // Array → shape summary, and crucially NOT a render of the cells.
        let array = NodeValueProjection::Array {
            rows: 2,
            cols: 3,
            cells: vec![
                vec![number("1"), number("2"), number("3")],
                vec![number("4"), number("5"), number("6")],
            ],
        };
        let text = value_text(&array);
        assert_eq!(text, "2\u{00d7}3 array");
        assert!(
            !text.contains('1') && !text.contains('4'),
            "the array summary must not fabricate a render of the cells: {text:?}"
        );
    }

    /// Header bands cover the visible range clamped to the extent: with a small
    /// extent the header stops at the last real column/row, and with a viewport
    /// showing exactly N bands the plan carries exactly N headers.
    #[test]
    fn header_bands_cover_the_visible_range_clamped_to_extent() {
        let m = GridMetrics::default();
        // Viewport wide/tall enough for many bands, but the grid extent is tiny.
        let v = viewport(2000.0, 2000.0);
        let grid = grid_with(Vec::new(), 4, 3);
        let plan = build_render_plan(&grid, &m, &v);

        let cols: Vec<u32> = plan.col_headers.iter().map(|h| h.col).collect();
        let rows: Vec<u32> = plan.row_headers.iter().map(|h| h.row).collect();
        assert_eq!(cols, vec![1, 2, 3], "columns clamp to the 3-column extent");
        assert_eq!(rows, vec![1, 2, 3, 4], "rows clamp to the 4-row extent");
        // Column headers are Excel A1 letters; row headers are 1-based indices.
        assert_eq!(plan.col_headers[0].label, "A");
        assert_eq!(plan.col_headers[2].label, "C");
        assert_eq!(plan.row_headers[3].label, "4");
        // A column header spans the top strip; a row header the left strip.
        assert_eq!(plan.col_headers[0].rect.y, 0.0);
        assert_eq!(plan.col_headers[0].rect.h, m.header_h);
        assert_eq!(plan.row_headers[0].rect.x, 0.0);
        assert_eq!(plan.row_headers[0].rect.w, m.header_w);
    }

    /// Over the REAL host-core demo workbook (native dev-dep), the plan's cell
    /// texts match the projection's computed values, exactly the windowed cells
    /// are planned (none fabricated, none dropped), and every planned rect
    /// round-trips through `hit_test` — the RenderPlan proven over genuine engine
    /// truth, not a hand-mock.
    #[test]
    fn build_render_plan_over_the_real_demo_grid_is_faithful() {
        use crate::active_grid;
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();
        let grid = active_grid(&ws).expect("the demo's first sheet grid resolves");

        let m = GridMetrics::default();
        // A viewport large enough to cover the whole demo used-range
        // (Sheet1: A1..B5).
        let v = viewport(1000.0, 800.0);
        let plan = build_render_plan(grid, &m, &v);

        // Exactly the windowed cells are planned (the used-range fits the
        // viewport, so nothing is clipped): same addresses, same count.
        let planned: std::collections::BTreeSet<(u32, u32)> =
            plan.cells.iter().map(|c| (c.row, c.col)).collect();
        let windowed: std::collections::BTreeSet<(u32, u32)> =
            grid.cells.iter().map(|c| (c.row, c.col)).collect();
        assert_eq!(
            planned, windowed,
            "the plan is exactly the projection's windowed cells (none dropped/fabricated)"
        );

        // Known demo values render honestly: Sheet1 A1 = 1, B1 = A1*10 = 10,
        // B5 = A5*10 = 50.
        let text_at = |row: u32, col: u32| -> String {
            plan.cells
                .iter()
                .find(|c| c.row == row && c.col == col)
                .unwrap_or_else(|| panic!("planned cell R{row}C{col}"))
                .text
                .clone()
        };
        assert_eq!(text_at(1, 1), "1", "Sheet1!A1 = 1");
        assert_eq!(text_at(1, 2), "10", "Sheet1!B1 = A1*10 = 10");
        assert_eq!(text_at(5, 2), "50", "Sheet1!B5 = A5*10 = 50");

        // Each planned cell's text equals the projection's own display, and its
        // rect round-trips through hit_test to its own address.
        for planned_cell in &plan.cells {
            let source = grid
                .cells
                .iter()
                .find(|c| c.row == planned_cell.row && c.col == planned_cell.col)
                .expect("a planned cell exists in the projection");
            assert_eq!(
                planned_cell.text,
                value_text(&source.value),
                "planned text is a faithful render of the projected value"
            );
            let cx = planned_cell.rect.x + planned_cell.rect.w / 2.0;
            let cy = planned_cell.rect.y + planned_cell.rect.h / 2.0;
            assert_eq!(
                hit_test(&m, &v, plan.extent_rows, plan.extent_cols, cx, cy),
                HitTarget::Cell {
                    row: planned_cell.row,
                    col: planned_cell.col,
                },
                "demo cell R{}C{} rect round-trips through hit_test",
                planned_cell.row,
                planned_cell.col,
            );
        }
    }

    /// A1 column labels (Excel-strict header row): bijective base-26, rows stay
    /// numeric. Boundary cases at each carry (Z→AA, AZ→BA, ZZ→AAA).
    #[test]
    fn col_label_matches_excel_a1_columns() {
        for (col, expected) in [
            (1, "A"),
            (2, "B"),
            (26, "Z"),
            (27, "AA"),
            (28, "AB"),
            (52, "AZ"),
            (53, "BA"),
            (702, "ZZ"),
            (703, "AAA"),
        ] {
            assert_eq!(col_label(col), expected, "column {col} labels as {expected}");
        }
    }

    /// EXTENT GUARD (review finding #1): a projection cell reported OUTSIDE the
    /// grid's `[1, max_rows] × [1, max_cols]` extent is NOT planned — otherwise it
    /// would render but fail to round-trip through `hit_test` (which clamps to
    /// the same extent), i.e. a visible-but-unclickable cell. The in-extent cell
    /// is still planned; the out-of-extent one is skipped and, at its own center,
    /// `hit_test` correctly reports `Outside`.
    #[test]
    fn build_render_plan_skips_cells_outside_the_grid_extent() {
        let in_extent = cell(2, 2, number("7"));
        let past_rows = cell(5, 1, number("99")); // row 5 > max_rows 3
        let grid = grid_with(vec![in_extent, past_rows], 3, 3);
        let m = GridMetrics::default();
        let v = viewport(800.0, 600.0);
        let plan = build_render_plan(&grid, &m, &v);

        let planned: Vec<(u32, u32)> = plan.cells.iter().map(|c| (c.row, c.col)).collect();
        assert!(planned.contains(&(2, 2)), "the in-extent cell is planned");
        assert!(
            !planned.contains(&(5, 1)),
            "the out-of-extent cell (row 5 > max_rows 3) is skipped, not rendered-but-unhittable"
        );

        // And at that skipped cell's would-be center, hit_test agrees it is Outside.
        let ghost = cell_rect(&m, &v, 5, 1);
        assert_eq!(
            hit_test(
                &m,
                &v,
                plan.extent_rows,
                plan.extent_cols,
                ghost.x + ghost.w / 2.0,
                ghost.y + ghost.h / 2.0,
            ),
            HitTarget::Outside,
            "a cell past the extent is Outside — so skipping it keeps plan↔hit_test exact"
        );
    }
}

//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Sheet stage (S3): the Excel-strict profile's home stage — a Canvas2D
//! grid rendered from a windowed [`dnacalc_skin_ir::GridProjection`]. The plan
//! (SHEET_SPEC) is a canvas tile renderer + exactly one DOM overlay editor, both
//! driven by a **RenderPlan IR** whose geometry and hit-test are pure functions
//! with unit-tested invariants (Foundation doctrine — no screenshot assertions).
//! Honest-degrade v1: the stage works at bounded-sheet scale until G4 makes
//! `SetGridInterest` real, single-cell selection until G3, and the Detail zoom
//! tier until the labeled-block/district tiers land.
//!
//! **S3.1 (the scaffold)** stood up the [`dnacalc_shell::StageSurface`] under
//! [`dnacalc_shell::StageId::Sheet`] and rendered honestly from host truth: an
//! explicit honest-empty card when the workspace carries no grid, else a
//! placeholder readout of the active grid's real extent + windowed cell count.
//! It is never rendered blank.
//!
//! **S3.5 (this bead)** replaces that placeholder with a real `<canvas>` that
//! draws the [`RenderPlan`] via [`crate::canvas::draw_render_plan`]: the honest
//! pixels of the windowed cells, header bands, gridlines, and value text. The
//! new code is the thin DOM-facing draw layer ([`crate::canvas`]) plus the
//! reactive redraw wiring in [`SheetStage::mount`] — device-pixel-ratio sizing
//! for crisp text, live `--dna-*` palette resolution, and a full redraw on each
//! workspace change (real tiling / `SetGridInterest` narrowing is G4 / S3.9).
//! The geometry and the plan themselves ([`crate::geometry`],
//! [`crate::render_plan`]) are pure + unit-tested and merely *consumed* here.
//! The honest-empty card still stands in for a workspace with no grid.
//!
//! A visually-hidden debug readout (`data-testid="sheet-render-plan"`) mirrors
//! the drawn plan's cell count + extent, so the S3.11 browser test can assert
//! the demo grid rendered without pixel-reading the canvas.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::{GridProjection, WorkspaceState};

pub mod canvas;
pub mod geometry;
pub mod render_plan;

pub use canvas::{Palette, draw_render_plan, looks_numeric, resolve_palette};
pub use geometry::{
    CellRect, GridMetrics, HitTarget, Viewport, cell_rect, hit_test, visible_col_range,
    visible_row_range,
};
pub use render_plan::{
    PlannedCell, PlannedColHeader, PlannedRowHeader, RenderPlan, build_render_plan, col_label,
    value_text,
};

/// The Sheet stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct SheetStage;

impl SheetStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StageSurface for SheetStage {
    fn id(&self) -> StageId {
        StageId::Sheet
    }

    fn title(&self) -> &'static str {
        "Sheet"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        // The one host-truth read the stage makes. `ReadSignal` is `Copy`, so
        // every reactive read below re-derives from live host truth — the grid
        // view is never a copy that can drift.
        let workspace = ctx.workspace;

        // The canvas node, and the two signals the redraw effect publishes for
        // the debug readout. All are created ONCE here in `mount` (the stable
        // owner), so a workspace re-projection re-runs only the inner reactive
        // fragments, never remints the canvas or its effect.
        let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
        let plan_cell_count = RwSignal::new(0usize);
        let plan_extent = RwSignal::new(String::new());

        // Whether the workspace carries a grid to draw. A `Memo` fires only when
        // this boolean flips, so the `<canvas>` is built once (not remade on
        // every workspace change) — the redraw effect owns the per-projection
        // repaint, not a view rebuild.
        let has_grid = Memo::new(move |_| workspace.with(|ws| active_grid(ws).is_some()));

        // The reactive redraw. Subscribes to the workspace (a re-projection
        // repaints) and to `canvas_ref` (bound after the canvas mounts). Every
        // JS-interop step degrades gracefully (`let ... else` / `ok()`), never
        // unwrapping: a not-yet-mounted canvas, a missing `window`, or a null 2d
        // context is an early return, not a panic. Under `ssr` (native) Leptos
        // never runs effects, so this closure is the wasm-only draw path with an
        // automatic native no-op — no `cfg` gate needed.
        Effect::new(move |_| {
            // The canvas may not be in the DOM yet (first run, or no grid): bail
            // until its ref binds, at which point this effect re-runs.
            let Some(el) = canvas_ref.get() else {
                return;
            };
            let Some(window) = web_sys::window() else {
                return;
            };

            // DPR sizing: the backing store is device px (crisp text) while the
            // CSS box stays the container size (the stylesheet sizes it to 100%).
            // Setting width/height also resets the 2d transform, so we re-apply
            // the DPR scale each redraw and thereafter draw in CSS px.
            let dpr = window.device_pixel_ratio().max(1.0);
            let css_w = f64::from(el.client_width());
            let css_h = f64::from(el.client_height());
            el.set_width((css_w * dpr).round().max(0.0) as u32);
            el.set_height((css_h * dpr).round().max(0.0) as u32);

            let Some(ctx2d) = el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|obj| obj.dyn_into::<CanvasRenderingContext2d>().ok())
            else {
                return;
            };
            let _ = ctx2d.scale(dpr, dpr);

            let metrics = GridMetrics::default();
            // Origin viewport: scroll/zoom is S3.9. The demo used-range fits the
            // top-left, which is exactly what the plan windows here.
            let viewport = Viewport {
                scroll_x: 0.0,
                scroll_y: 0.0,
                width: css_w,
                height: css_h,
            };
            let palette = resolve_palette(&el);

            // Build + draw the plan from the SAME workspace read, and publish its
            // honest cell count + extent for the debug readout. Reading the
            // workspace here is what subscribes this effect to re-projections.
            let published = workspace.with(|ws| {
                active_grid(ws).map(|grid| {
                    let plan = build_render_plan(grid, &metrics, &viewport);
                    draw_render_plan(&ctx2d, &plan, &metrics, &viewport, &palette);
                    (plan.cells.len(), plan.extent_rows, plan.extent_cols)
                })
            });
            if let Some((count, rows, cols)) = published {
                plan_cell_count.set(count);
                plan_extent.set(format!("{rows}\u{00d7}{cols}"));
            }
        });

        let view = view! {
            <style>{SHEET_CSS}</style>
            <section
                class="dna-sheet"
                data-stage="sheet"
                data-testid="sheet-root"
                data-dna-density="working"
            >
                {move || {
                    if has_grid.get() {
                        // The real canvas + a visually-hidden debug readout that
                        // mirrors the drawn plan (so S3.11 asserts the demo grid
                        // without pixel-reading). Built once per grid-presence
                        // flip; the redraw effect repaints on every projection.
                        view! {
                            <canvas
                                class="dna-sheet__canvas"
                                data-testid="sheet-canvas"
                                node_ref=canvas_ref
                            ></canvas>
                            <span
                                class="dna-sheet__debug"
                                data-testid="sheet-render-plan"
                                aria-hidden="true"
                                data-cell-count=move || plan_cell_count.get().to_string()
                                data-extent=move || plan_extent.get()
                            >
                                {move || {
                                    format!(
                                        "{} cells \u{00b7} {}",
                                        plan_cell_count.get(),
                                        plan_extent.get(),
                                    )
                                }}
                            </span>
                        }
                        .into_any()
                    } else {
                        // Honest-empty: no grid to draw, never a blank canvas.
                        view! {
                            <p class="dna-sheet__empty" data-testid="sheet-empty">
                                "No grid in this workbook."
                            </p>
                        }
                        .into_any()
                    }
                }}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// The active grid to render: the first sheet's backing grid (the active-sheet
/// selection is a sheet-tabs concern, S3 later). `None` when the workspace
/// carries no sheet, or the sheet's grid is not in the current window. A pure
/// function of [`WorkspaceState`], so it is unit-testable against the real demo
/// workbook.
#[must_use]
pub fn active_grid(ws: &WorkspaceState) -> Option<&GridProjection> {
    let sheet = ws.sheets.first()?;
    ws.grids.get(&sheet.grid_node_id)
}

/// The Sheet stage's scoped stylesheet — Strand `--dna-*` tokens only. The
/// canvas fills the stage (its CSS box is the container size; the redraw effect
/// sizes the device-px backing store off `clientWidth`/`clientHeight`). The
/// debug readout is visually hidden (off-screen, not `display:none`) so it stays
/// queryable by `data-testid`/`data-*` for the S3.11 browser test.
pub const SHEET_CSS: &str = "\
.dna-sheet{display:flex;flex-direction:column;gap:var(--dna-gap-3);padding:var(--dna-gap-4);color:var(--dna-ink);height:100%;min-height:0}
.dna-sheet__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-sheet__canvas{flex:1 1 auto;width:100%;min-height:0;display:block}
.dna-sheet__debug{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_stage_identity() {
        let stage = SheetStage::new();
        assert_eq!(stage.id(), StageId::Sheet);
        assert_eq!(stage.title(), "Sheet");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }

    #[test]
    fn active_grid_is_honest_none_for_an_empty_workspace() {
        let ws = WorkspaceState::default();
        assert!(
            active_grid(&ws).is_none(),
            "no grid is fabricated for an empty workspace"
        );
    }

    /// Over the REAL host-core demo workbook (native dev-dep), the active grid
    /// resolves to the first sheet's backing grid and carries a real extent +
    /// windowed cells — proving the stage renders from genuine host truth, not a
    /// hand-mock.
    #[test]
    fn active_grid_resolves_the_real_demo_grid() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();

        let first_sheet = ws.sheets.first().expect("the demo workbook has a sheet");
        let grid = active_grid(&ws).expect("the first sheet's grid resolves");
        assert_eq!(grid.grid_node_id, first_sheet.grid_node_id);
        assert!(grid.max_rows > 0 && grid.max_cols > 0, "the grid has a real extent");
        assert!(
            !grid.cells.is_empty(),
            "the demo grid windows at least one computed cell"
        );
    }
}

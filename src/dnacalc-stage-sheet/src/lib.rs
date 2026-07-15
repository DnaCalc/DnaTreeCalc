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
//!
//! **S3.6 (this bead)** adds the core grid interaction: a click on the canvas
//! hit-tests to a cell ([`hit_test`]), which becomes the single selected/active
//! cell; exactly ONE DOM overlay editor ([`dnacalc_bridge::FormulaBridgeDegrade`])
//! then floats over the canvas at that cell's rect ([`cell_rect`]), seeded from
//! the cell's own authored text. Enter commits through the universal entry verb
//! ([`edit::enter_cell_intent`]) and reads the host's three-way outcome back
//! ([`edit::interpret_receipt`]); a successful commit closes the overlay and the
//! workbook dispatcher's re-projection repaints the canvas automatically (no
//! manual refresh), while a rejection keeps the editor open with the typed
//! diagnostics underlined; Esc cancels. The click-mapping and editor positioning
//! use the SAME [`GridMetrics::default`] + origin [`Viewport`] the redraw effect
//! draws with, so a click lands on — and the editor sits over — the drawn cell.
//! Single-cell selection only: arrow-key navigation and range/fill are S3.8.

use std::sync::Arc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::intent::Dispatcher;
use dnacalc_skin_ir::{
    GridEntryDiagnosticProjection, GridProjection, NodeId, WorkspaceState,
};

pub mod canvas;
pub mod edit;
pub mod geometry;
pub mod render_plan;

pub use canvas::{Palette, draw_render_plan, looks_numeric, resolve_palette};
pub use edit::{CellOutcome, enter_cell_intent, interpret_receipt};
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

        // --- S3.6 single-cell selection + one overlay editor -------------------
        // The one dispatcher a commit crosses (SHELL_SPEC §6). Held in a
        // `StoredValue` (Copy) so both the click handler and the overlay closure
        // — which are embedded inside the re-runnable `has_grid` branch — stay
        // `Copy` and can be re-embedded when grid-presence flips. The workbook
        // dispatcher re-projects on every accepted dispatch, so a committed edit
        // repaints the canvas through the redraw effect with no refresh of our own.
        let dispatch: StoredValue<Arc<dyn Dispatcher>> = StoredValue::new(ctx.dispatch.clone());
        // The single selected/active cell. `None` closes the editor. Created ONCE
        // here in `mount` (the stable owner), so a workspace re-projection never
        // remints it — selection survives a repaint.
        let selected_cell: RwSignal<Option<(u32, u32)>> = RwSignal::new(None);
        // The one editor's live buffer, its last commit's typed rejections, and a
        // remount counter. Like the Notebook's per-block state but singular (there
        // is exactly one active-cell editor): the buffer is seeded on selection and
        // read UNTRACKED on remount, so typing never remounts the bridge — only a
        // cell change or a commit/revert (`editor_revision`) does.
        let edit_text = RwSignal::new(String::new());
        let editor_rejections: RwSignal<Vec<GridEntryDiagnosticProjection>> =
            RwSignal::new(Vec::new());
        let editor_revision = RwSignal::new(0usize);

        // Click → select. Map the pointer into the SAME origin viewport / metrics
        // the redraw effect draws with, hit-test, and set the active cell. The
        // canvas fills its viewport wrapper (`position:absolute; inset:0`) with no
        // border/padding, so `offset_x`/`offset_y` are already in the drawn
        // coordinate space (CSS px, origin viewport) — a click lands on the cell
        // under the cursor. All captures are `Copy`, so this handler is `Copy`.
        let click_workspace = workspace;
        let on_canvas_mousedown = move |ev: leptos::ev::MouseEvent| {
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            let metrics = GridMetrics::default();
            let viewport = Viewport::default(); // origin (scroll 0): matches drawn pixels
            // Grid id + extent from live host truth (untracked — a handler never
            // subscribes). `None` (no grid) leaves selection untouched.
            let Some((grid, extent_rows, extent_cols)) = click_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return;
            };
            match hit_test(&metrics, &viewport, extent_rows, extent_cols, x, y) {
                HitTarget::Cell { row, col } => {
                    // Seed the one editor from THIS cell's own authored text (host
                    // truth), clear any stale rejection underline, remount the
                    // bridge, then open the overlay at this cell by selecting it.
                    let seed = click_workspace
                        .with_untracked(|ws| edit::current_authored_seed(ws, &grid, row, col));
                    edit_text.set(seed);
                    editor_rejections.set(Vec::new());
                    editor_revision.update(|revision| *revision += 1);
                    selected_cell.set(Some((row, col)));
                }
                // Header / corner / outside close the editor. Single-cell selection
                // only — header selection + ranges are S3.8; never a fabricated one.
                HitTarget::ColumnHeader { .. }
                | HitTarget::RowHeader { .. }
                | HitTarget::Corner
                | HitTarget::Outside => selected_cell.set(None),
            }
        };

        // The single overlay editor. Renders ONLY when a cell is selected AND the
        // grid resolves; positioned at the selected cell's `cell_rect` (the same
        // origin viewport / metrics the canvas drew with, so it sits over the drawn
        // cell). All captures are `Copy` (the `Arc` lives in `dispatch`, a
        // `StoredValue`), so this closure is `Copy` and safe to re-embed when the
        // `has_grid` branch re-runs.
        let overlay_workspace = workspace;
        let cell_overlay = move || {
            // Subscribe to the remount signal so a rejected commit re-applies the
            // underline (a rejection leaves the workspace untouched, so nothing
            // else would remount the bridge). `selected_cell` drives open/close.
            editor_revision.get();
            let Some((row, col)) = selected_cell.get() else {
                return ().into_any();
            };
            // Resolve the target from host truth (untracked: the click seeded the
            // buffer; a successful commit closes via `selected_cell`, and a
            // rejection re-runs via `editor_revision` — neither needs a workspace
            // subscription here, and reading untracked keeps typing undisturbed).
            let Some(target) =
                overlay_workspace.with_untracked(|ws| resolve_cell_edit_target(ws, row, col))
            else {
                return ().into_any();
            };
            let rect = cell_rect(&GridMetrics::default(), &Viewport::default(), row, col);
            // Honor the cell rect as the anchor + minimum. The degrade bridge is a
            // full card (editor box + rejection list), so it cannot fit a 22px cell
            // exactly; anchoring at the cell's top-left with the cell's min-width /
            // min-height lets it grow honestly (Excel widens its editor too) rather
            // than clip to a fake cell-sized box.
            let position = format!(
                "left:{}px;top:{}px;min-width:{}px;min-height:{}px",
                rect.x, rect.y, rect.w, rect.h
            );
            match target {
                CellEditTarget::ReadOnly { reason } => {
                    // A non-editable cell (repeated-region / merged / spill / table
                    // follower): an honest read-only note at the cell, never a fake
                    // editor (SHELL_SPEC §6 honesty).
                    view! {
                        <div
                            class="dna-sheet__cell-editor dna-sheet__cell-editor--readonly"
                            data-testid="sheet-cell-editor"
                            data-editable="false"
                            style=position
                        >
                            <p
                                class="dna-sheet__cell-readonly"
                                data-testid="sheet-cell-readonly"
                            >
                                {reason}
                            </p>
                        </div>
                    }
                    .into_any()
                }
                CellEditTarget::Editable { grid } => {
                    // Remount-gated seed: read the buffer + rejections UNTRACKED so
                    // typing (which updates `edit_text`) never remounts the bridge.
                    let seed_now = edit_text.get_untracked();
                    let rejections_now = editor_rejections.get_untracked();
                    let commit_grid = grid.clone();
                    let commit_dispatch = dispatch.with_value(Arc::clone);
                    let on_event = Callback::new(move |event: BridgeEvent| match event {
                        // Verbatim text; the host classifies `=`-vs-literal, never
                        // the skin (SHELL_SPEC §6 layering law).
                        BridgeEvent::TextEdited { text, .. } => edit_text.set(text),
                        BridgeEvent::CommitRequested => {
                            let text = edit_text.get_untracked();
                            let receipt = commit_dispatch.dispatch(enter_cell_intent(
                                commit_grid.clone(),
                                row,
                                col,
                                text,
                            ));
                            match interpret_receipt(&receipt) {
                                CellOutcome::Rejected(diagnostics) => {
                                    // Keep the editor open with the entered text
                                    // intact and underline the typed diagnostics
                                    // (the host guarantees no mutation on this path).
                                    editor_rejections.set(diagnostics);
                                    editor_revision.update(|revision| *revision += 1);
                                }
                                // Accepted (Literal / Formula / Cleared / NoChange):
                                // close the overlay. The workbook dispatcher
                                // re-projects, so the canvas repaints the new value
                                // automatically through the redraw effect.
                                _ => {
                                    editor_rejections.set(Vec::new());
                                    selected_cell.set(None);
                                }
                            }
                        }
                        // Esc: cancel — close the overlay, dispatch NOTHING.
                        BridgeEvent::RevertRequested => {
                            editor_rejections.set(Vec::new());
                            selected_cell.set(None);
                        }
                        _ => {}
                    });
                    view! {
                        <div
                            class="dna-sheet__cell-editor"
                            data-testid="sheet-cell-editor"
                            data-editable="true"
                            data-cell=format!("{row}:{col}")
                            style=position
                        >
                            <FormulaBridgeDegrade
                                text=seed_now
                                rejections=rejections_now
                                on_event=on_event
                            />
                        </div>
                    }
                    .into_any()
                }
            }
        };

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
                        // The real canvas in a POSITIONED viewport wrapper (the
                        // overlay editor's absolute-positioning ancestor), the one
                        // active-cell overlay, and a visually-hidden debug readout
                        // that mirrors the drawn plan (so S3.11 asserts the demo
                        // grid without pixel-reading). Built once per grid-presence
                        // flip; the redraw effect repaints on every projection, and
                        // the overlay reacts to the selected cell.
                        view! {
                            <div class="dna-sheet__viewport" data-testid="sheet-viewport">
                                <canvas
                                    class="dna-sheet__canvas"
                                    data-testid="sheet-canvas"
                                    node_ref=canvas_ref
                                    on:mousedown=on_canvas_mousedown
                                ></canvas>
                                {cell_overlay}
                            </div>
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

/// The overlay decision for a selected cell, resolved from host truth: either an
/// editor for an editable cell, or an honest read-only note for a non-editable
/// one. Never a third case, and never a fake editor for a read-only cell.
enum CellEditTarget {
    /// The cell admits authoring: build the degrade editor, committing against
    /// this grid at the selected `(row, col)`.
    Editable { grid: NodeId },
    /// The cell is non-editable (a repeated-region / merged / spill / table
    /// follower) — carry the typed reason for the read-only note.
    ReadOnly { reason: &'static str },
}

/// Resolve the overlay decision for the selected `(row, col)` on the active grid.
/// `None` when the workspace carries no grid. A cell whose authored record is
/// non-`Editable` becomes [`CellEditTarget::ReadOnly`] (via
/// [`edit::editability_note`]); an editable cell — or a blank cell with no
/// authored record (Excel authors into it from empty) — becomes
/// [`CellEditTarget::Editable`]. Pure read of [`WorkspaceState`].
fn resolve_cell_edit_target(ws: &WorkspaceState, row: u32, col: u32) -> Option<CellEditTarget> {
    let grid = active_grid(ws)?;
    let grid_id = grid.grid_node_id.clone();
    let editability = grid
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.col == col)
        .and_then(|cell| cell.authored.as_ref())
        .map(|authored| authored.editability.clone());
    match editability.as_ref().and_then(edit::editability_note) {
        Some(reason) => Some(CellEditTarget::ReadOnly { reason }),
        None => Some(CellEditTarget::Editable { grid: grid_id }),
    }
}

/// The Sheet stage's scoped stylesheet — Strand `--dna-*` tokens only. The
/// canvas fills a POSITIONED viewport wrapper (`.dna-sheet__viewport`) that is the
/// overlay editor's absolute-positioning ancestor; the canvas's CSS box is the
/// viewport size (the redraw effect sizes the device-px backing store off
/// `clientWidth`/`clientHeight`). The overlay editor is absolutely positioned at
/// the selected cell's rect, in the SAME coordinate space the canvas draws with.
/// The debug readout is visually hidden (off-screen, not `display:none`) so it
/// stays queryable by `data-testid`/`data-*` for the S3.11 browser test.
pub const SHEET_CSS: &str = "\
.dna-sheet{display:flex;flex-direction:column;gap:var(--dna-gap-3);padding:var(--dna-gap-4);color:var(--dna-ink);height:100%;min-height:0}
.dna-sheet__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-sheet__viewport{position:relative;flex:1 1 auto;min-height:0;overflow:hidden}
.dna-sheet__canvas{position:absolute;inset:0;width:100%;height:100%;display:block}
.dna-sheet__cell-editor{position:absolute;z-index:5;box-sizing:border-box}
.dna-sheet__cell-editor--readonly{background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-2) var(--dna-gap-3)}
.dna-sheet__cell-readonly{margin:0;color:var(--dna-ink-3);font-style:italic;font-size:12px;white-space:nowrap}
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

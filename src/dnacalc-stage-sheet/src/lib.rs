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
//! **S3.1 (this scaffold)** stands up the [`dnacalc_shell::StageSurface`] under
//! [`dnacalc_shell::StageId::Sheet`] and renders honestly from host truth: an
//! explicit honest-empty card when the workspace carries no grid, else a
//! placeholder readout of the active grid's real extent + windowed cell count
//! (the Canvas2D renderer replaces this placeholder in S3.5). It is never
//! rendered blank.

use leptos::prelude::*;

use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::{GridProjection, WorkspaceState};

pub mod geometry;
pub mod render_plan;

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
        // The one host-truth read the stage makes. `ReadSignal` is `Copy`, so the
        // closure re-derives on every workspace change — the grid view is never a
        // copy that can drift from host truth.
        let workspace = ctx.workspace;
        let view = view! {
            <style>{SHEET_CSS}</style>
            <section
                class="dna-sheet"
                data-stage="sheet"
                data-testid="sheet-root"
                data-dna-density="working"
            >
                {move || workspace.with(render_sheet)}
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

/// Render the Sheet body from workspace truth: an honest-empty card when there
/// is no active grid, else the S3.1 placeholder readout of the grid's REAL
/// extent + windowed cell count (replaced by the Canvas2D renderer in S3.5).
/// Never blank.
fn render_sheet(ws: &WorkspaceState) -> AnyView {
    let Some(grid) = active_grid(ws) else {
        return view! {
            <p class="dna-sheet__empty" data-testid="sheet-empty">
                "No grid in this workbook."
            </p>
        }
        .into_any();
    };

    // Honest placeholder: the real grid extent + how many computed cells the
    // host windowed into this projection. The Canvas2D renderer (S3.5) draws
    // these cells; until then the readout proves render-from-host-truth.
    let grid_id = grid.grid_node_id.to_string();
    let extent = format!("{}\u{00d7}{}", grid.max_rows, grid.max_cols);
    let windowed = grid.cells.len();
    view! {
        <div class="dna-sheet__placeholder" data-testid="sheet-grid-placeholder">
            <span class="dna-sheet__grid-id" data-testid="sheet-grid-id">{grid_id}</span>
            <span class="dna-sheet__extent" data-testid="sheet-grid-extent">{extent}</span>
            <span
                class="dna-sheet__windowed"
                data-testid="sheet-grid-windowed"
                data-cell-count=windowed.to_string()
            >
                {format!("{windowed} cells in window")}
            </span>
        </div>
    }
    .into_any()
}

/// The Sheet stage's scoped stylesheet — Strand `--dna-*` tokens only.
pub const SHEET_CSS: &str = "\
.dna-sheet{display:flex;flex-direction:column;gap:var(--dna-gap-3);padding:var(--dna-gap-4);color:var(--dna-ink);height:100%;min-height:0}
.dna-sheet__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-sheet__placeholder{display:flex;gap:var(--dna-gap-3);align-items:baseline;flex-wrap:wrap;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px}
.dna-sheet__grid-id{color:var(--dna-ink-2)}
.dna-sheet__extent{color:var(--dna-value-ink);font-weight:600}
.dna-sheet__windowed{color:var(--dna-ink-3)}
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

//! WORKBOOK — the strict-Excel grid lens shell.
//!
//! The K track (route-map §C, §E.4) builds the Excel workbook experience —
//! grid, cell edit loop, formula bar + name box, sheet tabs, defined-names
//! manager — on top of the shared [`crate::grid_canvas`] component. This file
//! is the **shell** minted by K1a: it establishes the `WorkbookLens` skin and
//! renders the workbook's grid-backed sheet nodes through
//! `grid_canvas::grid_surface`, so the extraction has a second live consumer
//! and later K beads have a home to grow into.
//!
//! K1a was a pure extraction (unchanged behavior). K1b (§C.2, §E.4) layers the
//! two mandatory grid upgrades on top through the shared `grid_canvas`
//! component: interest coalescing and authored-aware cell rendering. K1b adds
//! no UI of its own here — no editing (K2), no tabs (K4), no formula bar (K3)
//! — so `show_formulas` stays `false` until K3 gives the workbook shell a
//! toggle to drive.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest, SkinState,
    WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const WORKBOOK_ID: SkinId = SkinId::new("workbook");

/// Workbook lens state. K1a carries no toggles of its own; later K beads
/// (show-formulas, manual mode, audit toggle) extend this in place.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbookState;

impl SkinState for WorkbookState {
    fn schema_version() -> u32 {
        1
    }
}

/// The strict-Excel workbook lens. K1a ships the grid-rendering shell over the
/// shared [`crate::grid_canvas`] component; K1b–K8 layer the edit loop, formula
/// bar, tabs, and managers on top.
#[derive(Default)]
pub struct WorkbookLens;

impl WorkbookLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for WorkbookLens {
    type State = WorkbookState;

    fn id(&self) -> SkinId {
        WORKBOOK_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Workbook",
            description: "Strict-Excel grid: windowed cells over the shared grid canvas.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: false,
            renders_arrays_inline: true,
            renders_table_values: true,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        crate::spine_widgets::stamp_active_lens(cx.shared, WORKBOOK_ID, cx.slot);
        SkinHandle::new(view! { <WorkbookView cx=cx /> }.into_any())
    }
}

/// Render every grid-backed sheet node through the shared grid canvas. Each
/// surface is built once (untracked) so its scroll box persists across
/// projection updates, exactly as the SheetLens read path does.
#[component]
fn WorkbookView(cx: SkinContext<WorkbookState>) -> impl IntoView {
    let workspace = cx.workspace;
    let dispatch: Arc<dyn dnatreecalc_skin_framework::Dispatcher> = cx.dispatch.clone();

    // K1b ships no show-formulas toggle for the workbook shell (K3 owns the
    // formula bar); the grid still supports the mode end-to-end via the
    // shared component, so wiring up a toggle later is additive.
    let show_formulas = Signal::from(false);
    let grid_surfaces = workspace
        .get_untracked()
        .grids
        .keys()
        .cloned()
        .map(|grid_id| {
            crate::grid_canvas::grid_surface(grid_id, workspace, dispatch.clone(), show_formulas)
        })
        .collect::<Vec<_>>();

    let css = format!("{}\n{WORKBOOK_CSS}", crate::grid_canvas::GRID_CANVAS_CSS);

    view! {
        <style>{css}</style>
        <section class="dtc-workbook" aria-label="Workbook">
            {grid_surfaces}
        </section>
    }
}

const WORKBOOK_CSS: &str = r#"
.dtc-workbook {
  display: flex; flex-direction: column; height: 100%; min-height: 0;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  padding: 8px 12px; overflow: auto;
}
"#;

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
//! component: interest coalescing and authored-aware cell rendering. K2
//! (§C.3, §E.4) adds the in-grid cell edit loop entirely inside
//! `grid_canvas::grid_surface` — this shell only needs to concatenate the
//! shared editor's [`dnatreecalc_skin_framework::CELL_ENTRY_CSS`] alongside
//! `GRID_CANVAS_CSS` so the mounted `CellEntryEditor`/`EntryDiagnostics`
//! render styled. K4 (§C.5) adds the Excel-style sheet-tab strip below the
//! active sheet's grid — switch / add / rename / delete / move, dispatching the
//! H7 sheet-lifecycle intents. The formula bar (K3) is still to come;
//! `show_formulas` stays `false` until K3 gives the shell a toggle to drive.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    NodeId, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest,
    SkinState, WorkspaceIntent, WorkspaceSkin,
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

/// Render the active sheet through the shared grid canvas, with an Excel-style
/// tab strip (K4) below it. The tab strip is reactive over the workbook's
/// `sheets` projection; each tab switches the active sheet, and the strip
/// dispatches the H7 sheet-lifecycle intents (add / rename / delete / move) —
/// the `WorkbookHostDispatcher` republishes the full snapshot on each accepted
/// mutation, so the strip and the grid stay in step.
///
/// Only the active sheet's surface is mounted; it is rebuilt only when the
/// active sheet changes (not on every edit), so scroll persists within a sheet
/// while cell values keep updating through the shared component.
#[component]
fn WorkbookView(cx: SkinContext<WorkbookState>) -> impl IntoView {
    let workspace = cx.workspace;
    let dispatch: Arc<dyn dnatreecalc_skin_framework::Dispatcher> = cx.dispatch.clone();

    // K1b ships no show-formulas toggle for the workbook shell (K3 owns the
    // formula bar); the grid still supports the mode end-to-end via the
    // shared component, so wiring up a toggle later is additive.
    let show_formulas = Signal::from(false);

    let active_sheet: RwSignal<Option<NodeId>> = RwSignal::new(
        workspace
            .get_untracked()
            .sheets
            .first()
            .map(|sheet| sheet.grid_node_id.clone()),
    );
    // Which tab (if any) is being renamed inline, and the shared edit buffer.
    let renaming: RwSignal<Option<NodeId>> = RwSignal::new(None);
    let rename_buffer = RwSignal::new(String::new());

    // Keep the active sheet valid: snap to the first sheet whenever the current
    // one vanishes (a delete) or none is selected yet.
    Effect::new(move |_| {
        let sheets = workspace.get().sheets;
        let still_valid = active_sheet
            .get_untracked()
            .as_ref()
            .is_some_and(|gid| sheets.iter().any(|sheet| &sheet.grid_node_id == gid));
        if !still_valid {
            active_sheet.set(sheets.first().map(|sheet| sheet.grid_node_id.clone()));
        }
    });

    let grid_dispatch = dispatch.clone();
    let grid_area = move || {
        active_sheet.get().and_then(|gid| {
            workspace.get_untracked().grids.contains_key(&gid).then(|| {
                crate::grid_canvas::grid_surface(
                    gid,
                    workspace,
                    grid_dispatch.clone(),
                    show_formulas,
                )
            })
        })
    };

    let tab_dispatch = dispatch.clone();
    let tabs = move || {
        let sheets = workspace.get().sheets;
        let active = active_sheet.get();
        let renaming_now = renaming.get();
        let count = sheets.len();
        sheets
            .into_iter()
            .enumerate()
            .map(|(index, sheet)| {
                let gid = sheet.grid_node_id.clone();
                let name = sheet.display_name.clone();
                let is_active = active.as_ref() == Some(&gid);

                if renaming_now.as_ref() == Some(&gid) {
                    let commit_dispatch = tab_dispatch.clone();
                    let commit_gid = gid.clone();
                    let commit = move || {
                        let new_name = rename_buffer.get_untracked().trim().to_string();
                        if !new_name.is_empty() {
                            commit_dispatch.dispatch(WorkspaceIntent::RenameSheet {
                                grid: commit_gid.clone(),
                                new_name,
                            });
                        }
                        renaming.set(None);
                    };
                    let commit_on_key = commit.clone();
                    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
                        ev.stop_propagation();
                        match ev.key().as_str() {
                            "Enter" => {
                                ev.prevent_default();
                                commit_on_key();
                            }
                            "Escape" => {
                                ev.prevent_default();
                                renaming.set(None);
                            }
                            _ => {}
                        }
                    };
                    return view! {
                        <input
                            class="dtc-workbook__tab-rename"
                            type="text"
                            prop:value=move || rename_buffer.get()
                            on:input=move |ev| rename_buffer.set(event_target_value(&ev))
                            on:keydown=on_keydown
                            on:blur=move |_| commit()
                            aria-label="Rename sheet"
                        />
                    }
                    .into_any();
                }

                let select_dispatch = tab_dispatch.clone();
                let select_gid = gid.clone();
                let on_click = move |_| {
                    active_sheet.set(Some(select_gid.clone()));
                    select_dispatch.dispatch(WorkspaceIntent::SelectNode(Some(select_gid.clone())));
                };
                let rename_gid = gid.clone();
                let rename_name = name.clone();
                let on_dblclick = move |_| {
                    rename_buffer.set(rename_name.clone());
                    renaming.set(Some(rename_gid.clone()));
                };

                let move_left = (index > 0).then(|| {
                    let move_dispatch = tab_dispatch.clone();
                    let move_gid = gid.clone();
                    view! {
                        <button
                            class="dtc-workbook__tab-move"
                            title="Move sheet left"
                            on:click=move |ev| {
                                ev.stop_propagation();
                                move_dispatch.dispatch(WorkspaceIntent::MoveSheet {
                                    grid: move_gid.clone(),
                                    new_position: index as u32 - 1,
                                });
                            }
                        >"‹"</button>
                    }
                });
                let move_right = (index + 1 < count).then(|| {
                    let move_dispatch = tab_dispatch.clone();
                    let move_gid = gid.clone();
                    view! {
                        <button
                            class="dtc-workbook__tab-move"
                            title="Move sheet right"
                            on:click=move |ev| {
                                ev.stop_propagation();
                                move_dispatch.dispatch(WorkspaceIntent::MoveSheet {
                                    grid: move_gid.clone(),
                                    new_position: index as u32 + 1,
                                });
                            }
                        >"›"</button>
                    }
                });
                // A workbook keeps at least one sheet, so the close affordance
                // only appears when a delete would leave one behind.
                let delete = (count > 1).then(|| {
                    let delete_dispatch = tab_dispatch.clone();
                    let delete_gid = gid.clone();
                    view! {
                        <button
                            class="dtc-workbook__tab-close"
                            title="Delete sheet"
                            on:click=move |ev| {
                                ev.stop_propagation();
                                delete_dispatch
                                    .dispatch(WorkspaceIntent::DeleteSheet { grid: delete_gid.clone() });
                            }
                        >"×"</button>
                    }
                });

                let class = if is_active {
                    "dtc-workbook__tab dtc-workbook__tab--active"
                } else {
                    "dtc-workbook__tab"
                };
                view! {
                    <div
                        class=class
                        role="tab"
                        aria-selected=if is_active { "true" } else { "false" }
                        on:click=on_click
                        on:dblclick=on_dblclick
                    >
                        {move_left}
                        <span class="dtc-workbook__tab-name">{name}</span>
                        {move_right}
                        {delete}
                    </div>
                }
                .into_any()
            })
            .collect_view()
    };

    let add_dispatch = dispatch.clone();
    let on_add = move |_| {
        add_dispatch.dispatch(WorkspaceIntent::AddSheet { name: None });
        // The dispatcher republishes synchronously, so the new sheet is already
        // in the projection — open it.
        if let Some(last) = workspace.get_untracked().sheets.last() {
            active_sheet.set(Some(last.grid_node_id.clone()));
        }
    };

    let css = format!(
        "{}\n{}\n{WORKBOOK_CSS}",
        crate::grid_canvas::GRID_CANVAS_CSS,
        dnatreecalc_skin_framework::CELL_ENTRY_CSS,
    );

    view! {
        <style>{css}</style>
        <section class="dtc-workbook" aria-label="Workbook">
            <div class="dtc-workbook__grid">{grid_area}</div>
            <div class="dtc-workbook__tabs" role="tablist">
                {tabs}
                <button class="dtc-workbook__tab-add" title="Add sheet" on:click=on_add>"+"</button>
            </div>
        </section>
    }
}

const WORKBOOK_CSS: &str = r#"
.dtc-workbook {
  display: flex; flex-direction: column; height: 100%; min-height: 0;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
}
.dtc-workbook__grid {
  flex: 1 1 auto; min-height: 0; overflow: auto; padding: 8px 12px;
}
.dtc-workbook__tabs {
  flex: 0 0 auto; display: flex; align-items: stretch; gap: 2px;
  padding: 3px 6px 0; border-top: 1px solid var(--dtc-border, #d0d0d0);
  background: var(--dtc-surface-muted, #f3f3f3); overflow-x: auto;
}
.dtc-workbook__tab {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 4px 8px; cursor: pointer; user-select: none;
  border: 1px solid var(--dtc-border, #d0d0d0); border-bottom: none;
  border-radius: 4px 4px 0 0; background: var(--dtc-surface-muted, #eaeaea);
  color: var(--dtc-text-muted, #555); font-size: 12px; white-space: nowrap;
}
.dtc-workbook__tab--active {
  background: var(--dtc-surface, #fff); color: var(--dtc-text, #111);
  font-weight: 600;
}
.dtc-workbook__tab-name { padding: 0 2px; }
.dtc-workbook__tab-move, .dtc-workbook__tab-close, .dtc-workbook__tab-add {
  border: none; background: transparent; cursor: pointer;
  color: var(--dtc-text-muted, #888); font-size: 12px; line-height: 1;
  padding: 2px 4px; border-radius: 3px;
}
.dtc-workbook__tab-move:hover, .dtc-workbook__tab-close:hover,
.dtc-workbook__tab-add:hover {
  background: var(--dtc-border, #ddd); color: var(--dtc-text, #111);
}
.dtc-workbook__tab-add { align-self: center; font-size: 15px; }
.dtc-workbook__tab-rename {
  padding: 4px 6px; margin: 0; font-size: 12px; width: 8ch;
  border: 1px solid var(--dtc-accent, #2563eb); border-radius: 4px 4px 0 0;
}
"#;

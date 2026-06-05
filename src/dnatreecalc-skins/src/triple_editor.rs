use dnatreecalc_skin_framework::{
    NodeId, NodeView, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId,
    SkinManifest, SkinState, WorkspaceIntent, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::render_value;

/// Stable skin id; used as the meta-namespace path component once
/// persistence lands (dtc-osq.7).
pub const TRIPLE_EDITOR_ID: SkinId = SkinId::new("triple-editor");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripleEditorState {
    pub nav_rail_width: u32,
    pub value_pane_height: u32,
}

impl Default for TripleEditorState {
    fn default() -> Self {
        Self {
            nav_rail_width: 260,
            value_pane_height: 240,
        }
    }
}

impl SkinState for TripleEditorState {
    fn schema_version() -> u32 {
        1
    }
}

/// The TripleEditor skin — Editor-category, three-pane layout.
///
/// Walking-skeleton scope: nav rail lists workspace nodes (read from the
/// workspace signal); clicking a row dispatches `SelectNode`; the formula
/// pane shows the selected node's content text verbatim; the value pane
/// shows the projected computed value. Inline editing of the formula
/// content lands with W003 (`UX-FE-001`/`UX-FE-003`); full nav-rail
/// virtualization, drag-reorder, context menu, breadcrumb arrive with
/// `UX-TR`/`UX-SH` in W003.
#[derive(Default)]
pub struct TripleEditor;

impl TripleEditor {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for TripleEditor {
    type State = TripleEditorState;

    fn id(&self) -> SkinId {
        TRIPLE_EDITOR_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Triple editor",
            description: "Nav rail + formula pane + value pane; the default editing skin.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: false,
            renders_arrays_inline: false,
            renders_table_values: false,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        let view = view! { <TripleEditorView cx=cx /> }.into_any();
        SkinHandle::new(view)
    }
}

#[component]
fn TripleEditorView(cx: SkinContext<TripleEditorState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let dispatch = cx.dispatch.clone();

    let rows = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.node_order
                .iter()
                .filter_map(|id| ws.node(id).map(row_for))
                .collect::<Vec<_>>()
        })
    });

    let selected_snapshot = Memo::new(move |_| {
        let selected = selection.with(|s| s.primary.clone());
        workspace.with(|ws| selected.as_ref().and_then(|id| ws.node(id).cloned()))
    });

    view! {
        <section class="dtc-triple-editor">
            <aside class="dtc-triple-editor__nav" aria-label="Workspace navigation">
                <div class="dtc-section-label">"Nodes"</div>
                {move || {
                    let dispatch = dispatch.clone();
                    rows.with(|rs| {
                        rs.iter()
                            .map(|row| nav_row(row.clone(), selection, dispatch.clone()))
                            .collect::<Vec<_>>()
                    })
                }}
            </aside>
            <section class="dtc-triple-editor__editor">
                <div class="dtc-section-label">"Formula"</div>
                <div class="dtc-formula-display">
                    {move || selected_snapshot.with(|node| match node {
                        Some(node) if node.content_text.is_empty() => "(empty)".to_string(),
                        Some(node) => node.content_text.clone(),
                        None => "Select a node from the nav rail to view its formula.".to_string(),
                    })}
                </div>
            </section>
            <section class="dtc-triple-editor__value">
                <div class="dtc-section-label">"Value"</div>
                {move || selected_snapshot.with(|node| match node {
                    Some(node) => render_value(&node.computed_value).into_any(),
                    None => view! { <div class="dtc-value-display">"—"</div> }.into_any(),
                })}
            </section>
        </section>
    }
}

#[derive(Clone, PartialEq, Eq)]
struct NavRow {
    id: NodeId,
    label: String,
    depth: u32,
    is_meta: bool,
}

fn row_for(node: &NodeView) -> NavRow {
    NavRow {
        id: node.id.clone(),
        label: node.display_name.clone(),
        depth: node.depth,
        is_meta: node.is_meta,
    }
}

fn nav_row(
    row: NavRow,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: std::sync::Arc<dyn dnatreecalc_skin_framework::Dispatcher>,
) -> impl IntoView {
    let id_for_selection = row.id.clone();
    let id_for_click = row.id.clone();
    let is_selected =
        Memo::new(move |_| selection.with(|s| s.primary.as_ref() == Some(&id_for_selection)));
    let indent = format!("padding-left: {}rem;", 0.25 + (row.depth as f32) * 0.75);

    view! {
        <div
            class="dtc-tree-row"
            class:dtc-tree-row--selected=move || is_selected.get()
            class:dtc-tree-row--meta=row.is_meta
            style=indent
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            <span>{row.label}</span>
        </div>
    }
}

use dnatreecalc_skin_framework::{
    NodeId, NodeView, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId,
    SkinManifest, SkinState, WorkspaceIntent, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::render_value;

pub const FORMULA_TREE_ID: SkinId = SkinId::new("formula-tree");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaTreeState {
    pub editor_rows: u32,
}

impl Default for FormulaTreeState {
    fn default() -> Self {
        Self { editor_rows: 4 }
    }
}

impl SkinState for FormulaTreeState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct FormulaTree;

impl FormulaTree {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for FormulaTree {
    type State = FormulaTreeState;

    fn id(&self) -> SkinId {
        FORMULA_TREE_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Formula tree",
            description: "Tree navigation with direct content entry and recalculation.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: true,
            supports_meta_node_display: true,
            renders_arrays_inline: true,
            renders_table_values: false,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        SkinHandle::new(view! { <FormulaTreeView cx=cx /> }.into_any())
    }
}

#[component]
fn FormulaTreeView(cx: SkinContext<FormulaTreeState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let dispatch = cx.dispatch.clone();
    let editor_text = RwSignal::new(String::new());

    let rows = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.node_order
                .iter()
                .filter_map(|id| ws.node(id).map(row_for))
                .collect::<Vec<_>>()
        })
    });

    let selected = Memo::new(move |_| {
        let selected = selection.with(|s| s.primary.clone());
        workspace.with(|ws| selected.as_ref().and_then(|id| ws.node(id).cloned()))
    });

    Effect::new(move |_| {
        if let Some(node) = selected.get() {
            editor_text.set(node.content_text);
        }
    });

    let apply_dispatch = dispatch.clone();
    let recalc_dispatch = dispatch.clone();

    view! {
        <section class="dtc-formula-tree">
            <aside class="dtc-formula-tree__nav" aria-label="Formula tree nodes">
                {move || {
                    let dispatch = dispatch.clone();
                    rows.with(|rs| {
                        rs.iter()
                            .map(|row| tree_row(row.clone(), selection, dispatch.clone()))
                            .collect::<Vec<_>>()
                    })
                }}
            </aside>
            <section class="dtc-formula-tree__workbench">
                <div class="dtc-section-label">"Content"</div>
                <textarea
                    class="dtc-formula-tree__input"
                    prop:value=move || editor_text.get()
                    on:input=move |ev| editor_text.set(event_target_value(&ev))
                />
                <div class="dtc-formula-tree__commands">
                    <button
                        type="button"
                        on:click=move |_| {
                            if let Some(node) = selected.get_untracked() {
                                apply_dispatch.dispatch(WorkspaceIntent::EditContent {
                                    node: node.id,
                                    content: editor_text.get_untracked(),
                                });
                            }
                        }
                    >
                        "Apply"
                    </button>
                    <button
                        type="button"
                        on:click=move |_| {
                            recalc_dispatch.dispatch(WorkspaceIntent::Recalculate);
                        }
                    >
                        "Recalculate"
                    </button>
                </div>
                <div class="dtc-section-label">"Result"</div>
                {move || selected.with(|node| match node {
                    Some(node) => render_value(&node.computed_value),
                    None => view! { <div class="dtc-value-display">"-"</div> }.into_any(),
                })}
            </section>
        </section>
    }
}

#[derive(Clone, PartialEq, Eq)]
struct TreeRow {
    id: NodeId,
    label: String,
    depth: u32,
    is_meta: bool,
}

fn row_for(node: &NodeView) -> TreeRow {
    TreeRow {
        id: node.id.clone(),
        label: node.display_name.clone(),
        depth: node.depth,
        is_meta: node.is_meta,
    }
}

fn tree_row(
    row: TreeRow,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: std::sync::Arc<dyn dnatreecalc_skin_framework::Dispatcher>,
) -> impl IntoView {
    let id_for_selection = row.id.clone();
    let id_for_click = row.id.clone();
    let is_selected =
        Memo::new(move |_| selection.with(|s| s.primary.as_ref() == Some(&id_for_selection)));
    let indent = format!("padding-left: {}rem;", 0.25 + (row.depth as f32) * 0.75);

    view! {
        <button
            type="button"
            class="dtc-tree-row"
            class:dtc-tree-row--selected=move || is_selected.get()
            class:dtc-tree-row--meta=row.is_meta
            style=indent
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            {row.label}
        </button>
    }
}

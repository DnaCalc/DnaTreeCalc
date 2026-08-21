use dnatreecalc_skin_framework::{
    NodeId, NodeKey, NodeView, SelectableItemA11y, SkinCapabilities, SkinCategory, SkinContext,
    SkinHandle, SkinId, SkinManifest, SkinState, WorkspaceIntent, WorkspaceRecalcMode,
    WorkspaceSkin, tree_a11y,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::node_management::NodeManagementPanel;
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
            allowed_slots: None,
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
    let shared = cx.shared;
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
    let tree_attrs = Memo::new(move |_| {
        rows.with(|rs| {
            let active_key = selection.with(|s| {
                s.primary
                    .as_ref()
                    .and_then(|selected| rs.iter().find(|row| &row.id == selected))
                    .map(|row| row.key.clone())
            });
            tree_a11y(
                "Formula tree nodes",
                "dtc-formula-node",
                active_key.as_ref(),
            )
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
    let nav_dispatch = dispatch.clone();
    let management_dispatch = dispatch.clone();

    view! {
        <section class="dtc-formula-tree">
            <aside class="dtc-formula-tree__nav" aria-label="Formula tree nodes">
                <div
                    role=move || tree_attrs.get().role
                    aria-label=move || tree_attrs.get().aria_label
                    attr:aria-activedescendant=move || tree_attrs.get().aria_activedescendant
                >
                {move || {
                    let dispatch = nav_dispatch.clone();
                    rows.with(|rs| {
                        let setsize = rs.len();
                        let has_visible_selection = selection.with(|s| {
                            s.primary
                                .as_ref()
                                .is_some_and(|selected| rs.iter().any(|row| &row.id == selected))
                        });
                        rs.iter()
                            .enumerate()
                            .map(|(index, row)| {
                                tree_row(
                                    row.clone(),
                                    index + 1,
                                    setsize,
                                    !has_visible_selection && index == 0,
                                    selection,
                                    dispatch.clone(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                }}
                </div>
            </aside>
            <section class="dtc-formula-tree__workbench">
                <NodeManagementPanel
                    workspace=workspace
                    selection=selection
                    dispatch=management_dispatch.clone()
                />
                <div class="dtc-section-label">"Content"</div>
                <textarea
                    class="dtc-formula-tree__input"
                    autocapitalize="off"
                    spellcheck="false"
                    prop:value=move || editor_text.get()
                    on:input=move |ev| editor_text.set(event_target_value(&ev))
                />
                <div class="dtc-formula-tree__commands">
                    <button
                        type="button"
                        on:click=move |_| {
                            if let Some(node) = selected.get_untracked() {
                                let content = editor_text.get_untracked();
                                match shared.get_untracked().recalc_mode {
                                    WorkspaceRecalcMode::Auto => {
                                        apply_dispatch.dispatch(WorkspaceIntent::EditContent {
                                            node: node.id,
                                            content,
                                        });
                                        shared.update(|state| {
                                            state.manual_recalc_pending = false;
                                        });
                                    }
                                    WorkspaceRecalcMode::Manual => {
                                        apply_dispatch.dispatch(
                                            WorkspaceIntent::EditContentDeferred {
                                                node: node.id,
                                                content,
                                            },
                                        );
                                        shared.update(|state| {
                                            state.manual_recalc_pending = true;
                                        });
                                    }
                                }
                            }
                        }
                    >
                        "Apply"
                    </button>
                    <button
                        type="button"
                        on:click=move |_| {
                            recalc_dispatch.dispatch(WorkspaceIntent::Recalculate);
                            shared.update(|state| {
                                state.manual_recalc_pending = false;
                            });
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
    key: NodeKey,
    label: String,
    depth: u32,
    is_meta: bool,
}

fn row_for(node: &NodeView) -> TreeRow {
    TreeRow {
        id: node.id.clone(),
        key: node.key.clone(),
        label: node.display_name.clone(),
        depth: node.depth,
        is_meta: node.is_meta,
    }
}

fn tree_row(
    row: TreeRow,
    posinset: usize,
    setsize: usize,
    fallback_focusable: bool,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: std::sync::Arc<dyn dnatreecalc_skin_framework::Dispatcher>,
) -> impl IntoView {
    let id_for_selection = row.id.clone();
    let id_for_focus = row.id.clone();
    let id_for_click = row.id.clone();
    let is_selected =
        Memo::new(move |_| selection.with(|s| s.primary.as_ref() == Some(&id_for_selection)));
    let a11y = Memo::new(move |_| {
        selection.with(|s| {
            let selected = s.primary.as_ref() == Some(&id_for_focus);
            let focusable = selected || fallback_focusable;
            SelectableItemA11y::for_tree_item(
                "dtc-formula-node",
                &row.key,
                selected,
                focusable,
                row.depth + 1,
                posinset,
                setsize,
            )
        })
    });
    let indent = format!("padding-left: {}rem;", 0.25 + (row.depth as f32) * 0.75);

    view! {
        <button
            type="button"
            id=move || a11y.get().id
            class="dtc-tree-row"
            class:dtc-tree-row--selected=move || is_selected.get()
            class:dtc-tree-row--meta=row.is_meta
            style=indent
            role=move || a11y.get().role
            attr:aria-selected=move || a11y.get().aria_selected
            attr:aria-level=move || a11y.get().aria_level
            attr:aria-posinset=move || a11y.get().aria_posinset
            attr:aria-setsize=move || a11y.get().aria_setsize
            tabindex=move || a11y.get().tabindex
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            {row.label}
        </button>
    }
}

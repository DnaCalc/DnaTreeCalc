use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, NodeId, SelectionState, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;

#[component]
pub(crate) fn NodeManagementPanel(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let new_symbol = RwSignal::new(String::new());
    let new_content = RwSignal::new(String::new());
    let rename_symbol = RwSignal::new(String::new());

    let selected = Memo::new(move |_| {
        let selected = selection.with(|state| state.primary.clone());
        workspace.with(|state| selected.as_ref().and_then(|id| state.node(id).cloned()))
    });

    Effect::new(move |_| {
        rename_symbol.set(
            selected
                .get()
                .map(|node| node.display_name)
                .unwrap_or_default(),
        );
    });

    let add_root_dispatch = dispatch.clone();
    let add_child_dispatch = dispatch.clone();
    let rename_dispatch = dispatch.clone();
    let move_root_dispatch = dispatch.clone();
    let move_up_dispatch = dispatch.clone();
    let move_down_dispatch = dispatch.clone();
    let delete_dispatch = dispatch.clone();

    view! {
        <section class="dtc-node-management" aria-label="Node management">
            <div class="dtc-node-management__row">
                <input
                    class="dtc-node-management__symbol"
                    placeholder="Node"
                    prop:value=move || new_symbol.get()
                    on:input=move |ev| new_symbol.set(event_target_value(&ev))
                />
                <input
                    class="dtc-node-management__content"
                    placeholder="Content"
                    prop:value=move || new_content.get()
                    on:input=move |ev| new_content.set(event_target_value(&ev))
                />
            </div>
            <div class="dtc-node-management__commands">
                <button
                    type="button"
                    title="Add root node"
                    on:click=move |_| {
                        let symbol = new_symbol.get_untracked();
                        if !symbol.trim().is_empty() {
                            add_root_dispatch.dispatch(WorkspaceIntent::AddNode {
                                parent: None,
                                symbol: symbol.trim().to_string(),
                                content: new_content.get_untracked(),
                            });
                            new_symbol.set(String::new());
                            new_content.set(String::new());
                        }
                    }
                >
                    "Add root"
                </button>
                <button
                    type="button"
                    title="Add child under the selected node"
                    on:click=move |_| {
                        let symbol = new_symbol.get_untracked();
                        let Some(parent) = selection.get_untracked().primary else {
                            return;
                        };
                        if !symbol.trim().is_empty() {
                            add_child_dispatch.dispatch(WorkspaceIntent::AddNode {
                                parent: Some(parent),
                                symbol: symbol.trim().to_string(),
                                content: new_content.get_untracked(),
                            });
                            new_symbol.set(String::new());
                            new_content.set(String::new());
                        }
                    }
                >
                    "Add child"
                </button>
            </div>
            <div class="dtc-node-management__row">
                <input
                    class="dtc-node-management__symbol"
                    placeholder="Rename"
                    prop:value=move || rename_symbol.get()
                    on:input=move |ev| rename_symbol.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    title="Rename selected node"
                    on:click=move |_| {
                        let Some(node) = selection.get_untracked().primary else {
                            return;
                        };
                        let symbol = rename_symbol.get_untracked();
                        if !symbol.trim().is_empty() {
                            rename_dispatch.dispatch(WorkspaceIntent::RenameNode {
                                node,
                                new_symbol: symbol.trim().to_string(),
                            });
                        }
                    }
                >
                    "Rename"
                </button>
            </div>
            <div class="dtc-node-management__commands">
                <button
                    type="button"
                    title="Move selected node to workspace root"
                    on:click=move |_| {
                        if let Some(node) = selection.get_untracked().primary {
                            move_root_dispatch.dispatch(WorkspaceIntent::MoveNode {
                                node,
                                new_parent: None,
                                new_index: None,
                            });
                        }
                    }
                >
                    "To root"
                </button>
                <button
                    type="button"
                    title="Move selected node earlier among siblings"
                    on:click=move |_| {
                        if let Some((node, index)) =
                            selected_sibling_index(&workspace.get_untracked(), &selection.get_untracked())
                        {
                            if index > 0 {
                                move_up_dispatch.dispatch(WorkspaceIntent::ReorderNode {
                                    node,
                                    new_index: index - 1,
                                });
                            }
                        }
                    }
                >
                    "Up"
                </button>
                <button
                    type="button"
                    title="Move selected node later among siblings"
                    on:click=move |_| {
                        if let Some((node, index)) =
                            selected_sibling_index(&workspace.get_untracked(), &selection.get_untracked())
                        {
                            move_down_dispatch.dispatch(WorkspaceIntent::ReorderNode {
                                node,
                                new_index: index + 1,
                            });
                        }
                    }
                >
                    "Down"
                </button>
                <button
                    type="button"
                    title="Delete selected node"
                    on:click=move |_| {
                        if let Some(node) = selection.get_untracked().primary {
                            delete_dispatch.dispatch(WorkspaceIntent::DeleteNode { node });
                        }
                    }
                >
                    "Delete"
                </button>
            </div>
        </section>
    }
}

fn selected_sibling_index(
    workspace: &WorkspaceState,
    selection: &SelectionState,
) -> Option<(NodeId, usize)> {
    let selected = selection.primary.as_ref()?;
    let node = workspace.node(selected)?;
    let siblings = node
        .parent
        .as_ref()
        .and_then(|parent| workspace.node(parent).map(|node| node.children.as_slice()))
        .unwrap_or(workspace.root_paths.as_slice());
    siblings
        .iter()
        .position(|candidate| candidate == selected)
        .map(|index| (selected.clone(), index))
}

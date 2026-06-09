use std::sync::Arc;

use dnatreecalc_skin_framework::{
    CommandCatalogProjection, CommandIntentKindProjection, Dispatcher,
    InitialNodeContentProjection, NodeId, SelectionState, WorkspaceIntent, WorkspaceState,
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
    let selected_template = RwSignal::new(String::new());
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
    let command_catalog = Memo::new(move |_| {
        workspace.with(|workspace| selection.with(|selection| workspace.command_catalog(selection)))
    });

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
            <div class="dtc-node-management__row">
                <select
                    class="dtc-node-management__template"
                    aria-label="Initial content template"
                    prop:value=move || selected_template.get()
                    on:change=move |ev| selected_template.set(event_target_value(&ev))
                >
                    <option value="">"Custom content"</option>
                    {move || {
                        workspace.with(|state| {
                            state
                                .templates
                                .entries
                                .iter()
                                .map(|template| {
                                    let label = match template.preview_content.as_deref() {
                                        Some(preview) if !preview.is_empty() => {
                                            format!("{} ({preview})", template.name)
                                        }
                                        _ => template.name.clone(),
                                    };
                                    view! {
                                        <option value=template.template_id.clone()>{label}</option>
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                    }}
                </select>
            </div>
            <div class="dtc-node-management__hints" aria-label="Command shortcuts">
                {move || command_hints(&command_catalog.get()).into_iter().map(|hint| view! {
                    <span class=("dtc-command-hint--disabled", !hint.enabled)>
                        <span>{hint.title}</span>
                        {hint.binding.map(|binding| view! { <kbd>{binding}</kbd> })}
                    </span>
                }).collect::<Vec<_>>()}
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
                                initial: initial_content_for_selection(
                                    &workspace.get_untracked(),
                                    &selected_template.get_untracked(),
                                    new_content.get_untracked(),
                                ),
                                is_meta: false,
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
                                initial: initial_content_for_selection(
                                    &workspace.get_untracked(),
                                    &selected_template.get_untracked(),
                                    new_content.get_untracked(),
                                ),
                                is_meta: false,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandHint {
    title: &'static str,
    binding: Option<&'static str>,
    enabled: bool,
}

fn command_hints(catalog: &CommandCatalogProjection) -> Vec<CommandHint> {
    [
        CommandIntentKindProjection::AddNode,
        CommandIntentKindProjection::RenameNode,
        CommandIntentKindProjection::DeleteNode,
        CommandIntentKindProjection::Recalculate,
    ]
    .into_iter()
    .filter_map(|kind| {
        let command = catalog.get(kind)?;
        Some(CommandHint {
            title: command.title,
            binding: command.effective_binding,
            enabled: command.enabled,
        })
    })
    .collect()
}

fn initial_content_for_selection(
    workspace: &WorkspaceState,
    template_id: &str,
    literal_content: String,
) -> InitialNodeContentProjection {
    workspace
        .templates
        .entries
        .iter()
        .find(|template| template.template_id == template_id)
        .map(|template| template.initial.clone())
        .unwrap_or(InitialNodeContentProjection::Literal {
            content: literal_content,
        })
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

#[cfg(test)]
mod tests {
    use dnatreecalc_skin_framework::{
        CommandIntentKindProjection, CommandMetaProjection, TemplateManifestProjection,
        TemplateProjection,
    };

    use super::*;

    #[test]
    fn command_hints_read_projected_command_catalog() {
        let catalog = CommandCatalogProjection {
            entries: vec![
                CommandMetaProjection {
                    intent_kind: CommandIntentKindProjection::AddNode,
                    title: "Add Node",
                    shortcut: Some("A"),
                    effective_binding: Some("A"),
                    enabled: true,
                    disabled_reason: None,
                },
                CommandMetaProjection {
                    intent_kind: CommandIntentKindProjection::DeleteNode,
                    title: "Delete Node",
                    shortcut: Some("Delete"),
                    effective_binding: Some("Delete"),
                    enabled: false,
                    disabled_reason: Some("no node is selected".to_string()),
                },
            ],
        };

        let hints = command_hints(&catalog);
        assert_eq!(
            hints,
            vec![
                CommandHint {
                    title: "Add Node",
                    binding: Some("A"),
                    enabled: true,
                },
                CommandHint {
                    title: "Delete Node",
                    binding: Some("Delete"),
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn initial_content_for_selection_uses_projected_template_payload() {
        let workspace = WorkspaceState {
            templates: TemplateManifestProjection {
                entries: vec![TemplateProjection {
                    template_id: "starter".to_string(),
                    name: "Starter".to_string(),
                    description: None,
                    initial: InitialNodeContentProjection::TemplateBound {
                        template_id: "starter".to_string(),
                    },
                    preview_content: Some("=1+1".to_string()),
                    built_in: true,
                }],
            },
            ..WorkspaceState::default()
        };

        assert_eq!(
            initial_content_for_selection(&workspace, "starter", "7".to_string()),
            InitialNodeContentProjection::TemplateBound {
                template_id: "starter".to_string()
            }
        );
        assert_eq!(
            initial_content_for_selection(&workspace, "", "7".to_string()),
            InitialNodeContentProjection::Literal {
                content: "7".to_string()
            }
        );
    }
}

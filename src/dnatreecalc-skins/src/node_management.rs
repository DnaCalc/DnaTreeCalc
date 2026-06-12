use std::sync::Arc;

use dnatreecalc_skin_framework::{
    CommandCatalogProjection, CommandIntentKindProjection, Dispatcher,
    InitialNodeContentProjection, MutationImpactBlockedReasonProjection,
    MutationImpactIntentProjection, MutationImpactProjection, NodeId, Persona, PreviewService,
    SelectionState, SharedSkinStateHandle, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;

#[component]
pub(crate) fn NodeManagementPanel(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
    #[prop(optional_no_strip)] preview: Option<Arc<dyn PreviewService>>,
    #[prop(optional_no_strip)] shared: Option<SharedSkinStateHandle>,
) -> impl IntoView {
    // The active persona governs which structural verbs are dispatchable. An
    // absent shared handle (the legacy editors) reads as Author — full rights,
    // unchanged behaviour. Reactive so a SetPersona re-disables in place.
    let shared_signal = shared.map(|handle| handle.signal());
    let active_persona = Memo::new(move |_| {
        shared_signal.map_or(Persona::Author, |signal| signal.with(|state| state.persona))
    });
    let allows = move |kind: CommandIntentKindProjection| active_persona.get().allows_intent_kind(kind);
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

    // Pre-commit mutation impact (tenet 7): when the host offers foresight,
    // dry-run the prospective add / rename / delete and surface the typed
    // collision / orphan verdict *before* the button is pressed. Optional —
    // an absent host leaves every signal `None`, the buttons stay enabled,
    // and the post-attempt rejection receipt remains the only feedback (so
    // the legacy editor call sites that omit `preview` are unaffected).
    let add_root_impact = RwSignal::new(Option::<MutationImpactProjection>::None);
    let add_child_impact = RwSignal::new(Option::<MutationImpactProjection>::None);
    let rename_impact = RwSignal::new(Option::<MutationImpactProjection>::None);
    let delete_impact = RwSignal::new(Option::<MutationImpactProjection>::None);
    if let Some(service) = preview.clone() {
        for parent_is_selection in [false, true] {
            let service = service.clone();
            let target = if parent_is_selection {
                add_child_impact
            } else {
                add_root_impact
            };
            Effect::new(move |_| {
                let symbol = new_symbol.get();
                let symbol = symbol.trim();
                let parent = if parent_is_selection {
                    selection.with(|state| state.primary.clone())
                } else {
                    None
                };
                // The child add is only meaningful with a parent selected.
                if symbol.is_empty() || (parent_is_selection && parent.is_none()) {
                    target.set(None);
                    return;
                }
                let template = selected_template.get();
                let content = new_content.get();
                let intent = workspace.with(|ws| MutationImpactIntentProjection::AddNode {
                    parent,
                    symbol: symbol.to_string(),
                    initial: initial_content_for_selection(ws, &template, content),
                    is_meta: false,
                });
                target.set(service.preview_mutation_impact(&intent).ok());
            });
        }

        let service_rename = service.clone();
        Effect::new(move |_| {
            let node = selection.with(|state| state.primary.clone());
            let symbol = rename_symbol.get();
            let symbol = symbol.trim();
            let Some(node) = node else {
                rename_impact.set(None);
                return;
            };
            if symbol.is_empty() {
                rename_impact.set(None);
                return;
            }
            // Track the model so collisions re-evaluate as the tree changes.
            workspace.track();
            let intent = MutationImpactIntentProjection::RenameNode {
                node,
                new_symbol: symbol.to_string(),
            };
            rename_impact.set(service_rename.preview_mutation_impact(&intent).ok());
        });

        let service_delete = service;
        Effect::new(move |_| {
            let node = selection.with(|state| state.primary.clone());
            let Some(node) = node else {
                delete_impact.set(None);
                return;
            };
            workspace.track();
            let intent = MutationImpactIntentProjection::DeleteNode { node };
            delete_impact.set(service_delete.preview_mutation_impact(&intent).ok());
        });
    }

    let add_root_dispatch = dispatch.clone();
    let add_child_dispatch = dispatch.clone();
    let rename_dispatch = dispatch.clone();
    let move_root_dispatch = dispatch.clone();
    let move_up_dispatch = dispatch.clone();
    let move_down_dispatch = dispatch.clone();
    let delete_dispatch = dispatch.clone();
    let command_catalog = Memo::new(move |_| {
        workspace
            .with(|workspace| selection.with(|selection| workspace.command_catalog(selection)))
            .governed_by(active_persona.get())
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
                    prop:disabled=move || {
                        !allows(CommandIntentKindProjection::AddNode)
                            || add_root_impact.get().is_some_and(|impact| !impact.legal)
                    }
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
                    prop:disabled=move || {
                        !allows(CommandIntentKindProjection::AddNode)
                            || add_child_impact.get().is_some_and(|impact| !impact.legal)
                    }
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
            {move || {
                // Show the impact for the contextual add: child when a node is
                // selected (the collision-prone case), otherwise the root add.
                let contextual = if selection.with(|state| state.primary.is_some()) {
                    add_child_impact.get()
                } else {
                    add_root_impact.get()
                };
                impact_view(contextual)
            }}
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
                    prop:disabled=move || {
                        !allows(CommandIntentKindProjection::RenameNode)
                            || rename_impact.get().is_some_and(|impact| !impact.legal)
                    }
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
            {move || impact_view(rename_impact.get())}
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
                    prop:disabled=move || {
                        !allows(CommandIntentKindProjection::DeleteNode)
                            || delete_impact.get().is_some_and(|impact| !impact.legal)
                    }
                    on:click=move |_| {
                        if let Some(node) = selection.get_untracked().primary {
                            delete_dispatch.dispatch(WorkspaceIntent::DeleteNode { node });
                        }
                    }
                >
                    "Delete"
                </button>
            </div>
            {move || impact_view(delete_impact.get())}
        </section>
    }
}

/// The reader-facing summary of a prospective edit's typed impact: nothing to
/// say (legal and consequence-free), an orphan warning (legal but breaks
/// dependents), or a hard block (with any name collisions). Pure over the
/// projection so the decision is unit-testable without a DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ImpactVerdict {
    Clear,
    Orphan(usize),
    Blocked {
        label: &'static str,
        collisions: Vec<(String, String)>,
    },
}

fn impact_verdict(impact: &MutationImpactProjection) -> ImpactVerdict {
    if !impact.legal {
        return ImpactVerdict::Blocked {
            label: impact.blocked_reason.map_or("blocked", blocked_label),
            collisions: impact
                .collisions
                .iter()
                .map(|collision| (collision.attempted.clone(), collision.existing.to_string()))
                .collect(),
        };
    }
    if impact.orphaned_dependents.is_empty() {
        ImpactVerdict::Clear
    } else {
        ImpactVerdict::Orphan(impact.orphaned_dependents.len())
    }
}

/// Render the typed pre-commit verdict for a prospective structural edit.
/// Returns an empty view for the clear case so the panel only grows chrome
/// when it has something to say.
fn impact_view(impact: Option<MutationImpactProjection>) -> AnyView {
    let Some(impact) = impact else {
        return ().into_any();
    };
    match impact_verdict(&impact) {
        ImpactVerdict::Clear => ().into_any(),
        ImpactVerdict::Orphan(count) => {
            let plural = if count == 1 { "" } else { "s" };
            view! {
                <div class="dtc-node-management__impact dtc-node-management__impact--warn">
                    {format!("⚠ {count} dependent{plural} would be orphaned")}
                </div>
            }
            .into_any()
        }
        ImpactVerdict::Blocked { label, collisions } => {
            let collisions = collisions
                .into_iter()
                .map(|(attempted, existing)| {
                    view! { <li>{format!("‘{attempted}’ collides with {existing}")}</li> }
                })
                .collect::<Vec<_>>();
            view! {
                <ul class="dtc-node-management__impact dtc-node-management__impact--blocked">
                    <li>{label}</li>
                    {collisions}
                </ul>
            }
            .into_any()
        }
    }
}

/// Human-facing label for a typed block reason. The projection's `stable_id`
/// is the wire/audit token; this is the reader-facing phrasing.
fn blocked_label(reason: MutationImpactBlockedReasonProjection) -> &'static str {
    use MutationImpactBlockedReasonProjection as Reason;
    match reason {
        Reason::SyntaxDiagnostics => "blocked: syntax error",
        Reason::BindDiagnostics => "blocked: does not bind",
        Reason::ProfileViolation => "blocked: not allowed by this profile",
        Reason::NameCollision => "blocked: name already exists here",
        Reason::InvalidDrop => "blocked: invalid move target",
        Reason::UnsupportedInitialContent => "blocked: unsupported initial content",
        Reason::TableCollision => "blocked: conflicts with an existing table cell",
        Reason::DuplicateInput => "blocked: duplicate input",
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
        CommandIntentKindProjection, CommandMetaProjection, NameCollisionProjection, NodeKey,
        RecalcPlanProjection, TemplateManifestProjection, TemplateProjection,
    };

    use super::*;

    fn impact_fixture(legal: bool) -> MutationImpactProjection {
        MutationImpactProjection {
            intent: MutationImpactIntentProjection::DeleteNode {
                node: NodeId::new("tree-node:1"),
            },
            legal,
            blocked_reason: None,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            invalidation_plan: RecalcPlanProjection::default(),
            requires_rebind: Vec::new(),
            affected_refs: Vec::new(),
            orphaned_dependents: Vec::new(),
            collisions: Vec::new(),
        }
    }

    #[test]
    fn impact_verdict_classifies_clear_orphan_and_blocked() {
        // Legal and consequence-free ⇒ nothing to warn about.
        assert_eq!(impact_verdict(&impact_fixture(true)), ImpactVerdict::Clear);

        // Legal but breaks dependents ⇒ an orphan count.
        let mut orphaning = impact_fixture(true);
        orphaning.orphaned_dependents = vec![NodeId::new("tree-node:2"), NodeId::new("tree-node:3")];
        assert_eq!(impact_verdict(&orphaning), ImpactVerdict::Orphan(2));

        // Illegal ⇒ a labelled block surfacing the collision pair verbatim.
        let mut blocked = impact_fixture(false);
        blocked.blocked_reason = Some(MutationImpactBlockedReasonProjection::NameCollision);
        blocked.collisions = vec![NameCollisionProjection {
            attempted: "Net".to_string(),
            existing: NodeId::new("tree-node:9"),
            existing_key: NodeKey::new("k9"),
        }];
        assert_eq!(
            impact_verdict(&blocked),
            ImpactVerdict::Blocked {
                label: "blocked: name already exists here",
                collisions: vec![("Net".to_string(), "tree-node:9".to_string())],
            }
        );
    }

    #[test]
    fn blocked_label_covers_every_reason() {
        use MutationImpactBlockedReasonProjection as Reason;
        for reason in [
            Reason::SyntaxDiagnostics,
            Reason::BindDiagnostics,
            Reason::ProfileViolation,
            Reason::NameCollision,
            Reason::InvalidDrop,
            Reason::UnsupportedInitialContent,
            Reason::TableCollision,
            Reason::DuplicateInput,
        ] {
            assert!(blocked_label(reason).starts_with("blocked: "));
        }
    }

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

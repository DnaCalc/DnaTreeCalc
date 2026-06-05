use dnatreecalc_skin_framework::{
    NodeId, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest,
    SkinState, WorkspaceIntent, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEPENDENCY_INSPECTOR_ID: SkinId = SkinId::new("dependency-inspector");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyInspectorState {
    pub show_reverse_edges: bool,
}

impl SkinState for DependencyInspectorState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct DependencyInspector;

impl DependencyInspector {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for DependencyInspector {
    type State = DependencyInspectorState;

    fn id(&self) -> SkinId {
        DEPENDENCY_INSPECTOR_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Dependencies",
            description: "Dependency graph, collections, invalidations, and diagnostics.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: true,
            renders_arrays_inline: true,
            renders_table_values: true,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        SkinHandle::new(view! { <DependencyInspectorView cx=cx /> }.into_any())
    }
}

#[component]
fn DependencyInspectorView(cx: SkinContext<DependencyInspectorState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let dispatch = cx.dispatch.clone();

    let selected_id = Memo::new(move |_| selection.with(|s| s.primary.clone()));
    let rows = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.node_order
                .iter()
                .filter_map(|id| {
                    let node = ws.node(id)?;
                    let outgoing = ws.dependencies.outgoing_count(id);
                    let incoming = ws.dependencies.incoming_count(id);
                    Some((id.clone(), node.display_name.clone(), outgoing, incoming))
                })
                .collect::<Vec<_>>()
        })
    });

    view! {
        <section class="dtc-dependency-inspector">
            <div class="dtc-dependency-inspector__list">
                {move || {
                    let dispatch = dispatch.clone();
                    rows.with(|rs| {
                        rs.iter()
                            .map(|(id, name, outgoing, incoming)| {
                                dependency_summary_row(
                                    id.clone(),
                                    name.clone(),
                                    *outgoing,
                                    *incoming,
                                    selected_id,
                                    dispatch.clone(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                }}
            </div>
            <div class="dtc-dependency-inspector__detail">
                {move || {
                    workspace.with(|ws| {
                        let Some(id) = selected_id.get() else {
                            return view! { <div class="dtc-empty-detail">"Select a node."</div> }.into_any();
                        };
                        let descriptors = ws
                            .dependencies
                            .descriptors_by_owner
                            .get(&id)
                            .cloned()
                            .unwrap_or_default();
                        view! {
                            <section>
                                <div class="dtc-section-label">"Descriptors"</div>
                                {descriptors.iter().map(|descriptor| {
                                    view! {
                                        <article class="dtc-dependency-card">
                                            <div>{descriptor.kind.clone()}</div>
                                            <code>{descriptor.carrier_detail.clone()}</code>
                                            {descriptor.collection.as_ref().map(|collection| {
                                                view! {
                                                    <div class="dtc-dependency-card__collection">
                                                        <span>{collection.family.clone()}</span>
                                                        <span>{format!("members: {}", collection.members.len())}</span>
                                                    </div>
                                                }
                                            })}
                                        </article>
                                    }
                                }).collect::<Vec<_>>()}
                            </section>
                        }.into_any()
                    })
                }}
            </div>
        </section>
    }
}

fn dependency_summary_row(
    id: NodeId,
    name: String,
    outgoing: usize,
    incoming: usize,
    selected_id: Memo<Option<NodeId>>,
    dispatch: std::sync::Arc<dyn dnatreecalc_skin_framework::Dispatcher>,
) -> impl IntoView {
    let id_for_active = id.clone();
    let id_for_click = id.clone();
    let is_active = Memo::new(move |_| selected_id.get() == Some(id_for_active.clone()));
    view! {
        <button
            type="button"
            class="dtc-dependency-row"
            class:dtc-dependency-row--selected=move || is_active.get()
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            <span>{name}</span>
            <span>{format!("out {}", outgoing)}</span>
            <span>{format!("in {}", incoming)}</span>
        </button>
    }
}

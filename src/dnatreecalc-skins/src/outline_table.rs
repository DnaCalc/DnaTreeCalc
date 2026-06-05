use dnatreecalc_skin_framework::{
    NodeId, NodeValueProjection, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId,
    SkinManifest, SkinState, WorkspaceIntent, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::value_text;

pub const OUTLINE_TABLE_ID: SkinId = SkinId::new("outline-table");

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OutlineTableState {
    pub filter: String,
}

impl SkinState for OutlineTableState {
    fn schema_version() -> u32 {
        1
    }
}

/// The OutlineTable skin — Overview-category, flat-table layout.
///
/// Walking-skeleton scope: renders one row per node (filtering out
/// `is_meta`), columns name + formula + value, clickable for selection.
/// Virtualized scrolling, sortable headers, sticky filter, column
/// resize, and per-row inline edits all land later (`UX-VA-002`,
/// W003/W006). The point of including it in the skeleton is to prove
/// runtime switching between a *category-Editor* skin and a
/// *category-Overview* skin: two genuinely different mental models
/// reading the same shared signals.
#[derive(Default)]
pub struct OutlineTable;

impl OutlineTable {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for OutlineTable {
    type State = OutlineTableState;

    fn id(&self) -> SkinId {
        OUTLINE_TABLE_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Outline table",
            description: "Flat tabular overview of every node and its value.",
            category: SkinCategory::Overview,
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
        let view = view! { <OutlineTableView cx=cx /> }.into_any();
        SkinHandle::new(view)
    }
}

#[component]
fn OutlineTableView(cx: SkinContext<OutlineTableState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let dispatch = cx.dispatch.clone();

    let rows = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.node_order
                .iter()
                .filter_map(|id| {
                    let node = ws.node(id)?;
                    if node.is_meta {
                        return None;
                    }
                    Some(OutlineRow {
                        id: node.id.clone(),
                        path: node.id.as_str().to_string(),
                        formula: if node.content_text.is_empty() {
                            String::new()
                        } else {
                            node.content_text.clone()
                        },
                        value: value_text(&node.computed_value),
                        is_error: matches!(node.computed_value, NodeValueProjection::Error(_)),
                    })
                })
                .collect::<Vec<_>>()
        })
    });

    view! {
        <table class="dtc-outline-table">
            <thead>
                <tr>
                    <th>"Path"</th>
                    <th>"Formula"</th>
                    <th>"Value"</th>
                </tr>
            </thead>
            <tbody>
                {move || {
                    let dispatch = dispatch.clone();
                    rows.with(|rs| {
                        rs.iter()
                            .map(|row| outline_row(row.clone(), selection, dispatch.clone()))
                            .collect::<Vec<_>>()
                    })
                }}
            </tbody>
        </table>
    }
}

#[derive(Clone, PartialEq, Eq)]
struct OutlineRow {
    id: NodeId,
    path: String,
    formula: String,
    value: String,
    is_error: bool,
}

fn outline_row(
    row: OutlineRow,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: std::sync::Arc<dyn dnatreecalc_skin_framework::Dispatcher>,
) -> impl IntoView {
    let id_for_selection = row.id.clone();
    let id_for_click = row.id.clone();
    let is_selected =
        Memo::new(move |_| selection.with(|s| s.primary.as_ref() == Some(&id_for_selection)));

    view! {
        <tr
            class:dtc-outline-row--selected=move || is_selected.get()
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            <td>{row.path}</td>
            <td>{row.formula}</td>
            <td class:dtc-value-display--error=row.is_error>{row.value}</td>
        </tr>
    }
}

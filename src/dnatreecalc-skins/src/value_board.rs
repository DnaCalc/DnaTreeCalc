use dnatreecalc_skin_framework::{
    NodeValueProjection, SkinCapabilities, SkinCategory, SkinContext, SkinHandle, SkinId,
    SkinManifest, SkinState, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::{render_value, value_text};

pub const VALUE_BOARD_ID: SkinId = SkinId::new("value-board");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueBoardState {
    pub compact: bool,
}

impl SkinState for ValueBoardState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct ValueBoard;

impl ValueBoard {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for ValueBoard {
    type State = ValueBoardState;

    fn id(&self) -> SkinId {
        VALUE_BOARD_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Value board",
            description: "Array-safe value cards with table and run-state context.",
            category: SkinCategory::Overview,
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
        SkinHandle::new(view! { <ValueBoardView cx=cx /> }.into_any())
    }
}

#[component]
fn ValueBoardView(cx: SkinContext<ValueBoardState>) -> impl IntoView {
    let workspace = cx.workspace;

    let cards = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.node_order
                .iter()
                .filter_map(|id| {
                    let node = ws.node(id)?;
                    if node.is_meta {
                        return None;
                    }
                    let has_visible_value =
                        !matches!(node.computed_value, NodeValueProjection::Unevaluated);
                    (has_visible_value || node.table.is_some()).then(|| {
                        (
                            node.id.as_str().to_string(),
                            node.display_name.clone(),
                            node.content_text.clone(),
                            node.computed_value.clone(),
                            node.table.clone(),
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
    });

    view! {
        <section class="dtc-value-board">
            {move || cards.with(|cards| {
                cards.iter().map(|(path, name, content, value, table)| {
                    view! {
                        <article class="dtc-value-card">
                            <header>
                                <span>{name.clone()}</span>
                                <code>{path.clone()}</code>
                            </header>
                            <div class="dtc-value-card__formula">{content.clone()}</div>
                            <div title=value_text(value)>{render_value(value)}</div>
                            {table.as_ref().map(|table| {
                                view! {
                                    <dl class="dtc-table-summary">
                                        <dt>"table"</dt><dd>{table.table_name.clone()}</dd>
                                        <dt>"rows"</dt><dd>{table.row_count}</dd>
                                        <dt>"columns"</dt><dd>{table.column_count}</dd>
                                    </dl>
                                }
                            })}
                        </article>
                    }
                }).collect::<Vec<_>>()
            })}
        </section>
    }
}

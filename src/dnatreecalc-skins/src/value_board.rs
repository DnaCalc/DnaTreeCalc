use dnatreecalc_skin_framework::{
    Dispatcher, NodeId, NodeValueProjection, SkinCapabilities, SkinCategory, SkinContext,
    SkinHandle, SkinId, SkinManifest, SkinState, TableCellInput, TableColumnBodyProjection,
    TableProjection, TableRowInput, WorkspaceIntent, WorkspaceSkin,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    let dispatch = cx.dispatch.clone();

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
                let dispatch = dispatch.clone();
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
                                render_table_card(path.clone(), table.clone(), dispatch.clone())
                            })}
                        </article>
                    }
                }).collect::<Vec<_>>()
            })}
        </section>
    }
}

fn render_table_card(
    table_node: String,
    table: TableProjection,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let constant_columns = table
        .columns
        .iter()
        .filter(|column| matches!(column.body, TableColumnBodyProjection::ConstantCells))
        .map(|column| {
            (
                column.column_id.clone(),
                column.name.clone(),
                RwSignal::new(String::new()),
            )
        })
        .collect::<Vec<_>>();
    let next_row_id = RwSignal::new(format!("row:new{}", table.row_count + 1));
    let table_node_for_add = table_node.clone();
    let add_dispatch = dispatch.clone();
    let add_inputs = constant_columns.clone();
    let next_column_id = RwSignal::new(format!("col:new{}", table.column_count + 1));
    let next_column_name = RwSignal::new(format!("New {}", table.column_count + 1));
    let column_values = table
        .rows
        .iter()
        .map(|row| (row.row_id.clone(), RwSignal::new(String::new())))
        .collect::<Vec<_>>();
    let column_value_inputs = column_values.clone();
    let column_add_dispatch = dispatch.clone();
    let table_node_for_column_add = table_node.clone();
    let delete_column_id = RwSignal::new(
        constant_columns
            .first()
            .map(|(column_id, _, _)| column_id.clone())
            .unwrap_or_default(),
    );
    let delete_column_dispatch = dispatch.clone();
    let table_node_for_column_delete = table_node.clone();

    view! {
        <section class="dtc-table-card">
            <dl class="dtc-table-summary">
                <dt>"table"</dt><dd>{table.table_name.clone()}</dd>
                <dt>"rows"</dt><dd>{table.row_count}</dd>
                <dt>"columns"</dt><dd>{table.column_count}</dd>
            </dl>
            {render_table_grid(&table_node, &table, dispatch)}
            <div class="dtc-table-card__add-row">
                <input
                    class="dtc-table-card__row-id"
                    aria-label="New row id"
                    prop:value=move || next_row_id.get()
                    on:input=move |ev| next_row_id.set(event_target_value(&ev))
                />
                {constant_columns.iter().map(|(column_id, name, value)| {
                    let label = format!("{name} value");
                    let value_signal = *value;
                    view! {
                        <input
                            class="dtc-table-card__cell-input"
                            aria-label=label
                            title=column_id.clone()
                            placeholder=name.clone()
                            prop:value=move || value_signal.get()
                            on:input=move |ev| value_signal.set(event_target_value(&ev))
                        />
                    }
                }).collect::<Vec<_>>()}
                <button
                    type="button"
                    on:click=move |_| {
                        let row_id = next_row_id.get_untracked();
                        let values = add_inputs
                            .iter()
                            .map(|(column_id, _, value)| TableCellInput {
                                column_id: column_id.clone(),
                                content: value.get_untracked(),
                            })
                            .collect::<Vec<_>>();
                        let receipt = add_dispatch.dispatch(table_row_add_intent(
                            &table_node_for_add,
                            row_id,
                            values,
                        ));
                        if receipt.accepted {
                            for (_, _, value) in &add_inputs {
                                value.set(String::new());
                            }
                        }
                    }
                >
                    "Add row"
                </button>
            </div>
            <div class="dtc-table-card__add-column">
                <input
                    class="dtc-table-card__column-id"
                    aria-label="New column id"
                    prop:value=move || next_column_id.get()
                    on:input=move |ev| next_column_id.set(event_target_value(&ev))
                />
                <input
                    class="dtc-table-card__column-name"
                    aria-label="New column name"
                    prop:value=move || next_column_name.get()
                    on:input=move |ev| next_column_name.set(event_target_value(&ev))
                />
                {column_values.iter().map(|(row_id, value)| {
                    let label = format!("{row_id} value");
                    let value_signal = *value;
                    view! {
                        <input
                            class="dtc-table-card__cell-input"
                            aria-label=label
                            title=row_id.clone()
                            placeholder=row_id.clone()
                            prop:value=move || value_signal.get()
                            on:input=move |ev| value_signal.set(event_target_value(&ev))
                        />
                    }
                }).collect::<Vec<_>>()}
                <button
                    type="button"
                    on:click=move |_| {
                        let values = column_value_inputs
                            .iter()
                            .map(|(row_id, value)| TableRowInput {
                                row_id: row_id.clone(),
                                content: value.get_untracked(),
                            })
                            .collect::<Vec<_>>();
                        let receipt = column_add_dispatch.dispatch(table_column_add_intent(
                            &table_node_for_column_add,
                            next_column_id.get_untracked(),
                            next_column_name.get_untracked(),
                            values,
                        ));
                        if receipt.accepted {
                            for (_, value) in &column_value_inputs {
                                value.set(String::new());
                            }
                        }
                    }
                >
                    "Add column"
                </button>
            </div>
            <div class="dtc-table-card__delete-column">
                <select
                    aria-label="Delete column"
                    prop:value=move || delete_column_id.get()
                    on:change=move |ev| delete_column_id.set(event_target_value(&ev))
                >
                    {constant_columns.iter().map(|(column_id, name, _)| {
                        view! {
                            <option value=column_id.clone()>{name.clone()}</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
                <button
                    type="button"
                    disabled=constant_columns.is_empty()
                    on:click=move |_| {
                        delete_column_dispatch.dispatch(table_column_delete_intent(
                            &table_node_for_column_delete,
                            &delete_column_id.get_untracked(),
                        ));
                    }
                >
                    "Delete column"
                </button>
            </div>
        </section>
    }
    .into_any()
}

fn render_table_grid(
    table_node: &str,
    table: &TableProjection,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let Some(cells) = table.cells.as_ref() else {
        return view! { <div class="dtc-table-card__empty">"No cell values"</div> }.into_any();
    };
    let grid_style = format!("--dtc-table-cols: {};", table.column_count.max(1) + 1);

    view! {
        <div class="dtc-table-card__grid" role="table" style=grid_style>
            <div class="dtc-table-card__row dtc-table-card__row--header" role="row">
                {table.columns.iter().map(|column| {
                    view! {
                        <span class="dtc-table-card__header-cell" role="columnheader">
                            {column.name.clone()}
                        </span>
                    }
                }).collect::<Vec<_>>()}
                <span class="dtc-table-card__header-cell" role="columnheader"></span>
            </div>
            {cells.body_rows.iter().enumerate().map(|(row_index, row)| {
                let fallback_row_id = table.rows.get(row_index).map(|row| row.row_id.clone());
                let row_id_for_delete = fallback_row_id.clone().unwrap_or_default();
                let table_node_for_delete = table_node.to_string();
                let delete_dispatch = dispatch.clone();
                view! {
                    <div class="dtc-table-card__row" role="row">
                        {row.iter().enumerate().map(|(column_index, cell)| {
                            let Some(column) = table.columns.get(column_index) else {
                                return view! {
                                    <span class="dtc-table-card__formula-cell" role="cell"></span>
                                }
                                .into_any();
                            };
                            let value = cell
                                .as_ref()
                                .map(|cell| cell.value.display_text())
                                .unwrap_or_default();
                            let row_id = cell
                                .as_ref()
                                .and_then(|cell| cell.row_id.clone())
                                .or_else(|| fallback_row_id.clone())
                                .unwrap_or_default();
                            let table_node = table_node.to_string();
                            let column_id = column.column_id.clone();
                            let edit_dispatch = dispatch.clone();
                            if matches!(column.body, TableColumnBodyProjection::ConstantCells) {
                                view! {
                                    <input
                                        class="dtc-table-card__cell-input"
                                        aria-label=format!("{} {}", row_id, column.name)
                                        prop:value=value
                                        on:change=move |ev| {
                                            edit_dispatch.dispatch(table_cell_edit_intent(
                                                &table_node,
                                                &row_id,
                                                &column_id,
                                                event_target_value(&ev),
                                            ));
                                        }
                                    />
                                }
                                .into_any()
                            } else {
                                view! {
                                    <span class="dtc-table-card__formula-cell" role="cell">
                                        {value}
                                    </span>
                                }
                                .into_any()
                            }
                        }).collect::<Vec<_>>()}
                        <button
                            type="button"
                            class="dtc-table-card__row-action"
                            disabled=row_id_for_delete.is_empty()
                            on:click=move |_| {
                                delete_dispatch.dispatch(table_row_delete_intent(
                                    &table_node_for_delete,
                                    &row_id_for_delete,
                                ));
                            }
                        >
                            "Delete"
                        </button>
                    </div>
                }
            }).collect::<Vec<_>>()}
            {if table.totals_row_present {
                view! {
                    <div class="dtc-table-card__row dtc-table-card__row--totals" role="row">
                        {cells.totals_row.iter().map(|cell| {
                            view! {
                                <span class="dtc-table-card__formula-cell" role="cell">
                                    {cell.as_ref().map(|cell| cell.value.display_text()).unwrap_or_default()}
                                </span>
                            }
                        }).collect::<Vec<_>>()}
                        <span class="dtc-table-card__formula-cell" role="cell"></span>
                    </div>
                }
                .into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
        </div>
    }
    .into_any()
}

fn table_cell_edit_intent(
    table: &str,
    row_id: &str,
    column_id: &str,
    content: String,
) -> WorkspaceIntent {
    WorkspaceIntent::EditTableCell {
        table: NodeId::new(table),
        row_id: row_id.to_string(),
        column_id: column_id.to_string(),
        content,
    }
}

fn table_row_add_intent(
    table: &str,
    row_id: String,
    values: Vec<TableCellInput>,
) -> WorkspaceIntent {
    WorkspaceIntent::AddTableRow {
        table: NodeId::new(table),
        row_id,
        values,
    }
}

fn table_row_delete_intent(table: &str, row_id: &str) -> WorkspaceIntent {
    WorkspaceIntent::DeleteTableRow {
        table: NodeId::new(table),
        row_id: row_id.to_string(),
    }
}

fn table_column_add_intent(
    table: &str,
    column_id: String,
    name: String,
    values: Vec<TableRowInput>,
) -> WorkspaceIntent {
    WorkspaceIntent::AddTableColumn {
        table: NodeId::new(table),
        column_id,
        name,
        values,
    }
}

fn table_column_delete_intent(table: &str, column_id: &str) -> WorkspaceIntent {
    WorkspaceIntent::DeleteTableColumn {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_board_table_cell_edit_uses_skin_ir_intent() {
        assert_eq!(
            table_cell_edit_intent("SalesTable", "row:east", "col:amount", "25".to_string()),
            WorkspaceIntent::EditTableCell {
                table: NodeId::new("SalesTable"),
                row_id: "row:east".to_string(),
                column_id: "col:amount".to_string(),
                content: "25".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_row_add_uses_skin_ir_intent() {
        let values = vec![TableCellInput {
            column_id: "col:amount".to_string(),
            content: "40".to_string(),
        }];
        assert_eq!(
            table_row_add_intent("SalesTable", "row:south".to_string(), values.clone()),
            WorkspaceIntent::AddTableRow {
                table: NodeId::new("SalesTable"),
                row_id: "row:south".to_string(),
                values,
            }
        );
    }

    #[test]
    fn value_board_table_row_delete_uses_skin_ir_intent() {
        assert_eq!(
            table_row_delete_intent("SalesTable", "row:east"),
            WorkspaceIntent::DeleteTableRow {
                table: NodeId::new("SalesTable"),
                row_id: "row:east".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_column_add_uses_skin_ir_intent() {
        let values = vec![TableRowInput {
            row_id: "row:east".to_string(),
            content: "3".to_string(),
        }];
        assert_eq!(
            table_column_add_intent(
                "SalesTable",
                "col:discount".to_string(),
                "Discount".to_string(),
                values.clone(),
            ),
            WorkspaceIntent::AddTableColumn {
                table: NodeId::new("SalesTable"),
                column_id: "col:discount".to_string(),
                name: "Discount".to_string(),
                values,
            }
        );
    }

    #[test]
    fn value_board_table_column_delete_uses_skin_ir_intent() {
        assert_eq!(
            table_column_delete_intent("SalesTable", "col:discount"),
            WorkspaceIntent::DeleteTableColumn {
                table: NodeId::new("SalesTable"),
                column_id: "col:discount".to_string(),
            }
        );
    }
}

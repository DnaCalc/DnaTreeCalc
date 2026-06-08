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
    let formula_columns = table
        .columns
        .iter()
        .filter_map(|column| {
            let TableColumnBodyProjection::Formula(formula) = &column.body else {
                return None;
            };
            Some((
                column.column_id.clone(),
                column.name.clone(),
                formula.formula_text.clone(),
                RwSignal::new(formula.formula_text.clone()),
            ))
        })
        .collect::<Vec<_>>();
    let deletable_columns = table
        .columns
        .iter()
        .map(|column| (column.column_id.clone(), column.name.clone()))
        .collect::<Vec<_>>();
    let editable_columns = table
        .columns
        .iter()
        .map(|column| {
            (
                column.column_id.clone(),
                column.name.clone(),
                RwSignal::new(column.name.clone()),
            )
        })
        .collect::<Vec<_>>();
    let totals_columns = table
        .columns
        .iter()
        .map(|column| {
            (
                column.column_id.clone(),
                column.name.clone(),
                RwSignal::new(
                    column
                        .totals_formula
                        .as_ref()
                        .map(|formula| formula.formula_text.clone())
                        .unwrap_or_default(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let next_row_id = RwSignal::new(format!("row:new{}", table.row_count + 1));
    let editable_rows = table
        .rows
        .iter()
        .map(|row| (row.row_id.clone(), RwSignal::new(row.row_id.clone())))
        .collect::<Vec<_>>();
    let table_name = RwSignal::new(table.table_name.clone());
    let table_rename_dispatch = dispatch.clone();
    let table_node_for_table_rename = table_node.clone();
    let table_node_for_add = table_node.clone();
    let add_dispatch = dispatch.clone();
    let add_inputs = constant_columns.clone();
    let rename_rows = editable_rows.clone();
    let rename_row_dispatch = dispatch.clone();
    let table_node_for_row_rename = table_node.clone();
    let reorder_row_id = RwSignal::new(
        table
            .rows
            .first()
            .map(|row| row.row_id.clone())
            .unwrap_or_default(),
    );
    let reorder_row_index = RwSignal::new(String::from("0"));
    let reorder_row_dispatch = dispatch.clone();
    let table_node_for_row_reorder = table_node.clone();
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
        deletable_columns
            .first()
            .map(|(column_id, _)| column_id.clone())
            .unwrap_or_default(),
    );
    let delete_column_dispatch = dispatch.clone();
    let table_node_for_column_delete = table_node.clone();
    let next_formula_column_id = RwSignal::new(format!("col:formula{}", table.column_count + 1));
    let next_formula_column_name = RwSignal::new(format!("Formula {}", table.column_count + 1));
    let next_formula_column_text = RwSignal::new("=[@Amount]".to_string());
    let formula_column_add_dispatch = dispatch.clone();
    let table_node_for_formula_column_add = table_node.clone();
    let formula_edit_dispatch = dispatch.clone();
    let table_node_for_formula_edit = table_node.clone();
    let totals_formula_columns = totals_columns.clone();
    let totals_set_dispatch = dispatch.clone();
    let totals_clear_dispatch = dispatch.clone();
    let table_node_for_totals_set = table_node.clone();
    let table_node_for_totals_clear = table_node.clone();
    let header_visibility_dispatch = dispatch.clone();
    let table_node_for_header_visibility = table_node.clone();
    let totals_visibility_dispatch = dispatch.clone();
    let table_node_for_totals_visibility = table_node.clone();
    let rename_columns = editable_columns.clone();
    let rename_dispatch = dispatch.clone();
    let table_node_for_rename = table_node.clone();
    let reorder_column_id = RwSignal::new(
        deletable_columns
            .first()
            .map(|(column_id, _)| column_id.clone())
            .unwrap_or_default(),
    );
    let reorder_index = RwSignal::new(String::from("0"));
    let reorder_dispatch = dispatch.clone();
    let table_node_for_reorder = table_node.clone();

    view! {
        <section class="dtc-table-card">
            <dl class="dtc-table-summary">
                <dt>"table"</dt><dd>{table.table_name.clone()}</dd>
                <dt>"rows"</dt><dd>{table.row_count}</dd>
                <dt>"columns"</dt><dd>{table.column_count}</dd>
            </dl>
            <label class="dtc-table-card__table-rename">
                <span>"Table name"</span>
                <input
                    class="dtc-table-card__table-name"
                    aria-label="Table name"
                    prop:value=move || table_name.get()
                    on:input=move |ev| table_name.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    on:click=move |_| {
                        table_rename_dispatch.dispatch(table_rename_intent(
                            &table_node_for_table_rename,
                            table_name.get_untracked(),
                        ));
                    }
                >
                    "Rename table"
                </button>
            </label>
            <label class="dtc-table-card__header-toggle">
                <input
                    type="checkbox"
                    aria-label="Show header row"
                    prop:checked=table.header_row_present
                    on:change=move |ev| {
                        header_visibility_dispatch.dispatch(table_header_row_visible_intent(
                            &table_node_for_header_visibility,
                            event_target_checked(&ev),
                        ));
                    }
                />
                <span>"Header row"</span>
            </label>
            <label class="dtc-table-card__totals-toggle">
                <input
                    type="checkbox"
                    aria-label="Show totals row"
                    prop:checked=table.totals_row_present
                    on:change=move |ev| {
                        totals_visibility_dispatch.dispatch(table_totals_row_visible_intent(
                            &table_node_for_totals_visibility,
                            event_target_checked(&ev),
                        ));
                    }
                />
                <span>"Totals row"</span>
            </label>
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
            <div class="dtc-table-card__row-metadata">
                {editable_rows.iter().map(|(row_id, value)| {
                    let value_signal = *value;
                    let table_node = table_node_for_row_rename.clone();
                    let row_id_for_rename = row_id.clone();
                    let rename_dispatch = rename_row_dispatch.clone();
                    view! {
                        <label class="dtc-table-card__row-rename">
                            <span>{row_id.clone()}</span>
                            <input
                                class="dtc-table-card__row-id"
                                aria-label=format!("{row_id} row id")
                                prop:value=move || value_signal.get()
                                on:input=move |ev| value_signal.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                on:click=move |_| {
                                    rename_dispatch.dispatch(table_row_rename_intent(
                                        &table_node,
                                        &row_id_for_rename,
                                        value_signal.get_untracked(),
                                    ));
                                }
                            >
                                "Rename row"
                            </button>
                        </label>
                    }
                }).collect::<Vec<_>>()}
                <div class="dtc-table-card__row-reorder">
                    <select
                        aria-label="Move row"
                        prop:value=move || reorder_row_id.get()
                        on:change=move |ev| reorder_row_id.set(event_target_value(&ev))
                    >
                        {rename_rows.iter().map(|(row_id, _)| {
                            view! {
                                <option value=row_id.clone()>{row_id.clone()}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                    <input
                        class="dtc-table-card__row-index"
                        aria-label="Row index"
                        prop:value=move || reorder_row_index.get()
                        on:input=move |ev| reorder_row_index.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        disabled=rename_rows.is_empty()
                        on:click=move |_| {
                            let new_index = reorder_row_index
                                .get_untracked()
                                .parse::<usize>()
                                .unwrap_or(0);
                            reorder_row_dispatch.dispatch(table_row_reorder_intent(
                                &table_node_for_row_reorder,
                                &reorder_row_id.get_untracked(),
                                new_index,
                            ));
                        }
                    >
                        "Move row"
                    </button>
                </div>
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
            <div class="dtc-table-card__add-formula-column">
                <input
                    class="dtc-table-card__column-id"
                    aria-label="New formula column id"
                    prop:value=move || next_formula_column_id.get()
                    on:input=move |ev| next_formula_column_id.set(event_target_value(&ev))
                />
                <input
                    class="dtc-table-card__column-name"
                    aria-label="New formula column name"
                    prop:value=move || next_formula_column_name.get()
                    on:input=move |ev| next_formula_column_name.set(event_target_value(&ev))
                />
                <input
                    class="dtc-table-card__formula-input"
                    aria-label="New formula column formula"
                    prop:value=move || next_formula_column_text.get()
                    on:input=move |ev| next_formula_column_text.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    on:click=move |_| {
                        formula_column_add_dispatch.dispatch(table_formula_column_add_intent(
                            &table_node_for_formula_column_add,
                            next_formula_column_id.get_untracked(),
                            next_formula_column_name.get_untracked(),
                            next_formula_column_text.get_untracked(),
                        ));
                    }
                >
                    "Add formula"
                </button>
            </div>
            {if formula_columns.is_empty() {
                view! { <span></span> }.into_any()
            } else {
                view! {
                    <div class="dtc-table-card__formula-edits">
                        {formula_columns.iter().map(|(column_id, name, original_formula, value)| {
                            let value_signal = *value;
                            let table_node = table_node_for_formula_edit.clone();
                            let column_id_for_edit = column_id.clone();
                            let edit_dispatch = formula_edit_dispatch.clone();
                            view! {
                                <label class="dtc-table-card__formula-edit">
                                    <span>{name.clone()}</span>
                                    <input
                                        class="dtc-table-card__formula-input"
                                        aria-label=format!("{name} formula")
                                        title=original_formula.clone()
                                        prop:value=move || value_signal.get()
                                        on:change=move |ev| {
                                            edit_dispatch.dispatch(table_formula_column_edit_intent(
                                                &table_node,
                                                &column_id_for_edit,
                                                event_target_value(&ev),
                                            ));
                                        }
                                    />
                                </label>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
            <div class="dtc-table-card__totals-formulas">
                {totals_formula_columns.iter().map(|(column_id, name, value)| {
                    let value_signal = *value;
                    let set_table_node = table_node_for_totals_set.clone();
                    let clear_table_node = table_node_for_totals_clear.clone();
                    let column_id_for_set = column_id.clone();
                    let column_id_for_clear = column_id.clone();
                    let set_dispatch = totals_set_dispatch.clone();
                    let clear_dispatch = totals_clear_dispatch.clone();
                    view! {
                        <label class="dtc-table-card__totals-formula">
                            <span>{name.clone()}</span>
                            <input
                                class="dtc-table-card__formula-input"
                                aria-label=format!("{name} totals formula")
                                prop:value=move || value_signal.get()
                                on:input=move |ev| value_signal.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                on:click=move |_| {
                                    set_dispatch.dispatch(table_totals_formula_set_intent(
                                        &set_table_node,
                                        &column_id_for_set,
                                        value_signal.get_untracked(),
                                    ));
                                }
                            >
                                "Set total"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| {
                                    clear_dispatch.dispatch(table_totals_formula_clear_intent(
                                        &clear_table_node,
                                        &column_id_for_clear,
                                    ));
                                }
                            >
                                "Clear"
                            </button>
                        </label>
                    }
                }).collect::<Vec<_>>()}
            </div>
            <div class="dtc-table-card__column-metadata">
                {editable_columns.iter().map(|(column_id, name, value)| {
                    let value_signal = *value;
                    let table_node = table_node_for_rename.clone();
                    let column_id_for_rename = column_id.clone();
                    let rename_dispatch = rename_dispatch.clone();
                    view! {
                        <label class="dtc-table-card__column-rename">
                            <span>{name.clone()}</span>
                            <input
                                class="dtc-table-card__column-name"
                                aria-label=format!("{name} column name")
                                prop:value=move || value_signal.get()
                                on:input=move |ev| value_signal.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                on:click=move |_| {
                                    rename_dispatch.dispatch(table_column_rename_intent(
                                        &table_node,
                                        &column_id_for_rename,
                                        value_signal.get_untracked(),
                                    ));
                                }
                            >
                                "Rename"
                            </button>
                        </label>
                    }
                }).collect::<Vec<_>>()}
                <div class="dtc-table-card__column-reorder">
                    <select
                        aria-label="Move column"
                        prop:value=move || reorder_column_id.get()
                        on:change=move |ev| reorder_column_id.set(event_target_value(&ev))
                    >
                        {rename_columns.iter().map(|(column_id, name, _)| {
                            view! {
                                <option value=column_id.clone()>{name.clone()}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                    <input
                        class="dtc-table-card__column-index"
                        aria-label="Column index"
                        prop:value=move || reorder_index.get()
                        on:input=move |ev| reorder_index.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        disabled=rename_columns.is_empty()
                        on:click=move |_| {
                            let new_index = reorder_index
                                .get_untracked()
                                .parse::<usize>()
                                .unwrap_or(0);
                            reorder_dispatch.dispatch(table_column_reorder_intent(
                                &table_node_for_reorder,
                                &reorder_column_id.get_untracked(),
                                new_index,
                            ));
                        }
                    >
                        "Move"
                    </button>
                </div>
            </div>
            <div class="dtc-table-card__delete-column">
                <select
                    aria-label="Delete column"
                    prop:value=move || delete_column_id.get()
                    on:change=move |ev| delete_column_id.set(event_target_value(&ev))
                >
                    {deletable_columns.iter().map(|(column_id, name)| {
                        view! {
                            <option value=column_id.clone()>{name.clone()}</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
                <button
                    type="button"
                    disabled=deletable_columns.is_empty()
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
            {table.header_row_present.then(|| view! {
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
            })}
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
            {table.totals_row_present.then(|| view! {
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
            })}
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

fn table_row_rename_intent(table: &str, row_id: &str, new_row_id: String) -> WorkspaceIntent {
    WorkspaceIntent::RenameTableRow {
        table: NodeId::new(table),
        row_id: row_id.to_string(),
        new_row_id,
    }
}

fn table_row_reorder_intent(table: &str, row_id: &str, new_index: usize) -> WorkspaceIntent {
    WorkspaceIntent::ReorderTableRow {
        table: NodeId::new(table),
        row_id: row_id.to_string(),
        new_index,
    }
}

fn table_rename_intent(table: &str, name: String) -> WorkspaceIntent {
    WorkspaceIntent::RenameTable {
        table: NodeId::new(table),
        name,
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

fn table_column_rename_intent(table: &str, column_id: &str, name: String) -> WorkspaceIntent {
    WorkspaceIntent::RenameTableColumn {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
        name,
    }
}

fn table_column_reorder_intent(table: &str, column_id: &str, new_index: usize) -> WorkspaceIntent {
    WorkspaceIntent::ReorderTableColumn {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
        new_index,
    }
}

fn table_formula_column_add_intent(
    table: &str,
    column_id: String,
    name: String,
    formula_text: String,
) -> WorkspaceIntent {
    WorkspaceIntent::AddTableFormulaColumn {
        table: NodeId::new(table),
        column_id,
        name,
        formula_text,
    }
}

fn table_formula_column_edit_intent(
    table: &str,
    column_id: &str,
    formula_text: String,
) -> WorkspaceIntent {
    WorkspaceIntent::EditTableColumnFormula {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
        formula_text,
    }
}

fn table_totals_formula_set_intent(
    table: &str,
    column_id: &str,
    formula_text: String,
) -> WorkspaceIntent {
    WorkspaceIntent::SetTableTotalsFormula {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
        formula_text,
    }
}

fn table_totals_formula_clear_intent(table: &str, column_id: &str) -> WorkspaceIntent {
    WorkspaceIntent::ClearTableTotalsFormula {
        table: NodeId::new(table),
        column_id: column_id.to_string(),
    }
}

fn table_header_row_visible_intent(table: &str, visible: bool) -> WorkspaceIntent {
    WorkspaceIntent::SetTableHeaderRowVisible {
        table: NodeId::new(table),
        visible,
    }
}

fn table_totals_row_visible_intent(table: &str, visible: bool) -> WorkspaceIntent {
    WorkspaceIntent::SetTableTotalsRowVisible {
        table: NodeId::new(table),
        visible,
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
    fn value_board_table_row_rename_uses_skin_ir_intent() {
        assert_eq!(
            table_row_rename_intent("SalesTable", "row:east", "row:central".to_string()),
            WorkspaceIntent::RenameTableRow {
                table: NodeId::new("SalesTable"),
                row_id: "row:east".to_string(),
                new_row_id: "row:central".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_row_reorder_uses_skin_ir_intent() {
        assert_eq!(
            table_row_reorder_intent("SalesTable", "row:north", 0),
            WorkspaceIntent::ReorderTableRow {
                table: NodeId::new("SalesTable"),
                row_id: "row:north".to_string(),
                new_index: 0,
            }
        );
    }

    #[test]
    fn value_board_table_rename_uses_skin_ir_intent() {
        assert_eq!(
            table_rename_intent("SalesTable", "Revenue".to_string()),
            WorkspaceIntent::RenameTable {
                table: NodeId::new("SalesTable"),
                name: "Revenue".to_string(),
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

    #[test]
    fn value_board_table_column_rename_uses_skin_ir_intent() {
        assert_eq!(
            table_column_rename_intent("SalesTable", "col:tax", "VAT".to_string()),
            WorkspaceIntent::RenameTableColumn {
                table: NodeId::new("SalesTable"),
                column_id: "col:tax".to_string(),
                name: "VAT".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_column_reorder_uses_skin_ir_intent() {
        assert_eq!(
            table_column_reorder_intent("SalesTable", "col:tax", 0),
            WorkspaceIntent::ReorderTableColumn {
                table: NodeId::new("SalesTable"),
                column_id: "col:tax".to_string(),
                new_index: 0,
            }
        );
    }

    #[test]
    fn value_board_table_formula_column_add_uses_skin_ir_intent() {
        assert_eq!(
            table_formula_column_add_intent(
                "SalesTable",
                "col:double".to_string(),
                "Double".to_string(),
                "=[@Amount] * 2".to_string(),
            ),
            WorkspaceIntent::AddTableFormulaColumn {
                table: NodeId::new("SalesTable"),
                column_id: "col:double".to_string(),
                name: "Double".to_string(),
                formula_text: "=[@Amount] * 2".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_formula_column_edit_uses_skin_ir_intent() {
        assert_eq!(
            table_formula_column_edit_intent(
                "SalesTable",
                "col:tax",
                "=[@Amount] * 0.2".to_string()
            ),
            WorkspaceIntent::EditTableColumnFormula {
                table: NodeId::new("SalesTable"),
                column_id: "col:tax".to_string(),
                formula_text: "=[@Amount] * 0.2".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_totals_formula_set_uses_skin_ir_intent() {
        assert_eq!(
            table_totals_formula_set_intent(
                "SalesTable",
                "col:amount",
                "=SUM([Amount])".to_string()
            ),
            WorkspaceIntent::SetTableTotalsFormula {
                table: NodeId::new("SalesTable"),
                column_id: "col:amount".to_string(),
                formula_text: "=SUM([Amount])".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_totals_formula_clear_uses_skin_ir_intent() {
        assert_eq!(
            table_totals_formula_clear_intent("SalesTable", "col:amount"),
            WorkspaceIntent::ClearTableTotalsFormula {
                table: NodeId::new("SalesTable"),
                column_id: "col:amount".to_string(),
            }
        );
    }

    #[test]
    fn value_board_table_header_row_visible_uses_skin_ir_intent() {
        assert_eq!(
            table_header_row_visible_intent("SalesTable", false),
            WorkspaceIntent::SetTableHeaderRowVisible {
                table: NodeId::new("SalesTable"),
                visible: false,
            }
        );
    }

    #[test]
    fn value_board_table_totals_row_visible_uses_skin_ir_intent() {
        assert_eq!(
            table_totals_row_visible_intent("SalesTable", false),
            WorkspaceIntent::SetTableTotalsRowVisible {
                table: NodeId::new("SalesTable"),
                visible: false,
            }
        );
    }
}

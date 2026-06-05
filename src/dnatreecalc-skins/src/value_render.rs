use dnatreecalc_skin_framework::NodeValueProjection;
use leptos::prelude::*;

const ARRAY_PREVIEW_ROWS: usize = 10;
const ARRAY_PREVIEW_COLS: usize = 5;

pub(crate) fn value_text(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Unevaluated => "-".to_string(),
        NodeValueProjection::Pending => "...".to_string(),
        NodeValueProjection::Scalar(text) | NodeValueProjection::Error(text) => text.clone(),
        NodeValueProjection::Array(rows) => rows
            .iter()
            .map(|row| row.join(" | "))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

pub(crate) fn render_value(value: &NodeValueProjection) -> AnyView {
    match value {
        NodeValueProjection::Unevaluated => view! {
            <div class="dtc-value-display">"-"</div>
        }
        .into_any(),
        NodeValueProjection::Pending => view! {
            <div class="dtc-value-display">"..."</div>
        }
        .into_any(),
        NodeValueProjection::Scalar(text) => view! {
            <div class="dtc-value-display">{text.clone()}</div>
        }
        .into_any(),
        NodeValueProjection::Error(text) => view! {
            <div class="dtc-value-display dtc-value-display--error">
                {format!("Error: {text}")}
            </div>
        }
        .into_any(),
        NodeValueProjection::Array(rows) => {
            view! { <ArrayValueView rows=rows.clone() /> }.into_any()
        }
    }
}

#[component]
fn ArrayValueView(rows: Vec<Vec<String>>) -> impl IntoView {
    let show_full = RwSignal::new(false);
    let row_count = rows.len();
    let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let is_truncated = row_count > ARRAY_PREVIEW_ROWS || col_count > ARRAY_PREVIEW_COLS;
    let summary = format!("{row_count}x{col_count} array");
    let omitted_summary = omitted_summary(row_count, col_count);

    view! {
        <div class="dtc-array-value-shell">
            <div class="dtc-array-value-shell__summary">
                <span>{summary}</span>
                {if is_truncated {
                    view! {
                        <span class="dtc-array-value-shell__omitted">
                            {omitted_summary}
                        </span>
                        <button
                            type="button"
                            class="dtc-array-value-shell__toggle"
                            on:click=move |_| show_full.update(|value| *value = !*value)
                        >
                            {move || if show_full.get() { "Show preview" } else { "Show full" }}
                        </button>
                    }
                    .into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>
            {move || {
                let visible_rows = if show_full.get() {
                    rows.clone()
                } else {
                    rows.iter()
                        .take(ARRAY_PREVIEW_ROWS)
                        .map(|row| {
                            row.iter()
                                .take(ARRAY_PREVIEW_COLS)
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                };
                render_array_grid(&visible_rows)
            }}
        </div>
    }
}

fn render_array_grid(rows: &[Vec<String>]) -> AnyView {
    view! {
        <div class="dtc-array-value" role="table">
            {rows.iter().map(|row| {
                view! {
                    <div class="dtc-array-value__row" role="row">
                        {row.iter().map(|cell| {
                            view! {
                                <span class="dtc-array-value__cell" role="cell">{cell.clone()}</span>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

fn omitted_summary(row_count: usize, col_count: usize) -> String {
    let hidden_rows = row_count.saturating_sub(ARRAY_PREVIEW_ROWS);
    let hidden_cols = col_count.saturating_sub(ARRAY_PREVIEW_COLS);
    match (hidden_rows, hidden_cols) {
        (0, 0) => String::new(),
        (rows, 0) => format!("showing first {ARRAY_PREVIEW_ROWS} rows, {rows} more"),
        (0, cols) => format!("showing first {ARRAY_PREVIEW_COLS} columns, {cols} more"),
        (rows, cols) => format!(
            "showing first {ARRAY_PREVIEW_ROWS}x{ARRAY_PREVIEW_COLS}, {rows} rows and {cols} columns more"
        ),
    }
}

use dnatreecalc_skin_framework::NodeValueProjection;
use leptos::prelude::*;

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
            <div class="dtc-value-display dtc-value-display--error">{text.clone()}</div>
        }
        .into_any(),
        NodeValueProjection::Array(rows) => view! {
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
        .into_any(),
    }
}

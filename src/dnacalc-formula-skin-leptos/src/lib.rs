//! Shared Leptos formula surface. It consumes Skin IR and emits SkinIntent only.

use dnacalc_skin_ir::{
    FormulaDrillNodeProjection, FormulaResultSurface, OneFormulaIntent, OneFormulaProjection,
    SkinIntent,
};
use leptos::prelude::*;

pub const FORMULA_SKIN_CSS: &str = r#"
.dna-formula-array{display:table;table-layout:fixed;overflow:auto}
.dna-formula-array__row{display:table-row;height:24px}
.dna-formula-array__cell{display:table-cell;height:24px;min-width:72px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
"#;

#[component]
pub fn FormulaSurface(
    projection: OneFormulaProjection,
    on_intent: Callback<SkinIntent>,
) -> impl IntoView {
    let formula_id = projection.formula_space_id.clone();
    let source = projection.editor.source_text.clone();
    let result = projection.result.clone();
    let completion = projection.assist.completion.clone();
    let signature = projection.assist.signature_help.clone();
    let help = projection.assist.function_help.clone();
    let drill = projection.drill.clone();
    let format_code = projection
        .formatting
        .number_format_code
        .clone()
        .unwrap_or_default();
    let edit_id = formula_id.clone();
    let drill_id = formula_id.clone();
    let format_id = formula_id.clone();
    let completion_id = formula_id.clone();
    let diagnostics = projection.editor.diagnostics.clone();
    let cf_rules = projection.formatting.conditional_formatting_rules.clone();
    view! {
        <section class="dna-formula" data-formula-id=formula_id>
            <style>{FORMULA_SKIN_CSS}</style>
            <textarea class="dna-formula__editor" prop:value=source on:input=move |event| {
                let text = event_target_value(&event);
                on_intent.run(SkinIntent::OneFormula(OneFormulaIntent::EditText { formula_space_id:edit_id.clone(), caret_offset:text.len(), text }));
            } />
            {completion.map(|surface| view! { <ul class="dna-formula__completion">{surface.items.into_iter().map(|item| { let proposal_id=item.proposal_id; let formula_id=completion_id.clone(); view! { <li><button on:click=move |_| on_intent.run(SkinIntent::OneFormula(OneFormulaIntent::ApplyCompletion { formula_space_id:formula_id.clone(), proposal_id:proposal_id.clone() }))>{item.display_text}</button></li> } }).collect_view()}</ul> })}
            {signature.map(|surface| view! { <div class="dna-formula__signature">{surface.callee_text}</div> })}
            {help.map(|surface| view! { <aside class="dna-formula__help"><strong>{surface.display_name}</strong><span>{surface.signature}</span><p>{surface.short_description}</p><small>{surface.availability_summary}</small>{surface.deferred_or_profile_limited.then(|| view! { <em>"Profile limited"</em> })}</aside> })}
            <ul class="dna-formula__diagnostics">{diagnostics.into_iter().map(|diagnostic| view! { <li>{diagnostic.message}</li> }).collect_view()}</ul>
            <ResultView result=result formula_id=projection.formula_space_id.clone() on_intent=on_intent />
            <label>"Number format"<input class="dna-formula__format" prop:value=format_code on:change=move |event| on_intent.run(SkinIntent::OneFormula(OneFormulaIntent::SetNumberFormat { formula_space_id:format_id.clone(), number_format_code:Some(event_target_value(&event)) })) /></label>
            <ul class="dna-formula__conditional-formatting">{cf_rules.into_iter().map(|rule| view! { <li>{format!("{:?} {:?} {:?}", rule.typed_rule, rule.font_color, rule.fill_color)}</li> }).collect_view()}</ul>
            <button class="dna-formula__drill-toggle" on:click=move |_| on_intent.run(SkinIntent::OneFormula(OneFormulaIntent::ToggleFormulaDrill { formula_space_id:drill_id.clone() }))>"Formula drill"</button>
            {drill.expanded.then(|| view! { <><ul>{drill.diagnostics.into_iter().map(|d| view! { <li>{d.message}</li> }).collect_view()}</ul><ol class="dna-formula__drill">{drill.tree.into_iter().map(|node| view! { <DrillNodeView node=node formula_id=projection.formula_space_id.clone() on_intent=on_intent /> }).collect_view()}</ol></> })}
        </section>
    }
}

#[component]
fn DrillNodeView(
    node: FormulaDrillNodeProjection,
    formula_id: String,
    on_intent: Callback<SkinIntent>,
) -> AnyView {
    let node_id = node.node_id.clone();
    view! { <li><span>{node.label}</span>{node.array_preview.map(|preview| { let rows=preview.total_rows; let cols=preview.total_cols; let visible_rows=preview.rows.len(); let visible_cols=preview.rows.first().map_or(0, Vec::len); let row_offset=preview.row_offset; let col_offset=preview.col_offset; let formula_id=formula_id.clone(); has_next_window(rows, cols, row_offset, col_offset, visible_rows, visible_cols).then(|| view! { <span>{format!("{}x{} preview", rows, cols)}</span><button on:click=move |_| on_intent.run(drill_expand_intent(&formula_id, &node_id, rows, cols, row_offset, col_offset, visible_rows, visible_cols))>"Next intermediate window"</button> }) })}<ol>{node.children.into_iter().map(|child| view! { <DrillNodeView node=child formula_id=formula_id.clone() on_intent=on_intent /> }).collect_view()}</ol></li> }.into_any()
}

#[component]
fn ResultView(
    result: FormulaResultSurface,
    formula_id: String,
    on_intent: Callback<SkinIntent>,
) -> impl IntoView {
    match result {
        FormulaResultSurface::Array {
            total_rows,
            total_cols,
            window,
            truncated,
            ..
        } => {
            let expand_id = formula_id.clone();
            let visible_rows = window.cells.len();
            let visible_cols = window.cells.first().map_or(0, Vec::len);
            let row_offset = window.row_offset;
            let col_offset = window.col_offset;
            view! { <div class="dna-formula-array" role="grid" aria-label="Formula result array" data-row-offset=row_offset data-col-offset=col_offset>
                {window.cells.into_iter().map(|row| view! { <div class="dna-formula-array__row" role="row">{row.into_iter().map(|cell| view! { <span class="dna-formula-array__cell" role="gridcell">{cell.display_text}</span> }).collect_view()}</div> }).collect_view()}
                {(truncated && has_next_window(total_rows, total_cols, row_offset, col_offset, visible_rows, visible_cols)).then(|| view! { <button on:click=move |_| on_intent.run(result_expand_intent(&expand_id, total_rows, total_cols, row_offset, col_offset, visible_rows, visible_cols))>"Next array window"</button> })}
            </div>
            }.into_any()
        }
        FormulaResultSurface::Display {
            text,
            applied_font_color,
            applied_fill_color,
            ..
        } => {
            let style = format!(
                "color:{};background:{}",
                applied_font_color.unwrap_or_default(),
                applied_fill_color.unwrap_or_default()
            );
            view! { <output class="dna-formula__result" style=style>{text}</output> }.into_any()
        }
        FormulaResultSurface::Error { code, .. } => {
            view! { <output class="dna-formula__error">{code}</output> }.into_any()
        }
        FormulaResultSurface::Pending => view! { <output>"Calculating…"</output> }.into_any(),
        FormulaResultSurface::Empty => view! { <output></output> }.into_any(),
    }
}

#[must_use]
pub fn result_expand_intent(
    formula_id: &str,
    rows: usize,
    cols: usize,
    row_offset: usize,
    col_offset: usize,
    visible_rows: usize,
    visible_cols: usize,
) -> SkinIntent {
    let (next_row, next_col) = if row_offset.saturating_add(visible_rows) < rows {
        (row_offset + visible_rows, col_offset)
    } else {
        (0, col_offset.saturating_add(visible_cols))
    };
    SkinIntent::OneFormula(OneFormulaIntent::RequestResultArrayWindow {
        formula_space_id: formula_id.into(),
        row_offset: next_row,
        col_offset: next_col,
        row_count: rows.saturating_sub(next_row).min(64),
        col_count: cols.saturating_sub(next_col).min(64),
    })
}

#[must_use]
pub fn drill_expand_intent(
    formula_id: &str,
    node_id: &str,
    rows: usize,
    cols: usize,
    row_offset: usize,
    col_offset: usize,
    visible_rows: usize,
    visible_cols: usize,
) -> SkinIntent {
    let (next_row, next_col) = if row_offset.saturating_add(visible_rows) < rows {
        (row_offset + visible_rows, col_offset)
    } else {
        (0, col_offset + visible_cols)
    };
    SkinIntent::OneFormula(OneFormulaIntent::RequestDrillArrayWindow {
        formula_space_id: formula_id.into(),
        node_id: node_id.into(),
        row_offset: next_row,
        col_offset: next_col,
        row_count: rows.saturating_sub(next_row).min(64),
        col_count: cols.saturating_sub(next_col).min(64),
    })
}

fn has_next_window(
    rows: usize,
    cols: usize,
    row_offset: usize,
    col_offset: usize,
    visible_rows: usize,
    visible_cols: usize,
) -> bool {
    row_offset.saturating_add(visible_rows) < rows || col_offset.saturating_add(visible_cols) < cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_pins_headerless_array_rows() {
        assert!(FORMULA_SKIN_CSS.contains("display:table"));
        assert!(FORMULA_SKIN_CSS.contains("height:24px"));
    }

    #[test]
    fn expansion_intents_are_bounded_and_typed() {
        assert!(matches!(
            result_expand_intent("f", 100, 80, 0, 0, 64, 64),
            SkinIntent::OneFormula(OneFormulaIntent::RequestResultArrayWindow {
                row_offset: 64,
                row_count: 36,
                col_count: 64,
                ..
            })
        ));
        assert!(
            matches!(drill_expand_intent("f", "n", 2, 2, 0, 0, 1, 2), SkinIntent::OneFormula(OneFormulaIntent::RequestDrillArrayWindow { node_id, .. }) if node_id == "n")
        );
        assert!(!has_next_window(2, 2, 1, 0, 1, 2));
    }
}

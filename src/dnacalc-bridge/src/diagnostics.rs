//! The staged diagnostics list: severity (Error/Warning/Info) and stage
//! (Syntax/Bind/SemanticPlan) rendered verbatim from the IR — the bridge
//! never reclassifies. Span underlines live in the editor's segment layer;
//! this list carries the row form with span metadata for correlation.

use leptos::prelude::*;

use dnacalc_skin_ir::formula::FormulaDiagnosticProjection;

use crate::vm::{severity_id, severity_label, stage_label};

/// One row per diagnostic. The first row takes `tabindex="-1"` so screen
/// readers can land on the list when it appears (estate convention).
#[component]
pub fn DiagnosticsList(diagnostics: Vec<FormulaDiagnosticProjection>) -> impl IntoView {
    if diagnostics.is_empty() {
        return ().into_any();
    }
    let rows = diagnostics
        .into_iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            let span_end = diagnostic.span_start.saturating_add(diagnostic.span_len);
            view! {
                <li
                    class=format!(
                        "dna-bridge__diagnostic dna-bridge__diagnostic--{}",
                        severity_id(diagnostic.severity),
                    )
                    role="alert"
                    tabindex=if index == 0 { "-1" } else { "" }
                    data-diagnostic-id=diagnostic.diagnostic_id.clone()
                    data-span-start=diagnostic.span_start.to_string()
                    data-span-end=span_end.to_string()
                >
                    <span class="dna-bridge__diagnostic-severity">
                        {severity_label(diagnostic.severity)}
                    </span>
                    <span class="dna-bridge__diagnostic-stage">
                        {stage_label(diagnostic.stage)}
                    </span>
                    {diagnostic.code.clone().map(|code| view! {
                        <span class="dna-bridge__diagnostic-code">{code}</span>
                    })}
                    <span class="dna-bridge__diagnostic-message">{diagnostic.message}</span>
                </li>
            }
        })
        .collect::<Vec<_>>();
    view! {
        <ul class="dna-bridge__diagnostics" aria-label="Formula diagnostics">
            {rows}
        </ul>
    }
    .into_any()
}

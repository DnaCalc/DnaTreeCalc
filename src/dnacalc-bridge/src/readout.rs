//! The readout row (BENCH_SPEC §2): resolved-target summary when the
//! surfaces provide it — callee + active parameter from signature help,
//! target/shape/value previews from the drill node under the caret when the
//! X-Ray is expanded. Renders nothing when the surfaces offer nothing.

use leptos::prelude::*;

use dnacalc_skin_ir::formula::{FormulaAssistSurface, FormulaDrillSurface};

use crate::events::BridgeEvents;
use crate::vm::{drill_node_at_caret, next_preview_window, readout};

#[component]
pub fn ReadoutRow(
    assist: FormulaAssistSurface,
    drill: FormulaDrillSurface,
    /// Caret as a UTF-8 byte offset (the surface's own `caret_offset`).
    caret: usize,
    on_event: BridgeEvents,
) -> impl IntoView {
    let vm = readout(&assist, &drill, caret);
    let pager = drill
        .expanded
        .then(|| drill_node_at_caret(&drill.tree, caret))
        .flatten()
        .and_then(|node| node.array_preview.as_ref())
        .filter(|preview| preview.truncated)
        .and_then(next_preview_window);
    if vm.is_empty() && pager.is_none() {
        return ().into_any();
    }
    view! {
        <div class="dna-bridge__readout" role="status" aria-label="Formula readout">
            {vm.callee.map(|callee| view! {
                <span class="dna-bridge__readout-callee">{callee}</span>
            })}
            {vm.active_parameter.map(|parameter| view! {
                <span class="dna-bridge__readout-arg">{format!("arg: {parameter}")}</span>
            })}
            {vm.target_label.map(|target| view! {
                <span class="dna-bridge__readout-target">{target}</span>
            })}
            {vm.shape.map(|shape| view! {
                <span class="dna-bridge__readout-shape">{shape}</span>
            })}
            {vm.value_preview.map(|value| view! {
                <span class="dna-bridge__readout-value">{value}</span>
            })}
            {pager.map(|event| view! {
                <button
                    type="button"
                    class="dna-bridge__readout-pager"
                    on:click=move |_| on_event.run(event.clone())
                >
                    "More"
                </button>
            })}
        </div>
    }
    .into_any()
}

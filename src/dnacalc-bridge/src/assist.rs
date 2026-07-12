//! Assist surfaces: completion popup (px-anchored, keyboard-driven from the
//! editor), signature help pane, function help pane. All render host truth
//! verbatim — the popup applies proposals by id and never edits text.

use leptos::prelude::*;

use dnacalc_skin_ir::formula::{CompletionSurface, FunctionHelpSurface, SignatureHelpSurface};

use crate::events::{BridgeEvent, BridgeEvents};
use crate::vm::{completion_kind_class, completion_kind_glyph, completion_kind_id};

/// The completion popup, anchored at the host-computed pixel position
/// (`anchor_left_px` / `anchor_top_px` + `line_height_px` below the line).
///
/// Keyboard handling (Up/Down/Enter/Esc) lives in the editor's keydown
/// handler — focus never leaves the textarea; this component receives the
/// shared `highlighted`/`open` state. Mousedown on an item applies it too.
#[component]
pub fn CompletionPopup(
    surface: CompletionSurface,
    highlighted: RwSignal<usize>,
    open: RwSignal<bool>,
    on_event: BridgeEvents,
) -> impl IntoView {
    let style = StoredValue::new(format!(
        "left:{}px;top:{}px;",
        surface.anchor_left_px,
        surface.anchor_top_px.saturating_add(surface.line_height_px),
    ));
    let items = StoredValue::new(surface.items);
    move || {
        open.get().then(|| {
            let rows = items.with_value(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let apply = BridgeEvent::CompletionApplied {
                            proposal_id: item.proposal_id.clone(),
                        };
                        let kind = item.kind;
                        view! {
                            <li
                                class=move || {
                                    let mut class = format!(
                                        "dna-bridge__completion-item {}",
                                        completion_kind_class(kind),
                                    );
                                    if highlighted.get() == index {
                                        class.push_str(" dna-bridge__completion-item--active");
                                    }
                                    class
                                }
                                role="option"
                                aria-selected=move || (highlighted.get() == index).to_string()
                                data-kind=completion_kind_id(kind)
                                data-proposal-id=item.proposal_id.clone()
                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                    ev.prevent_default();
                                    open.set(false);
                                    on_event.run(apply.clone());
                                }
                            >
                                <span class="dna-bridge__completion-glyph" aria-hidden="true">
                                    {completion_kind_glyph(kind)}
                                </span>
                                <span class="dna-bridge__completion-text">
                                    {item.display_text.clone()}
                                </span>
                                {item.documentation_ref.clone().map(|doc| view! {
                                    <span class="dna-bridge__completion-doc">{doc}</span>
                                })}
                            </li>
                        }
                    })
                    .collect_view()
            });
            view! {
                <ul
                    class="dna-bridge__completion"
                    role="listbox"
                    aria-label="Completions"
                    style=style.get_value()
                >
                    {rows}
                </ul>
            }
        })
    }
}

/// The signature help pane: callee + parameter list with the active
/// parameter emphasized (host decides which one is active).
#[component]
pub fn SignaturePane(surface: SignatureHelpSurface) -> impl IntoView {
    let active = surface.active_parameter;
    let count = surface.parameters.len();
    let parameters = surface
        .parameters
        .into_iter()
        .enumerate()
        .map(|(index, parameter)| {
            let is_active = parameter.is_active || active == Some(index);
            let class = if is_active {
                "dna-bridge__signature-param dna-bridge__signature-param--active"
            } else {
                "dna-bridge__signature-param"
            };
            view! {
                <span class=class>
                    {parameter.name}
                    {(index + 1 < count).then_some(", ")}
                </span>
            }
        })
        .collect_view();
    view! {
        <div
            class="dna-bridge__signature"
            data-active-parameter=active.map(|index| index.to_string()).unwrap_or_default()
        >
            <span class="dna-bridge__signature-callee">{surface.callee_text}</span>
            "("
            {parameters}
            ")"
        </div>
    }
}

/// The function help pane: registry-fed documentation for the callee.
#[component]
pub fn FunctionHelpPane(surface: FunctionHelpSurface) -> impl IntoView {
    view! {
        <aside class="dna-bridge__function-help" data-lookup-key=surface.lookup_key.clone()>
            <strong class="dna-bridge__function-help-name">{surface.display_name}</strong>
            {surface.signature.map(|signature| view! {
                <code class="dna-bridge__function-help-signature">{signature}</code>
            })}
            {surface.short_description.map(|description| view! {
                <p class="dna-bridge__function-help-description">{description}</p>
            })}
            {surface.availability_summary.map(|availability| view! {
                <small class="dna-bridge__function-help-availability">{availability}</small>
            })}
            {surface.deferred_or_profile_limited.then(|| view! {
                <em class="dna-bridge__function-help-limited">"Profile limited"</em>
            })}
        </aside>
    }
}

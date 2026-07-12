//! The full-fidelity formula editor (SHELL_SPEC §6, Bench context): token-lit
//! underlay from `FormulaEditorSurface.syntax_runs`, caret/selection mirrored
//! both directions, completion/signature/function-help panes, diagnostics,
//! readout row, X-Ray toggle.
//!
//! Layering law: everything visible is host truth. The `<textarea>` is the
//! input capture surface; the colored text underneath is a render of the
//! host's own runs over the host's own text. When the local buffer and the
//! host surface diverge (a keystroke the host has not echoed yet), the token
//! underlay hides and the textarea's plain ink shows — the bridge never
//! paints token colors it did not receive.

use leptos::prelude::*;
use leptos::web_sys::HtmlTextAreaElement;

use dnacalc_skin_ir::formula::{FormulaAssistSurface, FormulaDrillSurface, FormulaEditorSurface};

use crate::assist::{CompletionPopup, FunctionHelpPane, SignaturePane};
use crate::diagnostics::DiagnosticsList;
use crate::events::{BridgeEvent, BridgeEvents, EditDiscipline};
use crate::readout::ReadoutRow;
use crate::vm::{
    RenderSegment, completion_applied, completion_next, editor_segments, is_stale, role_class,
    selection_from_dom, severity_class, text_edited_from_dom, utf8_to_utf16,
};

/// Render one [`RenderSegment`] as a span in the token underlay. Shared with
/// degrade mode, which only ever produces `role = None` segments.
pub(crate) fn segment_view(segment: RenderSegment) -> impl IntoView {
    let mut class = String::from("dna-bridge__seg");
    if let Some(role) = segment.role {
        class.push(' ');
        class.push_str(&role_class(role));
    }
    if let Some(severity) = segment.diag {
        class.push(' ');
        class.push_str(&severity_class(severity));
    }
    view! {
        <span
            class=class
            data-span-start=segment.byte_start.to_string()
            data-span-end=segment.byte_end.to_string()
        >
            {segment.text}
        </span>
    }
}

/// Read the DOM selection out of the textarea in IR terms.
fn dom_selection_event(target: &HtmlTextAreaElement) -> BridgeEvent {
    let text = target.value();
    let start = target.selection_start().ok().flatten().unwrap_or(0);
    let end = target.selection_end().ok().flatten().unwrap_or(start);
    let backward = target
        .selection_direction()
        .ok()
        .flatten()
        .is_some_and(|direction| direction == "backward");
    selection_from_dom(&text, start, end, backward)
}

/// The full-mode formula workbench editor.
///
/// Emits [`BridgeEvent`]s only — host adapters construct intents. Surfaces
/// arrive as plain props: the host remounts (or re-renders) the bridge with
/// each new projection, the estate's established skin pattern.
#[component]
pub fn FormulaBridge(
    /// The host's editor surface (source text, caret/selection, syntax runs,
    /// diagnostics, freshness).
    editor: FormulaEditorSurface,
    /// Completion / signature help / function help, when the host offers them.
    #[prop(optional)]
    assist: FormulaAssistSurface,
    /// Drill surface for readout previews (X-Ray data when expanded).
    #[prop(optional)]
    drill: FormulaDrillSurface,
    /// The bridge's semantic-event sink.
    on_event: BridgeEvents,
) -> impl IntoView {
    let source = editor.source_text.clone();
    let segments = editor_segments(&editor);
    let stale = is_stale(&editor);
    let caret = editor.caret_offset;

    // Local echo tracking: the underlay only paints while DOM text == host
    // text. `buffer` mirrors what the user sees; it starts equal to source.
    let buffer = RwSignal::new(source.clone());
    let echo_pending = {
        let source = source.clone();
        Memo::new(move |_| buffer.get() != source)
    };

    let edit_state = RwSignal::new(EditDiscipline::Selected);

    let completion = assist.completion.clone();
    let completion_items = StoredValue::new(
        completion
            .as_ref()
            .map(|c| c.items.clone())
            .unwrap_or_default(),
    );
    let completion_open = RwSignal::new(completion.as_ref().is_some_and(|c| !c.items.is_empty()));
    let highlighted = RwSignal::new(
        completion
            .as_ref()
            .map(|c| c.selected_index)
            .unwrap_or(0)
            .min(completion_items.with_value(|items| items.len().saturating_sub(1))),
    );

    let input_ref = NodeRef::<leptos::html::Textarea>::new();

    // IR -> DOM: mirror the surface's selection into the textarea on mount.
    // Offsets convert from the IR's UTF-8 bytes to the DOM's UTF-16 units.
    {
        let anchor16 = utf8_to_utf16(&source, editor.selection_anchor);
        let focus16 = utf8_to_utf16(&source, editor.selection_focus);
        Effect::new(move |_| {
            if let Some(input) = input_ref.get() {
                let direction = if focus16 < anchor16 {
                    "backward"
                } else {
                    "forward"
                };
                let _ = input.set_selection_range_with_direction(
                    anchor16.min(focus16),
                    anchor16.max(focus16),
                    direction,
                );
            }
        });
    }

    // DOM -> IR: emit SelectionSet when the DOM selection moves away from
    // what we last knew (surface value at mount, then whatever we emitted).
    let last_selection = StoredValue::new((editor.selection_anchor, editor.selection_focus));
    let emit_selection = move |target: HtmlTextAreaElement| {
        let event = dom_selection_event(&target);
        if let BridgeEvent::SelectionSet { anchor, focus } = &event
            && last_selection.get_value() != (*anchor, *focus)
        {
            last_selection.set_value((*anchor, *focus));
            on_event.run(event);
        }
    };

    let on_input = move |ev| {
        let target: HtmlTextAreaElement = event_target(&ev);
        let text = target.value();
        let caret16 = target
            .selection_start()
            .ok()
            .flatten()
            .unwrap_or_else(|| utf8_to_utf16(&text, text.len()));
        buffer.set(text.clone());
        edit_state.set(EditDiscipline::Editing);
        // Verbatim passthrough: `text` is never inspected or altered here.
        on_event.run(text_edited_from_dom(text, caret16));
    };

    // Propagation policy (bead dtc-1tk.1 / H1b): `stop_propagation()` is
    // called ONLY on the branches below that actually consume the key —
    // completion navigation (Up/Down/Enter/Esc while the popup is open),
    // plain Enter (commit), and plain Escape (revert). Every other key,
    // including F9 (Recalculate) and Ctrl+K (command deck) — the whole
    // F-key-exemption class SHELL_SPEC §5 requires to work from inside edit
    // buffers — falls through untouched and bubbles to the shell's own
    // `.dna-shell` keydown handler in the composed Bench app.
    //
    // Precedence for Escape specifically: the editor's own Escape (exact
    // revert, host-side) stops propagation deliberately, even though the
    // shell's text-entry guard would already suppress a bare, unmodified
    // Escape originating from this textarea (see
    // `dnacalc-shell::shell::event_target_is_text_entry` /
    // `keyboard::route_key`). One keystroke gets one effect: the editor has
    // already fully handled it (closed the popup, or reverted the buffer),
    // so it must not also reach the shell's overlay-Esc ladder in the same
    // keystroke. This is belt-and-suspenders — it holds even if the guard's
    // suppression rule ever changes — and it is the only place this handler
    // stops propagation for a key it isn't exclusively responsible for.
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let key = ev.key();
        // Overlay-first grammar: while the completion popup is open it owns
        // Up/Down/Enter/Esc (Esc closes the topmost overlay before anything
        // else — SHELL_SPEC §1).
        if completion_open.get_untracked() {
            match key.as_str() {
                "ArrowDown" => {
                    ev.prevent_default();
                    ev.stop_propagation();
                    let len = completion_items.with_value(Vec::len);
                    highlighted.update(|h| *h = completion_next(len, *h, 1));
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    ev.stop_propagation();
                    let len = completion_items.with_value(Vec::len);
                    highlighted.update(|h| *h = completion_next(len, *h, -1));
                }
                "Enter" => {
                    ev.prevent_default();
                    ev.stop_propagation();
                    let applied = completion_items
                        .with_value(|items| completion_applied(items, highlighted.get_untracked()));
                    if let Some(event) = applied {
                        completion_open.set(false);
                        on_event.run(event);
                    }
                }
                "Escape" => {
                    ev.prevent_default();
                    ev.stop_propagation();
                    completion_open.set(false);
                }
                // Anything else — F9, Ctrl+K, plain letters while the popup
                // happens to be open — is not the popup's concern: let it
                // bubble so the shell's guard-order exemptions still apply.
                _ => {}
            }
            return;
        }
        match key.as_str() {
            // Enter commits through the context's single entry verb;
            // Shift+Enter stays a plain newline in the buffer.
            "Enter" if !ev.shift_key() => {
                ev.prevent_default();
                ev.stop_propagation();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::CommitRequested);
            }
            // Esc exact-reverts — host-side. The bridge only reports it, but
            // still stops propagation here — see the precedence note above.
            "Escape" => {
                ev.prevent_default();
                ev.stop_propagation();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::RevertRequested);
            }
            // Every other key — F9, Ctrl+K, and the rest of the F-key
            // exemption class — is not consumed here and must bubble
            // undisturbed.
            _ => {}
        }
    };

    let token_spans = segments.into_iter().map(segment_view).collect_view();

    view! {
        <div
            class="dna-bridge dna-bridge--full"
            data-mode="full"
            data-edit-state=move || edit_state.get().as_str()
            data-stale=stale.to_string()
        >
            <style>{crate::bridge_css()}</style>
            {stale.then(|| view! {
                <span class="dna-bridge__stale" role="status">"stale surface"</span>
            })}
            <div
                class=move || {
                    if echo_pending.get() {
                        "dna-bridge__editor dna-bridge__editor--pending-echo"
                    } else {
                        "dna-bridge__editor"
                    }
                }
            >
                <div class="dna-bridge__tokens" aria-hidden="true">{token_spans}</div>
                <textarea
                    class="dna-bridge__input"
                    node_ref=input_ref
                    prop:value=source.clone()
                    aria-label="Formula editor"
                    spellcheck="false"
                    autocomplete="off"
                    wrap="soft"
                    on:input=on_input
                    on:keydown=on_keydown
                    on:keyup=move |ev| emit_selection(event_target(&ev))
                    on:mouseup=move |ev| emit_selection(event_target(&ev))
                    on:select=move |ev| emit_selection(event_target(&ev))
                >
                </textarea>
            </div>
            <ReadoutRow
                assist=assist.clone()
                drill=drill.clone()
                caret=caret
                on_event=on_event
            />
            {completion.map(|surface| view! {
                <CompletionPopup
                    surface=surface
                    highlighted=highlighted
                    open=completion_open
                    on_event=on_event
                />
            })}
            {assist.signature_help.clone().map(|surface| view! {
                <SignaturePane surface=surface />
            })}
            {assist.function_help.clone().map(|surface| view! {
                <FunctionHelpPane surface=surface />
            })}
            <DiagnosticsList diagnostics=editor.diagnostics.clone() />
            <button
                type="button"
                class="dna-bridge__drill-toggle"
                on:click=move |_| on_event.run(BridgeEvent::DrillToggled)
            >
                "X-Ray"
            </button>
        </div>
    }
}

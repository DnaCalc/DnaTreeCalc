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
    RenderSegment, buffer_is_dirty, completion_applied, completion_next, drill_state_id,
    drill_state_label, editor_segments, is_stale, partial_eval, role_class, segment_lit_by_caret,
    selection_from_dom, severity_class, should_consume_undo_redo_locally, text_edited_from_dom,
    utf8_to_utf16,
};

/// Render one [`RenderSegment`] as a span in the token underlay. Shared with
/// degrade mode, which only ever produces `role = None` segments (and never
/// lights a caret token — reference X-Ray is a full-mode affordance).
pub(crate) fn segment_view(segment: RenderSegment) -> impl IntoView {
    segment_view_caret(segment, false)
}

/// Render one segment, optionally lit as the reference-X-Ray caret token
/// (mechanism 07): `caret_lit` adds `dna-bridge__seg--caret` so the token the
/// caret rests in is highlighted in the editor.
pub(crate) fn segment_view_caret(segment: RenderSegment, caret_lit: bool) -> impl IntoView {
    let mut class = String::from("dna-bridge__seg");
    if let Some(role) = segment.role {
        class.push(' ');
        class.push_str(&role_class(role));
    }
    if let Some(severity) = segment.diag {
        class.push(' ');
        class.push_str(&severity_class(severity));
    }
    if caret_lit {
        class.push_str(" dna-bridge__seg--caret");
    }
    view! {
        <span
            class=class
            data-span-start=segment.byte_start.to_string()
            data-span-end=segment.byte_end.to_string()
            data-caret-lit=caret_lit.to_string()
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

    // Partial-evaluation pill (BENCH_SPEC §4): the drill node matched to the
    // current non-empty selection span. Host truth only — the drill tree
    // already carries every subexpression's evaluated value, so the pill is a
    // read, never a mutation (Esc collapses the selection and it dismisses).
    let pill = partial_eval(&drill, editor.selection_anchor, editor.selection_focus);

    // Local echo tracking: the underlay only paints while DOM text == host
    // text. `buffer` mirrors what the user sees; it starts equal to source.
    // This is also the exact "dirty" predicate the undo/redo carve-out below
    // gates on (bead dtc-lfz.2 / SHELL_SPEC §5) — one definition of "the
    // buffer holds uncommitted keystrokes", not two.
    let buffer = RwSignal::new(source.clone());
    let echo_pending = {
        let source = source.clone();
        Memo::new(move |_| buffer_is_dirty(&buffer.get(), &source))
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

    // Propagation policy (bead dtc-1tk.1 / H1b, extended by dtc-lfz.2 / S1.1):
    // `stop_propagation()` is called ONLY on the branches below that actually
    // consume the key — completion navigation (Up/Down/Enter/Esc while the
    // popup is open), plain Enter (commit), plain Escape (revert), and now
    // Ctrl+Z/Ctrl+Y/Ctrl+Shift+Z but ONLY while the buffer is dirty (see
    // below). Every other key, including F9 (Recalculate) and Ctrl+K
    // (command deck) — the whole F-key-exemption class SHELL_SPEC §5
    // requires to work from inside edit buffers — falls through untouched
    // and bubbles to the shell's own `.dna-shell` keydown handler in the
    // composed Bench app.
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
    // suppression rule ever changes.
    //
    // Undo/redo carve-out (owner-ratified 2026-07-12, bead dtc-lfz.2):
    // Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z are TEXT-LOCAL WHILE THE BUFFER IS
    // DIRTY — see the guarded match arm below. Clipboard (Ctrl+X/C/V) and
    // every other modified chord are unaffected by buffer dirtiness; the
    // owner chose the narrower undo-only carve-out, not a broader
    // "capture everything while dirty" rule.
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
            // Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z, but only while the buffer is
            // dirty (bead dtc-lfz.2 / SHELL_SPEC §5 carve-out): consume the
            // chord locally so the browser's own textarea undo/redo stack
            // fires on the user's uncommitted typing. Deliberately NO
            // `prevent_default()` here — that native undo/redo IS the
            // effect this carve-out exists to produce; preventing the
            // default would silence it while still stopping the shell from
            // ever seeing the chord. `stop_propagation()` alone is correct:
            // it keeps the keystroke out of the shell's Undo/Redo verbs for
            // this one keydown without touching what the browser does with
            // it. Once the buffer is clean this guard is false and the
            // match falls through to the catch-all below, so the same chord
            // bubbles untouched and the shell's model Undo/Redo fires,
            // exactly as before this bead.
            _ if should_consume_undo_redo_locally(
                &key,
                ev.ctrl_key() || ev.meta_key(),
                ev.alt_key(),
                echo_pending.get_untracked(),
            ) =>
            {
                ev.stop_propagation();
            }
            // Every other key — F9, Ctrl+K, clipboard chords, and the rest
            // of the F-key exemption class — is not consumed here and must
            // bubble undisturbed.
            _ => {}
        }
    };

    let token_spans = segments
        .into_iter()
        .map(|segment| {
            let lit = segment_lit_by_caret(&segment, caret);
            segment_view_caret(segment, lit)
        })
        .collect_view();

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
                {pill.map(|pill| {
                    let state_id = drill_state_id(pill.state);
                    let state_label = drill_state_label(pill.state);
                    view! {
                        <div
                            class="dna-bridge__pill"
                            role="status"
                            aria-label="Partial evaluation"
                            data-pill-node=pill.node_id.clone()
                        >
                            <span class="dna-bridge__pill-expr">{pill.expression}</span>
                            <span class="dna-bridge__pill-type">{pill.type_label}</span>
                            {pill.shape.map(|shape| view! {
                                <span class="dna-bridge__pill-shape">{shape}</span>
                            })}
                            {pill.value_preview.map(|value| view! {
                                <span class="dna-bridge__pill-value">{value}</span>
                            })}
                            <span class="dna-bridge__pill-chip" data-state=state_id>
                                {state_label}
                            </span>
                        </div>
                    }
                })}
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

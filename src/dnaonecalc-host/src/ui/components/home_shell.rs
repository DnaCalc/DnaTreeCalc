//! WS-14 Pre-MVP home shell.
//!
//! Single-component shell that replaces (eventually retires) the legacy
//! `OneCalcShellApp` + mode shells. The pre-MVP slice mounts only a
//! formula caption + native `<textarea>` + result block + status foot,
//! driven through the existing `NativeOxfmlHostSession`.
//!
//! Subsequent WS-14 phases grow this file into the progressive-disclosure
//! home (drill-downs, scenario breadcrumb, compare entry, command palette,
//! …). The signature, props, and bridge plumbing established here remain
//! stable across those phases.
//!
//! References:
//! * `docs/WS14_PRE_MVP_PATH.md` §4 — eight-step slice
//! * `docs/APP_UX_REALIZATION.md` §4.1 — eventual editor-hero contract
//! * `docs/WS14_DESIGN_FORMULA_EDITOR.md` §4 AD-1..AD-5 — native textarea
//!   discipline this slice already follows

use std::sync::Arc;

use dnacalc_formula_skin_leptos::FormulaSurface;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlTextAreaElement, InputEvent as WebInputEvent, KeyboardEvent as WebKeyboardEvent,
    MouseEvent as WebMouseEvent,
};

use crate::adapters::oxfml::{FormulaTextSpan, NativeOxfmlHostSession};
use crate::app::reducer::{
    accept_completion_by_proposal_id_on_active_formula_space,
    accept_selected_completion_with_suppression_on_active_formula_space,
    apply_editor_box_metrics_to_active_formula_space, apply_editor_input_to_active_formula_space,
    close_scenario_breadcrumb, dismiss_completion_popup_on_active_formula_space,
    move_completion_popup_selection_on_active_formula_space,
    toggle_formula_drill_on_active_formula_space, toggle_scenario_breadcrumb,
    toggle_view_mode_on_workspace, VbaModuleSourceLoadRequest,
};
use crate::services::completion_popup::CompletionAcceptance;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, ArrayCellFormatView, BridgeHealth, CapabilityContextView,
    CommandPaletteEntry, CommandPaletteEntryKind, CommandPaletteView, CompletionPopupItemView,
    CompletionPopupView, ConditionalFormattingRuleView, ContextChipField, DataBarDirectionView,
    DiagnosticSquiggle, EditorMetricsChip, EntryModePill, FormattingControlsView,
    FormulaDrillDiagnosticRow, FormulaDrillNode, FormulaDrillPhaseChip, FormulaDrillPhaseState,
    FormulaDrillView, FormulaTabChip, FormulaTabStripView, FunctionHelpCardView, ManageFormulasRow,
    ManageFormulasView, NumberFormatPreset, ResultClassPill, ResultContextChip, ResultKind,
    ResultView, ScenarioBreadcrumbAction, ScenarioBreadcrumbActionId, ScenarioBreadcrumbEntry,
    ScenarioBreadcrumbView, ScenarioPolicyView, SignatureHelpView, StatusView,
    ValueCapabilityFactKind, VbaHostAssociationView, VbaHostContextView,
};
use crate::services::live_edit::apply_live_editor_input;
#[cfg(target_arch = "wasm32")]
use crate::services::live_edit::{flush_pending_runtime_recalc, AUTO_DEBOUNCE_IDLE_WINDOW_MS};
use crate::state::OneCalcHostState;
use crate::state::ViewMode;
use crate::ui::design_tokens::theme::ThemeStyleTag;
use crate::ui::editor::caret_box_measurement::measure_textarea_box;
use crate::ui::editor::commands::{classify_dom_input, EditorInputEvent, EditorInputKind};
use crate::ui::editor::geometry::caret_box_for_offset;
use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};

type HostCore = dnaonecalc_core::OneCalcCore<OneCalcHostState>;
type HostStateSignal = RwSignal<HostCore, LocalStorage>;

#[component]
pub fn HomeShell(
    initial_state: OneCalcHostState,
    #[prop(default = None)] editor_bridge: Option<Arc<NativeOxfmlHostSession>>,
) -> impl IntoView {
    // Hydrate from `localStorage["dnaonecalc.workspace.v1"]` before
    // the state signal sees its first subscriber, so the user's
    // pinned ids and last-edited formula land in `initial_state`
    // *before* the reactive view-model first runs. On non-wasm
    // targets this is a no-op (the SSR build doesn't have
    // localStorage); the same call site keeps both branches
    // visually identical.
    let mut initial_core = HostCore::new(initial_state);
    initial_core.hydrate_with(&mut crate::persistence::LocalWorkspacePersistence);

    let state: HostStateSignal = RwSignal::new_local(initial_core);

    // Auto-save the workspace envelope to localStorage on every
    // state change. The serialise + write path is cheap (<1 ms for
    // a typical workspace) and fires inside the browser's main
    // thread, so any heavier persistence mechanism would have to
    // schedule itself anyway. Storage failures (quota, disabled
    // site data) log to console without taking the rest of the app
    // down.
    Effect::new(move |_| {
        state.with(|core| {
            core.persist_with(&mut crate::persistence::LocalWorkspacePersistence);
        });
    });

    // Reactive view-model: rebuilds whenever the state signal changes.
    let view_model = Memo::new(move |_| state.with(|core| build_home_shell_view_model(core)));

    // NodeRef on the editor textarea so we can imperatively sync
    // `value` + `selectionStart/End` from host state after each
    // reactive flush. Without this, two failure modes appear under
    // slow recalc:
    //
    // 1. `prop:value=textarea_value` writes `textarea.value = X`
    //    even when `X` is what the textarea already has. Some
    //    browsers reset the caret to the end of the field on any
    //    `node.value = …` assignment. The user clicks at offset
    //    10 → bridge runs → state re-renders → `prop:value`
    //    re-applies the same string → caret jumps to end.
    //
    // 2. The host's `editor_surface_state.selection` is the source
    //    of truth for where the caret should be after a bridge
    //    round-trip (bridge result rebuilds it from the prior host
    //    selection). If the DOM disagrees with the host (cursor
    //    reset, completion accept that splices text without
    //    matching caret update, scenario load), we need to
    //    actively restore.
    //
    // The effect below is conservative: it reads the host text +
    // selection on every state change, compares to the DOM, and
    // writes only when divergent. Idempotent; cheap when nothing
    // moved. Skipping the effect when the textarea is unmounted
    // (NodeRef::get returns None) keeps the SSR-render path inert.
    let textarea_ref: NodeRef<leptos::html::Textarea> = NodeRef::new();
    Effect::new(move |_| {
        // Subscribe to the state signal so the effect re-runs on
        // every reducer-driven update.
        let (host_text, host_anchor, host_focus) = state.with(|s| {
            let active_id = s
                .workspace_shell
                .active_formula_space_id
                .clone()
                .or_else(|| {
                    s.active_formula_space_view
                        .selected_formula_space_id
                        .clone()
                });
            let space = active_id.as_ref().and_then(|id| s.formula_spaces.get(id));
            let text = space
                .map(|sp| sp.raw_entered_cell_text.clone())
                .unwrap_or_default();
            let anchor = space
                .map(|sp| sp.editor_surface_state.selection.anchor as u32)
                .unwrap_or(0);
            let focus = space
                .map(|sp| sp.editor_surface_state.selection.focus as u32)
                .unwrap_or(0);
            (text, anchor, focus)
        });
        let Some(textarea_el) = textarea_ref.get() else {
            return;
        };
        // Sync text only when divergent. On a match this is a
        // no-op (browser does NOT reset caret because we never
        // assigned to .value).
        if textarea_el.value() != host_text {
            textarea_el.set_value(&host_text);
        }
        // Restore selection from host state when the DOM diverges.
        // After a pure caret-only round-trip (mouse click, arrow
        // navigation), this is the call that pins the caret back
        // to where the click landed even if some upstream prop
        // binding momentarily reset it.
        let dom_anchor = textarea_el.selection_start().ok().flatten();
        let dom_focus = textarea_el.selection_end().ok().flatten();
        if dom_anchor != Some(host_anchor) || dom_focus != Some(host_focus) {
            let _ = textarea_el.set_selection_range(host_anchor, host_focus);
        }
    });

    // Function-help hover state. Component-local because hover is
    // a UI concern that doesn't need to persist into the reducer
    // state. Set by the editor-frame `on:mouseover` delegation
    // handler when the pointer enters a `.syn-fn` span whose name
    // matches the bridge's function-help packet; cleared by the
    // frame's `on:mouseleave` and by an Effect that watches the
    // raw textarea text (any keystroke dismisses the hover).
    //
    // First-version note: the WS-14 plan §2.3 calls for a 400 ms
    // delay before showing the tooltip. v1 ships without the
    // delay (hover shows immediately) — the wiring is what this
    // bead pins; a follow-up bead can layer the delay on without
    // touching the projector or component data flow.
    let hover_target: RwSignal<Option<FunctionHelpHoverTarget>> = RwSignal::new(None);

    // Auto-debounce timer handle. When `apply_live_editor_input`
    // signals `runtime_recalc_pending`, we (re-)arm a `setTimeout`
    // for `AUTO_DEBOUNCE_IDLE_WINDOW_MS` ms; on fire, the timer
    // calls `flush_pending_runtime_recalc` which runs the deferred
    // runtime pass once. Subsequent input events cancel the
    // outstanding timer (if any) before reading the new outcome,
    // so a fast typing burst stays responsive — only the trailing
    // idle window triggers the runtime pass.
    //
    // The handle lives in a `RwSignal<Option<i32>>` so it survives
    // across event-handler closures without an `Rc<RefCell<_>>`.
    let pending_recalc_timer_handle: RwSignal<Option<i32>> = RwSignal::new(None);

    // Helper: arm the idle-window timer that flushes a pending
    // runtime recalc after `AUTO_DEBOUNCE_IDLE_WINDOW_MS` of input
    // silence. Cancels any outstanding timer first so the window
    // is reset by every subsequent keystroke (debounce semantics).
    // No-op on non-wasm targets — host tests drive the flush
    // directly via `flush_pending_runtime_recalc`.
    let editor_bridge_for_flush = editor_bridge.clone();
    let arm_runtime_recalc_flush = move || {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            let Some(window) = web_sys::window() else {
                return;
            };
            // Cancel any in-flight timer so the idle window restarts.
            if let Some(handle) = pending_recalc_timer_handle.get_untracked() {
                window.clear_timeout_with_handle(handle);
            }
            let bridge_for_timer = editor_bridge_for_flush.clone();
            let cb = Closure::once_into_js(move || {
                pending_recalc_timer_handle.set(None);
                state.update(|s| {
                    if let Some(bridge) = bridge_for_timer.as_ref() {
                        if let Err(error) = flush_pending_runtime_recalc(bridge.as_ref(), s) {
                            web_sys::console::warn_1(
                                &format!("[onecalc] runtime flush failed: {error:?}").into(),
                            );
                        }
                    }
                });
            });
            match window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                AUTO_DEBOUNCE_IDLE_WINDOW_MS as i32,
            ) {
                Ok(handle) => {
                    pending_recalc_timer_handle.set(Some(handle));
                }
                Err(error) => {
                    web_sys::console::warn_1(
                        &format!("[onecalc] arming runtime flush timer failed: {error:?}").into(),
                    );
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = editor_bridge_for_flush.clone();
            let _ = pending_recalc_timer_handle;
        }
    };

    // Bridge dispatcher closure shared with the textarea's on:input.
    let editor_bridge_for_input = editor_bridge.clone();
    let arm_flush_for_input = arm_runtime_recalc_flush.clone();
    let on_editor_input = Callback::new(move |event: EditorInputEvent| {
        let mut runtime_recalc_pending = false;
        state.update(|state| {
            if let Some(bridge) = editor_bridge_for_input.as_ref() {
                if let Ok(outcome) = apply_live_editor_input(bridge.as_ref(), state, event) {
                    runtime_recalc_pending = outcome.runtime_recalc_pending;
                }
            } else {
                let _ = apply_editor_input_to_active_formula_space(state, event);
            }
        });
        if runtime_recalc_pending {
            arm_flush_for_input();
        }
    });

    // Helper: apply a CompletionAcceptance — splice the textarea
    // value, build a synthetic input event, and run it through the
    // bridge so proposals / diagnostics / metrics refresh. Used by
    // both the click-to-accept and keyboard-accept paths. Wrapped
    // in a `Callback` so multiple long-lived event listeners can
    // share it without each needing a unique clone of every captured
    // state slot.
    let editor_bridge_for_accept = editor_bridge.clone();
    let apply_acceptance: Callback<CompletionAcceptance> =
        Callback::new(move |acceptance: CompletionAcceptance| {
            let bridge = editor_bridge_for_accept.clone();
            state.update(|state| {
                if let Some(formula_space) = state
                    .workspace_shell
                    .active_formula_space_id
                    .clone()
                    .and_then(|id| state.formula_spaces.get(&id))
                {
                    let new_text = splice_textarea_value(
                        &formula_space.raw_entered_cell_text,
                        acceptance.replacement_span,
                        &acceptance.insert_text,
                    );
                    let event = EditorInputEvent {
                        text: new_text,
                        selection_start: Some(acceptance.new_caret_offset),
                        selection_end: Some(acceptance.new_caret_offset),
                        input_kind: EditorInputKind::InsertText,
                        inserted_text: Some(acceptance.insert_text),
                    };
                    if let Some(bridge) = bridge.as_ref() {
                        let _ = apply_live_editor_input(bridge.as_ref(), state, event);
                    } else {
                        let _ = apply_editor_input_to_active_formula_space(state, event);
                    }
                }
            });
        });

    // Click-to-accept closure for popup rows. Splices the proposal's
    // `insert_text` into the textarea's value at `replacement_span`,
    // moves the caret to the end of the inserted text, dispatches a
    // synthetic input event so the bridge re-runs, then transitions
    // the popup to Hidden via the reducer entry point. The reducer
    // entry point also sets the suppression flag so the bridge
    // refresh that the synthetic input triggers does NOT auto-reopen
    // the popup over the just-accepted proposal.
    let on_completion_click = Callback::new(move |proposal_id: String| {
        let mut acceptance_holder: Option<CompletionAcceptance> = None;
        state.update(|state| {
            acceptance_holder =
                accept_completion_by_proposal_id_on_active_formula_space(state, &proposal_id);
        });
        if let Some(acceptance) = acceptance_holder {
            apply_acceptance.run(acceptance);
        }
    });

    // Keyboard policy. The handler is INSTALLED unconditionally on
    // the textarea, but is a no-op (no preventDefault, no reducer
    // call) when the popup is Hidden — so native textarea behaviour
    // (Arrow / Home / End / Backspace / Delete / IME / clipboard /
    // selection) is preserved verbatim. This is the discipline WS-13
    // got wrong: handlers leaked onto the textarea even when no
    // popup was visible.
    //
    // When the popup IS Open, the handler intercepts ONLY the five
    // popup keys (ArrowUp, ArrowDown, Tab, Enter, Escape) and
    // preventDefault's them. Every other key is allowed through to
    // the textarea unchanged.
    let on_textarea_keydown_inner = move |ev: WebKeyboardEvent| {
        // Workspace view-mode toggle: Ctrl+Alt+D OR Ctrl+Shift+D.
        // Both are accepted because environments differ:
        //   * Ctrl+Alt+D collides with Windows Magnifier's
        //     "dock mode" shortcut on machines where Magnifier
        //     is active or its shortcuts are registered, and the
        //     OS swallows it before the browser sees it.
        //   * Ctrl+Shift+D is bound by Chrome / Edge to "Bookmark
        //     all tabs" but the page's keydown listener fires
        //     first, so preventDefault here prevents the dialog.
        // Either chord works; the status-foot button (rendered
        // always) is the discoverable fallback for users who
        // don't reach for chords.
        if ev.ctrl_key() && (ev.alt_key() || ev.shift_key()) && ev.key().eq_ignore_ascii_case("d") {
            ev.prevent_default();
            state.update(|state| {
                let _ = toggle_view_mode_on_workspace(state);
            });
            return;
        }

        // Ctrl+D (no Shift, no Alt) toggles the formula
        // drill-down. Handled BEFORE the popup-open early-return
        // because the chord is global — works whether the popup
        // is open or closed. preventDefault shadows the browser's
        // native bookmark-this-page behaviour. The shift_key /
        // alt_key gates ensure Ctrl+Shift+D and Ctrl+Alt+D fall
        // through to the view-mode toggle above rather than
        // accidentally toggling the drill.
        if ev.ctrl_key() && !ev.shift_key() && !ev.alt_key() && ev.key().eq_ignore_ascii_case("d") {
            ev.prevent_default();
            state.update(|state| {
                let _ = toggle_formula_drill_on_active_formula_space(state);
            });
            return;
        }

        // (F9 handling moved to the outer shell `on:keydown` so
        // it works even when focus is outside the textarea — see
        // the shell-level handler below.)

        // Read popup-open state directly from the source signal, NOT
        // via the `view_model` memo. The memo recomputes lazily and
        // synthetic-event keystrokes fire inside `dispatchEvent`
        // synchronously — there is no microtask boundary between the
        // last reducer-driven state mutation and the keydown handler,
        // so the memo's cached value can be one tick behind. Reading
        // the popup state straight off `FormulaSpaceState` sidesteps
        // any memo staleness.
        let popup_open = state.with_untracked(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .map(|fs| {
                    matches!(
                        fs.completion_popup,
                        crate::services::completion_popup::CompletionPopupState::Open { .. }
                    )
                })
                .unwrap_or(false)
        });
        if !popup_open {
            return;
        }
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = move_completion_popup_selection_on_active_formula_space(state, 1);
                });
            }
            "ArrowUp" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = move_completion_popup_selection_on_active_formula_space(state, -1);
                });
            }
            "Tab" | "Enter" => {
                ev.prevent_default();
                let mut acceptance_holder: Option<CompletionAcceptance> = None;
                state.update(|state| {
                    acceptance_holder =
                        accept_selected_completion_with_suppression_on_active_formula_space(state);
                });
                if let Some(acceptance) = acceptance_holder {
                    apply_acceptance.run(acceptance);
                }
            }
            "Escape" => {
                ev.prevent_default();
                state.update(|state| {
                    let _ = dismiss_completion_popup_on_active_formula_space(state);
                });
            }
            _ => {
                // All other keys (Arrow Left/Right, plain typing, IME
                // composition, clipboard shortcuts) fall through to
                // the textarea's native handling. NO preventDefault.
            }
        }
    };
    // Wrap the keydown closure in a Callback so the host shell's
    // section render closure (called multiple times during reactive
    // re-renders) can pass it via `on:keydown` without consuming it
    // — `Callback` is `Copy`, the inner closure is not (it captures
    // the bridge `Arc` and other non-Copy state).
    let on_textarea_keydown: Callback<WebKeyboardEvent> = Callback::new(on_textarea_keydown_inner);

    // Focus-out: when the textarea loses focus (user clicks
    // elsewhere, Tab navigates away, ...) dismiss the popup so it
    // doesn't sit stale on an unfocused editor.
    let on_textarea_focusout = move |_| {
        state.update(|state| {
            let _ = dismiss_completion_popup_on_active_formula_space(state);
        });
    };

    // Reactive readers. Each closure runs whenever the underlying signal
    // it touches changes; Leptos handles the diff.
    let textarea_value = move || {
        view_model
            .get()
            .map(|vm| vm.raw_entered_cell_text)
            .unwrap_or_default()
    };
    let has_active_formula_space = move || view_model.get().is_some();
    let entry_mode_pill = move || view_model.get().map(|vm| vm.entry_mode_pill);
    let result_class_pill = move || view_model.get().and_then(|vm| vm.result_class_pill);
    let syntax_runs = move || {
        view_model
            .get()
            .map(|vm| vm.syntax_runs)
            .unwrap_or_default()
    };
    // WS-14 plan §2.3: bracket-pair highlight at the caret. The matcher
    // returns the open / close offsets that pair under the cursor; the
    // syntax overlay surfaces them as `data-bracket-active="true"` on the
    // matching delimiter spans. Returns `None` when the caret is not
    // adjacent to a bracket or the brackets are unbalanced — the
    // highlight simply turns off in that case.
    let bracket_pair_highlight = move || {
        view_model.get().and_then(|vm| {
            crate::ui::editor::bracket_matcher::bracket_pair_for_caret(
                &vm.raw_entered_cell_text,
                vm.editor_surface_state.caret.offset,
            )
        })
    };
    let diagnostic_squiggles = move || {
        view_model
            .get()
            .map(|vm| vm.diagnostic_squiggles)
            .unwrap_or_default()
    };
    let editor_metrics = move || view_model.get().map(|vm| vm.editor_metrics);
    // The view-model returns `Option<ResultContextChip>` directly
    // (the chip collapses on default-state formulas — see
    // `project_result_context`). Flatten through `and_then` so the
    // renderer's `Option<ResultContextChip>` parameter has a single
    // None for "no active formula" or "default formula".
    let result_context = move || view_model.get().and_then(|vm| vm.result_context);
    let formula_tab_strip = move || view_model.get().map(|vm| vm.formula_tab_strip);
    let command_palette = move || view_model.get().map(|vm| vm.command_palette);
    let manage_formulas = move || view_model.get().map(|vm| vm.manage_formulas);
    let completion_popup = move || view_model.get().and_then(|vm| vm.completion_popup);
    let signature_help = move || view_model.get().and_then(|vm| vm.signature_help);
    let function_help_card = move || view_model.get().and_then(|vm| vm.function_help_card);
    let hover_target_for_render = hover_target;
    let function_help_hover = move || hover_target_for_render.get();
    let formula_drill = move || view_model.get().map(|vm| vm.formula_drill);
    let capability_context = move || view_model.get().map(|vm| vm.capability_context);
    let result_view = move || view_model.get().map(|vm| vm.result_view);
    let status_view = move || view_model.get().map(|vm| vm.status);
    let scenario_breadcrumb = move || view_model.get().map(|vm| vm.scenario_breadcrumb);
    let formatting_controls = move || view_model.get().map(|vm| vm.formatting_controls);
    let view_mode = move || {
        view_model
            .get()
            .map(|vm| vm.view_mode)
            .unwrap_or(ViewMode::User)
    };
    let skin_schema = move || {
        view_model
            .get()
            .map(|vm| vm.skin_snapshot.schema_id)
            .unwrap_or_else(|| "dnacalc.skin_ir.none".to_string())
    };
    let skin_document_kind = move || {
        view_model
            .get()
            .map(|vm| match vm.skin_snapshot.document {
                dnacalc_skin_ir::SkinDocumentProjection::OneFormula(_) => "one_formula",
                dnacalc_skin_ir::SkinDocumentProjection::TreeWorkspace(_) => "tree_workspace",
            })
            .unwrap_or("none")
    };
    let skin_reference_capability = move || {
        view_model
            .get()
            .map(|vm| match vm.skin_snapshot.host_capabilities.references {
                dnacalc_skin_ir::ReferenceCapabilityProjection::Absent => "absent",
                dnacalc_skin_ir::ReferenceCapabilityProjection::TreeWorkspace => "tree_workspace",
                dnacalc_skin_ir::ReferenceCapabilityProjection::ExternalProvider => {
                    "external_provider"
                }
                dnacalc_skin_ir::ReferenceCapabilityProjection::Unsupported => "unsupported",
            })
            .unwrap_or("none")
    };
    let shared_formula_projection = move || {
        view_model
            .get()
            .and_then(|vm| match vm.skin_snapshot.document {
                dnacalc_skin_ir::SkinDocumentProjection::OneFormula(projection) => Some(projection),
                dnacalc_skin_ir::SkinDocumentProjection::TreeWorkspace(_) => None,
            })
    };
    let on_shared_formula_intent = Callback::new(move |intent| {
        state.update(|state| {
            let _ = crate::app::reducer::apply_skin_intent_to_host_state(state, intent);
        });
    });

    // Trigger row callback shared between the editor-foot toggle
    // and the keyboard chord — both routes through the same
    // reducer entry so the test corpus can pin behaviour
    // identically regardless of input.
    let on_formula_drill_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_formula_drill_on_active_formula_space(state);
        });
    });

    // View-mode toggle callback — used by both the status-foot
    // button (mouse) and the Ctrl+Alt+D / Ctrl+Shift+D chords
    // (keyboard).
    let on_view_mode_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_view_mode_on_workspace(state);
        });
    });

    // Slice 5 — formatting-control callbacks. Each setter dispatches
    // to the matching reducer AND, on a real change, re-runs the live
    // bridge so the publication-surface
    // `effective_display_text` produced by the new format code flows
    // straight into the result hero. Without the post-mutation bridge
    // refresh the formatting only takes effect on the next keystroke,
    // which feels broken — the user clicks "Currency" and nothing
    // changes until they type something.
    let editor_bridge_for_formatting = editor_bridge.clone();
    let refresh_after_formatting_change = move |state: &mut crate::state::OneCalcHostState| {
        if let Some(bridge) = editor_bridge_for_formatting.as_ref() {
            let _ =
                crate::services::live_edit::refresh_active_formula_space(bridge.as_ref(), state);
        }
    };
    let on_set_number_format_code = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_number_format_code(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_font_color = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_font_color(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_fill_color = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: String| {
            state.update(|s| {
                if crate::app::reducer::set_active_fill_color(s, value) {
                    refresh(s);
                }
            });
        })
    };
    let on_set_date1904 = {
        let refresh = refresh_after_formatting_change.clone();
        Callback::new(move |value: bool| {
            state.update(|s| {
                if crate::app::reducer::set_active_date1904(s, value) {
                    refresh(s);
                }
            });
        })
    };
    // WS-14 plan §5.3, item 8: collapsible formatting panel above the
    // result section. Click the summary chip to expand the full
    // formatting controls row; click again to collapse back. The
    // reducer flips `formula_space.formatting_panel_open`; the view
    // model lifts that flag onto `FormattingControlsView.is_open`.
    let on_formatting_panel_toggle = Callback::new(move |()| {
        state.update(|s| {
            let _ = crate::app::reducer::toggle_formatting_panel_on_active_formula_space(s);
        });
    });
    // Calc-options + CF rule callbacks. Each chains a bridge refresh
    // when the underlying state actually changed, so the result hero
    // updates live (same pattern as `refresh_after_formatting_change`
    // for the per-field setters above).
    let editor_bridge_for_calc_opts = editor_bridge.clone();
    let refresh_after_calc_opts_change = move |state: &mut crate::state::OneCalcHostState| {
        if let Some(bridge) = editor_bridge_for_calc_opts.as_ref() {
            let _ =
                crate::services::live_edit::refresh_active_formula_space(bridge.as_ref(), state);
        }
    };
    let on_set_scenario_policy = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |policy: crate::persistence::ScenarioPolicy| {
            state.update(|s| {
                if crate::app::reducer::set_active_scenario_policy(s, policy) {
                    refresh(s);
                }
            });
        })
    };
    // Workspace locale preset. Drives the date / datetime / time
    // format-code defaults applied to General-format result heroes
    // via the presentation hint AND the runtime `LocaleFormatContext`
    // built in `live_bridge::build_runtime_locale_context`. With
    // OxFunc W094 + OxFml's locale-context wiring in place, switching
    // the dropdown to e.g. `de-DE` now flips both the presentation
    // hint defaults *and* the runtime month / weekday tables, decimal
    // / thousands separators, currency symbol, and `General` rendering
    // for the active formula. (Was `SEAM-OXFML-LOCALE-EXPAND`.)
    let on_set_locale_preset = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |language_tag: String| {
            state.update(|s| {
                if crate::app::reducer::set_workspace_locale_preset(s, language_tag) {
                    refresh(s);
                }
            });
        })
    };
    let on_add_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |()| {
            state.update(|s| {
                let default_rule = crate::state::FormulaConditionalFormattingRule {
                    rule_kind: "cell_value".to_string(),
                    operator: Some("greaterThan".to_string()),
                    thresholds: vec!["0".to_string()],
                    font_color: None,
                    fill_color: Some("#ffe9b3".to_string()),
                    typed_rule: None,
                };
                if crate::app::reducer::add_active_conditional_formatting_rule(s, default_rule)
                    .is_some()
                {
                    refresh(s);
                }
            });
        })
    };
    let on_remove_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(move |index: usize| {
            state.update(|s| {
                if crate::app::reducer::remove_active_conditional_formatting_rule(s, index) {
                    refresh(s);
                }
            });
        })
    };
    let on_update_cf_rule = {
        let refresh = refresh_after_calc_opts_change.clone();
        Callback::new(
            move |(index, rule): (usize, crate::state::FormulaConditionalFormattingRule)| {
                state.update(|s| {
                    if crate::app::reducer::update_active_conditional_formatting_rule(
                        s, index, rule,
                    ) {
                        refresh(s);
                    }
                });
            },
        )
    };
    let on_vba_project_path_input = Callback::new(move |path: String| {
        state.update(|s| {
            let _ = crate::app::reducer::set_pending_vba_project_path(s, path);
        });
    });
    let on_vba_project_path_add = Callback::new(move |()| {
        state.update(|s| {
            let _ = crate::app::reducer::add_pending_vba_project_path(s);
        });
    });
    let editor_bridge_for_vba = editor_bridge.clone();
    let on_vba_module_file_loaded = Callback::new(move |request: VbaModuleSourceLoadRequest| {
        let mut loaded_runtime = None;
        state.update(|s| {
            loaded_runtime =
                crate::app::reducer::load_vba_module_source_for_host_context(s, request);
        });
        // Install the compiled VBA runtime in the editor bridge so
        // subsequent formula evaluations can resolve and invoke UDFs.
        if let (Some(runtime), Some(bridge)) = (loaded_runtime, editor_bridge_for_vba.as_ref()) {
            bridge.install_vba_runtime(runtime);
        }
    });
    let editor_bridge_for_vba_remove = editor_bridge.clone();
    let on_vba_association_remove = Callback::new(move |association_id: String| {
        let mut removed = false;
        state.update(|s| {
            removed = crate::app::reducer::remove_vba_host_association(s, &association_id);
        });
        if removed {
            if let Some(bridge) = editor_bridge_for_vba_remove.as_ref() {
                bridge.clear_vba_runtime();
            }
        }
    });
    // Manual recalculate trigger. Wired both to the F9 key
    // (handled in `on_textarea_keydown`) and to a small button in
    // the editor-foot row. In Deterministic policy this re-runs
    // the bridge against the same fixed seeds; in LiveRecalc the
    // bridge picks fresh seeds so volatile functions advance. In
    // ManualRecalc this is the *only* path that runs the runtime
    // pass — the keystroke-driven bridge skips it.
    let editor_bridge_for_recalc = editor_bridge.clone();
    let on_recalculate = Callback::new(move |()| {
        let bridge = editor_bridge_for_recalc.clone();
        state.update(|state| {
            if let Some(bridge) = bridge.as_ref() {
                let _ = crate::services::live_edit::force_runtime_recalc_on_active_formula_space(
                    bridge.as_ref(),
                    state,
                );
            }
        });
    });

    // Scenario breadcrumb dropdown lifecycle. The toggle is wired
    // to the breadcrumb button click. The close callback fires
    // from outside-click delegation in the shell and from `Esc`
    // in the global keydown handler.
    let on_scenario_breadcrumb_toggle = Callback::new(move |()| {
        state.update(|state| {
            let _ = toggle_scenario_breadcrumb(state);
        });
    });
    let on_scenario_breadcrumb_close = Callback::new(move |()| {
        state.update(|state| {
            let _ = close_scenario_breadcrumb(state);
        });
    });
    // Click on a Recent / Pinned row → switch to that formula. The
    // dropdown closes after the switch so the user gets visible
    // feedback. `reopen_formula_space` handles both the
    // open-but-not-active case (just flip active id) and the
    // closed-and-recent case (re-mount from `recent_formula_spaces`).
    let on_scenario_entry_select = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::reopen_formula_space(state, &formula_space_id);
            let _ = close_scenario_breadcrumb(state);
        });
    });
    // Pin glyph on a Recent / Pinned row → toggle the pin without
    // switching the active formula. Stops click propagation in the
    // renderer so the row's select handler doesn't also fire.
    let on_scenario_entry_pin_toggle = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::toggle_pin_formula_space(state, &formula_space_id);
        });
    });
    // Tab-strip close button (per WS-14 §1 minimum-viable surface).
    // Closing a tab routes through `close_formula_space`, which also
    // handles the "last formula closed" case by spinning a fresh
    // `untitled-N` so the editor never has nothing to mount against.
    let on_close_formula_tab = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::close_formula_space(state, &formula_space_id);
        });
    });
    // Tab-strip `+ new formula` button — alias for Ctrl+N. The
    // breadcrumb dropdown's `New formula` action does the same
    // thing; the tab-strip surface puts it in click reach without
    // having to open the dropdown.
    let on_new_formula_from_tab_strip = Callback::new(move |()| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::new_formula_space(state);
        });
    });
    // Inline-rename callbacks. The user double-clicks a tab name
    // to start; types into the input; presses Enter (or blurs) to
    // commit, or Esc to cancel. Both the active formula and any
    // pinned formula can be renamed this way — there's no separate
    // "rename pinned" path.
    let on_begin_rename_formula_tab = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::begin_formula_rename(state, &formula_space_id);
        });
    });
    let on_update_rename_text = Callback::new(move |next_text: String| {
        state.update(|state| {
            crate::app::case_lifecycle::update_pending_rename_text(state, next_text);
        });
    });
    let on_commit_rename = Callback::new(move |()| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::commit_formula_rename(state);
        });
    });
    let on_cancel_rename = Callback::new(move |()| {
        state.update(|state| {
            crate::app::case_lifecycle::cancel_formula_rename(state);
        });
    });
    // Manage-formulas overlay callbacks. The overlay is a single
    // surface for browsing / searching every formula in the
    // workspace and acting on each one without paging through the
    // breadcrumb dropdown. All actions either re-use existing
    // reducers or compose them; the overlay is purely a view + a
    // search-filter on the same data.
    let on_close_manage_formulas = Callback::new(move |()| {
        state.update(|state| {
            let _ = crate::app::reducer::close_manage_formulas(state);
        });
    });
    let on_manage_formulas_search = Callback::new(move |query: String| {
        state.update(|state| {
            let _ = crate::app::reducer::set_manage_formulas_search_query(state, query);
        });
    });
    let on_manage_formulas_open = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::reopen_formula_space(state, &formula_space_id);
            let _ = crate::app::reducer::close_manage_formulas(state);
        });
    });
    let on_manage_formulas_rename = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            // Switch to the formula first so the inline-rename input
            // appears on the active tab — otherwise the rename target
            // would be off-screen.
            let _ = crate::app::case_lifecycle::reopen_formula_space(state, &formula_space_id);
            let _ = crate::app::case_lifecycle::begin_formula_rename(state, &formula_space_id);
            let _ = crate::app::reducer::close_manage_formulas(state);
        });
    });
    let on_manage_formulas_toggle_pin = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            if state.workspace_shell.pinned_formula_space_ids.contains(
                &crate::domain::ids::FormulaSpaceId::new(formula_space_id.clone()),
            ) {
                let _ = crate::app::case_lifecycle::unpin_formula_space(state, &formula_space_id);
            } else {
                let _ = crate::app::case_lifecycle::pin_formula_space(state, &formula_space_id);
            }
        });
    });
    let on_manage_formulas_clone = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            // Reopen first if it's a recent (clone needs the source
            // to live in `formula_spaces`); reopen is idempotent for
            // already-open ids.
            let _ = crate::app::case_lifecycle::reopen_formula_space(state, &formula_space_id);
            let _ = crate::app::case_lifecycle::duplicate_formula_space(state, &formula_space_id);
            let _ = crate::app::reducer::close_manage_formulas(state);
        });
    });
    let on_manage_formulas_close = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ = crate::app::case_lifecycle::close_formula_space(state, &formula_space_id);
        });
    });
    let on_manage_formulas_forget = Callback::new(move |formula_space_id: String| {
        state.update(|state| {
            let _ =
                crate::app::case_lifecycle::forget_recent_formula_space(state, &formula_space_id);
        });
    });
    // Command-palette callbacks. Type-to-filter, arrow keys to
    // move selection, Enter to dispatch. Esc / outside-click /
    // a second Ctrl+K closes the overlay.
    let on_command_palette_query = Callback::new(move |query: String| {
        state.update(|state| {
            let _ = crate::app::reducer::set_command_palette_query(state, query);
        });
    });
    let editor_bridge_for_palette = editor_bridge.clone();
    let on_command_palette_dispatch = Callback::new(move |kind: CommandPaletteEntryKind| {
        let bridge = editor_bridge_for_palette.clone();
        state.update(|state| {
            // Always close the palette after dispatch — the user
            // got the command they came for.
            let _ = crate::app::reducer::close_command_palette(state);
            match kind {
                CommandPaletteEntryKind::SwitchFormula(id) => {
                    let _ = crate::app::case_lifecycle::reopen_formula_space(state, &id);
                }
                CommandPaletteEntryKind::ScenarioAction(action_id) => {
                    use crate::services::home_shell_view_model::ScenarioBreadcrumbActionId;
                    match action_id {
                        ScenarioBreadcrumbActionId::NewScenario => {
                            let _ = crate::app::case_lifecycle::new_formula_space(state);
                        }
                        ScenarioBreadcrumbActionId::Duplicate => {
                            let _ = crate::app::case_lifecycle::clone_active_formula_space(
                                state,
                            );
                        }
                        ScenarioBreadcrumbActionId::RenameActive => {
                            if let Some(active_id) =
                                state.workspace_shell.active_formula_space_id.clone()
                            {
                                let _ = crate::app::case_lifecycle::begin_formula_rename(
                                    state,
                                    active_id.as_str(),
                                );
                            }
                        }
                        ScenarioBreadcrumbActionId::PinActive => {
                            let _ = crate::app::case_lifecycle::pin_active_formula_space(state);
                        }
                        ScenarioBreadcrumbActionId::UnpinActive => {
                            if let Some(active_id) =
                                state.workspace_shell.active_formula_space_id.clone()
                            {
                                let _ = crate::app::case_lifecycle::unpin_formula_space(
                                    state,
                                    active_id.as_str(),
                                );
                            }
                        }
                        ScenarioBreadcrumbActionId::ManageScenarios => {
                            let _ = crate::app::reducer::open_manage_formulas(state);
                        }
                        // SaveAs / Open dispatch through the breadcrumb's
                        // own async paths (file picker / write); the
                        // palette mirrors them for discoverability but
                        // the file-IO routing belongs in the breadcrumb
                        // handler. Console-log so the user sees the
                        // click was received.
                        ScenarioBreadcrumbActionId::SaveAs | ScenarioBreadcrumbActionId::Open => {
                            #[cfg(target_arch = "wasm32")]
                            web_sys::console::log_1(
                                &format!(
                                    "[onecalc] palette dispatch {action_id:?}: pending palette-side wiring",
                                )
                                .into(),
                            );
                        }
                    }
                }
                CommandPaletteEntryKind::ToggleFormattingPanel => {
                    let _ = crate::app::reducer::toggle_formatting_panel_on_active_formula_space(
                        state,
                    );
                }
                CommandPaletteEntryKind::ToggleFormulaDrill => {
                    let _ =
                        crate::app::reducer::toggle_formula_drill_on_active_formula_space(state);
                }
                CommandPaletteEntryKind::ForceRecalc => {
                    if let Some(bridge) = bridge.as_ref() {
                        let _ =
                            crate::services::live_edit::force_runtime_recalc_on_active_formula_space(
                                bridge.as_ref(),
                                state,
                            );
                    }
                }
            }
        });
    });
    // Scenario action dispatcher (slice 1b). NewScenario / Duplicate
    // run synchronously through their existing reducers. SaveAs
    // projects the active formula to the persisted `Scenario` shape,
    // serialises to XML, and triggers a browser-native download.
    // Open spawns an async task that surfaces the file picker, reads
    // the chosen file, parses it, and inserts it into the workspace.
    // ManageScenarios is still a SEAM stub (no UI for the manage
    // page yet — that's a later slice).
    let on_scenario_action = Callback::new(move |action_id: ScenarioBreadcrumbActionId| {
        // Always close the dropdown so the user gets visible feedback
        // that the click was received, regardless of which action.
        let close_dropdown = || {
            state.update(|state| {
                let _ = close_scenario_breadcrumb(state);
            });
        };

        match action_id {
            ScenarioBreadcrumbActionId::NewScenario => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::new_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::Duplicate => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::clone_active_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::RenameActive => {
                state.update(|state| {
                    if let Some(active_id) = state.workspace_shell.active_formula_space_id.clone() {
                        let _ = crate::app::case_lifecycle::begin_formula_rename(
                            state,
                            active_id.as_str(),
                        );
                    }
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::PinActive => {
                state.update(|state| {
                    let _ = crate::app::case_lifecycle::pin_active_formula_space(state);
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::UnpinActive => {
                state.update(|state| {
                    if let Some(active_id) = state.workspace_shell.active_formula_space_id.clone() {
                        let _ = crate::app::case_lifecycle::unpin_formula_space(
                            state,
                            active_id.as_str(),
                        );
                    }
                });
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::SaveAs => {
                #[cfg(target_arch = "wasm32")]
                {
                    let payload = state.with_untracked(|s| build_save_payload(s));
                    if let Some((filename, xml)) = payload {
                        match crate::persistence::save_xml_via_download(&filename, &xml) {
                            Ok(()) => {
                                // Save established the canonical
                                // dna: extension on disk, so any
                                // "imported from Excel-only" warning
                                // is no longer accurate. Clear it.
                                state.update(|s| {
                                    if let Some(active_id) =
                                        s.workspace_shell.active_formula_space_id.clone()
                                    {
                                        if let Some(formula_space) =
                                            s.formula_spaces.get_mut(&active_id)
                                        {
                                            formula_space.load_diagnostics.clear();
                                        }
                                    }
                                });
                            }
                            Err(error) => {
                                web_sys::console::error_1(
                                    &format!("[onecalc] save failed: {error}").into(),
                                );
                            }
                        }
                    }
                }
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::Open => {
                #[cfg(target_arch = "wasm32")]
                {
                    let state = state;
                    wasm_bindgen_futures::spawn_local(async move {
                        match crate::persistence::open_xml_via_file_input().await {
                            Ok(Some(opened)) => {
                                match crate::persistence::read_formula_xml(&opened.xml) {
                                    Ok(loaded) => {
                                        state.update(|s| {
                                        let _ =
                                            crate::app::case_lifecycle::open_loaded_scenario_into_workspace(
                                                s, loaded,
                                            );
                                    });
                                    }
                                    Err(error) => {
                                        web_sys::console::error_1(
                                            &format!(
                                                "[onecalc] failed to parse `{}`: {error}",
                                                opened.filename,
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            }
                            Ok(None) => {
                                // user cancelled — no-op
                            }
                            Err(error) => {
                                web_sys::console::error_1(
                                    &format!("[onecalc] open dialog failed: {error}").into(),
                                );
                            }
                        }
                    });
                }
                close_dropdown();
            }
            ScenarioBreadcrumbActionId::ManageScenarios => {
                // Open the manage-formulas overlay (the searchable
                // browse-everything-in-the-workspace surface). Closing
                // the dropdown happens implicitly: the overlay's
                // backdrop and Esc handler call `close_manage_formulas`,
                // and the breadcrumb dropdown closes via the same
                // outside-click path the overlay's backdrop already
                // covers. Bulk operations + drag-reorder are open
                // follow-ups; the row-by-row v1 covers the searchable
                // workspace browser the WS-14 plan called for.
                state.update(|state| {
                    let _ = crate::app::reducer::open_manage_formulas(state);
                });
                close_dropdown();
            }
        }
    });

    // Editor-frame mouseover delegation: when the pointer is over a
    // `.syn-fn` span whose `data-token-text` matches the bridge's
    // current `function_help.lookup_key`, surface a hover target.
    // Non-function spans are ignored. We compute the anchor via
    // `caret_box_for_offset(token_start, metrics)` rather than
    // reading the span's bounding-client-rect, so the tooltip
    // stays at the same pixel position the syntax overlay
    // measured for that token (deterministic across reflows).
    let on_overlay_mouseover = move |ev: WebMouseEvent| {
        let target = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        let Some(target) = target else {
            return;
        };
        if target.get_attribute("data-token-role").as_deref() != Some("function") {
            return;
        }
        let Some(token_text) = target.get_attribute("data-token-text") else {
            return;
        };
        let Some(token_start) = target
            .get_attribute("data-token-start")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            return;
        };
        let card_lookup_key = view_model.with_untracked(|vm| {
            vm.as_ref()
                .and_then(|vm| vm.function_help_card.as_ref().map(|c| c.lookup_key.clone()))
        });
        let Some(lookup_key) = card_lookup_key else {
            return;
        };
        if !lookup_key.eq_ignore_ascii_case(&token_text) {
            return;
        }
        let anchor = state.with_untracked(|s| {
            let formula_space = s
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))?;
            let metrics = formula_space.editor_box_metrics?;
            let anchor =
                caret_box_for_offset(&formula_space.raw_entered_cell_text, token_start, metrics);
            Some((anchor.left_px, anchor.top_px, metrics.line_height_px.max(1)))
        });
        let Some((anchor_left_px, anchor_top_px, line_height_px)) = anchor else {
            return;
        };
        hover_target.set(Some(FunctionHelpHoverTarget {
            token_text,
            anchor_left_px,
            anchor_top_px,
            line_height_px,
        }));
    };

    let hover_target_for_clear = hover_target;
    let on_overlay_mouseleave = move |_ev: WebMouseEvent| {
        hover_target_for_clear.set(None);
    };

    // Any input change dismisses the hover — once the user types,
    // the formula structure under the pointer might be stale.
    let hover_target_for_effect = hover_target;
    Effect::new(move |prev: Option<String>| {
        let current = view_model
            .get()
            .map(|vm| vm.raw_entered_cell_text)
            .unwrap_or_default();
        if let Some(prev_value) = prev.as_ref() {
            if prev_value != &current {
                hover_target_for_effect.set(None);
            }
        }
        current
    });
    // Browser-measured caret-box metrics surfaced as data-attributes on
    // the editor frame. The corpus uses these to assert that
    // measurement actually happened on the first keystroke.
    let editor_box_char_width = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .and_then(|fs| fs.editor_box_metrics.map(|m| m.char_width_px))
        })
    };
    let editor_box_line_height = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .and_then(|fs| fs.editor_box_metrics.map(|m| m.line_height_px))
        })
    };
    let editor_box_measure_tick = move || {
        state.with(|s| {
            s.workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| s.formula_spaces.get(id))
                .map(|fs| fs.editor_box_metrics_tick)
                .unwrap_or(0)
        })
    };

    view! {
        <ThemeStyleTag />
        <div
            class="onecalc-home-shell"
            data-view-mode=move || view_mode().slug()
            data-skin-schema=skin_schema
            data-skin-document-kind=skin_document_kind
            data-skin-reference-capability=skin_reference_capability
            on:keydown=move |ev: WebKeyboardEvent| {
                if ev.key() == "Escape"
                    && state.with_untracked(|s| s.global_ui_chrome.scenario_breadcrumb_open)
                {
                    on_scenario_breadcrumb_close.run(());
                    return;
                }
                // F9 — recalculate the active formula. Bound at the
                // shell level (rather than the textarea) so it works
                // when focus is on the formatting panel, the
                // recalc button, or anywhere else inside the shell.
                // `preventDefault` shadows Firefox's "find again"
                // browser default.
                if !ev.ctrl_key()
                    && !ev.shift_key()
                    && !ev.alt_key()
                    && ev.key() == "F9"
                {
                    ev.prevent_default();
                    on_recalculate.run(());
                    return;
                }
                // Ctrl+K — command palette (placeholder; opens once
                // the palette UI lands). Ctrl+P collides with the
                // browser's print dialog, so the canonical chord is
                // Ctrl+K (modern app convention) plus Ctrl+Shift+P
                // as a discoverable secondary chord. Today both are
                // wired but produce a no-op until
                // `services::command_palette` lands.
                if ev.ctrl_key()
                    && !ev.alt_key()
                    && (ev.key() == "k"
                        || ev.key() == "K"
                        || (ev.shift_key() && (ev.key() == "p" || ev.key() == "P")))
                {
                    ev.prevent_default();
                    // Ctrl+K toggles the command palette. The chord
                    // also closes it on a second press, mirroring
                    // VS Code / Linear / GitHub.
                    state.update(|state| {
                        let _ = crate::app::reducer::toggle_command_palette(state);
                    });
                    return;
                }
            }
            on:click=move |ev: WebMouseEvent| {
                if !state.with_untracked(|s| s.global_ui_chrome.scenario_breadcrumb_open) {
                    return;
                }
                let inside_breadcrumb = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .and_then(|el| el.closest(".onecalc-home-shell__breadcrumb-wrap").ok().flatten())
                    .is_some();
                if !inside_breadcrumb {
                    on_scenario_breadcrumb_close.run(());
                }
            }
        >
            <header class="onecalc-home-shell__titlebar">
                <span class="onecalc-home-shell__brand">"DnaOneCalc"</span>
                {move || render_scenario_breadcrumb(
                    scenario_breadcrumb(),
                    on_scenario_breadcrumb_toggle,
                    on_scenario_breadcrumb_close,
                    on_scenario_action,
                    on_scenario_entry_select,
                    on_scenario_entry_pin_toggle,
                )}
                // Command-palette button. Ctrl+P / Ctrl+K collide
                // with browser-reserved chords (print / search), so
                // a click target in the chrome is the reliable
                // entry point. The keyboard chord still works as a
                // best-effort accelerator on Tauri / dev tools but
                // is no longer the primary surface.
                <button
                    type="button"
                    class="onecalc-home-shell__titlebar-action"
                    title="Command palette"
                    aria-label="open command palette"
                    on:click=move |_| {
                        state.update(|state| {
                            let _ = crate::app::reducer::toggle_command_palette(state);
                        });
                    }
                >
                    <span class="onecalc-home-shell__titlebar-action-glyph" aria-hidden="true">"⌘"</span>
                    <span class="onecalc-home-shell__titlebar-action-label">"Command palette"</span>
                </button>
            </header>

            {move || render_formula_tab_strip(
                formula_tab_strip(),
                on_scenario_entry_select,
                on_close_formula_tab,
                on_new_formula_from_tab_strip,
                on_begin_rename_formula_tab,
                on_update_rename_text,
                on_commit_rename,
                on_cancel_rename,
            )}

            <section
                class="onecalc-home-shell__shared-formula-surface"
                aria-label="shared formula surface"
                data-skin-driven="true"
            >
                {move || shared_formula_projection().map(|projection| view! {
                    <FormulaSurface
                        projection=projection
                        on_intent=on_shared_formula_intent
                    />
                })}
            </section>

            <main class="onecalc-home-shell__body">
                <Show
                    when=has_active_formula_space
                    fallback=|| view! {
                        <p class="onecalc-home-shell__no-formula-space">
                            "No active formula space."
                        </p>
                    }
                >
                    <section class="onecalc-home-shell__editor">
                        <div class="onecalc-home-shell__caption-row">
                            <span class="onecalc-home-shell__caption">"formula ▸"</span>
                            {move || render_entry_mode_pill(entry_mode_pill())}
                        </div>
                        <div
                            class="onecalc-home-shell__editor-frame"
                            data-char-width=move || {
                                editor_box_char_width()
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "0".to_string())
                            }
                            data-line-height=move || {
                                editor_box_line_height()
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "0".to_string())
                            }
                            data-measure-tick=move || editor_box_measure_tick().to_string()
                            on:mouseover=on_overlay_mouseover
                            on:mouseleave=on_overlay_mouseleave
                        >
                            <div
                                class="onecalc-home-shell__editor-overlay"
                                aria-hidden="true"
                            >
                                {move || render_syntax_overlay(
                                    syntax_runs(),
                                    textarea_value(),
                                    bracket_pair_highlight(),
                                )}
                            </div>
                            <div
                                class="onecalc-home-shell__editor-squiggles"
                                aria-hidden="true"
                            >
                                {move || render_diagnostic_squiggle_overlay(
                                    diagnostic_squiggles(),
                                    textarea_value(),
                                )}
                            </div>
                            <textarea
                                class="onecalc-home-shell__textarea"
                                spellcheck="false"
                                autocomplete="off"
                                aria-label="formula editor"
                                node_ref=textarea_ref
                                on:keydown=move |ev| on_textarea_keydown.run(ev)
                                on:focusout=on_textarea_focusout
                                on:keyup=move |ev: WebKeyboardEvent| {
                                    // Caret-only navigation keys (arrows, Home,
                                    // End, PageUp/Down) don't fire `on:input` —
                                    // browser moves the caret natively. Fire a
                                    // synthetic `EditorInputEvent` with the
                                    // current text + new selection so the
                                    // bridge re-runs and popups update against
                                    // the new caret position. Filtering by key
                                    // avoids double-firing on text-input keys
                                    // (those go through `on:input`).
                                    if !is_caret_navigation_key(&ev.key()) {
                                        return;
                                    }
                                    if let Some(textarea) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
                                    {
                                        on_editor_input.run(synthesize_caret_sync_event(&textarea));
                                    }
                                }
                                on:click=move |ev: WebMouseEvent| {
                                    // Mouse-click positions the caret. Same
                                    // synthesis path so popups reflect the new
                                    // caret position.
                                    if let Some(textarea) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
                                    {
                                        on_editor_input.run(synthesize_caret_sync_event(&textarea));
                                    }
                                }
                                on:input=move |ev| {
                                    let textarea = event_target::<HtmlTextAreaElement>(&ev);
                                    let web_input_event = ev.dyn_ref::<WebInputEvent>();
                                    let event = EditorInputEvent {
                                        text: event_target_value(&ev),
                                        selection_start: textarea
                                            .selection_start()
                                            .ok()
                                            .flatten()
                                            .map(|offset| offset as usize),
                                        selection_end: textarea
                                            .selection_end()
                                            .ok()
                                            .flatten()
                                            .map(|offset| offset as usize),
                                        input_kind: web_input_event
                                            .map(|input_event| {
                                                classify_dom_input(&input_event.input_type())
                                            })
                                            .unwrap_or(EditorInputKind::Other),
                                        inserted_text: web_input_event
                                            .and_then(|input_event| input_event.data()),
                                    };
                                    // Measure first so the geometry layer
                                    // has fresh metrics by the time the
                                    // popup view-model needs them this
                                    // tick. Self-correcting on resize and
                                    // first input: even if the very-first
                                    // mount happens before any layout,
                                    // the user's first keystroke will
                                    // measure before any popup is shown.
                                    if let Some(document) = web_sys::window()
                                        .and_then(|w| w.document())
                                    {
                                        if let Some(metrics) =
                                            measure_textarea_box(&textarea, &document)
                                        {
                                            state.update(|state| {
                                                let _ =
                                                    apply_editor_box_metrics_to_active_formula_space(
                                                        state, metrics,
                                                    );
                                            });
                                        }
                                    }
                                    on_editor_input.run(event);
                                }
                            ></textarea>
                            {move || render_completion_popup(completion_popup(), on_completion_click)}
                            {move || render_signature_help(signature_help())}
                            {move || render_function_help_card(
                                function_help_hover(),
                                function_help_card(),
                            )}
                        </div>
                        <div class="onecalc-home-shell__foot-row">
                            {move || render_editor_metrics_chip(editor_metrics(), view_mode())}
                            {move || render_formula_drill_toggle(
                                formula_drill(),
                                on_formula_drill_toggle,
                            )}
                            {render_recalculate_button(on_recalculate)}
                        </div>
                    </section>

                    <section class="onecalc-home-shell__formula-drill-section">
                        {move || render_formula_drill_panel(
                            formula_drill(),
                            capability_context(),
                            view_mode(),
                            on_view_mode_toggle,
                        )}
                    </section>

                    <section class="onecalc-home-shell__formatting-section">
                        {move || render_formatting_panel(
                            formatting_controls(),
                            on_formatting_panel_toggle,
                            on_set_number_format_code,
                            on_set_font_color,
                            on_set_fill_color,
                            on_set_date1904,
                            on_set_scenario_policy,
                            on_set_locale_preset,
                            on_add_cf_rule,
                            on_remove_cf_rule,
                            on_update_cf_rule,
                        )}
                        {move || render_vba_host_panel(
                            view_model.get().map(|vm| vm.vba_host_context),
                            on_vba_project_path_input,
                            on_vba_project_path_add,
                            on_vba_module_file_loaded,
                            on_vba_association_remove,
                        )}
                    </section>

                    <section class="onecalc-home-shell__result-section">
                        <div class="onecalc-home-shell__caption-row">
                            <span class="onecalc-home-shell__caption">"result ▸"</span>
                            {move || render_result_class_pill(result_class_pill())}
                        </div>
                        <div
                            class="onecalc-home-shell__result-block"
                            data-kind=move || result_view().map(result_kind_attr).unwrap_or("none")
                        >
                            {move || render_result_view(result_view(), state)}
                        </div>
                        <div class="onecalc-home-shell__foot-row">
                            {move || render_result_context_chip(result_context(), view_mode())}
                        </div>
                    </section>
                </Show>
            </main>

            <footer class="onecalc-home-shell__statusfoot">
                {move || render_status_foot(status_view())}
            </footer>

            {move || render_command_palette(
                command_palette(),
                on_command_palette_query,
                on_command_palette_dispatch,
                state,
            )}

            {move || render_manage_formulas_overlay(
                manage_formulas(),
                on_close_manage_formulas,
                on_manage_formulas_search,
                on_manage_formulas_open,
                on_manage_formulas_rename,
                on_manage_formulas_toggle_pin,
                on_manage_formulas_clone,
                on_manage_formulas_close,
                on_manage_formulas_forget,
            )}
        </div>
    }
}

/// Render the view-mode toggle button in the status-foot.
/// Always rendered so users can opt into Developer view without
/// needing to discover the keyboard chord. The button:
///
/// * In User mode shows a muted "dev" pill with `aria-pressed=
///   false` — clicking flips the workspace into Developer mode.
/// * In Developer mode shows the same pill in the accent palette
///   with `aria-pressed=true` — clicking flips back to User.
///
/// Uses `on:mousedown` (not `on:click`) so a click does not pull
/// focus away from the textarea: the textarea retains its caret
/// throughout the toggle.
/// Render the titlebar scenario-breadcrumb button + dropdown.
///
/// The button is always rendered when there is an active formula
/// space (the view-model returns `None` when none is active and
/// this helper short-circuits to an empty span). The dropdown
/// menu is keyboard-focusable; Esc inside it closes via the
/// `on_close` callback. Outside-click is handled by the document
/// listener wired in the parent component.
/// Render the command-palette overlay. Shown when the view-model
/// reports `is_open == true`; hidden otherwise. The palette is a
/// modal centered over the home shell with:
///
/// * a single-line filter input (autofocused on open),
/// * a scrollable list of commands grouped by section, each row
///   showing the command label, optional detail line, optional
///   keyboard chord; the row at `selected_index` is highlighted,
/// * keyboard handling: ArrowUp/Down moves selection, Enter
///   dispatches the selected command, Esc closes.
///
/// Outside-click on the backdrop closes the palette without
/// dispatching anything (the backdrop swallows the click and
/// fires `close_command_palette`).
fn render_command_palette(
    palette: Option<CommandPaletteView>,
    on_query: Callback<String>,
    on_dispatch: Callback<CommandPaletteEntryKind>,
    state: HostStateSignal,
) -> AnyView {
    let Some(palette) = palette else {
        return view! { <></> }.into_any();
    };
    if !palette.is_open {
        return view! { <></> }.into_any();
    }
    let total = palette.commands.len();
    let selected_index = palette.selected_index;
    let query_for_input = palette.query.clone();
    let close_palette = move || {
        state.update(|state| {
            let _ = crate::app::reducer::close_command_palette(state);
        });
    };
    let on_backdrop_click = {
        let close = close_palette;
        move |_: WebMouseEvent| close()
    };
    let on_keydown = {
        let on_dispatch = on_dispatch;
        let close = close_palette;
        move |ev: WebKeyboardEvent| {
            match ev.key().as_str() {
                "Escape" => {
                    ev.prevent_default();
                    close();
                }
                "ArrowDown" => {
                    ev.prevent_default();
                    state.update(|state| {
                        let _ =
                            crate::app::reducer::move_command_palette_selection(state, 1, total);
                    });
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    state.update(|state| {
                        let _ =
                            crate::app::reducer::move_command_palette_selection(state, -1, total);
                    });
                }
                "Enter" => {
                    ev.prevent_default();
                    // Read the selected command directly off the
                    // current palette projection so we always
                    // dispatch the live row, not a stale clone.
                    let selected_kind = state.with_untracked(|state| {
                        crate::services::home_shell_view_model::project_command_palette_entry_for_dispatch(
                            state,
                        )
                    });
                    if let Some(kind) = selected_kind {
                        on_dispatch.run(kind);
                    }
                }
                _ => {}
            }
        }
    };
    let rows: Vec<_> = palette
        .commands
        .into_iter()
        .enumerate()
        .map(|(index, entry)| render_command_palette_row(index, entry, selected_index, on_dispatch))
        .collect();
    let empty_marker = if total == 0 {
        view! {
            <div class="onecalc-home-shell__palette-empty">
                "No commands match your filter."
            </div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    view! {
        <div
            class="onecalc-home-shell__palette-backdrop"
            data-component="command-palette"
            on:click=on_backdrop_click
            on:keydown=on_keydown
        >
            <div
                class="onecalc-home-shell__palette"
                role="dialog"
                aria-modal="true"
                aria-label="command palette"
                on:click=move |ev: WebMouseEvent| ev.stop_propagation()
            >
                <input
                    type="text"
                    class="onecalc-home-shell__palette-input"
                    placeholder="Type a command, formula, or setting…"
                    autofocus="autofocus"
                    aria-label="command palette filter"
                    prop:value=query_for_input
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        on_query.run(target.value());
                    }
                />
                <div class="onecalc-home-shell__palette-list" role="listbox">
                    {rows}
                    {empty_marker}
                </div>
            </div>
        </div>
    }
    .into_any()
}

fn render_command_palette_row(
    index: usize,
    entry: CommandPaletteEntry,
    selected_index: usize,
    on_dispatch: Callback<CommandPaletteEntryKind>,
) -> AnyView {
    let is_selected = index == selected_index;
    let selected_attr = if is_selected { "true" } else { "false" };
    let kind = entry.kind.clone();
    let label = entry.label.clone();
    let section = entry.section;
    let detail = entry.detail.clone();
    let chord = entry.chord;
    let detail_view = detail.map(|d| {
        view! {
            <span class="onecalc-home-shell__palette-row-detail">{d}</span>
        }
        .into_any()
    });
    let chord_view = if chord.is_empty() {
        view! { <></> }.into_any()
    } else {
        view! {
            <span class="onecalc-home-shell__palette-row-chord">{chord}</span>
        }
        .into_any()
    };
    view! {
        <button
            type="button"
            class="onecalc-home-shell__palette-row"
            role="option"
            data-section=section
            data-is-selected=selected_attr
            aria-selected=selected_attr
            on:click=move |_| on_dispatch.run(kind.clone())
        >
            <span class="onecalc-home-shell__palette-row-section">{section}</span>
            <span class="onecalc-home-shell__palette-row-label">{label}</span>
            {detail_view}
            {chord_view}
        </button>
    }
    .into_any()
}

/// Render the manage-formulas overlay. Mounted below the home
/// shell as a centered modal with a search input + scrollable
/// row list. Closed when the view-model reports
/// `is_open == false`; the renderer short-circuits to an empty
/// span in that case.
///
/// Each row carries:
///   * display name (with active marker), pinned star, dirty dot,
///   * a muted formula-preview line (first ~80 chars, whitespace
///     collapsed),
///   * a per-row toolbar: Open/Active, Rename, Pin/Unpin, Clone,
///     Close (open) or Forget (recent).
///
/// Outside-click on the backdrop closes the overlay; Esc inside
/// closes too.
#[allow(clippy::too_many_arguments)]
fn render_manage_formulas_overlay(
    view: Option<ManageFormulasView>,
    on_close: Callback<()>,
    on_search: Callback<String>,
    on_open_formula: Callback<String>,
    on_rename: Callback<String>,
    on_toggle_pin: Callback<String>,
    on_clone: Callback<String>,
    on_close_formula: Callback<String>,
    on_forget: Callback<String>,
) -> AnyView {
    let Some(view) = view else {
        return view! { <></> }.into_any();
    };
    if !view.is_open {
        return view! { <></> }.into_any();
    }
    let total_count = view.total_count;
    let filtered_count = view.rows.len();
    let search_query = view.search_query.clone();
    let close_for_backdrop = on_close;
    let close_for_keydown = on_close;
    let on_backdrop_click = move |_: WebMouseEvent| close_for_backdrop.run(());
    let on_keydown = move |ev: WebKeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            close_for_keydown.run(());
        }
    };
    let rows: Vec<AnyView> = view
        .rows
        .into_iter()
        .map(|row| {
            render_manage_formulas_row(
                row,
                on_open_formula,
                on_rename,
                on_toggle_pin,
                on_clone,
                on_close_formula,
                on_forget,
            )
        })
        .collect();
    let count_label = if !search_query.is_empty() && filtered_count != total_count {
        format!("{filtered_count} of {total_count}")
    } else {
        total_count.to_string()
    };
    let empty_state: AnyView = if rows.is_empty() {
        let message = if search_query.is_empty() {
            "No formulas yet. Press Ctrl+N or click + to start one."
        } else {
            "No formulas match your search."
        };
        view! {
            <div class="onecalc-home-shell__manage-formulas-empty">{message}</div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    view! {
        <div
            class="onecalc-home-shell__manage-formulas-backdrop"
            role="presentation"
            on:click=on_backdrop_click
            on:keydown=on_keydown
        >
            <div
                class="onecalc-home-shell__manage-formulas"
                role="dialog"
                aria-label="manage formulas"
                tabindex="-1"
                on:click=move |ev: WebMouseEvent| ev.stop_propagation()
            >
                <header class="onecalc-home-shell__manage-formulas-header">
                    <h2 class="onecalc-home-shell__manage-formulas-title">
                        {format!("Manage formulas · {count_label}")}
                    </h2>
                    <button
                        type="button"
                        class="onecalc-home-shell__manage-formulas-close"
                        title="Close (Esc)"
                        aria-label="close manage formulas"
                        on:click=move |_| on_close.run(())
                    >
                        "✕"
                    </button>
                </header>
                <div class="onecalc-home-shell__manage-formulas-search">
                    <input
                        type="text"
                        class="onecalc-home-shell__manage-formulas-search-input"
                        placeholder="Search by name or formula text…"
                        aria-label="search formulas"
                        prop:value=search_query
                        autofocus
                        on:input=move |ev| {
                            let target = ev
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
                            if let Some(input) = target {
                                on_search.run(input.value());
                            }
                        }
                    />
                </div>
                <div
                    class="onecalc-home-shell__manage-formulas-rows"
                    role="list"
                    aria-label="formulas in workspace"
                >
                    {rows}
                    {empty_state}
                </div>
            </div>
        </div>
    }
    .into_any()
}

#[allow(clippy::too_many_arguments)]
fn render_manage_formulas_row(
    row: ManageFormulasRow,
    on_open_formula: Callback<String>,
    on_rename: Callback<String>,
    on_toggle_pin: Callback<String>,
    on_clone: Callback<String>,
    on_close_formula: Callback<String>,
    on_forget: Callback<String>,
) -> AnyView {
    let id_attr = row.formula_space_id.clone();
    let id_for_open = row.formula_space_id.clone();
    let id_for_rename = row.formula_space_id.clone();
    let id_for_pin = row.formula_space_id.clone();
    let id_for_clone = row.formula_space_id.clone();
    let id_for_close_or_forget = row.formula_space_id.clone();
    let display_name = row.display_name.clone();
    let formula_preview = row.formula_preview.clone();
    let active_attr = if row.is_active { "true" } else { "false" };
    let pinned_attr = if row.is_pinned { "true" } else { "false" };
    let open_attr = if row.is_open { "true" } else { "false" };
    let dirty_attr = if row.is_dirty { "true" } else { "false" };
    let pin_label = if row.is_pinned { "Unpin" } else { "Pin" };
    let pin_glyph = if row.is_pinned { "★" } else { "☆" };
    let open_label = if row.is_active {
        "Active"
    } else if row.is_open {
        "Switch"
    } else {
        "Open"
    };
    let close_or_forget_label = if row.is_open { "Close" } else { "Forget" };
    let close_or_forget_run: Callback<String> = if row.is_open {
        on_close_formula
    } else {
        on_forget
    };
    let preview_view: AnyView = if formula_preview.is_empty() {
        view! {
            <span class="onecalc-home-shell__manage-formulas-row-preview" data-empty="true">
                "(empty formula)"
            </span>
        }
        .into_any()
    } else {
        view! {
            <span class="onecalc-home-shell__manage-formulas-row-preview">{formula_preview}</span>
        }
        .into_any()
    };
    let pin_marker: AnyView = if row.is_pinned {
        view! {
            <span class="onecalc-home-shell__manage-formulas-row-pin" aria-hidden="true">"★"</span>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    let dirty_marker: AnyView = if row.is_dirty {
        view! {
            <span class="onecalc-home-shell__manage-formulas-row-dirty" aria-hidden="true">"●"</span>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    view! {
        <div
            class="onecalc-home-shell__manage-formulas-row"
            role="listitem"
            data-formula-space-id=id_attr
            data-is-active=active_attr
            data-is-pinned=pinned_attr
            data-is-open=open_attr
            data-is-dirty=dirty_attr
        >
            <div class="onecalc-home-shell__manage-formulas-row-info">
                <div class="onecalc-home-shell__manage-formulas-row-name">
                    {pin_marker}
                    <span class="onecalc-home-shell__manage-formulas-row-name-text">{display_name}</span>
                    {dirty_marker}
                </div>
                {preview_view}
            </div>
            <div class="onecalc-home-shell__manage-formulas-row-actions" role="group">
                <button
                    type="button"
                    class="onecalc-home-shell__manage-formulas-row-action"
                    data-action="open"
                    disabled=row.is_active
                    on:click=move |_| on_open_formula.run(id_for_open.clone())
                >
                    {open_label}
                </button>
                <button
                    type="button"
                    class="onecalc-home-shell__manage-formulas-row-action"
                    data-action="rename"
                    on:click=move |_| on_rename.run(id_for_rename.clone())
                >
                    "Rename"
                </button>
                <button
                    type="button"
                    class="onecalc-home-shell__manage-formulas-row-action"
                    data-action="pin"
                    title=pin_label
                    on:click=move |_| on_toggle_pin.run(id_for_pin.clone())
                >
                    {pin_glyph}
                </button>
                <button
                    type="button"
                    class="onecalc-home-shell__manage-formulas-row-action"
                    data-action="clone"
                    on:click=move |_| on_clone.run(id_for_clone.clone())
                >
                    "Clone"
                </button>
                <button
                    type="button"
                    class="onecalc-home-shell__manage-formulas-row-action"
                    data-action="close-or-forget"
                    on:click=move |_| close_or_forget_run.run(id_for_close_or_forget.clone())
                >
                    {close_or_forget_label}
                </button>
            </div>
        </div>
    }
    .into_any()
}

/// Render the tab strip between the titlebar and the editor
/// caption (WS-14 §1 minimum-viable surface). One chip per
/// `workspace_shell.open_formula_space_order` entry, in stable
/// order. The active chip is styled distinctly; each chip has a
/// small `✕` close button and a dirty-marker dot when the
/// formula has uncommitted changes. A trailing `+` button calls
/// `on_new_formula` (alias for Ctrl+N).
///
/// Hidden when only one formula is open (the breadcrumb already
/// names it; another row of chrome would just take vertical
/// space).
fn render_formula_tab_strip(
    strip: Option<FormulaTabStripView>,
    on_select: Callback<String>,
    on_close: Callback<String>,
    on_new_formula: Callback<()>,
    on_begin_rename: Callback<String>,
    on_update_rename_text: Callback<String>,
    on_commit_rename: Callback<()>,
    on_cancel_rename: Callback<()>,
) -> AnyView {
    let Some(strip) = strip else {
        return view! { <></> }.into_any();
    };
    if !strip.is_visible {
        return view! { <></> }.into_any();
    }
    let chips: Vec<_> = strip
        .chips
        .into_iter()
        .map(|chip| {
            render_formula_tab_chip(
                chip,
                on_select,
                on_close,
                on_begin_rename,
                on_update_rename_text,
                on_commit_rename,
                on_cancel_rename,
            )
        })
        .collect();
    view! {
        <nav class="onecalc-home-shell__tab-strip" role="tablist" aria-label="open formulas">
            {chips}
            <button
                type="button"
                class="onecalc-home-shell__tab-strip-new"
                title="New formula (Ctrl+N)"
                aria-label="new formula"
                on:click=move |_| on_new_formula.run(())
            >
                "+"
            </button>
        </nav>
    }
    .into_any()
}

fn render_formula_tab_chip(
    chip: FormulaTabChip,
    on_select: Callback<String>,
    on_close: Callback<String>,
    on_begin_rename: Callback<String>,
    on_update_rename_text: Callback<String>,
    on_commit_rename: Callback<()>,
    on_cancel_rename: Callback<()>,
) -> AnyView {
    let active_attr = if chip.is_active { "true" } else { "false" };
    let dirty_attr = if chip.is_dirty { "true" } else { "false" };
    let pinned_attr = if chip.is_pinned { "true" } else { "false" };
    let renaming_attr = if chip.is_renaming { "true" } else { "false" };
    let id_for_select = chip.formula_space_id.clone();
    let id_for_close = chip.formula_space_id.clone();
    let id_for_begin_rename = chip.formula_space_id.clone();
    let id_attr = chip.formula_space_id.clone();
    let display_name = chip.display_name.clone();
    let pinned_marker = chip.is_pinned.then(|| {
        view! { <span class="onecalc-home-shell__tab-strip-pin" aria-hidden="true">"★"</span> }
            .into_any()
    });
    let dirty_marker = chip.is_dirty.then(|| {
        view! { <span class="onecalc-home-shell__tab-strip-dirty" aria-hidden="true">"●"</span> }
            .into_any()
    });
    let label_body: AnyView = if chip.is_renaming {
        // Inline rename: render a text input bound to the buffered
        // text. The label-button wrapper is replaced with a form-
        // looking <span> so the input doesn't accidentally toggle
        // tab selection. Enter / Esc are handled in on:keydown;
        // the on:input event keeps the buffer in sync; on:blur
        // commits the rename.
        let rename_buffer_attr = chip.rename_buffer.clone();
        view! {
            <span class="onecalc-home-shell__tab-strip-chip-label" data-mode="rename">
                {pinned_marker}
                <input
                    type="text"
                    class="onecalc-home-shell__tab-strip-rename-input"
                    aria-label="rename formula"
                    prop:value=rename_buffer_attr
                    autofocus
                    on:input=move |ev| {
                        let target = ev
                            .target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
                        if let Some(input) = target {
                            on_update_rename_text.run(input.value());
                        }
                    }
                    on:keydown=move |ev: WebKeyboardEvent| {
                        match ev.key().as_str() {
                            "Enter" => {
                                ev.prevent_default();
                                on_commit_rename.run(());
                            }
                            "Escape" => {
                                ev.prevent_default();
                                on_cancel_rename.run(());
                            }
                            _ => {}
                        }
                    }
                    on:blur=move |_| on_commit_rename.run(())
                />
                {dirty_marker}
            </span>
        }
        .into_any()
    } else {
        view! {
            <button
                type="button"
                class="onecalc-home-shell__tab-strip-chip-label"
                title="Click to switch · double-click to rename"
                on:click=move |_| on_select.run(id_for_select.clone())
                on:dblclick=move |ev| {
                    ev.stop_propagation();
                    on_begin_rename.run(id_for_begin_rename.clone());
                }
            >
                {pinned_marker}
                <span class="onecalc-home-shell__tab-strip-chip-name">{display_name}</span>
                {dirty_marker}
            </button>
        }
        .into_any()
    };
    view! {
        <div
            class="onecalc-home-shell__tab-strip-chip"
            role="tab"
            data-formula-space-id=id_attr
            data-is-active=active_attr
            data-is-dirty=dirty_attr
            data-is-pinned=pinned_attr
            data-is-renaming=renaming_attr
            aria-selected=active_attr
        >
            {label_body}
            <button
                type="button"
                class="onecalc-home-shell__tab-strip-chip-close"
                title="Close formula"
                aria-label="close formula"
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(id_for_close.clone());
                }
            >
                "✕"
            </button>
        </div>
    }
    .into_any()
}

fn render_scenario_breadcrumb(
    breadcrumb: Option<ScenarioBreadcrumbView>,
    on_toggle: Callback<()>,
    on_close: Callback<()>,
    on_action: Callback<ScenarioBreadcrumbActionId>,
    on_entry_select: Callback<String>,
    on_entry_pin_toggle: Callback<String>,
) -> AnyView {
    let Some(breadcrumb) = breadcrumb else {
        return view! { <span class="onecalc-home-shell__breadcrumb-wrap" /> }.into_any();
    };
    let dirty_attr = if breadcrumb.is_dirty { "true" } else { "false" };
    let open_attr = if breadcrumb.is_open { "true" } else { "false" };
    let aria_expanded = if breadcrumb.is_open { "true" } else { "false" };
    let aria_hidden = if breadcrumb.is_open { "false" } else { "true" };
    let label = breadcrumb.active_label.clone();
    let label_for_button = label.clone();
    let recent = breadcrumb.recent.clone();
    let pinned = breadcrumb.pinned.clone();
    let actions = breadcrumb.actions.clone();
    view! {
        <span
            class="onecalc-home-shell__breadcrumb-wrap"
            data-open=open_attr
        >
            <button
                type="button"
                class="onecalc-home-shell__breadcrumb-button"
                data-dirty=dirty_attr
                aria-haspopup="menu"
                aria-expanded=aria_expanded
                aria-label=format!("formula: {}", label_for_button)
                on:click=move |_| {
                    on_toggle.run(());
                }
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        on_close.run(());
                    }
                }
            >
                <span class="onecalc-home-shell__breadcrumb-dot" aria-hidden="true"></span>
                <span class="onecalc-home-shell__breadcrumb-label">{label}</span>
                <span class="onecalc-home-shell__breadcrumb-caret" aria-hidden="true">"▾"</span>
            </button>
            <div
                class="onecalc-home-shell__scenario-menu"
                role="menu"
                aria-hidden=aria_hidden
                data-open=open_attr
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        on_close.run(());
                    }
                }
            >
                <div class="onecalc-home-shell__scenario-menu-section" data-section="recent">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Recent"</div>
                    {render_scenario_menu_entries(recent, "recent", on_entry_select, on_entry_pin_toggle)}
                </div>
                <div class="onecalc-home-shell__scenario-menu-section" data-section="pinned">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Pinned"</div>
                    {render_scenario_menu_entries(pinned, "pinned", on_entry_select, on_entry_pin_toggle)}
                </div>
                <div class="onecalc-home-shell__scenario-menu-section" data-section="actions">
                    <div class="onecalc-home-shell__scenario-menu-heading">"Actions"</div>
                    {render_scenario_menu_actions(actions, on_action)}
                </div>
            </div>
        </span>
    }
    .into_any()
}

fn render_scenario_menu_entries(
    entries: Vec<ScenarioBreadcrumbEntry>,
    section: &'static str,
    on_entry_select: Callback<String>,
    on_entry_pin_toggle: Callback<String>,
) -> AnyView {
    if entries.is_empty() {
        return view! {
            <div
                class="onecalc-home-shell__scenario-menu-empty"
                data-section=section
            >
                {match section {
                    "pinned" => "No pinned formulas",
                    "recent" => "No recent formulas",
                    _ => "(empty)",
                }}
            </div>
        }
        .into_any();
    }
    let rows: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let is_active_attr = if entry.is_active { "true" } else { "false" };
            let is_pinned_attr = if entry.is_pinned { "true" } else { "false" };
            let formula_space_id = entry.formula_space_id.clone();
            let display_name = entry.display_name.clone();
            let meta = entry.meta.clone();
            // Click row → switch to this formula. Click pin glyph
            // → toggle pin without switching. The two actions are
            // separate buttons to keep the click target unambiguous
            // (the pin glyph stops propagation so the row's click
            // handler doesn't also fire).
            let id_for_select = formula_space_id.clone();
            let id_for_pin = formula_space_id.clone();
            let pin_title = if entry.is_pinned { "Unpin" } else { "Pin" };
            let pin_glyph = if entry.is_pinned { "★" } else { "☆" };
            let id_for_outer = formula_space_id.clone();
            view! {
                <div
                    class="onecalc-home-shell__scenario-menu-row"
                    data-formula-space-id=id_for_outer
                    data-is-active=is_active_attr
                    data-is-pinned=is_pinned_attr
                    data-section=section
                >
                    <button
                        type="button"
                        class="onecalc-home-shell__scenario-menu-item"
                        role="menuitem"
                        data-formula-space-id=formula_space_id
                        data-is-active=is_active_attr
                        data-is-pinned=is_pinned_attr
                        data-section=section
                        on:click=move |_| on_entry_select.run(id_for_select.clone())
                    >
                        <span class="onecalc-home-shell__scenario-menu-item-name">
                            {display_name}
                        </span>
                        <span class="onecalc-home-shell__scenario-menu-item-meta">
                            {meta}
                        </span>
                    </button>
                    <button
                        type="button"
                        class="onecalc-home-shell__scenario-menu-pin"
                        data-action="pin-toggle"
                        data-is-pinned=is_pinned_attr
                        title=pin_title
                        aria-label=pin_title
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_entry_pin_toggle.run(id_for_pin.clone());
                        }
                    >
                        {pin_glyph}
                    </button>
                </div>
            }
            .into_any()
        })
        .collect();
    view! { <>{rows}</> }.into_any()
}

fn render_scenario_menu_actions(
    actions: Vec<ScenarioBreadcrumbAction>,
    on_action: Callback<ScenarioBreadcrumbActionId>,
) -> AnyView {
    let buttons: Vec<_> = actions
        .into_iter()
        .map(|action| {
            let action_id = action.action_id;
            let chord = action.chord_label;
            let label = action.label;
            let seam = action.seam_id;
            let title = seam.map(|s| format!("Pending: {s}")).unwrap_or_default();
            view! {
                <button
                    type="button"
                    class="onecalc-home-shell__scenario-menu-item"
                    role="menuitem"
                    data-action-id=action_id.slug()
                    data-section="actions"
                    data-seam-id=seam.unwrap_or("")
                    title=title
                    on:click=move |_| {
                        on_action.run(action_id);
                    }
                >
                    <span class="onecalc-home-shell__scenario-menu-item-name">
                        {label}
                    </span>
                    <span class="onecalc-home-shell__scenario-menu-item-meta">
                        {if chord.is_empty() {
                            seam.map(|s| s.to_string()).unwrap_or_default()
                        } else {
                            chord.to_string()
                        }}
                    </span>
                </button>
            }
            .into_any()
        })
        .collect();
    view! { <>{buttons}</> }.into_any()
}

/// Render the status-foot strip: a colored dot reflecting bridge health
/// (sage when Live, amber when Stale), the literal "live-bridge" label,
/// a separator, and the current green-tree key (or "—" placeholder).
fn render_status_foot(status: Option<StatusView>) -> AnyView {
    let status = match status {
        Some(s) => s,
        None => {
            return view! {
                <span class="onecalc-home-shell__statusfoot-dot" data-health="stale"></span>
                <span>"no formula space"</span>
            }
            .into_any();
        }
    };
    let dot_health = match status.bridge_health {
        BridgeHealth::Live => "live",
        BridgeHealth::Stale => "stale",
    };
    let green_key = status
        .green_tree_key
        .as_deref()
        .map(short_green_tree_key)
        .unwrap_or_else(|| "—".to_string());
    let scenario_label = status.scenario_label.clone();
    let scenario_label_attr = scenario_label.clone();
    let load_diagnostics = status.load_diagnostics.clone();
    view! {
        <span class="onecalc-home-shell__statusfoot-dot" data-health=dot_health></span>
        <span>"live-bridge"</span>
        <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
        <span>{format!("green-tree {green_key}")}</span>
        <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
        <span class="onecalc-home-shell__statusfoot-scenario">
            "formula · "
            <span
                class="onecalc-home-shell__statusfoot-scenario-name"
                data-scenario-label=scenario_label_attr
            >
                {scenario_label}
            </span>
        </span>
        {render_load_diagnostic_chips(load_diagnostics)}
    }
    .into_any()
}

/// Render the WS-14 §5.3-item-8 collapsible formatting panel that sits
/// between the formula drill-down and the result section. The panel
/// has two surfaces:
///
/// * **Collapsed (default)**: a single `format ▸ <summary>` chip the
///   user can click to expand. The summary string comes from
///   `FormattingControlsView.summary` and reads e.g.
///   `"General"` (defaults), `"Currency"` (matched preset), or
///   `"$#,##0.00 · font #ff0000 · Date1904"` (multi-override).
/// * **Expanded**: the full `render_formatting_controls` row plus a
///   ▾ caption that flips back to ▸ when the user clicks again.
///
/// The chip emits `data-formatting-panel-expanded` (`"true" | "false"`)
/// and `data-formatting-summary` so the browser corpus can pin both.
fn render_formatting_panel(
    controls: Option<FormattingControlsView>,
    on_toggle: Callback<()>,
    on_set_number_format_code: Callback<String>,
    on_set_font_color: Callback<String>,
    on_set_fill_color: Callback<String>,
    on_set_date1904: Callback<bool>,
    on_set_scenario_policy: Callback<crate::persistence::ScenarioPolicy>,
    on_set_locale_preset: Callback<String>,
    on_add_cf_rule: Callback<()>,
    on_remove_cf_rule: Callback<usize>,
    on_update_cf_rule: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let Some(controls) = controls else {
        return view! { <></> }.into_any();
    };
    let is_open = controls.is_open;
    let summary = controls.summary.clone();
    let summary_for_attr = summary.clone();
    let expanded_attr = if is_open { "true" } else { "false" };
    let aria_expanded = expanded_attr;
    let aria_hidden_body = if is_open { "false" } else { "true" };
    let toggle_label = if is_open {
        format!("▾ format ▸ {}", summary)
    } else {
        format!("▸ format ▸ {}", summary)
    };
    view! {
        <div
            class="onecalc-home-shell__formatting-panel"
            data-formatting-panel-expanded=expanded_attr
            data-formatting-summary=summary_for_attr
        >
            <button
                type="button"
                class="onecalc-home-shell__formatting-toggle-button"
                data-expanded=expanded_attr
                aria-expanded=aria_expanded
                aria-controls="onecalc-formatting-panel-body"
                on:click=move |_| on_toggle.run(())
            >
                {toggle_label}
            </button>
            <div
                id="onecalc-formatting-panel-body"
                class="onecalc-home-shell__formatting-panel-body"
                data-expanded=expanded_attr
                aria-hidden=aria_hidden_body
            >
                {if is_open {
                    render_formatting_controls(
                        controls,
                        on_set_number_format_code,
                        on_set_font_color,
                        on_set_fill_color,
                        on_set_date1904,
                        on_set_scenario_policy,
                        on_set_locale_preset,
                        on_add_cf_rule,
                        on_remove_cf_rule,
                        on_update_cf_rule,
                    )
                } else {
                    view! { <></> }.into_any()
                }}
            </div>
        </div>
    }
    .into_any()
}

fn render_vba_host_panel(
    context: Option<VbaHostContextView>,
    on_path_input: Callback<String>,
    on_add_path: Callback<()>,
    on_vba_module_file_loaded: Callback<VbaModuleSourceLoadRequest>,
    on_remove: Callback<String>,
) -> AnyView {
    let Some(context) = context else {
        return view! { <></> }.into_any();
    };
    let path_value = context.pending_project_path.clone();
    let rows = context
        .associations
        .into_iter()
        .map(|association| render_vba_association_row(association, on_remove))
        .collect::<Vec<_>>();
    let row_count = rows.len().to_string();
    #[cfg(target_arch = "wasm32")]
    let native_picker_available =
        crate::persistence::tauri_file_io::tauri_command_bridge_available();
    #[cfg(not(target_arch = "wasm32"))]
    let native_picker_available = false;
    let module_picker = if native_picker_available {
        let callback = on_vba_module_file_loaded.clone();
        view! {
            <button
                type="button"
                class="onecalc-home-shell__formatting-preset"
                data-vba-action="select-native-module-file"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let callback = callback.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            match crate::persistence::tauri_file_io::select_vba_module_source().await {
                                Ok(Some(selection)) => {
                                    callback.run(VbaModuleSourceLoadRequest::native_file(
                                        selection.display_name,
                                        selection.source_path,
                                        selection.source_text,
                                        selection.diagnostics,
                                    ));
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    callback.run(VbaModuleSourceLoadRequest::native_file(
                                        "VBA module selection failed".to_string(),
                                        "tauri-command:select_vba_module_source".to_string(),
                                        String::new(),
                                        vec![error],
                                    ));
                                }
                            }
                        });
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = &callback;
                    }
                }
            >
                "Select .bas"
            </button>
        }
        .into_any()
    } else {
        let callback = on_vba_module_file_loaded.clone();
        view! {
            <label class="onecalc-home-shell__formatting-field">
                <span class="onecalc-home-shell__formatting-field-label">".bas file"</span>
                <input
                    type="file"
                    class="onecalc-home-shell__formatting-input"
                    data-vba-field="module-file"
                    accept=".bas"
                    on:change=move |ev| {
                        let target: web_sys::HtmlInputElement =
                            event_target::<web_sys::HtmlInputElement>(&ev);
                        if let Some(files) = target.files() {
                            if let Some(file) = files.get(0) {
                                let file_name = file.name();
                                let callback = callback.clone();
                                #[cfg(target_arch = "wasm32")]
                                {
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Ok(text) = crate::persistence::browser_file_io::read_bas_file_as_text(file).await {
                                            callback.run(VbaModuleSourceLoadRequest::browser_file(file_name, text));
                                        }
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (file, file_name, callback);
                                }
                            }
                        }
                    }
                />
            </label>
        }
        .into_any()
    };
    view! {
        <div class="onecalc-home-shell__vba-panel" data-component="vba-host-context">
            <div class="onecalc-home-shell__formatting-row" data-vba-row="picker">
                <span class="onecalc-home-shell__formatting-caption">"VBA ▸"</span>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"project path"</span>
                    <input
                        type="text"
                        class="onecalc-home-shell__formatting-input"
                        data-vba-field="project-path"
                        placeholder="C:\\path\\Project.basproj or folder"
                        prop:value=path_value.clone()
                        value=path_value
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_path_input.run(target.value());
                        }
                    />
                </label>
                <button
                    type="button"
                    class="onecalc-home-shell__formatting-preset"
                    data-vba-action="add-project-path"
                    on:click=move |_| on_add_path.run(())
                >
                    "Add"
                </button>
                {module_picker}
                <span class="onecalc-home-shell__formatting-field-label" data-vba-summary="true">
                    {context.summary}
                </span>
            </div>
            <div class="onecalc-home-shell__vba-associations" data-vba-association-count=row_count>
                {rows}
            </div>
        </div>
    }
    .into_any()
}

fn render_vba_association_row(
    association: VbaHostAssociationView,
    on_remove: Callback<String>,
) -> AnyView {
    let id = association.association_id.clone();
    let udf_detail = if association.admitted_udfs.is_empty() {
        format!(
            "{} UDF(s) · {} rejected",
            association.admitted_udf_count, association.rejected_candidate_count,
        )
    } else {
        format!(
            "{} UDF(s) · {} rejected · admitted: {}",
            association.admitted_udf_count,
            association.rejected_candidate_count,
            association.admitted_udfs.join(", "),
        )
    };
    let rejected = if association.rejected_candidates.is_empty() {
        String::new()
    } else {
        format!("rejected: {}", association.rejected_candidates.join(", "))
    };
    view! {
        <div class="onecalc-home-shell__vba-association" data-vba-association-id=association.association_id>
            <span class="onecalc-home-shell__vba-source-kind">{association.source_kind}</span>
            <span class="onecalc-home-shell__vba-source-name">{association.display_name}</span>
            <span class="onecalc-home-shell__vba-source-status">{association.status_label}</span>
            <span class="onecalc-home-shell__vba-source-detail">{udf_detail}</span>
            <span class="onecalc-home-shell__vba-source-ref">{association.source_ref}</span>
            <button
                type="button"
                class="onecalc-home-shell__formatting-preset"
                data-vba-action="remove"
                on:click=move |_| on_remove.run(id.clone())
            >
                "Remove"
            </button>
            {(!rejected.is_empty()).then(|| view! {
                <span class="onecalc-home-shell__vba-source-rejected">{rejected}</span>
            })}
        </div>
    }
    .into_any()
}

/// Render the formatting-controls body. Three rows:
///
/// 1. **Format row** — number-format text input + the full family
///    preset chip strip + font / fill colour pickers + Date1904
///    toggle.
/// 2. **Calc-options row** — Deterministic / LiveRecalc segmented
///    control. Drives clock and random-provider selection for the
///    bridge.
/// 3. **Conditional formatting** — list of rules with per-rule
///    remove + a `+ add rule` affordance.
fn render_formatting_controls(
    controls: FormattingControlsView,
    on_set_number_format_code: Callback<String>,
    on_set_font_color: Callback<String>,
    on_set_fill_color: Callback<String>,
    on_set_date1904: Callback<bool>,
    on_set_scenario_policy: Callback<crate::persistence::ScenarioPolicy>,
    on_set_locale_preset: Callback<String>,
    on_add_cf_rule: Callback<()>,
    on_remove_cf_rule: Callback<usize>,
    on_update_cf_rule: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let number_format_code_value = controls.number_format_code.clone();
    let number_format_code_attr = number_format_code_value.clone();
    let font_color_value = controls.font_color.clone();
    let fill_color_value = controls.fill_color.clone();
    let date1904 = controls.date1904;
    let scenario_policy = controls.scenario_policy;
    let cf_rules = controls.conditional_formatting_rules.clone();
    let locale_language_tag = controls.locale_language_tag.clone();
    let locale_presets = controls.locale_presets.clone();
    let locale_seam_id_for_panel = controls.locale_seam_id;
    view! {
        <div class="onecalc-home-shell__formatting-rows" role="group" aria-label="formula formatting">
            <div class="onecalc-home-shell__formatting-row">
                <span class="onecalc-home-shell__formatting-caption">"format ▸"</span>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"number format"</span>
                    <input
                        type="text"
                        class="onecalc-home-shell__formatting-input"
                        data-formatting-field="number-format-code"
                        placeholder="General"
                        prop:value=number_format_code_value
                        value=number_format_code_attr
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_number_format_code.run(target.value());
                        }
                    />
                </label>
                <span class="onecalc-home-shell__formatting-presets">
                    {render_number_format_presets(
                        controls.number_format_presets.clone(),
                        on_set_number_format_code,
                    )}
                </span>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"font color"</span>
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-formatting-field="font-color"
                        prop:value=move || normalize_color_for_input(&font_color_value)
                        value=normalize_color_for_input(&controls.font_color)
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_font_color.run(target.value());
                        }
                    />
                </label>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"fill color"</span>
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-formatting-field="fill-color"
                        prop:value=move || normalize_color_for_input(&fill_color_value)
                        value=normalize_color_for_input(&controls.fill_color)
                        on:input=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_fill_color.run(target.value());
                        }
                    />
                </label>
                <label class="onecalc-home-shell__formatting-field">
                    <span class="onecalc-home-shell__formatting-field-label">"1904 dates"</span>
                    <input
                        type="checkbox"
                        class="onecalc-home-shell__formatting-toggle"
                        data-formatting-field="date1904"
                        prop:checked=date1904
                        on:change=move |ev| {
                            let target: web_sys::HtmlInputElement =
                                event_target::<web_sys::HtmlInputElement>(&ev);
                            on_set_date1904.run(target.checked());
                        }
                    />
                </label>
            </div>
            <div class="onecalc-home-shell__formatting-row" data-formatting-row="calc-options">
                <span class="onecalc-home-shell__formatting-caption">"calc ▸"</span>
                {render_scenario_policy_toggle(scenario_policy, on_set_scenario_policy)}
                {render_locale_picker(&locale_language_tag, &locale_presets, locale_seam_id_for_panel, on_set_locale_preset)}
            </div>
            <div class="onecalc-home-shell__formatting-row" data-formatting-row="cf-rules">
                <span class="onecalc-home-shell__formatting-caption">"CF ▸"</span>
                {render_conditional_formatting_section(
                    cf_rules,
                    on_add_cf_rule,
                    on_remove_cf_rule,
                    on_update_cf_rule,
                )}
            </div>
        </div>
    }
    .into_any()
}

/// Workspace-locale picker. The `<select>` updates the workspace's
/// `AmbientAppContext` (date / datetime / time format-code triple)
/// and is forwarded through the bridge as the per-edit
/// `language_tag`, which `live_bridge::build_runtime_locale_context`
/// resolves through `LocaleProfileId::from_bcp47_language_tag` into
/// a runtime `LocaleFormatContext`. Switching to e.g. `de-DE` now
/// flips both the presentation-hint default *and* the runtime month
/// / weekday tables, decimal / thousands separators, currency
/// symbol, and `General` rendering. The `seam_id` argument is kept
/// for any host that wants to flag *additional* locale-related gaps
/// (e.g. an as-yet-unmapped Excel locale id) — the core dropdown is
/// no longer SEAM-tagged.
fn render_locale_picker(
    language_tag: &str,
    presets: &[(&'static str, &'static str)],
    seam_id: Option<&'static str>,
    on_set_locale_preset: Callback<String>,
) -> AnyView {
    let seam_attr = seam_id.unwrap_or("").to_string();
    let title = match seam_id {
        Some(seam) => format!("Workspace locale (runtime tables pending: {seam})",),
        None => "Workspace locale".to_string(),
    };
    let seam_badge = seam_id.map(|seam| {
        view! {
            <span class="onecalc-home-shell__formatting-locale-seam"
                data-seam-id=seam.to_string()
                title=format!("<NOT IMPLEMENTED> {seam}")
            >"⚠"</span>
        }
        .into_any()
    });
    let current = language_tag.to_string();
    let options: Vec<_> = presets
        .iter()
        .map(|(tag, label)| {
            let tag_str = (*tag).to_string();
            let label_str = (*label).to_string();
            let selected = current == *tag;
            view! {
                <option value=tag_str.clone() selected=selected>
                    {format!("{label_str} ({tag_str})")}
                </option>
            }
            .into_any()
        })
        .collect();
    view! {
        <label
            class="onecalc-home-shell__formatting-locale"
            data-seam-id=seam_attr
            title=title
        >
            <span class="onecalc-home-shell__formatting-field-label">"locale"</span>
            <select
                class="onecalc-home-shell__formatting-locale-select"
                data-formatting-field="locale-language-tag"
                on:change=move |ev| {
                    let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                    on_set_locale_preset.run(target.value());
                }
            >
                {options}
            </select>
            {seam_badge}
        </label>
    }
    .into_any()
}

/// Three-button segmented control: Deterministic | Live | Manual.
/// Clicking the inactive button switches the active formula's
/// scenario policy; the active button is highlighted via the
/// `[data-active="true"]` selector. Manual recalc is the user's
/// lever for keeping the editor responsive when the formula is
/// expensive (large REDUCE / MAKEARRAY / LAMBDA workloads); typing
/// runs parse / bind / popups every keystroke but skips the
/// runtime-evaluation pass until F9 / Calculate.
fn render_scenario_policy_toggle(
    current: ScenarioPolicyView,
    on_set: Callback<crate::persistence::ScenarioPolicy>,
) -> AnyView {
    let is_deterministic = matches!(current, ScenarioPolicyView::Deterministic);
    let is_live = matches!(current, ScenarioPolicyView::LiveRecalc);
    let is_manual = matches!(current, ScenarioPolicyView::ManualRecalc);
    view! {
        <div
            class="onecalc-home-shell__formatting-policy-toggle"
            role="group"
            aria-label="scenario calc-options policy"
        >
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="deterministic"
                data-active=if is_deterministic { "true" } else { "false" }
                aria-pressed=if is_deterministic { "true" } else { "false" }
                title="Pin NOW / RAND seeds for reproducible authoring"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::Deterministic)
            >
                "Deterministic"
            </button>
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="live"
                data-active=if is_live { "true" } else { "false" }
                aria-pressed=if is_live { "true" } else { "false" }
                title="NOW advances per round-trip; RAND rolls each time"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::LiveRecalc)
            >
                "Live"
            </button>
            <button
                type="button"
                class="onecalc-home-shell__formatting-policy-button"
                data-policy="manual"
                data-active=if is_manual { "true" } else { "false" }
                aria-pressed=if is_manual { "true" } else { "false" }
                title="Skip runtime evaluation on text edits; recalc on F9 / Calculate only"
                on:click=move |_| on_set.run(crate::persistence::ScenarioPolicy::ManualRecalc)
            >
                "Manual"
            </button>
        </div>
    }
    .into_any()
}

/// Render the conditional-formatting rules section: zero or more
/// rule cards followed by a `+ add rule` button. Each rule card
/// lets the user edit operator / threshold / font / fill inline,
/// and remove the rule. SEAM-marked rule kinds (color scales, data
/// bars, icon sets, …) render with a `<NOT IMPLEMENTED>` chip.
fn render_conditional_formatting_section(
    rules: Vec<ConditionalFormattingRuleView>,
    on_add: Callback<()>,
    on_remove: Callback<usize>,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let rule_cards: Vec<_> = rules
        .into_iter()
        .enumerate()
        .map(|(index, rule)| render_cf_rule_card(index, rule, on_remove, on_update))
        .collect();
    view! {
        <div class="onecalc-home-shell__cf-rules" role="group" aria-label="conditional formatting rules">
            {rule_cards}
            <button
                type="button"
                class="onecalc-home-shell__cf-add-button"
                title="Add a default cell-value > 0 rule; edit thresholds / colours inline"
                on:click=move |_| on_add.run(())
            >
                "+ add rule"
            </button>
        </div>
    }
    .into_any()
}

fn render_cf_rule_card(
    index: usize,
    rule: ConditionalFormattingRuleView,
    on_remove: Callback<usize>,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let rule_kind_value = rule.rule_kind.clone();
    let operator_value = rule.operator.clone().unwrap_or_default();
    let threshold_value = rule.thresholds.first().cloned().unwrap_or_default();
    let font_color_value = rule.font_color.clone().unwrap_or_default();
    let fill_color_value = rule.fill_color.clone().unwrap_or_default();
    let seam_badge = rule.seam_id.map(|seam| {
        view! {
            <span
                class="onecalc-home-shell__cf-rule-seam"
                data-seam-id=seam.to_string()
                title=format!("<NOT IMPLEMENTED> {seam}")
            >"⚠ NOT IMPL"</span>
        }
        .into_any()
    });
    let rule_for_kind = rule.clone();
    let rule_for_op = rule.clone();
    let rule_for_threshold = rule.clone();
    let rule_for_font = rule.clone();
    let rule_for_fill = rule.clone();
    view! {
        <div
            class="onecalc-home-shell__cf-rule"
            data-cf-rule-index=index.to_string()
            data-cf-rule-kind=rule.rule_kind.clone()
        >
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"kind"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-rule-field="kind"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_kind);
                        next.rule_kind = target.value();
                        seed_visualization_rule_defaults(&mut next);
                        on_update.run((index, next));
                    }
                >
                    <optgroup label="Per-cell">
                        <option value="cell_value" selected=rule_kind_value == "cell_value">"cell value"</option>
                        <option value="text" selected=rule_kind_value == "text">"text"</option>
                        <option value="dates" selected=rule_kind_value == "dates">"dates"</option>
                        <option value="blanks" selected=rule_kind_value == "blanks">"blanks"</option>
                        <option value="noBlanks" selected=rule_kind_value == "noBlanks">"no blanks"</option>
                        <option value="errors" selected=rule_kind_value == "errors">"errors"</option>
                        <option value="noErrors" selected=rule_kind_value == "noErrors">"no errors"</option>
                        <option value="expression" selected=rule_kind_value == "expression">"expression"</option>
                    </optgroup>
                    <optgroup label="Aggregate (array as range)">
                        <option value="colorScale" selected=rule_kind_value == "colorScale">"color scale"</option>
                        <option value="dataBar" selected=rule_kind_value == "dataBar">"data bar"</option>
                        <option value="iconSet" selected=rule_kind_value == "iconSet">"icon set"</option>
                        <option value="aboveAverage" selected=rule_kind_value == "aboveAverage">"above average"</option>
                        <option value="belowAverage" selected=rule_kind_value == "belowAverage">"below average"</option>
                        <option value="top" selected=rule_kind_value == "top">"top N"</option>
                        <option value="bottom" selected=rule_kind_value == "bottom">"bottom N"</option>
                        <option value="uniqueValues" selected=rule_kind_value == "uniqueValues">"unique values"</option>
                        <option value="duplicateValues" selected=rule_kind_value == "duplicateValues">"duplicate values"</option>
                    </optgroup>
                </select>
            </label>
            {render_cf_rule_operator_dropdown(rule.rule_kind.clone(), operator_value.clone(), index, rule_for_op, on_update)}
            {render_cf_rule_threshold_control(rule.rule_kind.clone(), threshold_value, index, rule_for_threshold, on_update)}
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"font"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-rule-field="font-color"
                    prop:value=normalize_color_for_input(&font_color_value)
                    value=normalize_color_for_input(&font_color_value)
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_font);
                        next.font_color = Some(target.value());
                        on_update.run((index, next));
                    }
                />
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"fill"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-rule-field="fill-color"
                    prop:value=normalize_color_for_input(&fill_color_value)
                    value=normalize_color_for_input(&fill_color_value)
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let mut next = host_cf_rule_from_view(&rule_for_fill);
                        next.fill_color = Some(target.value());
                        on_update.run((index, next));
                    }
                />
            </label>
            {seam_badge}
            <button
                type="button"
                class="onecalc-home-shell__cf-rule-remove"
                title="Remove this rule"
                aria-label="remove conditional formatting rule"
                on:click=move |_| on_remove.run(index)
            >
                "✕"
            </button>
            {render_cf_rule_typed_subform(rule.clone(), index, on_update)}
        </div>
    }
    .into_any()
}

/// Render the operator dropdown for a CF rule card. Operator
/// strings are the canonical Excel CF names that OxFml's
/// `evaluate_operator_rule` matches after stripping non-alphanumerics
/// and lowercasing — so `greaterThan` / `greaterThanOrEqual` /
/// `lessThan` / `lessThanOrEqual` / `equal` rather than abbreviated
/// `gt` / `gte` / `lt` / `lte` / `eq` (which OxFml does not match).
///
/// Predicate rule kinds (`blanks` / `noBlanks` / `errors` /
/// `noErrors` / `dates` / `expression`) and visualization rule
/// kinds (`colorScale` / `dataBar` / `iconSet` /
/// `aboveAverage` / `belowAverage` / `uniqueValues` /
/// `duplicateValues`) don't take an operator — the kind itself
/// is the predicate or the aggregate-context computation. The
/// dropdown collapses to a no-op span in those cases.
fn render_cf_rule_operator_dropdown(
    rule_kind: String,
    operator_value: String,
    index: usize,
    rule_for_op: ConditionalFormattingRuleView,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    if matches!(
        rule_kind.to_ascii_lowercase().as_str(),
        "blanks"
            | "noblanks"
            | "errors"
            | "noerrors"
            | "dates"
            | "expression"
            | "colorscale"
            | "databar"
            | "iconset"
            | "aboveaverage"
            | "belowaverage"
            | "top"
            | "bottom"
            | "uniquevalues"
            | "duplicatevalues"
    ) {
        return view! { <span></span> }.into_any();
    }
    view! {
        <label class="onecalc-home-shell__cf-rule-field">
            <span class="onecalc-home-shell__cf-rule-field-label">"op"</span>
            <select
                class="onecalc-home-shell__cf-rule-input"
                data-cf-rule-field="operator"
                on:change=move |ev| {
                    let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                    let mut next = host_cf_rule_from_view(&rule_for_op);
                    let raw = target.value();
                    next.operator = if raw.is_empty() { None } else { Some(raw) };
                    on_update.run((index, next));
                }
            >
                <option value="greaterThan" selected=operator_value == "greaterThan">">"</option>
                <option value="greaterThanOrEqual" selected=operator_value == "greaterThanOrEqual">"≥"</option>
                <option value="lessThan" selected=operator_value == "lessThan">"<"</option>
                <option value="lessThanOrEqual" selected=operator_value == "lessThanOrEqual">"≤"</option>
                <option value="equal" selected=operator_value == "equal">"="</option>
                <option value="notEqual" selected=operator_value == "notEqual">"≠"</option>
                <option value="between" selected=operator_value == "between">"between"</option>
                <option value="notBetween" selected=operator_value == "notBetween">"not between"</option>
                <option value="containsText" selected=operator_value == "containsText">"contains"</option>
                <option value="notContainsText" selected=operator_value == "notContainsText">"not contains"</option>
                <option value="beginsWith" selected=operator_value == "beginsWith">"begins with"</option>
                <option value="endsWith" selected=operator_value == "endsWith">"ends with"</option>
            </select>
        </label>
    }
    .into_any()
}

/// Render the threshold control, adapting to the rule kind:
///
/// - **`dates`** → a relative-date dropdown matching the W070
///   landed predicates (today / yesterday / tomorrow / last 7
///   days / this week / last week / next week / this month /
///   last month / next month). The selected value is stored as
///   `thresholds[0]` so OxFml can dispatch.
/// - **`blanks` / `noBlanks` / `errors` / `noErrors`** → no
///   control (predicate fires from the kind alone).
/// - **`expression`** → free-text input for the formula body.
/// - **`top` / `bottom`** → numeric input for the count or
///   percentage.
/// - **`colorScale` / `dataBar` / `iconSet` / `aboveAverage` /
///   `belowAverage` / `uniqueValues` / `duplicateValues`** →
///   no control (aggregate-context computation; the array
///   itself is the input). SEAM-marked because OxFml hasn't
///   landed the aggregate evaluation yet — see
///   `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`.
/// - **everything else** (`cell_value`, `text`, etc.) → free-text
///   numeric / textual threshold.
fn render_cf_rule_threshold_control(
    rule_kind: String,
    threshold_value: String,
    index: usize,
    rule_for_threshold: ConditionalFormattingRuleView,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let kind_lc = rule_kind.to_ascii_lowercase();
    match kind_lc.as_str() {
        "blanks" | "noblanks" | "errors" | "noerrors" => view! { <span></span> }.into_any(),
        // W073-typed families (`top` / `bottom` included) own their
        // configuration through the per-kind sub-form; the bounded
        // `thresholds` field is upstream-ignored, so the threshold
        // control above the subform would just confuse the user.
        "colorscale"
        | "databar"
        | "iconset"
        | "aboveaverage"
        | "belowaverage"
        | "top"
        | "bottom"
        | "uniquevalues"
        | "duplicatevalues" => view! { <span></span> }.into_any(),
        "dates" => {
            view! {
                <label class="onecalc-home-shell__cf-rule-field">
                    <span class="onecalc-home-shell__cf-rule-field-label">"when"</span>
                    <select
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-rule-field="threshold"
                        on:change=move |ev| {
                            let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                            let mut next = host_cf_rule_from_view(&rule_for_threshold);
                            next.thresholds = vec![target.value()];
                            on_update.run((index, next));
                        }
                    >
                        <option value="today" selected=threshold_value == "today">"today"</option>
                        <option value="yesterday" selected=threshold_value == "yesterday">"yesterday"</option>
                        <option value="tomorrow" selected=threshold_value == "tomorrow">"tomorrow"</option>
                        <option value="last7Days" selected=threshold_value == "last7Days">"last 7 days"</option>
                        <option value="thisWeek" selected=threshold_value == "thisWeek">"this week"</option>
                        <option value="lastWeek" selected=threshold_value == "lastWeek">"last week"</option>
                        <option value="nextWeek" selected=threshold_value == "nextWeek">"next week"</option>
                        <option value="thisMonth" selected=threshold_value == "thisMonth">"this month"</option>
                        <option value="lastMonth" selected=threshold_value == "lastMonth">"last month"</option>
                        <option value="nextMonth" selected=threshold_value == "nextMonth">"next month"</option>
                    </select>
                </label>
            }
            .into_any()
        }
        _ => {
            let placeholder = if kind_lc == "expression" {
                "=A1>5"
            } else {
                "0"
            };
            let label = if kind_lc == "expression" {
                "formula"
            } else {
                "value"
            };
            view! {
                <label class="onecalc-home-shell__cf-rule-field">
                    <span class="onecalc-home-shell__cf-rule-field-label">{label}</span>
                    <input
                        type="text"
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-rule-field="threshold"
                        placeholder=placeholder
                        prop:value=threshold_value.clone()
                        value=threshold_value
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let mut next = host_cf_rule_from_view(&rule_for_threshold);
                            next.thresholds = vec![target.value()];
                            on_update.run((index, next));
                        }
                    />
                </label>
            }
            .into_any()
        }
    }
}

/// Seed default `typed_rule` and visible-style values when the user
/// picks an aggregate visualization or rank/average rule kind.
/// Existing values are preserved — this only fills in *empty* slots
/// so the rule is immediately functional after a kind switch
/// without forcing the user through a config dialog.
///
/// Per OxFml W073 (`HANDOFF-DNAONECALC-012`, 2026-05-04 update),
/// `typed_rule` is the **only** accepted metadata source for the
/// seven typed families: `colorScale`, `dataBar`, `iconSet`, `top`,
/// `bottom`, `aboveAverage`, `belowAverage`. The W072 bounded-string
/// `thresholds` convention is intentionally ignored upstream for
/// those kinds; the host therefore stops seeding `thresholds` for
/// them (and clears any stale entries on kind switch) and lets the
/// per-kind sub-form populate `typed_rule` directly.
///
/// `thresholds` still carries the real rule input for kinds that
/// need it — `cell_value` / `text` / `dates` / `expression` — and
/// `uniqueValues` / `duplicateValues` use only the kind itself.
fn seed_visualization_rule_defaults(rule: &mut crate::state::FormulaConditionalFormattingRule) {
    use crate::state::{
        FormulaAverageRuleOptions, FormulaColorScaleRuleOptions, FormulaColorScaleStop,
        FormulaConditionalFormattingRank, FormulaConditionalFormattingThreshold,
        FormulaConditionalFormattingTypedRule, FormulaDataBarRuleOptions,
        FormulaIconSetRuleOptions, FormulaRankRuleOptions,
    };

    let kind = rule.rule_kind.to_ascii_lowercase();
    match kind.as_str() {
        "colorscale" => {
            // OxFml W073 ignores bounded `thresholds` for this family.
            // Drop any stale entries so they don't persist and confuse
            // the typed-rule subform.
            rule.thresholds.clear();
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    color_scale: Some(FormulaColorScaleRuleOptions {
                        stops: vec![
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Min,
                                color: "#F8696B".to_string(),
                            },
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Percentile(50.0),
                                color: "#FFEB84".to_string(),
                            },
                            FormulaColorScaleStop {
                                position: FormulaConditionalFormattingThreshold::Max,
                                color: "#63BE7B".to_string(),
                            },
                        ],
                    }),
                    ..Default::default()
                });
            }
        }
        "databar" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#638EC6".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    data_bar: Some(FormulaDataBarRuleOptions {
                        bar_color: Some("#638EC6".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
        "iconset" => {
            rule.thresholds.clear();
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    icon_set: Some(FormulaIconSetRuleOptions {
                        set_kind: "3Arrows".to_string(),
                        thresholds: Vec::new(),
                    }),
                    ..Default::default()
                });
            }
        }
        "aboveaverage" | "belowaverage" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    average: Some(FormulaAverageRuleOptions::default()),
                    ..Default::default()
                });
            }
        }
        "top" | "bottom" => {
            rule.thresholds.clear();
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
            if rule.typed_rule.is_none() {
                rule.typed_rule = Some(FormulaConditionalFormattingTypedRule {
                    rank: Some(FormulaRankRuleOptions {
                        rank: FormulaConditionalFormattingRank::Count(10),
                    }),
                    ..Default::default()
                });
            }
        }
        "uniquevalues" | "duplicatevalues" => {
            if rule.fill_color.is_none() {
                rule.fill_color = Some("#FFE9B3".to_string());
            }
        }
        _ => {}
    }
}

/// Lift a view-model CF rule back to the host's state shape so the
/// per-field on-change handlers can produce a fresh, fully-populated
/// rule for the reducer. Used inline by `render_cf_rule_card`.
fn host_cf_rule_from_view(
    rule: &ConditionalFormattingRuleView,
) -> crate::state::FormulaConditionalFormattingRule {
    crate::state::FormulaConditionalFormattingRule {
        rule_kind: rule.rule_kind.clone(),
        operator: rule.operator.clone(),
        thresholds: rule.thresholds.clone(),
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule: rule.typed_rule.clone(),
    }
}

// ---------------------------------------------------------------------------
// Typed CF rule per-kind sub-form
//
// Renders a kind-specific authoring surface below the card header for
// the seven W073-typed families. Per OxFml `HANDOFF-DNAONECALC-012`
// (2026-05-04 update), `typed_rule` is the **only** accepted metadata
// source for these kinds — the bounded-string `thresholds` is
// upstream-ignored, so the sub-form is the rule's only authoring path.
//
// The seven typed kinds: colorScale, dataBar, iconSet, top, bottom,
// aboveAverage, belowAverage. Kinds outside this set render no
// sub-form (the existing top-row threshold control covers them).
// ---------------------------------------------------------------------------

fn render_cf_rule_typed_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    let kind = rule.rule_kind.to_ascii_lowercase();
    match kind.as_str() {
        "colorscale" => render_color_scale_subform(rule, index, on_update),
        "databar" => render_data_bar_subform(rule, index, on_update),
        "iconset" => render_icon_set_subform(rule, index, on_update),
        "top" | "bottom" => render_rank_subform(rule, index, on_update),
        "aboveaverage" | "belowaverage" => render_average_subform(rule, index, on_update),
        _ => view! { <span></span> }.into_any(),
    }
}

fn render_color_scale_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaColorScaleRuleOptions, FormulaColorScaleStop, FormulaConditionalFormattingTypedRule,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.color_scale.clone().unwrap_or_default();
    let stops = options.stops.clone();
    let stop_rows: Vec<_> = stops
        .iter()
        .enumerate()
        .map(|(stop_index, stop)| {
            let rule_for_kind = rule.clone();
            let rule_for_value = rule.clone();
            let rule_for_color = rule.clone();
            let rule_for_remove = rule.clone();
            let position_kind = threshold_kind_label(&stop.position);
            let position_value = threshold_numeric_value(&stop.position).unwrap_or(0.0);
            let needs_value = matches!(position_kind, "percent" | "percentile" | "num");
            let value_input = if needs_value {
                view! {
                    <input
                        type="number"
                        step="any"
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-typed-field="color-scale-stop-value"
                        prop:value=position_value.to_string()
                        value=position_value.to_string()
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                            let mut next = host_cf_rule_from_view(&rule_for_value);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                let kind = threshold_kind_label(&stop.position);
                                stop.position = threshold_from_kind_and_value(kind, parsed);
                            });
                            on_update.run((index, next));
                        }
                    />
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            };
            let color_value = normalize_color_for_input(&stop.color);
            view! {
                <div class="onecalc-home-shell__cf-rule-typed-stop">
                    <select
                        class="onecalc-home-shell__cf-rule-input"
                        data-cf-typed-field="color-scale-stop-kind"
                        on:change=move |ev| {
                            let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                            let kind = target.value();
                            let mut next = host_cf_rule_from_view(&rule_for_kind);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                let value = threshold_numeric_value(&stop.position).unwrap_or(0.0);
                                stop.position = threshold_from_kind_and_value(&kind, value);
                            });
                            on_update.run((index, next));
                        }
                    >
                        <option value="min" selected=position_kind == "min">"min"</option>
                        <option value="mid" selected=position_kind == "mid">"mid"</option>
                        <option value="max" selected=position_kind == "max">"max"</option>
                        <option value="percent" selected=position_kind == "percent">"%"</option>
                        <option value="percentile" selected=position_kind == "percentile">"pctl"</option>
                        <option value="num" selected=position_kind == "num">"num"</option>
                    </select>
                    {value_input}
                    <input
                        type="color"
                        class="onecalc-home-shell__formatting-color"
                        data-cf-typed-field="color-scale-stop-color"
                        prop:value=color_value.clone()
                        value=color_value
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            let color = target.value();
                            let mut next = host_cf_rule_from_view(&rule_for_color);
                            update_color_scale_stop(&mut next, stop_index, |stop| {
                                stop.color = color.clone();
                            });
                            on_update.run((index, next));
                        }
                    />
                    <button
                        type="button"
                        class="onecalc-home-shell__cf-rule-typed-stop-remove"
                        title="Remove this stop"
                        aria-label="remove color scale stop"
                        on:click=move |_| {
                            let mut next = host_cf_rule_from_view(&rule_for_remove);
                            remove_color_scale_stop(&mut next, stop_index);
                            on_update.run((index, next));
                        }
                    >
                        "✕"
                    </button>
                </div>
            }
            .into_any()
        })
        .collect();
    let rule_for_add = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="color-scale">
            {stop_rows}
            <button
                type="button"
                class="onecalc-home-shell__cf-rule-typed-add"
                title="Add a stop to the gradient"
                on:click=move |_| {
                    let mut next = host_cf_rule_from_view(&rule_for_add);
                    let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                    let options = typed
                        .color_scale
                        .get_or_insert_with(FormulaColorScaleRuleOptions::default);
                    options.stops.push(FormulaColorScaleStop {
                        position: crate::state::FormulaConditionalFormattingThreshold::Percentile(50.0),
                        color: "#FFEB84".to_string(),
                    });
                    on_update.run((index, next));
                }
            >
                "+ stop"
            </button>
        </div>
    }
    .into_any()
}

fn render_data_bar_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaConditionalFormattingTypedRule, FormulaDataBarDirection, FormulaDataBarRuleOptions,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.data_bar.clone().unwrap_or_default();
    let bar_color = options
        .bar_color
        .clone()
        .unwrap_or_else(|| "#638EC6".to_string());
    let direction_label = match options.direction.unwrap_or(FormulaDataBarDirection::Left) {
        FormulaDataBarDirection::Left => "left",
        FormulaDataBarDirection::Right => "right",
    };
    let show_bar_only = options.show_bar_only;
    let rule_for_color = rule.clone();
    let rule_for_dir = rule.clone();
    let rule_for_show = rule.clone();
    let bar_color_for_input = normalize_color_for_input(&bar_color);
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="data-bar">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"bar"</span>
                <input
                    type="color"
                    class="onecalc-home-shell__formatting-color"
                    data-cf-typed-field="data-bar-color"
                    prop:value=bar_color_for_input.clone()
                    value=bar_color_for_input
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let value = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_color);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.bar_color = Some(value);
                        on_update.run((index, next));
                    }
                />
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"dir"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="data-bar-direction"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let direction = match target.value().as_str() {
                            "right" => FormulaDataBarDirection::Right,
                            _ => FormulaDataBarDirection::Left,
                        };
                        let mut next = host_cf_rule_from_view(&rule_for_dir);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.direction = Some(direction);
                        on_update.run((index, next));
                    }
                >
                    <option value="left" selected=direction_label == "left">"left"</option>
                    <option value="right" selected=direction_label == "right">"right"</option>
                </select>
            </label>
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="data-bar-show-bar-only"
                    prop:checked=show_bar_only
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_show);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .data_bar
                            .get_or_insert_with(FormulaDataBarRuleOptions::default);
                        options.show_bar_only = checked;
                        on_update.run((index, next));
                    }
                />
                "show bar only"
            </label>
        </div>
    }
    .into_any()
}

fn render_icon_set_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{FormulaConditionalFormattingTypedRule, FormulaIconSetRuleOptions};
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed
        .icon_set
        .clone()
        .unwrap_or_else(|| FormulaIconSetRuleOptions {
            set_kind: "3Arrows".to_string(),
            thresholds: Vec::new(),
        });
    let set_kind = options.set_kind.clone();
    let icon_kinds = [
        "3Arrows",
        "3ArrowsGray",
        "3Flags",
        "3Symbols",
        "3Symbols2",
        "3Stars",
        "3Triangles",
        "4Arrows",
        "4ArrowsGray",
        "4RedToBlack",
        "4Rating",
        "4TrafficLights",
        "5Arrows",
        "5ArrowsGray",
        "5Rating",
        "5Quarters",
    ];
    let options_views: Vec<_> = icon_kinds
        .iter()
        .map(|kind| {
            let selected = *kind == set_kind;
            view! {
                <option value=kind.to_string() selected=selected>{kind.to_string()}</option>
            }
            .into_any()
        })
        .collect();
    let rule_for_kind = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="icon-set">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"set"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="icon-set-kind"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let value = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_kind);
                        // OxFml W073 ignores bounded `thresholds` for
                        // iconSet; drop any stale entries on edit so
                        // they don't survive into the saved file.
                        next.thresholds.clear();
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .icon_set
                            .get_or_insert_with(|| FormulaIconSetRuleOptions {
                                set_kind: value.clone(),
                                thresholds: Vec::new(),
                            });
                        options.set_kind = value;
                        on_update.run((index, next));
                    }
                >
                    {options_views}
                </select>
            </label>
        </div>
    }
    .into_any()
}

fn render_rank_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{
        FormulaConditionalFormattingRank, FormulaConditionalFormattingTypedRule,
        FormulaRankRuleOptions,
    };
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed
        .rank
        .clone()
        .unwrap_or_else(|| FormulaRankRuleOptions {
            rank: FormulaConditionalFormattingRank::Count(10),
        });
    let (mode_label, value): (&'static str, f64) = match &options.rank {
        FormulaConditionalFormattingRank::Count(count) => ("count", *count as f64),
        FormulaConditionalFormattingRank::Percent(value) => ("percent", *value),
    };
    let rule_for_mode = rule.clone();
    let rule_for_value = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="rank">
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"mode"</span>
                <select
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="rank-mode"
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
                        let mode = target.value();
                        let mut next = host_cf_rule_from_view(&rule_for_mode);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .rank
                            .get_or_insert_with(|| FormulaRankRuleOptions {
                                rank: FormulaConditionalFormattingRank::Count(10),
                            });
                        let prior_value: f64 = match &options.rank {
                            FormulaConditionalFormattingRank::Count(count) => *count as f64,
                            FormulaConditionalFormattingRank::Percent(value) => *value,
                        };
                        options.rank = match mode.as_str() {
                            "percent" => FormulaConditionalFormattingRank::Percent(prior_value),
                            _ => FormulaConditionalFormattingRank::Count(prior_value.max(0.0) as usize),
                        };
                        on_update.run((index, next));
                    }
                >
                    <option value="count" selected=mode_label == "count">"count"</option>
                    <option value="percent" selected=mode_label == "percent">"percent"</option>
                </select>
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"n"</span>
                <input
                    type="number"
                    step="any"
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="rank-value"
                    prop:value=value.to_string()
                    value=value.to_string()
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                        let mut next = host_cf_rule_from_view(&rule_for_value);
                        // OxFml W073 ignores bounded `thresholds` for
                        // top/bottom; drop any stale entries so they
                        // don't survive into the saved file.
                        next.thresholds.clear();
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .rank
                            .get_or_insert_with(|| FormulaRankRuleOptions {
                                rank: FormulaConditionalFormattingRank::Count(10),
                            });
                        options.rank = match &options.rank {
                            FormulaConditionalFormattingRank::Count(_) => {
                                FormulaConditionalFormattingRank::Count(parsed.max(0.0) as usize)
                            }
                            FormulaConditionalFormattingRank::Percent(_) => {
                                FormulaConditionalFormattingRank::Percent(parsed)
                            }
                        };
                        on_update.run((index, next));
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

fn render_average_subform(
    rule: ConditionalFormattingRuleView,
    index: usize,
    on_update: Callback<(usize, crate::state::FormulaConditionalFormattingRule)>,
) -> AnyView {
    use crate::state::{FormulaAverageRuleOptions, FormulaConditionalFormattingTypedRule};
    let typed = rule.typed_rule.clone().unwrap_or_default();
    let options = typed.average.clone().unwrap_or_default();
    let include_equal = options.include_equal;
    let stddev = options.stddev_multiplier.unwrap_or(0.0);
    let stddev_set = options.stddev_multiplier.is_some();
    let rule_for_equal = rule.clone();
    let rule_for_stddev_toggle = rule.clone();
    let rule_for_stddev_value = rule.clone();
    view! {
        <div class="onecalc-home-shell__cf-rule-typed-subform" data-cf-typed-kind="average">
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="average-include-equal"
                    prop:checked=include_equal
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_equal);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.include_equal = checked;
                        on_update.run((index, next));
                    }
                />
                "include equal"
            </label>
            <label class="onecalc-home-shell__cf-rule-typed-checkbox">
                <input
                    type="checkbox"
                    data-cf-typed-field="average-stddev-enabled"
                    prop:checked=stddev_set
                    on:change=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let checked = target.checked();
                        let mut next = host_cf_rule_from_view(&rule_for_stddev_toggle);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.stddev_multiplier = if checked {
                            Some(options.stddev_multiplier.unwrap_or(1.0))
                        } else {
                            None
                        };
                        on_update.run((index, next));
                    }
                />
                "stddev offset"
            </label>
            <label class="onecalc-home-shell__cf-rule-field">
                <span class="onecalc-home-shell__cf-rule-field-label">"k"</span>
                <input
                    type="number"
                    step="any"
                    class="onecalc-home-shell__cf-rule-input"
                    data-cf-typed-field="average-stddev-value"
                    prop:value=stddev.to_string()
                    value=stddev.to_string()
                    disabled=!stddev_set
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        let parsed = target.value().parse::<f64>().unwrap_or(0.0);
                        let mut next = host_cf_rule_from_view(&rule_for_stddev_value);
                        let typed = next.typed_rule.get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
                        let options = typed
                            .average
                            .get_or_insert_with(FormulaAverageRuleOptions::default);
                        options.stddev_multiplier = Some(parsed);
                        on_update.run((index, next));
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

// --- typed-rule helpers ---

fn threshold_kind_label(
    threshold: &crate::state::FormulaConditionalFormattingThreshold,
) -> &'static str {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match threshold {
        T::Min => "min",
        T::Mid => "mid",
        T::Max => "max",
        T::Percent(_) => "percent",
        T::Percentile(_) => "percentile",
        T::Number(_) => "num",
    }
}

fn threshold_numeric_value(
    threshold: &crate::state::FormulaConditionalFormattingThreshold,
) -> Option<f64> {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match threshold {
        T::Percent(value) | T::Percentile(value) | T::Number(value) => Some(*value),
        T::Min | T::Mid | T::Max => None,
    }
}

fn threshold_from_kind_and_value(
    kind: &str,
    value: f64,
) -> crate::state::FormulaConditionalFormattingThreshold {
    use crate::state::FormulaConditionalFormattingThreshold as T;
    match kind {
        "min" => T::Min,
        "mid" => T::Mid,
        "max" => T::Max,
        "percent" => T::Percent(value),
        "percentile" => T::Percentile(value),
        _ => T::Number(value),
    }
}

fn update_color_scale_stop(
    rule: &mut crate::state::FormulaConditionalFormattingRule,
    stop_index: usize,
    mutator: impl FnOnce(&mut crate::state::FormulaColorScaleStop),
) {
    use crate::state::{FormulaColorScaleRuleOptions, FormulaConditionalFormattingTypedRule};
    let typed = rule
        .typed_rule
        .get_or_insert_with(FormulaConditionalFormattingTypedRule::default);
    let options = typed
        .color_scale
        .get_or_insert_with(FormulaColorScaleRuleOptions::default);
    if let Some(stop) = options.stops.get_mut(stop_index) {
        mutator(stop);
    }
}

fn remove_color_scale_stop(
    rule: &mut crate::state::FormulaConditionalFormattingRule,
    stop_index: usize,
) {
    if let Some(typed) = rule.typed_rule.as_mut() {
        if let Some(options) = typed.color_scale.as_mut() {
            if stop_index < options.stops.len() {
                options.stops.remove(stop_index);
            }
        }
    }
}

fn render_number_format_presets(
    presets: Vec<NumberFormatPreset>,
    on_set: Callback<String>,
) -> AnyView {
    let chips: Vec<_> = presets
        .into_iter()
        .map(|preset| {
            let label = preset.label;
            let format_code = preset.format_code;
            let seam_id = preset.seam_id.unwrap_or("");
            let seam_attr = seam_id.to_string();
            let seam_badge = preset.seam_id.map(|seam| {
                view! {
                    <span
                        class="onecalc-home-shell__formatting-preset-seam"
                        data-seam-id=seam.to_string()
                        title=format!("<NOT IMPLEMENTED> {seam}")
                    >"⚠"</span>
                }
                .into_any()
            });
            view! {
                <button
                    type="button"
                    class="onecalc-home-shell__formatting-preset"
                    data-format-code=format_code
                    data-seam-id=seam_attr
                    on:click=move |_| {
                        on_set.run(format_code.to_string());
                    }
                >
                    {label}
                    {seam_badge}
                </button>
            }
            .into_any()
        })
        .collect();
    view! { <>{chips}</> }.into_any()
}

/// `<input type="color">` requires a `#RRGGBB` value with a leading
/// hash; an empty string makes Edge / Chrome render the picker as
/// black. Map empty → `#000000` for the control's prop:value while
/// preserving the empty state in the underlying scenario (so an
/// untouched color still serialises as empty / inherit).
fn normalize_color_for_input(raw: &str) -> String {
    if raw.is_empty() {
        "#000000".to_string()
    } else {
        raw.to_string()
    }
}

/// Render persistence-loader warning chips in the status-foot
/// (slice 3). Empty when `load_diagnostics` is empty so the chrome
/// stays minimal. The chip's `data-load-diagnostic` attribute
/// carries the diagnostic slug for browser-test inspection; the
/// `title` carries the human-readable message.
fn render_load_diagnostic_chips(diagnostics: Vec<crate::persistence::LoadDiagnostic>) -> AnyView {
    if diagnostics.is_empty() {
        return view! { <></> }.into_any();
    }
    let chips: Vec<_> = diagnostics
        .into_iter()
        .map(|diagnostic| {
            let slug = diagnostic.slug();
            let message = diagnostic.user_message();
            view! {
                <>
                    <span class="onecalc-home-shell__statusfoot-sep">"·"</span>
                    <span
                        class="onecalc-home-shell__statusfoot-load-warning"
                        data-load-diagnostic=slug
                        title=message
                    >
                        "⚠ imported (Excel-only)"
                    </span>
                </>
            }
            .into_any()
        })
        .collect();
    view! { <>{chips}</> }.into_any()
}

/// Trim a long `green:abcdef0123...` key down to a status-foot-friendly
/// `abcdef…` form, matching the WS-14 mockup convention.
fn short_green_tree_key(key: &str) -> String {
    let body = key.strip_prefix("green:").unwrap_or(key);
    if body.chars().count() <= 7 {
        return body.to_string();
    }
    let mut short = body.chars().take(6).collect::<String>();
    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    use super::short_green_tree_key;

    #[test]
    fn short_green_tree_key_strips_prefix_and_trims_long_keys() {
        let key = "green:a3f91eabc1234";
        assert_eq!(short_green_tree_key(key), "a3f91e…");
    }

    #[test]
    fn short_green_tree_key_passes_short_keys_through() {
        assert_eq!(short_green_tree_key("green:abc123"), "abc123");
        assert_eq!(short_green_tree_key("abc123"), "abc123");
        assert_eq!(short_green_tree_key(""), "");
    }

    #[test]
    fn short_green_tree_key_handles_non_prefixed_keys() {
        let key = "abcdefghijklmnop";
        assert_eq!(short_green_tree_key(key), "abcdef…");
    }
}

/// Render the appropriate result-block content per `ResultView` variant.
/// All variants reach into the result-block container and supply class +
/// content; the container's CSS supplies the layout (centered, large).
fn render_result_view(view: Option<ResultView>, state: HostStateSignal) -> AnyView {
    match view {
        None => view! { <em class="muted">"awaiting input"</em> }.into_any(),
        Some(ResultView::Empty) => view! { <em class="muted">"awaiting input"</em> }.into_any(),
        Some(ResultView::Pending) => view! { <em class="muted">"…"</em> }.into_any(),
        Some(ResultView::Display {
            text,
            kind,
            applied_font_color,
            applied_fill_color,
        }) => {
            // CF-applied font / fill colours flow through inline
            // `style="color: …; background: …"`. `data-cf-applied`
            // is set when either is present so the corpus can pin
            // the visible-CF state without parsing inline CSS.
            let mut style = String::new();
            if let Some(font) = applied_font_color.as_deref() {
                style.push_str(&format!("color: {}; ", font));
            }
            if let Some(fill) = applied_fill_color.as_deref() {
                style.push_str(&format!("background: {}; ", fill));
            }
            let cf_applied = applied_font_color.is_some() || applied_fill_color.is_some();
            view! {
                <span
                    class="value"
                    data-kind=display_kind_attr(kind)
                    data-cf-applied=if cf_applied { "true" } else { "false" }
                    style=style
                >
                    {text}
                </span>
            }
            .into_any()
        }
        Some(ResultView::Error { code, surface_repr }) => {
            let code_for_attr = code.clone();
            view! {
                <span class="value error" data-code=code_for_attr>
                    <span class="value__code">{code}</span>
                    {surface_repr.map(|repr| view! {
                        <span class="value__surface">{repr}</span>
                    })}
                </span>
            }
            .into_any()
        }
        Some(ResultView::Array {
            total_rows,
            total_cols,
            label: _,
            cells,
            cell_format,
            truncated,
        }) => render_array_browser(total_rows, total_cols, cells, cell_format, truncated, state),
    }
}

/// Render the array-result browser (WS-14 §3 item 6). The container
/// is `overflow: auto; resize: both` so the user can scroll and drag-
/// resize the panel. The grid itself uses CSS-grid with sticky row /
/// column headers so the addresses stay visible as the user scrolls.
/// When the bridge truncated the preview window, surface a chip with
/// `+N rows · +M cols hidden` so the user knows the visible cells
/// are a subset.
///
/// `cell_format`, when present, supplies per-cell CF outcomes
/// (W071 + W072): font/fill colour, data bar fill ratio, and / or
/// icon glyph. Each cell renders with the matching style; cells
/// without an outcome render in the default chrome.
fn render_array_browser(
    total_rows: usize,
    total_cols: usize,
    cells: Vec<Vec<String>>,
    cell_format: Option<Vec<Vec<ArrayCellFormatView>>>,
    truncated: bool,
    state: HostStateSignal,
) -> AnyView {
    let preview_rows = cells.len();
    let preview_cols = cells.first().map(|row| row.len()).unwrap_or(0);
    let hidden_rows = total_rows.saturating_sub(preview_rows);
    let hidden_cols = total_cols.saturating_sub(preview_cols);
    // Read transient session state + workspace display options
    // up-front so the renderer doesn't reactively re-subscribe
    // for every cell we render. Each closure-captured state
    // mutation re-runs the parent reactive context, which
    // re-runs `render_array_browser` from scratch with the new
    // values.
    let (zoom, selection, column_widths, row_heights, display) = state.with(|s| {
        let space = s
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| s.formula_spaces.get(id));
        let zoom = space.map(|space| space.array_browser.zoom).unwrap_or(1.0);
        let selection = space.and_then(|space| space.array_browser.selection);
        let column_widths = space
            .map(|space| space.array_browser.column_widths_rem.clone())
            .unwrap_or_default();
        let row_heights = space
            .map(|space| space.array_browser.row_heights_rem.clone())
            .unwrap_or_default();
        (
            zoom,
            selection,
            column_widths,
            row_heights,
            s.global_ui_chrome.array_browser_display,
        )
    });
    // Build the per-column track template from `column_widths`,
    // falling back to `minmax(4rem, max-content)` for any column
    // the user hasn't resized. The leading `2.4rem` slot is the
    // row-number column — only present when headers are visible,
    // otherwise the first data column would still render at the
    // narrow row-number width.
    let show_headers = display.show_row_column_headers;
    let mut tracks = if show_headers {
        String::from("2.4rem ")
    } else {
        String::new()
    };
    for col in 0..preview_cols.max(1) {
        if let Some(width) = column_widths.get(&col) {
            tracks.push_str(&format!("{width:.2}rem "));
        } else {
            tracks.push_str("minmax(4rem, max-content) ");
        }
    }
    let mut row_tracks = if show_headers {
        String::from("minmax(1.6rem, max-content) ")
    } else {
        String::new()
    };
    for row in 0..preview_rows.max(1) {
        if let Some(height) = row_heights.get(&row) {
            row_tracks.push_str(&format!("{height:.2}rem "));
        } else {
            row_tracks.push_str("minmax(1.6rem, max-content) ");
        }
    }
    let grid_template =
        format!("grid-template-columns: {tracks}; grid-template-rows: {row_tracks};");
    let mut header_cells: Vec<AnyView> = if show_headers {
        Vec::with_capacity(preview_cols + 1)
    } else {
        Vec::new()
    };
    if show_headers {
        // Top-left "select-all" corner: clicking selects every
        // visible cell in the preview window. Excel uses this same
        // corner the same way.
        let select_all_rows = preview_rows;
        let select_all_cols = preview_cols;
        header_cells.push(
            view! {
                <div
                    class="onecalc-array-browser__header onecalc-array-browser__corner"
                    title="Click to select all visible cells"
                    aria-label="select all"
                    on:click=move |ev: WebMouseEvent| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        select_all_visible_cells(state, select_all_rows, select_all_cols);
                    }
                ></div>
            }
            .into_any(),
        );
        for col in 0..preview_cols {
            let label = column_index_to_a1_label(col);
            let col_for_resize = col;
            let col_for_select = col;
            let select_rows_for_col = preview_rows;
            let initial_width = column_widths.get(&col).copied().unwrap_or(4.0); // matches the `4rem` minmax minimum.
            header_cells.push(
                view! {
                    <div
                        class="onecalc-array-browser__header onecalc-array-browser__column-header"
                        data-col=col.to_string()
                        title="Click to select column · Shift+click to extend"
                        on:click=move |ev: WebMouseEvent| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            select_column(
                                state,
                                col_for_select,
                                select_rows_for_col,
                                ev.shift_key(),
                            );
                        }
                    >
                        <span class="onecalc-array-browser__header-label">{label}</span>
                        <span
                            class="onecalc-array-browser__resize-handle onecalc-array-browser__resize-handle--col"
                            title="Drag to resize column"
                            on:mousedown=move |ev: WebMouseEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                start_column_resize(state, col_for_resize, initial_width, ev.client_x());
                            }
                        ></span>
                    </div>
                }
                .into_any(),
            );
        }
    }
    let body_cell_capacity = if show_headers {
        preview_rows * (preview_cols + 1)
    } else {
        preview_rows * preview_cols
    };
    let mut body_cells: Vec<AnyView> = Vec::with_capacity(body_cell_capacity);
    for (row_index, row) in cells.into_iter().enumerate() {
        if show_headers {
            let row_label = (row_index + 1).to_string();
            let row_for_resize = row_index;
            let row_for_select = row_index;
            let select_cols_for_row = preview_cols;
            let initial_height = row_heights.get(&row_index).copied().unwrap_or(1.6);
            body_cells.push(
                view! {
                    <div
                        class="onecalc-array-browser__header onecalc-array-browser__row-header"
                        data-row=row_index.to_string()
                        title="Click to select row · Shift+click to extend"
                        on:click=move |ev: WebMouseEvent| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            select_row(
                                state,
                                row_for_select,
                                select_cols_for_row,
                                ev.shift_key(),
                            );
                        }
                    >
                        <span class="onecalc-array-browser__header-label">{row_label}</span>
                        <span
                            class="onecalc-array-browser__resize-handle onecalc-array-browser__resize-handle--row"
                            title="Drag to resize row"
                            on:mousedown=move |ev: WebMouseEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                start_row_resize(state, row_for_resize, initial_height, ev.client_y());
                            }
                        ></span>
                    </div>
                }
                .into_any(),
            );
        }
        let row_len = row.len();
        for (col_index, cell_value) in row.into_iter().enumerate() {
            let format_for_cell = cell_format
                .as_ref()
                .and_then(|grid| grid.get(row_index))
                .and_then(|row| row.get(col_index));
            let is_selected = selection
                .map(|sel| sel.contains(row_index, col_index))
                .unwrap_or(false);
            body_cells.push(render_array_browser_cell(
                row_index,
                col_index,
                cell_value,
                format_for_cell,
                is_selected,
                state,
            ));
        }
        // Pad the final row if it's shorter than the column count
        // (defensive — the bridge already pads, but the cell count
        // attribute only counts cells it emitted).
        let row_parity_attr = if row_index % 2 == 0 { "even" } else { "odd" };
        for col_pad in row_len..preview_cols {
            body_cells.push(
                view! {
                    <div
                        class="onecalc-array-browser__cell onecalc-array-browser__cell--empty"
                        data-row=row_index.to_string()
                        data-col=col_pad.to_string()
                        data-row-parity=row_parity_attr
                    ></div>
                }
                .into_any(),
            );
        }
    }
    let truncation_chip = if truncated {
        let mut bits: Vec<String> = Vec::new();
        if hidden_rows > 0 {
            bits.push(format!("+{} rows", hidden_rows));
        }
        if hidden_cols > 0 {
            bits.push(format!("+{} cols", hidden_cols));
        }
        let detail = if bits.is_empty() {
            "more cells hidden".to_string()
        } else {
            format!("{} hidden", bits.join(" · "))
        };
        view! {
            <div class="onecalc-array-browser__truncation" data-truncated="true">
                {detail}
            </div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };
    let zoom_attr = format!("{:.2}", zoom);
    // Use the `zoom` CSS property so the whole grid scales — cell
    // padding, column widths (in rem), row heights, fonts, gaps,
    // borders. `font-size` alone only scales glyphs but leaves
    // rem-based column widths fixed (rem is root-relative), which
    // is what produced the "only fonts grow" complaint. `zoom` is
    // supported in Chrome/Edge/Safari and shipped in Firefox
    // since 126; on the few engines that still ignore it, the
    // grid simply renders at 1× — degraded but not broken.
    let zoom_style = format!("zoom: {:.2};", zoom);
    let grid_lines_attr = if display.show_grid_lines {
        "true"
    } else {
        "false"
    };
    let alt_rows_attr = if display.show_alternating_rows {
        "true"
    } else {
        "false"
    };
    let headers_attr = if display.show_row_column_headers {
        "true"
    } else {
        "false"
    };
    let cells_for_copy = state.with(|s| {
        s.workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| s.formula_spaces.get(id))
            .and_then(|space| space.array_preview.as_ref())
            .map(|preview| preview.rows.clone())
            .unwrap_or_default()
    });
    let toolbar = render_array_browser_toolbar(
        state,
        zoom,
        selection,
        display,
        total_rows,
        total_cols,
        cells_for_copy,
    );
    view! {
        <div
            class="onecalc-array-browser"
            data-total-rows=total_rows.to_string()
            data-total-cols=total_cols.to_string()
            data-preview-rows=preview_rows.to_string()
            data-preview-cols=preview_cols.to_string()
            data-truncated=if truncated { "true" } else { "false" }
            data-zoom=zoom_attr
            data-show-grid-lines=grid_lines_attr
            data-show-alternating-rows=alt_rows_attr
            data-show-headers=headers_attr
            role="region"
            aria-label="array result browser"
            tabindex="0"
            on:keydown=move |ev: WebKeyboardEvent| {
                // Ctrl+C: copy the current rectangular block
                // selection (or the whole preview when nothing is
                // selected) as tab-separated text.
                if ev.ctrl_key() && (ev.key() == "c" || ev.key() == "C") {
                    ev.prevent_default();
                    copy_array_browser_selection_to_clipboard(state);
                }
                // Ctrl+A: select every cell in the visible preview
                // window. Mirrors the corner-cell click affordance.
                if ev.ctrl_key() && (ev.key() == "a" || ev.key() == "A") {
                    ev.prevent_default();
                    select_all_visible_cells(state, preview_rows, preview_cols);
                }
                // Ctrl + +/-/0: zoom keyboard shortcuts.
                if ev.ctrl_key() && (ev.key() == "=" || ev.key() == "+") {
                    ev.prevent_default();
                    state.update(|state| {
                        let _ = crate::app::reducer::zoom_in_active_array_browser(state);
                    });
                } else if ev.ctrl_key() && ev.key() == "-" {
                    ev.prevent_default();
                    state.update(|state| {
                        let _ = crate::app::reducer::zoom_out_active_array_browser(state);
                    });
                } else if ev.ctrl_key() && ev.key() == "0" {
                    ev.prevent_default();
                    state.update(|state| {
                        let _ = crate::app::reducer::reset_active_array_browser_zoom(state);
                    });
                }
            }
            on:wheel=move |ev: web_sys::WheelEvent| {
                // Ctrl+wheel zoom mirrors Excel / browser convention.
                if !ev.ctrl_key() {
                    return;
                }
                ev.prevent_default();
                let delta_y = ev.delta_y();
                state.update(|state| {
                    if delta_y < 0.0 {
                        let _ = crate::app::reducer::zoom_in_active_array_browser(state);
                    } else if delta_y > 0.0 {
                        let _ = crate::app::reducer::zoom_out_active_array_browser(state);
                    }
                });
            }
        >
            {toolbar}
            <div class="onecalc-array-browser__caption">
                {format!("Array[{} × {}]", total_rows, total_cols)}
            </div>
            <div
                class="onecalc-array-browser__scroll"
                style=format!("{} {}", grid_template, zoom_style)
                role="grid"
                aria-rowcount=total_rows.to_string()
                aria-colcount=total_cols.to_string()
            >
                {header_cells}
                {body_cells}
            </div>
            {truncation_chip}
        </div>
    }
    .into_any()
}

/// Render the array-browser toolbar above the grid: zoom buttons,
/// display-option toggles, current-selection summary, and a Copy
/// button for the rectangular block selection.
fn render_array_browser_toolbar(
    state: HostStateSignal,
    zoom: f32,
    selection: Option<crate::state::ArrayBlockSelection>,
    display: crate::state::ArrayBrowserDisplaySettings,
    total_rows: usize,
    total_cols: usize,
    cells: Vec<Vec<String>>,
) -> AnyView {
    use crate::app::reducer::ArrayBrowserDisplayToggle;
    let zoom_label = format!("{:.0}%", zoom * 100.0);
    let selection_summary = match selection {
        Some(sel) => {
            let (r0, r1, c0, c1) = sel.normalized();
            let rows = r1 - r0 + 1;
            let cols = c1 - c0 + 1;
            format!(
                "{}×{} selected · {}",
                rows,
                cols,
                cell_range_label(r0, c0, r1, c1)
            )
        }
        None => format!("{}×{} preview", total_rows, total_cols),
    };
    let cells_for_copy_btn = cells;
    view! {
        <div class="onecalc-array-browser__toolbar" role="toolbar" aria-label="array browser toolbar">
            <div class="onecalc-array-browser__toolbar-group">
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    title="Zoom out (Ctrl+-)"
                    aria-label="zoom out"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::zoom_out_active_array_browser(s);
                    })
                >
                    "−"
                </button>
                <span class="onecalc-array-browser__toolbar-zoom-label" data-zoom=format!("{:.2}", zoom)>
                    {zoom_label}
                </span>
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    title="Zoom in (Ctrl++)"
                    aria-label="zoom in"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::zoom_in_active_array_browser(s);
                    })
                >
                    "+"
                </button>
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    title="Reset zoom (Ctrl+0)"
                    aria-label="reset zoom"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::reset_active_array_browser_zoom(s);
                    })
                >
                    "100%"
                </button>
            </div>
            <div class="onecalc-array-browser__toolbar-group">
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    data-active=if display.show_grid_lines { "true" } else { "false" }
                    aria-pressed=if display.show_grid_lines { "true" } else { "false" }
                    title="Toggle grid lines"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::toggle_array_browser_display_option(
                            s, ArrayBrowserDisplayToggle::GridLines,
                        );
                    })
                >
                    "Grid"
                </button>
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    data-active=if display.show_alternating_rows { "true" } else { "false" }
                    aria-pressed=if display.show_alternating_rows { "true" } else { "false" }
                    title="Toggle alternating row stripes"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::toggle_array_browser_display_option(
                            s, ArrayBrowserDisplayToggle::AlternatingRows,
                        );
                    })
                >
                    "Stripes"
                </button>
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    data-active=if display.show_row_column_headers { "true" } else { "false" }
                    aria-pressed=if display.show_row_column_headers { "true" } else { "false" }
                    title="Toggle row / column headers"
                    on:click=move |_| state.update(|s| {
                        let _ = crate::app::reducer::toggle_array_browser_display_option(
                            s, ArrayBrowserDisplayToggle::RowColumnHeaders,
                        );
                    })
                >
                    "Headers"
                </button>
            </div>
            <div class="onecalc-array-browser__toolbar-group onecalc-array-browser__toolbar-group--info">
                <span class="onecalc-array-browser__toolbar-info" aria-live="polite">
                    {selection_summary}
                </span>
                <button
                    type="button"
                    class="onecalc-array-browser__toolbar-button"
                    title="Copy selection (Ctrl+C)"
                    aria-label="copy selection"
                    on:click=move |_| {
                        copy_cells_to_clipboard(state, &cells_for_copy_btn);
                    }
                >
                    "Copy"
                </button>
            </div>
        </div>
    }
    .into_any()
}

/// Format an inclusive cell range into Excel-style A1 notation
/// (`A1:C5` etc.). Used by the toolbar's selection-summary text.
fn cell_range_label(r0: usize, c0: usize, r1: usize, c1: usize) -> String {
    let start = format!("{}{}", column_index_to_a1_label(c0), r0 + 1);
    if r0 == r1 && c0 == c1 {
        start
    } else {
        format!("{}:{}{}", start, column_index_to_a1_label(c1), r1 + 1)
    }
}

/// Copy the current rectangular block selection (or the entire
/// preview when nothing is selected) as tab-separated text. On
/// wasm this writes to the system clipboard via the async
/// Clipboard API; on non-wasm this is a no-op (tests don't have
/// a clipboard).
fn copy_array_browser_selection_to_clipboard(state: HostStateSignal) {
    let cells = state.with(|s| {
        s.workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| s.formula_spaces.get(id))
            .and_then(|space| space.array_preview.as_ref())
            .map(|preview| preview.rows.clone())
            .unwrap_or_default()
    });
    copy_cells_to_clipboard(state, &cells);
}

#[cfg(target_arch = "wasm32")]
fn copy_cells_to_clipboard(state: HostStateSignal, cells: &[Vec<String>]) {
    let selection = state.with(|s| {
        s.workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| s.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection)
    });
    let payload = build_clipboard_tsv(cells, selection);
    let Some(window) = web_sys::window() else {
        return;
    };
    let nav = window.navigator();
    let clipboard = nav.clipboard();
    let _ = clipboard.write_text(&payload);
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_cells_to_clipboard(state: HostStateSignal, cells: &[Vec<String>]) {
    // No clipboard access on the SSR / test path. The
    // serialisation logic still runs here so native builds keep
    // the same selection-to-TSV path type-checked.
    let selection = state.with(|s| {
        s.workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| s.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection)
    });
    drop(build_clipboard_tsv(cells, selection));
}

/// Build the tab-separated payload for clipboard copy. Without a
/// selection: the whole preview, row-by-row, tab between cells,
/// `\n` between rows. With a selection: the rectangular sub-grid
/// inside the selection's normalised bounds.
pub(crate) fn build_clipboard_tsv(
    cells: &[Vec<String>],
    selection: Option<crate::state::ArrayBlockSelection>,
) -> String {
    let (r0, r1, c0, c1) = match selection {
        Some(sel) => sel.normalized(),
        None => {
            let rows = cells.len();
            let cols = cells.first().map(|row| row.len()).unwrap_or(0);
            if rows == 0 || cols == 0 {
                return String::new();
            }
            (0, rows - 1, 0, cols - 1)
        }
    };
    let mut out = String::new();
    for r in r0..=r1 {
        let Some(row) = cells.get(r) else {
            continue;
        };
        let row_part: Vec<&str> = (c0..=c1)
            .map(|c| row.get(c).map(|s| s.as_str()).unwrap_or(""))
            .collect();
        out.push_str(&row_part.join("\t"));
        if r < r1 {
            out.push('\n');
        }
    }
    out
}

/// Render one cell of the array browser, applying CF formatting
/// when the cell carries a per-cell outcome.
///
/// Composition order (for cells with multiple outcomes):
/// 1. Inline `style="background: …; color: …"` for fill / font.
/// 2. Data-bar background overlay via a separate inline-block
///    sized to the bar's `fill_ratio`.
/// 3. Icon glyph rendered ahead of the value.
///
/// A cell with `show_bar_only = true` on its data-bar still emits
/// the value text but hidden via `visibility: hidden` so the bar
/// width remains anchored to the cell's natural width.
fn render_array_browser_cell(
    row_index: usize,
    col_index: usize,
    cell_value: String,
    cell_format: Option<&ArrayCellFormatView>,
    is_selected: bool,
    state: HostStateSignal,
) -> AnyView {
    let cell_for_attr = cell_value.clone();
    let mut style = String::new();
    let mut data_attrs: Vec<(&'static str, String)> = Vec::new();

    if let Some(format) = cell_format {
        if let Some(font) = format.effective_font_color.as_deref() {
            style.push_str(&format!("color: {}; ", font));
        }
        if let Some(fill) = format.effective_fill_color.as_deref() {
            style.push_str(&format!("background: {}; ", fill));
        }
        data_attrs.push(("data-cf-applied", "true".to_string()));
    }

    // Icon glyph (if any) rendered ahead of the value.
    let icon_glyph = cell_format
        .and_then(|format| format.icon.as_ref())
        .map(|icon| icon_glyph_for(&icon.set_kind, icon.icon_index));

    // Data bar overlay (if any) rendered as a background layer
    // sized to fill_ratio.
    let data_bar_overlay = cell_format
        .and_then(|format| format.data_bar.as_ref())
        .map(|bar| {
            let percent = (bar.fill_ratio.clamp(0.0, 1.0) * 100.0).round() as u32;
            let direction_attr = match bar.direction {
                DataBarDirectionView::Left => "left",
                DataBarDirectionView::Right => "right",
            };
            let bar_color = bar.bar_color.clone();
            let style = format!("width: {percent}%; background: {bar_color};");
            view! {
                <span
                    class="onecalc-array-browser__data-bar"
                    data-direction=direction_attr
                    style=style
                ></span>
            }
            .into_any()
        });
    let value_visibility_class = cell_format
        .and_then(|format| format.data_bar.as_ref())
        .map(|bar| {
            if bar.show_bar_only {
                "onecalc-array-browser__cell-value--hidden"
            } else {
                ""
            }
        })
        .unwrap_or("");

    let mut cell_classes = String::from("onecalc-array-browser__cell");
    if cell_format.is_some() {
        cell_classes.push_str(" onecalc-array-browser__cell--cf");
    }
    if is_selected {
        cell_classes.push_str(" onecalc-array-browser__cell--selected");
    }

    let icon_attr = icon_glyph
        .as_ref()
        .map(|(set_kind, _)| set_kind.clone())
        .unwrap_or_default();
    let icon_view = icon_glyph.map(|(_, glyph)| {
        view! { <span class="onecalc-array-browser__icon">{glyph}</span> }.into_any()
    });

    let selected_attr = if is_selected { "true" } else { "false" };
    let row_parity_attr = if row_index % 2 == 0 { "even" } else { "odd" };
    view! {
        <div
            class=cell_classes
            data-row=row_index.to_string()
            data-col=col_index.to_string()
            data-row-parity=row_parity_attr
            data-cf-applied=if cell_format.is_some() { "true" } else { "false" }
            data-icon-set=icon_attr
            data-is-selected=selected_attr
            title=cell_for_attr
            style=style
            on:mousedown=move |ev: WebMouseEvent| {
                // Mouse-down begins a rectangular block selection.
                // Anchor + focus both at this cell; drag extends
                // focus via the document-level mousemove handler
                // installed by `start_block_selection_drag`.
                ev.prevent_default();
                let extend = ev.shift_key();
                start_block_selection_drag(state, row_index, col_index, extend);
            }
            on:mouseenter=move |ev: WebMouseEvent| {
                // While the drag is active (button held), update
                // the focus corner to wherever the cursor is now.
                if (ev.buttons() & 1) == 1 {
                    extend_block_selection_drag(state, row_index, col_index);
                }
            }
        >
            {data_bar_overlay}
            {icon_view}
            <span class={
                if value_visibility_class.is_empty() {
                    "onecalc-array-browser__cell-value".to_string()
                } else {
                    format!("onecalc-array-browser__cell-value {value_visibility_class}")
                }
            }>
                {cell_value}
            </span>
        </div>
    }
    .into_any()
}

/// Begin a rectangular block selection at `(row, col)`. When
/// `extend == true` (shift-click), keeps the existing anchor and
/// just moves the focus to this cell — Excel's "extend the
/// selection to here" behaviour. Otherwise both corners snap to
/// the new cell. The continuing-drag-extends behaviour is wired
/// at the cell level via `on:mouseenter` while the mouse button
/// is held.
fn start_block_selection_drag(state: HostStateSignal, row: usize, col: usize, extend: bool) {
    use crate::state::ArrayBlockSelection;
    state.update(|state| {
        let next = if extend {
            // Extend: keep the current anchor (or fall back to
            // the same cell when no selection exists yet).
            let prior = state
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .and_then(|id| state.formula_spaces.get(id))
                .and_then(|space| space.array_browser.selection);
            match prior {
                Some(prior) => {
                    ArrayBlockSelection::from_corners(prior.anchor_row, prior.anchor_col, row, col)
                }
                None => ArrayBlockSelection::from_corners(row, col, row, col),
            }
        } else {
            ArrayBlockSelection::from_corners(row, col, row, col)
        };
        let _ = crate::app::reducer::set_active_array_browser_selection(state, Some(next));
    });
}

fn extend_block_selection_drag(state: HostStateSignal, row: usize, col: usize) {
    use crate::state::ArrayBlockSelection;
    state.update(|state| {
        let prior = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection);
        let Some(prior) = prior else {
            return;
        };
        let next = ArrayBlockSelection::from_corners(prior.anchor_row, prior.anchor_col, row, col);
        let _ = crate::app::reducer::set_active_array_browser_selection(state, Some(next));
    });
}

/// Select an entire column in the active array browser. Thin
/// wrapper that reads the prior selection out of host state, calls
/// the pure helper [`crate::app::reducer::compute_column_header_selection`],
/// and stores the result. Plain click replaces the selection;
/// Shift+click extends from the prior anchor.
fn select_column(state: HostStateSignal, col: usize, preview_rows: usize, extend: bool) {
    state.update(|state| {
        let prior = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection);
        let next =
            crate::app::reducer::compute_column_header_selection(prior, col, preview_rows, extend);
        let _ = crate::app::reducer::set_active_array_browser_selection(state, next);
    });
}

/// Select an entire row in the active array browser. Mirror of
/// [`select_column`] — see
/// [`crate::app::reducer::compute_row_header_selection`] for the
/// computation rule.
fn select_row(state: HostStateSignal, row: usize, preview_cols: usize, extend: bool) {
    state.update(|state| {
        let prior = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection);
        let next =
            crate::app::reducer::compute_row_header_selection(prior, row, preview_cols, extend);
        let _ = crate::app::reducer::set_active_array_browser_selection(state, next);
    });
}

/// Select every cell in the visible preview window. Hooked up to
/// the corner-cell click handler and the `Ctrl+A` keyboard
/// shortcut. The selection covers exactly the rendered preview
/// rectangle — if the bridge truncated the array, hidden cells
/// remain unselected.
fn select_all_visible_cells(state: HostStateSignal, preview_rows: usize, preview_cols: usize) {
    let next =
        crate::app::reducer::compute_select_all_visible_selection(preview_rows, preview_cols);
    state.update(|state| {
        let _ = crate::app::reducer::set_active_array_browser_selection(state, next);
    });
}

/// Begin a column-resize gesture. Installs document-level
/// mousemove + mouseup handlers that translate horizontal cursor
/// motion into width adjustments on the dragged column. When the
/// dragged column is part of a multi-column selection, every
/// column in that selection resizes together — each column moves
/// to its own initial width plus the same delta, so the user's
/// proportional layout survives the drag.
#[cfg(target_arch = "wasm32")]
fn start_column_resize(state: HostStateSignal, column: usize, initial_rem: f32, initial_x: i32) {
    let targets = resolve_column_resize_targets(state, column, initial_rem);
    install_resize_drag(state, ResizeAxis::Columns(targets), initial_x);
}

#[cfg(not(target_arch = "wasm32"))]
fn start_column_resize(
    _state: HostStateSignal,
    _column: usize,
    _initial_rem: f32,
    _initial_x: i32,
) {
}

#[cfg(target_arch = "wasm32")]
fn start_row_resize(state: HostStateSignal, row: usize, initial_rem: f32, initial_y: i32) {
    let targets = resolve_row_resize_targets(state, row, initial_rem);
    install_resize_drag(state, ResizeAxis::Rows(targets), initial_y);
}

#[cfg(not(target_arch = "wasm32"))]
fn start_row_resize(_state: HostStateSignal, _row: usize, _initial_rem: f32, _initial_y: i32) {}

/// One axis-target captured at drag start. Holds the (index,
/// initial-rem) pair so the drag closure can compute every
/// target's final size from `initial + delta` independently —
/// preserving any pre-existing per-column / per-row width
/// differences across a multi-track resize.
#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct ResizeTarget {
    index: usize,
    initial_rem: f32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
enum ResizeAxis {
    Columns(Vec<ResizeTarget>),
    Rows(Vec<ResizeTarget>),
}

/// Build the list of columns to resize together when the user
/// drags column `column`'s resize handle. When the column is
/// part of a multi-column selection, the whole selection range
/// resizes; each column captures its own pre-drag width so the
/// drag preserves any existing relative widths.
#[cfg(target_arch = "wasm32")]
fn resolve_column_resize_targets(
    state: HostStateSignal,
    column: usize,
    initial_rem: f32,
) -> Vec<ResizeTarget> {
    state.with_untracked(|state| {
        let space = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id));
        let multi_column_selection = space
            .and_then(|space| space.array_browser.selection)
            .filter(|selection| {
                let (_, _, c0, c1) = selection.normalized();
                c0 <= column && column <= c1 && c1 > c0
            });
        match multi_column_selection {
            Some(selection) => {
                let (_, _, c0, c1) = selection.normalized();
                let widths = space
                    .map(|space| &space.array_browser.column_widths_rem)
                    .cloned()
                    .unwrap_or_default();
                (c0..=c1)
                    .map(|index| {
                        let captured = if index == column {
                            initial_rem
                        } else {
                            widths.get(&index).copied().unwrap_or(4.0)
                        };
                        ResizeTarget {
                            index,
                            initial_rem: captured,
                        }
                    })
                    .collect()
            }
            None => vec![ResizeTarget {
                index: column,
                initial_rem,
            }],
        }
    })
}

/// Mirror of [`resolve_column_resize_targets`] for rows.
#[cfg(target_arch = "wasm32")]
fn resolve_row_resize_targets(
    state: HostStateSignal,
    row: usize,
    initial_rem: f32,
) -> Vec<ResizeTarget> {
    state.with_untracked(|state| {
        let space = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id));
        let multi_row_selection = space
            .and_then(|space| space.array_browser.selection)
            .filter(|selection| {
                let (r0, r1, _, _) = selection.normalized();
                r0 <= row && row <= r1 && r1 > r0
            });
        match multi_row_selection {
            Some(selection) => {
                let (r0, r1, _, _) = selection.normalized();
                let heights = space
                    .map(|space| &space.array_browser.row_heights_rem)
                    .cloned()
                    .unwrap_or_default();
                (r0..=r1)
                    .map(|index| {
                        let captured = if index == row {
                            initial_rem
                        } else {
                            heights.get(&index).copied().unwrap_or(1.6)
                        };
                        ResizeTarget {
                            index,
                            initial_rem: captured,
                        }
                    })
                    .collect()
            }
            None => vec![ResizeTarget {
                index: row,
                initial_rem,
            }],
        }
    })
}

#[cfg(target_arch = "wasm32")]
fn install_resize_drag(state: HostStateSignal, axis: ResizeAxis, initial_pos: i32) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    // Approximate 1rem ≈ 16px (the host's base font-size). A
    // future polish slice can read the actual computed font-size
    // off the array-browser root so user font-size overrides
    // round-trip into the drag math.
    let rem_per_px = 1.0 / 16.0;
    // Hold a strong reference to the closures across both events
    // so they don't get GC'd before mouseup fires.
    let move_handle: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::MouseEvent)>>>> =
        Rc::new(RefCell::new(None));
    let up_handle: Rc<RefCell<Option<Closure<dyn FnMut(web_sys::MouseEvent)>>>> =
        Rc::new(RefCell::new(None));

    let move_handle_for_move = move_handle.clone();
    let move_closure = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
        let delta_px = match &axis {
            ResizeAxis::Columns(_) => ev.client_x() - initial_pos,
            ResizeAxis::Rows(_) => ev.client_y() - initial_pos,
        };
        let delta_rem = delta_px as f32 * rem_per_px;
        state.update(|state| match &axis {
            ResizeAxis::Columns(targets) => {
                for target in targets {
                    let next_rem = (target.initial_rem + delta_rem).max(0.5);
                    let _ = crate::app::reducer::set_active_array_browser_column_width(
                        state,
                        target.index,
                        next_rem,
                    );
                }
            }
            ResizeAxis::Rows(targets) => {
                for target in targets {
                    let next_rem = (target.initial_rem + delta_rem).max(0.5);
                    let _ = crate::app::reducer::set_active_array_browser_row_height(
                        state,
                        target.index,
                        next_rem,
                    );
                }
            }
        });
        // Keep ourselves alive — the borrow is just a refcount
        // bump. Without this the closure could be dropped
        // mid-drag if Rust's lifetime accounting decided so.
        let _ = move_handle_for_move.borrow();
    }) as Box<dyn FnMut(_)>);

    let document_for_up = document.clone();
    let move_closure_handle_for_up = move_handle.clone();
    let up_closure_handle_for_up = up_handle.clone();
    let up_closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
        if let Some(closure) = move_closure_handle_for_up.borrow_mut().take() {
            let _ = document_for_up
                .remove_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref());
        }
        if let Some(closure) = up_closure_handle_for_up.borrow_mut().take() {
            let _ = document_for_up
                .remove_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(_)>);

    let _ = document
        .add_event_listener_with_callback("mousemove", move_closure.as_ref().unchecked_ref());
    let _ =
        document.add_event_listener_with_callback("mouseup", up_closure.as_ref().unchecked_ref());
    *move_handle.borrow_mut() = Some(move_closure);
    *up_handle.borrow_mut() = Some(up_closure);
}

#[cfg(test)]
mod array_browser_tests {
    use super::build_clipboard_tsv;
    use crate::state::ArrayBlockSelection;

    fn cells_3x3() -> Vec<Vec<String>> {
        vec![
            vec!["1".into(), "2".into(), "3".into()],
            vec!["4".into(), "5".into(), "6".into()],
            vec!["7".into(), "8".into(), "9".into()],
        ]
    }

    #[test]
    fn clipboard_tsv_without_selection_emits_full_grid() {
        let cells = cells_3x3();
        let tsv = build_clipboard_tsv(&cells, None);
        assert_eq!(tsv, "1\t2\t3\n4\t5\t6\n7\t8\t9");
    }

    #[test]
    fn clipboard_tsv_with_block_selection_emits_subgrid() {
        let cells = cells_3x3();
        // Select rows 1..2, cols 0..1 (the 2x2 block in the
        // bottom-left): rows ["4 5", "7 8"].
        let sel = ArrayBlockSelection::from_corners(1, 0, 2, 1);
        let tsv = build_clipboard_tsv(&cells, Some(sel));
        assert_eq!(tsv, "4\t5\n7\t8");
    }

    #[test]
    fn clipboard_tsv_handles_inverted_corners() {
        // Anchor at bottom-right, focus at top-left — the
        // selection rectangle is the same.
        let cells = cells_3x3();
        let sel = ArrayBlockSelection::from_corners(2, 2, 0, 0);
        let tsv = build_clipboard_tsv(&cells, Some(sel));
        assert_eq!(tsv, "1\t2\t3\n4\t5\t6\n7\t8\t9");
    }

    #[test]
    fn clipboard_tsv_single_cell_selection_is_just_that_cell() {
        let cells = cells_3x3();
        let sel = ArrayBlockSelection::from_corners(1, 1, 1, 1);
        let tsv = build_clipboard_tsv(&cells, Some(sel));
        assert_eq!(tsv, "5");
    }

    #[test]
    fn clipboard_tsv_empty_grid_emits_empty_string() {
        let cells: Vec<Vec<String>> = Vec::new();
        let tsv = build_clipboard_tsv(&cells, None);
        assert_eq!(tsv, "");
    }
}

/// Map an icon-set kind + index to a Unicode glyph. Excel's icon
/// sets are pixel art in the .xlsx renderer; here we ship the
/// closest representative Unicode glyph per kind. Unknown kinds
/// fall back to the index as a number wrapped in a circle.
fn icon_glyph_for(set_kind: &str, icon_index: usize) -> (String, String) {
    let glyphs: &[&str] = match set_kind {
        // 3-icon sets
        "3Arrows" | "3ArrowsGray" => &["↓", "→", "↑"],
        "3TrafficLights1" | "3TrafficLights2" => &["🔴", "🟡", "🟢"],
        "3Signs" => &["⛔", "⚠", "✅"],
        "3Symbols" | "3Symbols2" => &["✗", "!", "✓"],
        "3Flags" => &["🚩", "🟨", "🟩"],
        // 4-icon sets
        "4Arrows" | "4ArrowsGray" => &["↓", "↘", "↗", "↑"],
        "4Rating" => &["▁", "▃", "▅", "▇"],
        "4RedToBlack" => &["⬛", "🟥", "🟧", "🟩"],
        "4TrafficLights" => &["🔴", "🟠", "🟡", "🟢"],
        // 5-icon sets
        "5Arrows" | "5ArrowsGray" => &["↓", "↘", "→", "↗", "↑"],
        "5Rating" => &["▁", "▂", "▄", "▆", "█"],
        "5Quarters" => &["○", "◔", "◐", "◕", "●"],
        _ => &["•"],
    };
    let glyph = glyphs.get(icon_index).copied().unwrap_or("•").to_string();
    (set_kind.to_string(), glyph)
}

/// True when a key press is a caret-only navigation key — moves
/// the caret without changing the text. The textarea's
/// `on:keyup` filters on these so caret-only navigation triggers
/// a popup-refresh round-trip, while text-input keys fall through
/// to the existing `on:input` path (which already fires the
/// bridge with up-to-date selection).
fn is_caret_navigation_key(key: &str) -> bool {
    matches!(
        key,
        "ArrowLeft"
            | "ArrowRight"
            | "ArrowUp"
            | "ArrowDown"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
    )
}

/// Build a synthetic `EditorInputEvent` from the textarea's current
/// value + selection, used to push a "caret moved, but text didn't
/// change" signal through `apply_live_editor_input`. The reducer
/// updates the editor surface's selection; the bridge refresh
/// re-evaluates signature-help / completion-popup / function-help
/// against the new caret position.
fn synthesize_caret_sync_event(textarea: &HtmlTextAreaElement) -> EditorInputEvent {
    EditorInputEvent {
        text: textarea.value(),
        selection_start: textarea
            .selection_start()
            .ok()
            .flatten()
            .map(|offset| offset as usize),
        selection_end: textarea
            .selection_end()
            .ok()
            .flatten()
            .map(|offset| offset as usize),
        // Caret-sync — the bridge will skip the runtime-evaluation
        // pass. Popups still refresh.
        input_kind: EditorInputKind::CaretSync,
        inserted_text: None,
    }
}

/// Convert a 0-based column index into an Excel-style A1 column
/// label. 0 → "A", 25 → "Z", 26 → "AA", etc. Used by the array
/// browser's column header row so users can read addresses against
/// a familiar mental model.
fn column_index_to_a1_label(index: usize) -> String {
    let mut n = index;
    let mut buf = Vec::new();
    loop {
        let rem = n % 26;
        buf.push(b'A' + rem as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap_or_else(|_| index.to_string())
}

/// Render the syntax-coloured overlay that sits behind the textarea.
///
/// When `runs` is empty (no editor document, or document is a stale
/// snapshot from a prior keystroke), fall back to rendering the raw
/// textarea text uncoloured so the overlay stays character-aligned with
/// the textarea contents and the user never sees coloured tokens at the
/// wrong offset. The trailing newline preserves the textarea's last line
/// height in the overlay box (`white-space: pre-wrap` swallows it
/// otherwise).
fn render_syntax_overlay(
    runs: Vec<SyntaxRun>,
    fallback_text: String,
    bracket_pair: Option<crate::ui::editor::bracket_matcher::BracketPairHighlight>,
) -> AnyView {
    if runs.is_empty() {
        return view! {
            <span class="syn-text">{fallback_text}{"\n"}</span>
        }
        .into_any();
    }
    // Walk the runs once, computing bracket depth + active flag for
    // every delimiter run that is one of `()[]{}`. Depth wraps modulo
    // the rotating-colour palette in CSS — depth 0 → teal, 1 → rust,
    // 2 → amber, 3 → sage, then rotates. The matching pair under the
    // cursor (open + close) is tagged with `data-bracket-active="true"`
    // and bolded by CSS. Non-bracket delimiters (commas, dots, the
    // leading `=`) are passed through untouched.
    let mut current_depth: usize = 0;
    let spans: Vec<AnyView> = runs
        .into_iter()
        .map(|run| {
            let role_slug = role_slug(run.role);
            let span_start = run.span_start;
            let token_text = run.text.clone();
            let is_bracket = is_bracket_token(&run.text);
            let bracket_depth_attr = if is_bracket {
                let depth_at = if is_open_bracket_text(&run.text) {
                    let d = current_depth;
                    current_depth = current_depth.saturating_add(1);
                    d
                } else {
                    current_depth = current_depth.saturating_sub(1);
                    current_depth
                };
                Some(depth_at)
            } else {
                None
            };
            let bracket_active = match (bracket_depth_attr, &bracket_pair) {
                (Some(_), Some(pair)) => {
                    span_start == pair.open_offset || span_start == pair.close_offset
                }
                _ => false,
            };
            let mut class = format!("syn {}", role_class(run.role));
            if let Some(depth) = bracket_depth_attr {
                let depth_class = depth % BRACKET_DEPTH_COLOR_COUNT;
                class.push_str(&format!(" syn-bracket syn-bracket--depth-{}", depth_class));
                if bracket_active {
                    class.push_str(" syn-bracket--active");
                }
            }
            let depth_attr_value = bracket_depth_attr
                .map(|d| d.to_string())
                .unwrap_or_default();
            let active_attr_value = if bracket_active { "true" } else { "false" };
            view! {
                <span
                    class=class
                    data-token-start=span_start.to_string()
                    data-token-text=token_text.clone()
                    data-token-role=role_slug
                    data-bracket-depth=depth_attr_value
                    data-bracket-active=active_attr_value
                >
                    {run.text}
                </span>
            }
            .into_any()
        })
        .collect();
    view! {
        <>
            {spans}
            {"\n"}
        </>
    }
    .into_any()
}

/// Number of distinct rotating colours offered by the bracket-depth
/// CSS rules. Walking deeper than this wraps back to depth 0 — chosen
/// so the visual signal stays useful even in pathological 10-level
/// nests; four colours is the rainbow-bracket sweet spot the eye can
/// track without the palette feeling noisy.
const BRACKET_DEPTH_COLOR_COUNT: usize = 4;

fn is_bracket_token(text: &str) -> bool {
    matches!(text, "(" | ")" | "[" | "]" | "{" | "}")
}

fn is_open_bracket_text(text: &str) -> bool {
    matches!(text, "(" | "[" | "{")
}

/// Render the diagnostic-squiggle overlay. Splits `text` into alternating
/// non-squiggled / squiggled segments by character offset, so that the
/// wavy underline lines up with the textarea characters at each
/// `LiveDiagnostic.primary_span`. Squiggled segments are also given a
/// `title` attribute so a browser-native hover-tooltip carries the
/// `diagnostic_id: message` summary without any JS popover work.
///
/// Both layers (this one and the syntax overlay) render the same text;
/// CSS makes only the wavy underlines visible by setting the text colour
/// transparent here.
fn render_diagnostic_squiggle_overlay(squiggles: Vec<DiagnosticSquiggle>, text: String) -> AnyView {
    if squiggles.is_empty() {
        // No diagnostics: render the raw text invisibly so the squiggle
        // box keeps the same height as the textarea (whitespace-pre-wrap
        // collapses zero-content boxes otherwise).
        return view! { <span>{text}{"\n"}</span> }.into_any();
    }

    // Walk the text in character offsets, building segments. The
    // squiggle list is already sorted-and-deduped by the projector.
    let chars: Vec<char> = text.chars().collect();
    let mut segments: Vec<AnyView> = Vec::new();
    let mut cursor: usize = 0;
    for squiggle in squiggles {
        let span_start = squiggle.span_start.min(chars.len());
        let span_end = span_start
            .saturating_add(squiggle.span_len)
            .min(chars.len());
        if span_start > cursor {
            let segment: String = chars[cursor..span_start].iter().collect();
            segments.push(view! { <span>{segment}</span> }.into_any());
        }
        if span_end > span_start {
            let segment: String = chars[span_start..span_end].iter().collect();
            let class = format!("squiggle squiggle--{}", squiggle.severity.slug());
            let title = format!("{}: {}", squiggle.diagnostic_id, squiggle.message);
            // OxFml W067: surface `code`, `stage`, and
            // `worksheet_error_class` as data attributes so browser
            // tests and the eventual UI grouping surface can read
            // them without inference.
            let code_attr = squiggle.code.clone().unwrap_or_default();
            let worksheet_error_class_attr =
                squiggle.worksheet_error_class.clone().unwrap_or_default();
            segments.push(
                view! {
                    <span
                        class=class
                        data-diagnostic-id=squiggle.diagnostic_id
                        data-severity=squiggle.severity.slug()
                        data-stage=squiggle.stage.slug()
                        data-code=code_attr
                        data-worksheet-error-class=worksheet_error_class_attr
                        data-span-start=squiggle.span_start.to_string()
                        data-span-len=squiggle.span_len.to_string()
                        title=title
                    >
                        {segment}
                    </span>
                }
                .into_any(),
            );
            cursor = span_end;
        } else if cursor < span_start {
            cursor = span_start;
        }
    }
    if cursor < chars.len() {
        let trailing: String = chars[cursor..].iter().collect();
        segments.push(view! { <span>{trailing}</span> }.into_any());
    }
    view! {
        <>
            {segments}
            {"\n"}
        </>
    }
    .into_any()
}

/// Splice `insert_text` into `raw_text` at `replacement_span`.
/// Splits / joins on Rust `char` boundaries so non-ASCII inputs do not
/// corrupt. When `replacement_span` is `None`, the insertion is
/// appended at the end (matches the popup-state model's "no anchor"
/// behaviour for proposals without a replacement context).
fn splice_textarea_value(
    raw_text: &str,
    replacement_span: Option<FormulaTextSpan>,
    insert_text: &str,
) -> String {
    let chars: Vec<char> = raw_text.chars().collect();
    let (start, end) = match replacement_span {
        Some(span) => {
            let start = span.start.min(chars.len());
            let end = start.saturating_add(span.len).min(chars.len());
            (start, end)
        }
        None => {
            let end = chars.len();
            (end, end)
        }
    };
    let mut out: String = chars[..start].iter().collect();
    out.push_str(insert_text);
    let trailing: String = chars[end..].iter().collect();
    out.push_str(&trailing);
    out
}

/// Render the completion popup. Returns an empty fragment when the
/// view-model has `None` (popup hidden or not yet measurable).
/// Positioned absolutely within the editor frame at the caret anchor;
/// the popup wrapper is `pointer-events: none` so background clicks
/// fall through to the textarea, while each item row reactivates
/// `pointer-events: auto` for click handling.
/// Editor-foot button that re-runs the active formula through the
/// bridge. Mirrors the F9 keystroke. In Deterministic policy this
/// produces an identical re-render; in LiveRecalc it advances NOW
/// and re-rolls RAND.
///
/// `on:mousedown` (rather than `on:click`) so a click does not pull
/// focus away from the textarea — caret stays where it was.
fn render_recalculate_button(on_recalculate: Callback<()>) -> AnyView {
    view! {
        <button
            type="button"
            class="onecalc-home-shell__recalculate-button"
            data-action="recalculate"
            title="Recalculate formula (F9)"
            aria-label="Recalculate formula"
            on:mousedown=move |ev| {
                ev.prevent_default();
                on_recalculate.run(());
            }
        >
            "↻ Calculate"
        </button>
    }
    .into_any()
}

/// Render the editor-foot trigger row for the formula drill-down.
/// Always visible alongside the live-metrics chip; aria-expanded
/// follows the panel's expansion state.
fn render_formula_drill_toggle(
    drill: Option<FormulaDrillView>,
    on_toggle: Callback<()>,
) -> AnyView {
    let Some(drill) = drill else {
        return view! { <span></span> }.into_any();
    };
    let aria_expanded = if drill.expanded { "true" } else { "false" };
    let label = if drill.expanded {
        "▾ hide formula drill-down"
    } else {
        "▸ show formula drill-down"
    };
    let row_count = drill.tree.len();
    view! {
        <button
            type="button"
            class="onecalc-home-shell__formula-drill-toggle"
            data-expanded=aria_expanded
            data-row-count=row_count.to_string()
            aria-expanded=aria_expanded
            aria-controls="onecalc-formula-drill-panel"
            on:click=move |_| on_toggle.run(())
        >
            {label}
        </button>
    }
    .into_any()
}

/// Render the formula drill-down panel itself. Always emits the
/// outer panel div (so the corpus can read `data-expanded`); the
/// body content is gated by the `expanded` flag and the rows /
/// phase-strip rendering branches on `view_mode`.
fn render_formula_drill_panel(
    drill: Option<FormulaDrillView>,
    capability_context: Option<CapabilityContextView>,
    view_mode: ViewMode,
    on_view_mode_toggle: Callback<()>,
) -> AnyView {
    let Some(drill) = drill else {
        return view! { <span></span> }.into_any();
    };
    let aria_hidden = if drill.expanded { "false" } else { "true" };
    let expanded_attr = if drill.expanded { "true" } else { "false" };
    let fresh_attr = if drill.document_is_fresh {
        "true"
    } else {
        "false"
    };
    let mode_attr = view_mode.slug();
    let row_count = drill.tree.len();
    let body = if !drill.expanded {
        view! { <span></span> }.into_any()
    } else if !drill.document_is_fresh {
        view! {
            <div class="onecalc-home-shell__formula-drill-loading" role="status">
                "(loading…)"
            </div>
        }
        .into_any()
    } else {
        let nodes_view: Vec<AnyView> = drill
            .tree
            .iter()
            .map(|node| render_formula_drill_row(node.clone(), view_mode, 0))
            .collect();
        let diagnostics_view = render_formula_drill_diagnostics(&drill.diagnostics, view_mode);
        let phase_strip = match view_mode {
            ViewMode::Developer => {
                render_formula_drill_phase_strip_developer(&drill.phase_summaries)
            }
            ViewMode::User => render_formula_drill_phase_strip_user(&drill.phase_summaries),
        };
        let capability_context_view =
            render_capability_context_panel(capability_context, view_mode);
        let view_toggle = render_drill_view_mode_toggle(view_mode, on_view_mode_toggle);
        view! {
            {view_toggle}
            {diagnostics_view}
            <div
                class="onecalc-home-shell__formula-drill-tree"
                role="tree"
                aria-label="formula walk tree"
                data-mode=mode_attr
            >
                {nodes_view}
            </div>
            {phase_strip}
            {capability_context_view}
        }
        .into_any()
    };
    view! {
        <div
            id="onecalc-formula-drill-panel"
            class="onecalc-home-shell__formula-drill-panel"
            data-expanded=expanded_attr
            data-document-fresh=fresh_attr
            data-row-count=row_count.to_string()
            data-mode=mode_attr
            aria-hidden=aria_hidden
            tabindex="-1"
        >
            {body}
        </div>
    }
    .into_any()
}

fn render_capability_context_panel(
    capability_context: Option<CapabilityContextView>,
    view_mode: ViewMode,
) -> AnyView {
    if view_mode != ViewMode::Developer {
        return view! { <span></span> }.into_any();
    }
    let Some(context) = capability_context else {
        return view! { <span></span> }.into_any();
    };

    let profile_rows: Vec<AnyView> = context
        .function_profiles
        .iter()
        .map(|row| {
            let policies = match view_mode {
                ViewMode::Developer => format!(
                    "{}{}{}",
                    row.numerical_reduction_policy
                        .as_deref()
                        .unwrap_or("no reduction policy"),
                    if row.error_algebra.is_some() { " · " } else { "" },
                    row.error_algebra.as_deref().unwrap_or("")
                ),
                ViewMode::User => {
                    if row.reduction_sensitive || row.error_collapse_sensitive {
                        "semantic profile active".to_string()
                    } else {
                        "ordinary function profile".to_string()
                    }
                }
            };
            let version = match view_mode {
                ViewMode::Developer => row.semantic_kernel_metadata_version.clone(),
                ViewMode::User => row.function_id.clone(),
            };
            view! {
                <li class="onecalc-home-shell__capability-row" data-function=row.surface_name.clone()>
                    <span class="onecalc-home-shell__capability-name">{row.surface_name.clone()}</span>
                    <span class="onecalc-home-shell__capability-detail">{policies}</span>
                    <span class="onecalc-home-shell__capability-version">{version}</span>
                </li>
            }
            .into_any()
        })
        .collect();

    let value_rows: Vec<AnyView> = context
        .value_capability_facts
        .iter()
        .map(|fact| {
            let kind = match fact.fact_kind {
                ValueCapabilityFactKind::ProducerCanProvide => "producer",
                ValueCapabilityFactKind::ExercisedThisRun => "exercised",
            };
            view! {
                <li class="onecalc-home-shell__capability-row" data-capability-kind=kind>
                    <span class="onecalc-home-shell__capability-name">{kind}</span>
                    <span class="onecalc-home-shell__capability-detail">{fact.key.clone()}</span>
                </li>
            }
            .into_any()
        })
        .collect();

    let input_rows: Vec<AnyView> = context
        .formula_inputs
        .iter()
        .map(|input| {
            view! {
                <li class="onecalc-home-shell__capability-row" data-input=input.label.clone()>
                    <span class="onecalc-home-shell__capability-name">{input.label.clone()}</span>
                    <span class="onecalc-home-shell__capability-detail">{input.reference_descriptor.clone()}</span>
                    <span class="onecalc-home-shell__capability-version">{input.value_preview.clone()}</span>
                </li>
            }
            .into_any()
        })
        .collect();

    let mode_attr = view_mode.slug();
    let snapshot_id = context.snapshot.capability_snapshot_id.clone();
    let oxfunc_version_count = context
        .snapshot
        .oxfunc_metadata
        .semantic_kernel_metadata_versions
        .len();
    view! {
        <section
            class="onecalc-home-shell__capability-context"
            data-view-mode=mode_attr
            data-snapshot-id=snapshot_id.clone()
            aria-label="Capability context"
        >
            <div class="onecalc-home-shell__capability-heading">
                <span>"capability context"</span>
                <span class="onecalc-home-shell__capability-summary">
                    {format!("{oxfunc_version_count} OxFunc metadata version set(s)")}
                </span>
            </div>
            <ul class="onecalc-home-shell__capability-list" data-section="functions">
                {profile_rows}
            </ul>
            <ul class="onecalc-home-shell__capability-list" data-section="value-capabilities">
                {value_rows}
            </ul>
            <ul class="onecalc-home-shell__capability-list" data-section="formula-inputs">
                {input_rows}
            </ul>
        </section>
    }
    .into_any()
}

/// Render one node of the formula drill-down. Uses `<details>`
/// for nodes with children — the user clicks the chevron to
/// collapse / expand each subtree (browser-native).
/// Children are rendered nested inside the details body so the
/// visual hierarchy mirrors the formula's call structure.
fn render_formula_drill_row(node: FormulaDrillNode, view_mode: ViewMode, depth: usize) -> AnyView {
    let has_children = !node.children.is_empty();
    let has_children_attr = if has_children { "true" } else { "false" };
    let state_slug = formula_drill_state_slug(node.state);
    let value_preview_full = node.value_preview.clone();
    let value_preview = value_preview_full.clone().unwrap_or_default();
    let array_preview = render_formula_drill_array_preview(node.array_preview.clone());
    let aria_level = (depth + 1).to_string();
    let mode_attr = view_mode.slug();
    let span_start_attr = node
        .source_span_start
        .map(|span| span.to_string())
        .unwrap_or_default();
    let span_len_attr = node
        .source_span_len
        .map(|span| span.to_string())
        .unwrap_or_default();
    let branch_attr = node.branch_disposition.clone().unwrap_or_default();
    let kind_attr = node.kind.clone().unwrap_or_default();
    let expression_title = node.expression_text.clone().unwrap_or_default();
    let row_inner = match view_mode {
        ViewMode::Developer => {
            let developer_label = node
                .developer_label
                .clone()
                .unwrap_or_else(|| node.label.clone());
            let value_view = if label_includes_value(&developer_label) {
                view! { <></> }.into_any()
            } else {
                view! {
                    <span
                        class="onecalc-home-shell__formula-drill-value"
                        title=value_preview.clone()
                    >
                        {truncate_for_drill(value_preview.clone())}
                    </span>
                }
                .into_any()
            };
            view! {
                <>
                    <span
                        class="onecalc-home-shell__formula-drill-state"
                        aria-label=state_slug
                        data-state=state_slug
                    >
                        {formula_drill_state_label(node.state)}
                    </span>
                    <span
                        class="onecalc-home-shell__formula-drill-label"
                        title=expression_title.clone()
                    >
                        {developer_label}
                    </span>
                    {node.branch_disposition.clone().map(|branch| view! {
                        <span
                            class="onecalc-home-shell__formula-drill-branch"
                            data-branch=branch.clone()
                        >
                            {branch.clone()}
                        </span>
                    })}
                    {node.argument_role.clone().map(|role| view! {
                        <span
                            class="onecalc-home-shell__formula-drill-role"
                            data-role=role.clone()
                        >
                            {role.clone()}
                        </span>
                    })}
                    {value_view}
                </>
            }
            .into_any()
        }
        ViewMode::User => render_formula_drill_row_user_mode(
            node.label.clone(),
            node.state,
            node.branch_disposition.clone(),
            node.error_message.clone(),
            value_preview_full,
        ),
    };
    if has_children {
        let children_view: Vec<AnyView> = node
            .children
            .into_iter()
            .map(|child| render_formula_drill_row(child, view_mode, depth + 1))
            .collect();
        view! {
            <details
                class="onecalc-home-shell__formula-drill-row onecalc-home-shell__formula-drill-row--branch"
                data-depth=depth.to_string()
                data-has-children=has_children_attr
                data-state=state_slug
                data-node-id=node.node_id
                data-aria-level=aria_level
                data-mode=mode_attr
                data-kind=kind_attr
                data-span-start=span_start_attr
                data-span-len=span_len_attr
                data-branch=branch_attr
                open
            >
                <summary class="onecalc-home-shell__formula-drill-row-summary">
                    {row_inner}
                </summary>
                {array_preview}
                <div class="onecalc-home-shell__formula-drill-row-children">
                    {children_view}
                </div>
            </details>
        }
        .into_any()
    } else {
        view! {
            <div
                class="onecalc-home-shell__formula-drill-row onecalc-home-shell__formula-drill-row--leaf"
                role="treeitem"
                data-depth=depth.to_string()
                data-has-children=has_children_attr
                data-state=state_slug
                data-node-id=node.node_id
                data-aria-level=aria_level
                data-mode=mode_attr
                data-kind=kind_attr
                data-span-start=span_start_attr
                data-span-len=span_len_attr
                data-branch=branch_attr
            >
                {row_inner}
                {array_preview}
            </div>
        }
        .into_any()
    }
}

fn render_formula_drill_array_preview(
    preview: Option<crate::adapters::oxfml::FormulaDrillArrayPreview>,
) -> AnyView {
    let Some(preview) = preview else {
        return view! { <></> }.into_any();
    };
    if preview.rows.is_empty() {
        return view! { <></> }.into_any();
    }
    let preview_rows = preview.rows.len();
    let preview_cols = preview.rows.iter().map(|row| row.len()).max().unwrap_or(0);
    if preview_cols == 0 {
        return view! { <></> }.into_any();
    }
    let hidden_rows = preview.total_rows.saturating_sub(preview_rows);
    let hidden_cols = preview.total_cols.saturating_sub(preview_cols);
    let hidden = if preview.truncated {
        let mut bits = Vec::new();
        if hidden_rows > 0 {
            bits.push(format!("+{hidden_rows} rows"));
        }
        if hidden_cols > 0 {
            bits.push(format!("+{hidden_cols} cols"));
        }
        if bits.is_empty() {
            Some("more cells".to_string())
        } else {
            Some(bits.join(" · "))
        }
    } else {
        None
    };
    let grid_style = format!(
        "grid-template-columns: repeat({}, minmax(2.75rem, max-content));",
        preview_cols.max(1)
    );
    let cells: Vec<AnyView> = preview
        .rows
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            (0..preview_cols).map(move |col_index| {
                let value = row.get(col_index).cloned().unwrap_or_default();
                view! {
                    <span
                        class="onecalc-home-shell__formula-drill-array-cell"
                        data-row=row_index.to_string()
                        data-col=col_index.to_string()
                        title=value.clone()
                    >
                        {value.clone()}
                    </span>
                }
                .into_any()
            })
        })
        .collect();
    view! {
        <details
            class="onecalc-home-shell__formula-drill-array"
            data-total-rows=preview.total_rows.to_string()
            data-total-cols=preview.total_cols.to_string()
            data-preview-rows=preview_rows.to_string()
            data-preview-cols=preview_cols.to_string()
            data-truncated=if preview.truncated { "true" } else { "false" }
        >
            <summary class="onecalc-home-shell__formula-drill-array-summary">
                {format!("Array[{} × {}] preview", preview.total_rows, preview.total_cols)}
                {hidden.map(|hidden| view! {
                    <span class="onecalc-home-shell__formula-drill-array-hidden">
                        {hidden}
                    </span>
                })}
            </summary>
            <div class="onecalc-home-shell__formula-drill-array-grid" style=grid_style>
                {cells}
            </div>
        </details>
    }
    .into_any()
}

/// Render the view-mode toggle inside the drill-down panel
/// header. The drill-down is the only surface that meaningfully
/// branches on view mode (User mode hides phase chips, state
/// slugs, and SEAM markers; Developer mode surfaces them all),
/// so the toggle lives where it has effect — top-right of the
/// panel body, small and quiet.
fn render_drill_view_mode_toggle(mode: ViewMode, on_toggle: Callback<()>) -> AnyView {
    let mode_attr = mode.slug();
    let label = match mode {
        ViewMode::User => "▸ developer view",
        ViewMode::Developer => "▾ developer view",
    };
    let pressed = matches!(mode, ViewMode::Developer);
    view! {
        <div class="onecalc-home-shell__formula-drill-mode-toggle">
            <button
                type="button"
                class="onecalc-home-shell__formula-drill-mode-button"
                data-view-mode=mode_attr
                aria-pressed=if pressed { "true" } else { "false" }
                title="Toggle Developer view (shows state chips, phase strip, SEAM markers)"
                on:mousedown=move |ev| {
                    ev.prevent_default();
                    on_toggle.run(());
                }
            >
                {label}
            </button>
        </div>
    }
    .into_any()
}

/// Render the diagnostics list inside the drill-down panel.
/// Empty when there are no diagnostics; otherwise emits one row
/// per diagnostic with its severity, message, and (in Developer
/// mode) the diagnostic id / stage / span. Click a row to
/// (eventually) scroll the editor to the span — for now the row
/// is read-only but the `data-span-start` / `data-span-len`
/// attributes are emitted so the click handler is a single-line
/// follow-up when it lands.
fn render_formula_drill_diagnostics(
    diagnostics: &[FormulaDrillDiagnosticRow],
    view_mode: ViewMode,
) -> AnyView {
    if diagnostics.is_empty() {
        return view! { <></> }.into_any();
    }
    let mode_attr = view_mode.slug();
    let rows: Vec<AnyView> = diagnostics
        .iter()
        .map(|diag| {
            let severity_slug = diag.severity.slug();
            let detail = match view_mode {
                ViewMode::Developer => {
                    let code = diag.code.clone().unwrap_or_default();
                    format!(
                        "[{stage}] {code}{sep}{msg}",
                        stage = diag.stage.slug(),
                        code = code,
                        sep = if code.is_empty() { "" } else { " · " },
                        msg = diag.message,
                    )
                }
                ViewMode::User => diag.message.clone(),
            };
            view! {
                <li
                    class="onecalc-home-shell__formula-drill-diagnostic"
                    data-severity=severity_slug
                    data-stage=diag.stage.slug()
                    data-span-start=diag.span_start.to_string()
                    data-span-len=diag.span_len.to_string()
                    data-mode=mode_attr
                >
                    <span
                        class="onecalc-home-shell__formula-drill-diagnostic-severity"
                        data-severity=severity_slug
                    >
                        {severity_slug}
                    </span>
                    <span class="onecalc-home-shell__formula-drill-diagnostic-message">
                        {detail}
                    </span>
                </li>
            }
            .into_any()
        })
        .collect();
    view! {
        <ul
            class="onecalc-home-shell__formula-drill-diagnostics"
            role="list"
            aria-label="formula diagnostics"
            data-mode=mode_attr
        >
            {rows}
        </ul>
    }
    .into_any()
}

/// User-mode row layout: `label = value` (or `label · blocked
/// <reason>` for blocked rows). The state chip is dropped; the
/// only non-text element is a tiny inline tag for blocked rows
/// because that is the one row state an Excel user genuinely
/// needs to notice.
fn render_formula_drill_row_user_mode(
    label: String,
    state: crate::adapters::oxfml::FormulaDrillNodeState,
    branch_disposition: Option<String>,
    error_message: Option<String>,
    value_preview: Option<String>,
) -> AnyView {
    use crate::adapters::oxfml::FormulaDrillNodeState as State;
    let label_view = view! {
        <span class="onecalc-home-shell__formula-drill-label">{label.clone()}</span>
    };
    match state {
        State::Blocked | State::Error => {
            let value_text = value_preview.clone().unwrap_or_default();
            let truncated = truncate_for_drill(value_text.clone());
            let tag = if state == State::Error {
                "error"
            } else {
                "blocked"
            };
            let title = error_message.unwrap_or(value_text.clone());
            let value_view = if label_includes_value(&label) {
                view! { <></> }.into_any()
            } else {
                view! {
                    <span
                        class="onecalc-home-shell__formula-drill-value"
                        title=title
                    >
                        {truncated}
                    </span>
                }
                .into_any()
            };
            view! {
                <>
                    {label_view}
                    <span class="onecalc-home-shell__formula-drill-blocked-tag">{tag}</span>
                    {value_view}
                </>
            }
            .into_any()
        }
        State::Skipped => view! {
            <>
                {label_view}
                {(!label.to_ascii_lowercase().contains("skipped")).then(|| view! {
                    <span
                        class="onecalc-home-shell__formula-drill-branch"
                        data-branch=branch_disposition.clone().unwrap_or_else(|| "Skipped".to_string())
                    >
                        "skipped"
                    </span>
                })}
            </>
        }
        .into_any(),
        _ => {
            if label_includes_value(&label) {
                return view! {
                    <>
                        {label_view}
                    </>
                }
                .into_any();
            }
            let (value_text, value_title) = match value_preview {
                Some(v) => (truncate_for_drill(v.clone()), v),
                None => ("…".to_string(), String::new()),
            };
            view! {
                <>
                    {label_view}
                    <span class="onecalc-home-shell__formula-drill-equals" aria-hidden="true">"="</span>
                    <span
                        class="onecalc-home-shell__formula-drill-value"
                        title=value_title
                    >
                        {value_text}
                    </span>
                </>
            }
            .into_any()
        }
    }
}

fn label_includes_value(label: &str) -> bool {
    label.contains(" = ")
}

/// Developer-mode phase strip: parse / bind / eval chips, one per phase.
fn render_formula_drill_phase_strip_developer(chips: &[FormulaDrillPhaseChip]) -> AnyView {
    let phase_view: Vec<AnyView> = chips
        .iter()
        .map(|chip| render_formula_drill_phase_chip(chip.clone()))
        .collect();
    view! {
        <div
            class="onecalc-home-shell__formula-drill-phase-strip"
            data-mode="developer"
        >
            {phase_view}
        </div>
    }
    .into_any()
}

/// User-mode phase strip: a single status line. Reads as
/// "evaluated in <duration>" when clean, or "blocked: <reason>"
/// when any phase is blocked. The eval-phase chip's detail
/// carries the duration_text in the form "<n> step(s) · <ms>"
/// so we extract the duration suffix; if the format changes we
/// fall back to the raw detail.
fn render_formula_drill_phase_strip_user(chips: &[FormulaDrillPhaseChip]) -> AnyView {
    if chips.is_empty() {
        return view! { <span></span> }.into_any();
    }
    let any_blocked = chips
        .iter()
        .any(|c| c.state == FormulaDrillPhaseState::Blocked);
    let status_class = if any_blocked {
        "onecalc-home-shell__formula-drill-status onecalc-home-shell__formula-drill-status--blocked"
    } else {
        "onecalc-home-shell__formula-drill-status onecalc-home-shell__formula-drill-status--ok"
    };
    let summary = if any_blocked {
        chips
            .iter()
            .find(|c| c.state == FormulaDrillPhaseState::Blocked)
            .map(|c| format!("blocked at {}: {}", c.label, c.detail))
            .unwrap_or_else(|| "blocked".to_string())
    } else {
        chips
            .iter()
            .find(|c| c.label == "eval")
            .map(|c| {
                // eval detail is "<n> step(s) · <duration_text>"
                // — pull the segment after the last " · ". If
                // unavailable, fall back to the whole detail.
                let last_segment = c
                    .detail
                    .rsplit(" · ")
                    .next()
                    .unwrap_or_else(|| c.detail.as_str());
                format!("evaluated in {last_segment}")
            })
            .unwrap_or_else(|| "evaluated".to_string())
    };
    let status_state = if any_blocked { "blocked" } else { "ok" };
    view! {
        <div
            class="onecalc-home-shell__formula-drill-phase-strip"
            data-mode="user"
        >
            <span class=status_class data-status=status_state>{summary}</span>
        </div>
    }
    .into_any()
}

fn render_formula_drill_phase_chip(chip: FormulaDrillPhaseChip) -> AnyView {
    let state_slug = chip.state.slug();
    let label = chip.label;
    view! {
        <span
            class="onecalc-home-shell__formula-drill-phase"
            data-phase=label
            data-state=state_slug
        >
            <strong>{label}</strong>
            ": "
            {chip.detail}
        </span>
    }
    .into_any()
}

fn formula_drill_state_slug(state: crate::adapters::oxfml::FormulaDrillNodeState) -> &'static str {
    use crate::adapters::oxfml::FormulaDrillNodeState as State;
    match state {
        State::Pending => "pending",
        State::Evaluated => "evaluated",
        State::Bound => "bound",
        State::Skipped => "skipped",
        State::Opaque => "opaque",
        State::Blocked => "blocked",
        State::Error => "error",
    }
}

fn formula_drill_state_label(state: crate::adapters::oxfml::FormulaDrillNodeState) -> &'static str {
    use crate::adapters::oxfml::FormulaDrillNodeState as State;
    match state {
        State::Pending => "pending",
        State::Evaluated => "evaluated",
        State::Bound => "bound",
        State::Skipped => "skipped",
        State::Opaque => "opaque",
        State::Blocked => "blocked",
        State::Error => "error",
    }
}

fn truncate_for_drill(value: String) -> String {
    let limit = 32;
    if value.chars().count() <= limit {
        value
    } else {
        let mut out: String = value.chars().take(limit).collect();
        out.push('…');
        out
    }
}

/// Hover-state for the function-help tooltip. Component-local
/// (not in the reducer state) because hover is purely a UI
/// concern. Set by the editor-frame `on:mouseover` handler when
/// the pointer enters a `.syn-fn` span whose `data-token-text`
/// matches the bridge's `function_help.lookup_key`. Cleared by
/// the frame's `on:mouseleave` and by an Effect that watches
/// `raw_entered_cell_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionHelpHoverTarget {
    token_text: String,
    anchor_left_px: usize,
    anchor_top_px: usize,
    line_height_px: usize,
}

/// Render the function-help tooltip. Returns an empty span when
/// either the hover state or the function-help card is missing,
/// so visibility is reactive on both signals at once. The tooltip
/// is positioned BELOW the hovered token (anchor_top + line_height
/// + small gap) — different from the signature help which sits
/// above the caret. Layout-wise it lives in the same editor-frame
/// container as the popup and signature help, so it stays inside
/// the editor's coordinate system.
///
/// Wrapper is `pointer-events: none` so the user can move the
/// mouse off the function token without the tooltip itself
/// stealing the hover.
fn render_function_help_card(
    hover: Option<FunctionHelpHoverTarget>,
    card: Option<FunctionHelpCardView>,
) -> AnyView {
    let (Some(hover), Some(card)) = (hover, card) else {
        return view! { <span></span> }.into_any();
    };
    if !card.lookup_key.eq_ignore_ascii_case(&hover.token_text) {
        return view! { <span></span> }.into_any();
    }
    let style = format!(
        "left: {}px; top: {}px;",
        hover.anchor_left_px,
        hover.anchor_top_px.saturating_add(hover.line_height_px),
    );
    let availability = card.availability_summary.clone().unwrap_or_default();
    let signature_view = card
        .signature
        .clone()
        .map(|sig| view! { <div class="onecalc-function-help__signature">{sig}</div> }.into_any())
        .unwrap_or_else(|| view! { <span></span> }.into_any());
    let description_view = card
        .short_description
        .clone()
        .map(|desc| {
            view! { <div class="onecalc-function-help__description">{desc}</div> }.into_any()
        })
        .unwrap_or_else(|| view! { <span></span> }.into_any());
    let availability_view = if !availability.is_empty() {
        view! {
            <div class="onecalc-function-help__availability">{availability}</div>
        }
        .into_any()
    } else {
        view! { <span></span> }.into_any()
    };
    let deferred_attr = if card.deferred_or_profile_limited {
        "true"
    } else {
        "false"
    };
    view! {
        <div
            class="onecalc-function-help"
            role="tooltip"
            data-lookup-key=card.lookup_key.clone()
            data-deferred=deferred_attr
            style=style
        >
            <div class="onecalc-function-help__heading">{card.display_name.clone()}</div>
            {signature_view}
            {description_view}
            {availability_view}
        </div>
    }
    .into_any()
}

/// Render the signature-help line ABOVE the caret.
///
/// The view-model emits `None` whenever the help should be hidden
/// (no call in progress, document stale, metrics unmeasured, popup
/// open at the same caret). This function only positions the help
/// and renders the parameter list with the active parameter
/// bolded; all suppression is the projector's responsibility.
///
/// Anchor strategy: the projector hands us the caret-box top-left in
/// pixels; we offset upward by `signature_help_height + gap`. The
/// help line is `max-height: 28px` so it's narrow enough not to
/// fight the syntax overlay or the squiggle layer for stacking
/// space. Below-the-caret fallback (when the line would clip the
/// frame top) is handled with CSS `transform: translateY(...)` —
/// see the theme's `.onecalc-signature-help--flipped` rule.
///
/// Wrapper is `pointer-events: none` so background clicks fall
/// through to the textarea (the help is non-interactive).
fn render_signature_help(help: Option<SignatureHelpView>) -> AnyView {
    let Some(help) = help else {
        return view! { <span></span> }.into_any();
    };
    // Anchor at the caret-line TOP (in editor-frame coordinates,
    // i.e. metric-space y plus the textarea padding). The CSS
    // transform `translateY(-100% - 6px)` then lifts the help
    // tooltip's bottom edge 6 px above that line, putting it
    // immediately over the line without the actual rendered
    // height needing to be guessed in pixels. Without the
    // padding offset the top would be 0 → the help renders
    // ABOVE the editor frame, far from the caret (the bug the
    // user reported as "placed very high").
    let style = format!(
        "left: {}px; top: {}px;",
        help.anchor_left_px.saturating_add(EDITOR_FRAME_PAD_PX),
        help.anchor_top_px.saturating_add(EDITOR_FRAME_PAD_PX),
    );
    let parameter_count = help.parameters.len();
    let active_index_attr = help
        .active_parameter
        .map(|i| i.to_string())
        .unwrap_or_else(|| "-1".to_string());
    let parameters = help
        .parameters
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            let is_last = index + 1 == parameter_count;
            let separator = if is_last { "" } else { ", " };
            let class = if param.is_active {
                "onecalc-signature-help__parameter onecalc-signature-help__parameter--active"
            } else {
                "onecalc-signature-help__parameter"
            };
            let active_attr = if param.is_active { "true" } else { "false" };
            view! {
                <span class=class data-active=active_attr>{param.name}</span>
                <span class="onecalc-signature-help__separator" aria-hidden="true">
                    {separator}
                </span>
            }
            .into_any()
        })
        .collect::<Vec<_>>();
    view! {
        <div
            class="onecalc-signature-help"
            role="status"
            aria-live="polite"
            data-active-parameter=active_index_attr
            data-parameter-count=parameter_count.to_string()
            style=style
        >
            <span class="onecalc-signature-help__callee">{help.callee_text}</span>
            <span class="onecalc-signature-help__paren" aria-hidden="true">"("</span>
            {parameters}
            <span class="onecalc-signature-help__paren" aria-hidden="true">")"</span>
        </div>
    }
    .into_any()
}

/// Editor-frame inner padding, in pixels. Matches the
/// `padding: var(--oc-space-4)` rule on the textarea + overlay
/// in `theme.rs` at the default 16 px html font-size. The
/// caret-box geometry layer reports coordinates in metric-space
/// (line N starts at `N * line_height_px` from y=0); the actual
/// text renders at that y plus this padding offset because the
/// textarea / overlay have padding inside the editor-frame box.
/// All caret-anchored popovers (completion popup, signature
/// help, future hover-help) MUST add this offset to their top
/// value or they will land inside the line of text rather than
/// above / below it.
const EDITOR_FRAME_PAD_PX: usize = 16;

fn render_completion_popup(
    popup: Option<CompletionPopupView>,
    on_click: Callback<String>,
) -> AnyView {
    let Some(popup) = popup else {
        return view! { <span></span> }.into_any();
    };
    // Anchor 4 px below the bottom of the caret line so the
    // popup never overlaps the typed text. The caret line
    // bottom = padding-top + caret_top_px + line_height_px.
    let style = format!(
        "left: {}px; top: {}px;",
        popup.anchor_left_px.saturating_add(EDITOR_FRAME_PAD_PX),
        popup
            .anchor_top_px
            .saturating_add(popup.line_height_px)
            .saturating_add(EDITOR_FRAME_PAD_PX)
            .saturating_add(4),
    );
    let item_count = popup.items.len();
    let items = popup
        .items
        .into_iter()
        .map(|item| render_completion_popup_item(item, on_click))
        .collect::<Vec<_>>();
    view! {
        <div
            class="onecalc-completion-popup"
            data-selected-index=popup.selected_index.to_string()
            data-item-count=item_count.to_string()
            role="listbox"
            aria-label="completion proposals"
            style=style
        >
            {items}
        </div>
    }
    .into_any()
}

fn render_completion_popup_item(
    item: CompletionPopupItemView,
    on_click: Callback<String>,
) -> AnyView {
    let proposal_id_for_click = item.proposal_id.clone();
    let proposal_id_for_attr = item.proposal_id.clone();
    let kind_label = item.kind_label;
    view! {
        <div
            class="onecalc-completion-popup__item"
            data-proposal-id=proposal_id_for_attr
            data-selected=if item.is_selected { "true" } else { "false" }
            data-kind=item.kind_label.to_ascii_lowercase()
            role="option"
            aria-selected=if item.is_selected { "true" } else { "false" }
            on:mousedown=move |ev| {
                // mousedown (not click) so the textarea doesn't lose
                // focus before the splice runs; preventDefault keeps
                // the focus on the textarea throughout.
                ev.prevent_default();
                on_click.run(proposal_id_for_click.clone());
            }
        >
            <span class="onecalc-completion-popup__glyph" aria-hidden="true">
                {item.kind_glyph.to_string()}
            </span>
            <span class="onecalc-completion-popup__text">{item.display_text}</span>
            <span class="onecalc-completion-popup__kind" aria-hidden="true">
                {kind_label}
            </span>
        </div>
    }
    .into_any()
}

/// Render the editor-foot live-metrics chip. Output shape branches
/// on the view-mode:
///
/// * Developer mode: full counts — `tokens N · functions M ·
///   diagnostics K`. Same as before this bead.
/// * User mode (default): a single status chip carrying the
///   actionable signal an Excel user wants. `<N> issue<s>: <first
///   message>` in the warning palette when diagnostics exist;
///   muted "ready" when the formula is well-formed; nothing when
///   the textarea is empty (no document, all counts zero).
///
/// The data-tokens / data-functions / data-diagnostics attributes
/// stay on the rendered span in BOTH modes so the seam-status
/// board (later bead) and the corpus can read them without
/// switching modes.
fn render_editor_metrics_chip(metrics: Option<EditorMetricsChip>, view_mode: ViewMode) -> AnyView {
    let Some(metrics) = metrics else {
        return view! { <span></span> }.into_any();
    };
    let data_tokens = metrics.token_count.to_string();
    let data_functions = metrics.function_count.to_string();
    let data_diagnostics = metrics.diagnostic_count.to_string();
    match view_mode {
        ViewMode::Developer => {
            let summary = format!(
                "tokens {} · functions {} · diagnostics {}",
                metrics.token_count, metrics.function_count, metrics.diagnostic_count
            );
            view! {
                <span
                    class="onecalc-home-shell__chip onecalc-home-shell__chip--metrics"
                    data-mode="developer"
                    data-tokens=data_tokens
                    data-functions=data_functions
                    data-diagnostics=data_diagnostics
                >
                    {summary}
                </span>
            }
            .into_any()
        }
        ViewMode::User => {
            // Empty mount (no document yet, no input): omit the
            // chip entirely — nothing useful to say.
            if metrics.token_count == 0 && metrics.diagnostic_count == 0 {
                return view! { <span></span> }.into_any();
            }
            if metrics.diagnostic_count == 0 {
                return view! {
                    <span
                        class="onecalc-home-shell__chip \
                               onecalc-home-shell__chip--metrics \
                               onecalc-home-shell__chip--ready"
                        data-mode="user"
                        data-status="ready"
                        data-tokens=data_tokens
                        data-functions=data_functions
                        data-diagnostics=data_diagnostics
                    >
                        "ready"
                    </span>
                }
                .into_any();
            }
            let plural = if metrics.diagnostic_count == 1 {
                "issue"
            } else {
                "issues"
            };
            let message = metrics.first_diagnostic_message.clone().unwrap_or_default();
            let summary = if message.is_empty() {
                format!("{} {plural}", metrics.diagnostic_count)
            } else {
                format!("{} {plural}: {}", metrics.diagnostic_count, message)
            };
            view! {
                <span
                    class="onecalc-home-shell__chip \
                           onecalc-home-shell__chip--metrics \
                           onecalc-home-shell__chip--warning"
                    data-mode="user"
                    data-status="diagnostic"
                    data-tokens=data_tokens
                    data-functions=data_functions
                    data-diagnostics=data_diagnostics
                >
                    {summary}
                </span>
            }
            .into_any()
        }
    }
}

/// Render the result-foot active-context chip: `locale · format ·
/// policy`. Output shape branches on the view-mode:
///
/// * Developer mode: SEAM-pending fields carry a trailing
///   `<NOT IMPL:SEAM-id>` sentinel and the `data-seam-id` /
///   `aria-describedby` attributes (same as before this bead).
/// * User mode (default): plain `value · value · value` — no SEAM
///   sentinels, no warning palette. The data-seam-id attribute
///   stays on the field span so the seam-status board can read
///   it without switching modes; only the user-visible badge text
///   is hidden.
fn render_result_context_chip(chip: Option<ResultContextChip>, view_mode: ViewMode) -> AnyView {
    let Some(chip) = chip else {
        return view! { <span></span> }.into_any();
    };
    let mode_attr = view_mode.slug();
    view! {
        <span
            class="onecalc-home-shell__chip onecalc-home-shell__chip--context"
            data-mode=mode_attr
        >
            {render_context_field(&chip.format, "format", view_mode)}
            <span class="onecalc-home-shell__chip-sep">" · "</span>
            {render_context_field(&chip.policy, "policy", view_mode)}
        </span>
    }
    .into_any()
}

fn render_context_field(
    field: &ContextChipField,
    role: &'static str,
    view_mode: ViewMode,
) -> AnyView {
    let value = field.value().to_string();
    let render_seam_label = matches!(view_mode, ViewMode::Developer);
    match field.seam_id() {
        None => view! {
            <span class="onecalc-home-shell__chip-field" data-role=role>
                {value}
            </span>
        }
        .into_any(),
        Some(seam_id) => {
            let seam_owned = seam_id.to_string();
            let aria_owned = seam_id.to_string();
            // Always carry data-seam-id so the seam-status board
            // can find these regardless of mode. Only the badge
            // TEXT is mode-conditional.
            let badge = if render_seam_label {
                let seam_label = format!("<NOT IMPL:{seam_id}>");
                view! {
                    <span class="onecalc-home-shell__chip-seam">{seam_label}</span>
                }
                .into_any()
            } else {
                view! { <span></span> }.into_any()
            };
            let class = if render_seam_label {
                "onecalc-home-shell__chip-field onecalc-home-shell__chip-field--seam"
            } else {
                "onecalc-home-shell__chip-field"
            };
            view! {
                <span
                    class=class
                    data-role=role
                    data-seam-id=seam_owned
                    aria-describedby=aria_owned
                >
                    {value}
                    {badge}
                </span>
            }
            .into_any()
        }
    }
}

fn role_class(role: SyntaxTokenRole) -> &'static str {
    match role {
        SyntaxTokenRole::Operator => "syn-op",
        SyntaxTokenRole::Function => "syn-fn",
        SyntaxTokenRole::Number => "syn-num",
        SyntaxTokenRole::Delimiter => "syn-delim",
        SyntaxTokenRole::Identifier => "syn-id",
        SyntaxTokenRole::Text => "syn-text",
        SyntaxTokenRole::Trivia => "syn-trivia",
    }
}

/// Slug for `data-token-role` attribute on syntax-overlay spans.
/// Mirrors `role_class` but stripped of the `syn-` prefix so the
/// attribute reads like an enum tag.
fn role_slug(role: SyntaxTokenRole) -> &'static str {
    match role {
        SyntaxTokenRole::Operator => "operator",
        SyntaxTokenRole::Function => "function",
        SyntaxTokenRole::Number => "number",
        SyntaxTokenRole::Delimiter => "delimiter",
        SyntaxTokenRole::Identifier => "identifier",
        SyntaxTokenRole::Text => "text",
        SyntaxTokenRole::Trivia => "trivia",
    }
}

/// Render the editor-caption entry-mode pill. The pill is always present
/// (even for `Empty`) so the caption row keeps a stable height.
fn render_entry_mode_pill(pill: Option<EntryModePill>) -> AnyView {
    let Some(pill) = pill else {
        return view! { <span></span> }.into_any();
    };
    view! {
        <span
            class="onecalc-home-shell__caption-pill onecalc-home-shell__caption-pill--entry"
            data-mode=pill.slug()
        >
            {pill.label()}
        </span>
    }
    .into_any()
}

/// Render the result-caption result-class pill. Suppressed entirely for
/// `Empty` and `Pending` so the caption reads simply "result ▸".
fn render_result_class_pill(pill: Option<ResultClassPill>) -> AnyView {
    let Some(pill) = pill else {
        return view! { <span></span> }.into_any();
    };
    view! {
        <span
            class="onecalc-home-shell__caption-pill onecalc-home-shell__caption-pill--result"
            data-class=pill.slug()
        >
            {pill.label()}
        </span>
    }
    .into_any()
}

fn result_kind_attr(view: ResultView) -> &'static str {
    match view {
        ResultView::Empty => "empty",
        ResultView::Pending => "pending",
        ResultView::Display { .. } => "display",
        ResultView::Error { .. } => "error",
        ResultView::Array { .. } => "array",
    }
}

fn display_kind_attr(kind: ResultKind) -> &'static str {
    match kind {
        ResultKind::Number => "number",
        ResultKind::Text => "text",
        ResultKind::Logical => "logical",
        ResultKind::RichValue => "rich-value",
        ResultKind::Other => "other",
    }
}

#[cfg(target_arch = "wasm32")]
fn build_save_payload(state: &OneCalcHostState) -> Option<(String, String)> {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref()?;
    let formula_space = state.formula_spaces.get(active_id)?;
    let now = current_iso8601_utc();
    let scenario = crate::persistence::formula_space_to_scenario(formula_space, now.clone(), now);
    let stem =
        crate::persistence::suggested_filename_stem(&scenario.identity.name, &scenario.identity.id);
    let xml = crate::persistence::write_formula_xml(&scenario);
    Some((format!("{stem}.dnafml"), xml))
}

#[cfg(target_arch = "wasm32")]
fn current_iso8601_utc() -> String {
    let date = js_sys::Date::new_0();
    date.to_iso_string().as_string().unwrap_or_default()
}

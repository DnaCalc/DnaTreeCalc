//! Shared ATLAS spine widgets — the embedded **Lens** (inspector), **Console**
//! (health + profile), and **Name-Box** that every Phase-A mono-lens embeds,
//! plus the keyboard/commit helpers the grammar contract requires.
//!
//! Defined once so the suite reads as one tool: the same selection facts, the
//! same health tallies, the same `/` matching, and the same modeless edit
//! buffer in every lens. In the Phase-B cockpit these become real companion
//! slots without changing their reads/writes.
//!
//! Grammar discipline codified here:
//! - [`event_target_is_text_entry`] — bare-key verbs must never fire while the
//!   user is typing; every lens keydown handler calls this first.
//! - [`event_target_is_interactive`] — Enter on a focused button must activate
//!   the button, not the Commit verb.
//! - Text inputs owned by these widgets stop propagation of their keydowns so
//!   no lens-level handler can steal typed characters.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, FormulaBindPreviewProjection, IntentReceipt, MutationImpactProjection, NodeId,
    NodeValueProjection, PreviewService, SelectionState, SharedSkinStateHandle, SharedStateChange,
    SharedStateOrigin, WorkspaceIntent, WorkspaceRecalcMode, WorkspaceState, calc_state_class,
};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use crate::value_render::render_value;

/// True when the keyboard event originates from a text-entry element. Bare-key
/// grammar verbs belong to the edit buffer while one has focus.
pub(crate) fn event_target_is_text_entry(ev: &leptos::ev::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
        return false;
    };
    matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        || element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false")
}

/// True when the keyboard event originates from an interactive control whose
/// native key handling (Enter/Space activation) must win over grammar verbs.
pub(crate) fn event_target_is_interactive(ev: &leptos::ev::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
        return false;
    };
    matches!(
        element.tag_name().as_str(),
        "BUTTON" | "A" | "INPUT" | "TEXTAREA" | "SELECT"
    )
}

/// The one commit path for content edits: respects the shared recalc mode and
/// maintains the `manual_recalc_pending` flag through the audited shared-state
/// chokepoint, identically in every lens.
pub(crate) fn commit_content_edit(
    dispatch: &Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    origin: &SharedStateOrigin,
    node: NodeId,
    content: String,
) -> IntentReceipt {
    match shared.get_untracked().recalc_mode {
        WorkspaceRecalcMode::Auto => {
            let receipt = dispatch.dispatch(WorkspaceIntent::EditContent { node, content });
            if receipt.accepted {
                shared.apply(
                    SharedStateChange::SetManualRecalcPending(false),
                    origin.clone(),
                );
            }
            receipt
        }
        WorkspaceRecalcMode::Manual => {
            let receipt = dispatch.dispatch(WorkspaceIntent::EditContentDeferred { node, content });
            if receipt.accepted {
                shared.apply(
                    SharedStateChange::SetManualRecalcPending(true),
                    origin.clone(),
                );
            }
            receipt
        }
    }
}

/// Clear a node's contents (the Excel `Delete` key): set it Empty,
/// non-structurally, through the one recalc-mode-aware commit path.
pub(crate) fn clear_content_edit(
    dispatch: &Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    origin: &SharedStateOrigin,
    node: NodeId,
) -> IntentReceipt {
    commit_content_edit(dispatch, shared, origin, node, String::new())
}

/// Clear a table cell's value (Excel `Delete`). Valid only for direct-input
/// cells; the host rejects a formula-backed cell, so the caller gates on
/// editability before calling.
pub(crate) fn clear_table_cell_edit(
    dispatch: &Arc<dyn Dispatcher>,
    table: NodeId,
    row_id: String,
    column_id: String,
) -> IntentReceipt {
    dispatch.dispatch(WorkspaceIntent::EditTableCell {
        table,
        row_id,
        column_id,
        content: String::new(),
    })
}

/// Whether deleting a node needs confirmation, and the message to show.
/// `None` = a clean leaf with no external dependents (delete directly);
/// `Some(message)` = destructive (has child nodes or breaks references) — show
/// a [`ConfirmBar`] first. Pure over the projection so it is unit-testable.
pub(crate) fn delete_confirm_message(
    impact: Option<&MutationImpactProjection>,
    child_count: usize,
) -> Option<String> {
    let breaks = impact.map_or(0, |impact| impact.orphaned_dependents.len());
    if child_count == 0 && breaks == 0 {
        return None;
    }
    let mut clauses = Vec::new();
    if child_count > 0 {
        clauses.push(if child_count == 1 {
            "removes 1 child".to_string()
        } else {
            format!("removes {child_count} children")
        });
    }
    if breaks > 0 {
        clauses.push(if breaks == 1 {
            "breaks 1 reference".to_string()
        } else {
            format!("breaks {breaks} references")
        });
    }
    Some(format!("Delete this node? It {}.", clauses.join(" and ")))
}

/// Record the active lens through the audited chokepoint. Only a Main-slot
/// mount claims the active-lens chrome — a companion mount never does.
pub(crate) fn stamp_active_lens(
    shared: SharedSkinStateHandle,
    skin_id: dnatreecalc_skin_framework::SkinId,
    slot: dnatreecalc_skin_framework::SkinMountSlot,
) {
    if slot == dnatreecalc_skin_framework::SkinMountSlot::Main {
        shared.apply(
            SharedStateChange::SetActiveLens(skin_id.as_str().to_string()),
            SharedStateOrigin::lens(skin_id, slot),
        );
    }
}

/// The canonical `/` model query: first visible (non-effective-meta) node whose
/// path or name contains the query, case-insensitively. Defined once so `/`
/// matches identically in every lens.
pub(crate) fn first_match(workspace: &WorkspaceState, query: &str) -> Option<NodeId> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    workspace.node_order.iter().find_map(|id| {
        let node = workspace.node(id)?;
        if workspace.is_effective_meta(&node.key) {
            return None;
        }
        let matches = id.as_str().to_lowercase().contains(&needle)
            || node.display_name.to_lowercase().contains(&needle);
        matches.then(|| id.clone())
    })
}

/// The `/` Name-Box quick-jump bar. Renders only while `name_box` is `Some`.
///
/// The input is **uncontrolled**: the render closure depends only on the
/// open/closed state (so typing never rebuilds the element), focus is applied
/// programmatically on mount (`autofocus` is unreliable on dynamic insertion),
/// and Enter reads the *live* value from the event target — never a captured
/// snapshot. Every keydown stops propagation so typed characters can never
/// reach a lens-level grammar handler (the grammar contract).
#[component]
pub(crate) fn NameBoxBar(
    name_box: RwSignal<Option<String>>,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let open = Memo::new(move |_| name_box.with(Option::is_some));
    let input_ref = NodeRef::<leptos::html::Input>::new();
    // Focus the box whenever it mounts (NodeRef tracks attachment).
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });
    let jump_dispatch = dispatch.clone();
    view! {
        {move || {
            if !open.get() {
                return ().into_any();
            }
            {
                let jump_dispatch = jump_dispatch.clone();
                view! {
                    <div class="dtc-namebox">
                        <span class="dtc-namebox__sigil">"/"</span>
                        <input
                            class="dtc-namebox__input"
                            node_ref=input_ref
                            placeholder="jump to node…"
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                // Typed characters belong to this input alone.
                                ev.stop_propagation();
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        let query = event_target_value(&ev);
                                        if let Some(id) = first_match(&workspace.get_untracked(), &query) {
                                            jump_dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id)));
                                        }
                                        name_box.set(None);
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        name_box.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        />
                    </div>
                }
                .into_any()
            }
        }}
    }
}

/// Health tallies over the visible (non-effective-meta) population.
pub(crate) fn health_tallies(workspace: &WorkspaceState) -> (usize, usize, usize, usize) {
    let mut total = 0usize;
    let mut clean = 0usize;
    let mut stale = 0usize;
    let mut error = 0usize;
    for node in workspace.nodes_by_key.values() {
        if workspace.is_effective_meta(&node.key) {
            continue;
        }
        total += 1;
        if matches!(node.computed_value, NodeValueProjection::Error(_)) {
            error += 1;
        }
        let class = calc_state_class(node.calc_state);
        if class == "dtc-calc--clean" {
            clean += 1;
        } else if class == "dtc-calc--stale" || class == "dtc-calc--unknown" {
            stale += 1;
        }
    }
    (total, clean, stale, error)
}

/// The embedded **Console**: calc-state health tallies, node count, the
/// capability-profile badge, and the recalculate affordance. One per lens,
/// identical everywhere.
///
/// An EMBEDDED console (the default) **stands down** while a Console companion
/// occupies the `BottomConsole` slot — same truth, one rendering, in every
/// lens for free. The companion itself passes `standalone=true`.
#[component]
pub(crate) fn ConsoleBar(
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    #[prop(optional)] standalone: bool,
) -> impl IntoView {
    let shared_signal = shared.signal();
    let stood_down = Memo::new(move |_| {
        !standalone
            && shared_signal.with(|s| {
                s.companion_slots_active
                    .contains(&dnatreecalc_skin_framework::SkinMountSlot::BottomConsole)
            })
    });
    let health = Memo::new(move |_| workspace.with(health_tallies));
    let profile = Memo::new(move |_| workspace.with(|ws| ws.profile.clone()));

    view! {
        {move || {
            if stood_down.get() {
                return ().into_any();
            }
            let recalc_dispatch = dispatch.clone();
            view! {
                <footer class="dtc-console" aria-label="Model health">
                    <span class="dtc-console__group">
                        <span class="dtc-calc-dot dtc-calc--clean"></span>
                        <span>{move || format!("{} clean", health.get().1)}</span>
                    </span>
                    <span class="dtc-console__group">
                        <span class="dtc-calc-dot dtc-calc--stale"></span>
                        <span>{move || format!("{} stale", health.get().2)}</span>
                    </span>
                    <span class="dtc-console__group">
                        <span class="dtc-calc-dot dtc-calc--rejected"></span>
                        <span>{move || format!("{} error", health.get().3)}</span>
                    </span>
                    <span class="dtc-console__spacer"></span>
                    <span class="dtc-console__group">{move || format!("{} nodes", health.get().0)}</span>
                    <span class="dtc-console__profile" title="Capability profile">{move || profile.get()}</span>
                    <button type="button" on:click=move |_| { recalc_dispatch.dispatch(WorkspaceIntent::Recalculate); }>
                        "Recalculate (F9)"
                    </button>
                </footer>
            }
            .into_any()
        }}
    }
}

/// The embedded **Lens**: selection facts, value, the modeless edit buffer with
/// the 1-bit SELECTED/EDITING cue, and binding diagnostics. Lens-specific
/// extras (Flow's explain stack, Bench's override panel, …) render as
/// `children` below the shared content.
///
/// The edit buffer's `editing`/`editor_text`/`edit_ref` signals are owned by
/// the caller so the lens grammar (Enter-to-edit with programmatic focus) can
/// drive them. The textarea stops propagation of every keydown — its keys are
/// its own while focused.
///
/// An EMBEDDED inspector (the default) **stands down** while a Lens companion
/// occupies the `RightInspector` slot — same truth, one rendering, in every
/// lens for free. The companion itself passes `standalone=true`.
#[component]
pub(crate) fn NodeInspector(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    editing: RwSignal<bool>,
    editor_text: RwSignal<String>,
    edit_ref: NodeRef<leptos::html::Textarea>,
    commit: Arc<dyn Fn() + Send + Sync>,
    shared: SharedSkinStateHandle,
    #[prop(optional_no_strip)] preview: Option<Arc<dyn PreviewService>>,
    #[prop(optional)] standalone: bool,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let shared_signal = shared.signal();
    let stood_down = Memo::new(move |_| {
        !standalone
            && shared_signal.with(|s| {
                s.companion_slots_active
                    .contains(&dnatreecalc_skin_framework::SkinMountSlot::RightInspector)
            })
    });
    let detail =
        Memo::new(move |_| selection.with(|sel| workspace.with(|ws| ws.active_node_detail(sel))));
    let commit_for_button = commit.clone();
    let commit_for_keys = commit;

    // Live legality (tenet 7): while the buffer is being edited and the host
    // offers foresight, dry-bind the prospective content through the engine
    // and surface the typed verdict — never computed lens-side. Synchronous
    // per keystroke; cheap at interactive model sizes (OxFml parse+bind only).
    let live_verdict = RwSignal::new(Option::<FormulaBindPreviewProjection>::None);
    if let Some(service) = preview.clone() {
        Effect::new(move |_| {
            if !editing.get() {
                live_verdict.set(None);
                return;
            }
            let content = editor_text.get();
            let node = selection.with_untracked(|sel| sel.primary.clone());
            let Some(node) = node else {
                live_verdict.set(None);
                return;
            };
            live_verdict.set(service.preview_formula_bind(&node, &content).ok());
        });
    }

    view! {
        <aside
            class="dtc-inspector"
            class:dtc-inspector--stood-down=move || stood_down.get()
            aria-label="Selection inspector"
        >
            {move || if stood_down.get() {
                ().into_any()
            } else {
                match detail.get() {
                None => view! { <p class="dtc-inspector__empty">"Select a node to inspect it."</p> }.into_any(),
                Some(detail) => {
                    let calc = detail.calc_state.map_or("—".to_string(), |state| state.to_string());
                    let format = detail
                        .effective_format
                        .as_ref()
                        .and_then(|fmt| fmt.number_format_code.clone())
                        .unwrap_or_else(|| "General".to_string());
                    let out_refs = detail.outgoing_references.len();
                    let in_refs = detail.incoming_reference_handles.len();
                    let diagnostics = detail.binding_diagnostics.clone();
                    let commit_for_keys = commit_for_keys.clone();
                    let commit_for_button = commit_for_button.clone();
                    let extras = children.clone();
                    view! {
                        <header class="dtc-inspector__header">
                            <span class="dtc-inspector__name">{detail.display_name.clone()}</span>
                            <code class="dtc-inspector__path">{detail.node.to_string()}</code>
                        </header>
                        <dl class="dtc-inspector__facts">
                            <div><dt>"calc-state"</dt><dd>{calc}</dd></div>
                            <div><dt>"format"</dt><dd>{format}</dd></div>
                            <div><dt>"precedents"</dt><dd>{out_refs}</dd></div>
                            <div><dt>"dependents"</dt><dd>{in_refs}</dd></div>
                        </dl>
                        <div class="dtc-section-label">"Value"</div>
                        {render_value(&detail.value)}
                        <div class="dtc-section-label">"Content"</div>
                        <textarea
                            class="dtc-inspector__edit"
                            class:dtc-inspector__edit--active=move || editing.get()
                            node_ref=edit_ref
                            prop:value=move || editor_text.get()
                            on:focus=move |_| editing.set(true)
                            on:input=move |ev| editor_text.set(event_target_value(&ev))
                            on:keydown={
                                move |ev: leptos::ev::KeyboardEvent| {
                                    // Keys typed in the buffer are the buffer's.
                                    ev.stop_propagation();
                                    match ev.key().as_str() {
                                        "Enter" if !ev.shift_key() => {
                                            ev.prevent_default();
                                            (commit_for_keys)();
                                        }
                                        "Escape" => {
                                            ev.prevent_default();
                                            editing.set(false);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        />
                        {move || {
                            if !editing.get() {
                                return ().into_any();
                            }
                            match live_verdict.get() {
                                None => ().into_any(),
                                Some(verdict) if verdict.legal => view! {
                                    <div class="dtc-inspector__legality dtc-inspector__legality--ok">
                                        "✓ binds clean"
                                    </div>
                                }
                                .into_any(),
                                Some(verdict) => view! {
                                    <ul class="dtc-inspector__legality dtc-inspector__legality--blocked">
                                        {verdict
                                            .diagnostics
                                            .iter()
                                            .map(|diag| view! { <li>{format!("{}: {}", diag.stage, diag.message)}</li> })
                                            .collect::<Vec<_>>()}
                                        {verdict
                                            .profile_violations
                                            .iter()
                                            .map(|violation| view! { <li>{violation.message.clone()}</li> })
                                            .collect::<Vec<_>>()}
                                    </ul>
                                }
                                .into_any(),
                            }
                        }}
                        <button
                            type="button"
                            class="dtc-inspector__commit"
                            on:click=move |_| (commit_for_button)()
                        >
                            "Commit (Enter)"
                        </button>
                        {(!diagnostics.is_empty()).then(|| view! {
                            <div class="dtc-section-label">"Binding diagnostics"</div>
                            <ul class="dtc-inspector__diagnostics">
                                {diagnostics.into_iter().map(|diag| view! {
                                    <li>{diag.message}</li>
                                }).collect::<Vec<_>>()}
                            </ul>
                        }.into_any())}
                        {extras.map(|extra| extra())}
                    }
                    .into_any()
                }
            }}}
        </aside>
    }
}

/// Move keyboard focus to the lens edit buffer with the caret at the end.
/// No-op when the textarea is not mounted (e.g. nothing selected).
pub(crate) fn focus_edit_buffer(edit_ref: NodeRef<leptos::html::Textarea>) {
    if let Some(textarea) = edit_ref.get_untracked() {
        let _ = textarea.focus();
        // Browsers clamp the range to the value length.
        let _ = textarea.set_selection_range(u32::MAX, u32::MAX);
    }
}

/// A pending destructive action awaiting confirmation — surfaced by [`ConfirmBar`].
#[derive(Clone)]
pub(crate) struct PendingConfirm {
    pub message: String,
    pub on_confirm: Arc<dyn Fn() + Send + Sync>,
}

/// An inline confirm strip (no native dialogs) for guarded destructive actions.
/// Renders only while `pending` is `Some`; Confirm runs the action then clears,
/// Cancel just clears. A lens wires `Escape` to `pending.set(None)`.
#[component]
pub(crate) fn ConfirmBar(pending: RwSignal<Option<PendingConfirm>>) -> impl IntoView {
    view! {
        {move || {
            let Some(confirm) = pending.get() else {
                return ().into_any();
            };
            let on_confirm = confirm.on_confirm.clone();
            view! {
                <div class="dtc-confirm" role="alertdialog" aria-live="assertive">
                    <span class="dtc-confirm__message">{confirm.message.clone()}</span>
                    <button
                        type="button"
                        class="dtc-confirm__go"
                        on:click=move |_| {
                            (on_confirm)();
                            pending.set(None);
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        class="dtc-confirm__cancel"
                        on:click=move |_| pending.set(None)
                    >
                        "Cancel"
                    </button>
                </div>
            }
            .into_any()
        }}
    }
}

/// A label that becomes an input on double-click for in-place renaming. Enter
/// or blur commit via `on_commit`; Escape cancels. Stops propagation of its
/// keydowns (the grammar contract). Commit fires once — the blur that follows
/// an Enter/Escape sees `editing` already cleared and is skipped.
#[component]
pub(crate) fn InlineLabelEdit(
    value: String,
    on_commit: Arc<dyn Fn(String) + Send + Sync>,
    #[prop(optional)] label_class: &'static str,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let text = RwSignal::new(value.clone());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if editing.get()
            && let Some(input) = input_ref.get()
        {
            let _ = input.focus();
        }
    });
    let label = value.clone();
    let seed = value;
    let commit_enter = on_commit.clone();
    let commit_blur = on_commit;
    view! {
        {move || {
            if editing.get() {
                let commit_enter = commit_enter.clone();
                let commit_blur = commit_blur.clone();
                view! {
                    <input
                        class="dtc-inline-edit"
                        node_ref=input_ref
                        prop:value=move || text.get()
                        on:input=move |ev| text.set(event_target_value(&ev))
                        on:blur=move |_| {
                            if editing.get_untracked() {
                                (commit_blur)(text.get_untracked());
                                editing.set(false);
                            }
                        }
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            ev.stop_propagation();
                            match ev.key().as_str() {
                                "Enter" => {
                                    ev.prevent_default();
                                    (commit_enter)(text.get_untracked());
                                    editing.set(false);
                                }
                                "Escape" => {
                                    ev.prevent_default();
                                    editing.set(false);
                                }
                                _ => {}
                            }
                        }
                    />
                }
                .into_any()
            } else {
                let label = label.clone();
                let seed = seed.clone();
                view! {
                    <span
                        class=label_class
                        title="Double-click to rename"
                        on:dblclick=move |_| {
                            text.set(seed.clone());
                            editing.set(true);
                        }
                    >
                        {label}
                    </span>
                }
                .into_any()
            }
        }}
    }
}

/// Shared CSS for the spine widgets. Include once per lens, after
/// [`dnatreecalc_skin_framework::ATLAS_SPINE_CSS`].
pub(crate) const SPINE_WIDGETS_CSS: &str = r#"
.dtc-namebox {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-namebox__sigil { font-weight: 700; color: var(--dtc-accent); }
.dtc-namebox__input { flex: 1; padding: 3px 6px; border: 1px solid var(--dtc-border); border-radius: 5px; background: var(--dtc-surface-input); color: var(--dtc-text); }

.dtc-console {
  display: flex; align-items: center; gap: 14px; padding: 6px 12px;
  border-top: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-subtle);
}
.dtc-console__group { display: inline-flex; align-items: center; gap: 6px; color: var(--dtc-text-muted); }
.dtc-console__spacer { flex: 1; }
.dtc-console__profile {
  padding: 1px 8px; border: 1px solid var(--dtc-border); border-radius: 9px;
  color: var(--dtc-text-subtle); font-size: 11px;
}
.dtc-console button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}

.dtc-inspector {
  width: 320px; flex-shrink: 0; overflow: auto; padding: 12px;
  border-left: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-panel);
}
.dtc-inspector--stood-down { display: none; }
.dtc-inspector__empty { color: var(--dtc-text-subtle); font-style: italic; }
.dtc-inspector__header { display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px; }
.dtc-inspector__name { font-weight: 700; font-size: 15px; }
.dtc-inspector__path { color: var(--dtc-text-subtle); font-size: 12px; }
.dtc-inspector__facts { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px; margin: 0 0 8px; }
.dtc-inspector__facts div { display: flex; justify-content: space-between; }
.dtc-inspector__facts dt { color: var(--dtc-text-subtle); }
.dtc-inspector__facts dd { margin: 0; font-variant-numeric: tabular-nums; }
.dtc-inspector__edit { width: 100%; min-height: 60px; box-sizing: border-box; padding: 6px; border: 1px solid var(--dtc-border); border-radius: 5px; background: var(--dtc-surface-input); color: var(--dtc-text); font-family: ui-monospace, monospace; }
.dtc-inspector__edit--active { border-color: var(--dtc-warning); box-shadow: inset 0 0 0 1px var(--dtc-warning); }
.dtc-inspector__commit {
  margin-top: 6px; background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border);
  border-radius: 5px; padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-inspector__diagnostics { margin: 4px 0; padding-left: 16px; color: var(--dtc-danger-text); }
.dtc-inspector__legality { margin: 4px 0; font-size: 12px; }
.dtc-inspector__legality--ok { color: var(--dtc-success-text); }
.dtc-inspector__legality--blocked {
  color: var(--dtc-danger-text); padding: 4px 8px 4px 20px;
  background: var(--dtc-danger-surface); border: 1px solid var(--dtc-danger-border); border-radius: 5px;
}

.dtc-section-label { margin: 10px 0 4px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--dtc-text-subtle); }
.dtc-value-display { padding: 4px 0; font-variant-numeric: tabular-nums; }
.dtc-value-display--error { color: var(--dtc-danger-text); }

.dtc-confirm {
  display: flex; align-items: center; gap: 8px; margin: 6px 0; padding: 6px 10px;
  background: var(--dtc-danger-surface); border: 1px solid var(--dtc-danger-border);
  border-radius: 6px; color: var(--dtc-danger-text);
}
.dtc-confirm__message { flex: 1; }
.dtc-confirm__go {
  background: var(--dtc-danger, #c0392b); color: #fff; border: none; border-radius: 5px;
  padding: 2px 10px; cursor: pointer; font-weight: 600;
}
.dtc-confirm__cancel {
  background: var(--dtc-surface); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 10px; cursor: pointer; color: var(--dtc-text);
}
.dtc-inline-edit {
  width: 100%; box-sizing: border-box; padding: 2px 6px; border: 1px solid var(--dtc-warning);
  border-radius: 4px; background: var(--dtc-surface-input); color: var(--dtc-text); font: inherit;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{InMemoryDispatcher, NodeKey, WorkspaceRecalcMode};
    use std::collections::BTreeMap;

    fn workspace_with(nodes: Vec<dnatreecalc_skin_framework::NodeView>) -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for node in nodes {
            ws.node_order.push(node.id.clone());
            ws.key_order.push(node.key.clone());
            ws.paths_by_key.insert(node.key.clone(), node.id.clone());
            ws.nodes.insert(node.id.clone(), node.clone());
            ws.nodes_by_key.insert(node.key.clone(), node);
        }
        ws
    }

    fn node(key: &str, path: &str, is_meta: bool) -> dnatreecalc_skin_framework::NodeView {
        dnatreecalc_skin_framework::NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(path),
            display_name: path.rsplit('.').next().unwrap_or(path).to_string(),
            parent: path
                .rsplit_once('.')
                .map(|(parent, _)| NodeId::new(parent)),
            children: Vec::new(),
            depth: path.matches('.').count() as u32,
            content_kind: dnatreecalc_skin_framework::NodeContentKind::Constant,
            content_text: "1".to_string(),
            computed_value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta,
            table: None,
        }
    }

    #[test]
    fn first_match_skips_effective_meta_descendants() {
        // Meta parent with a regular-flagged child: the child is effectively
        // meta and must not match.
        let ws = workspace_with(vec![
            node("k:m", "Meta", true),
            node("k:mc", "Meta.Child", false),
            node("k:v", "Visible", false),
        ]);
        assert_eq!(first_match(&ws, "child"), None);
        assert_eq!(first_match(&ws, "visible"), Some(NodeId::new("Visible")));
    }

    #[test]
    fn health_tallies_skip_effective_meta() {
        let ws = workspace_with(vec![
            node("k:m", "Meta", true),
            node("k:mc", "Meta.Child", false),
            node("k:v", "Visible", false),
        ]);
        let (total, _, _, _) = health_tallies(&ws);
        assert_eq!(total, 1);
    }

    #[test]
    fn commit_content_edit_matches_recalc_mode() {
        let selection = RwSignal::new(SelectionState::default());
        let dispatcher = InMemoryDispatcher::new(selection);
        let log = std::sync::Arc::new(dispatcher);
        let dispatch: Arc<dyn Dispatcher> = log.clone();
        let shared = SharedSkinStateHandle::new(dnatreecalc_skin_framework::SharedSkinState {
            recalc_mode: WorkspaceRecalcMode::Manual,
            ..Default::default()
        });

        let origin = SharedStateOrigin::lens(
            dnatreecalc_skin_framework::SkinId::new("test-lens"),
            dnatreecalc_skin_framework::SkinMountSlot::Main,
        );
        let receipt =
            commit_content_edit(&dispatch, shared, &origin, NodeId::new("A"), "=1+1".to_string());
        assert!(receipt.accepted);
        assert!(shared.get_untracked().manual_recalc_pending);
        assert!(matches!(
            log.intents()[0],
            WorkspaceIntent::EditContentDeferred { .. }
        ));
        // The pending flag flowed through the audited chokepoint.
        let audit = shared.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].origin, origin);
        assert!(matches!(
            audit[0].change,
            SharedStateChange::SetManualRecalcPending(true)
        ));
    }

    #[test]
    fn delete_confirm_message_only_warns_when_destructive() {
        use dnatreecalc_skin_framework::{
            MutationImpactIntentProjection, RecalcPlanProjection,
        };
        // Clean leaf, no dependents → delete directly (no confirm).
        assert_eq!(delete_confirm_message(None, 0), None);
        // Has children → confirm.
        assert_eq!(
            delete_confirm_message(None, 2).as_deref(),
            Some("Delete this node? It removes 2 children.")
        );
        // Breaks references → confirm (impact-driven).
        let impact = MutationImpactProjection {
            intent: MutationImpactIntentProjection::DeleteNode {
                node: NodeId::new("A"),
            },
            legal: true,
            blocked_reason: None,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            invalidation_plan: RecalcPlanProjection::default(),
            requires_rebind: Vec::new(),
            affected_refs: Vec::new(),
            orphaned_dependents: vec![NodeId::new("B")],
            collisions: Vec::new(),
        };
        assert_eq!(
            delete_confirm_message(Some(&impact), 0).as_deref(),
            Some("Delete this node? It breaks 1 reference.")
        );
        // Both clauses.
        assert_eq!(
            delete_confirm_message(Some(&impact), 1).as_deref(),
            Some("Delete this node? It removes 1 child and breaks 1 reference.")
        );
    }
}

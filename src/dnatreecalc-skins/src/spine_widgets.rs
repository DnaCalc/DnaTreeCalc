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
    Dispatcher, IntentReceipt, NodeId, NodeValueProjection, SelectionState, SharedSkinStateHandle,
    WorkspaceIntent, WorkspaceRecalcMode, WorkspaceState, calc_state_class,
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
/// maintains the `manual_recalc_pending` flag, identically in every lens.
pub(crate) fn commit_content_edit(
    dispatch: &Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    node: NodeId,
    content: String,
) -> IntentReceipt {
    match shared.get_untracked().recalc_mode {
        WorkspaceRecalcMode::Auto => {
            let receipt = dispatch.dispatch(WorkspaceIntent::EditContent { node, content });
            if receipt.accepted {
                shared.update(|state| state.manual_recalc_pending = false);
            }
            receipt
        }
        WorkspaceRecalcMode::Manual => {
            let receipt = dispatch.dispatch(WorkspaceIntent::EditContentDeferred { node, content });
            if receipt.accepted {
                shared.update(|state| state.manual_recalc_pending = true);
            }
            receipt
        }
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
/// Every keydown stops propagation so typed characters can never reach a
/// lens-level grammar handler (the grammar contract).
#[component]
pub(crate) fn NameBoxBar(
    name_box: RwSignal<Option<String>>,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let jump_dispatch = dispatch.clone();
    view! {
        {move || {
            name_box.get().map(|query| {
                let jump_dispatch = jump_dispatch.clone();
                let query_for_jump = query.clone();
                view! {
                    <div class="dtc-namebox">
                        <span class="dtc-namebox__sigil">"/"</span>
                        <input
                            class="dtc-namebox__input"
                            autofocus=true
                            placeholder="jump to node…"
                            prop:value=query.clone()
                            on:input=move |ev| name_box.set(Some(event_target_value(&ev)))
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                // Typed characters belong to this input alone.
                                ev.stop_propagation();
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        if let Some(id) = first_match(&workspace.get_untracked(), &query_for_jump) {
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
            })
            .unwrap_or_else(|| ().into_any())
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
/// identical everywhere; promoted to a real companion slot in Phase B.
#[component]
pub(crate) fn ConsoleBar(
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let health = Memo::new(move |_| workspace.with(health_tallies));
    let profile = Memo::new(move |_| workspace.with(|ws| ws.profile));
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
#[component]
pub(crate) fn NodeInspector(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    editing: RwSignal<bool>,
    editor_text: RwSignal<String>,
    edit_ref: NodeRef<leptos::html::Textarea>,
    commit: Arc<dyn Fn() + Send + Sync>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    let detail =
        Memo::new(move |_| selection.with(|sel| workspace.with(|ws| ws.active_node_detail(sel))));
    let commit_for_button = commit.clone();
    let commit_for_keys = commit;

    view! {
        <aside class="dtc-inspector" aria-label="Selection inspector">
            {move || match detail.get() {
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
            }}
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

.dtc-section-label { margin: 10px 0 4px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--dtc-text-subtle); }
.dtc-value-display { padding: 4px 0; font-variant-numeric: tabular-nums; }
.dtc-value-display--error { color: var(--dtc-danger-text); }
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

        let receipt =
            commit_content_edit(&dispatch, shared, NodeId::new("A"), "=1+1".to_string());
        assert!(receipt.accepted);
        assert!(shared.get_untracked().manual_recalc_pending);
        assert!(matches!(
            log.intents()[0],
            WorkspaceIntent::EditContentDeferred { .. }
        ));
    }
}

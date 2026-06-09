use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, ErasedSkinContext, KeyChord, KeybindingRegistry, NodeId, SelectionState,
    SharedSkinStateHandle, SkinId, SkinMountSlot, SkinRegistry, SkinStatePersistenceStore, SkinVerb,
    ThemeTokens, WorkspaceDelta, WorkspaceIntent, WorkspaceRecalcMode, WorkspaceState,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::theme::SHELL_CSS;

/// The walking-skeleton workspace shell.
///
/// Renders the universal chrome (context strip + status foot) and a main
/// mount slot whose contents are the currently-active skin's view. The
/// active skin is tracked in a local `RwSignal<SkinId>`; switching writes
/// to that signal, which re-runs the mount closure — the slot tears down
/// the old skin's view, calls `mount` on the new skin's registered factory,
/// and runs the previous skin's `on_deactivate` hook (if any) through
/// Leptos's `on_cleanup`.
///
/// Crucially, switching never calls calculation: the dispatch handle the
/// new skin receives is the same `Arc<dyn Dispatcher>`, and no
/// `WorkspaceIntent` is emitted as part of the switch itself. Tests in
/// the host crate assert that the direct OxCalc context recalculation count
/// is unchanged across a switch.
#[component]
pub fn WorkspaceShell(
    workspace: ReadSignal<WorkspaceState>,
    latest_delta: ReadSignal<WorkspaceDelta>,
    selection: RwSignal<SelectionState>,
    shared: SharedSkinStateHandle,
    skin_state_store: Arc<dyn SkinStatePersistenceStore>,
    dispatch: Arc<dyn Dispatcher>,
    registry: Arc<SkinRegistry>,
    initial_skin: SkinId,
    tokens: ThemeTokens,
) -> impl IntoView {
    let current_skin = RwSignal::new(initial_skin);
    let registry_for_view = registry.clone();
    let dispatch_for_view = dispatch.clone();
    let skin_state_store_for_view = skin_state_store.clone();
    let shortcut_skin_ids = registry.ids();
    // The one grammar. It is a constant today; per-user remapping would later
    // thread a customized instance through context instead of reconstructing it.
    let keybindings = KeybindingRegistry::universal();

    let title = Memo::new(move |_| {
        workspace.with(|ws| {
            if ws.workspace_id.is_empty() {
                "Untitled workspace".to_string()
            } else {
                ws.workspace_id.clone()
            }
        })
    });
    let profile = Memo::new(move |_| workspace.with(|ws| ws.profile));
    let node_count = Memo::new(move |_| workspace.with(WorkspaceState::len));
    let selected_label = Memo::new(move |_| {
        selection.with(|s| {
            s.primary
                .as_ref()
                .map(|id| id.as_str().to_string())
                .unwrap_or_else(|| "—".to_string())
        })
    });

    let registry_for_tabs = registry.clone();
    let shortcut_dispatch = dispatch.clone();
    let shell_css = format!("{}\n{}", tokens.css_rule(".dtc-shell"), SHELL_CSS);

    view! {
        <style>{shell_css}</style>
        <div
            class="dtc-shell"
            tabindex="0"
            on:keydown=move |ev| {
                handle_shell_keydown(
                    ev,
                    workspace,
                    selection,
                    current_skin,
                    &shortcut_skin_ids,
                    shortcut_dispatch.clone(),
                    shared,
                    &keybindings,
                );
            }
        >
            <header class="dtc-context-strip">
                <div class="dtc-brand-block">
                    <span class="dtc-context-strip__title">{move || title.get()}</span>
                    <span class="dtc-context-strip__profile">{move || profile.get()}</span>
                </div>
                <WorkspaceManagementControl
                    shared=shared
                    dispatch=dispatch.clone()
                />
                <span class="dtc-context-strip__spacer"></span>
                <SkinSwitcher
                    registry=registry_for_tabs
                    current=current_skin
                />
            </header>
            <ShortcutBar />
            <main class="dtc-main-slot">
                {move || {
                    let id = current_skin.get();
                    let Some(registered) = registry_for_view.get(id) else {
                        return ().into_any();
                    };
                    let cx = ErasedSkinContext {
                        workspace,
                        latest_delta,
                        selection: selection.read_only(),
                        shared,
                        tokens: tokens.clone(),
                        slot: SkinMountSlot::Main,
                        skin_state_store: skin_state_store_for_view.clone(),
                        dispatch: dispatch_for_view.clone(),
                    };
                    let handle = registered.mount(cx);
                    if let Some(hook) = handle.on_deactivate {
                        on_cleanup(hook);
                    }
                    handle.view
                }}
            </main>
            <footer class="dtc-status-foot">
                <span>{move || format!("nodes: {}", node_count.get())}</span>
                <span>{move || format!("selected: {}", selected_label.get())}</span>
                <span class="dtc-context-strip__spacer"></span>
                <RecalcModeControl
                    shared=shared
                    dispatch=dispatch.clone()
                />
                <span class="dtc-status-pill">"clean"</span>
            </footer>
        </div>
    }
}

#[component]
fn ShortcutBar() -> impl IntoView {
    view! {
        <div class="dtc-shortcut-bar" aria-label="Keyboard shortcuts">
            <ShortcutHint keys="Ctrl 1-9" label="lens" />
            <ShortcutHint keys="↑ ↓ ← →" label="navigate" />
            <ShortcutHint keys="F9" label="calculate" />
            <ShortcutHint keys="Ctrl Z / Y" label="undo · redo" />
            <ShortcutHint keys="Ctrl N" label="new workspace" />
        </div>
    }
}

#[component]
fn ShortcutHint(keys: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <span class="dtc-shortcut-hint">
            <kbd>{keys}</kbd>
            <span>{label}</span>
        </span>
    }
}

// The shell keydown dispatcher threads the host-owned signals plus the grammar
// registry; grouping them into a struct would not improve clarity here.
#[allow(clippy::too_many_arguments)]
fn handle_shell_keydown(
    ev: web_sys::KeyboardEvent,
    workspace: ReadSignal<WorkspaceState>,
    selection: RwSignal<SelectionState>,
    current_skin: RwSignal<SkinId>,
    skin_ids: &[SkinId],
    dispatch: Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    keybindings: &KeybindingRegistry,
) {
    let command = ev.ctrl_key() || ev.meta_key();
    let chord = KeyChord::from_parts(ev.key(), command, ev.alt_key(), ev.shift_key());
    let Some(verb) = keybindings.resolve(&chord) else {
        return;
    };

    // While the user is typing, honor only modified global verbs — bare-key
    // grammar (Commit, Fold, NameBox, Explain, arrows, …) belongs to the edit
    // buffer, not the shell.
    if keyboard_target_is_text_entry(&ev) && !command {
        return;
    }

    match verb {
        // --- Global verbs the shell owns ---
        SkinVerb::Recalculate => {
            ev.prevent_default();
            dispatch.dispatch(WorkspaceIntent::Recalculate);
            shared.update(|state| {
                state.manual_recalc_pending = false;
            });
        }
        SkinVerb::NewWorkspace => {
            ev.prevent_default();
            dispatch.dispatch(WorkspaceIntent::NewWorkspace);
        }
        SkinVerb::Undo => {
            ev.prevent_default();
            dispatch.dispatch(WorkspaceIntent::Undo);
        }
        SkinVerb::Redo => {
            ev.prevent_default();
            dispatch.dispatch(WorkspaceIntent::Redo);
        }
        SkinVerb::SwitchLens(slot) => {
            if let Some(id) = skin_id_for_slot(skin_ids, slot) {
                ev.prevent_default();
                current_skin.set(id);
            }
        }
        SkinVerb::NavPrev | SkinVerb::NavNext => {
            ev.prevent_default();
            let direction = if matches!(verb, SkinVerb::NavPrev) { -1 } else { 1 };
            if let Some(next) = adjacent_node_selection(
                &workspace.get_untracked(),
                &selection.get_untracked(),
                direction,
            ) {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(next)));
            }
        }
        SkinVerb::ToParent => {
            if let Some(parent) = parent_of_selection(
                &workspace.get_untracked(),
                &selection.get_untracked(),
            ) {
                ev.prevent_default();
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(parent)));
            }
        }
        SkinVerb::ToChild => {
            if let Some(child) = first_child_of_selection(
                &workspace.get_untracked(),
                &selection.get_untracked(),
            ) {
                ev.prevent_default();
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(child)));
            }
        }
        // --- Lens-local verbs: leave them for the focused lens to handle ---
        SkinVerb::Commit
        | SkinVerb::Fill
        | SkinVerb::Fold
        | SkinVerb::Unfold
        | SkinVerb::TraceForward
        | SkinVerb::TraceBack
        | SkinVerb::NameBox
        | SkinVerb::Leader
        | SkinVerb::Explain
        | SkinVerb::Escape => {}
    }
}

fn skin_id_for_slot(skin_ids: &[SkinId], slot: u8) -> Option<SkinId> {
    let index = (slot as usize).checked_sub(1)?;
    skin_ids.get(index).copied()
}

fn parent_of_selection(workspace: &WorkspaceState, selection: &SelectionState) -> Option<NodeId> {
    let id = selection.primary.as_ref()?;
    workspace.node(id)?.parent.clone()
}

fn first_child_of_selection(
    workspace: &WorkspaceState,
    selection: &SelectionState,
) -> Option<NodeId> {
    let id = selection.primary.as_ref()?;
    workspace.node(id)?.children.first().cloned()
}

fn adjacent_node_selection(
    workspace: &WorkspaceState,
    selection: &SelectionState,
    direction: isize,
) -> Option<NodeId> {
    if workspace.node_order.is_empty() {
        return None;
    }
    let current_index = selection
        .primary
        .as_ref()
        .and_then(|selected| {
            workspace
                .node_order
                .iter()
                .position(|candidate| candidate == selected)
        })
        .unwrap_or(0);
    let last_index = workspace.node_order.len().saturating_sub(1);
    let next_index = if direction < 0 {
        current_index.saturating_sub(1)
    } else {
        (current_index + 1).min(last_index)
    };
    workspace.node_order.get(next_index).cloned()
}

fn keyboard_target_is_text_entry(ev: &web_sys::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        || element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false")
}

#[component]
fn WorkspaceManagementControl(
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let shared_signal = shared.signal();
    let workspaces = Memo::new(move |_| shared_signal.with(|state| state.workspace_ids.clone()));
    let active_workspace = Memo::new(move |_| {
        shared_signal.with(|state| state.active_workspace_id.clone().unwrap_or_default())
    });
    let switch_dispatch = dispatch.clone();
    let new_dispatch = dispatch;

    view! {
        <div class="dtc-workspace-control" aria-label="Workspace management">
            <select
                class="dtc-workspace-control__select"
                prop:value=move || active_workspace.get()
                on:change=move |ev| {
                    let workspace_id = event_target_value(&ev);
                    if !workspace_id.is_empty() {
                        switch_dispatch.dispatch(WorkspaceIntent::SwitchWorkspace {
                            workspace_id,
                        });
                    }
                }
            >
                {move || workspaces.get()
                    .into_iter()
                    .map(|workspace_id| {
                        let label = workspace_id.clone();
                        view! {
                            <option value=workspace_id>{label}</option>
                        }
                    })
                    .collect::<Vec<_>>()
                }
            </select>
            <button
                type="button"
                class="dtc-workspace-control__new"
                title="Create a new workspace"
                on:click=move |_| {
                    new_dispatch.dispatch(WorkspaceIntent::NewWorkspace);
                }
            >
                <span>"New"</span>
                <kbd>"Ctrl N"</kbd>
            </button>
        </div>
    }
}

#[component]
fn RecalcModeControl(
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let shared_signal = shared.signal();
    let mode = Memo::new(move |_| shared_signal.with(|state| state.recalc_mode));
    let pending = Memo::new(move |_| shared_signal.with(|state| state.manual_recalc_pending));
    let auto_shared = shared;
    let manual_shared = shared;
    let calculate_shared = shared;

    view! {
        <div class="dtc-recalc-mode" aria-label="Recalculation mode">
            <button
                type="button"
                class:dtc-recalc-mode__button--active=move || mode.get() == WorkspaceRecalcMode::Auto
                title="Recalculate content edits immediately"
                on:click=move |_| {
                    auto_shared.update(|state| {
                        state.recalc_mode = WorkspaceRecalcMode::Auto;
                        state.manual_recalc_pending = false;
                    });
                }
            >
                "Auto"
            </button>
            <button
                type="button"
                class:dtc-recalc-mode__button--active=move || mode.get() == WorkspaceRecalcMode::Manual
                title="Defer content-edit recalculation until Calculate"
                on:click=move |_| {
                    manual_shared.update(|state| {
                        state.recalc_mode = WorkspaceRecalcMode::Manual;
                    });
                }
            >
                "Manual"
            </button>
            <button
                type="button"
                class:dtc-recalc-mode__calculate
                class:dtc-recalc-mode__calculate--pending=move || pending.get()
                title="Recalculate workspace now"
                on:click=move |_| {
                    dispatch.dispatch(WorkspaceIntent::Recalculate);
                    calculate_shared.update(|state| {
                        state.manual_recalc_pending = false;
                    });
                }
            >
                <span>{move || if pending.get() { "Calculate*" } else { "Calculate" }}</span>
                <kbd>"Ctrl Enter"</kbd>
            </button>
        </div>
    }
}

#[component]
fn SkinSwitcher(registry: Arc<SkinRegistry>, current: RwSignal<SkinId>) -> impl IntoView {
    let tabs: Vec<_> = registry
        .iter()
        .enumerate()
        .map(|(index, skin)| (skin.id(), skin.manifest().display_name, index + 1))
        .collect();

    view! {
        <nav class="dtc-skin-switcher" role="tablist">
            {tabs
                .into_iter()
                .map(|(id, name, shortcut)| {
                    let is_active = Memo::new(move |_| current.get() == id);
                    view! {
                        <button
                            class="dtc-skin-switcher__tab"
                            class:dtc-skin-switcher__tab--active=move || is_active.get()
                            role="tab"
                            aria-selected=move || if is_active.get() { "true" } else { "false" }
                            on:click=move |_| {
                                if current.get_untracked() != id {
                                    // Switching the active skin emits no
                                    // WorkspaceIntent and changes no host-owned
                                    // signal beyond `current_skin`, so the
                                    // selection signal (host-owned, not skin-
                                    // owned) survives and calculation is never
                                    // requested.
                                    current.set(id);
                                }
                            }
                        >
                            <span>{name}</span>
                            <kbd>{shortcut.to_string()}</kbd>
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </nav>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::WorkspaceState;

    #[test]
    fn skin_slot_maps_lens_numbers_to_ordered_ids() {
        let ids = [SkinId::new("one"), SkinId::new("two"), SkinId::new("three")];
        assert_eq!(skin_id_for_slot(&ids, 1), Some(SkinId::new("one")));
        assert_eq!(skin_id_for_slot(&ids, 3), Some(SkinId::new("three")));
        assert_eq!(skin_id_for_slot(&ids, 0), None);
        assert_eq!(skin_id_for_slot(&ids, 4), None);
    }

    #[test]
    fn adjacent_node_selection_moves_through_projected_order() {
        let workspace = WorkspaceState {
            node_order: vec![NodeId::new("A"), NodeId::new("B"), NodeId::new("C")],
            ..WorkspaceState::default()
        };
        let selection = SelectionState::with_primary(Some(NodeId::new("B")));

        assert_eq!(
            adjacent_node_selection(&workspace, &selection, -1),
            Some(NodeId::new("A"))
        );
        assert_eq!(
            adjacent_node_selection(&workspace, &selection, 1),
            Some(NodeId::new("C"))
        );
    }
}

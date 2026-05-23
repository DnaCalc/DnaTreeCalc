use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, ErasedSkinContext, SelectionState, SharedSkinStateHandle, SkinId, SkinRegistry,
    WorkspaceState,
};
use leptos::prelude::*;

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
/// Crucially, switching never calls the bridge: the dispatch handle the
/// new skin receives is the same `Arc<dyn Dispatcher>`, and no
/// `WorkspaceIntent` is emitted as part of the switch itself. Tests in
/// the host crate use a recording bridge to assert that the bridge call
/// count is unchanged across a switch.
#[component]
pub fn WorkspaceShell(
    workspace: ReadSignal<WorkspaceState>,
    selection: RwSignal<SelectionState>,
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
    registry: Arc<SkinRegistry>,
    initial_skin: SkinId,
) -> impl IntoView {
    let current_skin = RwSignal::new(initial_skin);
    let registry_for_view = registry.clone();
    let dispatch_for_view = dispatch.clone();

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

    view! {
        <style>{SHELL_CSS}</style>
        <div class="dtc-shell">
            <header class="dtc-context-strip">
                <span class="dtc-context-strip__title">{move || title.get()}</span>
                <span class="dtc-context-strip__profile">{move || profile.get()}</span>
                <span class="dtc-context-strip__spacer"></span>
                <SkinSwitcher
                    registry=registry_for_tabs
                    current=current_skin
                />
            </header>
            <main class="dtc-main-slot">
                {move || {
                    let id = current_skin.get();
                    let Some(registered) = registry_for_view.get(id) else {
                        return ().into_any();
                    };
                    let cx = ErasedSkinContext {
                        workspace,
                        selection: selection.read_only(),
                        shared,
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
                <span>"clean"</span>
            </footer>
        </div>
    }
}

#[component]
fn SkinSwitcher(registry: Arc<SkinRegistry>, current: RwSignal<SkinId>) -> impl IntoView {
    let tabs: Vec<_> = registry
        .iter()
        .map(|skin| (skin.id(), skin.manifest().display_name))
        .collect();

    view! {
        <nav class="dtc-skin-switcher" role="tablist">
            {tabs
                .into_iter()
                .map(|(id, name)| {
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
                                    // owned) survives and the bridge is never
                                    // called.
                                    current.set(id);
                                }
                            }
                        >
                            {name}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </nav>
    }
}

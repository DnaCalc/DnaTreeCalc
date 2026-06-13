use std::sync::Arc;

use dnatreecalc_skin_framework::{
    Dispatcher, ErasedSkinContext, KeyChord, KeybindingRegistry, NodeId, PersistedSkinStateRecord,
    Persona, PreviewService, SelectionState, SharedSkinStateHandle, SharedStateChange,
    SharedStateOrigin,
    SkinId, SkinMountSlot, SkinRegistry, SkinStatePersistenceKey, SkinStatePersistenceStore,
    SkinVerb, ThemeTokens, WorkspaceDelta, WorkspaceIntent, WorkspaceRecalcMode, WorkspaceState,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::theme::SHELL_CSS;

/// Which skin occupies each cockpit slot. Skins are referenced by their stable
/// id string so the layout persists and survives registry evolution (an
/// unregistered id simply renders the fail-loud fallback card).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CockpitLayout {
    pub main: String,
    pub inspector: Option<String>,
    pub console: Option<String>,
    /// Side-by-side main-class panes for running two lenses at once. Default
    /// (and `#[serde(default)]`) so a v1 persisted layout without these fields
    /// loads as the single-main composition it was.
    #[serde(default)]
    pub split_left: Option<String>,
    #[serde(default)]
    pub split_right: Option<String>,
}

impl CockpitLayout {
    #[must_use]
    pub fn solo(main: SkinId) -> Self {
        Self {
            main: main.as_str().to_string(),
            inspector: None,
            console: None,
            split_left: None,
            split_right: None,
        }
    }

    /// The companion slots this layout has occupied — what lenses read to
    /// stand their embedded copies down. Split panes are main-class lenses, so
    /// they are *not* reported here (a split lens runs standalone, it never
    /// stands another lens's embedded widgets down).
    #[must_use]
    pub fn companion_slots(&self) -> Vec<SkinMountSlot> {
        let mut slots = Vec::new();
        if self.inspector.is_some() {
            slots.push(SkinMountSlot::RightInspector);
        }
        if self.console.is_some() {
            slots.push(SkinMountSlot::BottomConsole);
        }
        slots
    }
}

/// A named cockpit composition. Preset application keeps only ids that are
/// actually registered, so a preset referencing an absent skin degrades to
/// the slots that exist instead of failing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CockpitPreset {
    pub name: &'static str,
    pub main: &'static str,
    pub inspector: Option<&'static str>,
    pub console: Option<&'static str>,
}

/// The built-in cockpit presets (`cockpit-preset-registry`, ATLAS W5).
#[must_use]
pub fn builtin_presets() -> Vec<CockpitPreset> {
    vec![
        CockpitPreset {
            name: "Solo",
            main: "flow",
            inspector: None,
            console: None,
        },
        CockpitPreset {
            name: "Modeling",
            main: "flow",
            inspector: Some("lens"),
            console: Some("console"),
        },
        CockpitPreset {
            name: "Author",
            main: "tree",
            inspector: Some("lens"),
            console: None,
        },
        CockpitPreset {
            name: "Audit",
            main: "ledger",
            inspector: Some("lens"),
            console: Some("console"),
        },
    ]
}

/// Resolve a registered skin by its stable id string.
fn resolve_skin<'r>(
    registry: &'r SkinRegistry,
    id: &str,
) -> Option<&'r dnatreecalc_skin_framework::RegisteredSkin> {
    registry.iter().find(|skin| skin.id().as_str() == id)
}

/// The companion slots whose configured skins can actually mount: registered
/// AND negotiating their slot successfully. Published as
/// `companion_slots_active` so lenses only stand their embedded widgets down
/// for companions that will really render (a fail-loud fallback card must not
/// blank the embedded copy too).
#[must_use]
pub fn validated_companion_slots(
    registry: &SkinRegistry,
    layout: &CockpitLayout,
) -> Vec<SkinMountSlot> {
    let mut slots = Vec::new();
    let mut check = |slot: SkinMountSlot, id: &Option<String>| {
        if let Some(id) = id
            && let Some(skin) = resolve_skin(registry, id)
            && skin.negotiate_slot(slot).is_ok()
        {
            slots.push(slot);
        }
    };
    check(SkinMountSlot::RightInspector, &layout.inspector);
    check(SkinMountSlot::BottomConsole, &layout.console);
    check(SkinMountSlot::SplitLeft, &layout.split_left);
    check(SkinMountSlot::SplitRight, &layout.split_right);
    slots
}

/// Build a layout from a preset, keeping the current main when the preset's
/// main is unregistered and dropping unregistered companions.
#[must_use]
pub fn layout_from_preset(
    registry: &SkinRegistry,
    preset: &CockpitPreset,
    current_main: &str,
) -> CockpitLayout {
    let main = if resolve_skin(registry, preset.main).is_some() {
        preset.main.to_string()
    } else {
        current_main.to_string()
    };
    let resolve_companion = |id: Option<&'static str>| {
        id.filter(|candidate| resolve_skin(registry, candidate).is_some())
            .map(str::to_string)
    };
    CockpitLayout {
        main,
        inspector: resolve_companion(preset.inspector),
        console: resolve_companion(preset.console),
        // Built-in presets do not open split panes; the user opens those
        // through the companion toggles or a saved user preset.
        split_left: None,
        split_right: None,
    }
}

const COCKPIT_LAYOUT_SCHEMA: u32 = 1;

fn cockpit_layout_key(workspace_id: &str) -> SkinStatePersistenceKey {
    SkinStatePersistenceKey::new(
        SkinId::new("shell-cockpit"),
        SkinMountSlot::Main,
        workspace_id,
    )
}

/// Load the persisted layout for a workspace; `None` on absence or any
/// store/decode failure (the shell falls back to solo).
fn load_cockpit_layout(
    store: &dyn SkinStatePersistenceStore,
    workspace_id: &str,
) -> Option<CockpitLayout> {
    let record = store.load(&cockpit_layout_key(workspace_id)).ok()??;
    if record.schema_version != COCKPIT_LAYOUT_SCHEMA {
        return None;
    }
    serde_json::from_value(record.value).ok()
}

fn save_cockpit_layout(
    store: &dyn SkinStatePersistenceStore,
    workspace_id: &str,
    layout: &CockpitLayout,
) {
    if let Ok(value) = serde_json::to_value(layout) {
        let record = PersistedSkinStateRecord::new(COCKPIT_LAYOUT_SCHEMA, value);
        let _ = store.save(&cockpit_layout_key(workspace_id), &record);
    }
}

/// A user-named cockpit composition saved on top of the built-ins. Unlike a
/// [`CockpitPreset`] (a `&'static` built-in) this owns a full
/// [`CockpitLayout`], so it round-trips the user's exact slot assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreset {
    pub name: String,
    pub layout: CockpitLayout,
}

const USER_PRESETS_SCHEMA: u32 = 1;

fn user_presets_key(workspace_id: &str) -> SkinStatePersistenceKey {
    SkinStatePersistenceKey::new(
        SkinId::new("shell-presets-user"),
        SkinMountSlot::Main,
        workspace_id,
    )
}

/// The user's saved presets for a workspace; empty on absence or any
/// store/decode failure (user presets are additive — a missing entry is "none
/// saved yet", never an error the shell surfaces).
fn load_user_presets(store: &dyn SkinStatePersistenceStore, workspace_id: &str) -> Vec<UserPreset> {
    let Ok(Some(record)) = store.load(&user_presets_key(workspace_id)) else {
        return Vec::new();
    };
    if record.schema_version != USER_PRESETS_SCHEMA {
        return Vec::new();
    }
    serde_json::from_value(record.value).unwrap_or_default()
}

fn save_user_presets(
    store: &dyn SkinStatePersistenceStore,
    workspace_id: &str,
    presets: &[UserPreset],
) {
    if let Ok(value) = serde_json::to_value(presets) {
        let record = PersistedSkinStateRecord::new(USER_PRESETS_SCHEMA, value);
        let _ = store.save(&user_presets_key(workspace_id), &record);
    }
}

/// Insert or replace a preset by name (case-sensitive), returning the updated
/// list. Replacing keeps the list position; a new name appends.
fn upsert_user_preset(mut presets: Vec<UserPreset>, preset: UserPreset) -> Vec<UserPreset> {
    if let Some(existing) = presets.iter_mut().find(|p| p.name == preset.name) {
        existing.layout = preset.layout;
    } else {
        presets.push(preset);
    }
    presets
}

/// Everything a slot mount needs from the shell; bundled so slot views stay
/// prop-light.
#[derive(Clone)]
struct SlotParts {
    workspace: ReadSignal<WorkspaceState>,
    latest_delta: ReadSignal<WorkspaceDelta>,
    selection: RwSignal<SelectionState>,
    shared: SharedSkinStateHandle,
    tokens: ThemeTokens,
    skin_state_store: Arc<dyn SkinStatePersistenceStore>,
    dispatch: Arc<dyn Dispatcher>,
    registry: Arc<SkinRegistry>,
    preview: Option<Arc<dyn PreviewService>>,
}

/// The cockpit workspace shell.
///
/// Phase B.1 of the ATLAS suite: a multi-slot composition over one
/// workspace/selection/shared-state source of truth. The main slot carries a
/// primary lens; the right-inspector and bottom-console slots carry companion
/// skins, negotiated against each skin's capability manifest at mount. The
/// layout persists per workspace; clicking a slot focuses it (recorded as
/// audited shared state); companion occupancy is published so lenses stand
/// their embedded copies down.
///
/// Switching any slot's skin emits no `WorkspaceIntent` and never
/// recalculates — host-owned signals survive across mounts.
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
    #[prop(optional_no_strip)] preview: Option<Arc<dyn PreviewService>>,
) -> impl IntoView {
    let initial_workspace_id = workspace.get_untracked().workspace_id;
    let layout = RwSignal::new(
        load_cockpit_layout(skin_state_store.as_ref(), &initial_workspace_id)
            .unwrap_or_else(|| CockpitLayout::solo(initial_skin)),
    );
    let user_presets = RwSignal::new(load_user_presets(
        skin_state_store.as_ref(),
        &initial_workspace_id,
    ));
    let shortcut_skin_ids = registry.ids();

    let parts = SlotParts {
        workspace,
        latest_delta,
        selection,
        shared,
        tokens: tokens.clone(),
        skin_state_store: skin_state_store.clone(),
        dispatch: dispatch.clone(),
        registry: registry.clone(),
        preview,
    };

    // The workspace the current layout belongs to — saves are keyed by this,
    // never by whatever workspace happens to be active at save time, so a
    // workspace switch can never clobber another workspace's persisted layout.
    let layout_owner = StoredValue::new(initial_workspace_id.clone());
    let workspace_id = Memo::new(move |_| workspace.with(|ws| ws.workspace_id.clone()));

    // Reload the persisted layout when the active workspace changes. A
    // workspace with no persisted layout adopts the current composition
    // (saved under its own key by the sync effect below).
    let reload_store = skin_state_store.clone();
    Effect::new(move |_| {
        let id = workspace_id.get();
        if layout_owner.get_value() != id {
            let next = load_cockpit_layout(reload_store.as_ref(), &id)
                .unwrap_or_else(|| layout.get_untracked());
            user_presets.set(load_user_presets(reload_store.as_ref(), &id));
            layout_owner.set_value(id);
            layout.set(next);
        }
    });

    // Keep companion occupancy, focus sanity, and persistence in sync with
    // the layout. Occupancy is VALIDATED (registered + slot-negotiated) so a
    // fallback card never stands embedded widgets down, and a focused slot
    // that no longer exists snaps back to Main.
    let sync_store = skin_state_store.clone();
    let sync_registry = registry.clone();
    Effect::new(move |_| {
        let current = layout.get();
        let active = validated_companion_slots(&sync_registry, &current);
        shared.apply(
            SharedStateChange::SetCompanionSlots(active.clone()),
            SharedStateOrigin::Shell,
        );
        if let Some(focused) = shared.get_untracked().focused_slot
            && focused != SkinMountSlot::Main
            && !active.contains(&focused)
        {
            shared.apply(
                SharedStateChange::SetFocusedSlot(Some(SkinMountSlot::Main)),
                SharedStateOrigin::Shell,
            );
        }
        save_cockpit_layout(sync_store.as_ref(), &layout_owner.get_value(), &current);
    });

    let title = Memo::new(move |_| {
        workspace.with(|ws| {
            if ws.workspace_id.is_empty() {
                "Untitled workspace".to_string()
            } else {
                ws.workspace_id.clone()
            }
        })
    });
    let profile = Memo::new(move |_| workspace.with(|ws| ws.profile.clone()));
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
    let registry_for_presets = registry.clone();
    let presets_store = skin_state_store.clone();
    let shortcut_dispatch = dispatch.clone();
    let shell_css = format!("{}\n{}\n{}", tokens.css_rule(".dtc-shell"), SHELL_CSS, COCKPIT_CSS);

    let split_left_parts = parts.clone();
    let split_right_parts = parts.clone();
    let inspector_parts = parts.clone();
    let console_parts = parts.clone();
    let main_parts = parts;

    let has_split_left = Memo::new(move |_| layout.with(|l| l.split_left.is_some()));
    let has_split_right = Memo::new(move |_| layout.with(|l| l.split_right.is_some()));
    let has_inspector = Memo::new(move |_| layout.with(|l| l.inspector.is_some()));
    let has_console = Memo::new(move |_| layout.with(|l| l.console.is_some()));

    view! {
        <style>{shell_css}</style>
        <div
            class="dtc-shell"
            tabindex="0"
            on:keydown=move |ev| {
                // Build the active grammar from the user's persisted overrides
                // each keydown (cheap; ~28 entries). A persisted map that fails
                // validation — e.g. a stale verb from another build — falls back
                // to the universal table rather than dropping the keystroke.
                let keybindings = shared
                    .signal()
                    .with_untracked(|state| {
                        KeybindingRegistry::with_overrides(&state.keybinding_overrides)
                    })
                    .unwrap_or_else(|_| KeybindingRegistry::universal());
                handle_shell_keydown(
                    ev,
                    workspace,
                    selection,
                    layout,
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
                <PresetPicker
                    registry=registry_for_presets
                    layout=layout
                    store=presets_store
                    workspace_id=workspace_id
                    user_presets=user_presets
                />
                <span class="dtc-context-strip__spacer"></span>
                <SkinSwitcher
                    registry=registry_for_tabs
                    layout=layout
                />
            </header>
            <ShortcutBar />
            <div class="dtc-cockpit" class:dtc-cockpit--with-inspector=move || has_inspector.get()>
                {move || {
                    has_split_left.get().then(|| {
                        let parts = split_left_parts.clone();
                        view! {
                            <aside class="dtc-cockpit__split dtc-cockpit__split--left">
                                <CompanionHeader
                                    registry=parts.registry.clone()
                                    layout=layout
                                    mount_slot=SkinMountSlot::SplitLeft
                                />
                                <SlotView
                                    parts=parts
                                    layout=layout
                                    mount_slot=SkinMountSlot::SplitLeft
                                />
                            </aside>
                        }
                        .into_any()
                    })
                }}
                <main class="dtc-cockpit__main">
                    <SlotView
                        parts=main_parts
                        layout=layout
                        mount_slot=SkinMountSlot::Main
                    />
                </main>
                {move || {
                    has_split_right.get().then(|| {
                        let parts = split_right_parts.clone();
                        view! {
                            <aside class="dtc-cockpit__split dtc-cockpit__split--right">
                                <CompanionHeader
                                    registry=parts.registry.clone()
                                    layout=layout
                                    mount_slot=SkinMountSlot::SplitRight
                                />
                                <SlotView
                                    parts=parts
                                    layout=layout
                                    mount_slot=SkinMountSlot::SplitRight
                                />
                            </aside>
                        }
                        .into_any()
                    })
                }}
                {move || {
                    has_inspector.get().then(|| {
                        let parts = inspector_parts.clone();
                        view! {
                            <aside class="dtc-cockpit__inspector">
                                <CompanionHeader
                                    registry=parts.registry.clone()
                                    layout=layout
                                    mount_slot=SkinMountSlot::RightInspector
                                />
                                <SlotView
                                    parts=parts
                                    layout=layout
                                    mount_slot=SkinMountSlot::RightInspector
                                />
                            </aside>
                        }
                        .into_any()
                    })
                }}
            </div>
            {move || {
                has_console.get().then(|| {
                    let parts = console_parts.clone();
                    view! {
                        <div class="dtc-cockpit__console">
                            <CompanionHeader
                                registry=parts.registry.clone()
                                layout=layout
                                mount_slot=SkinMountSlot::BottomConsole
                            />
                            <SlotView
                                parts=parts
                                layout=layout
                                mount_slot=SkinMountSlot::BottomConsole
                            />
                        </div>
                    }
                    .into_any()
                })
            }}
            <footer class="dtc-status-foot">
                <span>{move || format!("nodes: {}", node_count.get())}</span>
                <span>{move || format!("selected: {}", selected_label.get())}</span>
                <span class="dtc-context-strip__spacer"></span>
                <CompanionToggles registry=registry.clone() layout=layout />
                <PersonaControl shared=shared dispatch=dispatch.clone() />
                <RecalcModeControl
                    shared=shared
                    dispatch=dispatch.clone()
                />
                <span class="dtc-status-pill">"clean"</span>
            </footer>
        </div>
    }
}

/// The skin id string a layout assigns to a slot.
fn layout_slot_id(layout: &CockpitLayout, slot: SkinMountSlot) -> Option<String> {
    match slot {
        SkinMountSlot::Main => Some(layout.main.clone()),
        SkinMountSlot::RightInspector => layout.inspector.clone(),
        SkinMountSlot::BottomConsole => layout.console.clone(),
        SkinMountSlot::SplitLeft => layout.split_left.clone(),
        SkinMountSlot::SplitRight => layout.split_right.clone(),
    }
}

/// Mount one slot: resolve the configured skin, negotiate the slot against its
/// capability manifest, and render either the skin or a fail-loud fallback.
/// Clicking anywhere in the slot records it as the focused slot (audited).
#[component]
fn SlotView(parts: SlotParts, layout: RwSignal<CockpitLayout>, mount_slot: SkinMountSlot) -> impl IntoView {
    let shared = parts.shared;
    let slot_id = Memo::new(move |_| layout.with(|l| layout_slot_id(l, mount_slot)));
    let focused = Memo::new(move |_| {
        shared
            .signal()
            .with(|s| s.focused_slot == Some(mount_slot))
    });

    view! {
        <div
            class="dtc-slot"
            class:dtc-slot--focused=move || focused.get()
            on:mousedown=move |_| {
                if shared.get_untracked().focused_slot != Some(mount_slot) {
                    shared.apply(
                        SharedStateChange::SetFocusedSlot(Some(mount_slot)),
                        SharedStateOrigin::Shell,
                    );
                }
            }
        >
            {move || {
                let Some(id) = slot_id.get() else {
                    return ().into_any();
                };
                let Some(registered) = resolve_skin(&parts.registry, &id).cloned() else {
                    return slot_fallback(format!("skin '{id}' is not registered")).into_any();
                };
                if let Err(error) = registered.negotiate_slot(mount_slot) {
                    return slot_fallback(error.to_string()).into_any();
                }
                let cx = ErasedSkinContext {
                    workspace: parts.workspace,
                    latest_delta: parts.latest_delta,
                    selection: parts.selection.read_only(),
                    shared: parts.shared,
                    tokens: parts.tokens.clone(),
                    slot: mount_slot,
                    skin_state_store: parts.skin_state_store.clone(),
                    dispatch: parts.dispatch.clone(),
                    preview: parts.preview.clone(),
                };
                let handle = registered.mount(cx);
                if let Some(hook) = handle.on_deactivate {
                    on_cleanup(hook);
                }
                handle.view
            }}
        </div>
    }
}

/// Fail-loud slot content for negotiation refusals / unknown ids.
fn slot_fallback(message: String) -> impl IntoView {
    view! {
        <div class="dtc-slot-fallback" role="alert">
            <span class="dtc-slot-fallback__title">"Slot unavailable"</span>
            <span class="dtc-slot-fallback__detail">{message}</span>
        </div>
    }
}

/// Companion slot chrome: which skin is mounted + picker + close.
#[component]
fn CompanionHeader(
    registry: Arc<SkinRegistry>,
    layout: RwSignal<CockpitLayout>,
    mount_slot: SkinMountSlot,
) -> impl IntoView {
    let options: Vec<(String, &'static str)> = registry
        .iter()
        .filter(|skin| skin.capabilities().slot_allowed(mount_slot))
        .map(|skin| (skin.id().as_str().to_string(), skin.manifest().display_name))
        .collect();
    let current = Memo::new(move |_| layout.with(|l| layout_slot_id(l, mount_slot).unwrap_or_default()));

    view! {
        <div class="dtc-companion-header">
            <select
                class="dtc-companion-header__picker"
                prop:value=move || current.get()
                on:change=move |ev| {
                    let id = event_target_value(&ev);
                    layout.update(|l| set_layout_slot(l, mount_slot, Some(id)));
                }
            >
                {options
                    .into_iter()
                    .map(|(id, name)| view! { <option value=id>{name}</option> })
                    .collect::<Vec<_>>()}
            </select>
            <button
                type="button"
                class="dtc-companion-header__close"
                title="Close this slot"
                on:click=move |_| layout.update(|l| set_layout_slot(l, mount_slot, None))
            >
                "✕"
            </button>
        </div>
    }
}

fn set_layout_slot(layout: &mut CockpitLayout, slot: SkinMountSlot, id: Option<String>) {
    match slot {
        SkinMountSlot::Main => {
            if let Some(id) = id {
                layout.main = id;
            }
        }
        SkinMountSlot::RightInspector => layout.inspector = id,
        SkinMountSlot::BottomConsole => layout.console = id,
        SkinMountSlot::SplitLeft => layout.split_left = id,
        SkinMountSlot::SplitRight => layout.split_right = id,
    }
}

/// Footer toggles to open/close the companion slots without a preset.
#[component]
fn CompanionToggles(registry: Arc<SkinRegistry>, layout: RwSignal<CockpitLayout>) -> impl IntoView {
    let default_inspector = registry
        .iter()
        .find(|skin| skin.capabilities().slot_allowed(SkinMountSlot::RightInspector))
        .map(|skin| skin.id().as_str().to_string());
    let default_console = registry
        .iter()
        .find(|skin| skin.capabilities().slot_allowed(SkinMountSlot::BottomConsole))
        .map(|skin| skin.id().as_str().to_string());
    let inspector_open = Memo::new(move |_| layout.with(|l| l.inspector.is_some()));
    let console_open = Memo::new(move |_| layout.with(|l| l.console.is_some()));
    let split_left_open = Memo::new(move |_| layout.with(|l| l.split_left.is_some()));
    let split_right_open = Memo::new(move |_| layout.with(|l| l.split_right.is_some()));
    let inspector_available = default_inspector.is_some();
    let console_available = default_console.is_some();

    view! {
        <div class="dtc-companion-toggles" aria-label="Companion slots">
            <button
                type="button"
                class:dtc-companion-toggles__button--active=move || inspector_open.get()
                disabled=!inspector_available
                title="Toggle the Lens companion slot"
                on:click=move |_| {
                    let default_inspector = default_inspector.clone();
                    layout.update(|l| {
                        l.inspector = if l.inspector.is_some() { None } else { default_inspector };
                    });
                }
            >
                "Lens"
            </button>
            <button
                type="button"
                class:dtc-companion-toggles__button--active=move || console_open.get()
                disabled=!console_available
                title="Toggle the Console companion slot"
                on:click=move |_| {
                    let default_console = default_console.clone();
                    layout.update(|l| {
                        l.console = if l.console.is_some() { None } else { default_console };
                    });
                }
            >
                "Console"
            </button>
            <button
                type="button"
                class:dtc-companion-toggles__button--active=move || split_left_open.get()
                title="Toggle a side-by-side lens on the left"
                on:click=move |_| {
                    // Open with a second copy of the main lens; the split's own
                    // picker repoints it. Main-class skins always negotiate.
                    layout.update(|l| {
                        l.split_left =
                            if l.split_left.is_some() { None } else { Some(l.main.clone()) };
                    });
                }
            >
                "Split ◧"
            </button>
            <button
                type="button"
                class:dtc-companion-toggles__button--active=move || split_right_open.get()
                title="Toggle a side-by-side lens on the right"
                on:click=move |_| {
                    layout.update(|l| {
                        l.split_right =
                            if l.split_right.is_some() { None } else { Some(l.main.clone()) };
                    });
                }
            >
                "Split ◨"
            </button>
        </div>
    }
}

/// Named cockpit compositions: the built-in presets plus the user's saved
/// layouts, with an inline "save current as…" control. Built-ins rebuild
/// through `layout_from_preset` (degrading absent skins); a saved preset
/// restores the user's exact slot assignments verbatim.
#[component]
fn PresetPicker(
    registry: Arc<SkinRegistry>,
    layout: RwSignal<CockpitLayout>,
    store: Arc<dyn SkinStatePersistenceStore>,
    workspace_id: Memo<String>,
    user_presets: RwSignal<Vec<UserPreset>>,
) -> impl IntoView {
    let builtin_names: Vec<&'static str> = builtin_presets().iter().map(|p| p.name).collect();
    let save_name = RwSignal::new(String::new());

    view! {
        <div class="dtc-preset-picker" aria-label="Cockpit presets">
            <select
                class="dtc-preset-picker__select"
                prop:value=""
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    if let Some(name) = value.strip_prefix("user:") {
                        // A saved preset owns a full layout — restore it verbatim.
                        if let Some(preset) = user_presets
                            .with_untracked(|presets| presets.iter().find(|p| p.name == name).cloned())
                        {
                            layout.set(preset.layout);
                        }
                    } else if let Some(preset) =
                        builtin_presets().into_iter().find(|p| p.name == value)
                    {
                        let current_main = layout.with_untracked(|l| l.main.clone());
                        layout.set(layout_from_preset(&registry, &preset, &current_main));
                    }
                    // Snap back to the placeholder so the same preset can be
                    // re-applied (a select holding its last value fires no
                    // change event for a repeat pick).
                    if let Some(select) = ev
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                    {
                        select.set_value("");
                    }
                }
            >
                <option value="" disabled=true selected=true>"Cockpit…"</option>
                {builtin_names
                    .into_iter()
                    .map(|name| view! { <option value=name>{name}</option> })
                    .collect::<Vec<_>>()}
                {move || {
                    let presets = user_presets.get();
                    (!presets.is_empty()).then(|| {
                        view! {
                            <optgroup label="Saved">
                                {presets
                                    .into_iter()
                                    .map(|preset| {
                                        let value = format!("user:{}", preset.name);
                                        view! { <option value=value>{preset.name}</option> }
                                    })
                                    .collect::<Vec<_>>()}
                            </optgroup>
                        }
                    })
                }}
            </select>
            <input
                class="dtc-preset-picker__name"
                placeholder="Save as…"
                prop:value=move || save_name.get()
                on:input=move |ev| save_name.set(event_target_value(&ev))
            />
            <button
                type="button"
                class="dtc-preset-picker__save"
                title="Save the current cockpit composition as a preset"
                prop:disabled=move || save_name.with(|name| name.trim().is_empty())
                on:click=move |_| {
                    let name = save_name.get_untracked().trim().to_string();
                    if name.is_empty() {
                        return;
                    }
                    let updated = upsert_user_preset(
                        user_presets.get_untracked(),
                        UserPreset {
                            name,
                            layout: layout.get_untracked(),
                        },
                    );
                    save_user_presets(store.as_ref(), &workspace_id.get_untracked(), &updated);
                    user_presets.set(updated);
                    save_name.set(String::new());
                }
            >
                "Save"
            </button>
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
    layout: RwSignal<CockpitLayout>,
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

    // While the user is typing, honor only modified global verbs and function
    // keys (F9 must recalculate from anywhere, exactly like Excel) — other
    // bare-key grammar (Commit, Fold, NameBox, Explain, arrows, …) belongs to
    // the edit buffer, not the shell.
    if keyboard_target_is_text_entry(&ev) && !command && !is_function_key(&chord.key) {
        return;
    }

    match verb {
        // --- Global verbs the shell owns ---
        SkinVerb::Recalculate => {
            ev.prevent_default();
            dispatch.dispatch(WorkspaceIntent::Recalculate);
            shared.apply(
                SharedStateChange::SetManualRecalcPending(false),
                SharedStateOrigin::Shell,
            );
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
                layout.update(|l| l.main = id.as_str().to_string());
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

/// `F1`..`F24` style keys — exempt from the typing suppression so F9
/// recalculates from inside any edit buffer.
fn is_function_key(key: &str) -> bool {
    key.len() >= 2
        && key.starts_with('F')
        && key[1..].chars().all(|ch| ch.is_ascii_digit())
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
                    auto_shared.apply(
                        SharedStateChange::SetRecalcMode(WorkspaceRecalcMode::Auto),
                        SharedStateOrigin::Shell,
                    );
                    auto_shared.apply(
                        SharedStateChange::SetManualRecalcPending(false),
                        SharedStateOrigin::Shell,
                    );
                }
            >
                "Auto"
            </button>
            <button
                type="button"
                class:dtc-recalc-mode__button--active=move || mode.get() == WorkspaceRecalcMode::Manual
                title="Defer content-edit recalculation until Calculate"
                on:click=move |_| {
                    manual_shared.apply(
                        SharedStateChange::SetRecalcMode(WorkspaceRecalcMode::Manual),
                        SharedStateOrigin::Shell,
                    );
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
                    calculate_shared.apply(
                        SharedStateChange::SetManualRecalcPending(false),
                        SharedStateOrigin::Shell,
                    );
                }
            >
                <span>{move || if pending.get() { "Calculate*" } else { "Calculate" }}</span>
                <kbd>"Ctrl Enter"</kbd>
            </button>
        </div>
    }
}

/// Governing-persona selector. Persona switching travels as an intent so it
/// is logged, receipted, and reflected into audited shared state by the host.
#[component]
fn PersonaControl(shared: SharedSkinStateHandle, dispatch: Arc<dyn Dispatcher>) -> impl IntoView {
    let shared_signal = shared.signal();
    let current = Memo::new(move |_| shared_signal.with(|s| s.persona.stable_id().to_string()));
    let personas = [Persona::Author, Persona::Reviewer, Persona::ReadOnly];

    view! {
        <select
            class="dtc-persona-control"
            aria-label="Governing persona"
            prop:value=move || current.get()
            on:change=move |ev| {
                let id = event_target_value(&ev);
                if let Some(persona) = [Persona::Author, Persona::Reviewer, Persona::ReadOnly]
                    .into_iter()
                    .find(|p| p.stable_id() == id)
                {
                    dispatch.dispatch(WorkspaceIntent::SetPersona { persona });
                }
            }
        >
            {personas
                .into_iter()
                .map(|p| view! { <option value=p.stable_id()>{p.stable_id()}</option> })
                .collect::<Vec<_>>()}
        </select>
    }
}

#[component]
fn SkinSwitcher(registry: Arc<SkinRegistry>, layout: RwSignal<CockpitLayout>) -> impl IntoView {
    // Only Main-capable skins appear as primary tabs; companions are composed
    // through presets and the companion toggles instead.
    let tabs: Vec<_> = registry
        .iter()
        .enumerate()
        .filter(|(_, skin)| skin.capabilities().slot_allowed(SkinMountSlot::Main))
        .map(|(index, skin)| (skin.id(), skin.manifest().display_name, index + 1))
        .collect();

    view! {
        <nav class="dtc-skin-switcher" role="tablist">
            {tabs
                .into_iter()
                .map(|(id, name, shortcut)| {
                    let is_active =
                        Memo::new(move |_| layout.with(|l| l.main == id.as_str()));
                    view! {
                        <button
                            class="dtc-skin-switcher__tab"
                            class:dtc-skin-switcher__tab--active=move || is_active.get()
                            role="tab"
                            aria-selected=move || if is_active.get() { "true" } else { "false" }
                            on:click=move |_| {
                                // Switching the main lens emits no
                                // WorkspaceIntent and changes no host-owned
                                // signal beyond the layout, so selection
                                // survives and calculation is never requested.
                                layout.update(|l| l.main = id.as_str().to_string());
                            }
                        >
                            <span>{name}</span>
                            // Only the first nine slots have Ctrl+N bindings.
                            {(shortcut <= 9).then(|| view! { <kbd>{shortcut.to_string()}</kbd> })}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </nav>
    }
}

const COCKPIT_CSS: &str = r#"
.dtc-cockpit { display: flex; flex: 1; min-height: 0; }
.dtc-cockpit__main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.dtc-cockpit__main > .dtc-slot { flex: 1; min-height: 0; }
.dtc-cockpit__inspector {
  width: 340px; flex-shrink: 0; display: flex; flex-direction: column;
  border-left: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-panel);
}
.dtc-cockpit__inspector > .dtc-slot { flex: 1; min-height: 0; overflow: auto; }
.dtc-cockpit__console { border-top: 1px solid var(--dtc-border-muted); }

/* Split panes are main-class: they share width with the main lens (equal
   thirds when both are open) rather than sitting as a narrow sidebar. */
.dtc-cockpit__split {
  flex: 1; min-width: 0; display: flex; flex-direction: column;
  background: var(--dtc-surface-panel);
}
.dtc-cockpit__split--left { border-right: 1px solid var(--dtc-border-muted); }
.dtc-cockpit__split--right { border-left: 1px solid var(--dtc-border-muted); }
.dtc-cockpit__split > .dtc-slot { flex: 1; min-height: 0; overflow: auto; }

.dtc-preset-picker { display: inline-flex; gap: 4px; align-items: center; }
.dtc-preset-picker__name { width: 7rem; min-width: 0; }

.dtc-slot { position: relative; height: 100%; }
.dtc-slot--focused { box-shadow: inset 0 0 0 2px var(--dtc-accent-border); }

.dtc-slot-fallback {
  display: flex; flex-direction: column; gap: 4px; padding: 16px;
  color: var(--dtc-danger-text); background: var(--dtc-danger-surface);
  border: 1px dashed var(--dtc-danger-border); border-radius: 6px; margin: 12px;
}
.dtc-slot-fallback__title { font-weight: 700; }

.dtc-companion-header {
  display: flex; align-items: center; gap: 6px; padding: 4px 8px;
  border-bottom: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-subtle);
}
.dtc-companion-header__picker { flex: 1; font-size: 12px; }
.dtc-companion-header__close {
  border: 1px solid var(--dtc-border); border-radius: 4px; background: var(--dtc-surface-panel);
  cursor: pointer; color: var(--dtc-text-subtle); padding: 0 6px;
}

.dtc-companion-toggles { display: inline-flex; gap: 4px; }
.dtc-companion-toggles button {
  border: 1px solid var(--dtc-border); border-radius: 5px; background: var(--dtc-surface-panel);
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-companion-toggles__button--active { background: var(--dtc-accent-surface); border-color: var(--dtc-accent-border); }

.dtc-preset-picker__select { font-size: 12px; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{InMemorySkinStatePersistenceStore, WorkspaceState};

    #[test]
    fn function_keys_are_exempt_from_typing_suppression() {
        assert!(is_function_key("F9"));
        assert!(is_function_key("F12"));
        assert!(!is_function_key("F"));
        assert!(!is_function_key("Enter"));
        assert!(!is_function_key("e"));
        // 'Fn' style or alphabetic suffixes are not function keys.
        assert!(!is_function_key("Fn"));
    }

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

    #[test]
    fn cockpit_layout_reports_companion_slots() {
        let mut layout = CockpitLayout::solo(SkinId::new("flow"));
        assert!(layout.companion_slots().is_empty());
        layout.inspector = Some("lens".to_string());
        layout.console = Some("console".to_string());
        assert_eq!(
            layout.companion_slots(),
            vec![SkinMountSlot::RightInspector, SkinMountSlot::BottomConsole]
        );
    }

    #[test]
    fn cockpit_layout_persists_per_workspace() {
        let store = InMemorySkinStatePersistenceStore::new();
        let layout = CockpitLayout {
            main: "flow".to_string(),
            inspector: Some("lens".to_string()),
            console: None,
            split_left: None,
            split_right: None,
        };
        save_cockpit_layout(&store, "workspace:a", &layout);

        assert_eq!(load_cockpit_layout(&store, "workspace:a"), Some(layout));
        assert_eq!(load_cockpit_layout(&store, "workspace:b"), None);
    }

    #[test]
    fn validated_companion_slots_exclude_unmountable_ids() {
        // Empty registry: occupied slots whose skins can't resolve are NOT
        // published as active, so embedded widgets don't stand down for a
        // fallback card.
        let registry = SkinRegistry::new();
        let layout = CockpitLayout {
            main: "flow".to_string(),
            inspector: Some("lens".to_string()),
            console: Some("missing".to_string()),
            split_left: None,
            split_right: None,
        };
        assert!(validated_companion_slots(&registry, &layout).is_empty());
    }

    #[test]
    fn split_slots_route_through_the_layout_and_stay_main_class() {
        let mut layout = CockpitLayout::solo(SkinId::new("flow"));
        // Splits are main-class, not companions — opening one does not stand
        // embedded widgets down.
        set_layout_slot(&mut layout, SkinMountSlot::SplitLeft, Some("tree".to_string()));
        set_layout_slot(
            &mut layout,
            SkinMountSlot::SplitRight,
            Some("ledger".to_string()),
        );
        assert_eq!(layout.split_left.as_deref(), Some("tree"));
        assert_eq!(
            layout_slot_id(&layout, SkinMountSlot::SplitRight).as_deref(),
            Some("ledger")
        );
        assert!(
            layout.companion_slots().is_empty(),
            "split panes are main-class, never companion slots"
        );

        set_layout_slot(&mut layout, SkinMountSlot::SplitLeft, None);
        assert_eq!(layout.split_left, None);
    }

    #[test]
    fn user_presets_round_trip_and_upsert_by_name() {
        let store = InMemorySkinStatePersistenceStore::new();
        assert!(load_user_presets(&store, "workspace:a").is_empty());

        let modeling = UserPreset {
            name: "My Modeling".to_string(),
            layout: CockpitLayout {
                main: "flow".to_string(),
                inspector: Some("lens".to_string()),
                console: None,
                split_left: Some("tree".to_string()),
                split_right: None,
            },
        };
        let presets = upsert_user_preset(Vec::new(), modeling.clone());
        save_user_presets(&store, "workspace:a", &presets);
        assert_eq!(load_user_presets(&store, "workspace:a"), vec![modeling]);

        // Same name replaces in place; a new name appends.
        let modeling_v2 = UserPreset {
            name: "My Modeling".to_string(),
            layout: CockpitLayout::solo(SkinId::new("ledger")),
        };
        let audit = UserPreset {
            name: "Audit".to_string(),
            layout: CockpitLayout::solo(SkinId::new("ledger")),
        };
        let updated =
            upsert_user_preset(upsert_user_preset(presets, modeling_v2.clone()), audit.clone());
        assert_eq!(updated, vec![modeling_v2, audit]);

        // Presets are scoped per workspace.
        assert!(load_user_presets(&store, "workspace:b").is_empty());
    }

    #[test]
    fn v1_layout_without_split_fields_loads_as_single_main() {
        // A layout persisted before split panes existed still deserializes —
        // the new fields default to None (the composition it was).
        let store = InMemorySkinStatePersistenceStore::new();
        let v1 = serde_json::json!({ "main": "flow", "inspector": null, "console": null });
        store
            .save(
                &cockpit_layout_key("workspace:legacy"),
                &PersistedSkinStateRecord::new(COCKPIT_LAYOUT_SCHEMA, v1),
            )
            .expect("seed legacy layout");

        let loaded = load_cockpit_layout(&store, "workspace:legacy").expect("loads v1 layout");
        assert_eq!(loaded.main, "flow");
        assert_eq!(loaded.split_left, None);
        assert_eq!(loaded.split_right, None);
    }

    #[test]
    fn preset_application_degrades_to_registered_skins() {
        // An empty registry: preset main is unregistered, companions dropped.
        let registry = SkinRegistry::new();
        let preset = CockpitPreset {
            name: "Modeling",
            main: "flow",
            inspector: Some("lens"),
            console: Some("console"),
        };
        let layout = layout_from_preset(&registry, &preset, "tree");
        assert_eq!(layout.main, "tree");
        assert_eq!(layout.inspector, None);
        assert_eq!(layout.console, None);
    }
}

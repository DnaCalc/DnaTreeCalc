//! Integration tests for the W005 walking-skeleton shell + skin framework
//! (bead `dtc-osq.5`).
//!
//! These tests prove the bead's load-bearing invariants without depending
//! on a DOM runtime:
//!
//! - **UX-SK-001:** the default registry contains the two skeleton skins
//!   (`triple-editor` and `outline-table`); each can be mounted through
//!   the registry's erased factory.
//! - **UX-SK-003:** mounting either skin — including a "switch" (mounting
//!   one then mounting the other against the same workspace and selection
//!   signals) — never engages the bridge. Proved against a
//!   `RecordingBridge` that counts `execute_recalc` calls.
//! - **UX-TR-004:** a `SelectNode` dispatched through the host dispatcher
//!   updates the shared selection signal that every mounted skin
//!   observes, and selection survives a skin switch.
//!
//! DOM-level click-through evidence is dtc-osq.8's responsibility; this
//! file proves the architecture's invariants at the framework + host
//! level.

use std::sync::Arc;

use dnatreecalc_host::app::{
    HostDispatcher, build_default_registry, preview_accounts_workspace_state,
};
use dnatreecalc_host::test_support::RecordingBridge;
use dnatreecalc_skin_framework::{
    Dispatcher, ErasedSkinContext, NodeId, SelectionState, SharedSkinState, SharedSkinStateHandle,
    WorkspaceIntent,
};
use dnatreecalc_skins::{OUTLINE_TABLE_ID, TRIPLE_EDITOR_ID};
use leptos::prelude::*;

#[test]
fn default_registry_ships_triple_editor_and_outline_table() {
    let registry = build_default_registry();
    assert_eq!(registry.len(), 2);

    let ids = registry.ids();
    assert!(ids.contains(&TRIPLE_EDITOR_ID));
    assert!(ids.contains(&OUTLINE_TABLE_ID));

    let triple = registry
        .get(TRIPLE_EDITOR_ID)
        .expect("triple-editor must be registered");
    assert_eq!(triple.manifest().display_name, "Triple editor");

    let outline = registry
        .get(OUTLINE_TABLE_ID)
        .expect("outline-table must be registered");
    assert_eq!(outline.manifest().display_name, "Outline table");
}

#[test]
fn select_node_intent_updates_selection_without_calling_the_bridge() {
    let _owner = Owner::new();
    let selection = RwSignal::new(SelectionState::default());

    let recording_bridge = Arc::new(RecordingBridge::new());
    let dispatcher = HostDispatcher::new(selection, Some(recording_bridge.clone()));

    let receipt = dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new("Accounts"))));
    assert!(receipt.accepted);

    assert_eq!(
        selection
            .get_untracked()
            .primary
            .as_ref()
            .map(NodeId::as_str),
        Some("Accounts")
    );
    assert_eq!(
        recording_bridge.recalc_count(),
        0,
        "SelectNode must never call the bridge"
    );

    let log = dispatcher.intents();
    assert_eq!(log.len(), 1);
}

#[test]
fn mounting_and_switching_skins_never_calls_the_bridge() {
    let _owner = Owner::new();

    let workspace_state = preview_accounts_workspace_state();
    let workspace = RwSignal::new(workspace_state);
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    let recording_bridge = Arc::new(RecordingBridge::new());
    let dispatcher = Arc::new(HostDispatcher::new(
        selection,
        Some(recording_bridge.clone()),
    ));
    let dispatch: Arc<dyn Dispatcher> = dispatcher.clone();

    // Simulate the click that selected a node in the nav rail — this is
    // what the skin-switcher pass below must preserve.
    dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(
        "Accounts.2005.Q1.Margin",
    ))));
    let selection_before_switch = selection.get_untracked().primary.clone();
    assert_eq!(
        selection_before_switch.as_ref().map(NodeId::as_str),
        Some("Accounts.2005.Q1.Margin")
    );

    let registry = build_default_registry();
    let calls_before_any_mount = recording_bridge.recalc_count();

    // Mount A — triple-editor.
    let cx_a = ErasedSkinContext {
        workspace: workspace.read_only(),
        selection: selection.read_only(),
        shared,
        dispatch: dispatch.clone(),
    };
    let handle_a = registry
        .get(TRIPLE_EDITOR_ID)
        .expect("triple-editor must be registered")
        .mount(cx_a);
    drop(handle_a);

    let calls_after_mount_a = recording_bridge.recalc_count();

    // Mount B — outline-table. This is the "switch": same workspace and
    // selection signals, different skin in the same conceptual slot.
    let cx_b = ErasedSkinContext {
        workspace: workspace.read_only(),
        selection: selection.read_only(),
        shared,
        dispatch: dispatch.clone(),
    };
    let handle_b = registry
        .get(OUTLINE_TABLE_ID)
        .expect("outline-table must be registered")
        .mount(cx_b);
    drop(handle_b);

    let calls_after_mount_b = recording_bridge.recalc_count();

    // Invariants for `dtc-osq.5`:
    assert_eq!(calls_before_any_mount, 0);
    assert_eq!(
        calls_after_mount_a, 0,
        "Mounting a skin must not call the bridge"
    );
    assert_eq!(
        calls_after_mount_b, 0,
        "Switching to a second skin must not call the bridge"
    );

    // Selection survives the switch.
    let selection_after_switch = selection.get_untracked().primary.clone();
    assert_eq!(selection_before_switch, selection_after_switch);
}

#[test]
fn selection_signal_visible_to_both_skins_via_their_contexts() {
    // The skin contract gives each skin a `ReadSignal<SelectionState>`.
    // Asserting two erased contexts read the same selection value
    // proves the shared-signal wiring the shell relies on for
    // `UX-TR-004` ("select in one skin, observe in another").
    let _owner = Owner::new();

    let workspace = RwSignal::new(preview_accounts_workspace_state());
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let dispatcher = Arc::new(HostDispatcher::new(selection, None));
    let dispatch: Arc<dyn Dispatcher> = dispatcher.clone();

    let cx_for_first = ErasedSkinContext {
        workspace: workspace.read_only(),
        selection: selection.read_only(),
        shared,
        dispatch: dispatch.clone(),
    };
    let cx_for_second = ErasedSkinContext {
        workspace: workspace.read_only(),
        selection: selection.read_only(),
        shared,
        dispatch,
    };

    dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(
        "Accounts.2005.Q2.Net",
    ))));

    assert_eq!(
        cx_for_first
            .selection
            .get_untracked()
            .primary
            .as_ref()
            .map(NodeId::as_str),
        Some("Accounts.2005.Q2.Net")
    );
    assert_eq!(
        cx_for_second
            .selection
            .get_untracked()
            .primary
            .as_ref()
            .map(NodeId::as_str),
        Some("Accounts.2005.Q2.Net")
    );
}

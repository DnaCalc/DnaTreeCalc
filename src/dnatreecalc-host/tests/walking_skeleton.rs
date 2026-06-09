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
//!   signals) — never recalculates the direct OxCalc context.
//! - **UX-TR-004:** a `SelectNode` dispatched through the host dispatcher
//!   updates the shared selection signal that every mounted skin
//!   observes, and selection survives a skin switch.
//!
//! DOM-level click-through evidence is dtc-osq.8's responsibility; this
//! file proves the architecture's invariants at the framework + host
//! level.

use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use dnatreecalc_host::app::LocalFileWorkspaceDocumentStore;
use dnatreecalc_host::app::{
    HostDispatcher, InMemoryWorkspaceDocumentStore, TreeWorkspaceSession, WorkspaceDocumentStore,
    build_default_registry, preview_accounts_workspace_state,
    workspace_session_from_document_store_or_default,
};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    Dispatcher, ErasedSkinContext, InMemorySkinStatePersistenceStore, NodeId, SelectionState,
    SharedSkinState, SharedSkinStateHandle, SkinMountSlot, ThemeTokens, WorkspaceDelta,
    WorkspaceIntent,
};
use dnatreecalc_skins::{
    DEPENDENCY_INSPECTOR_ID, FORMULA_TREE_ID, OUTLINE_TABLE_ID, TRIPLE_EDITOR_ID, VALUE_BOARD_ID,
};
use leptos::prelude::*;

#[test]
fn default_registry_ships_triple_editor_and_outline_table() {
    let registry = build_default_registry();
    assert_eq!(registry.len(), 5);

    let ids = registry.ids();
    assert!(ids.contains(&TRIPLE_EDITOR_ID));
    assert!(ids.contains(&FORMULA_TREE_ID));
    assert!(ids.contains(&OUTLINE_TABLE_ID));
    assert!(ids.contains(&VALUE_BOARD_ID));
    assert!(ids.contains(&DEPENDENCY_INSPECTOR_ID));

    let triple = registry
        .get(TRIPLE_EDITOR_ID)
        .expect("triple-editor must be registered");
    assert_eq!(triple.manifest().display_name, "Triple editor");

    let outline = registry
        .get(OUTLINE_TABLE_ID)
        .expect("outline-table must be registered");
    assert_eq!(outline.manifest().display_name, "Outline table");

    assert_eq!(
        registry
            .get(FORMULA_TREE_ID)
            .expect("formula-tree must be registered")
            .manifest()
            .display_name,
        "Formula tree"
    );
    assert!(
        registry
            .get(FORMULA_TREE_ID)
            .expect("formula-tree must be registered")
            .capabilities()
            .supports_inline_formula_edit
    );
    assert!(
        registry
            .get(VALUE_BOARD_ID)
            .expect("value-board must be registered")
            .capabilities()
            .renders_arrays_inline
    );
    assert_eq!(
        registry
            .get(DEPENDENCY_INSPECTOR_ID)
            .expect("dependency-inspector must be registered")
            .manifest()
            .category,
        dnatreecalc_skin_framework::SkinCategory::Inspector
    );
}

#[test]
fn select_node_intent_updates_selection_without_recalculating() {
    let _owner = Owner::new();
    let selection = RwSignal::new(SelectionState::default());

    let dispatcher = HostDispatcher::new(selection);

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

    let log = dispatcher.intents();
    assert_eq!(log.len(), 1);
}

#[test]
fn mounting_and_switching_skins_never_recalculates_the_oxcalc_context() {
    let _owner = Owner::new();

    let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let session = Arc::new(std::sync::Mutex::new(
        TreeWorkspaceSession::from_model(&model).unwrap(),
    ));
    let workspace_state = session.lock().unwrap().workspace_state().unwrap();
    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store = Arc::new(InMemorySkinStatePersistenceStore::new());

    let dispatcher = Arc::new(HostDispatcher::with_session(
        selection,
        workspace,
        latest_delta,
        session.clone(),
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
    let recalc_before_any_mount = session.lock().unwrap().recalc_count();

    // Mount A — triple-editor.
    let cx_a = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot: SkinMountSlot::Main,
        skin_state_store: skin_state_store.clone(),
        dispatch: dispatch.clone(),
    };
    let handle_a = registry
        .get(TRIPLE_EDITOR_ID)
        .expect("triple-editor must be registered")
        .mount(cx_a);
    drop(handle_a);

    let recalc_after_mount_a = session.lock().unwrap().recalc_count();

    // Mount B — outline-table. This is the "switch": same workspace and
    // selection signals, different skin in the same conceptual slot.
    let cx_b = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot: SkinMountSlot::Main,
        skin_state_store,
        dispatch: dispatch.clone(),
    };
    let handle_b = registry
        .get(OUTLINE_TABLE_ID)
        .expect("outline-table must be registered")
        .mount(cx_b);
    drop(handle_b);

    let recalc_after_mount_b = session.lock().unwrap().recalc_count();

    // Invariants for `dtc-osq.5`:
    assert_eq!(recalc_before_any_mount, 0);
    assert_eq!(
        recalc_after_mount_a, 0,
        "Mounting a skin must not recalculate OxCalc"
    );
    assert_eq!(
        recalc_after_mount_b, 0,
        "Switching to a second skin must not recalculate OxCalc"
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
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store = Arc::new(InMemorySkinStatePersistenceStore::new());
    let dispatcher = Arc::new(HostDispatcher::new(selection));
    let dispatch: Arc<dyn Dispatcher> = dispatcher.clone();

    let cx_for_first = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot: SkinMountSlot::Main,
        skin_state_store: skin_state_store.clone(),
        dispatch: dispatch.clone(),
    };
    let cx_for_second = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot: SkinMountSlot::Main,
        skin_state_store,
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

#[test]
fn edit_formula_intent_recalculates_direct_context_and_updates_workspace_signal() {
    let _owner = Owner::new();

    let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let mut initial_session = TreeWorkspaceSession::from_model(&model).unwrap();
    initial_session.recalculate().unwrap();
    let workspace_state = initial_session.workspace_state().unwrap();
    let session = Arc::new(std::sync::Mutex::new(initial_session));

    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher =
        HostDispatcher::with_session(selection, workspace, latest_delta, session.clone());

    let receipt = dispatcher.dispatch(WorkspaceIntent::EditFormula {
        node: NodeId::new("Accounts.2005.Q1.Income.Sales"),
        content: "20".to_string(),
    });

    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_eq!(latest_delta.get_untracked(), receipt.delta);
    assert_eq!(session.lock().unwrap().recalc_count(), 2);
    let state = workspace.get_untracked();
    assert_eq!(
        state
            .node(&NodeId::new("Accounts.2005.Q1.Income"))
            .and_then(|node| node.computed_value.scalar_display_text()),
        Some("4")
    );
}

#[test]
fn workspace_management_intents_create_and_switch_projected_sessions() {
    let _owner = Owner::new();

    let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let mut initial_session = TreeWorkspaceSession::from_model(&model).unwrap();
    initial_session.recalculate().unwrap();
    let workspace_state = initial_session.workspace_state().unwrap();
    let session = Arc::new(std::sync::Mutex::new(initial_session));

    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::with_primary(Some(NodeId::new(
        "Accounts.2005.Q1.Income",
    ))));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let dispatcher = HostDispatcher::with_session_and_shared(
        selection,
        workspace,
        latest_delta,
        session,
        Some(shared),
    );

    assert_eq!(
        shared.get_untracked().active_workspace_id.as_deref(),
        Some("accounts")
    );
    assert_eq!(
        shared.get_untracked().workspace_ids,
        vec!["accounts".to_string()]
    );

    let receipt = dispatcher.dispatch(WorkspaceIntent::NewWorkspace);
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_eq!(latest_delta.get_untracked(), receipt.delta);
    assert_eq!(workspace.get_untracked().workspace_id, "Workspace 1");
    assert_eq!(workspace.get_untracked().len(), 0);
    assert_eq!(selection.get_untracked().primary, None);
    assert_eq!(
        shared.get_untracked().active_workspace_id.as_deref(),
        Some("Workspace 1")
    );
    assert_eq!(
        shared.get_untracked().workspace_ids,
        vec!["Workspace 1".to_string(), "accounts".to_string()]
    );

    let add_receipt = dispatcher.dispatch(WorkspaceIntent::AddNode {
        parent: None,
        symbol: "Root".to_string(),
        initial: dnatreecalc_skin_framework::InitialNodeContentProjection::Literal {
            content: "1".to_string(),
        },
        is_meta: false,
    });
    assert!(add_receipt.accepted, "{:?}", add_receipt.error);
    assert_eq!(workspace.get_untracked().len(), 1);

    let switch_receipt = dispatcher.dispatch(WorkspaceIntent::SwitchWorkspace {
        workspace_id: "accounts".to_string(),
    });
    assert!(switch_receipt.accepted, "{:?}", switch_receipt.error);
    assert_eq!(workspace.get_untracked().workspace_id, "accounts");
    assert!(workspace.get_untracked().len() > 1);
    assert_eq!(selection.get_untracked().primary, None);
    assert_eq!(
        shared.get_untracked().active_workspace_id.as_deref(),
        Some("accounts")
    );

    let switch_back = dispatcher.dispatch(WorkspaceIntent::SwitchWorkspace {
        workspace_id: "Workspace 1".to_string(),
    });
    assert!(switch_back.accepted, "{:?}", switch_back.error);
    assert_eq!(workspace.get_untracked().workspace_id, "Workspace 1");
    assert!(
        workspace
            .get_untracked()
            .node(&NodeId::new("Root"))
            .is_some()
    );
}

#[test]
fn host_dispatcher_autosaves_workspace_documents_through_store() {
    let _owner = Owner::new();

    let mut initial_session = TreeWorkspaceSession::from_model(
        &WorkspaceModel::try_from(WorkspaceFixture::from_repo_fixture("accounts").unwrap())
            .unwrap(),
    )
    .unwrap();
    initial_session.recalculate().unwrap();
    let workspace_state = initial_session.workspace_state().unwrap();
    let session = Arc::new(std::sync::Mutex::new(initial_session));

    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::with_primary(Some(NodeId::new(
        "Accounts.2005.Q1.Income",
    ))));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let store = Arc::new(InMemoryWorkspaceDocumentStore::new());
    let dispatcher = HostDispatcher::with_session_shared_and_workspace_store(
        selection,
        workspace,
        latest_delta,
        session,
        Some(shared),
        Some(store.clone()),
    );

    let receipt = dispatcher.dispatch(WorkspaceIntent::EditContent {
        node: NodeId::new("Accounts.2005.Q1.Income.Sales"),
        content: "4".to_string(),
    });
    assert!(receipt.accepted, "{:?}", receipt.error);

    let catalog = store
        .load_catalog()
        .unwrap()
        .expect("accepted intent saves workspace catalog");
    assert_eq!(catalog.active_workspace_id.as_deref(), Some("accounts"));
    assert_eq!(catalog.workspace_ids, vec!["accounts".to_string()]);

    let document = store
        .load_workspace("accounts")
        .unwrap()
        .expect("accepted intent saves active workspace document");
    assert_eq!(
        document.selected_node.as_deref(),
        Some("Accounts.2005.Q1.Income")
    );

    let new_receipt = dispatcher.dispatch(WorkspaceIntent::NewWorkspace);
    assert!(new_receipt.accepted, "{:?}", new_receipt.error);
    let add_receipt = dispatcher.dispatch(WorkspaceIntent::AddNode {
        parent: None,
        symbol: "Root".to_string(),
        initial: dnatreecalc_skin_framework::InitialNodeContentProjection::Literal {
            content: "1".to_string(),
        },
        is_meta: false,
    });
    assert!(add_receipt.accepted, "{:?}", add_receipt.error);
    let catalog = store
        .load_catalog()
        .unwrap()
        .expect("new workspace updates persisted catalog");
    assert_eq!(
        catalog.workspace_ids,
        vec!["Workspace 1".to_string(), "accounts".to_string()]
    );
    assert_eq!(catalog.active_workspace_id.as_deref(), Some("Workspace 1"));

    let (mut restored, restored_selection) =
        workspace_session_from_document_store_or_default(store.as_ref()).unwrap();
    restored.recalculate().unwrap();
    let restored_state = restored.workspace_state().unwrap();
    assert_eq!(restored_state.workspace_id, "Workspace 1");
    assert_eq!(
        restored_selection.as_ref().map(NodeId::as_str),
        Some("Root")
    );

    let restored_workspace = RwSignal::new(restored_state);
    let restored_delta = RwSignal::new(WorkspaceDelta::unchanged(
        restored_workspace.get_untracked().projection_seq,
    ));
    let restored_selection_signal =
        RwSignal::new(SelectionState::with_primary(restored_selection.clone()));
    let restored_shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let restored_dispatcher = HostDispatcher::with_session_shared_and_workspace_store(
        restored_selection_signal,
        restored_workspace,
        restored_delta,
        Arc::new(std::sync::Mutex::new(restored)),
        Some(restored_shared),
        Some(store.clone()),
    );
    assert_eq!(
        restored_shared.get_untracked().workspace_ids,
        vec!["Workspace 1".to_string(), "accounts".to_string()]
    );
    let switch_receipt = restored_dispatcher.dispatch(WorkspaceIntent::SwitchWorkspace {
        workspace_id: "accounts".to_string(),
    });
    assert!(switch_receipt.accepted, "{:?}", switch_receipt.error);
    let restored_state = restored_workspace.get_untracked();
    assert_eq!(
        scalar_value(&restored_state, "Accounts.2005.Q1.Income"),
        Some("0.8")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_file_workspace_document_store_roundtrips_for_desktop_storage() {
    let root = std::env::temp_dir().join(format!(
        "dnatreecalc-workspace-store-test-{}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let store = LocalFileWorkspaceDocumentStore::new(&root);

    let mut session = TreeWorkspaceSession::from_model(
        &WorkspaceModel::try_from(WorkspaceFixture::from_repo_fixture("accounts").unwrap())
            .unwrap(),
    )
    .unwrap();
    session.recalculate().unwrap();
    let document = session
        .export_dnatree_document(Some(&NodeId::new("Accounts.2005.Q1.Income")))
        .unwrap();
    store
        .save_catalog(&dnatreecalc_host::app::WorkspaceDocumentCatalog::new(
            vec!["accounts".to_string()],
            Some("accounts".to_string()),
        ))
        .unwrap();
    store.save_workspace("accounts", &document).unwrap();

    let (restored, selection) = workspace_session_from_document_store_or_default(&store).unwrap();
    let restored_state = restored.workspace_state().unwrap();
    assert_eq!(
        selection.as_ref().map(NodeId::as_str),
        Some("Accounts.2005.Q1.Income")
    );
    assert_eq!(restored_state.workspace_id, "accounts");
    assert!(root.join("catalog.json").exists());

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn walking_skeleton_click_through_harness_edits_switches_saves_and_reopens() {
    let _owner = Owner::new();

    let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let mut initial_session = TreeWorkspaceSession::from_model(&model).unwrap();
    initial_session.recalculate().unwrap();
    let workspace_state = initial_session.workspace_state().unwrap();
    let session = Arc::new(std::sync::Mutex::new(initial_session));

    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store = Arc::new(InMemorySkinStatePersistenceStore::new());
    let dispatcher = Arc::new(HostDispatcher::with_session(
        selection,
        workspace,
        latest_delta,
        session.clone(),
    ));
    let dispatch: Arc<dyn Dispatcher> = dispatcher.clone();

    dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(
        "Accounts.2005.Q1.Income.Sales",
    ))));
    let receipt = dispatcher.dispatch(WorkspaceIntent::EditFormula {
        node: NodeId::new("Accounts.2005.Q1.Income.Sales"),
        content: "20".to_string(),
    });
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_eq!(
        scalar_value(&workspace.get_untracked(), "Accounts.2005.Q1.Income"),
        Some("4")
    );

    let registry = build_default_registry();
    let recalc_before_switch = session.lock().unwrap().recalc_count();
    for skin_id in [
        TRIPLE_EDITOR_ID,
        FORMULA_TREE_ID,
        OUTLINE_TABLE_ID,
        VALUE_BOARD_ID,
        DEPENDENCY_INSPECTOR_ID,
    ] {
        let cx = ErasedSkinContext {
            workspace: workspace.read_only(),
            latest_delta: latest_delta.read_only(),
            selection: selection.read_only(),
            shared,
            tokens: ThemeTokens::light(),
            slot: SkinMountSlot::Main,
            skin_state_store: skin_state_store.clone(),
            dispatch: dispatch.clone(),
        };
        let handle = registry
            .get(skin_id)
            .expect("skeleton skin must be registered")
            .mount(cx);
        drop(handle);
    }
    assert_eq!(
        session.lock().unwrap().recalc_count(),
        recalc_before_switch,
        "switching skeleton skins must not recalculate"
    );

    let selected = selection.get_untracked().primary.clone();
    let document = session
        .lock()
        .unwrap()
        .export_dnatree_document(selected.as_ref())
        .unwrap();
    let json = serde_json::to_string_pretty(&document).unwrap();
    let reparsed = serde_json::from_str(&json).unwrap();
    let (reopened, reopened_selection) =
        TreeWorkspaceSession::from_dnatree_document(reparsed).unwrap();
    let reopened_state = reopened.workspace_state().unwrap();

    assert_eq!(
        reopened_selection.as_ref().map(NodeId::as_str),
        Some("Accounts.2005.Q1.Income.Sales")
    );
    assert_eq!(
        scalar_value(&reopened_state, "Accounts.2005.Q1.Income"),
        Some("4")
    );
}

fn scalar_value<'a>(
    state: &'a dnatreecalc_skin_framework::WorkspaceState,
    node_id: &str,
) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id))
        .and_then(|node| node.computed_value.scalar_display_text())
}

//! Framework-level tests over a fake "inert" skin.
//!
//! Concrete skin implementations live in `dnatreecalc-skins`; integration
//! tests that mount the real skins through the shell + a direct host session
//! live in `dnatreecalc-host` under `tests/`. The point of this module is
//! to prove the trait/registry contract without dragging the rest of the
//! workspace into a circular dep.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
use crate::intent::{
    Dispatcher, InMemoryDispatcher, IntentReceipt, WorkspaceDelta, WorkspaceIntent,
};
use crate::manifest::{SkinCapabilities, SkinCategory, SkinManifest};
use crate::registry::SkinRegistry;
use crate::selection::SelectionState;
use crate::skin::{ErasedSkinContext, SkinContext, SkinHandle, WorkspaceSkin};
use crate::state::{
    InMemorySkinStatePersistenceStore, MigrationError, PersistedSkinStateRecord, SharedSkinState,
    SharedSkinStateHandle, SkinState, SkinStatePersistenceKey,
};
use crate::theme::{ThemeMode, ThemeTokens};
use crate::workspace::WorkspaceState;
use crate::{
    SelectableItemA11y, SelectableRowA11y, listbox_a11y, roving_tabindex, stable_node_dom_id,
    tree_a11y,
};

#[derive(Default, Clone, Serialize, Deserialize)]
struct InertState {
    pub mounts: u32,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct PersistedTestState {
    pub mounts: u32,
    pub label: String,
    pub remembered_nodes: HashSet<NodeKey>,
}

impl SkinState for PersistedTestState {
    fn schema_version() -> u32 {
        2
    }

    fn migrate(prior_version: u32, prior_value: serde_json::Value) -> Result<Self, MigrationError> {
        if prior_version != 1 {
            return Err(MigrationError::Failed(format!(
                "unsupported prior version {prior_version}"
            )));
        }
        #[derive(Deserialize)]
        struct V1 {
            label: String,
            remembered_nodes: HashSet<NodeKey>,
        }
        let prior: V1 = serde_json::from_value(prior_value)
            .map_err(|error| MigrationError::Failed(error.to_string()))?;
        Ok(Self {
            mounts: 0,
            label: prior.label,
            remembered_nodes: prior.remembered_nodes,
        })
    }

    fn gc(&mut self, live_nodes: &HashSet<NodeKey>) {
        self.remembered_nodes
            .retain(|node| live_nodes.contains(node));
    }
}

struct PersistedSkin {
    observed: Arc<Mutex<Vec<PersistedTestState>>>,
}

impl WorkspaceSkin for PersistedSkin {
    type State = PersistedTestState;

    fn id(&self) -> SkinId {
        SkinId::new("persisted-test")
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Persisted",
            description: "Test skin for persisted state.",
            category: SkinCategory::Inspector,
            version: "test",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities::default()
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        self.observed
            .lock()
            .expect("observed state lock poisoned")
            .push(cx.state.get_untracked());
        cx.state.update(|state| {
            state.mounts += 1;
            if state.label.is_empty() {
                state.label = "mounted".to_string();
            }
        });
        SkinHandle::new(().into_any())
    }
}

impl SkinState for InertState {
    fn schema_version() -> u32 {
        1
    }
}

struct InertSkin {
    id: SkinId,
}

impl InertSkin {
    fn new(id: &'static str) -> Self {
        Self {
            id: SkinId::new(id),
        }
    }
}

impl WorkspaceSkin for InertSkin {
    type State = InertState;

    fn id(&self) -> SkinId {
        self.id
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Inert",
            description: "Test skin that does not render anything beyond a placeholder.",
            category: SkinCategory::Editor,
            version: "test",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities::default()
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        cx.state.update(|s| s.mounts += 1);
        SkinHandle::new(().into_any())
    }
}

#[test]
fn registry_preserves_insertion_order_and_lookup() {
    let mut registry = SkinRegistry::new();
    let alpha = registry.register(InertSkin::new("alpha"));
    let beta = registry.register(InertSkin::new("beta"));
    assert_eq!(registry.len(), 2);
    assert_eq!(registry.ids(), vec![alpha, beta]);
    assert_eq!(registry.get(alpha).map(|s| s.id()), Some(alpha));
    assert_eq!(registry.get(beta).map(|s| s.id()), Some(beta));
}

#[test]
fn registry_iteration_is_stable() {
    let mut registry = SkinRegistry::new();
    registry.register(InertSkin::new("alpha"));
    registry.register(InertSkin::new("beta"));
    let names: Vec<_> = registry.iter().map(|s| s.manifest().display_name).collect();
    assert_eq!(names, vec!["Inert", "Inert"]);
}

#[test]
fn in_memory_dispatcher_applies_select_node_intents() {
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher = InMemoryDispatcher::new(selection);

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
    assert!(matches!(
        log[0],
        WorkspaceIntent::SelectNode(Some(ref id)) if id.as_str() == "Accounts"
    ));
}

#[test]
fn registry_mount_invokes_typed_skin_with_default_state() {
    let mut registry = SkinRegistry::new();
    let id = registry.register(InertSkin::new("alpha"));
    let workspace = RwSignal::new(WorkspaceState::default());
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher: Arc<dyn Dispatcher> = Arc::new(InMemoryDispatcher::new(selection));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let skin_state_store = Arc::new(InMemorySkinStatePersistenceStore::new());

    let cx = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot: SkinMountSlot::Main,
        skin_state_store,
        dispatch: dispatcher,
    };

    let skin = registry.get(id).expect("registered skin must resolve");
    let handle = skin.mount(cx);
    assert!(handle.on_deactivate.is_none());
    assert_eq!(ThemeTokens::light().mode, ThemeMode::Light);
    // The view itself is opaque AnyView; reaching it would require a
    // rendered tree which the framework crate deliberately does not
    // depend on. Skin behavior under render is covered by the shell-
    // level integration tests.
}

#[test]
fn registry_mount_roundtrips_skin_state_by_skin_slot_and_workspace() {
    let mut registry = SkinRegistry::new();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let id = registry.register(PersistedSkin {
        observed: observed.clone(),
    });
    let store = Arc::new(InMemorySkinStatePersistenceStore::new());
    let skin = registry.get(id).expect("registered skin must resolve");

    mount_persisted_skin(
        skin,
        store.clone(),
        SkinMountSlot::Main,
        "workspace:a",
        vec![NodeKey::new("node:a")],
    );
    mount_persisted_skin(
        skin,
        store.clone(),
        SkinMountSlot::Main,
        "workspace:a",
        vec![NodeKey::new("node:a")],
    );
    mount_persisted_skin(
        skin,
        store,
        SkinMountSlot::SplitLeft,
        "workspace:a",
        vec![NodeKey::new("node:a")],
    );

    let observed = observed.lock().expect("observed state lock poisoned");
    assert_eq!(observed[0].mounts, 0);
    assert_eq!(observed[1].mounts, 1);
    assert_eq!(observed[1].label, "mounted");
    assert_eq!(
        observed[2].mounts, 0,
        "different slots must not share persisted SkinState"
    );
}

#[test]
fn registry_mount_migrates_and_garbage_collects_nodekey_state() {
    let mut registry = SkinRegistry::new();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let id = registry.register(PersistedSkin {
        observed: observed.clone(),
    });
    let store = Arc::new(InMemorySkinStatePersistenceStore::new());
    let key = SkinStatePersistenceKey::new(
        SkinId::new("persisted-test"),
        SkinMountSlot::Main,
        "workspace:migrated",
    );
    store
        .insert(
            key.clone(),
            PersistedSkinStateRecord::new(
                1,
                serde_json::json!({
                    "label": "legacy",
                    "remembered_nodes": ["node:live", "node:deleted"]
                }),
            ),
        )
        .expect("seed persisted state");

    let skin = registry.get(id).expect("registered skin must resolve");
    mount_persisted_skin(
        skin,
        store.clone(),
        SkinMountSlot::Main,
        "workspace:migrated",
        vec![NodeKey::new("node:live")],
    );

    let observed = observed.lock().expect("observed state lock poisoned");
    assert_eq!(observed[0].label, "legacy");
    assert_eq!(
        observed[0].remembered_nodes,
        [NodeKey::new("node:live")].into_iter().collect()
    );
    drop(observed);

    let stored = store
        .get(&key)
        .expect("read persisted state")
        .expect("state should be persisted after migration");
    assert_eq!(stored.schema_version, PersistedTestState::schema_version());
    let saved: PersistedTestState =
        serde_json::from_value(stored.value).expect("stored value should deserialize");
    assert_eq!(saved.label, "legacy");
    assert_eq!(saved.mounts, 1);
    assert_eq!(
        saved.remembered_nodes,
        [NodeKey::new("node:live")].into_iter().collect()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_file_skin_state_store_roundtrips_records() {
    let root = std::env::temp_dir().join(format!(
        "dnatreecalc-skin-state-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = crate::state::LocalFileSkinStatePersistenceStore::new(&root);
    let key = SkinStatePersistenceKey::new(
        SkinId::new("persisted-test"),
        SkinMountSlot::RightInspector,
        "workspace:file",
    );
    let record = PersistedSkinStateRecord::new(7, serde_json::json!({ "ok": true }));

    crate::state::SkinStatePersistenceStore::save(&store, &key, &record)
        .expect("save local file record");
    let loaded = crate::state::SkinStatePersistenceStore::load(&store, &key)
        .expect("load local file record")
        .expect("record should exist");
    assert_eq!(loaded, record);
    std::fs::remove_dir_all(&root).expect("cleanup local file store");
}

#[test]
fn theme_tokens_emit_skin_css_custom_properties() {
    let light = ThemeTokens::light();
    let css = light.css_rule(".dtc-shell");

    assert_eq!(light.mode, ThemeMode::Light);
    assert!(css.starts_with(".dtc-shell {"));
    assert!(css.contains("--dtc-surface: #ffffff;"));
    assert!(css.contains("--dtc-accent: #245f9c;"));
    assert!(
        ThemeTokens::dark()
            .css_custom_properties()
            .contains("#111827")
    );
    assert!(
        ThemeTokens::high_contrast()
            .css_custom_properties()
            .contains("--dtc-focus: #ffff00;")
    );
}

#[test]
fn a11y_helpers_encode_selection_and_roving_focus() {
    let key = NodeKey::new("tree-node:42");
    let selected = SelectableItemA11y::for_tree_item("dtc-node", &key, true, true, 3, 2, 7);
    assert_eq!(selected.id, "dtc-node-tree-node-42");
    assert_eq!(selected.role, "treeitem");
    assert_eq!(selected.aria_selected, "true");
    assert_eq!(selected.aria_level.as_deref(), Some("3"));
    assert_eq!(selected.aria_posinset.as_deref(), Some("2"));
    assert_eq!(selected.aria_setsize.as_deref(), Some("7"));
    assert_eq!(selected.tabindex, "0");

    let unselected_focusable = SelectableRowA11y::new("dtc-row", &key, false, true);
    assert_eq!(unselected_focusable.aria_selected, "false");
    assert_eq!(unselected_focusable.tabindex, "0");
    assert_eq!(roving_tabindex(false), "-1");
    assert_eq!(stable_node_dom_id("prefix", &key), "prefix-tree-node-42");
    assert_eq!(
        tree_a11y("Nodes", "dtc-node", Some(&key)).aria_activedescendant,
        Some("dtc-node-tree-node-42".to_string())
    );
    assert_eq!(listbox_a11y("Nodes", "dtc-node", None).role, "listbox");
}

fn mount_persisted_skin(
    skin: &crate::skin::RegisteredSkin,
    store: Arc<InMemorySkinStatePersistenceStore>,
    slot: SkinMountSlot,
    workspace_id: &str,
    live_keys: Vec<NodeKey>,
) {
    let mut workspace_state = WorkspaceState {
        workspace_id: workspace_id.to_string(),
        ..WorkspaceState::default()
    };
    workspace_state.key_order = live_keys;
    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher: Arc<dyn Dispatcher> = Arc::new(InMemoryDispatcher::new(selection));
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());

    let handle = skin.mount(ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        tokens: ThemeTokens::light(),
        slot,
        skin_state_store: store,
        dispatch: dispatcher,
    });
    drop(handle);
}

#[test]
fn in_memory_dispatcher_records_edit_formula_without_applying() {
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher = InMemoryDispatcher::new(selection);
    let receipt = dispatcher.dispatch(WorkspaceIntent::EditFormula {
        node: NodeId::new("Margin"),
        content: "=Net/Income".into(),
    });
    assert!(receipt.accepted);
    assert!(receipt.error.is_none());
    assert_eq!(dispatcher.intents().len(), 1);
}

#[allow(dead_code)]
fn _silence_unused_warnings() {
    // The IntentReceipt::rejected constructor is for live host dispatchers;
    // the skeleton tests only the accepted path. Keep it referenced so
    // unused-helper lint stays quiet.
    let _ = IntentReceipt::rejected(crate::intent::IntentError::Unsupported);
}

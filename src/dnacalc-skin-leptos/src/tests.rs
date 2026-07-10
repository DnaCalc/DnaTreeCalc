//! Framework-level tests over a fake "inert" skin.
//!
//! Concrete skin implementations live in `dnatreecalc-skins`; integration
//! tests that mount the real skins through the shell + a direct host session
//! live in `dnatreecalc-host` under `tests/`. The point of this module is
//! to prove the trait/registry contract without dragging the rest of the
//! workspace into a circular dep.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use dnacalc_skin_ir::CommandIntentKindProjection;
use dnacalc_skin_ir::identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
use dnacalc_skin_ir::intent::{Dispatcher, IntentReceipt, WorkspaceDelta, WorkspaceIntent};
use dnacalc_skin_ir::manifest::{SkinCapabilities, SkinCategory, SkinManifest};
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{
    InMemorySkinStatePersistenceStore, MigrationError, PersistedSkinStateRecord, SharedSkinState,
    SkinState, SkinStatePersistenceKey,
};
use dnacalc_skin_ir::workspace::{
    CandidateProjection, ClipboardNodeValueProjection, ClipboardOperationProjection,
    ClipboardPayloadProjection, ClipboardProjection, NodeContentKind, NodeValueProjection,
    NodeView, RevisionHistoryEntryProjection, RevisionHistoryProjection,
    ScenarioManifestProjection, ScenarioProjection, ScenarioSourceProjection,
    SpeculationPressureProjection, SweepManifestProjection, SweepPointProjection, SweepProjection,
    WorkspaceRevisionProjection, WorkspaceState,
};

use crate::in_memory_dispatcher::InMemoryDispatcher;
use crate::registry::SkinRegistry;
use crate::skin::{ErasedSkinContext, SkinContext, SkinHandle, WorkspaceSkin};
use crate::state_handles::SharedSkinStateHandle;
use crate::theme::{ThemeMode, ThemeTokens};
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
        preview: None,
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

#[test]
fn command_catalog_projects_selection_sensitive_node_commands() {
    let workspace = workspace_with_nodes(vec![node("node:a", "A", NodeContentKind::Formula)]);
    let catalog = workspace.command_catalog(&SelectionState::default());

    let rename = catalog
        .get(CommandIntentKindProjection::RenameNode)
        .expect("rename command is cataloged");
    assert!(!rename.enabled);
    assert_eq!(
        rename.disabled_reason.as_deref(),
        Some("no node is selected")
    );

    let selection = SelectionState::with_primary(Some(NodeId::new("A")));
    let catalog = workspace.command_catalog(&selection);
    assert!(
        catalog
            .get(CommandIntentKindProjection::RenameNode)
            .expect("rename command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::CopyFormula)
            .expect("copy formula command is cataloged")
            .enabled
    );
    assert_eq!(
        catalog
            .get(CommandIntentKindProjection::Recalculate)
            .expect("recalc command is cataloged")
            .effective_binding,
        Some("F9")
    );
}

#[test]
fn command_catalog_projects_clipboard_and_candidate_affordances() {
    let mut workspace = workspace_with_nodes(vec![node("node:a", "A", NodeContentKind::Constant)]);
    let selection = SelectionState::with_primary(Some(NodeId::new("A")));
    let empty_catalog = workspace.command_catalog(&selection);
    assert!(
        !empty_catalog
            .get(CommandIntentKindProjection::PasteClipboardValues)
            .expect("paste command is cataloged")
            .enabled
    );
    assert!(
        !empty_catalog
            .get(CommandIntentKindProjection::CommitCandidate)
            .expect("commit candidate command is cataloged")
            .enabled
    );

    workspace.clipboard = Some(ClipboardProjection {
        operation: ClipboardOperationProjection::Copy,
        payload: ClipboardPayloadProjection::Values {
            nodes: vec![ClipboardNodeValueProjection {
                node: NodeKey::new("node:a"),
                path: NodeId::new("A"),
                content_kind: NodeContentKind::Constant,
                constant_input_text: Some("1".to_string()),
                literalized_input_text: None,
                value: NodeValueProjection::Number {
                    raw: "1".to_string(),
                    display: "1".to_string(),
                },
            }],
        },
        plain_text: Some("1".to_string()),
    });
    workspace.candidates.push(CandidateProjection {
        handle: "candidate:1".to_string(),
        basis_revision_id: "rev:1".to_string(),
        parent_handle: None,
        retention_pin_count: 0,
        workspace_revision_id: "rev:c1".to_string(),
        revision_history: RevisionHistoryProjection::default(),
        nodes: Vec::new(),
        values_by_key: BTreeMap::new(),
        run: None,
    });
    workspace.speculation_pressure = SpeculationPressureProjection {
        retained_candidate_count: 1,
        reclaimable_candidate_count: 1,
        ..SpeculationPressureProjection::default()
    };

    let catalog = workspace.command_catalog(&selection);
    assert!(
        catalog
            .get(CommandIntentKindProjection::PasteClipboardValues)
            .expect("paste command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::CommitCandidate)
            .expect("commit candidate command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::ReapCandidates)
            .expect("reap command is cataloged")
            .enabled
    );
}

#[test]
fn command_catalog_projects_scenario_sweep_and_revision_navigation_state() {
    let input = node("node:input", "Input", NodeContentKind::Constant);
    let mut workspace = workspace_with_nodes(vec![input]);
    workspace.revision_history = RevisionHistoryProjection {
        current_revision_id: Some("rev:2".to_string()),
        entries: vec![
            RevisionHistoryEntryProjection {
                revision_id: "rev:1".to_string(),
                parent_revision_id: None,
                structural_snapshot_id: "struct:1".to_string(),
                node_input_snapshot_id: "input:1".to_string(),
                namespace_snapshot_id: "ns:1".to_string(),
                transaction_id: None,
                transaction_summary: None,
                is_current: false,
            },
            RevisionHistoryEntryProjection {
                revision_id: "rev:2".to_string(),
                parent_revision_id: Some("rev:1".to_string()),
                structural_snapshot_id: "struct:2".to_string(),
                node_input_snapshot_id: "input:2".to_string(),
                namespace_snapshot_id: "ns:2".to_string(),
                transaction_id: Some("tx:2".to_string()),
                transaction_summary: None,
                is_current: true,
            },
            RevisionHistoryEntryProjection {
                revision_id: "rev:3".to_string(),
                parent_revision_id: Some("rev:2".to_string()),
                structural_snapshot_id: "struct:3".to_string(),
                node_input_snapshot_id: "input:3".to_string(),
                namespace_snapshot_id: "ns:3".to_string(),
                transaction_id: Some("tx:3".to_string()),
                transaction_summary: None,
                is_current: false,
            },
        ],
    };
    workspace.scenarios = ScenarioManifestProjection {
        active: Some("scenario:base".to_string()),
        entries: vec![ScenarioProjection {
            id: "scenario:base".to_string(),
            name: "Base".to_string(),
            source: ScenarioSourceProjection::Candidate {
                handle: "candidate:scenario".to_string(),
            },
            override_count: 0,
            overridden_nodes: Vec::new(),
            override_values: BTreeMap::new(),
            value_epoch: Some(1),
            is_active: true,
        }],
    };
    workspace.sweeps = SweepManifestProjection {
        active: Some("sweep:input".to_string()),
        entries: vec![SweepProjection {
            id: "sweep:input".to_string(),
            name: "Input Sweep".to_string(),
            input_node: NodeKey::new("node:input"),
            base_scenario_id: None,
            points: vec![SweepPointProjection {
                id: "low".to_string(),
                label: "Low".to_string(),
                input_value: NodeValueProjection::Number {
                    raw: "1".to_string(),
                    display: "1".to_string(),
                },
                scenario_id: "scenario:sweep-low".to_string(),
                value_epoch: Some(2),
            }],
            value_epoch: Some(2),
            is_active: true,
        }],
    };

    let catalog =
        workspace.command_catalog(&SelectionState::with_primary(Some(NodeId::new("Input"))));
    assert!(
        catalog
            .get(CommandIntentKindProjection::DeleteScenario)
            .expect("delete scenario command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::DeleteSweep)
            .expect("delete sweep command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::CreateScenarioSweep)
            .expect("create sweep command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::Undo)
            .expect("undo command is cataloged")
            .enabled
    );
    assert!(
        catalog
            .get(CommandIntentKindProjection::Redo)
            .expect("redo command is cataloged")
            .enabled
    );
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
        preview: None,
    });
    drop(handle);
}

fn workspace_with_nodes(nodes: Vec<NodeView>) -> WorkspaceState {
    let mut workspace = WorkspaceState {
        workspace_id: "workspace:test".to_string(),
        profile: "test".to_string(),
        revision: WorkspaceRevisionProjection {
            workspace_revision_id: Some("rev:1".to_string()),
            ..WorkspaceRevisionProjection::default()
        },
        ..WorkspaceState::default()
    };
    for node in nodes {
        workspace.node_order.push(node.id.clone());
        workspace.key_order.push(node.key.clone());
        workspace
            .paths_by_key
            .insert(node.key.clone(), node.id.clone());
        workspace.nodes.insert(node.id.clone(), node.clone());
        workspace.nodes_by_key.insert(node.key.clone(), node);
    }
    workspace
}

fn node(key: &str, path: &str, content_kind: NodeContentKind) -> NodeView {
    NodeView {
        key: NodeKey::new(key),
        id: NodeId::new(path),
        display_name: path.to_string(),
        parent: None,
        children: Vec::new(),
        depth: 0,
        content_kind,
        content_text: if content_kind == NodeContentKind::Formula {
            "=1".to_string()
        } else {
            "1".to_string()
        },
        computed_value: NodeValueProjection::Number {
            raw: "1".to_string(),
            display: "1".to_string(),
        },
        scenario_override: None,
        literalized_value_input: Some("1".to_string()),
        value_epoch: Some(1),
        calc_state: None,
        effective_format: None,
        note: None,
        attributes: BTreeMap::new(),
        binding_diagnostics: Vec::new(),
        is_meta: false,
        table: None,
    }
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

// --- ATLAS shared spine: keybinding registry, continuity, styling ---------

use crate::keybinding::{KeyChord, KeybindingRegistry, SkinVerb};
use crate::style::{ProvenanceTint, calc_state_class, provenance_tint, selection_mode_class};
use crate::workspace::{CleaveFilter, CleavePredicate, CleaveSort, NodeCalcStateProjection};

fn numeric_node(key: &str, path: &str, value: &str) -> NodeView {
    let mut view = node(key, path, NodeContentKind::Constant);
    view.computed_value = NodeValueProjection::Number {
        raw: value.to_string(),
        display: value.to_string(),
    };
    view
}

#[test]
fn universal_grammar_is_collision_free() {
    let registry = KeybindingRegistry::universal();
    let mut seen = std::collections::HashSet::new();
    for (chord, _) in registry.bindings() {
        assert!(
            seen.insert(chord.clone()),
            "chord {chord:?} is bound twice — the universal grammar must be collision-free"
        );
    }
}

#[test]
fn universal_grammar_resolves_the_canonical_verbs() {
    let registry = KeybindingRegistry::universal();
    assert_eq!(
        registry.resolve(&KeyChord::bare("Enter")),
        Some(SkinVerb::Commit)
    );
    assert_eq!(
        registry.resolve(&KeyChord::bare("F9")),
        Some(SkinVerb::Recalculate)
    );
    // Ctrl+Enter is a compatibility chord for the same verb.
    assert_eq!(
        registry.resolve(&KeyChord::ctrl("Enter")),
        Some(SkinVerb::Recalculate)
    );
    assert_eq!(registry.resolve(&KeyChord::ctrl("d")), Some(SkinVerb::Fill));
    assert_eq!(registry.resolve(&KeyChord::bare("h")), Some(SkinVerb::Fold));
    assert_eq!(
        registry.resolve(&KeyChord::bare("]")),
        Some(SkinVerb::TraceForward)
    );
    assert_eq!(
        registry.resolve(&KeyChord::bare("/")),
        Some(SkinVerb::NameBox)
    );
    assert_eq!(
        registry.resolve(&KeyChord::bare(" ")),
        Some(SkinVerb::Leader)
    );
    assert_eq!(
        registry.resolve(&KeyChord::bare("e")),
        Some(SkinVerb::Explain)
    );
    assert_eq!(registry.resolve(&KeyChord::ctrl("z")), Some(SkinVerb::Undo));
    assert_eq!(
        registry.resolve(&KeyChord::ctrl("3")),
        Some(SkinVerb::SwitchLens(3))
    );
    // An unbound chord resolves to nothing.
    assert_eq!(registry.resolve(&KeyChord::bare("q")), None);
}

#[test]
fn keychord_normalizes_alphabetic_case() {
    let upper = KeyChord::from_parts("D", true, false, true);
    assert_eq!(upper.key, "d");
    assert!(upper.ctrl && upper.shift && !upper.alt);
    // Shift is carried by the flag, not the key case, so Ctrl+Shift+D does NOT
    // collide with the Ctrl+D fill chord.
    let registry = KeybindingRegistry::universal();
    assert_eq!(registry.resolve(&upper), None);
}

#[test]
fn verb_command_kind_joins_the_catalog() {
    assert_eq!(
        SkinVerb::Recalculate.command_kind(),
        Some(CommandIntentKindProjection::Recalculate)
    );
    assert_eq!(
        SkinVerb::Undo.command_kind(),
        Some(CommandIntentKindProjection::Undo)
    );
    assert_eq!(SkinVerb::Explain.command_kind(), None);
}

/// The command catalog's advertised shortcuts must agree with the universal
/// keybinding registry: a command may not advertise a key the registry binds to
/// a *different* verb, or the hint tells the user a key does X when it actually
/// does Y. Regression guard for the "Delete Node [Delete]" confusion —
/// Delete/Backspace clear contents and F2 edits in place, so the structural
/// Delete/Rename commands must no longer advertise those keys.
#[test]
fn catalog_shortcuts_agree_with_the_keybinding_registry() {
    fn parse_shortcut(label: &str) -> Option<KeyChord> {
        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        let mut key = None;
        for part in label.split('+') {
            match part {
                "Ctrl" => ctrl = true,
                "Alt" => alt = true,
                "Shift" => shift = true,
                other => key = Some(other.to_string()),
            }
        }
        Some(KeyChord::from_parts(key?, ctrl, alt, shift))
    }

    let registry = KeybindingRegistry::universal();
    let workspace = workspace_with_nodes(vec![node("node:a", "A", NodeContentKind::Formula)]);
    let selection = SelectionState::with_primary(Some(NodeId::new("A")));
    let catalog = workspace.command_catalog(&selection);

    for entry in &catalog.entries {
        let Some(shortcut) = entry.effective_binding else {
            continue;
        };
        let Some(chord) = parse_shortcut(shortcut) else {
            continue;
        };
        let Some(verb) = registry.resolve(&chord) else {
            // A shortcut the universal registry does not bind (e.g. a lens-local
            // or browser-level chord like Ctrl+C) cannot contradict it.
            continue;
        };
        // Enter is the universal Commit key; every edit/commit command rides on
        // it legitimately, so it is the one verb allowed to back several kinds.
        if verb == SkinVerb::Commit {
            continue;
        }
        assert_eq!(
            verb.command_kind(),
            Some(entry.intent_kind),
            "command {:?} advertises shortcut {shortcut:?}, but the registry binds \
             that chord to {verb:?} — the hint would lie",
            entry.intent_kind,
        );
    }
}

#[test]
fn shared_store_applies_typed_changes_and_records_audit() {
    use crate::state::{SharedStateChange, SharedStateOrigin};

    let handle = SharedSkinStateHandle::new(SharedSkinState::default());
    let lens_origin = SharedStateOrigin::lens(SkinId::new("ledger"), SkinMountSlot::Main);

    handle.apply(
        SharedStateChange::ToggleSelection(NodeKey::new("k:a")),
        lens_origin.clone(),
    );
    handle.apply(
        SharedStateChange::Fold(NodeKey::new("k:b")),
        lens_origin.clone(),
    );
    handle.apply(
        SharedStateChange::SetFocusedSlot(Some(SkinMountSlot::RightInspector)),
        SharedStateOrigin::Shell,
    );
    // Toggle removes on the second application.
    handle.apply(
        SharedStateChange::ToggleSelection(NodeKey::new("k:a")),
        lens_origin.clone(),
    );

    let state = handle.get_untracked();
    assert!(state.selection_set.is_empty());
    assert!(state.collapsed_keys.contains(&NodeKey::new("k:b")));
    assert_eq!(state.focused_slot, Some(SkinMountSlot::RightInspector));

    let audit = handle.audit_log();
    assert_eq!(audit.len(), 4);
    assert_eq!(audit[0].seq, 0);
    assert_eq!(audit[3].seq, 3);
    assert_eq!(audit[2].origin, SharedStateOrigin::Shell);
    assert!(matches!(
        &audit[1].change,
        SharedStateChange::Fold(key) if key == &NodeKey::new("k:b")
    ));
}

#[test]
fn shared_store_tracks_workspace_names_and_save_status() {
    use crate::state::{SharedStateChange, SharedStateOrigin};

    let handle = SharedSkinStateHandle::new(SharedSkinState::default());
    let mut names = std::collections::BTreeMap::new();
    names.insert("ws-1".to_string(), "Quarterly model".to_string());
    handle.apply(
        SharedStateChange::SetWorkspaceNames(names.clone()),
        SharedStateOrigin::Host,
    );
    handle.apply(
        SharedStateChange::SetLastSaveSuccess(Some(true)),
        SharedStateOrigin::Host,
    );

    let state = handle.get_untracked();
    assert_eq!(
        state.workspace_names.get("ws-1").map(String::as_str),
        Some("Quarterly model")
    );
    assert_eq!(state.last_save_success, Some(true));

    // A failed save flips the indicator without disturbing the names.
    handle.apply(
        SharedStateChange::SetLastSaveSuccess(Some(false)),
        SharedStateOrigin::Host,
    );
    let state = handle.get_untracked();
    assert_eq!(state.last_save_success, Some(false));
    assert_eq!(state.workspace_names, names);
}

#[test]
fn shared_store_audit_ring_is_bounded() {
    use crate::state::{SHARED_AUDIT_CAPACITY, SharedStateChange, SharedStateOrigin};

    let handle = SharedSkinStateHandle::new(SharedSkinState::default());
    for index in 0..(SHARED_AUDIT_CAPACITY + 10) {
        handle.apply(
            SharedStateChange::SetManualRecalcPending(index % 2 == 0),
            SharedStateOrigin::Host,
        );
    }
    let audit = handle.audit_log();
    assert_eq!(audit.len(), SHARED_AUDIT_CAPACITY);
    // Oldest entries dropped; sequence numbers stay monotonic.
    assert_eq!(audit.first().map(|record| record.seq), Some(10));
    assert_eq!(
        audit.last().map(|record| record.seq),
        Some((SHARED_AUDIT_CAPACITY + 9) as u64)
    );
}

#[test]
fn slot_negotiation_defaults_to_main_class() {
    let mut registry = SkinRegistry::new();
    let id = registry.register(InertSkin::new("main-class"));
    let registered = registry.get(id).expect("registered");

    // Main-class: the primary pane and the side-by-side split panes.
    assert!(registered.negotiate_slot(SkinMountSlot::Main).is_ok());
    assert!(registered.negotiate_slot(SkinMountSlot::SplitLeft).is_ok());
    assert!(registered.negotiate_slot(SkinMountSlot::SplitRight).is_ok());
    // Companion slots stay refused for a default skin.
    let refused = registered
        .negotiate_slot(SkinMountSlot::RightInspector)
        .expect_err("companion slot must be refused for a main-class skin");
    assert!(refused.to_string().contains("right_inspector"));
}

#[test]
fn shared_continuity_round_trips_and_gcs() {
    let mut state = SharedSkinState {
        selection_set: vec![NodeKey::new("k:a"), NodeKey::new("k:b")],
        selection_anchor: Some(NodeKey::new("k:a")),
        collapsed_keys: HashSet::from([NodeKey::new("k:a"), NodeKey::new("k:b")]),
        pinned_keys: vec![NodeKey::new("k:b")],
        focus_key: Some(NodeKey::new("k:a")),
        cleave: Some(CleavePredicate {
            filter: Some(CleaveFilter::DependsOn(NodeKey::new("k:b"))),
            sort: Some(CleaveSort::ValueDesc),
        }),
        active_lens: Some("flow".to_string()),
        ..SharedSkinState::default()
    };

    let json = serde_json::to_string(&state).expect("serialize shared state");
    let restored: SharedSkinState = serde_json::from_str(&json).expect("deserialize shared state");
    assert_eq!(state, restored);

    // GC to a world where only k:a is live.
    let live = HashSet::from([NodeKey::new("k:a")]);
    state.gc(&live);
    assert_eq!(state.selection_set, vec![NodeKey::new("k:a")]);
    assert!(state.pinned_keys.is_empty());
    assert_eq!(state.collapsed_keys, HashSet::from([NodeKey::new("k:a")]));
    assert_eq!(state.focus_key, Some(NodeKey::new("k:a")));
    // The cleave predicate referenced the now-dead k:b, so it is cleared.
    assert!(state.cleave.is_none());
}

#[test]
fn cleave_value_desc_keeps_numbers_before_non_numbers() {
    let mut text_node = node("k:t", "Text", NodeContentKind::Constant);
    text_node.computed_value = NodeValueProjection::Text("hello".to_string());
    let workspace = workspace_with_nodes(vec![
        numeric_node("k:a", "Alpha", "1"),
        text_node,
        numeric_node("k:b", "Beta", "3"),
    ]);

    let descending = CleavePredicate {
        filter: None,
        sort: Some(CleaveSort::ValueDesc),
    };
    // Numbers descend among themselves but the non-numeric value stays LAST —
    // descending must not invert the numeric-before-non-numeric class order.
    assert_eq!(
        workspace.cleave_filtered_keys(&descending),
        vec![
            NodeKey::new("k:b"),
            NodeKey::new("k:a"),
            NodeKey::new("k:t"),
        ]
    );
}

#[test]
fn effective_meta_walks_the_ancestor_chain() {
    let mut parent = node("k:m", "Meta", NodeContentKind::Constant);
    parent.is_meta = true;
    let mut child = node("k:mc", "Meta.Child", NodeContentKind::Constant);
    child.parent = Some(NodeId::new("Meta"));
    let regular = node("k:v", "Visible", NodeContentKind::Constant);
    let workspace = workspace_with_nodes(vec![parent, child, regular]);

    assert!(workspace.is_effective_meta(&NodeKey::new("k:m")));
    assert!(
        workspace.is_effective_meta(&NodeKey::new("k:mc")),
        "regular-flagged child under a meta parent is effectively meta"
    );
    assert!(!workspace.is_effective_meta(&NodeKey::new("k:v")));
    assert!(!workspace.is_effective_meta(&NodeKey::new("k:unknown")));
}

#[test]
fn cleave_predicate_filters_and_sorts_per_lens() {
    let workspace = workspace_with_nodes(vec![
        numeric_node("k:a", "Alpha", "3"),
        numeric_node("k:b", "Beta", "1"),
        numeric_node("k:c", "Gamma", "2"),
    ]);

    let ascending = CleavePredicate {
        filter: None,
        sort: Some(CleaveSort::ValueAsc),
    };
    assert_eq!(
        workspace.cleave_filtered_keys(&ascending),
        vec![
            NodeKey::new("k:b"),
            NodeKey::new("k:c"),
            NodeKey::new("k:a"),
        ]
    );

    let text = CleavePredicate {
        filter: Some(CleaveFilter::TextMatch("eta".to_string())),
        sort: None,
    };
    assert_eq!(
        workspace.cleave_filtered_keys(&text),
        vec![NodeKey::new("k:b")]
    );
}

#[test]
fn style_maps_calc_state_provenance_and_selection() {
    assert_eq!(
        calc_state_class(Some(NodeCalcStateProjection::Clean)),
        "dtc-calc--clean"
    );
    assert_eq!(
        calc_state_class(Some(NodeCalcStateProjection::RejectedPendingRepair)),
        "dtc-calc--rejected"
    );
    assert_eq!(calc_state_class(None), "dtc-calc--unknown");

    assert_eq!(selection_mode_class(true, true), "dtc-sel--editing");
    assert_eq!(selection_mode_class(true, false), "dtc-sel--selected");
    assert_eq!(selection_mode_class(false, false), "");

    let mut workspace = workspace_with_nodes(vec![numeric_node("k:a", "Alpha", "3")]);
    let published = workspace.node_by_key(&NodeKey::new("k:a")).unwrap().clone();
    assert_eq!(
        provenance_tint(&published, &workspace),
        ProvenanceTint::Published
    );

    // Active scenario + an override on the node ⇒ Scenario provenance.
    workspace.scenarios.active = Some("s1".to_string());
    let mut overridden = published.clone();
    overridden.scenario_override = Some(NodeValueProjection::Number {
        raw: "9".to_string(),
        display: "9".to_string(),
    });
    assert_eq!(
        provenance_tint(&overridden, &workspace),
        ProvenanceTint::Scenario
    );

    let mut pending = published;
    pending.computed_value = NodeValueProjection::Pending;
    assert_eq!(
        provenance_tint(&pending, &workspace),
        ProvenanceTint::Pending
    );
}

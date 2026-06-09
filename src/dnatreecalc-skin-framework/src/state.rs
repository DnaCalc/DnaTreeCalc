use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use leptos::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
use crate::workspace::{CleaveFilter, CleavePredicate};

/// The typed, persistable per-skin state a [`crate::WorkspaceSkin`] declares.
///
/// Mirrors `docs/ux/SKINS.md` §2.3. The walking skeleton uses only `Default`,
/// `Clone`, and `Send + Sync + 'static`; the `Serialize + DeserializeOwned`
/// bounds are present because persistence (W005 `dtc-osq.7`) and
/// schema-versioned migration (W006) need them — making them part of the
/// contract from the start keeps later worksets additive.
pub trait SkinState:
    Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static
{
    /// Current schema version of this state shape. Bumped when fields
    /// change incompatibly.
    fn schema_version() -> u32;

    /// Migrate a value persisted under an older schema version.
    ///
    /// The default rejects; skins making breaking changes override.
    fn migrate(
        _prior_version: u32,
        _prior_value: serde_json::Value,
    ) -> Result<Self, MigrationError> {
        Err(MigrationError::NoMigrationDefined)
    }

    /// Garbage-collect references to nodes that no longer exist.
    ///
    /// Called by the host periodically and on save; default no-op.
    fn gc(&mut self, _live_nodes: &HashSet<NodeKey>) {}
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MigrationError {
    #[error("no migration defined for this skin state")]
    NoMigrationDefined,
    #[error("migration failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SkinStatePersistenceKey {
    pub skin_id: String,
    pub slot: SkinMountSlot,
    pub workspace_id: String,
}

impl SkinStatePersistenceKey {
    #[must_use]
    pub fn new(skin_id: SkinId, slot: SkinMountSlot, workspace_id: impl Into<String>) -> Self {
        Self {
            skin_id: skin_id.as_str().to_string(),
            slot,
            workspace_id: workspace_id.into(),
        }
    }

    #[must_use]
    pub fn storage_key(&self) -> String {
        format!(
            "skin={};slot={};workspace={}",
            self.skin_id,
            self.slot.stable_id(),
            self.workspace_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedSkinStateRecord {
    pub schema_version: u32,
    pub value: serde_json::Value,
}

impl PersistedSkinStateRecord {
    #[must_use]
    pub fn new(schema_version: u32, value: serde_json::Value) -> Self {
        Self {
            schema_version,
            value,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SkinStatePersistenceError {
    #[error("skin state store failed while {operation}: {detail}")]
    Store {
        operation: &'static str,
        detail: String,
    },
    #[error("skin state serialization failed: {0}")]
    Serialize(String),
    #[error("skin state deserialization failed: {0}")]
    Deserialize(String),
    #[error("skin state migration from schema {from_version} failed: {detail}")]
    Migration { from_version: u32, detail: String },
}

pub trait SkinStatePersistenceStore: Send + Sync {
    fn load(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError>;

    fn save(
        &self,
        key: &SkinStatePersistenceKey,
        record: &PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError>;
}

#[derive(Default)]
pub struct InMemorySkinStatePersistenceStore {
    records: Mutex<BTreeMap<SkinStatePersistenceKey, PersistedSkinStateRecord>>,
}

impl InMemorySkinStatePersistenceStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &self,
        key: SkinStatePersistenceKey,
        record: PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError> {
        self.records
            .lock()
            .map_err(|_| SkinStatePersistenceError::Store {
                operation: "locking in-memory store",
                detail: "mutex poisoned".to_string(),
            })?
            .insert(key, record);
        Ok(())
    }

    pub fn get(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| SkinStatePersistenceError::Store {
                operation: "locking in-memory store",
                detail: "mutex poisoned".to_string(),
            })?
            .get(key)
            .cloned())
    }
}

impl SkinStatePersistenceStore for InMemorySkinStatePersistenceStore {
    fn load(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError> {
        self.get(key)
    }

    fn save(
        &self,
        key: &SkinStatePersistenceKey,
        record: &PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError> {
        self.insert(key.clone(), record.clone())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct LocalFileSkinStatePersistenceStore {
    root: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl LocalFileSkinStatePersistenceStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, key: &SkinStatePersistenceKey) -> PathBuf {
        self.root
            .join(safe_path_component(&key.skin_id))
            .join(key.slot.stable_id())
            .join(format!("{}.json", safe_path_component(&key.workspace_id)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SkinStatePersistenceStore for LocalFileSkinStatePersistenceStore {
    fn load(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError> {
        let path = self.path_for(key);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|error| SkinStatePersistenceError::Deserialize(error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(SkinStatePersistenceError::Store {
                operation: "reading local skin state",
                detail: error.to_string(),
            }),
        }
    }

    fn save(
        &self,
        key: &SkinStatePersistenceKey,
        record: &PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| SkinStatePersistenceError::Store {
                operation: "creating local skin state directory",
                detail: error.to_string(),
            })?;
        }
        let text = serde_json::to_string_pretty(record)
            .map_err(|error| SkinStatePersistenceError::Serialize(error.to_string()))?;
        std::fs::write(path, text).map_err(|error| SkinStatePersistenceError::Store {
            operation: "writing local skin state",
            detail: error.to_string(),
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn safe_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Clone)]
struct SkinStatePersistenceBinding {
    key: SkinStatePersistenceKey,
    store: Arc<dyn SkinStatePersistenceStore>,
}

/// Reactive handle over a skin's typed state.
///
/// Wraps a Leptos `RwSignal` so reads are tracked and writes fire updates.
/// The handle is `Copy` so closures and child components can capture it
/// freely; the underlying state lives in the Leptos arena.
pub struct SkinStateHandle<S: SkinState> {
    signal: RwSignal<S>,
    persistence: Option<SkinStatePersistenceBinding>,
    _marker: PhantomData<S>,
}

impl<S: SkinState> SkinStateHandle<S> {
    #[must_use]
    pub fn new(initial: S) -> Self {
        Self {
            signal: RwSignal::new(initial),
            persistence: None,
            _marker: PhantomData,
        }
    }

    pub fn from_persistence(
        key: SkinStatePersistenceKey,
        store: Arc<dyn SkinStatePersistenceStore>,
        live_nodes: &HashSet<NodeKey>,
    ) -> Result<Self, SkinStatePersistenceError> {
        let initial = load_skin_state::<S>(store.as_ref(), &key, live_nodes)?;
        let handle = Self {
            signal: RwSignal::new(initial),
            persistence: Some(SkinStatePersistenceBinding { key, store }),
            _marker: PhantomData,
        };
        handle.persist_current()?;
        Ok(handle)
    }

    /// Tracked read: callers in a reactive scope re-run on updates.
    pub fn with<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        self.signal.with(f)
    }

    /// Untracked read: convenient for tests; never subscribes the caller.
    pub fn get_untracked(&self) -> S {
        self.signal.get_untracked()
    }

    pub fn update(&self, f: impl FnOnce(&mut S)) {
        self.signal.update(f);
        self.persist_current()
            .expect("persisting skin state after update must succeed");
    }

    pub fn set(&self, value: S) {
        self.signal.set(value);
        self.persist_current()
            .expect("persisting skin state after set must succeed");
    }

    /// Expose the raw signal for advanced cases (e.g., derived memos).
    #[must_use]
    pub fn signal(&self) -> RwSignal<S> {
        self.signal
    }

    pub fn persist_current(&self) -> Result<(), SkinStatePersistenceError> {
        let Some(binding) = &self.persistence else {
            return Ok(());
        };
        let value = serde_json::to_value(self.signal.get_untracked())
            .map_err(|error| SkinStatePersistenceError::Serialize(error.to_string()))?;
        let record = PersistedSkinStateRecord::new(S::schema_version(), value);
        binding.store.save(&binding.key, &record)
    }
}

impl<S: SkinState> Clone for SkinStateHandle<S> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal,
            persistence: self.persistence.clone(),
            _marker: PhantomData,
        }
    }
}

fn load_skin_state<S: SkinState>(
    store: &dyn SkinStatePersistenceStore,
    key: &SkinStatePersistenceKey,
    live_nodes: &HashSet<NodeKey>,
) -> Result<S, SkinStatePersistenceError> {
    let mut state = match store.load(key)? {
        Some(record) if record.schema_version == S::schema_version() => {
            serde_json::from_value(record.value)
                .map_err(|error| SkinStatePersistenceError::Deserialize(error.to_string()))?
        }
        Some(record) => S::migrate(record.schema_version, record.value).map_err(|error| {
            SkinStatePersistenceError::Migration {
                from_version: record.schema_version,
                detail: error.to_string(),
            }
        })?,
        None => S::default(),
    };
    state.gc(live_nodes);
    Ok(state)
}

/// Cross-skin shared state stored under the `skins.shared` meta-namespace
/// (`docs/ux/SKINS.md` §2.4). The walking skeleton uses only the collapse
/// set and the pinned list; richer fields land with W003 (recent
/// selections) and W007 (per-skin format defaults).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceRecalcMode {
    #[default]
    Auto,
    Manual,
}

impl WorkspaceRecalcMode {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSkinState {
    pub tree_collapsed: HashSet<NodeId>,
    pub pinned: Vec<NodeId>,
    pub recalc_mode: WorkspaceRecalcMode,
    pub manual_recalc_pending: bool,
    pub workspace_ids: Vec<String>,
    pub active_workspace_id: Option<String>,

    // --- ATLAS suite continuity (NodeKey-keyed; survives lens switch) ---
    /// Multi-select set; the dispatcher-routed `SelectionState.primary` remains
    /// the auditable single anchor and is unchanged.
    #[serde(default)]
    pub selection_set: Vec<NodeKey>,
    /// Anchor for range/extend selection.
    #[serde(default)]
    pub selection_anchor: Option<NodeKey>,
    /// Collapse set shared across every lens (NodeKey-keyed).
    #[serde(default)]
    pub collapsed_keys: HashSet<NodeKey>,
    /// Pin set shared across every lens (NodeKey-keyed).
    #[serde(default)]
    pub pinned_keys: Vec<NodeKey>,
    /// Degree-of-interest center used by Flow's trace layout.
    #[serde(default)]
    pub focus_key: Option<NodeKey>,
    /// The shared filter/sort predicate (`cleave-predicate-shared`).
    #[serde(default)]
    pub cleave: Option<CleavePredicate>,
    /// The active lens's `SkinId` string, so chrome/console reflects it.
    #[serde(default)]
    pub active_lens: Option<String>,
}

impl SharedSkinState {
    /// Garbage-collect continuity references to nodes that no longer exist.
    /// Mirrors [`SkinState::gc`] but for the host-owned shared state.
    pub fn gc(&mut self, live_nodes: &HashSet<NodeKey>) {
        self.selection_set.retain(|key| live_nodes.contains(key));
        self.collapsed_keys.retain(|key| live_nodes.contains(key));
        self.pinned_keys.retain(|key| live_nodes.contains(key));
        if self
            .selection_anchor
            .as_ref()
            .is_some_and(|anchor| !live_nodes.contains(anchor))
        {
            self.selection_anchor = None;
        }
        if self
            .focus_key
            .as_ref()
            .is_some_and(|focus| !live_nodes.contains(focus))
        {
            self.focus_key = None;
        }
        let cleave_references_dead_node = matches!(
            &self.cleave,
            Some(CleavePredicate {
                filter: Some(CleaveFilter::DependsOn(target)),
                ..
            }) if !live_nodes.contains(target)
        );
        if cleave_references_dead_node {
            self.cleave = None;
        }
    }
}

/// Reactive handle to the shared meta-namespace state, mirrored from
/// [`SkinStateHandle`] but specialized for [`SharedSkinState`] so the
/// host can hand the same handle to every mounted skin.
pub struct SharedSkinStateHandle {
    signal: RwSignal<SharedSkinState>,
}

impl SharedSkinStateHandle {
    #[must_use]
    pub fn new(initial: SharedSkinState) -> Self {
        Self {
            signal: RwSignal::new(initial),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&SharedSkinState) -> R) -> R {
        self.signal.with(f)
    }

    pub fn get_untracked(&self) -> SharedSkinState {
        self.signal.get_untracked()
    }

    pub fn update(&self, f: impl FnOnce(&mut SharedSkinState)) {
        self.signal.update(f);
    }

    #[must_use]
    pub fn signal(&self) -> RwSignal<SharedSkinState> {
        self.signal
    }
}

impl Clone for SharedSkinStateHandle {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for SharedSkinStateHandle {}

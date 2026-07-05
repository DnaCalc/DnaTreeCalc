use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::identity::{NodeId, NodeKey, SkinId, SkinMountSlot};
use crate::workspace::{CleaveFilter, CleavePredicate};

/// The typed, persistable per-skin state a `WorkspaceSkin` declares.
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
    /// Human display names per `workspace_id` (`workspace_id` → name). The id
    /// stays the immutable key; this is the renamable label the title and
    /// switcher show, falling back to the id when a workspace has no entry.
    #[serde(default)]
    pub workspace_names: std::collections::BTreeMap<String, String>,
    /// Outcome of the most recent autosave: `Some(true)` saved, `Some(false)`
    /// failed, `None` nothing saved yet. Surfaced as the footer save pill.
    #[serde(default)]
    pub last_save_success: Option<bool>,

    // --- Cockpit composition (Phase B) ---
    /// The slot that currently owns keyboard focus in the cockpit.
    #[serde(default)]
    pub focused_slot: Option<SkinMountSlot>,
    /// Cross-lens hover highlight (shared-focus arbitration).
    #[serde(default)]
    pub hovered: Option<NodeKey>,
    /// Companion slots the shell currently has mounted, so a lens can adapt
    /// (e.g. collapse its embedded inspector when a Lens companion is open).
    #[serde(default)]
    pub companion_slots_active: Vec<SkinMountSlot>,
    /// The governing persona (tenet 9). Written by the HOST when a
    /// `SetPersona` intent is accepted — lenses read it to disable mutating
    /// affordances, but enforcement lives at the dispatcher, never here.
    #[serde(default)]
    pub persona: crate::permissions::Persona,
    /// Per-user keybinding remap layered over the universal grammar. Persisted
    /// and audited like everything else; the shell builds the active
    /// `KeybindingRegistry` from it via
    /// `with_overrides`, falling back to the universal table if a persisted
    /// map fails validation.
    #[serde(default)]
    pub keybinding_overrides: crate::keychord::KeybindingOverrideMap,

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

/// A typed mutation of [`SharedSkinState`] — the audited-shared-state
/// vocabulary. Every continuity change a lens or the shell makes goes through
/// [`SharedSkinStateHandle::apply`] as one of these, so shared view-state
/// mutations are observable and attributable like intents are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedStateChange {
    SetCleave(Option<CleavePredicate>),
    SetSelectionSet(Vec<NodeKey>),
    ToggleSelection(NodeKey),
    ClearSelectionSet,
    SetSelectionAnchor(Option<NodeKey>),
    Fold(NodeKey),
    Unfold(NodeKey),
    Pin(NodeKey),
    Unpin(NodeKey),
    SetFocusKey(Option<NodeKey>),
    SetHovered(Option<NodeKey>),
    SetActiveLens(String),
    SetRecalcMode(WorkspaceRecalcMode),
    SetManualRecalcPending(bool),
    SetWorkspaceIds(Vec<String>),
    SetActiveWorkspaceId(Option<String>),
    SetWorkspaceNames(std::collections::BTreeMap<String, String>),
    SetLastSaveSuccess(Option<bool>),
    SetFocusedSlot(Option<SkinMountSlot>),
    SetCompanionSlots(Vec<SkinMountSlot>),
    SetPersona(crate::permissions::Persona),
    SetKeybindingOverrides(crate::keychord::KeybindingOverrideMap),
}

/// Who asked for a shared-state change. Lenses identify by skin id + slot so
/// the audit trail distinguishes the same lens mounted in different slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedStateOrigin {
    Shell,
    Host,
    Lens {
        skin_id: String,
        slot: SkinMountSlot,
    },
}

impl SharedStateOrigin {
    #[must_use]
    pub fn lens(skin_id: SkinId, slot: SkinMountSlot) -> Self {
        Self::Lens {
            skin_id: skin_id.as_str().to_string(),
            slot,
        }
    }
}

/// One audited shared-state mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedStateAuditRecord {
    pub seq: u64,
    pub origin: SharedStateOrigin,
    pub change: SharedStateChange,
}

/// Maximum retained audit records; older entries are dropped.
pub const SHARED_AUDIT_CAPACITY: usize = 256;

impl SharedSkinState {
    /// Apply one typed change. Pure so the vocabulary is unit-testable.
    pub fn apply_change(&mut self, change: &SharedStateChange) {
        match change {
            SharedStateChange::SetCleave(cleave) => self.cleave = cleave.clone(),
            SharedStateChange::SetSelectionSet(keys) => self.selection_set = keys.clone(),
            SharedStateChange::ToggleSelection(key) => {
                if let Some(index) = self.selection_set.iter().position(|k| k == key) {
                    self.selection_set.remove(index);
                } else {
                    self.selection_set.push(key.clone());
                }
            }
            SharedStateChange::ClearSelectionSet => {
                self.selection_set.clear();
                self.selection_anchor = None;
            }
            SharedStateChange::SetSelectionAnchor(anchor) => {
                self.selection_anchor = anchor.clone();
            }
            SharedStateChange::Fold(key) => {
                self.collapsed_keys.insert(key.clone());
            }
            SharedStateChange::Unfold(key) => {
                self.collapsed_keys.remove(key);
            }
            SharedStateChange::Pin(key) => {
                if !self.pinned_keys.contains(key) {
                    self.pinned_keys.push(key.clone());
                }
            }
            SharedStateChange::Unpin(key) => {
                self.pinned_keys.retain(|k| k != key);
            }
            SharedStateChange::SetFocusKey(key) => self.focus_key = key.clone(),
            SharedStateChange::SetHovered(key) => self.hovered = key.clone(),
            SharedStateChange::SetActiveLens(id) => self.active_lens = Some(id.clone()),
            SharedStateChange::SetRecalcMode(mode) => self.recalc_mode = *mode,
            SharedStateChange::SetManualRecalcPending(pending) => {
                self.manual_recalc_pending = *pending;
            }
            SharedStateChange::SetWorkspaceIds(ids) => self.workspace_ids = ids.clone(),
            SharedStateChange::SetActiveWorkspaceId(id) => {
                self.active_workspace_id = id.clone();
            }
            SharedStateChange::SetWorkspaceNames(names) => {
                self.workspace_names = names.clone();
            }
            SharedStateChange::SetLastSaveSuccess(success) => {
                self.last_save_success = *success;
            }
            SharedStateChange::SetFocusedSlot(slot) => self.focused_slot = *slot,
            SharedStateChange::SetCompanionSlots(slots) => {
                self.companion_slots_active = slots.clone();
            }
            SharedStateChange::SetPersona(persona) => self.persona = *persona,
            SharedStateChange::SetKeybindingOverrides(overrides) => {
                self.keybinding_overrides = overrides.clone();
            }
        }
    }
}

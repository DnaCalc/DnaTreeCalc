use std::collections::HashSet;
use std::marker::PhantomData;

use leptos::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::identity::NodeId;

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
    fn gc(&mut self, _live_nodes: &HashSet<NodeId>) {}
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MigrationError {
    #[error("no migration defined for this skin state")]
    NoMigrationDefined,
    #[error("migration failed: {0}")]
    Failed(String),
}

/// Reactive handle over a skin's typed state.
///
/// Wraps a Leptos `RwSignal` so reads are tracked and writes fire updates.
/// The handle is `Copy` so closures and child components can capture it
/// freely; the underlying state lives in the Leptos arena.
pub struct SkinStateHandle<S: SkinState> {
    signal: RwSignal<S>,
    _marker: PhantomData<S>,
}

impl<S: SkinState> SkinStateHandle<S> {
    #[must_use]
    pub fn new(initial: S) -> Self {
        Self {
            signal: RwSignal::new(initial),
            _marker: PhantomData,
        }
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
    }

    pub fn set(&self, value: S) {
        self.signal.set(value);
    }

    /// Expose the raw signal for advanced cases (e.g., derived memos).
    #[must_use]
    pub fn signal(&self) -> RwSignal<S> {
        self.signal
    }
}

impl<S: SkinState> Clone for SkinStateHandle<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: SkinState> Copy for SkinStateHandle<S> {}

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

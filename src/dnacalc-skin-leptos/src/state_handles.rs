//! Leptos-signal-backed state handles.
//!
//! These wrap Leptos `RwSignal`s so reads are tracked and writes fire
//! updates; the pure `SkinState` contract, `SharedSkinState`, and the
//! persistence store types they build on live in `dnacalc_skin_ir::state`.
//! Split out of the IR crate so the wire-protocol layer carries no Leptos
//! dependency (bead dtc-ajl.1).

use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::identity::NodeKey;
use dnacalc_skin_ir::state::{
    PersistedSkinStateRecord, SHARED_AUDIT_CAPACITY, SharedSkinState, SharedStateAuditRecord,
    SharedStateChange, SharedStateOrigin, SkinState, SkinStatePersistenceError,
    SkinStatePersistenceKey, SkinStatePersistenceStore,
};

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

/// Reactive handle to the shared meta-namespace state, mirrored from
/// [`SkinStateHandle`] but specialized for [`SharedSkinState`] so the
/// host can hand the same handle to every mounted skin.
///
/// Mutations flow through [`SharedSkinStateHandle::apply`] — the audited
/// chokepoint. The raw `update` escape hatch remains for host-internal
/// bookkeeping (e.g. recent-selection maintenance) but suite code must use
/// typed changes.
pub struct SharedSkinStateHandle {
    signal: RwSignal<SharedSkinState>,
    audit: RwSignal<std::collections::VecDeque<SharedStateAuditRecord>>,
    next_seq: RwSignal<u64>,
}

impl SharedSkinStateHandle {
    #[must_use]
    pub fn new(initial: SharedSkinState) -> Self {
        Self {
            signal: RwSignal::new(initial),
            audit: RwSignal::new(std::collections::VecDeque::new()),
            next_seq: RwSignal::new(0),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&SharedSkinState) -> R) -> R {
        self.signal.with(f)
    }

    pub fn get_untracked(&self) -> SharedSkinState {
        self.signal.get_untracked()
    }

    /// Apply one typed, attributed change — the audited mutation path.
    pub fn apply(&self, change: SharedStateChange, origin: SharedStateOrigin) {
        let seq = self.next_seq.get_untracked();
        self.next_seq.set(seq + 1);
        self.audit.update(|ring| {
            ring.push_back(SharedStateAuditRecord {
                seq,
                origin,
                change: change.clone(),
            });
            while ring.len() > SHARED_AUDIT_CAPACITY {
                ring.pop_front();
            }
        });
        self.signal.update(|state| state.apply_change(&change));
    }

    /// Snapshot of the audit ring (newest last).
    #[must_use]
    pub fn audit_log(&self) -> Vec<SharedStateAuditRecord> {
        self.audit
            .with_untracked(|ring| ring.iter().cloned().collect())
    }

    /// Raw mutation escape hatch — host-internal bookkeeping only; suite code
    /// uses [`Self::apply`] so changes stay audited.
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

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::identity::{NodeKey, SkinId, SkinMountSlot};
use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta};
use dnacalc_skin_ir::manifest::{SkinCapabilities, SkinManifest};
use dnacalc_skin_ir::preview::PreviewService;
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SkinState, SkinStatePersistenceKey, SkinStatePersistenceStore};
use dnacalc_skin_ir::workspace::WorkspaceState;

use crate::state_handles::{SharedSkinStateHandle, SkinStateHandle};
use crate::theme::ThemeTokens;

/// Typed context passed to a [`WorkspaceSkin`] at mount time.
///
/// Skins read `workspace`/`selection` through tracked signals so reactive
/// renders fire when the host publishes changes; they write through
/// `dispatch`. Per-skin typed state lives in `state`; cross-skin state in
/// `shared`. The walking skeleton omits the `profile` and `format` fields
/// from `docs/ux/SKINS.md` §2.2 — those land with W004/W007.
#[derive(Clone)]
pub struct SkinContext<S: SkinState> {
    pub workspace: ReadSignal<WorkspaceState>,
    pub latest_delta: ReadSignal<WorkspaceDelta>,
    pub selection: ReadSignal<SelectionState>,
    pub shared: SharedSkinStateHandle,
    pub tokens: ThemeTokens,
    /// The slot this mount occupies — lenses use it as their audited
    /// shared-state origin and to adapt to cockpit composition.
    pub slot: SkinMountSlot,
    pub state: SkinStateHandle<S>,
    pub dispatch: Arc<dyn Dispatcher>,
    /// Non-mutating foresight (tenet 7). `None` in minimal hosts/tests;
    /// lenses degrade to post-attempt receipt feedback.
    pub preview: Option<Arc<dyn PreviewService>>,
}

/// Type-erased context the registry hands to an [`ErasedSkinFactory`].
///
/// Erasure happens at the registry boundary so the registry can store
/// `Vec<RegisteredSkin>` without leaking each skin's associated state type.
/// Concrete state is re-typed inside the factory's `mount` (it knows `S`).
#[derive(Clone)]
pub struct ErasedSkinContext {
    pub workspace: ReadSignal<WorkspaceState>,
    pub latest_delta: ReadSignal<WorkspaceDelta>,
    pub selection: ReadSignal<SelectionState>,
    pub shared: SharedSkinStateHandle,
    pub tokens: ThemeTokens,
    pub slot: SkinMountSlot,
    pub skin_state_store: Arc<dyn SkinStatePersistenceStore>,
    pub dispatch: Arc<dyn Dispatcher>,
    pub preview: Option<Arc<dyn PreviewService>>,
}

/// What a mounted skin returns to the host: a renderable view and an
/// optional lifecycle hook that runs on teardown.
///
/// The spec's `event_handler` field (`SKINS.md` §2.9) is deferred until
/// a skin needs targeted incremental updates outside the workspace
/// signal — the skeleton's reactive Leptos pattern covers re-renders.
/// The hook is `Send + Sync` because Leptos's `on_cleanup` runs through
/// the reactive owner, which carries those bounds.
pub struct SkinHandle {
    pub view: AnyView,
    pub on_deactivate: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl SkinHandle {
    #[must_use]
    pub fn new(view: AnyView) -> Self {
        Self {
            view,
            on_deactivate: None,
        }
    }

    #[must_use]
    pub fn with_on_deactivate(mut self, hook: impl FnOnce() + Send + Sync + 'static) -> Self {
        self.on_deactivate = Some(Box::new(hook));
        self
    }
}

/// The core trait every skin implements.
///
/// Not object-safe (carries an associated `State` type) — the registry
/// stores [`RegisteredSkin`] which wraps a typed factory.
pub trait WorkspaceSkin: Send + Sync + 'static {
    /// The typed state this skin persists. See [`SkinState`].
    type State: SkinState;

    /// Stable identifier; the meta-namespace path component and the
    /// persistent skin handle.
    fn id(&self) -> SkinId;

    /// Display-time identity.
    fn manifest(&self) -> SkinManifest;

    /// Declared capabilities (drives feature-detection in the shell).
    fn capabilities(&self) -> SkinCapabilities;

    /// Build and mount the skin's root UI.
    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle;
}

/// Object-safe factory the registry stores per skin.
pub trait ErasedSkinFactory: Send + Sync {
    fn id(&self) -> SkinId;
    fn manifest(&self) -> SkinManifest;
    fn capabilities(&self) -> SkinCapabilities;
    fn mount(&self, cx: ErasedSkinContext) -> SkinHandle;
}

struct TypedFactory<K: WorkspaceSkin> {
    skin: Arc<K>,
}

impl<K: WorkspaceSkin> ErasedSkinFactory for TypedFactory<K> {
    fn id(&self) -> SkinId {
        self.skin.id()
    }

    fn manifest(&self) -> SkinManifest {
        self.skin.manifest()
    }

    fn capabilities(&self) -> SkinCapabilities {
        self.skin.capabilities()
    }

    fn mount(&self, cx: ErasedSkinContext) -> SkinHandle {
        let workspace_snapshot = cx.workspace.get_untracked();
        let live_node_keys = workspace_snapshot
            .key_order
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<NodeKey>>();
        let persistence_key = SkinStatePersistenceKey::new(
            self.skin.id(),
            cx.slot,
            workspace_snapshot.workspace_id.clone(),
        );
        let typed_state = SkinStateHandle::<K::State>::from_persistence(
            persistence_key,
            cx.skin_state_store.clone(),
            &live_node_keys,
        )
        .expect("loading persisted skin state must succeed");
        let typed_cx = SkinContext {
            workspace: cx.workspace,
            latest_delta: cx.latest_delta,
            selection: cx.selection,
            shared: cx.shared,
            tokens: cx.tokens,
            slot: cx.slot,
            state: typed_state,
            dispatch: cx.dispatch,
            preview: cx.preview,
        };
        self.skin.mount(typed_cx)
    }
}

/// What the registry stores: erased mount factory plus pure-data manifest /
/// capabilities for cheap inspection without mounting.
#[derive(Clone)]
pub struct RegisteredSkin {
    id: SkinId,
    manifest: SkinManifest,
    capabilities: SkinCapabilities,
    factory: Arc<dyn ErasedSkinFactory>,
}

impl RegisteredSkin {
    pub fn from_skin<K: WorkspaceSkin>(skin: K) -> Self {
        let manifest = skin.manifest();
        let capabilities = skin.capabilities();
        let id = skin.id();
        Self {
            id,
            manifest,
            capabilities,
            factory: Arc::new(TypedFactory {
                skin: Arc::new(skin),
            }),
        }
    }

    #[must_use]
    pub fn id(&self) -> SkinId {
        self.id
    }

    #[must_use]
    pub fn manifest(&self) -> &SkinManifest {
        &self.manifest
    }

    #[must_use]
    pub fn capabilities(&self) -> &SkinCapabilities {
        &self.capabilities
    }

    /// Mount this skin with the erased context. Wraps the factory call
    /// so callers do not need to know about the erased trait.
    pub fn mount(&self, cx: ErasedSkinContext) -> SkinHandle {
        self.factory.mount(cx)
    }

    /// Capability-manifest negotiation for a slot: refuse loudly before
    /// mounting rather than rendering wrongly.
    pub fn negotiate_slot(
        &self,
        slot: SkinMountSlot,
    ) -> Result<(), dnacalc_skin_ir::manifest::CapabilityError> {
        if self.capabilities.slot_allowed(slot) {
            Ok(())
        } else {
            Err(
                dnacalc_skin_ir::manifest::CapabilityError::SlotNotSupported {
                    skin_id: self.id.as_str().to_string(),
                    slot,
                },
            )
        }
    }
}

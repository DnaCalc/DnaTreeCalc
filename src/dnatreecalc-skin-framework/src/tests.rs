//! Framework-level tests over a fake "inert" skin.
//!
//! Concrete skin implementations live in `dnatreecalc-skins`; integration
//! tests that mount the real skins through the shell + a direct host session
//! live in `dnatreecalc-host` under `tests/`. The point of this module is
//! to prove the trait/registry contract without dragging the rest of the
//! workspace into a circular dep.

use std::sync::Arc;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::identity::{NodeId, SkinId};
use crate::intent::{
    Dispatcher, InMemoryDispatcher, IntentReceipt, WorkspaceDelta, WorkspaceIntent,
};
use crate::manifest::{SkinCapabilities, SkinCategory, SkinManifest};
use crate::registry::SkinRegistry;
use crate::selection::SelectionState;
use crate::skin::{ErasedSkinContext, SkinContext, SkinHandle, WorkspaceSkin};
use crate::state::{SharedSkinState, SharedSkinStateHandle, SkinState};
use crate::workspace::WorkspaceState;

#[derive(Default, Clone, Serialize, Deserialize)]
struct InertState {
    pub mounts: u32,
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

    let cx = ErasedSkinContext {
        workspace: workspace.read_only(),
        latest_delta: latest_delta.read_only(),
        selection: selection.read_only(),
        shared,
        dispatch: dispatcher,
    };

    let skin = registry.get(id).expect("registered skin must resolve");
    let handle = skin.mount(cx);
    assert!(handle.on_deactivate.is_none());
    // The view itself is opaque AnyView; reaching it would require a
    // rendered tree which the framework crate deliberately does not
    // depend on. Skin behavior under render is covered by the shell-
    // level integration tests.
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

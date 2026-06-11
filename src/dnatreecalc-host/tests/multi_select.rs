//! B.3.4 proof: population selection is a dispatched, audited intent. The
//! host validates keys against the live projection, mirrors the set into
//! shared state (Host origin), and a stale key is a typed rejection.

use std::sync::Arc;

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, NodeKey, SelectionState, SharedSkinState, SharedSkinStateHandle,
    SharedStateChange, SharedStateOrigin, WorkspaceDelta, WorkspaceIntent,
};
use leptos::prelude::*;

#[test]
fn select_nodes_validates_mirrors_and_audits() {
    let _owner = Owner::new();
    let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let session = Arc::new(std::sync::Mutex::new(
        TreeWorkspaceSession::from_model(&model).unwrap(),
    ));
    let workspace_state = session.lock().unwrap().workspace_state().unwrap();
    // Pick two real keys from the projection.
    let keys: Vec<NodeKey> = workspace_state.key_order.iter().take(2).cloned().collect();
    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let shared = SharedSkinStateHandle::new(SharedSkinState::default());
    let dispatcher = Arc::new(HostDispatcher::with_session_and_shared(
        selection,
        workspace,
        latest_delta,
        session,
        Some(shared),
    ));

    // Accepted: real keys land in shared state with the anchor, audited Host.
    let receipt = dispatcher.dispatch(WorkspaceIntent::SelectNodes {
        keys: keys.clone(),
        anchor: Some(keys[0].clone()),
    });
    assert!(receipt.accepted);
    let state = shared.get_untracked();
    assert_eq!(state.selection_set, keys);
    assert_eq!(state.selection_anchor, Some(keys[0].clone()));
    assert!(shared.audit_log().iter().any(|record| {
        record.origin == SharedStateOrigin::Host
            && matches!(record.change, SharedStateChange::SetSelectionSet(_))
    }));

    // Rejected: a stale key is a typed rejection and shared state is untouched.
    let stale = dispatcher.dispatch(WorkspaceIntent::SelectNodes {
        keys: vec![NodeKey::new("tree-node:99999")],
        anchor: None,
    });
    assert!(matches!(stale.error, Some(IntentError::UnknownNode { .. })));
    assert_eq!(shared.get_untracked().selection_set, keys);

    // The whole exchange is in the audited intent log.
    let records = dispatcher.intent_records();
    assert_eq!(records.len(), 2);
    assert!(records[0].accepted);
    assert!(!records[1].accepted);
}

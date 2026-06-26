//! B.3.2 persona-gating proof: the dispatcher chokepoint enforces the
//! persona policy per intent BEFORE any host or engine work (tenet 9), the
//! rejection is typed (`IntentError::Forbidden`), and persona switches are
//! audited intents reflected into shared state.

use std::sync::Arc;

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, NodeId, Persona, SelectionState, SharedSkinState,
    SharedSkinStateHandle, SharedStateChange, WorkspaceDelta, WorkspaceIntent,
};
use leptos::prelude::*;

fn live_dispatcher() -> (Arc<HostDispatcher>, SharedSkinStateHandle) {
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
    let dispatcher = Arc::new(HostDispatcher::with_session_and_shared(
        selection,
        workspace,
        latest_delta,
        session,
        Some(shared),
    ));
    (dispatcher, shared)
}

#[test]
fn reviewer_persona_blocks_mutation_but_allows_notes_and_speculation() {
    let _owner = Owner::new();
    let (dispatcher, shared) = live_dispatcher();

    let switched = dispatcher.dispatch(WorkspaceIntent::SetPersona {
        persona: Persona::Reviewer,
    });
    assert!(switched.accepted);
    assert_eq!(shared.get_untracked().persona, Persona::Reviewer);
    // The persona change is reflected through the audited chokepoint.
    assert!(shared.audit_log().iter().any(|record| matches!(
        record.change,
        SharedStateChange::SetPersona(Persona::Reviewer)
    )));

    // A content edit is forbidden, typed, and touches nothing.
    let edit = dispatcher.dispatch(WorkspaceIntent::EditContent {
        node: NodeId::new("Accounts.2005.Q1.Sales"),
        content: "99".to_string(),
    });
    assert!(!edit.accepted);
    assert_eq!(
        edit.error,
        Some(IntentError::Forbidden {
            persona: "reviewer".to_string()
        })
    );

    // Annotating and non-publishing speculation remain available.
    let note = dispatcher.dispatch(WorkspaceIntent::SetNote {
        node: dnatreecalc_skin_framework::NodeKey::new("tree-node:6"),
        note: Some("reviewer was here".to_string()),
    });
    assert!(note.accepted, "reviewer note: {:?}", note.error);
    let candidate = dispatcher.dispatch(WorkspaceIntent::OpenCandidate { parent: None });
    assert!(
        candidate.accepted,
        "reviewer candidate: {:?}",
        candidate.error
    );
    // But committing the candidate would publish — forbidden.
    let commit = dispatcher.dispatch(WorkspaceIntent::CommitCandidate {
        handle: "candidate:any".to_string(),
    });
    assert!(matches!(commit.error, Some(IntentError::Forbidden { .. })));
}

#[test]
fn read_only_persona_blocks_recalculate_but_allows_selection() {
    let _owner = Owner::new();
    let (dispatcher, _shared) = live_dispatcher();

    dispatcher.dispatch(WorkspaceIntent::SetPersona {
        persona: Persona::ReadOnly,
    });

    let recalc = dispatcher.dispatch(WorkspaceIntent::Recalculate);
    assert_eq!(
        recalc.error,
        Some(IntentError::Forbidden {
            persona: "read_only".to_string()
        })
    );

    let select = dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(
        "Accounts.2005.Q1.Sales",
    ))));
    assert!(select.accepted);

    // Switching back to Author restores full capability.
    dispatcher.dispatch(WorkspaceIntent::SetPersona {
        persona: Persona::Author,
    });
    let recalc = dispatcher.dispatch(WorkspaceIntent::Recalculate);
    assert!(recalc.accepted, "author recalc: {:?}", recalc.error);
}

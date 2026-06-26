//! B.3.3 intent-log-replay proof: the dispatcher records every dispatch with
//! its outcome, persona, and resulting value epoch; replaying the records
//! onto a FRESH session of the same fixture reproduces the workspace
//! deterministically — including rejections rejecting again.

use std::sync::Arc;

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    Dispatcher, InitialNodeContentProjection, NodeId, Persona, SelectionState, WorkspaceDelta,
    WorkspaceIntent, replay,
};
use leptos::prelude::*;

fn fresh_dispatcher() -> (
    Arc<HostDispatcher>,
    RwSignal<dnatreecalc_skin_framework::WorkspaceState>,
) {
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
    let dispatcher = Arc::new(HostDispatcher::with_session(
        selection,
        workspace,
        latest_delta,
        session,
    ));
    (dispatcher, workspace)
}

#[test]
fn recorded_session_replays_deterministically_onto_a_fresh_fixture() {
    let _owner = Owner::new();

    // --- Record a working session: edits, structure, a persona dance with a
    // forbidden attempt, and a recalc.
    let (original, original_ws) = fresh_dispatcher();
    original.dispatch(WorkspaceIntent::EditContent {
        node: NodeId::new("Accounts.2005.Q1.Income.Sales"),
        content: "42".to_string(),
    });
    original.dispatch(WorkspaceIntent::AddNode {
        parent: Some(NodeId::new("Accounts.2005")),
        symbol: "Q3".to_string(),
        initial: InitialNodeContentProjection::Literal {
            content: "7".to_string(),
        },
        is_meta: false,
    });
    original.dispatch(WorkspaceIntent::SetPersona {
        persona: Persona::ReadOnly,
    });
    // Forbidden under ReadOnly — recorded as rejected.
    let forbidden = original.dispatch(WorkspaceIntent::EditContent {
        node: NodeId::new("Accounts.2005.Q1.Income.Sales"),
        content: "0".to_string(),
    });
    assert!(!forbidden.accepted);
    original.dispatch(WorkspaceIntent::SetPersona {
        persona: Persona::Author,
    });
    original.dispatch(WorkspaceIntent::Recalculate);

    let records = original.intent_records();
    assert_eq!(records.len(), 6);
    assert!(records[3].error.is_some(), "the forbidden edit is recorded");
    assert_eq!(records[3].persona, Persona::ReadOnly);

    // The log is exportable: serde round-trip.
    let exported = serde_json::to_string(&records).expect("records serialize");
    let imported: Vec<dnatreecalc_skin_framework::IntentRecord> =
        serde_json::from_str(&exported).expect("records deserialize");

    // --- Replay onto a fresh session of the same fixture.
    let (fresh, fresh_ws) = fresh_dispatcher();
    let outcome = replay(&imported, fresh.as_ref());
    assert_eq!(outcome.dispatched, 6);
    assert!(
        outcome.mismatches.is_empty(),
        "replay diverged at seqs {:?}",
        outcome.mismatches
    );

    // The replayed workspace matches the original: same node population and
    // the same published values where it matters.
    let original_state = original_ws.get_untracked();
    let fresh_state = fresh_ws.get_untracked();
    assert_eq!(original_state.len(), fresh_state.len());
    for path in [
        "Accounts.2005.Q1.Income.Sales",
        "Accounts.2005.Q3",
        "Accounts.2005.Total",
    ] {
        let id = NodeId::new(path);
        assert_eq!(
            original_state.node(&id).map(|n| n.computed_value.clone()),
            fresh_state.node(&id).map(|n| n.computed_value.clone()),
            "published value diverged at {path}"
        );
    }
}

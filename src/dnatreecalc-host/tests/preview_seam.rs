//! B.3.1 preview-seam proof: the live dispatcher answers non-mutating
//! foresight through the framework `PreviewService` trait (tenet 7 — predict
//! before you pay), and a preview NEVER mutates engine or projection state.

use std::sync::Arc;

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    MutationImpactBlockedReasonProjection, MutationImpactIntentProjection, NodeId, PreviewService,
    SelectionState, WorkspaceDelta,
};
use leptos::prelude::*;

fn live_preview_service() -> (
    Arc<dyn PreviewService>,
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
fn formula_bind_preview_reports_typed_diagnostics_without_mutating() {
    let _owner = Owner::new();
    let (preview, workspace) = live_preview_service();
    let before = workspace.get_untracked();

    // A syntactically broken formula yields a typed, illegal verdict.
    let bad = preview
        .preview_formula_bind(&NodeId::new("Accounts.2005.Q1.Margin"), "=Sales*")
        .expect("preview must answer");
    assert!(!bad.legal, "trailing operator must not bind");
    assert!(
        !bad.diagnostics.is_empty(),
        "an illegal verdict carries typed diagnostics"
    );

    // A healthy formula binds clean (Income and Net are Margin's siblings).
    let good = preview
        .preview_formula_bind(&NodeId::new("Accounts.2005.Q1.Margin"), "=Net/Income")
        .expect("preview must answer");
    assert!(good.legal, "diagnostics: {:?}", good.diagnostics);

    // Previews are pure observers: nothing republished, nothing mutated.
    let after = workspace.get_untracked();
    assert_eq!(before.projection_seq, after.projection_seq);
    assert_eq!(
        before.revision.workspace_revision_id,
        after.revision.workspace_revision_id
    );
}

#[test]
fn mutation_impact_preview_reports_collisions_and_invalidation_without_mutating() {
    let _owner = Owner::new();
    let (preview, workspace) = live_preview_service();
    let before = workspace.get_untracked();

    // Renaming Q1.Margin to an existing sibling name is blocked by collision.
    let collision = preview
        .preview_mutation_impact(&MutationImpactIntentProjection::RenameNode {
            node: NodeId::new("Accounts.2005.Q1.Margin"),
            new_symbol: "Net".to_string(),
        })
        .expect("preview must answer");
    assert!(!collision.legal);
    assert_eq!(
        collision.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::NameCollision)
    );
    assert!(!collision.collisions.is_empty());

    // Editing a depended-upon node reports a non-empty invalidation plan.
    let impact = preview
        .preview_mutation_impact(&MutationImpactIntentProjection::EditContent {
            node: NodeId::new("Accounts.2005.Q1.Net"),
            content: "=Income*Margin*2".to_string(),
        })
        .expect("preview must answer");
    assert!(
        impact.legal,
        "edit should be legal: {:?}",
        impact.bind_diagnostics
    );
    assert!(
        !impact.invalidation_plan.invalidated_nodes.is_empty(),
        "editing Q1.Net must show downstream invalidation (Total depends on it)"
    );

    let after = workspace.get_untracked();
    assert_eq!(before.projection_seq, after.projection_seq);
    assert_eq!(
        before.revision.workspace_revision_id,
        after.revision.workspace_revision_id
    );
}

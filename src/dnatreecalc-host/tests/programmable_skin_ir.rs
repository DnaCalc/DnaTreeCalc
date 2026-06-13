mod support;

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{
    TableCellFixture, TableColumnBodyFixture, TableColumnBodyKind, TableColumnFixture,
    TableIdentityPolicyFixture, TableNodeFixture, TableRowFixture, TableSectionFixture,
    WorkspaceFixture, WorkspaceNodeFixture,
};
use dnatreecalc_skin_framework::{
    ActiveSelectionDetailProjection, AuthoringScope, CalcRunStateProjection, CandidateProjection,
    ClipboardOperationProjection, ClipboardPayloadKind, ClipboardPayloadProjection,
    CommandIntentKindProjection, ComparativeSourceProjection, FormulaBindPreviewDiagnosticStage,
    FormulaBindPreviewInputKind, FormulaReferenceInsertionTarget, InitialNodeContentProjection,
    IntentError, InvalidationReasonProjection, MutationImpactBlockedReasonProjection,
    MutationImpactIntentProjection, NodeAttributePatch, NodeContentKind, NodeId,
    NodeValueProjection, RecalcPlanMutation, ReferenceTargetProjection,
    RuntimeEffectFamilyProjection, RuntimeOverlayKindProjection, ScenarioSourceProjection,
    SweepPointInput, TableCellEditabilityProjection, TableCellRegionProjection,
    TableColumnBodyProjection, TableDependencyFactKindProjection,
    TableDependencyFactStatusProjection, TreeReferenceCollectionFamilyProjection,
    WorkspaceDeltaChange, WorkspaceRecalcMode,
};

use support::programmable::{Harness, revision_fingerprint};

fn candidate_children(candidate: &CandidateProjection, node: &str) -> Vec<NodeId> {
    candidate
        .nodes
        .iter()
        .find(|candidate_node| candidate_node.id == NodeId::new(node))
        .unwrap_or_else(|| panic!("{node} should project in candidate"))
        .children
        .clone()
}

fn sweep_point(point_id: &str, label: &str, raw: &str) -> SweepPointInput {
    SweepPointInput {
        point_id: point_id.to_string(),
        label: label.to_string(),
        value: NodeValueProjection::Number {
            raw: raw.to_string(),
            display: raw.to_string(),
        },
    }
}

#[test]
fn programmable_skin_reads_command_catalog_from_projected_state() {
    let harness = Harness::empty();
    let skin = &harness.driver;

    skin.add_node(None, "Input", "10");
    skin.add_node(None, "Output", "=Input*2");
    let input_key = skin
        .state()
        .node(&NodeId::new("Input"))
        .expect("input node projects")
        .key
        .clone();

    skin.select(None);
    let catalog = skin.command_catalog();
    assert!(
        !catalog
            .get(CommandIntentKindProjection::RenameNode)
            .expect("rename command is cataloged")
            .enabled
    );

    skin.select(Some("Output"));
    let selected_catalog = skin.command_catalog();
    assert!(
        selected_catalog
            .get(CommandIntentKindProjection::RenameNode)
            .expect("rename command is cataloged")
            .enabled
    );
    assert!(
        selected_catalog
            .get(CommandIntentKindProjection::CopyFormula)
            .expect("copy formula command is cataloged")
            .enabled
    );
    assert_eq!(
        selected_catalog
            .get(CommandIntentKindProjection::Recalculate)
            .expect("recalculate command is cataloged")
            .effective_binding,
        Some("F9")
    );

    skin.try_copy_to_clipboard(
        AuthoringScope::Node(input_key.clone()),
        ClipboardPayloadKind::Values,
    );
    let clipboard_catalog = skin.command_catalog();
    assert!(
        clipboard_catalog
            .get(CommandIntentKindProjection::PasteClipboardValues)
            .expect("paste values command is cataloged")
            .enabled
    );

    let open = skin.try_open_candidate();
    assert!(open.accepted);
    let candidate_handle = skin
        .state()
        .candidates
        .first()
        .expect("candidate projects")
        .handle
        .clone();
    skin.try_evaluate_candidate(&candidate_handle);
    let candidate_catalog = skin.command_catalog();
    assert!(
        candidate_catalog
            .get(CommandIntentKindProjection::CommitCandidate)
            .expect("commit candidate command is cataloged")
            .enabled
    );

    assert!(
        skin.try_create_scenario("scenario:base", "Base", None)
            .accepted
    );
    assert!(
        skin.try_create_scenario_sweep(
            "sweep:input",
            "Input Sweep",
            Some("scenario:base"),
            input_key,
            vec![
                sweep_point("low", "Low", "5"),
                sweep_point("high", "High", "15")
            ],
        )
        .accepted
    );
    let what_if_catalog = skin.command_catalog();
    assert!(
        what_if_catalog
            .get(CommandIntentKindProjection::DeleteScenario)
            .expect("delete scenario command is cataloged")
            .enabled
    );
    assert!(
        what_if_catalog
            .get(CommandIntentKindProjection::DeleteSweep)
            .expect("delete sweep command is cataloged")
            .enabled
    );
}

#[test]
fn programmable_skin_uses_projected_template_initial_content() {
    let harness = Harness::empty();
    let skin = &harness.driver;

    let state = skin.state();
    let starter = state
        .templates
        .entries
        .iter()
        .find(|template| template.template_id == "starter")
        .expect("starter template projects")
        .clone();
    assert!(starter.built_in);
    assert_eq!(starter.name, "Starter Formula");
    assert_eq!(starter.preview_content.as_deref(), Some("=1+1"));

    let receipt = skin.try_add_node_initial(None, "FromTemplate", starter.initial, false);
    assert!(receipt.accepted, "{:?}", receipt.error);
    let state = skin.state();
    let templated = state
        .node(&NodeId::new("FromTemplate"))
        .expect("template-created node projects");
    assert_eq!(templated.content_kind, NodeContentKind::Formula);
    assert_eq!(templated.content_text, "=1+1");
    assert_eq!(
        state
            .templates
            .entries
            .iter()
            .map(|template| template.template_id.as_str())
            .collect::<Vec<_>>(),
        vec!["starter", "input-zero"]
    );
}

#[test]
fn programmable_skin_commands_route_through_host_dispatcher() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    assert!(skin.try_add_node(None, "Root", "").accepted);
    assert!(skin.try_add_node(Some("Root"), "A", "3").accepted);
    assert!(skin.try_add_node(Some("Root"), "B", "=A+1").accepted);

    let state = skin.state();
    assert_eq!(state.node_order.len(), 3);
    assert_eq!(state.root_paths, vec![NodeId::new("Root")]);
    assert_eq!(
        state.node(&NodeId::new("Root.A")).unwrap().content_text,
        "3"
    );
    assert_eq!(
        state.node(&NodeId::new("Root.B")).unwrap().content_kind,
        NodeContentKind::Formula
    );
    skin.assert_scalar("Root.A", "3");
    skin.assert_scalar("Root.B", "4");
}

#[test]
fn programmable_skin_projection_refreshes_after_each_calc_affecting_intent() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    let after_root = skin.state().revision.clone();
    assert!(after_root.structural_snapshot_id.is_some());
    assert!(after_root.workspace_revision_id.is_some());

    skin.add_node(Some("Root"), "A", "3");
    let after_add = skin.state().revision.clone();
    assert_ne!(
        after_root.structural_snapshot_id,
        after_add.structural_snapshot_id
    );
    assert_ne!(
        after_root.workspace_revision_id,
        after_add.workspace_revision_id
    );

    skin.edit("Root.A", "4");
    let after_edit = skin.state().revision.clone();
    assert_eq!(
        after_add.structural_snapshot_id,
        after_edit.structural_snapshot_id
    );
    assert_ne!(
        after_add.node_input_snapshot_id,
        after_edit.node_input_snapshot_id
    );
    assert_ne!(
        after_add.workspace_revision_id,
        after_edit.workspace_revision_id
    );
    skin.assert_scalar("Root.A", "4");
    assert!(skin.state().last_run.is_some());
}

#[test]
fn programmable_skin_projects_and_navigates_retained_revision_history() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let original = skin.state();
    let original_revision = original
        .revision
        .workspace_revision_id
        .clone()
        .expect("original revision should project");
    assert_eq!(
        original.revision_history.current_revision_id.as_deref(),
        Some(original_revision.as_str())
    );
    assert!(
        original
            .revision_history
            .entries
            .iter()
            .any(|entry| entry.revision_id == original_revision && entry.is_current)
    );
    skin.assert_scalar("Root.B", "2");

    skin.edit("Root.A", "5");
    let edited = skin.state();
    let edited_revision = edited
        .revision
        .workspace_revision_id
        .clone()
        .expect("edited revision should project");
    assert_ne!(edited_revision, original_revision);
    skin.assert_scalar("Root.B", "6");
    let edited_a_key = edited
        .node(&NodeId::new("Root.A"))
        .expect("Root.A should project")
        .key
        .clone();
    let edited_b_key = edited
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    let edited_summary = edited
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == edited_revision)
        .and_then(|entry| entry.transaction_summary.as_ref())
        .expect("edited revision should project transaction invalidation summary");
    assert!(!edited_summary.transaction_id.is_empty());
    assert_eq!(
        edited_summary.estimated_node_count,
        edited_summary.invalidated_nodes.len()
    );
    assert!(
        edited_summary
            .invalidated_nodes
            .iter()
            .any(|entry| entry.node == edited_a_key && !entry.reasons.is_empty())
    );
    assert!(edited_summary.invalidated_nodes.iter().any(|entry| {
        entry.node == edited_b_key
            && entry
                .reasons
                .contains(&InvalidationReasonProjection::UpstreamPublication)
    }));

    let receipt = skin.navigate_revision(&original_revision);
    assert!(receipt.accepted);
    assert_eq!(
        receipt.produced_revision.as_deref(),
        Some(original_revision.as_str())
    );
    let restored = skin.state();
    assert_eq!(
        restored.revision.workspace_revision_id.as_deref(),
        Some(original_revision.as_str())
    );
    assert_eq!(
        restored.revision_history.current_revision_id.as_deref(),
        Some(original_revision.as_str())
    );
    assert!(
        restored
            .revision_history
            .entries
            .iter()
            .any(|entry| entry.revision_id == edited_revision && !entry.is_current)
    );
    skin.assert_scalar("Root.B", "2");

    skin.edit("Root.A", "7");
    let branch = skin.state();
    let branch_revision = branch
        .revision
        .workspace_revision_id
        .clone()
        .expect("branch revision should project");
    assert_ne!(branch_revision, edited_revision);
    assert_ne!(branch_revision, original_revision);
    let children_of_original = branch
        .revision_history
        .entries
        .iter()
        .filter(|entry| entry.parent_revision_id.as_deref() == Some(original_revision.as_str()))
        .map(|entry| entry.revision_id.clone())
        .collect::<Vec<_>>();
    assert!(children_of_original.contains(&edited_revision));
    assert!(children_of_original.contains(&branch_revision));
    skin.assert_scalar("Root.B", "8");
}

#[test]
fn programmable_skin_undo_redo_routes_through_retained_revisions() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.select(Some("Root.A"));
    let original_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("original revision should project");
    skin.assert_scalar("Root.B", "2");

    skin.edit("Root.A", "5");
    let edited_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("edited revision should project");
    assert_ne!(edited_revision, original_revision);
    skin.assert_scalar("Root.B", "6");

    let undo = skin.undo();
    assert!(undo.accepted, "{:?}", undo.error);
    assert_eq!(
        undo.produced_revision.as_deref(),
        Some(original_revision.as_str())
    );
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(original_revision.as_str())
    );
    assert_eq!(skin.selected().as_deref(), Some("Root.A"));
    skin.assert_scalar("Root.B", "2");

    let redo = skin.redo();
    assert!(redo.accepted, "{:?}", redo.error);
    assert_eq!(
        redo.produced_revision.as_deref(),
        Some(edited_revision.as_str())
    );
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(edited_revision.as_str())
    );
    assert_eq!(skin.selected().as_deref(), Some("Root.A"));
    skin.assert_scalar("Root.B", "6");

    assert!(skin.undo().accepted);
    skin.edit("Root.A", "7");
    let branch_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("branch revision should project");
    assert_ne!(branch_revision, edited_revision);
    skin.assert_scalar("Root.B", "8");
    let redo_after_branch = skin.redo();
    assert!(!redo_after_branch.accepted);
}

#[test]
fn programmable_skin_projects_dependencies_after_deferred_edit_without_recalc() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B*2");
    skin.recalc();
    assert_eq!(skin.outgoing_count("Root.C"), 1);
    assert_eq!(skin.incoming_count("Root.B"), 1);

    skin.edit_deferred("Root.C", "=A*10");

    let state = skin.state();
    assert!(
        state.last_run.is_none(),
        "a deferred edit should not leave a calculation run behind"
    );
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let c_key = state.node(&NodeId::new("Root.C")).unwrap().key.clone();
    let c_edges = state
        .dependencies
        .edges_by_owner_key
        .get(&c_key)
        .expect("dependencies should stay projected without a calculation run");
    assert!(
        c_edges.iter().any(|edge| edge.target_key == a_key),
        "the projected graph should reflect the applied edit, not the stale run graph"
    );
    assert!(
        c_edges.iter().all(|edge| edge.target_key != b_key),
        "the stale pre-edit dependency should not survive the deferred edit"
    );
    assert!(
        state
            .dependencies
            .reverse_edges_by_key
            .get(&a_key)
            .is_some_and(|edges| edges.iter().any(|edge| edge.owner_key == c_key)),
        "reverse edges should name the dependent without a calculation run"
    );
    assert!(
        state
            .dependencies
            .reference_resolutions
            .values()
            .any(|resolution| resolution.owner_key == c_key),
        "reference resolutions should stay projected without a calculation run"
    );
    assert_eq!(skin.outgoing_count("Root.B"), 1);
    assert_eq!(skin.incoming_count("Root.A"), 2);
}

#[test]
fn programmable_skin_projects_dependencies_after_undo_without_recalc() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B*2");
    skin.recalc();
    skin.edit("Root.C", "=A*10");
    assert_eq!(skin.incoming_count("Root.B"), 0);

    let undo = skin.undo();
    assert!(undo.accepted, "{:?}", undo.error);

    let state = skin.state();
    assert!(
        state.last_run.is_none(),
        "revision navigation should not fabricate a calculation run"
    );
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let c_key = state.node(&NodeId::new("Root.C")).unwrap().key.clone();
    assert!(
        state
            .dependencies
            .edges_by_owner_key
            .get(&c_key)
            .is_some_and(|edges| edges.iter().any(|edge| edge.target_key == b_key)),
        "undo should restore the revision's dependency graph, not blank it"
    );
    assert_eq!(skin.outgoing_count("Root.C"), 1);
    assert_eq!(skin.incoming_count("Root.B"), 1);
    assert_eq!(skin.incoming_count("Root.A"), 1);
}

#[test]
fn programmable_skin_projects_dependencies_after_candidate_commit_without_recalc() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B*2");
    skin.recalc();
    assert_eq!(skin.incoming_count("Root.B"), 1);

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin
        .state()
        .candidates
        .first()
        .expect("candidate should project")
        .handle
        .clone();
    let edit = skin.try_edit_candidate_content(&handle, "Root.C", "=A*10");
    assert!(edit.accepted, "{:?}", edit.error);
    // Commit deliberately without evaluating the candidate: the promoted
    // workspace state then carries no calculation run at all.
    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);

    let state = skin.state();
    assert!(
        state.last_run.is_none(),
        "an unevaluated candidate commit publishes without a calculation run"
    );
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let c_key = state.node(&NodeId::new("Root.C")).unwrap().key.clone();
    let c_edges = state
        .dependencies
        .edges_by_owner_key
        .get(&c_key)
        .expect("dependencies should stay projected after candidate commit");
    assert!(
        c_edges.iter().any(|edge| edge.target_key == a_key),
        "the projected graph should carry the committed candidate dependency"
    );
    assert!(
        c_edges.iter().all(|edge| edge.target_key != b_key),
        "the pre-commit dependency should not survive the committed edit"
    );
    assert_eq!(skin.incoming_count("Root.B"), 0);
    assert_eq!(skin.incoming_count("Root.A"), 2);
}

#[test]
fn programmable_skin_projects_candidate_values_without_publishing_until_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");
    let b_key = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    skin.assert_scalar("Root.B", "2");

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    assert!(open.produced_revision.is_none());
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    let handle = skin
        .state()
        .candidates
        .first()
        .expect("candidate should project")
        .handle
        .clone();
    assert_eq!(
        skin.state().candidates[0].basis_revision_id,
        published_revision
    );
    let open_state = skin.state();
    let open_candidate = &open_state.candidates[0];
    assert!(
        open_candidate
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.A")
                && node.parent.as_ref() == Some(&NodeId::new("Root")))
    );

    let edit = skin.try_edit_candidate_content(&handle, "Root.A", "5");
    assert!(edit.accepted, "{:?}", edit.error);
    assert_eq!(edit.transaction_id, None);
    let edited_candidate = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected after edit")
        .clone();
    let edited_revision_entry = edited_candidate
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == edited_candidate.workspace_revision_id)
        .expect("candidate edit revision should project");
    let candidate_transaction_id = edited_revision_entry
        .transaction_id
        .as_deref()
        .expect("candidate private edit should project its real transaction id");
    assert!(
        candidate_transaction_id.starts_with("transaction:programmable-skin-ir:"),
        "{candidate_transaction_id}"
    );
    let candidate_summary = edited_revision_entry
        .transaction_summary
        .as_ref()
        .expect("candidate private edit should project planned invalidation summary");
    assert_eq!(candidate_summary.transaction_id, candidate_transaction_id);
    assert_eq!(candidate_summary.estimated_node_count, 2);
    assert!(candidate_summary.requires_rebind.is_empty());
    assert_eq!(
        candidate_summary.invalidated_nodes.len(),
        2,
        "{candidate_summary:?}"
    );
    let evaluate = skin.try_evaluate_candidate(&handle);
    assert!(evaluate.accepted, "{:?}", evaluate.error);
    assert_eq!(evaluate.transaction_id, None);
    assert!(evaluate.produced_revision.is_none());
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    skin.assert_scalar("Root.B", "2");
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert_eq!(
        candidate
            .values_by_key
            .get(&b_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6")
    );
    assert!(evaluate.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateChanged(candidate) if candidate.handle == handle)
    }));

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert_eq!(
        commit.transaction_id.as_deref(),
        Some(candidate_transaction_id)
    );
    assert!(commit.produced_revision.is_some());
    assert_ne!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    assert!(skin.state().candidates.is_empty());
    skin.assert_scalar("Root.B", "6");
    assert!(commit.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateRemoved(removed) if removed == &handle)
    }));
}

#[test]
fn programmable_skin_child_candidate_tracks_parent_private_edits_after_open() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");
    let b_key = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    skin.assert_scalar("Root.B", "2");

    let parent_open = skin.try_open_candidate();
    assert!(parent_open.accepted, "{:?}", parent_open.error);
    let parent_handle = skin
        .state()
        .candidates
        .first()
        .expect("parent candidate should project")
        .handle
        .clone();
    let child_open = skin.try_open_child_candidate(&parent_handle);
    assert!(child_open.accepted, "{:?}", child_open.error);
    let child_handle = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.parent_handle.as_deref() == Some(parent_handle.as_str()))
        .expect("child candidate should project")
        .handle
        .clone();

    let parent_edit = skin.try_edit_candidate_content(&parent_handle, "Root.A", "5");
    assert!(parent_edit.accepted, "{:?}", parent_edit.error);
    let evaluate_child = skin.try_evaluate_candidate(&child_handle);
    assert!(evaluate_child.accepted, "{:?}", evaluate_child.error);

    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    skin.assert_scalar("Root.B", "2");
    let state = skin.state();
    let child = state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == child_handle)
        .expect("child candidate should remain projected");
    assert_eq!(child.parent_handle.as_deref(), Some(parent_handle.as_str()));
    assert_eq!(
        child
            .values_by_key
            .get(&b_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6"),
        "child candidate should show parent candidate edits made after child open"
    );
}

#[test]
fn programmable_skin_renames_candidate_node_without_publishing_until_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let a_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.A"))
        .expect("candidate node should project")
        .key
        .clone();

    let rename = skin.try_rename_candidate_node(&handle, a_key.clone(), "Renamed");
    assert!(rename.accepted, "{:?}", rename.error);
    assert_eq!(rename.transaction_id, None);
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    assert!(skin.state().node(&NodeId::new("Root.A")).is_some());
    assert!(skin.state().node(&NodeId::new("Root.Renamed")).is_none());
    let renamed_state = skin.state();
    let renamed_candidate = renamed_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert!(
        renamed_candidate
            .nodes
            .iter()
            .any(|node| node.key == a_key && node.id == NodeId::new("Root.Renamed"))
    );
    let rename_revision_entry = renamed_candidate
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == renamed_candidate.workspace_revision_id)
        .expect("candidate rename revision should project");
    let candidate_transaction_id = rename_revision_entry
        .transaction_id
        .as_deref()
        .expect("candidate private rename should project its real transaction id");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert_eq!(
        commit.transaction_id.as_deref(),
        Some(candidate_transaction_id)
    );
    assert!(commit.produced_revision.is_some());
    assert!(skin.state().node(&NodeId::new("Root.A")).is_none());
    assert!(skin.state().node(&NodeId::new("Root.Renamed")).is_some());
}

#[test]
fn programmable_skin_moves_candidate_node_without_publishing_until_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Group", "");
    skin.add_node(Some("Root"), "A", "1");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let a_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.A"))
        .expect("candidate source node should project")
        .key
        .clone();
    let group_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.Group"))
        .expect("candidate parent node should project")
        .key
        .clone();

    let moved = skin.try_move_candidate_node(&handle, a_key.clone(), Some(group_key.clone()), None);
    assert!(moved.accepted, "{:?}", moved.error);
    assert_eq!(moved.transaction_id, None);
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    assert!(skin.state().node(&NodeId::new("Root.A")).is_some());
    assert!(skin.state().node(&NodeId::new("Root.Group.A")).is_none());
    let moved_state = skin.state();
    let moved_candidate = moved_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert!(moved_candidate.nodes.iter().any(|node| node.key == a_key
        && node.id == NodeId::new("Root.Group.A")
        && node.parent.as_ref() == Some(&NodeId::new("Root.Group"))));
    let move_revision_entry = moved_candidate
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == moved_candidate.workspace_revision_id)
        .expect("candidate move revision should project");
    let candidate_transaction_id = move_revision_entry
        .transaction_id
        .as_deref()
        .expect("candidate private move should project its real transaction id");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert_eq!(
        commit.transaction_id.as_deref(),
        Some(candidate_transaction_id)
    );
    assert!(commit.produced_revision.is_some());
    assert!(skin.state().node(&NodeId::new("Root.A")).is_none());
    assert!(skin.state().node(&NodeId::new("Root.Group.A")).is_some());
}

#[test]
fn programmable_skin_deletes_candidate_node_without_publishing_until_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let a_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.A"))
        .expect("candidate node should project")
        .key
        .clone();

    let deleted = skin.try_delete_candidate_node(&handle, a_key.clone());
    assert!(deleted.accepted, "{:?}", deleted.error);
    assert_eq!(deleted.transaction_id, None);
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    assert!(skin.state().node(&NodeId::new("Root.A")).is_some());
    let deleted_state = skin.state();
    let deleted_candidate = deleted_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert!(!deleted_candidate.nodes.iter().any(|node| node.key == a_key));
    let delete_revision_entry = deleted_candidate
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == deleted_candidate.workspace_revision_id)
        .expect("candidate delete revision should project");
    let candidate_transaction_id = delete_revision_entry
        .transaction_id
        .as_deref()
        .expect("candidate private delete should project its real transaction id");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert_eq!(
        commit.transaction_id.as_deref(),
        Some(candidate_transaction_id)
    );
    assert!(commit.produced_revision.is_some());
    assert!(skin.state().node(&NodeId::new("Root.A")).is_none());
}

#[test]
fn programmable_skin_adds_candidate_node_without_publishing_until_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    let published_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published revision should project");

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let root_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root"))
        .expect("candidate parent should project")
        .key
        .clone();

    let added = skin.try_add_candidate_node(
        &handle,
        Some(root_key),
        "Added",
        InitialNodeContentProjection::Literal {
            content: "7".to_string(),
        },
        false,
    );
    assert!(added.accepted, "{:?}", added.error);
    assert_eq!(added.transaction_id, None);
    assert_eq!(
        skin.state().revision.workspace_revision_id.as_deref(),
        Some(published_revision.as_str())
    );
    assert!(skin.state().node(&NodeId::new("Root.Added")).is_none());
    let added_state = skin.state();
    let added_candidate = added_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    let added_node = added_candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.Added"))
        .expect("candidate-added node should project");
    assert_eq!(added_node.content_text, "7");
    let add_revision_entry = added_candidate
        .revision_history
        .entries
        .iter()
        .find(|entry| entry.revision_id == added_candidate.workspace_revision_id)
        .expect("candidate add revision should project");
    let candidate_transaction_id = add_revision_entry
        .transaction_id
        .as_deref()
        .expect("candidate private add should project its real transaction id");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert_eq!(
        commit.transaction_id.as_deref(),
        Some(candidate_transaction_id)
    );
    assert!(commit.produced_revision.is_some());
    assert!(skin.state().node(&NodeId::new("Root.Added")).is_some());
}

#[test]
fn programmable_skin_adds_candidate_formula_node_against_private_structure() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let a_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.A"))
        .expect("candidate source should project")
        .key
        .clone();

    let rename = skin.try_rename_candidate_node(&handle, a_key, "PrivateA");
    assert!(rename.accepted, "{:?}", rename.error);
    let renamed_state = skin.state();
    let renamed_candidate = renamed_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("renamed candidate should project");
    let root_key = renamed_candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root"))
        .expect("candidate parent should project")
        .key
        .clone();

    let added = skin.try_add_candidate_node(
        &handle,
        Some(root_key),
        "Formula",
        InitialNodeContentProjection::Literal {
            content: "=PrivateA+1".to_string(),
        },
        false,
    );
    assert!(added.accepted, "{:?}", added.error);
    assert!(skin.state().node(&NodeId::new("Root.Formula")).is_none());
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    let formula_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.Formula"))
        .expect("candidate formula node should project")
        .key
        .clone();

    let evaluate = skin.try_evaluate_candidate(&handle);
    assert!(evaluate.accepted, "{:?}", evaluate.error);
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert_eq!(
        candidate
            .values_by_key
            .get(&formula_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
    let run = candidate
        .run
        .as_ref()
        .expect("candidate run should project");
    assert!(run.evaluation_order.contains(&NodeId::new("Root.Formula")));
}

#[test]
fn programmable_skin_adds_candidate_node_from_template_initial_content() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let root_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root"))
        .expect("candidate parent should project")
        .key
        .clone();

    let added = skin.try_add_candidate_node(
        &handle,
        Some(root_key),
        "Templated",
        InitialNodeContentProjection::TemplateBound {
            template_id: "starter".to_string(),
        },
        false,
    );
    assert!(added.accepted, "{:?}", added.error);
    assert!(skin.state().node(&NodeId::new("Root.Templated")).is_none());
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    let templated = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root.Templated"))
        .expect("candidate template node should project");
    assert_eq!(templated.content_text, "=1+1");
    let templated_key = templated.key.clone();

    let evaluate = skin.try_evaluate_candidate(&handle);
    assert!(evaluate.accepted, "{:?}", evaluate.error);
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    assert_eq!(
        candidate
            .values_by_key
            .get(&templated_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
}

#[test]
fn programmable_skin_adds_candidate_node_from_inherited_table_column_formula() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();

    let formula_column =
        skin.try_add_table_formula_column("SalesTable", "col:fixed", "Fixed", "=1+1");
    assert!(formula_column.accepted, "{:?}", formula_column.error);
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let table_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("SalesTable"))
        .expect("candidate table should project")
        .key
        .clone();

    let added = skin.try_add_candidate_node(
        &handle,
        Some(table_key),
        "Inherited",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:fixed".to_string(),
        },
        false,
    );
    assert!(added.accepted, "{:?}", added.error);
    assert!(
        skin.state()
            .node(&NodeId::new("SalesTable.Inherited"))
            .is_none()
    );
    let candidate_state = skin.state();
    let candidate = candidate_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("candidate should remain projected");
    let inherited = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("SalesTable.Inherited"))
        .expect("candidate inherited node should project");
    assert_eq!(inherited.content_text, "=1+1");
}

#[test]
fn programmable_skin_rejects_candidate_row_context_column_formula_inheritance() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();

    let rejected = skin.try_add_candidate_node(
        &handle,
        None,
        "InheritedTax",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:tax".to_string(),
        },
        false,
    );
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.error,
        Some(IntentError::InitialContentBindRejected {
            policy: "inherit_column_formula".to_string()
        })
    );
    assert!(
        skin.state()
            .candidates
            .iter()
            .flat_map(|candidate| candidate.nodes.iter())
            .all(|node| node.id != NodeId::new("InheritedTax"))
    );
}

#[test]
fn programmable_skin_rejects_candidate_add_invalid_formula_initial() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let state = skin.state();
    let candidate = state.candidates.first().expect("candidate should project");
    let handle = candidate.handle.clone();
    let root_key = candidate
        .nodes
        .iter()
        .find(|node| node.id == NodeId::new("Root"))
        .expect("candidate parent should project")
        .key
        .clone();

    let rejected = skin.try_add_candidate_node(
        &handle,
        Some(root_key),
        "Formula",
        InitialNodeContentProjection::Literal {
            content: "=MissingNode+1".to_string(),
        },
        false,
    );
    assert!(!rejected.accepted);
    assert!(matches!(
        rejected.error,
        Some(IntentError::InitialContentBindRejected { .. })
    ));
    assert!(
        skin.state()
            .candidates
            .iter()
            .flat_map(|candidate| candidate.nodes.iter())
            .all(|node| node.id != NodeId::new("Root.Formula"))
    );
}

#[test]
fn programmable_skin_reaps_candidates_to_budget_and_projects_pressure() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    for _ in 0..3 {
        let open = skin.try_open_candidate();
        assert!(open.accepted, "{:?}", open.error);
    }
    let before = skin.state();
    assert_eq!(before.candidates.len(), 3);
    assert_eq!(before.speculation_pressure.retained_candidate_count, 3);
    assert_eq!(
        before.speculation_pressure.child_protected_candidate_count,
        0
    );
    assert_eq!(before.speculation_pressure.host_pinned_candidate_count, 0);
    assert_eq!(before.speculation_pressure.protected_candidate_count, 0);
    assert_eq!(before.speculation_pressure.reclaimable_candidate_count, 3);
    assert_eq!(before.speculation_pressure.over_budget_candidate_count, 1);
    let first = before.candidates[0].handle.clone();
    let second = before.candidates[1].handle.clone();

    let receipt = skin.try_reap_candidates(1);
    assert!(receipt.accepted, "{:?}", receipt.error);
    let state = skin.state();
    assert_eq!(state.candidates.len(), 1);
    assert_eq!(state.speculation_pressure.retained_candidate_count, 1);
    assert_eq!(state.speculation_pressure.over_budget_candidate_count, 0);
    assert!(receipt.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateRemoved(handle) if handle == &first)
    }));
    assert!(receipt.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateRemoved(handle) if handle == &second)
    }));
}

#[test]
fn programmable_skin_pins_candidate_retention_against_reaping() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    assert!(skin.try_open_candidate().accepted);
    assert!(skin.try_open_candidate().accepted);
    let before = skin.state();
    let pinned_handle = before.candidates[0].handle.clone();
    let reclaimable_handle = before.candidates[1].handle.clone();

    let pin = skin.try_pin_candidate_retention(&pinned_handle);
    assert!(pin.accepted, "{:?}", pin.error);
    let pinned_state = skin.state();
    let pinned = pinned_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == pinned_handle)
        .expect("pinned candidate should project");
    assert_eq!(pinned.retention_pin_count, 1);
    assert_eq!(
        pinned_state.speculation_pressure.retained_candidate_count,
        2
    );
    assert_eq!(
        pinned_state
            .speculation_pressure
            .host_pinned_candidate_count,
        1
    );
    assert_eq!(
        pinned_state.speculation_pressure.protected_candidate_count,
        1
    );
    assert_eq!(
        pinned_state
            .speculation_pressure
            .reclaimable_candidate_count,
        1
    );
    assert!(pin.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateChanged(candidate) if candidate.handle == pinned_handle && candidate.retention_pin_count == 1)
    }));

    let reap = skin.try_reap_candidates(1);
    assert!(reap.accepted, "{:?}", reap.error);
    let reaped_state = skin.state();
    assert_eq!(reaped_state.candidates.len(), 1);
    assert_eq!(reaped_state.candidates[0].handle, pinned_handle);
    assert!(reap.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateRemoved(handle) if handle == &reclaimable_handle)
    }));

    let unpin = skin.try_unpin_candidate_retention(&pinned_handle);
    assert!(unpin.accepted, "{:?}", unpin.error);
    let unpinned_state = skin.state();
    assert_eq!(unpinned_state.candidates[0].retention_pin_count, 0);
    assert_eq!(
        unpinned_state
            .speculation_pressure
            .host_pinned_candidate_count,
        0
    );
    assert!(unpin.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateChanged(candidate) if candidate.handle == pinned_handle && candidate.retention_pin_count == 0)
    }));

    let extra_unpin = skin.try_unpin_candidate_retention(&pinned_handle);
    assert!(!extra_unpin.accepted);
}

#[test]
fn programmable_skin_projects_scenario_manifest_over_candidate_handles() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    assert!(skin.try_open_candidate().accepted);
    assert!(skin.try_open_candidate().accepted);
    let before = skin.state();
    let scenario_handle = before.candidates[0].handle.clone();
    let reclaimable_handle = before.candidates[1].handle.clone();

    let create = skin.try_create_scenario_from_candidate("scenario:bull", "Bull", &scenario_handle);
    assert!(create.accepted, "{:?}", create.error);
    let created_state = skin.state();
    assert_eq!(created_state.scenarios.entries.len(), 1);
    let scenario = &created_state.scenarios.entries[0];
    assert_eq!(scenario.id, "scenario:bull");
    assert_eq!(scenario.name, "Bull");
    assert_eq!(
        scenario.source,
        ScenarioSourceProjection::Candidate {
            handle: scenario_handle.clone()
        }
    );
    assert_eq!(scenario.override_count, 0);
    assert!(scenario.overridden_nodes.is_empty());
    assert!(scenario.value_epoch.is_some());
    assert!(!scenario.is_active);
    assert_eq!(created_state.comparison.columns.len(), 1);
    assert_eq!(created_state.comparison.columns[0].label, "Bull");
    assert_eq!(
        created_state.comparison.columns[0].source,
        ComparativeSourceProjection::Scenario {
            id: "scenario:bull".to_string()
        }
    );
    assert!(
        created_state.comparison.columns[0].values.is_empty(),
        "unevaluated scenario columns should not fabricate values"
    );
    assert_eq!(created_state.series.entries.len(), 2);
    assert_eq!(created_state.series.entries[0].id, "series:published");
    assert_eq!(created_state.series.entries[0].label, "Published");
    assert_eq!(
        created_state.series.entries[0].source,
        ComparativeSourceProjection::Published
    );
    assert_eq!(created_state.series.entries[0].points.len(), 1);
    assert_eq!(created_state.series.entries[0].points[0].label, "Root");
    assert_eq!(
        created_state.series.entries[1].id,
        "series:scenario:scenario:bull"
    );
    assert_eq!(created_state.series.entries[1].label, "Bull");
    assert_eq!(
        created_state.series.entries[1].source,
        ComparativeSourceProjection::Scenario {
            id: "scenario:bull".to_string()
        }
    );
    assert!(
        created_state.series.entries[1].points.is_empty(),
        "unevaluated scenario series should not fabricate points"
    );
    assert_eq!(
        created_state
            .candidates
            .iter()
            .find(|candidate| candidate.handle == scenario_handle)
            .unwrap()
            .retention_pin_count,
        1
    );
    assert!(create.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::ScenarioChanged(changed) if changed.id == "scenario:bull")
    }));

    let activate = skin.try_activate_scenario(Some("scenario:bull"));
    assert!(activate.accepted, "{:?}", activate.error);
    let active_state = skin.state();
    assert_eq!(
        active_state.scenarios.active.as_deref(),
        Some("scenario:bull")
    );
    assert!(active_state.scenarios.entries[0].is_active);

    let reap = skin.try_reap_candidates(1);
    assert!(reap.accepted, "{:?}", reap.error);
    let reaped_state = skin.state();
    assert_eq!(reaped_state.candidates.len(), 1);
    assert_eq!(reaped_state.candidates[0].handle, scenario_handle);
    assert_eq!(reaped_state.scenarios.entries.len(), 1);
    assert!(reap.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateRemoved(handle) if handle == &reclaimable_handle)
    }));

    let duplicate =
        skin.try_create_scenario_from_candidate("scenario:bull", "Other", &scenario_handle);
    assert!(!duplicate.accepted);
    assert!(matches!(
        duplicate.error,
        Some(IntentError::ScenarioAlreadyExists { .. })
    ));

    let manual_unpin = skin.try_unpin_candidate_retention(&scenario_handle);
    assert!(manual_unpin.accepted, "{:?}", manual_unpin.error);
    assert_eq!(
        skin.state()
            .candidates
            .iter()
            .find(|candidate| candidate.handle == scenario_handle)
            .unwrap()
            .retention_pin_count,
        0
    );

    let delete = skin.try_delete_scenario("scenario:bull");
    assert!(delete.accepted, "{:?}", delete.error);
    let deleted_state = skin.state();
    assert!(deleted_state.scenarios.entries.is_empty());
    assert!(deleted_state.comparison.columns.is_empty());
    assert_eq!(deleted_state.series.entries.len(), 1);
    assert_eq!(
        deleted_state.series.entries[0].source,
        ComparativeSourceProjection::Published
    );
    assert_eq!(deleted_state.scenarios.active, None);
    assert_eq!(deleted_state.candidates[0].retention_pin_count, 0);
    assert!(delete.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::ScenarioRemoved(id) if id == "scenario:bull")
    }));
}

#[test]
fn programmable_skin_sets_and_clears_candidate_backed_scenario_overrides() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let root_a = skin
        .state()
        .node(&NodeId::new("Root.A"))
        .expect("Root.A should project")
        .key
        .clone();
    let root_b = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    skin.assert_scalar("Root.B", "2");
    assert_eq!(
        skin.state()
            .comparison
            .basis
            .values
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
    let initial_series = skin.state().series;
    assert_eq!(initial_series.entries.len(), 1);
    assert_eq!(initial_series.entries[0].id, "series:published");
    assert_eq!(initial_series.entries[0].label, "Published");
    assert_eq!(
        initial_series.entries[0]
            .points
            .iter()
            .map(|point| point.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Root", "Root.A", "Root.B"]
    );
    assert_eq!(
        initial_series.entries[0]
            .points
            .iter()
            .find(|point| point.key == root_b)
            .map(|point| point.value.display_text())
            .as_deref(),
        Some("2")
    );

    assert!(skin.try_open_candidate().accepted);
    let handle = skin.state().candidates[0].handle.clone();
    let create = skin.try_create_scenario_from_candidate("scenario:bull", "Bull", &handle);
    assert!(create.accepted, "{:?}", create.error);
    let create_epoch = skin.state().scenarios.entries[0]
        .value_epoch
        .expect("created scenario should carry an epoch");

    let set = skin.try_set_scenario_override(
        "scenario:bull",
        root_a.clone(),
        NodeValueProjection::Number {
            raw: "5".to_string(),
            display: "5".to_string(),
        },
    );
    assert!(set.accepted, "{:?}", set.error);
    let overridden = skin.state();
    let scenario = &overridden.scenarios.entries[0];
    assert_eq!(scenario.override_count, 1);
    assert_eq!(scenario.overridden_nodes, vec![root_a.clone()]);
    let set_epoch = scenario
        .value_epoch
        .expect("scenario override should advance epoch");
    assert!(set_epoch > create_epoch);
    assert!(set.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::ScenarioChanged(changed) if changed.override_count == 1)
    }));

    assert!(skin.try_activate_scenario(Some("scenario:bull")).accepted);
    let active_projection = skin.state();
    let active_override = active_projection
        .node(&NodeId::new("Root.A"))
        .and_then(|node| node.scenario_override.as_ref())
        .expect("active scenario should project node override");
    assert_eq!(active_override.display_text(), "5");
    assert_eq!(
        skin.state()
            .node(&NodeId::new("Root.B"))
            .unwrap()
            .scenario_override,
        None
    );

    assert!(skin.try_evaluate_candidate(&handle).accepted);
    let evaluated = skin.state();
    let evaluate_epoch = evaluated.scenarios.entries[0]
        .value_epoch
        .expect("candidate evaluation should advance scenario epoch");
    assert!(evaluate_epoch > set_epoch);
    let candidate = &evaluated.candidates[0];
    assert_eq!(
        candidate
            .nodes
            .iter()
            .find(|node| node.key == root_a)
            .map(|node| node.content_text.as_str()),
        Some("5")
    );
    assert_eq!(
        candidate
            .values_by_key
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6")
    );
    let comparison_column = evaluated
        .comparison
        .columns
        .iter()
        .find(|column| {
            matches!(
                &column.source,
                ComparativeSourceProjection::Scenario { id } if id == "scenario:bull"
            )
        })
        .expect("scenario comparison column should project");
    assert_eq!(comparison_column.label, "Bull");
    assert_eq!(
        comparison_column.value_epoch,
        evaluated.scenarios.entries[0].value_epoch
    );
    assert_eq!(
        comparison_column
            .values
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6")
    );
    let scenario_series = evaluated
        .series
        .entries
        .iter()
        .find(|entry| {
            matches!(
                &entry.source,
                ComparativeSourceProjection::Scenario { id } if id == "scenario:bull"
            )
        })
        .expect("scenario series should project");
    assert_eq!(scenario_series.label, "Bull");
    assert_eq!(scenario_series.unit, None);
    assert_eq!(
        scenario_series
            .points
            .iter()
            .find(|point| point.key == root_b)
            .map(|point| point.value.display_text())
            .as_deref(),
        Some("6")
    );
    let basis_series = evaluated
        .series
        .entries
        .iter()
        .find(|entry| entry.source == ComparativeSourceProjection::Published)
        .expect("published series should project");
    assert_eq!(
        basis_series
            .points
            .iter()
            .find(|point| point.key == root_b)
            .map(|point| point.value.display_text())
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        evaluated
            .comparison
            .basis
            .values
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
    skin.assert_scalar("Root.B", "2");

    assert!(skin.try_activate_scenario(None).accepted);
    assert_eq!(
        skin.state()
            .node(&NodeId::new("Root.A"))
            .unwrap()
            .scenario_override,
        None
    );
    assert!(skin.try_activate_scenario(Some("scenario:bull")).accepted);

    let update = skin.try_set_scenario_override(
        "scenario:bull",
        root_a.clone(),
        NodeValueProjection::Number {
            raw: "7".to_string(),
            display: "7".to_string(),
        },
    );
    assert!(update.accepted, "{:?}", update.error);
    let update_epoch = skin.state().scenarios.entries[0]
        .value_epoch
        .expect("scenario update should carry an epoch");
    assert!(update_epoch > evaluate_epoch);
    assert!(skin.try_evaluate_candidate(&handle).accepted);
    assert_eq!(
        skin.state().candidates[0]
            .values_by_key
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("8")
    );

    let clear = skin.try_clear_scenario_override("scenario:bull", root_a.clone());
    assert!(clear.accepted, "{:?}", clear.error);
    let clear_epoch = skin.state().scenarios.entries[0]
        .value_epoch
        .expect("scenario clear should carry an epoch");
    assert!(clear_epoch > update_epoch);
    assert_eq!(skin.state().scenarios.entries[0].override_count, 0);
    assert!(
        skin.state().scenarios.entries[0]
            .overridden_nodes
            .is_empty()
    );
    assert!(skin.try_evaluate_candidate(&handle).accepted);
    assert_eq!(
        skin.state().candidates[0]
            .values_by_key
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );

    let array_override = skin.try_set_scenario_override(
        "scenario:bull",
        root_a.clone(),
        NodeValueProjection::Array {
            rows: 1,
            cols: 2,
            cells: vec![vec![
                NodeValueProjection::Number {
                    raw: "2".to_string(),
                    display: "2".to_string(),
                },
                NodeValueProjection::Number {
                    raw: "3".to_string(),
                    display: "3".to_string(),
                },
            ]],
        },
    );
    assert!(array_override.accepted, "{:?}", array_override.error);
    let array_projection = skin.state();
    let array_override_projection = array_projection
        .node(&NodeId::new("Root.A"))
        .and_then(|node| node.scenario_override.as_ref())
        .expect("active scenario should project array override");
    assert!(matches!(
        array_override_projection,
        NodeValueProjection::Array {
            rows: 1,
            cols: 2,
            ..
        }
    ));
    assert!(skin.try_evaluate_candidate(&handle).accepted);
    let array_state = skin.state();
    let Some(NodeValueProjection::Array { rows, cols, cells }) =
        array_state.candidates[0].values_by_key.get(&root_a)
    else {
        panic!(
            "scenario array override should project as candidate array value, got {:?}",
            array_state.candidates[0].values_by_key.get(&root_a)
        );
    };
    assert_eq!((*rows, *cols), (1, 2));
    assert_eq!(cells[0][0].display_text(), "2");
    assert_eq!(cells[0][1].display_text(), "3");
    assert!(
        skin.try_clear_scenario_override("scenario:bull", root_a.clone())
            .accepted
    );

    let unsupported = skin.try_set_scenario_override(
        "scenario:bull",
        root_a.clone(),
        NodeValueProjection::Scalar("9".into()),
    );
    assert!(!unsupported.accepted);
    assert!(matches!(
        unsupported.error,
        Some(IntentError::UnsupportedScenarioOverrideValue { .. })
    ));

    let set_deleted_node_override = skin.try_set_scenario_override(
        "scenario:bull",
        root_a.clone(),
        NodeValueProjection::Number {
            raw: "10".to_string(),
            display: "10".to_string(),
        },
    );
    assert!(
        set_deleted_node_override.accepted,
        "{:?}",
        set_deleted_node_override.error
    );
    let delete_overridden_node = skin.try_delete_candidate_node(&handle, root_a);
    assert!(
        delete_overridden_node.accepted,
        "{:?}",
        delete_overridden_node.error
    );
    assert!(
        skin.state().scenarios.entries[0]
            .overridden_nodes
            .is_empty()
    );
}

#[test]
fn programmable_skin_projects_direct_sweep_comparison_and_series() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Input", "1");
    skin.add_node(Some("Root"), "Double", "=Input*2");
    skin.add_node(Some("Root"), "Label", "\"published\"");
    let state = skin.state();
    let input_key = state
        .node(&NodeId::new("Root.Input"))
        .expect("Root.Input should project")
        .key
        .clone();
    let double_key = state
        .node(&NodeId::new("Root.Double"))
        .expect("Root.Double should project")
        .key
        .clone();
    skin.assert_scalar("Root.Double", "2");

    let create = skin.try_create_scenario_sweep(
        "sweep:input",
        "Input Sweep",
        None,
        input_key.clone(),
        vec![
            sweep_point("low", "Low", "1"),
            sweep_point("mid", "Mid", "2"),
            sweep_point("high", "High", "3"),
        ],
    );
    assert!(create.accepted, "{:?}", create.error);
    assert!(create.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::SweepChanged(sweep) if sweep.id == "sweep:input")
    }));

    let swept = skin.state();
    assert_eq!(swept.sweeps.entries.len(), 1);
    let sweep = &swept.sweeps.entries[0];
    assert_eq!(sweep.id, "sweep:input");
    assert_eq!(sweep.name, "Input Sweep");
    assert_eq!(sweep.input_node, input_key);
    assert_eq!(sweep.points.len(), 3);
    assert_eq!(
        sweep
            .points
            .iter()
            .map(|point| point.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Low", "Mid", "High"]
    );
    assert_eq!(
        sweep
            .points
            .iter()
            .map(|point| point.input_value.display_text())
            .collect::<Vec<_>>(),
        vec!["1", "2", "3"]
    );
    assert!(
        swept.scenarios.entries.is_empty(),
        "sweep backing scenarios should not project as ordinary scenario rails"
    );
    assert_eq!(swept.candidates.len(), 3);
    assert!(
        swept
            .candidates
            .iter()
            .all(|candidate| candidate.retention_pin_count == 1)
    );

    let sweep_columns = swept
        .comparison
        .columns
        .iter()
        .filter_map(|column| match &column.source {
            ComparativeSourceProjection::SweepPoint {
                sweep_id,
                point_id,
                scenario_id,
            } if sweep_id == "sweep:input" => Some((
                point_id.as_str(),
                scenario_id.as_str(),
                column
                    .values
                    .get(&double_key)
                    .map(NodeValueProjection::display_text),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sweep_columns,
        vec![
            ("low", "sweep:sweep:input:point:low", Some("2".to_string())),
            ("mid", "sweep:sweep:input:point:mid", Some("4".to_string())),
            (
                "high",
                "sweep:sweep:input:point:high",
                Some("6".to_string())
            ),
        ]
    );
    assert_eq!(
        swept
            .comparison
            .basis
            .values
            .get(&double_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
    skin.assert_scalar("Root.Double", "2");

    let double_series = swept
        .series_for_scope(&AuthoringScope::Node(double_key.clone()))
        .expect("sweep target scope should expand");
    let sweep_series_values = double_series
        .entries
        .iter()
        .filter_map(|entry| match &entry.source {
            ComparativeSourceProjection::SweepPoint { point_id, .. } => {
                Some((point_id.as_str(), entry.points[0].value.display_text()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sweep_series_values,
        vec![
            ("low", "2".to_string()),
            ("mid", "4".to_string()),
            ("high", "6".to_string()),
        ]
    );

    let activate = skin.try_activate_sweep(Some("sweep:input"));
    assert!(activate.accepted, "{:?}", activate.error);
    assert_eq!(skin.state().sweeps.active.as_deref(), Some("sweep:input"));
    assert!(skin.state().sweeps.entries[0].is_active);

    let delete = skin.try_delete_sweep("sweep:input");
    assert!(delete.accepted, "{:?}", delete.error);
    let deleted = skin.state();
    assert!(deleted.sweeps.entries.is_empty());
    assert!(deleted.scenarios.entries.is_empty());
    assert!(deleted.comparison.columns.is_empty());
    assert_eq!(deleted.series.entries.len(), 1);
    assert!(delete.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::SweepRemoved(id) if id == "sweep:input")
    }));
}

#[test]
fn programmable_skin_layers_sweep_points_over_visible_scenario() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "2");
    skin.add_node(Some("Root"), "Total", "=A+B");
    let state = skin.state();
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let total_key = state.node(&NodeId::new("Root.Total")).unwrap().key.clone();

    assert!(skin.try_open_candidate().accepted);
    let base_handle = skin.state().candidates[0].handle.clone();
    assert!(
        skin.try_create_scenario_from_candidate("scenario:base", "Base What-If", &base_handle)
            .accepted
    );
    assert!(
        skin.try_set_scenario_override(
            "scenario:base",
            b_key,
            NodeValueProjection::Number {
                raw: "10".to_string(),
                display: "10".to_string(),
            },
        )
        .accepted
    );
    assert!(skin.try_evaluate_candidate(&base_handle).accepted);

    let create = skin.try_create_scenario_sweep(
        "sweep:a",
        "A Sweep",
        Some("scenario:base"),
        a_key,
        vec![
            sweep_point("one", "One", "1"),
            sweep_point("two", "Two", "2"),
        ],
    );
    assert!(create.accepted, "{:?}", create.error);
    let swept = skin.state();
    assert_eq!(
        swept
            .scenarios
            .entries
            .iter()
            .map(|scenario| scenario.id.as_str())
            .collect::<Vec<_>>(),
        vec!["scenario:base"]
    );
    assert_eq!(
        swept.sweeps.entries[0].base_scenario_id.as_deref(),
        Some("scenario:base")
    );

    let sweep_values = swept
        .comparison
        .columns
        .iter()
        .filter_map(|column| match &column.source {
            ComparativeSourceProjection::SweepPoint { point_id, .. } => Some((
                point_id.as_str(),
                column
                    .values
                    .get(&total_key)
                    .map(NodeValueProjection::display_text),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sweep_values,
        vec![
            ("one", Some("11".to_string())),
            ("two", Some("12".to_string()))
        ]
    );
    assert_eq!(
        swept
            .comparison
            .columns
            .iter()
            .find(|column| {
                matches!(
                    &column.source,
                    ComparativeSourceProjection::Scenario { id } if id == "scenario:base"
                )
            })
            .and_then(|column| column.values.get(&total_key))
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("11")
    );

    let delete_base = skin.try_delete_scenario("scenario:base");
    assert!(delete_base.accepted, "{:?}", delete_base.error);
    let cleaned = skin.state();
    assert!(cleaned.scenarios.entries.is_empty());
    assert!(cleaned.sweeps.entries.is_empty());
    assert!(cleaned.comparison.columns.is_empty());
}

#[test]
fn programmable_skin_persists_managed_scenarios_and_sweeps_through_workspace_document() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A*2");
    let state = skin.state();
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();

    let create = skin.try_create_scenario("scenario:managed", "Managed", None);
    assert!(create.accepted, "{:?}", create.error);
    let set = skin.try_set_scenario_override(
        "scenario:managed",
        a_key.clone(),
        NodeValueProjection::Number {
            raw: "5".to_string(),
            display: "5".to_string(),
        },
    );
    assert!(set.accepted, "{:?}", set.error);
    let scenario_handle = match &skin.state().scenarios.entries[0].source {
        ScenarioSourceProjection::Candidate { handle } => handle.clone(),
    };
    assert!(skin.try_evaluate_candidate(&scenario_handle).accepted);
    let create_child = skin.try_create_scenario(
        "scenario:a-child",
        "Managed Child",
        Some("scenario:managed"),
    );
    assert!(create_child.accepted, "{:?}", create_child.error);
    let set_child = skin.try_set_scenario_override(
        "scenario:a-child",
        a_key.clone(),
        NodeValueProjection::Number {
            raw: "6".to_string(),
            display: "6".to_string(),
        },
    );
    assert!(set_child.accepted, "{:?}", set_child.error);
    let child_handle = match &skin
        .state()
        .scenarios
        .entries
        .iter()
        .find(|scenario| scenario.id == "scenario:a-child")
        .expect("child scenario should project")
        .source
    {
        ScenarioSourceProjection::Candidate { handle } => handle.clone(),
    };
    assert!(skin.try_evaluate_candidate(&child_handle).accepted);
    assert!(
        skin.try_create_scenario_sweep(
            "sweep:managed",
            "Managed Sweep",
            Some("scenario:managed"),
            a_key,
            vec![
                sweep_point("six", "Six", "6"),
                sweep_point("seven", "Seven", "7")
            ],
        )
        .accepted
    );
    assert!(
        skin.try_activate_scenario(Some("scenario:a-child"))
            .accepted
    );
    assert!(skin.try_activate_sweep(Some("sweep:managed")).accepted);

    let document = harness
        .session
        .lock()
        .unwrap()
        .export_dnatree_document(None)
        .expect("document export succeeds");
    assert_eq!(
        document.what_if.active_scenario_id.as_deref(),
        Some("scenario:a-child")
    );
    assert_eq!(
        document.what_if.active_sweep_id.as_deref(),
        Some("sweep:managed")
    );
    assert_eq!(document.what_if.scenarios.len(), 2);
    assert_eq!(
        document
            .what_if
            .scenarios
            .iter()
            .map(|scenario| scenario.id.as_str())
            .collect::<Vec<_>>(),
        vec!["scenario:a-child", "scenario:managed"],
        "document order is map order, so restore must not rely on bases appearing first"
    );
    assert_eq!(document.what_if.sweeps.len(), 1);

    let (imported, _) =
        TreeWorkspaceSession::from_dnatree_document(document).expect("document import succeeds");
    let restored = imported.workspace_state().expect("imported state projects");
    assert_eq!(
        restored.scenarios.active.as_deref(),
        Some("scenario:a-child")
    );
    assert_eq!(restored.sweeps.active.as_deref(), Some("sweep:managed"));
    assert_eq!(restored.scenarios.entries.len(), 2);
    assert_eq!(restored.sweeps.entries.len(), 1);
    assert_eq!(
        restored
            .comparison
            .basis
            .values
            .get(&b_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        restored
            .comparison
            .columns
            .iter()
            .find(|column| {
                matches!(
                    &column.source,
                    ComparativeSourceProjection::Scenario { id } if id == "scenario:managed"
                )
            })
            .and_then(|column| column.values.get(&b_key))
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("10")
    );
    assert_eq!(
        restored
            .comparison
            .columns
            .iter()
            .find(|column| {
                matches!(
                    &column.source,
                    ComparativeSourceProjection::Scenario { id } if id == "scenario:a-child"
                )
            })
            .and_then(|column| column.values.get(&b_key))
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("12")
    );
    let restored_sweep_values = restored
        .comparison
        .columns
        .iter()
        .filter_map(|column| match &column.source {
            ComparativeSourceProjection::SweepPoint { point_id, .. } => Some((
                point_id.as_str(),
                column
                    .values
                    .get(&b_key)
                    .map(NodeValueProjection::display_text),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        restored_sweep_values,
        vec![
            ("six", Some("12".to_string())),
            ("seven", Some("14".to_string())),
        ]
    );
}

#[test]
fn programmable_skin_does_not_persist_arbitrary_candidate_backed_scenarios_as_overrides() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    assert!(skin.try_open_candidate().accepted);
    let handle = skin.state().candidates[0].handle.clone();
    assert!(
        skin.try_edit_candidate_content(&handle, "Root.A", "9")
            .accepted
    );
    assert!(skin.try_evaluate_candidate(&handle).accepted);
    let create = skin.try_create_scenario_from_candidate("scenario:freeform", "Freeform", &handle);
    assert!(create.accepted, "{:?}", create.error);
    assert_eq!(skin.state().scenarios.entries.len(), 1);

    let document = harness
        .session
        .lock()
        .unwrap()
        .export_dnatree_document(None)
        .expect("document export succeeds");
    assert!(
        document.what_if.scenarios.is_empty(),
        "arbitrary candidate-backed scenarios are transient until OxCalc exposes durable candidate/scenario snapshots"
    );
    let (imported, _) =
        TreeWorkspaceSession::from_dnatree_document(document).expect("document import succeeds");
    assert!(
        imported
            .workspace_state()
            .expect("imported state projects")
            .scenarios
            .entries
            .is_empty()
    );
}

#[test]
fn programmable_skin_projects_scoped_series_with_unit_metadata() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "3");

    let state = skin.state();
    let a_key = state
        .node(&NodeId::new("Root.A"))
        .expect("Root.A should project")
        .key
        .clone();
    let b_key = state
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    let c_key = state
        .node(&NodeId::new("Root.C"))
        .expect("Root.C should project")
        .key
        .clone();

    let set_a_unit =
        skin.try_set_node_attributes(a_key.clone(), NodeAttributePatch::set("unit", "USD"));
    assert!(set_a_unit.accepted, "{:?}", set_a_unit.error);
    let set_b_unit =
        skin.try_set_node_attributes(b_key.clone(), NodeAttributePatch::set("series_unit", "USD"));
    assert!(set_b_unit.accepted, "{:?}", set_b_unit.error);
    let set_c_unit =
        skin.try_set_node_attributes(c_key.clone(), NodeAttributePatch::set("unit", "EUR"));
    assert!(set_c_unit.accepted, "{:?}", set_c_unit.error);

    let scoped = skin
        .state()
        .series_for_scope(&AuthoringScope::Nodes(vec![a_key.clone(), b_key.clone()]))
        .expect("scoped series should expand");
    assert_eq!(scoped.entries.len(), 1);
    assert_eq!(
        scoped.entries[0].source,
        ComparativeSourceProjection::Published
    );
    assert_eq!(scoped.entries[0].unit.as_deref(), Some("USD"));
    assert_eq!(
        scoped.entries[0]
            .points
            .iter()
            .map(|point| point.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Root.A", "Root.B"]
    );
    assert_eq!(
        scoped.entries[0]
            .points
            .iter()
            .map(|point| point.value.display_text())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );

    let mixed = skin
        .state()
        .series_for_scope(&AuthoringScope::Nodes(vec![a_key.clone(), c_key.clone()]))
        .expect("mixed unit scope should still project");
    assert_eq!(mixed.entries[0].unit, None);

    assert!(skin.try_open_candidate().accepted);
    let handle = skin.state().candidates[0].handle.clone();
    let create = skin.try_create_scenario_from_candidate("scenario:bull", "Bull", &handle);
    assert!(create.accepted, "{:?}", create.error);
    let set_a_override = skin.try_set_scenario_override(
        "scenario:bull",
        a_key.clone(),
        NodeValueProjection::Number {
            raw: "10".to_string(),
            display: "10".to_string(),
        },
    );
    assert!(set_a_override.accepted, "{:?}", set_a_override.error);
    let evaluate = skin.try_evaluate_candidate(&handle);
    assert!(evaluate.accepted, "{:?}", evaluate.error);

    let scenario_scoped = skin
        .state()
        .series_for_scope(&AuthoringScope::Nodes(vec![a_key.clone(), b_key.clone()]))
        .expect("scoped scenario series should expand");
    assert_eq!(scenario_scoped.entries.len(), 2);
    let scenario_entry = scenario_scoped
        .entries
        .iter()
        .find(|entry| {
            matches!(
                &entry.source,
                ComparativeSourceProjection::Scenario { id } if id == "scenario:bull"
            )
        })
        .expect("scenario series should project");
    assert_eq!(scenario_entry.unit.as_deref(), Some("USD"));
    assert_eq!(
        scenario_entry
            .points
            .iter()
            .map(|point| point.value.display_text())
            .collect::<Vec<_>>(),
        vec!["10", "11"]
    );
}

#[test]
fn programmable_skin_rejects_stale_candidate_commit_without_losing_candidate() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let root_a_key = skin
        .state()
        .node(&NodeId::new("Root.A"))
        .expect("Root.A should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin
        .state()
        .candidates
        .first()
        .expect("candidate should project")
        .handle
        .clone();
    assert!(
        skin.try_edit_candidate_content(&handle, "Root.A", "5")
            .accepted
    );
    assert!(skin.try_evaluate_candidate(&handle).accepted);

    skin.edit("Root.A", "9");
    skin.assert_scalar("Root.B", "10");
    let commit = skin.try_commit_candidate(&handle);
    assert!(!commit.accepted);
    assert!(matches!(
        commit.error,
        Some(IntentError::CandidateBasisNotCurrent { .. })
    ));
    assert_eq!(skin.state().candidates.len(), 1);
    assert_eq!(skin.state().candidates[0].handle, handle);
    skin.assert_scalar("Root.B", "10");
    let rebase = skin.try_rebase_candidate(&handle);
    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes == vec![root_a_key.clone()]
    ));
    assert_eq!(skin.state().candidates.len(), 1);

    let discard = skin.try_discard_candidate(&handle);
    assert!(discard.accepted, "{:?}", discard.error);
    assert!(skin.state().candidates.is_empty());
}

#[test]
fn programmable_skin_rejects_candidate_add_rebase_when_live_parent_order_changed() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    let root_key = skin
        .state()
        .node(&NodeId::new("Root"))
        .expect("Root should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let added = skin.try_add_candidate_node(
        &handle,
        Some(root_key.clone()),
        "CandidateChild",
        InitialNodeContentProjection::Literal {
            content: "7".to_string(),
        },
        false,
    );
    assert!(added.accepted, "{:?}", added.error);

    skin.add_node(Some("Root"), "LiveChild", "9");
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes == vec![root_key.clone()]
    ));
    assert!(
        skin.state()
            .node(&NodeId::new("Root.CandidateChild"))
            .is_none()
    );
    assert!(skin.state().node(&NodeId::new("Root.LiveChild")).is_some());
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rejects_candidate_move_rebase_when_live_destination_order_changed() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Group", "");
    skin.add_node(Some("Root"), "A", "1");
    let state = skin.state();
    let group_key = state
        .node(&NodeId::new("Root.Group"))
        .expect("Group should project")
        .key
        .clone();
    let a_key = state
        .node(&NodeId::new("Root.A"))
        .expect("A should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let moved = skin.try_move_candidate_node(&handle, a_key, Some(group_key.clone()), None);
    assert!(moved.accepted, "{:?}", moved.error);

    skin.add_node(Some("Root.Group"), "LiveChild", "9");
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes == vec![group_key.clone()]
    ));
    assert!(skin.state().node(&NodeId::new("Root.Group.A")).is_none());
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Group.LiveChild"))
            .is_some()
    );
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rejects_candidate_move_rebase_when_live_old_parent_order_changed() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Source", "");
    skin.add_node(Some("Root"), "Dest", "");
    skin.add_node(Some("Root.Source"), "A", "1");
    let state = skin.state();
    let source_key = state
        .node(&NodeId::new("Root.Source"))
        .expect("Source should project")
        .key
        .clone();
    let dest_key = state
        .node(&NodeId::new("Root.Dest"))
        .expect("Dest should project")
        .key
        .clone();
    let a_key = state
        .node(&NodeId::new("Root.Source.A"))
        .expect("A should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let moved = skin.try_move_candidate_node(&handle, a_key, Some(dest_key), None);
    assert!(moved.accepted, "{:?}", moved.error);

    skin.add_node(Some("Root.Source"), "LiveChild", "9");
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes == vec![source_key.clone()]
    ));
    assert!(skin.state().node(&NodeId::new("Root.Dest.A")).is_none());
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Source.LiveChild"))
            .is_some()
    );
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rejects_candidate_delete_rebase_when_live_descendant_changed() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "Child", "1");
    let state = skin.state();
    let parent_key = state
        .node(&NodeId::new("Root.Parent"))
        .expect("Parent should project")
        .key
        .clone();
    let child_key = state
        .node(&NodeId::new("Root.Parent.Child"))
        .expect("Child should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let deleted = skin.try_delete_candidate_node(&handle, parent_key);
    assert!(deleted.accepted, "{:?}", deleted.error);

    skin.edit("Root.Parent.Child", "2");
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes == vec![child_key.clone()]
    ));
    assert!(skin.state().node(&NodeId::new("Root.Parent")).is_some());
    assert_eq!(
        skin.state()
            .node(&NodeId::new("Root.Parent.Child"))
            .expect("Child should remain published")
            .content_text,
        "2"
    );
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rebases_stale_candidate_before_commit() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "10");
    let root_b = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let original_basis = skin.state().candidates[0].basis_revision_id.clone();

    assert!(
        skin.try_edit_candidate_content(&handle, "Root.A", "5")
            .accepted
    );
    skin.edit("Root.C", "20");
    skin.assert_scalar("Root.B", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");
    assert_ne!(current_revision, original_basis);
    let stale_commit = skin.try_commit_candidate(&handle);
    assert!(!stale_commit.accepted);
    assert!(matches!(
        stale_commit.error,
        Some(IntentError::CandidateBasisNotCurrent { .. })
    ));

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    assert!(rebase.delta.changes.iter().any(|change| {
        matches!(change, WorkspaceDeltaChange::CandidateChanged(candidate) if candidate.handle == handle)
    }));
    let rebased_state = skin.state();
    let rebased = rebased_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained");
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert_ne!(rebased.workspace_revision_id, original_basis);
    assert!(
        rebased.values_by_key.is_empty(),
        "rebase should not project stale candidate values before candidate evaluation"
    );

    assert!(skin.try_evaluate_candidate(&handle).accepted);
    assert_eq!(
        skin.state().candidates[0]
            .values_by_key
            .get(&root_b)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6")
    );
    skin.assert_scalar("Root.B", "2");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_scalar("Root.B", "6");
    assert!(skin.state().candidates.is_empty());
}

#[test]
fn programmable_skin_rebases_candidate_add_when_live_only_edits_parent_content() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "1");
    let root_key = skin
        .state()
        .node(&NodeId::new("Root"))
        .expect("Root should project")
        .key
        .clone();
    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let original_basis = skin.state().candidates[0].basis_revision_id.clone();

    let add = skin.try_add_candidate_node(
        &handle,
        Some(root_key),
        "CandidateChild",
        InitialNodeContentProjection::Literal {
            content: "7".to_string(),
        },
        false,
    );
    assert!(add.accepted, "{:?}", add.error);

    skin.edit("Root", "2");
    skin.assert_scalar("Root", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");
    assert_ne!(current_revision, original_basis);

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased_state = skin.state();
    let rebased = rebased_state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained");
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(
        rebased
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.CandidateChild") && node.content_text == "7")
    );
    assert!(
        rebased.values_by_key.is_empty(),
        "rebase should not carry stale candidate values"
    );
    assert!(
        skin.state()
            .node(&NodeId::new("Root.CandidateChild"))
            .is_none(),
        "rebase must not publish candidate-only structure"
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_scalar("Root", "2");
    skin.assert_scalar("Root.CandidateChild", "7");
    assert!(skin.state().candidates.is_empty());
}

#[test]
fn programmable_skin_rebases_candidate_rename_and_move_over_live_content_edits() {
    {
        let harness = Harness::empty();
        let skin = harness.driver.clone();

        skin.add_node(None, "Root", "1");
        let root_key = skin
            .state()
            .node(&NodeId::new("Root"))
            .expect("Root should project")
            .key
            .clone();
        let open = skin.try_open_candidate();
        assert!(open.accepted, "{:?}", open.error);
        let handle = skin.state().candidates[0].handle.clone();

        let rename = skin.try_rename_candidate_node(&handle, root_key, "Renamed");
        assert!(rename.accepted, "{:?}", rename.error);

        skin.edit("Root", "2");
        skin.assert_scalar("Root", "2");
        let current_revision = skin
            .state()
            .revision
            .workspace_revision_id
            .clone()
            .expect("published workspace revision should project");

        let rebase = skin.try_rebase_candidate(&handle);
        assert!(rebase.accepted, "{:?}", rebase.error);
        let rebased = skin
            .state()
            .candidates
            .iter()
            .find(|candidate| candidate.handle == handle)
            .expect("rebased candidate should remain retained")
            .clone();
        assert_eq!(rebased.basis_revision_id, current_revision);
        assert!(
            rebased
                .nodes
                .iter()
                .any(|node| node.id == NodeId::new("Renamed") && node.content_text == "2")
        );
        assert!(skin.state().node(&NodeId::new("Renamed")).is_none());
        skin.assert_scalar("Root", "2");

        let commit = skin.try_commit_candidate(&handle);
        assert!(commit.accepted, "{:?}", commit.error);
        assert!(skin.state().node(&NodeId::new("Root")).is_none());
        skin.assert_scalar("Renamed", "2");
    }

    {
        let harness = Harness::empty();
        let skin = harness.driver.clone();

        skin.add_node(None, "SourceParent", "");
        skin.add_node(None, "DestinationParent", "");
        skin.add_node(Some("SourceParent"), "Moved", "1");
        let moved_key = skin
            .state()
            .node(&NodeId::new("SourceParent.Moved"))
            .expect("moved node should project")
            .key
            .clone();
        let destination_key = skin
            .state()
            .node(&NodeId::new("DestinationParent"))
            .expect("destination parent should project")
            .key
            .clone();
        let open = skin.try_open_candidate();
        assert!(open.accepted, "{:?}", open.error);
        let handle = skin.state().candidates[0].handle.clone();

        let moved = skin.try_move_candidate_node(&handle, moved_key, Some(destination_key), None);
        assert!(moved.accepted, "{:?}", moved.error);

        skin.edit("SourceParent.Moved", "2");
        skin.assert_scalar("SourceParent.Moved", "2");
        let current_revision = skin
            .state()
            .revision
            .workspace_revision_id
            .clone()
            .expect("published workspace revision should project");

        let rebase = skin.try_rebase_candidate(&handle);
        assert!(rebase.accepted, "{:?}", rebase.error);
        let rebased = skin
            .state()
            .candidates
            .iter()
            .find(|candidate| candidate.handle == handle)
            .expect("rebased candidate should remain retained")
            .clone();
        assert_eq!(rebased.basis_revision_id, current_revision);
        assert!(rebased.nodes.iter().any(|node| {
            node.id == NodeId::new("DestinationParent.Moved") && node.content_text == "2"
        }));
        assert!(
            skin.state()
                .node(&NodeId::new("DestinationParent.Moved"))
                .is_none()
        );
        skin.assert_scalar("SourceParent.Moved", "2");

        let commit = skin.try_commit_candidate(&handle);
        assert!(commit.accepted, "{:?}", commit.error);
        assert!(
            skin.state()
                .node(&NodeId::new("SourceParent.Moved"))
                .is_none()
        );
        skin.assert_scalar("DestinationParent.Moved", "2");
    }
}

#[test]
fn programmable_skin_rebases_multi_edit_candidate_over_live_content_edits() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "SourceParent", "10");
    skin.add_node(None, "DestinationParent", "");
    skin.add_node(Some("SourceParent"), "RenameMe", "1");
    skin.add_node(Some("SourceParent"), "MoveMe", "2");
    let source_parent_key = skin
        .state()
        .node(&NodeId::new("SourceParent"))
        .expect("source parent should project")
        .key
        .clone();
    let destination_parent_key = skin
        .state()
        .node(&NodeId::new("DestinationParent"))
        .expect("destination parent should project")
        .key
        .clone();
    let rename_key = skin
        .state()
        .node(&NodeId::new("SourceParent.RenameMe"))
        .expect("rename target should project")
        .key
        .clone();
    let move_key = skin
        .state()
        .node(&NodeId::new("SourceParent.MoveMe"))
        .expect("move target should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();

    let rename = skin.try_rename_candidate_node(&handle, rename_key, "Renamed");
    assert!(rename.accepted, "{:?}", rename.error);
    let moved = skin.try_move_candidate_node(&handle, move_key, Some(destination_parent_key), None);
    assert!(moved.accepted, "{:?}", moved.error);
    let add = skin.try_add_candidate_node(
        &handle,
        Some(source_parent_key),
        "CandidateOnly",
        InitialNodeContentProjection::Literal {
            content: "5".to_string(),
        },
        false,
    );
    assert!(add.accepted, "{:?}", add.error);

    skin.edit("SourceParent", "11");
    skin.edit("SourceParent.RenameMe", "3");
    skin.edit("SourceParent.MoveMe", "4");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("SourceParent.Renamed") && node.content_text == "3"
    }));
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("DestinationParent.MoveMe") && node.content_text == "4"
    }));
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("SourceParent.CandidateOnly") && node.content_text == "5"
    }));
    assert!(
        rebased.values_by_key.is_empty(),
        "rebase should not carry stale candidate values"
    );
    assert!(
        skin.state()
            .node(&NodeId::new("SourceParent.CandidateOnly"))
            .is_none()
    );
    skin.assert_scalar("SourceParent", "11");
    skin.assert_scalar("SourceParent.RenameMe", "3");
    skin.assert_scalar("SourceParent.MoveMe", "4");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("SourceParent.RenameMe"))
            .is_none()
    );
    assert!(
        skin.state()
            .node(&NodeId::new("SourceParent.MoveMe"))
            .is_none()
    );
    skin.assert_scalar("SourceParent", "11");
    skin.assert_scalar("SourceParent.Renamed", "3");
    skin.assert_scalar("DestinationParent.MoveMe", "4");
    skin.assert_scalar("SourceParent.CandidateOnly", "5");
}

#[test]
fn programmable_skin_rebases_candidate_rename_over_live_move_same_node() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Source", "");
    skin.add_node(Some("Root"), "Destination", "");
    skin.add_node(Some("Root.Source"), "Original", "1");
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Source.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let rename = skin.try_rename_candidate_node(&handle, node_key.clone(), "Renamed");
    assert!(rename.accepted, "{:?}", rename.error);

    let live_move = skin.try_move_node("Root.Source.Original", Some("Root.Destination"), None);
    assert!(live_move.accepted, "{:?}", live_move.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Source.Original"))
            .is_none()
    );
    skin.assert_scalar("Root.Destination.Original", "1");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(rebased.nodes.iter().any(|node| {
        node.key == node_key && node.id == NodeId::new("Root.Destination.Renamed")
    }));
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Destination.Renamed"))
            .is_none()
    );
    skin.assert_scalar("Root.Destination.Original", "1");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Destination.Original"))
            .is_none()
    );
    skin.assert_scalar("Root.Destination.Renamed", "1");
}

#[test]
fn programmable_skin_rebases_candidate_move_over_live_rename_same_node() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Source", "");
    skin.add_node(Some("Root"), "Destination", "");
    skin.add_node(Some("Root.Source"), "Original", "1");
    let state = skin.state();
    let node_key = state
        .node(&NodeId::new("Root.Source.Original"))
        .expect("Original should project")
        .key
        .clone();
    let destination_key = state
        .node(&NodeId::new("Root.Destination"))
        .expect("Destination should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let moved =
        skin.try_move_candidate_node(&handle, node_key.clone(), Some(destination_key), None);
    assert!(moved.accepted, "{:?}", moved.error);

    let live_rename = skin.try_rename("Root.Source.Original", "Renamed");
    assert!(live_rename.accepted, "{:?}", live_rename.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Source.Original"))
            .is_none()
    );
    skin.assert_scalar("Root.Source.Renamed", "1");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(rebased.nodes.iter().any(|node| {
        node.key == node_key && node.id == NodeId::new("Root.Destination.Renamed")
    }));
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Destination.Renamed"))
            .is_none()
    );
    skin.assert_scalar("Root.Source.Renamed", "1");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Source.Renamed"))
            .is_none()
    );
    skin.assert_scalar("Root.Destination.Renamed", "1");
}

#[test]
fn programmable_skin_rejects_candidate_rebase_when_live_and_candidate_rename_same_node() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Original", "1");
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_rename =
        skin.try_rename_candidate_node(&handle, node_key.clone(), "CandidateName");
    assert!(candidate_rename.accepted, "{:?}", candidate_rename.error);

    let live_rename = skin.try_rename("Root.Original", "LiveName");
    assert!(live_rename.accepted, "{:?}", live_rename.error);
    skin.assert_scalar("Root.LiveName", "1");
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes.contains(&node_key)
    ));
    assert!(
        skin.state()
            .node(&NodeId::new("Root.CandidateName"))
            .is_none()
    );
    skin.assert_scalar("Root.LiveName", "1");
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rebases_candidate_rename_over_live_sibling_add_without_name_collision() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "Original", "1");
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_rename = skin.try_rename_candidate_node(&handle, node_key.clone(), "Renamed");
    assert!(candidate_rename.accepted, "{:?}", candidate_rename.error);

    let live_add = skin.try_add_node(Some("Root.Parent"), "LiveAdded", "2");
    assert!(live_add.accepted, "{:?}", live_add.error);
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.LiveAdded", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(
        rebased
            .nodes
            .iter()
            .any(|node| { node.key == node_key && node.id == NodeId::new("Root.Parent.Renamed") })
    );
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("Root.Parent.LiveAdded") && node.content_text == "2"
    }));
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.Renamed"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.LiveAdded", "2");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.Original"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.Renamed", "1");
    skin.assert_scalar("Root.Parent.LiveAdded", "2");
}

#[test]
fn programmable_skin_rejects_candidate_rename_over_live_sibling_add_name_collision() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "Original", "1");
    let parent_key = skin
        .state()
        .node(&NodeId::new("Root.Parent"))
        .expect("Parent should project")
        .key
        .clone();
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_rename = skin.try_rename_candidate_node(&handle, node_key, "LiveAdded");
    assert!(candidate_rename.accepted, "{:?}", candidate_rename.error);

    let live_add = skin.try_add_node(Some("Root.Parent"), "LiveAdded", "2");
    assert!(live_add.accepted, "{:?}", live_add.error);
    let rebase = skin.try_rebase_candidate(&handle);

    assert!(!rebase.accepted);
    assert!(matches!(
        rebase.error,
        Some(IntentError::CandidateRebaseConflict {
            overlapping_nodes,
            ..
        }) if overlapping_nodes.contains(&parent_key)
    ));
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.LiveAdded", "2");
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.LiveAdded"))
            .is_some()
    );
    assert!(
        skin.state()
            .candidates
            .iter()
            .any(|candidate| candidate.handle == handle)
    );
}

#[test]
fn programmable_skin_rebases_candidate_rename_over_live_sibling_reorder() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "Original", "1");
    skin.add_node(Some("Root.Parent"), "Reordered", "2");
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_rename = skin.try_rename_candidate_node(&handle, node_key.clone(), "Renamed");
    assert!(candidate_rename.accepted, "{:?}", candidate_rename.error);

    let live_reorder = skin.try_reorder("Root.Parent.Reordered", 0);
    assert!(live_reorder.accepted, "{:?}", live_reorder.error);
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.Reordered", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(
        rebased
            .nodes
            .iter()
            .any(|node| { node.key == node_key && node.id == NodeId::new("Root.Parent.Renamed") })
    );
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.Renamed"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.Original", "1");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.Original"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.Renamed", "1");
    skin.assert_scalar("Root.Parent.Reordered", "2");
}

#[test]
fn programmable_skin_rebases_candidate_reorder_over_live_sibling_rename() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "Original", "1");
    skin.add_node(Some("Root.Parent"), "RenameMe", "2");
    let node_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Original"))
        .expect("Original should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_reorder = skin.try_reorder_candidate_node(&handle, node_key.clone(), 1);
    assert!(candidate_reorder.accepted, "{:?}", candidate_reorder.error);

    let live_rename = skin.try_rename("Root.Parent.RenameMe", "Renamed");
    assert!(live_rename.accepted, "{:?}", live_rename.error);
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.Renamed", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(
        rebased
            .nodes
            .iter()
            .any(|node| { node.key == node_key && node.id == NodeId::new("Root.Parent.Original") })
    );
    assert!(
        rebased.nodes.iter().any(|node| {
            node.id == NodeId::new("Root.Parent.Renamed") && node.content_text == "2"
        })
    );
    skin.assert_scalar("Root.Parent.Renamed", "2");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_scalar("Root.Parent.Original", "1");
    skin.assert_scalar("Root.Parent.Renamed", "2");
}

#[test]
fn programmable_skin_rebases_candidate_add_over_live_sibling_delete() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "DeleteMe", "1");
    let parent_key = skin
        .state()
        .node(&NodeId::new("Root.Parent"))
        .expect("Parent should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_add = skin.try_add_candidate_node(
        &handle,
        Some(parent_key),
        "CandidateAdded",
        InitialNodeContentProjection::Literal {
            content: "2".to_string(),
        },
        false,
    );
    assert!(candidate_add.accepted, "{:?}", candidate_add.error);

    let live_delete = skin.try_delete("Root.Parent.DeleteMe");
    assert!(live_delete.accepted, "{:?}", live_delete.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.DeleteMe"))
            .is_none()
    );
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("Root.Parent.CandidateAdded") && node.content_text == "2"
    }));
    assert!(
        !rebased
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.Parent.DeleteMe"))
    );
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.CandidateAdded"))
            .is_none()
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.DeleteMe"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.CandidateAdded", "2");
}

#[test]
fn programmable_skin_rebases_candidate_delete_over_live_sibling_add() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "DeleteMe", "1");
    let delete_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.DeleteMe"))
        .expect("DeleteMe should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_delete = skin.try_delete_candidate_node(&handle, delete_key);
    assert!(candidate_delete.accepted, "{:?}", candidate_delete.error);

    let live_add = skin.try_add_node(Some("Root.Parent"), "LiveAdded", "2");
    assert!(live_add.accepted, "{:?}", live_add.error);
    skin.assert_scalar("Root.Parent.DeleteMe", "1");
    skin.assert_scalar("Root.Parent.LiveAdded", "2");
    let current_revision = skin
        .state()
        .revision
        .workspace_revision_id
        .clone()
        .expect("published workspace revision should project");

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(rebased.basis_revision_id, current_revision);
    assert!(
        !rebased
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.Parent.DeleteMe"))
    );
    assert!(rebased.nodes.iter().any(|node| {
        node.id == NodeId::new("Root.Parent.LiveAdded") && node.content_text == "2"
    }));
    skin.assert_scalar("Root.Parent.LiveAdded", "2");

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.DeleteMe"))
            .is_none()
    );
    skin.assert_scalar("Root.Parent.LiveAdded", "2");
}

#[test]
fn programmable_skin_rebases_candidate_add_over_live_sibling_reorder() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "First", "1");
    skin.add_node(Some("Root.Parent"), "Second", "2");
    let parent_key = skin
        .state()
        .node(&NodeId::new("Root.Parent"))
        .expect("Parent should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_add = skin.try_add_candidate_node(
        &handle,
        Some(parent_key),
        "CandidateAdded",
        InitialNodeContentProjection::Literal {
            content: "3".to_string(),
        },
        false,
    );
    assert!(candidate_add.accepted, "{:?}", candidate_add.error);

    let live_reorder = skin.try_reorder("Root.Parent.Second", 0);
    assert!(live_reorder.accepted, "{:?}", live_reorder.error);
    skin.assert_children("Root.Parent", &["Root.Parent.Second", "Root.Parent.First"]);

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(
        candidate_children(&rebased, "Root.Parent"),
        vec![
            NodeId::new("Root.Parent.Second"),
            NodeId::new("Root.Parent.First"),
            NodeId::new("Root.Parent.CandidateAdded")
        ]
    );
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.CandidateAdded"))
            .is_none()
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_children(
        "Root.Parent",
        &[
            "Root.Parent.Second",
            "Root.Parent.First",
            "Root.Parent.CandidateAdded",
        ],
    );
    skin.assert_scalar("Root.Parent.CandidateAdded", "3");
}

#[test]
fn programmable_skin_rebases_candidate_reorder_over_live_sibling_add() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "First", "1");
    skin.add_node(Some("Root.Parent"), "Second", "2");
    let second_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Second"))
        .expect("Second should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_reorder = skin.try_reorder_candidate_node(&handle, second_key, 0);
    assert!(candidate_reorder.accepted, "{:?}", candidate_reorder.error);

    let live_add = skin.try_add_node(Some("Root.Parent"), "LiveAdded", "3");
    assert!(live_add.accepted, "{:?}", live_add.error);
    skin.assert_children(
        "Root.Parent",
        &[
            "Root.Parent.First",
            "Root.Parent.Second",
            "Root.Parent.LiveAdded",
        ],
    );

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(
        candidate_children(&rebased, "Root.Parent"),
        vec![
            NodeId::new("Root.Parent.Second"),
            NodeId::new("Root.Parent.First"),
            NodeId::new("Root.Parent.LiveAdded")
        ]
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_children(
        "Root.Parent",
        &[
            "Root.Parent.Second",
            "Root.Parent.First",
            "Root.Parent.LiveAdded",
        ],
    );
    skin.assert_scalar("Root.Parent.LiveAdded", "3");
}

#[test]
fn programmable_skin_rebases_candidate_delete_over_live_sibling_reorder() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "First", "1");
    skin.add_node(Some("Root.Parent"), "Second", "2");
    skin.add_node(Some("Root.Parent"), "DeleteMe", "3");
    let delete_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.DeleteMe"))
        .expect("DeleteMe should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_delete = skin.try_delete_candidate_node(&handle, delete_key);
    assert!(candidate_delete.accepted, "{:?}", candidate_delete.error);

    let live_reorder = skin.try_reorder("Root.Parent.Second", 0);
    assert!(live_reorder.accepted, "{:?}", live_reorder.error);

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(
        candidate_children(&rebased, "Root.Parent"),
        vec![
            NodeId::new("Root.Parent.Second"),
            NodeId::new("Root.Parent.First")
        ]
    );
    assert!(
        !rebased
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.Parent.DeleteMe"))
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_children("Root.Parent", &["Root.Parent.Second", "Root.Parent.First"]);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.DeleteMe"))
            .is_none()
    );
}

#[test]
fn programmable_skin_rebases_candidate_reorder_over_live_sibling_delete() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Parent", "");
    skin.add_node(Some("Root.Parent"), "First", "1");
    skin.add_node(Some("Root.Parent"), "Second", "2");
    skin.add_node(Some("Root.Parent"), "DeleteMe", "3");
    let second_key = skin
        .state()
        .node(&NodeId::new("Root.Parent.Second"))
        .expect("Second should project")
        .key
        .clone();

    let open = skin.try_open_candidate();
    assert!(open.accepted, "{:?}", open.error);
    let handle = skin.state().candidates[0].handle.clone();
    let candidate_reorder = skin.try_reorder_candidate_node(&handle, second_key, 0);
    assert!(candidate_reorder.accepted, "{:?}", candidate_reorder.error);

    let live_delete = skin.try_delete("Root.Parent.DeleteMe");
    assert!(live_delete.accepted, "{:?}", live_delete.error);

    let rebase = skin.try_rebase_candidate(&handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let rebased = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.handle == handle)
        .expect("rebased candidate should remain retained")
        .clone();
    assert_eq!(
        candidate_children(&rebased, "Root.Parent"),
        vec![
            NodeId::new("Root.Parent.Second"),
            NodeId::new("Root.Parent.First")
        ]
    );
    assert!(
        !rebased
            .nodes
            .iter()
            .any(|node| node.id == NodeId::new("Root.Parent.DeleteMe"))
    );

    let commit = skin.try_commit_candidate(&handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_children("Root.Parent", &["Root.Parent.Second", "Root.Parent.First"]);
    assert!(
        skin.state()
            .node(&NodeId::new("Root.Parent.DeleteMe"))
            .is_none()
    );
}

#[test]
fn programmable_skin_projects_layered_child_candidate_values() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let b_key = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();
    skin.assert_scalar("Root.B", "2");

    let parent_open = skin.try_open_candidate();
    assert!(parent_open.accepted, "{:?}", parent_open.error);
    let parent_handle = skin.state().candidates[0].handle.clone();
    assert!(
        skin.try_edit_candidate_content(&parent_handle, "Root.A", "5")
            .accepted
    );
    assert!(skin.try_evaluate_candidate(&parent_handle).accepted);

    let child_open = skin.try_open_child_candidate(&parent_handle);
    assert!(child_open.accepted, "{:?}", child_open.error);
    let child_handle = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.parent_handle.as_deref() == Some(parent_handle.as_str()))
        .expect("child candidate should project parent handle")
        .handle
        .clone();
    assert!(skin.try_evaluate_candidate(&child_handle).accepted);
    let state = skin.state();
    let child = state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == child_handle)
        .expect("child candidate should remain projected");
    assert_eq!(child.parent_handle.as_deref(), Some(parent_handle.as_str()));
    assert_eq!(
        child
            .values_by_key
            .get(&b_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("6")
    );
    skin.assert_scalar("Root.B", "2");

    let parent_discard = skin.try_discard_candidate(&parent_handle);
    assert!(!parent_discard.accepted);
    assert!(matches!(
        parent_discard.error,
        Some(IntentError::CandidateHasRetainedChild { .. })
    ));

    let child_commit = skin.try_commit_candidate(&child_handle);
    assert!(child_commit.accepted, "{:?}", child_commit.error);
    skin.assert_scalar("Root.B", "6");
    assert_eq!(skin.state().candidates.len(), 1);
    assert_eq!(skin.state().candidates[0].handle, parent_handle);

    let stale_parent = skin.try_commit_candidate(&parent_handle);
    assert!(!stale_parent.accepted);
    assert!(matches!(
        stale_parent.error,
        Some(IntentError::CandidateBasisNotCurrent { .. })
    ));
}

#[test]
fn programmable_skin_rebases_parented_candidate_as_flattened_layer() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "10");
    let b_key = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("Root.B should project")
        .key
        .clone();

    let parent_open = skin.try_open_candidate();
    assert!(parent_open.accepted, "{:?}", parent_open.error);
    let parent_handle = skin.state().candidates[0].handle.clone();
    assert!(
        skin.try_edit_candidate_content(&parent_handle, "Root.A", "5")
            .accepted
    );

    let child_open = skin.try_open_child_candidate(&parent_handle);
    assert!(child_open.accepted, "{:?}", child_open.error);
    let child_handle = skin
        .state()
        .candidates
        .iter()
        .find(|candidate| candidate.parent_handle.as_deref() == Some(parent_handle.as_str()))
        .expect("child candidate should project")
        .handle
        .clone();
    assert!(
        skin.try_edit_candidate_content(&child_handle, "Root.A", "7")
            .accepted
    );

    skin.edit("Root.C", "20");
    skin.assert_scalar("Root.B", "2");
    assert!(!skin.try_discard_candidate(&parent_handle).accepted);

    let rebase = skin.try_rebase_candidate(&child_handle);
    assert!(rebase.accepted, "{:?}", rebase.error);
    let state = skin.state();
    let child = state
        .candidates
        .iter()
        .find(|candidate| candidate.handle == child_handle)
        .expect("child candidate should remain projected");
    assert_eq!(child.parent_handle, None);
    assert!(
        child.values_by_key.is_empty(),
        "flattened rebase should not carry stale candidate values"
    );

    let parent_discard = skin.try_discard_candidate(&parent_handle);
    assert!(parent_discard.accepted, "{:?}", parent_discard.error);
    assert!(skin.try_evaluate_candidate(&child_handle).accepted);
    assert_eq!(
        skin.state().candidates[0]
            .values_by_key
            .get(&b_key)
            .map(NodeValueProjection::display_text)
            .as_deref(),
        Some("8")
    );
    skin.assert_scalar("Root.B", "2");

    let commit = skin.try_commit_candidate(&child_handle);
    assert!(commit.accepted, "{:?}", commit.error);
    skin.assert_scalar("Root.B", "8");
    assert!(skin.state().candidates.is_empty());
}

#[test]
fn programmable_skin_projects_per_node_published_value_epochs() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "=2");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=100");

    let initial = skin.state();
    let initial_a_epoch = initial.node(&NodeId::new("Root.A")).unwrap().value_epoch;
    let initial_b_epoch = initial.node(&NodeId::new("Root.B")).unwrap().value_epoch;
    let initial_c_epoch = initial.node(&NodeId::new("Root.C")).unwrap().value_epoch;
    assert!(initial_a_epoch.is_some());
    assert!(initial_b_epoch.is_some());
    assert!(initial_c_epoch.is_some());

    skin.edit("Root.A", "=3");
    let edited = skin.state();
    assert_eq!(
        edited
            .node(&NodeId::new("Root.A"))
            .unwrap()
            .computed_value
            .display_text(),
        "3"
    );
    assert_eq!(
        edited
            .node(&NodeId::new("Root.B"))
            .unwrap()
            .computed_value
            .display_text(),
        "4"
    );
    assert_ne!(
        edited.node(&NodeId::new("Root.A")).unwrap().value_epoch,
        initial_a_epoch
    );
    assert_ne!(
        edited.node(&NodeId::new("Root.B")).unwrap().value_epoch,
        initial_b_epoch
    );
    assert_eq!(
        edited.node(&NodeId::new("Root.C")).unwrap().value_epoch,
        initial_c_epoch
    );

    let detail = edited
        .active_node_detail(&dnatreecalc_skin_framework::SelectionState::with_primary(
            Some(NodeId::new("Root.B")),
        ))
        .expect("active detail projects selected formula");
    assert_eq!(
        detail.value_epoch,
        edited.node(&NodeId::new("Root.B")).unwrap().value_epoch
    );
}

#[test]
fn programmable_skin_previews_recalc_plan_from_host_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let value_plan = harness.preview_recalc_plan(&[RecalcPlanMutation::SetNodeInput {
        node: NodeId::new("Root.A"),
    }]);
    assert_eq!(value_plan.estimated_node_count, 3);
    assert_eq!(
        value_plan
            .invalidated_nodes
            .iter()
            .map(|entry| entry.node.as_str())
            .collect::<Vec<_>>(),
        vec!["Root.A", "Root.B", "Root.C"]
    );
    assert!(value_plan.requires_rebind.is_empty());
    assert_eq!(
        value_plan
            .evaluation_order
            .iter()
            .map(NodeId::as_str)
            .collect::<Vec<_>>(),
        vec!["Root.B", "Root.C"]
    );

    let formula_plan = harness.preview_recalc_plan(&[RecalcPlanMutation::EditContent {
        node: NodeId::new("Root.B"),
        content: "=A+2".to_string(),
    }]);
    assert_eq!(formula_plan.estimated_node_count, 2);
    assert_eq!(formula_plan.requires_rebind, vec![NodeId::new("Root.B")]);
    assert!(
        formula_plan
            .invalidated_nodes
            .iter()
            .find(|entry| entry.node == NodeId::new("Root.B"))
            .is_some_and(|entry| entry
                .reasons
                .contains(&InvalidationReasonProjection::StructuralRebindRequired))
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    skin.assert_scalar("Root.C", "3");
}

#[test]
fn programmable_skin_previews_formula_bind_from_host_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let valid = harness.preview_formula_bind("Root.B", "=A+2");
    assert_eq!(valid.node, NodeId::new("Root.B"));
    assert_eq!(valid.input_kind, FormulaBindPreviewInputKind::Formula);
    assert!(valid.legal, "{valid:?}");
    assert!(valid.diagnostics.is_empty());
    assert!(valid.profile_violations.is_empty());
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    skin.assert_scalar("Root.B", "2");

    let syntax = harness.preview_formula_bind("Root.B", "=1+");
    assert!(!syntax.legal);
    assert!(
        syntax
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax })
    );

    let bind = harness.preview_formula_bind("Root.B", "=LAMBDA(x,x,x)");
    assert!(!bind.legal);
    assert!(bind.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind
            && diagnostic.message == "duplicate LAMBDA parameter name 'x'"
    }));

    let literal = harness.preview_formula_bind("Root.B", "7");
    assert_eq!(literal.input_kind, FormulaBindPreviewInputKind::Literal);
    assert!(literal.legal);
    assert!(literal.diagnostics.is_empty());
}

#[test]
fn programmable_skin_inserts_formula_reference_through_oxfml_authoring() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "7");
    skin.add_node(Some("Root"), "Total", "=SUM(0)");

    let before = skin.state();
    let total_key = before
        .node(&NodeId::new("Root.Total"))
        .expect("formula target projects")
        .key
        .clone();
    let a_key = before
        .node(&NodeId::new("Root.A"))
        .expect("reference target projects")
        .key
        .clone();

    let receipt = skin.try_insert_formula_reference(
        total_key,
        "=SUM(0)",
        "=SUM(".chars().count(),
        1,
        FormulaReferenceInsertionTarget::Node(a_key),
    );
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);
    let insertion = formula_reference_inserted_delta(&receipt);
    assert_eq!(insertion.inserted_text, "A");
    assert_eq!(insertion.updated_formula_text, "=SUM(A)");
    assert_eq!(insertion.applied_start, "=SUM(".chars().count());
    assert_eq!(insertion.applied_len, 1);
    assert!(matches!(
        insertion.target,
        FormulaReferenceInsertionTarget::Node(_)
    ));

    let after = skin.state();
    let total = after
        .node(&NodeId::new("Root.Total"))
        .expect("formula target still projects");
    assert_eq!(total.content_text, "=SUM(A)");
    assert_eq!(total.computed_value.display_text(), "7");
    assert!(
        after
            .dependencies
            .edges_by_owner_key
            .get(&total.key)
            .is_some_and(|edges| edges
                .iter()
                .any(|edge| edge.target == NodeId::new("Root.A"))),
        "inserted reference should be resolved by OxCalc after OxFml composition"
    );
}

#[test]
fn programmable_skin_inserts_reference_collection_through_oxfml_authoring() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Base", "");
    skin.add_node(Some("Base"), "A", "2");
    skin.add_node(Some("Base"), "B", "3");
    skin.add_node(None, "Total", "=SUM(0)");

    let before = skin.state();
    let total_key = before
        .node(&NodeId::new("Total"))
        .expect("formula target projects")
        .key
        .clone();
    let base_key = before
        .node(&NodeId::new("Base"))
        .expect("collection base projects")
        .key
        .clone();

    let receipt = skin.try_insert_formula_reference(
        total_key,
        "=SUM(0)",
        "=SUM(".chars().count(),
        1,
        FormulaReferenceInsertionTarget::HostReferenceCollection {
            base: Some(base_key),
            collection_family: "children".to_string(),
        },
    );
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);
    let insertion = formula_reference_inserted_delta(&receipt);
    assert_eq!(insertion.inserted_text, "Base.@CHILDREN");
    assert_eq!(insertion.updated_formula_text, "=SUM(Base.@CHILDREN)");
    assert!(matches!(
        insertion.target,
        FormulaReferenceInsertionTarget::HostReferenceCollection { .. }
    ));

    let after = skin.state();
    let total = after
        .node(&NodeId::new("Total"))
        .expect("formula target still projects");
    assert_eq!(total.content_text, "=SUM(Base.@CHILDREN)");
    assert_eq!(total.computed_value.display_text(), "5");
    let outgoing = after
        .dependencies
        .edges_by_owner_key
        .get(&total.key)
        .expect("collection insertion should publish dependency edges");
    assert!(
        outgoing
            .iter()
            .any(|edge| edge.target == NodeId::new("Base.A"))
    );
    assert!(
        outgoing
            .iter()
            .any(|edge| edge.target == NodeId::new("Base.B"))
    );
}

#[test]
fn programmable_skin_inserts_structural_selector_through_oxfml_authoring() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "2");
    skin.add_node(Some("Root"), "B", "3");
    skin.add_node(Some("Root"), "Total", "=SUM(0)");

    let before = skin.state();
    let total_key = before
        .node(&NodeId::new("Root.Total"))
        .expect("formula target projects")
        .key
        .clone();
    let a_key = before
        .node(&NodeId::new("Root.A"))
        .expect("selector base projects")
        .key
        .clone();

    let receipt = skin.try_insert_formula_reference(
        total_key,
        "=SUM(0)",
        "=SUM(".chars().count(),
        1,
        FormulaReferenceInsertionTarget::HostStructuralSelector {
            base: a_key,
            selector_family: "next".to_string(),
        },
    );
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);
    let insertion = formula_reference_inserted_delta(&receipt);
    assert_eq!(insertion.inserted_text, "A.@NEXT");
    assert_eq!(insertion.updated_formula_text, "=SUM(A.@NEXT)");
    assert!(matches!(
        insertion.target,
        FormulaReferenceInsertionTarget::HostStructuralSelector { .. }
    ));

    let after = skin.state();
    let total = after
        .node(&NodeId::new("Root.Total"))
        .expect("formula target still projects");
    assert_eq!(total.content_text, "=SUM(A.@NEXT)");
    assert_eq!(total.computed_value.display_text(), "3");
    assert!(
        after
            .dependencies
            .edges_by_owner_key
            .get(&total.key)
            .is_some_and(|edges| edges
                .iter()
                .any(|edge| edge.target == NodeId::new("Root.B"))),
        "structural selector insertion should resolve through OxCalc"
    );
}

#[test]
fn programmable_skin_previews_table_formula_bind_from_table_subjects() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    let before_revision = revision_fingerprint(&skin.state().revision);

    let body = harness.preview_table_column_formula_bind("SalesTable", "col:tax", "=[@Amount]*0.2");
    assert_eq!(body.table, NodeId::new("SalesTable"));
    assert_eq!(body.table_id, "tree-table:sales");
    assert_eq!(body.column_id, "col:tax");
    assert_eq!(body.region, TableCellRegionProjection::Body);
    assert_eq!(body.input_kind, FormulaBindPreviewInputKind::Formula);
    assert!(body.legal, "{body:?}");
    assert!(body.diagnostics.is_empty(), "{body:?}");
    assert!(body.profile_violations.is_empty(), "{body:?}");

    let new_column = harness.preview_new_table_column_formula_bind(
        "SalesTable",
        "col:double",
        "Double",
        "=[@Amount]*2",
    );
    assert_eq!(new_column.table, NodeId::new("SalesTable"));
    assert_eq!(new_column.table_id, "tree-table:sales");
    assert_eq!(new_column.column_id, "col:double");
    assert_eq!(new_column.region, TableCellRegionProjection::Body);
    assert_eq!(new_column.input_kind, FormulaBindPreviewInputKind::Formula);
    assert!(new_column.legal, "{new_column:?}");
    assert!(new_column.diagnostics.is_empty(), "{new_column:?}");
    assert!(new_column.profile_violations.is_empty(), "{new_column:?}");

    let new_column_impact = harness.preview_new_table_column_formula_impact(
        "SalesTable",
        "col:double",
        "Double",
        "=[@Amount]*2",
    );
    assert!(new_column_impact.legal, "{new_column_impact:?}");
    assert!(new_column_impact.blocked_reason.is_none());
    assert!(new_column_impact.bind_diagnostics.is_empty());
    assert!(new_column_impact.profile_violations.is_empty());
    assert!(
        new_column_impact
            .requires_rebind
            .contains(&NodeId::new("SalesTable"))
    );
    let MutationImpactIntentProjection::AddTableFormulaColumn {
        table,
        column_id,
        name,
        formula_text,
    } = &new_column_impact.intent
    else {
        panic!("expected add table formula column impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(column_id, "col:double");
    assert_eq!(name, "Double");
    assert_eq!(formula_text, "=[@Amount]*2");
    let sales_entry = new_column_impact
        .invalidation_plan
        .invalidated_nodes
        .iter()
        .find(|entry| entry.node == NodeId::new("SalesTable"))
        .expect("SalesTable should be invalidated");
    assert!(sales_entry.requires_rebind);
    assert!(
        sales_entry
            .reasons
            .contains(&InvalidationReasonProjection::StructuredTableColumnChanged)
    );
    assert!(
        sales_entry
            .reasons
            .contains(&InvalidationReasonProjection::StructuredTableRegionChanged)
    );
    assert!(
        sales_entry
            .reasons
            .contains(&InvalidationReasonProjection::StructuredTableContextChanged)
    );

    let totals =
        harness.preview_table_totals_formula_bind("SalesTable", "col:amount", "=SUM([Amount])");
    assert_eq!(totals.table, NodeId::new("SalesTable"));
    assert_eq!(totals.table_id, "tree-table:sales");
    assert_eq!(totals.column_id, "col:amount");
    assert_eq!(totals.region, TableCellRegionProjection::Totals);
    assert!(totals.legal, "{totals:?}");
    assert!(totals.diagnostics.is_empty(), "{totals:?}");

    let syntax =
        harness.preview_table_totals_formula_bind("SalesTable", "col:amount", "=SUM([Amount]) +");
    assert!(!syntax.legal);
    assert!(
        syntax
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    let after = skin.state();
    let table = after
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("SalesTable projects");
    let tax = table
        .columns
        .iter()
        .find(|column| column.column_id == "col:tax")
        .expect("tax column projects");
    let TableColumnBodyProjection::Formula(tax_formula) = &tax.body else {
        panic!("tax column remains formula-backed");
    };
    assert_eq!(tax_formula.formula_text, "=[@Amount] * 0.1");
    assert!(
        !table
            .columns
            .iter()
            .any(|column| column.column_id == "col:double")
    );
}

#[test]
fn programmable_skin_previews_table_row_column_structural_impact() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    let before_revision = revision_fingerprint(&skin.state().revision);

    let add_row = harness.preview_add_table_row_impact(
        "SalesTable",
        "row:south",
        &[("col:region", "South"), ("col:amount", "7")],
    );
    assert!(add_row.legal, "{add_row:?}");
    assert!(add_row.blocked_reason.is_none());
    assert!(add_row.requires_rebind.contains(&NodeId::new("SalesTable")));
    let MutationImpactIntentProjection::AddTableRow {
        table,
        row_id,
        values,
    } = &add_row.intent
    else {
        panic!("expected add-table-row impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(row_id, "row:south");
    assert_eq!(values.len(), 2);

    let duplicate_row =
        harness.preview_add_table_row_impact("SalesTable", "row:east", &[("col:amount", "9")]);
    assert!(!duplicate_row.legal);
    assert_eq!(
        duplicate_row.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::TableCollision)
    );
    let duplicate_row_input = harness.preview_add_table_row_impact(
        "SalesTable",
        "row:dup",
        &[("col:amount", "9"), ("col:amount", "10")],
    );
    assert!(!duplicate_row_input.legal);
    assert_eq!(
        duplicate_row_input.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::DuplicateInput)
    );

    let delete_row = harness.preview_delete_table_row_impact("SalesTable", "row:east");
    assert!(delete_row.legal, "{delete_row:?}");
    assert!(
        delete_row
            .requires_rebind
            .contains(&NodeId::new("SalesTable"))
    );
    let MutationImpactIntentProjection::DeleteTableRow { table, row_id } = &delete_row.intent
    else {
        panic!("expected delete-table-row impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(row_id, "row:east");

    let rename_row =
        harness.preview_rename_table_row_impact("SalesTable", "row:west", "row:west-renamed");
    assert!(rename_row.legal, "{rename_row:?}");
    let MutationImpactIntentProjection::RenameTableRow {
        table,
        row_id,
        new_row_id,
    } = &rename_row.intent
    else {
        panic!("expected rename-table-row impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(row_id, "row:west");
    assert_eq!(new_row_id, "row:west-renamed");

    let reorder_row = harness.preview_reorder_table_row_impact("SalesTable", "row:north", 0);
    assert!(reorder_row.legal, "{reorder_row:?}");
    let MutationImpactIntentProjection::ReorderTableRow {
        table,
        row_id,
        new_index,
    } = &reorder_row.intent
    else {
        panic!("expected reorder-table-row impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(row_id, "row:north");
    assert_eq!(new_index, &0);

    let add_column = harness.preview_add_table_column_impact(
        "SalesTable",
        "col:notes",
        "Notes",
        &[("row:east", "ok")],
    );
    assert!(add_column.legal, "{add_column:?}");
    assert!(
        add_column
            .requires_rebind
            .contains(&NodeId::new("SalesTable"))
    );
    let MutationImpactIntentProjection::AddTableColumn {
        table,
        column_id,
        name,
        values,
    } = &add_column.intent
    else {
        panic!("expected add-table-column impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(column_id, "col:notes");
    assert_eq!(name, "Notes");
    assert_eq!(values.len(), 1);

    let duplicate_column =
        harness.preview_add_table_column_impact("SalesTable", "col:amount", "Amount", &[]);
    assert!(!duplicate_column.legal);
    assert_eq!(
        duplicate_column.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::TableCollision)
    );
    let duplicate_column_input = harness.preview_add_table_column_impact(
        "SalesTable",
        "col:dup",
        "Dup",
        &[("row:east", "9"), ("row:east", "10")],
    );
    assert!(!duplicate_column_input.legal);
    assert_eq!(
        duplicate_column_input.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::DuplicateInput)
    );

    let delete_column = harness.preview_delete_table_column_impact("SalesTable", "col:region");
    assert!(delete_column.legal, "{delete_column:?}");
    let MutationImpactIntentProjection::DeleteTableColumn { table, column_id } =
        &delete_column.intent
    else {
        panic!("expected delete-table-column impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(column_id, "col:region");

    let rename_column =
        harness.preview_rename_table_column_impact("SalesTable", "col:amount", "Amount USD");
    assert!(rename_column.legal, "{rename_column:?}");
    let MutationImpactIntentProjection::RenameTableColumn {
        table,
        column_id,
        name,
    } = &rename_column.intent
    else {
        panic!("expected rename-table-column impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(column_id, "col:amount");
    assert_eq!(name, "Amount USD");

    let reorder_column = harness.preview_reorder_table_column_impact("SalesTable", "col:tax", 0);
    assert!(reorder_column.legal, "{reorder_column:?}");
    let MutationImpactIntentProjection::ReorderTableColumn {
        table,
        column_id,
        new_index,
    } = &reorder_column.intent
    else {
        panic!("expected reorder-table-column impact intent");
    };
    assert_eq!(table, &NodeId::new("SalesTable"));
    assert_eq!(column_id, "col:tax");
    assert_eq!(new_index, &0);

    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    let state = skin.state();
    let table = state.tables.get(&NodeId::new("SalesTable")).unwrap();
    assert!(!table.rows.iter().any(|row| row.row_id == "row:south"));
    assert!(
        !table
            .columns
            .iter()
            .any(|column| column.column_id == "col:notes")
    );
    assert!(table.rows.iter().any(|row| row.row_id == "row:east"));
    assert!(
        table
            .columns
            .iter()
            .any(|column| column.column_id == "col:region")
    );
}

#[test]
fn programmable_skin_previews_content_edit_legality_impact_from_host_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let impact = harness.preview_content_edit_impact("Root.B", "=A+2");
    assert!(impact.legal, "{impact:?}");
    assert!(impact.blocked_reason.is_none());
    assert!(impact.bind_diagnostics.is_empty());
    assert!(impact.profile_violations.is_empty());
    assert_eq!(impact.requires_rebind, vec![NodeId::new("Root.B")]);
    assert_eq!(impact.affected_refs, vec![NodeId::new("Root.C")]);
    assert!(impact.orphaned_dependents.is_empty());
    assert!(impact.collisions.is_empty());
    assert_eq!(impact.invalidation_plan.estimated_node_count, 2);
    let MutationImpactIntentProjection::EditContent { node, content } = &impact.intent else {
        panic!("expected edit content impact intent");
    };
    assert_eq!(node, &NodeId::new("Root.B"));
    assert_eq!(content, "=A+2");
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    skin.assert_scalar("Root.C", "3");

    let syntax = harness.preview_content_edit_impact("Root.B", "=1+");
    assert!(!syntax.legal);
    assert_eq!(
        syntax.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics)
    );
    assert!(
        syntax
            .bind_diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax })
    );

    let bind = harness.preview_content_edit_impact("Root.B", "=LAMBDA(x,x,x)");
    assert!(!bind.legal);
    assert_eq!(
        bind.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::BindDiagnostics)
    );
    assert!(bind.bind_diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind
            && diagnostic.message == "duplicate LAMBDA parameter name 'x'"
    }));
}

#[test]
fn programmable_skin_previews_scoped_content_edit_legality_impact_from_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    skin.add_node(Some("Root"), "D", "=C+1");
    let before_revision = revision_fingerprint(&skin.state().revision);
    let state = skin.state();
    let b_key = state
        .node(&NodeId::new("Root.B"))
        .expect("B projects")
        .key
        .clone();
    let c_key = state
        .node(&NodeId::new("Root.C"))
        .expect("C projects")
        .key
        .clone();

    let scope = AuthoringScope::Nodes(vec![b_key.clone(), c_key.clone()]);
    let impact = harness.preview_scoped_content_edit_impact(scope.clone(), "=A+10");

    assert!(impact.legal, "{impact:?}");
    assert!(impact.blocked_reason.is_none());
    assert!(impact.bind_diagnostics.is_empty());
    assert!(impact.profile_violations.is_empty());
    assert!(impact.requires_rebind.contains(&NodeId::new("Root.B")));
    assert!(impact.requires_rebind.contains(&NodeId::new("Root.C")));
    assert_eq!(impact.affected_refs, vec![NodeId::new("Root.D")]);
    assert_eq!(impact.invalidation_plan.estimated_node_count, 3);
    let MutationImpactIntentProjection::EditScopedContent {
        scope: projected_scope,
        content,
    } = &impact.intent
    else {
        panic!("expected scoped content impact intent");
    };
    assert_eq!(content, "=A+10");
    assert_eq!(
        projected_scope,
        &AuthoringScope::Nodes(vec![b_key.clone(), c_key.clone()])
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    skin.assert_scalar("Root.D", "4");

    let edit = skin.try_edit_scoped_content(scope.clone(), "=A+10");
    assert!(edit.accepted, "{:?}", edit.error);
    assert!(
        edit.transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:programmable-skin-ir:")),
        "{edit:?}"
    );
    let edited_state = skin.state();
    assert_eq!(
        edited_state
            .node(&NodeId::new("Root.B"))
            .expect("B projects after scoped edit")
            .content_text,
        "=A+10"
    );
    assert_eq!(
        edited_state
            .node(&NodeId::new("Root.C"))
            .expect("C projects after scoped edit")
            .content_text,
        "=A+10"
    );
    skin.assert_scalar("Root.B", "11");
    skin.assert_scalar("Root.C", "11");
    skin.assert_scalar("Root.D", "12");
}

#[test]
fn programmable_skin_previews_add_node_initial_content_policy() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Existing", "1");
    let literal = harness.preview_add_node_impact(
        Some("Root"),
        "Child",
        InitialNodeContentProjection::Literal {
            content: "=Existing+1".to_string(),
        },
        false,
    );
    assert!(literal.legal, "{literal:?}");
    assert!(literal.blocked_reason.is_none());
    assert!(literal.collisions.is_empty());
    assert!(literal.invalidation_plan.invalidated_nodes.is_empty());
    let MutationImpactIntentProjection::AddNode {
        parent,
        symbol,
        initial,
        is_meta,
    } = &literal.intent
    else {
        panic!("expected add-node impact intent");
    };
    assert_eq!(parent, &Some(NodeId::new("Root")));
    assert_eq!(symbol, "Child");
    assert_eq!(
        initial,
        &InitialNodeContentProjection::Literal {
            content: "=Existing+1".to_string()
        }
    );
    assert!(!is_meta);

    let syntax = harness.preview_add_node_impact(
        Some("Root"),
        "Broken",
        InitialNodeContentProjection::Literal {
            content: "=1+".to_string(),
        },
        false,
    );
    assert!(!syntax.legal, "{syntax:?}");
    assert_eq!(
        syntax.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics)
    );
    assert!(
        syntax
            .bind_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    );

    let bind = harness.preview_add_node_impact(
        Some("Root"),
        "BadBind",
        InitialNodeContentProjection::Literal {
            content: "=LAMBDA(x,x,x)".to_string(),
        },
        false,
    );
    assert!(!bind.legal, "{bind:?}");
    assert_eq!(
        bind.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::BindDiagnostics)
    );
    assert!(
        bind.bind_diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.message == "duplicate LAMBDA parameter name 'x'" })
    );

    let empty_meta = harness.preview_add_node_impact(
        None,
        "MetaRoot",
        InitialNodeContentProjection::Empty,
        true,
    );
    assert!(empty_meta.legal, "{empty_meta:?}");
    let MutationImpactIntentProjection::AddNode {
        parent,
        symbol,
        initial,
        is_meta,
    } = &empty_meta.intent
    else {
        panic!("expected add-node impact intent");
    };
    assert_eq!(parent, &None);
    assert_eq!(symbol, "MetaRoot");
    assert_eq!(initial, &InitialNodeContentProjection::Empty);
    assert!(is_meta);

    let collision = harness.preview_add_node_impact(
        Some("Root"),
        "Existing",
        InitialNodeContentProjection::Empty,
        false,
    );
    assert!(!collision.legal);
    assert_eq!(
        collision.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::NameCollision)
    );
    assert_eq!(collision.collisions.len(), 1);
    assert_eq!(collision.collisions[0].attempted, "Root.Existing");

    let template = harness.preview_add_node_impact(
        Some("Root"),
        "Templated",
        InitialNodeContentProjection::TemplateBound {
            template_id: "starter".to_string(),
        },
        false,
    );
    assert!(template.legal, "{template:?}");
    assert!(template.blocked_reason.is_none());
    assert!(template.bind_diagnostics.is_empty());
    assert!(template.profile_violations.is_empty());

    let template_receipt = skin.try_add_node_initial(
        Some("Root"),
        "Templated",
        InitialNodeContentProjection::TemplateBound {
            template_id: "starter".to_string(),
        },
        false,
    );
    assert!(template_receipt.accepted, "{:?}", template_receipt.error);
    assert_any_transaction(&template_receipt);
    let template_state = skin.state();
    let templated = template_state
        .node(&NodeId::new("Root.Templated"))
        .expect("template-bound node should project");
    assert_eq!(templated.content_text, "=1+1");
    skin.assert_scalar("Root.Templated", "2");

    let unknown_template = harness.preview_add_node_impact(
        Some("Root"),
        "UnknownTemplate",
        InitialNodeContentProjection::TemplateBound {
            template_id: "missing".to_string(),
        },
        false,
    );
    assert!(!unknown_template.legal);
    assert_eq!(
        unknown_template.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::UnsupportedInitialContent)
    );

    let unknown_template_receipt = skin.try_add_node_initial(
        Some("Root"),
        "UnknownTemplate",
        InitialNodeContentProjection::TemplateBound {
            template_id: "missing".to_string(),
        },
        false,
    );
    assert!(!unknown_template_receipt.accepted);
    assert_eq!(
        unknown_template_receipt.error,
        Some(IntentError::UnsupportedInitialContent {
            policy: "template_bound".to_string()
        })
    );
    assert!(skin.state().node(&NodeId::new("Root.Child")).is_none());
    assert!(skin.state().node(&NodeId::new("Root.Broken")).is_none());
    assert!(skin.state().node(&NodeId::new("MetaRoot")).is_none());
    assert!(
        skin.state()
            .node(&NodeId::new("Root.UnknownTemplate"))
            .is_none()
    );
    skin.assert_scalar("Root.Existing", "1");

    let before_rejected_formula = revision_fingerprint(&skin.state().revision);
    let rejected_formula = skin.try_add_node_initial(
        Some("Root"),
        "Broken",
        InitialNodeContentProjection::Literal {
            content: "=1+".to_string(),
        },
        false,
    );
    assert!(!rejected_formula.accepted);
    assert_eq!(
        rejected_formula.error,
        Some(IntentError::InitialContentBindRejected {
            policy: "literal".to_string()
        })
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_rejected_formula
    );

    let meta_receipt =
        skin.try_add_node_initial(None, "MetaRoot", InitialNodeContentProjection::Empty, true);
    assert!(meta_receipt.accepted, "{:?}", meta_receipt.error);
    assert!(skin.state().node(&NodeId::new("MetaRoot")).unwrap().is_meta);
}

#[test]
fn programmable_skin_inherits_table_column_formula_for_add_node_initial_content() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();

    let formula_column =
        skin.try_add_table_formula_column("SalesTable", "col:fixed", "Fixed", "=1+1");
    assert!(formula_column.accepted, "{:?}", formula_column.error);
    assert_table_transaction(&formula_column);

    let preview = harness.preview_add_node_impact(
        Some("SalesTable"),
        "Inherited",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:fixed".to_string(),
        },
        false,
    );
    assert!(preview.legal, "{preview:?}");
    assert!(preview.bind_diagnostics.is_empty());
    assert!(preview.profile_violations.is_empty());

    let add = skin.try_add_node_initial(
        Some("SalesTable"),
        "Inherited",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:fixed".to_string(),
        },
        false,
    );
    assert!(add.accepted, "{:?}", add.error);
    assert_any_transaction(&add);
    let state = skin.state();
    let inherited = state
        .node(&NodeId::new("SalesTable.Inherited"))
        .expect("inherited node projects after add");
    assert_eq!(inherited.content_text, "=1+1");
}

#[test]
fn programmable_skin_rejects_row_context_column_formula_inheritance_without_faking_context() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();

    let preview = harness.preview_add_node_impact(
        None,
        "InheritedTax",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:tax".to_string(),
        },
        false,
    );
    assert!(!preview.legal, "{preview:?}");
    assert_eq!(
        preview.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::BindDiagnostics)
    );
    assert!(!preview.bind_diagnostics.is_empty());

    let rejected = skin.try_add_node_initial(
        None,
        "InheritedTax",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:tax".to_string(),
        },
        false,
    );
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.error,
        Some(IntentError::InitialContentBindRejected {
            policy: "inherit_column_formula".to_string()
        })
    );
    assert!(skin.state().node(&NodeId::new("InheritedTax")).is_none());

    let constant_column = skin.try_add_node_initial(
        None,
        "InheritedAmount",
        InitialNodeContentProjection::InheritColumnFormula {
            table: NodeId::new("SalesTable"),
            column_id: "col:amount".to_string(),
        },
        false,
    );
    assert!(!constant_column.accepted);
    assert_eq!(
        constant_column.error,
        Some(IntentError::ConstantTableColumnFormulaEdit {
            table: "SalesTable".to_string(),
            column_id: "col:amount".to_string()
        })
    );
}

#[test]
fn programmable_skin_previews_rename_legality_impact_and_collisions() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let impact = harness.preview_rename_node_impact("Root.A", "AA");
    assert!(impact.legal, "{impact:?}");
    assert!(impact.blocked_reason.is_none());
    assert!(impact.bind_diagnostics.is_empty());
    assert!(impact.profile_violations.is_empty());
    assert!(impact.collisions.is_empty());
    assert!(impact.requires_rebind.contains(&NodeId::new("Root.A")));
    assert_eq!(
        impact.affected_refs,
        vec![NodeId::new("Root.B"), NodeId::new("Root.C")]
    );
    let MutationImpactIntentProjection::RenameNode { node, new_symbol } = &impact.intent else {
        panic!("expected rename impact intent");
    };
    assert_eq!(node, &NodeId::new("Root.A"));
    assert_eq!(new_symbol, "AA");
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    assert!(skin.state().node(&NodeId::new("Root.A")).is_some());
    skin.assert_scalar("Root.C", "3");

    let collision = harness.preview_rename_node_impact("Root.B", "A");
    assert!(!collision.legal);
    assert_eq!(
        collision.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::NameCollision)
    );
    assert_eq!(collision.collisions.len(), 1);
    assert_eq!(collision.collisions[0].attempted, "Root.A");
    assert_eq!(collision.collisions[0].existing, NodeId::new("Root.A"));
    assert!(collision.requires_rebind.contains(&NodeId::new("Root.B")));
    assert_eq!(collision.affected_refs, vec![NodeId::new("Root.C")]);
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
}

#[test]
fn programmable_skin_previews_move_drop_legality_impact_and_collisions() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    skin.add_node(Some("Root"), "Group", "");
    skin.add_node(Some("Root"), "Existing", "8");
    skin.add_node(Some("Root.Group"), "Existing", "9");
    skin.add_node(Some("Root.Group"), "Child", "");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let impact = harness.preview_move_node_impact("Root.B", Some("Root.Group"), Some(0));
    assert!(impact.legal, "{impact:?}");
    assert!(impact.blocked_reason.is_none());
    assert!(impact.collisions.is_empty());
    assert!(impact.requires_rebind.contains(&NodeId::new("Root.B")));
    assert_eq!(impact.affected_refs, vec![NodeId::new("Root.C")]);
    let MutationImpactIntentProjection::MoveNode {
        node,
        new_parent,
        new_index,
    } = &impact.intent
    else {
        panic!("expected move impact intent");
    };
    assert_eq!(node, &NodeId::new("Root.B"));
    assert_eq!(new_parent, &Some(NodeId::new("Root.Group")));
    assert_eq!(new_index, &Some(0));
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    assert!(skin.state().node(&NodeId::new("Root.B")).is_some());
    assert!(skin.state().node(&NodeId::new("Root.Group.B")).is_none());

    let collision = harness.preview_move_node_impact("Root.Existing", Some("Root.Group"), None);
    assert!(!collision.legal);
    assert_eq!(
        collision.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::NameCollision)
    );
    assert_eq!(collision.collisions.len(), 1);
    assert_eq!(collision.collisions[0].attempted, "Root.Group.Existing");
    assert_eq!(
        collision.collisions[0].existing,
        NodeId::new("Root.Group.Existing")
    );

    let invalid = harness.preview_move_node_impact("Root.Group", Some("Root.Group.Child"), None);
    assert!(!invalid.legal);
    assert_eq!(
        invalid.blocked_reason,
        Some(MutationImpactBlockedReasonProjection::InvalidDrop)
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
}

#[test]
fn programmable_skin_previews_delete_orphan_impact_without_mutating() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    let before_revision = revision_fingerprint(&skin.state().revision);

    let impact = harness.preview_delete_node_impact("Root.A");
    assert!(impact.legal, "{impact:?}");
    assert!(impact.blocked_reason.is_none());
    assert!(impact.collisions.is_empty());
    assert_eq!(impact.orphaned_dependents, vec![NodeId::new("Root.B")]);
    assert_eq!(
        impact.affected_refs,
        vec![NodeId::new("Root.B"), NodeId::new("Root.C")]
    );
    assert!(impact.requires_rebind.contains(&NodeId::new("Root.A")));
    let MutationImpactIntentProjection::DeleteNode { node } = &impact.intent else {
        panic!("expected delete impact intent");
    };
    assert_eq!(node, &NodeId::new("Root.A"));
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    assert!(skin.state().node(&NodeId::new("Root.A")).is_some());
    skin.assert_scalar("Root.C", "3");
}

#[test]
fn programmable_skin_receipts_carry_projection_deltas() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    let add_root = skin.try_add_node(None, "Root", "");
    assert!(add_root.accepted, "{:?}", add_root.error);
    assert_eq!(add_root.delta.from_seq, 0);
    assert_eq!(add_root.delta.to_seq, 1);
    assert!(add_root.produced_revision.is_some());
    let add_root_transaction_id = add_root
        .transaction_id
        .as_deref()
        .expect("add-node receipts carry OxCalc transaction ids");
    assert!(
        add_root_transaction_id.starts_with("transaction:programmable-skin-ir:"),
        "{add_root_transaction_id}"
    );
    assert!(add_root
        .delta
        .changes
        .iter()
        .any(|change| matches!(change, WorkspaceDeltaChange::Structural(delta) if !delta.added.is_empty())));
    assert_eq!(skin.state().projection_seq, add_root.delta.to_seq);
    assert_eq!(skin.latest_delta(), add_root.delta);

    skin.add_node(Some("Root"), "A", "3");
    skin.add_node(Some("Root"), "B", "=A+1");
    let edit = skin.try_edit("Root.A", "4");
    assert!(edit.accepted, "{:?}", edit.error);
    let edit_transaction_id = edit
        .transaction_id
        .as_deref()
        .expect("node edit receipts carry OxCalc transaction ids");
    assert!(
        edit_transaction_id.starts_with("transaction:programmable-skin-ir:"),
        "{edit_transaction_id}"
    );
    assert_eq!(edit.delta.from_seq + 1, edit.delta.to_seq);
    assert_eq!(skin.state().projection_seq, edit.delta.to_seq);
    assert_eq!(skin.latest_delta(), edit.delta);
    assert!(edit.delta.changes.iter().any(
        |change| matches!(change, WorkspaceDeltaChange::ValuesChanged(values) if values
            .iter()
            .any(|value| value.value.scalar_display_text() == Some("4")))
    ));
    assert!(edit
        .delta
        .changes
        .iter()
        .any(|change| matches!(change, WorkspaceDeltaChange::CalcRun(run) if !run.invalidated_nodes.is_empty())));

    let before_select_seq = skin.state().projection_seq;
    let select = skin.try_select(Some("Root.B"));
    assert!(select.accepted, "{:?}", select.error);
    assert_eq!(select.transaction_id, None);
    assert_eq!(select.delta.from_seq, before_select_seq);
    assert_eq!(select.delta.to_seq, before_select_seq);
    assert!(select.delta.changes.is_empty());
    assert_eq!(skin.latest_delta(), select.delta);

    let reject_seq = skin.state().projection_seq;
    let rejected_select = skin.try_select_table_cell("MissingTable", Some("row:none"), "col:none");
    assert!(!rejected_select.accepted);
    assert_eq!(rejected_select.delta.from_seq, reject_seq);
    assert_eq!(rejected_select.delta.to_seq, reject_seq);
    assert!(rejected_select.delta.changes.is_empty());
    assert_eq!(skin.latest_delta(), rejected_select.delta);

    let new_workspace = skin.try_new_workspace();
    assert!(new_workspace.accepted, "{:?}", new_workspace.error);
    assert_eq!(new_workspace.transaction_id, None);
    assert_eq!(skin.latest_delta(), new_workspace.delta);
    assert!(
        new_workspace
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::FullReset))
    );
    assert_eq!(skin.state().workspace_id, "Workspace 1");

    let switch_back = skin.try_switch_workspace("programmable-skin-ir");
    assert!(switch_back.accepted, "{:?}", switch_back.error);
    assert_eq!(switch_back.transaction_id, None);
    assert_eq!(skin.latest_delta(), switch_back.delta);
    assert!(
        switch_back
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::FullReset))
    );
    assert_eq!(skin.state().workspace_id, "programmable-skin-ir");
}

#[test]
fn programmable_skin_selects_table_cells_without_recalculating() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before_seq = skin.state().projection_seq;
    let before_run = skin.state().last_run.clone();
    let body_select = skin.try_select_table_cell("SalesTable", Some("row:east"), "col:amount");
    assert!(body_select.accepted, "{:?}", body_select.error);
    assert_eq!(body_select.delta.from_seq, before_seq);
    assert_eq!(body_select.delta.to_seq, before_seq);
    assert_eq!(skin.selected(), Some("SalesTable".to_string()));
    assert_eq!(
        skin.selected_table_cell(),
        Some((
            "SalesTable".to_string(),
            Some("row:east".to_string()),
            "col:amount".to_string()
        ))
    );
    assert_eq!(skin.state().projection_seq, before_seq);
    assert_eq!(skin.state().last_run, before_run);
    let body_detail = skin
        .active_table_cell_detail()
        .expect("body table cell has active detail");
    assert_eq!(body_detail.table, NodeId::new("SalesTable"));
    assert_eq!(body_detail.table_id, "tree-table:sales");
    assert_eq!(body_detail.table_name, "SalesTable");
    assert_eq!(body_detail.row_id.as_deref(), Some("row:east"));
    assert_eq!(body_detail.row_ordinal, Some(2));
    assert_eq!(body_detail.column_id, "col:amount");
    assert_eq!(body_detail.column_name, "Amount");
    assert_eq!(body_detail.column_ordinal, 2);
    assert_eq!(body_detail.region, TableCellRegionProjection::Body);
    assert_eq!(
        body_detail.editability,
        TableCellEditabilityProjection::DirectInput
    );
    assert!(body_detail.formula.is_none());
    let body_state = skin.state();
    let body_table = body_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("SalesTable projects");
    let body_column_index = body_table
        .columns
        .iter()
        .position(|column| column.column_id == "col:amount")
        .expect("Amount column projects");
    let body_row_index = body_table
        .rows
        .iter()
        .position(|row| row.row_id == "row:east")
        .expect("East row projects");
    let body_cell = body_table
        .cells
        .as_ref()
        .expect("SalesTable cells project")
        .body_rows
        .get(body_row_index)
        .and_then(|row| row.get(body_column_index))
        .and_then(Option::as_ref)
        .expect("East Amount cell projects");
    assert_eq!(body_detail.node_key, body_cell.node_key);
    assert_eq!(body_detail.value.display_text(), "20");

    let formula_select = skin.try_select_table_cell("SalesTable", Some("row:east"), "col:tax");
    assert!(formula_select.accepted, "{:?}", formula_select.error);
    let formula_detail = skin
        .active_table_cell_detail()
        .expect("formula body table cell has active detail");
    assert_eq!(formula_detail.table, NodeId::new("SalesTable"));
    assert_eq!(formula_detail.row_id.as_deref(), Some("row:east"));
    assert_eq!(formula_detail.column_id, "col:tax");
    assert_eq!(formula_detail.column_name, "Tax");
    assert_eq!(formula_detail.column_ordinal, 3);
    assert_eq!(formula_detail.region, TableCellRegionProjection::Body);
    assert_eq!(
        formula_detail.editability,
        TableCellEditabilityProjection::FormulaBacked
    );
    let formula = formula_detail
        .formula
        .as_ref()
        .expect("Tax body cells carry column formula metadata");
    assert_eq!(
        formula.formula_artifact_id,
        "formula:SalesTable.Columns.Tax"
    );
    assert_eq!(
        formula.bind_artifact_id.as_deref(),
        Some("bind:SalesTable.Columns.Tax")
    );
    assert_eq!(formula.formula_text, "=[@Amount] * 0.1");
    assert_eq!(formula_detail.value.display_text(), "2");
    let formula_state = skin.state();
    let expected_formula_outgoing = formula_state
        .dependencies
        .reference_resolutions
        .values()
        .filter(|resolution| resolution.owner_key == formula_detail.node_key)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        formula_detail.outgoing_references,
        expected_formula_outgoing
    );
    assert_eq!(
        formula_detail.incoming_reference_handles,
        formula_state
            .dependencies
            .reverse_references
            .get(&formula_detail.node_key)
            .cloned()
            .unwrap_or_default()
    );

    let totals_select = skin.try_select_table_cell("SalesTable", None, "col:amount");
    assert!(totals_select.accepted, "{:?}", totals_select.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some(("SalesTable".to_string(), None, "col:amount".to_string()))
    );
    let totals_detail = skin
        .active_table_cell_detail()
        .expect("totals table cell has active detail");
    assert_eq!(totals_detail.table, NodeId::new("SalesTable"));
    assert_eq!(totals_detail.table_id, "tree-table:sales");
    assert_eq!(totals_detail.table_name, "SalesTable");
    assert_eq!(totals_detail.row_id, None);
    assert_eq!(totals_detail.row_ordinal, None);
    assert_eq!(totals_detail.column_id, "col:amount");
    assert_eq!(totals_detail.column_name, "Amount");
    assert_eq!(totals_detail.column_ordinal, 2);
    assert_eq!(totals_detail.region, TableCellRegionProjection::Totals);
    assert_eq!(
        totals_detail.editability,
        TableCellEditabilityProjection::TotalsFormula
    );
    let totals_formula = totals_detail
        .formula
        .as_ref()
        .expect("Amount totals cell carries totals formula metadata");
    assert_eq!(
        totals_formula.formula_artifact_id,
        "formula:SalesTable.Totals.Amount"
    );
    assert_eq!(totals_formula.formula_text, "=SUM(SalesTable[Amount])");
    let totals_state = skin.state();
    let totals_table = totals_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("SalesTable projects");
    let totals_column_index = totals_table
        .columns
        .iter()
        .position(|column| column.column_id == "col:amount")
        .expect("Amount column projects");
    let totals_cell = totals_table
        .cells
        .as_ref()
        .expect("SalesTable cells project")
        .totals_row
        .get(totals_column_index)
        .and_then(Option::as_ref)
        .expect("Amount totals cell projects");
    assert_eq!(totals_detail.node_key, totals_cell.node_key);
    assert_eq!(totals_detail.value.display_text(), "60");
    let expected_totals_outgoing = totals_state
        .dependencies
        .reference_resolutions
        .values()
        .filter(|resolution| resolution.owner_key == totals_detail.node_key)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(totals_detail.outgoing_references, expected_totals_outgoing);
    assert_eq!(
        totals_detail.incoming_reference_handles,
        totals_state
            .dependencies
            .reverse_references
            .get(&totals_detail.node_key)
            .cloned()
            .unwrap_or_default()
    );

    let missing_row = skin.try_select_table_cell("SalesTable", Some("row:missing"), "col:amount");
    assert!(!missing_row.accepted, "{missing_row:?}");
    assert!(matches!(
        missing_row.error,
        Some(IntentError::UnknownTableCell {
            ref table,
            ref row_id,
            ref column_id
        }) if table == "SalesTable" && row_id == "row:missing" && column_id == "col:amount"
    ));
    let missing_column = skin.try_select_table_cell("SalesTable", Some("row:east"), "col:missing");
    assert!(!missing_column.accepted, "{missing_column:?}");
    assert!(matches!(
        missing_column.error,
        Some(IntentError::UnknownTableCell {
            ref table,
            ref row_id,
            ref column_id
        }) if table == "SalesTable" && row_id == "row:east" && column_id == "col:missing"
    ));
    let missing_table = skin.try_select_table_cell("MissingTable", Some("row:east"), "col:amount");
    assert!(!missing_table.accepted, "{missing_table:?}");
    assert!(matches!(
        missing_table.error,
        Some(IntentError::UnknownTableCell {
            ref table,
            ref row_id,
            ref column_id
        }) if table == "MissingTable" && row_id == "row:east" && column_id == "col:amount"
    ));

    skin.select(Some("SalesTable"));
    assert_eq!(skin.selected(), Some("SalesTable".to_string()));
    assert_eq!(skin.selected_table_cell(), None);
    assert!(skin.active_table_cell_detail().is_none());
}

#[test]
fn programmable_skin_moves_table_cell_focus_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let no_focus_move = skin.try_move_table_cell_focus(0, 1);
    assert!(!no_focus_move.accepted, "{no_focus_move:?}");

    skin.select_table_cell("SalesTable", Some("row:west"), "col:region");
    let right = skin.try_move_table_cell_focus(0, 1);
    assert!(right.accepted, "{:?}", right.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some((
            "SalesTable".to_string(),
            Some("row:west".to_string()),
            "col:amount".to_string()
        ))
    );

    let down = skin.try_move_table_cell_focus(1, 0);
    assert!(down.accepted, "{:?}", down.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some((
            "SalesTable".to_string(),
            Some("row:east".to_string()),
            "col:amount".to_string()
        ))
    );

    skin.select_table_cell("SalesTable", Some("row:north"), "col:amount");
    let to_totals = skin.try_move_table_cell_focus(1, 0);
    assert!(to_totals.accepted, "{:?}", to_totals.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some(("SalesTable".to_string(), None, "col:amount".to_string()))
    );

    let from_totals = skin.try_move_table_cell_focus(-1, 1);
    assert!(from_totals.accepted, "{:?}", from_totals.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some((
            "SalesTable".to_string(),
            Some("row:north".to_string()),
            "col:tax".to_string()
        ))
    );

    let clamp_right = skin.try_move_table_cell_focus(0, 1);
    assert!(clamp_right.accepted, "{:?}", clamp_right.error);
    assert_eq!(
        skin.selected_table_cell(),
        Some((
            "SalesTable".to_string(),
            Some("row:north".to_string()),
            "col:tax".to_string()
        ))
    );
}

#[test]
fn programmable_skin_builds_interrelated_tree_and_reads_results() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "3");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B*2");
    skin.recalc();

    skin.assert_scalar("Root.A", "3");
    skin.assert_scalar("Root.B", "4");
    skin.assert_scalar("Root.C", "8");
    assert_eq!(skin.outgoing_count("Root.C"), 1);
    assert_eq!(skin.incoming_count("Root.B"), 1);

    skin.edit("Root.A", "4");
    skin.assert_scalar("Root.B", "5");
    skin.assert_scalar("Root.C", "10");

    let state = skin.state();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let c_key = state.node(&NodeId::new("Root.C")).unwrap().key.clone();
    assert_eq!(state.node_id_for_key(&c_key), Some(&NodeId::new("Root.C")));
    assert_eq!(
        state.nodes_by_key.get(&c_key).map(|node| &node.id),
        Some(&NodeId::new("Root.C"))
    );
    assert_eq!(
        state
            .dependencies
            .edges_by_owner_key
            .get(&c_key)
            .map(Vec::len),
        Some(1)
    );
    assert!(
        state.dependencies.edges_by_owner_key[&c_key]
            .iter()
            .any(|edge| edge.owner_key == c_key && edge.target_key == b_key)
    );
    assert_eq!(
        state
            .dependencies
            .reverse_edges_by_key
            .get(&b_key)
            .map(Vec::len),
        Some(1)
    );
    assert!(
        state
            .dependencies
            .descriptors_by_owner_key
            .get(&c_key)
            .is_some_and(|descriptors| {
                descriptors
                    .iter()
                    .any(|descriptor| descriptor.target_key == Some(b_key.clone()))
            })
    );
    assert!(state.dependencies.cycle_group_keys.is_empty());
    let run = state.last_run.as_ref().expect("recalc projects last run");
    assert!(matches!(
        run.run_state,
        CalcRunStateProjection::Published | CalcRunStateProjection::VerifiedClean
    ));
    assert!(!run.evaluation_order.is_empty());
}

#[test]
fn programmable_skin_projects_full_derivation_trace_payload() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "3");
    skin.add_node(Some("Root"), "B", "=A+4");

    let state = skin.state();
    let run = state.last_run.as_ref().expect("recalc projects last run");
    assert_eq!(run.derivation_trace_count, run.derivation_traces.len());
    let trace = run
        .derivation_traces
        .iter()
        .find(|trace| trace.owner.as_str() == "Root.B")
        .expect("formula node projects derivation trace");
    assert_eq!(
        trace.owner_key,
        state.node(&NodeId::new("Root.B")).unwrap().key
    );
    assert_eq!(
        trace.trace_schema_id,
        "oxcalc.derivation_trace.invoke_outcome.v1"
    );
    assert_eq!(trace.trace_mode, "PreparedCalls");
    assert_eq!(trace.kernel_returned_value, "7");
    assert!(matches!(
        &trace.kernel_returned_value_typed,
        Some(NodeValueProjection::Number { display, .. }) if display == "7"
    ));
    assert!(!trace.formula_stable_id.is_empty());
    assert!(!trace.template_selection.plan_template_key.is_empty());
    assert!(!trace.hole_bindings.is_empty());
    assert!(!trace.sub_invocation_tree.is_empty());
    let root_call = &trace.sub_invocation_tree[0];
    assert!(!root_call.function_id.is_empty());
    assert!(!root_call.function_name.is_empty());
    assert_eq!(root_call.kernel_returned_value.as_deref(), Some("7"));
    assert!(matches!(
        &root_call.kernel_returned_value_typed,
        Some(NodeValueProjection::Number { display, .. }) if display == "7"
    ));
    assert!(root_call.children.iter().any(|child| {
        matches!(
            &child.kernel_returned_value_typed,
            Some(NodeValueProjection::Number { display, .. }) if display == "7"
        )
    }));
    assert!(root_call.children.iter().any(|child| {
        child.prepared_arguments.iter().any(|argument| {
            matches!(
                &argument.resolved_value_typed,
                Some(NodeValueProjection::Number { display, .. }) if display == "3"
            )
        })
    }));
}

#[test]
fn programmable_skin_projects_runtime_effects_and_overlays() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "2");
    skin.add_node(Some("Root"), "B1", "4");
    skin.add_node(Some("Root"), "B2", "5");
    skin.add_node(Some("Root"), "B3", "6");
    skin.add_node(Some("Root"), "C", "=INDIRECT(\"B\"&A)");

    skin.assert_scalar("Root.C", "5");
    let state = skin.state();
    let run = state.last_run.as_ref().expect("recalc projects last run");
    assert_eq!(run.runtime_effect_count, run.runtime_effects.len());
    assert_eq!(run.runtime_overlay_count, run.runtime_overlays.len());
    let effect = run
        .runtime_effects
        .iter()
        .find(|effect| effect.family == RuntimeEffectFamilyProjection::DynamicDependency)
        .expect("dynamic reference runtime effect projects");
    assert_eq!(effect.kind, "runtime_effect.dynamic_reference");
    assert!(effect.detail.contains("owner_node:"));
    assert!(effect.detail.contains("target_node:"));

    let overlay = run
        .runtime_overlays
        .iter()
        .find(|overlay| overlay.kind == RuntimeOverlayKindProjection::DynamicDependency)
        .expect("dynamic reference runtime overlay projects");
    assert_eq!(overlay.owner.as_str(), "Root.C");
    assert_eq!(
        overlay.owner_key,
        state.node(&NodeId::new("Root.C")).unwrap().key
    );
    assert!(
        overlay
            .payload_identity
            .as_deref()
            .is_some_and(|payload| { payload.contains("runtime_effect") })
    );
    assert!(overlay.detail.contains("runtime_effect.dynamic_reference"));
}

#[test]
fn programmable_skin_projects_array_values_from_oxcalc_calc_values() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "ArrayNode", "=SEQUENCE(3)");

    let state = skin.state();
    let array_node = state
        .node(&NodeId::new("Root.ArrayNode"))
        .expect("array node projects");
    let NodeValueProjection::Array { rows, cols, cells } = &array_node.computed_value else {
        panic!(
            "SEQUENCE should project as an array, got {:?}",
            array_node.computed_value
        );
    };
    assert_eq!((*rows, *cols), (3, 1));
    assert_eq!(cells[0][0].display_text(), "1");
    assert_eq!(cells[1][0].display_text(), "2");
    assert_eq!(cells[2][0].display_text(), "3");

    skin.recalc();
    let state = skin.state();
    let array_node = state
        .node(&NodeId::new("Root.ArrayNode"))
        .expect("array node still projects after explicit recalc");
    assert!(
        matches!(array_node.computed_value, NodeValueProjection::Array { .. }),
        "explicit recalc should retain the typed array CalcValue, got {:?}",
        array_node.computed_value
    );
    assert_eq!(
        array_node.literalized_value_input.as_deref(),
        Some("={1;2;3}")
    );
}

#[test]
fn programmable_skin_projects_scalar_values_as_typed_variants() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Number", "42");
    skin.add_node(Some("Root"), "Text", "hello");
    skin.add_node(Some("Root"), "Logical", "true");

    let state = skin.state();
    assert!(matches!(
        &state.node(&NodeId::new("Root.Number")).unwrap().computed_value,
        NodeValueProjection::Number { raw, display } if raw == "42" && display == "42"
    ));
    assert!(matches!(
        &state.node(&NodeId::new("Root.Text")).unwrap().computed_value,
        NodeValueProjection::Text(text) if text == "hello"
    ));
    assert!(matches!(
        &state.node(&NodeId::new("Root.Logical")).unwrap().computed_value,
        NodeValueProjection::Logical { value: true, display } if display == "true"
    ));
}

#[test]
fn programmable_skin_projects_sequence_5_by_5() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Grid", "=SEQUENCE(5,5)");

    let state = skin.state();
    let grid = state
        .node(&NodeId::new("Root.Grid"))
        .expect("SEQUENCE node projects");
    let NodeValueProjection::Array { rows, cols, cells } = &grid.computed_value else {
        panic!(
            "SEQUENCE should project as an array, got {:?}",
            grid.computed_value
        );
    };
    assert_eq!((*rows, *cols), (5, 5));
    assert!(cells.iter().all(|row| row.len() == 5));
    assert_eq!(cells[0][0].display_text(), "1");
    assert_eq!(cells[4][4].display_text(), "25");
}

#[test]
fn programmable_skin_maps_inline_sequence_with_local_lambda() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Mapped", "=MAP(SEQUENCE(3),LAMBDA(v,v+1))");

    let state = skin.state();
    let mapped = state
        .node(&NodeId::new("Root.Mapped"))
        .expect("MAP node projects");
    let NodeValueProjection::Array { rows, cols, cells } = &mapped.computed_value else {
        panic!(
            "MAP over inline SEQUENCE should project as an array, got {:?}",
            mapped.computed_value
        );
    };
    assert_eq!((*rows, *cols), (3, 1));
    assert_eq!(cells[0][0].display_text(), "2");
    assert_eq!(cells[1][0].display_text(), "3");
    assert_eq!(cells[2][0].display_text(), "4");
}

#[test]
#[ignore = "upstream OxCalc/OxFml bridge gap: TreeCalc node-array MAP lambda currently returns a 1x1 Value error"]
fn programmable_skin_maps_node_array_with_lambda_host_capture() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "x", "1");
    skin.add_node(Some("Root"), "a", "=SEQUENCE(5,5)");
    skin.add_node(Some("Root"), "m", "=MAP(a,LAMBDA(v,v+x))");

    let state = skin.state();
    let mapped = state
        .node(&NodeId::new("Root.m"))
        .expect("mapped node projects");
    let NodeValueProjection::Array { rows, cols, cells } = &mapped.computed_value else {
        panic!(
            "MAP over TreeCalc node array should project as an array, got {:?}",
            mapped.computed_value
        );
    };
    assert_eq!(
        (*rows, *cols),
        (5, 5),
        "expected a 5x5 mapped array, got {:?}",
        mapped.computed_value
    );
    assert_eq!(cells[0][0].display_text(), "2");
    assert_eq!(cells[4][4].display_text(), "26");
}

#[test]
fn programmable_skin_projects_randarray_with_oxcalc_host_random_provider() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "Random", "=RANDARRAY(5,5)");

    let state = skin.state();
    let random = state
        .node(&NodeId::new("Root.Random"))
        .expect("RANDARRAY node projects");
    let NodeValueProjection::Array { rows, cols, cells } = &random.computed_value else {
        panic!(
            "RANDARRAY should project as an array, got {:?}",
            random.computed_value
        );
    };
    assert_eq!((*rows, *cols), (5, 5));
    assert!(cells.iter().all(|row| row.len() == 5));
}

#[test]
fn programmable_skin_manual_recalc_mode_defers_content_recalculation() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.assert_scalar("Root.B", "2");

    let before_recalc_count = harness.recalc_count();
    skin.set_recalc_mode(WorkspaceRecalcMode::Manual);
    skin.edit_deferred("Root.A", "5");
    skin.set_manual_recalc_pending(true);

    let deferred_state = skin.state();
    assert_eq!(
        deferred_state
            .node(&NodeId::new("Root.A"))
            .map(|node| node.content_text.as_str()),
        Some("5")
    );
    assert_eq!(skin.scalar("Root.B").as_deref(), Some("2"));
    assert_eq!(harness.recalc_count(), before_recalc_count);
    assert_eq!(skin.recalc_mode(), WorkspaceRecalcMode::Manual);
    assert!(skin.manual_recalc_pending());

    skin.recalc();
    skin.set_manual_recalc_pending(false);
    skin.assert_scalar("Root.B", "6");
    assert!(!skin.manual_recalc_pending());
}

#[test]
fn programmable_skin_exercises_structural_edits_from_outside_ir() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "X", "1");
    skin.add_node(Some("Root"), "Y", "2");
    skin.add_node(Some("Root"), "Z", "3");
    skin.assert_children("Root", &["Root.X", "Root.Y", "Root.Z"]);

    let reorder = skin.try_reorder("Root.Z", 0);
    assert!(reorder.accepted, "{:?}", reorder.error);
    assert!(
        reorder
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:programmable-skin-ir:")),
        "{reorder:?}"
    );
    skin.assert_children("Root", &["Root.Z", "Root.X", "Root.Y"]);

    let rename = skin.try_rename("Root.X", "Renamed");
    assert!(rename.accepted, "{:?}", rename.error);
    assert!(
        rename
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:programmable-skin-ir:")),
        "{rename:?}"
    );
    assert!(skin.state().node(&NodeId::new("Root.Renamed")).is_some());
    skin.assert_children("Root", &["Root.Z", "Root.Renamed", "Root.Y"]);

    let moved = skin.try_move_node("Root.Y", None, None);
    assert!(moved.accepted, "{:?}", moved.error);
    assert!(
        moved
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:programmable-skin-ir:")),
        "{moved:?}"
    );
    let state = skin.state();
    assert!(state.node(&NodeId::new("Y")).is_some());
    assert!(state.root_paths.iter().any(|node| node.as_str() == "Y"));

    let delete = skin.try_delete("Root.Z");
    assert!(delete.accepted, "{:?}", delete.error);
    assert!(
        delete
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:programmable-skin-ir:")),
        "{delete:?}"
    );
    assert!(skin.state().node(&NodeId::new("Root.Z")).is_none());
}

#[test]
fn programmable_skin_management_intents_keep_selection_on_surviving_nodes() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    assert_eq!(skin.selected().as_deref(), Some("Root"));

    skin.add_node(Some("Root"), "A", "1");
    assert_eq!(skin.selected().as_deref(), Some("Root.A"));

    skin.rename("Root.A", "Renamed");
    assert_eq!(skin.selected().as_deref(), Some("Root.Renamed"));

    skin.add_node(Some("Root"), "B", "2");
    skin.reorder("Root.B", 0);
    assert_eq!(skin.selected().as_deref(), Some("Root.B"));
    skin.assert_children("Root", &["Root.B", "Root.Renamed"]);

    skin.move_node("Root.Renamed", None, None);
    assert_eq!(skin.selected().as_deref(), Some("Renamed"));

    skin.delete("Root.B");
    assert_eq!(skin.selected().as_deref(), Some("Root"));

    skin.delete("Renamed");
    assert_eq!(skin.selected(), None);
}

#[test]
fn programmable_skin_structural_edits_recalculate_dependent_formulas() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");
    skin.add_node(Some("Root"), "C", "=B+1");
    skin.assert_scalar("Root.C", "3");

    skin.edit("Root.A", "5");
    skin.assert_scalar("Root.B", "6");
    skin.assert_scalar("Root.C", "7");

    skin.add_node(Some("Root"), "D", "=C+1");
    skin.assert_scalar("Root.D", "8");

    skin.delete("Root.D");
    assert!(skin.state().node(&NodeId::new("Root.D")).is_none());
    skin.assert_scalar("Root.C", "7");
}

#[test]
fn programmable_skin_projects_reference_resolution_map_and_reverse_index() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "=A+1");

    let state = skin.state();
    let direct = state
        .dependencies
        .reference_resolutions
        .values()
        .find(|resolution| resolution.owner.as_str() == "Root.B")
        .expect("direct reference resolution projects");
    let ReferenceTargetProjection::Node { node, key } = &direct.target else {
        panic!(
            "direct reference should target a node, got {:?}",
            direct.target
        );
    };
    assert_eq!(node.as_str(), "Root.A");
    assert_eq!(key, &state.node(&NodeId::new("Root.A")).unwrap().key);
    assert!(direct.descriptor_ids.iter().any(|id| !id.is_empty()));
    assert!(
        state
            .dependencies
            .reverse_references
            .get(key)
            .is_some_and(|handles| handles.contains(&direct.source_reference_handle))
    );

    let collection_harness = Harness::from_repo_fixture("children-raw-active");
    let collection_skin = collection_harness.driver.clone();
    collection_skin.recalc();

    let collection_state = collection_skin.state();
    let collection_resolution = collection_state
        .dependencies
        .reference_resolutions
        .values()
        .find(|resolution| resolution.owner.as_str() == "Root.DirectChildren")
        .expect("collection reference resolution projects");
    let ReferenceTargetProjection::Collection {
        collection,
        member_keys,
    } = &collection_resolution.target
    else {
        panic!(
            "children reference should target a collection, got {:?}",
            collection_resolution.target
        );
    };
    assert_eq!(
        collection.family,
        TreeReferenceCollectionFamilyProjection::Children
    );
    assert_eq!(
        collection.members,
        vec![
            NodeId::new("Root.DirectChildren.A"),
            NodeId::new("Root.DirectChildren.B")
        ]
    );
    assert_eq!(
        member_keys,
        &collection
            .members
            .iter()
            .map(|member| collection_state.node(member).unwrap().key.clone())
            .collect::<Vec<_>>()
    );
    for member_key in member_keys {
        assert!(
            collection_state
                .dependencies
                .reverse_references
                .get(member_key)
                .is_some_and(|handles| {
                    handles.contains(&collection_resolution.source_reference_handle)
                })
        );
    }
}

#[test]
fn programmable_skin_expands_authoring_scope_subjects_from_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    skin.add_node(Some("Root"), "B", "");
    skin.add_node(Some("Root.B"), "B1", "2");
    skin.add_node(Some("Root.B"), "B2", "3");
    skin.add_node(Some("Root"), "C", "4");
    skin.add_node(Some("Root"), "D", "=A");

    let state = skin.state();
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();
    let b1_key = state.node(&NodeId::new("Root.B.B1")).unwrap().key.clone();
    let b2_key = state.node(&NodeId::new("Root.B.B2")).unwrap().key.clone();

    assert_eq!(
        state.expand_authoring_scope(&AuthoringScope::Node(a_key.clone())),
        Ok(vec![a_key.clone()])
    );
    assert_eq!(
        state.expand_authoring_scope(&AuthoringScope::Nodes(vec![
            b_key.clone(),
            a_key.clone(),
            b_key.clone()
        ])),
        Ok(vec![b_key.clone(), a_key.clone()])
    );
    assert_eq!(
        state.expand_authoring_scope(&AuthoringScope::Subtree(b_key.clone())),
        Ok(vec![b_key, b1_key, b2_key])
    );

    let collection_harness = Harness::from_repo_fixture("children-raw-active");
    let collection_skin = collection_harness.driver.clone();
    collection_skin.recalc();

    let collection_state = collection_skin.state();
    let collection_resolution = collection_state
        .dependencies
        .reference_resolutions
        .values()
        .find(|resolution| resolution.owner.as_str() == "Root.DirectChildren")
        .expect("collection reference resolution projects");
    let expected_members = match &collection_resolution.target {
        ReferenceTargetProjection::Collection { member_keys, .. } => member_keys.clone(),
        other => panic!("expected collection target, got {other:?}"),
    };
    assert_eq!(
        collection_state.expand_authoring_scope(&AuthoringScope::Collection {
            owner: collection_resolution.owner_key.clone(),
            source_reference_handle: collection_resolution.source_reference_handle.clone(),
        }),
        Ok(expected_members)
    );

    let direct_resolution = state
        .dependencies
        .reference_resolutions
        .values()
        .find(|resolution| {
            resolution.owner.as_str() == "Root.D"
                && matches!(resolution.target, ReferenceTargetProjection::Node { .. })
        })
        .expect("direct node reference projects");
    assert!(
        state
            .expand_authoring_scope(&AuthoringScope::Collection {
                owner: direct_resolution.owner_key.clone(),
                source_reference_handle: direct_resolution.source_reference_handle.clone(),
            })
            .is_err()
    );
}

#[test]
fn programmable_skin_reads_active_node_detail_from_workspace_and_selection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "3");
    skin.add_node(Some("Root"), "B", "=A+1");

    skin.select(None);
    assert!(skin.active_detail().is_none());
    skin.select(Some("Root.B"));
    let detail = skin
        .active_detail()
        .expect("selected projected node has active detail");
    let state = skin.state();
    let selected = state.node(&NodeId::new("Root.B")).unwrap();
    let target = state.node(&NodeId::new("Root.A")).unwrap();
    assert_eq!(detail.node.as_str(), "Root.B");
    assert_eq!(detail.node_key, selected.key);
    assert_eq!(detail.display_name, "B");
    assert_eq!(detail.content_kind, NodeContentKind::Formula);
    assert_eq!(detail.content_text, "=A+1");
    assert_eq!(detail.value.scalar_display_text(), Some("4"));
    let outgoing = detail
        .outgoing_references
        .iter()
        .find(|reference| {
            matches!(
                reference.target,
                ReferenceTargetProjection::Node { ref key, .. } if key == &target.key
            )
        })
        .expect("active formula detail includes outgoing reference");
    let outgoing_handle = outgoing.source_reference_handle.clone();

    skin.select(Some("Root.A"));
    let detail = skin.active_detail().expect("A has active detail");
    assert!(detail.incoming_reference_handles.contains(&outgoing_handle));
}

#[test]
fn programmable_skin_reads_unified_active_selection_detail() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "3");

    skin.select(None);
    assert!(skin.active_selection_detail().is_none());

    skin.select(Some("Root.A"));
    let active_node_selection = skin
        .active_selection_detail()
        .expect("node selection projects active detail");
    assert_eq!(active_node_selection.stable_id(), "node");
    let ActiveSelectionDetailProjection::Node(node_detail) = active_node_selection else {
        panic!("node selection should project active node detail");
    };
    assert_eq!(node_detail.node, NodeId::new("Root.A"));
    assert_eq!(node_detail.value.display_text(), "3");

    let table_harness = Harness::from_repo_fixture("tables");
    let table_skin = table_harness.driver.clone();
    table_skin.recalc();
    table_skin.select_table_cell("SalesTable", Some("row:east"), "col:tax");
    let active_cell_selection = table_skin
        .active_selection_detail()
        .expect("table cell selection projects active detail");
    assert_eq!(active_cell_selection.stable_id(), "table_cell");
    let ActiveSelectionDetailProjection::TableCell(cell_detail) = active_cell_selection else {
        panic!("table cell selection should project active table cell detail");
    };
    assert_eq!(cell_detail.table, NodeId::new("SalesTable"));
    assert_eq!(cell_detail.row_id.as_deref(), Some("row:east"));
    assert_eq!(cell_detail.column_id, "col:tax");
    assert_eq!(cell_detail.region.stable_id(), "body");
    assert_eq!(cell_detail.editability.stable_id(), "formula_backed");
    assert_eq!(cell_detail.value.display_text(), "2");
    assert_eq!(
        cell_detail
            .formula
            .as_ref()
            .map(|formula| formula.formula_text.as_str()),
        Some("=[@Amount] * 0.1")
    );
}

#[test]
fn programmable_skin_rejects_invalid_structural_intents_without_state_drift() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "1");
    let before_state = skin.state();
    let before_revision = revision_fingerprint(&before_state.revision);
    let before_recalc_count = harness.recalc_count();

    let duplicate = skin.try_add_node(Some("Root"), "A", "2");
    assert!(!duplicate.accepted, "{duplicate:?}");
    assert!(matches!(
        duplicate.error,
        Some(IntentError::DuplicateNode { ref node }) if node == "Root.A"
    ));
    let unknown_edit = skin.try_edit("Root.Missing", "3");
    assert!(!unknown_edit.accepted, "{unknown_edit:?}");
    assert!(matches!(
        unknown_edit.error,
        Some(IntentError::UnknownNode { ref node }) if node == "Root.Missing"
    ));
    let unknown_delete = skin.try_delete("Root.Missing");
    assert!(!unknown_delete.accepted, "{unknown_delete:?}");
    assert!(matches!(
        unknown_delete.error,
        Some(IntentError::UnknownNode { ref node }) if node == "Root.Missing"
    ));

    let after_state = skin.state();
    assert_eq!(revision_fingerprint(&after_state.revision), before_revision);
    assert_eq!(after_state.node_order, before_state.node_order);
    assert_eq!(harness.recalc_count(), before_recalc_count);
    skin.assert_scalar("Root.A", "1");
}

#[test]
fn programmable_skin_facade_state_does_not_recalculate() {
    let harness = Harness::from_repo_fixture("accounts");
    let skin = harness.driver.clone();
    let before_count = harness.recalc_count();
    let before_revision = revision_fingerprint(&skin.state().revision);

    skin.select(Some("Accounts.2005.Q1.Net"));
    skin.collapse("Accounts.2005.Q1");
    skin.pin("Accounts.2005.Q1.Net");

    assert_eq!(skin.selected().as_deref(), Some("Accounts.2005.Q1.Net"));
    assert_eq!(harness.recalc_count(), before_count);
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
}

#[test]
fn programmable_skin_reads_table_and_dependency_ir_from_fixture() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let state = skin.state();
    let table = state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects through skin IR");
    assert_eq!(table.table_id, "tree-table:sales");
    assert_eq!(table.row_count, 3);
    assert_eq!(table.column_count, 3);
    assert_eq!(table.virtual_anchor.workbook_scope_ref, "tables");
    assert_eq!(table.virtual_anchor.sheet_scope_ref, "SalesTable");
    assert_eq!(table.virtual_anchor.start_row, 1);
    assert_eq!(table.virtual_anchor.start_col, 1);
    assert!(
        !state
            .node_order
            .iter()
            .any(|node| node.as_str().contains("__table_body_"))
    );
    assert!(table.header_row_present);
    assert_eq!(
        table
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>(),
        vec!["row:west", "row:east", "row:north"]
    );
    assert_eq!(
        table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax"]
    );
    assert!(matches!(
        table.columns[0].body,
        TableColumnBodyProjection::ConstantCells
    ));
    let TableColumnBodyProjection::Formula(tax_formula) = &table.columns[2].body else {
        panic!("Tax column should project formula metadata");
    };
    assert_eq!(
        tax_formula.formula_artifact_id,
        "formula:SalesTable.Columns.Tax"
    );
    assert_eq!(
        tax_formula.bind_artifact_id.as_deref(),
        Some("bind:SalesTable.Columns.Tax")
    );
    assert_eq!(tax_formula.formula_text, "=[@Amount] * 0.1");
    assert_eq!(
        table.columns[1]
            .totals_formula
            .as_ref()
            .map(|formula| formula.formula_artifact_id.as_str()),
        Some("formula:SalesTable.Totals.Amount")
    );
    assert_eq!(
        table.columns[1]
            .totals_formula
            .as_ref()
            .map(|formula| formula.formula_text.as_str()),
        Some("=SUM(SalesTable[Amount])")
    );
    let cells = table.cells.as_ref().expect("table cell values project");
    assert_eq!(cells.body_rows.len(), 3);
    assert_eq!(
        cells.body_rows[0]
            .iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|cell| cell.value.display_text())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
        vec!["West", "10", "1"]
    );
    assert_eq!(
        cells.body_rows[1]
            .iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|cell| cell.value.display_text())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
        vec!["East", "20", "2"]
    );
    assert_eq!(
        cells.body_rows[2]
            .iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|cell| cell.value.display_text())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
        vec!["North", "30", "3"]
    );
    assert_eq!(
        cells
            .totals_row
            .iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|cell| cell.value.display_text())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>(),
        vec!["", "60", ""]
    );
    assert!(!table.dependency_inventory.is_empty());
    assert!(table.dependency_inventory.iter().any(|fact| {
        fact.kind == TableDependencyFactKindProjection::TableIdentity
            && fact.status == TableDependencyFactStatusProjection::Lowered
            && fact.table_id.as_deref() == Some("tree-table:sales")
            && fact.identity.is_some()
    }));
    assert!(table.dependency_inventory.iter().any(|fact| {
        fact.kind == TableDependencyFactKindProjection::DataRegion
            && fact.status == TableDependencyFactStatusProjection::Lowered
    }));
}

#[test]
fn programmable_skin_projects_effective_format_and_oxfml_rendered_display() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let state = skin.state();
    let sales = state.node(&NodeId::new("Book.Sales")).unwrap();
    let sales_format = sales
        .effective_format
        .as_ref()
        .expect("Sales inherits book format");
    assert_eq!(sales_format.number_format_code.as_deref(), Some("0.00"));
    assert_eq!(
        sales_format
            .inherited_from
            .as_ref()
            .map(|source| &source.node),
        Some(&NodeId::new("Book.Format"))
    );
    assert_eq!(sales.computed_value.display_text(), "1000.00");

    let margin = state.node(&NodeId::new("Book.Margin")).unwrap();
    let margin_format = margin
        .effective_format
        .as_ref()
        .expect("Margin has own format override");
    assert_eq!(margin_format.number_format_code.as_deref(), Some("0%"));
    assert_eq!(
        margin_format
            .inherited_from
            .as_ref()
            .map(|source| &source.node),
        Some(&NodeId::new("Book.Margin.Format"))
    );
    assert_eq!(margin.computed_value.display_text(), "20%");

    let detail = state
        .active_node_detail(&dnatreecalc_skin_framework::SelectionState::with_primary(
            Some(NodeId::new("Book.Margin")),
        ))
        .expect("active detail projects selected formatted node");
    assert_eq!(detail.effective_format, margin.effective_format);
    assert_eq!(detail.value.display_text(), "20%");
}

#[test]
fn programmable_skin_authors_number_format_via_skin_ir_intent() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let state = skin.state();
    let margin_key = state
        .node(&NodeId::new("Book.Margin"))
        .expect("Margin projects")
        .key
        .clone();
    let receipt = skin.try_set_number_format(AuthoringScope::Node(margin_key), Some("0.00"));
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);

    let state = skin.state();
    let margin = state.node(&NodeId::new("Book.Margin")).unwrap();
    assert_eq!(
        margin
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0.00")
    );
    assert_eq!(
        margin
            .effective_format
            .as_ref()
            .and_then(|format| format.inherited_from.as_ref())
            .map(|source| &source.node),
        Some(&NodeId::new("Book.Margin.Format"))
    );
    assert_eq!(margin.computed_value.display_text(), "0.20");
}

#[test]
fn programmable_skin_clears_local_number_format_via_skin_ir_intent() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let margin_key = skin
        .state()
        .node(&NodeId::new("Book.Margin"))
        .expect("Margin projects")
        .key
        .clone();
    let receipt = skin.try_set_number_format(AuthoringScope::Node(margin_key), None);
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);

    let state = skin.state();
    let margin = state.node(&NodeId::new("Book.Margin")).unwrap();
    assert_eq!(
        margin
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0.00")
    );
    assert_eq!(
        margin
            .effective_format
            .as_ref()
            .and_then(|format| format.inherited_from.as_ref())
            .map(|source| &source.node),
        Some(&NodeId::new("Book.Format"))
    );
}

#[test]
fn programmable_skin_authors_number_format_over_ordered_scope() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let state = skin.state();
    let sales_key = state.node(&NodeId::new("Book.Sales")).unwrap().key.clone();
    let margin_key = state.node(&NodeId::new("Book.Margin")).unwrap().key.clone();
    let receipt = skin.try_set_number_format(
        AuthoringScope::Nodes(vec![sales_key, margin_key]),
        Some("0%"),
    );
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);

    let state = skin.state();
    assert_eq!(
        state
            .node(&NodeId::new("Book.Sales"))
            .unwrap()
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0%")
    );
    assert_eq!(
        state
            .node(&NodeId::new("Book.Margin"))
            .unwrap()
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0%")
    );
}

#[test]
fn programmable_skin_rejects_number_format_when_meta_path_is_user_node() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let add_parent = skin.try_add_node(Some("Book"), "Visible", "1");
    assert!(add_parent.accepted, "{:?}", add_parent.error);
    let add_reserved_path = skin.try_add_node(Some("Book.Visible"), "Format", "");
    assert!(add_reserved_path.accepted, "{:?}", add_reserved_path.error);

    let visible_key = skin
        .state()
        .node(&NodeId::new("Book.Visible"))
        .expect("created node projects")
        .key
        .clone();
    let receipt = skin.try_set_number_format(AuthoringScope::Node(visible_key), Some("0.00"));
    assert!(!receipt.accepted);
    assert!(matches!(
        receipt.error,
        Some(IntentError::FormatPathReserved { ref node }) if node == "Book.Visible.Format"
    ));
}

#[test]
fn programmable_skin_authors_note_via_skin_ir_intent() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let margin_key = skin
        .state()
        .node(&NodeId::new("Book.Margin"))
        .expect("Margin projects")
        .key
        .clone();
    let receipt = skin.try_set_note(margin_key, Some("Review margin assumption"));
    assert!(receipt.accepted, "{:?}", receipt.error);
    assert_any_transaction(&receipt);

    let state = skin.state();
    let margin = state.node(&NodeId::new("Book.Margin")).unwrap();
    let note = margin.note.as_ref().expect("note projects");
    assert_eq!(note.text, "Review margin assumption");
    assert_eq!(note.source.node, NodeId::new("Book.Margin.Note"));

    let detail = state
        .active_node_detail(&dnatreecalc_skin_framework::SelectionState::with_primary(
            Some(NodeId::new("Book.Margin")),
        ))
        .expect("active detail projects selected node");
    assert_eq!(detail.note, margin.note);
}

#[test]
fn programmable_skin_note_round_trips_through_workspace_document() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let margin_key = skin
        .state()
        .node(&NodeId::new("Book.Margin"))
        .unwrap()
        .key
        .clone();
    let receipt = skin.try_set_note(margin_key, Some("Persist this note"));
    assert!(receipt.accepted, "{:?}", receipt.error);

    let document = harness
        .session
        .lock()
        .unwrap()
        .export_dnatree_document(None)
        .expect("document export succeeds");
    let (imported, _) =
        TreeWorkspaceSession::from_dnatree_document(document).expect("document import succeeds");
    let state = imported.workspace_state().expect("imported state projects");
    assert_eq!(
        state
            .node(&NodeId::new("Book.Margin"))
            .and_then(|node| node.note.as_ref())
            .map(|note| note.text.as_str()),
        Some("Persist this note")
    );
}

#[test]
fn programmable_skin_clears_note_via_skin_ir_intent() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    let margin_key = skin
        .state()
        .node(&NodeId::new("Book.Margin"))
        .unwrap()
        .key
        .clone();
    assert!(
        skin.try_set_note(margin_key.clone(), Some("Temporary note"))
            .accepted
    );
    let clear = skin.try_set_note(margin_key, None);
    assert!(clear.accepted, "{:?}", clear.error);
    assert_any_transaction(&clear);
    assert!(
        skin.state()
            .node(&NodeId::new("Book.Margin"))
            .unwrap()
            .note
            .is_none()
    );
}

#[test]
fn programmable_skin_rejects_note_when_meta_path_is_user_node() {
    let harness = Harness::from_repo_fixture("formatting");
    let skin = harness.driver.clone();
    skin.recalc();

    assert!(
        skin.try_add_node(Some("Book"), "VisibleNoteOwner", "1")
            .accepted
    );
    assert!(
        skin.try_add_node(Some("Book.VisibleNoteOwner"), "Note", "")
            .accepted
    );

    let owner_key = skin
        .state()
        .node(&NodeId::new("Book.VisibleNoteOwner"))
        .expect("created note owner projects")
        .key
        .clone();
    let receipt = skin.try_set_note(owner_key, Some("Should reject"));
    assert!(!receipt.accepted);
    assert!(matches!(
        receipt.error,
        Some(IntentError::NotePathReserved { ref node }) if node == "Book.VisibleNoteOwner.Note"
    ));
}

#[test]
fn programmable_skin_sets_meta_via_oxcalc_transaction() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Root", "").accepted);
    assert!(skin.try_add_node(Some("Root"), "A", "1").accepted);
    assert!(skin.try_add_node(Some("Root"), "B", "=A+1").accepted);
    skin.recalc();
    assert_eq!(
        skin.state()
            .node(&NodeId::new("Root.B"))
            .unwrap()
            .computed_value
            .display_text(),
        "2"
    );

    let before_revision = revision_fingerprint(&skin.state().revision);
    let a_key = skin
        .state()
        .node(&NodeId::new("Root.A"))
        .expect("A projects before meta toggle")
        .key
        .clone();
    let hide = skin.try_set_meta(a_key.clone(), true);
    assert!(hide.accepted, "{:?}", hide.error);
    assert_any_transaction(&hide);
    let hidden_state = skin.state();
    assert_ne!(
        revision_fingerprint(&hidden_state.revision),
        before_revision
    );
    assert!(
        hidden_state
            .node(&NodeId::new("Root.A"))
            .expect("meta node remains addressable")
            .is_meta
    );
    assert!(matches!(
        hidden_state
            .node(&NodeId::new("Root.B"))
            .unwrap()
            .computed_value,
        NodeValueProjection::Error(_)
    ));

    let hidden_revision = revision_fingerprint(&hidden_state.revision);
    let show = skin.try_set_meta(a_key, false);
    assert!(show.accepted, "{:?}", show.error);
    assert_any_transaction(&show);
    let shown_state = skin.state();
    assert_ne!(revision_fingerprint(&shown_state.revision), hidden_revision);
    assert!(
        !shown_state
            .node(&NodeId::new("Root.A"))
            .expect("A remains addressable after unhide")
            .is_meta
    );
    assert_eq!(
        shown_state
            .node(&NodeId::new("Root.B"))
            .unwrap()
            .computed_value
            .display_text(),
        "2"
    );
}

#[test]
fn programmable_skin_sets_node_attributes_via_meta_nodes() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Root", "").accepted);
    assert!(skin.try_add_node(Some("Root"), "A", "1").accepted);
    skin.recalc();

    let a_key = skin
        .state()
        .node(&NodeId::new("Root.A"))
        .expect("A projects before attribute patch")
        .key
        .clone();
    let before_revision = revision_fingerprint(&skin.state().revision);

    let set = skin.try_set_node_attributes(a_key.clone(), NodeAttributePatch::set("owner", "qa"));
    assert!(set.accepted, "{:?}", set.error);
    assert_any_transaction(&set);
    let attributed = skin.state();
    assert_ne!(revision_fingerprint(&attributed.revision), before_revision);
    assert_eq!(
        attributed
            .node(&NodeId::new("Root.A"))
            .unwrap()
            .attributes
            .get("owner")
            .map(String::as_str),
        Some("qa")
    );
    assert!(attributed.node(&NodeId::new("Root.A.Attributes")).is_none());

    skin.select(Some("Root.A"));
    let detail = skin
        .active_detail()
        .expect("selected node detail projects attributes");
    assert_eq!(
        detail.attributes.get("owner").map(String::as_str),
        Some("qa")
    );

    let hidden_revision = revision_fingerprint(&skin.state().revision);
    let clear = skin.try_set_node_attributes(a_key.clone(), NodeAttributePatch::clear("owner"));
    assert!(clear.accepted, "{:?}", clear.error);
    assert_any_transaction(&clear);
    let cleared = skin.state();
    assert_ne!(revision_fingerprint(&cleared.revision), hidden_revision);
    assert!(
        !cleared
            .node(&NodeId::new("Root.A"))
            .unwrap()
            .attributes
            .contains_key("owner")
    );

    let invalid_revision = revision_fingerprint(&skin.state().revision);
    let invalid =
        skin.try_set_node_attributes(a_key.clone(), NodeAttributePatch::set("review.status", "x"));
    assert!(!invalid.accepted);
    assert_eq!(
        invalid.error,
        Some(IntentError::InvalidAttributeKey {
            key: "review.status".to_string()
        })
    );
    assert_eq!(
        revision_fingerprint(&skin.state().revision),
        invalid_revision
    );

    assert!(skin.try_add_node(Some("Root"), "B", "2").accepted);
    assert!(skin.try_add_node(Some("Root.B"), "Attributes", "").accepted);
    let b_key = skin
        .state()
        .node(&NodeId::new("Root.B"))
        .expect("B projects before reserved attribute patch")
        .key
        .clone();
    let reserved = skin.try_set_node_attributes(b_key, NodeAttributePatch::set("owner", "dev"));
    assert!(!reserved.accepted);
    assert_eq!(
        reserved.error,
        Some(IntentError::AttributePathReserved {
            node: "Root.B.Attributes".to_string()
        })
    );
}

#[test]
fn programmable_skin_populates_typed_clipboard_carriers_from_projection() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Root", "").accepted);
    assert!(skin.try_add_node(Some("Root"), "A", "2").accepted);
    assert!(skin.try_add_node(Some("Root"), "B", "=A+1").accepted);
    assert!(skin.try_add_node(Some("Root.B"), "B1", "7").accepted);
    skin.recalc();

    let state = skin.state();
    let root_key = state.node(&NodeId::new("Root")).unwrap().key.clone();
    let a_key = state.node(&NodeId::new("Root.A")).unwrap().key.clone();
    let b_key = state.node(&NodeId::new("Root.B")).unwrap().key.clone();

    let values = skin.try_copy_to_clipboard(
        AuthoringScope::Nodes(vec![a_key.clone(), b_key.clone()]),
        ClipboardPayloadKind::Values,
    );
    assert!(values.accepted, "{:?}", values.error);
    assert_eq!(values.transaction_id, None);
    assert!(
        values
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::ClipboardChanged(Some(_))))
    );
    let state = skin.state();
    let Some(clipboard) = &state.clipboard else {
        panic!("clipboard projects after value copy");
    };
    assert_eq!(clipboard.operation, ClipboardOperationProjection::Copy);
    let ClipboardPayloadProjection::Values { nodes } = &clipboard.payload else {
        panic!("expected values clipboard payload");
    };
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].node, a_key);
    assert_eq!(nodes[0].path, NodeId::new("Root.A"));
    assert_eq!(nodes[0].content_kind, NodeContentKind::Constant);
    assert_eq!(nodes[0].constant_input_text.as_deref(), Some("2"));
    assert_eq!(nodes[0].value.display_text(), "2");
    assert_eq!(nodes[1].node, b_key);
    assert_eq!(nodes[1].path, NodeId::new("Root.B"));
    assert_eq!(nodes[1].content_kind, NodeContentKind::Formula);
    assert_eq!(nodes[1].constant_input_text, None);
    assert_eq!(nodes[1].value.display_text(), "3");
    assert_eq!(clipboard.plain_text.as_deref(), Some("2\n3"));

    let formula = skin.try_copy_to_clipboard(
        AuthoringScope::Node(b_key.clone()),
        ClipboardPayloadKind::Formula,
    );
    assert!(formula.accepted, "{:?}", formula.error);
    let state = skin.state();
    let Some(clipboard) = &state.clipboard else {
        panic!("clipboard projects after formula copy");
    };
    let ClipboardPayloadProjection::Formula {
        source,
        source_path,
        content,
    } = &clipboard.payload
    else {
        panic!("expected formula clipboard payload");
    };
    assert_eq!(source, &b_key);
    assert_eq!(source_path, &NodeId::new("Root.B"));
    assert_eq!(content, "=A+1");
    assert_eq!(clipboard.plain_text.as_deref(), Some("=A+1"));

    let constant_formula = skin.try_copy_to_clipboard(
        AuthoringScope::Node(a_key.clone()),
        ClipboardPayloadKind::Formula,
    );
    assert!(!constant_formula.accepted);
    assert_eq!(
        constant_formula.error,
        Some(IntentError::ClipboardScopeUnsupported {
            payload: "formula".to_string(),
            detail: "formula clipboard payload requires a formula node, got constant".to_string()
        })
    );

    let formatted = skin.try_set_number_format(AuthoringScope::Node(a_key.clone()), Some("0.00"));
    assert!(formatted.accepted, "{:?}", formatted.error);
    let format = skin.try_copy_to_clipboard(
        AuthoringScope::Node(a_key.clone()),
        ClipboardPayloadKind::Format,
    );
    assert!(format.accepted, "{:?}", format.error);
    let state = skin.state();
    let Some(clipboard) = &state.clipboard else {
        panic!("clipboard projects after format copy");
    };
    let ClipboardPayloadProjection::Format { nodes } = &clipboard.payload else {
        panic!("expected format clipboard payload");
    };
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node, a_key);
    assert_eq!(nodes[0].path, NodeId::new("Root.A"));
    assert_eq!(
        nodes[0]
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0.00")
    );
    assert_eq!(clipboard.plain_text, None);

    let subtree = skin.try_copy_to_clipboard(
        AuthoringScope::Subtree(root_key.clone()),
        ClipboardPayloadKind::Subtree,
    );
    assert!(subtree.accepted, "{:?}", subtree.error);
    let state = skin.state();
    let Some(clipboard) = &state.clipboard else {
        panic!("clipboard projects after subtree copy");
    };
    let ClipboardPayloadProjection::Subtree {
        root,
        root_path,
        nodes,
    } = &clipboard.payload
    else {
        panic!("expected subtree clipboard payload");
    };
    assert_eq!(root, &root_key);
    assert_eq!(root_path, &NodeId::new("Root"));
    assert_eq!(nodes.len(), 4);
    assert!(nodes.contains(&b_key));
    assert_eq!(clipboard.plain_text, None);

    let before_cut_revision = revision_fingerprint(&skin.state().revision);
    let cut = skin.try_cut_to_clipboard(
        AuthoringScope::Subtree(root_key.clone()),
        ClipboardPayloadKind::Subtree,
    );
    assert!(cut.accepted, "{:?}", cut.error);
    assert_eq!(cut.transaction_id, None);
    assert!(
        cut.delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::ClipboardChanged(Some(_))))
    );
    let cut_state = skin.state();
    assert_eq!(
        revision_fingerprint(&cut_state.revision),
        before_cut_revision
    );
    assert!(cut_state.node(&NodeId::new("Root")).is_some());
    assert!(cut_state.node(&NodeId::new("Root.B.B1")).is_some());
    let Some(clipboard) = &cut_state.clipboard else {
        panic!("clipboard projects after cut");
    };
    assert_eq!(clipboard.operation, ClipboardOperationProjection::Cut);
    let ClipboardPayloadProjection::Subtree {
        root,
        root_path,
        nodes,
    } = &clipboard.payload
    else {
        panic!("expected cut subtree clipboard payload");
    };
    assert_eq!(root, &root_key);
    assert_eq!(root_path, &NodeId::new("Root"));
    assert_eq!(nodes.len(), 4);

    let unsupported = skin.try_copy_to_clipboard(
        AuthoringScope::Nodes(vec![root_key, b_key]),
        ClipboardPayloadKind::Formula,
    );
    assert!(!unsupported.accepted);
    assert_eq!(
        unsupported.error,
        Some(IntentError::ClipboardScopeUnsupported {
            payload: "formula".to_string(),
            detail: "formula clipboard payload requires exactly one source node, got 2".to_string()
        })
    );
}

#[test]
fn programmable_skin_exports_array_clipboard_values_as_plain_text_grid() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    skin.add_node(None, "Book", "");
    skin.add_node(Some("Book"), "Grid", "=SEQUENCE(2,2)");

    let grid_key = skin
        .state()
        .node(&NodeId::new("Book.Grid"))
        .unwrap()
        .key
        .clone();
    let copy =
        skin.try_copy_to_clipboard(AuthoringScope::Node(grid_key), ClipboardPayloadKind::Values);
    assert!(copy.accepted, "{:?}", copy.error);
    let state = skin.state();
    let clipboard = state.clipboard.as_ref().expect("clipboard projects");
    assert_eq!(clipboard.plain_text.as_deref(), Some("1\t2\n3\t4"));
}

#[test]
fn programmable_skin_pastes_external_clipboard_text_as_authored_content() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "41").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target", "0").accepted);
    assert!(skin.try_add_node(Some("Book"), "Other", "0").accepted);
    skin.recalc();

    let state = skin.state();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let other_key = state.node(&NodeId::new("Book.Other")).unwrap().key.clone();

    let paste_constant =
        skin.try_paste_external_clipboard_text(AuthoringScope::Node(target_key.clone()), "42");
    assert!(paste_constant.accepted, "{:?}", paste_constant.error);
    assert_any_transaction(&paste_constant);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "42");
    assert_eq!(target.computed_value.display_text(), "42");

    let paste_formula = skin
        .try_paste_external_clipboard_text(AuthoringScope::Node(target_key.clone()), "=Source+1");
    assert!(paste_formula.accepted, "{:?}", paste_formula.error);
    assert_any_transaction(&paste_formula);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Formula);
    assert_eq!(target.content_text, "=Source+1");
    assert_eq!(target.computed_value.display_text(), "42");

    let paste_multi = skin.try_paste_external_clipboard_text(
        AuthoringScope::Nodes(vec![other_key.clone()]),
        "'literal text",
    );
    assert!(paste_multi.accepted, "{:?}", paste_multi.error);
    assert_any_transaction(&paste_multi);
    let state = skin.state();
    let other = state.node(&NodeId::new("Book.Other")).unwrap();
    assert_eq!(other.content_kind, NodeContentKind::Constant);
    assert_eq!(other.content_text, "'literal text");
    assert_eq!(other.computed_value.display_text(), "literal text");

    let paste_multiline_single = skin.try_paste_external_clipboard_text(
        AuthoringScope::Node(other_key.clone()),
        "'line one\nline two",
    );
    assert!(
        paste_multiline_single.accepted,
        "{:?}",
        paste_multiline_single.error
    );
    assert_any_transaction(&paste_multiline_single);
    let state = skin.state();
    let other = state.node(&NodeId::new("Book.Other")).unwrap();
    assert_eq!(other.content_kind, NodeContentKind::Constant);
    assert_eq!(other.content_text, "'line one\nline two");
    assert_eq!(other.computed_value.display_text(), "line one\nline two");

    let paste_ordered = skin.try_paste_external_clipboard_text(
        AuthoringScope::Nodes(vec![target_key.clone(), other_key.clone()]),
        "7\t=Source+1\n",
    );
    assert!(paste_ordered.accepted, "{:?}", paste_ordered.error);
    assert_any_transaction(&paste_ordered);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    let other = state.node(&NodeId::new("Book.Other")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "7");
    assert_eq!(target.computed_value.display_text(), "7");
    assert_eq!(other.content_kind, NodeContentKind::Formula);
    assert_eq!(other.content_text, "=Source+1");
    assert_eq!(other.computed_value.display_text(), "42");

    let mismatch = skin.try_paste_external_clipboard_text(
        AuthoringScope::Nodes(vec![target_key, other_key]),
        "1\t2\t3",
    );
    assert!(!mismatch.accepted);
    assert_eq!(
        mismatch.error,
        Some(IntentError::ClipboardScopeUnsupported {
            payload: "values".to_string(),
            detail: "external clipboard paste item count 3 does not match target count 2"
                .to_string()
        })
    );
    let state = skin.state();
    assert_eq!(
        state
            .node(&NodeId::new("Book.Target"))
            .unwrap()
            .content_text,
        "7"
    );
    assert_eq!(
        state.node(&NodeId::new("Book.Other")).unwrap().content_text,
        "=Source+1"
    );

    let empty_target = skin.try_paste_external_clipboard_text(AuthoringScope::Nodes(vec![]), "100");
    assert!(!empty_target.accepted);
    assert_eq!(
        empty_target.error,
        Some(IntentError::ClipboardScopeUnsupported {
            payload: "values".to_string(),
            detail: "external clipboard paste requires at least one target".to_string()
        })
    );
}

#[test]
fn programmable_skin_pastes_constant_clipboard_values_through_content_write_path() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "1000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source2", "2000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target", "=1+1").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target2", "0").accepted);
    assert!(skin.try_add_node(Some("Book"), "Other", "0").accepted);
    assert!(
        skin.try_add_node(Some("Book"), "FormulaSource", "=Source+1")
            .accepted
    );
    skin.recalc();

    let state = skin.state();
    let source_key = state.node(&NodeId::new("Book.Source")).unwrap().key.clone();
    let source2_key = state
        .node(&NodeId::new("Book.Source2"))
        .unwrap()
        .key
        .clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let target2_key = state
        .node(&NodeId::new("Book.Target2"))
        .unwrap()
        .key
        .clone();
    let other_key = state.node(&NodeId::new("Book.Other")).unwrap().key.clone();
    let formula_source_key = state
        .node(&NodeId::new("Book.FormulaSource"))
        .unwrap()
        .key
        .clone();

    let empty_paste = skin.try_paste_clipboard_values(AuthoringScope::Node(target_key.clone()));
    assert!(!empty_paste.accepted);
    assert_eq!(
        empty_paste.error,
        Some(IntentError::ClipboardPayloadMismatch {
            expected: "values".to_string(),
            actual: "empty".to_string()
        })
    );

    let copy = skin.try_copy_to_clipboard(
        AuthoringScope::Node(source_key.clone()),
        ClipboardPayloadKind::Values,
    );
    assert!(copy.accepted, "{:?}", copy.error);
    let paste = skin.try_paste_clipboard_values(AuthoringScope::Nodes(vec![
        target_key.clone(),
        other_key.clone(),
    ]));
    assert!(paste.accepted, "{:?}", paste.error);
    assert_any_transaction(&paste);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    let other = state.node(&NodeId::new("Book.Other")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "1000");
    assert_eq!(target.computed_value.display_text(), "1000");
    assert_eq!(other.content_kind, NodeContentKind::Constant);
    assert_eq!(other.content_text, "1000");
    assert_eq!(other.computed_value.display_text(), "1000");
    assert!(matches!(
        state.clipboard.as_ref().map(|clipboard| &clipboard.payload),
        Some(ClipboardPayloadProjection::Values { .. })
    ));

    let formula_copy = skin.try_copy_to_clipboard(
        AuthoringScope::Node(formula_source_key),
        ClipboardPayloadKind::Values,
    );
    assert!(formula_copy.accepted, "{:?}", formula_copy.error);
    let formula_paste = skin.try_paste_clipboard_values(AuthoringScope::Node(target_key.clone()));
    assert!(formula_paste.accepted, "{:?}", formula_paste.error);
    assert_any_transaction(&formula_paste);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "1001");
    assert_eq!(target.computed_value.display_text(), "1001");

    let multi_copy = skin.try_copy_to_clipboard(
        AuthoringScope::Nodes(vec![source_key.clone(), source2_key.clone()]),
        ClipboardPayloadKind::Values,
    );
    assert!(multi_copy.accepted, "{:?}", multi_copy.error);
    let multi_paste = skin.try_paste_clipboard_values(AuthoringScope::Nodes(vec![
        target_key.clone(),
        target2_key.clone(),
    ]));
    assert!(multi_paste.accepted, "{:?}", multi_paste.error);
    assert_any_transaction(&multi_paste);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    let target2 = state.node(&NodeId::new("Book.Target2")).unwrap();
    assert_eq!(target.content_text, "1000");
    assert_eq!(target.computed_value.display_text(), "1000");
    assert_eq!(target2.content_kind, NodeContentKind::Constant);
    assert_eq!(target2.content_text, "2000");
    assert_eq!(target2.computed_value.display_text(), "2000");

    let mismatched_multi_paste = skin.try_paste_clipboard_values(AuthoringScope::Node(target_key));
    assert!(!mismatched_multi_paste.accepted);
    assert_eq!(
        mismatched_multi_paste.error,
        Some(IntentError::ClipboardPayloadMismatch {
            expected: "ordered_literal_values".to_string(),
            actual: "value_count=2,target_count=1".to_string()
        })
    );
}

#[test]
fn programmable_skin_commits_multi_source_constant_clipboard_value_cut_paste_atomically() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "1000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source2", "2000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target", "0").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target2", "0").accepted);
    skin.recalc();

    let state = skin.state();
    let source_key = state.node(&NodeId::new("Book.Source")).unwrap().key.clone();
    let source2_key = state
        .node(&NodeId::new("Book.Source2"))
        .unwrap()
        .key
        .clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let target2_key = state
        .node(&NodeId::new("Book.Target2"))
        .unwrap()
        .key
        .clone();
    let cut = skin.try_cut_to_clipboard(
        AuthoringScope::Nodes(vec![source_key.clone(), source2_key.clone()]),
        ClipboardPayloadKind::Values,
    );
    assert!(cut.accepted, "{:?}", cut.error);

    let paste =
        skin.try_paste_clipboard_values(AuthoringScope::Nodes(vec![target_key, target2_key]));
    assert!(paste.accepted, "{:?}", paste.error);
    assert_any_transaction(&paste);
    assert!(
        paste
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::ClipboardChanged(None)))
    );
    let state = skin.state();
    let source = state.node(&NodeId::new("Book.Source")).unwrap();
    let source2 = state.node(&NodeId::new("Book.Source2")).unwrap();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    let target2 = state.node(&NodeId::new("Book.Target2")).unwrap();
    assert_eq!(source.content_kind, NodeContentKind::Empty);
    assert_eq!(source.content_text, "");
    assert_eq!(source2.content_kind, NodeContentKind::Empty);
    assert_eq!(source2.content_text, "");
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "1000");
    assert_eq!(target.computed_value.display_text(), "1000");
    assert_eq!(target2.content_kind, NodeContentKind::Constant);
    assert_eq!(target2.content_text, "2000");
    assert_eq!(target2.computed_value.display_text(), "2000");
    assert_eq!(state.clipboard, None);
}

#[test]
fn programmable_skin_pastes_mixed_constant_and_array_value_sources() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    skin.add_node(None, "Book", "");
    skin.add_node(Some("Book"), "Source", "1000");
    skin.add_node(Some("Book"), "ArraySource", "=SEQUENCE(2,2)");
    skin.add_node(Some("Book"), "Target", "0");
    skin.add_node(Some("Book"), "Target2", "0");

    let state = skin.state();
    let array_source = state.node(&NodeId::new("Book.ArraySource")).unwrap();
    assert_eq!(array_source.content_kind, NodeContentKind::Formula);
    assert!(
        matches!(
            array_source.computed_value,
            NodeValueProjection::Array { .. }
        ),
        "expected array source to project as array, got {:?}",
        array_source.computed_value
    );
    assert_eq!(
        array_source.literalized_value_input.as_deref(),
        Some("={1,2;3,4}")
    );
    let source_key = state.node(&NodeId::new("Book.Source")).unwrap().key.clone();
    let array_source_key = state
        .node(&NodeId::new("Book.ArraySource"))
        .unwrap()
        .key
        .clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let target2_key = state
        .node(&NodeId::new("Book.Target2"))
        .unwrap()
        .key
        .clone();
    let multi_copy = skin.try_copy_to_clipboard(
        AuthoringScope::Nodes(vec![source_key, array_source_key]),
        ClipboardPayloadKind::Values,
    );
    assert!(multi_copy.accepted, "{:?}", multi_copy.error);
    let multi_paste =
        skin.try_paste_clipboard_values(AuthoringScope::Nodes(vec![target_key, target2_key]));
    assert!(multi_paste.accepted, "{:?}", multi_paste.error);
    assert_any_transaction(&multi_paste);
    let state = skin.state();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    let target2 = state.node(&NodeId::new("Book.Target2")).unwrap();
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "1000");
    assert_eq!(target.computed_value.display_text(), "1000");
    assert_eq!(target2.content_kind, NodeContentKind::Formula);
    assert_eq!(target2.content_text, "={1,2;3,4}");
    let NodeValueProjection::Array { rows, cols, cells } = &target2.computed_value else {
        panic!(
            "array value paste should project as array, got {:?}",
            target2.computed_value
        );
    };
    assert_eq!((*rows, *cols), (2, 2));
    assert_eq!(cells[0][0].display_text(), "1");
    assert_eq!(cells[1][1].display_text(), "4");
}

#[test]
fn programmable_skin_commits_constant_clipboard_value_cut_paste_atomically() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "1000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target", "0").accepted);
    skin.recalc();

    let state = skin.state();
    let source_key = state.node(&NodeId::new("Book.Source")).unwrap().key.clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let cut = skin.try_cut_to_clipboard(
        AuthoringScope::Node(source_key.clone()),
        ClipboardPayloadKind::Values,
    );
    assert!(cut.accepted, "{:?}", cut.error);

    let paste = skin.try_paste_clipboard_values(AuthoringScope::Node(target_key));
    assert!(paste.accepted, "{:?}", paste.error);
    assert_any_transaction(&paste);
    assert!(
        paste
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::ClipboardChanged(None)))
    );
    let state = skin.state();
    let source = state.node(&NodeId::new("Book.Source")).unwrap();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    assert_eq!(source.content_kind, NodeContentKind::Empty);
    assert_eq!(source.content_text, "");
    assert_eq!(target.content_kind, NodeContentKind::Constant);
    assert_eq!(target.content_text, "1000");
    assert_eq!(target.computed_value.display_text(), "1000");
    assert_eq!(state.clipboard, None);
}

#[test]
fn programmable_skin_commits_array_value_cut_paste_atomically() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    skin.add_node(None, "Book", "");
    skin.add_node(Some("Book"), "ArraySource", "=SEQUENCE(2,2)");
    skin.add_node(Some("Book"), "Target", "0");

    let state = skin.state();
    let array_source = state.node(&NodeId::new("Book.ArraySource")).unwrap();
    assert_eq!(array_source.content_kind, NodeContentKind::Formula);
    assert!(
        matches!(
            array_source.computed_value,
            NodeValueProjection::Array { .. }
        ),
        "expected array source to project as array, got {:?}",
        array_source.computed_value
    );
    assert_eq!(
        array_source.literalized_value_input.as_deref(),
        Some("={1,2;3,4}")
    );
    let array_source_key = state
        .node(&NodeId::new("Book.ArraySource"))
        .unwrap()
        .key
        .clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let cut = skin.try_cut_to_clipboard(
        AuthoringScope::Node(array_source_key),
        ClipboardPayloadKind::Values,
    );
    assert!(cut.accepted, "{:?}", cut.error);

    let paste = skin.try_paste_clipboard_values(AuthoringScope::Node(target_key));
    assert!(paste.accepted, "{:?}", paste.error);
    assert_any_transaction(&paste);
    assert!(
        paste
            .delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::ClipboardChanged(None)))
    );
    let state = skin.state();
    let array_source = state.node(&NodeId::new("Book.ArraySource")).unwrap();
    let target = state.node(&NodeId::new("Book.Target")).unwrap();
    assert_eq!(array_source.content_kind, NodeContentKind::Empty);
    assert_eq!(array_source.content_text, "");
    assert_eq!(target.content_kind, NodeContentKind::Formula);
    assert_eq!(target.content_text, "={1,2;3,4}");
    assert!(matches!(
        target.computed_value,
        NodeValueProjection::Array { .. }
    ));
    assert_eq!(state.clipboard, None);
}

#[test]
fn programmable_skin_rejects_empty_target_value_cut_paste_without_clearing_source() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "1000").accepted);
    skin.recalc();

    let source_key = skin
        .state()
        .node(&NodeId::new("Book.Source"))
        .unwrap()
        .key
        .clone();
    let cut = skin.try_cut_to_clipboard(
        AuthoringScope::Node(source_key),
        ClipboardPayloadKind::Values,
    );
    assert!(cut.accepted, "{:?}", cut.error);

    let paste = skin.try_paste_clipboard_values(AuthoringScope::Nodes(vec![]));
    assert!(!paste.accepted);
    assert_eq!(
        paste.error,
        Some(IntentError::ClipboardScopeUnsupported {
            payload: "values".to_string(),
            detail: "value paste requires at least one target".to_string()
        })
    );
    let state = skin.state();
    let source = state.node(&NodeId::new("Book.Source")).unwrap();
    assert_eq!(source.content_kind, NodeContentKind::Constant);
    assert_eq!(source.content_text, "1000");
    assert!(matches!(
        state.clipboard.as_ref().map(|clipboard| &clipboard.payload),
        Some(ClipboardPayloadProjection::Values { .. })
    ));
}

#[test]
fn programmable_skin_pastes_clipboard_format_through_format_write_path() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Source", "1000").accepted);
    assert!(skin.try_add_node(Some("Book"), "Target", "0.2").accepted);
    assert!(skin.try_add_node(Some("Book"), "Plain", "42").accepted);
    skin.recalc();

    let state = skin.state();
    let source_key = state.node(&NodeId::new("Book.Source")).unwrap().key.clone();
    let target_key = state.node(&NodeId::new("Book.Target")).unwrap().key.clone();
    let plain_key = state.node(&NodeId::new("Book.Plain")).unwrap().key.clone();

    let empty_paste = skin.try_paste_clipboard_format(AuthoringScope::Node(target_key.clone()));
    assert!(!empty_paste.accepted);
    assert_eq!(
        empty_paste.error,
        Some(IntentError::ClipboardPayloadMismatch {
            expected: "format".to_string(),
            actual: "empty".to_string()
        })
    );

    let source_format =
        skin.try_set_number_format(AuthoringScope::Node(source_key.clone()), Some("0.00"));
    assert!(source_format.accepted, "{:?}", source_format.error);
    assert_any_transaction(&source_format);
    let copy = skin.try_copy_to_clipboard(
        AuthoringScope::Node(source_key.clone()),
        ClipboardPayloadKind::Format,
    );
    assert!(copy.accepted, "{:?}", copy.error);

    let paste = skin.try_paste_clipboard_format(AuthoringScope::Node(target_key.clone()));
    assert!(paste.accepted, "{:?}", paste.error);
    assert_any_transaction(&paste);
    let state = skin.state();
    assert_eq!(
        state
            .node(&NodeId::new("Book.Target"))
            .unwrap()
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0.00")
    );
    assert!(matches!(
        state.clipboard.as_ref().map(|clipboard| &clipboard.payload),
        Some(ClipboardPayloadProjection::Format { .. })
    ));

    let clear_source = skin.try_copy_to_clipboard(
        AuthoringScope::Node(plain_key.clone()),
        ClipboardPayloadKind::Format,
    );
    assert!(clear_source.accepted, "{:?}", clear_source.error);
    let clear = skin.try_paste_clipboard_format(AuthoringScope::Node(target_key.clone()));
    assert!(clear.accepted, "{:?}", clear.error);
    assert_any_transaction(&clear);
    let state = skin.state();
    assert_eq!(
        state
            .node(&NodeId::new("Book.Target"))
            .unwrap()
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        None
    );

    let multi_copy = skin.try_copy_to_clipboard(
        AuthoringScope::Nodes(vec![source_key, plain_key]),
        ClipboardPayloadKind::Format,
    );
    assert!(multi_copy.accepted, "{:?}", multi_copy.error);
    let multi_paste = skin.try_paste_clipboard_format(AuthoringScope::Node(target_key));
    assert!(!multi_paste.accepted);
    assert_eq!(
        multi_paste.error,
        Some(IntentError::ClipboardPayloadMismatch {
            expected: "single_format".to_string(),
            actual: "format_count=2".to_string()
        })
    );
}

#[test]
fn programmable_skin_duplicates_formula_free_subtree_through_skin_ir() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Template", "").accepted);
    assert!(
        skin.try_add_node(Some("Book.Template"), "Inputs", "7")
            .accepted
    );
    let state = skin.state();
    let input_key = state
        .node(&NodeId::new("Book.Template.Inputs"))
        .unwrap()
        .key
        .clone();
    assert!(
        skin.try_set_note(input_key.clone(), Some("Scenario input"))
            .accepted
    );
    assert!(
        skin.try_set_number_format(AuthoringScope::Node(input_key.clone()), Some("0.00"))
            .accepted
    );
    assert!(
        skin.try_set_node_attributes(input_key, NodeAttributePatch::set("owner", "planning"))
            .accepted
    );
    skin.recalc();

    let state = skin.state();
    let template_key = state
        .node(&NodeId::new("Book.Template"))
        .unwrap()
        .key
        .clone();
    let duplicate = skin.try_duplicate_subtree(template_key, Some("Book"), "Scenario");
    assert!(duplicate.accepted, "{:?}", duplicate.error);
    assert_any_transaction(&duplicate);

    let state = skin.state();
    let scenario = state.node(&NodeId::new("Book.Scenario")).unwrap();
    let input = state.node(&NodeId::new("Book.Scenario.Inputs")).unwrap();
    assert_ne!(
        scenario.key,
        state.node(&NodeId::new("Book.Template")).unwrap().key
    );
    assert_eq!(input.content_kind, NodeContentKind::Constant);
    assert_eq!(input.content_text, "7");
    assert_eq!(
        input.note.as_ref().map(|note| note.text.as_str()),
        Some("Scenario input")
    );
    assert_eq!(
        input
            .effective_format
            .as_ref()
            .and_then(|format| format.number_format_code.as_deref()),
        Some("0.00")
    );
    assert_eq!(
        input.attributes.get("owner").map(String::as_str),
        Some("planning")
    );
    skin.assert_scalar("Book.Scenario.Inputs", "7.00");
    assert!(duplicate.delta.changes.iter().any(|change| matches!(
        change,
        WorkspaceDeltaChange::Structural(structural)
            if structural.added.len() == 2
    )));
}

#[test]
fn programmable_skin_duplicates_hidden_custom_meta_subtree_without_projecting_it() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Template", "").accepted);
    assert!(
        skin.try_add_node(Some("Book.Template"), "Inputs", "7")
            .accepted
    );
    {
        let mut session = harness.session.lock().unwrap();
        session
            .add_node_transaction_with_meta(
                Some(&NodeId::new("Book.Template.Inputs")),
                "Scratch",
                "scratch",
                true,
            )
            .expect("custom meta parent is added");
        session
            .add_node_transaction_with_meta(
                Some(&NodeId::new("Book.Template.Inputs.Scratch")),
                "Review",
                "keep",
                true,
            )
            .expect("custom meta child is added");
        let document = session
            .export_dnatree_document(None)
            .expect("document export succeeds");
        let (reopened, _) =
            TreeWorkspaceSession::from_dnatree_document(document).expect("reopen succeeds");
        drop(session);
        let reopened = Harness::from_session(reopened);
        let reopened_skin = reopened.driver.clone();
        assert!(
            reopened_skin
                .state()
                .node(&NodeId::new("Book.Template.Inputs.Scratch"))
                .is_none()
        );
        let template_key = reopened_skin
            .state()
            .node(&NodeId::new("Book.Template"))
            .unwrap()
            .key
            .clone();
        let duplicate = reopened_skin.try_duplicate_subtree(template_key, Some("Book"), "Scenario");
        assert!(duplicate.accepted, "{:?}", duplicate.error);
        assert!(
            reopened_skin
                .state()
                .node(&NodeId::new("Book.Scenario.Inputs.Scratch"))
                .is_none()
        );
        let occupied = reopened_skin.try_add_node(Some("Book.Scenario.Inputs"), "Scratch", "");
        assert!(!occupied.accepted);
        assert_eq!(
            occupied.error,
            Some(IntentError::DuplicateNode {
                node: "Book.Scenario.Inputs.Scratch".to_string()
            })
        );
    }
}

#[test]
fn programmable_skin_duplicates_constant_table_subtree_through_skin_ir() {
    let harness = Harness::from_fixture(constant_table_fixture());
    let skin = harness.driver.clone();
    skin.recalc();

    let state = skin.state();
    let source_table = state
        .tables
        .get(&NodeId::new("InputTable"))
        .expect("source constant-only table projects");
    assert_eq!(table_body_row(source_table, 0), vec!["5", "1"]);
    assert_eq!(table_body_row(source_table, 1), vec!["15", "2"]);
    let source_table_id = source_table.table_id.clone();
    let source_key = state.node(&NodeId::new("InputTable")).unwrap().key.clone();

    let duplicate = skin.try_duplicate_subtree(source_key, None, "ScenarioTable");
    assert!(duplicate.accepted, "{:?}", duplicate.error);
    assert_any_transaction(&duplicate);

    let state = skin.state();
    let cloned = state
        .tables
        .get(&NodeId::new("ScenarioTable"))
        .expect("cloned table projects");
    assert_eq!(cloned.table_name, "ScenarioTable");
    assert_ne!(cloned.table_id, source_table_id);
    assert!(cloned.table_id.contains("ScenarioTable"));
    assert_eq!(cloned.row_count, 2);
    assert_eq!(cloned.column_count, 2);
    assert_eq!(table_body_row(cloned, 0), vec!["5", "1"]);
    assert_eq!(table_body_row(cloned, 1), vec!["15", "2"]);
    assert!(
        state
            .node(&NodeId::new("ScenarioTable.__table_body_r1_c1"))
            .is_none()
    );

    let occupied = skin.try_add_node(Some("ScenarioTable"), "__table_body_r1_c1", "");
    assert!(!occupied.accepted);
    assert_eq!(
        occupied.error,
        Some(IntentError::DuplicateNode {
            node: "ScenarioTable.__table_body_r1_c1".to_string()
        })
    );
}

#[test]
fn programmable_skin_rejects_formula_backed_table_subtree_duplicate() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    let source_key = skin
        .state()
        .node(&NodeId::new("SalesTable"))
        .unwrap()
        .key
        .clone();

    let duplicate = skin.try_duplicate_subtree(source_key, None, "SalesCopy");
    assert!(!duplicate.accepted);
    assert_eq!(
        duplicate.error,
        Some(IntentError::DuplicateSubtreeUnsupported {
            node: "SalesTable".to_string(),
            detail: "formula-backed table columns require OxFml-owned table formula rebind"
                .to_string()
        })
    );
    assert!(skin.state().node(&NodeId::new("SalesCopy")).is_none());
}

#[test]
fn programmable_skin_rejects_table_duplicate_that_would_collide_by_table_name() {
    let harness = Harness::from_fixture(nested_constant_table_fixture());
    let skin = harness.driver.clone();
    let source_key = skin.state().node(&NodeId::new("Root")).unwrap().key.clone();

    let duplicate = skin.try_duplicate_subtree(source_key, None, "Scenario");
    assert!(!duplicate.accepted);
    assert_eq!(
        duplicate.error,
        Some(IntentError::DuplicateSubtreeUnsupported {
            node: "Root.InputTable".to_string(),
            detail:
                "table subtree duplication would duplicate formula-visible table name InputTable"
                    .to_string()
        })
    );
    assert!(skin.state().node(&NodeId::new("Scenario")).is_none());
}

#[test]
fn programmable_skin_rejects_duplicate_subtree_that_needs_formula_rebind() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();
    assert!(skin.try_add_node(None, "Book", "").accepted);
    assert!(skin.try_add_node(Some("Book"), "Template", "").accepted);
    assert!(
        skin.try_add_node(Some("Book.Template"), "Input", "7")
            .accepted
    );
    assert!(
        skin.try_add_node(Some("Book.Template"), "Total", "=Input+1")
            .accepted
    );
    skin.recalc();

    let state = skin.state();
    let template_key = state
        .node(&NodeId::new("Book.Template"))
        .unwrap()
        .key
        .clone();
    let duplicate = skin.try_duplicate_subtree(template_key, Some("Book"), "Scenario");
    assert!(!duplicate.accepted);
    assert_eq!(
        duplicate.error,
        Some(IntentError::DuplicateSubtreeUnsupported {
            node: "Book.Template.Total".to_string(),
            detail: "formula-bearing subtree duplication requires OxFml-owned reference rebind"
                .to_string()
        })
    );
    let state = skin.state();
    assert!(state.node(&NodeId::new("Book.Scenario")).is_none());
}

#[test]
fn programmable_skin_edits_table_cells_and_adds_rows_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    let before_revision = revision_fingerprint(&before.revision);
    assert_eq!(table_body_row(before_table, 1), vec!["East", "20", "2"]);
    assert_eq!(table_totals_row(before_table), vec!["", "60", ""]);

    let edit = skin.try_edit_table_cell("SalesTable", "row:east", "col:amount", "25");
    assert!(edit.accepted, "{:?}", edit.error);
    assert_table_transaction(&edit);
    assert_ne!(
        revision_fingerprint(&skin.state().revision),
        before_revision
    );
    let edited_state = skin.state();
    let edited_table = edited_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after cell edit");
    assert_eq!(edited_table.row_count, 3);
    assert_eq!(table_body_row(edited_table, 1), vec!["East", "25", "2.5"]);
    assert_eq!(table_totals_row(edited_table), vec!["", "65", ""]);
    assert!(!edited_state.node_order.iter().any(|node| {
        node.as_str().contains("__table_body_") || node.as_str().contains("row:east")
    }));

    let formula_edit = skin.try_edit_table_cell("SalesTable", "row:east", "col:tax", "99");
    assert!(!formula_edit.accepted, "{formula_edit:?}");
    assert!(matches!(
        formula_edit.error,
        Some(IntentError::FormulaTableCellEdit {
            ref table,
            ref column_id
        }) if table == "SalesTable" && column_id == "col:tax"
    ));
    assert_eq!(
        table_body_row(
            skin.state()
                .tables
                .get(&NodeId::new("SalesTable"))
                .expect("table still projects"),
            1,
        ),
        vec!["East", "25", "2.5"]
    );

    let add = skin.try_add_table_row(
        "SalesTable",
        "row:south",
        &[("col:region", "South"), ("col:amount", "40")],
    );
    assert!(add.accepted, "{:?}", add.error);
    assert_table_transaction(&add);
    let added_state = skin.state();
    let added_table = added_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after row add");
    assert_eq!(added_table.row_count, 4);
    assert_eq!(
        added_table
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>(),
        vec!["row:west", "row:east", "row:north", "row:south"]
    );
    assert_eq!(table_body_row(added_table, 3), vec!["South", "40", "4"]);
    assert_eq!(table_totals_row(added_table), vec!["", "105", ""]);
    assert_ne!(
        before_table.row_membership_version,
        added_table.row_membership_version
    );
    assert_ne!(
        before_table.row_order_version,
        added_table.row_order_version
    );

    let duplicate = skin.try_add_table_row("SalesTable", "row:south", &[]);
    assert!(!duplicate.accepted, "{duplicate:?}");
    assert!(matches!(
        duplicate.error,
        Some(IntentError::DuplicateTableRow {
            ref table,
            ref row_id
        }) if table == "SalesTable" && row_id == "row:south"
    ));

    let formula_input = skin.try_add_table_row("SalesTable", "row:formula", &[("col:tax", "9")]);
    assert!(!formula_input.accepted, "{formula_input:?}");
    assert!(matches!(
        formula_input.error,
        Some(IntentError::FormulaTableCellEdit {
            ref table,
            ref column_id
        }) if table == "SalesTable" && column_id == "col:tax"
    ));

    let duplicate_input = skin.try_add_table_row(
        "SalesTable",
        "row:duplicate-input",
        &[("col:amount", "1"), ("col:amount", "2")],
    );
    assert!(!duplicate_input.accepted, "{duplicate_input:?}");
    assert!(matches!(
        duplicate_input.error,
        Some(IntentError::DuplicateTableCellInput {
            ref table,
            ref column_id
        }) if table == "SalesTable" && column_id == "col:amount"
    ));

    let delete = skin.try_delete_table_row("SalesTable", "row:east");
    assert!(delete.accepted, "{:?}", delete.error);
    assert_table_transaction(&delete);
    let deleted_state = skin.state();
    let deleted_table = deleted_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after row delete");
    assert_eq!(deleted_table.row_count, 3);
    assert_eq!(
        deleted_table
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>(),
        vec!["row:west", "row:north", "row:south"]
    );
    assert_eq!(table_body_row(deleted_table, 0), vec!["West", "10", "1"]);
    assert_eq!(table_body_row(deleted_table, 1), vec!["North", "30", "3"]);
    assert_eq!(table_body_row(deleted_table, 2), vec!["South", "40", "4"]);
    assert_eq!(table_totals_row(deleted_table), vec!["", "80", ""]);
    assert!(!deleted_state.node_order.iter().any(|node| {
        node.as_str().contains("__table_body_") || node.as_str().contains("row:east")
    }));

    let missing_delete = skin.try_delete_table_row("SalesTable", "row:east");
    assert!(!missing_delete.accepted, "{missing_delete:?}");
}

#[test]
fn programmable_skin_renames_and_reorders_table_rows_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    let before_membership_version = before_table.row_membership_version.clone();
    let before_order_version = before_table.row_order_version.clone();
    assert_eq!(
        before_table
            .rows
            .iter()
            .map(|row| (row.row_id.as_str(), row.ordinal))
            .collect::<Vec<_>>(),
        vec![("row:west", 1), ("row:east", 2), ("row:north", 3)]
    );

    let rename = skin.try_rename_table_row("SalesTable", "row:east", "row:central");
    assert!(rename.accepted, "{:?}", rename.error);
    assert_table_transaction(&rename);
    let renamed_state = skin.state();
    let renamed_table = renamed_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after row rename");
    assert_eq!(
        renamed_table
            .rows
            .iter()
            .map(|row| (row.row_id.as_str(), row.ordinal))
            .collect::<Vec<_>>(),
        vec![("row:west", 1), ("row:central", 2), ("row:north", 3)]
    );
    assert_eq!(table_body_row(renamed_table, 1), vec!["East", "20", "2"]);
    assert_ne!(
        before_membership_version,
        renamed_table.row_membership_version
    );
    assert_eq!(before_order_version, renamed_table.row_order_version);

    let old_id_edit = skin.try_edit_table_cell("SalesTable", "row:east", "col:amount", "25");
    assert!(!old_id_edit.accepted, "{old_id_edit:?}");
    let new_id_edit = skin.try_edit_table_cell("SalesTable", "row:central", "col:amount", "25");
    assert!(new_id_edit.accepted, "{:?}", new_id_edit.error);
    let edited_state = skin.state();
    let edited_table = edited_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after renamed row edit");
    assert_eq!(table_body_row(edited_table, 1), vec!["East", "25", "2.5"]);
    assert_eq!(table_totals_row(edited_table), vec!["", "65", ""]);

    let duplicate = skin.try_rename_table_row("SalesTable", "row:central", "row:west");
    assert!(!duplicate.accepted, "{duplicate:?}");
    let missing_rename = skin.try_rename_table_row("SalesTable", "row:missing", "row:new");
    assert!(!missing_rename.accepted, "{missing_rename:?}");

    let reorder = skin.try_reorder_table_row("SalesTable", "row:north", 0);
    assert!(reorder.accepted, "{:?}", reorder.error);
    assert_table_transaction(&reorder);
    let reordered_state = skin.state();
    let reordered_table = reordered_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after row reorder");
    assert_eq!(
        reordered_table
            .rows
            .iter()
            .map(|row| (row.row_id.as_str(), row.ordinal))
            .collect::<Vec<_>>(),
        vec![("row:north", 1), ("row:west", 2), ("row:central", 3)]
    );
    assert_eq!(table_body_row(reordered_table, 0), vec!["North", "30", "3"]);
    assert_eq!(
        table_body_row(reordered_table, 2),
        vec!["East", "25", "2.5"]
    );
    assert_ne!(
        edited_table.row_order_version,
        reordered_table.row_order_version
    );

    let bounded_reorder = skin.try_reorder_table_row("SalesTable", "row:north", usize::MAX);
    assert!(bounded_reorder.accepted, "{:?}", bounded_reorder.error);
    assert_table_transaction(&bounded_reorder);
    let bounded_state = skin.state();
    let bounded_table = bounded_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after bounded row reorder");
    assert_eq!(
        bounded_table
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>(),
        vec!["row:west", "row:central", "row:north"]
    );
    assert_eq!(table_body_row(bounded_table, 2), vec!["North", "30", "3"]);

    let missing_reorder = skin.try_reorder_table_row("SalesTable", "row:missing", 0);
    assert!(!missing_reorder.accepted, "{missing_reorder:?}");
}

#[test]
fn programmable_skin_adds_edits_and_deletes_constant_table_columns_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    let before_column_identity_version = before_table.column_identity_version.clone();
    assert_eq!(
        before_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax"]
    );

    let add = skin.try_add_table_column(
        "SalesTable",
        "col:discount",
        "Discount",
        &[("row:west", "1"), ("row:east", "2"), ("row:north", "3")],
    );
    assert!(add.accepted, "{:?}", add.error);
    assert_table_transaction(&add);

    let added_state = skin.state();
    let added_table = added_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after column add");
    assert_eq!(added_table.column_count, 4);
    assert_eq!(
        added_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax", "Discount"]
    );
    assert!(matches!(
        added_table.columns[3].body,
        TableColumnBodyProjection::ConstantCells
    ));
    assert_eq!(table_body_row(added_table, 0), vec!["West", "10", "1", "1"]);
    assert_eq!(table_body_row(added_table, 1), vec!["East", "20", "2", "2"]);
    assert_eq!(
        table_body_row(added_table, 2),
        vec!["North", "30", "3", "3"]
    );
    assert_eq!(table_totals_row(added_table), vec!["", "60", "", ""]);
    assert_ne!(
        before_column_identity_version,
        added_table.column_identity_version
    );
    assert!(!added_state.node_order.iter().any(|node| {
        node.as_str().contains("__table_body_") || node.as_str().contains("col:discount")
    }));

    let edit = skin.try_edit_table_cell("SalesTable", "row:east", "col:discount", "5");
    assert!(edit.accepted, "{:?}", edit.error);
    let edited_state = skin.state();
    let edited_table = edited_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after new column edit");
    assert_eq!(
        table_body_row(edited_table, 1),
        vec!["East", "20", "2", "5"]
    );

    let duplicate = skin.try_add_table_column("SalesTable", "col:discount", "Discount", &[]);
    assert!(!duplicate.accepted, "{duplicate:?}");

    let unknown_row =
        skin.try_add_table_column("SalesTable", "col:bad", "Bad", &[("row:missing", "9")]);
    assert!(!unknown_row.accepted, "{unknown_row:?}");

    let duplicate_input = skin.try_add_table_column(
        "SalesTable",
        "col:bad",
        "Bad",
        &[("row:west", "1"), ("row:west", "2")],
    );
    assert!(!duplicate_input.accepted, "{duplicate_input:?}");

    let delete = skin.try_delete_table_column("SalesTable", "col:discount");
    assert!(delete.accepted, "{:?}", delete.error);
    assert_table_transaction(&delete);
    let deleted_state = skin.state();
    let deleted_table = deleted_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after column delete");
    assert_eq!(deleted_table.column_count, 3);
    assert_eq!(
        deleted_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax"]
    );
    assert_eq!(table_body_row(deleted_table, 0), vec!["West", "10", "1"]);
    assert_eq!(table_body_row(deleted_table, 1), vec!["East", "20", "2"]);
    assert_eq!(table_body_row(deleted_table, 2), vec!["North", "30", "3"]);
    assert_eq!(table_totals_row(deleted_table), vec!["", "60", ""]);

    let missing_delete = skin.try_delete_table_column("SalesTable", "col:discount");
    assert!(!missing_delete.accepted, "{missing_delete:?}");
}

#[test]
fn programmable_skin_authors_formula_table_columns_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let add =
        skin.try_add_table_formula_column("SalesTable", "col:double", "Double", "=[@Amount] * 2");
    assert!(add.accepted, "{:?}", add.error);
    assert_table_transaction(&add);
    let added_state = skin.state();
    let added_table = added_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after formula column add");
    assert_eq!(added_table.column_count, 4);
    assert_eq!(
        added_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax", "Double"]
    );
    let TableColumnBodyProjection::Formula(double_formula) = &added_table.columns[3].body else {
        panic!("Double column should project formula metadata");
    };
    assert_eq!(double_formula.formula_text, "=[@Amount] * 2");
    assert_eq!(double_formula.formula_text_version, "v1");
    assert_eq!(
        double_formula.formula_artifact_id,
        "formula:SalesTable.Columns.col_double"
    );
    assert_eq!(
        double_formula.bind_artifact_id.as_deref(),
        Some("bind:SalesTable.Columns.col_double")
    );
    assert_eq!(
        table_body_row(added_table, 0),
        vec!["West", "10", "1", "20"]
    );
    assert_eq!(
        table_body_row(added_table, 1),
        vec!["East", "20", "2", "40"]
    );
    assert_eq!(
        table_body_row(added_table, 2),
        vec!["North", "30", "3", "60"]
    );
    assert_eq!(table_totals_row(added_table), vec!["", "60", "", ""]);

    let edit = skin.try_edit_table_column_formula("SalesTable", "col:double", "=[@Amount] + 5");
    assert!(edit.accepted, "{:?}", edit.error);
    assert_table_transaction(&edit);
    let edited_state = skin.state();
    let edited_table = edited_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after formula edit");
    let TableColumnBodyProjection::Formula(edited_formula) = &edited_table.columns[3].body else {
        panic!("Double column should stay formula metadata");
    };
    assert_eq!(edited_formula.formula_text, "=[@Amount] + 5");
    assert_eq!(edited_formula.formula_text_version, "v2");
    assert_eq!(
        table_body_row(edited_table, 0),
        vec!["West", "10", "1", "15"]
    );
    assert_eq!(
        table_body_row(edited_table, 1),
        vec!["East", "20", "2", "25"]
    );
    assert_eq!(
        table_body_row(edited_table, 2),
        vec!["North", "30", "3", "35"]
    );

    let constant_edit = skin.try_edit_table_column_formula("SalesTable", "col:amount", "=[@Tax]");
    assert!(!constant_edit.accepted, "{constant_edit:?}");

    let duplicate =
        skin.try_add_table_formula_column("SalesTable", "col:double", "Double", "=[@Amount]");
    assert!(!duplicate.accepted, "{duplicate:?}");

    let delete_formula = skin.try_delete_table_column("SalesTable", "col:double");
    assert!(delete_formula.accepted, "{:?}", delete_formula.error);
    assert_table_transaction(&delete_formula);
    let deleted_state = skin.state();
    let deleted_table = deleted_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after formula column delete");
    assert_eq!(deleted_table.column_count, 3);
    assert_eq!(
        deleted_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax"]
    );
    assert_eq!(table_body_row(deleted_table, 0), vec!["West", "10", "1"]);

    let delete_existing_formula = skin.try_delete_table_column("SalesTable", "col:tax");
    assert!(
        delete_existing_formula.accepted,
        "{:?}",
        delete_existing_formula.error
    );
    assert_table_transaction(&delete_existing_formula);
    let tax_deleted_state = skin.state();
    let tax_deleted_table = tax_deleted_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after existing formula column delete");
    assert_eq!(
        tax_deleted_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount"]
    );
    assert_eq!(table_body_row(tax_deleted_table, 0), vec!["West", "10"]);
    assert_eq!(table_totals_row(tax_deleted_table), vec!["", "60"]);
}

#[test]
fn programmable_skin_authors_table_totals_formulas_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    assert_eq!(table_totals_row(before_table), vec!["", "60", ""]);
    assert!(before_table.columns[0].totals_formula.is_none());
    assert!(before_table.columns[2].totals_formula.is_none());

    let tax_totals =
        skin.try_set_table_totals_formula("SalesTable", "col:tax", "=SUM(SalesTable[Tax])");
    assert!(tax_totals.accepted, "{:?}", tax_totals.error);
    assert_table_transaction(&tax_totals);
    let tax_state = skin.state();
    let tax_table = tax_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals formula add");
    assert_eq!(table_totals_row(tax_table), vec!["", "60", "6"]);
    let tax_formula = tax_table.columns[2]
        .totals_formula
        .as_ref()
        .expect("tax totals formula projects");
    assert_eq!(tax_formula.formula_text, "=SUM(SalesTable[Tax])");
    assert_eq!(tax_formula.formula_text_version, "v1");
    assert_eq!(
        tax_formula.formula_artifact_id,
        "formula:SalesTable.Totals.col_tax"
    );
    assert_eq!(
        tax_formula.bind_artifact_id.as_deref(),
        Some("bind:SalesTable.Totals.col_tax")
    );

    let amount_edit =
        skin.try_set_table_totals_formula("SalesTable", "col:amount", "=SUM([Amount])");
    assert!(amount_edit.accepted, "{:?}", amount_edit.error);
    assert_table_transaction(&amount_edit);
    let edited_state = skin.state();
    let edited_table = edited_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals formula edit");
    assert_eq!(table_totals_row(edited_table), vec!["", "60", "6"]);
    let amount_formula = edited_table.columns[1]
        .totals_formula
        .as_ref()
        .expect("amount totals formula projects");
    assert_eq!(amount_formula.formula_text, "=SUM([Amount])");
    assert_eq!(amount_formula.formula_text_version, "v2");
    assert_eq!(
        amount_formula.formula_artifact_id,
        "formula:SalesTable.Totals.Amount"
    );

    skin.edit_table_cell("SalesTable", "row:east", "col:amount", "25");
    let recalced_state = skin.state();
    let recalced_table = recalced_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals-driving body edit");
    assert_eq!(table_totals_row(recalced_table), vec!["", "65", "6.5"]);

    let clear_amount = skin.try_clear_table_totals_formula("SalesTable", "col:amount");
    assert!(clear_amount.accepted, "{:?}", clear_amount.error);
    assert_table_transaction(&clear_amount);
    let cleared_state = skin.state();
    let cleared_table = cleared_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals formula clear");
    assert_eq!(table_totals_row(cleared_table), vec!["", "", "6.5"]);
    assert!(cleared_table.columns[1].totals_formula.is_none());

    let missing_set =
        skin.try_set_table_totals_formula("SalesTable", "col:missing", "=SUM([Missing])");
    assert!(!missing_set.accepted, "{missing_set:?}");
    let missing_clear = skin.try_clear_table_totals_formula("SalesTable", "col:missing");
    assert!(!missing_clear.accepted, "{missing_clear:?}");
}

#[test]
fn programmable_skin_toggles_table_totals_row_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    skin.set_table_totals_formula("SalesTable", "col:tax", "=SUM(SalesTable[Tax])");
    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    assert!(before_table.totals_row_present);
    assert_eq!(table_totals_row(before_table), vec!["", "60", "6"]);

    let hide = skin.try_set_table_totals_row_visible("SalesTable", false);
    assert!(hide.accepted, "{:?}", hide.error);
    assert_table_transaction(&hide);
    let hidden_state = skin.state();
    let hidden_table = hidden_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals row hide");
    assert!(!hidden_table.totals_row_present);
    assert_eq!(
        hidden_table.columns[2]
            .totals_formula
            .as_ref()
            .map(|formula| formula.formula_text.as_str()),
        Some("=SUM(SalesTable[Tax])")
    );

    let show = skin.try_set_table_totals_row_visible("SalesTable", true);
    assert!(show.accepted, "{:?}", show.error);
    assert_table_transaction(&show);
    let shown_state = skin.state();
    let shown_table = shown_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after totals row show");
    assert!(shown_table.totals_row_present);
    assert_eq!(table_totals_row(shown_table), vec!["", "60", "6"]);

    let missing = skin.try_set_table_totals_row_visible("MissingTable", false);
    assert!(!missing.accepted, "{missing:?}");
}

#[test]
fn programmable_skin_toggles_table_header_row_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    assert!(before_table.header_row_present);
    assert_eq!(before_table.rows.len(), 3);
    assert_eq!(before_table.columns.len(), 3);
    let first_body_row = table_body_row(before_table, 0);

    let hide = skin.try_set_table_header_row_visible("SalesTable", false);
    assert!(hide.accepted, "{:?}", hide.error);
    assert_table_transaction(&hide);
    let hidden_state = skin.state();
    let hidden_table = hidden_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after header row hide");
    assert!(!hidden_table.header_row_present);
    assert_eq!(
        hidden_table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Region", "Amount", "Tax"]
    );
    assert_eq!(table_body_row(hidden_table, 0), first_body_row);

    skin.set_table_header_row_visible("SalesTable", true);
    let shown_state = skin.state();
    let shown_table = shown_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after header row show");
    assert!(shown_table.header_row_present);
    assert_eq!(table_body_row(shown_table, 0), first_body_row);

    let missing = skin.try_set_table_header_row_visible("MissingTable", false);
    assert!(!missing.accepted, "{missing:?}");
}

#[test]
fn programmable_skin_renames_table_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    let before_namespace_version = before_table.table_namespace_version.clone();
    let before_display_path = before_table.display_path.clone();
    let before_canonical_path = before_table.canonical_path.clone();
    let before_first_body_row = table_body_row(before_table, 0);
    assert_eq!(before_table.table_name, "SalesTable");
    assert!(before_display_path.ends_with("SalesTable"));
    assert!(before_canonical_path.ends_with("SalesTable"));

    let rename = skin.try_rename_table("SalesTable", "Revenue");
    assert!(rename.accepted, "{:?}", rename.error);
    assert!(
        rename
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:tables:")),
        "{rename:?}"
    );
    let renamed_state = skin.state();
    let renamed_table = renamed_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table remains keyed by node after logical table rename");
    assert_eq!(renamed_table.table_name, "Revenue");
    assert_eq!(renamed_table.display_path, before_display_path);
    assert_eq!(renamed_table.canonical_path, before_canonical_path);
    assert_ne!(
        renamed_table.table_namespace_version,
        before_namespace_version
    );
    assert_eq!(table_body_row(renamed_table, 0), before_first_body_row);

    skin.rename_table("SalesTable", " Revenue ");
    let trimmed_state = skin.state();
    let trimmed_table = trimmed_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after trimmed rename");
    assert_eq!(trimmed_table.table_name, "Revenue");

    let empty = skin.try_rename_table("SalesTable", " ");
    assert!(!empty.accepted, "{empty:?}");
    let missing = skin.try_rename_table("MissingTable", "Other");
    assert!(!missing.accepted, "{missing:?}");
}

#[test]
fn programmable_skin_renames_and_reorders_table_columns_from_outside_ir() {
    let harness = Harness::from_repo_fixture("tables");
    let skin = harness.driver.clone();
    skin.recalc();

    let before = skin.state();
    let before_table = before
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects");
    let before_column_identity_version = before_table.column_identity_version.clone();
    assert_eq!(
        before_table
            .columns
            .iter()
            .map(|column| (
                column.column_id.as_str(),
                column.name.as_str(),
                column.ordinal
            ))
            .collect::<Vec<_>>(),
        vec![
            ("col:region", "Region", 1),
            ("col:amount", "Amount", 2),
            ("col:tax", "Tax", 3)
        ]
    );

    let rename = skin.try_rename_table_column("SalesTable", "col:tax", "VAT");
    assert!(rename.accepted, "{:?}", rename.error);
    assert_table_transaction(&rename);
    let renamed_state = skin.state();
    let renamed_table = renamed_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after rename");
    assert_eq!(
        renamed_table
            .columns
            .iter()
            .map(|column| (
                column.column_id.as_str(),
                column.name.as_str(),
                column.ordinal
            ))
            .collect::<Vec<_>>(),
        vec![
            ("col:region", "Region", 1),
            ("col:amount", "Amount", 2),
            ("col:tax", "VAT", 3)
        ]
    );
    assert_eq!(table_body_row(renamed_table, 0), vec!["West", "10", "1"]);
    assert_ne!(
        before_column_identity_version,
        renamed_table.column_identity_version
    );

    let reorder = skin.try_reorder_table_column("SalesTable", "col:tax", 0);
    assert!(reorder.accepted, "{:?}", reorder.error);
    assert_table_transaction(&reorder);
    let reordered_state = skin.state();
    let reordered_table = reordered_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after reorder");
    assert_eq!(
        reordered_table
            .columns
            .iter()
            .map(|column| (
                column.column_id.as_str(),
                column.name.as_str(),
                column.ordinal
            ))
            .collect::<Vec<_>>(),
        vec![
            ("col:tax", "VAT", 1),
            ("col:region", "Region", 2),
            ("col:amount", "Amount", 3)
        ]
    );
    assert_eq!(table_body_row(reordered_table, 0), vec!["1", "West", "10"]);
    assert_eq!(table_body_row(reordered_table, 1), vec!["2", "East", "20"]);
    assert_eq!(table_totals_row(reordered_table), vec!["", "", "60"]);
    let TableColumnBodyProjection::Formula(tax_formula) = &reordered_table.columns[0].body else {
        panic!("VAT column should stay formula metadata after reorder");
    };
    assert_eq!(
        tax_formula.formula_artifact_id,
        "formula:SalesTable.Columns.Tax"
    );
    assert_eq!(tax_formula.formula_text, "=[@Amount] * 0.1");

    let bounded_reorder = skin.try_reorder_table_column("SalesTable", "col:tax", usize::MAX);
    assert!(bounded_reorder.accepted, "{:?}", bounded_reorder.error);
    assert_table_transaction(&bounded_reorder);
    let bounded_state = skin.state();
    let bounded_table = bounded_state
        .tables
        .get(&NodeId::new("SalesTable"))
        .expect("table projects after bounded reorder");
    assert_eq!(
        bounded_table
            .columns
            .iter()
            .map(|column| column.column_id.as_str())
            .collect::<Vec<_>>(),
        vec!["col:region", "col:amount", "col:tax"]
    );
    assert_eq!(table_body_row(bounded_table, 0), vec!["West", "10", "1"]);

    let missing_rename = skin.try_rename_table_column("SalesTable", "col:missing", "Missing");
    assert!(!missing_rename.accepted, "{missing_rename:?}");
    let missing_reorder = skin.try_reorder_table_column("SalesTable", "col:missing", 0);
    assert!(!missing_reorder.accepted, "{missing_reorder:?}");
}

#[test]
fn programmable_skin_projects_errors_and_diagnostics_after_rejected_recalc() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "0");
    let state = {
        let mut session = harness.session.lock().unwrap();
        session
            .edit_formula(&NodeId::new("Root.A"), "=Missing+1")
            .unwrap();
        let _ = session.recalculate();
        session.workspace_state().unwrap()
    };

    assert!(matches!(
        state.last_run.as_ref().map(|run| run.run_state),
        Some(CalcRunStateProjection::Rejected)
    ));
    assert!(!state.diagnostics.is_empty());
    let formula_node = state.node(&NodeId::new("Root.A")).unwrap();
    assert!(formula_node.binding_diagnostics.iter().any(|diagnostic| {
        diagnostic.node == NodeId::new("Root.A")
            && diagnostic.node_key == formula_node.key
            && diagnostic.message == "unresolved identifier 'Missing'"
            && diagnostic.span.start_utf8 == 1
            && diagnostic.span.end_utf8 == 8
    }));
    let run = state.last_run.as_ref().expect("rejected run projects");
    assert!(run.binding_diagnostics.iter().any(|diagnostic| {
        diagnostic.node == NodeId::new("Root.A")
            && diagnostic.node_key == formula_node.key
            && diagnostic.message == "unresolved identifier 'Missing'"
    }));
    let detail = state
        .active_node_detail(&dnatreecalc_skin_framework::SelectionState::with_primary(
            Some(NodeId::new("Root.A")),
        ))
        .expect("active detail projects selected formula");
    assert_eq!(detail.binding_diagnostics, formula_node.binding_diagnostics);
    assert!(matches!(
        &formula_node.computed_value,
        NodeValueProjection::Error(_)
    ));
}

fn table_body_row(
    table: &dnatreecalc_skin_framework::TableProjection,
    row_index: usize,
) -> Vec<String> {
    table
        .cells
        .as_ref()
        .expect("table cell values project")
        .body_rows[row_index]
        .iter()
        .map(|cell| {
            cell.as_ref()
                .map(|cell| cell.value.display_text())
                .unwrap_or_default()
        })
        .collect()
}

fn constant_table_fixture() -> WorkspaceFixture {
    WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "constant-table-duplicate".to_string(),
        description: Some("constant-only table duplicate Skin IR fixture".to_string()),
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: "InputTable".to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(TableNodeFixture {
                table_id: "tree-table:input".to_string(),
                display_path: None,
                canonical_path: None,
                table_namespace_version: "table-namespace:input:v1".to_string(),
                row_membership_version: "table-rows:input:membership:v1".to_string(),
                row_order_version: "table-rows:input:order:v1".to_string(),
                column_identity_version: "table-columns:input:v1".to_string(),
                identity_policy: TableIdentityPolicyFixture {
                    rename_preserves_table_id: true,
                    move_preserves_table_id: true,
                    delete_releases_table_id: true,
                    note: "constant-only test table".to_string(),
                },
                header: TableSectionFixture { present: true },
                totals: TableSectionFixture { present: false },
                rows: vec![
                    TableRowFixture {
                        row_id: "row:one".to_string(),
                        ordinal: 1,
                    },
                    TableRowFixture {
                        row_id: "row:two".to_string(),
                        ordinal: 2,
                    },
                ],
                columns: vec![
                    TableColumnFixture {
                        column_id: "col:amount".to_string(),
                        name: "Amount".to_string(),
                        ordinal: 1,
                        body: TableColumnBodyFixture {
                            kind: TableColumnBodyKind::ConstantCells,
                            constants: vec![
                                TableCellFixture {
                                    row_id: "row:one".to_string(),
                                    value: "5".to_string(),
                                },
                                TableCellFixture {
                                    row_id: "row:two".to_string(),
                                    value: "15".to_string(),
                                },
                            ],
                            formula: None,
                        },
                        totals_formula: None,
                    },
                    TableColumnFixture {
                        column_id: "col:rate".to_string(),
                        name: "Rate".to_string(),
                        ordinal: 2,
                        body: TableColumnBodyFixture {
                            kind: TableColumnBodyKind::ConstantCells,
                            constants: vec![
                                TableCellFixture {
                                    row_id: "row:one".to_string(),
                                    value: "1".to_string(),
                                },
                                TableCellFixture {
                                    row_id: "row:two".to_string(),
                                    value: "2".to_string(),
                                },
                            ],
                            formula: None,
                        },
                        totals_formula: None,
                    },
                ],
            }),
        }],
    }
}

fn nested_constant_table_fixture() -> WorkspaceFixture {
    let mut fixture = constant_table_fixture();
    fixture.workspace_id = "nested-constant-table-duplicate".to_string();
    fixture.nodes.insert(
        0,
        WorkspaceNodeFixture {
            node_id: "Root".to_string(),
            formula: String::new(),
            is_meta: false,
            table: None,
        },
    );
    fixture.nodes[1].node_id = "Root.InputTable".to_string();
    fixture
}

fn assert_table_transaction(receipt: &dnatreecalc_skin_framework::IntentReceipt) {
    assert!(
        receipt
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:tables:")),
        "{receipt:?}"
    );
}

fn assert_any_transaction(receipt: &dnatreecalc_skin_framework::IntentReceipt) {
    assert!(
        receipt
            .transaction_id
            .as_deref()
            .is_some_and(|id| id.starts_with("transaction:")),
        "{receipt:?}"
    );
}

fn formula_reference_inserted_delta(
    receipt: &dnatreecalc_skin_framework::IntentReceipt,
) -> &dnatreecalc_skin_framework::FormulaReferenceInsertionProjection {
    receipt
        .delta
        .changes
        .iter()
        .find_map(|change| match change {
            WorkspaceDeltaChange::FormulaReferenceInserted(insertion) => Some(insertion),
            _ => None,
        })
        .expect("formula reference insertion receipt carries authored insertion delta")
}

fn table_totals_row(table: &dnatreecalc_skin_framework::TableProjection) -> Vec<String> {
    table
        .cells
        .as_ref()
        .expect("table cell values project")
        .totals_row
        .iter()
        .map(|cell| {
            cell.as_ref()
                .map(|cell| cell.value.display_text())
                .unwrap_or_default()
        })
        .collect()
}

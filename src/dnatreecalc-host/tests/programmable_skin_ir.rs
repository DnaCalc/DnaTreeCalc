mod support;

use dnatreecalc_skin_framework::{
    CalcRunStateProjection, NodeContentKind, NodeId, NodeValueProjection, WorkspaceRecalcMode,
};

use support::programmable::{Harness, revision_fingerprint};

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
    let run = state.last_run.expect("recalc projects last run");
    assert!(matches!(
        run.run_state,
        CalcRunStateProjection::Published | CalcRunStateProjection::VerifiedClean
    ));
    assert!(!run.evaluation_order.is_empty());
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
    assert_eq!(
        array_node.computed_value,
        NodeValueProjection::Array(vec![
            vec!["1".to_string()],
            vec!["2".to_string()],
            vec!["3".to_string()]
        ])
    );
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
    let NodeValueProjection::Array(rows) = &grid.computed_value else {
        panic!(
            "SEQUENCE should project as an array, got {:?}",
            grid.computed_value
        );
    };
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|row| row.len() == 5));
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[4][4], "25");
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
    let NodeValueProjection::Array(rows) = &random.computed_value else {
        panic!(
            "RANDARRAY should project as an array, got {:?}",
            random.computed_value
        );
    };
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|row| row.len() == 5));
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

    skin.reorder("Root.Z", 0);
    skin.assert_children("Root", &["Root.Z", "Root.X", "Root.Y"]);

    skin.rename("Root.X", "Renamed");
    assert!(skin.state().node(&NodeId::new("Root.Renamed")).is_some());
    skin.assert_children("Root", &["Root.Z", "Root.Renamed", "Root.Y"]);

    skin.move_node("Root.Y", None, None);
    let state = skin.state();
    assert!(state.node(&NodeId::new("Y")).is_some());
    assert!(state.root_paths.iter().any(|node| node.as_str() == "Y"));

    skin.delete("Root.Z");
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
    let unknown_edit = skin.try_edit("Root.Missing", "3");
    assert!(!unknown_edit.accepted, "{unknown_edit:?}");
    let unknown_delete = skin.try_delete("Root.Missing");
    assert!(!unknown_delete.accepted, "{unknown_delete:?}");

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
    assert!(table.header_row_present);
    assert!(!table.dependency_inventory_summary.is_empty());
}

#[test]
fn programmable_skin_projects_errors_and_diagnostics_after_rejected_recalc() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    skin.add_node(None, "Root", "");
    skin.add_node(Some("Root"), "A", "=Missing+1");

    let state = skin.state();
    assert!(matches!(
        state.last_run.as_ref().map(|run| run.run_state),
        Some(CalcRunStateProjection::Rejected)
    ));
    assert!(!state.diagnostics.is_empty());
    assert!(matches!(
        &state.node(&NodeId::new("Root.A")).unwrap().computed_value,
        NodeValueProjection::Error(_)
    ));
}

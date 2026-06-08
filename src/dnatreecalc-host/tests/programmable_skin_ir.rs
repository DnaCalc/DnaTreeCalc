mod support;

use dnatreecalc_skin_framework::{
    CalcRunStateProjection, NodeContentKind, NodeId, NodeValueProjection,
    ReferenceTargetProjection, RuntimeEffectFamilyProjection, RuntimeOverlayKindProjection,
    TableColumnBodyProjection, TreeReferenceCollectionFamilyProjection, WorkspaceDeltaChange,
    WorkspaceRecalcMode,
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
fn programmable_skin_receipts_carry_projection_deltas() {
    let harness = Harness::empty();
    let skin = harness.driver.clone();

    let add_root = skin.try_add_node(None, "Root", "");
    assert!(add_root.accepted, "{:?}", add_root.error);
    assert_eq!(add_root.delta.from_seq, 0);
    assert_eq!(add_root.delta.to_seq, 1);
    assert!(add_root.produced_revision.is_some());
    assert!(add_root
        .delta
        .changes
        .iter()
        .any(|change| matches!(change, WorkspaceDeltaChange::Structural(delta) if !delta.added.is_empty())));
    assert_eq!(skin.state().projection_seq, add_root.delta.to_seq);

    skin.add_node(Some("Root"), "A", "3");
    skin.add_node(Some("Root"), "B", "=A+1");
    let edit = skin.try_edit("Root.A", "4");
    assert!(edit.accepted, "{:?}", edit.error);
    assert_eq!(edit.delta.from_seq + 1, edit.delta.to_seq);
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
    assert_eq!(select.delta.from_seq, before_select_seq);
    assert_eq!(select.delta.to_seq, before_select_seq);
    assert!(select.delta.changes.is_empty());

    let new_workspace = skin.try_new_workspace();
    assert!(new_workspace.accepted, "{:?}", new_workspace.error);
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
    assert!(!trace.formula_stable_id.is_empty());
    assert!(!trace.template_selection.plan_template_key.is_empty());
    assert!(!trace.hole_bindings.is_empty());
    assert!(!trace.sub_invocation_tree.is_empty());
    let root_call = &trace.sub_invocation_tree[0];
    assert!(!root_call.function_id.is_empty());
    assert!(!root_call.function_name.is_empty());
    assert_eq!(root_call.kernel_returned_value.as_deref(), Some("7"));
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
    assert!(!table.dependency_inventory_summary.is_empty());
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

    let formula_input = skin.try_add_table_row("SalesTable", "row:formula", &[("col:tax", "9")]);
    assert!(!formula_input.accepted, "{formula_input:?}");

    let duplicate_input = skin.try_add_table_row(
        "SalesTable",
        "row:duplicate-input",
        &[("col:amount", "1"), ("col:amount", "2")],
    );
    assert!(!duplicate_input.accepted, "{duplicate_input:?}");

    let delete = skin.try_delete_table_row("SalesTable", "row:east");
    assert!(delete.accepted, "{:?}", delete.error);
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

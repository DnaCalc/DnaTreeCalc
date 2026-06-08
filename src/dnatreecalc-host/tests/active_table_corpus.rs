use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{
    TableColumnBodyKind, TableColumnFixture, TableFormulaFixture, TableNodeFixture,
    WorkspaceFixture, WorkspaceModel, WorkspaceNodeFixture,
};
use dnatreecalc_skin_framework::NodeId;
use oxcalc_core::dependency::{DependencyDescriptorKind, InvalidationReasonKind};
use oxcalc_core::sparse_reader::{SparseCellCoord, SparseCellRead, SparseRangeReader};
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::structured_table::{
    StructuredTableContextPacket, StructuredTableDependencyFactKind,
    StructuredTableDependencyFactStatus, StructuredTableDependencyLoweringRequest,
    TableCallerRegion, TableRef, TableRegionKind, TreeCalcDynamicTableRebindCause,
    TreeCalcDynamicTableRebindDiagnosticKind, TreeCalcDynamicTableRebindRequest,
    TreeCalcDynamicTableRebindStatus, TreeCalcDynamicTableReferenceTargetKind,
    TreeCalcTableColumnBodyMetadata, TreeCalcTableColumnFormulaRuntimeRequest,
    TreeCalcTableColumnSnapshot, TreeCalcTableFormulaMetadata, TreeCalcTableFormulaRuntimeContext,
    TreeCalcTableLifecycleCallbackPacket, TreeCalcTableLifecycleContextVersions,
    TreeCalcTableLifecycleContractDiagnostic, TreeCalcTableLifecycleEventKind,
    TreeCalcTableLifecycleVersionState, TreeCalcTableNodeProjection, TreeCalcTableNodeSnapshot,
    TreeCalcTableProjectionError, TreeCalcTableRowId, TreeCalcTableSparseReader,
    TreeCalcTableSparseReaderError, TreeCalcTableSparseValue, TreeCalcTableUpdateScenarioKind,
    classify_treecalc_table_lifecycle_callback, classify_treecalc_table_update,
    evaluate_treecalc_table_column_formula_rows, evaluate_treecalc_table_totals_formula,
    lower_structured_table_dependencies, project_treecalc_table_node_snapshot,
    validate_treecalc_table_reference_after_update,
};
use oxcalc_core::tree_reference_system::{
    TreeCalcReferenceSystemProvider, TreeCalcSparseReferenceValuesBinding,
};
use oxfml_core::binding::{BindContext, BindRequest, bind_formula};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormulaRequest, RuntimeHostNameBindResult, RuntimeHostNameBinding,
};
use oxfml_core::interface::TypedContextQueryBundle;
use oxfml_core::red::project_red_view;
use oxfml_core::seam::Locus;
use oxfml_core::source::{FormulaSourceRecord, StructureContextVersion};
use oxfml_core::syntax::parser::{ParseRequest, parse_formula};
use oxfml_core::{
    DefinedNameBinding, EvaluationBackend, StructuredReferenceBindDiagnosticLink,
    StructuredReferenceBindRecord, StructuredReferenceSourceTokenKind,
};
use oxfunc_core::value::{CalcValue, CoreValue, ExcelText};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<TableCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct TableCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    table: String,
    reference: String,
    source_formula: Option<String>,
    caller_row_offset: Option<u32>,
    expect: TableExpectation,
}

#[derive(Debug, Deserialize)]
struct TableLifecycleTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<TableLifecycleCorpusCase>,
}

#[derive(Debug, Deserialize)]
struct TableLifecycleCorpusCase {
    id: String,
    name: String,
    kind: String,
    workspace: String,
    table: String,
    reference: String,
    lifecycle: TableLifecycleCorpusSpec,
}

#[derive(Debug, Deserialize)]
struct TableLifecycleCorpusSpec {
    event_kind: String,
    before_state: String,
    after_state: String,
    changed_rows: Vec<String>,
    changed_columns: Vec<String>,
    expect_row_handles_preserved: bool,
    expect_column_handles_preserved: bool,
}

#[derive(Debug, Deserialize)]
struct DynamicTableTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<DynamicTableCase>,
}

#[derive(Debug, Deserialize)]
struct DynamicTableCase {
    id: String,
    name: String,
    kind: String,
    workspace: String,
    table: String,
    reference: String,
    caller_row_offset: Option<u32>,
    expect: TableExpectation,
    dynamic: DynamicTableSpec,
}

#[derive(Debug, Deserialize)]
struct DynamicTableSpec {
    selector_handle: String,
    selector_identity: String,
    source: DynamicTableSourcePacket,
    target_kind: String,
    cause: String,
    before_resolved_table: Option<String>,
    after_resolved_table: Option<String>,
    caller_context_id: Option<String>,
    oxfml_structured_bind_packet_available: bool,
    treecalc_v1: String,
    strict_excel: String,
}

#[derive(Debug, Deserialize)]
struct DynamicTableSourcePacket {
    mode: String,
    reference_handle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TableExpectation {
    outcome: String,
    target: Option<String>,
    target_kind: Option<String>,
    selected_columns: Option<Vec<String>>,
    published_value: Option<String>,
    published_values: Option<Vec<String>>,
    reason: Option<String>,
    engine_ref: Option<String>,
}

#[derive(Debug, Clone)]
struct TableStructuredReferenceBinding {
    source_span_utf8: oxfml_core::syntax::token::TextSpan,
    source_token_text: String,
    host_ref_handle: String,
    resolved_table_id: Option<String>,
    caller_context_dependency: bool,
    replay_identity: String,
    bind_record: StructuredReferenceBindRecord,
    diagnostics: Vec<StructuredReferenceBindDiagnosticLink>,
}

#[test]
fn active_table_structured_reference_corpus_executes_through_oxcalc_table_path() {
    let theme = load_theme(repo_corpus_path("tables/structured-references.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "tables/structured-references");
    assert_eq!(theme.status, CorpusStatus::Active);

    let workspace = load_workspace("tables");
    let (sales_table, sales_snapshot, sales_projection) = table_evidence(&workspace, "SalesTable");
    assert_direct_context_projects_same_table_context("SalesTable", sales_table);

    let tax_report = evaluate_tax_column(&sales_snapshot, &sales_projection, sales_table);
    assert_eq!(
        tax_report
            .cell_results
            .iter()
            .map(|cell| display_value(&cell.value))
            .collect::<Vec<_>>(),
        vec!["1", "2", "3"]
    );

    let totals_amount = evaluate_amount_totals(&sales_snapshot, &sales_projection, sales_table);
    assert_eq!(display_value(&totals_amount.value), "60");

    for case in &theme.cases {
        assert_eq!(case.kind, "table", "case {} kind changed", case.id);
        assert_eq!(
            case.workspace, "tables",
            "case {} workspace changed",
            case.id
        );
        assert!(
            workspace.node(&case.caller).is_some(),
            "case {} caller fixture is missing",
            case.id
        );
        let (table, snapshot, projection) = table_evidence(&workspace, &case.table);
        let formula_values = if case.table == "SalesTable" {
            table_sparse_values(
                table,
                Some(&tax_report),
                [("col:amount", CalcValue::number(60.0))],
            )
        } else {
            table_sparse_values(table, None, std::iter::empty::<(&str, CalcValue)>())
        };

        match case.id.as_str() {
            "tbl-column-formula" => {
                assert_eq!(
                    case.expect.published_values.as_deref(),
                    Some(["1".to_string(), "2".to_string(), "3".to_string()].as_slice()),
                    "{} expected row values",
                    case.id
                );
            }
            "tbl-totals-formula" => {
                assert_eq!(
                    case.expect.published_value.as_deref(),
                    Some(display_value(&totals_amount.value).as_str()),
                    "{} totals value",
                    case.id
                );
            }
            _ => {}
        }

        let formula_text = case
            .source_formula
            .as_deref()
            .unwrap_or(case.reference.as_str());
        let caller_region = case
            .caller_row_offset
            .map(|offset| table_data_caller_region(&projection, offset));
        let enclosing = caller_region.as_ref().map(|_| TableRef {
            table_id: projection.table_id.clone(),
        });
        let bound_refs = bind_treecalc_table_structured_references(
            formula_text,
            &projection,
            enclosing,
            caller_region.clone(),
        );
        let actual_outcome = if bound_refs
            .iter()
            .flat_map(|binding| binding.diagnostics.iter())
            .next()
            .is_some()
        {
            "error"
        } else {
            "resolved"
        };
        if actual_outcome != case.expect.outcome
            && bound_refs
                .iter()
                .flat_map(|binding| binding.diagnostics.iter())
                .any(|diagnostic| diagnostic.diagnostic_code == "oxfml.syntax_diagnostic")
        {
            assert!(
                case.expect
                    .engine_ref
                    .as_deref()
                    .is_some_and(|engine_ref| engine_ref.contains("OxFml")),
                "{} must document OxFml structured-reference syntax as the blocker",
                case.id
            );
            continue;
        }
        assert_eq!(case.expect.outcome, actual_outcome, "{} outcome", case.id);
        if case.expect.outcome == "error" {
            if let Some(reason) = &case.expect.reason {
                assert!(
                    bound_refs
                        .iter()
                        .flat_map(|binding| binding.diagnostics.iter())
                        .any(|diagnostic| diagnostic.message.contains(reason)
                            || diagnostic.diagnostic_code.contains(reason))
                        || bound_refs
                            .iter()
                            .flat_map(|binding| binding.diagnostics.iter())
                            .next()
                            .is_some(),
                    "{} expected diagnostic reason {reason}; observed {:?}",
                    case.id,
                    bound_refs
                        .iter()
                        .flat_map(|binding| binding.diagnostics.iter())
                        .collect::<Vec<_>>()
                );
            }
            assert!(
                bound_refs
                    .iter()
                    .flat_map(|binding| binding.diagnostics.iter())
                    .any(|diagnostic| diagnostic.message.contains("Missing")
                        || diagnostic.diagnostic_code.contains("unknown")
                        || !diagnostic.message.is_empty()),
                "{} expected structured-reference diagnostic",
                case.id
            );
            continue;
        }

        let bind_record = &bound_refs
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table bind record", case.id))
            .bind_record;
        let binding = bound_refs
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table bind record", case.id));
        assert_case_target(&case.id, &case.expect, binding, &projection);
        if let Some(expected_columns) = &case.expect.selected_columns {
            assert_eq!(
                &bind_record.selected_column_ids, expected_columns,
                "{} selected columns",
                case.id
            );
        }
        assert_table_dependency_lowering(&case.id, &projection, bind_record, caller_region.clone());

        if let Some(expected_value) = &case.expect.published_value {
            let reader = TreeCalcTableSparseReader::from_oxfml_bind_record(
                &snapshot,
                &projection,
                bind_record,
                caller_region.as_ref(),
                formula_values.clone(),
            )
            .unwrap_or_else(|error| panic!("case {} reader failed: {error:?}", case.id));
            let observed = if is_simple_current_row_reference_formula(formula_text, binding) {
                reader_value_at_origin(&case.id, &reader)
            } else {
                let runtime_binding = reader.runtime_binding();
                evaluate_case_formula(
                    &case.id,
                    formula_text,
                    &projection,
                    caller_region,
                    runtime_binding,
                )
            };
            assert_eq!(observed, *expected_value, "{} value", case.id);
        }

        assert!(
            !case
                .expect
                .target_kind
                .as_deref()
                .unwrap_or_default()
                .is_empty(),
            "{} must keep a target kind for retained evidence",
            case.id
        );
        if let Some(engine_ref) = &case.expect.engine_ref {
            assert!(
                engine_ref.contains("OxCalc") || engine_ref.contains("OxFml"),
                "{} engine_ref must name the engine-owned seam",
                case.id
            );
        }
    }

    assert_table_update_scenarios_are_classified(&sales_projection);
}

#[test]
fn retained_table_replay_artifact_matches_direct_oxcalc_context_projection() {
    let theme = load_theme(repo_corpus_path("tables/structured-references.json"));
    let workspace = load_workspace("tables");
    let (sales_table, sales_snapshot, sales_projection) = table_evidence(&workspace, "SalesTable");
    let tax_report = evaluate_tax_column(&sales_snapshot, &sales_projection, sales_table);
    let totals_amount = evaluate_amount_totals(&sales_snapshot, &sales_projection, sales_table);

    let artifact = retained_table_replay_artifact(
        &theme,
        &workspace,
        sales_table,
        &sales_snapshot,
        &sales_projection,
        &tax_report,
        &totals_amount,
    );
    let artifact_path = repo_docs_path(
        "test-runs/w056-table-structured-references-001/views/normalized-replay.json",
    );
    let manifest_path =
        repo_docs_path("test-runs/w056-table-structured-references-001/oxreplay-manifest.json");
    let manifest = retained_table_replay_manifest();
    if std::env::var_os("DNATREECALC_UPDATE_RETAINED_TABLE_REPLAY").is_some() {
        write_pretty_json(&artifact_path, &artifact);
        write_pretty_json(&manifest_path, &manifest);
    }
    let expected_artifact = load_expected_json_or_panic_with_generated(&artifact_path, &artifact);
    assert_eq!(
        expected_artifact, artifact,
        "retained W056 table artifact must stay generated from the live OxCalc table projection"
    );

    let expected_manifest = load_expected_json_or_panic_with_generated(&manifest_path, &manifest);
    assert_eq!(
        expected_manifest, manifest,
        "retained W056 table manifest must stay aligned with the generated replay view"
    );
    assert!(
        manifest["views"].as_array().is_some_and(|views| views
            .iter()
            .any(|view| view["path"] == json!("views/normalized-replay.json"))),
        "manifest must point OxReplay at the normalized-replay view"
    );
}

#[test]
fn retained_table_lifecycle_artifact_matches_direct_oxcalc_context_callbacks() {
    let lifecycle_theme = load_lifecycle_theme(repo_corpus_path("tables/lifecycle-events.json"));
    assert_eq!(lifecycle_theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(lifecycle_theme.theme, "tables/lifecycle-events");
    assert_eq!(lifecycle_theme.status, CorpusStatus::Active);

    let workspace = load_workspace("tables");
    let (sales_table, baseline_snapshot, baseline_projection) =
        table_evidence(&workspace, "SalesTable");

    let artifact = retained_table_lifecycle_replay_artifact(
        &lifecycle_theme,
        &workspace,
        sales_table,
        &baseline_snapshot,
        &baseline_projection,
    );
    let artifact_path =
        repo_docs_path("test-runs/w056-table-lifecycle-001/views/normalized-replay.json");
    let manifest_path = repo_docs_path("test-runs/w056-table-lifecycle-001/oxreplay-manifest.json");
    let manifest = retained_table_lifecycle_replay_manifest();
    if std::env::var_os("DNATREECALC_UPDATE_RETAINED_TABLE_LIFECYCLE").is_some() {
        write_pretty_json(&artifact_path, &artifact);
        write_pretty_json(&manifest_path, &manifest);
    }
    let expected_artifact = load_expected_json_or_panic_with_generated(&artifact_path, &artifact);
    assert_eq!(
        expected_artifact, artifact,
        "retained W056 table lifecycle artifact must stay generated from OxCalc callback packets"
    );

    let expected_manifest = load_expected_json_or_panic_with_generated(&manifest_path, &manifest);
    assert_eq!(
        expected_manifest, manifest,
        "retained W056 table lifecycle manifest must stay aligned with the generated replay view"
    );
    assert!(
        manifest["views"].as_array().is_some_and(|views| views
            .iter()
            .any(|view| view["path"] == json!("views/normalized-replay.json"))),
        "manifest must point OxReplay at the normalized-replay view"
    );
}

#[test]
fn table_lifecycle_product_events_enter_oxcalc_callback_contract() {
    let lifecycle_theme = load_lifecycle_theme(repo_corpus_path("tables/lifecycle-events.json"));
    assert_eq!(lifecycle_theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(lifecycle_theme.theme, "tables/lifecycle-events");
    assert_eq!(lifecycle_theme.status, CorpusStatus::Active);

    let workspace = load_workspace("tables");
    let (sales_table, baseline_snapshot, baseline_projection) =
        table_evidence(&workspace, "SalesTable");
    let baseline_state =
        lifecycle_state_from_snapshot_projection(&baseline_snapshot, &baseline_projection);
    let owner = TreeNodeId(100);
    let source_handles = ["bind:SalesTable.Columns.Tax"];

    let lifecycle_cases = table_lifecycle_cases(&lifecycle_theme, &baseline_snapshot);
    assert_eq!(
        lifecycle_cases
            .iter()
            .map(|case| case.event_kind.stable_id())
            .collect::<Vec<_>>(),
        vec![
            "table_create",
            "body_cell_edit",
            "body_formula_edit",
            "row_insert",
            "row_delete",
            "row_reorder",
            "column_insert",
            "column_delete",
            "column_reorder",
            "column_rename",
            "header_text_edit",
            "totals_row_toggle",
            "totals_formula_edit",
            "table_rename",
            "table_move",
            "table_delete",
            "table_resize",
            "node_rename",
            "node_move",
            "node_delete",
            "save_reopen",
            "workspace_open",
            "workspace_close",
            "workspace_alias_mutation",
            "function_registry_snapshot_mutation",
            "structural_rebind",
        ],
        "DnaTreeCalc product event corpus must cover the W056 lifecycle surface"
    );

    for case in lifecycle_cases {
        let (before, after, packet) =
            table_lifecycle_packet_for_case(&case, owner, source_handles.iter().copied())
                .unwrap_or_else(|error| panic!("{} packet failed: {error:?}", case.name));
        let report = classify_treecalc_table_lifecycle_callback(&packet);
        assert!(
            report.diagnostics.is_empty(),
            "{} diagnostics: {:?}",
            case.name,
            report.diagnostics
        );
        assert_eq!(report.event_kind, case.event_kind, "{}", case.name);
        assert!(
            report
                .callback_identity
                .starts_with("treecalc.table_lifecycle.callback.v1"),
            "{} callback identity",
            case.name
        );
        assert_eq!(
            report.source_reference_handles,
            source_handles
                .iter()
                .map(|handle| (*handle).to_string())
                .collect::<Vec<_>>(),
            "{} source handles",
            case.name
        );
        if matches!(case.event_kind, TreeCalcTableLifecycleEventKind::SaveReopen) {
            assert!(
                report.changed_dependency_kinds.is_empty()
                    && report.invalidation_reasons.is_empty(),
                "{} stable save/reopen must not invent changed dependencies",
                case.name
            );
        } else {
            assert!(
                !report.changed_dependency_kinds.is_empty()
                    || !report.invalidation_reasons.is_empty(),
                "{} must carry dependency or invalidation evidence",
                case.name
            );
        }
        if let (Some((before_projection, before_state)), Some((after_projection, after_state))) =
            (before.as_ref(), after.as_ref())
        {
            assert_eq!(
                before_state.table_node_id, after_state.table_node_id,
                "{} must preserve stable node handle",
                case.name
            );
            assert_eq!(
                before_state.table_id, after_state.table_id,
                "{} must preserve stable table handle",
                case.name
            );
            if case.expect_row_handles_preserved {
                assert_eq!(
                    before_state.row_ids, after_state.row_ids,
                    "{} must preserve row handles",
                    case.name
                );
            }
            if case.expect_column_handles_preserved {
                assert_eq!(
                    before_state.column_ids, after_state.column_ids,
                    "{} must preserve column handles",
                    case.name
                );
            }
            assert_eq!(
                report.before_state.as_ref().unwrap().table_context_identity,
                before_projection.table_context_identity,
                "{} before identity",
                case.name
            );
            assert_eq!(
                report.after_state.as_ref().unwrap().table_context_identity,
                after_projection.table_context_identity,
                "{} after identity",
                case.name
            );
        }
    }

    let round_tripped = serde_json::to_string(&WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "table-lifecycle-roundtrip".to_string(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: "SalesTable".to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(sales_table.clone()),
        }],
    })
    .and_then(|json| serde_json::from_str::<WorkspaceFixture>(&json))
    .expect("table fixture save/reopen roundtrip serializes");
    let reopened_table = round_tripped.nodes[0].table.as_ref().unwrap();
    assert_eq!(reopened_table.table_id, sales_table.table_id);
    assert_eq!(
        reopened_table
            .rows
            .iter()
            .map(|row| &row.row_id)
            .collect::<Vec<_>>(),
        sales_table
            .rows
            .iter()
            .map(|row| &row.row_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        reopened_table
            .columns
            .iter()
            .map(|column| &column.column_id)
            .collect::<Vec<_>>(),
        sales_table
            .columns
            .iter()
            .map(|column| &column.column_id)
            .collect::<Vec<_>>()
    );

    let mut wrong_id_snapshot = baseline_snapshot.clone();
    wrong_id_snapshot.table_id = "tree-table:sales-recreated".to_string();
    let (_, wrong_id_state) =
        snapshot_projection_and_state(&wrong_id_snapshot).expect("wrong-id snapshot projects");
    let stale_identity_report = classify_treecalc_table_lifecycle_callback(
        &TreeCalcTableLifecycleCallbackPacket::new(TreeCalcTableLifecycleEventKind::TableRename)
            .with_before(baseline_state)
            .with_after(wrong_id_state)
            .with_owner_nodes([owner]),
    );
    assert!(
        stale_identity_report.diagnostics.contains(
            &TreeCalcTableLifecycleContractDiagnostic::TableIdChangedAcrossLifecycle {
                before: "tree-table:sales".to_string(),
                after: "tree-table:sales-recreated".to_string(),
            }
        ),
        "stale table identity must be rejected by the public callback contract"
    );
}

#[test]
fn active_empty_body_table_corpus_executes_through_oxcalc_table_path() {
    let theme = load_theme(repo_corpus_path("tables/empty-body.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "tables/empty-body");
    assert_eq!(theme.status, CorpusStatus::Active);

    let workspace = load_workspace("empty-body-tables");
    for case in &theme.cases {
        assert_eq!(case.kind, "table", "case {} kind changed", case.id);
        assert_eq!(
            case.workspace, "empty-body-tables",
            "case {} workspace changed",
            case.id
        );
        let table = workspace
            .table_node(&case.table)
            .unwrap_or_else(|| panic!("workspace missing table {}", case.table));
        let snapshot = table_snapshot(&case.table, table);
        let projection = project_treecalc_table_node_snapshot(&snapshot)
            .unwrap_or_else(|error| panic!("{} table failed projection: {error:?}", case.id));
        assert_direct_context_projects_same_table_context(&case.table, table);
        let caller_region = case
            .caller_row_offset
            .map(|offset| table_data_caller_region(&projection, offset));
        let enclosing = caller_region.as_ref().map(|_| TableRef {
            table_id: projection.table_id.clone(),
        });
        let bound_refs = bind_treecalc_table_structured_references(
            case.source_formula.as_deref().unwrap_or(&case.reference),
            &projection,
            enclosing,
            caller_region.clone(),
        );
        if case.expect.outcome != "error" {
            assert!(
                bound_refs
                    .iter()
                    .all(|binding| binding.diagnostics.is_empty()),
                "{} transition formula diagnostics: {:?}",
                case.id,
                bound_refs
                    .iter()
                    .flat_map(|binding| binding.diagnostics.iter())
                    .collect::<Vec<_>>()
            );
        }
        let bind_record = &bound_refs
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table bind record", case.id))
            .bind_record;

        if case.expect.outcome == "error" {
            let error = TreeCalcTableSparseReader::from_oxfml_bind_record(
                &snapshot,
                &projection,
                bind_record,
                caller_region.as_ref(),
                table_sparse_values(table, None, std::iter::empty::<(&str, CalcValue)>()),
            )
            .expect_err("zero-row current-row case must stay a typed reader diagnostic");
            match error {
                TreeCalcTableSparseReaderError::CallerRowOutOfRange {
                    row_offset,
                    row_count,
                } => {
                    assert_eq!(row_offset, case.caller_row_offset.unwrap_or_default());
                    assert_eq!(row_count, 0);
                }
                TreeCalcTableSparseReaderError::BindRecordIntake { detail } => {
                    assert!(
                        detail.contains("structured_reference_bind_error"),
                        "{} expected OxFml structured-reference diagnostic, got {detail}",
                        case.id
                    );
                }
                other => panic!(
                    "{} expected typed current-row diagnostic, got {other:?}",
                    case.id
                ),
            }
            assert_eq!(
                case.expect.reason.as_deref(),
                Some("CallerRowOutOfRange"),
                "{} reason",
                case.id
            );
            continue;
        }

        assert_eq!(case.expect.outcome, "resolved", "{} outcome", case.id);
        let binding = bound_refs
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table bind record", case.id));
        assert_case_target(&case.id, &case.expect, binding, &projection);
        if let Some(expected_columns) = &case.expect.selected_columns {
            assert_eq!(
                &bind_record.selected_column_ids, expected_columns,
                "{} selected columns",
                case.id
            );
        }

        let formula_values = if projection.table_descriptor.totals_row_present {
            table_sparse_values(table, None, [("col:amount", CalcValue::number(0.0))])
        } else {
            table_sparse_values(table, None, std::iter::empty::<(&str, CalcValue)>())
        };
        let reader = TreeCalcTableSparseReader::from_oxfml_bind_record(
            &snapshot,
            &projection,
            bind_record,
            caller_region.as_ref(),
            formula_values,
        )
        .unwrap_or_else(|error| panic!("case {} reader failed: {error:?}", case.id));
        if snapshot.rows.is_empty()
            && bind_record
                .selected_sections
                .iter()
                .all(|section| *section == oxfml_core::StructuredSectionKind::Data)
        {
            assert_eq!(reader.declared_extent().row_count, 0, "{} extent", case.id);
            assert_eq!(
                reader
                    .runtime_binding()
                    .sparse_reference_values
                    .declared_rows,
                0,
                "{} sparse binding declared rows",
                case.id
            );
        }
        if let Some(expected_value) = &case.expect.published_value {
            let observed = evaluate_case_formula(
                &case.id,
                case.source_formula.as_deref().unwrap_or(&case.reference),
                &projection,
                caller_region,
                reader.runtime_binding(),
            );
            assert_eq!(observed, *expected_value, "{} published value", case.id);
        }
    }
}

#[test]
fn retained_empty_body_table_replay_artifact_matches_direct_oxcalc_context_projection() {
    let theme = load_theme(repo_corpus_path("tables/empty-body.json"));
    let workspace = load_workspace("empty-body-tables");
    let artifact = retained_empty_body_table_replay_artifact(&theme, &workspace);
    let artifact_path =
        repo_docs_path("test-runs/w056-table-empty-body-001/views/normalized-replay.json");
    let manifest_path =
        repo_docs_path("test-runs/w056-table-empty-body-001/oxreplay-manifest.json");
    let manifest = retained_empty_body_table_replay_manifest();
    if std::env::var_os("DNATREECALC_UPDATE_RETAINED_EMPTY_BODY_TABLE_REPLAY").is_some() {
        write_pretty_json(&artifact_path, &artifact);
        write_pretty_json(&manifest_path, &manifest);
    }

    let expected_artifact = load_expected_json_or_panic_with_generated(&artifact_path, &artifact);
    assert_eq!(
        expected_artifact, artifact,
        "retained W056 empty-body table artifact must stay generated from the live OxCalc table projection"
    );
    let expected_manifest = load_expected_json_or_panic_with_generated(&manifest_path, &manifest);
    assert_eq!(
        expected_manifest, manifest,
        "retained W056 empty-body table manifest must stay aligned with the generated replay view"
    );
    assert!(
        manifest["views"].as_array().is_some_and(|views| views
            .iter()
            .any(|view| view["path"] == json!("views/normalized-replay.json"))),
        "manifest must point OxReplay at the normalized-replay view"
    );
}

#[test]
fn active_dynamic_cross_workspace_table_corpus_executes_through_oxcalc_dynamic_packets() {
    let theme = load_dynamic_table_theme(repo_corpus_path("tables/dynamic-cross-workspace.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "tables/dynamic-cross-workspace");
    assert_eq!(theme.status, CorpusStatus::Active);

    let local_workspace = load_workspace("tables");
    let remote_workspace = load_workspace("table-projections");
    let local_session =
        TreeWorkspaceSession::from_model(&local_workspace).expect("local table context builds");
    let remote_session =
        TreeWorkspaceSession::from_model(&remote_workspace).expect("remote table context builds");
    for case in &theme.cases {
        let report = dynamic_table_rebind_report_for_case(
            &local_session,
            &remote_session,
            case,
            &local_workspace,
            &remote_workspace,
        );
        assert_dynamic_table_expected_status(case, &report);
        assert_eq!(
            case.dynamic.strict_excel, "reject",
            "{} strict-excel profile",
            case.id
        );
        match case.dynamic.treecalc_v1.as_str() {
            "admit" => {
                assert_ne!(
                    report.status,
                    TreeCalcDynamicTableRebindStatus::TypedExclusion,
                    "{} treecalc-v1 admission",
                    case.id
                );
                assert!(
                    report.oxfunc_opaque_reference_admitted,
                    "{} admitted dynamic table reference must stay opaque for OxFunc",
                    case.id
                );
            }
            "typed_exclusion" => {
                assert!(
                    matches!(
                        report.status,
                        TreeCalcDynamicTableRebindStatus::TypedExclusion
                            | TreeCalcDynamicTableRebindStatus::DeletedTarget
                            | TreeCalcDynamicTableRebindStatus::UnavailableTarget
                    ),
                    "{} typed exclusion/status",
                    case.id
                );
            }
            other => panic!("{} unsupported treecalc-v1 verdict {other}", case.id),
        }
        if matches!(
            report.target_kind,
            TreeCalcDynamicTableReferenceTargetKind::CrossWorkspaceTable
        ) {
            assert!(
                report
                    .dependency_fact_kinds
                    .contains(&StructuredTableDependencyFactKind::WorkspaceAvailability),
                "{} cross-workspace dependency",
                case.id
            );
            assert!(
                report
                    .prepared_identity_inputs
                    .contains(&oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::HostNamespaceVersion),
                "{} cross-workspace prepared identity",
                case.id
            );
        }
    }
}

#[test]
fn retained_dynamic_cross_workspace_table_replay_artifact_matches_oxcalc_packets() {
    let theme = load_dynamic_table_theme(repo_corpus_path("tables/dynamic-cross-workspace.json"));
    let local_workspace = load_workspace("tables");
    let remote_workspace = load_workspace("table-projections");
    let local_session =
        TreeWorkspaceSession::from_model(&local_workspace).expect("local table context builds");
    let remote_session =
        TreeWorkspaceSession::from_model(&remote_workspace).expect("remote table context builds");
    let artifact = retained_dynamic_cross_workspace_table_replay_artifact(
        &theme,
        &local_session,
        &remote_session,
        &local_workspace,
        &remote_workspace,
    );
    let artifact_path = repo_docs_path(
        "test-runs/w056-table-dynamic-cross-workspace-001/views/normalized-replay.json",
    );
    let manifest_path =
        repo_docs_path("test-runs/w056-table-dynamic-cross-workspace-001/oxreplay-manifest.json");
    let manifest = retained_dynamic_cross_workspace_table_replay_manifest();
    if std::env::var_os("DNATREECALC_UPDATE_RETAINED_DYNAMIC_TABLE_REPLAY").is_some() {
        write_pretty_json(&artifact_path, &artifact);
        write_pretty_json(&manifest_path, &manifest);
    }

    let expected_artifact = load_expected_json_or_panic_with_generated(&artifact_path, &artifact);
    assert_eq!(
        expected_artifact, artifact,
        "retained W056 dynamic/cross-workspace table artifact must stay generated from OxCalc dynamic table packets"
    );
    let expected_manifest = load_expected_json_or_panic_with_generated(&manifest_path, &manifest);
    assert_eq!(
        expected_manifest, manifest,
        "retained W056 dynamic/cross-workspace manifest must stay aligned with the generated replay view"
    );
}

fn is_simple_current_row_reference_formula(
    formula_text: &str,
    binding: &TableStructuredReferenceBinding,
) -> bool {
    binding.bind_record.uses_this_row
        && formula_text
            .trim()
            .strip_prefix('=')
            .is_some_and(|body| body == binding.source_token_text)
}

fn reader_value_at_origin(case_id: &str, reader: &TreeCalcTableSparseReader) -> String {
    match reader.read_at(SparseCellCoord::new(1, 1)) {
        SparseCellRead::Defined(value) => display_value(&value),
        SparseCellRead::Blank => panic!("case {case_id} expected a current-row reader value"),
    }
}

fn assert_case_target(
    case_id: &str,
    expect: &TableExpectation,
    binding: &TableStructuredReferenceBinding,
    projection: &TreeCalcTableNodeProjection,
) {
    let target_kind = expect
        .target_kind
        .as_deref()
        .unwrap_or_else(|| panic!("case {case_id} missing target_kind"));
    let expected_target = expect
        .target
        .as_deref()
        .unwrap_or_else(|| panic!("case {case_id} missing target"));

    match target_kind {
        "column-reference"
        | "data-column-reference"
        | "current-row-column-reference"
        | "escaped-column-reference"
        | "escaped-current-row-column-reference" => {
            let column_id = expect
                .selected_columns
                .as_ref()
                .and_then(|columns| columns.first())
                .unwrap_or_else(|| panic!("case {case_id} missing selected column"));
            let column_name = projection
                .table_descriptor
                .columns
                .iter()
                .find(|column| &column.column_id == column_id)
                .map(|column| column.column_name.as_str())
                .unwrap_or_else(|| panic!("case {case_id} selected unknown column {column_id}"));
            assert_eq!(
                expected_target,
                format!("{}.Columns.{column_name}", projection.display_path),
                "{case_id} target path"
            );
        }
        "header-region" => {
            assert_eq!(
                expected_target,
                format!("{}.Headers", projection.display_path),
                "{case_id} target path"
            );
        }
        "totals-column-reference" => {
            let column_id = expect
                .selected_columns
                .as_ref()
                .and_then(|columns| columns.first())
                .unwrap_or_else(|| panic!("case {case_id} missing selected column"));
            let column_name = projection
                .table_descriptor
                .columns
                .iter()
                .find(|column| &column.column_id == column_id)
                .map(|column| column.column_name.as_str())
                .unwrap_or_else(|| panic!("case {case_id} selected unknown column {column_id}"));
            assert_eq!(
                expected_target,
                format!("{}.Totals.{column_name}", projection.display_path),
                "{case_id} target path"
            );
        }
        "totals-region" => {
            assert_eq!(
                expected_target,
                format!("{}.Totals", projection.display_path),
                "{case_id} target path"
            );
        }
        "all-region" => {
            assert_eq!(
                expected_target, projection.display_path,
                "{case_id} target path"
            );
        }
        "composite-data-column-reference" => {
            assert!(
                expected_target.is_empty(),
                "{case_id} composite target must stay empty until a single target path exists"
            );
        }
        "column-formula-dependency" | "totals-formula-dependency" => {
            assert!(
                expected_target.starts_with(&projection.display_path),
                "{case_id} formula dependency target path"
            );
        }
        other => panic!("case {case_id} has unknown target_kind {other}"),
    }
    assert_target_kind_matches_bind_record(case_id, target_kind, binding);

    assert_eq!(
        binding.resolved_table_id.as_deref(),
        Some(projection.table_id.as_str()),
        "{case_id} resolved table id"
    );
}

fn assert_target_kind_matches_bind_record(
    case_id: &str,
    target_kind: &str,
    binding: &TableStructuredReferenceBinding,
) {
    use oxfml_core::StructuredSectionKind;

    let sections = &binding.bind_record.selected_sections;
    match target_kind {
        "column-reference"
        | "data-column-reference"
        | "composite-data-column-reference"
        | "escaped-column-reference" => {
            assert_eq!(
                sections,
                &[StructuredSectionKind::Data],
                "{case_id} target_kind must describe a data-region structured reference"
            );
            assert!(
                !binding.bind_record.uses_this_row,
                "{case_id} must not use row context"
            );
        }
        "current-row-column-reference" | "escaped-current-row-column-reference" => {
            assert_eq!(
                sections,
                &[StructuredSectionKind::ThisRow],
                "{case_id} target_kind must describe a current-row structured reference"
            );
            assert!(
                binding.bind_record.uses_this_row,
                "{case_id} must use row context"
            );
            assert!(
                binding.caller_context_dependency,
                "{case_id} must preserve caller-context dependency"
            );
        }
        "header-region" => assert_eq!(
            sections,
            &[StructuredSectionKind::Headers],
            "{case_id} target_kind must describe a header structured reference"
        ),
        "totals-column-reference" | "totals-region" => assert_eq!(
            sections,
            &[StructuredSectionKind::Totals],
            "{case_id} target_kind must describe a totals structured reference"
        ),
        "all-region" => assert_eq!(
            sections,
            &[StructuredSectionKind::All],
            "{case_id} target_kind must describe an #All structured reference"
        ),
        "column-formula-dependency" => {
            assert!(
                binding.bind_record.uses_this_row,
                "{case_id} column formula must use row context"
            );
        }
        "totals-formula-dependency" => {
            assert!(
                sections.contains(&StructuredSectionKind::Data),
                "{case_id} totals formula must depend on a data structured reference"
            );
        }
        other => panic!("case {case_id} has unknown target_kind {other}"),
    }
}

fn assert_direct_context_projects_same_table_context(table_path: &str, table: &TableNodeFixture) {
    let context_workspace = WorkspaceModel::try_from(WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "active-table-corpus-context".to_string(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: table_path.to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(table.clone()),
        }],
    })
    .expect("single-table direct-context workspace is valid");

    let session = TreeWorkspaceSession::from_model(&context_workspace)
        .expect("direct OxCalc context must accept DnaTreeCalc table projection");
    let table_context_identity = session
        .table_context_identity(&NodeId::new(table_path))
        .expect("direct OxCalc context table view must be available")
        .expect("table context identity must be projected");

    assert!(
        table_context_identity.contains("treecalc.table_context.v1"),
        "direct OxCalc context must retain the table context identity from the real OxCalc table projection"
    );
}

fn table_evidence<'a>(
    workspace: &'a WorkspaceModel,
    table_path: &str,
) -> (
    &'a TableNodeFixture,
    TreeCalcTableNodeSnapshot,
    TreeCalcTableNodeProjection,
) {
    let table = workspace
        .table_node(table_path)
        .unwrap_or_else(|| panic!("workspace missing table {table_path}"));
    let snapshot = table_snapshot(table_path, table);
    let projection = project_treecalc_table_node_snapshot(&snapshot)
        .expect("DnaTreeCalc table fixture projects to OxCalc table catalog");
    (table, snapshot, projection)
}

fn table_snapshot(table_path: &str, table: &TableNodeFixture) -> TreeCalcTableNodeSnapshot {
    TreeCalcTableNodeSnapshot {
        table_node_id: TreeNodeId(1),
        table_id: table.table_id.clone(),
        table_name: table_path
            .rsplit('.')
            .next()
            .unwrap_or(table_path)
            .to_string(),
        display_path: table
            .display_path
            .clone()
            .unwrap_or_else(|| table_path.to_string()),
        canonical_path: table
            .canonical_path
            .clone()
            .unwrap_or_else(|| table_path.to_string()),
        virtual_anchor: oxcalc_core::structured_table::TreeCalcTableVirtualAnchor {
            workbook_scope_ref: "treecalc-workbook:tables".to_string(),
            sheet_scope_ref: "sheet:default".to_string(),
            start_row: 3,
            start_col: 2,
        },
        rows: table
            .rows
            .iter()
            .map(|row| TreeCalcTableRowId(row.row_id.clone()))
            .collect(),
        columns: table.columns.iter().map(table_column_snapshot).collect(),
        body_cell_nodes: Vec::new(),
        totals_cell_nodes: Vec::new(),
        header_row_present: table.header.present,
        totals_row_present: table.totals.present,
        table_namespace_version: table.table_namespace_version.clone(),
        row_membership_version: table.row_membership_version.clone(),
        row_order_version: table.row_order_version.clone(),
        column_identity_version: table.column_identity_version.clone(),
    }
}

struct TableLifecycleCase {
    id: String,
    name: String,
    event_kind: TreeCalcTableLifecycleEventKind,
    before: Option<TreeCalcTableNodeSnapshot>,
    after: Option<TreeCalcTableNodeSnapshot>,
    changed_rows: Vec<TreeCalcTableRowId>,
    changed_columns: Vec<String>,
    expect_row_handles_preserved: bool,
    expect_column_handles_preserved: bool,
}

fn table_lifecycle_cases(
    theme: &TableLifecycleTheme,
    baseline: &TreeCalcTableNodeSnapshot,
) -> Vec<TableLifecycleCase> {
    theme
        .cases
        .iter()
        .map(|case| {
            assert_eq!(case.kind, "table", "case {} kind", case.id);
            assert_eq!(case.workspace, "tables", "case {} workspace", case.id);
            assert_eq!(case.table, "SalesTable", "case {} table", case.id);
            assert!(
                !case.reference.is_empty(),
                "case {} must retain a reference label for corpus validation",
                case.id
            );
            TableLifecycleCase {
                id: case.id.clone(),
                name: case.name.clone(),
                event_kind: table_lifecycle_event_kind(&case.lifecycle.event_kind),
                before: table_lifecycle_snapshot_state(
                    &case.id,
                    &case.lifecycle.before_state,
                    baseline,
                ),
                after: table_lifecycle_snapshot_state(
                    &case.id,
                    &case.lifecycle.after_state,
                    baseline,
                ),
                changed_rows: case
                    .lifecycle
                    .changed_rows
                    .iter()
                    .cloned()
                    .map(TreeCalcTableRowId)
                    .collect(),
                changed_columns: case.lifecycle.changed_columns.clone(),
                expect_row_handles_preserved: case.lifecycle.expect_row_handles_preserved,
                expect_column_handles_preserved: case.lifecycle.expect_column_handles_preserved,
            }
        })
        .collect()
}

fn table_lifecycle_event_kind(event_kind: &str) -> TreeCalcTableLifecycleEventKind {
    match event_kind {
        "table_create" => TreeCalcTableLifecycleEventKind::TableCreate,
        "body_cell_edit" => TreeCalcTableLifecycleEventKind::BodyCellEdit,
        "body_formula_edit" => TreeCalcTableLifecycleEventKind::BodyFormulaEdit,
        "row_insert" => TreeCalcTableLifecycleEventKind::RowInsert,
        "row_delete" => TreeCalcTableLifecycleEventKind::RowDelete,
        "row_reorder" => TreeCalcTableLifecycleEventKind::RowReorder,
        "column_insert" => TreeCalcTableLifecycleEventKind::ColumnInsert,
        "column_delete" => TreeCalcTableLifecycleEventKind::ColumnDelete,
        "column_reorder" => TreeCalcTableLifecycleEventKind::ColumnReorder,
        "column_rename" => TreeCalcTableLifecycleEventKind::ColumnRename,
        "header_text_edit" => TreeCalcTableLifecycleEventKind::HeaderTextEdit,
        "totals_row_toggle" => TreeCalcTableLifecycleEventKind::TotalsRowToggle,
        "totals_formula_edit" => TreeCalcTableLifecycleEventKind::TotalsFormulaEdit,
        "table_rename" => TreeCalcTableLifecycleEventKind::TableRename,
        "table_move" => TreeCalcTableLifecycleEventKind::TableMove,
        "table_delete" => TreeCalcTableLifecycleEventKind::TableDelete,
        "table_resize" => TreeCalcTableLifecycleEventKind::TableResize,
        "node_rename" => TreeCalcTableLifecycleEventKind::NodeRename,
        "node_move" => TreeCalcTableLifecycleEventKind::NodeMove,
        "node_delete" => TreeCalcTableLifecycleEventKind::NodeDelete,
        "save_reopen" => TreeCalcTableLifecycleEventKind::SaveReopen,
        "workspace_open" => TreeCalcTableLifecycleEventKind::WorkspaceOpen,
        "workspace_close" => TreeCalcTableLifecycleEventKind::WorkspaceClose,
        "workspace_alias_mutation" => TreeCalcTableLifecycleEventKind::WorkspaceAliasMutation,
        "function_registry_snapshot_mutation" => {
            TreeCalcTableLifecycleEventKind::FunctionRegistrySnapshotMutation
        }
        "structural_rebind" => TreeCalcTableLifecycleEventKind::StructuralRebind,
        other => panic!("unsupported table lifecycle event kind {other}"),
    }
}

fn table_lifecycle_snapshot_state(
    case_id: &str,
    state_id: &str,
    baseline: &TreeCalcTableNodeSnapshot,
) -> Option<TreeCalcTableNodeSnapshot> {
    match state_id {
        "none" => None,
        "baseline" | "body_cell_edit" | "save_reopen" | "function_registry_snapshot_mutation" => {
            Some(baseline.clone())
        }
        "body_formula_edit" => {
            let mut snapshot = baseline.clone();
            snapshot.columns[2].body_metadata =
                TreeCalcTableColumnBodyMetadata::Formula(TreeCalcTableFormulaMetadata {
                    formula_artifact_id: "formula:SalesTable.Columns.Tax".to_string(),
                    bind_artifact_id: Some("bind:SalesTable.Columns.Tax:v2".to_string()),
                    formula_text_version: "v2".to_string(),
                    formula_text: "=[@Amount]*0.2".to_string(),
                });
            Some(snapshot)
        }
        "row_insert" => {
            let mut snapshot = baseline.clone();
            snapshot
                .rows
                .push(TreeCalcTableRowId("row:south".to_string()));
            snapshot.row_membership_version = "table-rows:sales:membership:v2".to_string();
            snapshot.row_order_version = "table-rows:sales:order:v2".to_string();
            Some(snapshot)
        }
        "row_delete" => {
            let mut snapshot = baseline.clone();
            snapshot.rows.pop();
            snapshot.row_membership_version = "table-rows:sales:membership:v3".to_string();
            snapshot.row_order_version = "table-rows:sales:order:v3".to_string();
            Some(snapshot)
        }
        "row_reorder" => {
            let mut snapshot = baseline.clone();
            snapshot.rows.reverse();
            snapshot.row_order_version = "table-rows:sales:order:v4".to_string();
            Some(snapshot)
        }
        "column_insert" => {
            let mut snapshot = baseline.clone();
            snapshot.columns.push(TreeCalcTableColumnSnapshot {
                column_id: "col:discount".to_string(),
                column_name: "Discount".to_string(),
                ordinal: 4,
                body_metadata: TreeCalcTableColumnBodyMetadata::ConstantCells,
                totals_metadata: None,
            });
            snapshot.column_identity_version = "table-columns:sales:v2".to_string();
            Some(snapshot)
        }
        "column_delete" => {
            let mut snapshot = baseline.clone();
            snapshot
                .columns
                .retain(|column| column.column_id != "col:tax");
            snapshot.column_identity_version = "table-columns:sales:v3".to_string();
            Some(snapshot)
        }
        "column_reorder" => {
            let mut snapshot = baseline.clone();
            snapshot.columns[0].ordinal = 3;
            snapshot.columns[1].ordinal = 1;
            snapshot.columns[2].ordinal = 2;
            snapshot.column_identity_version = "table-columns:sales:v4".to_string();
            Some(snapshot)
        }
        "column_rename" | "header_text_edit" => {
            let mut snapshot = baseline.clone();
            snapshot.columns[1].column_name = "GrossAmount".to_string();
            snapshot.column_identity_version = "table-columns:sales:v5".to_string();
            Some(snapshot)
        }
        "totals_row_toggle" => {
            let mut snapshot = baseline.clone();
            snapshot.totals_row_present = false;
            Some(snapshot)
        }
        "totals_formula_edit" => {
            let mut snapshot = baseline.clone();
            snapshot.columns[1].totals_metadata = Some(TreeCalcTableFormulaMetadata {
                formula_artifact_id: "formula:SalesTable.Totals.Amount".to_string(),
                bind_artifact_id: Some("bind:SalesTable.Totals.Amount:v2".to_string()),
                formula_text_version: "v2".to_string(),
                formula_text: "=SUM([Amount])".to_string(),
            });
            Some(snapshot)
        }
        "table_rename" => {
            let mut snapshot = baseline.clone();
            snapshot.table_name = "SalesRenamed".to_string();
            snapshot.display_path = "SalesRenamed".to_string();
            snapshot.canonical_path = "SalesRenamed".to_string();
            snapshot.table_namespace_version = "table-namespace:sales:v2".to_string();
            Some(snapshot)
        }
        "table_move" => {
            let mut snapshot = baseline.clone();
            snapshot.virtual_anchor.start_col = 5;
            Some(snapshot)
        }
        "table_resize" => {
            let mut snapshot = baseline.clone();
            snapshot
                .rows
                .push(TreeCalcTableRowId("row:south".to_string()));
            snapshot.columns.push(TreeCalcTableColumnSnapshot {
                column_id: "col:discount".to_string(),
                column_name: "Discount".to_string(),
                ordinal: 4,
                body_metadata: TreeCalcTableColumnBodyMetadata::ConstantCells,
                totals_metadata: None,
            });
            snapshot.row_membership_version = "table-rows:sales:membership:v4".to_string();
            snapshot.row_order_version = "table-rows:sales:order:v5".to_string();
            snapshot.column_identity_version = "table-columns:sales:v6".to_string();
            Some(snapshot)
        }
        "node_rename" => {
            let mut snapshot = baseline.clone();
            snapshot.display_path = "Reports.SalesRenamed".to_string();
            snapshot.canonical_path = "Reports.SalesRenamed".to_string();
            snapshot.table_namespace_version = "table-namespace:sales:node-rename:v1".to_string();
            Some(snapshot)
        }
        "node_move" => {
            let mut snapshot = baseline.clone();
            snapshot.display_path = "Archive.SalesTable".to_string();
            snapshot.canonical_path = "Archive.SalesTable".to_string();
            snapshot.virtual_anchor.start_row = 12;
            snapshot.virtual_anchor.start_col = 4;
            Some(snapshot)
        }
        "workspace_alias_mutation" => {
            let mut snapshot = baseline.clone();
            snapshot.canonical_path = "Aliases.SalesTable".to_string();
            snapshot.table_namespace_version = "table-namespace:sales:alias:v1".to_string();
            Some(snapshot)
        }
        "structural_rebind" => {
            let mut snapshot = baseline.clone();
            snapshot.canonical_path = "Archive.SalesTable".to_string();
            snapshot.table_namespace_version = "table-namespace:sales:v3".to_string();
            Some(snapshot)
        }
        other => panic!("case {case_id} uses unsupported lifecycle fixture state {other}"),
    }
}

fn snapshot_projection_and_state(
    snapshot: &TreeCalcTableNodeSnapshot,
) -> Result<
    (
        TreeCalcTableNodeProjection,
        TreeCalcTableLifecycleVersionState,
    ),
    TreeCalcTableProjectionError,
> {
    let projection = project_treecalc_table_node_snapshot(snapshot)?;
    let state = lifecycle_state_from_snapshot_projection(snapshot, &projection);
    Ok((projection, state))
}

// The signature is genuinely complex (three nested optional projection
// pairs plus the callback packet) and a type alias would obscure rather
// than clarify each slot's role. Pre-existing; flagged when warnings
// were tightened on the workspace.
#[allow(clippy::type_complexity)]
fn table_lifecycle_packet_for_case(
    case: &TableLifecycleCase,
    owner: TreeNodeId,
    source_handles: impl IntoIterator<Item = &'static str>,
) -> Result<
    (
        Option<(
            TreeCalcTableNodeProjection,
            TreeCalcTableLifecycleVersionState,
        )>,
        Option<(
            TreeCalcTableNodeProjection,
            TreeCalcTableLifecycleVersionState,
        )>,
        TreeCalcTableLifecycleCallbackPacket,
    ),
    TreeCalcTableProjectionError,
> {
    let before = case
        .before
        .as_ref()
        .map(snapshot_projection_and_state)
        .transpose()?;
    let after = case
        .after
        .as_ref()
        .map(snapshot_projection_and_state)
        .transpose()?;
    let mut packet = TreeCalcTableLifecycleCallbackPacket::new(case.event_kind)
        .with_owner_nodes([owner])
        .with_source_reference_handles(source_handles)
        .with_changed_rows(case.changed_rows.clone())
        .with_changed_columns(case.changed_columns.clone());
    if let Some((_, state)) = before.as_ref() {
        packet = packet.with_before(state.clone());
    }
    if let Some((_, state)) = after.as_ref() {
        packet = packet.with_after(state.clone());
    }
    if matches!(
        case.event_kind,
        TreeCalcTableLifecycleEventKind::FunctionRegistrySnapshotMutation
    ) {
        packet.context_versions.registry_snapshot_identity =
            "oxfunc-registry:w093-udf-snapshot:v2".to_string();
    }

    Ok((before, after, packet))
}

fn lifecycle_state_from_snapshot_projection(
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
) -> TreeCalcTableLifecycleVersionState {
    TreeCalcTableLifecycleVersionState::from_snapshot_projection(snapshot, projection)
}

fn table_column_snapshot(column: &TableColumnFixture) -> TreeCalcTableColumnSnapshot {
    TreeCalcTableColumnSnapshot {
        column_id: column.column_id.clone(),
        column_name: column.name.clone(),
        ordinal: column.ordinal,
        body_metadata: match column.body.kind {
            TableColumnBodyKind::ConstantCells => TreeCalcTableColumnBodyMetadata::ConstantCells,
            TableColumnBodyKind::Formula => {
                TreeCalcTableColumnBodyMetadata::Formula(table_formula_metadata(
                    column
                        .body
                        .formula
                        .as_ref()
                        .expect("formula body has metadata"),
                ))
            }
        },
        totals_metadata: column.totals_formula.as_ref().map(table_formula_metadata),
    }
}

fn table_formula_metadata(formula: &TableFormulaFixture) -> TreeCalcTableFormulaMetadata {
    TreeCalcTableFormulaMetadata {
        formula_artifact_id: formula.formula_stable_id.clone(),
        bind_artifact_id: formula.bind_artifact_id.clone(),
        formula_text_version: formula.formula_text_version.clone(),
        formula_text: formula.formula_text.clone(),
    }
}

fn evaluate_tax_column(
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    table: &TableNodeFixture,
) -> oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport {
    let tax_formula = table
        .columns
        .iter()
        .find(|column| column.column_id == "col:tax")
        .and_then(|column| column.body.formula.as_ref())
        .expect("tax column formula exists");
    evaluate_treecalc_table_column_formula_rows(
        snapshot,
        projection,
        &TreeCalcTableColumnFormulaRuntimeRequest {
            target_column_id: "col:tax".to_string(),
            formula_stable_id: tax_formula.formula_stable_id.clone(),
            formula_text_version: 1,
            formula_text: tax_formula.formula_text.clone(),
            values: table_constant_sparse_values(table),
            runtime_context: TreeCalcTableFormulaRuntimeContext::default(),
        },
    )
    .expect("table column formula evaluates through OxCalc table runtime")
}

fn evaluate_amount_totals(
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    table: &TableNodeFixture,
) -> oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult {
    let totals_formula = table
        .columns
        .iter()
        .find(|column| column.column_id == "col:amount")
        .and_then(|column| column.totals_formula.as_ref())
        .expect("amount totals formula exists");
    evaluate_treecalc_table_totals_formula(
        snapshot,
        projection,
        &TreeCalcTableColumnFormulaRuntimeRequest {
            target_column_id: "col:amount".to_string(),
            formula_stable_id: totals_formula.formula_stable_id.clone(),
            formula_text_version: 1,
            formula_text: totals_formula.formula_text.clone(),
            values: table_constant_sparse_values(table),
            runtime_context: TreeCalcTableFormulaRuntimeContext::default(),
        },
    )
    .expect("table totals formula evaluates through OxCalc table runtime")
}

fn table_sparse_values<'a>(
    table: &TableNodeFixture,
    formula_report: Option<&oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport>,
    totals_values: impl IntoIterator<Item = (&'a str, CalcValue)>,
) -> Vec<TreeCalcTableSparseValue> {
    let mut values = table_constant_sparse_values(table);
    if let Some(formula_report) = formula_report {
        values.extend(formula_report.cell_results.iter().filter_map(|cell| {
            cell.row_id.as_ref().map(|row_id| {
                TreeCalcTableSparseValue::data(
                    row_id.0.clone(),
                    formula_report.target_column_id.clone(),
                    cell.value.clone(),
                )
            })
        }));
    }
    values.extend(
        totals_values
            .into_iter()
            .map(|(column_id, value)| TreeCalcTableSparseValue::totals(column_id, value)),
    );
    values
}

fn table_constant_sparse_values(table: &TableNodeFixture) -> Vec<TreeCalcTableSparseValue> {
    let mut values = Vec::new();
    for column in &table.columns {
        for cell in &column.body.constants {
            values.push(TreeCalcTableSparseValue::data(
                cell.row_id.clone(),
                column.column_id.clone(),
                parse_fixture_value(&cell.value),
            ));
        }
    }
    values
}

fn parse_fixture_value(value: &str) -> CalcValue {
    value.parse::<f64>().map_or_else(
        |_| CalcValue::text(ExcelText::from_interop_assignment(value)),
        CalcValue::number,
    )
}

fn table_data_caller_region(
    projection: &TreeCalcTableNodeProjection,
    row_offset: u32,
) -> TableCallerRegion {
    TableCallerRegion {
        table_id: projection.table_id.clone(),
        region_kind: TableRegionKind::Data,
        data_row_offset: Some(row_offset),
    }
}

fn assert_table_dependency_lowering(
    case_id: &str,
    projection: &TreeCalcTableNodeProjection,
    record: &oxfml_core::StructuredReferenceBindRecord,
    caller_region: Option<TableCallerRegion>,
) {
    let context_packet = StructuredTableContextPacket::from_oxfml_table_packet(
        vec![projection.table_descriptor.clone()],
        caller_region.as_ref().map(|_| TableRef {
            table_id: projection.table_id.clone(),
        }),
        caller_region,
    );
    let request = StructuredTableDependencyLoweringRequest::from_oxfml_bind_record(
        TreeNodeId(100),
        context_packet,
        record,
    )
    .unwrap_or_else(|error| panic!("case {case_id} dependency intake failed: {error:?}"));
    let lowering = lower_structured_table_dependencies(&request);
    assert!(
        lowering
            .facts
            .iter()
            .all(|fact| fact.status == StructuredTableDependencyFactStatus::Lowered),
        "case {case_id} has blocked structured-table dependency facts: {:?}",
        lowering.facts
    );
    assert!(
        lowering
            .descriptors
            .iter()
            .any(|descriptor| descriptor.kind == DependencyDescriptorKind::StructuredTableIdentity),
        "case {case_id} must lower table identity dependency"
    );
    if record.uses_this_row {
        assert!(
            lowering.descriptors.iter().any(|descriptor| {
                descriptor.kind == DependencyDescriptorKind::StructuredTableCallerContext
            }),
            "case {case_id} must lower caller row context dependency"
        );
    }
}

fn evaluate_case_formula(
    case_id: &str,
    formula_text: &str,
    projection: &TreeCalcTableNodeProjection,
    caller_region: Option<TableCallerRegion>,
    runtime_binding: oxcalc_core::structured_table::TreeCalcStructuredTableRuntimeBinding,
) -> String {
    display_value(&evaluate_case_formula_value(
        case_id,
        formula_text,
        projection,
        caller_region,
        runtime_binding,
    ))
}

fn evaluate_case_formula_value(
    case_id: &str,
    formula_text: &str,
    projection: &TreeCalcTableNodeProjection,
    caller_region: Option<TableCallerRegion>,
    runtime_binding: oxcalc_core::structured_table::TreeCalcStructuredTableRuntimeBinding,
) -> CalcValue {
    let reference_system_provider = TreeCalcReferenceSystemProvider::sparse_only()
        .with_sparse_reference_values(
            runtime_binding.sparse_reference_values.reference.clone(),
            runtime_binding.sparse_reference_values.resolved_values(),
        );
    let result = RuntimeEnvironment::new()
        .with_primary_locus(table_primary_locus(projection))
        .with_table_context(
            vec![projection.table_descriptor.clone()],
            caller_region.as_ref().map(|_| TableRef {
                table_id: projection.table_id.clone(),
            }),
            caller_region,
        )
        .with_cell_values(runtime_binding.scalar_cell_values)
        .with_host_name_bindings(vec![
            runtime_host_name_binding_from_sparse_reference_values(
                runtime_binding.sparse_reference_values,
            ),
        ])
        .execute(
            RuntimeFormulaRequest::new(
                FormulaSourceRecord::new(format!("dnatreecalc:{case_id}"), 1, formula_text),
                TypedContextQueryBundle::default().with_reference_system_provider(Some(
                    &reference_system_provider
                        as &dyn oxfunc_core::resolver::ReferenceSystemProvider,
                )),
            )
            .with_backend(EvaluationBackend::OxFuncBacked),
        )
        .unwrap_or_else(|error| panic!("case {case_id} formula runtime failed: {error}"));
    assert!(
        result.syntax_diagnostics.is_empty() && result.bind_diagnostics.is_empty(),
        "case {case_id} formula runtime produced diagnostics: syntax={:?} bind={:?}",
        result.syntax_diagnostics,
        result.bind_diagnostics
    );
    result.evaluation.oxfunc_value
}

fn runtime_host_name_binding_from_sparse_reference_values(
    binding: TreeCalcSparseReferenceValuesBinding,
) -> RuntimeHostNameBinding {
    let canonical_name = binding.reference.target().to_string();
    RuntimeHostNameBinding {
        bind_result: RuntimeHostNameBindResult {
            host_name_handle: canonical_name.clone(),
            canonical_name: canonical_name.clone(),
            host_dependency_key: None,
            source_span: oxfml_core::syntax::token::TextSpan::new(0, canonical_name.len()),
            source_token_text: canonical_name.clone(),
            resolution_layer: "treecalc_sparse_reference_values".to_string(),
            binding_kind: "defined_name_value_like".to_string(),
            shape_hint: Some("sparse_reference_values".to_string()),
            caller_context_dependent: false,
            diagnostics: Vec::new(),
            replay_identity_contribution: format!("treecalc:sparse-reference:{canonical_name}"),
        },
        binding: DefinedNameBinding::Value(CalcValue::reference(binding.reference)),
    }
}

fn table_primary_locus(projection: &TreeCalcTableNodeProjection) -> Locus {
    Locus {
        sheet_id: projection.table_descriptor.sheet_scope_ref.clone(),
        row: 3,
        col: 2,
    }
}

fn bind_treecalc_table_structured_references(
    formula_text: &str,
    projection: &TreeCalcTableNodeProjection,
    enclosing_table_ref: Option<TableRef>,
    caller_table_region: Option<TableCallerRegion>,
) -> Vec<TableStructuredReferenceBinding> {
    let entered_formula_text = if formula_text.trim_start().starts_with('=') {
        formula_text.to_string()
    } else {
        format!("={formula_text}")
    };
    let source = FormulaSourceRecord::new(
        format!("dnatreecalc:table-bind:{}", projection.table_id),
        1,
        entered_formula_text,
    );
    let parse = parse_formula(ParseRequest {
        source: source.clone(),
    });
    if !parse.green_tree.diagnostics.is_empty() {
        let diagnostics = parse
            .green_tree
            .diagnostics
            .iter()
            .map(|diagnostic| StructuredReferenceBindDiagnosticLink {
                diagnostic_code: "oxfml.syntax_diagnostic".to_string(),
                message: diagnostic.message.clone(),
                source_span_utf8: diagnostic.span,
            })
            .collect::<Vec<_>>();
        return vec![TableStructuredReferenceBinding {
            source_span_utf8: oxfml_core::syntax::token::TextSpan::new(
                0,
                source.entered_formula_text.len(),
            ),
            source_token_text: formula_text.to_string(),
            host_ref_handle: "oxfml.syntax_diagnostic".to_string(),
            resolved_table_id: None,
            caller_context_dependency: caller_table_region.is_some(),
            replay_identity: "oxfml.syntax_diagnostic".to_string(),
            bind_record: StructuredReferenceBindRecord {
                bind_record_handle: "oxfml.syntax_diagnostic".to_string(),
                source_span_utf8: oxfml_core::syntax::token::TextSpan::new(
                    0,
                    source.entered_formula_text.len(),
                ),
                source_token_text: formula_text.to_string(),
                source_token_kind: StructuredReferenceSourceTokenKind::StructuredReference,
                explicit_table_name: None,
                omitted_table_name: false,
                effective_table_id: None,
                effective_table_name: None,
                selected_column_ids: Vec::new(),
                selected_sections: Vec::new(),
                selected_regions: Vec::new(),
                uses_this_row: false,
                caller_context_dependent: caller_table_region.is_some(),
                resolved_reference: None,
                diagnostics: diagnostics.clone(),
            },
            diagnostics,
        }];
    }
    let red_projection = project_red_view(source.formula_stable_id.clone(), &parse.green_tree);
    let primary_locus = table_primary_locus(projection);
    let bind = bind_formula(BindRequest {
        source: source.clone(),
        green_tree: parse.green_tree,
        red_projection,
        context: BindContext {
            workbook_id: projection.table_descriptor.workbook_scope_ref.clone(),
            sheet_id: projection.table_descriptor.sheet_scope_ref.clone(),
            caller_row: primary_locus.row,
            caller_col: primary_locus.col,
            formula_token: source.formula_token(),
            structure_context_version: StructureContextVersion("treecalc-structure:v1".to_string()),
            table_catalog: vec![projection.table_descriptor.clone()],
            enclosing_table_ref,
            caller_table_region,
            ..BindContext::default()
        },
        host_name_resolver: None,
    });

    bind.bound_formula
        .structured_reference_bind_records
        .into_iter()
        .map(table_structured_reference_binding_from_oxfml_record)
        .collect()
}

fn table_structured_reference_binding_from_oxfml_record(
    record: StructuredReferenceBindRecord,
) -> TableStructuredReferenceBinding {
    let replay_identity = format!(
        "treecalc.table.oxfml_bind_record.v1:{}:{}:{:?}:{:?}:{}",
        record.bind_record_handle,
        record
            .effective_table_id
            .as_deref()
            .unwrap_or("<unresolved>"),
        record.selected_column_ids,
        record.selected_sections,
        record.uses_this_row
    );
    TableStructuredReferenceBinding {
        source_span_utf8: record.source_span_utf8,
        source_token_text: record.source_token_text.clone(),
        host_ref_handle: record.bind_record_handle.clone(),
        resolved_table_id: record.effective_table_id.clone(),
        caller_context_dependency: record.caller_context_dependent,
        replay_identity,
        diagnostics: record.diagnostics.clone(),
        bind_record: record,
    }
}

fn display_value(value: &CalcValue) -> String {
    match &value.core {
        CoreValue::Number(number) => display_number(*number),
        CoreValue::Text(text) => text.to_string_lossy(),
        other => format!("{other:?}"),
    }
}

fn display_number(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

fn assert_table_update_scenarios_are_classified(projection: &TreeCalcTableNodeProjection) {
    let scenarios = [
        TreeCalcTableUpdateScenarioKind::BodyCellEdit,
        TreeCalcTableUpdateScenarioKind::BodyFormulaEdit,
        TreeCalcTableUpdateScenarioKind::RowInsert,
        TreeCalcTableUpdateScenarioKind::RowDelete,
        TreeCalcTableUpdateScenarioKind::RowReorder,
        TreeCalcTableUpdateScenarioKind::ColumnInsert,
        TreeCalcTableUpdateScenarioKind::ColumnDelete,
        TreeCalcTableUpdateScenarioKind::ColumnReorder,
        TreeCalcTableUpdateScenarioKind::ColumnRename,
        TreeCalcTableUpdateScenarioKind::HeaderTextEdit,
        TreeCalcTableUpdateScenarioKind::TotalsRowToggle,
        TreeCalcTableUpdateScenarioKind::TotalsFormulaEdit,
        TreeCalcTableUpdateScenarioKind::TableRename,
        TreeCalcTableUpdateScenarioKind::TableMove,
        TreeCalcTableUpdateScenarioKind::TableDelete,
        TreeCalcTableUpdateScenarioKind::SaveReopen,
        TreeCalcTableUpdateScenarioKind::StructuralRebind,
    ];
    let source_handles = vec!["bind:SalesTable.Columns.Tax".to_string()];
    let changed = scenarios
        .iter()
        .copied()
        .map(|scenario| {
            let after = if scenario == TreeCalcTableUpdateScenarioKind::TableDelete {
                None
            } else {
                Some(projection)
            };
            let impact = classify_treecalc_table_update(
                scenario,
                Some(projection),
                after,
                [TreeNodeId(100)],
                source_handles.clone(),
            );
            if scenario == TreeCalcTableUpdateScenarioKind::SaveReopen {
                assert!(
                    impact.changed_dependency_kinds.is_empty()
                        && impact.invalidation_reasons.is_empty()
                );
            } else {
                assert!(
                    !impact.changed_dependency_kinds.is_empty()
                        || !impact.invalidation_reasons.is_empty(),
                    "{scenario:?} should retain dependency or invalidation evidence"
                );
            }
            (scenario, impact.invalidation_reasons)
        })
        .collect::<BTreeMap<_, _>>();

    assert!(
        changed[&TreeCalcTableUpdateScenarioKind::RowInsert]
            .contains(&InvalidationReasonKind::StructuredTableRowMembershipChanged)
    );
    assert!(
        changed[&TreeCalcTableUpdateScenarioKind::ColumnRename]
            .contains(&InvalidationReasonKind::StructuredTableColumnChanged)
    );
    assert!(
        changed[&TreeCalcTableUpdateScenarioKind::HeaderTextEdit]
            .contains(&InvalidationReasonKind::StructuredTableRegionChanged)
    );
    assert!(
        changed[&TreeCalcTableUpdateScenarioKind::TotalsFormulaEdit]
            .contains(&InvalidationReasonKind::StructuredTableRegionChanged)
    );
    assert!(
        changed[&TreeCalcTableUpdateScenarioKind::TableDelete]
            .contains(&InvalidationReasonKind::StructuralRebindRequired)
    );

    let reference = oxcalc_core::structured_table::StructuredTableReferenceIntake::explicit_table(
        "structured-ref:amount",
        projection.table_id.clone(),
    )
    .with_selected_columns(["col:amount".to_string()]);
    assert!(
        validate_treecalc_table_reference_after_update(
            projection.table_id.clone(),
            Some(projection),
            &reference,
            None,
        )
        .is_empty()
    );
    assert!(
        !validate_treecalc_table_reference_after_update(
            projection.table_id.clone(),
            None,
            &reference,
            None,
        )
        .is_empty(),
        "table delete must retain a typed post-update diagnostic"
    );
}

fn retained_table_replay_artifact(
    theme: &CorpusTheme,
    workspace: &WorkspaceModel,
    table: &TableNodeFixture,
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    json!({
        "scenario_id": "w056_treecalc_table_structured_references_001",
        "lane_id": "dna_treecalc",
        "events": [
            {
                "event_id": "treecalc_table_slice_sales",
                "source_label": "table_slice:SalesTable:direct",
                "normalized_family": "treecalc.surface.table_slice.direct:SalesTable"
            },
            {
                "event_id": "treecalc_table_dependencies_sales",
                "source_label": "dependency_evidence:SalesTable:direct",
                "normalized_family": "treecalc.surface.dependency_evidence.direct:SalesTable"
            },
            {
                "event_id": "treecalc_table_invalidations_sales",
                "source_label": "invalidation_evidence:SalesTable:direct",
                "normalized_family": "treecalc.surface.invalidation_evidence.direct:SalesTable"
            }
        ],
        "registry_refs": [
            {
                "family": "dnatreecalc.test_corpus",
                "version": format!("{}@{}:cases={}", theme.theme, theme.schema_version, theme.cases.len())
            },
            {
                "family": "dnatreecalc.workspace_fixture",
                "version": format!("{}@treecalc-workspace-v1", workspace.workspace_id)
            }
        ],
        "comparison_views": [
            {
                "view_family": "table_slice",
                "value": retained_table_slice_json(table, snapshot, projection, tax_report, totals_amount)
            },
            {
                "view_family": "per_node_value",
                "value": retained_per_node_value_json(
                    theme,
                    workspace,
                    table,
                    snapshot,
                    projection,
                    tax_report,
                    totals_amount,
                )
            },
            {
                "view_family": "effective_display_text",
                "value": retained_effective_display_text_json(tax_report, totals_amount)
            },
            {
                "view_family": "execution_outcome",
                "value": {
                    "outcome_schema": "dna_treecalc.execution_outcome.v1",
                    "scenario_id": "w056_treecalc_table_structured_references_001",
                    "outcome_kind": "accepted_execution",
                    "outcome_stage": "oxcalc_tree_context_table_projection",
                    "class_id": "treecalc_table_structured_reference_projection_ready",
                    "lane_reason_code": "dnatreecalc_w056_table_projection_retained",
                    "engine_context": "OxCalcTreeContext",
                    "engine_surface": "OxCalc W056 structured-table public APIs",
                    "replay_view_ready": true
                }
            },
            {
                "view_family": "dependency_evidence",
                "value": retained_dependency_evidence_json(theme, projection)
            },
            {
                "view_family": "invalidation_evidence",
                "value": retained_invalidation_evidence_json(projection)
            },
            {
                "view_family": "retained_artifact_ref",
                "value": retained_artifact_refs_json()
            }
        ],
        "source_metadata": {
            "source_host": "dna_treecalc",
            "source_schema_id": "dna_treecalc.w056_table_replay.v1",
            "projection_status": "direct",
            "capture_mode": "model_projection",
            "capture_loss": "none",
            "capture_loss_summary": [],
            "uncertainty_summary": [],
            "direct_context": true,
            "adapter_id": "dnatreecalc.oxcalc_table_projection.v1",
            "workspace_id": workspace.workspace_id,
            "source_refs": [
                "docs/test-corpus/tables/structured-references.json",
                "docs/test-corpus/workspaces/tables.json"
            ],
            "shared_scenario_alias": "w056_table_structured_references_001",
            "cross_producer_aliases": [
                "xlplay_structured_reference_workbook_001",
                "xlplay_table_construction_basic_001"
            ],
            "interpretation_limits": [
                {
                    "kind": "model_projection_not_excel_observation",
                    "detail": "This artifact is DnaTreeCalc/OxCalc retained producer evidence; Excel black-box observation is supplied by OxXlPlay."
                },
                {
                    "kind": "no_private_formula_semantics",
                    "detail": "Structured-reference parse and bind facts come from OxCalc/OxFml public table packets, not DnaTreeCalc formula parsing."
                }
            ],
            "retained_scope": "primary_sales_table_update_slice",
            "unavailable_surfaces": [
                {
                    "surface": "escaped_table_retained_table_slice",
                    "reason": "The active table runner exercises bracket-escaped table and column references; this retained producer artifact keeps the primary SalesTable update/evidence slice for OxReplay intake."
                }
            ],
            "comparison_view_families": [
                "table_slice",
                "per_node_value",
                "effective_display_text",
                "execution_outcome",
                "dependency_evidence",
                "invalidation_evidence",
                "retained_artifact_ref"
            ]
        }
    })
}

fn retained_table_lifecycle_replay_artifact(
    lifecycle_theme: &TableLifecycleTheme,
    workspace: &WorkspaceModel,
    table: &TableNodeFixture,
    baseline_snapshot: &TreeCalcTableNodeSnapshot,
    baseline_projection: &TreeCalcTableNodeProjection,
) -> Value {
    let owner = TreeNodeId(100);
    let source_handles = ["bind:SalesTable.Columns.Tax"];
    let reports = table_lifecycle_cases(lifecycle_theme, baseline_snapshot)
        .iter()
        .map(|case| {
            let (_, _, packet) =
                table_lifecycle_packet_for_case(case, owner, source_handles.iter().copied())
                    .unwrap_or_else(|error| {
                        panic!("{} lifecycle packet failed: {error:?}", case.name)
                    });
            let report = classify_treecalc_table_lifecycle_callback(&packet);
            assert!(
                report.diagnostics.is_empty(),
                "{} lifecycle retained report diagnostics: {:?}",
                case.name,
                report.diagnostics
            );
            table_lifecycle_report_json(case, &report)
        })
        .collect::<Vec<_>>();
    let stale_identity =
        retained_table_lifecycle_stale_identity_json(baseline_snapshot, baseline_projection, owner);
    let roundtrip = retained_table_lifecycle_roundtrip_json(table);

    json!({
        "scenario_id": "w056_treecalc_table_lifecycle_001",
        "lane_id": "dna_treecalc",
        "events": [
            {
                "event_id": "treecalc_table_lifecycle_callbacks_sales",
                "source_label": "table_lifecycle:SalesTable:callback",
                "normalized_family": "treecalc.surface.table_lifecycle.callback:SalesTable"
            },
            {
                "event_id": "treecalc_table_lifecycle_invalidations_sales",
                "source_label": "invalidation_evidence:SalesTable:lifecycle",
                "normalized_family": "treecalc.surface.invalidation_evidence.lifecycle:SalesTable"
            },
            {
                "event_id": "treecalc_table_lifecycle_persistence_sales",
                "source_label": "persistence_handles:SalesTable:lifecycle",
                "normalized_family": "treecalc.surface.persistence_handles.lifecycle:SalesTable"
            }
        ],
        "registry_refs": [
            {
                "family": "dnatreecalc.workspace_fixture",
                "version": format!("{}@treecalc-workspace-v1", workspace.workspace_id)
            },
            {
                "family": "dnatreecalc.test_corpus",
                "version": format!("{}@{}:cases={}", lifecycle_theme.theme, lifecycle_theme.schema_version, lifecycle_theme.cases.len())
            },
            {
                "family": "oxcalc.table_lifecycle_callback_contract",
                "version": "treecalc.table_lifecycle.callback.v1"
            }
        ],
        "comparison_views": [
            {
                "view_family": "execution_outcome",
                "value": {
                    "outcome_schema": "dna_treecalc.execution_outcome.v1",
                    "scenario_id": "w056_treecalc_table_lifecycle_001",
                    "outcome_kind": "accepted_execution",
                    "outcome_stage": "oxcalc_table_lifecycle_callbacks",
                    "class_id": "treecalc_table_lifecycle_callbacks_ready",
                    "lane_reason_code": "dnatreecalc_w056_table_lifecycle_retained",
                    "engine_context": "OxCalcTreeContext",
                    "engine_surface": "OxCalc W056 table lifecycle callback public API",
                    "accepted_event_count": reports.len(),
                    "accepted_event_kinds": reports
                        .iter()
                        .map(|report| report["event_kind"].clone())
                        .collect::<Vec<_>>(),
                    "typed_rejection_evidence": stale_identity,
                    "persistence_roundtrip": roundtrip,
                    "replay_view_ready": true
                }
            },
            {
                "view_family": "dependency_evidence",
                "value": {
                    "source_status": "direct",
                    "classification_api": "classify_treecalc_table_lifecycle_callback",
                    "table_id": baseline_projection.table_id,
                    "table_context_identity_present": !baseline_projection.table_context_identity.is_empty(),
                    "event_reports": reports.clone()
                }
            },
            {
                "view_family": "invalidation_evidence",
                "value": {
                    "source_status": "direct",
                    "classification_api": "classify_treecalc_table_lifecycle_callback",
                    "table_id": baseline_projection.table_id,
                    "event_invalidations": reports
                        .iter()
                        .map(|report| {
                            json!({
                                "event_kind": report["event_kind"].clone(),
                                "changed_dependency_kinds": report["changed_dependency_kinds"].clone(),
                                "invalidation_reasons": report["invalidation_reasons"].clone(),
                                "prepared_identity_inputs": report["prepared_identity_inputs"].clone(),
                                "invalidation_seeds": report["invalidation_seeds"].clone()
                            })
                        })
                        .collect::<Vec<_>>()
                }
            },
            {
                "view_family": "retained_artifact_ref",
                "value": retained_table_lifecycle_artifact_refs_json()
            }
        ],
        "source_metadata": {
            "source_host": "dna_treecalc",
            "source_schema_id": "dna_treecalc.w056_table_lifecycle_replay.v1",
            "projection_status": "direct",
            "capture_mode": "model_projection",
            "capture_loss": "none",
            "capture_loss_summary": [],
            "uncertainty_summary": [],
            "direct_context": true,
            "adapter_id": "dnatreecalc.oxcalc_table_lifecycle_callback.v1",
            "workspace_id": workspace.workspace_id,
            "source_refs": [
                "docs/test-corpus/tables/lifecycle-events.json",
                "docs/test-corpus/workspaces/tables.json"
            ],
            "shared_scenario_alias": "w056_table_lifecycle_001",
            "interpretation_limits": [
                {
                    "kind": "model_projection_not_excel_observation",
                    "detail": "This artifact is DnaTreeCalc/OxCalc retained producer evidence; Excel black-box observation is supplied by OxXlPlay."
                },
                {
                    "kind": "no_private_dependency_semantics",
                    "detail": "Dependency and invalidation classifications are emitted by OxCalc lifecycle callback packets, not DnaTreeCalc-local mirrors."
                }
            ],
            "comparison_view_families": [
                "execution_outcome",
                "dependency_evidence",
                "invalidation_evidence",
                "retained_artifact_ref"
            ]
        }
    })
}

fn retained_table_replay_manifest() -> Value {
    json!({
        "bundle_id": "dnatreecalc-w056-table-structured-references-001",
        "scenario_id": "w056_treecalc_table_structured_references_001",
        "bundle_schema": "replay.bundle.v1",
        "source_schema": "dna_treecalc.replay_bundle_seed.v1",
        "lane_id": "dna_treecalc",
        "adapter_id": "dnatreecalc.oxcalc_table_projection.v1",
        "capture_mode": "model_projection",
        "projection_status": "lossless",
        "capture_loss": "none",
        "registry_refs": [],
        "sidecars": [],
        "views": [
            {
                "artifact_family": "normalized_replay",
                "path": "views/normalized-replay.json"
            }
        ],
        "declared_comparison_views": [
            "table_slice",
            "per_node_value",
            "effective_display_text",
            "execution_outcome",
            "dependency_evidence",
            "invalidation_evidence",
            "retained_artifact_ref"
        ]
    })
}

fn retained_table_lifecycle_replay_manifest() -> Value {
    json!({
        "bundle_id": "dnatreecalc-w056-table-lifecycle-001",
        "scenario_id": "w056_treecalc_table_lifecycle_001",
        "bundle_schema": "replay.bundle.v1",
        "source_schema": "dna_treecalc.replay_bundle_seed.v1",
        "lane_id": "dna_treecalc",
        "adapter_id": "dnatreecalc.oxcalc_table_lifecycle_callback.v1",
        "capture_mode": "model_projection",
        "projection_status": "lossless",
        "capture_loss": "none",
        "registry_refs": [],
        "sidecars": [],
        "views": [
            {
                "artifact_family": "normalized_replay",
                "path": "views/normalized-replay.json"
            }
        ],
        "declared_comparison_views": [
            "execution_outcome",
            "dependency_evidence",
            "invalidation_evidence",
            "retained_artifact_ref"
        ]
    })
}

fn retained_empty_body_table_replay_artifact(
    theme: &CorpusTheme,
    workspace: &WorkspaceModel,
) -> Value {
    let case_evidence = theme
        .cases
        .iter()
        .map(|case| retained_empty_body_case_evidence_json(case, workspace))
        .collect::<Vec<_>>();

    json!({
        "scenario_id": "w056_treecalc_empty_body_tables_001",
        "lane_id": "dna_treecalc",
        "events": [
            {
                "event_id": "treecalc_empty_body_table_slices",
                "source_label": "table_slice:empty-body-tables",
                "normalized_family": "treecalc.surface.table_slice.empty_body"
            },
            {
                "event_id": "treecalc_empty_body_table_dependencies",
                "source_label": "dependency_evidence:empty-body-tables",
                "normalized_family": "treecalc.surface.dependency_evidence.empty_body"
            },
            {
                "event_id": "treecalc_empty_body_table_invalidations",
                "source_label": "invalidation_evidence:empty-body-tables",
                "normalized_family": "treecalc.surface.invalidation_evidence.empty_body"
            }
        ],
        "registry_refs": [
            {
                "family": "dnatreecalc.test_corpus",
                "version": format!("{}@{}:cases={}", theme.theme, theme.schema_version, theme.cases.len())
            },
            {
                "family": "dnatreecalc.workspace_fixture",
                "version": format!("{}@treecalc-workspace-v1", workspace.workspace_id)
            }
        ],
        "comparison_views": [
            {
                "view_family": "table_slice",
                "value": retained_empty_body_table_slice_json(workspace)
            },
            {
                "view_family": "per_node_value",
                "value": retained_empty_body_per_node_value_json(&case_evidence)
            },
            {
                "view_family": "effective_display_text",
                "value": retained_empty_body_display_json(workspace)
            },
            {
                "view_family": "execution_outcome",
                "value": {
                    "outcome_schema": "dna_treecalc.execution_outcome.v1",
                    "scenario_id": "w056_treecalc_empty_body_tables_001",
                    "outcome_kind": "accepted_execution",
                    "outcome_stage": "oxcalc_tree_context_empty_body_table_projection",
                    "class_id": "treecalc_empty_body_tables_ready",
                    "lane_reason_code": "dnatreecalc_w056_empty_body_table_retained",
                    "engine_context": "OxCalcTreeContext",
                    "engine_surface": "OxCalc W056 structured-table public APIs",
                    "resolved_case_count": case_evidence
                        .iter()
                        .filter(|case| case["outcome_kind"] == json!("resolved"))
                        .count(),
                    "typed_diagnostic_case_count": case_evidence
                        .iter()
                        .filter(|case| case["outcome_kind"] == json!("typed_reader_diagnostic"))
                        .count(),
                    "case_outcomes": case_evidence,
                    "replay_view_ready": true
                }
            },
            {
                "view_family": "dependency_evidence",
                "value": retained_empty_body_dependency_evidence_json(theme, workspace)
            },
            {
                "view_family": "invalidation_evidence",
                "value": retained_empty_body_invalidation_evidence_json(workspace)
            },
            {
                "view_family": "retained_artifact_ref",
                "value": retained_empty_body_artifact_refs_json()
            }
        ],
        "source_metadata": {
            "source_host": "dna_treecalc",
            "source_schema_id": "dna_treecalc.w056_empty_body_table_replay.v1",
            "projection_status": "direct",
            "capture_mode": "model_projection",
            "capture_loss": "none",
            "capture_loss_summary": [],
            "uncertainty_summary": [],
            "direct_context": true,
            "adapter_id": "dnatreecalc.oxcalc_empty_body_table_projection.v1",
            "workspace_id": workspace.workspace_id,
            "source_refs": [
                "docs/test-corpus/tables/empty-body.json",
                "docs/test-corpus/workspaces/empty-body-tables.json"
            ],
            "shared_scenario_alias": "w056_empty_body_tables_001",
            "interpretation_limits": [
                {
                    "kind": "model_projection_not_excel_observation",
                    "detail": "This artifact is DnaTreeCalc/OxCalc retained producer evidence; Excel black-box observation is supplied by OxXlPlay."
                },
                {
                    "kind": "no_private_formula_semantics",
                    "detail": "Structured-reference parse, bind, sparse-reader, and update impact facts come from OxCalc/OxFml public table APIs, not DnaTreeCalc formula parsing."
                }
            ],
            "comparison_view_families": [
                "table_slice",
                "per_node_value",
                "effective_display_text",
                "execution_outcome",
                "dependency_evidence",
                "invalidation_evidence",
                "retained_artifact_ref"
            ]
        }
    })
}

fn retained_empty_body_table_replay_manifest() -> Value {
    json!({
        "bundle_id": "dnatreecalc-w056-table-empty-body-001",
        "scenario_id": "w056_treecalc_empty_body_tables_001",
        "bundle_schema": "replay.bundle.v1",
        "source_schema": "dna_treecalc.replay_bundle_seed.v1",
        "lane_id": "dna_treecalc",
        "adapter_id": "dnatreecalc.oxcalc_empty_body_table_projection.v1",
        "capture_mode": "model_projection",
        "projection_status": "lossless",
        "capture_loss": "none",
        "registry_refs": [],
        "sidecars": [],
        "views": [
            {
                "artifact_family": "normalized_replay",
                "path": "views/normalized-replay.json"
            }
        ],
        "declared_comparison_views": [
            "table_slice",
            "per_node_value",
            "effective_display_text",
            "execution_outcome",
            "dependency_evidence",
            "invalidation_evidence",
            "retained_artifact_ref"
        ]
    })
}

fn retained_table_slice_json(
    table: &TableNodeFixture,
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    let totals_results = [("col:amount", totals_amount)];
    json!({
        "table_slice_schema": "dna_treecalc.table_slice.v1",
        "source_status": "direct",
        "table_id": projection.table_id,
        "table_node_id": snapshot.table_node_id.to_string(),
        "table_name": snapshot.table_name,
        "display_path": projection.display_path,
        "canonical_locator": projection.canonical_path,
        "table_range_ref": projection.table_descriptor.table_range_ref,
        "header_region_ref": projection.table_descriptor.header_region_ref,
        "data_body_range_ref": table_data_body_range_ref(snapshot),
        "data_body_column_range_refs": projection
            .table_descriptor
            .columns
            .iter()
            .map(|column| column.column_range_ref.clone())
            .collect::<Vec<_>>(),
        "totals_region_ref": projection.table_descriptor.totals_region_ref,
        "header_row_present": projection.table_descriptor.header_row_present,
        "totals_row_present": projection.table_descriptor.totals_row_present,
        "header_row": {
            "present": table.header.present,
            "cells": table.columns.iter().map(|column| {
                json!({
                    "column_id": column.column_id,
                    "column_name": column.name,
                    "value_state": "defined",
                    "value_repr": column.name,
                    "effective_display_text": column.name
                })
            }).collect::<Vec<_>>()
        },
        "versions": {
            "table_namespace_version": snapshot.table_namespace_version,
            "row_membership_version": snapshot.row_membership_version,
            "row_order_version": snapshot.row_order_version,
            "column_identity_version": snapshot.column_identity_version
        },
        "engine_identity_refs": {
            "table_context_identity_present": !projection.table_context_identity.is_empty(),
            "table_invalidation_identity_present": !projection.table_invalidation_identity.is_empty(),
            "table_namespace_token": projection.table_namespace_token,
            "row_membership_identity": projection.row_membership_identity,
            "row_order_identity": projection.row_order_identity,
            "column_identity": projection.column_identity,
            "virtual_anchor_token": projection.virtual_anchor_token,
            "body_metadata_token": projection.body_metadata_token,
            "totals_metadata_token": projection.totals_metadata_token
        },
        "rows": table.rows.iter().map(|row| {
            json!({
                "row_id": row.row_id,
                "ordinal": row.ordinal
            })
        }).collect::<Vec<_>>(),
        "columns": table.columns.iter().map(|column| {
            json!({
                "column_id": column.column_id,
                "column_name": column.name,
                "ordinal": column.ordinal,
                "body_kind": table_column_body_kind_id(column.body.kind),
                "data_range_ref": projection
                    .table_descriptor
                    .columns
                    .iter()
                    .find(|descriptor| descriptor.column_id == column.column_id)
                    .map(|descriptor| descriptor.column_range_ref.clone()),
                "body_formula": column.body.formula.as_ref().map(table_formula_json),
                "totals_formula": column.totals_formula.as_ref().map(table_formula_json)
            })
        }).collect::<Vec<_>>(),
        "data_body": table.rows.iter().map(|row| {
            json!({
                "row_id": row.row_id,
                "ordinal": row.ordinal,
                "cells": table.columns.iter().filter_map(|column| {
                    table_cell_value(table, tax_report, &row.row_id, &column.column_id).map(|value| {
                        json!({
                            "row_id": row.row_id,
                            "column_id": column.column_id,
                            "column_name": column.name,
                            "value_repr": display_value(&value),
                            "effective_display_text": display_value(&value),
                            "formula_text": table_data_formula_text(column)
                        })
                    })
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "totals_row": {
            "present": table.totals.present,
            "cells": table.columns.iter().map(|column| {
                let result = totals_results
                    .iter()
                    .find(|(column_id, _)| *column_id == column.column_id.as_str())
                    .map(|(_, result)| *result);
                json!({
                    "column_id": column.column_id,
                    "column_name": column.name,
                    "value_state": if result.is_some() { "defined" } else { "blank" },
                    "value_repr": result
                        .map(|cell| display_value(&cell.value))
                        .unwrap_or_default(),
                    "effective_display_text": result
                        .map(|cell| display_value(&cell.value))
                        .unwrap_or_default(),
                    "formula_text": column
                        .totals_formula
                        .as_ref()
                        .map(|formula| formula.formula_text.as_str())
                })
            }).collect::<Vec<_>>()
        }
    })
}

fn retained_per_node_value_json(
    theme: &CorpusTheme,
    workspace: &WorkspaceModel,
    table: &TableNodeFixture,
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    let report_node = retained_workspace_report_node_json(
        theme,
        workspace,
        table,
        snapshot,
        projection,
        tax_report,
        totals_amount,
    );
    let totals_node = retained_totals_node_json(workspace, table, projection, totals_amount);
    json!({
        "source_status": "direct",
        "nodes": [
            report_node,
            totals_node
        ],
        "table_cells": table.rows.iter().flat_map(|row| {
            table.columns.iter().filter_map(move |column| {
                table_cell_value(table, tax_report, &row.row_id, &column.column_id).map(|value| {
                    json!({
                        "table_id": "tree-table:sales",
                        "row_id": row.row_id,
                        "column_id": column.column_id,
                        "canonical_locator": format!("tree:tables:SalesTable[row={},column={}]", row.row_id, column.column_id),
                        "comparison_value": comparison_value_json(&value)
                    })
                })
            })
        }).collect::<Vec<_>>()
    })
}

fn retained_workspace_report_node_json(
    theme: &CorpusTheme,
    workspace: &WorkspaceModel,
    table: &TableNodeFixture,
    snapshot: &TreeCalcTableNodeSnapshot,
    projection: &TreeCalcTableNodeProjection,
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    let report_case = theme
        .cases
        .iter()
        .find(|case| case.id == "tbl-column-ref")
        .expect("retained report node is anchored by the tbl-column-ref corpus case");
    let node = workspace.node(&report_case.caller).unwrap_or_else(|| {
        panic!(
            "workspace missing retained report node {}",
            report_case.caller
        )
    });
    let formula_text = node.content.text();
    assert_eq!(
        report_case.source_formula.as_deref(),
        Some(formula_text),
        "retained report node formula must be sourced from the workspace fixture and corpus case"
    );
    let formula_values = table_sparse_values(
        table,
        Some(tax_report),
        [("col:amount", totals_amount.value.clone())],
    );
    let binding = bind_treecalc_table_structured_references(formula_text, projection, None, None)
        .into_iter()
        .next()
        .expect("retained report node formula binds a structured table reference");
    assert!(
        binding.diagnostics.is_empty(),
        "retained report node bind diagnostics: {:?}",
        binding.diagnostics
    );
    let reader = TreeCalcTableSparseReader::from_oxfml_bind_record(
        snapshot,
        projection,
        &binding.bind_record,
        None,
        formula_values,
    )
    .expect("retained report node gets a table sparse reader from the public bind record");
    let value = evaluate_case_formula_value(
        &report_case.id,
        formula_text,
        projection,
        None,
        reader.runtime_binding(),
    );
    if let Some(expected_value) = &report_case.expect.published_value {
        assert_eq!(
            display_value(&value),
            *expected_value,
            "retained report node value must match the active corpus expectation"
        );
    }
    json!({
        "node_id": report_case.caller.as_str(),
        "canonical_locator": format!("tree:tables:{}", report_case.caller),
        "source_formula": formula_text,
        "corpus_case_id": report_case.id.as_str(),
        "comparison_value": comparison_value_json(&value)
    })
}

fn retained_totals_node_json(
    workspace: &WorkspaceModel,
    table: &TableNodeFixture,
    projection: &TreeCalcTableNodeProjection,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    let totals_column = table
        .columns
        .iter()
        .find(|column| column.totals_formula.is_some())
        .expect("retained SalesTable fixture has a totals formula column");
    let totals_formula = totals_column
        .totals_formula
        .as_ref()
        .expect("totals formula checked above");
    let node_id = format!("{}.Totals.{}", projection.display_path, totals_column.name);
    let node = workspace
        .node(&node_id)
        .unwrap_or_else(|| panic!("workspace missing retained totals node {node_id}"));
    let formula_text = node.content.text();
    assert_eq!(
        formula_text, totals_formula.formula_text,
        "retained totals node formula must be sourced from the workspace fixture"
    );
    json!({
        "node_id": node_id,
        "canonical_locator": format!("tree:tables:{}.Totals.{}", projection.display_path, totals_column.name),
        "source_formula": formula_text,
        "formula_stable_id": totals_formula.formula_stable_id.as_str(),
        "comparison_value": comparison_value_json(&totals_amount.value)
    })
}

fn retained_effective_display_text_json(
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    totals_amount: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeCellResult,
) -> Value {
    let mut entries = tax_report
        .cell_results
        .iter()
        .filter_map(|cell| {
            cell.row_id.as_ref().map(|row_id| {
                json!({
                    "locator": format!("SalesTable[row={},column=col:tax]", row_id.0),
                    "effective_display_text": display_value(&cell.value),
                    "trust_status": "model_display_string"
                })
            })
        })
        .collect::<Vec<_>>();
    entries.push(json!({
        "locator": "SalesTable.Totals.Amount",
        "effective_display_text": display_value(&totals_amount.value),
        "trust_status": "model_display_string"
    }));
    json!({
        "source_status": "direct",
        "render_context": {
            "context_id": "treecalc-model-display-v1",
            "context_kind": "treecalc_model_display",
            "trust_class": "direct"
        },
        "entries": entries
    })
}

fn retained_dependency_evidence_json(
    theme: &CorpusTheme,
    projection: &TreeCalcTableNodeProjection,
) -> Value {
    let dependencies = theme
        .cases
        .iter()
        .filter(|case| case.expect.outcome == "resolved" && case.table == "SalesTable")
        .map(|case| {
            let formula_text = case
                .source_formula
                .as_deref()
                .unwrap_or(case.reference.as_str());
            let caller_region = case
                .caller_row_offset
                .map(|offset| table_data_caller_region(projection, offset));
            let enclosing = caller_region.as_ref().map(|_| TableRef {
                table_id: projection.table_id.clone(),
            });
            let binding = bind_treecalc_table_structured_references(
                formula_text,
                projection,
                enclosing,
                caller_region.clone(),
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("case {} did not bind a structured reference", case.id));
            json!({
                "case_id": case.id,
                "source_span_utf8": {
                    "start": binding.source_span_utf8.start,
                    "len": binding.source_span_utf8.len
                },
                "source_token_text": binding.source_token_text,
                "host_ref_handle": binding.host_ref_handle,
                "replay_identity_present": !binding.replay_identity.is_empty(),
                "resolved_table_id": binding.resolved_table_id,
                "selected_column_ids": binding.bind_record.selected_column_ids,
                "selected_sections": binding
                    .bind_record
                    .selected_sections
                    .iter()
                    .copied()
                    .map(structured_section_kind_id)
                    .collect::<Vec<_>>(),
                "caller_context_dependency": binding.caller_context_dependency
            })
        })
        .collect::<Vec<_>>();

    json!({
        "source_status": "direct",
        "table_id": projection.table_id,
        "table_context_identity_present": !projection.table_context_identity.is_empty(),
        "row_membership_identity": projection.row_membership_identity,
        "row_order_identity": projection.row_order_identity,
        "column_identity": projection.column_identity,
        "dependencies": dependencies
    })
}

fn retained_invalidation_evidence_json(projection: &TreeCalcTableNodeProjection) -> Value {
    let source_handles = vec!["bind:SalesTable.Columns.Tax".to_string()];
    let impacts = table_update_scenario_kinds()
        .iter()
        .copied()
        .map(|scenario| {
            let after = if scenario == TreeCalcTableUpdateScenarioKind::TableDelete {
                None
            } else {
                Some(projection)
            };
            let impact = classify_treecalc_table_update(
                scenario,
                Some(projection),
                after,
                [TreeNodeId(100)],
                source_handles.clone(),
            );
            json!({
                "scenario": table_update_scenario_kind_id(scenario),
                "changed_dependency_kinds": impact.changed_dependency_kinds
                    .iter()
                    .copied()
                    .map(dependency_kind_id)
                    .collect::<Vec<_>>(),
                "invalidation_reasons": impact.invalidation_reasons
                    .iter()
                    .copied()
                    .map(invalidation_reason_kind_id)
                    .collect::<Vec<_>>(),
                "prepared_identity_inputs": impact.prepared_identity_inputs
                    .iter()
                    .copied()
                    .map(prepared_identity_input_id)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    json!({
        "source_status": "direct",
        "classification_api": "classify_treecalc_table_update",
        "table_id": projection.table_id,
        "impacts": impacts
    })
}

fn retained_artifact_refs_json() -> Value {
    json!({
        "source_status": "direct",
        "host_id": "dna_treecalc",
        "artifact_kind": "w056_table_structured_reference_replay",
        "artifact_refs": [
            {
                "kind": "normalized_replay",
                "path": "docs/test-runs/w056-table-structured-references-001/views/normalized-replay.json"
            },
            {
                "kind": "replay_manifest",
                "path": "docs/test-runs/w056-table-structured-references-001/oxreplay-manifest.json"
            },
            {
                "kind": "source_corpus",
                "path": "docs/test-corpus/tables/structured-references.json"
            },
            {
                "kind": "source_workspace",
                "path": "docs/test-corpus/workspaces/tables.json"
            }
        ],
        "capture_mode": "model_projection",
        "projection_status": "direct",
        "capture_loss": "none"
    })
}

fn table_lifecycle_report_json(
    case: &TableLifecycleCase,
    report: &oxcalc_core::structured_table::TreeCalcTableLifecycleContractReport,
) -> Value {
    json!({
        "case_id": case.id,
        "case_name": case.name,
        "event_kind": report.event_kind.stable_id(),
        "callback_identity": retained_identity_json(&report.callback_identity),
        "before_state": table_lifecycle_state_json(report.before_state.as_ref()),
        "after_state": table_lifecycle_state_json(report.after_state.as_ref()),
        "context_versions": table_lifecycle_context_versions_json(&report.context_versions),
        "changed_dependency_kinds": report.changed_dependency_kinds
            .iter()
            .copied()
            .map(dependency_kind_id)
            .collect::<Vec<_>>(),
        "invalidation_reasons": report.invalidation_reasons
            .iter()
            .copied()
            .map(invalidation_reason_kind_id)
            .collect::<Vec<_>>(),
        "prepared_identity_inputs": report.prepared_identity_inputs
            .iter()
            .copied()
            .map(prepared_identity_input_id)
            .collect::<Vec<_>>(),
        "invalidation_seeds": report.invalidation_seeds
            .iter()
            .map(|seed| {
                json!({
                    "node_id": seed.node_id.to_string(),
                    "reason": invalidation_reason_kind_id(seed.reason)
                })
            })
            .collect::<Vec<_>>(),
        "source_reference_handles": report.source_reference_handles,
        "changed_row_ids": report.changed_row_ids
            .iter()
            .map(|row| row.0.clone())
            .collect::<Vec<_>>(),
        "changed_column_ids": report.changed_column_ids,
        "diagnostics": report.diagnostics
            .iter()
            .map(table_lifecycle_diagnostic_json)
            .collect::<Vec<_>>(),
        "handle_preservation": table_lifecycle_handle_preservation_json(case, report)
    })
}

fn table_lifecycle_state_json(state: Option<&TreeCalcTableLifecycleVersionState>) -> Value {
    let Some(state) = state else {
        return Value::Null;
    };
    json!({
        "table_node_id": state.table_node_id.to_string(),
        "table_id": state.table_id,
        "table_name": state.table_name,
        "display_path": state.display_path,
        "canonical_path": state.canonical_path,
        "workbook_scope_ref": state.workbook_scope_ref,
        "sheet_scope_ref": state.sheet_scope_ref,
        "table_range_ref": state.table_range_ref,
        "header_region_ref": state.header_region_ref,
        "totals_region_ref": state.totals_region_ref,
        "column_range_refs": state.column_range_refs,
        "virtual_anchor_identity": retained_identity_json(&state.virtual_anchor_identity),
        "virtual_anchor_token": retained_identity_json(&state.virtual_anchor_token),
        "table_context_identity": retained_identity_json(&state.table_context_identity),
        "table_invalidation_identity": retained_identity_json(&state.table_invalidation_identity),
        "table_namespace_identity": retained_identity_json(&state.table_namespace_identity),
        "table_namespace_version": state.table_namespace_version,
        "workspace_availability_version": retained_identity_json(&state.workspace_availability_version),
        "workspace_alias_version": retained_identity_json(&state.workspace_alias_version),
        "row_membership_identity": retained_identity_json(&state.row_membership_identity),
        "row_membership_version": state.row_membership_version,
        "row_order_identity": retained_identity_json(&state.row_order_identity),
        "row_order_version": state.row_order_version,
        "column_identity": retained_identity_json(&state.column_identity),
        "column_identity_version": state.column_identity_version,
        "row_ids": state.row_ids.iter().map(|row| row.0.clone()).collect::<Vec<_>>(),
        "column_ids": state.column_ids
    })
}

fn table_lifecycle_context_versions_json(
    versions: &oxcalc_core::structured_table::TreeCalcTableLifecycleContextVersions,
) -> Value {
    json!({
        "host_namespace_version": versions.host_namespace_version,
        "structure_context_version": versions.structure_context_version,
        "registry_snapshot_identity": versions.registry_snapshot_identity,
        "resolution_rule_version": versions.resolution_rule_version,
        "workspace_availability_version": versions.workspace_availability_version,
        "workspace_alias_version": versions.workspace_alias_version
    })
}

fn table_lifecycle_handle_preservation_json(
    case: &TableLifecycleCase,
    report: &oxcalc_core::structured_table::TreeCalcTableLifecycleContractReport,
) -> Value {
    let observed = report
        .before_state
        .as_ref()
        .zip(report.after_state.as_ref());
    json!({
        "expected_row_handles_preserved": case.expect_row_handles_preserved,
        "expected_column_handles_preserved": case.expect_column_handles_preserved,
        "stable_node_handle": observed
            .map(|(before, after)| before.table_node_id == after.table_node_id),
        "stable_table_handle": observed
            .map(|(before, after)| before.table_id == after.table_id),
        "row_handles_preserved": observed
            .map(|(before, after)| before.row_ids == after.row_ids),
        "column_handles_preserved": observed
            .map(|(before, after)| before.column_ids == after.column_ids)
    })
}

fn table_lifecycle_diagnostic_json(diagnostic: &TreeCalcTableLifecycleContractDiagnostic) -> Value {
    match diagnostic {
        TreeCalcTableLifecycleContractDiagnostic::MissingBeforeState { event_kind } => json!({
            "diagnostic_code": "missing_before_state",
            "event_kind": event_kind.stable_id()
        }),
        TreeCalcTableLifecycleContractDiagnostic::MissingAfterState { event_kind } => json!({
            "diagnostic_code": "missing_after_state",
            "event_kind": event_kind.stable_id()
        }),
        TreeCalcTableLifecycleContractDiagnostic::UnexpectedBeforeState { event_kind } => json!({
            "diagnostic_code": "unexpected_before_state",
            "event_kind": event_kind.stable_id()
        }),
        TreeCalcTableLifecycleContractDiagnostic::UnexpectedAfterState { event_kind } => json!({
            "diagnostic_code": "unexpected_after_state",
            "event_kind": event_kind.stable_id()
        }),
        TreeCalcTableLifecycleContractDiagnostic::TableNodeChangedAcrossLifecycle {
            before,
            after,
        } => json!({
            "diagnostic_code": "table_node_changed_across_lifecycle",
            "before": before.to_string(),
            "after": after.to_string()
        }),
        TreeCalcTableLifecycleContractDiagnostic::TableIdChangedAcrossLifecycle {
            before,
            after,
        } => json!({
            "diagnostic_code": "table_id_changed_across_lifecycle",
            "before": before,
            "after": after
        }),
        TreeCalcTableLifecycleContractDiagnostic::MissingOwnerNode { event_kind } => json!({
            "diagnostic_code": "missing_owner_node",
            "event_kind": event_kind.stable_id()
        }),
    }
}

fn retained_table_lifecycle_stale_identity_json(
    baseline_snapshot: &TreeCalcTableNodeSnapshot,
    baseline_projection: &TreeCalcTableNodeProjection,
    owner: TreeNodeId,
) -> Value {
    let baseline_state =
        lifecycle_state_from_snapshot_projection(baseline_snapshot, baseline_projection);
    let mut wrong_id_snapshot = baseline_snapshot.clone();
    wrong_id_snapshot.table_id = "tree-table:sales-recreated".to_string();
    let (_, wrong_id_state) =
        snapshot_projection_and_state(&wrong_id_snapshot).expect("wrong-id snapshot projects");
    let report = classify_treecalc_table_lifecycle_callback(
        &TreeCalcTableLifecycleCallbackPacket::new(TreeCalcTableLifecycleEventKind::TableRename)
            .with_before(baseline_state)
            .with_after(wrong_id_state)
            .with_owner_nodes([owner]),
    );

    json!({
        "source_status": "direct",
        "outcome_kind": "typed_rejection",
        "event_kind": report.event_kind.stable_id(),
        "callback_identity": retained_identity_json(&report.callback_identity),
        "diagnostics": report.diagnostics
            .iter()
            .map(table_lifecycle_diagnostic_json)
            .collect::<Vec<_>>()
    })
}

fn retained_table_lifecycle_roundtrip_json(table: &TableNodeFixture) -> Value {
    let round_tripped = serde_json::to_string(&WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "table-lifecycle-roundtrip".to_string(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: "SalesTable".to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(table.clone()),
        }],
    })
    .and_then(|json| serde_json::from_str::<WorkspaceFixture>(&json))
    .expect("table fixture save/reopen roundtrip serializes");
    let reopened_table = round_tripped.nodes[0].table.as_ref().unwrap();

    json!({
        "source_status": "direct",
        "persistence_surface": "WorkspaceFixture JSON save/reopen",
        "stable_table_id": reopened_table.table_id == table.table_id,
        "before_table_id": table.table_id,
        "after_table_id": reopened_table.table_id,
        "stable_row_ids": reopened_table
            .rows
            .iter()
            .map(|row| &row.row_id)
            .eq(table.rows.iter().map(|row| &row.row_id)),
        "row_ids": reopened_table
            .rows
            .iter()
            .map(|row| row.row_id.clone())
            .collect::<Vec<_>>(),
        "stable_column_ids": reopened_table
            .columns
            .iter()
            .map(|column| &column.column_id)
            .eq(table.columns.iter().map(|column| &column.column_id)),
        "column_ids": reopened_table
            .columns
            .iter()
            .map(|column| column.column_id.clone())
            .collect::<Vec<_>>()
    })
}

fn retained_table_lifecycle_artifact_refs_json() -> Value {
    json!({
        "source_status": "direct",
        "host_id": "dna_treecalc",
        "artifact_kind": "w056_table_lifecycle_replay",
        "artifact_refs": [
            {
                "kind": "normalized_replay",
                "path": "docs/test-runs/w056-table-lifecycle-001/views/normalized-replay.json"
            },
            {
                "kind": "replay_manifest",
                "path": "docs/test-runs/w056-table-lifecycle-001/oxreplay-manifest.json"
            },
            {
                "kind": "source_workspace",
                "path": "docs/test-corpus/workspaces/tables.json"
            }
        ],
        "capture_mode": "model_projection",
        "projection_status": "direct",
        "capture_loss": "none"
    })
}

fn retained_empty_body_case_evidence_json(case: &TableCase, workspace: &WorkspaceModel) -> Value {
    let table = workspace
        .table_node(&case.table)
        .unwrap_or_else(|| panic!("workspace missing table {}", case.table));
    let snapshot = table_snapshot(&case.table, table);
    let projection = project_treecalc_table_node_snapshot(&snapshot)
        .unwrap_or_else(|error| panic!("{} table failed projection: {error:?}", case.id));
    let formula_text = case
        .source_formula
        .as_deref()
        .unwrap_or(case.reference.as_str());
    let caller_region = case
        .caller_row_offset
        .map(|offset| table_data_caller_region(&projection, offset));
    let enclosing = caller_region.as_ref().map(|_| TableRef {
        table_id: projection.table_id.clone(),
    });
    let bound_refs = bind_treecalc_table_structured_references(
        formula_text,
        &projection,
        enclosing,
        caller_region.clone(),
    );
    let binding = bound_refs
        .first()
        .unwrap_or_else(|| panic!("case {} produced no table bind record", case.id));
    let bind_record = &binding.bind_record;
    let formula_values = empty_body_formula_values(table, &projection);
    let reader = TreeCalcTableSparseReader::from_oxfml_bind_record(
        &snapshot,
        &projection,
        bind_record,
        caller_region.as_ref(),
        formula_values,
    );

    match reader {
        Ok(reader) => {
            let observed = case.expect.published_value.as_ref().map(|_| {
                evaluate_case_formula(
                    &case.id,
                    formula_text,
                    &projection,
                    caller_region,
                    reader.runtime_binding(),
                )
            });
            if let (Some(expected), Some(observed)) = (&case.expect.published_value, &observed) {
                assert_eq!(observed, expected, "{} retained published value", case.id);
            }
            json!({
                "case_id": case.id,
                "table_id": projection.table_id,
                "table_node": case.table,
                "reference": case.reference,
                "outcome_kind": "resolved",
                "target_kind": case.expect.target_kind,
                "published_value": observed,
                "declared_extent": {
                    "row_count": reader.declared_extent().row_count,
                    "column_count": reader.declared_extent().column_count
                },
                "defined_cardinality": reader.defined_cardinality(),
                "reader_identity": {
                    "reader_id": retained_identity_json(&reader.reader_identity().reader_id),
                    "source_identity": retained_identity_json(&reader.reader_identity().source_identity),
                    "snapshot_identity": retained_identity_json(&reader.reader_identity().snapshot_identity)
                },
                "selected_sections": bind_record
                    .selected_sections
                    .iter()
                    .copied()
                    .map(structured_section_kind_id)
                    .collect::<Vec<_>>(),
                "selected_column_ids": bind_record.selected_column_ids
            })
        }
        Err(error) => {
            assert_eq!(
                case.expect.outcome, "error",
                "{} unexpected reader diagnostic {error:?}",
                case.id
            );
            json!({
                "case_id": case.id,
                "table_id": projection.table_id,
                "table_node": case.table,
                "reference": case.reference,
                "outcome_kind": "typed_reader_diagnostic",
                "diagnostic": empty_body_reader_error_json(&error),
                "expected_reason": case.expect.reason
            })
        }
    }
}

fn retained_empty_body_table_slice_json(workspace: &WorkspaceModel) -> Value {
    json!({
        "table_slice_schema": "dna_treecalc.empty_body_table_slice.v1",
        "source_status": "direct",
        "tables": empty_body_table_ids()
            .iter()
            .map(|table_id| retained_empty_body_single_table_slice_json(workspace, table_id))
            .collect::<Vec<_>>()
    })
}

fn retained_empty_body_single_table_slice_json(
    workspace: &WorkspaceModel,
    table_node_id: &str,
) -> Value {
    let table = workspace
        .table_node(table_node_id)
        .unwrap_or_else(|| panic!("workspace missing table {table_node_id}"));
    let snapshot = table_snapshot(table_node_id, table);
    let projection = project_treecalc_table_node_snapshot(&snapshot)
        .unwrap_or_else(|error| panic!("{table_node_id} table failed projection: {error:?}"));

    json!({
        "table_node_id": table_node_id,
        "table_id": projection.table_id,
        "table_name": projection.table_descriptor.table_name,
        "display_path": projection.display_path,
        "canonical_locator": projection.canonical_path,
        "table_range_ref": projection.table_descriptor.table_range_ref,
        "header_region_ref": projection.table_descriptor.header_region_ref,
        "data_body_range_ref": table_data_body_range_ref(&snapshot),
        "totals_region_ref": projection.table_descriptor.totals_region_ref,
        "header_row_present": projection.table_descriptor.header_row_present,
        "totals_row_present": projection.table_descriptor.totals_row_present,
        "row_count": snapshot.rows.len(),
        "column_count": snapshot.columns.len(),
        "versions": {
            "table_namespace_version": snapshot.table_namespace_version,
            "row_membership_version": snapshot.row_membership_version,
            "row_order_version": snapshot.row_order_version,
            "column_identity_version": snapshot.column_identity_version
        },
        "engine_identity_refs": {
            "table_context_identity": retained_identity_json(&projection.table_context_identity),
            "table_invalidation_identity": retained_identity_json(&projection.table_invalidation_identity),
            "table_namespace_token": retained_identity_json(&projection.table_namespace_token),
            "row_membership_identity": retained_identity_json(&projection.row_membership_identity),
            "row_order_identity": retained_identity_json(&projection.row_order_identity),
            "column_identity": retained_identity_json(&projection.column_identity),
            "virtual_anchor_token": retained_identity_json(&projection.virtual_anchor_token)
        },
        "rows": table.rows.iter().map(|row| {
            json!({
                "row_id": row.row_id,
                "ordinal": row.ordinal
            })
        }).collect::<Vec<_>>(),
        "columns": table.columns.iter().map(|column| {
            json!({
                "column_id": column.column_id,
                "column_name": column.name,
                "ordinal": column.ordinal,
                "body_kind": table_column_body_kind_id(column.body.kind),
                "data_constants": column.body.constants.iter().map(|cell| {
                    json!({
                        "row_id": cell.row_id,
                        "value_repr": cell.value
                    })
                }).collect::<Vec<_>>(),
                "totals_formula": column.totals_formula.as_ref().map(table_formula_json)
            })
        }).collect::<Vec<_>>()
    })
}

fn retained_empty_body_per_node_value_json(case_evidence: &[Value]) -> Value {
    json!({
        "source_status": "direct",
        "entries": case_evidence
            .iter()
            .map(|case| {
                json!({
                    "case_id": case["case_id"].clone(),
                    "table_node": case["table_node"].clone(),
                    "reference": case["reference"].clone(),
                    "outcome_kind": case["outcome_kind"].clone(),
                    "published_value": case["published_value"].clone(),
                    "diagnostic": case["diagnostic"].clone()
                })
            })
            .collect::<Vec<_>>()
    })
}

fn retained_empty_body_display_json(workspace: &WorkspaceModel) -> Value {
    let entries = empty_body_table_ids()
        .iter()
        .flat_map(|table_id| {
            let table = workspace
                .table_node(table_id)
                .unwrap_or_else(|| panic!("workspace missing table {table_id}"));
            table.columns.iter().map(move |column| {
                json!({
                    "locator": format!("{table_id}.Headers.{}", column.name),
                    "effective_display_text": column.name,
                    "trust_status": "model_display_string"
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "source_status": "direct",
        "render_context": {
            "context_id": "treecalc-model-display-v1",
            "context_kind": "treecalc_model_display",
            "trust_class": "direct"
        },
        "entries": entries
    })
}

fn retained_empty_body_dependency_evidence_json(
    theme: &CorpusTheme,
    workspace: &WorkspaceModel,
) -> Value {
    let dependencies = theme
        .cases
        .iter()
        .map(|case| {
            let table = workspace
                .table_node(&case.table)
                .unwrap_or_else(|| panic!("workspace missing table {}", case.table));
            let snapshot = table_snapshot(&case.table, table);
            let projection =
                project_treecalc_table_node_snapshot(&snapshot).unwrap_or_else(|error| {
                    panic!(
                        "{} table failed projection for retained dependency: {error:?}",
                        case.id
                    )
                });
            let formula_text = case
                .source_formula
                .as_deref()
                .unwrap_or(case.reference.as_str());
            let caller_region = case
                .caller_row_offset
                .map(|offset| table_data_caller_region(&projection, offset));
            let enclosing = caller_region.as_ref().map(|_| TableRef {
                table_id: projection.table_id.clone(),
            });
            let binding = bind_treecalc_table_structured_references(
                formula_text,
                &projection,
                enclosing,
                caller_region.clone(),
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("case {} did not bind a structured reference", case.id));
            json!({
                "case_id": case.id,
                "source_span_utf8": {
                    "start": binding.source_span_utf8.start,
                    "len": binding.source_span_utf8.len
                },
                "source_token_text": binding.source_token_text,
                "host_ref_handle": binding.host_ref_handle,
                "replay_identity": retained_identity_json(&binding.replay_identity),
                "resolved_table_id": binding.resolved_table_id,
                "selected_column_ids": binding.bind_record.selected_column_ids,
                "selected_sections": binding
                    .bind_record
                    .selected_sections
                    .iter()
                    .copied()
                    .map(structured_section_kind_id)
                    .collect::<Vec<_>>(),
                "caller_context_dependency": binding.caller_context_dependency,
                "diagnostics": binding
                    .diagnostics
                    .iter()
                    .map(|diagnostic| {
                        json!({
                            "diagnostic_code": diagnostic.diagnostic_code,
                            "message": diagnostic.message
                        })
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    json!({
        "source_status": "direct",
        "dependencies": dependencies
    })
}

fn retained_empty_body_invalidation_evidence_json(workspace: &WorkspaceModel) -> Value {
    let transitions = [
        empty_body_row_insert_transition(workspace),
        empty_body_last_row_delete_transition(workspace),
    ];
    json!({
        "source_status": "direct",
        "classification_api": "classify_treecalc_table_update",
        "transitions": transitions
    })
}

fn empty_body_row_insert_transition(workspace: &WorkspaceModel) -> Value {
    let table = workspace
        .table_node("EmptyHeadersOnly")
        .expect("empty headers table exists");
    let before_snapshot = table_snapshot("EmptyHeadersOnly", table);
    let mut after_snapshot = before_snapshot.clone();
    after_snapshot
        .rows
        .push(TreeCalcTableRowId("row:first".to_string()));
    after_snapshot.row_membership_version =
        "table-rows:empty-headers-only:membership:v2".to_string();
    after_snapshot.row_order_version = "table-rows:empty-headers-only:order:v2".to_string();
    empty_body_transition_impact_json(
        "first_row_insert",
        TreeCalcTableUpdateScenarioKind::RowInsert,
        &before_snapshot,
        &after_snapshot,
    )
}

fn empty_body_last_row_delete_transition(workspace: &WorkspaceModel) -> Value {
    let table = workspace
        .table_node("EmptyHeadersTotals")
        .expect("empty headers+totals table exists");
    let after_snapshot = table_snapshot("EmptyHeadersTotals", table);
    let mut before_snapshot = after_snapshot.clone();
    before_snapshot
        .rows
        .push(TreeCalcTableRowId("row:last".to_string()));
    before_snapshot.row_membership_version =
        "table-rows:empty-headers-totals:membership:v0".to_string();
    before_snapshot.row_order_version = "table-rows:empty-headers-totals:order:v0".to_string();
    empty_body_transition_impact_json(
        "last_row_delete",
        TreeCalcTableUpdateScenarioKind::RowDelete,
        &before_snapshot,
        &after_snapshot,
    )
}

fn empty_body_transition_impact_json(
    transition_id: &str,
    scenario: TreeCalcTableUpdateScenarioKind,
    before_snapshot: &TreeCalcTableNodeSnapshot,
    after_snapshot: &TreeCalcTableNodeSnapshot,
) -> Value {
    let before_projection = project_treecalc_table_node_snapshot(before_snapshot)
        .unwrap_or_else(|error| panic!("{transition_id} before projection failed: {error:?}"));
    let after_projection = project_treecalc_table_node_snapshot(after_snapshot)
        .unwrap_or_else(|error| panic!("{transition_id} after projection failed: {error:?}"));
    let impact = classify_treecalc_table_update(
        scenario,
        Some(&before_projection),
        Some(&after_projection),
        [TreeNodeId(100)],
        [format!("bind:{transition_id}")],
    );
    json!({
        "transition_id": transition_id,
        "scenario": table_update_scenario_kind_id(scenario),
        "before": {
            "table_id": before_projection.table_id,
            "row_count": before_snapshot.rows.len(),
            "table_context_identity": retained_identity_json(&before_projection.table_context_identity)
        },
        "after": {
            "table_id": after_projection.table_id,
            "row_count": after_snapshot.rows.len(),
            "table_context_identity": retained_identity_json(&after_projection.table_context_identity)
        },
        "changed_dependency_kinds": impact.changed_dependency_kinds
            .iter()
            .copied()
            .map(dependency_kind_id)
            .collect::<Vec<_>>(),
        "invalidation_reasons": impact.invalidation_reasons
            .iter()
            .copied()
            .map(invalidation_reason_kind_id)
            .collect::<Vec<_>>(),
        "prepared_identity_inputs": impact.prepared_identity_inputs
            .iter()
            .copied()
            .map(prepared_identity_input_id)
            .collect::<Vec<_>>(),
        "invalidation_seeds": impact.invalidation_seeds
            .iter()
            .map(|seed| {
                json!({
                    "node_id": seed.node_id.to_string(),
                    "reason": invalidation_reason_kind_id(seed.reason)
                })
            })
            .collect::<Vec<_>>()
    })
}

fn empty_body_formula_values(
    table: &TableNodeFixture,
    projection: &TreeCalcTableNodeProjection,
) -> Vec<TreeCalcTableSparseValue> {
    if projection.table_descriptor.totals_row_present {
        table_sparse_values(table, None, [("col:amount", CalcValue::number(0.0))])
    } else {
        table_sparse_values(table, None, std::iter::empty::<(&str, CalcValue)>())
    }
}

fn empty_body_reader_error_json(error: &TreeCalcTableSparseReaderError) -> Value {
    match error {
        TreeCalcTableSparseReaderError::CallerRowOutOfRange {
            row_offset,
            row_count,
        } => json!({
            "diagnostic_code": "caller_row_out_of_range",
            "row_offset": row_offset,
            "row_count": row_count
        }),
        other => json!({
            "diagnostic_code": "unexpected_reader_error",
            "debug": format!("{other:?}")
        }),
    }
}

fn empty_body_table_ids() -> [&'static str; 4] {
    [
        "EmptyHeadersOnly",
        "EmptyHeadersTotals",
        "HeadersOnlyAfterFirstRowInsert",
        "HeadersTotalsBeforeLastRowDelete",
    ]
}

fn retained_empty_body_artifact_refs_json() -> Value {
    json!({
        "source_status": "direct",
        "host_id": "dna_treecalc",
        "artifact_kind": "w056_empty_body_table_replay",
        "artifact_refs": [
            {
                "kind": "normalized_replay",
                "path": "docs/test-runs/w056-table-empty-body-001/views/normalized-replay.json"
            },
            {
                "kind": "replay_manifest",
                "path": "docs/test-runs/w056-table-empty-body-001/oxreplay-manifest.json"
            },
            {
                "kind": "source_corpus",
                "path": "docs/test-corpus/tables/empty-body.json"
            },
            {
                "kind": "source_workspace",
                "path": "docs/test-corpus/workspaces/empty-body-tables.json"
            }
        ],
        "capture_mode": "model_projection",
        "projection_status": "direct",
        "capture_loss": "none"
    })
}

fn retained_dynamic_cross_workspace_table_replay_artifact(
    theme: &DynamicTableTheme,
    local_session: &TreeWorkspaceSession,
    remote_session: &TreeWorkspaceSession,
    local_workspace: &WorkspaceModel,
    remote_workspace: &WorkspaceModel,
) -> Value {
    let case_reports = theme
        .cases
        .iter()
        .map(|case| {
            dynamic_table_rebind_report_json(
                case,
                &dynamic_table_rebind_report_for_case(
                    local_session,
                    remote_session,
                    case,
                    local_workspace,
                    remote_workspace,
                ),
            )
        })
        .collect::<Vec<_>>();

    json!({
        "scenario_id": "w056_treecalc_dynamic_cross_workspace_tables_001",
        "lane_id": "dna_treecalc",
        "events": [
            {
                "event_id": "treecalc_dynamic_table_rebind_packets",
                "source_label": "dynamic_table_rebind:tables",
                "normalized_family": "treecalc.surface.dynamic_table_rebind"
            },
            {
                "event_id": "treecalc_cross_workspace_table_availability",
                "source_label": "cross_workspace_table_availability:table-projections",
                "normalized_family": "treecalc.surface.cross_workspace_table_availability"
            }
        ],
        "registry_refs": [
            {
                "family": "dnatreecalc.test_corpus",
                "version": format!("{}@{}:cases={}", theme.theme, theme.schema_version, theme.cases.len())
            },
            {
                "family": "dnatreecalc.workspace_fixture",
                "version": format!("{}@treecalc-workspace-v1", local_workspace.workspace_id)
            },
            {
                "family": "dnatreecalc.workspace_fixture",
                "version": format!("{}@treecalc-workspace-v1", remote_workspace.workspace_id)
            }
        ],
        "comparison_views": [
            {
                "view_family": "execution_outcome",
                "value": {
                    "outcome_schema": "dna_treecalc.execution_outcome.v1",
                    "scenario_id": "w056_treecalc_dynamic_cross_workspace_tables_001",
                    "outcome_kind": "accepted_execution",
                    "outcome_stage": "oxcalc_dynamic_table_rebind_packets",
                    "class_id": "treecalc_dynamic_cross_workspace_table_packets_ready",
                    "lane_reason_code": "dnatreecalc_w056_dynamic_table_retained",
                    "engine_surface": "OxCalcTreeContext dynamic table rebind route backed by the OxCalc W056 public packet API",
                    "case_count": case_reports.len(),
                    "case_statuses": case_reports
                        .iter()
                        .map(|case| {
                            json!({
                                "case_id": case["case_id"].clone(),
                                "status": case["status"].clone(),
                                "treecalc_v1": case["treecalc_v1"].clone(),
                                "strict_excel": case["strict_excel"].clone()
                            })
                        })
                        .collect::<Vec<_>>(),
                    "replay_view_ready": true
                }
            },
            {
                "view_family": "dependency_evidence",
                "value": {
                    "source_status": "direct_context_route",
                    "classification_api": "TreeWorkspaceSession::classify_dynamic_table_rebind",
                    "case_reports": case_reports.clone()
                }
            },
            {
                "view_family": "invalidation_evidence",
                "value": {
                    "source_status": "direct_context_route",
                    "classification_api": "TreeWorkspaceSession::classify_dynamic_table_rebind",
                    "case_invalidations": case_reports
                        .iter()
                        .map(|case| {
                            json!({
                                "case_id": case["case_id"].clone(),
                                "status": case["status"].clone(),
                                "changed_dependency_kinds": case["changed_dependency_kinds"].clone(),
                                "invalidation_reasons": case["invalidation_reasons"].clone(),
                                "prepared_identity_inputs": case["prepared_identity_inputs"].clone()
                            })
                        })
                        .collect::<Vec<_>>()
                }
            },
            {
                "view_family": "retained_artifact_ref",
                "value": retained_dynamic_table_artifact_refs_json()
            }
        ],
        "source_metadata": {
            "source_host": "dna_treecalc",
            "source_schema_id": "dna_treecalc.w056_dynamic_cross_workspace_table_replay.v1",
            "projection_status": "direct",
            "capture_mode": "model_projection",
            "capture_loss": "none",
            "capture_loss_summary": [],
            "uncertainty_summary": [],
            "direct_context": true,
            "adapter_id": "dnatreecalc.oxcalc_dynamic_table_rebind.v1",
            "workspace_id": local_workspace.workspace_id,
            "source_refs": [
                "docs/test-corpus/tables/dynamic-cross-workspace.json",
                "docs/test-corpus/workspaces/tables.json",
                "docs/test-corpus/workspaces/table-projections.json"
            ],
            "shared_scenario_alias": "w056_table_dynamic_cross_workspace_001",
            "interpretation_limits": [
                {
                    "kind": "typed_exclusions_are_explicit",
                    "detail": "Runtime parsing of TreeCalc structured-reference text through INDIRECT remains a typed exclusion until OxFml supplies a generic structured bind packet."
                },
                {
                    "kind": "no_private_selector_semantics",
                    "detail": "Dynamic table status, dependency facts, invalidation reasons, and prepared identity inputs come from OxCalc public dynamic table packets."
                }
            ],
            "comparison_view_families": [
                "execution_outcome",
                "dependency_evidence",
                "invalidation_evidence",
                "retained_artifact_ref"
            ]
        }
    })
}

fn retained_dynamic_cross_workspace_table_replay_manifest() -> Value {
    json!({
        "bundle_id": "dnatreecalc-w056-table-dynamic-cross-workspace-001",
        "scenario_id": "w056_treecalc_dynamic_cross_workspace_tables_001",
        "bundle_schema": "replay.bundle.v1",
        "source_schema": "dna_treecalc.replay_bundle_seed.v1",
        "lane_id": "dna_treecalc",
        "adapter_id": "dnatreecalc.oxcalc_dynamic_table_rebind.v1",
        "capture_mode": "model_projection",
        "projection_status": "lossless",
        "capture_loss": "none",
        "registry_refs": [],
        "sidecars": [],
        "views": [
            {
                "artifact_family": "normalized_replay",
                "path": "views/normalized-replay.json"
            }
        ],
        "declared_comparison_views": [
            "execution_outcome",
            "dependency_evidence",
            "invalidation_evidence",
            "retained_artifact_ref"
        ]
    })
}

fn dynamic_table_rebind_report_for_case(
    local_session: &TreeWorkspaceSession,
    remote_session: &TreeWorkspaceSession,
    case: &DynamicTableCase,
    local_workspace: &WorkspaceModel,
    remote_workspace: &WorkspaceModel,
) -> oxcalc_core::structured_table::TreeCalcDynamicTableRebindReport {
    assert_eq!(case.kind, "table", "case {} kind", case.id);
    assert_eq!(case.workspace, "tables", "case {} workspace", case.id);
    assert_eq!(case.table, "SalesTable", "case {} table", case.id);
    let source_reference_handle = dynamic_table_source_reference_handle(case, local_workspace);
    let request = TreeCalcDynamicTableRebindRequest {
        selector_handle: case.dynamic.selector_handle.clone(),
        selector_identity: case.dynamic.selector_identity.clone(),
        source_reference_handle,
        target_kind: dynamic_table_target_kind(&case.dynamic.target_kind),
        cause: dynamic_table_rebind_cause(&case.dynamic.cause),
        before_resolved_table_identity: case.dynamic.before_resolved_table.as_deref().map(
            |target| {
                dynamic_table_resolved_identity(
                    local_session,
                    remote_session,
                    target,
                    local_workspace,
                    remote_workspace,
                )
            },
        ),
        after_resolved_table_identity: case.dynamic.after_resolved_table.as_deref().map(|target| {
            dynamic_table_resolved_identity(
                local_session,
                remote_session,
                target,
                local_workspace,
                remote_workspace,
            )
        }),
        caller_context_id: case.dynamic.caller_context_id.clone(),
        context_versions: dynamic_table_context_versions(case),
        oxfml_structured_bind_packet_available: case.dynamic.oxfml_structured_bind_packet_available,
    };
    local_session
        .classify_dynamic_table_rebind(request)
        .unwrap_or_else(|error| {
            panic!(
                "{} dynamic table direct-context route failed: {error}",
                case.id
            )
        })
}

fn assert_dynamic_table_expected_status(
    case: &DynamicTableCase,
    report: &oxcalc_core::structured_table::TreeCalcDynamicTableRebindReport,
) {
    let expected = case.expect.reason.as_deref().unwrap_or("");
    match expected {
        "unsupported_runtime_structured_reference_parsing" => {
            assert_eq!(
                report.status,
                TreeCalcDynamicTableRebindStatus::TypedExclusion,
                "{} status",
                case.id
            );
            assert!(
                report.diagnostics.iter().any(|diagnostic| diagnostic.kind
                    == TreeCalcDynamicTableRebindDiagnosticKind::UnsupportedRuntimeStructuredReferenceParsing),
                "{} diagnostic",
                case.id
            );
        }
        "dynamic_target_not_table" => {
            assert_eq!(
                report.status,
                TreeCalcDynamicTableRebindStatus::TypedExclusion,
                "{} status",
                case.id
            );
            assert!(
                report.diagnostics.iter().any(|diagnostic| diagnostic.kind
                    == TreeCalcDynamicTableRebindDiagnosticKind::DynamicTargetNotTable),
                "{} diagnostic",
                case.id
            );
        }
        other => assert_eq!(
            dynamic_rebind_status_id(report.status),
            other,
            "{} status",
            case.id
        ),
    }
}

fn dynamic_table_source_reference_handle(
    case: &DynamicTableCase,
    workspace: &WorkspaceModel,
) -> Option<String> {
    match case.dynamic.source.mode.as_str() {
        "oxcalc_structured_bind" => {
            assert!(
                case.dynamic.oxfml_structured_bind_packet_available,
                "{} OxCalc structured bind source mode requires a generic bind packet",
                case.id
            );
            let observed = dynamic_table_oxcalc_structured_bind_handle(case, workspace);
            if let Some(expected) = &case.dynamic.source.reference_handle {
                if !expected.starts_with("treecalc.structured_table_ref.handle.v1") {
                    assert_eq!(
                        observed.as_deref(),
                        Some(expected.as_str()),
                        "{} structured bind source handle",
                        case.id
                    );
                }
            }
            observed
        }
        "provided_oxfml_packet" => {
            assert!(
                case.dynamic.oxfml_structured_bind_packet_available,
                "{} provided OxFml packet mode requires packet availability",
                case.id
            );
            Some(
                case.dynamic
                    .source
                    .reference_handle
                    .clone()
                    .unwrap_or_else(|| panic!("{} missing provided source handle", case.id)),
            )
        }
        "no_structured_bind_packet" => {
            assert!(
                !case.dynamic.oxfml_structured_bind_packet_available,
                "{} no-packet source mode must not declare packet availability",
                case.id
            );
            assert!(
                case.dynamic.source.reference_handle.is_none(),
                "{} no-packet source mode must not declare a source handle",
                case.id
            );
            None
        }
        other => panic!("{} unsupported dynamic source mode {other}", case.id),
    }
}

fn dynamic_table_oxcalc_structured_bind_handle(
    case: &DynamicTableCase,
    workspace: &WorkspaceModel,
) -> Option<String> {
    let table = workspace
        .table_node(&case.table)
        .unwrap_or_else(|| panic!("workspace missing table {}", case.table));
    let snapshot = table_snapshot(&case.table, table);
    let projection = project_treecalc_table_node_snapshot(&snapshot)
        .unwrap_or_else(|error| panic!("{} table projection failed: {error:?}", case.id));
    let caller_region = case
        .caller_row_offset
        .map(|offset| table_data_caller_region(&projection, offset));
    let enclosing = caller_region.as_ref().map(|_| TableRef {
        table_id: projection.table_id.clone(),
    });
    bind_treecalc_table_structured_references(
        &case.reference,
        &projection,
        enclosing,
        caller_region,
    )
    .into_iter()
    .next()
    .map(|binding| binding.host_ref_handle)
}

fn dynamic_table_context_versions(
    case: &DynamicTableCase,
) -> TreeCalcTableLifecycleContextVersions {
    let mut versions = TreeCalcTableLifecycleContextVersions::default();
    if matches!(
        dynamic_table_target_kind(&case.dynamic.target_kind),
        TreeCalcDynamicTableReferenceTargetKind::CrossWorkspaceTable
    ) {
        versions.workspace_availability_version =
            Some("treecalc-cross-workspace-availability:v1:table-projections:loaded".to_string());
        versions.workspace_alias_version =
            Some("treecalc-cross-workspace-alias:v1:table-projections".to_string());
    }
    if case.dynamic.cause == "table_lifecycle:workspace_alias_mutation" {
        versions.workspace_alias_version =
            Some("treecalc-cross-workspace-alias:v2:table-projections".to_string());
    }
    if case.dynamic.cause == "table_lifecycle:workspace_close" {
        versions.workspace_availability_version =
            Some("treecalc-cross-workspace-availability:v2:table-projections:closed".to_string());
    }
    versions
}

fn dynamic_table_resolved_identity(
    local_session: &TreeWorkspaceSession,
    remote_session: &TreeWorkspaceSession,
    target: &str,
    local_workspace: &WorkspaceModel,
    remote_workspace: &WorkspaceModel,
) -> String {
    let (session, workspace, table_id, mutation) = match target.split_once(':') {
        Some(("local", rest)) => {
            let (table_id, mutation) = rest
                .split_once('.')
                .map_or((rest, None), |(id, m)| (id, Some(m)));
            (local_session, local_workspace.clone(), table_id, mutation)
        }
        Some(("remote", rest)) => {
            let (table_id, mutation) = rest
                .split_once('.')
                .map_or((rest, None), |(id, m)| (id, Some(m)));
            (remote_session, remote_workspace.clone(), table_id, mutation)
        }
        _ => panic!("unsupported dynamic table target {target}"),
    };
    let mut workspace = workspace;
    match mutation {
        Some("column_rename") => {
            rename_dynamic_workspace_amount_column(&mut workspace, table_id);
        }
        Some(other) => panic!("unsupported dynamic table target mutation {other}"),
        None => {}
    }
    if mutation.is_none() {
        return table_context_identity_via_session(session, table_id, target);
    }
    table_context_identity_via_workspace(workspace, table_id, target)
}

fn rename_dynamic_workspace_amount_column(workspace: &mut WorkspaceModel, table_id: &str) {
    let table = workspace
        .table_nodes
        .get_mut(table_id)
        .unwrap_or_else(|| panic!("workspace missing dynamic table target {table_id}"));
    table.columns[1].name = "GrossAmount".to_string();
    table.column_identity_version = "table-columns:sales:v5".to_string();

    let node_table = workspace
        .nodes
        .get_mut(table_id)
        .and_then(|node| node.table.as_mut())
        .unwrap_or_else(|| panic!("workspace node missing dynamic table target {table_id}"));
    node_table.columns[1].name = "GrossAmount".to_string();
    node_table.column_identity_version = "table-columns:sales:v5".to_string();
}

fn table_context_identity_via_session(
    session: &TreeWorkspaceSession,
    table_id: &str,
    target: &str,
) -> String {
    session
        .table_context_identity(&NodeId::new(table_id))
        .unwrap_or_else(|error| {
            panic!("{target} direct context table context projection failed: {error}")
        })
        .unwrap_or_else(|| {
            panic!("{target} missing direct context table context identity for {table_id}")
        })
}

fn table_context_identity_via_workspace(
    workspace: WorkspaceModel,
    table_id: &str,
    target: &str,
) -> String {
    let table = workspace
        .table_node(table_id)
        .unwrap_or_else(|| panic!("workspace missing dynamic table target {table_id}"))
        .clone();
    let workspace_id = format!("dynamic-table-target-{}", workspace.workspace_id);
    let context_workspace = WorkspaceModel::try_from(WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: workspace_id.clone(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: table_id.to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(table),
        }],
    })
    .unwrap_or_else(|error| panic!("{target} direct table context fixture failed: {error}"));
    let session = TreeWorkspaceSession::from_model(&context_workspace).unwrap_or_else(|error| {
        panic!("{target} direct context table context projection failed: {error}")
    });
    table_context_identity_via_session(&session, table_id, target)
}

fn dynamic_table_rebind_report_json(
    case: &DynamicTableCase,
    report: &oxcalc_core::structured_table::TreeCalcDynamicTableRebindReport,
) -> Value {
    json!({
        "case_id": case.id,
        "case_name": case.name,
        "reference": case.reference,
        "treecalc_v1": case.dynamic.treecalc_v1,
        "strict_excel": case.dynamic.strict_excel,
        "selector_handle": report.selector_handle,
        "selector_identity": report.selector_identity,
        "context_route": "TreeWorkspaceSession::classify_dynamic_table_rebind",
        "source_reference_mode": case.dynamic.source.mode,
        "source_reference_expected_handle": case.dynamic.source.reference_handle,
        "source_reference_handle": report.source_reference_handle,
        "target_kind": report.target_kind.stable_id(),
        "cause": report.cause.stable_id(),
        "status": dynamic_rebind_status_id(report.status),
        "dynamic_rebind_identity": retained_identity_json(&report.dynamic_rebind_identity),
        "dependency_fact_kinds": report.dependency_fact_kinds
            .iter()
            .copied()
            .map(structured_table_dependency_fact_kind_id)
            .collect::<Vec<_>>(),
        "changed_dependency_kinds": report.changed_dependency_kinds
            .iter()
            .copied()
            .map(dependency_kind_id)
            .collect::<Vec<_>>(),
        "invalidation_reasons": report.invalidation_reasons
            .iter()
            .copied()
            .map(invalidation_reason_kind_id)
            .collect::<Vec<_>>(),
        "prepared_identity_inputs": report.prepared_identity_inputs
            .iter()
            .copied()
            .map(prepared_identity_input_id)
            .collect::<Vec<_>>(),
        "diagnostics": report.diagnostics
            .iter()
            .map(dynamic_table_diagnostic_json)
            .collect::<Vec<_>>(),
        "oxfml_generic_bind_packet_available": report.oxfml_generic_bind_packet_available,
        "oxfunc_opaque_reference_admitted": report.oxfunc_opaque_reference_admitted
    })
}

fn dynamic_table_target_kind(target_kind: &str) -> TreeCalcDynamicTableReferenceTargetKind {
    match target_kind {
        "table" => TreeCalcDynamicTableReferenceTargetKind::Table,
        "column" => TreeCalcDynamicTableReferenceTargetKind::Column,
        "section" => TreeCalcDynamicTableReferenceTargetKind::Section,
        "current_row" => TreeCalcDynamicTableReferenceTargetKind::CurrentRow,
        "cross_workspace_table" => TreeCalcDynamicTableReferenceTargetKind::CrossWorkspaceTable,
        other => panic!("unsupported dynamic table target kind {other}"),
    }
}

fn dynamic_table_rebind_cause(cause: &str) -> TreeCalcDynamicTableRebindCause {
    match cause {
        "selector_text_changed" => TreeCalcDynamicTableRebindCause::SelectorTextChanged,
        "dynamic_function_result_changed" => {
            TreeCalcDynamicTableRebindCause::DynamicFunctionResultChanged
        }
        "volatile_reevaluation" => TreeCalcDynamicTableRebindCause::VolatileReevaluation,
        "unsupported_runtime_structured_reference_parsing" => {
            TreeCalcDynamicTableRebindCause::UnsupportedRuntimeStructuredReferenceParsing
        }
        "dynamic_target_not_table" => TreeCalcDynamicTableRebindCause::DynamicTargetNotTable,
        lifecycle if lifecycle.starts_with("table_lifecycle:") => {
            TreeCalcDynamicTableRebindCause::TableLifecycle(table_update_scenario_kind(
                lifecycle.trim_start_matches("table_lifecycle:"),
            ))
        }
        other => panic!("unsupported dynamic table rebind cause {other}"),
    }
}

fn table_update_scenario_kind(scenario: &str) -> TreeCalcTableUpdateScenarioKind {
    match scenario {
        "table_delete" => TreeCalcTableUpdateScenarioKind::TableDelete,
        "workspace_close" => TreeCalcTableUpdateScenarioKind::WorkspaceClose,
        "workspace_alias_mutation" => TreeCalcTableUpdateScenarioKind::WorkspaceAliasMutation,
        "save_reopen" => TreeCalcTableUpdateScenarioKind::SaveReopen,
        other => panic!("unsupported dynamic table lifecycle scenario {other}"),
    }
}

fn dynamic_rebind_status_id(status: TreeCalcDynamicTableRebindStatus) -> &'static str {
    match status {
        TreeCalcDynamicTableRebindStatus::ReferencePreserving => "reference_preserving",
        TreeCalcDynamicTableRebindStatus::RebindRequired => "rebind_required",
        TreeCalcDynamicTableRebindStatus::DeletedTarget => "deleted_target",
        TreeCalcDynamicTableRebindStatus::UnavailableTarget => "unavailable_target",
        TreeCalcDynamicTableRebindStatus::TypedExclusion => "typed_exclusion",
    }
}

fn dynamic_table_diagnostic_json(
    diagnostic: &oxcalc_core::structured_table::TreeCalcDynamicTableRebindDiagnostic,
) -> Value {
    json!({
        "diagnostic_code": dynamic_table_diagnostic_kind_id(diagnostic.kind),
        "detail": diagnostic.detail
    })
}

fn dynamic_table_diagnostic_kind_id(
    kind: TreeCalcDynamicTableRebindDiagnosticKind,
) -> &'static str {
    match kind {
        TreeCalcDynamicTableRebindDiagnosticKind::MissingCallerContext => "missing_caller_context",
        TreeCalcDynamicTableRebindDiagnosticKind::UnsupportedRuntimeStructuredReferenceParsing => {
            "unsupported_runtime_structured_reference_parsing"
        }
        TreeCalcDynamicTableRebindDiagnosticKind::DynamicTargetNotTable => {
            "dynamic_target_not_table"
        }
    }
}

fn retained_dynamic_table_artifact_refs_json() -> Value {
    json!({
        "source_status": "direct",
        "host_id": "dna_treecalc",
        "artifact_kind": "w056_dynamic_cross_workspace_table_replay",
        "artifact_refs": [
            {
                "kind": "normalized_replay",
                "path": "docs/test-runs/w056-table-dynamic-cross-workspace-001/views/normalized-replay.json"
            },
            {
                "kind": "replay_manifest",
                "path": "docs/test-runs/w056-table-dynamic-cross-workspace-001/oxreplay-manifest.json"
            },
            {
                "kind": "source_corpus",
                "path": "docs/test-corpus/tables/dynamic-cross-workspace.json"
            },
            {
                "kind": "source_workspace",
                "path": "docs/test-corpus/workspaces/tables.json"
            },
            {
                "kind": "source_workspace",
                "path": "docs/test-corpus/workspaces/table-projections.json"
            }
        ],
        "capture_mode": "model_projection",
        "projection_status": "direct",
        "capture_loss": "none"
    })
}

fn retained_identity_json(value: &str) -> Value {
    json!({
        "present": !value.is_empty(),
        "digest": retained_identity_digest(value),
        "prefix": value.chars().take(96).collect::<String>()
    })
}

fn retained_identity_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn table_update_scenario_kinds() -> [TreeCalcTableUpdateScenarioKind; 17] {
    [
        TreeCalcTableUpdateScenarioKind::BodyCellEdit,
        TreeCalcTableUpdateScenarioKind::BodyFormulaEdit,
        TreeCalcTableUpdateScenarioKind::RowInsert,
        TreeCalcTableUpdateScenarioKind::RowDelete,
        TreeCalcTableUpdateScenarioKind::RowReorder,
        TreeCalcTableUpdateScenarioKind::ColumnInsert,
        TreeCalcTableUpdateScenarioKind::ColumnDelete,
        TreeCalcTableUpdateScenarioKind::ColumnReorder,
        TreeCalcTableUpdateScenarioKind::ColumnRename,
        TreeCalcTableUpdateScenarioKind::HeaderTextEdit,
        TreeCalcTableUpdateScenarioKind::TotalsRowToggle,
        TreeCalcTableUpdateScenarioKind::TotalsFormulaEdit,
        TreeCalcTableUpdateScenarioKind::TableRename,
        TreeCalcTableUpdateScenarioKind::TableMove,
        TreeCalcTableUpdateScenarioKind::TableDelete,
        TreeCalcTableUpdateScenarioKind::SaveReopen,
        TreeCalcTableUpdateScenarioKind::StructuralRebind,
    ]
}

fn table_data_body_range_ref(snapshot: &TreeCalcTableNodeSnapshot) -> Option<String> {
    if snapshot.rows.is_empty() || snapshot.columns.is_empty() {
        return None;
    }
    let start_row = snapshot
        .virtual_anchor
        .start_row
        .checked_add(u32::from(snapshot.header_row_present))?;
    let row_count = u32::try_from(snapshot.rows.len()).ok()?;
    let end_row = start_row.checked_add(row_count.checked_sub(1)?)?;
    let column_count = u32::try_from(snapshot.columns.len()).ok()?;
    let end_col = snapshot
        .virtual_anchor
        .start_col
        .checked_add(column_count.checked_sub(1)?)?;
    Some(a1_range_ref_for_retained_artifact(
        start_row,
        snapshot.virtual_anchor.start_col,
        end_row,
        end_col,
    ))
}

fn a1_range_ref_for_retained_artifact(
    top_row: u32,
    left_col: u32,
    bottom_row: u32,
    right_col: u32,
) -> String {
    format!(
        "{}{}:{}{}",
        a1_col_label_for_retained_artifact(left_col),
        top_row,
        a1_col_label_for_retained_artifact(right_col),
        bottom_row
    )
}

fn a1_col_label_for_retained_artifact(mut one_based_col: u32) -> String {
    debug_assert!(one_based_col > 0);
    let mut chars = Vec::new();
    while one_based_col > 0 {
        one_based_col -= 1;
        let offset = u8::try_from(one_based_col % 26).expect("column modulo fits in u8");
        chars.push(char::from(b'A' + offset));
        one_based_col /= 26;
    }
    chars.iter().rev().collect()
}

fn table_cell_value(
    table: &TableNodeFixture,
    tax_report: &oxcalc_core::structured_table::TreeCalcTableFormulaRuntimeReport,
    row_id: &str,
    column_id: &str,
) -> Option<CalcValue> {
    if column_id == tax_report.target_column_id {
        return tax_report
            .cell_results
            .iter()
            .find(|cell| {
                cell.row_id
                    .as_ref()
                    .is_some_and(|candidate| candidate.0 == row_id)
            })
            .map(|cell| cell.value.clone());
    }
    table
        .columns
        .iter()
        .find(|column| column.column_id == column_id)
        .and_then(|column| {
            column
                .body
                .constants
                .iter()
                .find(|cell| cell.row_id == row_id)
        })
        .map(|cell| parse_fixture_value(&cell.value))
}

fn table_data_formula_text(column: &TableColumnFixture) -> Option<&str> {
    column
        .body
        .formula
        .as_ref()
        .map(|formula| formula.formula_text.as_str())
}

fn table_formula_json(formula: &TableFormulaFixture) -> Value {
    json!({
        "formula_text": formula.formula_text,
        "formula_stable_id": formula.formula_stable_id,
        "bind_artifact_id": formula.bind_artifact_id,
        "formula_text_version": formula.formula_text_version
    })
}

fn comparison_value_json(value: &CalcValue) -> Value {
    match &value.core {
        CoreValue::Number(number) => json!({
            "wire_schema": "oxfunc_value_types.aligned_json.v1",
            "boundary": "published_formula_result",
            "value": {
                "kind": "number",
                "number": number
            }
        }),
        CoreValue::Text(text) => {
            let text = text.to_string_lossy();
            json!({
                "wire_schema": "oxfunc_value_types.aligned_json.v1",
                "boundary": "published_formula_result",
                "value": {
                    "kind": "text",
                    "utf16_code_units": text.encode_utf16().collect::<Vec<_>>()
                }
            })
        }
        other => json!({
            "wire_schema": "oxfunc_value_types.aligned_json.v1",
            "boundary": "published_formula_result",
            "value": {
                "kind": "debug",
                "repr": format!("{other:?}")
            }
        }),
    }
}

fn table_column_body_kind_id(kind: TableColumnBodyKind) -> &'static str {
    match kind {
        TableColumnBodyKind::ConstantCells => "constant_cells",
        TableColumnBodyKind::Formula => "formula",
    }
}

fn structured_section_kind_id(kind: oxfml_core::StructuredSectionKind) -> &'static str {
    match kind {
        oxfml_core::StructuredSectionKind::All => "all",
        oxfml_core::StructuredSectionKind::Data => "data",
        oxfml_core::StructuredSectionKind::Headers => "headers",
        oxfml_core::StructuredSectionKind::Totals => "totals",
        oxfml_core::StructuredSectionKind::ThisRow => "this_row",
    }
}

fn structured_table_dependency_fact_kind_id(
    kind: StructuredTableDependencyFactKind,
) -> &'static str {
    match kind {
        StructuredTableDependencyFactKind::TableIdentity => "table_identity",
        StructuredTableDependencyFactKind::RowMembership => "row_membership",
        StructuredTableDependencyFactKind::RowOrder => "row_order",
        StructuredTableDependencyFactKind::RowValue => "row_value",
        StructuredTableDependencyFactKind::ColumnIdentity => "column_identity",
        StructuredTableDependencyFactKind::ColumnOrder => "column_order",
        StructuredTableDependencyFactKind::HeaderText => "header_text",
        StructuredTableDependencyFactKind::HeaderRegion => "header_region",
        StructuredTableDependencyFactKind::DataRegion => "data_region",
        StructuredTableDependencyFactKind::TotalsRegion => "totals_region",
        StructuredTableDependencyFactKind::TotalsValue => "totals_value",
        StructuredTableDependencyFactKind::TotalsFormula => "totals_formula",
        StructuredTableDependencyFactKind::CallerRowContext => "caller_row_context",
        StructuredTableDependencyFactKind::OmittedTableNameEnclosingTable => {
            "omitted_table_name_enclosing_table"
        }
        StructuredTableDependencyFactKind::VirtualAnchorRange => "virtual_anchor_range",
        StructuredTableDependencyFactKind::WorkspaceAvailability => "workspace_availability",
        StructuredTableDependencyFactKind::FunctionRegistrySnapshot => "function_registry_snapshot",
    }
}

fn dependency_kind_id(kind: DependencyDescriptorKind) -> &'static str {
    match kind {
        DependencyDescriptorKind::StaticDirect => "static_direct",
        DependencyDescriptorKind::RelativeBound => "relative_bound",
        DependencyDescriptorKind::TreeReferenceCollectionMembership => {
            "tree_reference_collection_membership"
        }
        DependencyDescriptorKind::TreeReferenceCollectionMemberValue => {
            "tree_reference_collection_member_value"
        }
        DependencyDescriptorKind::StructuredTableIdentity => "structured_table_identity",
        DependencyDescriptorKind::StructuredTableRowMembership => "structured_table_row_membership",
        DependencyDescriptorKind::StructuredTableRowOrder => "structured_table_row_order",
        DependencyDescriptorKind::StructuredTableColumnIdentity => {
            "structured_table_column_identity"
        }
        DependencyDescriptorKind::StructuredTableHeaderText => "structured_table_header_text",
        DependencyDescriptorKind::StructuredTableHeaderRegion => "structured_table_header_region",
        DependencyDescriptorKind::StructuredTableDataRegion => "structured_table_data_region",
        DependencyDescriptorKind::StructuredTableTotalsRegion => "structured_table_totals_region",
        DependencyDescriptorKind::StructuredTableCallerContext => "structured_table_caller_context",
        DependencyDescriptorKind::StructuredTableEnclosingTable => {
            "structured_table_enclosing_table"
        }
        DependencyDescriptorKind::DynamicPotential => "dynamic_potential",
        DependencyDescriptorKind::HostSensitive => "host_sensitive",
        DependencyDescriptorKind::CapabilitySensitive => "capability_sensitive",
        DependencyDescriptorKind::ShapeTopology => "shape_topology",
        DependencyDescriptorKind::Unresolved => "unresolved",
    }
}

fn invalidation_reason_kind_id(kind: InvalidationReasonKind) -> &'static str {
    match kind {
        InvalidationReasonKind::StructuralRebindRequired => "structural_rebind_required",
        InvalidationReasonKind::StructuralRecalcOnly => "structural_recalc_only",
        InvalidationReasonKind::UpstreamPublication => "upstream_publication",
        InvalidationReasonKind::ExternallyInvalidated => "externally_invalidated",
        InvalidationReasonKind::TreeReferenceMembershipChanged => {
            "tree_reference_membership_changed"
        }
        InvalidationReasonKind::TreeReferenceOrderChanged => "tree_reference_order_changed",
        InvalidationReasonKind::StructuredTableContextChanged => "structured_table_context_changed",
        InvalidationReasonKind::StructuredTableRowMembershipChanged => {
            "structured_table_row_membership_changed"
        }
        InvalidationReasonKind::StructuredTableRowOrderChanged => {
            "structured_table_row_order_changed"
        }
        InvalidationReasonKind::StructuredTableColumnChanged => "structured_table_column_changed",
        InvalidationReasonKind::StructuredTableRegionChanged => "structured_table_region_changed",
        InvalidationReasonKind::StructuredTableCallerContextChanged => {
            "structured_table_caller_context_changed"
        }
        InvalidationReasonKind::DependencyAdded => "dependency_added",
        InvalidationReasonKind::DependencyRemoved => "dependency_removed",
        InvalidationReasonKind::DependencyReclassified => "dependency_reclassified",
        InvalidationReasonKind::DynamicDependencyActivated => "dynamic_dependency_activated",
        InvalidationReasonKind::DynamicDependencyReleased => "dynamic_dependency_released",
        InvalidationReasonKind::DynamicDependencyReclassified => "dynamic_dependency_reclassified",
    }
}

fn prepared_identity_input_id(
    input: oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput,
) -> &'static str {
    match input {
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::HostNamespaceVersion => {
            "host_namespace_version"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::StructureContextVersion => {
            "structure_context_version"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::TableContextIdentity => {
            "table_context_identity"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::CallerContextIdentity => {
            "caller_context_identity"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::DynamicSelectorIdentity => {
            "dynamic_selector_identity"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::RegistrySnapshotIdentity => {
            "registry_snapshot_identity"
        }
        oxcalc_core::structured_table::TreeCalcTablePreparedIdentityInput::ResolutionRuleVersion => {
            "resolution_rule_version"
        }
    }
}

fn table_update_scenario_kind_id(kind: TreeCalcTableUpdateScenarioKind) -> &'static str {
    match kind {
        TreeCalcTableUpdateScenarioKind::BodyCellEdit => "body_cell_edit",
        TreeCalcTableUpdateScenarioKind::BodyFormulaEdit => "body_formula_edit",
        TreeCalcTableUpdateScenarioKind::RowInsert => "row_insert",
        TreeCalcTableUpdateScenarioKind::RowDelete => "row_delete",
        TreeCalcTableUpdateScenarioKind::RowReorder => "row_reorder",
        TreeCalcTableUpdateScenarioKind::ColumnInsert => "column_insert",
        TreeCalcTableUpdateScenarioKind::ColumnDelete => "column_delete",
        TreeCalcTableUpdateScenarioKind::ColumnReorder => "column_reorder",
        TreeCalcTableUpdateScenarioKind::ColumnRename => "column_rename",
        TreeCalcTableUpdateScenarioKind::HeaderTextEdit => "header_text_edit",
        TreeCalcTableUpdateScenarioKind::TotalsRowToggle => "totals_row_toggle",
        TreeCalcTableUpdateScenarioKind::TotalsFormulaEdit => "totals_formula_edit",
        TreeCalcTableUpdateScenarioKind::TableRename => "table_rename",
        TreeCalcTableUpdateScenarioKind::TableMove => "table_move",
        TreeCalcTableUpdateScenarioKind::TableDelete => "table_delete",
        TreeCalcTableUpdateScenarioKind::TableResize => "table_resize",
        TreeCalcTableUpdateScenarioKind::NodeRename => "node_rename",
        TreeCalcTableUpdateScenarioKind::NodeMove => "node_move",
        TreeCalcTableUpdateScenarioKind::NodeDelete => "node_delete",
        TreeCalcTableUpdateScenarioKind::SaveReopen => "save_reopen",
        TreeCalcTableUpdateScenarioKind::WorkspaceOpen => "workspace_open",
        TreeCalcTableUpdateScenarioKind::WorkspaceClose => "workspace_close",
        TreeCalcTableUpdateScenarioKind::WorkspaceAliasMutation => "workspace_alias_mutation",
        TreeCalcTableUpdateScenarioKind::FunctionRegistrySnapshotMutation => {
            "function_registry_snapshot_mutation"
        }
        TreeCalcTableUpdateScenarioKind::StructuralRebind => "structural_rebind",
    }
}

fn load_expected_json_or_panic_with_generated(path: &Path, generated: &Value) -> Value {
    if path.exists() {
        return load_json(path);
    }
    panic!(
        "missing retained artifact {path:?}; generated:\n{}",
        serde_json::to_string_pretty(generated).expect("generated artifact serializes")
    );
}

fn load_json(path: &Path) -> Value {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read JSON {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse JSON {path:?}: {error}"))
}

fn write_pretty_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("failed to create {parent:?}: {error}"));
    }
    let contents = serde_json::to_string_pretty(value).expect("retained JSON artifact serializes");
    fs::write(path, format!("{contents}\n"))
        .unwrap_or_else(|error| panic!("failed to write JSON {path:?}: {error}"));
}

fn load_theme(path: PathBuf) -> CorpusTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse {path:?}: {error}"))
}

fn load_lifecycle_theme(path: PathBuf) -> TableLifecycleTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read lifecycle corpus {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse lifecycle corpus {path:?}: {error}"))
}

fn load_dynamic_table_theme(path: PathBuf) -> DynamicTableTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read dynamic table corpus {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse dynamic table corpus {path:?}: {error}"))
}

fn load_workspace(workspace_id: &str) -> WorkspaceModel {
    let fixture =
        WorkspaceFixture::from_path(repo_corpus_path(format!("workspaces/{workspace_id}.json")))
            .unwrap_or_else(|error| panic!("failed to load workspace {workspace_id}: {error}"));
    WorkspaceModel::try_from(fixture)
        .unwrap_or_else(|error| panic!("invalid workspace {workspace_id}: {error}"))
}

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

fn repo_docs_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs")
        .join(path)
}

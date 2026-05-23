use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::adapters::oxcalc::{
    LiveOxCalcTreeBridge, OxCalcTreeBridge, PreparedFormulaCatalog, TreeRecalcRequest,
};
use dnatreecalc_host::model::{
    TableColumnBodyKind, TableColumnFixture, TableFormulaFixture, TableNodeFixture,
    WorkspaceFixture, WorkspaceModel, WorkspaceNodeFixture,
};
use oxcalc_core::dependency::{DependencyDescriptorKind, InvalidationReasonKind};
use oxcalc_core::sparse_reader::{SparseCellCoord, SparseCellRead, SparseRangeReader};
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::structured_table::{
    StructuredTableContextPacket, StructuredTableDependencyFactStatus,
    StructuredTableDependencyLoweringRequest, TableCallerRegion, TableRef, TableRegionKind,
    TreeCalcTableColumnBodyMetadata, TreeCalcTableColumnFormulaRuntimeRequest,
    TreeCalcTableColumnSnapshot, TreeCalcTableFormulaMetadata, TreeCalcTableFormulaRuntimeContext,
    TreeCalcTableNodeProjection, TreeCalcTableNodeSnapshot, TreeCalcTableRowId,
    TreeCalcTableSparseReader, TreeCalcTableSparseValue, TreeCalcTableUpdateScenarioKind,
    classify_treecalc_table_update, evaluate_treecalc_table_column_formula_rows,
    evaluate_treecalc_table_totals_formula, lower_structured_table_dependencies,
    prebind_treecalc_table_structured_references, project_treecalc_table_node_snapshot,
    validate_treecalc_table_reference_after_update,
};
use oxfml_core::EvaluationBackend;
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest};
use oxfml_core::interface::TypedContextQueryBundle;
use oxfml_core::seam::Locus;
use oxfml_core::source::FormulaSourceRecord;
use oxfunc_core::value::{EvalValue, ExcelText};
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

#[test]
fn active_table_structured_reference_corpus_executes_through_oxcalc_table_path() {
    let theme = load_theme(repo_corpus_path("tables/structured-references.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "tables/structured-references");
    assert_eq!(theme.status, CorpusStatus::Active);

    let workspace = load_workspace("tables");
    let (sales_table, sales_snapshot, sales_projection) = table_evidence(&workspace, "SalesTable");
    assert_live_bridge_projects_same_table_context("SalesTable", sales_table);

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
                [("col:amount", EvalValue::Number(60.0))],
            )
        } else {
            table_sparse_values(table, None, std::iter::empty::<(&str, EvalValue)>())
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
        let prebound = prebind_treecalc_table_structured_references(
            formula_text,
            std::slice::from_ref(&projection),
            enclosing,
            caller_region.clone(),
        );
        assert_eq!(
            case.expect.outcome,
            if prebound
                .iter()
                .flat_map(|prebind| prebind.diagnostics.iter())
                .next()
                .is_some()
            {
                "error"
            } else {
                "resolved"
            },
            "{} outcome",
            case.id
        );
        if case.expect.outcome == "error" {
            if let Some(reason) = &case.expect.reason {
                assert!(
                    prebound
                        .iter()
                        .flat_map(|prebind| prebind.diagnostics.iter())
                        .any(|diagnostic| diagnostic.message.contains(reason)
                            || diagnostic.diagnostic_code.contains(reason)),
                    "{} expected diagnostic reason {reason}",
                    case.id
                );
            }
            assert!(
                prebound
                    .iter()
                    .flat_map(|prebind| prebind.diagnostics.iter())
                    .any(|diagnostic| diagnostic.message.contains("Missing")),
                "{} expected missing-column diagnostic",
                case.id
            );
            continue;
        }

        let bind_record = &prebound
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table prebind", case.id))
            .bind_record;
        let prebind = prebound
            .first()
            .unwrap_or_else(|| panic!("case {} produced no table prebind", case.id));
        assert_case_target(&case.id, &case.expect, prebind, &projection);
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
            let observed = if is_simple_current_row_reference_formula(formula_text, prebind) {
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
fn retained_table_replay_artifact_matches_live_oxcalc_projection() {
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

fn is_simple_current_row_reference_formula(
    formula_text: &str,
    prebind: &oxcalc_core::structured_table::TreeCalcTableStructuredReferencePrebind,
) -> bool {
    prebind.bind_record.uses_this_row
        && formula_text
            .trim()
            .strip_prefix('=')
            .is_some_and(|body| body == prebind.source_token_text)
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
    prebind: &oxcalc_core::structured_table::TreeCalcTableStructuredReferencePrebind,
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
    assert_target_kind_matches_bind_record(case_id, target_kind, prebind);

    assert_eq!(
        prebind.resolved_table_id.as_deref(),
        Some(projection.table_id.as_str()),
        "{case_id} resolved table id"
    );
    assert_eq!(
        prebind.bind_record.selected_column_ids, prebind.selector_payload.selected_column_ids,
        "{case_id} selector payload must match bind record columns"
    );
}

fn assert_target_kind_matches_bind_record(
    case_id: &str,
    target_kind: &str,
    prebind: &oxcalc_core::structured_table::TreeCalcTableStructuredReferencePrebind,
) {
    use oxfml_core::StructuredSectionKind;

    let sections = &prebind.bind_record.selected_sections;
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
                !prebind.bind_record.uses_this_row,
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
                prebind.bind_record.uses_this_row,
                "{case_id} must use row context"
            );
            assert!(
                prebind.caller_context_dependency,
                "{case_id} must preserve caller-context dependency"
            );
        }
        "header-region" => assert_eq!(
            sections,
            &[StructuredSectionKind::Headers],
            "{case_id} target_kind must describe a header structured reference"
        ),
        "totals-column-reference" => assert_eq!(
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
                prebind.bind_record.uses_this_row,
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

fn assert_live_bridge_projects_same_table_context(table_path: &str, table: &TableNodeFixture) {
    let bridge_workspace = WorkspaceModel::try_from(WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "active-table-corpus-bridge".to_string(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: table_path.to_string(),
            formula: String::new(),
            is_meta: false,
            table: Some(table.clone()),
        }],
    })
    .expect("single-table bridge workspace is valid");

    let result = LiveOxCalcTreeBridge::default()
        .execute_recalc(TreeRecalcRequest {
            workspace: bridge_workspace,
            formula_catalog: PreparedFormulaCatalog::default(),
            candidate_result_id: "cand:active-table-corpus-bridge".to_string(),
            publication_id: "pub:active-table-corpus-bridge".to_string(),
            compatibility_basis: "snapshot:active-table-corpus-bridge".to_string(),
            artifact_token_basis: "snapshot:active-table-corpus-bridge".to_string(),
            capability_profile_id: "treecalc-v1".to_string(),
            cycle_config: Default::default(),
        })
        .expect("LiveOxCalcTreeBridge must accept DnaTreeCalc table projection");

    assert!(
        result.table_context_identities[table_path].contains("treecalc.table_context.v1"),
        "bridge must retain the table context identity from the real OxCalc table projection"
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
        header_row_present: table.header.present,
        totals_row_present: table.totals.present,
        table_namespace_version: table.table_namespace_version.clone(),
        row_membership_version: table.row_membership_version.clone(),
        row_order_version: table.row_order_version.clone(),
        column_identity_version: table.column_identity_version.clone(),
    }
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
    totals_values: impl IntoIterator<Item = (&'a str, EvalValue)>,
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

fn parse_fixture_value(value: &str) -> EvalValue {
    value.parse::<f64>().map_or_else(
        |_| EvalValue::Text(ExcelText::from_interop_assignment(value)),
        EvalValue::Number,
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
) -> EvalValue {
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
        .with_sparse_reference_value_bindings(vec![runtime_binding.sparse_reference_values])
        .execute(
            RuntimeFormulaRequest::new(
                FormulaSourceRecord::new(format!("dnatreecalc:{case_id}"), 1, formula_text),
                TypedContextQueryBundle::default(),
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

fn table_primary_locus(projection: &TreeCalcTableNodeProjection) -> Locus {
    Locus {
        sheet_id: projection.table_descriptor.sheet_scope_ref.clone(),
        row: 3,
        col: 2,
    }
}

fn display_value(value: &EvalValue) -> String {
    match value {
        EvalValue::Number(number) => display_number(*number),
        EvalValue::Text(text) => text.to_string_lossy(),
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
                    "outcome_stage": "live_oxcalc_table_projection",
                    "class_id": "treecalc_table_structured_reference_projection_ready",
                    "lane_reason_code": "dnatreecalc_w056_table_projection_retained",
                    "bridge": "LiveOxCalcTreeBridge",
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
            "bridge_influenced": true,
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
    let prebind = prebind_treecalc_table_structured_references(
        formula_text,
        std::slice::from_ref(projection),
        None,
        None,
    )
    .into_iter()
    .next()
    .expect("retained report node formula prebinds a structured table reference");
    assert!(
        prebind.diagnostics.is_empty(),
        "retained report node prebind diagnostics: {:?}",
        prebind.diagnostics
    );
    let reader = TreeCalcTableSparseReader::from_oxfml_bind_record(
        snapshot,
        projection,
        &prebind.bind_record,
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
            let prebind = prebind_treecalc_table_structured_references(
                formula_text,
                std::slice::from_ref(projection),
                enclosing,
                caller_region.clone(),
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("case {} did not prebind", case.id));
            json!({
                "case_id": case.id,
                "source_span_utf8": {
                    "start": prebind.source_span_utf8.start,
                    "len": prebind.source_span_utf8.len
                },
                "source_token_text": prebind.source_token_text,
                "host_ref_handle": prebind.host_ref_handle,
                "replay_identity_present": !prebind.replay_identity.is_empty(),
                "resolved_table_id": prebind.resolved_table_id,
                "selected_column_ids": prebind.bind_record.selected_column_ids,
                "selected_sections": prebind
                    .bind_record
                    .selected_sections
                    .iter()
                    .copied()
                    .map(structured_section_kind_id)
                    .collect::<Vec<_>>(),
                "caller_context_dependency": prebind.caller_context_dependency
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
) -> Option<EvalValue> {
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

fn comparison_value_json(value: &EvalValue) -> Value {
    match value {
        EvalValue::Number(number) => json!({
            "wire_schema": "oxfunc_value_types.aligned_json.v1",
            "boundary": "published_formula_result",
            "value": {
                "kind": "number",
                "number": number
            }
        }),
        EvalValue::Text(text) => {
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
        TreeCalcTableUpdateScenarioKind::SaveReopen => "save_reopen",
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

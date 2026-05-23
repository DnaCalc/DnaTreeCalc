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
    display_value(&result.evaluation.oxfunc_value)
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

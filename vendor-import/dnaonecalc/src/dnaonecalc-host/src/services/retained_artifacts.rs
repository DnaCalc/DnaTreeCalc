use crate::domain::ids::FormulaSpaceId;
use crate::services::capability_snapshot::build_capability_ledger_snapshot;
use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, ProgrammaticArtifactCatalogEntry,
    ProgrammaticComparisonStatus,
};
use crate::services::verification_bundle::VerificationBundleReport;
use crate::state::{
    FormulaSpaceState, OneCalcHostState, ProjectionTruthSource, RetainedArtifactRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedArtifactImportRequest {
    pub formula_space_id: FormulaSpaceId,
    pub catalog_entry: ProgrammaticArtifactCatalogEntry,
    pub discrepancy_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualRetainedArtifactImportRequest {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub discrepancy_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationBundleImportRequest {
    pub report_json: String,
}

pub fn import_programmatic_artifact(
    state: &mut OneCalcHostState,
    request: RetainedArtifactImportRequest,
) {
    let snapshot = build_capability_ledger_snapshot(state);
    let record = RetainedArtifactRecord {
        artifact_id: request.catalog_entry.artifact_id.clone(),
        case_id: request.catalog_entry.case_id,
        formula_space_id: request.formula_space_id,
        comparison_status: request.catalog_entry.comparison_status,
        open_mode_hint: request.catalog_entry.open_mode_hint,
        discrepancy_summary: request.discrepancy_summary,
        bundle_report_path: None,
        case_output_dir: None,
        xml_extraction: None,
        upstream_gap_report: None,
        oxfml_comparison_value: None,
        excel_comparison_value: None,
        value_match: None,
        display_match: None,
        replay_equivalent: None,
        replay_mismatch_records: Vec::new(),
        replay_explain_records: Vec::new(),
        oxfml_effective_display_summary: None,
        excel_effective_display_text: None,
        capability_snapshot_id: Some(snapshot.capability_snapshot_id),
        oxfunc_semantic_kernel_metadata_versions: snapshot
            .oxfunc_metadata
            .semantic_kernel_metadata_versions,
        oxfunc_arg_admission_metadata_versions: snapshot
            .oxfunc_metadata
            .arg_admission_metadata_versions,
        producer_capability_set_keys: Vec::new(),
        exercised_capability_keys: Vec::new(),
    };

    state
        .retained_artifacts
        .catalog
        .insert(record.artifact_id.clone(), record);
}

pub fn import_programmatic_artifacts(
    state: &mut OneCalcHostState,
    requests: impl IntoIterator<Item = RetainedArtifactImportRequest>,
) {
    for request in requests {
        import_programmatic_artifact(state, request);
    }
}

pub fn open_retained_artifact_by_id(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> Result<(), String> {
    let Some(record) = state.retained_artifacts.catalog.get(artifact_id) else {
        return Err(format!("retained artifact not found: {artifact_id}"));
    };

    state.retained_artifacts.open_artifact_id = Some(record.artifact_id.clone());
    state.workspace_shell.active_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id =
        Some(record.formula_space_id.clone());
    state.active_formula_space_view.active_mode = match record.open_mode_hint {
        crate::services::programmatic_testing::ProgrammaticOpenModeHint::Inspect => {
            crate::state::AppMode::Inspect
        }
        crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench => {
            crate::state::AppMode::Workbench
        }
    };
    Ok(())
}

pub fn open_retained_artifact_in_inspect_by_id(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> Result<(), String> {
    let Some(record) = state.retained_artifacts.catalog.get(artifact_id) else {
        return Err(format!("retained artifact not found: {artifact_id}"));
    };

    state.retained_artifacts.open_artifact_id = Some(record.artifact_id.clone());
    state.workspace_shell.active_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id =
        Some(record.formula_space_id.clone());
    state.active_formula_space_view.active_mode = crate::state::AppMode::Inspect;
    Ok(())
}

pub fn import_manual_artifact_for_active_formula_space(
    state: &mut OneCalcHostState,
    request: ManualRetainedArtifactImportRequest,
) -> Result<(), String> {
    let Some(formula_space_id) = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())
    else {
        return Err("no active formula space for retained artifact import".to_string());
    };

    let artifact_id = request.artifact_id;
    import_programmatic_artifact(
        state,
        RetainedArtifactImportRequest {
            formula_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                artifact_id.clone(),
                request.case_id,
                request.comparison_status,
            ),
            discrepancy_summary: request.discrepancy_summary,
        },
    );
    open_retained_artifact_by_id(state, &artifact_id)
}

pub fn import_verification_bundle_report_json(
    state: &mut OneCalcHostState,
    request: VerificationBundleImportRequest,
) -> Result<Vec<String>, String> {
    let report: VerificationBundleReport = serde_json::from_str(&request.report_json)
        .map_err(|error| format!("failed to parse verification bundle report JSON: {error}"))?;

    if report.case_reports.is_empty() {
        return Err("verification bundle report did not contain any case reports".to_string());
    }

    let mut imported_artifact_ids = Vec::with_capacity(report.case_reports.len());
    for case_report in &report.case_reports {
        let formula_space_id =
            FormulaSpaceId::new(format!("verify-{}", sanitize_case_id(&case_report.case_id)));
        let mut formula_space = FormulaSpaceState::new(
            formula_space_id.clone(),
            case_report.entered_cell_text.clone(),
        );
        formula_space.latest_evaluation_summary =
            case_report.oxfml_summary.evaluation_summary.clone();
        formula_space.effective_display_summary =
            case_report.oxfml_summary.effective_display_summary.clone();
        formula_space.context.scenario_label = case_report.case_id.clone();
        formula_space.context.host_profile = report.host_profile.profile_id.clone();
        formula_space.context.packet_kind = if case_report.spreadsheet_xml_extraction.is_some() {
            "verification-bundle.spreadsheetml-2003".to_string()
        } else {
            "verification-bundle.formula-entry".to_string()
        };
        formula_space.context.capability_floor = report.capabilities.host_summary.clone();
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        formula_space.context.trace_summary = case_report
            .spreadsheet_xml_extraction
            .as_ref()
            .map(|extraction| {
                format!(
                    "XML source {} @ {}",
                    extraction.workbook_path, extraction.locator
                )
            })
            .or_else(|| Some("Imported verification bundle".to_string()));
        formula_space.context.blocked_reason = case_report.oxfml_summary.blocked_reason.clone();
        state.formula_spaces.insert(formula_space);
        if !state
            .workspace_shell
            .open_formula_space_order
            .contains(&formula_space_id)
        {
            state
                .workspace_shell
                .open_formula_space_order
                .push(formula_space_id.clone());
        }

        let snapshot = build_capability_ledger_snapshot(state);
        let artifact_id = case_report.artifact_catalog_entry.artifact_id.clone();
        let record = RetainedArtifactRecord {
            artifact_id: artifact_id.clone(),
            case_id: case_report.case_id.clone(),
            formula_space_id: formula_space_id.clone(),
            comparison_status: case_report.comparison_status,
            open_mode_hint: case_report.artifact_catalog_entry.open_mode_hint,
            discrepancy_summary: case_report.discrepancy_summary.clone(),
            bundle_report_path: Some(report.output_root.clone()),
            case_output_dir: Some(case_report.case_output_dir.clone()),
            xml_extraction: case_report.spreadsheet_xml_extraction.clone(),
            upstream_gap_report: case_report.upstream_gap_report.clone(),
            oxfml_comparison_value: case_report.oxfml_summary.comparison_value.clone(),
            excel_comparison_value: case_report
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.comparison_value.clone()),
            value_match: case_report.value_match,
            display_match: case_report.display_match,
            replay_equivalent: case_report.replay_equivalent,
            replay_mismatch_records: case_report.replay_mismatch_records.clone(),
            replay_explain_records: case_report.replay_explain_records.clone(),
            oxfml_effective_display_summary: case_report
                .oxfml_summary
                .effective_display_summary
                .clone(),
            excel_effective_display_text: case_report
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.effective_display_text.clone()),
            capability_snapshot_id: Some(snapshot.capability_snapshot_id),
            oxfunc_semantic_kernel_metadata_versions: snapshot
                .oxfunc_metadata
                .semantic_kernel_metadata_versions,
            oxfunc_arg_admission_metadata_versions: snapshot
                .oxfunc_metadata
                .arg_admission_metadata_versions,
            producer_capability_set_keys: Vec::new(),
            exercised_capability_keys: Vec::new(),
        };
        state
            .retained_artifacts
            .catalog
            .insert(artifact_id.clone(), record);
        imported_artifact_ids.push(artifact_id);
    }

    if let Some(first_artifact_id) = imported_artifact_ids.first() {
        let _ = open_retained_artifact_by_id(state, first_artifact_id);
    }

    Ok(imported_artifact_ids)
}

pub fn active_retained_artifact<'a>(
    state: &'a OneCalcHostState,
) -> Option<&'a RetainedArtifactRecord> {
    let artifact_id = state.retained_artifacts.open_artifact_id.as_ref()?;
    state.retained_artifacts.catalog.get(artifact_id)
}

pub fn retained_artifacts_for_formula_space<'a>(
    state: &'a OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
) -> Vec<&'a RetainedArtifactRecord> {
    let mut records = state
        .retained_artifacts
        .catalog
        .values()
        .filter(|record| &record.formula_space_id == formula_space_id)
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    records
}

fn sanitize_case_id(case_id: &str) -> String {
    case_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };
    use crate::services::spreadsheet_xml::{
        SpreadsheetXmlCellExtraction, VerificationObservationScope,
    };
    use crate::services::verification_bundle::{
        ExcelObservationSummary, OxfmlVerificationSummary, VerificationBundleReport,
        VerificationCaseReport, VerificationObservationGapReport,
    };
    use serde_json::json;

    #[test]
    fn importing_programmatic_artifact_populates_catalog() {
        let mut state = OneCalcHostState::default();

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );

        assert!(state.retained_artifacts.catalog.contains_key("artifact-1"));
    }

    #[test]
    fn importing_multiple_programmatic_artifacts_populates_sorted_formula_space_catalog() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifacts(
            &mut state,
            [
                RetainedArtifactImportRequest {
                    formula_space_id: FormulaSpaceId::new("space-1"),
                    catalog_entry: ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-2".to_string(),
                        case_id: "case-2".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Matched,
                        open_mode_hint: ProgrammaticOpenModeHint::Inspect,
                    },
                    discrepancy_summary: None,
                },
                RetainedArtifactImportRequest {
                    formula_space_id: FormulaSpaceId::new("space-1"),
                    catalog_entry: ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Mismatched,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                    discrepancy_summary: Some("dna=1 excel=2".to_string()),
                },
            ],
        );

        let records = retained_artifacts_for_formula_space(&state, &FormulaSpaceId::new("space-1"));
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].artifact_id, "artifact-1");
        assert_eq!(records[1].artifact_id, "artifact-2");
    }

    #[test]
    fn opening_retained_artifact_routes_shell_to_its_formula_space_and_mode() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Blocked,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        open_retained_artifact_by_id(&mut state, "artifact-1").expect("artifact should open");

        assert_eq!(
            state
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("space-1")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn opening_retained_artifact_in_inspect_routes_shell_to_inspect_mode() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );

        open_retained_artifact_in_inspect_by_id(&mut state, "artifact-1")
            .expect("artifact should open in inspect");

        assert_eq!(
            state
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("space-1")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Inspect
        );
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-1")
        );
    }

    #[test]
    fn importing_manual_artifact_uses_active_formula_space_and_opens_it() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());

        import_manual_artifact_for_active_formula_space(
            &mut state,
            ManualRetainedArtifactImportRequest {
                artifact_id: "artifact-9".to_string(),
                case_id: "case-9".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Blocked,
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        )
        .expect("manual artifact should import");

        let record = state
            .retained_artifacts
            .catalog
            .get("artifact-9")
            .expect("artifact imported");
        assert_eq!(record.formula_space_id, formula_space_id);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-9")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn importing_verification_bundle_report_creates_formula_spaces_and_preserves_xml_context() {
        let mut state = OneCalcHostState::default();
        let report = VerificationBundleReport {
            bundle_id: "bundle-1".to_string(),
            output_root: "target/onecalc-verification/bundle-1".to_string(),
            host_profile: crate::services::programmatic_testing::default_windows_excel_host_profile(),
            capabilities: crate::services::programmatic_testing::default_windows_excel_capability_profile(),
            batch_plan: crate::services::programmatic_testing::ProgrammaticBatchPlan {
                formula_count: 1,
                comparison_lane: crate::services::programmatic_testing::ProgrammaticComparisonLane::OxfmlAndExcel,
                discrepancy_index_required: true,
                retained_artifact_kinds: vec!["comparison_outcome".to_string()],
            },
            retained_artifact_catalog: vec![ProgrammaticArtifactCatalogEntry {
                artifact_id: "artifact-xml".to_string(),
                case_id: "xml-case-1".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Mismatched,
                open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            }],
            case_reports: vec![VerificationCaseReport {
                case_id: "xml-case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                artifact_catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-xml".to_string(),
                    case_id: "xml-case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                comparison_status: ProgrammaticComparisonStatus::Mismatched,
                value_match: Some(true),
                display_match: Some(false),
                replay_equivalent: Some(false),
                replay_mismatch_kinds: vec![
                    "effective_display_text".to_string(),
                    "projection_coverage_gap".to_string(),
                    "projection_coverage_gap".to_string(),
                ],
                replay_mismatch_records: vec![
                    crate::services::verification_bundle::OxReplayMismatchRecord {
                        mismatch_kind: "effective_display_text".to_string(),
                        severity: Some("informational".to_string()),
                        view_family: Some("effective_display_text".to_string()),
                        left_value_repr: Some("6".to_string()),
                        right_value_repr: Some("$6.00".to_string()),
                        detail: Some("comparison view values diverged".to_string()),
                    },
                    crate::services::verification_bundle::OxReplayMismatchRecord {
                        mismatch_kind: "projection_coverage_gap".to_string(),
                        severity: Some("coverage".to_string()),
                        view_family: Some("formatting_view".to_string()),
                        left_value_repr: None,
                        right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                        detail: Some("comparison view family `formatting_view` is missing on one side".to_string()),
                    },
                    crate::services::verification_bundle::OxReplayMismatchRecord {
                        mismatch_kind: "projection_coverage_gap".to_string(),
                        severity: Some("coverage".to_string()),
                        view_family: Some("conditional_formatting_view".to_string()),
                        left_value_repr: None,
                        right_value_repr: Some("[{\"range\":\"A1\",\"rule_kind\":\"expression\"}]".to_string()),
                        detail: Some("comparison view family `conditional_formatting_view` is missing on one side".to_string()),
                    },
                ],
                replay_explain_records: vec![
                    crate::services::verification_bundle::OxReplayExplainRecord {
                        query_id: Some("explain-01".to_string()),
                        summary: Some("comparison diverged on `effective_display_text`".to_string()),
                        mismatch_kind: "effective_display_text".to_string(),
                        severity: Some("informational".to_string()),
                        view_family: Some("effective_display_text".to_string()),
                        left_value_repr: Some("6".to_string()),
                        right_value_repr: Some("$6.00".to_string()),
                        detail: Some("comparison view values diverged".to_string()),
                    },
                ],
                discrepancy_summary: Some(
                    "Display divergence (effective_display_text): OxFml 6 vs Excel $6.00 | Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
                        .to_string(),
                ),
                oxfml_summary: OxfmlVerificationSummary {
                    evaluation_summary: Some("Number · 6".to_string()),
                    comparison_value: Some(json!(6)),
                    effective_display_summary: Some("6".to_string()),
                    blocked_reason: None,
                    parse_status: Some("Valid".to_string()),
                    green_tree_key: Some("green-1".to_string()),
                },
                excel_summary: Some(ExcelObservationSummary {
                    comparison_value: Some(json!(6)),
                    observed_value_repr: Some("$6.00".to_string()),
                    effective_display_text: Some("$6.00".to_string()),
                    observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
                    capture_status: "captured".to_string(),
                    render_locale_pinned: None,
                    render_locale_source: None,
                    render_locale_note: None,
                }),
                spreadsheet_xml_extraction: Some(SpreadsheetXmlCellExtraction {
                    workbook_path: "C:/tmp/formula-workbook.xml".to_string(),
                    locator: "Input!A1".to_string(),
                    worksheet_name: "Input".to_string(),
                    workbook_format_profile_hint: "excel-spreadsheetml-2003-default".to_string(),
                    formula_text: Some("=SUM(1,2,3)".to_string()),
                    entered_cell_text: "=SUM(1,2,3)".to_string(),
                    data_type: Some("Number".to_string()),
                    style_id: Some("calc".to_string()),
                    style_hierarchy: vec!["calcBase".to_string(), "calc".to_string()],
                    number_format_code: Some("$#,##0.00".to_string()),
                    font_color: Some("#112233".to_string()),
                    fill_color: Some("#445566".to_string()),
                    conditional_formats: vec![],
                    date1904: Some(false),
                    observation_scope: VerificationObservationScope {
                        oxfml_required_scope: vec!["format_profile".to_string()],
                        oxxlplay_required_surfaces: vec!["effective_display_text".to_string()],
                        oxreplay_required_views: vec![
                            "formatting_view".to_string(),
                            "conditional_formatting_view".to_string(),
                        ],
                    },
                }),
                upstream_gap_report: Some(VerificationObservationGapReport {
                    oxfml_scope_required: vec!["format_profile".to_string()],
                    oxxlplay_supported_surfaces: vec!["cell_value".to_string()],
                    oxxlplay_missing_surfaces: vec!["effective_display_text".to_string()],
                    oxreplay_required_views: vec![
                        "formatting_view".to_string(),
                        "conditional_formatting_view".to_string(),
                    ],
                    oxreplay_current_bundle_views: vec!["comparison_value".to_string()],
                    oxreplay_missing_views: vec![
                        "formatting_view".to_string(),
                        "conditional_formatting_view".to_string(),
                    ],
                }),
                case_output_dir: "target/onecalc-verification/bundle-1/cases/xml-case-1".to_string(),
                scenario_path: "target/onecalc-verification/bundle-1/cases/xml-case-1/scenario.json".to_string(),
            }],
        };

        let imported_artifact_ids = import_verification_bundle_report_json(
            &mut state,
            VerificationBundleImportRequest {
                report_json: serde_json::to_string(&report).expect("report json"),
            },
        )
        .expect("bundle import");

        assert_eq!(imported_artifact_ids, vec!["artifact-xml".to_string()]);
        let artifact = state
            .retained_artifacts
            .catalog
            .get("artifact-xml")
            .expect("artifact imported");
        assert!(artifact.xml_extraction.is_some());
        assert!(artifact.upstream_gap_report.is_some());
        assert_eq!(
            artifact.bundle_report_path.as_deref(),
            Some("target/onecalc-verification/bundle-1")
        );
        assert_eq!(
            artifact.excel_effective_display_text.as_deref(),
            Some("$6.00")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-xml")
        );
    }
}

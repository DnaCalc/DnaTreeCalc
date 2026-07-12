use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, build_programmatic_batch_plan,
    default_programmatic_corpus_formatting_context, default_verification_config,
    default_windows_excel_capability_profile, default_windows_excel_host_profile,
    ProgrammaticArtifactCatalogEntry, ProgrammaticBatchPlan, ProgrammaticCapabilityProfile,
    ProgrammaticComparisonStatus, ProgrammaticExcelRenderContext, ProgrammaticFormattingContext,
    ProgrammaticFormulaCase, ProgrammaticHostProfile,
};
use crate::services::spreadsheet_xml::{
    extract_cell_from_spreadsheet_xml, SpreadsheetXmlCellExtraction, VerificationObservationScope,
};

use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaEditRequest, NativeOxfmlHostSession, OxfmlHostSession,
};
use oxfml_core::consumer::replay::{ReplayProjectionRequest, ReplayProjectionService};
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest};
use oxfml_core::interface::TypedContextQueryBundle;
use oxfml_core::publication::{
    LocaleFormatContextSurface, VerificationConditionalFormattingRule,
    VerificationPublicationContext, VerificationPublicationSurface,
};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::FormulaChannelKind;
use oxfunc_core::locale_format::{
    excel_serial_from_ymd, format_profile, ymd_from_excel_serial, FormatCodeEngine, FormatFailure,
    FormatProfile, LocaleFormatContext, LocaleProfileId, LocaleValueParser, ParseFailure,
    WorkbookDateSystem,
};
use oxfunc_core::value::ExcelText;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationBatchRequest {
    #[serde(default = "default_windows_excel_host_profile")]
    pub host_profile: ProgrammaticHostProfile,
    #[serde(default = "default_windows_excel_capability_profile")]
    pub capabilities: ProgrammaticCapabilityProfile,
    #[serde(default = "default_verification_replay_policy")]
    pub replay_policy: VerificationReplayPolicy,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub render_contexts: BTreeMap<String, ProgrammaticExcelRenderContext>,
    pub cases: Vec<ProgrammaticFormulaCase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationReplayPolicy {
    Never,
    MismatchOnly,
    Always,
}

fn default_verification_replay_policy() -> VerificationReplayPolicy {
    VerificationReplayPolicy::MismatchOnly
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCommandCapture {
    pub command_label: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxfmlVerificationSummary {
    pub evaluation_summary: Option<String>,
    pub comparison_value: Option<Value>,
    pub effective_display_summary: Option<String>,
    pub blocked_reason: Option<String>,
    pub parse_status: Option<String>,
    pub green_tree_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcelObservationSummary {
    pub comparison_value: Option<Value>,
    pub observed_value_repr: Option<String>,
    pub effective_display_text: Option<String>,
    pub observed_formula_repr: Option<String>,
    pub capture_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_locale_pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_locale_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_locale_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxReplayMismatchRecord {
    pub mismatch_kind: String,
    pub severity: Option<String>,
    pub view_family: Option<String>,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxReplayExplainRecord {
    pub query_id: Option<String>,
    pub summary: Option<String>,
    pub mismatch_kind: String,
    pub severity: Option<String>,
    pub view_family: Option<String>,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCaseReport {
    pub case_id: String,
    pub entered_cell_text: String,
    pub artifact_catalog_entry: ProgrammaticArtifactCatalogEntry,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub value_match: Option<bool>,
    pub display_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub replay_mismatch_kinds: Vec<String>,
    pub replay_mismatch_records: Vec<OxReplayMismatchRecord>,
    pub replay_explain_records: Vec<OxReplayExplainRecord>,
    pub discrepancy_summary: Option<String>,
    pub oxfml_summary: OxfmlVerificationSummary,
    pub excel_summary: Option<ExcelObservationSummary>,
    pub spreadsheet_xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    pub upstream_gap_report: Option<VerificationObservationGapReport>,
    pub case_output_dir: String,
    pub scenario_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationObservationGapReport {
    pub oxfml_scope_required: Vec<String>,
    pub oxxlplay_supported_surfaces: Vec<String>,
    pub oxxlplay_missing_surfaces: Vec<String>,
    pub oxreplay_required_views: Vec<String>,
    pub oxreplay_current_bundle_views: Vec<String>,
    pub oxreplay_missing_views: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationBundleReport {
    pub bundle_id: String,
    pub output_root: String,
    pub host_profile: ProgrammaticHostProfile,
    pub capabilities: ProgrammaticCapabilityProfile,
    pub batch_plan: ProgrammaticBatchPlan,
    pub retained_artifact_catalog: Vec<ProgrammaticArtifactCatalogEntry>,
    pub case_reports: Vec<VerificationCaseReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OxxlplayBatchManifest {
    pub batch_id: String,
    pub output_root: String,
    pub shared_worker_options: OxxlplaySharedWorkerOptions,
    pub cases: Vec<OxxlplayBatchCaseManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OxxlplaySharedWorkerOptions {
    pub emit_bundle: bool,
    pub continue_after_case_failure: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cases_per_worker: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OxxlplayBatchCaseManifest {
    pub case_id: String,
    pub scenario_id: String,
    pub workbook_ref: String,
    pub workbook_kind: String,
    pub trigger: String,
    pub case_output_dir: String,
    pub observable_surfaces: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entered_cell_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_observation_scope: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cell_locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_workbook_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OxxlplayBatchOutputIndex {
    #[serde(default)]
    pub cases: Vec<OxxlplayBatchCaseOutputIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OxxlplayBatchCaseOutputIndex {
    pub case_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oxreplay_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_replay_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EffectiveExcelRenderContextProvenance {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    render_context_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EffectiveExcelRenderContext {
    #[serde(flatten)]
    context: ProgrammaticExcelRenderContext,
    provenance: EffectiveExcelRenderContextProvenance,
}

struct PreparedVerificationCase {
    case_dir: PathBuf,
    command_dir: PathBuf,
    oxxlplay_dir: PathBuf,
    oxreplay_dir: PathBuf,
    scenario_path: PathBuf,
    projection_path: PathBuf,
    effective_case: ProgrammaticFormulaCase,
    effective_excel_render_context: EffectiveExcelRenderContext,
    spreadsheet_xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    upstream_gap_report: Option<VerificationObservationGapReport>,
    oxfml_result: OxfmlCaseArtifacts,
    batch_case_manifest: OxxlplayBatchCaseManifest,
}

struct ExecutionOutcomeSurface {
    comparison_value: Value,
    ordinary_value_comparable: bool,
}

struct ReplayComparisonPlan {
    required_replay_views: Vec<String>,
    value_comparable: bool,
    display_comparable: bool,
}

pub trait VerificationCommandRunner {
    fn run_oxxlplay_capture_batch(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_validate_bundle(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_diff(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_explain(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String>;
}

#[derive(Debug, Default)]
pub struct ProcessVerificationCommandRunner;

impl VerificationCommandRunner for ProcessVerificationCommandRunner {
    fn run_oxxlplay_capture_batch(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String> {
        let manifest_path = absolute_path(manifest_path)?;
        run_command_capture(
            "oxxlplay-capture-batch",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxXlPlay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxxlplay-cli"),
                OsString::from("--"),
                OsString::from("capture-run-batch"),
                OsString::from("--manifest"),
                manifest_path.into_os_string(),
            ],
        )
    }

    fn run_oxreplay_validate_bundle(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String> {
        let manifest_path = absolute_path(manifest_path)?;
        run_command_capture(
            "oxreplay-validate-bundle",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("validate-bundle"),
                OsString::from("--bundle"),
                manifest_path.into_os_string(),
                OsString::from("--format"),
                OsString::from("json"),
            ],
        )
    }

    fn run_oxreplay_diff(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String> {
        let left_path = absolute_path(left_path)?;
        let right_path = absolute_path(right_path)?;
        run_command_capture(
            "oxreplay-diff",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("diff"),
                OsString::from("--left"),
                left_path.into_os_string(),
                OsString::from("--left-kind"),
                OsString::from(left_kind),
                OsString::from("--right"),
                right_path.into_os_string(),
                OsString::from("--right-kind"),
                OsString::from(right_kind),
            ],
        )
    }

    fn run_oxreplay_explain(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String> {
        let left_path = absolute_path(left_path)?;
        let right_path = absolute_path(right_path)?;
        run_command_capture(
            "oxreplay-explain",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("explain"),
                OsString::from("--left"),
                left_path.into_os_string(),
                OsString::from("--left-kind"),
                OsString::from(left_kind),
                OsString::from("--right"),
                right_path.into_os_string(),
                OsString::from("--right-kind"),
                OsString::from(right_kind),
            ],
        )
    }
}

pub fn load_verification_batch_request(
    input_path: impl AsRef<Path>,
) -> Result<VerificationBatchRequest, String> {
    let input_path = input_path.as_ref();
    let text = fs::read_to_string(input_path).map_err(|error| {
        format!(
            "failed to read verification batch request from `{}`: {error}",
            input_path.display()
        )
    })?;
    let request: VerificationBatchRequest = serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse verification batch request from `{}`: {error}",
            input_path.display()
        )
    })?;
    validate_verification_request(&request)?;
    Ok(request)
}

pub fn single_case_request(
    case_id: impl Into<String>,
    formula: impl Into<String>,
) -> VerificationBatchRequest {
    let config = default_verification_config();
    single_case_request_with_config(case_id, formula, &config)
}

pub fn single_case_request_with_config(
    case_id: impl Into<String>,
    formula: impl Into<String>,
    config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: config.host_profile.clone(),
        capabilities: config.capabilities.clone(),
        replay_policy: default_verification_replay_policy(),
        render_contexts: BTreeMap::new(),
        cases: vec![ProgrammaticFormulaCase {
            case_id: case_id.into(),
            entered_cell_text: formula.into(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_corpus_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        }],
    }
}

pub fn single_xml_case_request(
    case_id: impl Into<String>,
    workbook_path: impl Into<String>,
    locator: impl Into<String>,
) -> Result<VerificationBatchRequest, String> {
    let config = default_verification_config();
    single_xml_case_request_with_config(case_id, workbook_path, locator, &config)
}

pub fn single_xml_case_request_with_config(
    case_id: impl Into<String>,
    workbook_path: impl Into<String>,
    locator: impl Into<String>,
    config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
) -> Result<VerificationBatchRequest, String> {
    let case_id = case_id.into();
    let workbook_path = workbook_path.into();
    let locator = locator.into();
    let extraction = extract_cell_from_spreadsheet_xml(&workbook_path, &locator)?;

    Ok(VerificationBatchRequest {
        host_profile: config.host_profile.clone(),
        capabilities: config.capabilities.clone(),
        replay_policy: default_verification_replay_policy(),
        render_contexts: BTreeMap::new(),
        cases: vec![ProgrammaticFormulaCase {
            case_id,
            entered_cell_text: extraction.entered_cell_text,
            spreadsheet_xml_source: Some(
                crate::services::programmatic_testing::ProgrammaticSpreadsheetXmlSource {
                    workbook_path,
                    locator,
                },
            ),
            formatting_context: None,
            excel_render_context: None,
            render_context_ref: None,
        }],
    })
}

pub fn default_output_root() -> Result<PathBuf, String> {
    let repo_root = repo_root()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute timestamp for verification bundle: {error}"))?
        .as_secs();
    Ok(repo_root
        .join("target")
        .join("onecalc-verification")
        .join(format!("bundle-{timestamp}")))
}

pub fn run_verification_batch(
    request: &VerificationBatchRequest,
    output_root: impl AsRef<Path>,
) -> Result<VerificationBundleReport, String> {
    let runner = ProcessVerificationCommandRunner;
    run_verification_batch_with_runner(request, output_root, &runner)
}

pub fn run_verification_batch_with_runner<R: VerificationCommandRunner>(
    request: &VerificationBatchRequest,
    output_root: impl AsRef<Path>,
    runner: &R,
) -> Result<VerificationBundleReport, String> {
    let request = normalize_verification_request(request);
    validate_verification_request(&request)?;

    let repo_root = repo_root()?;
    let output_root = output_root.as_ref();
    fs::create_dir_all(output_root).map_err(|error| {
        format!(
            "failed to create verification bundle output root `{}`: {error}",
            output_root.display()
        )
    })?;

    let bundle_id = output_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("verification-bundle")
        .to_string();
    let commands_dir = output_root.join("commands");
    fs::create_dir_all(&commands_dir).map_err(|error| {
        format!(
            "failed to create verification command directory `{}`: {error}",
            commands_dir.display()
        )
    })?;
    let batch_plan =
        build_programmatic_batch_plan(&request.cases, &request.host_profile, &request.capabilities);

    write_json_file(output_root.join("input-request.json"), &request)?;
    write_json_file(output_root.join("batch-plan.json"), &batch_plan)?;

    let mut prepared_cases = Vec::with_capacity(request.cases.len());
    for case in &request.cases {
        prepared_cases.push(prepare_verification_case(
            &repo_root,
            output_root,
            case,
            &request.render_contexts,
            &request.host_profile,
            &request.capabilities,
        )?);
    }

    let mut batch_case_outputs = HashMap::new();
    let mut batch_failure_reason = None;
    if batch_plan.comparison_lane
        == crate::services::programmatic_testing::ProgrammaticComparisonLane::OxfmlAndExcel
    {
        let batch_output_root = output_root.join("oxxlplay-batch");
        fs::create_dir_all(&batch_output_root).map_err(|error| {
            format!(
                "failed to create OxXlPlay batch output root `{}`: {error}",
                batch_output_root.display()
            )
        })?;

        let batch_manifest = OxxlplayBatchManifest {
            batch_id: bundle_id.clone(),
            output_root: absolute_path(&batch_output_root)?
                .to_string_lossy()
                .replace('\\', "/"),
            shared_worker_options: OxxlplaySharedWorkerOptions {
                emit_bundle: true,
                continue_after_case_failure: true,
                max_cases_per_worker: None,
            },
            cases: prepared_cases
                .iter()
                .map(|case| case.batch_case_manifest.clone())
                .collect(),
        };
        let batch_manifest_path = commands_dir.join("oxxlplay-capture-batch.manifest.json");
        write_json_file(&batch_manifest_path, &batch_manifest)?;

        let capture = runner.run_oxxlplay_capture_batch(&batch_manifest_path)?;
        write_json_file(commands_dir.join("oxxlplay-capture-batch.json"), &capture)?;
        if capture.exit_code != 0 {
            batch_failure_reason = Some(format!(
                "OxXlPlay batch capture failed with exit code {}",
                capture.exit_code
            ));
        } else {
            let batch_output_index_path = batch_output_root.join("batch-output-index.json");
            match load_oxxlplay_batch_output_index(&batch_output_index_path) {
                Ok(batch_output_index) => {
                    write_json_file(
                        commands_dir.join("oxxlplay-capture-batch.output-index.json"),
                        &batch_output_index,
                    )?;
                    batch_case_outputs = batch_output_index
                        .cases
                        .into_iter()
                        .map(|case| (case.case_id.clone(), case))
                        .collect();
                }
                Err(error) => {
                    batch_failure_reason = Some(error);
                }
            }
        }
    }

    let mut retained_artifact_catalog = Vec::with_capacity(request.cases.len());
    let mut case_reports = Vec::with_capacity(request.cases.len());

    for prepared_case in prepared_cases {
        let case_id = prepared_case.effective_case.case_id.clone();
        let case_report = match batch_plan.comparison_lane {
            crate::services::programmatic_testing::ProgrammaticComparisonLane::OxfmlOnly => {
                finish_oxfml_only_case(&repo_root, prepared_case)?
            }
            crate::services::programmatic_testing::ProgrammaticComparisonLane::ExcelObservationBlocked => {
                finish_blocked_case(
                    &repo_root,
                    prepared_case,
                    format!(
                        "Excel observation is unavailable for host profile `{}` on `{}`",
                        request.host_profile.profile_id, request.capabilities.host_summary
                    ),
                    None,
                )?
            }
            crate::services::programmatic_testing::ProgrammaticComparisonLane::OxfmlAndExcel => {
                if let Some(reason) = &batch_failure_reason {
                    finish_blocked_case(&repo_root, prepared_case, reason.clone(), None)?
                } else {
                    let batch_case_output = batch_case_outputs.remove(&case_id);
                    finalize_excel_case(
                        &repo_root,
                        prepared_case,
                        batch_case_output.as_ref(),
                        request.replay_policy,
                        runner,
                    )?
                }
            }
        };
        retained_artifact_catalog.push(case_report.artifact_catalog_entry.clone());
        case_reports.push(case_report);
    }

    let report = VerificationBundleReport {
        bundle_id,
        output_root: display_repo_relative(output_root, &repo_root),
        host_profile: request.host_profile.clone(),
        capabilities: request.capabilities.clone(),
        batch_plan,
        retained_artifact_catalog,
        case_reports,
    };
    write_json_file(output_root.join("verification-bundle-report.json"), &report)?;
    write_json_file(
        output_root.join("retained-artifact-catalog.json"),
        &report.retained_artifact_catalog,
    )?;
    Ok(report)
}

fn normalize_verification_request(request: &VerificationBatchRequest) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: request.host_profile.clone(),
        capabilities: request.capabilities.clone(),
        replay_policy: request.replay_policy,
        render_contexts: request.render_contexts.clone(),
        cases: request
            .cases
            .iter()
            .map(normalize_programmatic_formula_case)
            .collect(),
    }
}

fn normalize_programmatic_formula_case(case: &ProgrammaticFormulaCase) -> ProgrammaticFormulaCase {
    let formatting_context = if case.spreadsheet_xml_source.is_none() {
        case.formatting_context
            .clone()
            .or_else(|| Some(default_programmatic_corpus_formatting_context()))
    } else {
        case.formatting_context.clone()
    };

    ProgrammaticFormulaCase {
        case_id: case.case_id.clone(),
        entered_cell_text: case.entered_cell_text.clone(),
        spreadsheet_xml_source: case.spreadsheet_xml_source.clone(),
        formatting_context,
        excel_render_context: case.excel_render_context.clone(),
        render_context_ref: case.render_context_ref.clone(),
    }
}

fn validate_verification_request(request: &VerificationBatchRequest) -> Result<(), String> {
    if request.cases.is_empty() {
        return Err("verification batch request must contain at least one case".to_string());
    }
    for case in &request.cases {
        if case.entered_cell_text.trim().is_empty() && case.spreadsheet_xml_source.is_none() {
            return Err(format!(
                "verification case `{}` must provide entered_cell_text or spreadsheet_xml_source",
                case.case_id
            ));
        }
        if case.excel_render_context.is_some() && case.render_context_ref.is_some() {
            return Err(format!(
                "verification case `{}` must not provide both `excel_render_context` and `render_context_ref`",
                case.case_id
            ));
        }
        if let Some(render_context_ref) = case.render_context_ref.as_deref() {
            if !request.render_contexts.contains_key(render_context_ref) {
                return Err(format!(
                    "verification case `{}` referenced unknown render context `{render_context_ref}`",
                    case.case_id
                ));
            }
        }
    }
    Ok(())
}

fn prepare_verification_case(
    repo_root: &Path,
    output_root: &Path,
    case: &ProgrammaticFormulaCase,
    render_contexts: &BTreeMap<String, ProgrammaticExcelRenderContext>,
    host_profile: &ProgrammaticHostProfile,
    capabilities: &ProgrammaticCapabilityProfile,
) -> Result<PreparedVerificationCase, String> {
    let case_dir = output_root
        .join("cases")
        .join(sanitize_case_id(&case.case_id));
    let command_dir = case_dir.join("commands");
    let oxxlplay_dir = case_dir.join("oxxlplay");
    let oxreplay_dir = case_dir.join("oxreplay");
    fs::create_dir_all(&command_dir).map_err(|error| {
        format!(
            "failed to create case command directory `{}`: {error}",
            command_dir.display()
        )
    })?;
    fs::create_dir_all(&oxxlplay_dir).map_err(|error| {
        format!(
            "failed to create OxXlPlay output directory `{}`: {error}",
            oxxlplay_dir.display()
        )
    })?;
    fs::create_dir_all(&oxreplay_dir).map_err(|error| {
        format!(
            "failed to create OxReplay output directory `{}`: {error}",
            oxreplay_dir.display()
        )
    })?;

    let spreadsheet_xml_extraction = if let Some(source) = &case.spreadsheet_xml_source {
        let extraction = extract_cell_from_spreadsheet_xml(&source.workbook_path, &source.locator)?;
        write_json_file(case_dir.join("xml-cell-extract.json"), &extraction)?;
        Some(extraction)
    } else {
        None
    };
    let requested_observation_scope =
        effective_requested_observation_scope(case, spreadsheet_xml_extraction.as_ref());
    write_json_file(
        case_dir.join("required-observation-scope.json"),
        &requested_observation_scope,
    )?;
    let effective_case = ProgrammaticFormulaCase {
        case_id: case.case_id.clone(),
        entered_cell_text: spreadsheet_xml_extraction
            .as_ref()
            .map(|extraction| extraction.entered_cell_text.clone())
            .unwrap_or_else(|| case.entered_cell_text.clone()),
        spreadsheet_xml_source: case.spreadsheet_xml_source.clone(),
        formatting_context: case.formatting_context.clone(),
        excel_render_context: case.excel_render_context.clone(),
        render_context_ref: case.render_context_ref.clone(),
    };
    let upstream_gap_report = spreadsheet_xml_extraction
        .as_ref()
        .map(build_observation_gap_report);
    if let Some(gap_report) = &upstream_gap_report {
        write_json_file(case_dir.join("upstream-gap-report.json"), gap_report)?;
    }

    let effective_excel_render_context = resolve_effective_excel_render_context(
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
        render_contexts,
    )?;

    write_json_file(
        case_dir.join("case-input.json"),
        &json!({
            "requested_case": case,
            "effective_case": &effective_case,
            "requested_observation_scope": &requested_observation_scope,
            "host_profile": host_profile,
            "capabilities": capabilities,
            "spreadsheet_xml_extraction": spreadsheet_xml_extraction,
            "excel_render_context": &effective_excel_render_context,
        }),
    )?;

    let oxfml_result = run_oxfml_case(
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
        Some(&effective_excel_render_context),
    )?;
    let projection_path = case_dir.join("oxfml-v1-replay-projection.json");
    persist_oxfml_case_artifacts(&case_dir, &projection_path, &oxfml_result)?;
    persist_oxfml_execution_context(
        &case_dir,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
        &effective_excel_render_context,
        "initial_pre_capture",
    )?;

    let workbook_path = case_dir.join("workbook.xml");
    let workbook_write = materialize_case_workbook(
        &workbook_path,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
    )?;
    write_json_file(command_dir.join("write-workbook.json"), &workbook_write)?;

    let scenario_path = case_dir.join("scenario.json");
    let scenario_json = build_oxxlplay_scenario_json(
        repo_root,
        &case_dir,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
        &effective_excel_render_context,
    );
    write_json_file(&scenario_path, &scenario_json)?;

    let batch_case_manifest = build_oxxlplay_batch_case_manifest(
        &case_dir,
        &oxxlplay_dir,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
    )?;

    Ok(PreparedVerificationCase {
        case_dir,
        command_dir,
        oxxlplay_dir,
        oxreplay_dir,
        scenario_path,
        projection_path,
        effective_case,
        effective_excel_render_context,
        spreadsheet_xml_extraction,
        upstream_gap_report,
        oxfml_result,
        batch_case_manifest,
    })
}

fn finalize_excel_case<R: VerificationCommandRunner>(
    repo_root: &Path,
    mut prepared: PreparedVerificationCase,
    batch_case_output: Option<&OxxlplayBatchCaseOutputIndex>,
    replay_policy: VerificationReplayPolicy,
    runner: &R,
) -> Result<VerificationCaseReport, String> {
    let Some(batch_case_output) = batch_case_output else {
        return finish_blocked_case(
            repo_root,
            prepared,
            "OxXlPlay batch output index did not include this case".to_string(),
            None,
        );
    };

    if matches!(replay_policy, VerificationReplayPolicy::Never) {
        return finish_blocked_case(
            repo_root,
            prepared,
            "Comparison blocked: replay equivalence is required for verification but replay policy is `Never`".to_string(),
            None,
        );
    }

    let resolved_output_dir = resolve_repo_or_absolute_path(
        repo_root,
        batch_case_output.output_dir.as_deref(),
        prepared.oxxlplay_dir.clone(),
    );
    let capture_path = resolve_repo_or_absolute_path(
        repo_root,
        batch_case_output.capture_path.as_deref(),
        resolved_output_dir.join("capture.json"),
    );
    let manifest_path = resolve_repo_or_absolute_path(
        repo_root,
        batch_case_output.oxreplay_manifest_path.as_deref(),
        resolved_output_dir.join("oxreplay-manifest.json"),
    );
    let normalized_replay_path = resolve_repo_or_absolute_path(
        repo_root,
        batch_case_output.normalized_replay_path.as_deref(),
        resolved_output_dir
            .join("views")
            .join("normalized-replay.json"),
    );

    import_captured_render_context_and_refresh_oxfml_if_needed(
        &mut prepared,
        &resolved_output_dir,
        run_oxfml_case,
    )?;

    let display_comparison_enabled = programmatic_display_comparison_enabled(
        &prepared.effective_case,
        prepared.spreadsheet_xml_extraction.as_ref(),
    );
    let excel_programmatic_authoring_rejection =
        excel_case_output_is_programmatic_authoring_rejection(batch_case_output);
    let excel_outcome = match classify_excel_execution_outcome(batch_case_output) {
        Ok(outcome) => outcome,
        Err(reason) => return finish_blocked_case(repo_root, prepared, reason, None),
    };

    let excel_summary = if excel_outcome.ordinary_value_comparable {
        let mut summary = summarize_excel_capture(capture_path)?;
        annotate_excel_observation_render_context(
            &prepared.effective_excel_render_context,
            &mut summary,
        );
        write_json_file(
            prepared.case_dir.join("excel-observation-summary.json"),
            &summary,
        )?;
        Some(summary)
    } else {
        None
    };

    let (oxfml_outcome, oxfml_execution_outcome_is_synthetic) =
        if excel_programmatic_authoring_rejection {
            prepared.oxfml_result.summary.blocked_reason = None;
            (
                ExecutionOutcomeSurface {
                    comparison_value: normalized_pre_execution_rejection_outcome(),
                    ordinary_value_comparable: false,
                },
                true,
            )
        } else if let Some(failure_reason) = prepared.oxfml_result.execution_failure.clone() {
            if let Some(outcome) = synthetic_oxfml_pre_execution_rejection_outcome_for_failure(
                &prepared.oxfml_result.summary,
                &failure_reason,
                &excel_outcome,
            ) {
                prepared.oxfml_result.summary.blocked_reason = None;
                (outcome, true)
            } else {
                return finish_blocked_case(repo_root, prepared, failure_reason, excel_summary);
            }
        } else {
            (
                classify_oxfml_execution_outcome(&prepared.oxfml_result.replay_projection_json),
                false,
            )
        };
    if locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
        &prepared.effective_case,
        prepared.spreadsheet_xml_extraction.as_ref(),
        &prepared.effective_excel_render_context,
        &prepared.oxfml_result.replay_projection_json,
        prepared.oxfml_result.summary.comparison_value.as_ref(),
    ) {
        let blocked_reason = "Comparison blocked: `comparison_value` is not comparison-eligible for this non-XML programmatic case because Excel render locale/separator state is unpinned while OxFml marks locale-sensitive semantic text dependency under explicit locale context".to_string();
        prepared.oxfml_result.summary.blocked_reason = Some(blocked_reason.clone());
        return finish_case_report(
            repo_root,
            prepared,
            ProgrammaticComparisonStatus::Blocked,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(blocked_reason),
            excel_summary,
        );
    }
    let comparison_plan =
        build_replay_comparison_plan(display_comparison_enabled, &oxfml_outcome, &excel_outcome);
    let requested_replay_views = comparison_plan.required_replay_views.clone();

    let compare_ready_projection_path = if oxfml_execution_outcome_is_synthetic {
        materialize_synthetic_compare_ready_projection(
            prepared
                .oxreplay_dir
                .join("oxfml-v1-replay-projection.compare-ready.json"),
            &prepared.effective_case,
            &oxfml_outcome.comparison_value,
        )?
    } else {
        materialize_compare_ready_projection(
            &prepared.projection_path,
            prepared
                .oxreplay_dir
                .join("oxfml-v1-replay-projection.compare-ready.json"),
            &comparison_plan.required_replay_views,
            &oxfml_outcome.comparison_value,
        )?
    };
    let compare_ready_replay_path = if excel_outcome.ordinary_value_comparable {
        materialize_compare_ready_normalized_replay(
            &normalized_replay_path,
            prepared
                .oxreplay_dir
                .join("normalized-replay.compare-ready.json"),
            &comparison_plan.required_replay_views,
            &excel_outcome.comparison_value,
        )?
    } else {
        materialize_synthetic_compare_ready_replay(
            prepared
                .oxreplay_dir
                .join("normalized-replay.compare-ready.json"),
            &prepared.effective_case.case_id,
            &excel_outcome.comparison_value,
        )?
    };

    let compare_ready_projection = read_json_file(&compare_ready_projection_path)?;
    let compare_ready_replay = read_json_file(&compare_ready_replay_path)?;
    if let Some(blocked_reason) = missing_required_replay_view_reason(
        &compare_ready_projection,
        &compare_ready_replay,
        &comparison_plan.required_replay_views,
    ) {
        prepared.oxfml_result.summary.blocked_reason = Some(blocked_reason.clone());
        return finish_case_report(
            repo_root,
            prepared,
            ProgrammaticComparisonStatus::Blocked,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(blocked_reason),
            excel_summary,
        );
    }

    if excel_outcome.ordinary_value_comparable {
        let validate_capture = runner.run_oxreplay_validate_bundle(&manifest_path)?;
        write_json_file(
            prepared.command_dir.join("oxreplay-validate-bundle.json"),
            &validate_capture,
        )?;
        if !validate_capture.stdout.trim().is_empty() {
            write_json_text_file(
                prepared.oxreplay_dir.join("validate-bundle.report.json"),
                &validate_capture.stdout,
            )?;
        }
        if validate_capture.exit_code != 0 {
            let blocked_reason = format!(
                "Comparison blocked: OxReplay validate-bundle failed (exit code {})",
                validate_capture.exit_code
            );
            prepared.oxfml_result.summary.blocked_reason = Some(blocked_reason.clone());
            return finish_case_report(
                repo_root,
                prepared,
                ProgrammaticComparisonStatus::Blocked,
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Some(blocked_reason),
                excel_summary,
            );
        }
    }

    let diff_capture = runner.run_oxreplay_diff(
        &compare_ready_projection_path,
        "oxfml-v1-replay-projection",
        &compare_ready_replay_path,
        "normalized-replay",
    )?;
    write_json_file(
        prepared.command_dir.join("oxreplay-diff.json"),
        &diff_capture,
    )?;
    if !diff_capture.stdout.trim().is_empty() {
        write_json_text_file(
            prepared.oxreplay_dir.join("diff.report.json"),
            &diff_capture.stdout,
        )?;
    }
    if diff_capture.exit_code != 0 {
        let blocked_reason = format!(
            "Comparison blocked: OxReplay diff failed (exit code {})",
            diff_capture.exit_code
        );
        prepared.oxfml_result.summary.blocked_reason = Some(blocked_reason.clone());
        return finish_case_report(
            repo_root,
            prepared,
            ProgrammaticComparisonStatus::Blocked,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(blocked_reason),
            excel_summary,
        );
    }

    let explain_capture = runner.run_oxreplay_explain(
        &compare_ready_projection_path,
        "oxfml-v1-replay-projection",
        &compare_ready_replay_path,
        "normalized-replay",
    )?;
    write_json_file(
        prepared.command_dir.join("oxreplay-explain.json"),
        &explain_capture,
    )?;
    if !explain_capture.stdout.trim().is_empty() {
        write_json_text_file(
            prepared.oxreplay_dir.join("explain.report.json"),
            &explain_capture.stdout,
        )?;
    }
    if explain_capture.exit_code != 0 {
        let blocked_reason = format!(
            "Comparison blocked: OxReplay explain failed (exit code {})",
            explain_capture.exit_code
        );
        prepared.oxfml_result.summary.blocked_reason = Some(blocked_reason.clone());
        return finish_case_report(
            repo_root,
            prepared,
            ProgrammaticComparisonStatus::Blocked,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(blocked_reason),
            excel_summary,
        );
    }

    let diff_report = parse_json_text(&diff_capture.stdout, "OxReplay diff stdout")?;
    let is_equivalent = diff_report
        .get("equivalent")
        .and_then(Value::as_bool)
        .ok_or_else(|| "OxReplay diff output did not contain a boolean `equivalent`".to_string())?;
    let replay_mismatch_records = filter_replay_mismatch_records_to_requested_views(
        parse_oxreplay_mismatch_records(&diff_report),
        &requested_replay_views,
    );
    let replay_explain_records = filter_replay_explain_records_to_requested_views(
        parse_oxreplay_explain_records(&explain_capture.stdout)?,
        &requested_replay_views,
    );
    let replay_mismatch_kinds = replay_mismatch_records
        .iter()
        .map(|record| record.mismatch_kind.clone())
        .collect::<Vec<_>>();
    let value_match = derive_replay_axis_match(
        &replay_mismatch_records,
        "comparison_value",
        comparison_plan.value_comparable,
    );
    let display_match = derive_replay_axis_match(
        &replay_mismatch_records,
        "effective_display_text",
        comparison_plan.display_comparable,
    );

    let comparison_status = derive_host_comparison_status_from_replay(is_equivalent);
    let discrepancy_summary = build_discrepancy_summary(
        comparison_status,
        value_match,
        display_match,
        &replay_mismatch_records,
        &prepared.oxfml_result.summary,
        excel_summary.as_ref(),
    );

    finish_case_report(
        repo_root,
        prepared,
        comparison_status,
        value_match,
        display_match,
        Some(is_equivalent),
        replay_mismatch_kinds,
        replay_mismatch_records,
        replay_explain_records,
        discrepancy_summary,
        excel_summary,
    )
}

fn finish_oxfml_only_case(
    repo_root: &Path,
    prepared: PreparedVerificationCase,
) -> Result<VerificationCaseReport, String> {
    if let Some(failure_reason) = prepared.oxfml_result.execution_failure.clone() {
        return finish_blocked_case(repo_root, prepared, failure_reason, None);
    }
    finish_case_report(
        repo_root,
        prepared,
        ProgrammaticComparisonStatus::Matched,
        None,
        None,
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
}

fn finish_blocked_case(
    repo_root: &Path,
    prepared: PreparedVerificationCase,
    blocked_reason: String,
    excel_summary: Option<ExcelObservationSummary>,
) -> Result<VerificationCaseReport, String> {
    finish_case_report(
        repo_root,
        prepared,
        ProgrammaticComparisonStatus::Blocked,
        None,
        None,
        None,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(blocked_reason),
        excel_summary,
    )
}

fn finish_case_report(
    repo_root: &Path,
    prepared: PreparedVerificationCase,
    comparison_status: ProgrammaticComparisonStatus,
    value_match: Option<bool>,
    display_match: Option<bool>,
    replay_equivalent: Option<bool>,
    replay_mismatch_kinds: Vec<String>,
    replay_mismatch_records: Vec<OxReplayMismatchRecord>,
    replay_explain_records: Vec<OxReplayExplainRecord>,
    discrepancy_summary: Option<String>,
    excel_summary: Option<ExcelObservationSummary>,
) -> Result<VerificationCaseReport, String> {
    let artifact_catalog_entry = build_programmatic_artifact_catalog_entry(
        format!(
            "artifact-{}",
            sanitize_case_id(&prepared.effective_case.case_id)
        ),
        prepared.effective_case.case_id.clone(),
        comparison_status,
    );
    let report = VerificationCaseReport {
        case_id: prepared.effective_case.case_id.clone(),
        entered_cell_text: prepared.effective_case.entered_cell_text.clone(),
        artifact_catalog_entry: artifact_catalog_entry.clone(),
        comparison_status,
        value_match,
        display_match,
        replay_equivalent,
        replay_mismatch_kinds,
        replay_mismatch_records,
        replay_explain_records,
        discrepancy_summary,
        oxfml_summary: prepared.oxfml_result.summary,
        excel_summary,
        spreadsheet_xml_extraction: prepared.spreadsheet_xml_extraction,
        upstream_gap_report: prepared.upstream_gap_report,
        case_output_dir: display_repo_relative(&prepared.case_dir, repo_root),
        scenario_path: display_repo_relative(&prepared.scenario_path, repo_root),
    };
    write_json_file(
        prepared
            .case_dir
            .join("programmatic-artifact-catalog-entry.json"),
        &artifact_catalog_entry,
    )?;
    write_json_file(prepared.case_dir.join("comparison-summary.json"), &report)?;
    Ok(report)
}

fn normalized_pre_execution_rejection_outcome() -> Value {
    json!({
        "outcome_kind": "rejected",
        "outcome_stage": "pre_execution",
        "class_id": "programmatic_formula_rejection",
        "lane_reason_code": "authoring_or_bind_rejected"
    })
}

fn normalized_completed_execution_outcome() -> Value {
    json!({
        "outcome_kind": "completed",
        "outcome_stage": "post_execution",
        "class_id": "worksheet_surface",
        "lane_reason_code": "value_or_display_surface_available"
    })
}

fn classify_oxfml_execution_outcome(projection: &Value) -> ExecutionOutcomeSurface {
    let commit_decision_kind = projection
        .get("commit_decision_kind")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase());
    let trace_event_kinds = projection
        .get("trace_event_kinds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rejected = commit_decision_kind.as_deref() == Some("rejected")
        || trace_event_kinds.iter().any(|value| {
            value
                .as_str()
                .map(|item| matches!(item, "RejectIssued" | "CommitRejected"))
                .unwrap_or(false)
        });

    if rejected {
        ExecutionOutcomeSurface {
            comparison_value: normalized_pre_execution_rejection_outcome(),
            ordinary_value_comparable: false,
        }
    } else {
        ExecutionOutcomeSurface {
            comparison_value: normalized_completed_execution_outcome(),
            ordinary_value_comparable: true,
        }
    }
}

fn excel_case_output_is_programmatic_authoring_rejection(
    batch_case_output: &OxxlplayBatchCaseOutputIndex,
) -> bool {
    batch_case_output
        .error
        .as_deref()
        .map(|error| {
            let lower = error.to_ascii_lowercase();
            lower.contains("programmatic_formula_authoring_failed")
                || lower.contains("formula2 assignment")
                || lower.contains("0x800a03ec")
        })
        .unwrap_or(false)
}

fn classify_excel_execution_outcome(
    batch_case_output: &OxxlplayBatchCaseOutputIndex,
) -> Result<ExecutionOutcomeSurface, String> {
    if batch_case_output.status.as_deref() == Some("succeeded") && batch_case_output.error.is_none()
    {
        return Ok(ExecutionOutcomeSurface {
            comparison_value: normalized_completed_execution_outcome(),
            ordinary_value_comparable: true,
        });
    }

    if excel_case_output_is_programmatic_authoring_rejection(batch_case_output) {
        return Ok(ExecutionOutcomeSurface {
            comparison_value: normalized_pre_execution_rejection_outcome(),
            ordinary_value_comparable: false,
        });
    }

    if let Some(error) = &batch_case_output.error {
        return Err(error.clone());
    }

    if let Some(status) = batch_case_output.status.as_deref() {
        return Err(format!("OxXlPlay batch case reported status `{status}`"));
    }

    Err("OxXlPlay batch case did not provide comparable execution evidence".to_string())
}

fn oxfml_execution_failure_is_explicit_syntax_diagnostic(
    summary: &OxfmlVerificationSummary,
    failure_reason: &str,
) -> bool {
    summary.parse_status.as_deref() == Some("Diagnostics")
        && failure_reason
            .to_ascii_lowercase()
            .contains("syntax diagnostics")
}

fn synthetic_oxfml_pre_execution_rejection_outcome_for_failure(
    summary: &OxfmlVerificationSummary,
    failure_reason: &str,
    excel_outcome: &ExecutionOutcomeSurface,
) -> Option<ExecutionOutcomeSurface> {
    if !oxfml_execution_failure_is_explicit_syntax_diagnostic(summary, failure_reason) {
        return None;
    }

    let normalized_rejection = normalized_pre_execution_rejection_outcome();
    if excel_outcome.ordinary_value_comparable
        || excel_outcome.comparison_value != normalized_rejection
    {
        return None;
    }

    Some(ExecutionOutcomeSurface {
        comparison_value: normalized_rejection,
        ordinary_value_comparable: false,
    })
}

fn build_replay_comparison_plan(
    display_comparison_enabled: bool,
    oxfml_outcome: &ExecutionOutcomeSurface,
    excel_outcome: &ExecutionOutcomeSurface,
) -> ReplayComparisonPlan {
    let value_comparable =
        oxfml_outcome.ordinary_value_comparable && excel_outcome.ordinary_value_comparable;
    let display_comparable = value_comparable && display_comparison_enabled;
    let mut required_replay_views = vec![EXECUTION_OUTCOME_VIEW_FAMILY.to_string()];
    if value_comparable {
        required_replay_views.push("comparison_value".to_string());
    }
    if display_comparable {
        required_replay_views.push("effective_display_text".to_string());
    }

    ReplayComparisonPlan {
        required_replay_views,
        value_comparable,
        display_comparable,
    }
}

fn compare_ready_view_is_present(replay: &Value, family: &str) -> bool {
    replay
        .get("comparison_views")
        .and_then(Value::as_array)
        .is_some_and(|views| {
            views.iter().any(|view| {
                view.get("view_family").and_then(Value::as_str) == Some(family)
                    && view.get("value").is_some()
                    && !view.get("value").is_some_and(Value::is_null)
            })
        })
}

fn missing_required_replay_view_reason(
    left: &Value,
    right: &Value,
    required_views: &[String],
) -> Option<String> {
    for view in required_views {
        let left_present = compare_ready_view_is_present(left, view);
        let right_present = compare_ready_view_is_present(right, view);
        if !left_present || !right_present {
            return Some(format!(
                "Comparison blocked: required replay comparison view `{view}` was unavailable on {}{}",
                if !left_present { "OxFml" } else { "" },
                if !left_present && !right_present {
                    " and Excel"
                } else if !right_present {
                    "Excel"
                } else {
                    ""
                }
            ));
        }
    }
    None
}

fn derive_replay_axis_match(
    replay_mismatch_records: &[OxReplayMismatchRecord],
    family: &str,
    expected: bool,
) -> Option<bool> {
    expected.then(|| {
        !replay_mismatch_records.iter().any(|record| {
            record.view_family.as_deref() == Some(family) || record.mismatch_kind == family
        })
    })
}

fn derive_host_comparison_status_from_replay(is_equivalent: bool) -> ProgrammaticComparisonStatus {
    if is_equivalent {
        ProgrammaticComparisonStatus::Matched
    } else {
        ProgrammaticComparisonStatus::Mismatched
    }
}

fn build_oxxlplay_batch_case_manifest(
    case_dir: &Path,
    oxxlplay_dir: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Result<OxxlplayBatchCaseManifest, String> {
    let locator = spreadsheet_xml_extraction
        .map(|extraction| extraction.locator.clone())
        .unwrap_or_else(|| "Sheet1!A1".to_string());
    let workbook_kind = if spreadsheet_xml_extraction.is_some() {
        "spreadsheetml-2003-import"
    } else {
        "programmatic-formula"
    };
    let requested_observation_scope = serde_json::to_value(effective_requested_observation_scope(
        case,
        spreadsheet_xml_extraction,
    ))
    .map(Some)
    .map_err(|error| {
        format!(
            "failed to serialize requested observation scope for `{}`: {error}",
            case.case_id
        )
    })?;

    Ok(OxxlplayBatchCaseManifest {
        case_id: case.case_id.clone(),
        scenario_id: format!("onecalc_verify_{}", sanitize_case_id(&case.case_id)),
        workbook_ref: absolute_path(&case_dir.join("workbook.xml"))?
            .to_string_lossy()
            .replace('\\', "/"),
        workbook_kind: workbook_kind.to_string(),
        trigger: "open_then_recalc".to_string(),
        case_output_dir: absolute_path(oxxlplay_dir)?
            .to_string_lossy()
            .replace('\\', "/"),
        observable_surfaces: build_oxxlplay_observable_surfaces(
            &locator,
            programmatic_effective_display_surface_requested(case, spreadsheet_xml_extraction),
        ),
        entered_cell_text: if spreadsheet_xml_extraction.is_none() {
            Some(case.entered_cell_text.clone())
        } else {
            None
        },
        requested_observation_scope,
        source_cell_locator: spreadsheet_xml_extraction
            .map(|extraction| extraction.locator.clone()),
        source_workbook_path: spreadsheet_xml_extraction
            .map(|extraction| extraction.workbook_path.clone()),
    })
}

fn load_oxxlplay_batch_output_index(
    batch_output_index_path: &Path,
) -> Result<OxxlplayBatchOutputIndex, String> {
    let value = read_json_file(batch_output_index_path)?;
    serde_json::from_value(value).map_err(|error| {
        format!(
            "failed to parse OxXlPlay batch output index `{}`: {error}",
            batch_output_index_path.display()
        )
    })
}

fn resolve_repo_or_absolute_path(
    repo_root: &Path,
    raw_path: Option<&str>,
    default_path: PathBuf,
) -> PathBuf {
    match raw_path {
        None => default_path,
        Some(raw_path) => {
            let path = PathBuf::from(raw_path);
            if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            }
        }
    }
}

struct OxfmlCaseArtifacts {
    summary: OxfmlVerificationSummary,
    replay_projection_json: Value,
    execution_failure: Option<String>,
}

fn run_oxfml_case(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    effective_excel_render_context: Option<&EffectiveExcelRenderContext>,
) -> Result<OxfmlCaseArtifacts, String> {
    let bridge = NativeOxfmlHostSession::default();
    let formula_edit_result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: case.case_id.clone(),
            entered_text: case.entered_cell_text.clone(),
            cursor_offset: case.entered_cell_text.len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::FullSemanticPlan,
            formatting_request: None,
            scenario_policy: crate::adapters::oxfml::ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: crate::adapters::oxfml::RecalcModeRequest::Auto,
            // Empty falls back to en-US in the bridge. The verification
            // path attaches its own typed-context query bundle a few
            // lines below, so this language_tag only steers the
            // bridge's interactive response (parse / bind / popup),
            // not the runtime locale used for the verification report.
            language_tag: String::new(),
            formal_input_bindings: Vec::new(),
            // Verification surfaces a per-prepared-call walk in the
            // workbench artifacts, so it needs the rich trace.
            trace_mode: crate::adapters::oxfml::TraceModeRequest::PreparedCalls,
        })
        .map_err(|error| {
            format!(
                "live OxFml bridge failed for case `{}`: {error:?}",
                case.case_id
            )
        })?;

    let source = FormulaSourceRecord::new(case.case_id.clone(), 1, case.entered_cell_text.clone())
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let locale_ctx = verification_locale_context(
        case,
        spreadsheet_xml_extraction,
        effective_excel_render_context,
    );
    let typed_query_bundle =
        TypedContextQueryBundle::new(None, None, Some(&locale_ctx), None, None);
    let include_effective_display =
        programmatic_display_comparison_enabled(case, spreadsheet_xml_extraction);
    let runtime_request = if let Some(context) =
        effective_verification_publication_context(case, spreadsheet_xml_extraction)
    {
        RuntimeFormulaRequest::new(source, typed_query_bundle)
            .with_verification_publication_context(context)
    } else {
        RuntimeFormulaRequest::new(source, typed_query_bundle)
    };
    let runtime_outcome = RuntimeEnvironment::new().execute(runtime_request);

    let evaluation_summary = formula_edit_result
        .document
        .value_presentation
        .as_ref()
        .map(|value| value.evaluation_summary.clone());
    let parse_status = formula_edit_result
        .document
        .parse_summary
        .as_ref()
        .map(|summary| summary.status.clone());
    let green_tree_key = Some(
        formula_edit_result
            .document
            .editor_syntax_snapshot
            .green_tree_key
            .clone(),
    );
    let bridge_blocked_reason = formula_edit_result
        .document
        .value_presentation
        .as_ref()
        .and_then(|value| value.blocked_reason.clone())
        .or_else(|| {
            formula_edit_result
                .document
                .provenance_summary
                .as_ref()
                .and_then(|summary| summary.blocked_reason.clone())
        });
    let display_context_blocked_reason =
        missing_programmatic_display_context_reason(case, spreadsheet_xml_extraction)
            .map(ToOwned::to_owned);

    match runtime_outcome {
        Ok(runtime_result) => {
            let projection = ReplayProjectionService::project(
                ReplayProjectionRequest::runtime_result(&runtime_result)
                    .with_source_case_id(case.case_id.clone())
                    .with_shared_scenario_alias(format!(
                        "onecalc_verify_{}",
                        sanitize_case_id(&case.case_id)
                    )),
            );
            let projection_json =
                serialize_replay_projection(&projection, include_effective_display);
            let summary = OxfmlVerificationSummary {
                evaluation_summary,
                comparison_value: projection_comparison_value(&projection_json, "comparison_value"),
                effective_display_summary: include_effective_display.then(|| {
                    runtime_result
                        .verification_publication_surface
                        .effective_display_text
                        .clone()
                }),
                blocked_reason: bridge_blocked_reason.or(display_context_blocked_reason),
                parse_status,
                green_tree_key,
            };
            Ok(OxfmlCaseArtifacts {
                summary,
                replay_projection_json: projection_json,
                execution_failure: None,
            })
        }
        Err(error) => {
            let failure_reason = format!(
                "OxFml runtime execution failed for case `{}`: {error}",
                case.case_id
            );
            let summary = OxfmlVerificationSummary {
                evaluation_summary,
                comparison_value: None,
                effective_display_summary: None,
                blocked_reason: Some(failure_reason.clone()),
                parse_status,
                green_tree_key,
            };
            Ok(OxfmlCaseArtifacts {
                summary,
                replay_projection_json: Value::Null,
                execution_failure: Some(failure_reason),
            })
        }
    }
}

fn verification_locale_context(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    effective_excel_render_context: Option<&EffectiveExcelRenderContext>,
) -> LocaleFormatContext<'static> {
    let date_system = if effective_programmatic_date1904(case, spreadsheet_xml_extraction) {
        WorkbookDateSystem::System1904
    } else {
        WorkbookDateSystem::System1900
    };
    let trusted_excel_separator_context =
        trusted_excel_separator_context(effective_excel_render_context);
    let profile_id = parse_programmatic_format_profile_id(
        spreadsheet_xml_extraction
            .map(|extraction| extraction.workbook_format_profile_hint.as_str())
            .or_else(|| {
                case.formatting_context
                    .as_ref()
                    .and_then(|context| context.format_profile_id.as_deref())
            })
            .or_else(|| {
                trusted_excel_separator_context
                    .as_ref()
                    .and_then(|context| context.requested_format_profile_id)
            }),
    );
    let mut profile = format_profile(profile_id).clone();
    apply_trusted_separator_overrides_to_profile(
        &mut profile,
        trusted_excel_separator_context.as_ref(),
    );

    LocaleFormatContext {
        profile,
        date_system,
        parser: &HOST_TEST_LOCALE_VALUE_PARSER,
        formatter: &HOST_TEST_FORMAT_CODE_ENGINE,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrustedExcelSeparatorContext<'a> {
    requested_format_profile_id: Option<&'a str>,
    decimal_separator: Option<&'a str>,
    thousands_separator: Option<&'a str>,
    list_separator: Option<&'a str>,
    date_separator: Option<&'a str>,
    time_separator: Option<&'a str>,
}

fn trusted_excel_separator_context(
    effective_excel_render_context: Option<&EffectiveExcelRenderContext>,
) -> Option<TrustedExcelSeparatorContext<'_>> {
    let context = effective_excel_render_context
        .map(|value| &value.context)
        .filter(|context| context.trusted)
        .filter(|context| {
            context.decimal_separator.is_some()
                || context.thousands_separator.is_some()
                || context.list_separator.is_some()
                || context.date_separator.is_some()
                || context.time_separator.is_some()
                || context.requested_format_profile_id.is_some()
        })?;

    Some(TrustedExcelSeparatorContext {
        requested_format_profile_id: context.requested_format_profile_id.as_deref(),
        decimal_separator: context.decimal_separator.as_deref(),
        thousands_separator: context.thousands_separator.as_deref(),
        list_separator: context.list_separator.as_deref(),
        date_separator: context.date_separator.as_deref(),
        time_separator: context.time_separator.as_deref(),
    })
}

fn leaked_separator_token(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

fn apply_trusted_separator_overrides_to_profile(
    profile: &mut FormatProfile,
    trusted_excel_separator_context: Option<&TrustedExcelSeparatorContext<'_>>,
) {
    let Some(context) = trusted_excel_separator_context else {
        return;
    };

    if let Some(decimal_separator) = context.decimal_separator {
        profile.decimal_separator = leaked_separator_token(decimal_separator);
    }
    if let Some(thousands_separator) = context.thousands_separator {
        profile.thousands_separator = leaked_separator_token(thousands_separator);
    }
    if let Some(date_separator) = context.date_separator {
        profile.date_separator = leaked_separator_token(date_separator);
    }
    if let Some(time_separator) = context.time_separator {
        profile.time_separator = leaked_separator_token(time_separator);
    }
}

fn effective_programmatic_date1904(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> bool {
    spreadsheet_xml_extraction
        .and_then(|value| value.date1904)
        .or_else(|| {
            case.formatting_context
                .as_ref()
                .and_then(|context| context.date1904)
        })
        == Some(true)
}

fn parse_programmatic_format_profile_id(raw: Option<&str>) -> LocaleProfileId {
    match raw
        .map(str::trim)
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("enus") | Some("en-us") | Some("en_us") => LocaleProfileId::EnUs,
        Some("currentexcelhost")
        | Some("current_excel_host")
        | Some("excel_host")
        | Some("windows_excel_default") => LocaleProfileId::CurrentExcelHost,
        _ => LocaleProfileId::CurrentExcelHost,
    }
}

struct HostTestLocaleValueParser;

struct HostTestFormatCodeEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerificationNegativeStyle {
    Minus,
    Parentheses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VerificationFormatPattern {
    Fixed {
        decimals: i32,
        use_grouping: bool,
    },
    Currency {
        decimals: i32,
        use_grouping: bool,
        negative_style: VerificationNegativeStyle,
    },
    Percent {
        decimals: i32,
    },
    IsoDate,
}

static HOST_TEST_LOCALE_VALUE_PARSER: HostTestLocaleValueParser = HostTestLocaleValueParser;

static HOST_TEST_FORMAT_CODE_ENGINE: HostTestFormatCodeEngine = HostTestFormatCodeEngine;

impl LocaleValueParser for HostTestLocaleValueParser {
    fn parse_value_text(
        &self,
        profile: &oxfunc_core::locale_format::FormatProfile,
        date_system: WorkbookDateSystem,
        text: &str,
    ) -> Result<f64, ParseFailure> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(ParseFailure::UnsupportedText(trimmed.to_string()));
        }

        if let Some(stripped) = trimmed.strip_suffix('%') {
            return parse_number_with_profile(profile, stripped)
                .map(|value| value / 100.0)
                .ok_or_else(|| ParseFailure::UnsupportedText(trimmed.to_string()));
        }

        let (negative, body) = if let Some(rest) = trimmed.strip_prefix('-') {
            (true, rest.trim_start())
        } else {
            (false, trimmed)
        };

        if let Some(rest) = body.strip_prefix(profile.currency_symbol) {
            let parsed = parse_number_with_profile(profile, rest.trim_start())
                .ok_or_else(|| ParseFailure::UnsupportedText(trimmed.to_string()))?;
            return Ok(if negative { -parsed } else { parsed });
        }

        if let Some((year, month, day)) = parse_iso_ymd(trimmed) {
            return excel_serial_from_ymd(date_system, year, month, day)
                .ok_or_else(|| ParseFailure::UnsupportedText(trimmed.to_string()));
        }

        if profile.id == LocaleProfileId::EnUs {
            if let Some((year, month, day)) = parse_en_us_slash_date(trimmed) {
                return excel_serial_from_ymd(date_system, year, month, day)
                    .ok_or_else(|| ParseFailure::UnsupportedText(trimmed.to_string()));
            }
        }

        parse_number_with_profile(profile, trimmed)
            .ok_or_else(|| ParseFailure::UnsupportedText(trimmed.to_string()))
    }
}

impl FormatCodeEngine for HostTestFormatCodeEngine {
    fn render_with_code(
        &self,
        profile: &oxfunc_core::locale_format::FormatProfile,
        date_system: WorkbookDateSystem,
        value: f64,
        code: &str,
    ) -> Result<ExcelText, FormatFailure> {
        let rendered = match classify_verification_format_code(profile, code, value) {
            Some(VerificationFormatPattern::Fixed {
                decimals,
                use_grouping,
            }) => render_fixed_with_style(
                profile,
                value,
                decimals,
                use_grouping,
                "",
                VerificationNegativeStyle::Minus,
            ),
            Some(VerificationFormatPattern::Currency {
                decimals,
                use_grouping,
                negative_style,
            }) => render_fixed_with_style(
                profile,
                value,
                decimals,
                use_grouping,
                profile.currency_symbol,
                negative_style,
            ),
            Some(VerificationFormatPattern::Percent { decimals }) => {
                let body = render_fixed_with_style(
                    profile,
                    value * 100.0,
                    decimals,
                    false,
                    "",
                    VerificationNegativeStyle::Minus,
                );
                format!("{body}%")
            }
            Some(VerificationFormatPattern::IsoDate) => {
                let Some((year, month, day)) = ymd_from_excel_serial(date_system, value) else {
                    return Err(FormatFailure::InvalidDateSerial);
                };
                format!("{year:04}-{month:02}-{day:02}")
            }
            None => return Err(FormatFailure::UnsupportedCode(code.to_string())),
        };
        Ok(excel_text_from_string(rendered))
    }

    fn render_currency(
        &self,
        profile: &oxfunc_core::locale_format::FormatProfile,
        value: f64,
        decimals: i32,
    ) -> Result<ExcelText, FormatFailure> {
        Ok(excel_text_from_string(render_fixed_common(
            profile,
            value,
            decimals,
            true,
            profile.currency_symbol,
        )))
    }

    fn render_fixed(
        &self,
        profile: &oxfunc_core::locale_format::FormatProfile,
        value: f64,
        decimals: i32,
        no_commas: bool,
    ) -> Result<ExcelText, FormatFailure> {
        Ok(excel_text_from_string(render_fixed_common(
            profile, value, decimals, !no_commas, "",
        )))
    }
}

fn excel_text_from_string(value: String) -> ExcelText {
    ExcelText::from_utf16_code_units(value.encode_utf16().collect())
}

fn normalize_numeric_text(
    profile: &oxfunc_core::locale_format::FormatProfile,
    raw: &str,
) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (negative, body) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };

    let mut normalized = body.replace(profile.thousands_separator, "");
    if profile.decimal_separator != "." {
        normalized = normalized.replace(profile.decimal_separator, ".");
    }

    if normalized.matches('.').count() > 1 {
        return None;
    }

    if negative {
        normalized.insert(0, '-');
    }
    Some(normalized)
}

fn parse_number_with_profile(
    profile: &oxfunc_core::locale_format::FormatProfile,
    raw: &str,
) -> Option<f64> {
    normalize_numeric_text(profile, raw)?.parse().ok()
}

fn parse_iso_ymd(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    (parts.next().is_none()).then_some((year, month, day))
}

fn parse_en_us_slash_date(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('/');
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    let year = parts.next()?.parse().ok()?;
    (parts.next().is_none()).then_some((year, month, day))
}

fn render_fixed_common(
    profile: &oxfunc_core::locale_format::FormatProfile,
    value: f64,
    decimals: i32,
    use_grouping: bool,
    prefix: &str,
) -> String {
    let frac_digits = decimals.max(0) as usize;
    let rounded = if frac_digits == 0 {
        value.round()
    } else {
        let scale = 10f64.powi(frac_digits as i32);
        (value * scale).round() / scale
    };
    let is_negative = rounded.is_sign_negative() && rounded != 0.0;
    let abs_value = rounded.abs();
    let base = format!("{:.*}", frac_digits, abs_value);
    let (int_part, frac_part) = match base.split_once('.') {
        Some((lhs, rhs)) => (lhs.to_string(), Some(rhs.to_string())),
        None => (base, None),
    };
    let grouped = if use_grouping {
        grouped_integer_string(&int_part, profile.thousands_separator)
    } else {
        int_part
    };

    let mut rendered = String::new();
    if is_negative {
        rendered.push('-');
    }
    rendered.push_str(prefix);
    rendered.push_str(&grouped);
    if let Some(frac) = frac_part {
        if frac_digits > 0 {
            rendered.push_str(profile.decimal_separator);
            rendered.push_str(&frac);
        }
    }
    rendered
}

fn render_fixed_with_style(
    profile: &oxfunc_core::locale_format::FormatProfile,
    value: f64,
    decimals: i32,
    use_grouping: bool,
    prefix: &str,
    negative_style: VerificationNegativeStyle,
) -> String {
    let magnitude = render_fixed_common(profile, value.abs(), decimals, use_grouping, prefix);
    if value.is_sign_negative() && value != 0.0 {
        match negative_style {
            VerificationNegativeStyle::Minus => format!("-{magnitude}"),
            VerificationNegativeStyle::Parentheses => format!("({magnitude})"),
        }
    } else {
        magnitude
    }
}

fn classify_verification_format_code(
    profile: &oxfunc_core::locale_format::FormatProfile,
    code: &str,
    value: f64,
) -> Option<VerificationFormatPattern> {
    let section = select_format_section(code, value);
    let negative_style = if section.contains('(') && section.contains(')') {
        VerificationNegativeStyle::Parentheses
    } else {
        VerificationNegativeStyle::Minus
    };
    let canonical = canonicalize_verification_format_section(section);
    let structural = canonical
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();

    match structural {
        "0" => {
            return Some(VerificationFormatPattern::Fixed {
                decimals: 0,
                use_grouping: false,
            });
        }
        "0.00" => {
            return Some(VerificationFormatPattern::Fixed {
                decimals: 2,
                use_grouping: false,
            });
        }
        "0%" => return Some(VerificationFormatPattern::Percent { decimals: 0 }),
        "0.00%" => return Some(VerificationFormatPattern::Percent { decimals: 2 }),
        "yyyy-mm-dd" => return Some(VerificationFormatPattern::IsoDate),
        _ => {}
    }

    if let Some(rest) = structural.strip_prefix(profile.currency_symbol) {
        let (use_grouping, decimals) = parse_numeric_placeholder_pattern(rest)?;
        return Some(VerificationFormatPattern::Currency {
            decimals,
            use_grouping,
            negative_style,
        });
    }

    let (use_grouping, decimals) = parse_numeric_placeholder_pattern(structural)?;
    Some(VerificationFormatPattern::Fixed {
        decimals,
        use_grouping,
    })
}

fn select_format_section(code: &str, value: f64) -> &str {
    let mut sections = code.split(';');
    let first = sections.next().unwrap_or(code);
    let second = sections.next();
    if value.is_sign_negative() && value != 0.0 {
        second.unwrap_or(first)
    } else {
        first
    }
}

fn canonicalize_verification_format_section(section: &str) -> String {
    let mut out = String::new();
    let mut chars = section.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                for inner in chars.by_ref() {
                    if inner == '"' {
                        break;
                    }
                }
            }
            '\\' => {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            }
            '_' | '*' => {
                let _ = chars.next();
            }
            '[' => {
                let mut token = String::new();
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                    token.push(inner);
                }
                if let Some(symbol) = extract_currency_symbol_token(&token) {
                    out.push_str(symbol);
                }
            }
            c if c.is_whitespace() => {}
            other => out.push(other),
        }
    }
    out
}

fn extract_currency_symbol_token(token: &str) -> Option<&str> {
    let currency = token.strip_prefix('$')?;
    let symbol = currency.split('-').next().unwrap_or(currency);
    (!symbol.is_empty()).then_some(symbol)
}

fn parse_numeric_placeholder_pattern(pattern: &str) -> Option<(bool, i32)> {
    if pattern.is_empty()
        || !pattern
            .chars()
            .all(|ch| matches!(ch, '#' | '0' | ',' | '.'))
    {
        return None;
    }
    let (int_part, frac_part) = match pattern.split_once('.') {
        Some((left, right)) => (left, Some(right)),
        None => (pattern, None),
    };
    if !int_part.chars().any(|ch| ch == '0') {
        return None;
    }
    let decimals = frac_part
        .map(|right| right.chars().filter(|ch| matches!(ch, '0' | '#')).count() as i32)
        .unwrap_or(0);
    Some((int_part.contains(','), decimals))
}

fn grouped_integer_string(int_part: &str, sep: &str) -> String {
    if int_part.len() <= 3 || sep.is_empty() {
        return int_part.to_string();
    }
    let mut out = String::new();
    let bytes = int_part.as_bytes();
    let first = int_part.len() % 3;
    let mut index = 0;
    if first > 0 {
        out.push_str(&int_part[..first]);
        index = first;
    }
    while index < bytes.len() {
        if !out.is_empty() {
            out.push_str(sep);
        }
        out.push_str(&int_part[index..index + 3]);
        index += 3;
    }
    out
}

fn build_verification_publication_context(
    extraction: &SpreadsheetXmlCellExtraction,
) -> VerificationPublicationContext {
    VerificationPublicationContext {
        format_profile: Some(extraction.workbook_format_profile_hint.clone()),
        number_format_code: extraction.number_format_code.clone(),
        style_id: extraction.style_id.clone(),
        style_hierarchy: extraction.style_hierarchy.clone(),
        font_color: extraction.font_color.clone(),
        fill_color: extraction.fill_color.clone(),
        conditional_formatting_rules: extraction
            .conditional_formats
            .iter()
            .map(build_verification_conditional_formatting_rule)
            .collect(),
    }
}

fn build_programmatic_verification_publication_context(
    context: &ProgrammaticFormattingContext,
) -> VerificationPublicationContext {
    VerificationPublicationContext {
        format_profile: context.format_profile_id.clone(),
        number_format_code: context.number_format_code.clone(),
        style_id: None,
        style_hierarchy: Vec::new(),
        font_color: None,
        fill_color: None,
        conditional_formatting_rules: Vec::new(),
    }
}

fn effective_verification_publication_context(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Option<VerificationPublicationContext> {
    spreadsheet_xml_extraction
        .map(build_verification_publication_context)
        .or_else(|| {
            case.formatting_context
                .as_ref()
                .map(build_programmatic_verification_publication_context)
        })
}

fn missing_programmatic_display_context_reason(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Option<&'static str> {
    (spreadsheet_xml_extraction.is_none() && case.formatting_context.is_none()).then_some(
        "Display comparison blocked: explicit programmatic formatting context is absent on this case",
    )
}

fn build_verification_conditional_formatting_rule(
    rule: &crate::services::spreadsheet_xml::ConditionalFormatRule,
) -> VerificationConditionalFormattingRule {
    let mut thresholds = Vec::new();
    if let Some(formula) = &rule.formula {
        thresholds.push(formula.clone());
    }
    if let Some(value1) = &rule.value1 {
        thresholds.push(value1.clone());
    }
    if let Some(value2) = &rule.value2 {
        thresholds.push(value2.clone());
    }

    VerificationConditionalFormattingRule {
        target_ranges: vec![rule.range.clone()],
        rule_kind: rule
            .rule_kind
            .clone()
            .unwrap_or_else(|| "expression".to_string())
            .to_ascii_lowercase(),
        operator: rule
            .operator
            .as_ref()
            .map(|value| value.to_ascii_lowercase()),
        thresholds,
        // Spreadsheet-XML-imported rules carry only the bounded
        // payload shape today; the typed payload arrives via the
        // host UI authoring path. The W072 fallback lets OxFml
        // continue to evaluate these rules unchanged.
        typed_rule: None,
        font_color: rule.font_color.clone(),
        fill_color: rule.interior_color.clone(),
        effective_display_text: None,
        applies: None,
        effective_font_color: None,
        effective_fill_color: None,
    }
}

fn build_oxxlplay_scenario_json(
    repo_root: &Path,
    case_dir: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    effective_excel_render_context: &EffectiveExcelRenderContext,
) -> Value {
    let scenario_id = format!("onecalc_verify_{}", sanitize_case_id(&case.case_id));
    let locator = spreadsheet_xml_extraction
        .map(|extraction| extraction.locator.clone())
        .unwrap_or_else(|| "Sheet1!A1".to_string());
    let workbook_kind = if spreadsheet_xml_extraction.is_some() {
        "spreadsheetml-2003-import"
    } else {
        "programmatic-formula"
    };
    let requested_observation_scope =
        effective_requested_observation_scope(case, spreadsheet_xml_extraction);
    let include_effective_display =
        programmatic_effective_display_surface_requested(case, spreadsheet_xml_extraction);
    let mut scenario = json!({
        "scenario_id": scenario_id,
        "replay_class": "capture_surface_basic",
        "retained_root": display_repo_relative(case_dir, repo_root),
        "workbook_ref": "./workbook.xml",
        "workbook_kind": workbook_kind,
        "trigger": "open_then_recalc",
        "observable_surfaces": build_oxxlplay_observable_surfaces(&locator, include_effective_display),
        "requested_observation_scope": requested_observation_scope,
        "source_cell_locator": spreadsheet_xml_extraction.map(|extraction| extraction.locator.clone()),
        "source_workbook_path": spreadsheet_xml_extraction.map(|extraction| extraction.workbook_path.clone()),
        "excel_render_context": effective_excel_render_context
    });
    if spreadsheet_xml_extraction.is_none() {
        scenario["entered_cell_text"] = Value::String(case.entered_cell_text.clone());
    }
    scenario
}

fn effective_requested_observation_scope(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> VerificationObservationScope {
    if let Some(extraction) = spreadsheet_xml_extraction {
        extraction.observation_scope.clone()
    } else if programmatic_display_comparison_enabled(case, spreadsheet_xml_extraction) {
        programmatic_formula_observation_scope_with_display()
    } else {
        programmatic_formula_observation_scope_without_display()
    }
}

const EXECUTION_OUTCOME_VIEW_FAMILY: &str = "execution_outcome";

fn programmatic_formula_observation_scope_with_display() -> VerificationObservationScope {
    VerificationObservationScope {
        oxfml_required_scope: vec![
            "entered_cell_text".to_string(),
            "returned_value_surface".to_string(),
            "format_profile".to_string(),
            "date1904".to_string(),
            "number_format_code".to_string(),
            "effective_display_text".to_string(),
        ],
        oxxlplay_required_surfaces: vec![
            "cell_value".to_string(),
            "effective_display_text".to_string(),
        ],
        oxreplay_required_views: vec![
            EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
            "comparison_value".to_string(),
            "effective_display_text".to_string(),
        ],
    }
}

fn programmatic_formula_observation_scope_without_display() -> VerificationObservationScope {
    VerificationObservationScope {
        oxfml_required_scope: vec![
            "entered_cell_text".to_string(),
            "returned_value_surface".to_string(),
        ],
        oxxlplay_required_surfaces: vec!["cell_value".to_string()],
        oxreplay_required_views: vec![
            EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
            "comparison_value".to_string(),
        ],
    }
}

fn programmatic_display_contract_is_explicit(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> bool {
    spreadsheet_xml_extraction.is_some()
        || case
            .formatting_context
            .as_ref()
            .and_then(|context| context.number_format_code.as_deref())
            .map(str::trim)
            .is_some_and(|code| !code.is_empty())
}

fn programmatic_display_comparison_enabled(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> bool {
    programmatic_display_contract_is_explicit(case, spreadsheet_xml_extraction)
}

fn programmatic_effective_display_surface_requested(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> bool {
    spreadsheet_xml_extraction.is_none()
        && programmatic_display_contract_is_explicit(case, spreadsheet_xml_extraction)
}

fn build_default_excel_render_context(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> ProgrammaticExcelRenderContext {
    ProgrammaticExcelRenderContext {
        render_locale_pinned: false,
        render_locale_source: Some("observation_machine_default".to_string()),
        render_locale_recorded: false,
        trusted: false,
        requested_format_profile_id: if spreadsheet_xml_extraction.is_none() {
            case.formatting_context
                .as_ref()
                .and_then(|context| context.format_profile_id.clone())
        } else {
            None
        },
        decimal_separator: None,
        thousands_separator: None,
        list_separator: None,
        date_separator: None,
        time_separator: None,
        note: Some(if spreadsheet_xml_extraction.is_some() {
            "SpreadsheetML-backed verification preserves workbook formatting/style evidence but DnaOneCalc does not record any separate Excel-side render-locale pin for the observation host.".to_string()
        } else {
            "Programmatic-formula verification injects locale context for OxFml but the generated workbook/scenario does not carry any Excel-side locale pin or locale-capture field; Excel text rendering reflects the observation machine environment.".to_string()
        }),
    }
}

fn resolve_effective_excel_render_context(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    render_contexts: &BTreeMap<String, ProgrammaticExcelRenderContext>,
) -> Result<EffectiveExcelRenderContext, String> {
    if let Some(render_context) = &case.excel_render_context {
        return Ok(EffectiveExcelRenderContext {
            context: render_context.clone(),
            provenance: EffectiveExcelRenderContextProvenance {
                kind: "inline".to_string(),
                render_context_ref: None,
            },
        });
    }

    if let Some(render_context_ref) = case.render_context_ref.as_deref() {
        let context = render_contexts.get(render_context_ref).ok_or_else(|| {
            format!(
                "verification case `{}` referenced unknown render context `{render_context_ref}`",
                case.case_id
            )
        })?;
        return Ok(EffectiveExcelRenderContext {
            context: context.clone(),
            provenance: EffectiveExcelRenderContextProvenance {
                kind: "shared_ref".to_string(),
                render_context_ref: Some(render_context_ref.to_string()),
            },
        });
    }

    Ok(EffectiveExcelRenderContext {
        context: build_default_excel_render_context(case, spreadsheet_xml_extraction),
        provenance: EffectiveExcelRenderContextProvenance {
            kind: "fallback".to_string(),
            render_context_ref: None,
        },
    })
}

fn annotate_excel_observation_render_context(
    effective_excel_render_context: &EffectiveExcelRenderContext,
    summary: &mut ExcelObservationSummary,
) {
    summary.render_locale_pinned =
        Some(effective_excel_render_context.context.render_locale_pinned);
    summary.render_locale_source = effective_excel_render_context
        .context
        .render_locale_source
        .clone();
    summary.render_locale_note = effective_excel_render_context.context.note.clone();
}

fn import_effective_excel_render_context_from_oxxlplay_output(
    case: &ProgrammaticFormulaCase,
    resolved_output_dir: &Path,
) -> Result<Option<EffectiveExcelRenderContext>, String> {
    let render_context_path = resolved_output_dir.join("render-context.json");
    if !render_context_path.is_file() {
        return Ok(None);
    }

    let artifact = read_json_file(&render_context_path)?;
    if artifact
        .get("render_context_schema")
        .and_then(Value::as_str)
        != Some("oxxlplay.excel_render_context.v1")
    {
        return Ok(None);
    }

    let render_formatting = match artifact.get("render_formatting").and_then(Value::as_object) {
        Some(value) => value,
        None => return Ok(None),
    };
    if render_formatting
        .get("capture_status")
        .and_then(Value::as_str)
        != Some("captured")
    {
        return Ok(None);
    }

    let use_system_separators = render_formatting
        .get("use_system_separators")
        .and_then(Value::as_bool);
    let decimal_separator = render_formatting
        .get("decimal_separator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let thousands_separator = render_formatting
        .get("thousands_separator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let list_separator = render_formatting
        .get("list_separator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let date_separator = render_formatting
        .get("date_separator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let time_separator = render_formatting
        .get("time_separator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let trusted = decimal_separator.is_some() && thousands_separator.is_some();
    if !trusted {
        return Ok(None);
    }

    Ok(Some(EffectiveExcelRenderContext {
        context: ProgrammaticExcelRenderContext {
            render_locale_pinned: !use_system_separators.unwrap_or(true),
            render_locale_source: Some("oxxlplay_render_context_capture".to_string()),
            render_locale_recorded: true,
            trusted: true,
            requested_format_profile_id: case
                .formatting_context
                .as_ref()
                .and_then(|context| context.format_profile_id.clone()),
            decimal_separator,
            thousands_separator,
            list_separator,
            date_separator,
            time_separator,
            note: Some(format!(
                "Imported trusted Excel render context from OxXlPlay render-context.json (use_system_separators={})",
                use_system_separators
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )),
        },
        provenance: EffectiveExcelRenderContextProvenance {
            kind: "oxxlplay_capture_artifact".to_string(),
            render_context_ref: None,
        },
    }))
}

fn persist_effective_excel_render_context(
    case_dir: &Path,
    scenario_path: &Path,
    effective_excel_render_context: &EffectiveExcelRenderContext,
) -> Result<(), String> {
    let excel_render_context =
        serde_json::to_value(effective_excel_render_context).map_err(|error| {
            format!(
                "failed to serialize effective Excel render context for `{}`: {error}",
                case_dir.display()
            )
        })?;

    let case_input_path = case_dir.join("case-input.json");
    if case_input_path.is_file() {
        let mut case_input = read_json_file(&case_input_path)?;
        case_input["excel_render_context"] = excel_render_context.clone();
        write_json_file(&case_input_path, &case_input)?;
    }

    if scenario_path.is_file() {
        let mut scenario = read_json_file(scenario_path)?;
        scenario["excel_render_context"] = excel_render_context;
        write_json_file(scenario_path, &scenario)?;
    }

    Ok(())
}

fn persist_oxfml_case_artifacts(
    case_dir: &Path,
    projection_path: &Path,
    oxfml_result: &OxfmlCaseArtifacts,
) -> Result<(), String> {
    write_json_file(
        case_dir.join("oxfml-runtime-summary.json"),
        &oxfml_result.summary,
    )?;
    write_json_file(projection_path, &oxfml_result.replay_projection_json)?;
    Ok(())
}

fn persist_oxfml_execution_context(
    case_dir: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    effective_excel_render_context: &EffectiveExcelRenderContext,
    execution_phase: &str,
) -> Result<(), String> {
    let locale_context = verification_locale_context(
        case,
        spreadsheet_xml_extraction,
        Some(effective_excel_render_context),
    );
    let trusted_separator_context =
        trusted_excel_separator_context(Some(effective_excel_render_context));
    write_json_file(
        case_dir.join("oxfml-execution-context.json"),
        &json!({
            "execution_phase": execution_phase,
            "effective_excel_render_context": effective_excel_render_context,
            "locale_query_bundle": {
                "profile_id": format!("{:?}", locale_context.profile.id),
                "date_system": format!("{:?}", locale_context.date_system),
                "decimal_separator": locale_context.profile.decimal_separator,
                "thousands_separator": locale_context.profile.thousands_separator,
                "date_separator": locale_context.profile.date_separator,
                "time_separator": locale_context.profile.time_separator,
            },
            "trusted_excel_separator_context": trusted_separator_context.map(|context| {
                json!({
                    "requested_format_profile_id": context.requested_format_profile_id,
                    "decimal_separator": context.decimal_separator,
                    "thousands_separator": context.thousands_separator,
                    "list_separator": context.list_separator,
                    "date_separator": context.date_separator,
                    "time_separator": context.time_separator,
                })
            })
        }),
    )
}

fn import_captured_render_context_and_refresh_oxfml_if_needed<F>(
    prepared: &mut PreparedVerificationCase,
    resolved_output_dir: &Path,
    refresh_oxfml: F,
) -> Result<bool, String>
where
    F: FnOnce(
        &ProgrammaticFormulaCase,
        Option<&SpreadsheetXmlCellExtraction>,
        Option<&EffectiveExcelRenderContext>,
    ) -> Result<OxfmlCaseArtifacts, String>,
{
    if prepared.effective_excel_render_context.context.trusted {
        return Ok(false);
    }

    let Some(imported_context) = import_effective_excel_render_context_from_oxxlplay_output(
        &prepared.effective_case,
        resolved_output_dir,
    )?
    else {
        return Ok(false);
    };

    prepared.effective_excel_render_context = imported_context;
    persist_effective_excel_render_context(
        &prepared.case_dir,
        &prepared.scenario_path,
        &prepared.effective_excel_render_context,
    )?;
    prepared.oxfml_result = refresh_oxfml(
        &prepared.effective_case,
        prepared.spreadsheet_xml_extraction.as_ref(),
        Some(&prepared.effective_excel_render_context),
    )?;
    persist_oxfml_case_artifacts(
        &prepared.case_dir,
        &prepared.projection_path,
        &prepared.oxfml_result,
    )?;
    persist_oxfml_execution_context(
        &prepared.case_dir,
        &prepared.effective_case,
        prepared.spreadsheet_xml_extraction.as_ref(),
        &prepared.effective_excel_render_context,
        "post_capture_trusted_refresh",
    )?;
    Ok(true)
}

fn build_oxxlplay_observable_surfaces(
    locator: &str,
    include_effective_display: bool,
) -> Vec<Value> {
    let mut surfaces = vec![
        json!({
            "surface_id": "sheet1_a1_value",
            "surface_kind": "cell_value",
            "locator": locator,
            "required": true
        }),
        json!({
            "surface_id": "sheet1_a1_formula",
            "surface_kind": "formula_text",
            "locator": locator,
            "required": false
        }),
    ];
    if include_effective_display {
        surfaces.push(json!({
            "surface_id": "sheet1_a1_display",
            "surface_kind": "effective_display_text",
            "locator": locator,
            "required": true
        }));
    }
    surfaces
}

fn build_observation_gap_report(
    observation_scope: &SpreadsheetXmlCellExtraction,
) -> VerificationObservationGapReport {
    let oxxlplay_supported_surfaces = vec![
        "cell_value".to_string(),
        "formula_text".to_string(),
        "effective_display_text".to_string(),
        "number_format_code".to_string(),
        "style_id".to_string(),
        "font_color".to_string(),
        "fill_color".to_string(),
        "conditional_formatting_rules".to_string(),
        "conditional_formatting_effective_style".to_string(),
    ];
    let oxxlplay_missing_surfaces = observation_scope
        .observation_scope
        .oxxlplay_required_surfaces
        .iter()
        .filter(|surface| {
            !oxxlplay_supported_surfaces
                .iter()
                .any(|supported| supported == *surface)
        })
        .cloned()
        .collect::<Vec<_>>();
    let oxreplay_current_bundle_views = vec![
        EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
        "comparison_value".to_string(),
        "effective_display_text".to_string(),
        "formatting_view".to_string(),
        "conditional_formatting_view".to_string(),
        "replay_normalized_events".to_string(),
    ];
    let oxreplay_missing_views = observation_scope
        .observation_scope
        .oxreplay_required_views
        .iter()
        .filter(|view| {
            !oxreplay_current_bundle_views
                .iter()
                .any(|supported| supported == *view)
        })
        .cloned()
        .collect::<Vec<_>>();

    VerificationObservationGapReport {
        oxfml_scope_required: observation_scope
            .observation_scope
            .oxfml_required_scope
            .clone(),
        oxxlplay_supported_surfaces,
        oxxlplay_missing_surfaces,
        oxreplay_required_views: observation_scope
            .observation_scope
            .oxreplay_required_views
            .clone(),
        oxreplay_current_bundle_views,
        oxreplay_missing_views,
    }
}

fn parse_oxreplay_mismatch_records(diff_report: &Value) -> Vec<OxReplayMismatchRecord> {
    diff_report
        .get("mismatches")
        .and_then(Value::as_array)
        .map(|mismatches| {
            mismatches
                .iter()
                .filter_map(parse_oxreplay_mismatch_record)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_oxreplay_mismatch_record(value: &Value) -> Option<OxReplayMismatchRecord> {
    let mismatch_kind = value
        .get("mismatch_kind")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)?;

    Some(OxReplayMismatchRecord {
        mismatch_kind,
        severity: value
            .get("severity")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        view_family: value
            .get("view_family")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        left_value_repr: json_value_to_repr(value.get("left_value")),
        right_value_repr: json_value_to_repr(value.get("right_value")),
        detail: value
            .get("detail")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

fn parse_oxreplay_explain_records(
    explain_stdout: &str,
) -> Result<Vec<OxReplayExplainRecord>, String> {
    if explain_stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let explain_report = parse_json_text(explain_stdout, "OxReplay explain stdout")?;
    Ok(explain_report
        .get("records")
        .and_then(Value::as_array)
        .map(|records| {
            records
                .iter()
                .filter_map(|record| {
                    let mismatch_kind = record
                        .get("mismatch_kind")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)?;
                    Some(OxReplayExplainRecord {
                        query_id: record
                            .get("query_id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        summary: record
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        mismatch_kind,
                        severity: record
                            .get("severity")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        view_family: record
                            .get("view_family")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        left_value_repr: json_value_to_repr(record.get("left_value")),
                        right_value_repr: json_value_to_repr(record.get("right_value")),
                        detail: record
                            .get("detail")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn json_value_to_repr(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

pub fn replay_display_comparison_summary(
    replay_mismatch_records: &[OxReplayMismatchRecord],
    fallback_left: Option<&str>,
    fallback_right: Option<&str>,
) -> Option<String> {
    if let Some(display_record) = replay_mismatch_records
        .iter()
        .find(|record| record.view_family.as_deref() == Some("effective_display_text"))
    {
        let left = display_record
            .left_value_repr
            .clone()
            .or_else(|| fallback_left.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        let right = display_record
            .right_value_repr
            .clone()
            .or_else(|| fallback_right.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        return Some(format!(
            "Display divergence (effective_display_text): OxFml {left} vs Excel {right}"
        ));
    }

    if let Some(value_record) = replay_mismatch_records.iter().find(|record| {
        record.view_family.as_deref() == Some("comparison_value")
            || record.mismatch_kind == "comparison_value"
    }) {
        let left = value_record
            .left_value_repr
            .clone()
            .or_else(|| fallback_left.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        let right = value_record
            .right_value_repr
            .clone()
            .or_else(|| fallback_right.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        return Some(format!(
            "Comparison value divergence: OxFml {left} vs Excel {right}"
        ));
    }

    match (fallback_left, fallback_right) {
        (Some(left), Some(right)) if left != right => {
            Some(format!("Display divergence: OxFml {left} vs Excel {right}"))
        }
        _ => None,
    }
}

pub fn value_comparison_summary(
    value_match: Option<bool>,
    left_value: Option<&Value>,
    right_value: Option<&Value>,
) -> Option<String> {
    if value_match != Some(false) {
        return None;
    }

    Some(format!(
        "Value divergence: OxFml {} vs Excel {}",
        left_value
            .map(render_comparison_value)
            .unwrap_or_else(|| "<unavailable>".to_string()),
        right_value
            .map(render_comparison_value)
            .unwrap_or_else(|| "<unavailable>".to_string())
    ))
}

pub fn display_comparison_summary(
    display_match: Option<bool>,
    left_display: Option<&str>,
    right_display: Option<&str>,
) -> Option<String> {
    if display_match != Some(false) {
        return None;
    }

    Some(format!(
        "Display divergence: OxFml {} vs Excel {}",
        left_display.unwrap_or("<unavailable>"),
        right_display.unwrap_or("<unavailable>")
    ))
}

pub fn replay_projection_coverage_gap_summaries(
    replay_mismatch_records: &[OxReplayMismatchRecord],
) -> Vec<String> {
    replay_mismatch_records
        .iter()
        .filter(|record| record.mismatch_kind == "projection_coverage_gap")
        .map(|record| match (record.view_family.as_deref(), record.detail.as_deref()) {
            (Some(view_family), Some(detail)) => {
                format!("Projection coverage gap ({view_family}): {detail}")
            }
            (Some(view_family), None) => {
                format!(
                    "Projection coverage gap ({view_family}): comparison family is missing on one side."
                )
            }
            (None, Some(detail)) => format!("Projection coverage gap: {detail}"),
            (None, None) => "Projection coverage gap: comparison family is missing on one side."
                .to_string(),
        })
        .collect()
}

fn summarize_excel_capture(capture_path: PathBuf) -> Result<ExcelObservationSummary, String> {
    let capture_json = read_json_file(&capture_path)?;
    let surfaces = capture_json
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "capture file `{}` did not contain a `surfaces` array",
                capture_path.display()
            )
        })?;

    let mut comparison_value = None;
    let mut observed_value_repr = None;
    let mut effective_display_text = None;
    let mut observed_formula_repr = None;
    let mut capture_status = "captured".to_string();

    for surface in surfaces {
        let surface_kind = surface
            .get("surface")
            .and_then(|value| value.get("surface_kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = surface
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let value_repr = surface
            .get("value_repr")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let comparison_surface_value = observed_surface_comparison_value(surface);

        match surface_kind {
            "cell_value" => {
                comparison_value = comparison_surface_value;
                observed_value_repr = comparison_value.as_ref().map(render_comparison_value);
                if status != "direct" && capture_status == "captured" {
                    capture_status = status.to_string();
                }
            }
            "effective_display_text" => {
                effective_display_text = value_repr;
                if status != "direct" && capture_status == "captured" {
                    capture_status = status.to_string();
                }
            }
            "formula_text" => {
                observed_formula_repr = value_repr;
            }
            _ => {}
        }
    }

    Ok(ExcelObservationSummary {
        comparison_value,
        observed_value_repr,
        effective_display_text,
        observed_formula_repr,
        capture_status,
        render_locale_pinned: None,
        render_locale_source: None,
        render_locale_note: None,
    })
}

fn preferred_excel_display_repr(summary: &ExcelObservationSummary) -> Option<&str> {
    summary.effective_display_text.as_deref()
}

fn locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
    _case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
    effective_excel_render_context: &EffectiveExcelRenderContext,
    oxfml_projection: &Value,
    oxfml_value: Option<&Value>,
) -> bool {
    spreadsheet_xml_extraction.is_none()
        && excel_render_context_is_untrusted_or_unpinned(effective_excel_render_context)
        && oxfml_value.is_some_and(comparison_value_is_text)
        && projection_marks_locale_sensitive_semantic_text_dependency(oxfml_projection)
}

fn excel_render_context_is_untrusted_or_unpinned(
    effective_excel_render_context: &EffectiveExcelRenderContext,
) -> bool {
    !effective_excel_render_context.context.trusted
}

fn comparison_value_is_text(value: &Value) -> bool {
    value.get("kind").and_then(Value::as_str) == Some("text")
}

fn projection_marks_locale_sensitive_semantic_text_dependency(projection: &Value) -> bool {
    projection
        .get("comparison_views")
        .and_then(Value::as_array)
        .and_then(|views| {
            views.iter().find_map(|view| {
                (view.get("view_family").and_then(Value::as_str) == Some("formatting_view"))
                    .then(|| view.get("value"))
                    .flatten()
            })
        })
        .and_then(|value| value.get("format_dependency_facts"))
        .and_then(Value::as_array)
        .is_some_and(|facts| {
            facts.iter().any(|fact| {
                fact.get("dependency_class").and_then(Value::as_str) == Some("semantic_formatting")
                    && fact.get("dependency_token").and_then(Value::as_str)
                        == Some("locale_format_context")
            })
        })
}

fn serialize_replay_projection(
    projection: &oxfml_core::consumer::replay::ReplayProjectionResult,
    include_effective_display: bool,
) -> Value {
    json!({
        "source_artifact_family": projection.source_artifact_family,
        "source_schema_id": projection.source_schema_id,
        "source_fixture_family": projection.source_fixture_family,
        "source_case_id": projection.source_case_id,
        "source_case_ids": projection.source_case_ids,
        "shared_scenario_alias": projection.shared_scenario_alias,
        "formula_stable_id": projection.formula_stable_id,
        "session_id": projection.session_id,
        "library_context_snapshot_ref": projection.library_context_snapshot_ref.as_ref().map(|value| {
            json!({
                "snapshot_id": value.snapshot_id,
                "snapshot_version": value.snapshot_version
            })
        }),
        "typed_query_bundle_spec": projection.typed_query_bundle_spec.as_ref().map(|value| format!("{value:?}")),
        "registry_pin": projection.registry_pin,
        "witness_id": projection.witness_id,
        "witness_lifecycle_state": projection.witness_lifecycle_state,
        "retention_policy_id": projection.retention_policy_id,
        "source_bundle_ref": projection.source_bundle_ref,
        "reduction_manifest_ref": projection.reduction_manifest_ref,
        "phase": projection.phase,
        "candidate_result_id": projection.candidate_result_id,
        "commit_decision_kind": projection.commit_decision_kind,
        "trace_event_kinds": projection.trace_event_kinds,
        "semantic_kernel_metadata_version": projection.semantic_kernel_metadata_version,
        "arg_admission_metadata_version": projection.arg_admission_metadata_version,
        "producer_capability_set_keys": projection.producer_capability_set_keys,
        "exercised_capability_keys": projection.exercised_capability_keys,
        "comparison_views": serialize_comparison_views(
            projection.comparison_views.as_deref().unwrap_or(&[]),
            projection.verification_publication_surface.as_ref(),
            include_effective_display,
        ),
        "verification_publication_surface": projection
            .verification_publication_surface
            .as_ref()
            .map(|surface| {
                serialize_verification_publication_surface(surface, include_effective_display)
            }),
    })
}

fn serialize_comparison_views(
    comparison_views: &[oxfml_core::consumer::replay::ReplayComparisonView],
    verification_publication_surface: Option<&VerificationPublicationSurface>,
    include_effective_display: bool,
) -> Value {
    let mut serialized = comparison_views
        .iter()
        .filter(|view| include_effective_display || view.view_family != "effective_display_text")
        .map(|view| {
            let value = if view.view_family == "comparison_value" {
                normalize_comparison_value(&view.value)
            } else {
                view.value.clone()
            };
            json!({
                "view_family": view.view_family,
                "value": value
            })
        })
        .collect::<Vec<_>>();

    let has_effective_display_text = serialized.iter().any(|view| {
        view.get("view_family").and_then(Value::as_str) == Some("effective_display_text")
    });
    if include_effective_display && !has_effective_display_text {
        if let Some(effective_display_text) = verification_publication_surface
            .map(|surface| surface.effective_display_text.clone())
            .filter(|value| !value.is_empty())
        {
            serialized.push(json!({
                "view_family": "effective_display_text",
                "value": effective_display_text
            }));
        }
    }

    Value::Array(serialized)
}

fn serialize_verification_publication_surface(
    surface: &VerificationPublicationSurface,
    include_effective_display: bool,
) -> Value {
    json!({
        "entered_cell_text": surface.entered_cell_text,
        "published_value": {
            "worksheet_value_class": format!("{:?}", surface.published_value_class),
            "payload": format!("{:?}", surface.published_value),
        },
        "effective_display_text": if include_effective_display {
            Some(surface.effective_display_text.clone())
        } else {
            None
        },
        "format_profile": if include_effective_display {
            surface.format_profile.clone()
        } else {
            None
        },
        "locale_format_context": if include_effective_display {
            surface
                .locale_format_context
                .as_ref()
                .map(serialize_locale_format_context_surface)
        } else {
            None
        },
        "date1904": if include_effective_display {
            Some(surface.date1904)
        } else {
            None
        },
        "number_format_code": if include_effective_display {
            surface.number_format_code.clone()
        } else {
            None
        },
        "style_id": surface.style_id,
        "style_hierarchy": surface.style_hierarchy,
        "format_dependency_facts": surface.format_dependency_facts.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
        "format_delta": surface.format_delta.as_ref().map(|value| format!("{value:?}")),
        "display_delta": surface.display_delta.as_ref().map(|value| format!("{value:?}")),
        "returned_value_surface": format!("{:?}", surface.returned_value_surface),
        "presentation_hint": surface.presentation_hint.as_ref().map(|value| format!("{value:?}")),
        "font_color": surface.font_color,
        "fill_color": surface.fill_color,
        "effective_font_color": surface.effective_font_color,
        "effective_fill_color": surface.effective_fill_color,
        "conditional_formatting_rules": surface.conditional_formatting_rules.iter().map(serialize_verification_conditional_formatting_rule).collect::<Vec<_>>(),
        "conditional_formatting_target_ranges": surface.conditional_formatting_target_ranges,
        "conditional_formatting_rule_kind": surface.conditional_formatting_rule_kind,
        "conditional_formatting_operator": surface.conditional_formatting_operator,
        "conditional_formatting_thresholds": surface.conditional_formatting_thresholds,
        "conditional_formatting_applies": surface.conditional_formatting_applies,
        "conditional_formatting_effective_font_color": surface.conditional_formatting_effective_font_color,
        "conditional_formatting_effective_fill_color": surface.conditional_formatting_effective_fill_color,
        "conditional_formatting_effective_display": surface.conditional_formatting_effective_display,
    })
}

fn serialize_locale_format_context_surface(surface: &LocaleFormatContextSurface) -> Value {
    json!({
        "locale_profile_id": surface.locale_profile_id,
        "date_system": surface.date_system,
        "decimal_separator": surface.decimal_separator,
        "thousands_separator": surface.thousands_separator,
        "currency_symbol": surface.currency_symbol,
        "date_separator": surface.date_separator,
        "time_separator": surface.time_separator,
    })
}

fn serialize_verification_conditional_formatting_rule(
    rule: &VerificationConditionalFormattingRule,
) -> Value {
    json!({
        "target_ranges": rule.target_ranges,
        "rule_kind": rule.rule_kind,
        "operator": rule.operator,
        "thresholds": rule.thresholds,
        "font_color": rule.font_color,
        "fill_color": rule.fill_color,
        "effective_display_text": rule.effective_display_text,
        "applies": rule.applies,
        "effective_font_color": rule.effective_font_color,
        "effective_fill_color": rule.effective_fill_color,
    })
}

fn execution_outcome_comparison_summary(
    replay_mismatch_records: &[OxReplayMismatchRecord],
) -> Option<String> {
    replay_mismatch_records
        .iter()
        .find(|record| {
            record.view_family.as_deref() == Some(EXECUTION_OUTCOME_VIEW_FAMILY)
                || record.mismatch_kind == EXECUTION_OUTCOME_VIEW_FAMILY
        })
        .map(|record| {
            let left = record
                .left_value_repr
                .clone()
                .unwrap_or_else(|| "<unavailable>".to_string());
            let right = record
                .right_value_repr
                .clone()
                .unwrap_or_else(|| "<unavailable>".to_string());
            format!("Execution outcome divergence: OxFml {left} vs Excel {right}")
        })
}

fn build_discrepancy_summary(
    comparison_status: ProgrammaticComparisonStatus,
    value_match: Option<bool>,
    display_match: Option<bool>,
    replay_mismatch_records: &[OxReplayMismatchRecord],
    oxfml_summary: &OxfmlVerificationSummary,
    excel_summary: Option<&ExcelObservationSummary>,
) -> Option<String> {
    match comparison_status {
        ProgrammaticComparisonStatus::Matched => None,
        ProgrammaticComparisonStatus::Blocked => {
            Some(oxfml_summary.blocked_reason.clone().unwrap_or_else(|| {
                "comparison blocked before both value and display axes completed".to_string()
            }))
        }
        ProgrammaticComparisonStatus::Mismatched => {
            let value_summary = value_comparison_summary(
                value_match,
                oxfml_summary.comparison_value.as_ref(),
                excel_summary.and_then(|summary| summary.comparison_value.as_ref()),
            );
            let display_summary = display_comparison_summary(
                display_match,
                oxfml_summary.effective_display_summary.as_deref(),
                excel_summary.and_then(preferred_excel_display_repr),
            );
            let execution_outcome_summary =
                execution_outcome_comparison_summary(replay_mismatch_records);
            let projection_gap_summary =
                replay_projection_coverage_gap_summaries(replay_mismatch_records);

            if value_summary.is_some()
                || display_summary.is_some()
                || execution_outcome_summary.is_some()
                || !projection_gap_summary.is_empty()
            {
                let mut parts = Vec::new();
                if let Some(value_summary) = value_summary {
                    parts.push(value_summary);
                }
                if let Some(display_summary) = display_summary {
                    parts.push(display_summary);
                }
                if let Some(execution_outcome_summary) = execution_outcome_summary {
                    parts.push(execution_outcome_summary);
                }
                if !projection_gap_summary.is_empty() {
                    parts.push(projection_gap_summary.join(" | "));
                }
                return Some(parts.join(" | "));
            }

            Some("comparison diverged".to_string())
        }
    }
}

fn projection_comparison_value(projection: &Value, family: &str) -> Option<Value> {
    projection
        .get("comparison_views")
        .and_then(Value::as_array)
        .and_then(|views| {
            views.iter().find_map(|view| {
                let current_family = view.get("view_family").and_then(Value::as_str)?;
                (current_family == family)
                    .then(|| view.get("value").map(normalize_comparison_value))
                    .flatten()
            })
        })
}

fn observed_surface_comparison_value(surface: &Value) -> Option<Value> {
    surface
        .get("comparison_value")
        .map(normalize_comparison_value)
}

fn materialize_compare_ready_projection(
    projection_path: &Path,
    output_path: impl AsRef<Path>,
    required_views: &[String],
    execution_outcome: &Value,
) -> Result<PathBuf, String> {
    let mut projection = read_json_file(projection_path)?;
    normalize_replay_comparison_views(&mut projection);
    upsert_comparison_view(
        &mut projection,
        EXECUTION_OUTCOME_VIEW_FAMILY,
        execution_outcome.clone(),
    )?;
    filter_comparison_views(&mut projection, required_views)?;

    let output_path = output_path.as_ref();
    write_json_file(output_path, &projection)?;
    Ok(output_path.to_path_buf())
}

fn materialize_compare_ready_normalized_replay(
    normalized_replay_path: &Path,
    output_path: impl AsRef<Path>,
    required_views: &[String],
    execution_outcome: &Value,
) -> Result<PathBuf, String> {
    let mut normalized_replay = read_json_file(normalized_replay_path)?;
    normalize_replay_comparison_views(&mut normalized_replay);
    upsert_comparison_view(
        &mut normalized_replay,
        EXECUTION_OUTCOME_VIEW_FAMILY,
        execution_outcome.clone(),
    )?;
    filter_comparison_views(&mut normalized_replay, required_views)?;

    let output_path = output_path.as_ref();
    write_json_file(output_path, &normalized_replay)?;
    Ok(output_path.to_path_buf())
}

fn materialize_synthetic_compare_ready_replay(
    output_path: impl AsRef<Path>,
    case_id: &str,
    execution_outcome: &Value,
) -> Result<PathBuf, String> {
    let replay = json!({
        "scenario_id": format!("onecalc_verify_{}", sanitize_case_id(case_id)),
        "source_case_id": case_id,
        "lane_id": "synthetic-host-observation",
        "events": [],
        "registry_refs": [],
        "comparison_views": [
            {
                "view_family": EXECUTION_OUTCOME_VIEW_FAMILY,
                "value": execution_outcome
            }
        ]
    });
    let output_path = output_path.as_ref();
    write_json_file(output_path, &replay)?;
    Ok(output_path.to_path_buf())
}

fn materialize_synthetic_compare_ready_projection(
    output_path: impl AsRef<Path>,
    case: &ProgrammaticFormulaCase,
    execution_outcome: &Value,
) -> Result<PathBuf, String> {
    let projection = json!({
        "candidate_result_id": "candidate:synthetic-host-normalized",
        "commit_decision_kind": "rejected",
        "comparison_views": [
            {
                "view_family": EXECUTION_OUTCOME_VIEW_FAMILY,
                "value": execution_outcome
            }
        ],
        "formula_stable_id": case.case_id,
        "library_context_snapshot_ref": Value::Null,
        "phase": "CommittedOrRejected",
        "reduction_manifest_ref": Value::Null,
        "registry_pin": Value::Null,
        "retention_policy_id": Value::Null,
        "session_id": "session:synthetic-host-normalized",
        "shared_scenario_alias": format!("onecalc_verify_{}", sanitize_case_id(&case.case_id)),
        "source_artifact_family": "runtime_formula_result",
        "source_bundle_ref": Value::Null,
        "source_case_id": case.case_id,
        "source_case_ids": [],
        "source_fixture_family": Value::Null,
        "source_schema_id": Value::Null,
        "semantic_kernel_metadata_version": Value::Null,
        "arg_admission_metadata_version": Value::Null,
        "producer_capability_set_keys": [],
        "exercised_capability_keys": [],
        "trace_event_kinds": ["AcceptedCandidateResultBuilt", "CommitRejected", "RejectIssued"],
        "typed_query_bundle_spec": Value::Null,
        "verification_publication_surface": {
            "conditional_formatting_applies": [],
            "conditional_formatting_effective_display": [],
            "conditional_formatting_effective_fill_color": [],
            "conditional_formatting_effective_font_color": [],
            "conditional_formatting_operator": [],
            "conditional_formatting_rule_kind": [],
            "conditional_formatting_rules": [],
            "conditional_formatting_target_ranges": [],
            "conditional_formatting_thresholds": [],
            "date1904": Value::Null,
            "display_delta": Value::Null,
            "effective_display_text": Value::Null,
            "effective_fill_color": Value::Null,
            "effective_font_color": Value::Null,
            "entered_cell_text": case.entered_cell_text,
            "fill_color": Value::Null,
            "font_color": Value::Null,
            "format_delta": Value::Null,
            "format_dependency_facts": [],
            "format_profile": Value::Null,
            "locale_format_context": Value::Null,
            "number_format_code": Value::Null,
            "presentation_hint": Value::Null,
            "published_value": Value::Null,
            "returned_value_surface": Value::Null,
            "style_hierarchy": [],
            "style_id": Value::Null
        },
        "witness_id": Value::Null,
        "witness_lifecycle_state": Value::Null
    });
    let output_path = output_path.as_ref();
    write_json_file(output_path, &projection)?;
    Ok(output_path.to_path_buf())
}

fn normalize_replay_comparison_views(replay: &mut Value) {
    let Some(comparison_views) = replay
        .get_mut("comparison_views")
        .and_then(Value::as_array_mut)
    else {
        return;
    };

    for view in comparison_views {
        if view.get("view_family").and_then(Value::as_str) != Some("comparison_value") {
            continue;
        }
        if let Some(value) = view.get_mut("value") {
            *value = normalize_compare_ready_comparison_value(value);
        }
    }
}

fn normalize_compare_ready_comparison_value(value: &Value) -> Value {
    let mut current = value;
    loop {
        let Some(object) = current.as_object() else {
            break;
        };
        if object.get("boundary").and_then(Value::as_str) == Some("published_formula_result")
            && object.get("value").is_some()
        {
            current = object.get("value").expect("checked is_some");
            continue;
        }
        break;
    }

    let Some(object) = current.as_object() else {
        return current.clone();
    };
    let Some(kind) = object
        .get("value_kind")
        .or_else(|| object.get("kind"))
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
    else {
        return current.clone();
    };

    let normalized = match kind.as_str() {
        "number" => extract_number_comparison_value_lexeme(object)
            .map(|number| json!({ "kind": "number", "number": number }))
            .unwrap_or_else(|| normalize_comparison_value(current)),
        "array" => normalize_compare_ready_array_comparison_value(object)
            .unwrap_or_else(|| normalize_comparison_value(current)),
        _ => normalize_comparison_value(current),
    };
    canonicalize_array_comparison_value_for_compare_ready(&normalized)
}

fn canonicalize_array_comparison_value_for_compare_ready(value: &Value) -> Value {
    let Some(object) = value.as_object() else {
        return value.clone();
    };
    if object.get("kind").and_then(Value::as_str) != Some("array") {
        return value.clone();
    }
    let Some(shape) = object.get("shape").and_then(Value::as_object) else {
        return value.clone();
    };
    let Some(rows) = shape.get("rows").and_then(Value::as_u64) else {
        return value.clone();
    };
    let Some(cols) = shape.get("cols").and_then(Value::as_u64) else {
        return value.clone();
    };
    let Some(cells) = object.get("cells").and_then(Value::as_array) else {
        return value.clone();
    };

    let rows = rows as usize;
    let cols = cols as usize;
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) != cells.len() {
        return value.clone();
    }
    if cells.iter().all(Value::is_array) {
        return value.clone();
    }

    let mut matrix_rows = Vec::with_capacity(rows);
    for row_index in 0..rows {
        let start = row_index * cols;
        let end = start + cols;
        matrix_rows.push(Value::Array(cells[start..end].to_vec()));
    }

    let mut canonical = object.clone();
    canonical.insert("cells".to_string(), Value::Array(matrix_rows));
    Value::Object(canonical)
}

fn upsert_comparison_view(
    replay: &mut Value,
    view_family: &str,
    value: Value,
) -> Result<(), String> {
    let object = replay
        .as_object_mut()
        .ok_or_else(|| "replay projection was not a JSON object".to_string())?;
    let comparison_views = object
        .entry("comparison_views".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let comparison_views = comparison_views
        .as_array_mut()
        .ok_or_else(|| "replay projection `comparison_views` was not an array".to_string())?;

    if let Some(existing) = comparison_views
        .iter_mut()
        .find(|view| view.get("view_family").and_then(Value::as_str) == Some(view_family))
    {
        if let Some(existing_object) = existing.as_object_mut() {
            existing_object.insert("value".to_string(), value);
        }
    } else {
        comparison_views.push(json!({
            "view_family": view_family,
            "value": value
        }));
    }

    Ok(())
}

fn filter_comparison_views(replay: &mut Value, required_views: &[String]) -> Result<(), String> {
    let Some(comparison_views) = replay
        .get_mut("comparison_views")
        .and_then(Value::as_array_mut)
    else {
        return Ok(());
    };

    comparison_views.retain(|view| {
        view.get("view_family")
            .and_then(Value::as_str)
            .is_some_and(|family| required_views.iter().any(|required| required == family))
    });

    Ok(())
}

fn normalize_comparison_value(value: &Value) -> Value {
    let mut current = value;
    loop {
        let Some(object) = current.as_object() else {
            break;
        };
        if object.get("boundary").and_then(Value::as_str) == Some("published_formula_result")
            && object.get("value").is_some()
        {
            current = object.get("value").expect("checked is_some");
            continue;
        }
        break;
    }

    let Some(object) = current.as_object() else {
        return current.clone();
    };
    let Some(kind) = object
        .get("value_kind")
        .or_else(|| object.get("kind"))
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
    else {
        return current.clone();
    };

    match kind.as_str() {
        "logical" => extract_logical_comparison_value(object)
            .map(|logical| json!({ "kind": "logical", "logical": logical }))
            .unwrap_or_else(|| current.clone()),
        "number" => extract_number_comparison_value(object)
            .map(|number| json!({ "kind": "number", "number": number }))
            .unwrap_or_else(|| current.clone()),
        "text" | "string" => extract_text_comparison_value(object)
            .map(|text| json!({ "kind": "text", "text": text }))
            .unwrap_or_else(|| current.clone()),
        "error" => extract_error_comparison_code(object)
            .map(|code| json!({ "kind": "error", "code": code }))
            .unwrap_or_else(|| current.clone()),
        "array" => normalize_array_comparison_value(object).unwrap_or_else(|| current.clone()),
        _ => current.clone(),
    }
}

fn nested_comparison_value_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Option<&'a Value> {
    object.get("value").and_then(Value::as_object)?.get(field)
}

fn extract_payload_comparison_value<'a>(
    object: &'a serde_json::Map<String, Value>,
) -> Option<&'a Value> {
    ["payload", "value", "items", "elements", "cells", "values"]
        .into_iter()
        .find_map(|key| object.get(key))
}

fn extract_logical_comparison_value(object: &serde_json::Map<String, Value>) -> Option<bool> {
    object
        .get("logical")
        .and_then(Value::as_bool)
        .or_else(|| object.get("value").and_then(Value::as_bool))
        .or_else(|| nested_comparison_value_field(object, "logical").and_then(Value::as_bool))
        .or_else(|| match extract_payload_comparison_value(object)? {
            Value::Bool(value) => Some(*value),
            Value::String(value) if value.eq_ignore_ascii_case("true") => Some(true),
            Value::String(value) if value.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        })
}

fn extract_number_comparison_value(object: &serde_json::Map<String, Value>) -> Option<Value> {
    object
        .get("number")
        .filter(|value| value.is_number())
        .cloned()
        .or_else(|| {
            object
                .get("numeric_value")
                .filter(|value| value.is_number())
                .cloned()
        })
        .or_else(|| {
            object
                .get("published_value")
                .filter(|value| value.is_number())
                .cloned()
        })
        .or_else(|| {
            object
                .get("value")
                .filter(|value| value.is_number())
                .cloned()
        })
        .or_else(|| {
            nested_comparison_value_field(object, "number")
                .filter(|value| value.is_number())
                .cloned()
        })
        .or_else(|| parse_number_comparison_value(extract_payload_comparison_value(object)?))
}

fn extract_number_comparison_value_lexeme(
    object: &serde_json::Map<String, Value>,
) -> Option<String> {
    object
        .get("number")
        .and_then(number_comparison_value_lexeme)
        .or_else(|| {
            object
                .get("numeric_value")
                .and_then(number_comparison_value_lexeme)
        })
        .or_else(|| {
            object
                .get("published_value")
                .and_then(number_comparison_value_lexeme)
        })
        .or_else(|| object.get("value").and_then(number_comparison_value_lexeme))
        .or_else(|| {
            nested_comparison_value_field(object, "number").and_then(number_comparison_value_lexeme)
        })
        .or_else(|| {
            extract_payload_comparison_value(object).and_then(number_comparison_value_lexeme)
        })
}

fn number_comparison_value_lexeme(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.as_str().to_string()),
        Value::String(text) => Some(text.clone()),
        Value::Object(object) => {
            extract_payload_comparison_value(object).and_then(number_comparison_value_lexeme)
        }
        _ => None,
    }
}

fn extract_text_comparison_value(object: &serde_json::Map<String, Value>) -> Option<String> {
    object
        .get("text")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            object
                .get("value")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            nested_comparison_value_field(object, "text")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            object
                .get("utf16_code_units")
                .and_then(decode_utf16_code_units_json)
        })
        .or_else(|| {
            nested_comparison_value_field(object, "utf16_code_units")
                .and_then(decode_utf16_code_units_json)
        })
        .or_else(|| match extract_payload_comparison_value(object)? {
            Value::String(value) => Some(value.clone()),
            Value::Object(payload) => payload
                .get("utf16_code_units")
                .and_then(decode_utf16_code_units_json),
            _ => None,
        })
}

fn extract_error_comparison_code(object: &serde_json::Map<String, Value>) -> Option<String> {
    object
        .get("error_kind")
        .and_then(Value::as_str)
        .map(normalize_error_code_alias)
        .or_else(|| {
            object
                .get("error_code")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            object
                .get("code")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            object
                .get("value")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            nested_comparison_value_field(object, "code")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            object
                .get("worksheet_error_code")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            nested_comparison_value_field(object, "worksheet_error_code")
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
        .or_else(|| {
            extract_payload_comparison_value(object)
                .and_then(Value::as_str)
                .map(normalize_error_code_alias)
        })
}

fn normalize_array_comparison_value(object: &serde_json::Map<String, Value>) -> Option<Value> {
    let payload = extract_payload_comparison_value(object)?;
    let cells = match payload {
        Value::Array(cells) => cells,
        Value::Object(nested) => extract_payload_comparison_value(nested)?.as_array()?,
        _ => return None,
    };

    let normalized_cells = cells
        .iter()
        .map(normalize_comparison_value)
        .collect::<Vec<_>>();
    let mut normalized = serde_json::Map::new();
    normalized.insert("kind".to_string(), Value::String("array".to_string()));
    let shape = object.get("shape").cloned().or_else(|| {
        let rows = object
            .get("rows")
            .or_else(|| object.get("row_count"))
            .and_then(Value::as_u64);
        let cols = object
            .get("cols")
            .or_else(|| object.get("columns"))
            .or_else(|| object.get("col_count"))
            .and_then(Value::as_u64);
        match (rows, cols) {
            (Some(rows), Some(cols)) => Some(json!({ "rows": rows, "cols": cols })),
            _ => None,
        }
    });
    if let Some(shape) = shape {
        normalized.insert("shape".to_string(), shape);
    }
    normalized.insert("cells".to_string(), Value::Array(normalized_cells));
    Some(Value::Object(normalized))
}

fn normalize_compare_ready_array_comparison_value(
    object: &serde_json::Map<String, Value>,
) -> Option<Value> {
    let payload = extract_payload_comparison_value(object)?;
    let cells = match payload {
        Value::Array(cells) => cells,
        Value::Object(nested) => extract_payload_comparison_value(nested)?.as_array()?,
        _ => return None,
    };

    let normalized_cells = cells
        .iter()
        .map(normalize_compare_ready_comparison_value)
        .collect::<Vec<_>>();
    let mut normalized = serde_json::Map::new();
    normalized.insert("kind".to_string(), Value::String("array".to_string()));
    let shape = object.get("shape").cloned().or_else(|| {
        let rows = object
            .get("rows")
            .or_else(|| object.get("row_count"))
            .and_then(Value::as_u64);
        let cols = object
            .get("cols")
            .or_else(|| object.get("columns"))
            .or_else(|| object.get("col_count"))
            .and_then(Value::as_u64);
        match (rows, cols) {
            (Some(rows), Some(cols)) => Some(json!({ "rows": rows, "cols": cols })),
            _ => None,
        }
    });
    if let Some(shape) = shape {
        normalized.insert("shape".to_string(), shape);
    }
    normalized.insert("cells".to_string(), Value::Array(normalized_cells));
    Some(Value::Object(normalized))
}

fn parse_number_comparison_value(value: &Value) -> Option<Value> {
    match value {
        Value::Number(_) => Some(value.clone()),
        Value::String(text) => {
            serde_json::Number::from_f64(text.parse::<f64>().ok()?).map(Value::Number)
        }
        Value::Object(object) => {
            extract_payload_comparison_value(object).and_then(parse_number_comparison_value)
        }
        _ => None,
    }
}

fn normalize_error_code_alias(value: &str) -> String {
    let compact = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();

    match compact.as_str() {
        "NULL" => "Null".to_string(),
        "DIV0" => "Div0".to_string(),
        "VALUE" => "Value".to_string(),
        "REF" => "Ref".to_string(),
        "NAME" => "Name".to_string(),
        "NUM" => "Num".to_string(),
        "NA" => "NA".to_string(),
        "BUSY" => "Busy".to_string(),
        "GETTINGDATA" => "GettingData".to_string(),
        "SPILL" => "Spill".to_string(),
        "CALC" => "Calc".to_string(),
        "FIELD" => "Field".to_string(),
        "BLOCKED" => "Blocked".to_string(),
        "CONNECT" => "Connect".to_string(),
        _ => value.to_string(),
    }
}

fn decode_utf16_code_units_json(value: &Value) -> Option<String> {
    let code_units = value.as_array()?;
    let decoded = code_units
        .iter()
        .map(|unit| unit.as_u64().and_then(|value| u16::try_from(value).ok()))
        .collect::<Option<Vec<_>>>()?;
    Some(String::from_utf16_lossy(&decoded))
}

fn replay_projection_gap_is_requested(
    view_family: Option<&str>,
    requested_views: &[String],
) -> bool {
    match view_family {
        Some(view_family) => requested_views.iter().any(|view| view == view_family),
        None => true,
    }
}

fn filter_replay_mismatch_records_to_requested_views(
    records: Vec<OxReplayMismatchRecord>,
    requested_views: &[String],
) -> Vec<OxReplayMismatchRecord> {
    records
        .into_iter()
        .filter(|record| {
            record.mismatch_kind != "projection_coverage_gap"
                || replay_projection_gap_is_requested(
                    record.view_family.as_deref(),
                    requested_views,
                )
        })
        .collect()
}

fn filter_replay_explain_records_to_requested_views(
    records: Vec<OxReplayExplainRecord>,
    requested_views: &[String],
) -> Vec<OxReplayExplainRecord> {
    records
        .into_iter()
        .filter(|record| {
            record.mismatch_kind != "projection_coverage_gap"
                || replay_projection_gap_is_requested(
                    record.view_family.as_deref(),
                    requested_views,
                )
        })
        .collect()
}

fn render_comparison_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "<unavailable>".to_string()),
    }
}

fn run_command_capture(
    command_label: &str,
    program: &str,
    args: &[OsString],
) -> Result<VerificationCommandCapture, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to start `{command_label}`: {error}"))?;

    Ok(VerificationCommandCapture {
        command_label: command_label.to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn materialize_case_workbook(
    workbook_path: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Result<VerificationCommandCapture, String> {
    if let Some(extraction) = spreadsheet_xml_extraction {
        fs::copy(&extraction.workbook_path, workbook_path).map_err(|error| {
            format!(
                "failed to copy SpreadsheetML workbook `{}` to `{}`: {error}",
                extraction.workbook_path,
                workbook_path.display()
            )
        })?;
        return Ok(VerificationCommandCapture {
            command_label: "copy-spreadsheetml-workbook".to_string(),
            exit_code: 0,
            stdout: format!(
                "copied workbook from {} to {}",
                extraction.workbook_path,
                workbook_path.display()
            ),
            stderr: String::new(),
        });
    }

    write_excel_2003_xml_workbook(workbook_path, &case.entered_cell_text)
}

fn write_excel_2003_xml_workbook(
    workbook_path: &Path,
    entered_cell_text: &str,
) -> Result<VerificationCommandCapture, String> {
    let cell_xml = spreadsheet_cell_xml(entered_cell_text);
    let workbook_xml = format!(
        r#"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:o="urn:schemas-microsoft-com:office:office"
 xmlns:x="urn:schemas-microsoft-com:office:excel"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
  <Worksheet ss:Name="Sheet1">
    <Table>
      <Row>
        {}
      </Row>
    </Table>
  </Worksheet>
</Workbook>
"#,
        cell_xml
    );
    fs::write(workbook_path, workbook_xml).map_err(|error| {
        format!(
            "failed to write Excel 2003 XML workbook `{}`: {error}",
            workbook_path.display()
        )
    })?;

    Ok(VerificationCommandCapture {
        command_label: "write-workbook".to_string(),
        exit_code: 0,
        stdout: format!("wrote workbook to {}", workbook_path.display()),
        stderr: String::new(),
    })
}

fn spreadsheet_cell_xml(entered_cell_text: &str) -> String {
    if entered_cell_text.starts_with('=') {
        return format!(
            r#"<Cell ss:Formula="{}"><Data ss:Type="Number">0</Data></Cell>"#,
            escape_spreadsheet_xml(entered_cell_text)
        );
    }

    if let Some(text) = entered_cell_text.strip_prefix('\'') {
        return format!(
            r#"<Cell><Data ss:Type="String">{}</Data></Cell>"#,
            escape_spreadsheet_xml(text)
        );
    }

    if let Ok(number) = entered_cell_text.parse::<f64>() {
        return format!(r#"<Cell><Data ss:Type="Number">{}</Data></Cell>"#, number);
    }

    if entered_cell_text.eq_ignore_ascii_case("true")
        || entered_cell_text.eq_ignore_ascii_case("false")
    {
        let boolean_value = if entered_cell_text.eq_ignore_ascii_case("true") {
            "1"
        } else {
            "0"
        };
        return format!(r#"<Cell><Data ss:Type="Boolean">{boolean_value}</Data></Cell>"#);
    }

    format!(
        r#"<Cell><Data ss:Type="String">{}</Data></Cell>"#,
        escape_spreadsheet_xml(entered_cell_text)
    )
}

fn escape_spreadsheet_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_json_file(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory for `{}`: {error}",
                path.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize JSON for `{}`: {error}", path.display()))?;
    fs::write(path, text)
        .map_err(|error| format!("failed to write JSON file `{}`: {error}", path.display()))
}

fn write_json_text_file(path: impl AsRef<Path>, text: &str) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory for `{}`: {error}",
                path.display()
            )
        })?;
    }
    let normalized = serde_json::from_str::<Value>(text).map_err(|error| {
        format!(
            "failed to parse JSON text before writing `{}`: {error}",
            path.display()
        )
    })?;
    write_json_file(path, &normalized)
}

fn read_json_file(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSON file `{}`: {error}", path.display()))?;
    parse_json_text(&text, &path.display().to_string())
}

fn parse_json_text(text: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(text)
        .map_err(|error| format!("failed to parse JSON from `{label}`: {error}"))
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

fn repo_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve DnaOneCalc repo root".to_string())
}

fn display_repo_relative(path: impl AsRef<Path>, repo_root: &Path) -> String {
    let path = path.as_ref();
    path.strip_prefix(repo_root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|error| {
            format!(
                "failed to resolve absolute path for `{}`: {error}",
                path.display()
            )
        })
}

#[cfg(test)]
mod consumer_shape_tests {
    use super::*;

    #[test]
    fn serialize_replay_projection_preserves_oxfunc_metadata_and_capability_facts() {
        let projection = oxfml_core::consumer::replay::ReplayProjectionResult {
            source_artifact_family: "runtime_formula_result".to_string(),
            source_schema_id: None,
            source_fixture_family: None,
            source_case_id: Some("metadata-case".to_string()),
            source_case_ids: Vec::new(),
            shared_scenario_alias: None,
            formula_stable_id: "formula:metadata".to_string(),
            session_id: None,
            library_context_snapshot_ref: None,
            typed_query_bundle_spec: None,
            registry_pin: None,
            witness_id: None,
            witness_lifecycle_state: None,
            retention_policy_id: None,
            source_bundle_ref: None,
            reduction_manifest_ref: None,
            phase: None,
            candidate_result_id: None,
            commit_decision_kind: None,
            execution_outcome_surface: None,
            trace_event_kinds: Vec::new(),
            comparison_views: None,
            verification_publication_surface: None,
            first_host_replay_capture_packet: None,
            semantic_kernel_metadata_version: Some(
                "semantic_kernel_metadata.v1;reduction_sensitive=true".to_string(),
            ),
            arg_admission_metadata_version: Some(
                "arg_admission_metadata.v1;existing_arg_preparation=values_only_pre_adapter"
                    .to_string(),
            ),
            producer_capability_set_keys: vec![
                "Materialisable(target_class=published_fallback_text)".to_string(),
            ],
            exercised_capability_keys: vec![
                "Materialisable(target_class=published_fallback_text)".to_string()
            ],
            prepared_formula_identity: None,
            host_formula_context: None,
            host_name_bind_results: Vec::new(),
            host_reference_bind_results: Vec::new(),
        };

        let serialized = serialize_replay_projection(&projection, true);

        assert_eq!(
            serialized["semantic_kernel_metadata_version"],
            json!("semantic_kernel_metadata.v1;reduction_sensitive=true")
        );
        assert_eq!(
            serialized["arg_admission_metadata_version"],
            json!("arg_admission_metadata.v1;existing_arg_preparation=values_only_pre_adapter")
        );
        assert_eq!(
            serialized["producer_capability_set_keys"],
            json!(["Materialisable(target_class=published_fallback_text)"])
        );
        assert_eq!(
            serialized["exercised_capability_keys"],
            json!(["Materialisable(target_class=published_fallback_text)"])
        );
    }

    #[test]
    fn replay_display_comparison_summary_prefers_effective_display_family() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "effective_display_text".to_string(),
                severity: Some("informational".to_string()),
                view_family: Some("effective_display_text".to_string()),
                left_value_repr: Some("6".to_string()),
                right_value_repr: Some("$6.00".to_string()),
                detail: Some("comparison view values diverged".to_string()),
            }],
            Some("6"),
            Some("$6.00"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00")
        );
    }

    #[test]
    fn replay_projection_coverage_gap_summaries_keep_family_specific_labels() {
        let summaries = replay_projection_coverage_gap_summaries(&[
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                detail: Some(
                    "comparison view family `formatting_view` is missing on one side".to_string(),
                ),
            },
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("conditional_formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("[{\"range\":\"A1\"}]".to_string()),
                detail: Some(
                    "comparison view family `conditional_formatting_view` is missing on one side"
                        .to_string(),
                ),
            },
        ]);

        assert_eq!(summaries.len(), 2);
        assert_eq!(
            summaries[0],
            "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
        );
        assert_eq!(
            summaries[1],
            "Projection coverage gap (conditional_formatting_view): comparison view family `conditional_formatting_view` is missing on one side"
        );
    }

    #[test]
    fn replay_display_comparison_summary_reads_comparison_value_family() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "comparison_value".to_string(),
                severity: Some("semantic".to_string()),
                view_family: Some("comparison_value".to_string()),
                left_value_repr: None,
                right_value_repr: None,
                detail: None,
            }],
            Some("6"),
            Some("7"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Comparison value divergence: OxFml 6 vs Excel 7")
        );
    }

    #[test]
    fn parse_oxreplay_records_keep_machine_readable_view_family_shape() {
        let diff_report = json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "formatting_view",
                    "right_value": { "number_format_code": "$#,##0.00" },
                    "detail": "comparison view family `formatting_view` is missing on one side"
                }
            ]
        });
        let explain_stdout = serde_json::to_string(&json!({
            "records": [
                {
                    "query_id": "explain-01",
                    "summary": "comparison diverged on `effective_display_text`",
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "query_id": "explain-02",
                    "summary": "comparison view family `conditional_formatting_view` is missing on one side",
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "conditional_formatting_view",
                    "right_value": [{ "range": "A1" }],
                    "detail": "comparison view family `conditional_formatting_view` is missing on one side"
                }
            ]
        }))
        .expect("json text");

        let mismatch_records = parse_oxreplay_mismatch_records(&diff_report);
        let explain_records =
            parse_oxreplay_explain_records(&explain_stdout).expect("explain records");

        assert_eq!(mismatch_records.len(), 2);
        assert_eq!(
            mismatch_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(mismatch_records[0].left_value_repr.as_deref(), Some("6"));
        assert_eq!(
            mismatch_records[1].view_family.as_deref(),
            Some("formatting_view")
        );
        assert_eq!(
            mismatch_records[1].right_value_repr.as_deref(),
            Some("{\"number_format_code\":\"$#,##0.00\"}")
        );
        assert_eq!(explain_records.len(), 2);
        assert_eq!(
            explain_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(
            explain_records[1].view_family.as_deref(),
            Some("conditional_formatting_view")
        );
        assert_eq!(
            explain_records[1].right_value_repr.as_deref(),
            Some("[{\"range\":\"A1\"}]")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn sample_programmatic_formatting_context() -> ProgrammaticFormattingContext {
        ProgrammaticFormattingContext {
            format_profile_id: Some("current_excel_host".to_string()),
            number_format_code: Some("$#,##0.00".to_string()),
            date1904: Some(false),
        }
    }

    fn default_programmatic_formatting_context() -> ProgrammaticFormattingContext {
        default_programmatic_corpus_formatting_context()
    }

    fn sample_trusted_excel_render_context() -> ProgrammaticExcelRenderContext {
        ProgrammaticExcelRenderContext {
            render_locale_pinned: true,
            render_locale_source: Some("captured_excel_host".to_string()),
            render_locale_recorded: true,
            trusted: true,
            requested_format_profile_id: Some("en-US".to_string()),
            decimal_separator: Some(".".to_string()),
            thousands_separator: Some(",".to_string()),
            list_separator: Some(",".to_string()),
            date_separator: Some("/".to_string()),
            time_separator: Some(":".to_string()),
            note: Some("Captured Excel render context".to_string()),
        }
    }

    fn sample_oxxlplay_render_context_artifact() -> Value {
        json!({
            "render_context_schema": "oxxlplay.excel_render_context.v1",
            "render_context_id": "excel-primary",
            "capture_mode": "excel_black_box_observation",
            "render_formatting": {
                "capture_status": "captured",
                "use_system_separators": true,
                "decimal_separator": ".",
                "thousands_separator": "\u{00A0}",
                "list_separator": ";",
                "date_separator": "/",
                "time_separator": ":"
            }
        })
    }

    fn fake_diff_report(left: &Value, right: &Value) -> Value {
        let mut families = left
            .get("comparison_views")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .chain(
                right
                    .get("comparison_views")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten(),
            )
            .filter_map(|view| view.get("view_family").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        families.sort();
        families.dedup();

        let mismatches = families
            .into_iter()
            .filter_map(|family| {
                let left_value = projection_comparison_value(left, &family);
                let right_value = projection_comparison_value(right, &family);
                match (left_value, right_value) {
                    (Some(left_value), Some(right_value)) if left_value != right_value => Some(json!({
                        "mismatch_kind": family,
                        "severity": "semantic",
                        "view_family": family,
                        "left_value": left_value,
                        "right_value": right_value,
                        "detail": "comparison view values diverged"
                    })),
                    (Some(_), None) | (None, Some(_)) => Some(json!({
                        "mismatch_kind": "projection_coverage_gap",
                        "severity": "coverage",
                        "view_family": family,
                        "detail": format!("comparison view family `{family}` is missing on one side")
                    })),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        json!({
            "equivalent": mismatches.is_empty(),
            "mismatches": mismatches
        })
    }

    #[derive(Default)]
    struct FakeVerificationRunner {
        capture_exit_code: i32,
        validate_exit_code: i32,
        diff_exit_code: i32,
        explain_exit_code: i32,
        diff_equivalent: bool,
        assert_compare_inputs_ready: bool,
        batch_case_status: Option<String>,
        batch_case_error: Option<String>,
        captured_cell_value: Option<Value>,
        captured_value_repr: Option<String>,
        captured_formula_text: Option<String>,
        captured_effective_display_text: Option<String>,
        captured_render_context_json: Option<Value>,
        calls: Mutex<Vec<String>>,
    }

    impl VerificationCommandRunner for FakeVerificationRunner {
        fn run_oxxlplay_capture_batch(
            &self,
            manifest_path: &Path,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls
                .lock()
                .expect("calls")
                .push("oxxlplay_capture_batch".to_string());
            let manifest: OxxlplayBatchManifest =
                serde_json::from_str(&fs::read_to_string(manifest_path).expect("batch manifest"))
                    .expect("batch manifest parse");
            let batch_output_root = PathBuf::from(&manifest.output_root);
            fs::create_dir_all(&batch_output_root).expect("batch output root");
            let mut case_index = Vec::new();

            for case in &manifest.cases {
                let output_dir = PathBuf::from(&case.case_output_dir);
                fs::create_dir_all(output_dir.join("views")).expect("views dir");
                let repo_root = repo_root().expect("repo root");
                let output_dir_repo_relative = display_repo_relative(&output_dir, &repo_root);
                let captured_cell_value = self.captured_cell_value.clone().unwrap_or_else(|| {
                    if self.diff_equivalent {
                        json!({
                            "kind": "number",
                            "number": 6.0
                        })
                    } else {
                        json!({
                            "kind": "number",
                            "number": 7.0
                        })
                    }
                });
                let captured_value_repr = self.captured_value_repr.clone().unwrap_or_else(|| {
                    if self.diff_equivalent {
                        "6".to_string()
                    } else {
                        "7".to_string()
                    }
                });
                let captured_formula_text = self
                    .captured_formula_text
                    .clone()
                    .unwrap_or_else(|| "=SUM(1,2,3)".to_string());
                let captured_effective_display_text = self
                    .captured_effective_display_text
                    .clone()
                    .unwrap_or_else(|| {
                        if self.diff_equivalent {
                            "6".to_string()
                        } else {
                            "$7.00".to_string()
                        }
                    });
                let batch_case_status = self
                    .batch_case_status
                    .clone()
                    .unwrap_or_else(|| "succeeded".to_string());
                if batch_case_status == "succeeded" && self.batch_case_error.is_none() {
                    write_json_file(
                        output_dir.join("capture.json"),
                        &json!({
                            "surfaces": [
                                {
                                    "surface": {
                                        "surface_id": "sheet1_a1_value",
                                        "surface_kind": "cell_value",
                                        "locator": "Sheet1!A1",
                                "required": true
                            },
                            "status": "direct",
                            "comparison_value": captured_cell_value,
                            "value_repr": captured_value_repr,
                            "capture_loss": "none",
                            "uncertainty": "none"
                                },
                                {
                                    "surface": {
                                        "surface_id": "sheet1_a1_formula",
                                        "surface_kind": "formula_text",
                                        "locator": "Sheet1!A1",
                                        "required": false
                                    },
                                    "status": "direct",
                                    "value_repr": captured_formula_text,
                                    "capture_loss": "none",
                                    "uncertainty": "none"
                                },
                                {
                                    "surface": {
                                        "surface_id": "sheet1_a1_display",
                                        "surface_kind": "effective_display_text",
                                        "locator": "Sheet1!A1",
                                        "required": true
                                    },
                                    "status": "direct",
                                    "value_repr": captured_effective_display_text,
                                    "capture_loss": "none",
                                    "uncertainty": "none"
                                }
                            ]
                        }),
                    )
                    .expect("capture should write");
                    if let Some(render_context_json) = &self.captured_render_context_json {
                        write_json_file(
                            output_dir.join("render-context.json"),
                            render_context_json,
                        )
                        .expect("render context should write");
                    }
                    write_json_file(
                        output_dir.join("oxreplay-manifest.json"),
                        &json!({
                            "bundle_id": "fake-bundle",
                            "scenario_id": case.scenario_id,
                            "bundle_schema": "replay.bundle.v1"
                        }),
                    )
                    .expect("manifest should write");
                    write_json_file(
                        output_dir.join("views").join("normalized-replay.json"),
                        &json!({
                            "scenario_id": case.scenario_id,
                            "lane_id": "oxxlplay",
                            "events": [
                                {
                                    "event_id": "sheet1_a1_value",
                                    "source_label": "cell_value:Sheet1!A1:direct",
                                    "normalized_family": "excel.surface.cell_value.direct:Sheet1!A1=6"
                                }
                            ],
                            "registry_refs": [],
                            "comparison_views": [
                                {
                                    "view_family": "comparison_value",
                                    "value": {
                                        "boundary": "published_formula_result",
                                        "value": captured_cell_value,
                                        "wire_schema": "oxfunc_value_types.aligned_json.v1"
                                    }
                                },
                                {
                                    "view_family": "effective_display_text",
                                    "value": captured_effective_display_text
                                }
                            ]
                        }),
                    )
                    .expect("normalized replay should write");
                    case_index.push(json!({
                        "case_id": case.case_id,
                        "status": batch_case_status,
                        "output_dir": output_dir_repo_relative,
                        "capture_path": format!("{output_dir_repo_relative}/capture.json"),
                        "oxreplay_manifest_path": format!("{output_dir_repo_relative}/oxreplay-manifest.json"),
                        "normalized_replay_path": format!("{output_dir_repo_relative}/views/normalized-replay.json")
                    }));
                } else {
                    case_index.push(json!({
                        "case_id": case.case_id,
                        "status": batch_case_status,
                        "error": self.batch_case_error.clone(),
                        "output_dir": output_dir_repo_relative
                    }));
                }
            }

            write_json_file(
                batch_output_root.join("batch-output-index.json"),
                &json!({ "cases": case_index }),
            )
            .expect("batch output index should write");
            Ok(VerificationCommandCapture {
                command_label: "oxxlplay-capture-batch".to_string(),
                exit_code: self.capture_exit_code,
                stdout: String::new(),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_validate_bundle(
            &self,
            _manifest_path: &Path,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls
                .lock()
                .expect("calls")
                .push("validate_bundle".to_string());
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-validate-bundle".to_string(),
                exit_code: self.validate_exit_code,
                stdout: "{\"status\":\"Valid\"}".to_string(),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_diff(
            &self,
            left_path: &Path,
            _left_kind: &str,
            right_path: &Path,
            _right_kind: &str,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls.lock().expect("calls").push("diff".to_string());
            let left = read_json_file(left_path).expect("left projection json");
            let right = read_json_file(right_path).expect("right replay json");
            if self.assert_compare_inputs_ready {
                assert!(
                    projection_comparison_value(&left, EXECUTION_OUTCOME_VIEW_FAMILY).is_some()
                );
                assert!(
                    projection_comparison_value(&right, EXECUTION_OUTCOME_VIEW_FAMILY).is_some()
                );
                assert!(projection_comparison_value(&left, "comparison_value").is_some());
                if let Some(display_value) =
                    projection_comparison_value(&left, "effective_display_text")
                {
                    assert_eq!(display_value, json!("6"));
                }
                // The right path is the compare-ready normalized replay
                // (Excel-side observation). With `diff_equivalent: false`,
                // the fake runner seeds Excel value `7.0` to simulate
                // divergence from OxFml's `=SUM(1,2,3) = 6`. The
                // compare-ready normalization preserves numeric witnesses
                // as their lexeme strings (see `normalize_compare_ready_comparison_value`).
                assert_eq!(
                    projection_comparison_value(&right, "comparison_value"),
                    Some(json!({
                        "kind": "number",
                        "number": "7.0"
                    }))
                );
            }
            let diff_report = if self.diff_equivalent {
                json!({"equivalent": true, "mismatches": []})
            } else {
                fake_diff_report(&left, &right)
            };
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-diff".to_string(),
                exit_code: self.diff_exit_code,
                stdout: serde_json::to_string(&diff_report).expect("diff report json"),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_explain(
            &self,
            _left_path: &Path,
            _left_kind: &str,
            _right_path: &Path,
            _right_kind: &str,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls
                .lock()
                .expect("calls")
                .push("explain".to_string());
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-explain".to_string(),
                exit_code: self.explain_exit_code,
                stdout: "{\"summary\":\"diff\"}".to_string(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn verification_batch_writes_mismatched_case_as_workbench_artifact() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: false,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(report.case_reports.len(), 1);
        assert_eq!(
            report.case_reports[0].comparison_status,
            ProgrammaticComparisonStatus::Mismatched
        );
        assert_eq!(
            report.case_reports[0].artifact_catalog_entry.open_mode_hint,
            crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench
        );
        assert!(output_root
            .join("verification-bundle-report.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-1")
            .join("comparison-summary.json")
            .is_file());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_uses_oxreplay_display_mismatch_for_host_verdict() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-display-mismatch".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: Some(sample_programmatic_formatting_context()),
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            captured_cell_value: Some(json!({
                "kind": "number",
                "number": 6.0
            })),
            captured_value_repr: Some("6".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Mismatched
        );
        assert_eq!(case_report.value_match, Some(true));
        assert_eq!(case_report.display_match, Some(false));
        assert_eq!(case_report.replay_equivalent, Some(false));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_treats_pre_execution_rejection_equivalence_as_matched() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "FTC-0448".to_string(),
                entered_cell_text: "=LET(dict,{\"x\",LAMBDA(100);\"y\",LAMBDA(200)},GETlambda,LAMBDA(d,LAMBDA(key,LET(keys,TAKE(d,,1),objects,DROP(d,,1),obj,XLOOKUP(key,keys,objects,\"not found\"),obj()))),getter,GETlambda(dict),getter(\"y\"))".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            batch_case_status: Some("failed".to_string()),
            batch_case_error: Some("programmatic_formula_authoring_failed: Excel COM rejected Formula2 assignment for entered_cell_text with 0x800A03EC".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(case_report.value_match, None);
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, Some(true));
        assert!(case_report.excel_summary.is_none());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn synthetic_oxfml_pre_execution_rejection_requires_excel_pre_execution_rejection() {
        let summary = OxfmlVerificationSummary {
            evaluation_summary: None,
            comparison_value: None,
            effective_display_summary: None,
            blocked_reason: Some("syntax diagnostics".to_string()),
            parse_status: Some("Diagnostics".to_string()),
            green_tree_key: None,
        };
        let failure_reason =
            "OxFml runtime execution failed for case `x`: formula execution rejected due to syntax diagnostics: expected ')' at 63:0";

        assert!(synthetic_oxfml_pre_execution_rejection_outcome_for_failure(
            &summary,
            failure_reason,
            &ExecutionOutcomeSurface {
                comparison_value: normalized_pre_execution_rejection_outcome(),
                ordinary_value_comparable: false,
            }
        )
        .is_some());

        assert!(synthetic_oxfml_pre_execution_rejection_outcome_for_failure(
            &summary,
            failure_reason,
            &ExecutionOutcomeSurface {
                comparison_value: normalized_completed_execution_outcome(),
                ordinary_value_comparable: true,
            }
        )
        .is_none());
    }

    #[test]
    fn verification_batch_treats_explicit_oxfml_syntax_rejection_and_excel_authoring_rejection_as_matched(
    ) {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "FTC-0916".to_string(),
                entered_cell_text:
                    "=((((((((((((((((1+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            batch_case_status: Some("failed".to_string()),
            batch_case_error: Some("programmatic_formula_authoring_failed: Excel COM rejected Formula2 assignment for entered_cell_text with 0x800A03EC".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(case_report.value_match, None);
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, Some(true));
        assert!(case_report.excel_summary.is_none());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_treats_completed_oxfml_and_excel_authoring_rejection_as_matched() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "FTC-0050".to_string(),
                entered_cell_text: "=1E+308*2".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            batch_case_status: Some("failed".to_string()),
            batch_case_error: Some("programmatic_formula_authoring_failed: Excel COM rejected Formula2 assignment for entered_cell_text with 0x800A03EC".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(case_report.value_match, None);
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, Some(true));
        assert!(case_report.excel_summary.is_none());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_blocks_when_required_replay_value_surface_is_missing() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-missing-compare-value".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            captured_cell_value: Some(Value::Null),
            captured_value_repr: Some("".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Blocked
        );
        assert!(case_report
            .discrepancy_summary
            .as_deref()
            .is_some_and(|summary| summary.contains("comparison_value")));
        assert_eq!(case_report.replay_equivalent, None);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_emits_programmatic_formula_scenario_for_formula_cases() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-formula".to_string(),
                entered_cell_text: "=LET(a,{1,2,3},b,{4,5,6},SUM(a*b))".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: false,
            diff_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(report.case_reports.len(), 1);
        let case_dir = output_root.join("cases").join("case-formula");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");

        assert_eq!(scenario["workbook_kind"], "programmatic-formula");
        assert_eq!(
            scenario["entered_cell_text"],
            "=LET(a,{1,2,3},b,{4,5,6},SUM(a*b))"
        );
        assert_eq!(scenario["workbook_ref"], "./workbook.xml");
        // The case carries `formatting_context: None`, so
        // `programmatic_display_contract_is_explicit` returns false (no
        // number_format_code), and `effective_requested_observation_scope`
        // selects the without-display variant. Display surfaces drop out
        // of the required scope; cell_value remains.
        assert_eq!(
            scenario["requested_observation_scope"]["oxxlplay_required_surfaces"],
            json!(["cell_value"])
        );
        assert_eq!(
            scenario["requested_observation_scope"]["oxreplay_required_views"],
            json!(["execution_outcome", "comparison_value"])
        );
        // With no explicit display contract, the effective_display_text
        // observable surface is omitted entirely (not merely downgraded
        // to required=false). The case ships only cell_value (required)
        // and formula_text (optional) as observable surfaces.
        let observable_surfaces = scenario["observable_surfaces"]
            .as_array()
            .expect("observable surfaces");
        assert!(observable_surfaces
            .iter()
            .any(|surface| surface["surface_kind"] == "cell_value"
                && surface["required"] == json!(true)));
        assert!(!observable_surfaces
            .iter()
            .any(|surface| surface["surface_kind"] == "effective_display_text"));
        let requested_scope: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("required-observation-scope.json"))
                .expect("requested observation scope json"),
        )
        .expect("requested observation scope parse");
        assert_eq!(
            requested_scope["oxxlplay_required_surfaces"],
            json!(["cell_value"])
        );
        assert_eq!(
            requested_scope["oxfml_required_scope"],
            json!(["entered_cell_text", "returned_value_surface"])
        );
        let batch_manifest: Value = serde_json::from_str(
            &fs::read_to_string(
                output_root
                    .join("commands")
                    .join("oxxlplay-capture-batch.manifest.json"),
            )
            .expect("batch manifest json"),
        )
        .expect("batch manifest parse");
        let manifest_case = batch_manifest["cases"]
            .as_array()
            .and_then(|cases| cases.first())
            .expect("manifest case");
        assert_eq!(
            manifest_case["requested_observation_scope"]["oxxlplay_required_surfaces"],
            json!(["cell_value"])
        );
        let manifest_observable_surfaces = manifest_case["observable_surfaces"]
            .as_array()
            .expect("manifest observable surfaces");
        assert!(manifest_observable_surfaces
            .iter()
            .any(|surface| surface["surface_kind"] == "cell_value"
                && surface["required"] == json!(true)));
        assert!(!manifest_observable_surfaces
            .iter()
            .any(|surface| surface["surface_kind"] == "effective_display_text"));
        assert!(case_dir.join("workbook.xml").is_file());

        let input_request: Value = serde_json::from_str(
            &fs::read_to_string(output_root.join("input-request.json"))
                .expect("input request json"),
        )
        .expect("input request parse");
        assert_eq!(
            input_request["cases"][0]["formatting_context"],
            json!({
                "format_profile_id": "en-US",
                "date1904": false
            })
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_unpinned_excel_render_context_for_programmatic_cases() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-programmatic-text".to_string(),
                entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner::default();

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_dir = output_root.join("cases").join("case-programmatic-text");
        let case_input: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("case-input.json")).expect("case input json"),
        )
        .expect("case input parse");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");

        assert_eq!(
            case_input["excel_render_context"]["render_locale_pinned"],
            json!(false)
        );
        assert_eq!(
            case_input["excel_render_context"]["render_locale_source"],
            json!("observation_machine_default")
        );
        assert_eq!(
            case_input["excel_render_context"]["provenance"]["kind"],
            json!("fallback")
        );
        assert_eq!(case_input["excel_render_context"]["trusted"], json!(false));
        assert_eq!(
            scenario["excel_render_context"]["render_locale_pinned"],
            json!(false)
        );
        assert_eq!(
            report.case_reports[0]
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_pinned),
            Some(false)
        );
        assert_eq!(
            report.case_reports[0]
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_source.as_deref()),
            Some("observation_machine_default")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_shared_render_context_refs_in_case_and_scenario() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let mut render_contexts = BTreeMap::new();
        render_contexts.insert("ctx-1".to_string(), sample_trusted_excel_render_context());
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts,
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-shared-render-context".to_string(),
                entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: Some("ctx-1".to_string()),
            }],
        };
        let runner = FakeVerificationRunner::default();

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_dir = output_root.join("cases").join("case-shared-render-context");
        let case_input: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("case-input.json")).expect("case input json"),
        )
        .expect("case input parse");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");

        assert_eq!(
            case_input["excel_render_context"]["provenance"]["kind"],
            json!("shared_ref")
        );
        assert_eq!(
            case_input["excel_render_context"]["provenance"]["render_context_ref"],
            json!("ctx-1")
        );
        assert_eq!(case_input["excel_render_context"]["trusted"], json!(true));
        assert_eq!(
            scenario["excel_render_context"]["render_locale_pinned"],
            json!(true)
        );
        assert_eq!(
            report.case_reports[0]
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_source.as_deref()),
            Some("captured_excel_host")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_inline_trusted_render_context_in_case_and_scenario() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-inline-render-context".to_string(),
                entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: Some(default_programmatic_formatting_context()),
                excel_render_context: Some(sample_trusted_excel_render_context()),
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner::default();

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_dir = output_root.join("cases").join("case-inline-render-context");
        let case_input: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("case-input.json")).expect("case input json"),
        )
        .expect("case input parse");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");

        assert_eq!(
            case_input["excel_render_context"]["provenance"]["kind"],
            json!("inline")
        );
        assert_eq!(scenario["excel_render_context"]["trusted"], json!(true));
        assert_eq!(
            report.case_reports[0]
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_pinned),
            Some(true)
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn resolve_effective_excel_render_context_uses_inline_case_context() {
        let case = ProgrammaticFormulaCase {
            case_id: "case-inline-render-context".to_string(),
            entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: Some(sample_trusted_excel_render_context()),
            render_context_ref: None,
        };

        let resolved = resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
            .expect("resolved render context");

        assert_eq!(resolved.provenance.kind, "inline");
        assert_eq!(resolved.provenance.render_context_ref, None);
        assert!(resolved.context.trusted);
        assert!(resolved.context.render_locale_pinned);
    }

    #[test]
    fn trusted_excel_separator_context_requires_trusted_render_context() {
        let context = EffectiveExcelRenderContext {
            context: ProgrammaticExcelRenderContext {
                trusted: false,
                ..sample_trusted_excel_render_context()
            },
            provenance: EffectiveExcelRenderContextProvenance {
                kind: "inline".to_string(),
                render_context_ref: None,
            },
        };

        assert_eq!(trusted_excel_separator_context(Some(&context)), None);
    }

    #[test]
    fn verification_locale_context_uses_trusted_render_context_profile_when_programmatic_context_is_absent(
    ) {
        let case = ProgrammaticFormulaCase {
            case_id: "case-trusted-render-context-profile".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: None,
            excel_render_context: Some(sample_trusted_excel_render_context()),
            render_context_ref: None,
        };
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        let locale_context =
            verification_locale_context(&case, None, Some(&effective_excel_render_context));

        assert_eq!(locale_context.profile.id, LocaleProfileId::EnUs);
        assert_eq!(locale_context.profile.decimal_separator, ".");
        assert_eq!(locale_context.profile.thousands_separator, ",");
    }

    #[test]
    fn verification_locale_context_overrides_profile_separators_from_trusted_render_context() {
        let case = ProgrammaticFormulaCase {
            case_id: "case-separator-override".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: Some(ProgrammaticExcelRenderContext {
                thousands_separator: Some("\u{A0}".to_string()),
                list_separator: Some(";".to_string()),
                ..sample_trusted_excel_render_context()
            }),
            render_context_ref: None,
        };
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        let locale_context =
            verification_locale_context(&case, None, Some(&effective_excel_render_context));

        assert_eq!(locale_context.profile.id, LocaleProfileId::EnUs);
        assert_eq!(locale_context.profile.decimal_separator, ".");
        assert_eq!(locale_context.profile.thousands_separator, "\u{A0}");
        assert_eq!(locale_context.profile.date_separator, "/");
        assert_eq!(locale_context.profile.time_separator, ":");
    }

    #[test]
    fn verification_locale_context_prefers_explicit_formatting_context_over_trusted_render_context_profile(
    ) {
        let case = ProgrammaticFormulaCase {
            case_id: "case-explicit-format-profile-precedence".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(ProgrammaticFormattingContext {
                format_profile_id: Some("current_excel_host".to_string()),
                number_format_code: None,
                date1904: Some(false),
            }),
            excel_render_context: Some(sample_trusted_excel_render_context()),
            render_context_ref: None,
        };
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        let locale_context =
            verification_locale_context(&case, None, Some(&effective_excel_render_context));

        assert_eq!(locale_context.profile.id, LocaleProfileId::CurrentExcelHost);
    }

    #[test]
    fn validate_verification_request_rejects_unknown_render_context_ref() {
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-missing-render-context".to_string(),
                entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: Some("missing-render-context".to_string()),
            }],
        };

        let error = validate_verification_request(&request).expect_err("missing ref should fail");
        assert!(error.contains("unknown render context `missing-render-context`"));
    }

    #[test]
    fn import_effective_excel_render_context_from_oxxlplay_output_promotes_trusted_capture() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp root");
        write_json_file(
            temp_root.join("render-context.json"),
            &sample_oxxlplay_render_context_artifact(),
        )
        .expect("render context json");
        let case = ProgrammaticFormulaCase {
            case_id: "case-captured-render-context".to_string(),
            entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };

        let imported =
            import_effective_excel_render_context_from_oxxlplay_output(&case, &temp_root)
                .expect("import result")
                .expect("captured context");

        assert_eq!(imported.provenance.kind, "oxxlplay_capture_artifact");
        assert!(imported.context.trusted);
        assert_eq!(
            imported.context.render_locale_source.as_deref(),
            Some("oxxlplay_render_context_capture")
        );
        assert_eq!(imported.context.decimal_separator.as_deref(), Some("."));
        assert_eq!(
            imported.context.thousands_separator.as_deref(),
            Some("\u{A0}")
        );
        assert_eq!(imported.context.list_separator.as_deref(), Some(";"));
        assert_eq!(imported.context.date_separator.as_deref(), Some("/"));
        assert_eq!(imported.context.time_separator.as_deref(), Some(":"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn import_captured_render_context_and_refresh_oxfml_if_needed_reruns_and_persists_outputs() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let case_dir = temp_root.join("cases").join("FTC-0288");
        let command_dir = case_dir.join("commands");
        let oxxlplay_dir = case_dir.join("oxxlplay");
        let oxreplay_dir = case_dir.join("oxreplay");
        fs::create_dir_all(&command_dir).expect("command dir");
        fs::create_dir_all(&oxxlplay_dir).expect("oxxlplay dir");
        fs::create_dir_all(&oxreplay_dir).expect("oxreplay dir");
        write_json_file(
            oxxlplay_dir.join("render-context.json"),
            &sample_oxxlplay_render_context_artifact(),
        )
        .expect("render context");

        let case = ProgrammaticFormulaCase {
            case_id: "FTC-0288".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");
        let scenario_path = case_dir.join("scenario.json");
        let projection_path = case_dir.join("oxfml-v1-replay-projection.json");
        write_json_file(
            case_dir.join("case-input.json"),
            &json!({
                "excel_render_context": &effective_excel_render_context
            }),
        )
        .expect("case input");
        write_json_file(
            &scenario_path,
            &json!({
                "excel_render_context": &effective_excel_render_context
            }),
        )
        .expect("scenario");

        let initial_oxfml_result = OxfmlCaseArtifacts {
            summary: OxfmlVerificationSummary {
                evaluation_summary: Some("Text · 1,234,567.89".to_string()),
                comparison_value: Some(json!({"kind":"text","text":"1,234,567.89"})),
                effective_display_summary: None,
                blocked_reason: None,
                parse_status: Some("Valid".to_string()),
                green_tree_key: Some("green:initial".to_string()),
            },
            replay_projection_json: json!({
                "comparison_views": [
                    {
                        "view_family": "comparison_value",
                        "value": {"kind":"text","text":"1,234,567.89"}
                    }
                ]
            }),
            execution_failure: None,
        };
        persist_oxfml_case_artifacts(&case_dir, &projection_path, &initial_oxfml_result)
            .expect("initial oxfml artifacts");

        let mut prepared = PreparedVerificationCase {
            case_dir: case_dir.clone(),
            command_dir,
            oxxlplay_dir,
            oxreplay_dir,
            scenario_path: scenario_path.clone(),
            projection_path: projection_path.clone(),
            effective_case: case,
            effective_excel_render_context,
            spreadsheet_xml_extraction: None,
            upstream_gap_report: None,
            oxfml_result: initial_oxfml_result,
            batch_case_manifest: OxxlplayBatchCaseManifest {
                case_id: "FTC-0288".to_string(),
                scenario_id: "onecalc_verify_FTC-0288".to_string(),
                workbook_ref: "./workbook.xml".to_string(),
                workbook_kind: "programmatic-formula".to_string(),
                trigger: "open_then_recalc".to_string(),
                case_output_dir: "./oxxlplay".to_string(),
                observable_surfaces: Vec::new(),
                entered_cell_text: Some("=TEXT(1234567.89,\"#,##0.00\")".to_string()),
                requested_observation_scope: None,
                source_cell_locator: None,
                source_workbook_path: None,
            },
        };

        let refresh_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let refresh_count_clone = refresh_count.clone();
        let resolved_output_dir = prepared.oxxlplay_dir.clone();
        let refreshed = import_captured_render_context_and_refresh_oxfml_if_needed(
            &mut prepared,
            &resolved_output_dir,
            move |_case, _spreadsheet_xml_extraction, effective_excel_render_context| {
                refresh_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(
                    effective_excel_render_context
                        .and_then(|value| value.context.thousands_separator.as_deref()),
                    Some("\u{A0}")
                );
                Ok(OxfmlCaseArtifacts {
                    summary: OxfmlVerificationSummary {
                        evaluation_summary: Some("Text · 1234,567.89".to_string()),
                        comparison_value: Some(json!({"kind":"text","text":"1234,567.89"})),
                        effective_display_summary: None,
                        blocked_reason: None,
                        parse_status: Some("Valid".to_string()),
                        green_tree_key: Some("green:refreshed".to_string()),
                    },
                    replay_projection_json: json!({
                        "comparison_views": [
                            {
                                "view_family": "comparison_value",
                                "value": {"kind":"text","text":"1234,567.89"}
                            }
                        ]
                    }),
                    execution_failure: None,
                })
            },
        )
        .expect("refresh should succeed");

        let case_input = read_json_file(case_dir.join("case-input.json")).expect("case input");
        let scenario = read_json_file(&scenario_path).expect("scenario");
        let projection = read_json_file(&projection_path).expect("projection");
        let runtime_summary =
            read_json_file(case_dir.join("oxfml-runtime-summary.json")).expect("summary");
        let execution_context =
            read_json_file(case_dir.join("oxfml-execution-context.json")).expect("context");

        assert!(refreshed);
        assert_eq!(refresh_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            prepared.effective_excel_render_context.provenance.kind,
            "oxxlplay_capture_artifact"
        );
        assert_eq!(
            case_input["excel_render_context"]["provenance"]["kind"],
            json!("oxxlplay_capture_artifact")
        );
        assert_eq!(scenario["excel_render_context"]["trusted"], json!(true));
        assert_eq!(
            projection["comparison_views"][0]["value"]["text"],
            json!("1234,567.89")
        );
        assert_eq!(
            runtime_summary["comparison_value"]["text"],
            json!("1234,567.89")
        );
        assert_eq!(
            execution_context["execution_phase"],
            json!("post_capture_trusted_refresh")
        );
        assert_eq!(
            execution_context["trusted_excel_separator_context"]["thousands_separator"],
            json!("\u{00A0}")
        );
        assert_eq!(
            execution_context["locale_query_bundle"]["thousands_separator"],
            json!("\u{00A0}")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_uses_captured_render_context_for_equal_locale_sensitive_text_case() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "FTC-1028".to_string(),
                entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            captured_cell_value: Some(json!({"kind":"text","text":"July"})),
            captured_value_repr: Some("July".to_string()),
            captured_formula_text: Some("=TEXT(DATE(2024,7,1),\"MMMM\")".to_string()),
            captured_effective_display_text: Some("July".to_string()),
            captured_render_context_json: Some(sample_oxxlplay_render_context_artifact()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];
        let case_input = read_json_file(
            output_root
                .join("cases")
                .join("FTC-1028")
                .join("case-input.json"),
        )
        .expect("case input");

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(case_report.value_match, Some(true));
        assert_eq!(
            case_report
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_source.as_deref()),
            Some("oxxlplay_render_context_capture")
        );
        assert_eq!(case_input["excel_render_context"]["trusted"], json!(true));
        assert_eq!(
            case_input["excel_render_context"]["provenance"]["kind"],
            json!("oxxlplay_capture_artifact")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_separator_context_for_locale_sensitive_text_cases() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-separator-context".to_string(),
                entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: Some(ProgrammaticFormattingContext {
                    format_profile_id: Some("en-US".to_string()),
                    number_format_code: Some("#,##0.00".to_string()),
                    date1904: Some(false),
                }),
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner::default();

        run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        let case_dir = output_root.join("cases").join("case-separator-context");
        let case_input: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("case-input.json")).expect("case input json"),
        )
        .expect("case input parse");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");
        let projection = read_json_file(case_dir.join("oxfml-v1-replay-projection.json"))
            .expect("projection json");

        assert_eq!(
            case_input["excel_render_context"]["render_locale_pinned"],
            json!(false)
        );
        assert_eq!(
            case_input["excel_render_context"]["requested_format_profile_id"],
            json!("en-US")
        );
        assert_eq!(
            scenario["excel_render_context"]["render_locale_source"],
            json!("observation_machine_default")
        );
        assert_eq!(
            projection["verification_publication_surface"]["number_format_code"],
            json!("#,##0.00")
        );
        assert_eq!(
            projection["verification_publication_surface"]["locale_format_context"]
                ["decimal_separator"],
            json!(".")
        );
        assert_eq!(
            projection["verification_publication_surface"]["locale_format_context"]
                ["thousands_separator"],
            json!(",")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_emits_programmatic_display_scope_when_formatting_context_present() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-formula-display".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: Some(sample_programmatic_formatting_context()),
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner::default();

        run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(
                output_root
                    .join("cases")
                    .join("case-formula-display")
                    .join("scenario.json"),
            )
            .expect("scenario json"),
        )
        .expect("scenario parse");
        assert_eq!(
            scenario["requested_observation_scope"]["oxxlplay_required_surfaces"],
            json!(["cell_value", "effective_display_text"])
        );
        assert!(scenario["observable_surfaces"]
            .as_array()
            .expect("observable surfaces")
            .iter()
            .any(|surface| surface["surface_kind"] == "effective_display_text"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_marks_capture_failure_as_blocked() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            capture_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(
            report.case_reports[0].comparison_status,
            ProgrammaticComparisonStatus::Blocked
        );
        assert_eq!(
            report.case_reports[0].artifact_catalog_entry.open_mode_hint,
            crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_supplies_default_locale_context_without_forcing_display_comparison() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-default-display".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: true,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(case_report.value_match, Some(true));
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, Some(true));
        assert_eq!(case_report.oxfml_summary.blocked_reason, None);
        assert_eq!(case_report.oxfml_summary.effective_display_summary, None);

        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(
                output_root
                    .join("cases")
                    .join("case-default-display")
                    .join("scenario.json"),
            )
            .expect("scenario json"),
        )
        .expect("scenario parse");
        assert_eq!(
            scenario["requested_observation_scope"]["oxxlplay_required_surfaces"],
            json!(["cell_value"])
        );
        assert!(!scenario["observable_surfaces"]
            .as_array()
            .expect("observable surfaces")
            .iter()
            .any(|surface| surface["surface_kind"] == "effective_display_text"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_spreadsheetml_scope_for_xml_backed_cases() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let xml_path = temp_root.join("source.xml");
        let output_root = temp_root.join("bundle");
        fs::create_dir_all(&temp_root).expect("temp root");
        fs::write(
            &xml_path,
            r##"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:x="urn:schemas-microsoft-com:office:excel">
  <Styles>
    <Style ss:ID="calc">
      <NumberFormat ss:Format="$#,##0.00"/>
      <Font ss:Color="#112233"/>
      <Interior ss:Color="#445566"/>
    </Style>
  </Styles>
  <Worksheet ss:Name="Input">
    <Table>
      <Row>
        <Cell ss:StyleID="calc" ss:Formula="=SUM(1,2,3)"><Data ss:Type="Number">0</Data></Cell>
      </Row>
    </Table>
    <ConditionalFormatting ss:Range="A1">
      <Condition ss:Type="Expression" ss:Formula="=A1>0"/>
      <Font ss:Color="#FF0000"/>
      <Interior ss:Color="#00FF00"/>
    </ConditionalFormatting>
  </Worksheet>
</Workbook>"##,
        )
        .expect("xml write");

        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-xml".to_string(),
                entered_cell_text: String::new(),
                spreadsheet_xml_source: Some(
                    crate::services::programmatic_testing::ProgrammaticSpreadsheetXmlSource {
                        workbook_path: xml_path.to_string_lossy().into_owned(),
                        locator: "Input!A1".to_string(),
                    },
                ),
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: false,
            diff_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(report.case_reports.len(), 1);
        assert_eq!(report.case_reports[0].entered_cell_text, "=SUM(1,2,3)");
        assert!(report.case_reports[0].spreadsheet_xml_extraction.is_some());
        assert!(report.case_reports[0].upstream_gap_report.is_some());
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("xml-cell-extract.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("required-observation-scope.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("upstream-gap-report.json")
            .is_file());
        let case_dir = output_root.join("cases").join("case-xml");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");
        assert_eq!(scenario["workbook_kind"], "spreadsheetml-2003-import");
        assert!(scenario.get("entered_cell_text").is_none());
        assert_eq!(
            scenario["requested_observation_scope"]["oxxlplay_required_surfaces"],
            json!([
                "formula_text",
                "cell_value",
                "effective_display_text",
                "number_format_code",
                "style_id",
                "font_color",
                "fill_color",
                "conditional_formatting_rules",
                "conditional_formatting_effective_style"
            ])
        );
        assert!(case_dir.join("workbook.xml").is_file());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn context_free_programmatic_formula_observation_scope_omits_display_surface() {
        let scope = programmatic_formula_observation_scope_without_display();

        assert_eq!(
            scope.oxfml_required_scope,
            vec![
                "entered_cell_text".to_string(),
                "returned_value_surface".to_string(),
            ]
        );
        assert_eq!(
            scope.oxxlplay_required_surfaces,
            vec!["cell_value".to_string()]
        );
        assert_eq!(
            scope.oxreplay_required_views,
            vec![
                "execution_outcome".to_string(),
                "comparison_value".to_string()
            ]
        );
    }

    #[test]
    fn explicit_programmatic_formatting_context_requests_display_surface() {
        let scope = programmatic_formula_observation_scope_with_display();

        assert_eq!(
            scope.oxfml_required_scope,
            vec![
                "entered_cell_text".to_string(),
                "returned_value_surface".to_string(),
                "format_profile".to_string(),
                "date1904".to_string(),
                "number_format_code".to_string(),
                "effective_display_text".to_string(),
            ]
        );
        assert_eq!(
            scope.oxxlplay_required_surfaces,
            vec![
                "cell_value".to_string(),
                "effective_display_text".to_string(),
            ]
        );
        assert_eq!(
            scope.oxreplay_required_views,
            vec![
                "execution_outcome".to_string(),
                "comparison_value".to_string(),
                "effective_display_text".to_string(),
            ]
        );
    }

    #[test]
    fn default_programmatic_context_does_not_enable_display_comparison() {
        let case = ProgrammaticFormulaCase {
            case_id: "case-default-context".to_string(),
            entered_cell_text: "=SUM(1,2,3)".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };

        assert!(!programmatic_display_comparison_enabled(&case, None));
        assert!(!programmatic_effective_display_surface_requested(
            &case, None
        ));
        assert_eq!(
            effective_requested_observation_scope(&case, None).oxxlplay_required_surfaces,
            vec!["cell_value".to_string()]
        );
    }

    #[test]
    fn explicit_number_format_context_enables_display_comparison() {
        let case = ProgrammaticFormulaCase {
            case_id: "case-explicit-display".to_string(),
            entered_cell_text: "=SUM(1,2,3)".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(sample_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };

        assert!(programmatic_display_comparison_enabled(&case, None));
        assert!(programmatic_effective_display_surface_requested(
            &case, None
        ));
        assert_eq!(
            effective_requested_observation_scope(&case, None).oxxlplay_required_surfaces,
            vec![
                "cell_value".to_string(),
                "effective_display_text".to_string()
            ]
        );
    }

    #[test]
    fn verification_batch_blocks_on_replay_validate_bundle_failure() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            validate_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Blocked
        );
        assert_eq!(case_report.value_match, None);
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, None);
        assert_eq!(case_report.replay_mismatch_records.len(), 0);
        assert!(case_report
            .discrepancy_summary
            .as_deref()
            .is_some_and(|summary| summary
                .contains("Comparison blocked: OxReplay validate-bundle failed (exit code 1)")));
        assert_eq!(
            runner.calls.lock().expect("calls").clone(),
            vec![
                "oxxlplay_capture_batch".to_string(),
                "validate_bundle".to_string()
            ]
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn discrepancy_summary_combines_display_divergence_and_projection_gaps() {
        let summary = build_discrepancy_summary(
            ProgrammaticComparisonStatus::Mismatched,
            Some(true),
            Some(false),
            &[
                OxReplayMismatchRecord {
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `formatting_view` is missing on one side"
                            .to_string(),
                    ),
                },
            ],
            &OxfmlVerificationSummary {
                evaluation_summary: Some("Number · 6".to_string()),
                comparison_value: Some(json!(6)),
                effective_display_summary: Some("6".to_string()),
                blocked_reason: None,
                parse_status: Some("Valid".to_string()),
                green_tree_key: Some("green-1".to_string()),
            },
            Some(&ExcelObservationSummary {
                comparison_value: Some(json!(6)),
                observed_value_repr: Some("$6.00".to_string()),
                effective_display_text: Some("$6.00".to_string()),
                observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
                capture_status: "captured".to_string(),
                render_locale_pinned: None,
                render_locale_source: None,
                render_locale_note: None,
            }),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Display divergence: OxFml 6 vs Excel $6.00 | Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side")
        );
    }

    #[test]
    fn parse_oxreplay_mismatch_records_keeps_view_family_and_values() {
        let diff_report = json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "formatting_view",
                    "right_value": { "number_format_code": "$#,##0.00" },
                    "detail": "comparison view family `formatting_view` is missing on one side"
                }
            ]
        });

        let records = parse_oxreplay_mismatch_records(&diff_report);

        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(records[0].left_value_repr.as_deref(), Some("6"));
        assert_eq!(records[0].right_value_repr.as_deref(), Some("$6.00"));
        assert_eq!(records[1].view_family.as_deref(), Some("formatting_view"));
        assert_eq!(
            records[1].right_value_repr.as_deref(),
            Some("{\"number_format_code\":\"$#,##0.00\"}")
        );
    }

    #[test]
    fn parse_oxreplay_explain_records_keeps_machine_readable_family_shape() {
        let explain_stdout = serde_json::to_string(&json!({
            "records": [
                {
                    "query_id": "explain-01",
                    "summary": "comparison diverged on `effective_display_text`",
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "query_id": "explain-02",
                    "summary": "comparison view family `conditional_formatting_view` is missing on one side",
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "conditional_formatting_view",
                    "right_value": [{ "range": "A1" }],
                    "detail": "comparison view family `conditional_formatting_view` is missing on one side"
                }
            ]
        }))
        .expect("json text");

        let records = parse_oxreplay_explain_records(&explain_stdout).expect("explain records");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].query_id.as_deref(), Some("explain-01"));
        assert_eq!(
            records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(
            records[1].view_family.as_deref(),
            Some("conditional_formatting_view")
        );
        assert_eq!(
            records[1].right_value_repr.as_deref(),
            Some("[{\"range\":\"A1\"}]")
        );
    }

    #[test]
    fn summarize_excel_capture_reads_effective_display_text_when_present() {
        let path = std::env::temp_dir().join(format!(
            "dnaonecalc-capture-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));

        write_json_file(
            &path,
            &json!({
                "surfaces": [
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_value",
                            "surface_kind": "cell_value",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "comparison_value": 6,
                        "value_repr": "6",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    },
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_display",
                            "surface_kind": "effective_display_text",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "value_repr": "$6.00",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    }
                ]
            }),
        )
        .expect("capture json");

        let summary = summarize_excel_capture(path.clone()).expect("capture summary");

        assert_eq!(summary.observed_value_repr.as_deref(), Some("6"));
        assert_eq!(summary.comparison_value, Some(json!(6)));
        assert_eq!(summary.effective_display_text.as_deref(), Some("$6.00"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn summarize_excel_capture_normalizes_published_formula_result_wrapper() {
        let path = std::env::temp_dir().join(format!(
            "dnaonecalc-capture-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));

        write_json_file(
            &path,
            &json!({
                "surfaces": [
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_value",
                            "surface_kind": "cell_value",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "comparison_value": {
                            "boundary": "published_formula_result",
                            "value": {
                                "kind": "logical",
                                "logical": false
                            },
                            "wire_schema": "oxfunc_value_types.aligned_json.v1"
                        },
                        "value_repr": "FALSE",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    }
                ]
            }),
        )
        .expect("capture json");

        let summary = summarize_excel_capture(path.clone()).expect("capture summary");

        assert_eq!(
            summary.comparison_value,
            Some(json!({
                "kind": "logical",
                "logical": false
            }))
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn materialize_synthetic_compare_ready_replay_includes_scenario_identity() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-synthetic-compare-ready-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_path = temp_root.join("normalized-replay.compare-ready.json");

        let replay_path = materialize_synthetic_compare_ready_replay(
            &output_path,
            "FTC-0448",
            &normalized_pre_execution_rejection_outcome(),
        )
        .expect("synthetic compare-ready replay");
        let replay = read_json_file(replay_path).expect("synthetic replay json");

        assert_eq!(replay["scenario_id"], json!("onecalc_verify_FTC-0448"));
        assert_eq!(replay["lane_id"], json!("synthetic-host-observation"));
        assert_eq!(
            projection_comparison_value(&replay, EXECUTION_OUTCOME_VIEW_FAMILY),
            Some(normalized_pre_execution_rejection_outcome())
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn materialize_synthetic_compare_ready_projection_includes_projection_identity() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-synthetic-projection-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_path = temp_root.join("oxfml-v1-replay-projection.compare-ready.json");
        let case = ProgrammaticFormulaCase {
            case_id: "FTC-0916".to_string(),
            entered_cell_text: "=((((((((((((((((1+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)+1)"
                .to_string(),
            spreadsheet_xml_source: None,
            formatting_context: None,
            excel_render_context: None,
            render_context_ref: None,
        };

        let projection_path = materialize_synthetic_compare_ready_projection(
            &output_path,
            &case,
            &normalized_pre_execution_rejection_outcome(),
        )
        .expect("synthetic compare-ready projection");
        let projection = read_json_file(projection_path).expect("synthetic projection json");

        assert_eq!(
            projection["source_artifact_family"],
            json!("runtime_formula_result")
        );
        assert_eq!(projection["source_case_id"], json!("FTC-0916"));
        assert_eq!(projection["commit_decision_kind"], json!("rejected"));
        assert_eq!(
            projection["shared_scenario_alias"],
            json!("onecalc_verify_FTC-0916")
        );
        assert_eq!(
            projection_comparison_value(&projection, EXECUTION_OUTCOME_VIEW_FAMILY),
            Some(normalized_pre_execution_rejection_outcome())
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn materialize_compare_ready_normalized_replay_normalizes_comparison_value_wrapper() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-compare-ready-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let input_path = temp_root.join("normalized-replay.json");
        let output_path = temp_root.join("normalized-replay.compare-ready.json");
        write_json_file(
            &input_path,
            &json!({
                "comparison_views": [
                    {
                        "view_family": "comparison_value",
                        "value": {
                            "boundary": "published_formula_result",
                            "value": {
                                "kind": "number",
                                "number": 55.0
                            },
                            "wire_schema": "oxfunc_value_types.aligned_json.v1"
                        }
                    },
                    {
                        "view_family": "effective_display_text",
                        "value": "55"
                    }
                ]
            }),
        )
        .expect("input replay json");

        let compare_ready_path = materialize_compare_ready_normalized_replay(
            &input_path,
            &output_path,
            &vec![
                EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
                "comparison_value".to_string(),
                "effective_display_text".to_string(),
            ],
            &normalized_completed_execution_outcome(),
        )
        .expect("compare-ready replay");
        let compare_ready = read_json_file(compare_ready_path).expect("compare-ready json");

        assert_eq!(
            projection_comparison_value(&compare_ready, "comparison_value"),
            Some(json!({
                "kind": "number",
                "number": "55.0"
            }))
        );
        assert_eq!(
            projection_comparison_value(&compare_ready, "effective_display_text"),
            Some(json!("55"))
        );
        assert_eq!(
            projection_comparison_value(&compare_ready, EXECUTION_OUTCOME_VIEW_FAMILY),
            Some(normalized_completed_execution_outcome())
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn materialize_compare_ready_normalized_replay_preserves_raw_excel_numeric_lexeme() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-compare-ready-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let input_path = temp_root.join("normalized-replay.json");
        let output_path = temp_root.join("normalized-replay.compare-ready.json");
        fs::create_dir_all(&temp_root).expect("temp root");
        fs::write(
            &input_path,
            r#"{
  "comparison_views": [
    {
      "view_family": "comparison_value",
      "value": {
        "wire_schema": "oxfunc_value_types.aligned_json.v1",
        "boundary": "published_formula_result",
        "value": {
          "kind": "number",
          "number": -240.30991269094474
        }
      }
    }
  ]
}"#,
        )
        .expect("input replay json");

        let compare_ready_path = materialize_compare_ready_normalized_replay(
            &input_path,
            &output_path,
            &vec![
                EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
                "comparison_value".to_string(),
            ],
            &normalized_completed_execution_outcome(),
        )
        .expect("compare-ready replay");
        let compare_ready = read_json_file(compare_ready_path).expect("compare-ready json");

        assert_eq!(
            projection_comparison_value(&compare_ready, "comparison_value"),
            Some(json!({
                "kind": "number",
                "number": "-240.30991269094474"
            }))
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn materialize_compare_ready_projection_preserves_raw_oxfml_numeric_lexeme() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-compare-ready-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let input_path = temp_root.join("oxfml-v1-replay-projection.json");
        let output_path = temp_root.join("oxfml-v1-replay-projection.compare-ready.json");
        fs::create_dir_all(&temp_root).expect("temp root");
        fs::write(
            &input_path,
            r#"{
  "comparison_views": [
    {
      "view_family": "comparison_value",
      "value": {
        "kind": "number",
        "number": 12599.999999999995
      }
    },
    {
      "view_family": "execution_outcome",
      "value": {
        "class_id": "executed_result",
        "outcome_kind": "executed_result",
        "outcome_stage": "executed"
      }
    }
  ]
}"#,
        )
        .expect("input projection json");

        let compare_ready_path = materialize_compare_ready_projection(
            &input_path,
            &output_path,
            &vec![
                EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
                "comparison_value".to_string(),
            ],
            &normalized_completed_execution_outcome(),
        )
        .expect("compare-ready projection");
        let compare_ready = read_json_file(compare_ready_path).expect("compare-ready json");

        assert_eq!(
            projection_comparison_value(&compare_ready, "comparison_value"),
            Some(json!({
                "kind": "number",
                "number": "12599.999999999995"
            }))
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn materialize_compare_ready_normalized_replay_canonicalizes_flat_column_array_payload() {
        let temp_root = std::env::temp_dir().join(format!(
            "dnaonecalc-compare-ready-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let input_path = temp_root.join("normalized-replay.json");
        let output_path = temp_root.join("normalized-replay.compare-ready.json");
        write_json_file(
            &input_path,
            &json!({
                "comparison_views": [
                    {
                        "view_family": "comparison_value",
                        "value": {
                            "kind": "array",
                            "shape": { "rows": 3, "cols": 1 },
                            "cells": [
                                { "kind": "number", "number": 3.0 },
                                { "kind": "number", "number": 3.0 },
                                { "kind": "number", "number": 1.0 }
                            ]
                        }
                    }
                ]
            }),
        )
        .expect("input replay json");

        let compare_ready_path = materialize_compare_ready_normalized_replay(
            &input_path,
            &output_path,
            &vec![
                EXECUTION_OUTCOME_VIEW_FAMILY.to_string(),
                "comparison_value".to_string(),
            ],
            &normalized_completed_execution_outcome(),
        )
        .expect("compare-ready replay");
        let compare_ready = read_json_file(compare_ready_path).expect("compare-ready json");

        assert_eq!(
            compare_ready["comparison_views"][0]["value"],
            json!({
                "kind": "array",
                "shape": { "rows": 3, "cols": 1 },
                "cells": [
                    [{ "kind": "number", "number": "3.0" }],
                    [{ "kind": "number", "number": "3.0" }],
                    [{ "kind": "number", "number": "1.0" }]
                ]
            })
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn normalize_comparison_value_coalesces_logical_aliases() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "logical",
                "value": true
            })),
            json!({
                "kind": "logical",
                "logical": true
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "boundary": "published_formula_result",
                "value": {
                    "kind": "logical",
                    "logical": false
                },
                "wire_schema": "oxfunc_value_types.aligned_json.v1"
            })),
            json!({
                "kind": "logical",
                "logical": false
            })
        );
    }

    #[test]
    fn normalize_comparison_value_coalesces_nested_number_and_logical_aliases() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "number",
                "value": {
                    "number": 42.5
                }
            })),
            json!({
                "kind": "number",
                "number": 42.5
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "boundary": "published_formula_result",
                "value": {
                    "kind": "logical",
                    "value": {
                        "logical": false
                    }
                },
                "wire_schema": "oxfunc_value_types.aligned_json.v1"
            })),
            json!({
                "kind": "logical",
                "logical": false
            })
        );
    }

    #[test]
    fn normalize_comparison_value_decodes_aligned_text_utf16_payloads() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "text",
                "utf16_code_units": [72, 101, 108, 108, 111]
            })),
            json!({
                "kind": "text",
                "text": "Hello"
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "boundary": "published_formula_result",
                "value": {
                    "kind": "text",
                    "value": {
                        "utf16_code_units": [78, 47, 65]
                    }
                },
                "wire_schema": "oxfunc_value_types.aligned_json.v1"
            })),
            json!({
                "kind": "text",
                "text": "N/A"
            })
        );
    }

    #[test]
    fn normalize_comparison_value_coalesces_nested_text_and_error_aliases() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "text",
                "value": {
                    "text": "Hello"
                }
            })),
            json!({
                "kind": "text",
                "text": "Hello"
            })
        );
        // Wire-form error code (`#VALUE!`) canonicalizes through
        // `normalize_error_code_alias` to the PascalCase canonical name
        // (`Value`); see `normalize_comparison_value_canonicalizes_error_code_case_aliases`
        // for the inverse case (`na` -> `NA`).
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "error",
                "value": {
                    "code": "#VALUE!"
                }
            })),
            json!({
                "kind": "error",
                "code": "Value"
            })
        );
    }

    #[test]
    fn normalize_comparison_value_coalesces_aligned_error_aliases() {
        // Wire-form `#N/A` and `#DIV/0!` canonicalize through
        // `normalize_error_code_alias` to their PascalCase canonical names
        // (`NA`, `Div0`).
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "error",
                "worksheet_error_code": "#N/A"
            })),
            json!({
                "kind": "error",
                "code": "NA"
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "boundary": "published_formula_result",
                "value": {
                    "kind": "error",
                    "value": {
                        "worksheet_error_code": "#DIV/0!"
                    }
                },
                "wire_schema": "oxfunc_value_types.aligned_json.v1"
            })),
            json!({
                "kind": "error",
                "code": "Div0"
            })
        );
    }

    #[test]
    fn normalize_comparison_value_canonicalizes_error_code_case_aliases() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "error",
                "code": "NA"
            })),
            json!({
                "kind": "error",
                "code": "NA"
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "error",
                "worksheet_error_code": "na"
            })),
            json!({
                "kind": "error",
                "code": "NA"
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "value_kind": "error",
                "payload": "div0"
            })),
            json!({
                "kind": "error",
                "code": "Div0"
            })
        );
    }

    #[test]
    fn normalize_comparison_value_normalizes_nested_array_text_and_error_cells() {
        assert_eq!(
            normalize_comparison_value(&json!({
                "kind": "array",
                "shape": {
                    "rows": 1,
                    "cols": 2
                },
                "cells": [
                    {
                        "kind": "text",
                        "utf16_code_units": [79, 75]
                    },
                    {
                        "kind": "error",
                        "worksheet_error_code": "na"
                    }
                ]
            })),
            json!({
                "kind": "array",
                "shape": {
                    "rows": 1,
                    "cols": 2
                },
                "cells": [
                    {
                        "kind": "text",
                        "text": "OK"
                    },
                    {
                        "kind": "error",
                        "code": "NA"
                    }
                ]
            })
        );
        assert_eq!(
            normalize_comparison_value(&json!({
                "value_kind": "array",
                "rows": 1,
                "cols": 1,
                "payload": [
                    {
                        "value_kind": "text",
                        "payload": {
                            "utf16_code_units": [79, 75]
                        }
                    }
                ]
            })),
            json!({
                "kind": "array",
                "shape": {
                    "rows": 1,
                    "cols": 1
                },
                "cells": [
                    {
                        "kind": "text",
                        "text": "OK"
                    }
                ]
            })
        );
    }

    #[test]
    fn summarize_excel_capture_normalizes_aligned_text_and_error_payloads() {
        let path = std::env::temp_dir().join(format!(
            "dnaonecalc-capture-aligned-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));

        write_json_file(
            &path,
            &json!({
                "surfaces": [
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_value",
                            "surface_kind": "cell_value",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "comparison_value": {
                            "kind": "text",
                            "utf16_code_units": [79, 75]
                        },
                        "value_repr": "OK",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    }
                ]
            }),
        )
        .expect("capture json");

        let summary = summarize_excel_capture(path.clone()).expect("capture summary");
        assert_eq!(
            summary.comparison_value,
            Some(json!({
                "kind": "text",
                "text": "OK"
            }))
        );

        write_json_file(
            &path,
            &json!({
                "surfaces": [
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_value",
                            "surface_kind": "cell_value",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "comparison_value": {
                            "kind": "error",
                            "worksheet_error_code": "#N/A"
                        },
                        "value_repr": "#N/A",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    }
                ]
            }),
        )
        .expect("error capture json");

        let error_summary = summarize_excel_capture(path.clone()).expect("error capture summary");
        // Wire-form `#N/A` canonicalizes to PascalCase canonical name `NA`
        // via normalize_error_code_alias.
        assert_eq!(
            error_summary.comparison_value,
            Some(json!({
                "kind": "error",
                "code": "NA"
            }))
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn default_programmatic_lane_filters_stale_visible_value_projection_gap() {
        let requested_views =
            programmatic_formula_observation_scope_with_display().oxreplay_required_views;
        let mismatch_records = filter_replay_mismatch_records_to_requested_views(
            vec![
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("visible_value_text".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `visible_value_text` is missing on one side"
                            .to_string(),
                    ),
                },
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `effective_display_text` is missing on one side"
                            .to_string(),
                    ),
                },
            ],
            &requested_views,
        );
        let explain_records = filter_replay_explain_records_to_requested_views(
            vec![
                OxReplayExplainRecord {
                    query_id: Some("q-visible".to_string()),
                    summary: Some(
                        "comparison view family `visible_value_text` is missing on one side"
                            .to_string(),
                    ),
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("visible_value_text".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `visible_value_text` is missing on one side"
                            .to_string(),
                    ),
                },
                OxReplayExplainRecord {
                    query_id: Some("q-display".to_string()),
                    summary: Some(
                        "comparison view family `effective_display_text` is missing on one side"
                            .to_string(),
                    ),
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `effective_display_text` is missing on one side"
                            .to_string(),
                    ),
                },
            ],
            &requested_views,
        );

        assert_eq!(mismatch_records.len(), 1);
        assert_eq!(
            mismatch_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(explain_records.len(), 1);
        assert_eq!(
            explain_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
    }

    #[test]
    fn preferred_excel_display_repr_uses_effective_display_text_before_observed_value() {
        let summary = ExcelObservationSummary {
            comparison_value: Some(json!(6)),
            observed_value_repr: Some("6".to_string()),
            effective_display_text: Some("$6.00".to_string()),
            observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
            capture_status: "captured".to_string(),
            render_locale_pinned: None,
            render_locale_source: None,
            render_locale_note: None,
        };

        assert_eq!(preferred_excel_display_repr(&summary), Some("$6.00"));
    }

    #[test]
    fn annotate_excel_observation_render_context_marks_programmatic_formula_as_unpinned() {
        let case = ProgrammaticFormulaCase {
            case_id: "case-text".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };
        let mut summary = ExcelObservationSummary {
            comparison_value: Some(json!({"kind":"text","text":"1234,567.89"})),
            observed_value_repr: Some("1234,567.89".to_string()),
            effective_display_text: None,
            observed_formula_repr: Some(case.entered_cell_text.clone()),
            capture_status: "captured".to_string(),
            render_locale_pinned: None,
            render_locale_source: None,
            render_locale_note: None,
        };

        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        annotate_excel_observation_render_context(&effective_excel_render_context, &mut summary);

        assert_eq!(summary.render_locale_pinned, Some(false));
        assert_eq!(
            summary.render_locale_source.as_deref(),
            Some("observation_machine_default")
        );
        assert!(summary
            .render_locale_note
            .as_deref()
            .expect("render locale note")
            .contains("does not carry any Excel-side locale pin"));
    }

    #[test]
    fn verification_batch_blocks_separator_sensitive_text_mismatch_shape_for_0288() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "FTC-0288".to_string(),
                entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            captured_cell_value: Some(json!({
                "kind": "text",
                "text": "1234,567.89"
            })),
            captured_value_repr: Some("1234,567.89".to_string()),
            captured_formula_text: Some("=TEXT(1234567.89,\"#,##0.00\")".to_string()),
            captured_effective_display_text: Some("1234,567.89".to_string()),
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_report = &report.case_reports[0];

        assert_eq!(
            case_report.comparison_status,
            ProgrammaticComparisonStatus::Blocked
        );
        assert_eq!(
            case_report.oxfml_summary.comparison_value,
            Some(json!({
                "kind": "text",
                "text": "1,234,567.89"
            }))
        );
        assert_eq!(
            case_report
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.comparison_value.clone()),
            Some(json!({
                "kind": "text",
                "text": "1234,567.89"
            }))
        );
        assert_eq!(case_report.value_match, None);
        assert_eq!(case_report.display_match, None);
        assert_eq!(case_report.replay_equivalent, None);
        assert!(case_report
            .discrepancy_summary
            .as_deref()
            .is_some_and(|summary| summary.contains("locale-sensitive semantic text")));
        assert_eq!(
            case_report
                .excel_summary
                .as_ref()
                .and_then(|summary| summary.render_locale_source.as_deref()),
            Some("observation_machine_default")
        );
        assert_eq!(
            runner.calls.lock().expect("calls").clone(),
            vec!["oxxlplay_capture_batch".to_string()]
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible_for_0288_shape() {
        let case = ProgrammaticFormulaCase {
            case_id: "FTC-0288".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };
        let projection = json!({
            "comparison_views": [
                {
                    "view_family": "formatting_view",
                    "value": {
                        "format_dependency_facts": [
                            {
                                "dependency_class": "semantic_formatting",
                                "dependency_token": "locale_format_context"
                            }
                        ]
                    }
                }
            ]
        });

        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        assert!(
            locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
                &case,
                None,
                &effective_excel_render_context,
                &projection,
                Some(&json!({"kind":"text","text":"1,234,567.89"}))
            )
        );
    }

    #[test]
    fn locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible_for_untrusted_inline_context(
    ) {
        let case = ProgrammaticFormulaCase {
            case_id: "FTC-0288-untrusted".to_string(),
            entered_cell_text: "=TEXT(1234567.89,\"#,##0.00\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: Some(ProgrammaticExcelRenderContext {
                trusted: false,
                ..sample_trusted_excel_render_context()
            }),
            render_context_ref: None,
        };
        let projection = json!({
            "comparison_views": [
                {
                    "view_family": "formatting_view",
                    "value": {
                        "format_dependency_facts": [
                            {
                                "dependency_class": "semantic_formatting",
                                "dependency_token": "locale_format_context"
                            }
                        ]
                    }
                }
            ]
        });
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        assert!(
            locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
                &case,
                None,
                &effective_excel_render_context,
                &projection,
                Some(&json!({"kind":"text","text":"1,234,567.89"}))
            )
        );
    }

    #[test]
    fn locale_sensitive_programmatic_text_value_surface_allows_trusted_pinned_context() {
        let case = ProgrammaticFormulaCase {
            case_id: "FTC-1028-trusted".to_string(),
            entered_cell_text: "=TEXT(DATE(2024,7,1),\"MMMM\")".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: Some(sample_trusted_excel_render_context()),
            render_context_ref: None,
        };
        let projection = json!({
            "comparison_views": [
                {
                    "view_family": "formatting_view",
                    "value": {
                        "format_dependency_facts": [
                            {
                                "dependency_class": "semantic_formatting",
                                "dependency_token": "locale_format_context"
                            }
                        ]
                    }
                }
            ]
        });
        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        assert!(
            !locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
                &case,
                None,
                &effective_excel_render_context,
                &projection,
                Some(&json!({"kind":"text","text":"July"}))
            )
        );
    }

    #[test]
    fn should_not_block_numeric_mismatch_without_locale_sensitive_text_path() {
        let case = ProgrammaticFormulaCase {
            case_id: "FTC-0406".to_string(),
            entered_cell_text: "=PDURATION(0.05,1000,2000)".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: Some(default_programmatic_formatting_context()),
            excel_render_context: None,
            render_context_ref: None,
        };
        let projection = json!({
            "comparison_views": [
                {
                    "view_family": "formatting_view",
                    "value": {
                        "format_dependency_facts": []
                    }
                }
            ]
        });

        let effective_excel_render_context =
            resolve_effective_excel_render_context(&case, None, &BTreeMap::new())
                .expect("effective render context");

        assert!(
            !locale_sensitive_programmatic_text_value_surface_is_not_compare_eligible(
                &case,
                None,
                &effective_excel_render_context,
                &projection,
                Some(&json!({"kind":"number","number":14.206699082890463}))
            )
        );
    }

    #[test]
    fn host_verdict_uses_oxreplay_equivalence_output() {
        assert_eq!(
            derive_host_comparison_status_from_replay(true),
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(
            derive_host_comparison_status_from_replay(false),
            ProgrammaticComparisonStatus::Mismatched
        );
    }

    #[test]
    fn exact_numeric_mismatch_shape_is_consumed_from_oxreplay_records() {
        let mismatch_records = parse_oxreplay_mismatch_records(&json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "comparison_value",
                    "severity": "semantic",
                    "view_family": "comparison_value",
                    "left_value": { "kind": "number", "number": 14.206699082890463_f64 },
                    "right_value": { "kind": "number", "number": 14.206699082890465_f64 },
                    "detail": "typed comparison values diverged"
                }
            ]
        }));

        assert_eq!(
            derive_replay_axis_match(&mismatch_records, "comparison_value", true),
            Some(false)
        );
    }

    #[test]
    fn display_divergence_is_consumed_from_oxreplay_records() {
        let mismatch_records = parse_oxreplay_mismatch_records(&json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "$1,234.50",
                    "right_value": "1234.5",
                    "detail": "comparison view values diverged"
                }
            ]
        }));

        assert_eq!(
            derive_replay_axis_match(&mismatch_records, "effective_display_text", true),
            Some(false)
        );
    }

    #[test]
    fn typed_execution_outcome_equivalence_is_consumed_from_oxreplay() {
        let left = json!({
            "comparison_views": [
                {
                    "view_family": "execution_outcome",
                    "value": normalized_pre_execution_rejection_outcome()
                }
            ]
        });
        let right = json!({
            "comparison_views": [
                {
                    "view_family": "execution_outcome",
                    "value": normalized_pre_execution_rejection_outcome()
                }
            ]
        });

        let diff_report = fake_diff_report(&left, &right);
        assert_eq!(diff_report["equivalent"], json!(true));
        assert_eq!(
            derive_host_comparison_status_from_replay(
                diff_report["equivalent"].as_bool().expect("equivalent")
            ),
            ProgrammaticComparisonStatus::Matched
        );
    }

    #[test]
    fn missing_required_replay_view_remains_host_blocked_policy() {
        let left = json!({
            "comparison_views": [
                {
                    "view_family": "execution_outcome",
                    "value": normalized_completed_execution_outcome()
                },
                {
                    "view_family": "comparison_value",
                    "value": { "kind": "number", "number": 6.0 }
                }
            ]
        });
        let right = json!({
            "comparison_views": [
                {
                    "view_family": "execution_outcome",
                    "value": normalized_completed_execution_outcome()
                }
            ]
        });

        assert_eq!(
            missing_required_replay_view_reason(
                &left,
                &right,
                &[
                    "execution_outcome".to_string(),
                    "comparison_value".to_string()
                ]
            )
            .as_deref(),
            Some("Comparison blocked: required replay comparison view `comparison_value` was unavailable on Excel")
        );
    }

    #[test]
    fn host_test_format_engine_supports_grouped_currency_codes() {
        let rendered = HOST_TEST_FORMAT_CODE_ENGINE
            .render_with_code(
                &format_profile(LocaleProfileId::EnUs),
                WorkbookDateSystem::System1900,
                1234.5,
                "$#,##0.00",
            )
            .expect("currency render");

        assert_eq!(rendered.to_string_lossy(), "$1,234.50");
    }

    #[test]
    fn host_test_format_engine_supports_grouped_fixed_codes() {
        let rendered = HOST_TEST_FORMAT_CODE_ENGINE
            .render_with_code(
                &format_profile(LocaleProfileId::EnUs),
                WorkbookDateSystem::System1900,
                1234.5,
                "#,##0.00",
            )
            .expect("fixed render");

        assert_eq!(rendered.to_string_lossy(), "1,234.50");
    }

    #[test]
    fn host_test_format_engine_uses_negative_section_for_parenthesized_currency() {
        let rendered = HOST_TEST_FORMAT_CODE_ENGINE
            .render_with_code(
                &format_profile(LocaleProfileId::EnUs),
                WorkbookDateSystem::System1900,
                -1234.5,
                "$#,##0.00;($#,##0.00)",
            )
            .expect("negative currency render");

        assert_eq!(rendered.to_string_lossy(), "($1,234.50)");
    }

    #[test]
    fn verification_batch_prepares_value_only_compare_inputs_for_default_programmatic_formula_cases(
    ) {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            replay_policy: default_verification_replay_policy(),
            render_contexts: BTreeMap::new(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-compare-ready".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
        };
        let runner = FakeVerificationRunner {
            assert_compare_inputs_ready: true,
            diff_equivalent: false,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");
        let case_dir = output_root.join("cases").join("case-compare-ready");
        let projection = read_json_file(case_dir.join("oxfml-v1-replay-projection.json"))
            .expect("projection json");

        assert_eq!(report.case_reports.len(), 1);
        assert!(projection_comparison_value(&projection, "comparison_value").is_some());
        assert_eq!(
            projection_comparison_value(&projection, "effective_display_text"),
            None
        );
        assert_eq!(
            projection["verification_publication_surface"]["effective_display_text"],
            Value::Null
        );
        assert_eq!(
            projection["verification_publication_surface"]["format_profile"],
            Value::Null
        );
        assert_eq!(
            projection["verification_publication_surface"]["date1904"],
            Value::Null
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn normalize_programmatic_formula_case_supplies_default_host_context() {
        let normalized = normalize_programmatic_formula_case(&ProgrammaticFormulaCase {
            case_id: "case-default-context".to_string(),
            entered_cell_text: "=1+2".to_string(),
            spreadsheet_xml_source: None,
            formatting_context: None,
            excel_render_context: None,
            render_context_ref: None,
        });

        assert_eq!(
            normalized.formatting_context,
            Some(default_programmatic_formatting_context())
        );
    }
}

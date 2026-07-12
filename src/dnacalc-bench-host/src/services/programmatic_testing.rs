use std::fs;
use std::path::Path;

use roxmltree::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticSpreadsheetXmlSource {
    pub workbook_path: String,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticFormattingContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date1904: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticExcelRenderContext {
    pub render_locale_pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_locale_source: Option<String>,
    #[serde(default)]
    pub render_locale_recorded: bool,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_format_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thousands_separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticFormulaCase {
    pub case_id: String,
    pub entered_cell_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spreadsheet_xml_source: Option<ProgrammaticSpreadsheetXmlSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatting_context: Option<ProgrammaticFormattingContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excel_render_context: Option<ProgrammaticExcelRenderContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_context_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticHostProfile {
    pub profile_id: String,
    pub requires_excel_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticCapabilityProfile {
    pub host_summary: String,
    pub excel_observation_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammaticVerificationConfig {
    pub host_profile: ProgrammaticHostProfile,
    pub capabilities: ProgrammaticCapabilityProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticComparisonLane {
    OxfmlOnly,
    OxfmlAndExcel,
    ExcelObservationBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticBatchPlan {
    pub formula_count: usize,
    pub comparison_lane: ProgrammaticComparisonLane,
    pub discrepancy_index_required: bool,
    pub retained_artifact_kinds: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticComparisonStatus {
    Matched,
    Mismatched,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammaticOpenModeHint {
    Inspect,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammaticArtifactCatalogEntry {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
}

pub fn default_windows_excel_host_profile() -> ProgrammaticHostProfile {
    ProgrammaticHostProfile {
        profile_id: "windows_excel_default".to_string(),
        requires_excel_observation: true,
    }
}

pub fn default_windows_excel_capability_profile() -> ProgrammaticCapabilityProfile {
    ProgrammaticCapabilityProfile {
        host_summary: "windows_native_excel_default".to_string(),
        excel_observation_available: true,
    }
}

pub fn default_programmatic_corpus_formatting_context() -> ProgrammaticFormattingContext {
    ProgrammaticFormattingContext {
        format_profile_id: Some("en-US".to_string()),
        number_format_code: None,
        date1904: Some(false),
    }
}

pub fn default_verification_config() -> ProgrammaticVerificationConfig {
    ProgrammaticVerificationConfig {
        host_profile: default_windows_excel_host_profile(),
        capabilities: default_windows_excel_capability_profile(),
    }
}

pub fn load_verification_config_xml(
    config_path: impl AsRef<Path>,
) -> Result<ProgrammaticVerificationConfig, String> {
    let config_path = config_path.as_ref();
    let xml = fs::read_to_string(config_path).map_err(|error| {
        format!(
            "failed to read verification config XML from `{}`: {error}",
            config_path.display()
        )
    })?;
    let document = Document::parse(&xml).map_err(|error| {
        format!(
            "failed to parse verification config XML from `{}`: {error}",
            config_path.display()
        )
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "verification-config" {
        return Err(format!(
            "verification config XML `{}` must use a <verification-config> root element",
            config_path.display()
        ));
    }

    let host_node = root
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "host-profile");
    let capabilities_node = root
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "capabilities");

    let default = default_verification_config();
    let host_profile = if let Some(node) = host_node {
        ProgrammaticHostProfile {
            profile_id: node
                .attribute("profile-id")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default.host_profile.profile_id.clone()),
            requires_excel_observation: parse_bool_attr(
                node.attribute("requires-excel-observation"),
                default.host_profile.requires_excel_observation,
                "requires-excel-observation",
                config_path,
            )?,
        }
    } else {
        default.host_profile
    };

    let capabilities = if let Some(node) = capabilities_node {
        ProgrammaticCapabilityProfile {
            host_summary: node
                .attribute("host-summary")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default.capabilities.host_summary.clone()),
            excel_observation_available: parse_bool_attr(
                node.attribute("excel-observation-available"),
                default.capabilities.excel_observation_available,
                "excel-observation-available",
                config_path,
            )?,
        }
    } else {
        default.capabilities
    };

    Ok(ProgrammaticVerificationConfig {
        host_profile,
        capabilities,
    })
}

fn parse_bool_attr(
    value: Option<&str>,
    default: bool,
    attribute_name: &str,
    config_path: &Path,
) -> Result<bool, String> {
    match value {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(format!(
            "verification config XML `{}` has invalid boolean `{other}` for `{attribute_name}`; expected `true` or `false`",
            config_path.display()
        )),
    }
}

pub fn build_programmatic_batch_plan(
    cases: &[ProgrammaticFormulaCase],
    host_profile: &ProgrammaticHostProfile,
    capabilities: &ProgrammaticCapabilityProfile,
) -> ProgrammaticBatchPlan {
    let comparison_lane = match (
        host_profile.requires_excel_observation,
        capabilities.excel_observation_available,
    ) {
        (false, _) => ProgrammaticComparisonLane::OxfmlOnly,
        (true, true) => ProgrammaticComparisonLane::OxfmlAndExcel,
        (true, false) => ProgrammaticComparisonLane::ExcelObservationBlocked,
    };

    let mut retained_artifact_kinds = vec![
        "scenario_input".to_string(),
        "capability_context".to_string(),
        "run_result".to_string(),
        "replay_bundle".to_string(),
    ];
    match comparison_lane {
        ProgrammaticComparisonLane::OxfmlOnly => {}
        ProgrammaticComparisonLane::OxfmlAndExcel => {
            retained_artifact_kinds.push("oxxlplay_batch_manifest".to_string());
            retained_artifact_kinds.push("oxxlplay_batch_output_index".to_string());
            retained_artifact_kinds.push("comparison_outcome".to_string());
            retained_artifact_kinds.push("discrepancy_index".to_string());
        }
        ProgrammaticComparisonLane::ExcelObservationBlocked => {
            retained_artifact_kinds.push("comparison_blocked".to_string());
            retained_artifact_kinds.push("discrepancy_index".to_string());
        }
    }

    ProgrammaticBatchPlan {
        formula_count: cases.len(),
        discrepancy_index_required: matches!(
            comparison_lane,
            ProgrammaticComparisonLane::OxfmlAndExcel
                | ProgrammaticComparisonLane::ExcelObservationBlocked
        ),
        comparison_lane,
        retained_artifact_kinds,
    }
}

pub fn build_programmatic_artifact_catalog_entry(
    artifact_id: impl Into<String>,
    case_id: impl Into<String>,
    comparison_status: ProgrammaticComparisonStatus,
) -> ProgrammaticArtifactCatalogEntry {
    let open_mode_hint = match comparison_status {
        ProgrammaticComparisonStatus::Matched => ProgrammaticOpenModeHint::Inspect,
        ProgrammaticComparisonStatus::Mismatched | ProgrammaticComparisonStatus::Blocked => {
            ProgrammaticOpenModeHint::Workbench
        }
    };

    ProgrammaticArtifactCatalogEntry {
        artifact_id: artifact_id.into(),
        case_id: case_id.into(),
        comparison_status,
        open_mode_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_plan_uses_excel_lane_when_profile_requires_it_and_host_can_observe() {
        let plan = build_programmatic_batch_plan(
            &[ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2)".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
            &ProgrammaticHostProfile {
                profile_id: "windows-excel".to_string(),
                requires_excel_observation: true,
            },
            &ProgrammaticCapabilityProfile {
                host_summary: "windows-native".to_string(),
                excel_observation_available: true,
            },
        );

        assert_eq!(plan.formula_count, 1);
        assert_eq!(
            plan.comparison_lane,
            ProgrammaticComparisonLane::OxfmlAndExcel
        );
        assert!(plan.discrepancy_index_required);
        assert!(plan
            .retained_artifact_kinds
            .contains(&"comparison_outcome".to_string()));
    }

    #[test]
    fn batch_plan_marks_excel_observation_as_blocked_when_host_cannot_observe() {
        let plan = build_programmatic_batch_plan(
            &[ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "'123.4".to_string(),
                spreadsheet_xml_source: None,
                formatting_context: None,
                excel_render_context: None,
                render_context_ref: None,
            }],
            &ProgrammaticHostProfile {
                profile_id: "browser".to_string(),
                requires_excel_observation: true,
            },
            &ProgrammaticCapabilityProfile {
                host_summary: "browser-web".to_string(),
                excel_observation_available: false,
            },
        );

        assert_eq!(
            plan.comparison_lane,
            ProgrammaticComparisonLane::ExcelObservationBlocked
        );
        assert!(plan
            .retained_artifact_kinds
            .contains(&"comparison_blocked".to_string()));
    }

    #[test]
    fn mismatches_and_blocked_results_open_in_workbench() {
        let mismatch = build_programmatic_artifact_catalog_entry(
            "artifact-1",
            "case-1",
            ProgrammaticComparisonStatus::Mismatched,
        );
        let blocked = build_programmatic_artifact_catalog_entry(
            "artifact-2",
            "case-2",
            ProgrammaticComparisonStatus::Blocked,
        );
        let matched = build_programmatic_artifact_catalog_entry(
            "artifact-3",
            "case-3",
            ProgrammaticComparisonStatus::Matched,
        );

        assert_eq!(mismatch.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
        assert_eq!(blocked.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
        assert_eq!(matched.open_mode_hint, ProgrammaticOpenModeHint::Inspect);
    }

    #[test]
    fn load_verification_config_xml_reads_host_and_capability_overrides() {
        let temp_path = std::env::temp_dir().join(format!(
            "onecalc-verification-config-{}-{}.xml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        fs::write(
            &temp_path,
            r#"<verification-config>
  <host-profile profile-id="windows_excel_default" requires-excel-observation="true" />
  <capabilities host-summary="windows_native_excel_default" excel-observation-available="false" />
</verification-config>"#,
        )
        .expect("config write");

        let config = load_verification_config_xml(&temp_path).expect("config");

        assert_eq!(config.host_profile.profile_id, "windows_excel_default");
        assert!(config.host_profile.requires_excel_observation);
        assert_eq!(
            config.capabilities.host_summary,
            "windows_native_excel_default"
        );
        assert!(!config.capabilities.excel_observation_available);

        let _ = fs::remove_file(temp_path);
    }
}

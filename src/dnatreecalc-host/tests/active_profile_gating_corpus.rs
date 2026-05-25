use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{CapabilityProfileId, NodeContent, WorkspaceFixture, WorkspaceModel};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct ProfileTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<ProfileCase>,
}

#[derive(Debug, Deserialize)]
struct ProfileCase {
    id: String,
    kind: String,
    reference: String,
    profiles: Value,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

struct ProfileCaseRun {
    capability_profile_id: String,
    run_state: Option<oxcalc_core::consumer::OxCalcTreeRunState>,
    diagnostics: Vec<String>,
}

#[test]
fn active_profile_gating_corpus_is_direct_context_typed_pending() {
    let theme = load_theme(repo_corpus_path("profiles/gating.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "profiles/gating");
    assert_eq!(theme.status, CorpusStatus::Active);

    for case in &theme.cases {
        assert_eq!(case.kind, "profile", "{} kind", case.id);

        let treecalc = run_profile_case(case, CapabilityProfileId::TreecalcV1);
        assert_eq!(
            treecalc.capability_profile_id, "host-capabilities:treecalc-v1",
            "{} must enter OxCalc with the TreeCalc capability profile, got {:?}",
            case.id, treecalc.diagnostics
        );

        let strict = run_profile_case(case, CapabilityProfileId::StrictExcel);
        assert_eq!(
            strict.capability_profile_id, "host-capabilities:strict-excel",
            "{} must enter OxCalc with the strict Excel capability profile, got {:?}",
            case.id, strict.diagnostics
        );

        if strict_profile_verdict(case) == "reject" {
            assert!(
                strict.run_state != Some(oxcalc_core::consumer::OxCalcTreeRunState::Published)
                    || strict.diagnostics.iter().any(|diagnostic| diagnostic
                        .contains("typed_exclusion")
                        || diagnostic.contains("unresolved_host_name")
                        || diagnostic.contains("parse")
                        || diagnostic.contains("unsupported")
                        || diagnostic
                            .contains("oxfml_returned_value_surface_payload_summary:Error(Name)")),
                "{} strict-excel must not silently accept TreeCalc-only syntax, got state {:?} diagnostics {:?}",
                case.id,
                strict.run_state,
                strict.diagnostics
            );
        } else {
            assert!(
                strict.run_state == Some(oxcalc_core::consumer::OxCalcTreeRunState::Published)
                    || strict
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.contains("typed_exclusion")),
                "{} profile-agnostic case should either publish now or remain an explicit pending lane, got state {:?} diagnostics {:?}",
                case.id,
                strict.run_state,
                strict.diagnostics
            );
        }
    }
}

fn run_profile_case(case: &ProfileCase, profile: CapabilityProfileId) -> ProfileCaseRun {
    let mut workspace = load_workspace("accounts");
    workspace.profile = profile;
    let caller = "Accounts.2005.Q1.Income";
    blank_non_target_formula_nodes(&mut workspace, caller);
    workspace
        .nodes
        .get_mut(caller)
        .expect("profile caller fixture must exist")
        .content = NodeContent::Formula(format!("={}", case.reference));
    let mut session = TreeWorkspaceSession::from_model(&workspace)
        .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
    let capability_profile_id = session.capability_profile_id().to_string();
    match session.recalculate() {
        Ok(outcome) => ProfileCaseRun {
            capability_profile_id,
            run_state: Some(outcome.run_state),
            diagnostics: outcome.diagnostics,
        },
        Err(error) => ProfileCaseRun {
            capability_profile_id,
            run_state: None,
            diagnostics: vec![error.to_string()],
        },
    }
}

fn strict_profile_verdict(case: &ProfileCase) -> &str {
    match &case.profiles["strict-excel"] {
        Value::String(value) => value.as_str(),
        Value::Object(object) => object
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        _ => "unknown",
    }
}

fn blank_non_target_formula_nodes(workspace: &mut WorkspaceModel, caller: &str) {
    for (path, node) in &mut workspace.nodes {
        if path != caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
}

fn load_theme(path: PathBuf) -> ProfileTheme {
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

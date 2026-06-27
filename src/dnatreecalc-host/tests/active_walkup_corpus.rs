use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::NodeId;
use oxcalc_core::consumer::OxCalcTreeRunState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<WalkupCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct WalkupCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: WalkupExpectation,
}

#[derive(Debug, Deserialize)]
struct WalkupExpectation {
    outcome: String,
    target: Option<String>,
    calc: Option<String>,
}

#[test]
fn active_walkup_corpus_executes_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("references/walkup-raw-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/walkup-raw-active");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        prepare_walkup_case_workspace(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} failed through direct context: {error}", case.id));

        match (case.expect.outcome.as_str(), case.expect.calc.as_deref()) {
            ("resolved", Some("cycle")) => {
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Rejected,
                    "{} cycle run state",
                    case.id
                );
                assert_dependency_members(&session, &result, case);
            }
            ("resolved", _) => {
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Published,
                    "{} run state: reject={:?}; diagnostics={:?}",
                    case.id,
                    result.reject_detail,
                    result.diagnostics
                );
                assert_dependency_members(&session, &result, case);
            }
            ("unresolved", _) => {
                // Excel-faithful: a tree node is a defined name, so an unresolved
                // name commits with #NAME? (it is not rejected) while still
                // surfacing the unresolved-name diagnostic.
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Published,
                    "{} unresolved run state",
                    case.id
                );
                assert!(
                    result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.contains("unresolved identifier")),
                    "{} should report unresolved host name, got {:?}",
                    case.id,
                    result.diagnostics
                );
            }
            _ => panic!("{} has unsupported expectation {:?}", case.id, case.expect),
        }
    }
}

fn prepare_walkup_case_workspace(workspace: &mut WorkspaceModel, case: &WalkupCase) {
    for (path, node) in &mut workspace.nodes {
        if path != &case.caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
    if let Some(target) = case.expect.target.as_ref()
        && target != &case.caller
        && let Some(node) = workspace.nodes.get_mut(target)
        && matches!(node.content, NodeContent::Empty)
    {
        node.content = NodeContent::Constant("0".to_string());
    }
    workspace
        .nodes
        .get_mut(&case.caller)
        .unwrap_or_else(|| panic!("{} caller missing", case.id))
        .content = NodeContent::Formula(format!("={}", case.reference));
}

fn assert_dependency_members(
    session: &TreeWorkspaceSession,
    result: &oxcalc_core::consumer::OxCalcTreeCalculationOutcome,
    case: &WalkupCase,
) {
    let expected = case
        .expect
        .target
        .as_ref()
        .unwrap_or_else(|| panic!("{} resolved case needs target", case.id));
    assert_eq!(
        session
            .dependency_members_for(result, &NodeId::new(case.caller.clone()))
            .unwrap_or_else(|error| panic!("{} dependency projection failed: {error}", case.id))
            .iter()
            .map(|member| member.as_str().to_string())
            .collect::<Vec<_>>(),
        vec![expected.clone()],
        "{} dependency membership",
        case.id
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

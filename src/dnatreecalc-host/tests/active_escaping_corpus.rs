use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{NodeId, NodeValueProjection, WorkspaceState};
use oxcalc_core::consumer::OxCalcTreeRunState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<EscapingCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct EscapingCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: EscapingExpectation,
}

#[derive(Debug, Deserialize)]
struct EscapingExpectation {
    outcome: String,
    target: String,
    published_value: String,
}

#[test]
fn active_escaping_corpus_executes_bracket_escaped_paths_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("references/escaping-raw-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/escaping-raw-active");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        assert_eq!(case.expect.outcome, "resolved", "{} outcome", case.id);

        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        prepare_escaping_case_workspace(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} failed through direct context: {error}", case.id));
        let state = session.workspace_state().unwrap_or_else(|error| {
            panic!("{} failed to project direct context: {error}", case.id)
        });

        assert_eq!(
            result.run_state,
            OxCalcTreeRunState::Published,
            "{} run state: reject={:?}; diagnostics={:?}",
            case.id,
            result.reject_detail,
            result.diagnostics
        );
        assert_eq!(
            scalar_value(&state, &case.caller),
            Some(case.expect.published_value.as_str()),
            "{} published value",
            case.id
        );
        assert_eq!(
            session
                .dependency_members_for(&result, &NodeId::new(case.caller.clone()))
                .unwrap_or_else(|error| panic!("{} dependency projection failed: {error}", case.id))
                .iter()
                .map(|member| member.as_str().to_string())
                .collect::<Vec<_>>(),
            vec![case.expect.target.clone()],
            "{} dependency membership",
            case.id
        );
    }
}

fn prepare_escaping_case_workspace(workspace: &mut WorkspaceModel, case: &EscapingCase) {
    for (path, node) in &mut workspace.nodes {
        if path != &case.caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
    workspace
        .nodes
        .get_mut(&case.caller)
        .unwrap_or_else(|| panic!("{} caller missing", case.id))
        .content = NodeContent::Formula(format!("={}", case.reference));
}

fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id.to_string()))
        .and_then(|node| match &node.computed_value {
            NodeValueProjection::Scalar(value) => Some(value.as_str()),
            _ => None,
        })
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

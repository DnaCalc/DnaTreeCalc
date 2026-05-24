use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{NodeId, NodeValueProjection, WorkspaceState};
use oxcalc_core::consumer::OxCalcTreeRunState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<OrderedCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct OrderedCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: OrderedExpectation,
}

#[derive(Debug, Deserialize)]
struct OrderedExpectation {
    outcome: String,
    members: Vec<String>,
    published_value: String,
}

#[test]
fn active_raw_ordered_selector_corpus_executes_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("references/ordered-raw-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/ordered-raw-active");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert_eq!(case.kind, "membership", "case {} kind changed", case.id);
        assert_eq!(
            case.expect.outcome, "resolved",
            "case {} is outside the active ordered-selector success slice",
            case.id
        );
        assert!(
            matches!(
                case.reference.as_str(),
                "@PRECEDING"
                    | "@FOLLOWING"
                    | "@ANCESTORS"
                    | "Base.**.Margin"
                    | "Root.StructuralPreceding.Total.@PRECEDING"
                    | "Root.StructuralFollowing.Total.@FOLLOWING"
                    | "Root.StructuralRecursive.Base.**.Margin"
            ),
            "case {} activates unsupported reference {}",
            case.id,
            case.reference
        );

        let workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("case {} failed to build context: {error}", case.id));
        let result = session.recalculate().unwrap_or_else(|error| {
            panic!("case {} failed through direct context: {error}", case.id)
        });
        let state = session.workspace_state().unwrap_or_else(|error| {
            panic!("case {} failed to project direct context: {error}", case.id)
        });

        assert_eq!(
            result.run_state,
            OxCalcTreeRunState::Published,
            "{}",
            case.id
        );
        assert_eq!(
            scalar_value(&state, &case.caller),
            Some(case.expect.published_value.as_str()),
            "{} published value",
            case.id
        );
        let actual_members = session
            .dependency_members_for(&result, &NodeId::new(case.caller.clone()))
            .unwrap_or_else(|error| {
                panic!("case {} dependency projection failed: {error}", case.id)
            })
            .iter()
            .map(|member| member.as_str().to_string())
            .collect::<BTreeSet<_>>();
        let expected_members = case.expect.members.iter().cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            actual_members, expected_members,
            "{} dependency membership",
            case.id
        );
    }
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

fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id))
        .and_then(|node| match &node.computed_value {
            NodeValueProjection::Scalar(value) => Some(value.as_str()),
            _ => None,
        })
}

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

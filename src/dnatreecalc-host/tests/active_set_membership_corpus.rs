use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{NodeId, WorkspaceState};
use oxcalc_core::consumer::OxCalcTreeRunState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<SetMembershipCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct SetMembershipCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    source_formula: String,
    expect: SetMembershipExpectation,
}

#[derive(Debug, Deserialize)]
struct SetMembershipExpectation {
    outcome: String,
    ordered: Option<bool>,
    members: Vec<String>,
    published_value: String,
}

#[test]
fn active_raw_set_membership_corpus_executes_broad_selectors_through_direct_context() {
    let theme = load_theme(repo_corpus_path("references/set-membership-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/set-membership-active");
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
            "case {} is outside the admitted set-membership success slice",
            case.id
        );
        assert!(
            matches!(
                case.reference.as_str(),
                "Q1.*"
                    | "@CHILDREN"
                    | "@ANCESTORS"
                    | "@PRECEDING"
                    | "@FOLLOWING"
                    | "Accounts.2005.**.Margin"
                    | "Q2.**"
            ),
            "case {} activates unsupported set-membership reference {}",
            case.id,
            case.reference
        );

        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("case {} failed to build context: {error}", case.id));
        let result = session.recalculate().unwrap_or_else(|error| {
            panic!("case {} failed through direct context: {error}", case.id)
        });
        let state = session.workspace_state().unwrap_or_else(|error| {
            panic!("case {} failed to project direct context: {error}", case.id)
        });

        assert_eq!(
            state
                .node(&NodeId::new(case.caller.clone()))
                .map(|node| node.content_text.as_str()),
            Some(case.source_formula.as_str()),
            "{} source formula must enter OxCalc unchanged",
            case.id
        );
        assert_eq!(
            result.run_state,
            OxCalcTreeRunState::Published,
            "{} run state; diagnostics {:?}",
            case.id,
            result.diagnostics
        );
        assert_eq!(
            scalar_value(&state, &case.caller),
            Some(case.expect.published_value.as_str()),
            "{} published value",
            case.id
        );
        let collections = session
            .collection_dependencies_for(&result, &NodeId::new(case.caller.clone()))
            .unwrap_or_else(|error| {
                panic!(
                    "case {} collection dependency projection failed: {error}",
                    case.id
                )
            });
        assert_eq!(
            collections.len(),
            1,
            "{} should publish exactly one collection dependency",
            case.id
        );
        let actual_members = collections[0]
            .members
            .iter()
            .map(|member| member.as_str().to_string())
            .collect::<Vec<_>>();
        if case.expect.ordered.unwrap_or(false) {
            assert_eq!(
                actual_members, case.expect.members,
                "{} ordered dependency membership",
                case.id
            );
        } else {
            assert_eq!(
                actual_members.into_iter().collect::<BTreeSet<_>>(),
                case.expect.members.iter().cloned().collect::<BTreeSet<_>>(),
                "{} dependency membership",
                case.id
            );
        }
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

fn blank_non_target_formula_nodes(workspace: &mut WorkspaceModel, caller: &str) {
    for (path, node) in &mut workspace.nodes {
        if path != caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
}

fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id.to_string()))
        .and_then(|node| node.computed_value.scalar_display_text())
}

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

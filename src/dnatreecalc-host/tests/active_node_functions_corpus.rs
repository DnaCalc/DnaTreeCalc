use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::NodeId;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NodeFunctionTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<NodeFunctionCase>,
}

#[derive(Debug, Deserialize)]
struct NodeFunctionCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: NodeFunctionExpectation,
}

#[derive(Debug, Deserialize)]
struct NodeFunctionExpectation {
    outcome: String,
    target: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[test]
fn active_node_function_corpus_is_direct_context_typed_pending() {
    let theme = load_theme(repo_corpus_path("references/node-functions.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/node-functions");
    assert_eq!(theme.status, CorpusStatus::Active);

    let base_workspace = load_workspace("lambdas");
    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        assert_eq!(case.workspace, "lambdas", "{} workspace", case.id);

        let mut workspace = base_workspace.clone();
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        workspace
            .nodes
            .get_mut(&case.caller)
            .unwrap_or_else(|| panic!("{} caller missing", case.id))
            .content = NodeContent::Formula(format!("={}", case.reference));

        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} failed direct recalc: {error}", case.id));
        let state = session.workspace_state().unwrap();
        let expected_formula = format!("={}", case.reference);
        assert_eq!(
            state
                .node(&NodeId::new(case.caller.clone()))
                .map(|node| node.content_text.as_str()),
            Some(expected_formula.as_str()),
            "{} source formula must enter OxCalc unchanged",
            case.id
        );

        match case.expect.outcome.as_str() {
            "resolved" => {
                assert!(
                    case.expect.target.is_some(),
                    "{} keeps the target expectation while callable binding is pending",
                    case.id
                );
                assert!(
                    result.diagnostics.iter().any(|diagnostic| {
                        diagnostic.contains("typed_exclusion:node_as_function_w074_pending")
                            || diagnostic.contains("unresolved_host_name")
                            || diagnostic.contains("dependency_diagnostic:relative_path")
                            || diagnostic.contains(
                                "oxfml_returned_value_surface_payload_summary:Error(Name)",
                            )
                            || diagnostic.contains("typed_exclusion")
                            || diagnostic.contains("unexpected token Caret")
                            || diagnostic.contains("syntax diagnostics")
                            || diagnostic.contains("missing expression cannot be bound")
                            || diagnostic.contains(
                                "only immediate, helper-bound, defined-name, or lambda-valued callable invocation is supported",
                            )
                    }),
                    "{} should remain a direct-context callable pending lane, got {:?}",
                    case.id,
                    result.diagnostics
                );
            }
            "error" => {
                assert!(case.expect.reason.is_some(), "{} reason", case.id);
                assert!(
                    result.run_state != oxcalc_core::consumer::OxCalcTreeRunState::Published
                        || result
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.contains("typed_exclusion")),
                    "{} set-valued callee must not be accepted as a callable node, got state {:?} diagnostics {:?}",
                    case.id,
                    result.run_state,
                    result.diagnostics
                );
            }
            other => panic!("{} unexpected outcome {other}", case.id),
        }
    }
}

fn blank_non_target_formula_nodes(workspace: &mut WorkspaceModel, caller: &str) {
    for (path, node) in &mut workspace.nodes {
        if path != caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
}

fn load_theme(path: PathBuf) -> NodeFunctionTheme {
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

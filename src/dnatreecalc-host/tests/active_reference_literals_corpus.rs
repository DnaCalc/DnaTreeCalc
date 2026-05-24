use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::NodeId;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<ReferenceLiteralCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct ReferenceLiteralCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    source_formula: String,
    expect: ReferenceLiteralExpectation,
}

#[derive(Debug, Deserialize)]
struct ReferenceLiteralExpectation {
    outcome: Option<String>,
    parse: Option<String>,
    members: Option<Vec<String>>,
    published_value: Option<String>,
    reason: Option<String>,
}

#[test]
fn active_reference_literal_array_corpus_is_direct_context_typed_pending() {
    let theme = load_theme(repo_corpus_path("references/literals-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/literals-active");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert!(
            matches!(case.kind.as_str(), "membership" | "syntax"),
            "case {} kind changed",
            case.id
        );
        assert!(
            matches!(case.reference.as_str(), "{A,B}" | "{A,A}" | "{A,1}"),
            "case {} activates unexpected reference literal {}",
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
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("case {} failed direct recalc: {error}", case.id));
        let state = session.workspace_state().unwrap();
        assert_eq!(
            state
                .node(&NodeId::new(case.caller.clone()))
                .map(|node| node.content_text.as_str()),
            Some(case.source_formula.as_str()),
            "{} source formula must enter OxCalc unchanged",
            case.id
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .contains("typed_exclusion:reference_literal_collection_raw_context_pending")
            }),
            "{} should remain a typed direct-context pending lane, got {:?}",
            case.id,
            result.diagnostics
        );
        if case.kind == "membership" {
            assert_eq!(
                case.expect.outcome.as_deref(),
                Some("resolved"),
                "{} preserves the old carrier-slice expectation as migration history",
                case.id
            );
            assert!(case.expect.members.is_some());
            assert!(case.expect.published_value.is_some());
        } else {
            assert_eq!(case.expect.parse.as_deref(), Some("reject"));
            assert!(case.expect.reason.is_some());
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

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

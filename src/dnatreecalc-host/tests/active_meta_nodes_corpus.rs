use std::collections::BTreeMap;
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
    cases: Vec<MetaCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct MetaCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: MetaExpectation,
}

#[derive(Debug, Deserialize)]
struct MetaExpectation {
    outcome: String,
    target: Option<String>,
    value: Option<String>,
}

#[test]
fn active_meta_nodes_corpus_executes_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("references/meta-nodes.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/meta-nodes");
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
        prepare_meta_case_workspace(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} failed through direct context: {error}", case.id));
        let state = session
            .workspace_state()
            .unwrap_or_else(|error| panic!("{} failed to project context: {error}", case.id));

        assert_eq!(
            state
                .node(&NodeId::new("Section.Config".to_string()))
                .map(|node| node.is_meta),
            Some(true),
            "{} meta flag should project from OxCalc",
            case.id
        );
        assert_eq!(
            state
                .node(&NodeId::new("Section.Config.Secret".to_string()))
                .map(|node| node.is_meta),
            Some(true),
            "{} descendant meta flag should project from OxCalc",
            case.id
        );

        match case.expect.outcome.as_str() {
            "resolved" => {
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
                    Some(case.expect.value.as_deref().unwrap_or("0.1")),
                    "{} published value",
                    case.id
                );
                if case.expect.target.is_some() {
                    assert_eq!(
                        dependency_members(&session, &result, &case.caller),
                        case.expect.target.iter().cloned().collect::<Vec<_>>(),
                        "{} dependency membership",
                        case.id
                    );
                }
            }
            "value" => {
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
                    case.expect.value.as_deref(),
                    "{} published value",
                    case.id
                );
            }
            "unresolved" => {
                assert!(
                    result.diagnostics.iter().any(|diagnostic| {
                        diagnostic.contains("unresolved_host_name:Secret")
                            || diagnostic.contains("oxfml_formal_reference:unresolved:Secret")
                            || diagnostic
                                .contains("oxfml_bind_diagnostic:unresolved identifier 'Secret'")
                            || diagnostic.contains("candidate_rejected:OxFml bind")
                            || diagnostic.contains(
                                "oxfml_returned_value_surface_payload_summary:Error(Name)",
                            )
                    }),
                    "{} hidden meta subtree lookup should remain unresolved, diagnostics={:?}",
                    case.id,
                    result.diagnostics
                );
                assert!(
                    !dependency_members(&session, &result, &case.caller)
                        .iter()
                        .any(|member| member == "Section.Config.Secret"),
                    "{} must not depend on hidden meta Secret",
                    case.id
                );
            }
            other => panic!("{} unexpected outcome {other}", case.id),
        }
    }
}

fn prepare_meta_case_workspace(workspace: &mut WorkspaceModel, case: &MetaCase) {
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

fn dependency_members(
    session: &TreeWorkspaceSession,
    result: &oxcalc_core::consumer::OxCalcTreeCalculationOutcome,
    caller: &str,
) -> Vec<String> {
    session
        .dependency_members_for(result, &NodeId::new(caller.to_string()))
        .unwrap_or_else(|error| panic!("{caller} dependency projection failed: {error}"))
        .iter()
        .map(|member| member.as_str().to_string())
        .collect()
}

fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id.to_string()))
        .and_then(|node| node.computed_value.scalar_display_text())
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

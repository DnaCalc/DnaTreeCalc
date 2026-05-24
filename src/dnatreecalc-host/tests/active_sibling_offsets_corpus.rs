use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{NodeId, NodeValueProjection, WorkspaceState};
use oxcalc_core::consumer::OxCalcTreeRunState;
use oxcalc_core::dependency::DependencyDescriptorKind;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<SiblingOffsetCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct SiblingOffsetCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: SiblingOffsetExpectation,
}

#[derive(Debug, Deserialize)]
struct SiblingOffsetExpectation {
    outcome: String,
    target: Option<String>,
    published_value: Option<String>,
    reason: Option<String>,
}

#[test]
fn active_sibling_offset_corpus_executes_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("references/sibling-offsets.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/sibling-offsets");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        assert!(
            matches!(
                case.reference.as_str(),
                "@PREV.Net" | "@NEXT.Margin" | "@NEXT"
            ),
            "{} activates unexpected sibling reference {}",
            case.id,
            case.reference
        );
        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        prepare_sibling_case_workspace(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} failed through direct context: {error}", case.id));
        let state = session
            .workspace_state()
            .unwrap_or_else(|error| panic!("{} failed to project context: {error}", case.id));

        if case.expect.outcome == "resolved" {
            assert_eq!(
                result.run_state,
                OxCalcTreeRunState::Published,
                "{} run state",
                case.id
            );
            assert_eq!(
                scalar_value(&state, &case.caller),
                case.expect.published_value.as_deref(),
                "{} published value",
                case.id
            );
            assert_eq!(
                dependency_members(&session, &result, &case.caller),
                case.expect.target.iter().cloned().collect::<Vec<_>>(),
                "{} dependency membership",
                case.id
            );
        } else {
            assert_eq!(case.expect.outcome, "error", "{} expected outcome", case.id);
            assert!(case.expect.reason.is_some(), "{} reason", case.id);
            assert!(
                result
                    .dependency_graph
                    .descriptors_by_owner
                    .values()
                    .flatten()
                    .any(|descriptor| {
                        descriptor.kind == DependencyDescriptorKind::RelativeBound
                            && descriptor.target_node_id.is_none()
                            && descriptor.carrier_detail == "sibling_offset:1:"
                            && descriptor.requires_rebind_on_structural_change
                    }),
                "{} should record an unresolved relative-bound sibling descriptor, got {:?}",
                case.id,
                result.dependency_graph.descriptors_by_owner
            );
        }
    }
}

fn prepare_sibling_case_workspace(workspace: &mut WorkspaceModel, case: &SiblingOffsetCase) {
    for (path, node) in &mut workspace.nodes {
        if path != &case.caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
    if case.id == "ref-prev-tail" {
        workspace
            .nodes
            .get_mut("Accounts.2005.Q1.Net")
            .expect("Q1.Net target exists")
            .content = NodeContent::Constant("7".to_string());
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

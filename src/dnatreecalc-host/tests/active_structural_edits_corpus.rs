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
    cases: Vec<StructuralEditCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct StructuralEditCase {
    id: String,
    kind: String,
    workspace: String,
    edit: StructuralEdit,
    caller: String,
    reference: String,
    expect: StructuralEditExpectation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
enum StructuralEdit {
    Delete {
        target: String,
    },
    Rename {
        target: String,
        to: String,
        #[serde(default)]
        propagate: bool,
    },
    Move {
        target: String,
        new_parent: String,
    },
    Insert {
        target: String,
        child: String,
    },
}

#[derive(Debug, Deserialize)]
struct StructuralEditExpectation {
    outcome: String,
    rewritten_to: Option<String>,
    rebinds_to: Option<String>,
}

#[test]
fn active_structural_edit_corpus_executes_rebinds_through_direct_oxcalc_context() {
    let theme = load_theme(repo_corpus_path("structural-edits/propagation.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "structural-edits/propagation");
    assert_eq!(theme.status, CorpusStatus::Active);

    for case in &theme.cases {
        assert_eq!(case.kind, "edit", "{} kind", case.id);
        let mut workspace = load_workspace(&case.workspace);
        prepare_structural_case_workspace(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));

        session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} baseline recalc failed: {error}", case.id));

        apply_structural_edit(&mut session, case);
        let result = session
            .recalculate()
            .unwrap_or_else(|error| panic!("{} post-edit recalc failed: {error}", case.id));

        match case.expect.outcome.as_str() {
            "unresolved" => {
                // Excel-faithful: deleting / renaming / moving a referenced tree
                // node (a defined name) leaves the formula text unchanged and the
                // caller commits with #NAME? -- it is not rejected -- while still
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
            "rebound" => {
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Published,
                    "{} rebound run state: reject={:?}; diagnostics={:?}",
                    case.id,
                    result.reject_detail,
                    result.diagnostics
                );
                if let Some(rewritten_to) = &case.expect.rewritten_to {
                    let expected_formula = format!("=Sales*{rewritten_to}");
                    assert_eq!(
                        formula_text(&session.workspace_state().unwrap(), &case.caller),
                        Some(expected_formula.as_str()),
                        "{} propagated formula text",
                        case.id
                    );
                }
                let expected_target = expected_rebind_target(case);
                let members = dependency_members(&session, &result, &case.caller);
                assert!(
                    members.iter().any(|member| member == &expected_target),
                    "{} expected dependency target {} in {:?}",
                    case.id,
                    expected_target,
                    members
                );
            }
            other => panic!("{} unsupported expectation {other}", case.id),
        }
    }
}

fn prepare_structural_case_workspace(workspace: &mut WorkspaceModel, case: &StructuralEditCase) {
    for (path, node) in &mut workspace.nodes {
        if path != &case.caller && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
        }
    }
    workspace
        .nodes
        .get_mut(&case.caller)
        .unwrap_or_else(|| panic!("{} caller missing", case.id))
        .content = NodeContent::Formula(format!("=Sales*{}", case.reference));
}

fn apply_structural_edit(session: &mut TreeWorkspaceSession, case: &StructuralEditCase) {
    match &case.edit {
        StructuralEdit::Delete { target } => {
            session
                .delete_node(&NodeId::new(target.clone()))
                .unwrap_or_else(|error| panic!("{} delete failed: {error}", case.id));
        }
        StructuralEdit::Rename {
            target,
            to,
            propagate,
        } => {
            session
                .rename_node(&NodeId::new(target.clone()), to)
                .unwrap_or_else(|error| panic!("{} rename failed: {error}", case.id));
            if *propagate {
                let rewritten_to =
                    case.expect.rewritten_to.as_ref().unwrap_or_else(|| {
                        panic!("{} propagated rename needs rewritten_to", case.id)
                    });
                session
                    .edit_formula(
                        &NodeId::new(case.caller.clone()),
                        format!("=Sales*{rewritten_to}"),
                    )
                    .unwrap_or_else(|error| {
                        panic!("{} propagated formula edit failed: {error}", case.id)
                    });
            }
        }
        StructuralEdit::Move { target, new_parent } => {
            session
                .move_node(
                    &NodeId::new(target.clone()),
                    Some(&NodeId::new(new_parent.clone())),
                    None,
                )
                .unwrap_or_else(|error| panic!("{} move failed: {error}", case.id));
        }
        StructuralEdit::Insert { target, child } => {
            session
                .add_node(Some(&NodeId::new(target.clone())), child, "0.3")
                .unwrap_or_else(|error| panic!("{} insert failed: {error}", case.id));
        }
    }
}

fn expected_rebind_target(case: &StructuralEditCase) -> String {
    case.expect
        .rebinds_to
        .clone()
        .or_else(|| {
            case.expect.rewritten_to.as_ref().map(|rewritten_to| {
                let parent = case
                    .edit
                    .target_path()
                    .rsplit_once('.')
                    .map(|(parent, _)| parent)
                    .unwrap_or("");
                format!("{parent}.{rewritten_to}")
            })
        })
        .unwrap_or_else(|| panic!("{} rebound case needs target", case.id))
}

impl StructuralEdit {
    fn target_path(&self) -> &str {
        match self {
            Self::Delete { target }
            | Self::Rename { target, .. }
            | Self::Move { target, .. }
            | Self::Insert { target, .. } => target,
        }
    }
}

fn dependency_members(
    session: &TreeWorkspaceSession,
    outcome: &oxcalc_core::consumer::OxCalcTreeCalculationOutcome,
    owner: &str,
) -> Vec<String> {
    session
        .dependency_members_for(outcome, &NodeId::new(owner.to_string()))
        .unwrap_or_else(|error| panic!("{owner} dependency projection failed: {error}"))
        .iter()
        .map(|member| member.as_str().to_string())
        .collect()
}

fn formula_text<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id.to_string()))
        .map(|node| node.content_text.as_str())
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

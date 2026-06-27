mod support;

use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::model::WorkspaceFixture;
use dnatreecalc_skin_framework::{CalcRunStateProjection, NodeId};
use serde::Deserialize;

use support::programmable::Harness;

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
fn programmable_skin_structural_corpus_cases_execute_from_outside_ir() {
    let theme = load_theme(repo_corpus_path("structural-edits/propagation.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "structural-edits/propagation");
    assert_eq!(theme.status, CorpusStatus::Active);

    for case in &theme.cases {
        assert_eq!(case.kind, "edit", "{} kind", case.id);
        let fixture = prepared_structural_case_fixture(case);
        let harness = Harness::from_fixture(fixture);
        let skin = harness.driver.clone();

        apply_structural_edit(&skin, case);
        let state = skin.state();

        match case.expect.outcome.as_str() {
            "unresolved" => {
                // Excel-faithful: an unresolved name (deleted / renamed / moved
                // defined name) commits with #NAME? -- it is not rejected -- while
                // still surfacing the unresolved-name diagnostic.
                assert_eq!(
                    state.last_run.as_ref().map(|run| run.run_state),
                    Some(CalcRunStateProjection::Published),
                    "{} unresolved run state",
                    case.id
                );
                assert!(
                    state
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.contains("unresolved_host_name")
                            || diagnostic.contains("unresolved identifier")),
                    "{} should report unresolved host name, got {:?}",
                    case.id,
                    state.diagnostics
                );
            }
            "rebound" => {
                assert_eq!(
                    state.last_run.as_ref().map(|run| run.run_state),
                    Some(CalcRunStateProjection::Published),
                    "{} rebound run state: diagnostics={:?}",
                    case.id,
                    state.diagnostics
                );
                if let Some(rewritten_to) = &case.expect.rewritten_to {
                    let expected_formula = format!("=Sales*{rewritten_to}");
                    assert_eq!(
                        state
                            .node(&NodeId::new(case.caller.clone()))
                            .map(|node| node.content_text.as_str()),
                        Some(expected_formula.as_str()),
                        "{} propagated formula text",
                        case.id
                    );
                }
                let expected_target = expected_rebind_target(case);
                let caller_key = state
                    .node(&NodeId::new(case.caller.clone()))
                    .expect("caller projects")
                    .key
                    .clone();
                let targets = state
                    .dependencies
                    .edges_by_owner_key
                    .get(&caller_key)
                    .into_iter()
                    .flatten()
                    .map(|edge| edge.target.as_str().to_string())
                    .collect::<Vec<_>>();
                assert!(
                    targets.iter().any(|target| target == &expected_target),
                    "{} expected dependency target {} in {:?}",
                    case.id,
                    expected_target,
                    targets
                );
            }
            other => panic!("{} unsupported expectation {other}", case.id),
        }
    }
}

fn prepared_structural_case_fixture(case: &StructuralEditCase) -> WorkspaceFixture {
    let mut fixture = WorkspaceFixture::from_repo_fixture(&case.workspace)
        .unwrap_or_else(|error| panic!("failed to load workspace {}: {error}", case.workspace));
    for node in &mut fixture.nodes {
        if node.node_id == case.caller {
            node.formula = format!("=Sales*{}", case.reference);
        } else if node.formula.starts_with('=') {
            node.formula.clear();
        }
    }
    fixture
}

fn apply_structural_edit(
    skin: &support::programmable::ProgrammableDriver,
    case: &StructuralEditCase,
) {
    match &case.edit {
        StructuralEdit::Delete { target } => {
            skin.delete(target);
        }
        StructuralEdit::Rename {
            target,
            to,
            propagate,
        } => {
            skin.rename(target, to);
            if *propagate {
                let rewritten_to =
                    case.expect.rewritten_to.as_ref().unwrap_or_else(|| {
                        panic!("{} propagated rename needs rewritten_to", case.id)
                    });
                skin.edit(&case.caller, &format!("=Sales*{rewritten_to}"));
            }
        }
        StructuralEdit::Move { target, new_parent } => {
            skin.move_node(target, Some(new_parent), None);
        }
        StructuralEdit::Insert { target, child } => {
            skin.add_node(Some(target), child, "0.3");
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

fn load_theme(path: PathBuf) -> CorpusTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse {path:?}: {error}"))
}

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

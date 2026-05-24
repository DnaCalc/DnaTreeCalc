use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::adapters::oxcalc::{
    LiveOxCalcTreeBridge, NodeCalcStateProjection, OxCalcTreeBridge, PreparedBinaryOp,
    PreparedFormula, PreparedFormulaCatalog, PreparedFormulaOperand, PreparedRelativePathBase,
    TreeRecalcRequest,
};
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
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
    canonical_path: Option<String>,
    engine_ref: String,
    engine_ref_shape: EngineRefShape,
    calc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct EngineRefShape {
    kind: EngineRefKind,
    base: EngineRefBase,
    ancestor_distance: Option<usize>,
    path: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EngineRefKind {
    RelativePath,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum EngineRefBase {
    #[serde(rename = "self")]
    SelfNode,
    #[serde(rename = "parent")]
    ParentNode,
    #[serde(rename = "ancestor")]
    Ancestor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedWalkupReference {
    base: PreparedRelativePathBase,
    path_segments: Vec<String>,
}

#[test]
fn active_walkup_corpus_executes_relative_references_through_live_oxcalc_bridge() {
    let theme = load_theme(repo_corpus_path("references/walkup.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/walkup");
    assert_eq!(theme.status, CorpusStatus::Active);

    let mut workspaces = BTreeMap::new();
    for case in &theme.cases {
        workspaces
            .entry(case.workspace.clone())
            .or_insert_with(|| load_workspace(&case.workspace));
    }

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "case {} kind changed", case.id);
        assert!(
            !case.reference.is_empty(),
            "case {} must retain the authored surface reference",
            case.id
        );
        assert!(
            !case.expect.engine_ref.is_empty(),
            "case {} must retain the informational engine_ref",
            case.id
        );
        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        retain_root_containing_caller(&mut workspace, &case.caller);
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        let formula_catalog =
            PreparedFormulaCatalog::new([(case.caller.clone(), prepared_walkup_formula(case))]);

        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge.execute_recalc(TreeRecalcRequest {
            workspace,
            formula_catalog,
            candidate_result_id: format!("cand:{}", case.id),
            publication_id: format!("pub:{}", case.id),
            compatibility_basis: format!("snapshot:{}", case.id),
            artifact_token_basis: format!("snapshot:{}", case.id),
            capability_profile_id: "treecalc-v1".to_string(),
            cycle_config: Default::default(),
        });

        match case.expect.outcome.as_str() {
            "resolved" if case.expect.calc.as_deref() == Some("cycle") => {
                let result = result.unwrap_or_else(|error| {
                    panic!(
                        "case {} failed before cycle classification: {error}",
                        case.id
                    )
                });
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Rejected,
                    "{}",
                    case.id
                );
                assert!(
                    result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.contains("cycle")),
                    "case {} expected cycle diagnostic, got: {:?}",
                    case.id,
                    result.diagnostics
                );
            }
            "resolved" => {
                let result = result.unwrap_or_else(|error| {
                    panic!("case {} failed through live bridge: {error}", case.id)
                });
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Published,
                    "{}",
                    case.id
                );
                assert_eq!(
                    result.dependency_edges_by_owner.get(&case.caller),
                    Some(&vec![case.expect.target.clone().expect("resolved target")]),
                    "{} dependency target",
                    case.id
                );
                if let Some(canonical_path) = &case.expect.canonical_path {
                    assert_eq!(
                        case.expect.target.as_ref(),
                        Some(canonical_path),
                        "{} canonical path",
                        case.id
                    );
                }
            }
            "unresolved" => {
                let result = result.unwrap_or_else(|error| {
                    panic!(
                        "case {} failed before unresolved classification: {error}",
                        case.id
                    )
                });
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Rejected,
                    "{}",
                    case.id
                );
                assert_eq!(
                    result.node_states.get(&case.caller),
                    Some(&NodeCalcStateProjection::RejectedPendingRepair),
                    "{} node state",
                    case.id
                );
                assert!(
                    result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.contains("unresolved")),
                    "case {} expected unresolved diagnostic, got: {:?}",
                    case.id,
                    result.diagnostics
                );
                assert!(
                    !result.dependency_edges_by_owner.contains_key(&case.caller),
                    "case {} must not project a resolved dependency edge",
                    case.id
                );
            }
            other => panic!("case {} has unsupported outcome {other}", case.id),
        }
    }
}

fn prepared_walkup_formula(case: &WalkupCase) -> PreparedFormula {
    let relative_ref = prepared_walkup_reference(&case.id, &case.expect.engine_ref_shape);

    PreparedFormula::Binary {
        op: PreparedBinaryOp::Add,
        left: PreparedFormulaOperand::RelativePath {
            base: relative_ref.base,
            path_segments: relative_ref.path_segments,
        },
        right: PreparedFormulaOperand::Literal {
            value: "0".to_string(),
        },
    }
}

fn prepared_walkup_reference(case_id: &str, shape: &EngineRefShape) -> PreparedWalkupReference {
    assert_eq!(
        shape.kind,
        EngineRefKind::RelativePath,
        "case {case_id} must use the typed relative-path shape"
    );
    let base = match shape.base {
        EngineRefBase::SelfNode => {
            assert!(
                shape.ancestor_distance.is_none(),
                "case {case_id} self base must not carry ancestor_distance"
            );
            PreparedRelativePathBase::SelfNode
        }
        EngineRefBase::ParentNode => {
            assert!(
                shape.ancestor_distance.is_none(),
                "case {case_id} parent base must not carry ancestor_distance"
            );
            PreparedRelativePathBase::ParentNode
        }
        EngineRefBase::Ancestor => PreparedRelativePathBase::Ancestor(
            shape
                .ancestor_distance
                .unwrap_or_else(|| panic!("case {case_id} ancestor base needs a distance")),
        ),
    };

    PreparedWalkupReference {
        base,
        path_segments: shape.path.clone(),
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

fn retain_root_containing_caller(workspace: &mut WorkspaceModel, caller: &str) {
    let root = workspace
        .root_paths
        .iter()
        .find(|root| caller == root.as_str() || caller.starts_with(&format!("{root}.")))
        .cloned()
        .unwrap_or_else(|| panic!("caller {caller} is not inside any workspace root"));
    let root_for_filter = root.clone();
    let root_prefix = format!("{root_for_filter}.");
    let keep_path =
        |path: &str| path == root_for_filter.as_str() || path.starts_with(root_prefix.as_str());
    workspace.root_paths = vec![root];
    workspace.node_order.retain(|path| keep_path(path));
    workspace.nodes.retain(|path, _| keep_path(path));
    workspace.table_nodes.retain(|path, _| keep_path(path));
}

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

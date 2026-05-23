use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::adapters::oxcalc::{
    LiveOxCalcTreeBridge, OxCalcTreeBridge, PreparedFormula, PreparedFormulaCatalog,
    PreparedFormulaReferenceCarrier, PreparedReferenceLiteralArrayElement, TreeRecalcRequest,
};
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use oxcalc_core::consumer::OxCalcTreeRunState;
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
    elements: Vec<ReferenceLiteralElement>,
    expect: ReferenceLiteralExpectation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ReferenceLiteralElement {
    Reference { path: String },
    Scalar { source_text: String },
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
fn active_reference_literal_array_corpus_executes_through_live_oxcalc_bridge() {
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
        let mut workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        let formula_catalog = PreparedFormulaCatalog::new([(
            case.caller.clone(),
            prepared_reference_literal_formula(case),
        )]);
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

        match case.kind.as_str() {
            "membership" => {
                assert_eq!(
                    case.expect.outcome.as_deref(),
                    Some("resolved"),
                    "case {} is outside the active reference-literal success slice",
                    case.id
                );
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
                    result.published_values.get(&case.caller),
                    case.expect.published_value.as_ref(),
                    "{} published value",
                    case.id
                );
                assert_eq!(
                    result.dependency_edges_by_owner.get(&case.caller),
                    case.expect.members.as_ref(),
                    "{} dependency membership",
                    case.id
                );
            }
            "syntax" => {
                assert_eq!(
                    case.expect.parse.as_deref(),
                    Some("reject"),
                    "case {} syntax expectation changed",
                    case.id
                );
                let error = result.expect_err("mixed reference/scalar carrier must reject");
                let error = error.to_string();
                assert!(
                    error.contains(
                        case.expect
                            .reason
                            .as_deref()
                            .expect("reject case carries a reason")
                    ),
                    "case {} expected reason in error: {error}",
                    case.id
                );
            }
            other => panic!("case {} has unsupported active kind {other}", case.id),
        }
    }
}

fn prepared_reference_literal_formula(case: &ReferenceLiteralCase) -> PreparedFormula {
    let source_token = format!("TREE_REF_LITERAL_{}", case.id.replace('-', "_"));
    let source_span_utf8 = Some(required_source_span_utf8(
        &case.id,
        &case.source_formula,
        &case.reference,
    ));
    let elements = case
        .elements
        .iter()
        .map(|element| match element {
            ReferenceLiteralElement::Reference { path } => {
                PreparedReferenceLiteralArrayElement::ReferencePath { path: path.clone() }
            }
            ReferenceLiteralElement::Scalar { source_text } => {
                PreparedReferenceLiteralArrayElement::ScalarValue {
                    source_text: source_text.clone(),
                }
            }
        })
        .collect::<Vec<_>>();

    PreparedFormula::OpaqueOxfml {
        source_text: format!("=SUM({source_token})"),
        reference_carriers: vec![PreparedFormulaReferenceCarrier::ReferenceLiteralArrayV1 {
            source_token,
            source_token_text: case.reference.clone(),
            source_span_utf8,
            elements,
        }],
    }
}

fn required_source_span_utf8(case_id: &str, source_formula: &str, token: &str) -> (usize, usize) {
    let start = source_formula
        .find(token)
        .unwrap_or_else(|| panic!("case {case_id} source formula does not contain {token}"));
    (start, start + token.len())
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

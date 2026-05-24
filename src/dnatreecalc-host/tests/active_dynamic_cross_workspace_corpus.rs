use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::adapters::oxcalc::{
    LiveOxCalcTreeBridge, OxCalcTreeBridge, PreparedFormula, PreparedFormulaCatalog,
    PreparedFormulaReferenceCarrier, TreeCalcCrossWorkspaceReferenceRequest,
    TreeCalcExternalWorkspace, TreeRecalcRequest,
};
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
use oxcalc_core::consumer::OxCalcTreeRunState;
use oxcalc_core::dependency::DependencyDescriptorKind;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CrossWorkspaceTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<CrossWorkspaceCase>,
}

#[derive(Debug, Deserialize)]
struct CrossWorkspaceCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: CrossWorkspaceExpectation,
}

#[derive(Debug, Deserialize)]
struct CrossWorkspaceExpectation {
    outcome: String,
    target: String,
    target_workspace: String,
}

#[derive(Debug, Deserialize)]
struct DynamicTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<DynamicCase>,
}

#[derive(Debug, Deserialize)]
struct DynamicCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    given: Option<BTreeMap<String, String>>,
    expect: DynamicExpectation,
}

#[derive(Debug, Deserialize)]
struct DynamicExpectation {
    outcome: String,
    target: Option<String>,
    depends_on: Option<Vec<String>>,
    engine_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[test]
fn active_cross_workspace_corpus_resolves_through_oxcalc_workspace_packets() {
    let theme = load_cross_workspace_theme(repo_corpus_path("references/cross-workspace.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/cross-workspace");
    assert_eq!(theme.status, CorpusStatus::Active);

    let accounts = load_workspace("accounts");
    let projections = load_workspace("projections");
    let bridge = LiveOxCalcTreeBridge::default();

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        assert_eq!(case.expect.outcome, "resolved", "{} outcome", case.id);
        let source_token = format!("TREE_XWS_{}", case.id.replace('-', "_"));
        let resolution = bridge
            .resolve_cross_workspace_reference(TreeCalcCrossWorkspaceReferenceRequest {
                current_workspace_handle: workspace_handle(&case.workspace),
                current_workspace: accounts.clone(),
                current_availability_version: availability_version(&case.workspace),
                external_workspaces: projection_workspace_entries(&projections),
                aliases: BTreeMap::from([(
                    "projections".to_string(),
                    workspace_handle("projections"),
                )]),
                base_token_text: case.reference.clone(),
                source_token: source_token.clone(),
            })
            .unwrap_or_else(|error| panic!("{} failed workspace resolution: {error}", case.id));

        assert_eq!(
            resolution.source_token, source_token,
            "{} source token",
            case.id
        );
        assert_eq!(
            resolution.target_path, case.expect.target,
            "{} target path",
            case.id
        );
        assert_eq!(
            workspace_id_from_handle(&resolution.workspace_handle),
            case.expect.target_workspace,
            "{} target workspace",
            case.id
        );
        assert!(
            resolution
                .resolution_identity
                .contains("treecalc-workspace-host-path:v1"),
            "{} resolution identity",
            case.id
        );

        let mut workspace = accounts.clone();
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::new([(
                    case.caller.clone(),
                    PreparedFormula::OpaqueOxfml {
                        source_text: "=0".to_string(),
                        reference_carriers: vec![resolution.prepared_carrier.clone()],
                    },
                )]),
                candidate_result_id: format!("cand:xws:{}", case.id),
                publication_id: format!("pub:xws:{}", case.id),
                compatibility_basis: format!("snapshot:xws:{}", case.id),
                artifact_token_basis: format!("snapshot:xws:{}", case.id),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap_or_else(|error| panic!("{} failed through live bridge: {error}", case.id));
        assert_eq!(
            result.run_state,
            OxCalcTreeRunState::Published,
            "{}",
            case.id
        );

        let workspace_edges = result
            .dependency_graph
            .workspace_reverse_edges
            .get(&resolution.target_node_handle)
            .unwrap_or_else(|| panic!("{} missing workspace reverse edge", case.id));
        assert!(
            workspace_edges.iter().any(|edge| {
                edge.owner_node_id == node_id_for_path(&accounts, &case.caller)
                    && edge.kind == DependencyDescriptorKind::HostSensitive
                    && edge.target.workspace_handle == resolution.workspace_handle
                    && edge.target.availability_version == resolution.availability_version
            }),
            "{} workspace-qualified dependency edge",
            case.id
        );
    }
}

#[test]
fn active_dynamic_indirect_corpus_executes_through_oxcalc_dynamic_carriers() {
    let theme = load_dynamic_theme(repo_corpus_path("dynamic-references/indirect.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "dynamic-references/indirect");
    assert_eq!(theme.status, CorpusStatus::Active);

    let base_workspace = load_workspace("dynamic");
    let bridge = LiveOxCalcTreeBridge::default();

    for case in &theme.cases {
        assert_eq!(case.kind, "dynamic", "{} kind", case.id);
        assert_eq!(case.workspace, "dynamic", "{} workspace", case.id);
        assert!(
            !case.expect.engine_ref.is_empty(),
            "{} keeps human dynamic engine evidence",
            case.id
        );
        let mut workspace = base_workspace.clone();
        apply_given_constants(&mut workspace, case.given.as_ref());
        blank_dynamic_formula_nodes(&mut workspace, case);
        let formula_catalog = dynamic_formula_catalog(case);
        let result = bridge.execute_recalc(TreeRecalcRequest {
            workspace,
            formula_catalog,
            candidate_result_id: format!("cand:dyn:{}", case.id),
            publication_id: format!("pub:dyn:{}", case.id),
            compatibility_basis: format!("snapshot:dyn:{}", case.id),
            artifact_token_basis: format!("snapshot:dyn:{}", case.id),
            capability_profile_id: "treecalc-v1".to_string(),
            cycle_config: Default::default(),
        });

        match case.expect.outcome.as_str() {
            "resolved" => {
                let result = result.unwrap_or_else(|error| {
                    panic!("{} failed through dynamic bridge: {error}", case.id)
                });
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Published,
                    "{}",
                    case.id
                );
                assert_dependency_set(&case.id, &result.dependency_edges_by_owner, case);
                assert_eq!(
                    result.published_values.get(&case.caller),
                    expected_dynamic_value(case).as_ref(),
                    "{} published value",
                    case.id
                );
            }
            "error" | "cycle_blocked" => {
                let result = result.unwrap_or_else(|error| {
                    panic!(
                        "{} failed before dynamic rejection classification: {error}",
                        case.id
                    )
                });
                assert_eq!(
                    result.run_state,
                    OxCalcTreeRunState::Rejected,
                    "{}",
                    case.id
                );
                if case.expect.outcome == "cycle_blocked" {
                    assert!(
                        result
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.contains("cycle")),
                        "{} expected cycle diagnostic, got {:?}",
                        case.id,
                        result.diagnostics
                    );
                } else {
                    assert!(
                        result
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.contains("dynamic")),
                        "{} expected dynamic diagnostic, got {:?}",
                        case.id,
                        result.diagnostics
                    );
                }
                if case.expect.depends_on.is_some() {
                    assert_dependency_set(&case.id, &result.dependency_edges_by_owner, case);
                }
            }
            other => panic!("{} unsupported outcome {other}", case.id),
        }
    }
}

fn dynamic_formula_catalog(case: &DynamicCase) -> PreparedFormulaCatalog {
    let mut bindings = vec![(case.caller.clone(), prepared_dynamic_formula(case, None))];
    if case.id == "dyn-ctro-multinode-cycle-blocked" {
        bindings.push((
            "CycB".to_string(),
            prepared_dynamic_formula(case, Some(("CycBName", "CycA"))),
        ));
    }
    PreparedFormulaCatalog::new(bindings)
}

fn prepared_dynamic_formula(
    case: &DynamicCase,
    override_selector_and_target: Option<(&str, &str)>,
) -> PreparedFormula {
    let source_token = format!(
        "TREE_DYN_TARGET_{}",
        override_selector_and_target
            .map(|(selector, _)| selector)
            .unwrap_or(case.id.as_str())
            .replace('-', "_")
    );
    let target = override_selector_and_target
        .map(|(_, target)| target.to_string())
        .or_else(|| case.expect.target.clone())
        .or_else(|| dynamic_cycle_target(case));
    let mut reference_carriers = Vec::new();
    let selector = override_selector_and_target
        .map(|(selector, _)| selector.to_string())
        .or_else(|| {
            case.given
                .as_ref()
                .and_then(|given| given.keys().next().cloned())
        });
    if let Some(selector) = selector {
        reference_carriers.push(PreparedFormulaReferenceCarrier::DirectNode {
            source_token: format!("TREE_DYN_SELECTOR_{}", selector.replace('.', "_")),
            path: selector,
        });
    }

    if let Some(target) = target {
        reference_carriers.push(PreparedFormulaReferenceCarrier::DynamicResolved {
            source_token: source_token.clone(),
            target_path: target,
            carrier_id: format!("dnatreecalc-dynamic:v1:{}", case.id),
            detail: case.expect.engine_ref.clone(),
        });
        PreparedFormula::OpaqueOxfml {
            source_text: format!("={source_token}+0"),
            reference_carriers,
        }
    } else {
        reference_carriers.push(PreparedFormulaReferenceCarrier::DynamicPotential {
            source_token,
            carrier_id: format!("dnatreecalc-dynamic-potential:v1:{}", case.id),
            detail: case.expect.engine_ref.clone(),
        });
        PreparedFormula::OpaqueOxfml {
            source_text: "=0".to_string(),
            reference_carriers,
        }
    }
}

fn dynamic_cycle_target(case: &DynamicCase) -> Option<String> {
    match case.id.as_str() {
        "dyn-ctro-self-cycle-blocked" => Some("Loop".to_string()),
        "dyn-ctro-multinode-cycle-blocked" => Some("CycB".to_string()),
        _ => None,
    }
}

fn expected_dynamic_value(case: &DynamicCase) -> Option<String> {
    match case.expect.target.as_deref()? {
        "Sheet1.Foo" => Some("42".to_string()),
        "Branch.A" => Some("10".to_string()),
        "Branch.B" => Some("20".to_string()),
        target => panic!("{} has unsupported dynamic value target {target}", case.id),
    }
}

fn assert_dependency_set(
    case_id: &str,
    dependency_edges_by_owner: &BTreeMap<String, Vec<String>>,
    case: &DynamicCase,
) {
    let expected = case
        .expect
        .depends_on
        .as_ref()
        .unwrap_or_else(|| panic!("{case_id} expected dependency set missing"))
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual = dependency_edges_by_owner
        .get(&case.caller)
        .unwrap_or_else(|| panic!("{case_id} missing dependency projection"))
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{case_id} dependencies");
}

fn projection_workspace_entries(workspace: &WorkspaceModel) -> Vec<TreeCalcExternalWorkspace> {
    vec![
        TreeCalcExternalWorkspace {
            workspace_handle: workspace_handle("projections"),
            workspace: workspace.clone(),
            availability_version: availability_version("projections"),
        },
        TreeCalcExternalWorkspace {
            workspace_handle: "C:\\Work\\projections.dnatree".to_string(),
            workspace: workspace.clone(),
            availability_version: "treecalc-workspace-availability:v1:quoted-projections"
                .to_string(),
        },
    ]
}

fn workspace_handle(workspace_id: &str) -> String {
    format!("treecalc-workspace:{workspace_id}")
}

fn availability_version(workspace_id: &str) -> String {
    format!("treecalc-workspace-availability:v1:{workspace_id}:loaded")
}

fn workspace_id_from_handle(handle: &str) -> String {
    handle
        .strip_prefix("treecalc-workspace:")
        .unwrap_or("projections")
        .to_string()
}

fn apply_given_constants(workspace: &mut WorkspaceModel, given: Option<&BTreeMap<String, String>>) {
    let Some(given) = given else {
        return;
    };
    for (path, value) in given {
        let node = workspace
            .nodes
            .get_mut(path)
            .unwrap_or_else(|| panic!("given node {path} missing from workspace"));
        node.content = NodeContent::Constant(value.clone());
    }
}

fn blank_dynamic_formula_nodes(workspace: &mut WorkspaceModel, case: &DynamicCase) {
    let keep = if case.id == "dyn-ctro-multinode-cycle-blocked" {
        BTreeSet::from([case.caller.as_str(), "CycB"])
    } else {
        BTreeSet::from([case.caller.as_str()])
    };
    for (path, node) in &mut workspace.nodes {
        if !keep.contains(path.as_str()) && matches!(node.content, NodeContent::Formula(_)) {
            node.content = NodeContent::Empty;
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

fn node_id_for_path(workspace: &WorkspaceModel, path: &str) -> oxcalc_core::structural::TreeNodeId {
    let index = workspace
        .node_order
        .iter()
        .position(|candidate| candidate == path)
        .unwrap_or_else(|| panic!("workspace {} missing path {path}", workspace.workspace_id));
    oxcalc_core::structural::TreeNodeId(u64::try_from(index + 1).expect("node index fits u64"))
}

fn load_cross_workspace_theme(path: PathBuf) -> CrossWorkspaceTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read cross-workspace corpus {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse cross-workspace corpus {path:?}: {error}"))
}

fn load_dynamic_theme(path: PathBuf) -> DynamicTheme {
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read dynamic corpus {path:?}: {error}"));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse dynamic corpus {path:?}: {error}"))
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

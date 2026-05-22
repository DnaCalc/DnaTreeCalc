use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::adapters::oxcalc::{
    LiveOxCalcTreeBridge, OxCalcTreeBridge, PreparedFormulaCatalog, TreeRecalcRequest,
};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use oxcalc_core::consumer::OxCalcTreeRunState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CorpusTheme {
    schema_version: String,
    theme: String,
    status: CorpusStatus,
    cases: Vec<OrderedCase>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CorpusStatus {
    Pending,
    Active,
}

#[derive(Debug, Deserialize)]
struct OrderedCase {
    id: String,
    kind: String,
    workspace: String,
    caller: String,
    reference: String,
    expect: OrderedExpectation,
}

#[derive(Debug, Deserialize)]
struct OrderedExpectation {
    outcome: String,
    members: Vec<String>,
    published_value: String,
}

#[test]
fn active_raw_ordered_selector_corpus_executes_through_live_oxcalc_bridge() {
    let theme = load_theme(repo_corpus_path("references/ordered-raw-active.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/ordered-raw-active");
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
            "case {} is outside the active ordered-selector success slice",
            case.id
        );
        assert!(
            matches!(
                case.reference.as_str(),
                "@PRECEDING"
                    | "@FOLLOWING"
                    | "@ANCESTORS"
                    | "Base.**.Margin"
                    | "Root.StructuralPreceding.Total.@PRECEDING"
                    | "Root.StructuralFollowing.Total.@FOLLOWING"
                    | "Root.StructuralRecursive.Base.**.Margin"
            ),
            "case {} activates unsupported reference {}",
            case.id,
            case.reference
        );

        let workspace = workspaces
            .get(&case.workspace)
            .expect("workspace fixture was loaded")
            .clone();
        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: format!("cand:{}", case.id),
                publication_id: format!("pub:{}", case.id),
                compatibility_basis: format!("snapshot:{}", case.id),
                artifact_token_basis: format!("snapshot:{}", case.id),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap_or_else(|error| panic!("case {} failed through live bridge: {error}", case.id));

        assert_eq!(
            result.run_state,
            OxCalcTreeRunState::Published,
            "{}",
            case.id
        );
        assert_eq!(
            result.published_values.get(&case.caller),
            Some(&case.expect.published_value),
            "{} published value",
            case.id
        );
        assert_eq!(
            result.dependency_edges_by_owner.get(&case.caller),
            Some(&case.expect.members),
            "{} dependency membership",
            case.id
        );
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

fn repo_corpus_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus")
        .join(path)
}

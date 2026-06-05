use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use dnatreecalc_host::app::TreeWorkspaceSession;
use dnatreecalc_host::model::{NodeContent, WorkspaceFixture, WorkspaceModel};
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
fn active_cross_workspace_corpus_is_direct_context_typed_pending() {
    let theme = load_cross_workspace_theme(repo_corpus_path("references/cross-workspace.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "references/cross-workspace");
    assert_eq!(theme.status, CorpusStatus::Active);

    let base_workspace = load_workspace("accounts");

    for case in &theme.cases {
        assert_eq!(case.kind, "resolution", "{} kind", case.id);
        assert_eq!(case.workspace, "accounts", "{} workspace", case.id);
        assert_eq!(case.expect.outcome, "resolved", "{} outcome", case.id);
        assert!(!case.expect.target.is_empty(), "{} target", case.id);
        assert!(
            !case.expect.target_workspace.is_empty(),
            "{} workspace",
            case.id
        );

        let mut workspace = base_workspace.clone();
        blank_non_target_formula_nodes(&mut workspace, &case.caller);
        workspace
            .nodes
            .get_mut(&case.caller)
            .unwrap_or_else(|| panic!("{} caller missing", case.id))
            .content = NodeContent::Formula(format!("={}", case.reference));
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let diagnostics = diagnostics_from_recalc(&mut session);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.contains("typed_exclusion:cross_workspace_host_name_pending")
                    || diagnostic.contains("typed_exclusion:bracket_escaped_host_path_pending")
                    || diagnostic.contains("oxfml_formal_reference:unresolved:")
                    || diagnostic.contains("oxfml_bind_diagnostic:unresolved identifier")
                    || diagnostic.contains("candidate_rejected:OxFml bind")
                    || diagnostic.contains("unsupported")
                    || diagnostic.contains("cross-workspace")
            }),
            "{} should remain a direct-context pending/exclusion lane, got {:?}",
            case.id,
            diagnostics
        );
    }
}

#[test]
fn active_dynamic_indirect_corpus_is_direct_context_typed_pending() {
    let theme = load_dynamic_theme(repo_corpus_path("dynamic-references/indirect.json"));
    assert_eq!(theme.schema_version, "treecalc-corpus-v1");
    assert_eq!(theme.theme, "dynamic-references/indirect");
    assert_eq!(theme.status, CorpusStatus::Active);

    let base_workspace = load_workspace("dynamic");

    for case in &theme.cases {
        assert_eq!(case.kind, "dynamic", "{} kind", case.id);
        assert_eq!(case.workspace, "dynamic", "{} workspace", case.id);
        assert!(
            !case.expect.engine_ref.is_empty(),
            "{} keeps human dynamic engine evidence",
            case.id
        );
        if case.expect.outcome == "resolved" {
            assert!(case.expect.target.is_some(), "{} target", case.id);
        }
        if case.expect.outcome != "error" {
            assert!(case.expect.depends_on.is_some(), "{} deps", case.id);
        }

        let mut workspace = base_workspace.clone();
        apply_given_constants(&mut workspace, case.given.as_ref());
        blank_dynamic_formula_nodes(&mut workspace, case);
        let mut session = TreeWorkspaceSession::from_model(&workspace)
            .unwrap_or_else(|error| panic!("{} failed to build context: {error}", case.id));
        let diagnostics = diagnostics_from_recalc(&mut session);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.contains("typed_exclusion:dynamic_indirect_raw_context_pending")
                    || diagnostic.contains("dynamic")
            }),
            "{} should remain a direct-context dynamic pending lane, got {:?}",
            case.id,
            diagnostics
        );
    }
}

fn diagnostics_from_recalc(session: &mut TreeWorkspaceSession) -> Vec<String> {
    match session.recalculate() {
        Ok(outcome) => outcome.diagnostics,
        Err(error) => vec![error.to_string()],
    }
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

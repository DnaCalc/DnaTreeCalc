use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WorkspaceFixture {
    pub schema_version: String,
    pub workspace_id: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<CapabilityProfileId>,
    pub nodes: Vec<WorkspaceNodeFixture>,
}

impl WorkspaceFixture {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, WorkspaceModelError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| WorkspaceModelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&contents).map_err(|source| WorkspaceModelError::Json {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn from_repo_fixture(workspace_id: &str) -> Result<Self, WorkspaceModelError> {
        Self::from_path(repo_fixture_path(workspace_id))
    }
}

fn repo_fixture_path(workspace_id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/test-corpus/workspaces")
        .join(format!("{workspace_id}.json"))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityProfileId {
    TreecalcV1,
    StrictExcel,
}

impl CapabilityProfileId {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TreecalcV1 => "treecalc-v1",
            Self::StrictExcel => "strict-excel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WorkspaceNodeFixture {
    pub node_id: String,
    #[serde(default)]
    pub formula: String,
    #[serde(default)]
    pub is_meta: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<TableNodeFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableNodeFixture {
    pub table_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_path: Option<String>,
    pub table_namespace_version: String,
    pub row_membership_version: String,
    pub row_order_version: String,
    pub column_identity_version: String,
    pub identity_policy: TableIdentityPolicyFixture,
    pub header: TableSectionFixture,
    pub totals: TableSectionFixture,
    pub rows: Vec<TableRowFixture>,
    pub columns: Vec<TableColumnFixture>,
}

impl TableNodeFixture {
    #[must_use]
    pub fn with_default_paths(mut self, node_path: &str) -> Self {
        if self.display_path.is_none() {
            self.display_path = Some(node_path.to_string());
        }
        if self.canonical_path.is_none() {
            self.canonical_path = Some(node_path.to_string());
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableIdentityPolicyFixture {
    pub rename_preserves_table_id: bool,
    pub move_preserves_table_id: bool,
    pub delete_releases_table_id: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableSectionFixture {
    pub present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableRowFixture {
    pub row_id: String,
    pub ordinal: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableColumnFixture {
    pub column_id: String,
    pub name: String,
    pub ordinal: u32,
    pub body: TableColumnBodyFixture,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub totals_formula: Option<TableFormulaFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableColumnBodyFixture {
    pub kind: TableColumnBodyKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constants: Vec<TableCellFixture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<TableFormulaFixture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TableColumnBodyKind {
    ConstantCells,
    Formula,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableCellFixture {
    pub row_id: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableFormulaFixture {
    pub formula_text: String,
    pub formula_stable_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_artifact_id: Option<String>,
    pub formula_text_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceModel {
    pub workspace_id: String,
    pub profile: CapabilityProfileId,
    pub node_order: Vec<String>,
    pub root_paths: Vec<String>,
    pub nodes: BTreeMap<String, WorkspaceNode>,
    pub table_nodes: BTreeMap<String, TableNodeFixture>,
}

impl WorkspaceModel {
    pub fn node(&self, path: &str) -> Option<&WorkspaceNode> {
        self.nodes.get(path)
    }

    pub fn table_node(&self, path: &str) -> Option<&TableNodeFixture> {
        self.table_nodes.get(path)
    }
}

impl TryFrom<WorkspaceFixture> for WorkspaceModel {
    type Error = WorkspaceModelError;

    fn try_from(fixture: WorkspaceFixture) -> Result<Self, Self::Error> {
        if fixture.schema_version != "treecalc-workspace-v1" {
            return Err(WorkspaceModelError::UnsupportedSchema {
                schema_version: fixture.schema_version,
            });
        }

        let mut seen = BTreeSet::new();
        let mut nodes = BTreeMap::new();
        let mut root_paths = Vec::new();

        for raw in &fixture.nodes {
            if !seen.insert(raw.node_id.clone()) {
                return Err(WorkspaceModelError::DuplicateNode {
                    node_id: raw.node_id.clone(),
                });
            }
        }

        for raw in &fixture.nodes {
            let parent_path = parent_path(&raw.node_id);
            if let Some(parent) = parent_path.as_ref() {
                if !seen.contains(parent) {
                    return Err(WorkspaceModelError::MissingParent {
                        node_id: raw.node_id.clone(),
                        parent_id: parent.clone(),
                    });
                }
            } else {
                root_paths.push(raw.node_id.clone());
            }

            let table = raw
                .table
                .clone()
                .map(|table| table.with_default_paths(&raw.node_id));

            nodes.insert(
                raw.node_id.clone(),
                WorkspaceNode {
                    path: raw.node_id.clone(),
                    name: node_name(&raw.node_id).to_string(),
                    parent_path,
                    child_paths: Vec::new(),
                    content: NodeContent::from(raw.formula.as_str()),
                    is_meta: raw.is_meta,
                    table,
                },
            );
        }

        for raw in &fixture.nodes {
            if let Some(parent) = parent_path(&raw.node_id) {
                let parent_node =
                    nodes
                        .get_mut(&parent)
                        .ok_or_else(|| WorkspaceModelError::MissingParent {
                            node_id: raw.node_id.clone(),
                            parent_id: parent.clone(),
                        })?;
                parent_node.child_paths.push(raw.node_id.clone());
            }
        }

        let table_nodes = nodes
            .iter()
            .filter_map(|(path, node)| node.table.clone().map(|table| (path.clone(), table)))
            .collect();

        Ok(Self {
            workspace_id: fixture.workspace_id,
            profile: fixture.profile.unwrap_or(CapabilityProfileId::TreecalcV1),
            node_order: fixture
                .nodes
                .iter()
                .map(|node| node.node_id.clone())
                .collect(),
            root_paths,
            nodes,
            table_nodes,
        })
    }
}

fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('.').map(|(parent, _)| parent.to_string())
}

fn node_name(path: &str) -> &str {
    path.rsplit_once('.').map_or(path, |(_, segment)| segment)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNode {
    pub path: String,
    pub name: String,
    pub parent_path: Option<String>,
    pub child_paths: Vec<String>,
    pub content: NodeContent,
    pub is_meta: bool,
    pub table: Option<TableNodeFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeContent {
    Empty,
    Constant(String),
    Formula(String),
}

impl NodeContent {
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Constant(text) | Self::Formula(text) => text,
        }
    }

    #[must_use]
    pub fn kind(&self) -> NodeContentKind {
        match self {
            Self::Empty => NodeContentKind::Empty,
            Self::Constant(_) => NodeContentKind::Constant,
            Self::Formula(_) => NodeContentKind::Formula,
        }
    }
}

impl From<&str> for NodeContent {
    fn from(value: &str) -> Self {
        if value.is_empty() {
            Self::Empty
        } else if value.starts_with('=') {
            Self::Formula(value.to_string())
        } else {
            Self::Constant(value.to_string())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeContentKind {
    Empty,
    Constant,
    Formula,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceModelError {
    #[error("failed to read workspace fixture {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse workspace fixture {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("unsupported workspace schema version {schema_version}")]
    UnsupportedSchema { schema_version: String },
    #[error("duplicate node id {node_id}")]
    DuplicateNode { node_id: String },
    #[error("node {node_id} references missing parent {parent_id}")]
    MissingParent { node_id: String, parent_id: String },
}

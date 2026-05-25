use std::collections::{BTreeMap, BTreeSet};

use dnatreecalc_skin_framework::{
    NodeContentKind as FrameworkContentKind, NodeId, NodeValueProjection, NodeView, WorkspaceState,
};
use oxcalc_core::consumer::{
    OxCalcTreeCalculationOutcome, OxCalcTreeContext, OxCalcTreeContextError,
    OxCalcTreeContextOptions, OxCalcTreeHostCapabilitySnapshot, OxCalcTreeNodeCreate,
    OxCalcTreeWorkspaceCreate, OxCalcTreeWorkspaceId, OxCalcTreeWorkspaceSnapshot,
};
use oxcalc_core::recalc::NodeCalcState;
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::structured_table::{
    TreeCalcDynamicTableRebindReport, TreeCalcDynamicTableRebindRequest,
    TreeCalcTableColumnBodyMetadata, TreeCalcTableColumnSnapshot, TreeCalcTableFormulaMetadata,
    TreeCalcTableNodeSnapshot, TreeCalcTableRowId, TreeCalcTableVirtualAnchor,
};
use serde::{Deserialize, Serialize};

use crate::model::{
    CapabilityProfileId, NodeContent, TableColumnBodyKind, TableColumnFixture, TableFormulaFixture,
    TableNodeFixture, WorkspaceModel,
};

const ENGINE_ROOT_SYMBOL: &str = "__dnatreecalc_workspace__";
const DNATREE_DOCUMENT_SCHEMA_VERSION: &str = "dnatreecalc-workspace-document-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnaTreeWorkspaceDocument {
    pub schema_version: String,
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_node: Option<String>,
    pub oxcalc_workspace: OxCalcTreeWorkspaceSnapshot,
}

pub struct TreeWorkspaceSession {
    context: OxCalcTreeContext,
    workspace_id: OxCalcTreeWorkspaceId,
    profile: &'static str,
    engine_root_id: TreeNodeId,
    node_ids: BTreeMap<NodeId, TreeNodeId>,
    display_order: Vec<NodeId>,
    recalc_count: usize,
}

impl TreeWorkspaceSession {
    pub fn from_model(model: &WorkspaceModel) -> Result<Self, TreeWorkspaceSessionError> {
        let mut context = context_for_profile(&model.profile);
        let workspace_id = context.create_workspace(
            OxCalcTreeWorkspaceCreate::new(&model.workspace_id)
                .with_root_symbol(ENGINE_ROOT_SYMBOL),
        )?;
        let engine_root_id = context.workspace_view(&workspace_id)?.root_node_id;
        let mut session = Self {
            context,
            workspace_id,
            profile: model.profile.as_str(),
            engine_root_id,
            node_ids: BTreeMap::new(),
            display_order: Vec::new(),
            recalc_count: 0,
        };

        for path in &model.node_order {
            let node = model
                .node(path)
                .ok_or_else(|| TreeWorkspaceSessionError::UnknownNodePath { node: path.clone() })?;
            let parent_node_id = match &node.parent_path {
                Some(parent) => Some(session.tree_node_id(parent)?),
                None => Some(engine_root_id),
            };
            let tree_node_id = session.context.add_node(
                &session.workspace_id,
                OxCalcTreeNodeCreate::new(node.name.clone(), node.content.text())
                    .with_meta(node.is_meta)
                    .under(parent_node_id.unwrap_or(engine_root_id)),
            )?;
            let node_id = NodeId::new(path.clone());
            session.node_ids.insert(node_id.clone(), tree_node_id);
            session.display_order.push(node_id.clone());

            if let Some(table) = &node.table {
                let snapshot = table_snapshot_from_fixture(
                    model.workspace_id.as_str(),
                    tree_node_id,
                    path.as_str(),
                    table,
                );
                session
                    .context
                    .set_node_table(&session.workspace_id, tree_node_id, snapshot)?;
            }
        }

        Ok(session)
    }

    pub fn from_workspace_snapshot(
        snapshot: OxCalcTreeWorkspaceSnapshot,
        profile: &'static str,
    ) -> Result<Self, TreeWorkspaceSessionError> {
        let mut context = context_for_profile_id(profile);
        let workspace_id = context.import_workspace_snapshot(snapshot)?;
        let workspace_view = context.workspace_view(&workspace_id)?;
        let engine_root_id = workspace_view.root_node_id;
        let mut session = Self {
            context,
            workspace_id,
            profile,
            engine_root_id,
            node_ids: BTreeMap::new(),
            display_order: Vec::new(),
            recalc_count: 0,
        };
        session.refresh_projection_from_context()?;
        Ok(session)
    }

    pub fn export_workspace_snapshot(
        &self,
    ) -> Result<OxCalcTreeWorkspaceSnapshot, TreeWorkspaceSessionError> {
        Ok(self.context.export_workspace_snapshot(&self.workspace_id)?)
    }

    pub fn export_dnatree_document(
        &self,
        selected_node: Option<&NodeId>,
    ) -> Result<DnaTreeWorkspaceDocument, TreeWorkspaceSessionError> {
        Ok(DnaTreeWorkspaceDocument {
            schema_version: DNATREE_DOCUMENT_SCHEMA_VERSION.to_string(),
            profile: self.profile.to_string(),
            selected_node: selected_node.map(|node| node.as_str().to_string()),
            oxcalc_workspace: self.export_workspace_snapshot()?,
        })
    }

    pub fn from_dnatree_document(
        document: DnaTreeWorkspaceDocument,
    ) -> Result<(Self, Option<NodeId>), TreeWorkspaceSessionError> {
        if document.schema_version != DNATREE_DOCUMENT_SCHEMA_VERSION {
            return Err(TreeWorkspaceSessionError::UnsupportedDocumentSchema {
                schema_version: document.schema_version,
            });
        }
        let profile = leaked_profile(document.profile);
        let selection = document
            .selected_node
            .as_ref()
            .map(|node| NodeId::new(node.clone()));
        let session = Self::from_workspace_snapshot(document.oxcalc_workspace, profile)?;
        Ok((session, selection))
    }

    pub fn table_context_identity(
        &self,
        node: &NodeId,
    ) -> Result<Option<String>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        Ok(self
            .context
            .table_view(&self.workspace_id, tree_node_id)?
            .map(|view| view.projection.table_context_identity))
    }

    pub fn classify_dynamic_table_rebind(
        &self,
        request: TreeCalcDynamicTableRebindRequest,
    ) -> Result<TreeCalcDynamicTableRebindReport, TreeWorkspaceSessionError> {
        Ok(self
            .context
            .classify_dynamic_table_rebind(&self.workspace_id, request)?)
    }

    pub fn recalculate(
        &mut self,
    ) -> Result<OxCalcTreeCalculationOutcome, TreeWorkspaceSessionError> {
        let outcome = self.context.recalculate(&self.workspace_id)?;
        self.recalc_count += 1;
        Ok(outcome)
    }

    #[must_use]
    pub fn recalc_count(&self) -> usize {
        self.recalc_count
    }

    #[must_use]
    pub fn capability_profile_id(&self) -> &str {
        &self
            .context
            .options()
            .host_capabilities
            .capability_profile_id
    }

    pub fn edit_formula(
        &mut self,
        node: &NodeId,
        content: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context
            .set_node_formula_text(&self.workspace_id, tree_node_id, content)?;
        Ok(())
    }

    pub fn add_node(
        &mut self,
        parent: Option<&NodeId>,
        symbol: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<NodeId, TreeWorkspaceSessionError> {
        let symbol = symbol.into();
        let parent_tree_node_id = match parent {
            Some(parent) => self.tree_node_id(parent.as_str())?,
            None => self.engine_root_id,
        };
        let node_id = NodeId::new(match parent {
            Some(parent) => format!("{}.{}", parent.as_str(), symbol),
            None => symbol.clone(),
        });
        if self.node_ids.contains_key(&node_id) {
            return Err(TreeWorkspaceSessionError::DuplicateNodePath {
                node: node_id.to_string(),
            });
        }

        let tree_node_id = self.context.add_node(
            &self.workspace_id,
            OxCalcTreeNodeCreate::new(symbol, content).under(parent_tree_node_id),
        )?;
        self.node_ids.insert(node_id.clone(), tree_node_id);
        self.display_order.push(node_id.clone());
        Ok(node_id)
    }

    pub fn rename_node(
        &mut self,
        node: &NodeId,
        new_symbol: impl Into<String>,
    ) -> Result<NodeId, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context
            .rename_node(&self.workspace_id, tree_node_id, new_symbol)?;
        self.refresh_projection_from_context()?;
        self.node_id_for_tree_node(tree_node_id)
    }

    pub fn move_node(
        &mut self,
        node: &NodeId,
        new_parent: Option<&NodeId>,
        new_index: Option<usize>,
    ) -> Result<NodeId, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let new_parent_id = match new_parent {
            Some(parent) => self.tree_node_id(parent.as_str())?,
            None => self.engine_root_id,
        };
        self.context
            .move_node(&self.workspace_id, tree_node_id, new_parent_id, new_index)?;
        self.refresh_projection_from_context()?;
        self.node_id_for_tree_node(tree_node_id)
    }

    pub fn reorder_node(
        &mut self,
        node: &NodeId,
        new_index: usize,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context
            .reorder_node(&self.workspace_id, tree_node_id, new_index)?;
        let Some(parent) = parent_path(node.as_str()) else {
            reorder_root(&mut self.display_order, node, new_index);
            return Ok(());
        };
        reorder_child(
            &mut self.display_order,
            &NodeId::new(parent),
            node,
            new_index,
        );
        Ok(())
    }

    pub fn delete_node(&mut self, node: &NodeId) -> Result<(), TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context.delete_node(&self.workspace_id, tree_node_id)?;
        self.refresh_projection_from_context()?;
        Ok(())
    }

    pub fn workspace_state(&self) -> Result<WorkspaceState, TreeWorkspaceSessionError> {
        let workspace_view = self.context.workspace_view(&self.workspace_id)?;
        let views_by_tree_id = workspace_view
            .nodes
            .into_iter()
            .map(|view| (view.node_id, view))
            .collect::<BTreeMap<_, _>>();

        let known_ids = self.display_order.iter().cloned().collect::<BTreeSet<_>>();
        let mut child_map = BTreeMap::<NodeId, Vec<NodeId>>::new();
        let mut root_paths = Vec::new();
        for node_id in &self.display_order {
            if let Some(parent) = parent_path(node_id.as_str()) {
                child_map
                    .entry(NodeId::new(parent))
                    .or_default()
                    .push(node_id.clone());
            } else {
                root_paths.push(node_id.clone());
            }
        }

        let mut nodes = BTreeMap::new();
        for node_id in &self.display_order {
            let tree_node_id = self.tree_node_id(node_id.as_str())?;
            let tree_view = views_by_tree_id.get(&tree_node_id).ok_or_else(|| {
                TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: node_id.to_string(),
                }
            })?;
            let parent = parent_path(node_id.as_str())
                .map(NodeId::new)
                .filter(|parent| known_ids.contains(parent));
            let content_kind = content_kind_for_text(&tree_view.formula_text);
            let computed_value =
                value_projection_for(tree_view.value_text.clone(), tree_view.calc_state);
            nodes.insert(
                node_id.clone(),
                NodeView {
                    id: node_id.clone(),
                    display_name: tree_view.symbol.clone(),
                    parent,
                    children: child_map.remove(node_id).unwrap_or_default(),
                    depth: depth_of(node_id.as_str()),
                    content_kind,
                    content_text: tree_view.formula_text.clone(),
                    computed_value,
                    is_meta: tree_view.is_meta,
                },
            );
        }

        Ok(WorkspaceState {
            workspace_id: self.workspace_id.as_str().to_string(),
            profile: self.profile,
            node_order: self.display_order.clone(),
            root_paths,
            nodes,
        })
    }

    pub fn dependency_members_for(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
        owner: &NodeId,
    ) -> Result<Vec<NodeId>, TreeWorkspaceSessionError> {
        let owner_node_id = self.tree_node_id(owner.as_str())?;
        outcome
            .dependency_graph
            .edges_by_owner
            .get(&owner_node_id)
            .map_or_else(
                || Ok(Vec::new()),
                |edges| {
                    edges
                        .iter()
                        .filter(|edge| edge.target_node_id != self.engine_root_id)
                        .map(|edge| self.node_id_for_tree_node(edge.target_node_id))
                        .collect()
                },
            )
    }

    fn tree_node_id(&self, node: &str) -> Result<TreeNodeId, TreeWorkspaceSessionError> {
        self.node_ids
            .get(&NodeId::new(node.to_string()))
            .copied()
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownNodePath {
                node: node.to_string(),
            })
    }

    fn node_id_for_tree_node(
        &self,
        tree_node_id: TreeNodeId,
    ) -> Result<NodeId, TreeWorkspaceSessionError> {
        self.node_ids
            .iter()
            .find_map(|(node_id, candidate)| (*candidate == tree_node_id).then(|| node_id.clone()))
            .ok_or(TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: format!("tree node {tree_node_id}"),
            })
    }

    fn refresh_projection_from_context(&mut self) -> Result<(), TreeWorkspaceSessionError> {
        let workspace_view = self.context.workspace_view(&self.workspace_id)?;
        let old_order = self
            .display_order
            .iter()
            .filter_map(|node_id| {
                self.node_ids
                    .get(node_id)
                    .copied()
                    .map(|tree_node_id| (tree_node_id, node_id.clone()))
            })
            .collect::<Vec<_>>();
        let mut refreshed = BTreeMap::new();
        for view in workspace_view.nodes {
            if view.node_id == self.engine_root_id {
                continue;
            }
            let node_id = node_id_from_canonical_path(&view.canonical_path)?;
            refreshed.insert(view.node_id, node_id);
        }

        let mut seen = BTreeSet::new();
        let mut display_order = old_order
            .into_iter()
            .filter_map(|(tree_node_id, _)| {
                refreshed
                    .get(&tree_node_id)
                    .and_then(|node_id| seen.insert(tree_node_id).then(|| node_id.clone()))
            })
            .collect::<Vec<_>>();
        for (tree_node_id, node_id) in &refreshed {
            if seen.insert(*tree_node_id) {
                display_order.push(node_id.clone());
            }
        }

        self.node_ids = refreshed
            .into_iter()
            .map(|(tree_node_id, node_id)| (node_id, tree_node_id))
            .collect();
        self.display_order = display_order;
        Ok(())
    }
}

fn context_for_profile(profile: &CapabilityProfileId) -> OxCalcTreeContext {
    context_for_profile_id(profile.as_str())
}

fn context_for_profile_id(profile: &str) -> OxCalcTreeContext {
    let capability_profile_id = match profile {
        "strict-excel" => "host-capabilities:strict-excel",
        _ => "host-capabilities:treecalc-v1",
    };
    OxCalcTreeContext::new(OxCalcTreeContextOptions::new().with_host_capabilities(
        OxCalcTreeHostCapabilitySnapshot {
            capability_profile_id: capability_profile_id.to_string(),
            dynamic_dependency_effects: true,
            execution_restriction_effects: true,
            capability_sensitive_effects: true,
            shape_topology_effects: true,
        },
    ))
}

fn leaked_profile(profile: String) -> &'static str {
    match profile.as_str() {
        "strict-excel" => "strict-excel",
        "treecalc-v1" => "treecalc-v1",
        other => Box::leak(other.to_string().into_boxed_str()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TreeWorkspaceSessionError {
    #[error(transparent)]
    OxCalc(#[from] OxCalcTreeContextError),
    #[error("unsupported .dnatree document schema {schema_version}")]
    UnsupportedDocumentSchema { schema_version: String },
    #[error("unknown DnaTreeCalc node path {node}")]
    UnknownNodePath { node: String },
    #[error("duplicate DnaTreeCalc node path {node}")]
    DuplicateNodePath { node: String },
    #[error("OxCalc context projection is out of sync for {node}")]
    ProjectionOutOfSync { node: String },
}

fn table_snapshot_from_fixture(
    workspace_id: &str,
    tree_node_id: TreeNodeId,
    path: &str,
    table: &TableNodeFixture,
) -> TreeCalcTableNodeSnapshot {
    let mut rows = table.rows.clone();
    rows.sort_by_key(|row| row.ordinal);
    let mut columns = table.columns.clone();
    columns.sort_by_key(|column| column.ordinal);

    TreeCalcTableNodeSnapshot {
        table_node_id: tree_node_id,
        table_id: table.table_id.clone(),
        table_name: path.rsplit('.').next().unwrap_or(path).to_string(),
        display_path: table
            .display_path
            .clone()
            .unwrap_or_else(|| path.to_string()),
        canonical_path: table
            .canonical_path
            .clone()
            .unwrap_or_else(|| path.to_string()),
        virtual_anchor: TreeCalcTableVirtualAnchor {
            workbook_scope_ref: workspace_id.to_string(),
            sheet_scope_ref: path.to_string(),
            start_row: 1,
            start_col: 1,
        },
        rows: rows
            .into_iter()
            .map(|row| TreeCalcTableRowId(row.row_id))
            .collect(),
        columns: columns
            .iter()
            .map(table_column_snapshot_from_fixture)
            .collect(),
        header_row_present: table.header.present,
        totals_row_present: table.totals.present,
        table_namespace_version: table.table_namespace_version.clone(),
        row_membership_version: table.row_membership_version.clone(),
        row_order_version: table.row_order_version.clone(),
        column_identity_version: table.column_identity_version.clone(),
    }
}

fn table_column_snapshot_from_fixture(column: &TableColumnFixture) -> TreeCalcTableColumnSnapshot {
    TreeCalcTableColumnSnapshot {
        column_id: column.column_id.clone(),
        column_name: column.name.clone(),
        ordinal: column.ordinal,
        body_metadata: match column.body.kind {
            TableColumnBodyKind::ConstantCells => TreeCalcTableColumnBodyMetadata::ConstantCells,
            TableColumnBodyKind::Formula => TreeCalcTableColumnBodyMetadata::Formula(
                table_formula_metadata(column.body.formula.as_ref()),
            ),
        },
        totals_metadata: column
            .totals_formula
            .as_ref()
            .map(|formula| table_formula_metadata(Some(formula))),
    }
}

fn table_formula_metadata(formula: Option<&TableFormulaFixture>) -> TreeCalcTableFormulaMetadata {
    formula.map_or_else(
        || TreeCalcTableFormulaMetadata {
            formula_artifact_id: "dnatreecalc.table_formula:missing".to_string(),
            bind_artifact_id: None,
            formula_text_version: "missing".to_string(),
        },
        |formula| TreeCalcTableFormulaMetadata {
            formula_artifact_id: formula.formula_stable_id.clone(),
            bind_artifact_id: formula.bind_artifact_id.clone(),
            formula_text_version: formula.formula_text_version.clone(),
        },
    )
}

fn node_id_from_canonical_path(canonical_path: &str) -> Result<NodeId, TreeWorkspaceSessionError> {
    let mut segments = canonical_path.split('/').collect::<Vec<_>>();
    if segments.first() == Some(&ENGINE_ROOT_SYMBOL) {
        segments.remove(0);
    }
    if segments.is_empty() {
        return Err(TreeWorkspaceSessionError::ProjectionOutOfSync {
            node: canonical_path.to_string(),
        });
    }
    Ok(NodeId::new(segments.join(".")))
}

fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('.').map(|(parent, _)| parent.to_string())
}

fn depth_of(path: &str) -> u32 {
    u32::try_from(path.matches('.').count()).unwrap_or(u32::MAX)
}

fn content_kind_for_text(text: &str) -> FrameworkContentKind {
    match NodeContent::from(text).kind() {
        crate::model::NodeContentKind::Empty => FrameworkContentKind::Empty,
        crate::model::NodeContentKind::Constant => FrameworkContentKind::Constant,
        crate::model::NodeContentKind::Formula => FrameworkContentKind::Formula,
    }
}

fn value_projection_for(
    value_text: Option<String>,
    calc_state: Option<NodeCalcState>,
) -> NodeValueProjection {
    match calc_state {
        Some(NodeCalcState::RejectedPendingRepair | NodeCalcState::CycleBlocked) => {
            NodeValueProjection::Error(
                value_text.unwrap_or_else(|| "calculation rejected".to_string()),
            )
        }
        _ => value_text.map_or(
            NodeValueProjection::Unevaluated,
            NodeValueProjection::Scalar,
        ),
    }
}

fn reorder_child(display_order: &mut [NodeId], parent: &NodeId, node: &NodeId, new_index: usize) {
    let mut children = display_order
        .iter()
        .filter(|candidate| parent_path(candidate.as_str()).as_deref() == Some(parent.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    reorder_root(&mut children, node, new_index);
    let child_positions = display_order
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            (parent_path(candidate.as_str()).as_deref() == Some(parent.as_str())).then_some(index)
        })
        .collect::<Vec<_>>();
    for (slot, child) in child_positions.into_iter().zip(children) {
        display_order[slot] = child;
    }
}

fn reorder_root(display_order: &mut Vec<NodeId>, node: &NodeId, new_index: usize) {
    let Some(current_index) = display_order.iter().position(|candidate| candidate == node) else {
        return;
    };
    let node = display_order.remove(current_index);
    let bounded_index = new_index.min(display_order.len());
    display_order.insert(bounded_index, node);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{WorkspaceFixture, WorkspaceNodeFixture};

    #[test]
    fn session_evaluates_accounts_fixture_through_oxcalc_context() {
        let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let mut session = TreeWorkspaceSession::from_model(&model).unwrap();

        session.recalculate().unwrap();
        let state = session.workspace_state().unwrap();

        assert_eq!(state.workspace_id, "accounts");
        assert_eq!(
            state
                .node(&NodeId::new("Accounts.2005.Q1.Income"))
                .and_then(|node| match &node.computed_value {
                    NodeValueProjection::Scalar(value) => Some(value.as_str()),
                    _ => None,
                }),
            Some("2")
        );
        assert_eq!(
            state
                .node(&NodeId::new("Accounts.2005.Q1.Net"))
                .and_then(|node| match &node.computed_value {
                    NodeValueProjection::Scalar(value) => Some(value.as_str()),
                    _ => None,
                }),
            Some("1.6")
        );
    }

    #[test]
    fn session_structural_edits_flow_through_oxcalc_context() {
        let fixture = WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "session-edits".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                WorkspaceNodeFixture {
                    node_id: "Root".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.A".to_string(),
                    formula: "=3".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.B".to_string(),
                    formula: "=A+1".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        };
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let mut session = TreeWorkspaceSession::from_model(&model).unwrap();

        session.recalculate().unwrap();
        assert_eq!(
            scalar_value(&session.workspace_state().unwrap(), "Root.B"),
            Some("4")
        );

        session
            .edit_formula(&NodeId::new("Root.A"), "=4")
            .expect("formula edit reaches OxCalc");
        session.recalculate().unwrap();
        assert_eq!(
            scalar_value(&session.workspace_state().unwrap(), "Root.B"),
            Some("5")
        );

        let c = session
            .add_node(Some(&NodeId::new("Root")), "C", "=B+1")
            .unwrap();
        assert_eq!(c.as_str(), "Root.C");
        session.recalculate().unwrap();
        assert_eq!(
            scalar_value(&session.workspace_state().unwrap(), "Root.C"),
            Some("6")
        );

        let renamed = session.rename_node(&NodeId::new("Root.C"), "D").unwrap();
        assert_eq!(renamed.as_str(), "Root.D");
        let moved = session
            .move_node(&NodeId::new("Root.D"), None, None)
            .expect("move to engine root");
        assert_eq!(moved.as_str(), "D");
        session.reorder_node(&NodeId::new("Root.B"), 0).unwrap();
        session.delete_node(&NodeId::new("D")).unwrap();

        let state = session.workspace_state().unwrap();
        assert!(state.node(&NodeId::new("D")).is_none());
        assert_eq!(state.root_paths, vec![NodeId::new("Root")]);
        assert_eq!(
            state.node(&NodeId::new("Root")).unwrap().children[0].as_str(),
            "Root.B"
        );
    }

    #[test]
    fn session_snapshot_roundtrip_preserves_oxcalc_identity() {
        let fixture = WorkspaceFixture::from_repo_fixture("tables").unwrap();
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let session = TreeWorkspaceSession::from_model(&model).unwrap();

        let snapshot = session.export_workspace_snapshot().unwrap();
        let imported =
            TreeWorkspaceSession::from_workspace_snapshot(snapshot, model.profile.as_str())
                .unwrap();

        let state = imported.workspace_state().unwrap();
        let table_node = state.node(&NodeId::new("SalesTable")).unwrap();
        assert_eq!(table_node.content_kind, FrameworkContentKind::Empty);
        assert_eq!(table_node.computed_value, NodeValueProjection::Unevaluated);
    }

    #[test]
    fn dnatree_document_roundtrip_reopens_oxcalc_snapshot_and_selection() {
        let fixture = WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "save-reopen".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                WorkspaceNodeFixture {
                    node_id: "Root".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.A".to_string(),
                    formula: "=3".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.B".to_string(),
                    formula: "=A+1".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        };
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let mut session = TreeWorkspaceSession::from_model(&model).unwrap();
        session.recalculate().unwrap();

        let document = session
            .export_dnatree_document(Some(&NodeId::new("Root.B")))
            .unwrap();
        let json = serde_json::to_string_pretty(&document).unwrap();
        let reparsed: DnaTreeWorkspaceDocument = serde_json::from_str(&json).unwrap();
        let (mut reopened, selected_node) =
            TreeWorkspaceSession::from_dnatree_document(reparsed).unwrap();

        assert_eq!(selected_node.as_ref().map(NodeId::as_str), Some("Root.B"));
        assert_eq!(
            scalar_value(&reopened.workspace_state().unwrap(), "Root.B"),
            Some("4")
        );

        reopened.recalculate().unwrap();
        assert_eq!(
            scalar_value(&reopened.workspace_state().unwrap(), "Root.B"),
            Some("4")
        );

        reopened
            .edit_formula(&NodeId::new("Root.A"), "=4")
            .expect("reopened .dnatree document remains bridge-ready");
        reopened.recalculate().unwrap();
        assert_eq!(
            scalar_value(&reopened.workspace_state().unwrap(), "Root.B"),
            Some("5")
        );
    }

    fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
        state
            .node(&NodeId::new(node_id))
            .and_then(|node| match &node.computed_value {
                NodeValueProjection::Scalar(value) => Some(value.as_str()),
                _ => None,
            })
    }
}

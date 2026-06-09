use std::collections::{BTreeMap, BTreeSet};

use dnatreecalc_skin_framework::{
    AuthoringScope, BindingDiagnosticProjection, CalcRunProjection, CalcRunStateProjection,
    DependencyDescriptorProjection, DependencyEdgeProjection, DependencyGraphProjection,
    DependencyKindProjection, DerivationHoleBindingProjection, DerivationInvocationProjection,
    DerivationOxfmlTraceEventProjection, DerivationPreparedArgumentProjection,
    DerivationTemplateHoleProjection, DerivationTemplateSelectionProjection,
    DerivationTraceProjection, EffectiveFormatProjection, FormatSourceProjection,
    FormulaBindPreviewDiagnosticProjection, FormulaBindPreviewDiagnosticStage,
    FormulaBindPreviewInputKind, FormulaBindPreviewProfileViolationKindProjection,
    FormulaBindPreviewProfileViolationProjection, FormulaBindPreviewProjection,
    InitialNodeContentProjection, InvalidationReasonProjection,
    MutationImpactBlockedReasonProjection, MutationImpactIntentProjection,
    MutationImpactProjection, NameCollisionProjection, NodeAttributePatch, NodeCalcStateProjection,
    NodeContentKind as FrameworkContentKind, NodeId, NodeInvalidationProjection, NodeKey,
    NodeNoteProjection, NodeValueProjection, NodeView, NoteSourceProjection, PhaseKeyProjection,
    RecalcPlanInvalidationProjection, RecalcPlanMutation, RecalcPlanProjection,
    ReferenceResolutionProjection, ReferenceTargetProjection, RuntimeEffectFamilyProjection,
    RuntimeEffectProjection, RuntimeOverlayKindProjection, RuntimeOverlayProjection,
    SourceSpanProjection, TableAnchorProjection, TableCellInput, TableCellProjection,
    TableCellRegionProjection, TableCellsProjection, TableColumnBodyProjection,
    TableColumnProjection, TableDependencyFactBlockerProjection, TableDependencyFactKindProjection,
    TableDependencyFactProjection, TableDependencyFactStatusProjection,
    TableFormulaBindPreviewProjection, TableFormulaMetadataProjection, TableProjection,
    TableRowInput, TableRowProjection, TreeReferenceCollectionFamilyProjection,
    TreeReferenceCollectionProjection, WorkspaceRevisionProjection, WorkspaceState,
};
use oxcalc_core::consumer::OxCalcTreeRunState;
use oxcalc_core::consumer::{
    OxCalcTreeCalculationOutcome, OxCalcTreeContext, OxCalcTreeContextError,
    OxCalcTreeContextOptions, OxCalcTreeDryBindDiagnosticStage, OxCalcTreeDryBindInputKind,
    OxCalcTreeDryBindProfileViolationKind, OxCalcTreeDryBindVerdict, OxCalcTreeEdit,
    OxCalcTreeEditResult, OxCalcTreeEditTransaction, OxCalcTreeHostCapabilitySnapshot,
    OxCalcTreeNodeCreate, OxCalcTreeNodeView, OxCalcTreePreviewMutation, OxCalcTreeRuntimePolicy,
    OxCalcTreeWorkspaceCreate, OxCalcTreeWorkspaceId, OxCalcTreeWorkspaceSnapshot,
    TransactionRecalcPolicy,
};
use oxcalc_core::coordinator::{RuntimeEffect, RuntimeEffectFamily};
use oxcalc_core::dependency::{
    DependencyDescriptor, DependencyDescriptorKind, InvalidationReasonKind,
    TreeReferenceCollectionDependency, TreeReferenceCollectionFamily,
};
use oxcalc_core::recalc::NodeCalcState;
use oxcalc_core::recalc::{OverlayEntry, OverlayKind};
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::structured_table::{
    StructuredTableDependencyFact, StructuredTableDependencyFactKind,
    StructuredTableDependencyFactStatus, StructuredTableLoweringBlocker,
    TreeCalcDynamicTableRebindReport, TreeCalcDynamicTableRebindRequest,
    TreeCalcTableBodyCellNodeBinding, TreeCalcTableColumnBodyMetadata,
    TreeCalcTableColumnFormulaRuntimeRequest, TreeCalcTableColumnSnapshot,
    TreeCalcTableFormulaMetadata, TreeCalcTableFormulaRuntimeContext,
    TreeCalcTableFormulaRuntimeReport, TreeCalcTableNodeSnapshot, TreeCalcTableRowId,
    TreeCalcTableSparseValue, TreeCalcTableTotalsCellNodeBinding, TreeCalcTableUpdateScenarioKind,
    TreeCalcTableVirtualAnchor, evaluate_treecalc_table_column_formula_rows,
    evaluate_treecalc_table_totals_formula,
};
use oxcalc_core::treecalc::{
    DerivationInvocationTraceNode, DerivationPreparedArgumentTrace, DerivationTraceRecord,
    LocalTreeCalcPhaseKey,
};
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeValueLiteralizationResult};
use oxfml_core::format::{oxfml_en_us_format_profile, render_with_code};
use oxfunc_core::locale_format::WorkbookDateSystem;
use oxfunc_core::value::{CalcValue, CoreValue, ExcelText};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeWorkspaceCollectionDependencyProjection {
    pub family: String,
    pub source_reference_handle: String,
    pub base_node: Option<NodeId>,
    pub membership_version: String,
    pub order_version: String,
    pub members: Vec<NodeId>,
}

pub struct TreeWorkspaceSession {
    context: OxCalcTreeContext,
    workspace_id: OxCalcTreeWorkspaceId,
    profile: &'static str,
    engine_root_id: TreeNodeId,
    node_ids: BTreeMap<NodeId, TreeNodeId>,
    node_paths_by_tree_id: BTreeMap<TreeNodeId, NodeId>,
    display_order: Vec<NodeId>,
    recalc_count: usize,
    last_outcome: Option<OxCalcTreeCalculationOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeWorkspaceTransactionEdit<T> {
    pub result: T,
    pub transaction_id: String,
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
            node_paths_by_tree_id: BTreeMap::new(),
            display_order: Vec::new(),
            recalc_count: 0,
            last_outcome: None,
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
            session
                .node_paths_by_tree_id
                .insert(tree_node_id, node_id.clone());
            session.display_order.push(node_id.clone());

            if let Some(table) = &node.table {
                let (body_cell_nodes, totals_cell_nodes) =
                    session.create_table_cell_nodes_from_fixture(&node_id, tree_node_id, table)?;
                let snapshot = table_snapshot_from_fixture(
                    model.workspace_id.as_str(),
                    tree_node_id,
                    path.as_str(),
                    table,
                    body_cell_nodes,
                    totals_cell_nodes,
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
            node_paths_by_tree_id: BTreeMap::new(),
            display_order: Vec::new(),
            recalc_count: 0,
            last_outcome: None,
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

    fn create_table_cell_nodes_from_fixture(
        &mut self,
        table_path: &NodeId,
        table_node_id: TreeNodeId,
        table: &TableNodeFixture,
    ) -> Result<
        (
            Vec<TreeCalcTableBodyCellNodeBinding>,
            Vec<TreeCalcTableTotalsCellNodeBinding>,
        ),
        TreeWorkspaceSessionError,
    > {
        let mut rows = table.rows.clone();
        rows.sort_by_key(|row| row.ordinal);
        let mut columns = table.columns.clone();
        columns.sort_by_key(|column| column.ordinal);

        let mut body_cell_nodes = Vec::new();
        for column in &columns {
            for row in &rows {
                let content = table_body_cell_content(column, row.row_id.as_str());
                let Some(content) = content else {
                    continue;
                };
                let symbol = table_body_cell_symbol(row.ordinal, column.ordinal);
                let node_id = self.context.add_node(
                    &self.workspace_id,
                    OxCalcTreeNodeCreate::new(symbol.clone(), content)
                        .with_meta(true)
                        .under(table_node_id),
                )?;
                self.register_generated_node(table_path, &symbol, node_id);
                body_cell_nodes.push(TreeCalcTableBodyCellNodeBinding {
                    row_id: TreeCalcTableRowId(row.row_id.clone()),
                    column_id: column.column_id.clone(),
                    node_id,
                });
            }
        }

        Ok((body_cell_nodes, Vec::new()))
    }

    fn register_generated_node(
        &mut self,
        parent_path: &NodeId,
        symbol: &str,
        tree_node_id: TreeNodeId,
    ) {
        let node_id = NodeId::new(format!("{}.{}", parent_path.as_str(), symbol));
        self.node_ids.insert(node_id.clone(), tree_node_id);
        self.node_paths_by_tree_id.insert(tree_node_id, node_id);
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
        self.last_outcome = Some(outcome.clone());
        Ok(outcome)
    }

    pub fn preview_recalc_plan(
        &self,
        mutations: &[RecalcPlanMutation],
    ) -> Result<RecalcPlanProjection, TreeWorkspaceSessionError> {
        let mut engine_mutations = Vec::new();
        for mutation in mutations {
            engine_mutations.extend(self.engine_preview_mutations(mutation)?);
        }
        let plan = self
            .context
            .plan_invalidation(&self.workspace_id, &engine_mutations)?;
        let invalidated_nodes = plan
            .invalidated_nodes
            .into_iter()
            .filter(|entry| entry.node_id != self.engine_root_id)
            .map(|entry| {
                Ok(RecalcPlanInvalidationProjection {
                    node: self.node_id_for_tree_node(entry.node_id)?,
                    node_key: node_key_for_tree_node(entry.node_id),
                    requires_rebind: entry.requires_rebind,
                    reasons: entry
                        .reasons
                        .into_iter()
                        .map(invalidation_reason_projection_for)
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, TreeWorkspaceSessionError>>()?;
        let evaluation_order = plan
            .evaluation_order
            .into_iter()
            .filter(|node_id| *node_id != self.engine_root_id)
            .map(|node_id| self.node_id_for_tree_node(node_id))
            .collect::<Result<Vec<_>, _>>()?;
        let requires_rebind = plan
            .requires_rebind
            .into_iter()
            .filter(|node_id| *node_id != self.engine_root_id)
            .map(|node_id| self.node_id_for_tree_node(node_id))
            .collect::<Result<Vec<_>, _>>()?;
        let cycle_risk = plan
            .cycle_risk
            .into_iter()
            .map(|group| {
                group
                    .into_iter()
                    .filter(|node_id| *node_id != self.engine_root_id)
                    .map(|node_id| self.node_id_for_tree_node(node_id))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RecalcPlanProjection {
            estimated_node_count: invalidated_nodes.len(),
            invalidated_nodes,
            evaluation_order,
            requires_rebind,
            cycle_risk,
        })
    }

    pub fn preview_formula_bind(
        &self,
        node: &NodeId,
        content: impl Into<String>,
    ) -> Result<FormulaBindPreviewProjection, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let verdict =
            self.context
                .dry_bind_node_formula_text(&self.workspace_id, tree_node_id, content)?;
        let preview = formula_bind_preview_from_oxcalc_verdict(verdict);
        Ok(FormulaBindPreviewProjection {
            node: node.clone(),
            node_key: node_key_for_tree_node(tree_node_id),
            input_kind: preview.input_kind,
            legal: preview.legal,
            diagnostics: preview.diagnostics,
            profile_violations: preview.profile_violations,
        })
    }

    pub fn preview_table_column_formula_bind(
        &self,
        table: &NodeId,
        column_id: &str,
        content: impl Into<String>,
    ) -> Result<TableFormulaBindPreviewProjection, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let verdict = self.context.dry_bind_table_column_formula_text(
            &self.workspace_id,
            table_node_id,
            column_id,
            content,
        )?;
        let preview = formula_bind_preview_from_oxcalc_verdict(verdict);
        Ok(TableFormulaBindPreviewProjection {
            table: table.clone(),
            table_key: node_key_for_tree_node(table_node_id),
            table_id: table_view.table_id,
            column_id: column_id.to_string(),
            region: TableCellRegionProjection::Body,
            input_kind: preview.input_kind,
            legal: preview.legal,
            diagnostics: preview.diagnostics,
            profile_violations: preview.profile_violations,
        })
    }

    pub fn preview_new_table_column_formula_bind(
        &self,
        table: &NodeId,
        column_id: &str,
        column_name: &str,
        content: impl Into<String>,
    ) -> Result<TableFormulaBindPreviewProjection, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let verdict = self.context.dry_bind_new_table_column_formula_text(
            &self.workspace_id,
            table_node_id,
            column_id,
            column_name,
            content,
        )?;
        let preview = formula_bind_preview_from_oxcalc_verdict(verdict);
        Ok(TableFormulaBindPreviewProjection {
            table: table.clone(),
            table_key: node_key_for_tree_node(table_node_id),
            table_id: table_view.table_id,
            column_id: column_id.to_string(),
            region: TableCellRegionProjection::Body,
            input_kind: preview.input_kind,
            legal: preview.legal,
            diagnostics: preview.diagnostics,
            profile_violations: preview.profile_violations,
        })
    }

    pub fn preview_table_totals_formula_bind(
        &self,
        table: &NodeId,
        column_id: &str,
        content: impl Into<String>,
    ) -> Result<TableFormulaBindPreviewProjection, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let verdict = self.context.dry_bind_table_totals_formula_text(
            &self.workspace_id,
            table_node_id,
            column_id,
            content,
        )?;
        let preview = formula_bind_preview_from_oxcalc_verdict(verdict);
        Ok(TableFormulaBindPreviewProjection {
            table: table.clone(),
            table_key: node_key_for_tree_node(table_node_id),
            table_id: table_view.table_id,
            column_id: column_id.to_string(),
            region: TableCellRegionProjection::Totals,
            input_kind: preview.input_kind,
            legal: preview.legal,
            diagnostics: preview.diagnostics,
            profile_violations: preview.profile_violations,
        })
    }

    pub fn preview_content_edit_impact(
        &self,
        node: &NodeId,
        content: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let content = content.into();
        let bind = self.preview_formula_bind(node, content.clone())?;
        let invalidation_plan = self.preview_recalc_plan(&[RecalcPlanMutation::EditContent {
            node: node.clone(),
            content: content.clone(),
        }])?;
        let blocked_reason = mutation_impact_blocked_reason_for_bind(&bind);
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::EditContent {
                node: node.clone(),
                content,
            },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: bind.profile_violations,
            bind_diagnostics: bind.diagnostics,
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| entry.node != *node)
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents: Vec::new(),
            collisions: Vec::new(),
            invalidation_plan,
        })
    }

    pub fn preview_scoped_content_edit_impact(
        &self,
        scope: AuthoringScope,
        content: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let content = content.into();
        let state = self.workspace_state()?;
        let target_keys = state.expand_authoring_scope(&scope).map_err(|error| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            }
        })?;
        let target_nodes = target_keys
            .iter()
            .map(|key| {
                state
                    .node_by_key(key)
                    .map(|node| node.id.clone())
                    .ok_or_else(|| TreeWorkspaceSessionError::ProjectionOutOfSync {
                        node: format!("node key {key}"),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bind_previews = target_nodes
            .iter()
            .map(|node| self.preview_formula_bind(node, content.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let invalidation_mutations = target_nodes
            .iter()
            .map(|node| RecalcPlanMutation::EditContent {
                node: node.clone(),
                content: content.clone(),
            })
            .collect::<Vec<_>>();
        let invalidation_plan = self.preview_recalc_plan(&invalidation_mutations)?;
        let blocked_reason = mutation_impact_blocked_reason_for_binds(&bind_previews);
        let target_node_set = target_nodes.iter().cloned().collect::<BTreeSet<_>>();
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::EditScopedContent { scope, content },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: bind_previews
                .iter()
                .flat_map(|bind| bind.profile_violations.clone())
                .collect(),
            bind_diagnostics: bind_previews
                .iter()
                .flat_map(|bind| bind.diagnostics.clone())
                .collect(),
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| !target_node_set.contains(&entry.node))
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents: Vec::new(),
            collisions: Vec::new(),
            invalidation_plan,
        })
    }

    pub fn preview_rename_node_impact(
        &self,
        node: &NodeId,
        new_symbol: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let new_symbol = new_symbol.into();
        self.tree_node_id(node.as_str())?;
        let attempted = renamed_node_path(node, &new_symbol);
        let collisions = self.name_collisions_for_attempt(node, &attempted);
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::RenameNode { node: node.clone() }])?;
        let blocked_reason = (!collisions.is_empty())
            .then_some(MutationImpactBlockedReasonProjection::NameCollision);
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::RenameNode {
                node: node.clone(),
                new_symbol,
            },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| entry.node != *node)
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents: Vec::new(),
            collisions,
            invalidation_plan,
        })
    }

    pub fn preview_move_node_impact(
        &self,
        node: &NodeId,
        new_parent: Option<&NodeId>,
        new_index: Option<usize>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        self.tree_node_id(node.as_str())?;
        if let Some(parent) = new_parent {
            self.tree_node_id(parent.as_str())?;
        }
        let state = self.workspace_state()?;
        let invalid_drop = self.move_would_target_self_or_descendant(&state, node, new_parent)?;
        let attempted = moved_node_path(node, new_parent);
        let collisions = self.name_collisions_for_attempt(node, &attempted);
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::MoveNode { node: node.clone() }])?;
        let blocked_reason = if invalid_drop {
            Some(MutationImpactBlockedReasonProjection::InvalidDrop)
        } else if !collisions.is_empty() {
            Some(MutationImpactBlockedReasonProjection::NameCollision)
        } else {
            None
        };
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::MoveNode {
                node: node.clone(),
                new_parent: new_parent.cloned(),
                new_index,
            },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| entry.node != *node)
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents: Vec::new(),
            collisions,
            invalidation_plan,
        })
    }

    pub fn preview_add_node_impact(
        &self,
        parent: Option<&NodeId>,
        symbol: impl Into<String>,
        initial: InitialNodeContentProjection,
        is_meta: bool,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let parent_tree_node_id = match parent {
            Some(parent) => self.tree_node_id(parent.as_str())?,
            None => self.engine_root_id,
        };
        let symbol = symbol.into();
        let attempted = added_node_path(parent, &symbol);
        let collisions = self.name_collisions_for_new_node(&attempted);
        let bind = if collisions.is_empty() {
            self.preview_add_node_initial_bind(parent_tree_node_id, &symbol, &initial, is_meta)?
        } else {
            None
        };
        let blocked_reason = if !collisions.is_empty() {
            Some(MutationImpactBlockedReasonProjection::NameCollision)
        } else if let Some(bind) = &bind {
            mutation_impact_blocked_reason_for_bind_payload(bind)
        } else if initial.supported_content().is_some() {
            None
        } else {
            Some(MutationImpactBlockedReasonProjection::UnsupportedInitialContent)
        };
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::AddNode {
                parent: parent.cloned(),
                symbol,
                initial,
                is_meta,
            },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: bind
                .as_ref()
                .map(|bind| bind.profile_violations.clone())
                .unwrap_or_default(),
            bind_diagnostics: bind.map(|bind| bind.diagnostics).unwrap_or_default(),
            requires_rebind: Vec::new(),
            affected_refs: Vec::new(),
            orphaned_dependents: Vec::new(),
            collisions,
            invalidation_plan: RecalcPlanProjection::default(),
        })
    }

    fn preview_add_node_initial_bind(
        &self,
        parent_tree_node_id: TreeNodeId,
        symbol: &str,
        initial: &InitialNodeContentProjection,
        is_meta: bool,
    ) -> Result<Option<FormulaBindPreviewPayload>, TreeWorkspaceSessionError> {
        let content = match initial {
            InitialNodeContentProjection::Literal { content } => content.clone(),
            InitialNodeContentProjection::InheritColumnFormula { table, column_id } => {
                self.inherited_column_formula_content(table, column_id)?
            }
            InitialNodeContentProjection::Empty
            | InitialNodeContentProjection::TemplateBound { .. } => {
                return Ok(None);
            }
        };
        if !content.trim_start().starts_with('=') {
            return Ok(None);
        }
        let verdict = self.context.dry_bind_new_node_formula_text(
            &self.workspace_id,
            OxCalcTreeNodeCreate::new(symbol, content)
                .under(parent_tree_node_id)
                .with_meta(is_meta),
        )?;
        Ok(Some(formula_bind_preview_from_oxcalc_verdict(verdict)))
    }

    fn inherited_column_formula_content(
        &self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<String, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let TreeCalcTableColumnBodyMetadata::Formula(formula) = &column.body_metadata else {
            return Err(TreeWorkspaceSessionError::ConstantTableColumnFormulaEdit {
                table: table.to_string(),
                column_id: column_id.to_string(),
            });
        };
        Ok(formula.formula_text.clone())
    }

    pub fn preview_delete_node_impact(
        &self,
        node: &NodeId,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        self.tree_node_id(node.as_str())?;
        let state = self.workspace_state()?;
        let deleted_keys = self.subtree_keys_for_state(&state, node)?;
        let deleted_nodes = deleted_keys
            .iter()
            .filter_map(|key| state.node_by_key(key).map(|node| node.id.clone()))
            .collect::<BTreeSet<_>>();
        let orphaned_dependents = orphaned_dependents_for_deleted_keys(&state, &deleted_keys);
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::DeleteNode { node: node.clone() }])?;
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::DeleteNode { node: node.clone() },
            legal: true,
            blocked_reason: None,
            profile_violations: Vec::new(),
            bind_diagnostics: Vec::new(),
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| !deleted_nodes.contains(&entry.node))
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents,
            collisions: Vec::new(),
            invalidation_plan,
        })
    }

    pub fn preview_new_table_column_formula_impact(
        &self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        formula_text: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        let formula_text = formula_text.into();
        let bind = self.preview_new_table_column_formula_bind(
            table,
            &column_id,
            &name,
            formula_text.clone(),
        )?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::AddTableFormulaColumn {
                table: table.clone(),
                column_id: column_id.clone(),
                name: name.clone(),
                formula_text: formula_text.clone(),
            }])?;
        let blocked_reason = mutation_impact_blocked_reason_for_table_bind(&bind);
        Ok(MutationImpactProjection {
            intent: MutationImpactIntentProjection::AddTableFormulaColumn {
                table: table.clone(),
                column_id,
                name,
                formula_text,
            },
            legal: blocked_reason.is_none(),
            blocked_reason,
            profile_violations: bind.profile_violations,
            bind_diagnostics: bind.diagnostics,
            requires_rebind: invalidation_plan.requires_rebind.clone(),
            affected_refs: invalidation_plan
                .invalidated_nodes
                .iter()
                .filter(|entry| entry.node != *table)
                .map(|entry| entry.node.clone())
                .collect(),
            orphaned_dependents: Vec::new(),
            collisions: Vec::new(),
            invalidation_plan,
        })
    }

    pub fn preview_add_table_row_impact(
        &self,
        table: &NodeId,
        row_id: impl Into<String>,
        values: Vec<TableCellInput>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        let blocked_reason = self.table_row_add_blocker(table, &row_id, &values)?;
        let invalidation_plan = if blocked_reason.is_none() {
            self.preview_recalc_plan(&[RecalcPlanMutation::AddTableRow {
                table: table.clone(),
                row_id: row_id.clone(),
                values: values.clone(),
            }])?
        } else {
            RecalcPlanProjection::default()
        };
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::AddTableRow {
                table: table.clone(),
                row_id,
                values,
            },
            blocked_reason,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_delete_table_row_impact(
        &self,
        table: &NodeId,
        row_id: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        self.table_row_index(table, &row_id)?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::DeleteTableRow {
                table: table.clone(),
                row_id: row_id.clone(),
            }])?;
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::DeleteTableRow {
                table: table.clone(),
                row_id,
            },
            None,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_rename_table_row_impact(
        &self,
        table: &NodeId,
        row_id: impl Into<String>,
        new_row_id: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        let new_row_id = new_row_id.into();
        self.table_row_index(table, &row_id)?;
        let blocked_reason = if row_id != new_row_id && self.table_has_row(table, &new_row_id)? {
            Some(MutationImpactBlockedReasonProjection::TableCollision)
        } else {
            None
        };
        let invalidation_plan = if blocked_reason.is_none() {
            self.preview_recalc_plan(&[RecalcPlanMutation::RenameTableRow {
                table: table.clone(),
                row_id: row_id.clone(),
                new_row_id: new_row_id.clone(),
            }])?
        } else {
            RecalcPlanProjection::default()
        };
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::RenameTableRow {
                table: table.clone(),
                row_id,
                new_row_id,
            },
            blocked_reason,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_reorder_table_row_impact(
        &self,
        table: &NodeId,
        row_id: impl Into<String>,
        new_index: usize,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        self.table_row_index(table, &row_id)?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::ReorderTableRow {
                table: table.clone(),
                row_id: row_id.clone(),
                new_index,
            }])?;
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::ReorderTableRow {
                table: table.clone(),
                row_id,
                new_index,
            },
            None,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_add_table_column_impact(
        &self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        values: Vec<TableRowInput>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        let blocked_reason = self.table_column_add_blocker(table, &column_id, &values)?;
        let invalidation_plan = if blocked_reason.is_none() {
            self.preview_recalc_plan(&[RecalcPlanMutation::AddTableColumn {
                table: table.clone(),
                column_id: column_id.clone(),
                name: name.clone(),
                values: values.clone(),
            }])?
        } else {
            RecalcPlanProjection::default()
        };
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::AddTableColumn {
                table: table.clone(),
                column_id,
                name,
                values,
            },
            blocked_reason,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_delete_table_column_impact(
        &self,
        table: &NodeId,
        column_id: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        self.table_column_index(table, &column_id)?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::DeleteTableColumn {
                table: table.clone(),
                column_id: column_id.clone(),
            }])?;
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::DeleteTableColumn {
                table: table.clone(),
                column_id,
            },
            None,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_rename_table_column_impact(
        &self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        self.table_column_index(table, &column_id)?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::RenameTableColumn {
                table: table.clone(),
                column_id: column_id.clone(),
                name: name.clone(),
            }])?;
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::RenameTableColumn {
                table: table.clone(),
                column_id,
                name,
            },
            None,
            invalidation_plan,
            table,
        ))
    }

    pub fn preview_reorder_table_column_impact(
        &self,
        table: &NodeId,
        column_id: impl Into<String>,
        new_index: usize,
    ) -> Result<MutationImpactProjection, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        self.table_column_index(table, &column_id)?;
        let invalidation_plan =
            self.preview_recalc_plan(&[RecalcPlanMutation::ReorderTableColumn {
                table: table.clone(),
                column_id: column_id.clone(),
                new_index,
            }])?;
        Ok(table_mutation_impact(
            MutationImpactIntentProjection::ReorderTableColumn {
                table: table.clone(),
                column_id,
                new_index,
            },
            None,
            invalidation_plan,
            table,
        ))
    }

    fn apply_single_edit_transaction(
        &mut self,
        edit: OxCalcTreeEdit,
        recalc_policy: TransactionRecalcPolicy,
    ) -> Result<String, TreeWorkspaceSessionError> {
        let outcome = self.context.apply_edit_transaction(
            OxCalcTreeEditTransaction::new(self.workspace_id.clone())
                .with_edit(edit)
                .with_recalc_policy(recalc_policy),
        )?;
        if let Some(calculation) = outcome.calculation {
            self.recalc_count += 1;
            self.last_outcome = Some(calculation);
        } else {
            self.last_outcome = None;
        }
        Ok(outcome.transaction_id.to_string())
    }

    fn apply_table_snapshot_transaction(
        &mut self,
        table_node_id: TreeNodeId,
        snapshot: TreeCalcTableNodeSnapshot,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::SetNodeTable {
                node_id: table_node_id,
                snapshot,
            },
            TransactionRecalcPolicy::ApplyOnly,
        )?;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    fn apply_table_snapshot_delete_nodes_transaction(
        &mut self,
        table_node_id: TreeNodeId,
        snapshot: TreeCalcTableNodeSnapshot,
        delete_node_ids: Vec<TreeNodeId>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone()).with_edit(
            OxCalcTreeEdit::SetNodeTable {
                node_id: table_node_id,
                snapshot,
            },
        );
        for node_id in delete_node_ids {
            transaction = transaction.with_edit(OxCalcTreeEdit::DeleteNode { node_id });
        }
        let outcome = self.context.apply_edit_transaction(
            transaction.with_recalc_policy(TransactionRecalcPolicy::ApplyOnly),
        )?;
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
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
        self.last_outcome = None;
        Ok(())
    }

    pub fn edit_formula_transaction(
        &mut self,
        node: &NodeId,
        content: impl Into<String>,
        recalc_policy: TransactionRecalcPolicy,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::SetNodeFormulaText {
                node_id: tree_node_id,
                formula_text: content.into(),
            },
            recalc_policy,
        )?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    pub fn edit_scoped_content_transaction(
        &mut self,
        scope: AuthoringScope,
        content: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let content = content.into();
        let state = self.workspace_state()?;
        let target_keys = state.expand_authoring_scope(&scope).map_err(|error| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            }
        })?;
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::RecalculateAndPublishOnce);
        for key in target_keys {
            let node = state.node_by_key(&key).ok_or_else(|| {
                TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: format!("node key {key}"),
                }
            })?;
            transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                node_id: self.tree_node_id(node.id.as_str())?,
                formula_text: content.clone(),
            });
        }
        let outcome = self.context.apply_edit_transaction(transaction)?;
        if let Some(calculation) = outcome.calculation {
            self.recalc_count += 1;
            self.last_outcome = Some(calculation);
        } else {
            self.last_outcome = None;
        }
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn edit_ordered_content_transaction(
        &mut self,
        scope: AuthoringScope,
        contents: Vec<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let state = self.workspace_state()?;
        let target_keys = state.expand_authoring_scope(&scope).map_err(|error| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            }
        })?;
        if target_keys.len() != contents.len() {
            return Err(TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: format!(
                    "ordered content count {} does not match target count {}",
                    contents.len(),
                    target_keys.len()
                ),
            });
        }
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::RecalculateAndPublishOnce);
        for (key, content) in target_keys.into_iter().zip(contents) {
            let node = state.node_by_key(&key).ok_or_else(|| {
                TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: format!("node key {key}"),
                }
            })?;
            transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                node_id: self.tree_node_id(node.id.as_str())?,
                formula_text: content,
            });
        }
        let outcome = self.context.apply_edit_transaction(transaction)?;
        if let Some(calculation) = outcome.calculation {
            self.recalc_count += 1;
            self.last_outcome = Some(calculation);
        } else {
            self.last_outcome = None;
        }
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn paste_constant_value_transaction(
        &mut self,
        target: AuthoringScope,
        source: NodeKey,
        content: impl Into<String>,
        clear_source_after_paste: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<bool>, TreeWorkspaceSessionError> {
        let transaction = self.paste_constant_values_transaction(
            target,
            vec![(source.clone(), content.into())],
            clear_source_after_paste,
        )?;
        let source_cleared = transaction.result.contains(&source);
        Ok(TreeWorkspaceTransactionEdit {
            result: source_cleared,
            transaction_id: transaction.transaction_id,
        })
    }

    pub fn paste_constant_values_transaction(
        &mut self,
        target: AuthoringScope,
        values: Vec<(NodeKey, String)>,
        clear_sources_after_paste: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<Vec<NodeKey>>, TreeWorkspaceSessionError> {
        let state = self.workspace_state()?;
        let target_keys = state.expand_authoring_scope(&target).map_err(|error| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            }
        })?;
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::RecalculateAndPublishOnce);
        let mut target_key_set = BTreeSet::new();

        let ordered_values = if values.len() == 1 {
            target_keys
                .iter()
                .map(|key| (key.clone(), values[0].1.clone()))
                .collect::<Vec<_>>()
        } else {
            if target_keys.len() != values.len() {
                return Err(TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: format!(
                        "constant value count {} does not match target count {}",
                        values.len(),
                        target_keys.len()
                    ),
                });
            }
            target_keys
                .iter()
                .cloned()
                .zip(values.iter().map(|(_, content)| content.clone()))
                .collect::<Vec<_>>()
        };

        for (key, content) in ordered_values {
            target_key_set.insert(key.clone());
            let node = state.node_by_key(&key).ok_or_else(|| {
                TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: format!("node key {key}"),
                }
            })?;
            transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                node_id: self.tree_node_id(node.id.as_str())?,
                formula_text: content,
            });
        }

        let mut cleared_sources = Vec::new();
        if clear_sources_after_paste {
            let mut seen_sources = BTreeSet::new();
            for (source, _) in values {
                if target_key_set.contains(&source) || !seen_sources.insert(source.clone()) {
                    continue;
                }
                let source_node = state.node_by_key(&source).ok_or_else(|| {
                    TreeWorkspaceSessionError::ProjectionOutOfSync {
                        node: format!("node key {source}"),
                    }
                })?;
                transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                    node_id: self.tree_node_id(source_node.id.as_str())?,
                    formula_text: String::new(),
                });
                cleared_sources.push(source);
            }
        }

        let outcome = self.context.apply_edit_transaction(transaction)?;
        if let Some(calculation) = outcome.calculation {
            self.recalc_count += 1;
            self.last_outcome = Some(calculation);
        } else {
            self.last_outcome = None;
        }
        Ok(TreeWorkspaceTransactionEdit {
            result: cleared_sources,
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn set_number_format_transaction(
        &mut self,
        scope: AuthoringScope,
        number_format_code: Option<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let state = self.workspace_state()?;
        let target_keys = state.expand_authoring_scope(&scope).map_err(|error| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            }
        })?;
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly);
        let views_by_tree_id = self.current_views_by_tree_id()?;

        for key in target_keys {
            let target = state.node_by_key(&key).ok_or_else(|| {
                TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: format!("node key {key}"),
                }
            })?;
            let format_path = NodeId::new(format!("{}.Format", target.id.as_str()));
            let number_format_path = NodeId::new(format!("{}.NumberFormat", format_path.as_str()));

            let format_tree_id =
                if let Some(format_tree_id) = self.node_ids.get(&format_path).copied() {
                    if !views_by_tree_id
                        .get(&format_tree_id)
                        .is_some_and(|view| view.is_meta)
                    {
                        return Err(TreeWorkspaceSessionError::FormatPathReserved {
                            node: format_path.to_string(),
                        });
                    }
                    format_tree_id
                } else if number_format_code.is_some() {
                    let reserved_format_id = self.context.reserve_node_id();
                    transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                        request: OxCalcTreeNodeCreate::new("Format", "")
                            .under(self.tree_node_id(target.id.as_str())?)
                            .with_meta(true)
                            .with_reserved_node_id(reserved_format_id),
                    });
                    reserved_format_id
                } else {
                    continue;
                };

            if let Some(number_format_tree_id) = self.node_ids.get(&number_format_path).copied() {
                if !views_by_tree_id
                    .get(&number_format_tree_id)
                    .is_some_and(|view| view.is_meta)
                {
                    return Err(TreeWorkspaceSessionError::FormatPathReserved {
                        node: number_format_path.to_string(),
                    });
                }
                match &number_format_code {
                    Some(code) => {
                        transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                            node_id: number_format_tree_id,
                            formula_text: code.clone(),
                        });
                    }
                    None => {
                        transaction = transaction.with_edit(OxCalcTreeEdit::DeleteNode {
                            node_id: number_format_tree_id,
                        });
                    }
                }
            } else if let Some(code) = &number_format_code {
                transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                    request: OxCalcTreeNodeCreate::new("NumberFormat", code.clone())
                        .under(format_tree_id)
                        .with_meta(true),
                });
            }
        }

        let outcome = self.context.apply_edit_transaction(transaction)?;
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn set_note_transaction(
        &mut self,
        node: NodeKey,
        note: Option<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let state = self.workspace_state()?;
        let target = state.node_by_key(&node).ok_or_else(|| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: format!("node key {node}"),
            }
        })?;
        let note_path = NodeId::new(format!("{}.Note", target.id.as_str()));
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly);
        let views_by_tree_id = self.current_views_by_tree_id()?;

        if let Some(note_tree_id) = self.node_ids.get(&note_path).copied() {
            if !views_by_tree_id
                .get(&note_tree_id)
                .is_some_and(|view| view.is_meta)
            {
                return Err(TreeWorkspaceSessionError::NotePathReserved {
                    node: note_path.to_string(),
                });
            }
            match note {
                Some(text) => {
                    transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                        node_id: note_tree_id,
                        formula_text: text,
                    });
                }
                None => {
                    transaction = transaction.with_edit(OxCalcTreeEdit::DeleteNode {
                        node_id: note_tree_id,
                    });
                }
            }
        } else if let Some(text) = note {
            transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                request: OxCalcTreeNodeCreate::new("Note", text)
                    .under(self.tree_node_id(target.id.as_str())?)
                    .with_meta(true),
            });
        }

        let outcome = self.context.apply_edit_transaction(transaction)?;
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn set_meta_transaction(
        &mut self,
        node: NodeKey,
        is_meta: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let state = self.workspace_state()?;
        let target = state.node_by_key(&node).ok_or_else(|| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: format!("node key {node}"),
            }
        })?;
        let tree_node_id = self.tree_node_id(target.id.as_str())?;
        let outcome = self.context.apply_edit_transaction(
            OxCalcTreeEditTransaction::new(self.workspace_id.clone())
                .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly)
                .with_edit(OxCalcTreeEdit::SetNodeMeta {
                    node_id: tree_node_id,
                    is_meta,
                }),
        )?;
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn set_node_attributes_transaction(
        &mut self,
        node: NodeKey,
        attrs: NodeAttributePatch,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        for key in attrs.set.keys().chain(attrs.clear.iter()) {
            if !attribute_key_is_path_safe(key) {
                return Err(TreeWorkspaceSessionError::InvalidAttributeKey { key: key.clone() });
            }
        }

        let state = self.workspace_state()?;
        let target = state.node_by_key(&node).ok_or_else(|| {
            TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: format!("node key {node}"),
            }
        })?;
        let attributes_path = NodeId::new(format!("{}.Attributes", target.id.as_str()));
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly);
        let views_by_tree_id = self.current_views_by_tree_id()?;

        let attributes_tree_id =
            if let Some(attributes_tree_id) = self.node_ids.get(&attributes_path).copied() {
                if !views_by_tree_id
                    .get(&attributes_tree_id)
                    .is_some_and(|view| view.is_meta)
                {
                    return Err(TreeWorkspaceSessionError::AttributePathReserved {
                        node: attributes_path.to_string(),
                    });
                }
                Some(attributes_tree_id)
            } else if attrs.set.is_empty() {
                None
            } else {
                let reserved_attributes_id = self.context.reserve_node_id();
                transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                    request: OxCalcTreeNodeCreate::new("Attributes", "")
                        .under(self.tree_node_id(target.id.as_str())?)
                        .with_meta(true)
                        .with_reserved_node_id(reserved_attributes_id),
                });
                Some(reserved_attributes_id)
            };

        for key in &attrs.clear {
            if attrs.set.contains_key(key) {
                continue;
            }
            let attribute_path = NodeId::new(format!("{}.{}", attributes_path.as_str(), key));
            if let Some(attribute_tree_id) = self.node_ids.get(&attribute_path).copied() {
                if !views_by_tree_id
                    .get(&attribute_tree_id)
                    .is_some_and(|view| view.is_meta)
                {
                    return Err(TreeWorkspaceSessionError::AttributePathReserved {
                        node: attribute_path.to_string(),
                    });
                }
                transaction = transaction.with_edit(OxCalcTreeEdit::DeleteNode {
                    node_id: attribute_tree_id,
                });
            }
        }

        if let Some(attributes_tree_id) = attributes_tree_id {
            for (key, value) in attrs.set {
                let attribute_path = NodeId::new(format!("{}.{}", attributes_path.as_str(), key));
                if let Some(attribute_tree_id) = self.node_ids.get(&attribute_path).copied() {
                    if !views_by_tree_id
                        .get(&attribute_tree_id)
                        .is_some_and(|view| view.is_meta)
                    {
                        return Err(TreeWorkspaceSessionError::AttributePathReserved {
                            node: attribute_path.to_string(),
                        });
                    }
                    transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeFormulaText {
                        node_id: attribute_tree_id,
                        formula_text: value,
                    });
                } else {
                    transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                        request: OxCalcTreeNodeCreate::new(key, value)
                            .under(attributes_tree_id)
                            .with_meta(true),
                    });
                }
            }
        }

        let outcome = self.context.apply_edit_transaction(transaction)?;
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
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
        self.node_paths_by_tree_id
            .insert(tree_node_id, node_id.clone());
        self.display_order.push(node_id.clone());
        self.last_outcome = None;
        Ok(node_id)
    }

    pub fn add_node_transaction(
        &mut self,
        parent: Option<&NodeId>,
        symbol: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<NodeId>, TreeWorkspaceSessionError> {
        self.add_node_transaction_with_meta(parent, symbol, content, false)
    }

    pub fn add_node_transaction_with_initial(
        &mut self,
        parent: Option<&NodeId>,
        symbol: impl Into<String>,
        initial: InitialNodeContentProjection,
        is_meta: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<NodeId>, TreeWorkspaceSessionError> {
        let symbol = symbol.into();
        let parent_tree_node_id = match parent {
            Some(parent) => self.tree_node_id(parent.as_str())?,
            None => self.engine_root_id,
        };
        let content =
            self.resolve_add_node_initial_content(parent_tree_node_id, &symbol, &initial, is_meta)?;
        self.add_node_transaction_with_meta(parent, symbol, content, is_meta)
    }

    fn resolve_add_node_initial_content(
        &self,
        parent_tree_node_id: TreeNodeId,
        symbol: &str,
        initial: &InitialNodeContentProjection,
        is_meta: bool,
    ) -> Result<String, TreeWorkspaceSessionError> {
        match initial {
            InitialNodeContentProjection::Empty => Ok(String::new()),
            InitialNodeContentProjection::Literal { content } => {
                if content.trim_start().starts_with('=') {
                    let bind = self.preview_add_node_initial_bind(
                        parent_tree_node_id,
                        symbol,
                        initial,
                        is_meta,
                    )?;
                    if bind.as_ref().is_some_and(|bind| !bind.legal) {
                        return Err(TreeWorkspaceSessionError::InitialContentBindRejected {
                            policy: initial.stable_id().to_string(),
                        });
                    }
                }
                Ok(content.clone())
            }
            InitialNodeContentProjection::InheritColumnFormula { table, column_id } => {
                let content = self.inherited_column_formula_content(table, column_id)?;
                if content.trim_start().starts_with('=') {
                    let bind = self.preview_add_node_initial_bind(
                        parent_tree_node_id,
                        symbol,
                        initial,
                        is_meta,
                    )?;
                    if bind.as_ref().is_some_and(|bind| !bind.legal) {
                        return Err(TreeWorkspaceSessionError::InitialContentBindRejected {
                            policy: initial.stable_id().to_string(),
                        });
                    }
                }
                Ok(content)
            }
            InitialNodeContentProjection::TemplateBound { .. } => {
                Err(TreeWorkspaceSessionError::UnsupportedInitialContent {
                    policy: initial.stable_id().to_string(),
                })
            }
        }
    }

    pub fn add_node_transaction_with_meta(
        &mut self,
        parent: Option<&NodeId>,
        symbol: impl Into<String>,
        content: impl Into<String>,
        is_meta: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<NodeId>, TreeWorkspaceSessionError> {
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

        let outcome = self.context.apply_edit_transaction(
            OxCalcTreeEditTransaction::new(self.workspace_id.clone())
                .with_edit(OxCalcTreeEdit::AddNode {
                    request: OxCalcTreeNodeCreate::new(symbol, content)
                        .under(parent_tree_node_id)
                        .with_meta(is_meta),
                })
                .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly),
        )?;
        let tree_node_id = match outcome.edit_results.as_slice() {
            [OxCalcTreeEditResult::NodeAdded { node_id }] => *node_id,
            _ => {
                return Err(TreeWorkspaceSessionError::ProjectionOutOfSync {
                    node: "OxCalc add-node transaction did not return a created node id"
                        .to_string(),
                });
            }
        };
        if let Some(calculation) = outcome.calculation {
            self.recalc_count += 1;
            self.last_outcome = Some(calculation);
        } else {
            self.last_outcome = None;
        }
        self.node_ids.insert(node_id.clone(), tree_node_id);
        self.node_paths_by_tree_id
            .insert(tree_node_id, node_id.clone());
        self.display_order.push(node_id.clone());
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: node_id,
            transaction_id: outcome.transaction_id.to_string(),
        })
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
        self.last_outcome = None;
        self.node_id_for_tree_node(tree_node_id)
    }

    pub fn rename_node_transaction(
        &mut self,
        node: &NodeId,
        new_symbol: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<NodeId>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::RenameNode {
                node_id: tree_node_id,
                new_symbol: new_symbol.into(),
            },
            TransactionRecalcPolicy::RecalculateAndPublishOnce,
        )?;
        self.refresh_projection_from_context()?;
        let result = self.node_id_for_tree_node(tree_node_id)?;
        Ok(TreeWorkspaceTransactionEdit {
            result,
            transaction_id,
        })
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
        self.last_outcome = None;
        self.node_id_for_tree_node(tree_node_id)
    }

    pub fn move_node_transaction(
        &mut self,
        node: &NodeId,
        new_parent: Option<&NodeId>,
        new_index: Option<usize>,
    ) -> Result<TreeWorkspaceTransactionEdit<NodeId>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let new_parent_id = match new_parent {
            Some(parent) => self.tree_node_id(parent.as_str())?,
            None => self.engine_root_id,
        };
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::MoveNode {
                node_id: tree_node_id,
                new_parent_id,
                new_index,
            },
            TransactionRecalcPolicy::RecalculateAndPublishOnce,
        )?;
        self.refresh_projection_from_context()?;
        let result = self.node_id_for_tree_node(tree_node_id)?;
        Ok(TreeWorkspaceTransactionEdit {
            result,
            transaction_id,
        })
    }

    pub fn reorder_node(
        &mut self,
        node: &NodeId,
        new_index: usize,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context
            .reorder_node(&self.workspace_id, tree_node_id, new_index)?;
        self.last_outcome = None;
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

    pub fn reorder_node_transaction(
        &mut self,
        node: &NodeId,
        new_index: usize,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::ReorderNode {
                node_id: tree_node_id,
                new_index,
            },
            TransactionRecalcPolicy::RecalculateAndPublishOnce,
        )?;
        if let Some(parent) = parent_path(node.as_str()) {
            reorder_child(
                &mut self.display_order,
                &NodeId::new(parent),
                node,
                new_index,
            );
        } else {
            reorder_root(&mut self.display_order, node, new_index);
        }
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    pub fn delete_node(&mut self, node: &NodeId) -> Result<(), TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        self.context.delete_node(&self.workspace_id, tree_node_id)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn delete_node_transaction(
        &mut self,
        node: &NodeId,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node.as_str())?;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::DeleteNode {
                node_id: tree_node_id,
            },
            TransactionRecalcPolicy::RecalculateAndPublishOnce,
        )?;
        self.refresh_projection_from_context()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    pub fn edit_table_cell(
        &mut self,
        table: &NodeId,
        row_id: &str,
        column_id: &str,
        content: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        if matches!(
            column.body_metadata,
            TreeCalcTableColumnBodyMetadata::Formula(_)
        ) {
            return Err(TreeWorkspaceSessionError::FormulaTableCellEdit {
                table: table.to_string(),
                column_id: column_id.to_string(),
            });
        }
        let binding = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .find(|binding| binding.row_id.0 == row_id && binding.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableCell {
                table: table.to_string(),
                row_id: row_id.to_string(),
                column_id: column_id.to_string(),
            })?;
        self.context
            .set_node_formula_text(&self.workspace_id, binding.node_id, content.into())?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn edit_table_cell_transaction(
        &mut self,
        table: &NodeId,
        row_id: &str,
        column_id: &str,
        content: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        if matches!(
            column.body_metadata,
            TreeCalcTableColumnBodyMetadata::Formula(_)
        ) {
            return Err(TreeWorkspaceSessionError::FormulaTableCellEdit {
                table: table.to_string(),
                column_id: column_id.to_string(),
            });
        }
        let binding = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .find(|binding| binding.row_id.0 == row_id && binding.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableCell {
                table: table.to_string(),
                row_id: row_id.to_string(),
                column_id: column_id.to_string(),
            })?;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::SetNodeFormulaText {
                node_id: binding.node_id,
                formula_text: content.into(),
            },
            TransactionRecalcPolicy::ApplyOnly,
        )?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    pub fn add_table_row(
        &mut self,
        table: &NodeId,
        row_id: impl Into<String>,
        values: Vec<TableCellInput>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if table_view.snapshot.rows.iter().any(|row| row.0 == row_id) {
            return Err(TreeWorkspaceSessionError::DuplicateTableRow {
                table: table.to_string(),
                row_id,
            });
        }

        let mut values_by_column = BTreeMap::new();
        for value in values {
            if values_by_column
                .insert(value.column_id.clone(), value.content)
                .is_some()
            {
                return Err(TreeWorkspaceSessionError::DuplicateTableCellInput {
                    table: table.to_string(),
                    column_id: value.column_id,
                });
            }
        }
        for column_id in values_by_column.keys() {
            let column = table_view
                .snapshot
                .columns
                .iter()
                .find(|column| column.column_id == *column_id)
                .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                    table: table.to_string(),
                    column_id: column_id.clone(),
                })?;
            if matches!(
                column.body_metadata,
                TreeCalcTableColumnBodyMetadata::Formula(_)
            ) {
                return Err(TreeWorkspaceSessionError::FormulaTableCellEdit {
                    table: table.to_string(),
                    column_id: column_id.clone(),
                });
            }
        }

        let row_ordinal = u32::try_from(table_view.snapshot.rows.len() + 1).unwrap_or(u32::MAX);
        table_view
            .snapshot
            .rows
            .push(TreeCalcTableRowId(row_id.clone()));

        let mut columns = table_view.snapshot.columns.clone();
        columns.sort_by_key(|column| column.ordinal);
        for column in columns {
            if !matches!(
                column.body_metadata,
                TreeCalcTableColumnBodyMetadata::ConstantCells
            ) {
                continue;
            }
            let content = values_by_column
                .get(&column.column_id)
                .cloned()
                .unwrap_or_default();
            let symbol = table_body_cell_symbol(row_ordinal, column.ordinal);
            let generated_node = NodeId::new(format!("{}.{}", table.as_str(), symbol));
            if self.node_ids.contains_key(&generated_node) {
                return Err(TreeWorkspaceSessionError::DuplicateNodePath {
                    node: generated_node.to_string(),
                });
            }
            let node_id = self.context.add_node(
                &self.workspace_id,
                OxCalcTreeNodeCreate::new(symbol.clone(), content)
                    .with_meta(true)
                    .under(table_node_id),
            )?;
            self.register_generated_node(table, &symbol, node_id);
            table_view
                .snapshot
                .body_cell_nodes
                .push(TreeCalcTableBodyCellNodeBinding {
                    row_id: TreeCalcTableRowId(row_id.clone()),
                    column_id: column.column_id,
                    node_id,
                });
        }

        table_view.snapshot.row_membership_version =
            bumped_table_version(&table_view.snapshot.row_membership_version, "row", &row_id);
        table_view.snapshot.row_order_version =
            bumped_table_version(&table_view.snapshot.row_order_version, "row", &row_id);
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn add_table_row_transaction(
        &mut self,
        table: &NodeId,
        row_id: impl Into<String>,
        values: Vec<TableCellInput>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let row_id = row_id.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if table_view.snapshot.rows.iter().any(|row| row.0 == row_id) {
            return Err(TreeWorkspaceSessionError::DuplicateTableRow {
                table: table.to_string(),
                row_id,
            });
        }

        let mut values_by_column = BTreeMap::new();
        for value in values {
            if values_by_column
                .insert(value.column_id.clone(), value.content)
                .is_some()
            {
                return Err(TreeWorkspaceSessionError::DuplicateTableCellInput {
                    table: table.to_string(),
                    column_id: value.column_id,
                });
            }
        }
        for column_id in values_by_column.keys() {
            let column = table_view
                .snapshot
                .columns
                .iter()
                .find(|column| column.column_id == *column_id)
                .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                    table: table.to_string(),
                    column_id: column_id.clone(),
                })?;
            if matches!(
                column.body_metadata,
                TreeCalcTableColumnBodyMetadata::Formula(_)
            ) {
                return Err(TreeWorkspaceSessionError::FormulaTableCellEdit {
                    table: table.to_string(),
                    column_id: column_id.clone(),
                });
            }
        }

        let row_ordinal = u32::try_from(table_view.snapshot.rows.len() + 1).unwrap_or(u32::MAX);
        table_view
            .snapshot
            .rows
            .push(TreeCalcTableRowId(row_id.clone()));

        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly);
        let mut generated_nodes = Vec::new();
        let mut columns = table_view.snapshot.columns.clone();
        columns.sort_by_key(|column| column.ordinal);
        for column in columns {
            if !matches!(
                column.body_metadata,
                TreeCalcTableColumnBodyMetadata::ConstantCells
            ) {
                continue;
            }
            let content = values_by_column
                .get(&column.column_id)
                .cloned()
                .unwrap_or_default();
            let symbol = table_body_cell_symbol(row_ordinal, column.ordinal);
            let generated_node = NodeId::new(format!("{}.{}", table.as_str(), symbol));
            if self.node_ids.contains_key(&generated_node) {
                return Err(TreeWorkspaceSessionError::DuplicateNodePath {
                    node: generated_node.to_string(),
                });
            }
            let node_id = self.context.reserve_node_id();
            transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                request: OxCalcTreeNodeCreate::new(symbol.clone(), content)
                    .with_meta(true)
                    .under(table_node_id)
                    .with_reserved_node_id(node_id),
            });
            generated_nodes.push((symbol, node_id));
            table_view
                .snapshot
                .body_cell_nodes
                .push(TreeCalcTableBodyCellNodeBinding {
                    row_id: TreeCalcTableRowId(row_id.clone()),
                    column_id: column.column_id,
                    node_id,
                });
        }

        table_view.snapshot.row_membership_version =
            bumped_table_version(&table_view.snapshot.row_membership_version, "row", &row_id);
        table_view.snapshot.row_order_version =
            bumped_table_version(&table_view.snapshot.row_order_version, "row", &row_id);
        transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeTable {
            node_id: table_node_id,
            snapshot: table_view.snapshot,
        });
        let outcome = self.context.apply_edit_transaction(transaction)?;
        for (symbol, node_id) in generated_nodes {
            self.register_generated_node(table, &symbol, node_id);
        }
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn delete_table_row(
        &mut self,
        table: &NodeId,
        row_id: &str,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let row_index = table_view
            .snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;

        let removed_body_node_ids = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .filter(|binding| binding.row_id.0 == row_id)
            .map(|binding| binding.node_id)
            .collect::<Vec<_>>();

        table_view.snapshot.rows.remove(row_index);
        table_view
            .snapshot
            .body_cell_nodes
            .retain(|binding| binding.row_id.0 != row_id);
        table_view.snapshot.row_membership_version = bumped_table_version(
            &table_view.snapshot.row_membership_version,
            "row-removed",
            row_id,
        );
        table_view.snapshot.row_order_version = bumped_table_version(
            &table_view.snapshot.row_order_version,
            "row-removed",
            row_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        for node_id in removed_body_node_ids {
            self.context.delete_node(&self.workspace_id, node_id)?;
        }
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn delete_table_row_transaction(
        &mut self,
        table: &NodeId,
        row_id: &str,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let removed_body_node_ids = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .filter(|binding| binding.row_id.0 == row_id)
            .map(|binding| binding.node_id)
            .collect::<Vec<_>>();
        let snapshot = self.preview_delete_table_row_snapshot(table, row_id)?;
        self.apply_table_snapshot_delete_nodes_transaction(
            table_node_id,
            snapshot,
            removed_body_node_ids,
        )
    }

    pub fn rename_table_row(
        &mut self,
        table: &NodeId,
        row_id: &str,
        new_row_id: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let new_row_id = new_row_id.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let row_index = table_view
            .snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;
        if row_id != new_row_id
            && table_view
                .snapshot
                .rows
                .iter()
                .any(|row| row.0 == new_row_id)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableRow {
                table: table.to_string(),
                row_id: new_row_id,
            });
        }

        table_view.snapshot.rows[row_index] = TreeCalcTableRowId(new_row_id.clone());
        for binding in &mut table_view.snapshot.body_cell_nodes {
            if binding.row_id.0 == row_id {
                binding.row_id = TreeCalcTableRowId(new_row_id.clone());
            }
        }
        table_view.snapshot.row_membership_version = bumped_table_version(
            &table_view.snapshot.row_membership_version,
            "row-renamed",
            row_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn rename_table_row_transaction(
        &mut self,
        table: &NodeId,
        row_id: &str,
        new_row_id: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let new_row_id = new_row_id.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        self.table_row_index(table, row_id)?;
        if row_id != new_row_id && self.table_has_row(table, &new_row_id)? {
            return Err(TreeWorkspaceSessionError::DuplicateTableRow {
                table: table.to_string(),
                row_id: new_row_id,
            });
        }
        let snapshot = self.preview_rename_table_row_snapshot(table, row_id, new_row_id)?;
        self.apply_table_snapshot_transaction(table_node_id, snapshot)
    }

    pub fn reorder_table_row(
        &mut self,
        table: &NodeId,
        row_id: &str,
        new_index: usize,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let row_index = table_view
            .snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;
        let row = table_view.snapshot.rows.remove(row_index);
        let bounded_index = new_index.min(table_view.snapshot.rows.len());
        table_view.snapshot.rows.insert(bounded_index, row);
        table_view.snapshot.row_order_version = bumped_table_version(
            &table_view.snapshot.row_order_version,
            "row-reordered",
            row_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn reorder_table_row_transaction(
        &mut self,
        table: &NodeId,
        row_id: &str,
        new_index: usize,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let snapshot = self.preview_reorder_table_row_snapshot(table, row_id, new_index)?;
        self.apply_table_snapshot_transaction(table_node_id, snapshot)
    }

    pub fn rename_table(
        &mut self,
        table: &NodeId,
        name: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(TreeWorkspaceSessionError::EmptyTableName {
                table: table.to_string(),
            });
        }
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if name != table_view.snapshot.table_name
            && self
                .context
                .workspace_table_views(&self.workspace_id)?
                .iter()
                .any(|view| view.table_node_id != table_node_id && view.table_name == name)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableName {
                name,
                table: table.to_string(),
            });
        }

        table_view.snapshot.table_name = name;
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn rename_table_transaction(
        &mut self,
        table: &NodeId,
        name: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(TreeWorkspaceSessionError::EmptyTableName {
                table: table.to_string(),
            });
        }
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if name != table_view.snapshot.table_name
            && self
                .context
                .workspace_table_views(&self.workspace_id)?
                .iter()
                .any(|view| view.table_node_id != table_node_id && view.table_name == name)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableName {
                name,
                table: table.to_string(),
            });
        }

        table_view.snapshot.table_name = name;
        let transaction_id = self.apply_single_edit_transaction(
            OxCalcTreeEdit::SetNodeTable {
                node_id: table_node_id,
                snapshot: table_view.snapshot,
            },
            TransactionRecalcPolicy::ApplyOnly,
        )?;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id,
        })
    }

    pub fn add_table_column(
        &mut self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        values: Vec<TableRowInput>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if table_view
            .snapshot
            .columns
            .iter()
            .any(|column| column.column_id == column_id)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableColumn {
                table: table.to_string(),
                column_id,
            });
        }

        let mut values_by_row = BTreeMap::new();
        for value in values {
            if values_by_row
                .insert(value.row_id.clone(), value.content)
                .is_some()
            {
                return Err(TreeWorkspaceSessionError::DuplicateTableRowInput {
                    table: table.to_string(),
                    row_id: value.row_id,
                });
            }
        }
        for row_id in values_by_row.keys() {
            if !table_view.snapshot.rows.iter().any(|row| row.0 == *row_id) {
                return Err(TreeWorkspaceSessionError::UnknownTableRow {
                    table: table.to_string(),
                    row_id: row_id.clone(),
                });
            }
        }

        let column_ordinal = table_view
            .snapshot
            .columns
            .iter()
            .map(|column| column.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        for (row_index, _) in table_view.snapshot.rows.iter().enumerate() {
            let row_ordinal = u32::try_from(row_index + 1).unwrap_or(u32::MAX);
            let symbol = table_body_cell_symbol(row_ordinal, column_ordinal);
            let generated_node = NodeId::new(format!("{}.{}", table.as_str(), symbol));
            if self.node_ids.contains_key(&generated_node) {
                return Err(TreeWorkspaceSessionError::DuplicateNodePath {
                    node: generated_node.to_string(),
                });
            }
        }

        table_view
            .snapshot
            .columns
            .push(TreeCalcTableColumnSnapshot {
                column_id: column_id.clone(),
                column_name: name,
                ordinal: column_ordinal,
                body_metadata: TreeCalcTableColumnBodyMetadata::ConstantCells,
                totals_metadata: None,
            });
        for (row_index, row) in table_view.snapshot.rows.iter().enumerate() {
            let row_ordinal = u32::try_from(row_index + 1).unwrap_or(u32::MAX);
            let symbol = table_body_cell_symbol(row_ordinal, column_ordinal);
            let content = values_by_row.get(&row.0).cloned().unwrap_or_default();
            let node_id = self.context.add_node(
                &self.workspace_id,
                OxCalcTreeNodeCreate::new(symbol.clone(), content)
                    .with_meta(true)
                    .under(table_node_id),
            )?;
            self.register_generated_node(table, &symbol, node_id);
            table_view
                .snapshot
                .body_cell_nodes
                .push(TreeCalcTableBodyCellNodeBinding {
                    row_id: row.clone(),
                    column_id: column_id.clone(),
                    node_id,
                });
        }

        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "column",
            &column_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn add_table_column_transaction(
        &mut self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        values: Vec<TableRowInput>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if table_view
            .snapshot
            .columns
            .iter()
            .any(|column| column.column_id == column_id)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableColumn {
                table: table.to_string(),
                column_id,
            });
        }

        let mut values_by_row = BTreeMap::new();
        for value in values {
            if values_by_row
                .insert(value.row_id.clone(), value.content)
                .is_some()
            {
                return Err(TreeWorkspaceSessionError::DuplicateTableRowInput {
                    table: table.to_string(),
                    row_id: value.row_id,
                });
            }
        }
        for row_id in values_by_row.keys() {
            if !table_view.snapshot.rows.iter().any(|row| row.0 == *row_id) {
                return Err(TreeWorkspaceSessionError::UnknownTableRow {
                    table: table.to_string(),
                    row_id: row_id.clone(),
                });
            }
        }

        let column_ordinal = table_view
            .snapshot
            .columns
            .iter()
            .map(|column| column.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        for (row_index, _) in table_view.snapshot.rows.iter().enumerate() {
            let row_ordinal = u32::try_from(row_index + 1).unwrap_or(u32::MAX);
            let symbol = table_body_cell_symbol(row_ordinal, column_ordinal);
            let generated_node = NodeId::new(format!("{}.{}", table.as_str(), symbol));
            if self.node_ids.contains_key(&generated_node) {
                return Err(TreeWorkspaceSessionError::DuplicateNodePath {
                    node: generated_node.to_string(),
                });
            }
        }

        table_view
            .snapshot
            .columns
            .push(TreeCalcTableColumnSnapshot {
                column_id: column_id.clone(),
                column_name: name,
                ordinal: column_ordinal,
                body_metadata: TreeCalcTableColumnBodyMetadata::ConstantCells,
                totals_metadata: None,
            });
        let mut transaction = OxCalcTreeEditTransaction::new(self.workspace_id.clone())
            .with_recalc_policy(TransactionRecalcPolicy::ApplyOnly);
        let mut generated_nodes = Vec::new();
        for (row_index, row) in table_view.snapshot.rows.iter().enumerate() {
            let row_ordinal = u32::try_from(row_index + 1).unwrap_or(u32::MAX);
            let symbol = table_body_cell_symbol(row_ordinal, column_ordinal);
            let content = values_by_row.get(&row.0).cloned().unwrap_or_default();
            let node_id = self.context.reserve_node_id();
            transaction = transaction.with_edit(OxCalcTreeEdit::AddNode {
                request: OxCalcTreeNodeCreate::new(symbol.clone(), content)
                    .with_meta(true)
                    .under(table_node_id)
                    .with_reserved_node_id(node_id),
            });
            generated_nodes.push((symbol, node_id));
            table_view
                .snapshot
                .body_cell_nodes
                .push(TreeCalcTableBodyCellNodeBinding {
                    row_id: row.clone(),
                    column_id: column_id.clone(),
                    node_id,
                });
        }

        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "column",
            &column_id,
        );
        transaction = transaction.with_edit(OxCalcTreeEdit::SetNodeTable {
            node_id: table_node_id,
            snapshot: table_view.snapshot,
        });
        let outcome = self.context.apply_edit_transaction(transaction)?;
        for (symbol, node_id) in generated_nodes {
            self.register_generated_node(table, &symbol, node_id);
        }
        self.last_outcome = None;
        self.refresh_projection_from_context()?;
        self.recalculate()?;
        Ok(TreeWorkspaceTransactionEdit {
            result: (),
            transaction_id: outcome.transaction_id.to_string(),
        })
    }

    pub fn add_table_formula_column(
        &mut self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        formula_text: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let column_id = column_id.into();
        let name = name.into();
        let formula_text = formula_text.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let snapshot = self.preview_formula_column_table_snapshot(
            table,
            table_node_id,
            column_id,
            name,
            formula_text,
        )?;
        self.context
            .set_node_table(&self.workspace_id, table_node_id, snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn add_table_formula_column_transaction(
        &mut self,
        table: &NodeId,
        column_id: impl Into<String>,
        name: impl Into<String>,
        formula_text: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let snapshot = self.preview_formula_column_table_snapshot(
            table,
            table_node_id,
            column_id.into(),
            name.into(),
            formula_text.into(),
        )?;
        self.apply_table_snapshot_transaction(table_node_id, snapshot)
    }

    fn preview_formula_column_table_snapshot(
        &self,
        table: &NodeId,
        table_node_id: TreeNodeId,
        column_id: String,
        name: String,
        formula_text: String,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        if table_view
            .snapshot
            .columns
            .iter()
            .any(|column| column.column_id == column_id)
        {
            return Err(TreeWorkspaceSessionError::DuplicateTableColumn {
                table: table.to_string(),
                column_id,
            });
        }

        let column_ordinal = table_view
            .snapshot
            .columns
            .iter()
            .map(|column| column.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        table_view
            .snapshot
            .columns
            .push(TreeCalcTableColumnSnapshot {
                column_id: column_id.clone(),
                column_name: name,
                ordinal: column_ordinal,
                body_metadata: TreeCalcTableColumnBodyMetadata::Formula(
                    table_formula_metadata_for_column(table.as_str(), &column_id, formula_text),
                ),
                totals_metadata: None,
            });

        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "formula-column",
            &column_id,
        );
        Ok(table_view.snapshot)
    }

    pub fn edit_table_column_formula(
        &mut self,
        table: &NodeId,
        column_id: &str,
        formula_text: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let TreeCalcTableColumnBodyMetadata::Formula(formula) = &mut column.body_metadata else {
            return Err(TreeWorkspaceSessionError::ConstantTableColumnFormulaEdit {
                table: table.to_string(),
                column_id: column_id.to_string(),
            });
        };
        formula.formula_text = formula_text.into();
        formula.formula_text_version = bumped_formula_text_version(&formula.formula_text_version);

        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "formula-edited",
            column_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn edit_table_column_formula_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
        formula_text: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let TreeCalcTableColumnBodyMetadata::Formula(formula) = &mut column.body_metadata else {
            return Err(TreeWorkspaceSessionError::ConstantTableColumnFormulaEdit {
                table: table.to_string(),
                column_id: column_id.to_string(),
            });
        };
        formula.formula_text = formula_text.into();
        formula.formula_text_version = bumped_formula_text_version(&formula.formula_text_version);
        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "formula-edited",
            column_id,
        );
        self.apply_table_snapshot_transaction(table_node_id, table_view.snapshot)
    }

    pub fn set_table_totals_formula(
        &mut self,
        table: &NodeId,
        column_id: &str,
        formula_text: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let formula_text = formula_text.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        match &mut column.totals_metadata {
            Some(formula) => {
                formula.formula_text = formula_text;
                formula.formula_text_version =
                    bumped_formula_text_version(&formula.formula_text_version);
            }
            None => {
                column.totals_metadata = Some(table_formula_metadata_for_totals(
                    table.as_str(),
                    column_id,
                    formula_text,
                ));
            }
        }

        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn set_table_totals_formula_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
        formula_text: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let formula_text = formula_text.into();
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        match &mut column.totals_metadata {
            Some(formula) => {
                formula.formula_text = formula_text;
                formula.formula_text_version =
                    bumped_formula_text_version(&formula.formula_text_version);
            }
            None => {
                column.totals_metadata = Some(table_formula_metadata_for_totals(
                    table.as_str(),
                    column_id,
                    formula_text,
                ));
            }
        }
        self.apply_table_snapshot_transaction(table_node_id, table_view.snapshot)
    }

    pub fn clear_table_totals_formula(
        &mut self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        column.totals_metadata = None;

        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn clear_table_totals_formula_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        column.totals_metadata = None;
        self.apply_table_snapshot_transaction(table_node_id, table_view.snapshot)
    }

    pub fn set_table_header_row_visible(
        &mut self,
        table: &NodeId,
        visible: bool,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        table_view.snapshot.header_row_present = visible;

        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn set_table_header_row_visible_transaction(
        &mut self,
        table: &NodeId,
        visible: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        table_view.snapshot.header_row_present = visible;
        self.apply_table_snapshot_transaction(table_node_id, table_view.snapshot)
    }

    pub fn set_table_totals_row_visible(
        &mut self,
        table: &NodeId,
        visible: bool,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        table_view.snapshot.totals_row_present = visible;

        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn set_table_totals_row_visible_transaction(
        &mut self,
        table: &NodeId,
        visible: bool,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        table_view.snapshot.totals_row_present = visible;
        self.apply_table_snapshot_transaction(table_node_id, table_view.snapshot)
    }

    pub fn rename_table_column(
        &mut self,
        table: &NodeId,
        column_id: &str,
        name: impl Into<String>,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column = table_view
            .snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        column.column_name = name.into();

        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "column-renamed",
            column_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn rename_table_column_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
        name: impl Into<String>,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let snapshot = self.preview_rename_table_column_snapshot(table, column_id, name.into())?;
        self.apply_table_snapshot_transaction(table_node_id, snapshot)
    }

    pub fn reorder_table_column(
        &mut self,
        table: &NodeId,
        column_id: &str,
        new_index: usize,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let mut columns = std::mem::take(&mut table_view.snapshot.columns);
        columns.sort_by_key(|column| column.ordinal);
        let current_index = columns
            .iter()
            .position(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let column = columns.remove(current_index);
        let bounded_index = new_index.min(columns.len());
        columns.insert(bounded_index, column);
        for (index, column) in columns.iter_mut().enumerate() {
            column.ordinal = u32::try_from(index + 1).unwrap_or(u32::MAX);
        }
        table_view.snapshot.columns = columns;
        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "column-reordered",
            column_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn reorder_table_column_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
        new_index: usize,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let snapshot = self.preview_reorder_table_column_snapshot(table, column_id, new_index)?;
        self.apply_table_snapshot_transaction(table_node_id, snapshot)
    }

    pub fn delete_table_column(
        &mut self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<(), TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let mut table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let column_index = table_view
            .snapshot
            .columns
            .iter()
            .position(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let removed_body_node_ids = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .filter(|binding| binding.column_id == column_id)
            .map(|binding| binding.node_id)
            .collect::<Vec<_>>();

        table_view.snapshot.columns.remove(column_index);
        table_view
            .snapshot
            .body_cell_nodes
            .retain(|binding| binding.column_id != column_id);
        table_view.snapshot.column_identity_version = bumped_table_version(
            &table_view.snapshot.column_identity_version,
            "column-removed",
            column_id,
        );
        self.context
            .set_node_table(&self.workspace_id, table_node_id, table_view.snapshot)?;
        for node_id in removed_body_node_ids {
            self.context.delete_node(&self.workspace_id, node_id)?;
        }
        self.refresh_projection_from_context()?;
        self.last_outcome = None;
        Ok(())
    }

    pub fn delete_table_column_transaction(
        &mut self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<TreeWorkspaceTransactionEdit<()>, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        let table_view = self
            .context
            .table_view(&self.workspace_id, table_node_id)?
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })?;
        let removed_body_node_ids = table_view
            .snapshot
            .body_cell_nodes
            .iter()
            .filter(|binding| binding.column_id == column_id)
            .map(|binding| binding.node_id)
            .collect::<Vec<_>>();
        let snapshot = self.preview_delete_table_column_snapshot(table, column_id)?;
        self.apply_table_snapshot_delete_nodes_transaction(
            table_node_id,
            snapshot,
            removed_body_node_ids,
        )
    }

    pub fn workspace_state(&self) -> Result<WorkspaceState, TreeWorkspaceSessionError> {
        let workspace_view = self.context.workspace_view(&self.workspace_id)?;
        let revision = WorkspaceRevisionProjection {
            structural_snapshot_id: Some(workspace_view.snapshot_id.to_string()),
            workspace_revision_id: Some(workspace_view.workspace_revision_id.to_string()),
            node_input_snapshot_id: Some(workspace_view.node_input_snapshot_id.to_string()),
            namespace_snapshot_id: Some(workspace_view.namespace_snapshot_id.to_string()),
            formula_binding_snapshot_id: Some(
                workspace_view.formula_binding_snapshot_id.to_string(),
            ),
            dependency_shape_snapshot_id: Some(
                workspace_view.dependency_shape_snapshot_id.to_string(),
            ),
            publication_snapshot_id: Some(workspace_view.publication_snapshot_id.to_string()),
            runtime_overlay_set_id: Some(workspace_view.runtime_overlay_set_id.to_string()),
            value_epoch: workspace_view.value_epoch,
        };
        let views_by_tree_id = workspace_view
            .nodes
            .into_iter()
            .map(|view| (view.node_id, view))
            .collect::<BTreeMap<_, _>>();
        let table_views_by_tree_id = workspace_view
            .tables
            .iter()
            .map(|view| {
                (
                    view.table_node_id,
                    table_projection_for(
                        view,
                        &views_by_tree_id,
                        self.last_outcome
                            .as_ref()
                            .map(|outcome| &outcome.published_calc_values),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let binding_diagnostics_by_tree_id = self.last_outcome.as_ref().map_or_else(
            || Ok(BTreeMap::<TreeNodeId, Vec<BindingDiagnosticProjection>>::new()),
            |outcome| self.binding_diagnostics_by_tree_id(outcome),
        )?;

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
            let outcome_calc_value = self
                .last_outcome
                .as_ref()
                .and_then(|outcome| outcome.published_calc_values.get(&tree_node_id));
            let calc_value = outcome_calc_value.or(tree_view.calc_value.as_ref());
            let effective_format = self.effective_format_for_node(node_id, &views_by_tree_id)?;
            let note = self.note_for_node(node_id, &views_by_tree_id)?;
            let attributes = self.attributes_for_node(node_id, &views_by_tree_id)?;
            let computed_value = value_projection_for(
                tree_view.value_text.clone(),
                tree_view.calc_state,
                calc_value,
                effective_format.as_ref(),
            );
            let literalized_value_input = literalized_value_input_for(calc_value);
            let table = table_views_by_tree_id.get(&tree_node_id).cloned();
            let binding_diagnostics = binding_diagnostics_by_tree_id
                .get(&tree_node_id)
                .cloned()
                .unwrap_or_default();
            nodes.insert(
                node_id.clone(),
                NodeView {
                    key: node_key_for_tree_node(tree_node_id),
                    id: node_id.clone(),
                    display_name: tree_view.symbol.clone(),
                    parent,
                    children: child_map.remove(node_id).unwrap_or_default(),
                    depth: depth_of(node_id.as_str()),
                    content_kind,
                    content_text: tree_view.formula_text.clone(),
                    computed_value,
                    literalized_value_input,
                    value_epoch: tree_view.published_value_epoch,
                    calc_state: tree_view.calc_state.map(calc_state_projection_for),
                    effective_format,
                    note,
                    attributes,
                    binding_diagnostics,
                    is_meta: tree_view.is_meta,
                    table,
                },
            );
        }
        let tables = table_views_by_tree_id
            .into_iter()
            .map(|(tree_node_id, table)| Ok((self.node_id_for_tree_node(tree_node_id)?, table)))
            .collect::<Result<BTreeMap<_, _>, TreeWorkspaceSessionError>>()?;
        let dependencies = self.last_outcome.as_ref().map_or_else(
            || Ok(DependencyGraphProjection::default()),
            |outcome| self.dependency_graph_projection(outcome),
        )?;
        let last_run = self
            .last_outcome
            .as_ref()
            .map(|outcome| self.calc_run_projection(outcome))
            .transpose()?;
        let diagnostics = workspace_view
            .diagnostics
            .into_iter()
            .chain(
                self.last_outcome
                    .as_ref()
                    .into_iter()
                    .flat_map(|outcome| outcome.diagnostics.clone()),
            )
            .collect();

        let paths_by_key = nodes
            .iter()
            .map(|(node_id, view)| (view.key.clone(), node_id.clone()))
            .collect::<BTreeMap<_, _>>();
        let nodes_by_key = nodes
            .values()
            .map(|view| (view.key.clone(), view.clone()))
            .collect::<BTreeMap<_, _>>();
        let root_keys = root_paths
            .iter()
            .filter_map(|root| nodes.get(root).map(|node| node.key.clone()))
            .collect::<Vec<_>>();

        Ok(WorkspaceState {
            workspace_id: self.workspace_id.as_str().to_string(),
            profile: self.profile,
            projection_seq: 0,
            revision,
            last_run,
            node_order: self.display_order.clone(),
            key_order: self
                .display_order
                .iter()
                .map(|node_id| {
                    self.tree_node_id(node_id.as_str())
                        .map(node_key_for_tree_node)
                })
                .collect::<Result<Vec<_>, _>>()?,
            paths_by_key,
            root_keys,
            root_paths,
            nodes_by_key,
            nodes,
            dependencies,
            tables,
            clipboard: None,
            diagnostics,
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

    pub fn collection_dependencies_for(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
        owner: &NodeId,
    ) -> Result<Vec<TreeWorkspaceCollectionDependencyProjection>, TreeWorkspaceSessionError> {
        let owner_node_id = self.tree_node_id(owner.as_str())?;
        outcome
            .dependency_graph
            .descriptors_by_owner
            .get(&owner_node_id)
            .map_or_else(
                || Ok(Vec::new()),
                |descriptors| {
                    descriptors
                        .iter()
                        .filter(|descriptor| {
                            descriptor.kind
                                == DependencyDescriptorKind::TreeReferenceCollectionMembership
                        })
                        .filter_map(|descriptor| descriptor.tree_reference_collection.as_ref())
                        .map(|collection| {
                            let base_node = if collection.base_node_id == self.engine_root_id {
                                None
                            } else {
                                Some(self.node_id_for_tree_node(collection.base_node_id)?)
                            };
                            let members = collection
                                .member_node_ids
                                .iter()
                                .map(|member| self.node_id_for_tree_node(*member))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(TreeWorkspaceCollectionDependencyProjection {
                                family: collection.family.stable_id().to_string(),
                                source_reference_handle: collection.host_ref_handle.clone(),
                                base_node,
                                membership_version: collection.membership_version.clone(),
                                order_version: collection.order_version.clone(),
                                members,
                            })
                        })
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

    fn current_views_by_tree_id(
        &self,
    ) -> Result<BTreeMap<TreeNodeId, OxCalcTreeNodeView>, TreeWorkspaceSessionError> {
        Ok(self
            .context
            .workspace_view(&self.workspace_id)?
            .nodes
            .into_iter()
            .map(|view| (view.node_id, view))
            .collect())
    }

    fn name_collisions_for_attempt(
        &self,
        node: &NodeId,
        attempted: &NodeId,
    ) -> Vec<NameCollisionProjection> {
        self.node_ids
            .get(attempted)
            .filter(|_| *attempted != *node)
            .map(|tree_node_id| {
                vec![NameCollisionProjection {
                    attempted: attempted.to_string(),
                    existing: attempted.clone(),
                    existing_key: node_key_for_tree_node(*tree_node_id),
                }]
            })
            .unwrap_or_default()
    }

    fn name_collisions_for_new_node(&self, attempted: &NodeId) -> Vec<NameCollisionProjection> {
        self.node_ids
            .get(attempted)
            .map(|tree_node_id| {
                vec![NameCollisionProjection {
                    attempted: attempted.to_string(),
                    existing: attempted.clone(),
                    existing_key: node_key_for_tree_node(*tree_node_id),
                }]
            })
            .unwrap_or_default()
    }

    fn table_row_add_blocker(
        &self,
        table: &NodeId,
        row_id: &str,
        values: &[TableCellInput],
    ) -> Result<Option<MutationImpactBlockedReasonProjection>, TreeWorkspaceSessionError> {
        let snapshot = self.table_snapshot(table)?;
        if snapshot.rows.iter().any(|row| row.0 == row_id) {
            return Ok(Some(MutationImpactBlockedReasonProjection::TableCollision));
        }
        let mut columns_seen = BTreeSet::new();
        for value in values {
            if !columns_seen.insert(value.column_id.clone()) {
                return Ok(Some(MutationImpactBlockedReasonProjection::DuplicateInput));
            }
            let column = snapshot
                .columns
                .iter()
                .find(|column| column.column_id == value.column_id)
                .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                    table: table.to_string(),
                    column_id: value.column_id.clone(),
                })?;
            if matches!(
                column.body_metadata,
                TreeCalcTableColumnBodyMetadata::Formula(_)
            ) {
                return Err(TreeWorkspaceSessionError::FormulaTableCellEdit {
                    table: table.to_string(),
                    column_id: value.column_id.clone(),
                });
            }
        }
        Ok(None)
    }

    fn table_column_add_blocker(
        &self,
        table: &NodeId,
        column_id: &str,
        values: &[TableRowInput],
    ) -> Result<Option<MutationImpactBlockedReasonProjection>, TreeWorkspaceSessionError> {
        let snapshot = self.table_snapshot(table)?;
        if snapshot
            .columns
            .iter()
            .any(|column| column.column_id == column_id)
        {
            return Ok(Some(MutationImpactBlockedReasonProjection::TableCollision));
        }
        let mut rows_seen = BTreeSet::new();
        for value in values {
            if !rows_seen.insert(value.row_id.clone()) {
                return Ok(Some(MutationImpactBlockedReasonProjection::DuplicateInput));
            }
            if !snapshot.rows.iter().any(|row| row.0 == value.row_id) {
                return Err(TreeWorkspaceSessionError::UnknownTableRow {
                    table: table.to_string(),
                    row_id: value.row_id.clone(),
                });
            }
        }
        Ok(None)
    }

    fn table_has_row(
        &self,
        table: &NodeId,
        row_id: &str,
    ) -> Result<bool, TreeWorkspaceSessionError> {
        Ok(self
            .table_snapshot(table)?
            .rows
            .iter()
            .any(|row| row.0 == row_id))
    }

    fn table_row_index(
        &self,
        table: &NodeId,
        row_id: &str,
    ) -> Result<usize, TreeWorkspaceSessionError> {
        self.table_snapshot(table)?
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })
    }

    fn table_column_index(
        &self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<usize, TreeWorkspaceSessionError> {
        self.table_snapshot(table)?
            .columns
            .iter()
            .position(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })
    }

    fn table_snapshot(
        &self,
        table: &NodeId,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let table_node_id = self.tree_node_id(table.as_str())?;
        self.context
            .table_view(&self.workspace_id, table_node_id)?
            .map(|view| view.snapshot)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTable {
                table: table.to_string(),
            })
    }

    fn table_snapshot_preview_mutation(
        &self,
        table: &NodeId,
        snapshot: TreeCalcTableNodeSnapshot,
        scenario: TreeCalcTableUpdateScenarioKind,
    ) -> Result<Vec<OxCalcTreePreviewMutation>, TreeWorkspaceSessionError> {
        Ok(vec![OxCalcTreePreviewMutation::SetNodeTable {
            node_id: self.tree_node_id(table.as_str())?,
            snapshot,
            scenario,
        }])
    }

    fn preview_add_table_row_snapshot(
        &self,
        table: &NodeId,
        row_id: String,
        values: Vec<TableCellInput>,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        self.table_row_add_blocker(table, &row_id, &values)?;
        let mut snapshot = self.table_snapshot(table)?;
        snapshot.rows.push(TreeCalcTableRowId(row_id.clone()));
        snapshot.row_membership_version =
            bumped_table_version(&snapshot.row_membership_version, "row-preview", &row_id);
        snapshot.row_order_version =
            bumped_table_version(&snapshot.row_order_version, "row-preview", &row_id);
        Ok(snapshot)
    }

    fn preview_delete_table_row_snapshot(
        &self,
        table: &NodeId,
        row_id: &str,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let row_index = snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;
        snapshot.rows.remove(row_index);
        snapshot
            .body_cell_nodes
            .retain(|binding| binding.row_id.0 != row_id);
        snapshot.row_membership_version = bumped_table_version(
            &snapshot.row_membership_version,
            "row-removed-preview",
            row_id,
        );
        snapshot.row_order_version =
            bumped_table_version(&snapshot.row_order_version, "row-removed-preview", row_id);
        Ok(snapshot)
    }

    fn preview_rename_table_row_snapshot(
        &self,
        table: &NodeId,
        row_id: &str,
        new_row_id: String,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let row_index = snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;
        snapshot.rows[row_index] = TreeCalcTableRowId(new_row_id.clone());
        for binding in &mut snapshot.body_cell_nodes {
            if binding.row_id.0 == row_id {
                binding.row_id = TreeCalcTableRowId(new_row_id.clone());
            }
        }
        snapshot.row_membership_version = bumped_table_version(
            &snapshot.row_membership_version,
            "row-renamed-preview",
            row_id,
        );
        Ok(snapshot)
    }

    fn preview_reorder_table_row_snapshot(
        &self,
        table: &NodeId,
        row_id: &str,
        new_index: usize,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let row_index = snapshot
            .rows
            .iter()
            .position(|row| row.0 == row_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableRow {
                table: table.to_string(),
                row_id: row_id.to_string(),
            })?;
        let row = snapshot.rows.remove(row_index);
        let bounded_index = new_index.min(snapshot.rows.len());
        snapshot.rows.insert(bounded_index, row);
        snapshot.row_order_version =
            bumped_table_version(&snapshot.row_order_version, "row-reordered-preview", row_id);
        Ok(snapshot)
    }

    fn preview_add_table_column_snapshot(
        &self,
        table: &NodeId,
        column_id: String,
        name: String,
        values: Vec<TableRowInput>,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        self.table_column_add_blocker(table, &column_id, &values)?;
        let mut snapshot = self.table_snapshot(table)?;
        let column_ordinal = snapshot
            .columns
            .iter()
            .map(|column| column.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        snapshot.columns.push(TreeCalcTableColumnSnapshot {
            column_id: column_id.clone(),
            column_name: name,
            ordinal: column_ordinal,
            body_metadata: TreeCalcTableColumnBodyMetadata::ConstantCells,
            totals_metadata: None,
        });
        snapshot.column_identity_version = bumped_table_version(
            &snapshot.column_identity_version,
            "column-preview",
            &column_id,
        );
        Ok(snapshot)
    }

    fn preview_delete_table_column_snapshot(
        &self,
        table: &NodeId,
        column_id: &str,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let column_index = snapshot
            .columns
            .iter()
            .position(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        snapshot.columns.remove(column_index);
        snapshot.columns.sort_by_key(|column| column.ordinal);
        for (index, column) in snapshot.columns.iter_mut().enumerate() {
            column.ordinal = u32::try_from(index + 1).unwrap_or(u32::MAX);
        }
        snapshot
            .body_cell_nodes
            .retain(|binding| binding.column_id != column_id);
        snapshot.column_identity_version = bumped_table_version(
            &snapshot.column_identity_version,
            "column-removed-preview",
            column_id,
        );
        Ok(snapshot)
    }

    fn preview_rename_table_column_snapshot(
        &self,
        table: &NodeId,
        column_id: &str,
        name: String,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let column = snapshot
            .columns
            .iter_mut()
            .find(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        column.column_name = name;
        snapshot.column_identity_version = bumped_table_version(
            &snapshot.column_identity_version,
            "column-renamed-preview",
            column_id,
        );
        Ok(snapshot)
    }

    fn preview_reorder_table_column_snapshot(
        &self,
        table: &NodeId,
        column_id: &str,
        new_index: usize,
    ) -> Result<TreeCalcTableNodeSnapshot, TreeWorkspaceSessionError> {
        let mut snapshot = self.table_snapshot(table)?;
        let mut columns = std::mem::take(&mut snapshot.columns);
        columns.sort_by_key(|column| column.ordinal);
        let current_index = columns
            .iter()
            .position(|column| column.column_id == column_id)
            .ok_or_else(|| TreeWorkspaceSessionError::UnknownTableColumn {
                table: table.to_string(),
                column_id: column_id.to_string(),
            })?;
        let column = columns.remove(current_index);
        let bounded_index = new_index.min(columns.len());
        columns.insert(bounded_index, column);
        for (index, column) in columns.iter_mut().enumerate() {
            column.ordinal = u32::try_from(index + 1).unwrap_or(u32::MAX);
        }
        snapshot.columns = columns;
        snapshot.column_identity_version = bumped_table_version(
            &snapshot.column_identity_version,
            "column-reordered-preview",
            column_id,
        );
        Ok(snapshot)
    }

    fn move_would_target_self_or_descendant(
        &self,
        state: &WorkspaceState,
        node: &NodeId,
        new_parent: Option<&NodeId>,
    ) -> Result<bool, TreeWorkspaceSessionError> {
        let Some(new_parent) = new_parent else {
            return Ok(false);
        };
        if new_parent == node {
            return Ok(true);
        }
        let node_key = state
            .node(node)
            .map(|node| node.key.clone())
            .ok_or_else(|| TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: node.to_string(),
            })?;
        let new_parent_key = state
            .node(new_parent)
            .map(|node| node.key.clone())
            .ok_or_else(|| TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: new_parent.to_string(),
            })?;
        let subtree = state
            .expand_authoring_scope(&AuthoringScope::Subtree(node_key))
            .map_err(|error| TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            })?;
        Ok(subtree.contains(&new_parent_key))
    }

    fn subtree_keys_for_state(
        &self,
        state: &WorkspaceState,
        node: &NodeId,
    ) -> Result<Vec<NodeKey>, TreeWorkspaceSessionError> {
        let node_key = state
            .node(node)
            .map(|node| node.key.clone())
            .ok_or_else(|| TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: node.to_string(),
            })?;
        state
            .expand_authoring_scope(&AuthoringScope::Subtree(node_key))
            .map_err(|error| TreeWorkspaceSessionError::ProjectionOutOfSync {
                node: error.to_string(),
            })
    }

    fn engine_preview_mutations(
        &self,
        mutation: &RecalcPlanMutation,
    ) -> Result<Vec<OxCalcTreePreviewMutation>, TreeWorkspaceSessionError> {
        match mutation {
            RecalcPlanMutation::SetNodeInput { node } => {
                Ok(vec![OxCalcTreePreviewMutation::SetNodeInput {
                    node_id: self.tree_node_id(node.as_str())?,
                }])
            }
            RecalcPlanMutation::EditContent { node, content } => {
                Ok(vec![OxCalcTreePreviewMutation::SetNodeFormulaText {
                    node_id: self.tree_node_id(node.as_str())?,
                    formula_text: content.clone(),
                }])
            }
            RecalcPlanMutation::EditScopedContent { scope, content } => {
                let state = self.workspace_state()?;
                let target_keys = state.expand_authoring_scope(scope).map_err(|error| {
                    TreeWorkspaceSessionError::ProjectionOutOfSync {
                        node: error.to_string(),
                    }
                })?;
                target_keys
                    .iter()
                    .map(|key| {
                        let node = state.node_by_key(key).ok_or_else(|| {
                            TreeWorkspaceSessionError::ProjectionOutOfSync {
                                node: format!("node key {key}"),
                            }
                        })?;
                        Ok(OxCalcTreePreviewMutation::SetNodeFormulaText {
                            node_id: self.tree_node_id(node.id.as_str())?,
                            formula_text: content.clone(),
                        })
                    })
                    .collect()
            }
            RecalcPlanMutation::AddTableFormulaColumn {
                table,
                column_id,
                name,
                formula_text,
            } => {
                let table_node_id = self.tree_node_id(table.as_str())?;
                Ok(vec![OxCalcTreePreviewMutation::SetNodeTable {
                    node_id: table_node_id,
                    snapshot: self.preview_formula_column_table_snapshot(
                        table,
                        table_node_id,
                        column_id.clone(),
                        name.clone(),
                        formula_text.clone(),
                    )?,
                    scenario: TreeCalcTableUpdateScenarioKind::ColumnInsert,
                }])
            }
            RecalcPlanMutation::AddTableRow {
                table,
                row_id,
                values,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_add_table_row_snapshot(table, row_id.clone(), values.clone())?,
                TreeCalcTableUpdateScenarioKind::RowInsert,
            ),
            RecalcPlanMutation::DeleteTableRow { table, row_id } => self
                .table_snapshot_preview_mutation(
                    table,
                    self.preview_delete_table_row_snapshot(table, row_id)?,
                    TreeCalcTableUpdateScenarioKind::RowDelete,
                ),
            RecalcPlanMutation::RenameTableRow {
                table,
                row_id,
                new_row_id,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_rename_table_row_snapshot(table, row_id, new_row_id.clone())?,
                TreeCalcTableUpdateScenarioKind::RowInsert,
            ),
            RecalcPlanMutation::ReorderTableRow {
                table,
                row_id,
                new_index,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_reorder_table_row_snapshot(table, row_id, *new_index)?,
                TreeCalcTableUpdateScenarioKind::RowReorder,
            ),
            RecalcPlanMutation::AddTableColumn {
                table,
                column_id,
                name,
                values,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_add_table_column_snapshot(
                    table,
                    column_id.clone(),
                    name.clone(),
                    values.clone(),
                )?,
                TreeCalcTableUpdateScenarioKind::ColumnInsert,
            ),
            RecalcPlanMutation::DeleteTableColumn { table, column_id } => self
                .table_snapshot_preview_mutation(
                    table,
                    self.preview_delete_table_column_snapshot(table, column_id)?,
                    TreeCalcTableUpdateScenarioKind::ColumnDelete,
                ),
            RecalcPlanMutation::RenameTableColumn {
                table,
                column_id,
                name,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_rename_table_column_snapshot(table, column_id, name.clone())?,
                TreeCalcTableUpdateScenarioKind::ColumnRename,
            ),
            RecalcPlanMutation::ReorderTableColumn {
                table,
                column_id,
                new_index,
            } => self.table_snapshot_preview_mutation(
                table,
                self.preview_reorder_table_column_snapshot(table, column_id, *new_index)?,
                TreeCalcTableUpdateScenarioKind::ColumnReorder,
            ),
            RecalcPlanMutation::RenameNode { node } => {
                Ok(vec![OxCalcTreePreviewMutation::RenameNode {
                    node_id: self.tree_node_id(node.as_str())?,
                }])
            }
            RecalcPlanMutation::MoveNode { node } => {
                Ok(vec![OxCalcTreePreviewMutation::MoveNode {
                    node_id: self.tree_node_id(node.as_str())?,
                }])
            }
            RecalcPlanMutation::ReorderNode { node } => {
                Ok(vec![OxCalcTreePreviewMutation::ReorderNode {
                    node_id: self.tree_node_id(node.as_str())?,
                }])
            }
            RecalcPlanMutation::DeleteNode { node } => {
                Ok(vec![OxCalcTreePreviewMutation::DeleteNode {
                    node_id: self.tree_node_id(node.as_str())?,
                }])
            }
            RecalcPlanMutation::InvalidateNode { node, reason } => {
                Ok(vec![OxCalcTreePreviewMutation::InvalidateNode {
                    node_id: self.tree_node_id(node.as_str())?,
                    reason: invalidation_reason_kind_for_projection(*reason),
                }])
            }
        }
    }

    fn node_id_for_tree_node(
        &self,
        tree_node_id: TreeNodeId,
    ) -> Result<NodeId, TreeWorkspaceSessionError> {
        self.node_paths_by_tree_id
            .get(&tree_node_id)
            .cloned()
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
        let mut non_meta_refreshed = BTreeSet::new();
        for view in workspace_view.nodes {
            if view.node_id == self.engine_root_id {
                continue;
            }
            let node_id = node_id_from_canonical_path(&view.canonical_path)?;
            if !view.is_meta {
                non_meta_refreshed.insert(view.node_id);
            }
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
            if non_meta_refreshed.contains(tree_node_id) && seen.insert(*tree_node_id) {
                display_order.push(node_id.clone());
            }
        }

        self.node_ids = refreshed
            .into_iter()
            .map(|(tree_node_id, node_id)| (node_id, tree_node_id))
            .collect();
        self.node_paths_by_tree_id = self
            .node_ids
            .iter()
            .map(|(node_id, tree_node_id)| (*tree_node_id, node_id.clone()))
            .collect();
        self.display_order = display_order;
        Ok(())
    }

    fn dependency_graph_projection(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
    ) -> Result<DependencyGraphProjection, TreeWorkspaceSessionError> {
        let mut descriptors_by_owner = BTreeMap::new();
        let mut descriptors_by_owner_key = BTreeMap::new();
        for (owner, descriptors) in &outcome.dependency_graph.descriptors_by_owner {
            if *owner == self.engine_root_id {
                continue;
            }
            let owner_path = self.node_id_for_tree_node(*owner)?;
            let owner_key = node_key_for_tree_node(*owner);
            let projected = descriptors
                .iter()
                .map(|descriptor| {
                    let collection = descriptor
                        .tree_reference_collection
                        .as_ref()
                        .map(|collection| self.tree_reference_collection_projection(collection))
                        .transpose()?;
                    let target_node_id = descriptor
                        .target_node_id
                        .filter(|target| *target != self.engine_root_id);
                    Ok(DependencyDescriptorProjection {
                        descriptor_id: descriptor.descriptor_id.clone(),
                        source_reference_handle: descriptor.source_reference_handle.clone(),
                        target: target_node_id
                            .map(|target| self.node_id_for_tree_node(target))
                            .transpose()?,
                        target_key: target_node_id.map(node_key_for_tree_node),
                        workspace_target: descriptor
                            .workspace_target
                            .as_ref()
                            .map(|target| target.target_node_handle.clone()),
                        kind: dependency_kind_projection_for(descriptor.kind),
                        carrier_detail: descriptor.carrier_detail.clone(),
                        collection,
                        requires_rebind_on_structural_change: descriptor
                            .requires_rebind_on_structural_change,
                    })
                })
                .collect::<Result<Vec<_>, TreeWorkspaceSessionError>>()?;
            descriptors_by_owner.insert(owner_path, projected.clone());
            descriptors_by_owner_key.insert(owner_key, projected);
        }

        let mut edges_by_owner = BTreeMap::new();
        let mut edges_by_owner_key = BTreeMap::new();
        for (owner, edges) in &outcome.dependency_graph.edges_by_owner {
            if *owner == self.engine_root_id {
                continue;
            }
            let owner_path = self.node_id_for_tree_node(*owner)?;
            let owner_key = node_key_for_tree_node(*owner);
            let projected = edges
                .iter()
                .filter(|edge| edge.target_node_id != self.engine_root_id)
                .map(|edge| self.dependency_edge_projection(edge))
                .collect::<Result<Vec<_>, _>>()?;
            edges_by_owner.insert(owner_path, projected.clone());
            edges_by_owner_key.insert(owner_key, projected);
        }

        let mut reverse_edges = BTreeMap::new();
        let mut reverse_edges_by_key = BTreeMap::new();
        for (target, edges) in &outcome.dependency_graph.reverse_edges {
            if *target == self.engine_root_id {
                continue;
            }
            let target_path = self.node_id_for_tree_node(*target)?;
            let target_key = node_key_for_tree_node(*target);
            let projected = edges
                .iter()
                .filter(|edge| {
                    edge.owner_node_id != self.engine_root_id
                        && edge.target_node_id != self.engine_root_id
                })
                .map(|edge| self.dependency_edge_projection(edge))
                .collect::<Result<Vec<_>, _>>()?;
            reverse_edges.insert(target_path, projected.clone());
            reverse_edges_by_key.insert(target_key, projected);
        }

        let cycle_group_keys = outcome
            .dependency_graph
            .cycle_groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .filter(|node| **node != self.engine_root_id)
                    .map(|node| node_key_for_tree_node(*node))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let cycle_groups = outcome
            .dependency_graph
            .cycle_groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .filter(|node| **node != self.engine_root_id)
                    .map(|node| self.node_id_for_tree_node(*node))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (reference_resolutions, reverse_references) =
            self.reference_resolution_projection(outcome)?;

        Ok(DependencyGraphProjection {
            descriptors_by_owner_key,
            edges_by_owner_key,
            reverse_edges_by_key,
            descriptors_by_owner,
            edges_by_owner,
            reverse_edges,
            reference_resolutions,
            reverse_references,
            cycle_group_keys,
            cycle_groups,
            diagnostics: outcome
                .dependency_graph
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    format!(
                        "{}:{:?}:{}",
                        diagnostic.descriptor_id, diagnostic.kind, diagnostic.detail
                    )
                })
                .collect(),
        })
    }

    fn binding_diagnostics_by_tree_id(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
    ) -> Result<BTreeMap<TreeNodeId, Vec<BindingDiagnosticProjection>>, TreeWorkspaceSessionError>
    {
        let mut diagnostics = BTreeMap::<TreeNodeId, Vec<BindingDiagnosticProjection>>::new();
        for diagnostic in &outcome.binding_diagnostics {
            if diagnostic.owner_node_id == self.engine_root_id {
                continue;
            }
            diagnostics
                .entry(diagnostic.owner_node_id)
                .or_default()
                .push(BindingDiagnosticProjection {
                    node: self.node_id_for_tree_node(diagnostic.owner_node_id)?,
                    node_key: node_key_for_tree_node(diagnostic.owner_node_id),
                    message: diagnostic.message.clone(),
                    span: SourceSpanProjection {
                        start_utf8: diagnostic.span_start_utf8,
                        end_utf8: diagnostic
                            .span_start_utf8
                            .saturating_add(diagnostic.span_len_utf8),
                    },
                });
        }
        Ok(diagnostics)
    }

    fn effective_format_for_node(
        &self,
        node_id: &NodeId,
        views_by_tree_id: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    ) -> Result<Option<EffectiveFormatProjection>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node_id.as_str())?;
        if views_by_tree_id
            .get(&tree_node_id)
            .is_some_and(|view| view.is_meta)
        {
            return Ok(None);
        }

        let mut cursor = Some(node_id.as_str().to_string());
        while let Some(path) = cursor {
            let number_format_node_id = NodeId::new(format!("{path}.Format.NumberFormat"));
            if let Some(number_format_tree_id) = self.node_ids.get(&number_format_node_id) {
                if let Some(format_code) = views_by_tree_id
                    .get(number_format_tree_id)
                    .filter(|view| view.is_meta)
                    .map(|view| view.formula_text.trim().to_string())
                    .filter(|format_code| !format_code.is_empty())
                {
                    let format_node_id = NodeId::new(format!("{path}.Format"));
                    let source_tree_id = self
                        .node_ids
                        .get(&format_node_id)
                        .copied()
                        .unwrap_or(*number_format_tree_id);
                    return Ok(Some(EffectiveFormatProjection {
                        number_format_code: Some(format_code),
                        inherited_from: Some(FormatSourceProjection {
                            node: self.node_id_for_tree_node(source_tree_id)?,
                            node_key: node_key_for_tree_node(source_tree_id),
                        }),
                    }));
                }
            }
            cursor = parent_path(&path);
        }

        Ok(None)
    }

    fn note_for_node(
        &self,
        node_id: &NodeId,
        views_by_tree_id: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    ) -> Result<Option<NodeNoteProjection>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node_id.as_str())?;
        if views_by_tree_id
            .get(&tree_node_id)
            .is_some_and(|view| view.is_meta)
        {
            return Ok(None);
        }

        let note_node_id = NodeId::new(format!("{}.Note", node_id.as_str()));
        let Some(note_tree_id) = self.node_ids.get(&note_node_id) else {
            return Ok(None);
        };
        let Some(note_text) = views_by_tree_id
            .get(note_tree_id)
            .filter(|view| view.is_meta)
            .map(|view| view.formula_text.clone())
            .filter(|text| !text.is_empty())
        else {
            return Ok(None);
        };

        Ok(Some(NodeNoteProjection {
            text: note_text,
            source: NoteSourceProjection {
                node: self.node_id_for_tree_node(*note_tree_id)?,
                node_key: node_key_for_tree_node(*note_tree_id),
            },
        }))
    }

    fn attributes_for_node(
        &self,
        node_id: &NodeId,
        views_by_tree_id: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    ) -> Result<BTreeMap<String, String>, TreeWorkspaceSessionError> {
        let tree_node_id = self.tree_node_id(node_id.as_str())?;
        if views_by_tree_id
            .get(&tree_node_id)
            .is_some_and(|view| view.is_meta)
        {
            return Ok(BTreeMap::new());
        }

        let attributes_node_id = NodeId::new(format!("{}.Attributes", node_id.as_str()));
        let Some(attributes_tree_id) = self.node_ids.get(&attributes_node_id) else {
            return Ok(BTreeMap::new());
        };
        if !views_by_tree_id
            .get(attributes_tree_id)
            .is_some_and(|view| view.is_meta)
        {
            return Ok(BTreeMap::new());
        }

        let prefix = format!("{}.", attributes_node_id.as_str());
        let mut attributes = BTreeMap::new();
        for (candidate_node_id, candidate_tree_id) in &self.node_ids {
            let candidate_path = candidate_node_id.as_str();
            let Some(key) = candidate_path.strip_prefix(&prefix) else {
                continue;
            };
            if key.is_empty() || key.contains('.') {
                continue;
            }
            let Some(value) = views_by_tree_id
                .get(candidate_tree_id)
                .filter(|view| view.is_meta)
                .map(|view| view.formula_text.clone())
            else {
                continue;
            };
            attributes.insert(key.to_string(), value);
        }
        Ok(attributes)
    }

    fn reference_resolution_projection(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
    ) -> Result<
        (
            BTreeMap<String, ReferenceResolutionProjection>,
            BTreeMap<NodeKey, Vec<String>>,
        ),
        TreeWorkspaceSessionError,
    > {
        let mut resolutions = BTreeMap::new();
        for (owner, descriptors) in &outcome.dependency_graph.descriptors_by_owner {
            if *owner == self.engine_root_id {
                continue;
            }
            let owner_id = self.node_id_for_tree_node(*owner)?;
            let owner_key = node_key_for_tree_node(*owner);
            for descriptor in descriptors {
                let Some(handle) = descriptor.source_reference_handle.as_ref() else {
                    continue;
                };
                let target = self.reference_target_projection(descriptor)?;
                let entry = resolutions.entry(handle.clone()).or_insert_with(|| {
                    ReferenceResolutionProjection {
                        source_reference_handle: handle.clone(),
                        owner: owner_id.clone(),
                        owner_key: owner_key.clone(),
                        descriptor_ids: Vec::new(),
                        token_span: None,
                        target: ReferenceTargetProjection::Unresolved,
                        primary_kind: dependency_kind_projection_for(descriptor.kind),
                        requires_rebind_on_structural_change: false,
                    }
                });
                if !entry.descriptor_ids.contains(&descriptor.descriptor_id) {
                    entry.descriptor_ids.push(descriptor.descriptor_id.clone());
                }
                if should_replace_reference_target(&entry.target, &target) {
                    entry.target = target;
                    entry.primary_kind = dependency_kind_projection_for(descriptor.kind);
                }
                entry.requires_rebind_on_structural_change |=
                    descriptor.requires_rebind_on_structural_change;
            }
        }

        let mut reverse_references = BTreeMap::<NodeKey, BTreeSet<String>>::new();
        for (handle, resolution) in &resolutions {
            for target_key in reference_target_keys(&resolution.target) {
                reverse_references
                    .entry(target_key)
                    .or_default()
                    .insert(handle.clone());
            }
        }

        Ok((
            resolutions,
            reverse_references
                .into_iter()
                .map(|(node, handles)| (node, handles.into_iter().collect()))
                .collect(),
        ))
    }

    fn reference_target_projection(
        &self,
        descriptor: &DependencyDescriptor,
    ) -> Result<ReferenceTargetProjection, TreeWorkspaceSessionError> {
        if let Some(collection) = descriptor.tree_reference_collection.as_ref() {
            let collection_projection = self.tree_reference_collection_projection(collection)?;
            let member_keys = collection
                .member_node_ids
                .iter()
                .filter(|member| **member != self.engine_root_id)
                .map(|member| node_key_for_tree_node(*member))
                .collect();
            return Ok(ReferenceTargetProjection::Collection {
                collection: collection_projection,
                member_keys,
            });
        }

        if let Some(target) = descriptor
            .target_node_id
            .filter(|target| *target != self.engine_root_id)
        {
            return Ok(ReferenceTargetProjection::Node {
                node: self.node_id_for_tree_node(target)?,
                key: node_key_for_tree_node(target),
            });
        }

        if let Some(target) = descriptor.workspace_target.as_ref() {
            return Ok(ReferenceTargetProjection::External {
                target: target.target_node_handle.clone(),
            });
        }

        Ok(ReferenceTargetProjection::Unresolved)
    }

    fn tree_reference_collection_projection(
        &self,
        collection: &TreeReferenceCollectionDependency,
    ) -> Result<TreeReferenceCollectionProjection, TreeWorkspaceSessionError> {
        Ok(TreeReferenceCollectionProjection {
            family: tree_collection_family_projection_for(collection.family),
            source_reference_handle: collection.host_ref_handle.clone(),
            base_node: if collection.base_node_id == self.engine_root_id {
                None
            } else {
                Some(self.node_id_for_tree_node(collection.base_node_id)?)
            },
            base_node_key: (collection.base_node_id != self.engine_root_id)
                .then(|| node_key_for_tree_node(collection.base_node_id)),
            membership_version: collection.membership_version.clone(),
            order_version: collection.order_version.clone(),
            members: collection
                .member_node_ids
                .iter()
                .filter(|member| **member != self.engine_root_id)
                .map(|member| self.node_id_for_tree_node(*member))
                .collect::<Result<Vec<_>, _>>()?,
            member_keys: collection
                .member_node_ids
                .iter()
                .filter(|member| **member != self.engine_root_id)
                .map(|member| node_key_for_tree_node(*member))
                .collect(),
        })
    }

    fn dependency_edge_projection(
        &self,
        edge: &oxcalc_core::dependency::DependencyEdge,
    ) -> Result<DependencyEdgeProjection, TreeWorkspaceSessionError> {
        Ok(DependencyEdgeProjection {
            edge_id: edge.edge_id.clone(),
            descriptor_id: edge.descriptor_id.clone(),
            owner: self.node_id_for_tree_node(edge.owner_node_id)?,
            owner_key: node_key_for_tree_node(edge.owner_node_id),
            target: self.node_id_for_tree_node(edge.target_node_id)?,
            target_key: node_key_for_tree_node(edge.target_node_id),
            kind: dependency_kind_projection_for(edge.kind),
        })
    }

    fn calc_run_projection(
        &self,
        outcome: &OxCalcTreeCalculationOutcome,
    ) -> Result<CalcRunProjection, TreeWorkspaceSessionError> {
        let invalidated_nodes = outcome
            .invalidation_closure
            .impacted_order
            .iter()
            .filter_map(|node_id| outcome.invalidation_closure.records.get(node_id))
            .filter(|record| record.node_id != self.engine_root_id)
            .map(|record| {
                Ok(NodeInvalidationProjection {
                    node: self.node_id_for_tree_node(record.node_id)?,
                    node_key: node_key_for_tree_node(record.node_id),
                    calc_state: calc_state_projection_for(record.calc_state),
                    requires_rebind: record.requires_rebind,
                    reasons: record
                        .reasons
                        .iter()
                        .copied()
                        .map(invalidation_reason_projection_for)
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, TreeWorkspaceSessionError>>()?;
        Ok(CalcRunProjection {
            run_state: match outcome.run_state {
                OxCalcTreeRunState::Published => CalcRunStateProjection::Published,
                OxCalcTreeRunState::VerifiedClean => CalcRunStateProjection::VerifiedClean,
                OxCalcTreeRunState::Rejected => CalcRunStateProjection::Rejected,
            },
            evaluation_order: outcome
                .evaluation_order
                .iter()
                .filter(|node| **node != self.engine_root_id)
                .map(|node| self.node_id_for_tree_node(*node))
                .collect::<Result<Vec<_>, _>>()?,
            runtime_effect_count: outcome.runtime_effects.len(),
            runtime_effects: outcome
                .runtime_effects
                .iter()
                .map(runtime_effect_projection)
                .collect(),
            runtime_overlay_count: outcome.runtime_effect_overlays.len(),
            runtime_overlays: outcome
                .runtime_effect_overlays
                .iter()
                .map(|overlay| self.runtime_overlay_projection(overlay))
                .collect::<Result<Vec<_>, _>>()?,
            derivation_trace_count: outcome.derivation_traces.len(),
            derivation_traces: outcome
                .derivation_traces
                .iter()
                .map(|trace| self.derivation_trace_projection(trace))
                .collect::<Result<Vec<_>, _>>()?,
            invalidated_nodes,
            binding_diagnostics: self
                .binding_diagnostics_by_tree_id(outcome)?
                .into_values()
                .flatten()
                .collect(),
            phase_timings_micros: outcome
                .phase_timings_micros
                .iter()
                .map(|(phase, micros)| (phase_key_projection_for(phase), *micros))
                .collect(),
            diagnostics: outcome.diagnostics.clone(),
        })
    }

    fn derivation_trace_projection(
        &self,
        trace: &DerivationTraceRecord,
    ) -> Result<DerivationTraceProjection, TreeWorkspaceSessionError> {
        Ok(DerivationTraceProjection {
            trace_schema_id: trace.trace_schema_id.clone(),
            owner: self.node_id_for_tree_node(trace.owner_node_id)?,
            owner_key: node_key_for_tree_node(trace.owner_node_id),
            formula_artifact_id: trace.formula_artifact_id.clone(),
            bind_artifact_id: trace.bind_artifact_id.clone(),
            formula_stable_id: trace.formula_stable_id.clone(),
            trace_mode: trace.trace_mode.clone(),
            template_selection: DerivationTemplateSelectionProjection {
                prepared_formula_key: trace.template_selection.prepared_formula_key.clone(),
                shape_key: trace.template_selection.shape_key.clone(),
                dispatch_skeleton_key: trace.template_selection.dispatch_skeleton_key.clone(),
                plan_template_key: trace.template_selection.plan_template_key.clone(),
                template_holes: trace
                    .template_selection
                    .template_holes
                    .iter()
                    .map(|hole| DerivationTemplateHoleProjection {
                        hole_id: hole.hole_id.clone(),
                        ordinal: hole.ordinal,
                        path: hole.path.clone(),
                        kind: hole.kind.clone(),
                    })
                    .collect(),
            },
            hole_bindings: trace
                .hole_bindings
                .iter()
                .map(|binding| DerivationHoleBindingProjection {
                    hole_id: binding.hole_id.clone(),
                    payload: binding.payload.clone(),
                })
                .collect(),
            sub_invocation_tree: trace
                .sub_invocation_tree
                .iter()
                .map(derivation_invocation_projection)
                .collect(),
            kernel_returned_value: trace.kernel_returned_value.clone(),
            kernel_returned_value_typed: trace
                .kernel_returned_calc_value
                .as_ref()
                .map(|value| calc_value_projection(value, None)),
            oxfml_trace_events: trace
                .oxfml_trace_events
                .iter()
                .map(|event| DerivationOxfmlTraceEventProjection {
                    trace_schema_id: event.trace_schema_id.clone(),
                    event_kind: event.event_kind.clone(),
                    formula_stable_id: event.formula_stable_id.clone(),
                    session_id: event.session_id.clone(),
                    candidate_result_id: event.candidate_result_id.clone(),
                    commit_attempt_id: event.commit_attempt_id.clone(),
                    event_order_key: event.event_order_key,
                })
                .collect(),
        })
    }

    fn runtime_overlay_projection(
        &self,
        overlay: &OverlayEntry,
    ) -> Result<RuntimeOverlayProjection, TreeWorkspaceSessionError> {
        Ok(RuntimeOverlayProjection {
            owner: self.node_id_for_tree_node(overlay.key.owner_node_id)?,
            owner_key: node_key_for_tree_node(overlay.key.owner_node_id),
            kind: runtime_overlay_kind_projection_for(overlay.key.overlay_kind),
            structural_snapshot_id: overlay.key.structural_snapshot_id.to_string(),
            compatibility_basis: overlay.key.compatibility_basis.clone(),
            payload_identity: overlay.key.payload_identity.clone(),
            is_protected: overlay.is_protected,
            is_eviction_eligible: overlay.is_eviction_eligible,
            detail: overlay.detail.clone(),
        })
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
    let runtime_policy = OxCalcTreeRuntimePolicy {
        policy_id: "runtime-policy:dnatreecalc-skin-ir-trace".to_string(),
        derivation_trace_enabled: true,
        ..OxCalcTreeRuntimePolicy::default()
    };
    OxCalcTreeContext::new(
        OxCalcTreeContextOptions::new()
            .with_runtime_policy(runtime_policy)
            .with_host_capabilities(OxCalcTreeHostCapabilitySnapshot {
                capability_profile_id: capability_profile_id.to_string(),
                dynamic_dependency_effects: true,
                execution_restriction_effects: true,
                capability_sensitive_effects: true,
                shape_topology_effects: true,
            }),
    )
}

fn leaked_profile(profile: String) -> &'static str {
    match profile.as_str() {
        "strict-excel" => "strict-excel",
        "treecalc-v1" => "treecalc-v1",
        other => Box::leak(other.to_string().into_boxed_str()),
    }
}

fn table_body_cell_content(column: &TableColumnFixture, row_id: &str) -> Option<String> {
    match column.body.kind {
        TableColumnBodyKind::ConstantCells => column
            .body
            .constants
            .iter()
            .find(|cell| cell.row_id == row_id)
            .map(|cell| cell.value.clone()),
        // Row-context table formulas need the table formula runtime path;
        // generating ordinary child formulas here rejects in current OxCalc.
        TableColumnBodyKind::Formula => None,
    }
}

fn table_body_cell_symbol(row_ordinal: u32, column_ordinal: u32) -> String {
    format!("__table_body_r{row_ordinal}_c{column_ordinal}")
}

fn bumped_table_version(current: &str, kind: &str, id: &str) -> String {
    format!("{current};{kind}+={id}")
}

fn bumped_formula_text_version(current: &str) -> String {
    let next = parse_formula_text_version(current).saturating_add(1);
    format!("v{next}")
}

fn table_formula_metadata_for_column(
    table_path: &str,
    column_id: &str,
    formula_text: String,
) -> TreeCalcTableFormulaMetadata {
    let identity = format!(
        "{}.Columns.{}",
        table_formula_identity_segment(table_path),
        table_formula_identity_segment(column_id)
    );
    TreeCalcTableFormulaMetadata {
        formula_artifact_id: format!("formula:{identity}"),
        bind_artifact_id: Some(format!("bind:{identity}")),
        formula_text_version: "v1".to_string(),
        formula_text,
    }
}

fn table_formula_metadata_for_totals(
    table_path: &str,
    column_id: &str,
    formula_text: String,
) -> TreeCalcTableFormulaMetadata {
    let identity = format!(
        "{}.Totals.{}",
        table_formula_identity_segment(table_path),
        table_formula_identity_segment(column_id)
    );
    TreeCalcTableFormulaMetadata {
        formula_artifact_id: format!("formula:{identity}"),
        bind_artifact_id: Some(format!("bind:{identity}")),
        formula_text_version: "v1".to_string(),
        formula_text,
    }
}

fn table_formula_identity_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn attribute_key_is_path_safe(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
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
    #[error("unknown table {table}")]
    UnknownTable { table: String },
    #[error("table {table} requires a non-empty name")]
    EmptyTableName { table: String },
    #[error("table name {name} is already used while renaming {table}")]
    DuplicateTableName { table: String, name: String },
    #[error("duplicate row {row_id} in table {table}")]
    DuplicateTableRow { table: String, row_id: String },
    #[error("duplicate column {column_id} in table {table}")]
    DuplicateTableColumn { table: String, column_id: String },
    #[error("unknown row {row_id} in table {table}")]
    UnknownTableRow { table: String, row_id: String },
    #[error("unknown column {column_id} in table {table}")]
    UnknownTableColumn { table: String, column_id: String },
    #[error("duplicate input for column {column_id} in table {table}")]
    DuplicateTableCellInput { table: String, column_id: String },
    #[error("duplicate input for row {row_id} in table {table}")]
    DuplicateTableRowInput { table: String, row_id: String },
    #[error("unknown cell {row_id}/{column_id} in table {table}")]
    UnknownTableCell {
        table: String,
        row_id: String,
        column_id: String,
    },
    #[error(
        "table formula column {column_id} in table {table} is calculated, not directly editable"
    )]
    FormulaTableCellEdit { table: String, column_id: String },
    #[error("table constant column {column_id} in table {table} does not carry formula metadata")]
    ConstantTableColumnFormulaEdit { table: String, column_id: String },
    #[error("initial node content policy {policy} is not yet supported")]
    UnsupportedInitialContent { policy: String },
    #[error("initial node content policy {policy} was rejected by formula bind")]
    InitialContentBindRejected { policy: String },
    #[error("format meta path {node} is occupied by a non-meta node")]
    FormatPathReserved { node: String },
    #[error("note meta path {node} is occupied by a non-meta node")]
    NotePathReserved { node: String },
    #[error("attribute meta path {node} is occupied by a non-meta node")]
    AttributePathReserved { node: String },
    #[error("attribute key {key} is not path-safe")]
    InvalidAttributeKey { key: String },
    #[error("OxCalc context projection is out of sync for {node}")]
    ProjectionOutOfSync { node: String },
}

fn table_snapshot_from_fixture(
    workspace_id: &str,
    tree_node_id: TreeNodeId,
    path: &str,
    table: &TableNodeFixture,
    body_cell_nodes: Vec<TreeCalcTableBodyCellNodeBinding>,
    totals_cell_nodes: Vec<TreeCalcTableTotalsCellNodeBinding>,
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
        body_cell_nodes,
        totals_cell_nodes,
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
            formula_text: String::new(),
        },
        |formula| TreeCalcTableFormulaMetadata {
            formula_artifact_id: formula.formula_stable_id.clone(),
            bind_artifact_id: formula.bind_artifact_id.clone(),
            formula_text_version: formula.formula_text_version.clone(),
            formula_text: formula.formula_text.clone(),
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

fn renamed_node_path(node: &NodeId, new_symbol: &str) -> NodeId {
    match parent_path(node.as_str()) {
        Some(parent) => NodeId::new(format!("{parent}.{new_symbol}")),
        None => NodeId::new(new_symbol.to_string()),
    }
}

fn added_node_path(parent: Option<&NodeId>, symbol: &str) -> NodeId {
    match parent {
        Some(parent) => NodeId::new(format!("{}.{}", parent.as_str(), symbol)),
        None => NodeId::new(symbol.to_string()),
    }
}

fn moved_node_path(node: &NodeId, new_parent: Option<&NodeId>) -> NodeId {
    let symbol = node_symbol(node.as_str());
    match new_parent {
        Some(parent) => NodeId::new(format!("{}.{}", parent.as_str(), symbol)),
        None => NodeId::new(symbol.to_string()),
    }
}

fn node_symbol(path: &str) -> &str {
    path.rsplit_once('.').map_or(path, |(_, symbol)| symbol)
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
    calc_value: Option<&CalcValue>,
    effective_format: Option<&EffectiveFormatProjection>,
) -> NodeValueProjection {
    match calc_state {
        Some(NodeCalcState::RejectedPendingRepair | NodeCalcState::CycleBlocked) => {
            NodeValueProjection::Error(
                value_text.unwrap_or_else(|| "calculation rejected".to_string()),
            )
        }
        _ => calc_value.map_or_else(
            || {
                value_text.map_or(
                    NodeValueProjection::Unevaluated,
                    NodeValueProjection::Scalar,
                )
            },
            |value| calc_value_projection(value, effective_format),
        ),
    }
}

fn literalized_value_input_for(calc_value: Option<&CalcValue>) -> Option<String> {
    let value = calc_value?;
    match RuntimeEnvironment::new().literalize_calc_value_for_authoring(value) {
        RuntimeValueLiteralizationResult::AuthoredInput(literalized) => {
            Some(literalized.authored_input_text)
        }
        RuntimeValueLiteralizationResult::Unsupported(_) => None,
    }
}

fn calc_value_projection(
    value: &CalcValue,
    effective_format: Option<&EffectiveFormatProjection>,
) -> NodeValueProjection {
    match value.core() {
        CoreValue::Array(array) => {
            let shape = array.shape();
            let cells = (0..shape.rows)
                .map(|row| {
                    (0..shape.cols)
                        .map(|col| {
                            array
                                .get(row, col)
                                .map(|value| calc_value_projection(value, effective_format))
                                .unwrap_or(NodeValueProjection::Empty)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            NodeValueProjection::Array {
                rows: shape.rows,
                cols: shape.cols,
                cells,
            }
        }
        CoreValue::Error(_) => NodeValueProjection::Error(calc_value_display_text(value)),
        CoreValue::Number(number) => {
            let display = number.to_string();
            NodeValueProjection::Number {
                raw: display.clone(),
                display: render_number_with_effective_format(*number, effective_format)
                    .unwrap_or(display),
            }
        }
        CoreValue::Text(text) => NodeValueProjection::Text(text.to_string_lossy()),
        CoreValue::Logical(logical) => {
            let display = logical.to_string();
            NodeValueProjection::Logical {
                value: *logical,
                display,
            }
        }
        CoreValue::Empty => NodeValueProjection::Empty,
        CoreValue::Missing => NodeValueProjection::Missing,
        CoreValue::Reference(reference) => NodeValueProjection::Reference {
            target: reference.target().to_string(),
        },
    }
}

fn render_number_with_effective_format(
    number: f64,
    effective_format: Option<&EffectiveFormatProjection>,
) -> Option<String> {
    let code = effective_format
        .and_then(|format| format.number_format_code.as_deref())?
        .trim();
    if code.is_empty() {
        return None;
    }
    render_with_code(
        &oxfml_en_us_format_profile(),
        WorkbookDateSystem::System1900,
        number,
        code,
    )
    .ok()
}

fn calc_value_display_text(value: &CalcValue) -> String {
    match value.core() {
        CoreValue::Number(number) => number.to_string(),
        CoreValue::Text(text) => text.to_string_lossy(),
        CoreValue::Logical(logical) => logical.to_string(),
        CoreValue::Error(error) => format!("{error:?}"),
        CoreValue::Empty => String::new(),
        CoreValue::Missing => "missing".to_string(),
        CoreValue::Array(array) => {
            let shape = array.shape();
            format!("Array({}x{})", shape.rows, shape.cols)
        }
        CoreValue::Reference(reference) => reference.target().to_string(),
    }
}

fn calc_state_projection_for(calc_state: NodeCalcState) -> NodeCalcStateProjection {
    match calc_state {
        NodeCalcState::Clean => NodeCalcStateProjection::Clean,
        NodeCalcState::DirtyPending => NodeCalcStateProjection::DirtyPending,
        NodeCalcState::Needed => NodeCalcStateProjection::Needed,
        NodeCalcState::Evaluating => NodeCalcStateProjection::Evaluating,
        NodeCalcState::VerifiedClean => NodeCalcStateProjection::VerifiedClean,
        NodeCalcState::PublishReady => NodeCalcStateProjection::PublishReady,
        NodeCalcState::RejectedPendingRepair => NodeCalcStateProjection::RejectedPendingRepair,
        NodeCalcState::CycleBlocked => NodeCalcStateProjection::CycleBlocked,
    }
}

fn node_key_for_tree_node(tree_node_id: TreeNodeId) -> NodeKey {
    NodeKey::from_engine_id(tree_node_id.0)
}

struct FormulaBindPreviewPayload {
    input_kind: FormulaBindPreviewInputKind,
    legal: bool,
    diagnostics: Vec<FormulaBindPreviewDiagnosticProjection>,
    profile_violations: Vec<FormulaBindPreviewProfileViolationProjection>,
}

fn formula_bind_preview_from_oxcalc_verdict(
    verdict: OxCalcTreeDryBindVerdict,
) -> FormulaBindPreviewPayload {
    FormulaBindPreviewPayload {
        input_kind: match verdict.input_kind {
            OxCalcTreeDryBindInputKind::Literal => FormulaBindPreviewInputKind::Literal,
            OxCalcTreeDryBindInputKind::Formula => FormulaBindPreviewInputKind::Formula,
        },
        legal: verdict.legal,
        diagnostics: verdict
            .diagnostics
            .into_iter()
            .map(|diagnostic| FormulaBindPreviewDiagnosticProjection {
                stage: match diagnostic.stage {
                    OxCalcTreeDryBindDiagnosticStage::Syntax => {
                        FormulaBindPreviewDiagnosticStage::Syntax
                    }
                    OxCalcTreeDryBindDiagnosticStage::Bind => {
                        FormulaBindPreviewDiagnosticStage::Bind
                    }
                },
                message: diagnostic.message,
                span: SourceSpanProjection {
                    start_utf8: diagnostic.span_start_utf8,
                    end_utf8: diagnostic.span_start_utf8 + diagnostic.span_len_utf8,
                },
            })
            .collect(),
        profile_violations: verdict
            .profile_violations
            .into_iter()
            .map(|violation| FormulaBindPreviewProfileViolationProjection {
                kind: match violation.kind {
                    OxCalcTreeDryBindProfileViolationKind::FunctionUnavailable {
                        function_id,
                        function_name,
                        reason,
                    } => FormulaBindPreviewProfileViolationKindProjection::FunctionUnavailable {
                        function_id,
                        function_name,
                        reason,
                    },
                },
                feature: violation.feature,
                message: violation.message,
                span: SourceSpanProjection {
                    start_utf8: violation.span_start_utf8,
                    end_utf8: violation.span_start_utf8 + violation.span_len_utf8,
                },
            })
            .collect(),
    }
}

fn mutation_impact_blocked_reason_for_bind(
    bind: &FormulaBindPreviewProjection,
) -> Option<MutationImpactBlockedReasonProjection> {
    if !bind.profile_violations.is_empty() {
        return Some(MutationImpactBlockedReasonProjection::ProfileViolation);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    {
        return Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind)
    {
        return Some(MutationImpactBlockedReasonProjection::BindDiagnostics);
    }
    None
}

fn mutation_impact_blocked_reason_for_bind_payload(
    bind: &FormulaBindPreviewPayload,
) -> Option<MutationImpactBlockedReasonProjection> {
    if !bind.profile_violations.is_empty() {
        return Some(MutationImpactBlockedReasonProjection::ProfileViolation);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    {
        return Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind)
    {
        return Some(MutationImpactBlockedReasonProjection::BindDiagnostics);
    }
    None
}

fn mutation_impact_blocked_reason_for_binds(
    binds: &[FormulaBindPreviewProjection],
) -> Option<MutationImpactBlockedReasonProjection> {
    if binds.iter().any(|bind| !bind.profile_violations.is_empty()) {
        return Some(MutationImpactBlockedReasonProjection::ProfileViolation);
    }
    if binds.iter().any(|bind| {
        bind.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    }) {
        return Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics);
    }
    if binds.iter().any(|bind| {
        bind.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind)
    }) {
        return Some(MutationImpactBlockedReasonProjection::BindDiagnostics);
    }
    None
}

fn mutation_impact_blocked_reason_for_table_bind(
    bind: &TableFormulaBindPreviewProjection,
) -> Option<MutationImpactBlockedReasonProjection> {
    if !bind.profile_violations.is_empty() {
        return Some(MutationImpactBlockedReasonProjection::ProfileViolation);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Syntax)
    {
        return Some(MutationImpactBlockedReasonProjection::SyntaxDiagnostics);
    }
    if bind
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.stage == FormulaBindPreviewDiagnosticStage::Bind)
    {
        return Some(MutationImpactBlockedReasonProjection::BindDiagnostics);
    }
    None
}

fn table_mutation_impact(
    intent: MutationImpactIntentProjection,
    blocked_reason: Option<MutationImpactBlockedReasonProjection>,
    invalidation_plan: RecalcPlanProjection,
    table: &NodeId,
) -> MutationImpactProjection {
    MutationImpactProjection {
        intent,
        legal: blocked_reason.is_none(),
        blocked_reason,
        profile_violations: Vec::new(),
        bind_diagnostics: Vec::new(),
        requires_rebind: invalidation_plan.requires_rebind.clone(),
        affected_refs: invalidation_plan
            .invalidated_nodes
            .iter()
            .filter(|entry| entry.node != *table)
            .map(|entry| entry.node.clone())
            .collect(),
        orphaned_dependents: Vec::new(),
        collisions: Vec::new(),
        invalidation_plan,
    }
}

fn orphaned_dependents_for_deleted_keys(
    state: &WorkspaceState,
    deleted_keys: &[NodeKey],
) -> Vec<NodeId> {
    let deleted_key_set = deleted_keys.iter().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut dependents = Vec::new();
    for deleted_key in deleted_keys {
        let Some(handles) = state.dependencies.reverse_references.get(deleted_key) else {
            continue;
        };
        for handle in handles {
            let Some(resolution) = state.dependencies.reference_resolutions.get(handle) else {
                continue;
            };
            if deleted_key_set.contains(&resolution.owner_key) {
                continue;
            }
            if seen.insert(resolution.owner.clone()) {
                dependents.push(resolution.owner.clone());
            }
        }
    }
    dependents
}

fn should_replace_reference_target(
    current: &ReferenceTargetProjection,
    candidate: &ReferenceTargetProjection,
) -> bool {
    match (current, candidate) {
        (ReferenceTargetProjection::Collection { .. }, _) => false,
        (_, ReferenceTargetProjection::Collection { .. }) => true,
        (ReferenceTargetProjection::Unresolved, ReferenceTargetProjection::Unresolved) => false,
        (ReferenceTargetProjection::Unresolved, _) => true,
        _ => false,
    }
}

fn reference_target_keys(target: &ReferenceTargetProjection) -> Vec<NodeKey> {
    match target {
        ReferenceTargetProjection::Node { key, .. } => vec![key.clone()],
        ReferenceTargetProjection::Collection { member_keys, .. } => member_keys.clone(),
        ReferenceTargetProjection::External { .. } | ReferenceTargetProjection::Unresolved => {
            Vec::new()
        }
    }
}

fn derivation_invocation_projection(
    invocation: &DerivationInvocationTraceNode,
) -> DerivationInvocationProjection {
    DerivationInvocationProjection {
        invocation_ordinal: invocation.invocation_ordinal,
        invocation_kind: invocation.invocation_kind.clone(),
        function_name: invocation.function_name.clone(),
        function_id: invocation.function_id.clone(),
        arg_preparation_profile: invocation.arg_preparation_profile.clone(),
        prepared_arguments: invocation
            .prepared_arguments
            .iter()
            .map(derivation_prepared_argument_projection)
            .collect(),
        kernel_returned_value: invocation.kernel_returned_value.clone(),
        kernel_returned_value_typed: invocation
            .kernel_returned_calc_value
            .as_ref()
            .map(|value| calc_value_projection(value, None)),
        children: invocation
            .children
            .iter()
            .map(derivation_invocation_projection)
            .collect(),
    }
}

fn derivation_prepared_argument_projection(
    argument: &DerivationPreparedArgumentTrace,
) -> DerivationPreparedArgumentProjection {
    DerivationPreparedArgumentProjection {
        ordinal: argument.ordinal,
        structure_class: argument.structure_class.clone(),
        source_class: argument.source_class.clone(),
        evaluation_mode: argument.evaluation_mode.clone(),
        blankness_class: argument.blankness_class.clone(),
        caller_context_sensitive: argument.caller_context_sensitive,
        reference_target: argument.reference_target.clone(),
        opaque_reason: argument.opaque_reason.clone(),
        resolved_value: argument.resolved_value.clone(),
        resolved_value_typed: argument
            .resolved_calc_value
            .as_ref()
            .map(|value| calc_value_projection(value, None)),
    }
}

fn runtime_effect_projection(effect: &RuntimeEffect) -> RuntimeEffectProjection {
    RuntimeEffectProjection {
        kind: effect.kind.clone(),
        family: runtime_effect_family_projection_for(&effect.family),
        detail: effect.detail.clone(),
    }
}

fn runtime_effect_family_projection_for(
    family: &RuntimeEffectFamily,
) -> RuntimeEffectFamilyProjection {
    match family {
        RuntimeEffectFamily::DynamicDependency => RuntimeEffectFamilyProjection::DynamicDependency,
        RuntimeEffectFamily::ExecutionRestriction => {
            RuntimeEffectFamilyProjection::ExecutionRestriction
        }
        RuntimeEffectFamily::CapabilitySensitive => {
            RuntimeEffectFamilyProjection::CapabilitySensitive
        }
        RuntimeEffectFamily::ShapeTopology => RuntimeEffectFamilyProjection::ShapeTopology,
    }
}

fn runtime_overlay_kind_projection_for(kind: OverlayKind) -> RuntimeOverlayKindProjection {
    match kind {
        OverlayKind::InvalidationExecutionState => {
            RuntimeOverlayKindProjection::InvalidationExecutionState
        }
        OverlayKind::DynamicDependency => RuntimeOverlayKindProjection::DynamicDependency,
        OverlayKind::ExecutionRestriction => RuntimeOverlayKindProjection::ExecutionRestriction,
        OverlayKind::ShapeTopology => RuntimeOverlayKindProjection::ShapeTopology,
        OverlayKind::CapabilityFenceAttachment => {
            RuntimeOverlayKindProjection::CapabilityFenceAttachment
        }
        OverlayKind::ObserverPriorityMetadata => {
            RuntimeOverlayKindProjection::ObserverPriorityMetadata
        }
    }
}

fn dependency_kind_projection_for(kind: DependencyDescriptorKind) -> DependencyKindProjection {
    match kind {
        DependencyDescriptorKind::StaticDirect => DependencyKindProjection::StaticDirect,
        DependencyDescriptorKind::RelativeBound => DependencyKindProjection::RelativeBound,
        DependencyDescriptorKind::TreeReferenceCollectionMembership => {
            DependencyKindProjection::TreeReferenceCollectionMembership
        }
        DependencyDescriptorKind::TreeReferenceCollectionMemberValue => {
            DependencyKindProjection::TreeReferenceCollectionMemberValue
        }
        DependencyDescriptorKind::StructuredTableIdentity => {
            DependencyKindProjection::StructuredTableIdentity
        }
        DependencyDescriptorKind::StructuredTableRowMembership => {
            DependencyKindProjection::StructuredTableRowMembership
        }
        DependencyDescriptorKind::StructuredTableRowOrder => {
            DependencyKindProjection::StructuredTableRowOrder
        }
        DependencyDescriptorKind::StructuredTableColumnIdentity => {
            DependencyKindProjection::StructuredTableColumnIdentity
        }
        DependencyDescriptorKind::StructuredTableHeaderText => {
            DependencyKindProjection::StructuredTableHeaderText
        }
        DependencyDescriptorKind::StructuredTableHeaderRegion => {
            DependencyKindProjection::StructuredTableHeaderRegion
        }
        DependencyDescriptorKind::StructuredTableDataRegion => {
            DependencyKindProjection::StructuredTableDataRegion
        }
        DependencyDescriptorKind::StructuredTableTotalsRegion => {
            DependencyKindProjection::StructuredTableTotalsRegion
        }
        DependencyDescriptorKind::StructuredTableCallerContext => {
            DependencyKindProjection::StructuredTableCallerContext
        }
        DependencyDescriptorKind::StructuredTableEnclosingTable => {
            DependencyKindProjection::StructuredTableEnclosingTable
        }
        DependencyDescriptorKind::DynamicPotential => DependencyKindProjection::DynamicPotential,
        DependencyDescriptorKind::HostSensitive => DependencyKindProjection::HostSensitive,
        DependencyDescriptorKind::CapabilitySensitive => {
            DependencyKindProjection::CapabilitySensitive
        }
        DependencyDescriptorKind::ShapeTopology => DependencyKindProjection::ShapeTopology,
        DependencyDescriptorKind::Unresolved => DependencyKindProjection::Unresolved,
    }
}

fn invalidation_reason_projection_for(
    reason: InvalidationReasonKind,
) -> InvalidationReasonProjection {
    match reason {
        InvalidationReasonKind::StructuralRebindRequired => {
            InvalidationReasonProjection::StructuralRebindRequired
        }
        InvalidationReasonKind::StructuralRecalcOnly => {
            InvalidationReasonProjection::StructuralRecalcOnly
        }
        InvalidationReasonKind::UpstreamPublication => {
            InvalidationReasonProjection::UpstreamPublication
        }
        InvalidationReasonKind::ExternallyInvalidated => {
            InvalidationReasonProjection::ExternallyInvalidated
        }
        InvalidationReasonKind::TreeReferenceMembershipChanged => {
            InvalidationReasonProjection::TreeReferenceMembershipChanged
        }
        InvalidationReasonKind::TreeReferenceOrderChanged => {
            InvalidationReasonProjection::TreeReferenceOrderChanged
        }
        InvalidationReasonKind::StructuredTableContextChanged => {
            InvalidationReasonProjection::StructuredTableContextChanged
        }
        InvalidationReasonKind::StructuredTableRowMembershipChanged => {
            InvalidationReasonProjection::StructuredTableRowMembershipChanged
        }
        InvalidationReasonKind::StructuredTableRowOrderChanged => {
            InvalidationReasonProjection::StructuredTableRowOrderChanged
        }
        InvalidationReasonKind::StructuredTableColumnChanged => {
            InvalidationReasonProjection::StructuredTableColumnChanged
        }
        InvalidationReasonKind::StructuredTableRegionChanged => {
            InvalidationReasonProjection::StructuredTableRegionChanged
        }
        InvalidationReasonKind::StructuredTableCallerContextChanged => {
            InvalidationReasonProjection::StructuredTableCallerContextChanged
        }
        InvalidationReasonKind::DependencyAdded => InvalidationReasonProjection::DependencyAdded,
        InvalidationReasonKind::DependencyRemoved => {
            InvalidationReasonProjection::DependencyRemoved
        }
        InvalidationReasonKind::DependencyReclassified => {
            InvalidationReasonProjection::DependencyReclassified
        }
        InvalidationReasonKind::DynamicDependencyActivated => {
            InvalidationReasonProjection::DynamicDependencyActivated
        }
        InvalidationReasonKind::DynamicDependencyReleased => {
            InvalidationReasonProjection::DynamicDependencyReleased
        }
        InvalidationReasonKind::DynamicDependencyReclassified => {
            InvalidationReasonProjection::DynamicDependencyReclassified
        }
    }
}

fn invalidation_reason_kind_for_projection(
    reason: InvalidationReasonProjection,
) -> InvalidationReasonKind {
    match reason {
        InvalidationReasonProjection::StructuralRebindRequired => {
            InvalidationReasonKind::StructuralRebindRequired
        }
        InvalidationReasonProjection::StructuralRecalcOnly => {
            InvalidationReasonKind::StructuralRecalcOnly
        }
        InvalidationReasonProjection::UpstreamPublication => {
            InvalidationReasonKind::UpstreamPublication
        }
        InvalidationReasonProjection::ExternallyInvalidated => {
            InvalidationReasonKind::ExternallyInvalidated
        }
        InvalidationReasonProjection::TreeReferenceMembershipChanged => {
            InvalidationReasonKind::TreeReferenceMembershipChanged
        }
        InvalidationReasonProjection::TreeReferenceOrderChanged => {
            InvalidationReasonKind::TreeReferenceOrderChanged
        }
        InvalidationReasonProjection::StructuredTableContextChanged => {
            InvalidationReasonKind::StructuredTableContextChanged
        }
        InvalidationReasonProjection::StructuredTableRowMembershipChanged => {
            InvalidationReasonKind::StructuredTableRowMembershipChanged
        }
        InvalidationReasonProjection::StructuredTableRowOrderChanged => {
            InvalidationReasonKind::StructuredTableRowOrderChanged
        }
        InvalidationReasonProjection::StructuredTableColumnChanged => {
            InvalidationReasonKind::StructuredTableColumnChanged
        }
        InvalidationReasonProjection::StructuredTableRegionChanged => {
            InvalidationReasonKind::StructuredTableRegionChanged
        }
        InvalidationReasonProjection::StructuredTableCallerContextChanged => {
            InvalidationReasonKind::StructuredTableCallerContextChanged
        }
        InvalidationReasonProjection::DependencyAdded => InvalidationReasonKind::DependencyAdded,
        InvalidationReasonProjection::DependencyRemoved => {
            InvalidationReasonKind::DependencyRemoved
        }
        InvalidationReasonProjection::DependencyReclassified => {
            InvalidationReasonKind::DependencyReclassified
        }
        InvalidationReasonProjection::DynamicDependencyActivated => {
            InvalidationReasonKind::DynamicDependencyActivated
        }
        InvalidationReasonProjection::DynamicDependencyReleased => {
            InvalidationReasonKind::DynamicDependencyReleased
        }
        InvalidationReasonProjection::DynamicDependencyReclassified => {
            InvalidationReasonKind::DynamicDependencyReclassified
        }
    }
}

fn tree_collection_family_projection_for(
    family: TreeReferenceCollectionFamily,
) -> TreeReferenceCollectionFamilyProjection {
    match family {
        TreeReferenceCollectionFamily::ChildrenV1 => {
            TreeReferenceCollectionFamilyProjection::Children
        }
        TreeReferenceCollectionFamily::ReferenceLiteralArrayV1 => {
            TreeReferenceCollectionFamilyProjection::ReferenceLiteralArray
        }
        TreeReferenceCollectionFamily::SiblingSetV1 => {
            TreeReferenceCollectionFamilyProjection::Siblings
        }
        TreeReferenceCollectionFamily::PrecedingV1 => {
            TreeReferenceCollectionFamilyProjection::Preceding
        }
        TreeReferenceCollectionFamily::FollowingV1 => {
            TreeReferenceCollectionFamilyProjection::Following
        }
        TreeReferenceCollectionFamily::AncestorsV1 => {
            TreeReferenceCollectionFamilyProjection::Ancestors
        }
        TreeReferenceCollectionFamily::RecursiveDescendantsV1 => {
            TreeReferenceCollectionFamilyProjection::RecursiveDescendants
        }
    }
}

fn phase_key_projection_for(phase: &LocalTreeCalcPhaseKey) -> PhaseKeyProjection {
    match phase {
        LocalTreeCalcPhaseKey::OxfmlPrepareFormulas => PhaseKeyProjection::OxfmlPrepareFormulas,
        LocalTreeCalcPhaseKey::DependencyDescriptorLowering => {
            PhaseKeyProjection::DependencyDescriptorLowering
        }
        LocalTreeCalcPhaseKey::DependencyDescriptorOwnerIndex => {
            PhaseKeyProjection::DependencyDescriptorOwnerIndex
        }
        LocalTreeCalcPhaseKey::DependencyGraphBuildAndCycleScan => {
            PhaseKeyProjection::DependencyGraphBuildAndCycleScan
        }
        LocalTreeCalcPhaseKey::InvalidationClosureDerivation => {
            PhaseKeyProjection::InvalidationClosureDerivation
        }
        LocalTreeCalcPhaseKey::RuntimeSetup => PhaseKeyProjection::RuntimeSetup,
        LocalTreeCalcPhaseKey::DiagnosticSeedCollection => {
            PhaseKeyProjection::DiagnosticSeedCollection
        }
        LocalTreeCalcPhaseKey::RecalcTrackerMarkDirtyNeeded => {
            PhaseKeyProjection::RecalcTrackerMarkDirtyNeeded
        }
        LocalTreeCalcPhaseKey::TopologicalFormulaOrder => {
            PhaseKeyProjection::TopologicalFormulaOrder
        }
        LocalTreeCalcPhaseKey::RebindGateScan => PhaseKeyProjection::RebindGateScan,
        LocalTreeCalcPhaseKey::DependencyDiagnosticRejectScan => {
            PhaseKeyProjection::DependencyDiagnosticRejectScan
        }
        LocalTreeCalcPhaseKey::EdgeValueCacheLookup => PhaseKeyProjection::EdgeValueCacheLookup,
        LocalTreeCalcPhaseKey::OxfmlFormulaEvaluation => PhaseKeyProjection::OxfmlFormulaEvaluation,
        LocalTreeCalcPhaseKey::DerivationTraceRecord => PhaseKeyProjection::DerivationTraceRecord,
        LocalTreeCalcPhaseKey::EdgeValueCacheStore => PhaseKeyProjection::EdgeValueCacheStore,
        LocalTreeCalcPhaseKey::EvaluationLoopTotal => PhaseKeyProjection::EvaluationLoopTotal,
        LocalTreeCalcPhaseKey::VerifiedCleanFinalize => PhaseKeyProjection::VerifiedCleanFinalize,
        LocalTreeCalcPhaseKey::CandidatePublication => PhaseKeyProjection::CandidatePublication,
        LocalTreeCalcPhaseKey::RejectionRecording => PhaseKeyProjection::RejectionRecording,
        LocalTreeCalcPhaseKey::TotalEngineExecute => PhaseKeyProjection::TotalEngineExecute,
        LocalTreeCalcPhaseKey::Other(value) => PhaseKeyProjection::Other(value.clone()),
    }
}

fn table_projection_for(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> TableProjection {
    TableProjection {
        table_id: view.table_id.clone(),
        table_name: view.table_name.clone(),
        display_path: view.display_path.clone(),
        canonical_path: view.canonical_path.clone(),
        virtual_anchor: TableAnchorProjection {
            workbook_scope_ref: view.snapshot.virtual_anchor.workbook_scope_ref.clone(),
            sheet_scope_ref: view.snapshot.virtual_anchor.sheet_scope_ref.clone(),
            start_row: view.snapshot.virtual_anchor.start_row,
            start_col: view.snapshot.virtual_anchor.start_col,
        },
        rows: view
            .snapshot
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| TableRowProjection {
                row_id: row.0.clone(),
                ordinal: index + 1,
            })
            .collect(),
        columns: view
            .snapshot
            .columns
            .iter()
            .map(table_column_projection)
            .collect(),
        cells: table_cells_projection_for(view, node_views, published_calc_values),
        row_count: view.snapshot.rows.len(),
        column_count: view.snapshot.columns.len(),
        header_row_present: view.snapshot.header_row_present,
        totals_row_present: view.snapshot.totals_row_present,
        table_namespace_version: view.snapshot.table_namespace_version.clone(),
        row_membership_version: view.snapshot.row_membership_version.clone(),
        row_order_version: view.snapshot.row_order_version.clone(),
        column_identity_version: view.snapshot.column_identity_version.clone(),
        dependency_inventory: view
            .dependency_inventory
            .facts
            .iter()
            .map(table_dependency_fact_projection)
            .collect(),
    }
}

fn table_dependency_fact_projection(
    fact: &StructuredTableDependencyFact,
) -> TableDependencyFactProjection {
    TableDependencyFactProjection {
        fact_id: fact.fact_id.clone(),
        kind: table_dependency_fact_kind_projection(fact.kind),
        status: table_dependency_fact_status_projection(fact.status),
        table_id: fact.table_id.clone(),
        column_id: fact.column_id.clone(),
        identity: fact.identity.clone(),
        blocker: fact.blocker.map(table_dependency_fact_blocker_projection),
        detail: fact.detail.clone(),
    }
}

fn table_dependency_fact_kind_projection(
    kind: StructuredTableDependencyFactKind,
) -> TableDependencyFactKindProjection {
    match kind {
        StructuredTableDependencyFactKind::TableIdentity => {
            TableDependencyFactKindProjection::TableIdentity
        }
        StructuredTableDependencyFactKind::RowMembership => {
            TableDependencyFactKindProjection::RowMembership
        }
        StructuredTableDependencyFactKind::RowOrder => TableDependencyFactKindProjection::RowOrder,
        StructuredTableDependencyFactKind::RowValue => TableDependencyFactKindProjection::RowValue,
        StructuredTableDependencyFactKind::ColumnIdentity => {
            TableDependencyFactKindProjection::ColumnIdentity
        }
        StructuredTableDependencyFactKind::ColumnOrder => {
            TableDependencyFactKindProjection::ColumnOrder
        }
        StructuredTableDependencyFactKind::HeaderText => {
            TableDependencyFactKindProjection::HeaderText
        }
        StructuredTableDependencyFactKind::HeaderRegion => {
            TableDependencyFactKindProjection::HeaderRegion
        }
        StructuredTableDependencyFactKind::DataRegion => {
            TableDependencyFactKindProjection::DataRegion
        }
        StructuredTableDependencyFactKind::TotalsRegion => {
            TableDependencyFactKindProjection::TotalsRegion
        }
        StructuredTableDependencyFactKind::TotalsValue => {
            TableDependencyFactKindProjection::TotalsValue
        }
        StructuredTableDependencyFactKind::TotalsFormula => {
            TableDependencyFactKindProjection::TotalsFormula
        }
        StructuredTableDependencyFactKind::CallerRowContext => {
            TableDependencyFactKindProjection::CallerRowContext
        }
        StructuredTableDependencyFactKind::OmittedTableNameEnclosingTable => {
            TableDependencyFactKindProjection::OmittedTableNameEnclosingTable
        }
        StructuredTableDependencyFactKind::VirtualAnchorRange => {
            TableDependencyFactKindProjection::VirtualAnchorRange
        }
        StructuredTableDependencyFactKind::WorkspaceAvailability => {
            TableDependencyFactKindProjection::WorkspaceAvailability
        }
        StructuredTableDependencyFactKind::FunctionRegistrySnapshot => {
            TableDependencyFactKindProjection::FunctionRegistrySnapshot
        }
    }
}

fn table_dependency_fact_status_projection(
    status: StructuredTableDependencyFactStatus,
) -> TableDependencyFactStatusProjection {
    match status {
        StructuredTableDependencyFactStatus::Lowered => {
            TableDependencyFactStatusProjection::Lowered
        }
        StructuredTableDependencyFactStatus::Blocked => {
            TableDependencyFactStatusProjection::Blocked
        }
    }
}

fn table_dependency_fact_blocker_projection(
    blocker: StructuredTableLoweringBlocker,
) -> TableDependencyFactBlockerProjection {
    match blocker {
        StructuredTableLoweringBlocker::MissingTableCatalogEntry => {
            TableDependencyFactBlockerProjection::MissingTableCatalogEntry
        }
        StructuredTableLoweringBlocker::MissingEnclosingTableContext => {
            TableDependencyFactBlockerProjection::MissingEnclosingTableContext
        }
        StructuredTableLoweringBlocker::MissingStableRowMembershipAndOrderPacket => {
            TableDependencyFactBlockerProjection::MissingStableRowMembershipAndOrderPacket
        }
        StructuredTableLoweringBlocker::MissingSelectedColumn => {
            TableDependencyFactBlockerProjection::MissingSelectedColumn
        }
        StructuredTableLoweringBlocker::MissingHeaderRegionRange => {
            TableDependencyFactBlockerProjection::MissingHeaderRegionRange
        }
        StructuredTableLoweringBlocker::MissingTotalsRegionRange => {
            TableDependencyFactBlockerProjection::MissingTotalsRegionRange
        }
        StructuredTableLoweringBlocker::HeaderRowAbsent => {
            TableDependencyFactBlockerProjection::HeaderRowAbsent
        }
        StructuredTableLoweringBlocker::TotalsRowAbsent => {
            TableDependencyFactBlockerProjection::TotalsRowAbsent
        }
        StructuredTableLoweringBlocker::MissingCallerTableRegion => {
            TableDependencyFactBlockerProjection::MissingCallerTableRegion
        }
        StructuredTableLoweringBlocker::CallerTableMismatch => {
            TableDependencyFactBlockerProjection::CallerTableMismatch
        }
        StructuredTableLoweringBlocker::CallerRegionNotData => {
            TableDependencyFactBlockerProjection::CallerRegionNotData
        }
        StructuredTableLoweringBlocker::CallerDataRowOffsetMissing => {
            TableDependencyFactBlockerProjection::CallerDataRowOffsetMissing
        }
        StructuredTableLoweringBlocker::OmittedTableEnclosingMismatch => {
            TableDependencyFactBlockerProjection::OmittedTableEnclosingMismatch
        }
    }
}

fn table_cells_projection_for(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> Option<TableCellsProjection> {
    let formula_reports = table_body_formula_reports(view, node_views, published_calc_values);
    let totals_values =
        table_totals_formula_values(view, node_views, published_calc_values, &formula_reports);
    if view.snapshot.body_cell_nodes.is_empty()
        && view.snapshot.totals_cell_nodes.is_empty()
        && formula_reports.is_empty()
        && totals_values.is_empty()
    {
        return None;
    }

    let body_bindings = view
        .snapshot
        .body_cell_nodes
        .iter()
        .map(|cell| ((cell.row_id.clone(), cell.column_id.clone()), cell.node_id))
        .collect::<BTreeMap<_, _>>();
    let formula_values = formula_reports
        .iter()
        .flat_map(|(column_id, report)| {
            report.cell_results.iter().filter_map(|cell| {
                cell.row_id
                    .as_ref()
                    .map(|row_id| ((row_id.clone(), column_id.clone()), cell.value.clone()))
            })
        })
        .collect::<BTreeMap<_, _>>();
    let totals_bindings = view
        .snapshot
        .totals_cell_nodes
        .iter()
        .map(|cell| (cell.column_id.clone(), cell.node_id))
        .collect::<BTreeMap<_, _>>();

    let body_rows = view
        .snapshot
        .rows
        .iter()
        .map(|row| {
            view.snapshot
                .columns
                .iter()
                .map(|column| {
                    body_bindings
                        .get(&(row.clone(), column.column_id.clone()))
                        .and_then(|node_id| {
                            table_cell_projection(
                                Some(row.0.clone()),
                                column.column_id.clone(),
                                *node_id,
                                node_views,
                                published_calc_values,
                            )
                        })
                        .or_else(|| {
                            formula_values
                                .get(&(row.clone(), column.column_id.clone()))
                                .map(|value| TableCellProjection {
                                    row_id: Some(row.0.clone()),
                                    column_id: column.column_id.clone(),
                                    node_key: NodeKey::new(format!(
                                        "table-formula-cell:{}:{}:{}",
                                        view.table_id, row.0, column.column_id
                                    )),
                                    value: value_projection_from_calc_value(value),
                                })
                        })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let totals_row = view
        .snapshot
        .columns
        .iter()
        .map(|column| {
            totals_bindings
                .get(&column.column_id)
                .and_then(|node_id| {
                    table_cell_projection(
                        None,
                        column.column_id.clone(),
                        *node_id,
                        node_views,
                        published_calc_values,
                    )
                })
                .or_else(|| {
                    totals_values
                        .get(&column.column_id)
                        .map(|value| TableCellProjection {
                            row_id: None,
                            column_id: column.column_id.clone(),
                            node_key: NodeKey::new(format!(
                                "table-totals-cell:{}:{}",
                                view.table_id, column.column_id
                            )),
                            value: value_projection_from_calc_value(value),
                        })
                })
        })
        .collect();

    Some(TableCellsProjection {
        body_rows,
        totals_row,
    })
}

fn table_body_formula_reports(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> BTreeMap<String, TreeCalcTableFormulaRuntimeReport> {
    let values = table_sparse_values_from_bound_cells(view, node_views, published_calc_values);
    view.snapshot
        .columns
        .iter()
        .filter_map(|column| {
            let TreeCalcTableColumnBodyMetadata::Formula(formula) = &column.body_metadata else {
                return None;
            };
            if formula.formula_text.trim().is_empty() {
                return None;
            }
            let request = TreeCalcTableColumnFormulaRuntimeRequest {
                target_column_id: column.column_id.clone(),
                formula_stable_id: formula.formula_artifact_id.clone(),
                formula_text_version: parse_formula_text_version(&formula.formula_text_version),
                formula_text: formula.formula_text.clone(),
                values: values.clone(),
                runtime_context: TreeCalcTableFormulaRuntimeContext::default(),
            };
            evaluate_treecalc_table_column_formula_rows(&view.snapshot, &view.projection, &request)
                .ok()
                .map(|report| (column.column_id.clone(), report))
        })
        .collect()
}

fn table_totals_formula_values(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
    body_formula_reports: &BTreeMap<String, TreeCalcTableFormulaRuntimeReport>,
) -> BTreeMap<String, CalcValue> {
    let values = table_sparse_values_with_formula_reports(
        view,
        node_views,
        published_calc_values,
        body_formula_reports,
    );
    view.snapshot
        .columns
        .iter()
        .filter_map(|column| {
            let formula = column.totals_metadata.as_ref()?;
            if formula.formula_text.trim().is_empty() {
                return None;
            }
            let request = TreeCalcTableColumnFormulaRuntimeRequest {
                target_column_id: column.column_id.clone(),
                formula_stable_id: formula.formula_artifact_id.clone(),
                formula_text_version: parse_formula_text_version(&formula.formula_text_version),
                formula_text: formula.formula_text.clone(),
                values: values.clone(),
                runtime_context: TreeCalcTableFormulaRuntimeContext::default(),
            };
            evaluate_treecalc_table_totals_formula(&view.snapshot, &view.projection, &request)
                .ok()
                .map(|cell| (column.column_id.clone(), cell.value))
        })
        .collect()
}

fn table_sparse_values_with_formula_reports(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
    body_formula_reports: &BTreeMap<String, TreeCalcTableFormulaRuntimeReport>,
) -> Vec<TreeCalcTableSparseValue> {
    let mut values = table_sparse_values_from_bound_cells(view, node_views, published_calc_values);
    values.extend(body_formula_reports.iter().flat_map(|(column_id, report)| {
        report.cell_results.iter().filter_map(|cell| {
            cell.row_id.as_ref().map(|row_id| {
                TreeCalcTableSparseValue::data(
                    row_id.0.clone(),
                    column_id.clone(),
                    cell.value.clone(),
                )
            })
        })
    }));
    values
}

fn table_sparse_values_from_bound_cells(
    view: &oxcalc_core::consumer::OxCalcTreeTableView,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> Vec<TreeCalcTableSparseValue> {
    view.snapshot
        .body_cell_nodes
        .iter()
        .filter_map(|cell| {
            table_cell_calc_value(cell.node_id, node_views, published_calc_values).map(|value| {
                TreeCalcTableSparseValue::data(cell.row_id.0.clone(), cell.column_id.clone(), value)
            })
        })
        .collect()
}

fn table_cell_calc_value(
    node_id: TreeNodeId,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> Option<CalcValue> {
    published_calc_values
        .and_then(|values| values.get(&node_id).cloned())
        .or_else(|| {
            node_views
                .get(&node_id)
                .and_then(|view| view.value_text.as_deref())
                .map(calc_value_from_display_text)
        })
}

fn value_projection_from_calc_value(value: &CalcValue) -> NodeValueProjection {
    calc_value_projection(value, None)
}

fn calc_value_from_display_text(value: &str) -> CalcValue {
    value
        .parse::<f64>()
        .map(CalcValue::number)
        .unwrap_or_else(|_| CalcValue::text(ExcelText::from_interop_assignment(value)))
}

fn parse_formula_text_version(version: &str) -> u64 {
    version
        .strip_prefix('v')
        .unwrap_or(version)
        .parse::<u64>()
        .unwrap_or(1)
}

fn table_cell_projection(
    row_id: Option<String>,
    column_id: String,
    node_id: TreeNodeId,
    node_views: &BTreeMap<TreeNodeId, OxCalcTreeNodeView>,
    published_calc_values: Option<&BTreeMap<TreeNodeId, CalcValue>>,
) -> Option<TableCellProjection> {
    let node_view = node_views.get(&node_id)?;
    let calc_value = published_calc_values.and_then(|values| values.get(&node_id));
    Some(TableCellProjection {
        row_id,
        column_id,
        node_key: node_key_for_tree_node(node_id),
        value: value_projection_for(
            node_view.value_text.clone(),
            node_view.calc_state,
            calc_value,
            None,
        ),
    })
}

fn table_column_projection(column: &TreeCalcTableColumnSnapshot) -> TableColumnProjection {
    TableColumnProjection {
        column_id: column.column_id.clone(),
        name: column.column_name.clone(),
        ordinal: column.ordinal,
        body: match &column.body_metadata {
            TreeCalcTableColumnBodyMetadata::ConstantCells => {
                TableColumnBodyProjection::ConstantCells
            }
            TreeCalcTableColumnBodyMetadata::Formula(formula) => {
                TableColumnBodyProjection::Formula(table_formula_projection(formula))
            }
        },
        totals_formula: column
            .totals_metadata
            .as_ref()
            .map(table_formula_projection),
    }
}

fn table_formula_projection(
    formula: &TreeCalcTableFormulaMetadata,
) -> TableFormulaMetadataProjection {
    TableFormulaMetadataProjection {
        formula_artifact_id: formula.formula_artifact_id.clone(),
        bind_artifact_id: formula.bind_artifact_id.clone(),
        formula_text_version: formula.formula_text_version.clone(),
        formula_text: formula.formula_text.clone(),
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
                .and_then(|node| node.computed_value.scalar_display_text()),
            Some("2")
        );
        assert_eq!(
            state
                .node(&NodeId::new("Accounts.2005.Q1.Net"))
                .and_then(|node| node.computed_value.scalar_display_text()),
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
    fn session_projects_stable_node_keys_and_typed_engine_classifications() {
        let fixture = WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "stable-keys".to_string(),
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

        let before = session.workspace_state().unwrap();
        let original_b_key = before.node(&NodeId::new("Root.B")).unwrap().key.clone();
        assert_eq!(
            before.node_id_for_key(&original_b_key),
            Some(&NodeId::new("Root.B"))
        );
        assert!(
            before.dependencies.descriptors_by_owner_key[&original_b_key]
                .iter()
                .any(|descriptor| descriptor.kind == DependencyKindProjection::StaticDirect)
        );
        assert!(
            before
                .last_run
                .as_ref()
                .unwrap()
                .invalidated_nodes
                .iter()
                .any(|record| record
                    .reasons
                    .contains(&InvalidationReasonProjection::UpstreamPublication))
        );

        let renamed = session
            .rename_node(&NodeId::new("Root.B"), "Renamed")
            .unwrap();
        let moved = session.move_node(&renamed, None, None).unwrap();
        session.recalculate().unwrap();

        let after = session.workspace_state().unwrap();
        assert_eq!(
            after.node(&moved).unwrap().key,
            original_b_key,
            "NodeKey should be OxCalc identity, not display path"
        );
        assert_ne!(moved.as_str(), "Root.B");
        assert!(after.key_order.contains(&original_b_key));
        assert_eq!(after.node_id_for_key(&original_b_key), Some(&moved));
        assert_eq!(
            after.node_by_key(&original_b_key).map(|node| &node.id),
            Some(&moved)
        );
    }

    #[test]
    fn calc_value_projection_preserves_reference_values() {
        let reference = oxfunc_core::value::ReferenceLike::new(
            oxfunc_core::value::ReferenceKind::A1,
            "Tree!A1".to_string(),
        );

        assert_eq!(
            calc_value_projection(&CalcValue::reference(reference), None),
            NodeValueProjection::Reference {
                target: "Tree!A1".to_string()
            }
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
    fn dnatree_document_roundtrip_preserves_table_formula_cells() {
        let fixture = WorkspaceFixture::from_repo_fixture("tables").unwrap();
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let mut session = TreeWorkspaceSession::from_model(&model).unwrap();
        session.recalculate().unwrap();

        let document = session
            .export_dnatree_document(Some(&NodeId::new("SalesTable")))
            .unwrap();
        let json = serde_json::to_string_pretty(&document).unwrap();
        let reparsed: DnaTreeWorkspaceDocument = serde_json::from_str(&json).unwrap();
        let (mut reopened, selected_node) =
            TreeWorkspaceSession::from_dnatree_document(reparsed).unwrap();

        assert_eq!(
            selected_node.as_ref().map(NodeId::as_str),
            Some("SalesTable")
        );

        reopened.recalculate().unwrap();
        let state = reopened.workspace_state().unwrap();
        let table = state
            .tables
            .get(&NodeId::new("SalesTable"))
            .expect("SalesTable projects after reopen");
        let tax_formula = match &table.columns[2].body {
            TableColumnBodyProjection::Formula(formula) => formula,
            TableColumnBodyProjection::ConstantCells => panic!("Tax column should stay formula"),
        };
        assert_eq!(tax_formula.formula_text, "=[@Amount] * 0.1");
        assert_eq!(
            table.columns[1]
                .totals_formula
                .as_ref()
                .map(|formula| formula.formula_text.as_str()),
            Some("=SUM(SalesTable[Amount])")
        );
        let cells = table.cells.as_ref().expect("table cells project");
        assert_eq!(
            table_row_display(&cells.body_rows[0]),
            vec!["West", "10", "1"]
        );
        assert_eq!(
            table_row_display(&cells.body_rows[1]),
            vec!["East", "20", "2"]
        );
        assert_eq!(
            table_row_display(&cells.body_rows[2]),
            vec!["North", "30", "3"]
        );
        assert_eq!(table_row_display(&cells.totals_row), vec!["", "60", ""]);
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
            .and_then(|node| node.computed_value.scalar_display_text())
    }

    fn table_row_display(row: &[Option<TableCellProjection>]) -> Vec<String> {
        row.iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|cell| cell.value.display_text())
                    .unwrap_or_default()
            })
            .collect()
    }
}

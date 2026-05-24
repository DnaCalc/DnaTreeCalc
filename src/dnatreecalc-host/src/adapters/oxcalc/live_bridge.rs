use std::collections::{BTreeMap, BTreeSet};

use oxcalc_core::consumer::{
    OxCalcTreeDocument, OxCalcTreeEnvironment, OxCalcTreeRecalcRequest, OxCalcTreeRuntimeFacade,
};
use oxcalc_core::dependency::{DependencyDescriptorKind, DependencyGraph};
use oxcalc_core::formula::{
    FixtureFormulaAst, FixtureFormulaBinaryOp, RelativeReferenceBase,
    TreeCalcChildrenReferenceCollection, TreeCalcFormulaTextPrebindContext,
    TreeCalcFormulaTextPrebindDiagnostic, TreeCalcOrderedSelectorFamily,
    TreeCalcOrderedSelectorQuery, TreeCalcOrderedSelectorResolution,
    TreeCalcOrderedSelectorResolutionLayer, TreeCalcOrderedSelectorTraversalPolicy,
    TreeCalcQualifiedBaseResolutionLayer, TreeCalcQualifiedChildrenBaseQuery,
    TreeCalcQualifiedChildrenBaseResolution, TreeCalcReferenceCollection,
    TreeCalcReferenceLiteralArrayCollection, TreeCalcReferenceLiteralArrayElement,
    TreeCalcWorkspaceResolutionRegistry, TreeFormula, TreeFormulaBinding, TreeFormulaCatalog,
    TreeFormulaReferenceCarrier, TreeReference, prebind_treecalc_formula_text_with_context,
    resolve_treecalc_workspace_host_path_base, treecalc_formula_text_needs_prebind,
    treecalc_formula_text_ordered_selector_queries,
    treecalc_formula_text_qualified_children_base_queries,
};
use oxcalc_core::structural::{
    BindArtifactId, FormulaArtifactId, StructuralNode, StructuralNodeKind, StructuralSnapshot,
    StructuralSnapshotId, TreeNodeId,
};
use oxcalc_core::structured_table::{
    TableCallerRegion, TableRef, TableRegionKind, TreeCalcDynamicTableRebindReport,
    TreeCalcDynamicTableRebindRequest, TreeCalcTableColumnBodyMetadata,
    TreeCalcTableColumnSnapshot, TreeCalcTableFormulaMetadata, TreeCalcTableNodeProjection,
    TreeCalcTableNodeSnapshot, TreeCalcTableProjectionError, TreeCalcTableRowId,
    TreeCalcTableVirtualAnchor, classify_treecalc_dynamic_table_rebind,
    prebind_treecalc_table_structured_references, project_treecalc_table_node_snapshot,
};

use super::bridge::{OxCalcTreeBridge, OxCalcTreeBridgeError};
use super::types::{
    NodeCalcStateProjection, PreparedBinaryOp, PreparedFormula, PreparedFormulaCatalog,
    PreparedFormulaOperand, PreparedFormulaReferenceCarrier, PreparedReferenceLiteralArrayElement,
    PreparedRelativePathBase, TreeCalcCrossWorkspaceReferenceRequest,
    TreeCalcCrossWorkspaceReferenceResolution, TreeRecalcRequest, TreeRecalcResult,
};
use crate::model::{
    NodeContentKind, TableColumnBodyKind, TableColumnFixture, TableFormulaFixture,
    TableNodeFixture, WorkspaceModel, WorkspaceNode,
};

#[derive(Debug, Default)]
pub struct LiveOxCalcTreeBridge {
    facade: OxCalcTreeRuntimeFacade,
}

impl LiveOxCalcTreeBridge {
    #[must_use]
    pub fn new(environment: OxCalcTreeEnvironment) -> Self {
        Self {
            facade: OxCalcTreeRuntimeFacade::new(environment),
        }
    }
}

impl OxCalcTreeBridge for LiveOxCalcTreeBridge {
    fn execute_recalc(
        &self,
        request: TreeRecalcRequest,
    ) -> Result<TreeRecalcResult, OxCalcTreeBridgeError> {
        let submission = PreparedSubmission::try_from_request(&request)?;
        let result = self
            .facade
            .execute(
                OxCalcTreeDocument {
                    structural_snapshot: submission.structural_snapshot.clone(),
                    formula_catalog: submission.formula_catalog.clone(),
                    seeded_published_values: BTreeMap::new(),
                },
                OxCalcTreeRecalcRequest {
                    candidate_result_id: request.candidate_result_id,
                    publication_id: request.publication_id,
                    compatibility_basis: request.compatibility_basis,
                    artifact_token_basis: request.artifact_token_basis,
                },
            )
            .map_err(|error| OxCalcTreeBridgeError::Upstream(error.to_string()))?;

        let published_values = result
            .published_values
            .iter()
            .filter_map(|(node_id, value)| {
                submission
                    .paths_by_node_id
                    .get(node_id)
                    .map(|path| (path.clone(), value.clone()))
            })
            .collect();

        let node_states = result
            .node_states
            .iter()
            .filter_map(|(node_id, state)| {
                submission
                    .paths_by_node_id
                    .get(node_id)
                    .map(|path| (path.clone(), NodeCalcStateProjection::from(*state)))
            })
            .collect();

        let evaluation_order = result
            .evaluation_order
            .iter()
            .filter_map(|node_id| submission.paths_by_node_id.get(node_id).cloned())
            .collect();

        let dependency_edges_by_owner =
            project_dependency_edges(&result.dependency_graph, &submission);
        let table_context_identities = project_table_context_identities(&submission);

        Ok(TreeRecalcResult {
            run_state: result.run_state,
            dependency_graph: result.dependency_graph,
            invalidation_closure: result.invalidation_closure,
            evaluation_order,
            dependency_edges_by_owner,
            table_context_identities,
            published_values,
            node_states,
            diagnostics: result.diagnostics,
        })
    }

    fn classify_dynamic_table_rebind(
        &self,
        request: TreeCalcDynamicTableRebindRequest,
    ) -> Result<TreeCalcDynamicTableRebindReport, OxCalcTreeBridgeError> {
        Ok(classify_treecalc_dynamic_table_rebind(&request))
    }

    fn resolve_cross_workspace_reference(
        &self,
        request: TreeCalcCrossWorkspaceReferenceRequest,
    ) -> Result<TreeCalcCrossWorkspaceReferenceResolution, OxCalcTreeBridgeError> {
        resolve_cross_workspace_reference_request(request)
    }
}

fn project_table_context_identities(submission: &PreparedSubmission) -> BTreeMap<String, String> {
    submission
        .table_projections
        .iter()
        .filter_map(|projection| {
            submission
                .paths_by_node_id
                .get(&projection.table_node_id)
                .map(|path| (path.clone(), projection.table_context_identity.clone()))
        })
        .collect()
}

fn project_dependency_edges(
    dependency_graph: &DependencyGraph,
    submission: &PreparedSubmission,
) -> BTreeMap<String, Vec<String>> {
    dependency_graph
        .descriptors_by_owner
        .iter()
        .filter_map(|(owner, descriptors)| {
            let owner_path = submission.paths_by_node_id.get(owner)?;
            let collection_handles = descriptors
                .iter()
                .filter_map(|descriptor| {
                    descriptor
                        .tree_reference_collection
                        .as_ref()
                        .map(|collection| collection.host_ref_handle.clone())
                })
                .collect::<BTreeSet<_>>();
            let mut target_paths = Vec::new();
            for descriptor in descriptors {
                if let Some(collection) = descriptor.tree_reference_collection.as_ref() {
                    target_paths.extend(
                        collection.member_node_ids.iter().filter_map(|node_id| {
                            submission.paths_by_node_id.get(node_id).cloned()
                        }),
                    );
                    continue;
                }
                if descriptor.kind == DependencyDescriptorKind::TreeReferenceCollectionMemberValue
                    && descriptor
                        .source_reference_handle
                        .as_ref()
                        .is_some_and(|handle| collection_handles.contains(handle))
                {
                    continue;
                }
                if let Some(target_node_id) = descriptor.target_node_id
                    && let Some(path) = submission.paths_by_node_id.get(&target_node_id)
                {
                    target_paths.push(path.clone());
                }
            }
            (!target_paths.is_empty()).then(|| (owner_path.clone(), target_paths))
        })
        .collect()
}

struct PreparedSubmission {
    structural_snapshot: StructuralSnapshot,
    formula_catalog: TreeFormulaCatalog,
    paths_by_node_id: BTreeMap<TreeNodeId, String>,
    table_projections: Vec<TreeCalcTableNodeProjection>,
}

impl PreparedSubmission {
    fn try_from_request(request: &TreeRecalcRequest) -> Result<Self, OxCalcTreeBridgeError> {
        let node_ids_by_path = assign_node_ids(&request.workspace);
        let paths_by_node_id = node_ids_by_path
            .iter()
            .map(|(path, node_id)| (*node_id, path.clone()))
            .collect::<BTreeMap<_, _>>();
        let root_node_id = root_node_id(&request.workspace, &node_ids_by_path)?;
        let table_projections = build_table_projections(&request.workspace, &node_ids_by_path)?;
        let structural_snapshot = build_structural_snapshot(
            &request.workspace,
            &request.formula_catalog,
            &node_ids_by_path,
            root_node_id,
            &table_projections,
        )?;
        let formula_catalog = build_formula_catalog(
            &request.workspace,
            &request.formula_catalog,
            &node_ids_by_path,
            &structural_snapshot,
            &table_projections,
        )?;

        Ok(Self {
            structural_snapshot,
            formula_catalog,
            paths_by_node_id,
            table_projections,
        })
    }
}

struct ReferenceResolutionWorkspaceProjection {
    workspace_handle: String,
    availability_version: String,
    structural_snapshot: StructuralSnapshot,
    paths_by_node_id: BTreeMap<TreeNodeId, String>,
}

impl ReferenceResolutionWorkspaceProjection {
    fn from_workspace(
        workspace_handle: String,
        workspace: &WorkspaceModel,
        availability_version: String,
    ) -> Result<Self, OxCalcTreeBridgeError> {
        let node_ids_by_path = assign_node_ids(workspace);
        let paths_by_node_id = node_ids_by_path
            .iter()
            .map(|(path, node_id)| (*node_id, path.clone()))
            .collect::<BTreeMap<_, _>>();
        let root_node_id = reference_resolution_root_node_id(workspace, &node_ids_by_path)?;
        let structural_snapshot = build_reference_resolution_structural_snapshot(
            workspace,
            &node_ids_by_path,
            root_node_id,
        )?;

        Ok(Self {
            workspace_handle,
            availability_version,
            structural_snapshot,
            paths_by_node_id,
        })
    }

    fn path_for(&self, node_id: TreeNodeId) -> Result<String, OxCalcTreeBridgeError> {
        if node_id == TreeNodeId(0) {
            return Ok("/".to_string());
        }
        self.paths_by_node_id.get(&node_id).cloned().ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!(
                "resolved node {node_id} is not present in workspace {}",
                self.workspace_handle
            ))
        })
    }
}

fn resolve_cross_workspace_reference_request(
    request: TreeCalcCrossWorkspaceReferenceRequest,
) -> Result<TreeCalcCrossWorkspaceReferenceResolution, OxCalcTreeBridgeError> {
    let mut projections = Vec::with_capacity(1 + request.external_workspaces.len());
    projections.push(ReferenceResolutionWorkspaceProjection::from_workspace(
        request.current_workspace_handle,
        &request.current_workspace,
        request.current_availability_version,
    )?);
    for external in &request.external_workspaces {
        projections.push(ReferenceResolutionWorkspaceProjection::from_workspace(
            external.workspace_handle.clone(),
            &external.workspace,
            external.availability_version.clone(),
        )?);
    }

    let current = projections
        .first()
        .expect("current workspace projection was inserted");
    let mut registry = TreeCalcWorkspaceResolutionRegistry::with_current_workspace(
        current.workspace_handle.clone(),
        &current.structural_snapshot,
        current.availability_version.clone(),
    );
    for projection in projections.iter().skip(1) {
        registry.add_workspace(
            projection.workspace_handle.clone(),
            &projection.structural_snapshot,
            projection.availability_version.clone(),
        );
    }
    for (selector, workspace_handle) in &request.aliases {
        registry.add_alias(selector.clone(), workspace_handle.clone());
    }

    let resolution = resolve_treecalc_workspace_host_path_base(&registry, &request.base_token_text)
        .map_err(|error| OxCalcTreeBridgeError::FormulaBindingUnavailable(error.to_string()))?;
    let target_projection = projections
        .iter()
        .find(|projection| projection.workspace_handle == resolution.workspace_handle)
        .ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!(
                "resolution returned unregistered workspace handle {}",
                resolution.workspace_handle
            ))
        })?;
    let target_path = target_projection.path_for(resolution.base_node_id)?;
    let carrier_id = format!("dnatreecalc-cross-workspace:v1:{}", request.source_token);
    let prepared_carrier = PreparedFormulaReferenceCarrier::CrossWorkspaceResolved {
        source_token: request.source_token.clone(),
        workspace_handle: resolution.workspace_handle.clone(),
        target_node_id: resolution.base_node_id.0,
        target_node_handle: resolution.base_node_handle.clone(),
        availability_version: resolution.availability_packet.availability_version.clone(),
        carrier_id,
        detail: resolution.resolution_identity.clone(),
    };

    Ok(TreeCalcCrossWorkspaceReferenceResolution {
        source_token: request.source_token,
        workspace_handle: resolution.workspace_handle,
        target_path,
        target_node_id: resolution.base_node_id.0,
        target_node_handle: resolution.base_node_handle,
        availability_version: resolution.availability_packet.availability_version,
        workspace_resolution_layer: format!("{:?}", resolution.workspace_resolution_layer),
        local_resolution_layer: format!("{:?}", resolution.local_resolution_layer),
        resolution_identity: resolution.resolution_identity,
        prepared_carrier,
    })
}

fn assign_node_ids(workspace: &WorkspaceModel) -> BTreeMap<String, TreeNodeId> {
    workspace
        .node_order
        .iter()
        .enumerate()
        .map(|(index, path)| {
            (
                path.clone(),
                TreeNodeId(u64::try_from(index + 1).expect("usize node index fits into u64")),
            )
        })
        .collect()
}

fn root_node_id(
    workspace: &WorkspaceModel,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<TreeNodeId, OxCalcTreeBridgeError> {
    if workspace.root_paths.len() != 1 {
        for root in &workspace.root_paths {
            node_id_for(root, node_ids_by_path)?;
        }
        return Ok(TreeNodeId(0));
    }

    node_ids_by_path
        .get(&workspace.root_paths[0])
        .copied()
        .ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!(
                "root {} has no assigned OxCalc node id",
                workspace.root_paths[0]
            ))
        })
}

fn reference_resolution_root_node_id(
    workspace: &WorkspaceModel,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<TreeNodeId, OxCalcTreeBridgeError> {
    if workspace.root_paths.len() == 1 {
        return node_id_for(&workspace.root_paths[0], node_ids_by_path);
    }

    Ok(TreeNodeId(0))
}

fn build_structural_snapshot(
    workspace: &WorkspaceModel,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    root_node_id: TreeNodeId,
    table_projections: &[TreeCalcTableNodeProjection],
) -> Result<StructuralSnapshot, OxCalcTreeBridgeError> {
    let mut nodes = Vec::new();
    let synthetic_root = root_node_id == TreeNodeId(0);

    if synthetic_root {
        let child_ids = workspace
            .root_paths
            .iter()
            .map(|root| node_id_for(root, node_ids_by_path))
            .collect::<Result<Vec<_>, _>>()?;
        nodes.push(StructuralNode {
            node_id: root_node_id,
            kind: StructuralNodeKind::Root,
            symbol: workspace.workspace_id.clone(),
            parent_id: None,
            child_ids,
            formula_artifact_id: None,
            bind_artifact_id: None,
            constant_value: None,
        });
    }

    for path in &workspace.node_order {
        let node = workspace.node(path).ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!("node {path} missing from workspace"))
        })?;
        nodes.push(build_structural_node(
            node,
            formula_catalog,
            node_ids_by_path,
            table_projections,
            synthetic_root.then_some(root_node_id),
        )?);
    }

    StructuralSnapshot::create(StructuralSnapshotId(1), root_node_id, nodes)
        .map_err(|error| OxCalcTreeBridgeError::InvalidWorkspace(error.to_string()))
}

fn build_reference_resolution_structural_snapshot(
    workspace: &WorkspaceModel,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    root_node_id: TreeNodeId,
) -> Result<StructuralSnapshot, OxCalcTreeBridgeError> {
    let mut nodes = Vec::new();

    let synthetic_root = root_node_id == TreeNodeId(0);
    if synthetic_root {
        let child_ids = workspace
            .root_paths
            .iter()
            .map(|root| node_id_for(root, node_ids_by_path))
            .collect::<Result<Vec<_>, _>>()?;
        nodes.push(StructuralNode {
            node_id: root_node_id,
            kind: StructuralNodeKind::Root,
            symbol: workspace.workspace_id.clone(),
            parent_id: None,
            child_ids,
            formula_artifact_id: None,
            bind_artifact_id: None,
            constant_value: None,
        });
    }

    for path in &workspace.node_order {
        let node = workspace.node(path).ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!("node {path} missing from workspace"))
        })?;
        let node_id = node_id_for(&node.path, node_ids_by_path)?;
        let parent_id = node
            .parent_path
            .as_deref()
            .map(|parent| node_id_for(parent, node_ids_by_path))
            .transpose()?
            .or_else(|| synthetic_root.then_some(root_node_id));
        let child_ids = node
            .child_paths
            .iter()
            .map(|child| node_id_for(child, node_ids_by_path))
            .collect::<Result<Vec<_>, _>>()?;
        let kind = if parent_id.is_none() {
            StructuralNodeKind::Root
        } else {
            match node.content.kind() {
                NodeContentKind::Constant => StructuralNodeKind::Constant,
                NodeContentKind::Empty | NodeContentKind::Formula => StructuralNodeKind::Container,
            }
        };
        let constant_value = if node.content.kind() == NodeContentKind::Constant {
            Some(node.content.text().to_string())
        } else {
            None
        };
        nodes.push(StructuralNode {
            node_id,
            kind,
            symbol: oxcalc_structural_symbol(&node.name),
            parent_id,
            child_ids,
            formula_artifact_id: None,
            bind_artifact_id: None,
            constant_value,
        });
    }

    StructuralSnapshot::create(StructuralSnapshotId(1), root_node_id, nodes)
        .map_err(|error| OxCalcTreeBridgeError::InvalidWorkspace(error.to_string()))
}

fn build_structural_node(
    node: &WorkspaceNode,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    table_projections: &[TreeCalcTableNodeProjection],
    synthetic_root_id: Option<TreeNodeId>,
) -> Result<StructuralNode, OxCalcTreeBridgeError> {
    let node_id = node_id_for(&node.path, node_ids_by_path)?;
    let parent_id = node
        .parent_path
        .as_deref()
        .map(|parent| node_id_for(parent, node_ids_by_path))
        .transpose()?
        .or(synthetic_root_id);
    let child_ids = node
        .child_paths
        .iter()
        .map(|child| node_id_for(child, node_ids_by_path))
        .collect::<Result<Vec<_>, _>>()?;
    let has_oxcalc_formula = has_oxcalc_formula_binding(node, formula_catalog, table_projections);

    let kind = if parent_id.is_none() {
        StructuralNodeKind::Root
    } else if has_oxcalc_formula {
        StructuralNodeKind::Calculation
    } else {
        match node.content.kind() {
            NodeContentKind::Empty => StructuralNodeKind::Container,
            NodeContentKind::Constant => StructuralNodeKind::Constant,
            NodeContentKind::Formula => {
                return Err(OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
                    "node {} has formula text but no prepared OxCalc formula binding",
                    node.path
                )));
            }
        }
    };

    let (formula_artifact_id, bind_artifact_id) = if has_oxcalc_formula {
        (
            Some(FormulaArtifactId(formula_artifact_id(&node.path))),
            Some(BindArtifactId(bind_artifact_id(&node.path))),
        )
    } else {
        (None, None)
    };

    let constant_value = if node.content.kind() == NodeContentKind::Constant {
        Some(node.content.text().to_string())
    } else {
        None
    };

    Ok(StructuralNode {
        node_id,
        kind,
        symbol: oxcalc_structural_symbol(&node.name),
        parent_id,
        child_ids,
        formula_artifact_id,
        bind_artifact_id,
        constant_value,
    })
}

fn has_oxcalc_formula_binding(
    node: &WorkspaceNode,
    formula_catalog: &PreparedFormulaCatalog,
    table_projections: &[TreeCalcTableNodeProjection],
) -> bool {
    formula_catalog.contains_path(&node.path)
        || (node.content.kind() == NodeContentKind::Formula
            && treecalc_formula_text_needs_prebind(node.content.text()))
        || (node.content.kind() == NodeContentKind::Formula
            && !prebind_treecalc_table_structured_references(
                node.content.text(),
                table_projections,
                None,
                None,
            )
            .is_empty())
}

fn build_formula_catalog(
    workspace: &WorkspaceModel,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    structural_snapshot: &StructuralSnapshot,
    table_projections: &[TreeCalcTableNodeProjection],
) -> Result<TreeFormulaCatalog, OxCalcTreeBridgeError> {
    let mut bindings = Vec::new();

    for path in &workspace.node_order {
        let node = workspace.node(path).ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!("node {path} missing from workspace"))
        })?;
        let owner_node_id = node_id_for(path, node_ids_by_path)?;
        let expression = if let Some(formula) = formula_catalog.get(path) {
            prepared_formula_to_tree_formula(formula, owner_node_id, node_ids_by_path)?
        } else if node.content.kind() == NodeContentKind::Formula
            && let Some(expression) = prebind_table_formula_text(
                path,
                node.content.text(),
                table_projections,
                node_ids_by_path,
            )?
        {
            expression
        } else if node.content.kind() == NodeContentKind::Formula
            && treecalc_formula_text_needs_prebind(node.content.text())
        {
            let resolved_bases = resolve_qualified_children_base_queries(
                workspace,
                path,
                node.content.text(),
                node_ids_by_path,
                structural_snapshot,
            )?;
            let resolved_ordered_selectors = resolve_ordered_selector_queries(
                workspace,
                path,
                node.content.text(),
                node_ids_by_path,
                structural_snapshot,
            )?;
            prebind_treecalc_formula_text_with_context(
                owner_node_id,
                node.content.text(),
                &TreeCalcFormulaTextPrebindContext {
                    qualified_children_bases: resolved_bases,
                    ordered_selector_resolutions: resolved_ordered_selectors,
                },
            )
            .map_err(|error| {
                OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
                    "node {path} raw TreeCalc formula text cannot be prebound by current OxCalc surface: {}",
                    format_prebind_diagnostics(&error.diagnostics)
                ))
            })?
        } else {
            continue;
        };

        bindings.push(TreeFormulaBinding {
            owner_node_id,
            formula_artifact_id: FormulaArtifactId(formula_artifact_id(path)),
            bind_artifact_id: Some(BindArtifactId(bind_artifact_id(path))),
            expression,
        });
    }

    Ok(TreeFormulaCatalog::new(bindings))
}

fn prebind_table_formula_text(
    path: &str,
    source_text: &str,
    table_projections: &[TreeCalcTableNodeProjection],
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<Option<TreeFormula>, OxCalcTreeBridgeError> {
    let (enclosing_table_ref, caller_table_region) =
        table_caller_context(path, table_projections, node_ids_by_path)?;
    let table_prebinds = prebind_treecalc_table_structured_references(
        source_text,
        table_projections,
        enclosing_table_ref,
        caller_table_region,
    );
    if table_prebinds.is_empty() {
        return Ok(None);
    }
    let diagnostics = table_prebinds
        .iter()
        .flat_map(|prebind| prebind.diagnostics.iter())
        .collect::<Vec<_>>();
    if !diagnostics.is_empty() {
        return Err(OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
            "node {path} raw TreeCalc table structured reference text cannot be prebound by current OxCalc surface: {}",
            diagnostics
                .iter()
                .map(|diagnostic| format!(
                    "{:?}:{}:{}",
                    diagnostic.source_span_utf8, diagnostic.diagnostic_code, diagnostic.message
                ))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    Ok(Some(TreeFormula::opaque_oxfml(
        source_text.to_string(),
        Vec::new(),
    )))
}

fn build_table_projections(
    workspace: &WorkspaceModel,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<Vec<TreeCalcTableNodeProjection>, OxCalcTreeBridgeError> {
    let mut projections = Vec::new();
    let mut next_start_row = 3u32;
    for path in &workspace.node_order {
        let Some(table) = workspace.table_node(path) else {
            continue;
        };
        let height = table_virtual_height(table)?;
        let projection =
            table_node_projection(workspace, path, table, node_ids_by_path, next_start_row)?;
        next_start_row = next_start_row
            .checked_add(height)
            .and_then(|row| row.checked_add(10))
            .ok_or_else(|| {
                OxCalcTreeBridgeError::InvalidWorkspace(
                    "table virtual-anchor row allocation overflowed".to_string(),
                )
            })?;
        projections.push(projection);
    }
    Ok(projections)
}

fn table_node_projection(
    workspace: &WorkspaceModel,
    path: &str,
    table: &TableNodeFixture,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    start_row: u32,
) -> Result<TreeCalcTableNodeProjection, OxCalcTreeBridgeError> {
    let snapshot = TreeCalcTableNodeSnapshot {
        table_node_id: node_id_for(path, node_ids_by_path)?,
        table_id: table.table_id.clone(),
        table_name: treecalc_table_name(path),
        display_path: table
            .display_path
            .clone()
            .unwrap_or_else(|| path.to_string()),
        canonical_path: table
            .canonical_path
            .clone()
            .unwrap_or_else(|| path.to_string()),
        virtual_anchor: treecalc_table_virtual_anchor(workspace, start_row),
        rows: table
            .rows
            .iter()
            .map(|row| TreeCalcTableRowId(row.row_id.clone()))
            .collect(),
        columns: table
            .columns
            .iter()
            .map(table_column_snapshot)
            .collect::<Result<Vec<_>, _>>()?,
        header_row_present: table.header.present,
        totals_row_present: table.totals.present,
        table_namespace_version: table.table_namespace_version.clone(),
        row_membership_version: table.row_membership_version.clone(),
        row_order_version: table.row_order_version.clone(),
        column_identity_version: table.column_identity_version.clone(),
    };
    project_treecalc_table_node_snapshot(&snapshot)
        .map_err(|error| table_projection_error(path, error))
}

fn table_column_snapshot(
    column: &TableColumnFixture,
) -> Result<TreeCalcTableColumnSnapshot, OxCalcTreeBridgeError> {
    Ok(TreeCalcTableColumnSnapshot {
        column_id: column.column_id.clone(),
        column_name: column.name.clone(),
        ordinal: column.ordinal,
        body_metadata: match column.body.kind {
            TableColumnBodyKind::ConstantCells => TreeCalcTableColumnBodyMetadata::ConstantCells,
            TableColumnBodyKind::Formula => TreeCalcTableColumnBodyMetadata::Formula(
                table_formula_metadata(column.body.formula.as_ref().ok_or_else(|| {
                    OxCalcTreeBridgeError::InvalidWorkspace(format!(
                        "table column {} declares formula body without formula metadata",
                        column.column_id
                    ))
                })?),
            ),
        },
        totals_metadata: column.totals_formula.as_ref().map(table_formula_metadata),
    })
}

fn table_formula_metadata(formula: &TableFormulaFixture) -> TreeCalcTableFormulaMetadata {
    TreeCalcTableFormulaMetadata {
        formula_artifact_id: formula.formula_stable_id.clone(),
        bind_artifact_id: formula.bind_artifact_id.clone(),
        formula_text_version: formula.formula_text_version.clone(),
    }
}

fn treecalc_table_virtual_anchor(
    workspace: &WorkspaceModel,
    start_row: u32,
) -> TreeCalcTableVirtualAnchor {
    TreeCalcTableVirtualAnchor {
        workbook_scope_ref: format!("treecalc-workbook:{}", workspace.workspace_id),
        sheet_scope_ref: "treecalc-virtual-sheet:tables".to_string(),
        start_row,
        start_col: 2,
    }
}

fn table_virtual_height(table: &TableNodeFixture) -> Result<u32, OxCalcTreeBridgeError> {
    let rows = u32::try_from(table.rows.len()).map_err(|_| {
        OxCalcTreeBridgeError::InvalidWorkspace("table row count exceeds u32".to_string())
    })?;
    rows.checked_add(u32::from(table.header.present))
        .and_then(|height| height.checked_add(u32::from(table.totals.present)))
        .ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(
                "table virtual-height calculation overflowed".to_string(),
            )
        })
}

fn treecalc_table_name(path: &str) -> String {
    path.rsplit('.').next().unwrap_or(path).to_string()
}

fn table_projection_error(
    path: &str,
    error: TreeCalcTableProjectionError,
) -> OxCalcTreeBridgeError {
    OxCalcTreeBridgeError::InvalidWorkspace(format!(
        "table node {path} cannot project to OxCalc table catalog: {error:?}"
    ))
}

fn table_caller_context(
    caller_path: &str,
    table_projections: &[TreeCalcTableNodeProjection],
    paths_by_node_id: &BTreeMap<String, TreeNodeId>,
) -> Result<(Option<TableRef>, Option<TableCallerRegion>), OxCalcTreeBridgeError> {
    let Some((table_path, projection)) =
        enclosing_table_projection(caller_path, table_projections, paths_by_node_id)
    else {
        return Ok((None, None));
    };
    let suffix = caller_path.strip_prefix(table_path.as_str()).unwrap_or("");
    let region_kind = if suffix.starts_with(".Headers") {
        TableRegionKind::Headers
    } else if suffix.starts_with(".Totals") {
        TableRegionKind::Totals
    } else {
        TableRegionKind::Data
    };
    Ok((
        Some(TableRef {
            table_id: projection.table_id.clone(),
        }),
        Some(TableCallerRegion {
            table_id: projection.table_id.clone(),
            region_kind,
            data_row_offset: None,
        }),
    ))
}

fn enclosing_table_projection<'a>(
    caller_path: &str,
    table_projections: &'a [TreeCalcTableNodeProjection],
    paths_by_node_id: &BTreeMap<String, TreeNodeId>,
) -> Option<(String, &'a TreeCalcTableNodeProjection)> {
    table_projections
        .iter()
        .filter_map(|projection| {
            let table_path = paths_by_node_id.iter().find_map(|(path, node_id)| {
                (*node_id == projection.table_node_id).then_some(path)
            })?;
            (caller_path == table_path || caller_path.starts_with(&format!("{table_path}.")))
                .then(|| (table_path.clone(), projection))
        })
        .max_by_key(|(path, _)| path.len())
}

fn resolve_qualified_children_base_queries(
    workspace: &WorkspaceModel,
    caller_path: &str,
    source_text: &str,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    structural_snapshot: &StructuralSnapshot,
) -> Result<Vec<TreeCalcQualifiedChildrenBaseResolution>, OxCalcTreeBridgeError> {
    treecalc_formula_text_qualified_children_base_queries(
        *node_ids_by_path.get(caller_path).ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!(
                "node {caller_path} missing from node id map"
            ))
        })?,
        source_text,
    )
    .into_iter()
    .map(|query| {
        if let Ok(resolution) = query.to_resolution_with_structural_path_base(structural_snapshot) {
            return Ok(resolution);
        }

        let base_path = resolve_qualified_children_base_path(workspace, caller_path, &query)?;
        let base_node_id = node_id_for(&base_path, node_ids_by_path)?;
        Ok(query.to_resolution_with_layer(
            base_node_id,
            TreeCalcQualifiedBaseResolutionLayer::CallerSuppliedResolvedBase,
            format!(
                "dnatreecalc-qualified-children-base:v1:caller={caller_path};base={base_path};token={}",
                query.base_token_text
            ),
        ))
    })
    .collect()
}

fn resolve_ordered_selector_queries(
    workspace: &WorkspaceModel,
    caller_path: &str,
    source_text: &str,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    structural_snapshot: &StructuralSnapshot,
) -> Result<Vec<oxcalc_core::formula::TreeCalcOrderedSelectorResolution>, OxCalcTreeBridgeError> {
    let owner_node_id = *node_ids_by_path.get(caller_path).ok_or_else(|| {
        OxCalcTreeBridgeError::InvalidWorkspace(format!(
            "node {caller_path} missing from node id map"
        ))
    })?;

    treecalc_formula_text_ordered_selector_queries(owner_node_id, source_text)
        .into_iter()
        .map(|query| {
            let base_path = resolve_ordered_selector_base_path(workspace, caller_path, &query)?;
            let member_paths =
                resolve_ordered_selector_member_paths(workspace, &base_path, &query)?;
            let base_node_id = node_id_for(&base_path, node_ids_by_path)?;
            let member_node_ids = member_paths
                .iter()
                .map(|path| node_id_for(path, node_ids_by_path))
                .collect::<Result<Vec<_>, _>>()?;

            if let Ok(resolution) = resolve_ordered_selector_with_structural_traversal(
                &query,
                structural_snapshot,
                owner_node_id,
            ) && resolution.base_node_id == base_node_id
                && resolution.member_node_ids == member_node_ids
            {
                return Ok(resolution);
            }

            Ok(query.to_resolution_with_layer(
                base_node_id,
                member_node_ids,
            TreeCalcOrderedSelectorResolutionLayer::CallerSuppliedResolvedCollection,
            format!(
                "dnatreecalc-ordered-selector:v1:caller={caller_path};base={base_path};token={}",
                query.source_token_text
            ),
        ))
        })
        .collect()
}

fn resolve_ordered_selector_with_structural_traversal(
    query: &TreeCalcOrderedSelectorQuery,
    structural_snapshot: &StructuralSnapshot,
    owner_node_id: TreeNodeId,
) -> Result<TreeCalcOrderedSelectorResolution, OxCalcTreeBridgeError> {
    let policy = TreeCalcOrderedSelectorTraversalPolicy::default();
    let resolved = if query.base_token_text.is_some() {
        query
            .to_resolution_with_structural_path_base_and_traversal(structural_snapshot, policy)
            .map_err(|error| {
                OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
                    "OxCalc structural ordered selector resolution failed for '{}': {error}",
                    query.source_token_text
                ))
            })?
    } else {
        query
            .to_resolution_with_structural_traversal(structural_snapshot, owner_node_id, policy)
            .map_err(|error| {
                OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
                    "OxCalc structural ordered selector traversal failed for '{}': {error}",
                    query.source_token_text
                ))
            })?
    };

    Ok(resolved.resolution)
}

fn resolve_ordered_selector_base_path(
    workspace: &WorkspaceModel,
    caller_path: &str,
    query: &TreeCalcOrderedSelectorQuery,
) -> Result<String, OxCalcTreeBridgeError> {
    let Some(base_token_text) = query.base_token_text.as_deref() else {
        return Ok(caller_path.to_string());
    };
    let Some(base_path) = resolve_relative_tree_path(workspace, caller_path, base_token_text)
    else {
        return Err(OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
            "node {caller_path} cannot resolve ordered selector base token '{}' from '{}'",
            base_token_text, query.source_token_text
        )));
    };
    Ok(base_path)
}

fn resolve_ordered_selector_member_paths(
    workspace: &WorkspaceModel,
    base_path: &str,
    query: &TreeCalcOrderedSelectorQuery,
) -> Result<Vec<String>, OxCalcTreeBridgeError> {
    match query.family {
        TreeCalcOrderedSelectorFamily::PrecedingV1 => {
            ordered_siblings_relative_to(workspace, base_path, SiblingSelection::Preceding)
        }
        TreeCalcOrderedSelectorFamily::FollowingV1 => {
            ordered_siblings_relative_to(workspace, base_path, SiblingSelection::Following)
        }
        TreeCalcOrderedSelectorFamily::AncestorsV1 => Ok(ancestor_paths(workspace, base_path)),
        TreeCalcOrderedSelectorFamily::RecursiveDescendantsV1 => Ok(recursive_descendant_paths(
            workspace,
            base_path,
            query.tail_token_text.as_deref(),
        )),
        TreeCalcOrderedSelectorFamily::SiblingSetV1 => {
            Err(OxCalcTreeBridgeError::FormulaBindingUnavailable(
                "TreeCalc has no authored raw @SIBLINGS selector in the current corpus".to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SiblingSelection {
    Preceding,
    Following,
}

fn ordered_siblings_relative_to(
    workspace: &WorkspaceModel,
    base_path: &str,
    selection: SiblingSelection,
) -> Result<Vec<String>, OxCalcTreeBridgeError> {
    let parent_path = parent_path(base_path).ok_or_else(|| {
        OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
            "node {base_path} has no parent for ordered sibling selector"
        ))
    })?;
    let parent = workspace.node(&parent_path).ok_or_else(|| {
        OxCalcTreeBridgeError::InvalidWorkspace(format!(
            "parent {parent_path} missing while resolving ordered sibling selector"
        ))
    })?;
    let index = parent
        .child_paths
        .iter()
        .position(|path| path == base_path)
        .ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!(
                "node {base_path} is not listed under parent {parent_path}"
            ))
        })?;
    let range = match selection {
        SiblingSelection::Preceding => &parent.child_paths[..index],
        SiblingSelection::Following => &parent.child_paths[index + 1..],
    };
    Ok(range
        .iter()
        .filter(|path| workspace.node(path).is_some_and(|node| !node.is_meta))
        .cloned()
        .collect())
}

fn ancestor_paths(workspace: &WorkspaceModel, base_path: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut current = parent_path(base_path);
    while let Some(path) = current {
        if workspace.node(&path).is_some_and(|node| !node.is_meta) {
            paths.push(path.clone());
        }
        current = parent_path(&path);
    }
    paths
}

fn recursive_descendant_paths(
    workspace: &WorkspaceModel,
    base_path: &str,
    tail_token_text: Option<&str>,
) -> Vec<String> {
    let prefix = format!("{base_path}.");
    let tail = tail_token_text.map(|tail| tail.trim_start_matches('.'));
    workspace
        .node_order
        .iter()
        .filter(|path| path.starts_with(&prefix))
        .filter(|path| workspace.node(path).is_some_and(|node| !node.is_meta))
        .filter(|path| tail.is_none_or(|tail| path.rsplit('.').next() == Some(tail)))
        .cloned()
        .collect()
}

fn resolve_qualified_children_base_path(
    workspace: &WorkspaceModel,
    caller_path: &str,
    query: &TreeCalcQualifiedChildrenBaseQuery,
) -> Result<String, OxCalcTreeBridgeError> {
    let Some(base_path) =
        resolve_relative_tree_path(workspace, caller_path, &query.base_token_text)
    else {
        return Err(OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
            "node {caller_path} cannot resolve qualified children base token '{}' from '{}'",
            query.base_token_text, query.source_token_text
        )));
    };
    Ok(base_path)
}

fn resolve_relative_tree_path(
    workspace: &WorkspaceModel,
    caller_path: &str,
    token_text: &str,
) -> Option<String> {
    let mut scope = parent_path(caller_path);
    while let Some(scope_path) = scope {
        if let Some(path) =
            find_workspace_path_case_insensitive(workspace, &format!("{scope_path}.{token_text}"))
        {
            return Some(path);
        }
        scope = parent_path(&scope_path);
    }

    find_workspace_path_case_insensitive(workspace, token_text)
}

fn find_workspace_path_case_insensitive(
    workspace: &WorkspaceModel,
    candidate: &str,
) -> Option<String> {
    workspace
        .nodes
        .keys()
        .find(|path| path.eq_ignore_ascii_case(candidate))
        .cloned()
}

fn format_prebind_diagnostics(diagnostics: &[TreeCalcFormulaTextPrebindDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{:?} at {:?} for '{}': {}",
                diagnostic.code,
                diagnostic.source_span_utf8,
                diagnostic.source_token_text,
                diagnostic.detail
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('.').map(|(parent, _)| parent.to_string())
}

fn prepared_formula_to_tree_formula(
    formula: &PreparedFormula,
    owner_node_id: TreeNodeId,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<TreeFormula, OxCalcTreeBridgeError> {
    match formula {
        PreparedFormula::Literal { .. } | PreparedFormula::Binary { .. } => {
            Ok(prepared_formula_to_fixture_ast(formula, node_ids_by_path)?
                .to_tree_formula(owner_node_id))
        }
        PreparedFormula::OpaqueOxfml {
            source_text,
            reference_carriers,
        } => {
            let carriers = reference_carriers
                .iter()
                .map(|carrier| {
                    prepared_reference_carrier_to_tree_carrier(
                        carrier,
                        owner_node_id,
                        node_ids_by_path,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TreeFormula::opaque_oxfml(source_text.clone(), carriers))
        }
    }
}

fn prepared_formula_to_fixture_ast(
    formula: &PreparedFormula,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<FixtureFormulaAst, OxCalcTreeBridgeError> {
    match formula {
        PreparedFormula::Literal { value } => Ok(FixtureFormulaAst::Literal {
            value: value.clone(),
        }),
        PreparedFormula::Binary { op, left, right } => Ok(FixtureFormulaAst::Binary {
            op: match op {
                PreparedBinaryOp::Add => FixtureFormulaBinaryOp::Add,
                PreparedBinaryOp::Subtract => FixtureFormulaBinaryOp::Subtract,
                PreparedBinaryOp::Multiply => FixtureFormulaBinaryOp::Multiply,
                PreparedBinaryOp::Divide => FixtureFormulaBinaryOp::Divide,
            },
            left: Box::new(prepared_operand_to_fixture_ast(left, node_ids_by_path)?),
            right: Box::new(prepared_operand_to_fixture_ast(right, node_ids_by_path)?),
        }),
        PreparedFormula::OpaqueOxfml { .. } => Err(OxCalcTreeBridgeError::InvalidWorkspace(
            "opaque OxFml source is already a TreeFormula and cannot be lowered through fixture AST"
                .to_string(),
        )),
    }
}

fn prepared_reference_carrier_to_tree_carrier(
    carrier: &PreparedFormulaReferenceCarrier,
    owner_node_id: TreeNodeId,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<TreeFormulaReferenceCarrier, OxCalcTreeBridgeError> {
    match carrier {
        PreparedFormulaReferenceCarrier::DirectNode { source_token, path } => {
            Ok(TreeFormulaReferenceCarrier::named(
                source_token.clone(),
                TreeReference::DirectNode {
                    target_node_id: node_id_for(path, node_ids_by_path)?,
                },
            ))
        }
        PreparedFormulaReferenceCarrier::ChildrenV1 {
            source_token,
            base_path,
            source_token_text,
            source_span_utf8,
        } => {
            let base_node_id = node_id_for(base_path, node_ids_by_path)?;
            let mut collection =
                TreeCalcChildrenReferenceCollection::new(base_node_id, source_token_text.clone());
            if let Some((start_byte, end_byte)) = source_span_utf8 {
                collection = collection.with_source_span_utf8(*start_byte, *end_byte);
            }

            Ok(TreeFormulaReferenceCarrier::named(
                source_token.clone(),
                TreeReference::ReferenceCollection(TreeCalcReferenceCollection::ChildrenV1(
                    collection,
                )),
            ))
        }
        PreparedFormulaReferenceCarrier::ReferenceLiteralArrayV1 {
            source_token,
            source_token_text,
            source_span_utf8,
            elements,
        } => {
            let elements = elements
                .iter()
                .map(|element| match element {
                    PreparedReferenceLiteralArrayElement::ReferencePath { path } => {
                        Ok(TreeCalcReferenceLiteralArrayElement::ReferenceNode(
                            node_id_for(path, node_ids_by_path)?,
                        ))
                    }
                    PreparedReferenceLiteralArrayElement::ScalarValue { source_text } => {
                        Ok(TreeCalcReferenceLiteralArrayElement::ScalarValue {
                            source_text: source_text.clone(),
                        })
                    }
                })
                .collect::<Result<Vec<_>, OxCalcTreeBridgeError>>()?;
            let carrier_id = format!("dnatreecalc-reference-literal-array:v1:{source_token}");
            let host_ref_handle =
                format!("treecalc-hostref:v1:reference_literal_array:{source_token}");
            let mut collection =
                TreeCalcReferenceLiteralArrayCollection::reference_only_with_handle(
                    carrier_id,
                    host_ref_handle,
                    owner_node_id,
                    source_token_text.clone(),
                    elements,
                )
                .map_err(|error| {
                    OxCalcTreeBridgeError::FormulaBindingUnavailable(format!(
                        "reference literal array carrier {source_token} is not admissible: {error}"
                    ))
                })?;
            if let Some((start_byte, end_byte)) = source_span_utf8 {
                collection = collection.with_source_span_utf8(*start_byte, *end_byte);
            }

            Ok(TreeFormulaReferenceCarrier::named(
                source_token.clone(),
                TreeReference::ReferenceCollection(
                    TreeCalcReferenceCollection::ReferenceLiteralArrayV1(collection),
                ),
            ))
        }
        PreparedFormulaReferenceCarrier::CrossWorkspaceResolved {
            source_token,
            workspace_handle,
            target_node_id,
            target_node_handle,
            availability_version,
            carrier_id,
            detail,
        } => Ok(TreeFormulaReferenceCarrier::named(
            source_token.clone(),
            TreeReference::CrossWorkspaceResolved {
                workspace_handle: workspace_handle.clone(),
                target_node_id: TreeNodeId(*target_node_id),
                target_node_handle: target_node_handle.clone(),
                availability_version: availability_version.clone(),
                carrier_id: carrier_id.clone(),
                detail: detail.clone(),
            },
        )),
        PreparedFormulaReferenceCarrier::DynamicResolved {
            source_token,
            target_path,
            carrier_id,
            detail,
        } => Ok(TreeFormulaReferenceCarrier::named(
            source_token.clone(),
            TreeReference::DynamicResolved {
                target_node_id: node_id_for(target_path, node_ids_by_path)?,
                carrier_id: carrier_id.clone(),
                detail: detail.clone(),
            },
        )),
        PreparedFormulaReferenceCarrier::DynamicPotential {
            source_token,
            carrier_id,
            detail,
        } => Ok(TreeFormulaReferenceCarrier::named(
            source_token.clone(),
            TreeReference::DynamicPotential {
                carrier_id: carrier_id.clone(),
                detail: detail.clone(),
            },
        )),
    }
}

fn prepared_operand_to_fixture_ast(
    operand: &PreparedFormulaOperand,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<FixtureFormulaAst, OxCalcTreeBridgeError> {
    match operand {
        PreparedFormulaOperand::Literal { value } => Ok(FixtureFormulaAst::Literal {
            value: value.clone(),
        }),
        PreparedFormulaOperand::DirectNode { path } => {
            let target_node_id = node_id_for(path, node_ids_by_path)?;
            Ok(FixtureFormulaAst::Reference(TreeReference::DirectNode {
                target_node_id,
            }))
        }
        PreparedFormulaOperand::RelativePath {
            base,
            path_segments,
        } => Ok(FixtureFormulaAst::Reference(TreeReference::RelativePath {
            base: prepared_relative_path_base(*base),
            path_segments: path_segments.clone(),
        })),
    }
}

fn prepared_relative_path_base(base: PreparedRelativePathBase) -> RelativeReferenceBase {
    match base {
        PreparedRelativePathBase::SelfNode => RelativeReferenceBase::SelfNode,
        PreparedRelativePathBase::ParentNode => RelativeReferenceBase::ParentNode,
        PreparedRelativePathBase::Ancestor(distance) => RelativeReferenceBase::Ancestor(distance),
    }
}

fn node_id_for(
    path: &str,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<TreeNodeId, OxCalcTreeBridgeError> {
    node_ids_by_path.get(path).copied().ok_or_else(|| {
        OxCalcTreeBridgeError::InvalidWorkspace(format!(
            "node {path} has no assigned OxCalc node id"
        ))
    })
}

fn oxcalc_structural_symbol(name: &str) -> String {
    name.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(name)
        .to_string()
}

fn formula_artifact_id(path: &str) -> String {
    format!("formula:{}", path.replace('.', "/"))
}

fn bind_artifact_id(path: &str) -> String {
    format!("bind:{}", path.replace('.', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcalc_core::consumer::OxCalcTreeRunState;
    use oxcalc_core::structured_table::prebind_treecalc_table_structured_references;

    use crate::model::{WorkspaceFixture, WorkspaceNodeFixture};

    #[test]
    fn live_bridge_executes_minimal_named_node_smoke_fixture() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w002-smoke".to_string(),
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
                    formula: "2".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.B".to_string(),
                    formula: "=A+3".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::new([(
                    "Root.B",
                    PreparedFormula::Binary {
                        op: PreparedBinaryOp::Add,
                        left: PreparedFormulaOperand::DirectNode {
                            path: "Root.A".to_string(),
                        },
                        right: PreparedFormulaOperand::Literal {
                            value: "3".to_string(),
                        },
                    },
                )]),
                candidate_result_id: "cand:w002-smoke".to_string(),
                publication_id: "pub:w002-smoke".to_string(),
                compatibility_basis: "snapshot:w002-smoke".to_string(),
                artifact_token_basis: "snapshot:w002-smoke".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap();

        assert_eq!(result.run_state, OxCalcTreeRunState::Published);
        assert_eq!(result.published_values["Root.B"], "5");
        assert_eq!(
            result.dependency_edges_by_owner["Root.B"],
            vec!["Root.A".to_string()]
        );
        assert_eq!(result.node_states["Root.B"], NodeCalcStateProjection::Clean);
        assert!(result.evaluation_order.contains(&"Root.B".to_string()));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic == "oxcalc_tree_environment_runtime_lane:local_sequential_treecalc"
        }));
    }

    #[test]
    fn live_bridge_prebinds_raw_children_formula_text_through_oxcalc() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w005-children-raw".to_string(),
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
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(@CHILDREN)".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.A".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.B".to_string(),
                    formula: "3".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:w005-children-raw".to_string(),
                publication_id: "pub:w005-children-raw".to_string(),
                compatibility_basis: "snapshot:w005-children-raw".to_string(),
                artifact_token_basis: "snapshot:w005-children-raw".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap();

        assert_eq!(result.run_state, OxCalcTreeRunState::Published);
        assert_eq!(result.published_values["Root.Inputs"], "5");
        assert_eq!(
            result.dependency_edges_by_owner["Root.Inputs"],
            vec!["Root.Inputs.A".to_string(), "Root.Inputs.B".to_string()]
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("oxfml_prepared_formula_key"))
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("oxfml_runtime_prepared_formula_key"))
        );
    }

    #[test]
    fn live_bridge_prebinds_raw_children_sugar_formula_text_through_oxcalc() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w005-children-sugar-raw".to_string(),
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
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(.*)".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.A".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.B".to_string(),
                    formula: "3".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:w005-children-sugar-raw".to_string(),
                publication_id: "pub:w005-children-sugar-raw".to_string(),
                compatibility_basis: "snapshot:w005-children-sugar-raw".to_string(),
                artifact_token_basis: "snapshot:w005-children-sugar-raw".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap();

        assert_eq!(result.run_state, OxCalcTreeRunState::Published);
        assert_eq!(result.published_values["Root.Inputs"], "5");
        assert_eq!(
            result.dependency_edges_by_owner["Root.Inputs"],
            vec!["Root.Inputs.A".to_string(), "Root.Inputs.B".to_string()]
        );
    }

    #[test]
    fn live_bridge_prebinds_qualified_raw_children_formula_text_through_oxcalc() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w005-qualified-children-raw".to_string(),
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
                    node_id: "Root.base".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.base.A".to_string(),
                    formula: "11".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.base.B".to_string(),
                    formula: "13".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(base.@CHILDREN)".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.InputsSugar".to_string(),
                    formula: "=SUM(base.*)".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let result = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:w005-qualified-children-raw".to_string(),
                publication_id: "pub:w005-qualified-children-raw".to_string(),
                compatibility_basis: "snapshot:w005-qualified-children-raw".to_string(),
                artifact_token_basis: "snapshot:w005-qualified-children-raw".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap();

        assert_eq!(result.run_state, OxCalcTreeRunState::Published);
        assert_eq!(result.published_values["Root.Inputs"], "24");
        assert_eq!(result.published_values["Root.InputsSugar"], "24");
        assert_eq!(
            result.dependency_edges_by_owner["Root.Inputs"],
            vec!["Root.base.A".to_string(), "Root.base.B".to_string()]
        );
        assert_eq!(
            result.dependency_edges_by_owner["Root.InputsSugar"],
            vec!["Root.base.A".to_string(), "Root.base.B".to_string()]
        );
    }

    #[test]
    fn live_bridge_uses_oxcalc_structural_path_and_traversal_resolvers() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w004-structural-resolvers".to_string(),
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
                    node_id: "Root.Base".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Base.A".to_string(),
                    formula: "11".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Base.B".to_string(),
                    formula: "13".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.UseChildren".to_string(),
                    formula: "=SUM(Root.Base.@CHILDREN)".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Following".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Following.Total".to_string(),
                    formula: "=SUM(Root.Following.Total.@FOLLOWING)".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Following.A".to_string(),
                    formula: "4".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Following.B".to_string(),
                    formula: "6".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();
        let node_ids_by_path = assign_node_ids(&workspace);
        let root_id = root_node_id(&workspace, &node_ids_by_path).unwrap();
        let table_projections = build_table_projections(&workspace, &node_ids_by_path).unwrap();
        let structural_snapshot = build_structural_snapshot(
            &workspace,
            &PreparedFormulaCatalog::default(),
            &node_ids_by_path,
            root_id,
            &table_projections,
        )
        .unwrap();

        let children_resolutions = resolve_qualified_children_base_queries(
            &workspace,
            "Root.UseChildren",
            "=SUM(Root.Base.@CHILDREN)",
            &node_ids_by_path,
            &structural_snapshot,
        )
        .unwrap();
        assert_eq!(children_resolutions.len(), 1);
        assert_eq!(
            children_resolutions[0].resolution_layer,
            TreeCalcQualifiedBaseResolutionLayer::OxCalcStructuralPath
        );
        assert_eq!(
            children_resolutions[0].base_node_id,
            node_ids_by_path["Root.Base"]
        );
        assert!(
            children_resolutions[0]
                .resolution_identity
                .contains("treecalc-explicit-host-path:v1")
        );

        let ordered_query = treecalc_formula_text_ordered_selector_queries(
            node_ids_by_path["Root.Following.Total"],
            "=SUM(Root.Following.Total.@FOLLOWING)",
        )
        .into_iter()
        .next()
        .expect("ordered selector query");
        let ordered_resolution = resolve_ordered_selector_with_structural_traversal(
            &ordered_query,
            &structural_snapshot,
            node_ids_by_path["Root.Following.Total"],
        )
        .unwrap();

        assert_eq!(
            ordered_resolution.resolution_layer,
            TreeCalcOrderedSelectorResolutionLayer::OxCalcStructuralTraversal
        );
        assert_eq!(
            ordered_resolution.base_node_id,
            node_ids_by_path["Root.Following.Total"]
        );
        assert_eq!(
            ordered_resolution.member_node_ids,
            vec![
                node_ids_by_path["Root.Following.A"],
                node_ids_by_path["Root.Following.B"]
            ]
        );
        assert!(
            ordered_resolution
                .resolution_identity
                .contains("base_resolution=treecalc-explicit-host-path:v1")
        );
    }

    #[test]
    fn live_bridge_keeps_ordered_traversal_bounds_as_typed_oxcalc_errors() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w004-traversal-bounds".to_string(),
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
                    node_id: "Root.Base".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Base.A".to_string(),
                    formula: "1".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Base.B".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Total".to_string(),
                    formula: "=SUM(Root.Base.**)".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();
        let node_ids_by_path = assign_node_ids(&workspace);
        let root_id = root_node_id(&workspace, &node_ids_by_path).unwrap();
        let table_projections = build_table_projections(&workspace, &node_ids_by_path).unwrap();
        let structural_snapshot = build_structural_snapshot(
            &workspace,
            &PreparedFormulaCatalog::default(),
            &node_ids_by_path,
            root_id,
            &table_projections,
        )
        .unwrap();
        let query = treecalc_formula_text_ordered_selector_queries(
            node_ids_by_path["Root.Total"],
            "=SUM(Root.Base.**)",
        )
        .into_iter()
        .next()
        .expect("recursive selector query");

        let error = query
            .to_resolution_with_structural_path_base_and_traversal(
                &structural_snapshot,
                TreeCalcOrderedSelectorTraversalPolicy {
                    max_recursive_descendants: 1,
                },
            )
            .expect_err("small traversal bound should fail before prebind");

        assert!(
            error
                .to_string()
                .contains("exceeded traversal policy treecalc-traversal-bound:v1")
        );
    }

    #[test]
    fn live_bridge_projects_table_nodes_into_oxcalc_table_catalog() {
        let workspace =
            WorkspaceModel::try_from(WorkspaceFixture::from_repo_fixture("tables").unwrap())
                .unwrap();
        let node_ids_by_path = assign_node_ids(&workspace);
        let table_projections = build_table_projections(&workspace, &node_ids_by_path).unwrap();

        assert_eq!(table_projections.len(), 2);
        let projection = table_projections
            .iter()
            .find(|projection| projection.table_id == "tree-table:sales")
            .expect("SalesTable projection exists");
        assert_eq!(projection.table_id, "tree-table:sales");
        assert_eq!(projection.table_descriptor.table_name, "SalesTable");
        assert_eq!(
            projection
                .context_packet
                .enclosing_table_ref
                .as_ref()
                .unwrap()
                .table_id,
            "tree-table:sales"
        );
        assert!(projection.context_packet.caller_table_region.is_none());
        assert!(
            projection
                .table_context_identity
                .contains("treecalc.table_context.v1")
        );
        assert!(
            projection
                .body_metadata_identity
                .contains("formula:SalesTable.Columns.Tax")
        );

        let prebound = prebind_treecalc_table_structured_references(
            "=SUM(SalesTable[Amount])",
            &table_projections,
            None,
            None,
        );
        assert_eq!(prebound.len(), 1);
        assert_eq!(
            prebound[0].bind_record.effective_table_id.as_deref(),
            Some("tree-table:sales")
        );
        assert_eq!(
            prebound[0].bind_record.selected_column_ids,
            vec!["col:amount"]
        );
        let escaped_prebound = prebind_treecalc_table_structured_references(
            "=SUM([Sales]]Table][[Gross]]Amount]])",
            &table_projections,
            None,
            None,
        );
        assert_eq!(escaped_prebound.len(), 1);
        assert_eq!(
            escaped_prebound[0]
                .bind_record
                .effective_table_id
                .as_deref(),
            Some("tree-table:escaped-sales")
        );
        assert_eq!(
            escaped_prebound[0].bind_record.selected_column_ids,
            vec!["col:gross-amount"]
        );

        let mut first_table = WorkspaceFixture::from_repo_fixture("tables")
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.table.is_some())
            .unwrap();
        let mut second_table = first_table.clone();
        first_table.node_id = "Root.First".to_string();
        second_table.node_id = "Root.Second".to_string();
        if let Some(table) = first_table.table.as_mut() {
            table.display_path = None;
            table.canonical_path = None;
        }
        if let Some(table) = second_table.table.as_mut() {
            table.table_id = "tree-table:second".to_string();
            table.display_path = None;
            table.canonical_path = None;
        }
        let multi_table_workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "multi-table-anchor".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                WorkspaceNodeFixture {
                    node_id: "Root".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                first_table,
                second_table,
            ],
        })
        .unwrap();
        let multi_node_ids = assign_node_ids(&multi_table_workspace);
        let multi_projections = build_table_projections(&multi_table_workspace, &multi_node_ids)
            .expect("multi-table anchors allocate without collision");
        assert_eq!(multi_projections.len(), 2);
        assert!(
            multi_projections[1].virtual_anchor_identity
                != multi_projections[0].virtual_anchor_identity
        );
        assert_eq!(
            multi_projections[0].table_descriptor.table_range_ref,
            "B3:D7"
        );
        assert_eq!(
            multi_projections[1].table_descriptor.table_range_ref,
            "B18:D22"
        );

        let mut runtime_table_node = WorkspaceFixture::from_repo_fixture("tables")
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.table.is_some())
            .unwrap();
        runtime_table_node.node_id = "Root".to_string();
        let runtime_workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "table-runtime-projection".to_string(),
            description: None,
            profile: None,
            nodes: vec![runtime_table_node],
        })
        .unwrap();
        let runtime_result = LiveOxCalcTreeBridge::default()
            .execute_recalc(TreeRecalcRequest {
                workspace: runtime_workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:table-runtime-projection".to_string(),
                publication_id: "pub:table-runtime-projection".to_string(),
                compatibility_basis: "snapshot:table-runtime-projection".to_string(),
                artifact_token_basis: "snapshot:table-runtime-projection".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .unwrap();
        assert!(
            runtime_result.table_context_identities["Root"].contains("treecalc.table_context.v1")
        );

        let mut formula_table_node = WorkspaceFixture::from_repo_fixture("tables")
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.table.is_some())
            .unwrap();
        formula_table_node.node_id = "Root".to_string();
        let formula_workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "table-formula-prebind".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                formula_table_node,
                WorkspaceNodeFixture {
                    node_id: "Root.Columns".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Columns.Tax".to_string(),
                    formula: "=[@Amount] * 0.1".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();
        let formula_submission = PreparedSubmission::try_from_request(&TreeRecalcRequest {
            workspace: formula_workspace,
            formula_catalog: PreparedFormulaCatalog::default(),
            candidate_result_id: "cand:table-formula-prebind".to_string(),
            publication_id: "pub:table-formula-prebind".to_string(),
            compatibility_basis: "snapshot:table-formula-prebind".to_string(),
            artifact_token_basis: "snapshot:table-formula-prebind".to_string(),
            capability_profile_id: "treecalc-v1".to_string(),
            cycle_config: Default::default(),
        })
        .expect("table column formula participates in the bridge prebind path");
        let tax_node_id = formula_submission
            .paths_by_node_id
            .iter()
            .find_map(|(node_id, path)| (path == "Root.Columns.Tax").then_some(*node_id))
            .unwrap();
        let tax_binding = formula_submission
            .formula_catalog
            .try_get_binding(tax_node_id)
            .expect("table column formula binding exists");
        assert_eq!(tax_binding.expression.source_text(), "=[@Amount] * 0.1");
    }

    #[test]
    fn live_bridge_rejects_unresolved_qualified_children_base() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w005-qualified-children-unresolved".to_string(),
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
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(base.@CHILDREN)".to_string(),
                    is_meta: false,
                    table: None,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let error = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:w005-qualified-children-unresolved".to_string(),
                publication_id: "pub:w005-qualified-children-unresolved".to_string(),
                compatibility_basis: "snapshot:w005-qualified-children-unresolved".to_string(),
                artifact_token_basis: "snapshot:w005-qualified-children-unresolved".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .expect_err("qualified children with unresolved base must remain diagnostic");

        assert!(
            error
                .to_string()
                .contains("cannot resolve qualified children base token 'base'")
        );
    }
}

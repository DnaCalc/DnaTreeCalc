use std::collections::BTreeMap;

use oxcalc_core::consumer::{
    OxCalcTreeDocument, OxCalcTreeEnvironment, OxCalcTreeRecalcRequest, OxCalcTreeRuntimeFacade,
};
use oxcalc_core::formula::{
    FixtureFormulaAst, FixtureFormulaBinaryOp, TreeCalcChildrenReferenceCollection,
    TreeCalcFormulaTextPrebindDiagnostic, TreeCalcReferenceCollection, TreeFormula,
    TreeFormulaBinding, TreeFormulaCatalog, TreeFormulaReferenceCarrier, TreeReference,
    prebind_treecalc_formula_text, treecalc_formula_text_needs_prebind,
};
use oxcalc_core::structural::{
    BindArtifactId, FormulaArtifactId, StructuralNode, StructuralNodeKind, StructuralSnapshot,
    StructuralSnapshotId, TreeNodeId,
};

use super::bridge::{OxCalcTreeBridge, OxCalcTreeBridgeError};
use super::types::{
    NodeCalcStateProjection, PreparedBinaryOp, PreparedFormula, PreparedFormulaCatalog,
    PreparedFormulaOperand, PreparedFormulaReferenceCarrier, TreeRecalcRequest, TreeRecalcResult,
};
use crate::model::{NodeContentKind, WorkspaceModel, WorkspaceNode};

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
            project_dependency_edges(&result.dependency_graph.edges_by_owner, &submission);

        Ok(TreeRecalcResult {
            run_state: result.run_state,
            dependency_graph: result.dependency_graph,
            invalidation_closure: result.invalidation_closure,
            evaluation_order,
            dependency_edges_by_owner,
            published_values,
            node_states,
            diagnostics: result.diagnostics,
        })
    }
}

fn project_dependency_edges(
    edges_by_owner: &BTreeMap<TreeNodeId, Vec<oxcalc_core::dependency::DependencyEdge>>,
    submission: &PreparedSubmission,
) -> BTreeMap<String, Vec<String>> {
    edges_by_owner
        .iter()
        .filter_map(|(owner, edges)| {
            let owner_path = submission.paths_by_node_id.get(owner)?;
            let target_paths = edges
                .iter()
                .filter_map(|edge| {
                    submission
                        .paths_by_node_id
                        .get(&edge.target_node_id)
                        .cloned()
                })
                .collect::<Vec<_>>();
            Some((owner_path.clone(), target_paths))
        })
        .collect()
}

struct PreparedSubmission {
    structural_snapshot: StructuralSnapshot,
    formula_catalog: TreeFormulaCatalog,
    paths_by_node_id: BTreeMap<TreeNodeId, String>,
}

impl PreparedSubmission {
    fn try_from_request(request: &TreeRecalcRequest) -> Result<Self, OxCalcTreeBridgeError> {
        let node_ids_by_path = assign_node_ids(&request.workspace);
        let paths_by_node_id = node_ids_by_path
            .iter()
            .map(|(path, node_id)| (*node_id, path.clone()))
            .collect::<BTreeMap<_, _>>();
        let root_node_id = root_node_id(&request.workspace, &node_ids_by_path)?;
        let structural_snapshot = build_structural_snapshot(
            &request.workspace,
            &request.formula_catalog,
            &node_ids_by_path,
            root_node_id,
        )?;
        let formula_catalog = build_formula_catalog(
            &request.workspace,
            &request.formula_catalog,
            &node_ids_by_path,
        )?;

        Ok(Self {
            structural_snapshot,
            formula_catalog,
            paths_by_node_id,
        })
    }
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
        return Err(OxCalcTreeBridgeError::InvalidWorkspace(format!(
            "W002 bridge smoke expects exactly one root, found {}",
            workspace.root_paths.len()
        )));
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

fn build_structural_snapshot(
    workspace: &WorkspaceModel,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
    root_node_id: TreeNodeId,
) -> Result<StructuralSnapshot, OxCalcTreeBridgeError> {
    let mut nodes = Vec::new();

    for path in &workspace.node_order {
        let node = workspace.node(path).ok_or_else(|| {
            OxCalcTreeBridgeError::InvalidWorkspace(format!("node {path} missing from workspace"))
        })?;
        nodes.push(build_structural_node(
            node,
            formula_catalog,
            node_ids_by_path,
        )?);
    }

    StructuralSnapshot::create(StructuralSnapshotId(1), root_node_id, nodes)
        .map_err(|error| OxCalcTreeBridgeError::InvalidWorkspace(error.to_string()))
}

fn build_structural_node(
    node: &WorkspaceNode,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
) -> Result<StructuralNode, OxCalcTreeBridgeError> {
    let node_id = node_id_for(&node.path, node_ids_by_path)?;
    let parent_id = node
        .parent_path
        .as_deref()
        .map(|parent| node_id_for(parent, node_ids_by_path))
        .transpose()?;
    let child_ids = node
        .child_paths
        .iter()
        .map(|child| node_id_for(child, node_ids_by_path))
        .collect::<Result<Vec<_>, _>>()?;
    let has_oxcalc_formula = has_oxcalc_formula_binding(node, formula_catalog);

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
        symbol: node.name.clone(),
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
) -> bool {
    formula_catalog.contains_path(&node.path)
        || (node.content.kind() == NodeContentKind::Formula
            && treecalc_formula_text_needs_prebind(node.content.text()))
}

fn build_formula_catalog(
    workspace: &WorkspaceModel,
    formula_catalog: &PreparedFormulaCatalog,
    node_ids_by_path: &BTreeMap<String, TreeNodeId>,
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
            && treecalc_formula_text_needs_prebind(node.content.text())
        {
            prebind_treecalc_formula_text(owner_node_id, node.content.text()).map_err(|error| {
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
                    prepared_reference_carrier_to_tree_carrier(carrier, node_ids_by_path)
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
                },
                WorkspaceNodeFixture {
                    node_id: "Root.A".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.B".to_string(),
                    formula: "=A+3".to_string(),
                    is_meta: false,
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
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(@CHILDREN)".to_string(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.A".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.B".to_string(),
                    formula: "3".to_string(),
                    is_meta: false,
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
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(.*)".to_string(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.A".to_string(),
                    formula: "2".to_string(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs.B".to_string(),
                    formula: "3".to_string(),
                    is_meta: false,
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
    fn live_bridge_keeps_unsupported_qualified_children_formula_pending_upstream() {
        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "w005-qualified-children-pending".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                WorkspaceNodeFixture {
                    node_id: "Root".to_string(),
                    formula: String::new(),
                    is_meta: false,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Inputs".to_string(),
                    formula: "=SUM(base.@CHILDREN)".to_string(),
                    is_meta: false,
                },
            ],
        })
        .unwrap();

        let bridge = LiveOxCalcTreeBridge::default();
        let error = bridge
            .execute_recalc(TreeRecalcRequest {
                workspace,
                formula_catalog: PreparedFormulaCatalog::default(),
                candidate_result_id: "cand:w005-qualified-children-pending".to_string(),
                publication_id: "pub:w005-qualified-children-pending".to_string(),
                compatibility_basis: "snapshot:w005-qualified-children-pending".to_string(),
                artifact_token_basis: "snapshot:w005-qualified-children-pending".to_string(),
                capability_profile_id: "treecalc-v1".to_string(),
                cycle_config: Default::default(),
            })
            .expect_err("qualified children syntax is outside the current OxCalc prebind surface");

        assert!(
            error
                .to_string()
                .contains("UnsupportedQualifiedHostReference")
        );
    }
}

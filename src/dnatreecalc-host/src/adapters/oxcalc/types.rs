use std::collections::BTreeMap;

use oxcalc_core::consumer::OxCalcTreeRunState;
use oxcalc_core::dependency::{DependencyGraph, InvalidationClosure};
use oxcalc_core::recalc::NodeCalcState;

use crate::model::WorkspaceModel;

#[derive(Debug, Clone, PartialEq)]
pub struct TreeRecalcRequest {
    pub workspace: WorkspaceModel,
    pub formula_catalog: PreparedFormulaCatalog,
    pub candidate_result_id: String,
    pub publication_id: String,
    pub compatibility_basis: String,
    pub artifact_token_basis: String,
    pub capability_profile_id: String,
    pub cycle_config: CycleConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CycleConfig {
    pub profile_id: CycleProfileId,
    pub maximum_iterations: u32,
    pub maximum_change: f64,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            profile_id: CycleProfileId::NonIterativeStage1,
            maximum_iterations: 100,
            maximum_change: 0.001,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleProfileId {
    NonIterativeStage1,
    ExcelMatchIterative,
    IterativeDeterministicV0,
}

impl CycleProfileId {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NonIterativeStage1 => "cycle.non_iterative_stage1",
            Self::ExcelMatchIterative => "cycle.excel_match_iterative",
            Self::IterativeDeterministicV0 => "cycle.iterative_deterministic_v0",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRecalcResult {
    pub run_state: OxCalcTreeRunState,
    pub dependency_graph: DependencyGraph,
    pub invalidation_closure: InvalidationClosure,
    pub evaluation_order: Vec<String>,
    pub dependency_edges_by_owner: BTreeMap<String, Vec<String>>,
    pub table_context_identities: BTreeMap<String, String>,
    pub published_values: BTreeMap<String, String>,
    pub node_states: BTreeMap<String, NodeCalcStateProjection>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeCalcExternalWorkspace {
    pub workspace_handle: String,
    pub workspace: WorkspaceModel,
    pub availability_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeCalcCrossWorkspaceReferenceRequest {
    pub current_workspace_handle: String,
    pub current_workspace: WorkspaceModel,
    pub current_availability_version: String,
    pub external_workspaces: Vec<TreeCalcExternalWorkspace>,
    pub aliases: BTreeMap<String, String>,
    pub base_token_text: String,
    pub source_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeCalcCrossWorkspaceReferenceResolution {
    pub source_token: String,
    pub workspace_handle: String,
    pub target_path: String,
    pub target_node_id: u64,
    pub target_node_handle: String,
    pub availability_version: String,
    pub workspace_resolution_layer: String,
    pub local_resolution_layer: String,
    pub resolution_identity: String,
    pub prepared_carrier: PreparedFormulaReferenceCarrier,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedFormulaCatalog {
    bindings: BTreeMap<String, PreparedFormula>,
}

impl PreparedFormulaCatalog {
    #[must_use]
    pub fn new(bindings: impl IntoIterator<Item = (impl Into<String>, PreparedFormula)>) -> Self {
        Self {
            bindings: bindings
                .into_iter()
                .map(|(path, formula)| (path.into(), formula))
                .collect(),
        }
    }

    #[must_use]
    pub fn contains_path(&self, path: &str) -> bool {
        self.bindings.contains_key(path)
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&PreparedFormula> {
        self.bindings.get(path)
    }

    #[must_use]
    pub fn bindings(&self) -> &BTreeMap<String, PreparedFormula> {
        &self.bindings
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedFormula {
    Literal {
        value: String,
    },
    Binary {
        op: PreparedBinaryOp,
        left: PreparedFormulaOperand,
        right: PreparedFormulaOperand,
    },
    OpaqueOxfml {
        source_text: String,
        reference_carriers: Vec<PreparedFormulaReferenceCarrier>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedFormulaOperand {
    Literal {
        value: String,
    },
    DirectNode {
        path: String,
    },
    RelativePath {
        base: PreparedRelativePathBase,
        path_segments: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedRelativePathBase {
    SelfNode,
    ParentNode,
    Ancestor(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedFormulaReferenceCarrier {
    DirectNode {
        source_token: String,
        path: String,
    },
    ChildrenV1 {
        source_token: String,
        base_path: String,
        source_token_text: String,
        source_span_utf8: Option<(usize, usize)>,
    },
    ReferenceLiteralArrayV1 {
        source_token: String,
        source_token_text: String,
        source_span_utf8: Option<(usize, usize)>,
        elements: Vec<PreparedReferenceLiteralArrayElement>,
    },
    CrossWorkspaceResolved {
        source_token: String,
        workspace_handle: String,
        target_node_id: u64,
        target_node_handle: String,
        availability_version: String,
        carrier_id: String,
        detail: String,
    },
    DynamicResolved {
        source_token: String,
        target_path: String,
        carrier_id: String,
        detail: String,
    },
    DynamicPotential {
        source_token: String,
        carrier_id: String,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedReferenceLiteralArrayElement {
    ReferencePath { path: String },
    ScalarValue { source_text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCalcStateProjection {
    Clean,
    DirtyPending,
    Needed,
    Evaluating,
    VerifiedClean,
    PublishReady,
    RejectedPendingRepair,
    CycleBlocked,
}

impl From<NodeCalcState> for NodeCalcStateProjection {
    fn from(value: NodeCalcState) -> Self {
        match value {
            NodeCalcState::Clean => Self::Clean,
            NodeCalcState::DirtyPending => Self::DirtyPending,
            NodeCalcState::Needed => Self::Needed,
            NodeCalcState::Evaluating => Self::Evaluating,
            NodeCalcState::VerifiedClean => Self::VerifiedClean,
            NodeCalcState::PublishReady => Self::PublishReady,
            NodeCalcState::RejectedPendingRepair => Self::RejectedPendingRepair,
            NodeCalcState::CycleBlocked => Self::CycleBlocked,
        }
    }
}

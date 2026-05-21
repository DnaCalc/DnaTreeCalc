use std::collections::BTreeMap;

use oxcalc_core::consumer::OxCalcTreeRunState;
use oxcalc_core::recalc::NodeCalcState;

use crate::model::WorkspaceModel;

#[derive(Debug, Clone, PartialEq)]
pub struct TreeRecalcRequest {
    pub workspace: WorkspaceModel,
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
    pub published_values: BTreeMap<String, String>,
    pub node_states: BTreeMap<String, NodeCalcStateProjection>,
    pub diagnostics: Vec<String>,
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

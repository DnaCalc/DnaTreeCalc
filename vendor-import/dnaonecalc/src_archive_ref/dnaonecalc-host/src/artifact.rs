use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Scenario,
    ScenarioRun,
    ResultSurface,
    CandidateResult,
    CommitDecision,
    RejectDecision,
    ExecutionTrace,
    ReplayCapture,
    Observation,
    Comparison,
    Witness,
    HandoffPacket,
    CapabilityLedgerSnapshot,
}

impl ArtifactKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Scenario => "scenario",
            Self::ScenarioRun => "scenario_run",
            Self::ResultSurface => "result_surface",
            Self::CandidateResult => "candidate_result",
            Self::CommitDecision => "commit_decision",
            Self::RejectDecision => "reject_decision",
            Self::ExecutionTrace => "execution_trace",
            Self::ReplayCapture => "replay_capture",
            Self::Observation => "observation",
            Self::Comparison => "comparison",
            Self::Witness => "witness",
            Self::HandoffPacket => "handoff_packet",
            Self::CapabilityLedgerSnapshot => "capability_ledger_snapshot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableArtifactRef {
    pub artifact_kind: String,
    pub logical_id: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLineageRef {
    pub relation: String,
    pub artifact_ref: StableArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactAttachmentRef {
    pub logical_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub schema_id: String,
    pub schema_version: String,
    pub artifact_kind: String,
    pub logical_id: String,
    pub content_hash: String,
    pub created_at_unix_ms: u64,
    pub created_by_build: String,
    pub host_profile_id: String,
    pub packet_kind: String,
    pub seam_pin_set_id: String,
    pub capability_floor: String,
    pub provisionality_state: String,
    pub lineage_refs: Vec<ArtifactLineageRef>,
    pub attachment_refs: Vec<ArtifactAttachmentRef>,
    pub capability_snapshot_ref: Option<StableArtifactRef>,
}

impl ArtifactEnvelope {
    pub fn stable_ref(&self) -> StableArtifactRef {
        StableArtifactRef {
            artifact_kind: self.artifact_kind.clone(),
            logical_id: self.logical_id.clone(),
            content_hash: Some(self.content_hash.clone()),
        }
    }
}

pub fn stable_hash<T: Hash>(value: &T) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

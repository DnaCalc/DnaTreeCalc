//! Deferred OxVba integration placeholder.
//!
//! The previous implementation consumed stale OxVba crates that no longer
//! exist. This resync intentionally removes that dependency path; WS-15 will
//! reintroduce VBA hosting against the current OxVba surface.

use oxfml_core::interface::{
    HostFunctionInvocation, HostFunctionProvider, HostFunctionProviderError,
    LibraryContextProvider, LibraryContextSnapshotRef,
};
use oxfml_core::semantics::LibraryContextSnapshot;
use oxfunc_core::value::{CalcValue, WorksheetErrorCode};

const DEFERRED_DIAGNOSTIC: &str =
    "OxVba integration is deferred for the native OxFml/OxFunc resync";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaSourceModule {
    pub module_name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaSourceProject {
    pub project_name: String,
    pub modules: Vec<VbaSourceModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VbaProjectAssociationScope {
    Workspace,
    FormulaSpace { formula_space_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VbaProjectLoadStatus {
    Unloaded,
    Loaded,
    Failed { diagnostic: String },
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaProjectAssociation {
    pub association_id: String,
    pub scope: VbaProjectAssociationScope,
    pub project_ref: String,
    pub project_identity: Option<String>,
    pub root_object_name: String,
    pub enabled: bool,
    pub source_fingerprint: Option<String>,
    pub last_load_status: VbaProjectLoadStatus,
}

impl VbaProjectAssociation {
    pub fn workspace_source(
        association_id: impl Into<String>,
        project_ref: impl Into<String>,
    ) -> Self {
        Self {
            association_id: association_id.into(),
            scope: VbaProjectAssociationScope::Workspace,
            project_ref: project_ref.into(),
            project_identity: None,
            root_object_name: "Application".to_string(),
            enabled: false,
            source_fingerprint: None,
            last_load_status: VbaProjectLoadStatus::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VbaUdfAdmissionStatus {
    Admitted,
    Rejected { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredVbaRegistration {
    pub function_name: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaUdfRegistration {
    pub association_id: String,
    pub formula_name: String,
    pub callable_id: String,
    pub registration: DeferredVbaRegistration,
    pub admission_status: VbaUdfAdmissionStatus,
    pub type_map_status: String,
    pub excel_oracle_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaUdfCandidate {
    pub association_id: String,
    pub project_name: String,
    pub module_name: String,
    pub procedure_name: String,
    pub callable_id: String,
    pub admission_status: VbaUdfAdmissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaUdfExcelOracleExpectation {
    pub case_id: String,
    pub formula_call: String,
    pub oracle_ref: String,
    pub expected_number: String,
}

#[derive(Debug, Clone)]
pub struct VbaHostRuntime {
    association: VbaProjectAssociation,
    candidates: Vec<VbaUdfCandidate>,
    registrations: Vec<VbaUdfRegistration>,
    snapshot: LibraryContextSnapshot,
}

impl VbaHostRuntime {
    pub fn load_source_project(
        _association: VbaProjectAssociation,
        _project: VbaSourceProject,
        _application_version: &str,
        _oracle_ref: Option<String>,
    ) -> Result<Self, String> {
        Err(DEFERRED_DIAGNOSTIC.to_string())
    }

    pub fn load_source_project_for_evaluation(
        association: VbaProjectAssociation,
        project: VbaSourceProject,
        application_version: &str,
    ) -> Result<Self, String> {
        Self::load_source_project(association, project, application_version, None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_project_path_for_evaluation(
        association: VbaProjectAssociation,
        _project_path: impl AsRef<std::path::Path>,
        _application_version: &str,
    ) -> Result<Self, String> {
        let _ = association;
        Err(DEFERRED_DIAGNOSTIC.to_string())
    }

    pub fn association(&self) -> &VbaProjectAssociation {
        &self.association
    }

    pub fn registrations(&self) -> &[VbaUdfRegistration] {
        &self.registrations
    }

    pub fn candidates(&self) -> &[VbaUdfCandidate] {
        &self.candidates
    }
}

impl HostFunctionProvider for VbaHostRuntime {
    fn invoke_host_function(
        &self,
        _invocation: &HostFunctionInvocation,
    ) -> Result<CalcValue, HostFunctionProviderError> {
        Ok(CalcValue::error(WorksheetErrorCode::Name))
    }
}

impl LibraryContextProvider for VbaHostRuntime {
    fn current_snapshot(&self) -> LibraryContextSnapshot {
        self.snapshot.clone()
    }

    fn snapshot_by_identity(
        &self,
        snapshot_ref: &LibraryContextSnapshotRef,
    ) -> Option<LibraryContextSnapshot> {
        (snapshot_ref == &LibraryContextSnapshotRef::from(&self.snapshot))
            .then(|| self.snapshot.clone())
    }
}

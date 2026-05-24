use super::types::{
    TreeCalcCrossWorkspaceReferenceRequest, TreeCalcCrossWorkspaceReferenceResolution,
    TreeRecalcRequest, TreeRecalcResult,
};
use oxcalc_core::structured_table::{
    TreeCalcDynamicTableRebindReport, TreeCalcDynamicTableRebindRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum OxCalcTreeBridgeError {
    #[error("upstream OxCalc runtime failed: {0}")]
    Upstream(String),
    #[error("workspace cannot be submitted to OxCalc: {0}")]
    InvalidWorkspace(String),
    #[error("TreeCalc formula binding is not available yet: {0}")]
    FormulaBindingUnavailable(String),
}

pub trait OxCalcTreeBridge {
    fn execute_recalc(
        &self,
        request: TreeRecalcRequest,
    ) -> Result<TreeRecalcResult, OxCalcTreeBridgeError>;

    fn classify_dynamic_table_rebind(
        &self,
        request: TreeCalcDynamicTableRebindRequest,
    ) -> Result<TreeCalcDynamicTableRebindReport, OxCalcTreeBridgeError>;

    fn resolve_cross_workspace_reference(
        &self,
        _request: TreeCalcCrossWorkspaceReferenceRequest,
    ) -> Result<TreeCalcCrossWorkspaceReferenceResolution, OxCalcTreeBridgeError> {
        Err(OxCalcTreeBridgeError::Upstream(
            "cross-workspace reference resolution is not implemented by this bridge".to_string(),
        ))
    }
}

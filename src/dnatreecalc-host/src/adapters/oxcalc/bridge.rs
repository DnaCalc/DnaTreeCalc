use super::types::{TreeRecalcRequest, TreeRecalcResult};

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
}

//! Reusable test fixtures for the host crate.
//!
//! The recording bridge here counts every call into the `OxCalcTreeBridge`
//! trait so tests can assert load-bearing invariants such as
//! "switching skins must not call the bridge."

use std::sync::Mutex;

use crate::adapters::oxcalc::{
    OxCalcTreeBridge, OxCalcTreeBridgeError, TreeRecalcRequest, TreeRecalcResult,
};
use oxcalc_core::structured_table::{
    TreeCalcDynamicTableRebindReport, TreeCalcDynamicTableRebindRequest,
};

/// A test-only bridge that records every `execute_recalc` call.
///
/// Always returns `Err(Upstream(_))` so a test using this bridge must
/// not depend on a successful recalc result — the point is to assert
/// *whether* the bridge was engaged, not what it produced. Use the
/// live bridge for tests that exercise real recalc behavior.
#[derive(Default)]
pub struct RecordingBridge {
    calls: Mutex<Vec<TreeRecalcRequest>>,
}

impl RecordingBridge {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of `execute_recalc` calls so far.
    pub fn recalc_count(&self) -> usize {
        self.calls.lock().expect("recording bridge poisoned").len()
    }

    /// Snapshot of all recorded requests, for debugging.
    pub fn calls(&self) -> Vec<TreeRecalcRequest> {
        self.calls
            .lock()
            .expect("recording bridge poisoned")
            .clone()
    }
}

impl OxCalcTreeBridge for RecordingBridge {
    fn execute_recalc(
        &self,
        request: TreeRecalcRequest,
    ) -> Result<TreeRecalcResult, OxCalcTreeBridgeError> {
        self.calls
            .lock()
            .expect("recording bridge poisoned")
            .push(request);
        Err(OxCalcTreeBridgeError::Upstream(
            "RecordingBridge is for invariant assertions only; it does not produce results."
                .to_string(),
        ))
    }

    fn classify_dynamic_table_rebind(
        &self,
        _request: TreeCalcDynamicTableRebindRequest,
    ) -> Result<TreeCalcDynamicTableRebindReport, OxCalcTreeBridgeError> {
        Err(OxCalcTreeBridgeError::Upstream(
            "RecordingBridge is for invariant assertions only; it does not produce dynamic table rebind reports."
                .to_string(),
        ))
    }
}

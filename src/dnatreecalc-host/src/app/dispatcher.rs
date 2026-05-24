use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, IntentReceipt, SelectionState, WorkspaceIntent,
};
use leptos::prelude::*;

use crate::adapters::oxcalc::OxCalcTreeBridge;

/// The live host-side dispatcher.
///
/// Routes selection intents to the shared `RwSignal<SelectionState>`
/// (no engine call, by design — selection is facade state per
/// `docs/ux/SKINS.md` §2.5 routing) and accepts `EditFormula` intents.
/// In this skeleton dispatcher the formula path simply records intents. The
/// `dtc-osq.6` corpus runner proves the minimal walk-up reference bridge path;
/// wiring formula-edit intents into shell projection remains the click-through
/// lane.
///
/// The bridge handle is held but unused for now so that landing the
/// next bead is an additive change inside `dispatch`, not a constructor
/// shape change rippling through every callsite.
pub struct HostDispatcher {
    selection: RwSignal<SelectionState>,
    #[allow(dead_code)]
    bridge: Option<Arc<dyn OxCalcTreeBridge + Send + Sync>>,
    log: Mutex<Vec<WorkspaceIntent>>,
}

impl HostDispatcher {
    #[must_use]
    pub fn new(
        selection: RwSignal<SelectionState>,
        bridge: Option<Arc<dyn OxCalcTreeBridge + Send + Sync>>,
    ) -> Self {
        Self {
            selection,
            bridge,
            log: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot of intents dispatched since construction. Tests use this
    /// to assert routing behavior without observing reactive state from
    /// the outside.
    pub fn intents(&self) -> Vec<WorkspaceIntent> {
        self.log.lock().expect("dispatcher log poisoned").clone()
    }

    pub fn clear_log(&self) {
        self.log.lock().expect("dispatcher log poisoned").clear();
    }
}

impl Dispatcher for HostDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        self.log
            .lock()
            .expect("dispatcher log poisoned")
            .push(intent.clone());
        match intent {
            WorkspaceIntent::SelectNode(target) => {
                self.selection.set(SelectionState::with_primary(target));
                IntentReceipt::accepted()
            }
            WorkspaceIntent::EditFormula { .. } => {
                // Walking skeleton accepts the intent so the dispatch path
                // is exercised, but does not yet call the bridge. The bridge
                // already runs activated corpora; here the missing piece is
                // the host's per-intent request builder and value projection.
                IntentReceipt::accepted()
            }
            // The framework's WorkspaceIntent is intentionally
            // `#[non_exhaustive]` so adding a variant in a future bead is
            // an additive change. A variant that reaches this branch is
            // one this dispatcher version does not know — reject loudly
            // rather than silently ignore.
            _ => IntentReceipt::rejected(IntentError::Unsupported),
        }
    }
}

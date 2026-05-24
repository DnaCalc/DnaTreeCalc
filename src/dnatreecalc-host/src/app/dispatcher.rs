use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, IntentReceipt, SelectionState, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;

use super::session::TreeWorkspaceSession;

/// The live host-side dispatcher.
///
/// Routes selection intents to the shared `RwSignal<SelectionState>`
/// (no engine call, by design — selection is UI/session state per
/// `docs/ux/SKINS.md` §2.5 routing). Formula edits are routed into the
/// direct OxCalc context session when one is attached, then the dispatcher
/// republishes the updated workspace projection for skins.
pub struct HostDispatcher {
    selection: RwSignal<SelectionState>,
    workspace: Option<RwSignal<WorkspaceState>>,
    session: Option<Arc<Mutex<TreeWorkspaceSession>>>,
    log: Mutex<Vec<WorkspaceIntent>>,
}

impl HostDispatcher {
    #[must_use]
    pub fn new(selection: RwSignal<SelectionState>) -> Self {
        Self {
            selection,
            workspace: None,
            session: None,
            log: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_session(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Self {
        Self {
            selection,
            workspace: Some(workspace),
            session: Some(session),
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
            WorkspaceIntent::EditFormula { node, content } => {
                self.apply_formula_edit(&node, content).map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                )
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

impl HostDispatcher {
    fn apply_formula_edit(
        &self,
        node: &dnatreecalc_skin_framework::NodeId,
        content: String,
    ) -> Result<(), String> {
        let Some(session) = &self.session else {
            return Ok(());
        };
        let mut session = session
            .lock()
            .map_err(|_| "workspace session mutex poisoned".to_string())?;
        session
            .edit_formula(node, content)
            .map_err(|error| error.to_string())?;
        session.recalculate().map_err(|error| error.to_string())?;
        if let Some(workspace) = self.workspace {
            workspace.set(
                session
                    .workspace_state()
                    .map_err(|error| error.to_string())?,
            );
        }
        Ok(())
    }
}

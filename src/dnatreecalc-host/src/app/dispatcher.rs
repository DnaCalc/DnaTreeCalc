use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, IntentReceipt, SelectionState, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::session::TreeWorkspaceSession;

thread_local! {
    static HOST_SESSIONS: RefCell<BTreeMap<u64, Arc<Mutex<TreeWorkspaceSession>>>> =
        const { RefCell::new(BTreeMap::new()) };
}

static NEXT_HOST_SESSION_ID: AtomicU64 = AtomicU64::new(1);

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
    session_id: Option<u64>,
    log: Mutex<Vec<WorkspaceIntent>>,
}

impl HostDispatcher {
    #[must_use]
    pub fn new(selection: RwSignal<SelectionState>) -> Self {
        Self {
            selection,
            workspace: None,
            session_id: None,
            log: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_session(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Self {
        let session_id = NEXT_HOST_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        HOST_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(session_id, session);
        });
        Self {
            selection,
            workspace: Some(workspace),
            session_id: Some(session_id),
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
            WorkspaceIntent::EditFormula { node, content } => self
                .apply_calc_affecting_edit(|session| session.edit_formula(&node, content))
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::EditContent { node, content } => self
                .apply_calc_affecting_edit(|session| session.edit_formula(&node, content))
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::Recalculate => self.apply_calc_affecting_edit(|_| Ok(())).map_or_else(
                |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                |_| IntentReceipt::accepted(),
            ),
            WorkspaceIntent::AddNode {
                parent,
                symbol,
                content,
            } => self
                .apply_calc_affecting_edit(|session| {
                    session
                        .add_node(parent.as_ref(), symbol, content)
                        .map(|_| ())
                })
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::RenameNode { node, new_symbol } => self
                .apply_calc_affecting_edit(|session| {
                    session.rename_node(&node, new_symbol).map(|_| ())
                })
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::MoveNode {
                node,
                new_parent,
                new_index,
            } => self
                .apply_calc_affecting_edit(|session| {
                    session
                        .move_node(&node, new_parent.as_ref(), new_index)
                        .map(|_| ())
                })
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::ReorderNode { node, new_index } => self
                .apply_calc_affecting_edit(|session| session.reorder_node(&node, new_index))
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::DeleteNode { node } => self
                .apply_calc_affecting_edit(|session| session.delete_node(&node))
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
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
    fn apply_calc_affecting_edit(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<(), super::session::TreeWorkspaceSessionError>,
    ) -> Result<(), String> {
        let Some(session_id) = self.session_id else {
            return Ok(());
        };
        HOST_SESSIONS.with(|sessions| {
            let session = sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| "workspace session handle is not available".to_string())?;
            let mut session = session
                .lock()
                .map_err(|_| "workspace session mutex poisoned".to_string())?;
            edit(&mut session).map_err(|error| error.to_string())?;
            session.recalculate().map_err(|error| error.to_string())?;
            if let Some(workspace) = self.workspace {
                workspace.set(
                    session
                        .workspace_state()
                        .map_err(|error| error.to_string())?,
                );
            }
            Ok(())
        })
    }
}

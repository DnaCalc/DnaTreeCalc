use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    Dispatcher, IntentError, IntentReceipt, NodeId, SelectionState, SharedSkinStateHandle,
    WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::session::TreeWorkspaceSession;
use crate::model::{WorkspaceFixture, WorkspaceModel};

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
    shared: Option<SharedSkinStateHandle>,
    session_id: Mutex<Option<u64>>,
    workspace_sessions: Mutex<BTreeMap<String, u64>>,
    next_workspace_ordinal: AtomicU64,
    log: Mutex<Vec<WorkspaceIntent>>,
}

impl HostDispatcher {
    #[must_use]
    pub fn new(selection: RwSignal<SelectionState>) -> Self {
        Self {
            selection,
            workspace: None,
            shared: None,
            session_id: Mutex::new(None),
            workspace_sessions: Mutex::new(BTreeMap::new()),
            next_workspace_ordinal: AtomicU64::new(1),
            log: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_session(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Self {
        Self::with_session_and_shared(selection, workspace, session, None)
    }

    #[must_use]
    pub fn with_session_and_shared(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
        shared: Option<SharedSkinStateHandle>,
    ) -> Self {
        let session_id = NEXT_HOST_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let workspace_id = session
            .lock()
            .expect("workspace session mutex poisoned")
            .workspace_state()
            .expect("workspace session must project")
            .workspace_id;
        HOST_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(session_id, session);
        });
        let mut workspace_sessions = BTreeMap::new();
        workspace_sessions.insert(workspace_id.clone(), session_id);
        if let Some(shared) = shared {
            shared.update(|state| {
                state.workspace_ids = vec![workspace_id.clone()];
                state.active_workspace_id = Some(workspace_id.clone());
            });
        }
        Self {
            selection,
            workspace: Some(workspace),
            shared,
            session_id: Mutex::new(Some(session_id)),
            workspace_sessions: Mutex::new(workspace_sessions),
            next_workspace_ordinal: AtomicU64::new(1),
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
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::EditContent { node, content } => self
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::EditContentDeferred { node, content } => self
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::ProjectOnly,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::Recalculate => self
                .apply_workspace_edit(|_| Ok(()), WorkspaceEditPublication::Recalculate)
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| IntentReceipt::accepted(),
                ),
            WorkspaceIntent::AddNode {
                parent,
                symbol,
                content,
            } => match self.apply_workspace_edit(
                |session| session.add_node(parent.as_ref(), symbol, content),
                WorkspaceEditPublication::Recalculate,
            ) {
                Ok(created) => {
                    self.selection
                        .set(SelectionState::with_primary(Some(created)));
                    IntentReceipt::accepted()
                }
                Err(error) => IntentReceipt::rejected(IntentError::Rejected(error)),
            },
            WorkspaceIntent::RenameNode { node, new_symbol } => match self.apply_workspace_edit(
                |session| session.rename_node(&node, new_symbol),
                WorkspaceEditPublication::Recalculate,
            ) {
                Ok(renamed) => {
                    self.selection
                        .set(SelectionState::with_primary(Some(renamed)));
                    IntentReceipt::accepted()
                }
                Err(error) => IntentReceipt::rejected(IntentError::Rejected(error)),
            },
            WorkspaceIntent::MoveNode {
                node,
                new_parent,
                new_index,
            } => match self.apply_workspace_edit(
                |session| session.move_node(&node, new_parent.as_ref(), new_index),
                WorkspaceEditPublication::Recalculate,
            ) {
                Ok(moved) => {
                    self.selection
                        .set(SelectionState::with_primary(Some(moved)));
                    IntentReceipt::accepted()
                }
                Err(error) => IntentReceipt::rejected(IntentError::Rejected(error)),
            },
            WorkspaceIntent::ReorderNode { node, new_index } => self
                .apply_workspace_edit(
                    |session| session.reorder_node(&node, new_index),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| {
                        self.selection.set(SelectionState::with_primary(Some(node)));
                        IntentReceipt::accepted()
                    },
                ),
            WorkspaceIntent::DeleteNode { node } => {
                let next_selection = parent_node_id(node.as_str());
                self.apply_workspace_edit(
                    |session| session.delete_node(&node),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    |_| {
                        self.selection
                            .set(SelectionState::with_primary(next_selection));
                        IntentReceipt::accepted()
                    },
                )
            }
            WorkspaceIntent::NewWorkspace => self.create_workspace().map_or_else(
                |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                |_| IntentReceipt::accepted(),
            ),
            WorkspaceIntent::SwitchWorkspace { workspace_id } => {
                self.switch_workspace(&workspace_id).map_or_else(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceEditPublication {
    Recalculate,
    ProjectOnly,
}

impl HostDispatcher {
    fn apply_workspace_edit<T>(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<T, super::session::TreeWorkspaceSessionError>,
        publication: WorkspaceEditPublication,
    ) -> Result<T, String> {
        let session_id = self.active_session_id()?;
        HOST_SESSIONS.with(|sessions| {
            let session = sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| "workspace session handle is not available".to_string())?;
            let mut session = session
                .lock()
                .map_err(|_| "workspace session mutex poisoned".to_string())?;
            let result = edit(&mut session).map_err(|error| error.to_string())?;
            if matches!(publication, WorkspaceEditPublication::Recalculate) {
                session.recalculate().map_err(|error| error.to_string())?;
            }
            if let Some(workspace) = self.workspace {
                workspace.set(
                    session
                        .workspace_state()
                        .map_err(|error| error.to_string())?,
                );
            }
            Ok(result)
        })
    }

    fn active_session_id(&self) -> Result<u64, String> {
        self.session_id
            .lock()
            .map_err(|_| "workspace session id mutex poisoned".to_string())?
            .ok_or_else(|| "workspace session handle is not attached".to_string())
    }

    fn create_workspace(&self) -> Result<String, String> {
        let ordinal = self.next_workspace_ordinal.fetch_add(1, Ordering::Relaxed);
        let workspace_id = format!("Workspace {ordinal}");
        let session = Arc::new(Mutex::new(empty_workspace_session(&workspace_id)?));
        let session_id = NEXT_HOST_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        HOST_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(session_id, session.clone());
        });
        self.workspace_sessions
            .lock()
            .map_err(|_| "workspace catalog mutex poisoned".to_string())?
            .insert(workspace_id.clone(), session_id);
        self.activate_session(&workspace_id, session_id, &session)?;
        Ok(workspace_id)
    }

    fn switch_workspace(&self, workspace_id: &str) -> Result<(), String> {
        let session_id = self
            .workspace_sessions
            .lock()
            .map_err(|_| "workspace catalog mutex poisoned".to_string())?
            .get(workspace_id)
            .copied()
            .ok_or_else(|| format!("unknown workspace '{workspace_id}'"))?;
        let session = HOST_SESSIONS.with(|sessions| {
            sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| "workspace session handle is not available".to_string())
        })?;
        self.activate_session(workspace_id, session_id, &session)
    }

    fn activate_session(
        &self,
        workspace_id: &str,
        session_id: u64,
        session: &Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Result<(), String> {
        *self
            .session_id
            .lock()
            .map_err(|_| "workspace session id mutex poisoned".to_string())? = Some(session_id);
        let state = session
            .lock()
            .map_err(|_| "workspace session mutex poisoned".to_string())?
            .workspace_state()
            .map_err(|error| error.to_string())?;
        if let Some(workspace) = self.workspace {
            workspace.set(state);
        }
        self.selection.set(SelectionState::default());
        if let Some(shared) = self.shared {
            let workspace_ids = self
                .workspace_sessions
                .lock()
                .map_err(|_| "workspace catalog mutex poisoned".to_string())?
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            shared.update(|state| {
                state.workspace_ids = workspace_ids;
                state.active_workspace_id = Some(workspace_id.to_string());
                state.manual_recalc_pending = false;
            });
        }
        Ok(())
    }
}

fn parent_node_id(path: &str) -> Option<NodeId> {
    path.rsplit_once('.')
        .map(|(parent, _)| NodeId::new(parent.to_string()))
}

fn empty_workspace_session(workspace_id: &str) -> Result<TreeWorkspaceSession, String> {
    let fixture = WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: workspace_id.to_string(),
        description: None,
        profile: None,
        nodes: Vec::new(),
    };
    let model = WorkspaceModel::try_from(fixture).map_err(|error| error.to_string())?;
    TreeWorkspaceSession::from_model(&model).map_err(|error| error.to_string())
}

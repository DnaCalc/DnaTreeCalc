use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    DependencyDeltaProjection, Dispatcher, IntentError, IntentReceipt, NodeId, NodeKey,
    NodeValueDeltaProjection, NodeView, SelectionState, SharedSkinStateHandle,
    StructuralDeltaProjection, WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent,
    WorkspaceState,
};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
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
    next_projection_seq: AtomicU64,
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
            next_projection_seq: AtomicU64::new(1),
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
            next_projection_seq: AtomicU64::new(1),
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
                    .with_delta(WorkspaceDelta::unchanged(self.current_projection_seq()))
            }
            WorkspaceIntent::EditFormula { node, content } => self
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::EditContent { node, content } => self
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::EditContentDeferred { node, content } => self
                .apply_workspace_edit(
                    |session| session.edit_formula(&node, content),
                    WorkspaceEditPublication::ProjectOnly,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::Recalculate => self
                .apply_workspace_edit(|_| Ok(()), WorkspaceEditPublication::Recalculate)
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::AddNode {
                parent,
                symbol,
                content,
            } => match self.apply_workspace_edit(
                |session| session.add_node(parent.as_ref(), symbol, content),
                WorkspaceEditPublication::Recalculate,
            ) {
                Ok(publication) => {
                    let created = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(created)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => IntentReceipt::rejected(IntentError::Rejected(error)),
            },
            WorkspaceIntent::RenameNode { node, new_symbol } => match self.apply_workspace_edit(
                |session| session.rename_node(&node, new_symbol),
                WorkspaceEditPublication::Recalculate,
            ) {
                Ok(publication) => {
                    let renamed = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(renamed)));
                    receipt_for_publication(publication.with_result(()))
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
                Ok(publication) => {
                    let moved = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(moved)));
                    receipt_for_publication(publication.with_result(()))
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
                    |publication| {
                        self.selection.set(SelectionState::with_primary(Some(node)));
                        receipt_for_publication(publication)
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
                    |publication| {
                        self.selection
                            .set(SelectionState::with_primary(next_selection));
                        receipt_for_publication(publication)
                    },
                )
            }
            WorkspaceIntent::EditTableCell {
                table,
                row_id,
                column_id,
                content,
            } => self
                .apply_workspace_edit(
                    |session| session.edit_table_cell(&table, &row_id, &column_id, content),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::AddTableRow {
                table,
                row_id,
                values,
            } => self
                .apply_workspace_edit(
                    |session| session.add_table_row(&table, row_id, values),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::DeleteTableRow { table, row_id } => self
                .apply_workspace_edit(
                    |session| session.delete_table_row(&table, &row_id),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::RenameTableRow {
                table,
                row_id,
                new_row_id,
            } => self
                .apply_workspace_edit(
                    |session| session.rename_table_row(&table, &row_id, new_row_id),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::ReorderTableRow {
                table,
                row_id,
                new_index,
            } => self
                .apply_workspace_edit(
                    |session| session.reorder_table_row(&table, &row_id, new_index),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::RenameTable { table, name } => self
                .apply_workspace_edit(
                    |session| session.rename_table(&table, name),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::AddTableColumn {
                table,
                column_id,
                name,
                values,
            } => self
                .apply_workspace_edit(
                    |session| session.add_table_column(&table, column_id, name, values),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::AddTableFormulaColumn {
                table,
                column_id,
                name,
                formula_text,
            } => self
                .apply_workspace_edit(
                    |session| {
                        session.add_table_formula_column(&table, column_id, name, formula_text)
                    },
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::EditTableColumnFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_edit(
                    |session| session.edit_table_column_formula(&table, &column_id, formula_text),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::SetTableTotalsFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_edit(
                    |session| session.set_table_totals_formula(&table, &column_id, formula_text),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::ClearTableTotalsFormula { table, column_id } => self
                .apply_workspace_edit(
                    |session| session.clear_table_totals_formula(&table, &column_id),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::SetTableHeaderRowVisible { table, visible } => self
                .apply_workspace_edit(
                    |session| session.set_table_header_row_visible(&table, visible),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::SetTableTotalsRowVisible { table, visible } => self
                .apply_workspace_edit(
                    |session| session.set_table_totals_row_visible(&table, visible),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::RenameTableColumn {
                table,
                column_id,
                name,
            } => self
                .apply_workspace_edit(
                    |session| session.rename_table_column(&table, &column_id, name),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::ReorderTableColumn {
                table,
                column_id,
                new_index,
            } => self
                .apply_workspace_edit(
                    |session| session.reorder_table_column(&table, &column_id, new_index),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::DeleteTableColumn { table, column_id } => self
                .apply_workspace_edit(
                    |session| session.delete_table_column(&table, &column_id),
                    WorkspaceEditPublication::Recalculate,
                )
                .map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
                ),
            WorkspaceIntent::NewWorkspace => self.create_workspace().map_or_else(
                |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                receipt_for_publication,
            ),
            WorkspaceIntent::SwitchWorkspace { workspace_id } => {
                self.switch_workspace(&workspace_id).map_or_else(
                    |error| IntentReceipt::rejected(IntentError::Rejected(error)),
                    receipt_for_publication,
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

#[derive(Debug, Clone)]
struct PublishedWorkspaceEdit<T> {
    result: T,
    delta: WorkspaceDelta,
    produced_revision: Option<String>,
}

impl<T> PublishedWorkspaceEdit<T> {
    fn with_result<U>(self, result: U) -> PublishedWorkspaceEdit<U> {
        PublishedWorkspaceEdit {
            result,
            delta: self.delta,
            produced_revision: self.produced_revision,
        }
    }
}

fn receipt_for_publication<T>(publication: PublishedWorkspaceEdit<T>) -> IntentReceipt {
    IntentReceipt::accepted()
        .with_delta(publication.delta)
        .with_produced_revision(publication.produced_revision)
}

impl HostDispatcher {
    fn apply_workspace_edit<T>(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<T, super::session::TreeWorkspaceSessionError>,
        publication: WorkspaceEditPublication,
    ) -> Result<PublishedWorkspaceEdit<T>, String> {
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
            let before = self.workspace.map(|workspace| workspace.get_untracked());
            let result = edit(&mut session).map_err(|error| error.to_string())?;
            if matches!(publication, WorkspaceEditPublication::Recalculate) {
                session.recalculate().map_err(|error| error.to_string())?;
            }
            let mut after = session
                .workspace_state()
                .map_err(|error| error.to_string())?;
            after.projection_seq = self.next_projection_seq.fetch_add(1, Ordering::Relaxed);
            let delta = workspace_delta(before.as_ref(), &after, false);
            let produced_revision = after.revision.workspace_revision_id.clone();
            if let Some(workspace) = self.workspace {
                workspace.set(after);
            }
            Ok(PublishedWorkspaceEdit {
                result,
                delta,
                produced_revision,
            })
        })
    }

    fn active_session_id(&self) -> Result<u64, String> {
        self.session_id
            .lock()
            .map_err(|_| "workspace session id mutex poisoned".to_string())?
            .ok_or_else(|| "workspace session handle is not attached".to_string())
    }

    fn current_projection_seq(&self) -> u64 {
        self.workspace
            .map(|workspace| workspace.get_untracked().projection_seq)
            .unwrap_or(0)
    }

    fn create_workspace(&self) -> Result<PublishedWorkspaceEdit<String>, String> {
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
        let publication = self.activate_session(&workspace_id, session_id, &session)?;
        Ok(publication.with_result(workspace_id))
    }

    fn switch_workspace(&self, workspace_id: &str) -> Result<PublishedWorkspaceEdit<()>, String> {
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
    ) -> Result<PublishedWorkspaceEdit<()>, String> {
        *self
            .session_id
            .lock()
            .map_err(|_| "workspace session id mutex poisoned".to_string())? = Some(session_id);
        let before = self.workspace.map(|workspace| workspace.get_untracked());
        let mut state = session
            .lock()
            .map_err(|_| "workspace session mutex poisoned".to_string())?
            .workspace_state()
            .map_err(|error| error.to_string())?;
        state.projection_seq = self.next_projection_seq.fetch_add(1, Ordering::Relaxed);
        let produced_revision = state.revision.workspace_revision_id.clone();
        let delta = workspace_delta(before.as_ref(), &state, true);
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
        Ok(PublishedWorkspaceEdit {
            result: (),
            delta,
            produced_revision,
        })
    }
}

fn workspace_delta(
    before: Option<&WorkspaceState>,
    after: &WorkspaceState,
    full_reset: bool,
) -> WorkspaceDelta {
    let from_seq = before.map_or(0, |state| state.projection_seq);
    let to_seq = after.projection_seq;
    let mut changes = Vec::new();
    if full_reset || before.is_none() {
        changes.push(WorkspaceDeltaChange::FullReset);
        return WorkspaceDelta {
            from_seq,
            to_seq,
            changes,
        };
    }

    let before = before.expect("checked above");
    let structural = structural_delta(before, after);
    if !structural.added.is_empty()
        || !structural.removed.is_empty()
        || !structural.changed.is_empty()
    {
        changes.push(WorkspaceDeltaChange::Structural(structural));
    }

    let changed_nodes = changed_node_keys(before, after);
    if !changed_nodes.is_empty() {
        changes.push(WorkspaceDeltaChange::NodesChanged(changed_nodes));
    }

    let value_changes = value_delta(before, after);
    if !value_changes.is_empty() {
        changes.push(WorkspaceDeltaChange::ValuesChanged(value_changes));
    }

    let dependency_changes = dependency_delta(after);
    if !dependency_changes.is_empty() {
        changes.push(WorkspaceDeltaChange::DepsChanged(dependency_changes));
    }

    if let Some(run) = after.last_run.clone() {
        changes.push(WorkspaceDeltaChange::CalcRun(run));
    }

    WorkspaceDelta {
        from_seq,
        to_seq,
        changes,
    }
}

fn structural_delta(before: &WorkspaceState, after: &WorkspaceState) -> StructuralDeltaProjection {
    let before_nodes = nodes_by_key(before);
    let after_nodes = nodes_by_key(after);
    let before_keys = before_nodes.keys().cloned().collect::<BTreeSet<_>>();
    let after_keys = after_nodes.keys().cloned().collect::<BTreeSet<_>>();
    let added = after_keys
        .difference(&before_keys)
        .cloned()
        .collect::<Vec<_>>();
    let removed = before_keys
        .difference(&after_keys)
        .cloned()
        .collect::<Vec<_>>();
    let changed = before_keys
        .intersection(&after_keys)
        .filter(|key| {
            let before = before_nodes.get(*key).expect("intersection key exists");
            let after = after_nodes.get(*key).expect("intersection key exists");
            before.id != after.id
                || before.display_name != after.display_name
                || before.parent != after.parent
                || before.children != after.children
                || before.depth != after.depth
                || before.content_kind != after.content_kind
                || before.content_text != after.content_text
                || before.is_meta != after.is_meta
                || before.table != after.table
        })
        .cloned()
        .collect::<Vec<_>>();
    StructuralDeltaProjection {
        added,
        removed,
        changed,
    }
}

fn changed_node_keys(before: &WorkspaceState, after: &WorkspaceState) -> Vec<NodeKey> {
    let before_nodes = nodes_by_key(before);
    nodes_by_key(after)
        .into_iter()
        .filter_map(|(key, after_node)| match before_nodes.get(&key) {
            Some(before_node) if *before_node == after_node => None,
            _ => Some(key),
        })
        .collect()
}

fn value_delta(before: &WorkspaceState, after: &WorkspaceState) -> Vec<NodeValueDeltaProjection> {
    let before_nodes = nodes_by_key(before);
    let invalidated = after
        .last_run
        .as_ref()
        .map(|run| {
            run.invalidated_nodes
                .iter()
                .map(|node| node.node_key.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    nodes_by_key(after)
        .into_iter()
        .filter_map(|(key, after_node)| {
            let before_value = before_nodes.get(&key).map(|node| &node.computed_value);
            let changed = before_value != Some(&after_node.computed_value);
            (changed || invalidated.contains(&key)).then(|| NodeValueDeltaProjection {
                node: key,
                value: after_node.computed_value.clone(),
            })
        })
        .collect()
}

fn dependency_delta(after: &WorkspaceState) -> Vec<DependencyDeltaProjection> {
    after
        .last_run
        .as_ref()
        .map(|run| {
            run.invalidated_nodes
                .iter()
                .filter_map(|node| {
                    after
                        .dependencies
                        .descriptors_by_owner
                        .get(&node.node)
                        .map(|descriptors| DependencyDeltaProjection {
                            owner: node.node_key.clone(),
                            kinds: descriptors
                                .iter()
                                .map(|descriptor| descriptor.kind.clone())
                                .collect(),
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn nodes_by_key(state: &WorkspaceState) -> BTreeMap<NodeKey, &NodeView> {
    state
        .nodes
        .values()
        .map(|node| (node.key.clone(), node))
        .collect()
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

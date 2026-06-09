use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    DependencyDeltaProjection, Dispatcher, InitialNodeContentProjection, IntentError,
    IntentReceipt, NodeId, NodeKey, NodeValueDeltaProjection, NodeView, SelectionState,
    SharedSkinStateHandle, StructuralDeltaProjection, TableCellSelection, WorkspaceDelta,
    WorkspaceDeltaChange, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use oxcalc_core::consumer::TransactionRecalcPolicy;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use super::session::{TreeWorkspaceSession, TreeWorkspaceSessionError};
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
            WorkspaceIntent::SelectTableCell {
                table,
                row_id,
                column_id,
            } => {
                let table_cell = TableCellSelection {
                    table: table.clone(),
                    row_id,
                    column_id,
                };
                if let Some(workspace) = self.workspace {
                    if !table_cell_exists(&workspace.get_untracked(), &table_cell) {
                        return IntentReceipt::rejected(IntentError::UnknownTableCell {
                            table: table_cell.table.to_string(),
                            row_id: table_cell.row_id.clone().unwrap_or_default(),
                            column_id: table_cell.column_id.clone(),
                        });
                    }
                }
                self.selection
                    .set(SelectionState::with_table_cell(table_cell));
                IntentReceipt::accepted()
                    .with_delta(WorkspaceDelta::unchanged(self.current_projection_seq()))
            }
            WorkspaceIntent::EditFormula { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::RecalculateAndPublishOnce,
                    )
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::EditContent { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::RecalculateAndPublishOnce,
                    )
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::EditContentDeferred { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::ApplyOnly,
                    )
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::EditScopedContent { scope, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_scoped_content_transaction(scope, content)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::Recalculate => self
                .apply_workspace_edit(|_| Ok(()), WorkspaceEditPublication::Recalculate)
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::AddNode {
                parent,
                symbol,
                initial,
                is_meta,
            } => match initial_node_content(&initial).and_then(|content| {
                self.apply_workspace_transaction_edit(|session| {
                    session.add_node_transaction_with_meta(
                        parent.as_ref(),
                        symbol,
                        content,
                        is_meta,
                    )
                })
            }) {
                Ok(publication) => {
                    let created = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(created)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => IntentReceipt::rejected(error),
            },
            WorkspaceIntent::RenameNode { node, new_symbol } => match self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_node_transaction(&node, new_symbol)
                }) {
                Ok(publication) => {
                    let renamed = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(renamed)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => IntentReceipt::rejected(error),
            },
            WorkspaceIntent::MoveNode {
                node,
                new_parent,
                new_index,
            } => match self.apply_workspace_transaction_edit(|session| {
                session.move_node_transaction(&node, new_parent.as_ref(), new_index)
            }) {
                Ok(publication) => {
                    let moved = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(moved)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => IntentReceipt::rejected(error),
            },
            WorkspaceIntent::ReorderNode { node, new_index } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_node_transaction(&node, new_index)
                })
                .map_or_else(IntentReceipt::rejected, |publication| {
                    self.selection.set(SelectionState::with_primary(Some(node)));
                    receipt_for_publication(publication)
                }),
            WorkspaceIntent::DeleteNode { node } => {
                let next_selection = parent_node_id(node.as_str());
                self.apply_workspace_transaction_edit(|session| {
                    session.delete_node_transaction(&node)
                })
                .map_or_else(IntentReceipt::rejected, |publication| {
                    self.selection
                        .set(SelectionState::with_primary(next_selection));
                    receipt_for_publication(publication)
                })
            }
            WorkspaceIntent::EditTableCell {
                table,
                row_id,
                column_id,
                content,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_table_cell_transaction(&table, &row_id, &column_id, content)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::AddTableRow {
                table,
                row_id,
                values,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.add_table_row_transaction(&table, row_id, values)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::DeleteTableRow { table, row_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.delete_table_row_transaction(&table, &row_id)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::RenameTableRow {
                table,
                row_id,
                new_row_id,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_row_transaction(&table, &row_id, new_row_id)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::ReorderTableRow {
                table,
                row_id,
                new_index,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_table_row_transaction(&table, &row_id, new_index)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::RenameTable { table, name } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_transaction(&table, name)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::AddTableColumn {
                table,
                column_id,
                name,
                values,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.add_table_column_transaction(&table, column_id, name, values)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::AddTableFormulaColumn {
                table,
                column_id,
                name,
                formula_text,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.add_table_formula_column_transaction(
                        &table,
                        column_id,
                        name,
                        formula_text,
                    )
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::EditTableColumnFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_table_column_formula_transaction(&table, &column_id, formula_text)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::SetTableTotalsFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_totals_formula_transaction(&table, &column_id, formula_text)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::ClearTableTotalsFormula { table, column_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.clear_table_totals_formula_transaction(&table, &column_id)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::SetTableHeaderRowVisible { table, visible } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_header_row_visible_transaction(&table, visible)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::SetTableTotalsRowVisible { table, visible } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_totals_row_visible_transaction(&table, visible)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::RenameTableColumn {
                table,
                column_id,
                name,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_column_transaction(&table, &column_id, name)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::ReorderTableColumn {
                table,
                column_id,
                new_index,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_table_column_transaction(&table, &column_id, new_index)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::DeleteTableColumn { table, column_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.delete_table_column_transaction(&table, &column_id)
                })
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::NewWorkspace => self
                .create_workspace()
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            WorkspaceIntent::SwitchWorkspace { workspace_id } => self
                .switch_workspace(&workspace_id)
                .map_or_else(IntentReceipt::rejected, receipt_for_publication),
            // The framework's WorkspaceIntent is intentionally
            // `#[non_exhaustive]` so adding a variant in a future bead is
            // an additive change. A variant that reaches this branch is
            // one this dispatcher version does not know — reject loudly
            // rather than silently ignore.
            _ => IntentReceipt::rejected(IntentError::Unsupported),
        }
    }
}

fn table_cell_exists(workspace: &WorkspaceState, selection: &TableCellSelection) -> bool {
    let Some(table) = workspace.tables.get(&selection.table) else {
        return false;
    };
    let Some(cells) = table.cells.as_ref() else {
        return false;
    };
    let column_index = table
        .columns
        .iter()
        .position(|column| column.column_id == selection.column_id);
    let Some(column_index) = column_index else {
        return false;
    };
    match selection.row_id.as_ref() {
        Some(row_id) => cells.body_rows.iter().any(|row| {
            row.get(column_index)
                .and_then(Option::as_ref)
                .and_then(|cell| cell.row_id.as_ref())
                == Some(row_id)
        }),
        None => table.totals_row_present && cells.totals_row.get(column_index).is_some(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceEditPublication {
    Recalculate,
}

#[derive(Debug, Clone)]
struct PublishedWorkspaceEdit<T> {
    result: T,
    delta: WorkspaceDelta,
    produced_revision: Option<String>,
    transaction_id: Option<String>,
}

impl<T> PublishedWorkspaceEdit<T> {
    fn with_result<U>(self, result: U) -> PublishedWorkspaceEdit<U> {
        PublishedWorkspaceEdit {
            result,
            delta: self.delta,
            produced_revision: self.produced_revision,
            transaction_id: self.transaction_id,
        }
    }
}

fn receipt_for_publication<T>(publication: PublishedWorkspaceEdit<T>) -> IntentReceipt {
    IntentReceipt::accepted()
        .with_delta(publication.delta)
        .with_produced_revision(publication.produced_revision)
        .with_transaction_id(publication.transaction_id)
}

impl HostDispatcher {
    fn apply_workspace_edit<T>(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<T, super::session::TreeWorkspaceSessionError>,
        publication: WorkspaceEditPublication,
    ) -> Result<PublishedWorkspaceEdit<T>, IntentError> {
        let session_id = self.active_session_id()?;
        HOST_SESSIONS.with(|sessions| {
            let session = sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| host_failure("workspace session handle is not available"))?;
            let mut session = session
                .lock()
                .map_err(|_| host_failure("workspace session mutex poisoned"))?;
            let before = self.workspace.map(|workspace| workspace.get_untracked());
            let result = edit(&mut session).map_err(intent_error_from_session)?;
            if matches!(publication, WorkspaceEditPublication::Recalculate) {
                session.recalculate().map_err(intent_error_from_session)?;
            }
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
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
                transaction_id: None,
            })
        })
    }

    fn apply_workspace_transaction_edit<T>(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<
            super::session::TreeWorkspaceTransactionEdit<T>,
            TreeWorkspaceSessionError,
        >,
    ) -> Result<PublishedWorkspaceEdit<T>, IntentError> {
        let session_id = self.active_session_id()?;
        HOST_SESSIONS.with(|sessions| {
            let session = sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| host_failure("workspace session handle is not available"))?;
            let mut session = session
                .lock()
                .map_err(|_| host_failure("workspace session mutex poisoned"))?;
            let before = self.workspace.map(|workspace| workspace.get_untracked());
            let transaction = edit(&mut session).map_err(intent_error_from_session)?;
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.projection_seq = self.next_projection_seq.fetch_add(1, Ordering::Relaxed);
            let delta = workspace_delta(before.as_ref(), &after, false);
            let produced_revision = after.revision.workspace_revision_id.clone();
            if let Some(workspace) = self.workspace {
                workspace.set(after);
            }
            Ok(PublishedWorkspaceEdit {
                result: transaction.result,
                delta,
                produced_revision,
                transaction_id: Some(transaction.transaction_id),
            })
        })
    }

    fn active_session_id(&self) -> Result<u64, IntentError> {
        self.session_id
            .lock()
            .map_err(|_| host_failure("workspace session id mutex poisoned"))?
            .ok_or_else(|| host_failure("workspace session handle is not attached"))
    }

    fn current_projection_seq(&self) -> u64 {
        self.workspace
            .map(|workspace| workspace.get_untracked().projection_seq)
            .unwrap_or(0)
    }

    fn create_workspace(&self) -> Result<PublishedWorkspaceEdit<String>, IntentError> {
        let ordinal = self.next_workspace_ordinal.fetch_add(1, Ordering::Relaxed);
        let workspace_id = format!("Workspace {ordinal}");
        let session = Arc::new(Mutex::new(
            empty_workspace_session(&workspace_id).map_err(IntentError::HostFailure)?,
        ));
        let session_id = NEXT_HOST_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        HOST_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(session_id, session.clone());
        });
        self.workspace_sessions
            .lock()
            .map_err(|_| host_failure("workspace catalog mutex poisoned"))?
            .insert(workspace_id.clone(), session_id);
        let publication = self.activate_session(&workspace_id, session_id, &session)?;
        Ok(publication.with_result(workspace_id))
    }

    fn switch_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<PublishedWorkspaceEdit<()>, IntentError> {
        let session_id = self
            .workspace_sessions
            .lock()
            .map_err(|_| host_failure("workspace catalog mutex poisoned"))?
            .get(workspace_id)
            .copied()
            .ok_or_else(|| {
                IntentError::HostFailure(format!("unknown workspace '{workspace_id}'"))
            })?;
        let session = HOST_SESSIONS.with(|sessions| {
            sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| host_failure("workspace session handle is not available"))
        })?;
        self.activate_session(workspace_id, session_id, &session)
    }

    fn activate_session(
        &self,
        workspace_id: &str,
        session_id: u64,
        session: &Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Result<PublishedWorkspaceEdit<()>, IntentError> {
        *self
            .session_id
            .lock()
            .map_err(|_| host_failure("workspace session id mutex poisoned"))? = Some(session_id);
        let before = self.workspace.map(|workspace| workspace.get_untracked());
        let mut state = session
            .lock()
            .map_err(|_| host_failure("workspace session mutex poisoned"))?
            .workspace_state()
            .map_err(intent_error_from_session)?;
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
                .map_err(|_| host_failure("workspace catalog mutex poisoned"))?
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
            transaction_id: None,
        })
    }
}

fn host_failure(message: impl Into<String>) -> IntentError {
    IntentError::HostFailure(message.into())
}

fn initial_node_content(initial: &InitialNodeContentProjection) -> Result<String, IntentError> {
    if let Some(content) = initial.supported_content() {
        Ok(content.to_string())
    } else {
        Err(IntentError::UnsupportedInitialContent {
            policy: initial.stable_id().to_string(),
        })
    }
}

fn intent_error_from_session(error: TreeWorkspaceSessionError) -> IntentError {
    match error {
        TreeWorkspaceSessionError::UnknownNodePath { node } => IntentError::UnknownNode { node },
        TreeWorkspaceSessionError::DuplicateNodePath { node } => {
            IntentError::DuplicateNode { node }
        }
        TreeWorkspaceSessionError::UnknownTable { table } => IntentError::UnknownTable { table },
        TreeWorkspaceSessionError::EmptyTableName { table } => {
            IntentError::EmptyTableName { table }
        }
        TreeWorkspaceSessionError::DuplicateTableName { table, name } => {
            IntentError::DuplicateTableName { table, name }
        }
        TreeWorkspaceSessionError::DuplicateTableRow { table, row_id } => {
            IntentError::DuplicateTableRow { table, row_id }
        }
        TreeWorkspaceSessionError::DuplicateTableColumn { table, column_id } => {
            IntentError::DuplicateTableColumn { table, column_id }
        }
        TreeWorkspaceSessionError::UnknownTableRow { table, row_id } => {
            IntentError::UnknownTableRow { table, row_id }
        }
        TreeWorkspaceSessionError::UnknownTableColumn { table, column_id } => {
            IntentError::UnknownTableColumn { table, column_id }
        }
        TreeWorkspaceSessionError::DuplicateTableCellInput { table, column_id } => {
            IntentError::DuplicateTableCellInput { table, column_id }
        }
        TreeWorkspaceSessionError::DuplicateTableRowInput { table, row_id } => {
            IntentError::DuplicateTableRowInput { table, row_id }
        }
        TreeWorkspaceSessionError::UnknownTableCell {
            table,
            row_id,
            column_id,
        } => IntentError::UnknownTableCell {
            table,
            row_id,
            column_id,
        },
        TreeWorkspaceSessionError::FormulaTableCellEdit { table, column_id } => {
            IntentError::FormulaTableCellEdit { table, column_id }
        }
        TreeWorkspaceSessionError::ConstantTableColumnFormulaEdit { table, column_id } => {
            IntentError::ConstantTableColumnFormulaEdit { table, column_id }
        }
        TreeWorkspaceSessionError::ProjectionOutOfSync { node } => {
            IntentError::ProjectionOutOfSync { node }
        }
        TreeWorkspaceSessionError::OxCalc(error) => IntentError::EngineRejected(error.to_string()),
        TreeWorkspaceSessionError::UnsupportedDocumentSchema { schema_version } => {
            IntentError::HostFailure(format!(
                "unsupported .dnatree document schema {schema_version}"
            ))
        }
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
                        .descriptors_by_owner_key
                        .get(&node.node_key)
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

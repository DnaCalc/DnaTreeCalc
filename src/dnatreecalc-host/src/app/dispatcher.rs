use std::sync::{Arc, Mutex};

use dnatreecalc_skin_framework::{
    AuthoringScope, CandidateProjection, ClipboardNodeFormatProjection,
    ClipboardNodeValueProjection, ClipboardOperationProjection, ClipboardPayloadKind,
    ClipboardPayloadProjection, ClipboardProjection, DependencyDeltaProjection, Dispatcher,
    FormulaBindPreviewProjection, GridProjection, IntentError, IntentReceipt, IntentRecord,
    MutationImpactIntentProjection, MutationImpactProjection, NodeContentKind, NodeId, NodeKey,
    NodeValueDeltaProjection, NodeValueProjection, NodeView, Persona, PreviewError, PreviewService,
    SelectionState, SharedSkinStateHandle, SharedStateChange, SharedStateOrigin,
    StructuralDeltaProjection, TableCellSelection, WorkspaceDelta, WorkspaceDeltaChange,
    WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use oxcalc_core::consumer::TransactionRecalcPolicy;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use super::persistence::{WorkspaceDocumentStore, persist_workspace_sessions};
use super::session::{TreeWorkspaceSession, TreeWorkspaceSessionError, node_key_for_tree_node};
use crate::model::{WorkspaceFixture, WorkspaceModel};

thread_local! {
    static HOST_SESSIONS: RefCell<BTreeMap<u64, Arc<Mutex<TreeWorkspaceSession>>>> =
        const { RefCell::new(BTreeMap::new()) };
}

pub(crate) fn with_host_sessions<R>(
    f: impl FnOnce(&BTreeMap<u64, Arc<Mutex<TreeWorkspaceSession>>>) -> R,
) -> R {
    HOST_SESSIONS.with(|sessions| f(&sessions.borrow()))
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
    latest_delta: Option<RwSignal<WorkspaceDelta>>,
    shared: Option<SharedSkinStateHandle>,
    workspace_document_store: Option<Arc<dyn WorkspaceDocumentStore>>,
    session_id: Mutex<Option<u64>>,
    workspace_sessions: Mutex<BTreeMap<String, u64>>,
    next_workspace_ordinal: AtomicU64,
    next_projection_seq: AtomicU64,
    undo_stack: Mutex<Vec<RevisionCursorEntry>>,
    redo_stack: Mutex<Vec<RevisionCursorEntry>>,
    records: Mutex<Vec<IntentRecord>>,
    persona: Mutex<Persona>,
}

#[derive(Debug, Clone)]
struct RevisionCursorEntry {
    revision_id: String,
    selection: SelectionState,
}

impl HostDispatcher {
    #[must_use]
    pub fn new(selection: RwSignal<SelectionState>) -> Self {
        Self {
            selection,
            workspace: None,
            latest_delta: None,
            shared: None,
            workspace_document_store: None,
            session_id: Mutex::new(None),
            workspace_sessions: Mutex::new(BTreeMap::new()),
            next_workspace_ordinal: AtomicU64::new(1),
            next_projection_seq: AtomicU64::new(1),
            undo_stack: Mutex::new(Vec::new()),
            redo_stack: Mutex::new(Vec::new()),
            records: Mutex::new(Vec::new()),
            persona: Mutex::new(Persona::default()),
        }
    }

    #[must_use]
    pub fn with_session(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
    ) -> Self {
        Self::with_session_and_shared(selection, workspace, latest_delta, session, None)
    }

    #[must_use]
    pub fn with_session_and_shared(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
        shared: Option<SharedSkinStateHandle>,
    ) -> Self {
        Self::with_session_shared_and_workspace_store(
            selection,
            workspace,
            latest_delta,
            session,
            shared,
            None,
        )
    }

    #[must_use]
    pub fn with_session_shared_and_workspace_store(
        selection: RwSignal<SelectionState>,
        workspace: RwSignal<WorkspaceState>,
        latest_delta: RwSignal<WorkspaceDelta>,
        session: Arc<Mutex<TreeWorkspaceSession>>,
        shared: Option<SharedSkinStateHandle>,
        workspace_document_store: Option<Arc<dyn WorkspaceDocumentStore>>,
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
            shared.apply(
                SharedStateChange::SetWorkspaceIds(vec![workspace_id.clone()]),
                SharedStateOrigin::Host,
            );
            shared.apply(
                SharedStateChange::SetActiveWorkspaceId(Some(workspace_id.clone())),
                SharedStateOrigin::Host,
            );
        }
        let dispatcher = Self {
            selection,
            workspace: Some(workspace),
            latest_delta: Some(latest_delta),
            shared,
            workspace_document_store,
            session_id: Mutex::new(Some(session_id)),
            workspace_sessions: Mutex::new(workspace_sessions),
            next_workspace_ordinal: AtomicU64::new(1),
            next_projection_seq: AtomicU64::new(1),
            undo_stack: Mutex::new(Vec::new()),
            redo_stack: Mutex::new(Vec::new()),
            records: Mutex::new(Vec::new()),
            persona: Mutex::new(Persona::default()),
        };
        dispatcher.hydrate_workspace_sessions_from_document_store(&workspace_id);
        dispatcher
    }

    /// Snapshot of intents dispatched since construction. Tests use this
    /// to assert routing behavior without observing reactive state from
    /// the outside.
    pub fn intents(&self) -> Vec<WorkspaceIntent> {
        self.records
            .lock()
            .expect("dispatcher record log poisoned")
            .iter()
            .map(|record| record.intent.clone())
            .collect()
    }

    /// The full audited intent log (tenet 9): every dispatch with its
    /// outcome, persona, and the value epoch it left behind. Exportable
    /// (serde) and replayable via `dnatreecalc_skin_framework::replay`.
    pub fn intent_records(&self) -> Vec<IntentRecord> {
        self.records
            .lock()
            .expect("dispatcher record log poisoned")
            .clone()
    }

    pub fn clear_log(&self) {
        self.records
            .lock()
            .expect("dispatcher record log poisoned")
            .clear();
    }
}

impl Dispatcher for HostDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        let recorded_intent = intent.clone();
        let persona = *self.persona.lock().expect("persona mutex poisoned");
        let receipt = self.dispatch_inner(intent);
        // Record AFTER execution so the record carries the true outcome and
        // the value epoch the intent left behind — every return path of
        // dispatch_inner is captured here.
        let value_epoch = self
            .workspace
            .map(|workspace| workspace.get_untracked().revision.value_epoch)
            .unwrap_or(0);
        let mut records = self.records.lock().expect("dispatcher record log poisoned");
        let seq = records.len() as u64;
        records.push(IntentRecord {
            seq,
            intent: recorded_intent,
            accepted: receipt.accepted,
            error: receipt.error.clone(),
            transaction_id: receipt.transaction_id.clone(),
            produced_revision: receipt.produced_revision.clone(),
            value_epoch,
            persona,
        });
        receipt
    }
}

impl HostDispatcher {
    fn dispatch_inner(&self, intent: WorkspaceIntent) -> IntentReceipt {
        // Governance chokepoint (tenet 9): the persona policy gates every
        // intent BEFORE any host or engine work. Persona switching itself is
        // always dispatchable in this first slice (audited via the log, the
        // receipt, and the shared-state audit ring).
        if !matches!(intent, WorkspaceIntent::SetPersona { .. }) {
            let persona = *self.persona.lock().expect("persona mutex poisoned");
            if !persona.allows(&intent) {
                return self.reject_current(IntentError::Forbidden {
                    persona: persona.stable_id().to_string(),
                });
            }
        }
        let receipt = match intent {
            WorkspaceIntent::SetPersona { persona } => {
                *self.persona.lock().expect("persona mutex poisoned") = persona;
                if let Some(shared) = self.shared {
                    shared.apply(
                        SharedStateChange::SetPersona(persona),
                        SharedStateOrigin::Host,
                    );
                }
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            WorkspaceIntent::SelectNode(target) => {
                self.selection.set(SelectionState::with_primary(target));
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            WorkspaceIntent::SelectNodes { keys, anchor } => {
                // Validate the population against the live projection before
                // accepting — a stale key is a typed rejection, not a silent
                // filter.
                if let Some(workspace) = self.workspace {
                    let state = workspace.get_untracked();
                    if let Some(unknown) = keys
                        .iter()
                        .chain(anchor.iter())
                        .find(|key| state.node_by_key(key).is_none())
                    {
                        return self.reject_current(IntentError::UnknownNode {
                            node: unknown.to_string(),
                        });
                    }
                }
                if let Some(shared) = self.shared {
                    shared.apply(
                        SharedStateChange::SetSelectionSet(keys),
                        SharedStateOrigin::Host,
                    );
                    shared.apply(
                        SharedStateChange::SetSelectionAnchor(anchor),
                        SharedStateOrigin::Host,
                    );
                }
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
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
                        return self.reject_current(IntentError::UnknownTableCell {
                            table: table_cell.table.to_string(),
                            row_id: table_cell.row_id.clone().unwrap_or_default(),
                            column_id: table_cell.column_id.clone(),
                        });
                    }
                }
                self.selection
                    .set(SelectionState::with_table_cell(table_cell));
                IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
            }
            WorkspaceIntent::SetGridInterest {
                grid,
                top_row,
                left_col,
                bottom_row,
                right_col,
            } => self.apply_grid_interest(&grid, top_row, left_col, bottom_row, right_col),
            WorkspaceIntent::EditFormula { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::RecalculateAndPublishOnce,
                    )
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::EditContent { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::RecalculateAndPublishOnce,
                    )
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::EditContentDeferred { node, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_formula_transaction(
                        &node,
                        content,
                        TransactionRecalcPolicy::ApplyOnly,
                    )
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::EditScopedContent { scope, content } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_scoped_content_transaction(scope, content)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetNumberFormat {
                scope,
                number_format_code,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_number_format_transaction(scope, number_format_code)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetNote { node, note } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_note_transaction(node, note)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetMeta { node, is_meta } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_meta_transaction(node, is_meta)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetNodeAttributes { node, attrs } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_node_attributes_transaction(node, attrs)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::CopyToClipboard { scope, payload } => self
                .populate_clipboard(scope, payload, ClipboardOperationProjection::Copy)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::CutToClipboard { scope, payload } => self
                .populate_clipboard(scope, payload, ClipboardOperationProjection::Cut)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::PasteClipboardFormat { target } => self
                .paste_clipboard_format(target)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::PasteClipboardValues { target } => self
                .paste_clipboard_values(target)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::PasteExternalClipboardText { target, text } => self
                .paste_external_clipboard_text(target, text)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::DuplicateSubtree {
                source,
                destination_parent,
                new_symbol,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.duplicate_subtree_transaction(
                        source,
                        destination_parent.as_ref(),
                        new_symbol,
                    )
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::InsertFormulaReference {
                node,
                current_formula_text,
                replacement_start,
                replacement_len,
                target,
            } => match self.apply_workspace_transaction_edit(|session| {
                session.insert_formula_reference_transaction(
                    node,
                    current_formula_text,
                    replacement_start,
                    replacement_len,
                    target,
                )
            }) {
                Ok(publication) => receipt_for_formula_reference_insertion(publication),
                Err(error) => self.reject_current(error),
            },
            WorkspaceIntent::OpenCandidate { parent } => self
                .apply_candidate_projection_edit(|session| {
                    session.open_candidate(parent.as_deref())
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::EditCandidateContent {
                handle,
                node,
                content,
            } => self
                .apply_candidate_projection_edit(|session| {
                    session.edit_candidate_content(&handle, &node, content)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::RenameCandidateNode {
                handle,
                node,
                new_symbol,
            } => self
                .apply_candidate_projection_edit(|session| {
                    session.rename_candidate_node(&handle, &node, new_symbol)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::MoveCandidateNode {
                handle,
                node,
                new_parent,
                new_index,
            } => self
                .apply_candidate_projection_edit(|session| {
                    session.move_candidate_node(&handle, &node, new_parent.as_ref(), new_index)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::ReorderCandidateNode {
                handle,
                node,
                new_index,
            } => self
                .apply_candidate_projection_edit(|session| {
                    session.reorder_candidate_node(&handle, &node, new_index)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::DeleteCandidateNode { handle, node } => self
                .apply_candidate_projection_edit(|session| {
                    session.delete_candidate_node(&handle, &node)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::AddCandidateNode {
                handle,
                parent,
                symbol,
                initial,
                is_meta,
            } => self
                .apply_candidate_projection_edit(|session| {
                    session.add_candidate_node(&handle, parent.as_ref(), symbol, initial, is_meta)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::EvaluateCandidate { handle } => self
                .apply_candidate_projection_edit(|session| session.evaluate_candidate(&handle))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::RebaseCandidate { handle } => self
                .apply_candidate_projection_edit(|session| session.rebase_candidate(&handle))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::DiscardCandidate { handle } => self
                .discard_candidate(&handle)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::PinCandidateRetention { handle } => self
                .apply_candidate_projection_edit(|session| session.pin_candidate_retention(&handle))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::UnpinCandidateRetention { handle } => self
                .apply_candidate_projection_edit(|session| {
                    session.unpin_candidate_retention(&handle)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_candidate_change,
                ),
            WorkspaceIntent::ReapCandidates { max_retained } => self
                .reap_candidates(max_retained)
                .unwrap_or_else(|error| self.reject_current(error)),
            WorkspaceIntent::CommitCandidate { handle } => self
                .commit_candidate(&handle)
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::CreateScenarioFromCandidate {
                scenario_id,
                name,
                candidate_handle,
            } => self
                .apply_projection_edit(|session| {
                    session.create_scenario_from_candidate(scenario_id, name, &candidate_handle)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::CreateScenario {
                scenario_id,
                name,
                base_scenario_id,
            } => self
                .apply_projection_edit(|session| {
                    session.create_scenario(scenario_id, name, base_scenario_id)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::ActivateScenario { scenario_id } => self
                .apply_projection_edit(|session| session.activate_scenario(scenario_id.as_deref()))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::DeleteScenario { scenario_id } => self
                .apply_projection_edit(|session| session.delete_scenario(&scenario_id))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::SetScenarioOverride {
                scenario_id,
                node,
                value,
            } => self
                .apply_projection_edit(|session| {
                    session.set_scenario_override(&scenario_id, &node, &value)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::ClearScenarioOverride { scenario_id, node } => self
                .apply_projection_edit(|session| {
                    session.clear_scenario_override(&scenario_id, &node)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::CreateScenarioSweep {
                sweep_id,
                name,
                base_scenario_id,
                input_node,
                points,
            } => self
                .apply_projection_edit(|session| {
                    session.create_scenario_sweep(
                        sweep_id,
                        name,
                        base_scenario_id,
                        input_node,
                        points,
                    )
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::ActivateSweep { sweep_id } => self
                .apply_projection_edit(|session| session.activate_sweep(sweep_id.as_deref()))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::DeleteSweep { sweep_id } => self
                .apply_projection_edit(|session| session.delete_sweep(&sweep_id))
                .map_or_else(
                    |error| self.reject_current(error),
                    receipt_for_projection_change,
                ),
            WorkspaceIntent::Recalculate => self
                .apply_workspace_edit(|_| Ok(()), WorkspaceEditPublication::Recalculate)
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::AddNode {
                parent,
                symbol,
                initial,
                is_meta,
            } => match self.apply_workspace_transaction_edit(|session| {
                session.add_node_transaction_with_initial(parent.as_ref(), symbol, initial, is_meta)
            }) {
                Ok(publication) => {
                    let created = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(created)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => self.reject_current(error),
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
                Err(error) => self.reject_current(error),
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
                Err(error) => self.reject_current(error),
            },
            WorkspaceIntent::ReorderNode { node, new_index } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_node_transaction(&node, new_index)
                })
                .map_or_else(
                    |error| self.reject_current(error),
                    |publication| {
                        self.selection.set(SelectionState::with_primary(Some(node)));
                        receipt_for_publication(publication)
                    },
                ),
            WorkspaceIntent::DeleteNode { node } => {
                let next_selection = parent_node_id(node.as_str());
                self.apply_workspace_transaction_edit(|session| {
                    session.delete_node_transaction(&node)
                })
                .map_or_else(
                    |error| self.reject_current(error),
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
                .apply_workspace_transaction_edit(|session| {
                    session.edit_table_cell_transaction(&table, &row_id, &column_id, content)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::CreateTable { parent, symbol } => match self
                .apply_workspace_transaction_edit(|session| {
                    session.create_table_transaction(parent.as_ref(), symbol)
                }) {
                Ok(publication) => {
                    let created = publication.result.clone();
                    self.selection
                        .set(SelectionState::with_primary(Some(created)));
                    receipt_for_publication(publication.with_result(()))
                }
                Err(error) => self.reject_current(error),
            },
            WorkspaceIntent::AddTableRow {
                table,
                row_id,
                values,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.add_table_row_transaction(&table, row_id, values)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::DeleteTableRow { table, row_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.delete_table_row_transaction(&table, &row_id)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::RenameTableRow {
                table,
                row_id,
                new_row_id,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_row_transaction(&table, &row_id, new_row_id)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::ReorderTableRow {
                table,
                row_id,
                new_index,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_table_row_transaction(&table, &row_id, new_index)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::RenameTable { table, name } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_transaction(&table, name)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::AddTableColumn {
                table,
                column_id,
                name,
                values,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.add_table_column_transaction(&table, column_id, name, values)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
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
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::EditTableColumnFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.edit_table_column_formula_transaction(&table, &column_id, formula_text)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetTableTotalsFormula {
                table,
                column_id,
                formula_text,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_totals_formula_transaction(&table, &column_id, formula_text)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::ClearTableTotalsFormula { table, column_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.clear_table_totals_formula_transaction(&table, &column_id)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetTableHeaderRowVisible { table, visible } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_header_row_visible_transaction(&table, visible)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SetTableTotalsRowVisible { table, visible } => self
                .apply_workspace_transaction_edit(|session| {
                    session.set_table_totals_row_visible_transaction(&table, visible)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::RenameTableColumn {
                table,
                column_id,
                name,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.rename_table_column_transaction(&table, &column_id, name)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::ReorderTableColumn {
                table,
                column_id,
                new_index,
            } => self
                .apply_workspace_transaction_edit(|session| {
                    session.reorder_table_column_transaction(&table, &column_id, new_index)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::DeleteTableColumn { table, column_id } => self
                .apply_workspace_transaction_edit(|session| {
                    session.delete_table_column_transaction(&table, &column_id)
                })
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::NavigateRevision { revision_id } => self
                .apply_workspace_edit(
                    |session| session.navigate_workspace_revision(&revision_id),
                    WorkspaceEditPublication::ProjectOnly,
                )
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::Undo => self
                .undo_revision()
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::Redo => self
                .redo_revision()
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::NewWorkspace => self
                .create_workspace()
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::SwitchWorkspace { workspace_id } => self
                .switch_workspace(&workspace_id)
                .map_or_else(|error| self.reject_current(error), receipt_for_publication),
            WorkspaceIntent::RenameWorkspace {
                workspace_id,
                new_name,
            } => self.rename_workspace(&workspace_id, &new_name),
            // The framework's WorkspaceIntent is intentionally
            // `#[non_exhaustive]` so adding a variant in a future bead is
            // an additive change. A variant that reaches this branch is
            // one this dispatcher version does not know — reject loudly
            // rather than silently ignore.
            _ => self.reject_current(IntentError::Unsupported),
        };
        if receipt.accepted {
            if let Err(error) = self.persist_active_workspace_document() {
                return self.reject_current(error);
            }
        }
        receipt
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
    ProjectOnly,
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

fn receipt_for_formula_reference_insertion(
    publication: PublishedWorkspaceEdit<
        dnatreecalc_skin_framework::FormulaReferenceInsertionProjection,
    >,
) -> IntentReceipt {
    let mut delta = publication.delta;
    delta
        .changes
        .push(WorkspaceDeltaChange::FormulaReferenceInserted(
            publication.result,
        ));
    IntentReceipt::accepted()
        .with_delta(delta)
        .with_produced_revision(publication.produced_revision)
        .with_transaction_id(publication.transaction_id)
}

fn receipt_for_candidate_change(
    publication: PublishedWorkspaceEdit<CandidateProjection>,
) -> IntentReceipt {
    IntentReceipt::accepted()
        .with_delta(publication.delta)
        .with_produced_revision(publication.produced_revision)
        .with_transaction_id(publication.transaction_id)
}

fn receipt_for_projection_change<T>(publication: PublishedWorkspaceEdit<T>) -> IntentReceipt {
    IntentReceipt::accepted()
        .with_delta(publication.delta)
        .with_produced_revision(publication.produced_revision)
        .with_transaction_id(publication.transaction_id)
}

impl HostDispatcher {
    fn populate_clipboard(
        &self,
        scope: AuthoringScope,
        payload: ClipboardPayloadKind,
        operation: ClipboardOperationProjection,
    ) -> Result<IntentReceipt, IntentError> {
        let workspace = self
            .workspace
            .ok_or_else(|| host_failure("workspace projection handle is not attached"))?;
        let before = workspace.get_untracked();
        let clipboard = clipboard_from_projection(&before, &scope, payload, operation)?;
        let mut after = before.clone();
        after.clipboard = Some(clipboard);
        let (_, delta) = self.publish_projection_state(Some(&before), after, false);
        Ok(IntentReceipt::accepted().with_delta(delta))
    }

    fn paste_clipboard_format(&self, target: AuthoringScope) -> Result<IntentReceipt, IntentError> {
        let number_format_code = self
            .workspace
            .ok_or_else(|| host_failure("workspace projection handle is not attached"))
            .and_then(|workspace| clipboard_number_format_code(&workspace.get_untracked()))?;
        match self.apply_workspace_transaction_edit(|session| {
            session.set_number_format_transaction(target, number_format_code)
        }) {
            Ok(publication) => Ok(receipt_for_publication(publication)),
            Err(error) => Ok(self.reject_current(error)),
        }
    }

    fn paste_clipboard_values(&self, target: AuthoringScope) -> Result<IntentReceipt, IntentError> {
        let before = self
            .workspace
            .ok_or_else(|| host_failure("workspace projection handle is not attached"))
            .map(|workspace| workspace.get_untracked())?;
        let targets = before.expand_authoring_scope(&target).map_err(|error| {
            clipboard_scope_error(ClipboardPayloadKind::Values, error.to_string())
        })?;
        if targets.is_empty() {
            return Err(clipboard_scope_error(
                ClipboardPayloadKind::Values,
                "value paste requires at least one target",
            ));
        }
        let payload = clipboard_literal_value_payload(&before)?;
        if payload.items.len() > 1
            && (!matches!(&target, AuthoringScope::Nodes(_))
                || payload.items.len() != targets.len())
        {
            return Err(clipboard_payload_mismatch(
                "ordered_literal_values",
                format!(
                    "value_count={},target_count={}",
                    payload.items.len(),
                    targets.len()
                ),
            ));
        }
        match self.apply_literal_value_paste_transaction(target, payload) {
            Ok(publication) => Ok(receipt_for_publication(publication)),
            Err(error) => Ok(self.reject_current(error)),
        }
    }

    fn paste_external_clipboard_text(
        &self,
        target: AuthoringScope,
        text: String,
    ) -> Result<IntentReceipt, IntentError> {
        let cells = external_clipboard_text_cells(&text);
        let before = self
            .workspace
            .ok_or_else(|| host_failure("workspace projection handle is not attached"))
            .map(|workspace| workspace.get_untracked())?;
        let targets = before.expand_authoring_scope(&target).map_err(|error| {
            clipboard_scope_error(ClipboardPayloadKind::Values, error.to_string())
        })?;
        if targets.is_empty() {
            return Err(clipboard_scope_error(
                ClipboardPayloadKind::Values,
                "external clipboard paste requires at least one target",
            ));
        }
        let paste_ordered_items =
            cells.len() > 1 && targets.len() > 1 && matches!(&target, AuthoringScope::Nodes(_));
        if paste_ordered_items && cells.len() != targets.len() {
            return Err(clipboard_scope_error(
                ClipboardPayloadKind::Values,
                format!(
                    "external clipboard paste item count {} does not match target count {}",
                    cells.len(),
                    targets.len()
                ),
            ));
        }

        let publication = if paste_ordered_items {
            self.apply_workspace_transaction_edit(|session| {
                session.edit_ordered_content_transaction(target, cells)
            })
        } else {
            self.apply_workspace_transaction_edit(|session| {
                session.edit_scoped_content_transaction(target, text)
            })
        };
        match publication {
            Ok(publication) => Ok(receipt_for_publication(publication)),
            Err(error) => Ok(self.reject_current(error)),
        }
    }

    fn apply_literal_value_paste_transaction(
        &self,
        target: AuthoringScope,
        payload: ClipboardLiteralValuePayload,
    ) -> Result<PublishedWorkspaceEdit<Vec<NodeKey>>, IntentError> {
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
            let clear_clipboard_after = payload.operation == ClipboardOperationProjection::Cut;
            let items = payload
                .items
                .into_iter()
                .map(|item| (item.source, item.content))
                .collect();
            let transaction = session
                .paste_constant_values_transaction(target, items, clear_clipboard_after)
                .map_err(intent_error_from_session)?;
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = if clear_clipboard_after {
                None
            } else {
                before.as_ref().and_then(|state| state.clipboard.clone())
            };
            let (after, delta) = self.publish_projection_state(before.as_ref(), after, false);
            let produced_revision = after.revision.workspace_revision_id.clone();
            Ok(PublishedWorkspaceEdit {
                result: transaction.result,
                delta,
                produced_revision,
                transaction_id: Some(transaction.transaction_id),
            })
        })
    }

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
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (after, delta) = self.publish_projection_state(before.as_ref(), after, false);
            let produced_revision = match before
                .as_ref()
                .and_then(|state| state.revision.workspace_revision_id.as_deref())
            {
                Some(before_revision)
                    if after.revision.workspace_revision_id.as_deref() == Some(before_revision) =>
                {
                    None
                }
                _ => after.revision.workspace_revision_id.clone(),
            };
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
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (after, delta) = self.publish_projection_state(before.as_ref(), after, false);
            let produced_revision = after.revision.workspace_revision_id.clone();
            self.record_revision_undo_boundary(before.as_ref(), &after)?;
            Ok(PublishedWorkspaceEdit {
                result: transaction.result,
                delta,
                produced_revision,
                transaction_id: Some(transaction.transaction_id),
            })
        })
    }

    /// Scope a grid's projection to a visible window and republish ("viewing is
    /// subscribing"). Read-shaping only: it runs no transaction and advances no
    /// revision; the projection diff emits the GridChanged delta the mirror
    /// patches in place.
    fn apply_grid_interest(
        &self,
        grid: &NodeId,
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    ) -> IntentReceipt {
        let session_id = match self.active_session_id() {
            Ok(id) => id,
            Err(error) => return self.reject_current(error),
        };
        let projected = HOST_SESSIONS.with(|sessions| -> Result<WorkspaceState, IntentError> {
            let session = sessions
                .borrow()
                .get(&session_id)
                .cloned()
                .ok_or_else(|| host_failure("workspace session handle is not available"))?;
            let mut session = session
                .lock()
                .map_err(|_| host_failure("workspace session mutex poisoned"))?;
            session
                .register_grid_interest(grid, top_row, left_col, bottom_row, right_col)
                .map_err(intent_error_from_session)?;
            session.workspace_state().map_err(intent_error_from_session)
        });
        match projected {
            Ok(mut after) => {
                let before = self.workspace.map(|workspace| workspace.get_untracked());
                after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
                let (_, delta) = self.publish_projection_state(before.as_ref(), after, false);
                IntentReceipt::accepted().with_delta(delta)
            }
            Err(error) => self.reject_current(error),
        }
    }

    fn apply_candidate_projection_edit(
        &self,
        edit: impl FnOnce(
            &mut TreeWorkspaceSession,
        ) -> Result<CandidateProjection, TreeWorkspaceSessionError>,
    ) -> Result<PublishedWorkspaceEdit<CandidateProjection>, IntentError> {
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
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (_, delta) = self.publish_projection_state(before.as_ref(), after, false);
            Ok(PublishedWorkspaceEdit {
                result,
                delta,
                produced_revision: None,
                transaction_id: None,
            })
        })
    }

    fn apply_projection_edit<T>(
        &self,
        edit: impl FnOnce(&mut TreeWorkspaceSession) -> Result<T, TreeWorkspaceSessionError>,
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
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (_, delta) = self.publish_projection_state(before.as_ref(), after, false);
            Ok(PublishedWorkspaceEdit {
                result,
                delta,
                produced_revision: None,
                transaction_id: None,
            })
        })
    }

    fn discard_candidate(&self, handle: &str) -> Result<IntentReceipt, IntentError> {
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
            session
                .discard_candidate(handle)
                .map_err(intent_error_from_session)?;
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (_, delta) = self.publish_projection_state(before.as_ref(), after, false);
            Ok(IntentReceipt::accepted().with_delta(delta))
        })
    }

    fn reap_candidates(&self, max_retained: usize) -> Result<IntentReceipt, IntentError> {
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
            session
                .reap_candidates(max_retained)
                .map_err(intent_error_from_session)?;
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (_, delta) = self.publish_projection_state(before.as_ref(), after, false);
            Ok(IntentReceipt::accepted().with_delta(delta))
        })
    }

    fn commit_candidate(
        &self,
        handle: &str,
    ) -> Result<PublishedWorkspaceEdit<String>, IntentError> {
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
            let removed = session
                .commit_candidate(handle)
                .map_err(intent_error_from_session)?;
            let mut after = session
                .workspace_state()
                .map_err(intent_error_from_session)?;
            after.clipboard = before.as_ref().and_then(|state| state.clipboard.clone());
            let (after, delta) = self.publish_projection_state(before.as_ref(), after, false);
            let produced_revision = after.revision.workspace_revision_id.clone();
            self.record_revision_undo_boundary(before.as_ref(), &after)?;
            Ok(PublishedWorkspaceEdit {
                result: removed.handle,
                delta,
                produced_revision,
                transaction_id: removed.transaction_id,
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

    fn undo_revision(&self) -> Result<PublishedWorkspaceEdit<()>, IntentError> {
        let target = self.pop_cursor_entry(&self.undo_stack, "undo history is empty")?;
        let current = match self.current_revision_cursor_entry() {
            Ok(current) => current,
            Err(error) => {
                self.push_cursor_entry(&self.undo_stack, target)?;
                return Err(error);
            }
        };
        self.push_cursor_entry(&self.redo_stack, current)?;
        match self.apply_workspace_edit(
            |session| session.navigate_workspace_revision(&target.revision_id),
            WorkspaceEditPublication::ProjectOnly,
        ) {
            Ok(publication) => {
                self.selection.set(target.selection);
                Ok(publication)
            }
            Err(error) => {
                let _ = self.pop_cursor_entry(&self.redo_stack, "redo rollback entry is missing");
                self.push_cursor_entry(&self.undo_stack, target)?;
                Err(error)
            }
        }
    }

    fn redo_revision(&self) -> Result<PublishedWorkspaceEdit<()>, IntentError> {
        let target = self.pop_cursor_entry(&self.redo_stack, "redo history is empty")?;
        let current = match self.current_revision_cursor_entry() {
            Ok(current) => current,
            Err(error) => {
                self.push_cursor_entry(&self.redo_stack, target)?;
                return Err(error);
            }
        };
        self.push_cursor_entry(&self.undo_stack, current)?;
        match self.apply_workspace_edit(
            |session| session.navigate_workspace_revision(&target.revision_id),
            WorkspaceEditPublication::ProjectOnly,
        ) {
            Ok(publication) => {
                self.selection.set(target.selection);
                Ok(publication)
            }
            Err(error) => {
                let _ = self.pop_cursor_entry(&self.undo_stack, "undo rollback entry is missing");
                self.push_cursor_entry(&self.redo_stack, target)?;
                Err(error)
            }
        }
    }

    fn current_revision_cursor_entry(&self) -> Result<RevisionCursorEntry, IntentError> {
        let revision_id = self
            .workspace
            .ok_or_else(|| host_failure("workspace projection handle is not attached"))?
            .get_untracked()
            .revision
            .workspace_revision_id
            .ok_or_else(|| host_failure("current workspace revision is not projected"))?;
        Ok(RevisionCursorEntry {
            revision_id,
            selection: self.selection.get_untracked(),
        })
    }

    fn pop_cursor_entry(
        &self,
        stack: &Mutex<Vec<RevisionCursorEntry>>,
        empty_message: &'static str,
    ) -> Result<RevisionCursorEntry, IntentError> {
        stack
            .lock()
            .map_err(|_| host_failure("revision cursor mutex poisoned"))?
            .pop()
            .ok_or_else(|| host_failure(empty_message))
    }

    fn push_cursor_entry(
        &self,
        stack: &Mutex<Vec<RevisionCursorEntry>>,
        entry: RevisionCursorEntry,
    ) -> Result<(), IntentError> {
        stack
            .lock()
            .map_err(|_| host_failure("revision cursor mutex poisoned"))?
            .push(entry);
        Ok(())
    }

    fn publish_projection_state(
        &self,
        before: Option<&WorkspaceState>,
        mut after: WorkspaceState,
        full_reset: bool,
    ) -> (WorkspaceState, WorkspaceDelta) {
        after.projection_seq = self.next_projection_seq.fetch_add(1, Ordering::Relaxed);
        let delta = workspace_delta(before, &after, full_reset);
        if let Some(workspace) = self.workspace {
            workspace.set(after.clone());
        }
        self.publish_delta(delta.clone());
        (after, delta)
    }

    fn publish_unchanged_delta(&self) -> WorkspaceDelta {
        let delta = WorkspaceDelta::unchanged(self.current_projection_seq());
        self.publish_delta(delta.clone());
        delta
    }

    fn reject_current(&self, error: IntentError) -> IntentReceipt {
        IntentReceipt::rejected(error).with_delta(self.publish_unchanged_delta())
    }

    fn publish_delta(&self, delta: WorkspaceDelta) {
        if let Some(latest_delta) = self.latest_delta {
            latest_delta.set(delta);
        }
    }

    fn persist_active_workspace_document(&self) -> Result<(), IntentError> {
        let Some(store) = &self.workspace_document_store else {
            return Ok(());
        };
        let active_session_id = self.active_session_id()?;
        let workspace_sessions = self
            .workspace_sessions
            .lock()
            .map_err(|_| host_failure("workspace catalog mutex poisoned"))?
            .clone();
        let active_workspace_id = workspace_sessions
            .iter()
            .find_map(|(workspace_id, session_id)| {
                (*session_id == active_session_id).then(|| workspace_id.clone())
            })
            .ok_or_else(|| host_failure("active workspace is missing from workspace catalog"))?;
        let selected = self.selection.get_untracked().primary;
        let workspace_names = self
            .shared
            .map(|shared| shared.with(|state| state.workspace_names.clone()))
            .unwrap_or_default();
        let outcome = persist_workspace_sessions(
            store,
            &workspace_sessions,
            &active_workspace_id,
            selected.as_ref(),
            &workspace_names,
        );
        // Surface the autosave result so the shell footer can show it. Honest
        // about the synchronous reality: this is the most-recent attempt, not a
        // full accounting of every mutation.
        if let Some(shared) = self.shared {
            shared.apply(
                SharedStateChange::SetLastSaveSuccess(Some(outcome.is_ok())),
                SharedStateOrigin::Host,
            );
        }
        outcome.map_err(|error| IntentError::HostFailure(error.to_string()))
    }

    fn hydrate_workspace_sessions_from_document_store(&self, active_workspace_id: &str) {
        let Some(store) = &self.workspace_document_store else {
            return;
        };
        let Ok(Some(catalog)) = store.load_catalog() else {
            self.refresh_shared_workspace_catalog(active_workspace_id);
            return;
        };
        if let Some(shared) = self.shared {
            shared.apply(
                SharedStateChange::SetWorkspaceNames(catalog.workspace_names.clone()),
                SharedStateOrigin::Host,
            );
        }
        for workspace_id in catalog.workspace_ids {
            let known = self
                .workspace_sessions
                .lock()
                .map(|sessions| sessions.contains_key(&workspace_id))
                .unwrap_or(true);
            if known {
                continue;
            }
            let Ok(Some(document)) = store.load_workspace(&workspace_id) else {
                continue;
            };
            let Ok((session, _)) = TreeWorkspaceSession::from_dnatree_document(document) else {
                continue;
            };
            let session = Arc::new(Mutex::new(session));
            let session_id = NEXT_HOST_SESSION_ID.fetch_add(1, Ordering::Relaxed);
            HOST_SESSIONS.with(|sessions| {
                sessions.borrow_mut().insert(session_id, session);
            });
            if let Ok(mut sessions) = self.workspace_sessions.lock() {
                sessions.insert(workspace_id, session_id);
            }
        }
        self.refresh_shared_workspace_catalog(active_workspace_id);
    }

    fn refresh_shared_workspace_catalog(&self, active_workspace_id: &str) {
        let Some(shared) = self.shared else {
            return;
        };
        let Ok(workspace_ids) = self
            .workspace_sessions
            .lock()
            .map(|sessions| sessions.keys().cloned().collect::<Vec<_>>())
        else {
            return;
        };
        shared.apply(
            SharedStateChange::SetWorkspaceIds(workspace_ids),
            SharedStateOrigin::Host,
        );
        shared.apply(
            SharedStateChange::SetActiveWorkspaceId(Some(active_workspace_id.to_string())),
            SharedStateOrigin::Host,
        );
    }

    fn record_revision_undo_boundary(
        &self,
        before: Option<&WorkspaceState>,
        after: &WorkspaceState,
    ) -> Result<(), IntentError> {
        let Some(before) = before else {
            return Ok(());
        };
        let Some(before_revision) = before.revision.workspace_revision_id.clone() else {
            return Ok(());
        };
        if Some(before_revision.as_str()) == after.revision.workspace_revision_id.as_deref() {
            return Ok(());
        }
        self.push_cursor_entry(
            &self.undo_stack,
            RevisionCursorEntry {
                revision_id: before_revision,
                selection: self.selection.get_untracked(),
            },
        )?;
        self.redo_stack
            .lock()
            .map_err(|_| host_failure("revision cursor mutex poisoned"))?
            .clear();
        Ok(())
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

    /// Rename a workspace's display label. The `workspace_id` is the immutable
    /// key; only the catalog/shared display name changes, so there is no model
    /// mutation and no revision. An unchanged delta keeps the projection
    /// channel monotonic, and the generic post-dispatch persist (which reads
    /// names from shared) writes the new label to the catalog.
    fn rename_workspace(&self, workspace_id: &str, new_name: &str) -> IntentReceipt {
        let Some(shared) = self.shared else {
            return self.reject_current(host_failure("workspace rename requires shared state"));
        };
        let known = self
            .workspace_sessions
            .lock()
            .map(|sessions| sessions.contains_key(workspace_id))
            .unwrap_or(false);
        if !known {
            return self.reject_current(IntentError::HostFailure(format!(
                "unknown workspace '{workspace_id}'"
            )));
        }
        let mut names = shared.with(|state| state.workspace_names.clone());
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            // An empty name falls back to the id — drop the entry rather than
            // store a blank label.
            names.remove(workspace_id);
        } else {
            names.insert(workspace_id.to_string(), trimmed.to_string());
        }
        shared.apply(
            SharedStateChange::SetWorkspaceNames(names),
            SharedStateOrigin::Host,
        );
        IntentReceipt::accepted().with_delta(self.publish_unchanged_delta())
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
        let state = session
            .lock()
            .map_err(|_| host_failure("workspace session mutex poisoned"))?
            .workspace_state()
            .map_err(intent_error_from_session)?;
        let (state, delta) = self.publish_projection_state(before.as_ref(), state, true);
        let produced_revision = state.revision.workspace_revision_id.clone();
        self.undo_stack
            .lock()
            .map_err(|_| host_failure("revision cursor mutex poisoned"))?
            .clear();
        self.redo_stack
            .lock()
            .map_err(|_| host_failure("revision cursor mutex poisoned"))?
            .clear();
        self.selection.set(SelectionState::default());
        if let Some(shared) = self.shared {
            let workspace_ids = self
                .workspace_sessions
                .lock()
                .map_err(|_| host_failure("workspace catalog mutex poisoned"))?
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            shared.apply(
                SharedStateChange::SetWorkspaceIds(workspace_ids),
                SharedStateOrigin::Host,
            );
            shared.apply(
                SharedStateChange::SetActiveWorkspaceId(Some(workspace_id.to_string())),
                SharedStateOrigin::Host,
            );
            shared.apply(
                SharedStateChange::SetManualRecalcPending(false),
                SharedStateOrigin::Host,
            );
        }
        Ok(PublishedWorkspaceEdit {
            result: (),
            delta,
            produced_revision,
            transaction_id: None,
        })
    }

    /// Read-only access to the active session for non-mutating previews.
    /// Takes the same session mutex as edits, so a preview can never observe
    /// a half-applied transaction.
    fn with_active_session_read<R>(
        &self,
        f: impl FnOnce(&TreeWorkspaceSession) -> Result<R, TreeWorkspaceSessionError>,
    ) -> Result<R, PreviewError> {
        let session_id = self
            .active_session_id()
            .map_err(|error| PreviewError::Host(error.to_string()))?;
        HOST_SESSIONS.with(|sessions| {
            let session =
                sessions.borrow().get(&session_id).cloned().ok_or_else(|| {
                    PreviewError::Host("workspace session is not available".into())
                })?;
            let session = session
                .lock()
                .map_err(|_| PreviewError::Host("workspace session mutex poisoned".into()))?;
            f(&session).map_err(|error| PreviewError::Host(error.to_string()))
        })
    }
}

/// The live host answers previews from the same session that executes intents
/// (tenet 7: the dispatcher that mutates can also report, without mutating,
/// what an intent would do). Every arm forwards to a session `preview_*`
/// method; nothing here computes legality host-side.
impl PreviewService for HostDispatcher {
    fn preview_formula_bind(
        &self,
        node: &NodeId,
        content: &str,
    ) -> Result<FormulaBindPreviewProjection, PreviewError> {
        self.with_active_session_read(|session| session.preview_formula_bind(node, content))
    }

    fn preview_mutation_impact(
        &self,
        intent: &MutationImpactIntentProjection,
    ) -> Result<MutationImpactProjection, PreviewError> {
        use MutationImpactIntentProjection as P;
        self.with_active_session_read(|session| match intent {
            P::AddNode {
                parent,
                symbol,
                initial,
                is_meta,
            } => {
                session.preview_add_node_impact(parent.as_ref(), symbol, initial.clone(), *is_meta)
            }
            P::EditContent { node, content } => session.preview_content_edit_impact(node, content),
            P::EditScopedContent { scope, content } => {
                session.preview_scoped_content_edit_impact(scope.clone(), content)
            }
            P::RenameNode { node, new_symbol } => {
                session.preview_rename_node_impact(node, new_symbol)
            }
            P::MoveNode {
                node,
                new_parent,
                new_index,
            } => session.preview_move_node_impact(node, new_parent.as_ref(), *new_index),
            P::DeleteNode { node } => session.preview_delete_node_impact(node),
            P::AddTableFormulaColumn {
                table,
                column_id,
                name,
                formula_text,
            } => session.preview_new_table_column_formula_impact(
                table,
                column_id,
                name,
                formula_text,
            ),
            P::AddTableRow {
                table,
                row_id,
                values,
            } => session.preview_add_table_row_impact(table, row_id, values.clone()),
            P::DeleteTableRow { table, row_id } => {
                session.preview_delete_table_row_impact(table, row_id)
            }
            P::RenameTableRow {
                table,
                row_id,
                new_row_id,
            } => session.preview_rename_table_row_impact(table, row_id, new_row_id),
            P::ReorderTableRow {
                table,
                row_id,
                new_index,
            } => session.preview_reorder_table_row_impact(table, row_id, *new_index),
            P::AddTableColumn {
                table,
                column_id,
                name,
                values,
            } => session.preview_add_table_column_impact(table, column_id, name, values.clone()),
            P::DeleteTableColumn { table, column_id } => {
                session.preview_delete_table_column_impact(table, column_id)
            }
            P::RenameTableColumn {
                table,
                column_id,
                name,
            } => session.preview_rename_table_column_impact(table, column_id, name),
            P::ReorderTableColumn {
                table,
                column_id,
                new_index,
            } => session.preview_reorder_table_column_impact(table, column_id, *new_index),
        })
    }
}

fn clipboard_from_projection(
    workspace: &WorkspaceState,
    scope: &AuthoringScope,
    payload: ClipboardPayloadKind,
    operation: ClipboardOperationProjection,
) -> Result<ClipboardProjection, IntentError> {
    let node_keys = workspace
        .expand_authoring_scope(scope)
        .map_err(|error| clipboard_scope_error(payload, error.to_string()))?;
    let payload = match payload {
        ClipboardPayloadKind::Values => ClipboardPayloadProjection::Values {
            nodes: node_keys
                .iter()
                .map(|node_key| {
                    let node = workspace.node_by_key(node_key).ok_or_else(|| {
                        clipboard_scope_error(
                            payload,
                            format!("expanded node {node_key} is absent from projection"),
                        )
                    })?;
                    Ok(ClipboardNodeValueProjection {
                        node: node.key.clone(),
                        path: node.id.clone(),
                        content_kind: node.content_kind,
                        constant_input_text: (node.content_kind == NodeContentKind::Constant)
                            .then(|| node.content_text.clone()),
                        literalized_input_text: node.literalized_value_input.clone(),
                        value: node.computed_value.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        ClipboardPayloadKind::Formula => {
            let node = single_clipboard_node(workspace, &node_keys, payload)?;
            if node.content_kind != NodeContentKind::Formula {
                return Err(clipboard_scope_error(
                    payload,
                    format!(
                        "formula clipboard payload requires a formula node, got {}",
                        node.content_kind
                    ),
                ));
            }
            ClipboardPayloadProjection::Formula {
                source: node.key.clone(),
                source_path: node.id.clone(),
                content: node.content_text.clone(),
            }
        }
        ClipboardPayloadKind::Format => ClipboardPayloadProjection::Format {
            nodes: node_keys
                .iter()
                .map(|node_key| {
                    let node = workspace.node_by_key(node_key).ok_or_else(|| {
                        clipboard_scope_error(
                            payload,
                            format!("expanded node {node_key} is absent from projection"),
                        )
                    })?;
                    Ok(ClipboardNodeFormatProjection {
                        node: node.key.clone(),
                        path: node.id.clone(),
                        effective_format: node.effective_format.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        ClipboardPayloadKind::Subtree => {
            let AuthoringScope::Subtree(root) = scope else {
                return Err(clipboard_scope_error(
                    payload,
                    "subtree clipboard payload requires AuthoringScope::Subtree",
                ));
            };
            let root_node = workspace.node_by_key(root).ok_or_else(|| {
                clipboard_scope_error(payload, format!("unknown subtree root {root}"))
            })?;
            ClipboardPayloadProjection::Subtree {
                root: root_node.key.clone(),
                root_path: root_node.id.clone(),
                nodes: node_keys,
            }
        }
    };
    let plain_text = clipboard_plain_text(&payload);
    Ok(ClipboardProjection {
        operation,
        payload,
        plain_text,
    })
}

fn clipboard_plain_text(payload: &ClipboardPayloadProjection) -> Option<String> {
    match payload {
        ClipboardPayloadProjection::Values { nodes } => Some(
            nodes
                .iter()
                .map(|node| clipboard_value_plain_text(&node.value))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        ClipboardPayloadProjection::Formula { content, .. } => Some(content.clone()),
        ClipboardPayloadProjection::Format { .. } | ClipboardPayloadProjection::Subtree { .. } => {
            None
        }
    }
}

fn clipboard_value_plain_text(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Array { cells, .. } => cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(clipboard_value_plain_text)
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => value.display_text(),
    }
}

fn external_clipboard_text_cells(text: &str) -> Vec<String> {
    let mut normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.ends_with('\n') {
        normalized.pop();
    }
    normalized
        .split('\n')
        .flat_map(|row| row.split('\t'))
        .map(ToString::to_string)
        .collect()
}

fn clipboard_number_format_code(workspace: &WorkspaceState) -> Result<Option<String>, IntentError> {
    let Some(clipboard) = &workspace.clipboard else {
        return Err(clipboard_payload_mismatch("format", "empty"));
    };
    let ClipboardPayloadProjection::Format { nodes } = &clipboard.payload else {
        return Err(clipboard_payload_mismatch(
            "format",
            clipboard_payload_actual(&clipboard.payload),
        ));
    };
    let [node] = nodes.as_slice() else {
        return Err(clipboard_payload_mismatch(
            "single_format",
            format!("format_count={}", nodes.len()),
        ));
    };
    Ok(node
        .effective_format
        .as_ref()
        .and_then(|format| format.number_format_code.clone()))
}

#[derive(Debug, Clone)]
struct ClipboardLiteralValuePayload {
    operation: ClipboardOperationProjection,
    items: Vec<ClipboardLiteralValueItem>,
}

#[derive(Debug, Clone)]
struct ClipboardLiteralValueItem {
    source: NodeKey,
    content: String,
}

fn clipboard_literal_value_payload(
    workspace: &WorkspaceState,
) -> Result<ClipboardLiteralValuePayload, IntentError> {
    let Some(clipboard) = &workspace.clipboard else {
        return Err(clipboard_payload_mismatch("values", "empty"));
    };
    let ClipboardPayloadProjection::Values { nodes } = &clipboard.payload else {
        return Err(clipboard_payload_mismatch(
            "values",
            clipboard_payload_actual(&clipboard.payload),
        ));
    };
    let mut items = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let content = node
            .constant_input_text
            .clone()
            .or_else(|| node.literalized_input_text.clone())
            .ok_or_else(|| {
                if nodes.len() == 1 {
                    clipboard_payload_mismatch(
                        "single_literal_value",
                        format!(
                            "source_content_kind={},value_literalization=unsupported",
                            node.content_kind
                        ),
                    )
                } else {
                    clipboard_payload_mismatch(
                        "ordered_literal_values",
                        format!(
                            "source_content_kind={},value_literalization=unsupported at index {index}",
                            node.content_kind
                        ),
                    )
                }
            })?;
        items.push(ClipboardLiteralValueItem {
            source: node.node.clone(),
            content,
        });
    }
    Ok(ClipboardLiteralValuePayload {
        operation: clipboard.operation,
        items,
    })
}

fn clipboard_payload_actual(payload: &ClipboardPayloadProjection) -> String {
    match payload {
        ClipboardPayloadProjection::Values { .. } => "values".to_string(),
        ClipboardPayloadProjection::Formula { .. } => "formula".to_string(),
        ClipboardPayloadProjection::Format { nodes } => format!("format_count={}", nodes.len()),
        ClipboardPayloadProjection::Subtree { .. } => "subtree".to_string(),
    }
}

fn single_clipboard_node<'a>(
    workspace: &'a WorkspaceState,
    node_keys: &[NodeKey],
    payload: ClipboardPayloadKind,
) -> Result<&'a NodeView, IntentError> {
    let [node_key] = node_keys else {
        return Err(clipboard_scope_error(
            payload,
            format!(
                "{} clipboard payload requires exactly one source node, got {}",
                payload.stable_id(),
                node_keys.len()
            ),
        ));
    };
    workspace.node_by_key(node_key).ok_or_else(|| {
        clipboard_scope_error(
            payload,
            format!("expanded node {node_key} is absent from projection"),
        )
    })
}

fn clipboard_scope_error(payload: ClipboardPayloadKind, detail: impl Into<String>) -> IntentError {
    IntentError::ClipboardScopeUnsupported {
        payload: payload.stable_id().to_string(),
        detail: detail.into(),
    }
}

fn clipboard_payload_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
) -> IntentError {
    IntentError::ClipboardPayloadMismatch {
        expected: expected.into(),
        actual: actual.into(),
    }
}

fn host_failure(message: impl Into<String>) -> IntentError {
    IntentError::HostFailure(message.into())
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
        TreeWorkspaceSessionError::UnsupportedInitialContent { policy } => {
            IntentError::UnsupportedInitialContent { policy }
        }
        TreeWorkspaceSessionError::InitialContentBindRejected { policy } => {
            IntentError::InitialContentBindRejected { policy }
        }
        TreeWorkspaceSessionError::FormatPathReserved { node } => {
            IntentError::FormatPathReserved { node }
        }
        TreeWorkspaceSessionError::NotePathReserved { node } => {
            IntentError::NotePathReserved { node }
        }
        TreeWorkspaceSessionError::AttributePathReserved { node } => {
            IntentError::AttributePathReserved { node }
        }
        TreeWorkspaceSessionError::InvalidAttributeKey { key } => {
            IntentError::InvalidAttributeKey { key }
        }
        TreeWorkspaceSessionError::FormulaReferenceInsertionFailed { node, detail } => {
            IntentError::FormulaReferenceInsertionFailed { node, detail }
        }
        TreeWorkspaceSessionError::DuplicateSubtreeUnsupported { node, detail } => {
            IntentError::DuplicateSubtreeUnsupported { node, detail }
        }
        TreeWorkspaceSessionError::UnknownCandidate { handle } => {
            IntentError::UnknownCandidate { handle }
        }
        TreeWorkspaceSessionError::ScenarioAlreadyExists { scenario_id } => {
            IntentError::ScenarioAlreadyExists { scenario_id }
        }
        TreeWorkspaceSessionError::UnknownScenario { scenario_id } => {
            IntentError::UnknownScenario { scenario_id }
        }
        TreeWorkspaceSessionError::UnknownScenarioOverride { scenario_id, node } => {
            IntentError::UnknownScenarioOverride { scenario_id, node }
        }
        TreeWorkspaceSessionError::UnsupportedScenarioOverrideValue {
            scenario_id,
            detail,
        } => IntentError::UnsupportedScenarioOverrideValue {
            scenario_id,
            detail,
        },
        TreeWorkspaceSessionError::SweepAlreadyExists { sweep_id } => {
            IntentError::SweepAlreadyExists { sweep_id }
        }
        TreeWorkspaceSessionError::UnknownSweep { sweep_id } => {
            IntentError::UnknownSweep { sweep_id }
        }
        TreeWorkspaceSessionError::DuplicateSweepPoint { sweep_id, point_id } => {
            IntentError::DuplicateSweepPoint { sweep_id, point_id }
        }
        TreeWorkspaceSessionError::EmptySweep { sweep_id } => IntentError::EmptySweep { sweep_id },
        TreeWorkspaceSessionError::ProjectionOutOfSync { node } => {
            IntentError::ProjectionOutOfSync { node }
        }
        TreeWorkspaceSessionError::GridInterest { node, detail } => {
            host_failure(format!("grid interest for {node}: {detail}"))
        }
        TreeWorkspaceSessionError::OxCalc(
            oxcalc_core::consumer::OxCalcTreeContextError::UnknownCandidate { handle },
        ) => IntentError::UnknownCandidate {
            handle: handle.to_string(),
        },
        TreeWorkspaceSessionError::OxCalc(
            oxcalc_core::consumer::OxCalcTreeContextError::CandidateBasisNotCurrent {
                handle,
                basis_revision_id,
                current_revision_id,
            },
        ) => IntentError::CandidateBasisNotCurrent {
            handle: handle.to_string(),
            basis_revision_id: basis_revision_id.to_string(),
            current_revision_id: current_revision_id.to_string(),
        },
        TreeWorkspaceSessionError::OxCalc(
            oxcalc_core::consumer::OxCalcTreeContextError::CandidateRebaseConflict {
                handle,
                basis_revision_id,
                current_revision_id,
                overlapping_nodes,
                ..
            },
        ) => IntentError::CandidateRebaseConflict {
            handle: handle.to_string(),
            basis_revision_id: basis_revision_id.to_string(),
            current_revision_id: current_revision_id.to_string(),
            overlapping_nodes: overlapping_nodes
                .into_iter()
                .map(node_key_for_tree_node)
                .collect(),
        },
        TreeWorkspaceSessionError::OxCalc(
            oxcalc_core::consumer::OxCalcTreeContextError::CandidateHasRetainedChild {
                handle,
                child_handle,
            },
        ) => IntentError::CandidateHasRetainedChild {
            handle: handle.to_string(),
            child_handle: child_handle.to_string(),
        },
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

    if before.clipboard != after.clipboard {
        changes.push(WorkspaceDeltaChange::ClipboardChanged(
            after.clipboard.clone(),
        ));
    }

    for candidate in &after.candidates {
        let before_candidate = before
            .candidates
            .iter()
            .find(|before| before.handle == candidate.handle);
        if before_candidate != Some(candidate) {
            changes.push(WorkspaceDeltaChange::CandidateChanged(candidate.clone()));
        }
    }
    for candidate in &before.candidates {
        if !after
            .candidates
            .iter()
            .any(|after| after.handle == candidate.handle)
        {
            changes.push(WorkspaceDeltaChange::CandidateRemoved(
                candidate.handle.clone(),
            ));
        }
    }

    for scenario in &after.scenarios.entries {
        let before_scenario = before
            .scenarios
            .entries
            .iter()
            .find(|before| before.id == scenario.id);
        if before_scenario != Some(scenario) {
            changes.push(WorkspaceDeltaChange::ScenarioChanged(scenario.clone()));
        }
    }
    for scenario in &before.scenarios.entries {
        if !after
            .scenarios
            .entries
            .iter()
            .any(|after| after.id == scenario.id)
        {
            changes.push(WorkspaceDeltaChange::ScenarioRemoved(scenario.id.clone()));
        }
    }

    for sweep in &after.sweeps.entries {
        let before_sweep = before
            .sweeps
            .entries
            .iter()
            .find(|before| before.id == sweep.id);
        if before_sweep != Some(sweep) {
            changes.push(WorkspaceDeltaChange::SweepChanged(sweep.clone()));
        }
    }
    for sweep in &before.sweeps.entries {
        if !after
            .sweeps
            .entries
            .iter()
            .any(|after| after.id == sweep.id)
        {
            changes.push(WorkspaceDeltaChange::SweepRemoved(sweep.id.clone()));
        }
    }

    // A grid-backed node whose windowed projection changed (recompute, or a moved
    // interest window) streams as a complete-replacement GridChanged the mirror
    // patches in place. When *only* the overlay descriptors changed (the cell
    // window held steady), the narrow GridOverlaysChanged path ships just the new
    // bundle so an overlay-only tick does not push the whole cell window. (Grid
    // *removal* has no intent in the read path yet; when a clear-grid verb lands
    // it must emit a removal/resync, since GridChanged alone cannot evict a grid
    // from the mirror.)
    for (node_id, grid) in &after.grids {
        match before.grids.get(node_id) {
            Some(previous) if previous == grid => {}
            Some(previous) if grid_change_is_overlay_only(previous, grid) => {
                changes.push(WorkspaceDeltaChange::GridOverlaysChanged {
                    grid_node_id: node_id.clone(),
                    overlays: grid.overlays.clone(),
                    overlay_epoch: grid.overlay_epoch,
                });
            }
            _ => changes.push(WorkspaceDeltaChange::GridChanged(grid.clone())),
        }
    }

    WorkspaceDelta {
        from_seq,
        to_seq,
        changes,
    }
}

/// Whether the only difference between two grid projections is the overlay set
/// (`overlays`/`overlay_epoch`) while the cells and every other field held
/// steady - the cue to take the narrow `GridOverlaysChanged` path instead of a
/// full `GridChanged`.
///
/// Robust to new `GridProjection` fields: it swaps the overlay fields onto a
/// clone of `before` and compares the whole struct, so any *other* field that
/// changed makes this `false` (forcing the full path) without this function
/// having to enumerate fields.
fn grid_change_is_overlay_only(before: &GridProjection, after: &GridProjection) -> bool {
    if before.overlays == after.overlays && before.overlay_epoch == after.overlay_epoch {
        return false;
    }
    let mut probe = before.clone();
    probe.overlays = after.overlays.clone();
    probe.overlay_epoch = after.overlay_epoch;
    probe == *after
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

#[cfg(test)]
mod overlay_delta_tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        GridCellProjection, GridMergedOverlayDescriptor, GridOverlayBundle, GridOverlayRect,
    };

    fn base_grid() -> GridProjection {
        GridProjection {
            grid_node_key: NodeKey::new("sheet"),
            grid_node_id: NodeId::new("Sheet1"),
            grid_id: "book:g:sheet:g".to_string(),
            max_rows: 100,
            max_cols: 26,
            cells: vec![GridCellProjection {
                row: 1,
                col: 1,
                value: NodeValueProjection::Number {
                    raw: "1".to_string(),
                    display: "1".to_string(),
                },
                value_epoch: 1,
            }],
            projection_epoch: 1,
            overlays: GridOverlayBundle::default(),
            overlay_epoch: 0,
            differential_clean: true,
        }
    }

    fn merged_bundle() -> GridOverlayBundle {
        GridOverlayBundle {
            merged: vec![GridMergedOverlayDescriptor {
                rect: GridOverlayRect {
                    top_row: 1,
                    left_col: 1,
                    bottom_row: 2,
                    right_col: 2,
                    clipped_top: false,
                    clipped_left: false,
                    clipped_bottom: false,
                    clipped_right: false,
                },
            }],
            ..Default::default()
        }
    }

    #[test]
    fn overlay_only_change_takes_the_narrow_path() {
        let before = base_grid();
        let mut after = base_grid();
        after.overlays = merged_bundle();
        after.overlay_epoch = 1;
        assert!(grid_change_is_overlay_only(&before, &after));
    }

    #[test]
    fn cell_change_takes_the_full_path() {
        let before = base_grid();
        let mut after = base_grid();
        after.cells[0].value = NodeValueProjection::Number {
            raw: "2".to_string(),
            display: "2".to_string(),
        };
        after.cells[0].value_epoch = 2;
        after.projection_epoch = 2;
        assert!(!grid_change_is_overlay_only(&before, &after));
    }

    #[test]
    fn identical_projection_is_not_overlay_only() {
        assert!(!grid_change_is_overlay_only(&base_grid(), &base_grid()));
    }

    #[test]
    fn combined_cell_and_overlay_change_takes_the_full_path() {
        let before = base_grid();
        let mut after = base_grid();
        after.cells[0].value_epoch = 2;
        after.projection_epoch = 2;
        after.overlays = merged_bundle();
        after.overlay_epoch = 1;
        assert!(
            !grid_change_is_overlay_only(&before, &after),
            "a combined cell+overlay change must take the full GridChanged path"
        );
    }
}

//! Leptos-signal-backed in-memory dispatcher.
//!
//! Splits out of the IR `intent` module (bead dtc-ajl.1): the IR crate keeps
//! the `Dispatcher` trait and the signal-free `RecordingDispatcher`; this
//! signal-backed variant lives here because it drives a Leptos `RwSignal`.

use std::sync::{Arc, Mutex};

use leptos::prelude::*;

use dnacalc_skin_ir::intent::{Dispatcher, IntentReceipt, WorkspaceIntent};
use dnacalc_skin_ir::selection::{SelectionState, TableCellSelection};

/// An in-memory dispatcher useful for unit tests and the walking-skeleton
/// host bootstrap before the live direct-context dispatcher is attached.
///
/// Selection intents update the provided [`RwSignal<SelectionState>`]; all
/// other intents are recorded and accepted. Holds a recording log so tests
/// can assert exactly what a skin dispatched.
pub struct InMemoryDispatcher {
    selection: RwSignal<SelectionState>,
    log: Arc<Mutex<Vec<WorkspaceIntent>>>,
}

impl InMemoryDispatcher {
    #[must_use]
    pub fn new(selection: RwSignal<SelectionState>) -> Self {
        Self {
            selection,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot the intents dispatched since the last reset.
    pub fn intents(&self) -> Vec<WorkspaceIntent> {
        self.log.lock().expect("intent log poisoned").clone()
    }

    pub fn clear_log(&self) {
        self.log.lock().expect("intent log poisoned").clear();
    }
}

impl Dispatcher for InMemoryDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        self.log
            .lock()
            .expect("intent log poisoned")
            .push(intent.clone());
        match intent {
            WorkspaceIntent::SelectNode(target) => {
                self.selection
                    .set(SelectionState::with_primary(target.clone()));
                IntentReceipt::accepted()
            }
            WorkspaceIntent::SelectTableCell {
                table,
                row_id,
                column_id,
            } => {
                self.selection
                    .set(SelectionState::with_table_cell(TableCellSelection {
                        table: table.clone(),
                        row_id: row_id.clone(),
                        column_id: column_id.clone(),
                    }));
                IntentReceipt::accepted()
            }
            WorkspaceIntent::EditFormula { .. } => {
                // The in-memory dispatcher records but does not apply
                // formula edits — the live host dispatcher does that through
                // direct OxCalc context. Tests for the skeleton
                // verify only the routing, not the calculation effect.
                IntentReceipt::accepted()
            }
            WorkspaceIntent::SelectNodes { .. }
            | WorkspaceIntent::Recalculate
            | WorkspaceIntent::EditContent { .. }
            | WorkspaceIntent::EditContentDeferred { .. }
            | WorkspaceIntent::EditScopedContent { .. }
            | WorkspaceIntent::SetNumberFormat { .. }
            | WorkspaceIntent::SetNote { .. }
            | WorkspaceIntent::SetMeta { .. }
            | WorkspaceIntent::SetNodeAttributes { .. }
            | WorkspaceIntent::CopyToClipboard { .. }
            | WorkspaceIntent::CutToClipboard { .. }
            | WorkspaceIntent::PasteClipboardFormat { .. }
            | WorkspaceIntent::PasteClipboardValues { .. }
            | WorkspaceIntent::PasteExternalClipboardText { .. }
            | WorkspaceIntent::DuplicateSubtree { .. }
            | WorkspaceIntent::InsertFormulaReference { .. }
            | WorkspaceIntent::OpenCandidate { .. }
            | WorkspaceIntent::EditCandidateContent { .. }
            | WorkspaceIntent::RenameCandidateNode { .. }
            | WorkspaceIntent::MoveCandidateNode { .. }
            | WorkspaceIntent::ReorderCandidateNode { .. }
            | WorkspaceIntent::DeleteCandidateNode { .. }
            | WorkspaceIntent::AddCandidateNode { .. }
            | WorkspaceIntent::EvaluateCandidate { .. }
            | WorkspaceIntent::RebaseCandidate { .. }
            | WorkspaceIntent::DiscardCandidate { .. }
            | WorkspaceIntent::PinCandidateRetention { .. }
            | WorkspaceIntent::UnpinCandidateRetention { .. }
            | WorkspaceIntent::ReapCandidates { .. }
            | WorkspaceIntent::CommitCandidate { .. }
            | WorkspaceIntent::CreateScenarioFromCandidate { .. }
            | WorkspaceIntent::CreateScenario { .. }
            | WorkspaceIntent::ActivateScenario { .. }
            | WorkspaceIntent::DeleteScenario { .. }
            | WorkspaceIntent::SetScenarioOverride { .. }
            | WorkspaceIntent::ClearScenarioOverride { .. }
            | WorkspaceIntent::CreateScenarioSweep { .. }
            | WorkspaceIntent::ActivateSweep { .. }
            | WorkspaceIntent::DeleteSweep { .. }
            | WorkspaceIntent::AddNode { .. }
            | WorkspaceIntent::RenameNode { .. }
            | WorkspaceIntent::MoveNode { .. }
            | WorkspaceIntent::ReorderNode { .. }
            | WorkspaceIntent::DeleteNode { .. }
            | WorkspaceIntent::EditTableCell { .. }
            | WorkspaceIntent::AddTableRow { .. }
            | WorkspaceIntent::DeleteTableRow { .. }
            | WorkspaceIntent::RenameTableRow { .. }
            | WorkspaceIntent::ReorderTableRow { .. }
            | WorkspaceIntent::RenameTable { .. }
            | WorkspaceIntent::AddTableColumn { .. }
            | WorkspaceIntent::AddTableFormulaColumn { .. }
            | WorkspaceIntent::EditTableColumnFormula { .. }
            | WorkspaceIntent::SetTableTotalsFormula { .. }
            | WorkspaceIntent::ClearTableTotalsFormula { .. }
            | WorkspaceIntent::SetTableHeaderRowVisible { .. }
            | WorkspaceIntent::SetTableTotalsRowVisible { .. }
            | WorkspaceIntent::RenameTableColumn { .. }
            | WorkspaceIntent::ReorderTableColumn { .. }
            | WorkspaceIntent::DeleteTableColumn { .. }
            | WorkspaceIntent::CreateTable { .. }
            | WorkspaceIntent::NewWorkspace
            | WorkspaceIntent::SwitchWorkspace { .. }
            | WorkspaceIntent::RenameWorkspace { .. }
            | WorkspaceIntent::NavigateRevision { .. }
            | WorkspaceIntent::Undo
            | WorkspaceIntent::Redo
            | WorkspaceIntent::SetGridInterest { .. }
            | WorkspaceIntent::SetPersona { .. } => IntentReceipt::accepted(),
            // `WorkspaceIntent` is `#[non_exhaustive]`; from outside its
            // defining crate an exhaustive match needs a wildcard. Behaviour is
            // unchanged: every non-selection intent is recorded and accepted.
            _ => IntentReceipt::accepted(),
        }
    }
}

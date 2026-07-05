//! Signal-free dispatcher double for IR-side and host-core tests.
//!
//! [`RecordingDispatcher`] mirrors the behaviour of the Leptos-backed
//! `InMemoryDispatcher` (in `dnacalc-skin-leptos`) but holds selection in a
//! plain `Arc<Mutex<SelectionState>>` and records the intent log, so tests
//! that only need to assert *what a skin dispatched* never have to pull in
//! Leptos or an engine.

use std::sync::{Arc, Mutex};

use crate::intent::{Dispatcher, IntentReceipt, WorkspaceIntent};
use crate::selection::{SelectionState, TableCellSelection};

/// An in-memory, signal-free dispatcher. Selection intents update a plain
/// mutex-guarded [`SelectionState`]; every intent is recorded so tests can
/// assert exactly what a skin dispatched. All intents are accepted.
#[derive(Clone, Default)]
pub struct RecordingDispatcher {
    selection: Arc<Mutex<SelectionState>>,
    log: Arc<Mutex<Vec<WorkspaceIntent>>>,
}

impl RecordingDispatcher {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current selection snapshot.
    #[must_use]
    pub fn selection(&self) -> SelectionState {
        self.selection.lock().expect("selection poisoned").clone()
    }

    /// Snapshot the intents dispatched since the last reset.
    #[must_use]
    pub fn intents(&self) -> Vec<WorkspaceIntent> {
        self.log.lock().expect("intent log poisoned").clone()
    }

    pub fn clear_log(&self) {
        self.log.lock().expect("intent log poisoned").clear();
    }
}

impl Dispatcher for RecordingDispatcher {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt {
        self.log
            .lock()
            .expect("intent log poisoned")
            .push(intent.clone());
        match intent {
            WorkspaceIntent::SelectNode(target) => {
                *self.selection.lock().expect("selection poisoned") =
                    SelectionState::with_primary(target);
            }
            WorkspaceIntent::SelectTableCell {
                table,
                row_id,
                column_id,
            } => {
                *self.selection.lock().expect("selection poisoned") =
                    SelectionState::with_table_cell(TableCellSelection {
                        table,
                        row_id,
                        column_id,
                    });
            }
            _ => {}
        }
        IntentReceipt::accepted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeId;

    #[test]
    fn records_and_applies_selection() {
        let dispatcher = RecordingDispatcher::new();
        let receipt = dispatcher.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new("A"))));
        assert!(receipt.accepted);
        assert_eq!(
            dispatcher.selection(),
            SelectionState::with_primary(Some(NodeId::new("A")))
        );
        assert_eq!(dispatcher.intents().len(), 1);
    }

    #[test]
    fn records_non_selection_intent_without_selection_change() {
        let dispatcher = RecordingDispatcher::new();
        dispatcher.dispatch(WorkspaceIntent::Recalculate);
        assert_eq!(dispatcher.selection(), SelectionState::default());
        assert_eq!(dispatcher.intents(), vec![WorkspaceIntent::Recalculate]);
    }
}

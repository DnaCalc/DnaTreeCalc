use std::sync::{Arc, Mutex};

use crate::identity::{NodeId, NodeKey};
use crate::selection::{SelectionState, TableCellSelection};
use crate::workspace::{CalcRunProjection, DependencyKindProjection, NodeValueProjection};
use leptos::prelude::*;

/// The closed set of asks a skin may make of the host.
///
/// Per `docs/ux/SKINS.md` §2.6 this is intended to be the canonical
/// command taxonomy (skins, undo, command palette read from the same
/// set). The walking skeleton enumerates only what it exercises;
/// structural edits land with W003, format/template ops with W007.
/// Adding a variant later is a deliberate extension — skins compile
/// against the closed set so each addition is reviewed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceIntent {
    /// Replace the host-wide primary selection. `None` clears.
    SelectNode(Option<NodeId>),
    /// Focus a table cell without changing calculation state.
    SelectTableCell {
        table: NodeId,
        row_id: Option<String>,
        column_id: String,
    },
    /// Force the host to run calculation and publish a fresh projection.
    Recalculate,
    /// Replace the content text of a node. Empty -> Empty kind;
    /// leading `=` -> Formula; otherwise Constant. OxCalc does the
    /// rebind; the skin does not parse formula text.
    EditFormula {
        node: NodeId,
        content: String,
    },
    /// Preferred spelling for content edits. Kept separate from
    /// `EditFormula` while the skeleton tests and skins still use the
    /// older variant name.
    EditContent {
        node: NodeId,
        content: String,
    },
    /// Replace node content without running calculation immediately.
    /// Manual recalc mode uses this to keep editing responsive; an
    /// explicit [`WorkspaceIntent::Recalculate`] publishes values.
    EditContentDeferred {
        node: NodeId,
        content: String,
    },
    AddNode {
        parent: Option<NodeId>,
        symbol: String,
        content: String,
    },
    RenameNode {
        node: NodeId,
        new_symbol: String,
    },
    MoveNode {
        node: NodeId,
        new_parent: Option<NodeId>,
        new_index: Option<usize>,
    },
    ReorderNode {
        node: NodeId,
        new_index: usize,
    },
    DeleteNode {
        node: NodeId,
    },
    EditTableCell {
        table: NodeId,
        row_id: String,
        column_id: String,
        content: String,
    },
    AddTableRow {
        table: NodeId,
        row_id: String,
        values: Vec<TableCellInput>,
    },
    DeleteTableRow {
        table: NodeId,
        row_id: String,
    },
    RenameTableRow {
        table: NodeId,
        row_id: String,
        new_row_id: String,
    },
    ReorderTableRow {
        table: NodeId,
        row_id: String,
        new_index: usize,
    },
    RenameTable {
        table: NodeId,
        name: String,
    },
    AddTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
        values: Vec<TableRowInput>,
    },
    AddTableFormulaColumn {
        table: NodeId,
        column_id: String,
        name: String,
        formula_text: String,
    },
    EditTableColumnFormula {
        table: NodeId,
        column_id: String,
        formula_text: String,
    },
    SetTableTotalsFormula {
        table: NodeId,
        column_id: String,
        formula_text: String,
    },
    ClearTableTotalsFormula {
        table: NodeId,
        column_id: String,
    },
    SetTableHeaderRowVisible {
        table: NodeId,
        visible: bool,
    },
    SetTableTotalsRowVisible {
        table: NodeId,
        visible: bool,
    },
    RenameTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
    },
    ReorderTableColumn {
        table: NodeId,
        column_id: String,
        new_index: usize,
    },
    DeleteTableColumn {
        table: NodeId,
        column_id: String,
    },
    NewWorkspace,
    SwitchWorkspace {
        workspace_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellInput {
    pub column_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRowInput {
    pub row_id: String,
    pub content: String,
}

/// Outcome of dispatching a single intent.
///
/// Carries a coarse acceptance flag plus the typed error variant
/// when the dispatcher refused. The spec calls out a `completed_signal`
/// for asynchronous intents (e.g., long template sync); the skeleton
/// uses only synchronous selection + formula edits, so that field
/// is omitted until W007.
#[derive(Debug, Clone)]
pub struct IntentReceipt {
    pub accepted: bool,
    pub error: Option<IntentError>,
    pub transaction_id: Option<String>,
    pub produced_revision: Option<String>,
    pub delta: WorkspaceDelta,
}

impl IntentReceipt {
    #[must_use]
    pub fn accepted() -> Self {
        Self {
            accepted: true,
            error: None,
            transaction_id: None,
            produced_revision: None,
            delta: WorkspaceDelta::unchanged(0),
        }
    }

    #[must_use]
    pub fn rejected(error: IntentError) -> Self {
        Self {
            accepted: false,
            error: Some(error),
            transaction_id: None,
            produced_revision: None,
            delta: WorkspaceDelta::unchanged(0),
        }
    }

    #[must_use]
    pub fn with_delta(mut self, delta: WorkspaceDelta) -> Self {
        self.delta = delta;
        self
    }

    #[must_use]
    pub fn with_produced_revision(mut self, produced_revision: Option<String>) -> Self {
        self.produced_revision = produced_revision;
        self
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum IntentError {
    #[error("intent variant not yet supported by this dispatcher")]
    Unsupported,
    #[error("dispatcher rejected the intent: {0}")]
    Rejected(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceDelta {
    pub from_seq: u64,
    pub to_seq: u64,
    pub changes: Vec<WorkspaceDeltaChange>,
}

impl WorkspaceDelta {
    #[must_use]
    pub fn unchanged(seq: u64) -> Self {
        Self {
            from_seq: seq,
            to_seq: seq,
            changes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceDeltaChange {
    FullReset,
    Structural(StructuralDeltaProjection),
    NodesChanged(Vec<NodeKey>),
    ValuesChanged(Vec<NodeValueDeltaProjection>),
    DepsChanged(Vec<DependencyDeltaProjection>),
    CalcRun(CalcRunProjection),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructuralDeltaProjection {
    pub added: Vec<NodeKey>,
    pub removed: Vec<NodeKey>,
    pub changed: Vec<NodeKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeValueDeltaProjection {
    pub node: NodeKey,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDeltaProjection {
    pub owner: NodeKey,
    pub kinds: Vec<DependencyKindProjection>,
}

/// The only path through which a skin may ask the host to change anything
/// outside its own typed state.
///
/// The skeleton ships an [`InMemoryDispatcher`] used by tests; the host
/// crate wires a real dispatcher backed by direct OxCalc context for the live
/// shell.
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt;
}

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
            WorkspaceIntent::Recalculate
            | WorkspaceIntent::EditContent { .. }
            | WorkspaceIntent::EditContentDeferred { .. }
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
            | WorkspaceIntent::NewWorkspace
            | WorkspaceIntent::SwitchWorkspace { .. } => IntentReceipt::accepted(),
        }
    }
}

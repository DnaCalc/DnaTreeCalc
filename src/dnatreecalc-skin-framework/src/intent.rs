use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::identity::{NodeId, NodeKey};
use crate::selection::{SelectionState, TableCellSelection};
use crate::workspace::{
    CalcRunProjection, ClipboardProjection, DependencyKindProjection, InitialNodeContentProjection,
    NodeValueProjection,
};
use leptos::prelude::*;

/// Typed subject for authoring verbs.
///
/// Skins may carry this value through commands, palettes, previews, and future
/// mutating intents, but scope expansion is host-owned because subtree
/// membership and reference-collection membership are projection/engine truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoringScope {
    Node(NodeKey),
    Nodes(Vec<NodeKey>),
    Subtree(NodeKey),
    Collection {
        owner: NodeKey,
        source_reference_handle: String,
    },
}

/// Patch for authored node attributes.
///
/// Attributes are model metadata, not formula-visible values. The host owns the
/// attribute storage policy and persists patches through revisioned model edits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeAttributePatch {
    pub set: BTreeMap<String, String>,
    pub clear: BTreeSet<String>,
}

impl NodeAttributePatch {
    #[must_use]
    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            set: BTreeMap::from([(key.into(), value.into())]),
            clear: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn clear(key: impl Into<String>) -> Self {
        Self {
            set: BTreeMap::new(),
            clear: BTreeSet::from([key.into()]),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.clear.is_empty()
    }
}

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
    /// Replace content on every node in a typed authoring scope as one host
    /// transaction. Scope expansion is host/projection-owned.
    EditScopedContent {
        scope: AuthoringScope,
        content: String,
    },
    /// Author or clear a number format over a typed scope. The host stores the
    /// authored property in canonical meta nodes; OxFml owns format rendering.
    SetNumberFormat {
        scope: AuthoringScope,
        number_format_code: Option<String>,
    },
    /// Author or clear a per-node note/comment. Notes are model metadata
    /// projected to skins; they are not formula-visible semantic values.
    SetNote {
        node: NodeKey,
        note: Option<String>,
    },
    /// Mark or unmark a node as meta. Meta-effective nodes are invisible to
    /// formula resolution, so the host routes this through OxCalc.
    SetMeta {
        node: NodeKey,
        is_meta: bool,
    },
    /// Patch authored per-node attributes. Attribute keys and values are
    /// host-model metadata; formulas do not see them.
    SetNodeAttributes {
        node: NodeKey,
        attrs: NodeAttributePatch,
    },
    /// Populate the host-owned clipboard carrier from a typed scope. The
    /// clipboard is distinct from the OS clipboard and carries typed model
    /// facts for later paste/duplicate verbs.
    CopyToClipboard {
        scope: AuthoringScope,
        payload: ClipboardPayloadKind,
    },
    /// Populate the host-owned clipboard carrier as a pending cut. This does
    /// not delete model nodes; a later paste/commit verb owns that mutation.
    CutToClipboard {
        scope: AuthoringScope,
        payload: ClipboardPayloadKind,
    },
    /// Paste the current clipboard format payload onto a typed target scope.
    /// Formula and value paste are separate ownership-sensitive verbs.
    PasteClipboardFormat {
        target: AuthoringScope,
    },
    /// Paste a constant-source value clipboard payload onto a typed target
    /// scope using authored constant input, not rendered display text.
    PasteClipboardValues {
        target: AuthoringScope,
    },
    /// Paste text supplied by the platform clipboard into a typed target
    /// scope as authored content. The host never touches the OS clipboard;
    /// skins/platform code own the actual clipboard read.
    PasteExternalClipboardText {
        target: AuthoringScope,
        text: String,
    },
    AddNode {
        parent: Option<NodeId>,
        symbol: String,
        initial: InitialNodeContentProjection,
        is_meta: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardPayloadKind {
    Values,
    Formula,
    Format,
    Subtree,
}

impl ClipboardPayloadKind {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Values => "values",
            Self::Formula => "formula",
            Self::Format => "format",
            Self::Subtree => "subtree",
        }
    }
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

    #[must_use]
    pub fn with_transaction_id(mut self, transaction_id: Option<String>) -> Self {
        self.transaction_id = transaction_id;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntentError {
    #[error("intent variant not yet supported by this dispatcher")]
    Unsupported,
    #[error("unknown node {node}")]
    UnknownNode { node: String },
    #[error("duplicate node {node}")]
    DuplicateNode { node: String },
    #[error("unknown table {table}")]
    UnknownTable { table: String },
    #[error("table {table} requires a non-empty name")]
    EmptyTableName { table: String },
    #[error("table name {name} is already used while renaming {table}")]
    DuplicateTableName { table: String, name: String },
    #[error("unknown row {row_id} in table {table}")]
    UnknownTableRow { table: String, row_id: String },
    #[error("duplicate row {row_id} in table {table}")]
    DuplicateTableRow { table: String, row_id: String },
    #[error("unknown column {column_id} in table {table}")]
    UnknownTableColumn { table: String, column_id: String },
    #[error("duplicate column {column_id} in table {table}")]
    DuplicateTableColumn { table: String, column_id: String },
    #[error("unknown cell {row_id}/{column_id} in table {table}")]
    UnknownTableCell {
        table: String,
        row_id: String,
        column_id: String,
    },
    #[error("duplicate input for column {column_id} in table {table}")]
    DuplicateTableCellInput { table: String, column_id: String },
    #[error("duplicate input for row {row_id} in table {table}")]
    DuplicateTableRowInput { table: String, row_id: String },
    #[error(
        "table formula column {column_id} in table {table} is calculated, not directly editable"
    )]
    FormulaTableCellEdit { table: String, column_id: String },
    #[error("table constant column {column_id} in table {table} does not carry formula metadata")]
    ConstantTableColumnFormulaEdit { table: String, column_id: String },
    #[error("initial node content policy {policy} is not yet supported")]
    UnsupportedInitialContent { policy: String },
    #[error("initial node content policy {policy} was rejected by formula bind")]
    InitialContentBindRejected { policy: String },
    #[error("host projection is out of sync for {node}")]
    ProjectionOutOfSync { node: String },
    #[error("format meta path {node} is occupied by a non-meta node")]
    FormatPathReserved { node: String },
    #[error("note meta path {node} is occupied by a non-meta node")]
    NotePathReserved { node: String },
    #[error("attribute meta path {node} is occupied by a non-meta node")]
    AttributePathReserved { node: String },
    #[error("attribute key {key} is not path-safe")]
    InvalidAttributeKey { key: String },
    #[error("clipboard payload {payload} cannot be built from this scope: {detail}")]
    ClipboardScopeUnsupported { payload: String, detail: String },
    #[error("clipboard does not contain a usable {expected} payload: {actual}")]
    ClipboardPayloadMismatch { expected: String, actual: String },
    #[error("engine rejected the intent: {0}")]
    EngineRejected(String),
    #[error("host failed to dispatch the intent: {0}")]
    HostFailure(String),
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
    ClipboardChanged(Option<ClipboardProjection>),
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

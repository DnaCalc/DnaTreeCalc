use std::collections::{BTreeMap, BTreeSet};

use crate::identity::{NodeId, NodeKey};
use crate::workspace::{
    CalcRunProjection, CandidateProjection, ClipboardProjection, DependencyKindProjection,
    GridOverlayBundle, GridProjection, InitialNodeContentProjection, NodeValueProjection,
    ScenarioProjection, SweepProjection,
};
use serde::{Deserialize, Serialize};

/// Typed subject for authoring verbs.
///
/// Skins may carry this value through commands, palettes, previews, and future
/// mutating intents, but scope expansion is host-owned because subtree
/// membership and reference-collection membership are projection/engine truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAttributePatch {
    pub set: BTreeMap<String, String>,
    pub clear: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormulaReferenceInsertionTarget {
    Node(NodeKey),
    HostReferenceCollection {
        base: Option<NodeKey>,
        collection_family: String,
    },
    HostStructuralSelector {
        base: NodeKey,
        selector_family: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaReferenceInsertionProjection {
    pub node: NodeKey,
    pub target: FormulaReferenceInsertionTarget,
    pub inserted_text: String,
    pub updated_formula_text: String,
    pub applied_start: usize,
    pub applied_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepPointInput {
    pub point_id: String,
    pub label: String,
    pub value: NodeValueProjection,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkspaceIntent {
    /// Replace the host-wide primary selection. `None` clears.
    SelectNode(Option<NodeId>),
    /// Replace the host-wide multi-selection set (and its anchor) — the
    /// population subject for bulk verbs. Dispatched (not raw view-state) so
    /// population selection is audited like everything else; the single
    /// `SelectNode` primary remains independent.
    SelectNodes {
        keys: Vec<NodeKey>,
        anchor: Option<NodeKey>,
    },
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
    /// Duplicate a formula-free subtree under a destination parent. Formula
    /// rebind remains OxFml-owned and is rejected until that API exists.
    DuplicateSubtree {
        source: NodeKey,
        destination_parent: Option<NodeId>,
        new_symbol: String,
    },
    /// Insert a host reference into a formula edit buffer. The skin owns the
    /// buffer/caret span; OxFml owns reference text composition and rebind.
    InsertFormulaReference {
        node: NodeKey,
        current_formula_text: String,
        replacement_start: usize,
        replacement_len: usize,
        target: FormulaReferenceInsertionTarget,
    },
    /// Open a non-publishing candidate overlay on the current workspace
    /// revision. Candidate semantics are engine-owned; skins receive only the
    /// opaque handle and projected candidate values.
    OpenCandidate {
        parent: Option<String>,
    },
    /// Apply a content edit inside a candidate without publishing workspace
    /// state.
    EditCandidateContent {
        handle: String,
        node: NodeId,
        content: String,
    },
    /// Rename a node inside a candidate without publishing workspace state.
    /// Nodes are addressed by stable key because candidate-private structural
    /// paths may diverge from the published workspace projection.
    RenameCandidateNode {
        handle: String,
        node: NodeKey,
        new_symbol: String,
    },
    /// Move a node inside a candidate without publishing workspace state.
    /// Parent is also key-addressed for candidate-private structural views.
    MoveCandidateNode {
        handle: String,
        node: NodeKey,
        new_parent: Option<NodeKey>,
        new_index: Option<usize>,
    },
    /// Reorder a node inside its candidate-private parent without publishing
    /// workspace state.
    ReorderCandidateNode {
        handle: String,
        node: NodeKey,
        new_index: usize,
    },
    /// Delete a node inside a candidate without publishing workspace state.
    DeleteCandidateNode {
        handle: String,
        node: NodeKey,
    },
    /// Add a node inside a candidate without publishing workspace state.
    /// Parent is key-addressed for candidate-private structural views.
    AddCandidateNode {
        handle: String,
        parent: Option<NodeKey>,
        symbol: String,
        initial: InitialNodeContentProjection,
        is_meta: bool,
    },
    /// Evaluate a candidate and publish the private result into the candidate
    /// projection only.
    EvaluateCandidate {
        handle: String,
    },
    /// Rebase a candidate by replaying its engine-owned private edit log onto
    /// the current workspace revision without publishing. Parented candidates
    /// are flattened by the engine during rebase.
    RebaseCandidate {
        handle: String,
    },
    /// Drop an addressable candidate overlay without changing workspace state.
    DiscardCandidate {
        handle: String,
    },
    /// Protect a candidate from engine-owned budget reaping while a host view
    /// or workflow actively retains it.
    PinCandidateRetention {
        handle: String,
    },
    /// Release one host retention pin previously held for a candidate.
    UnpinCandidateRetention {
        handle: String,
    },
    /// Ask the engine to reclaim unprotected candidates until the retained
    /// candidate count is at or below the requested budget.
    ReapCandidates {
        max_retained: usize,
    },
    /// Commit a candidate into the live workspace if its basis revision is
    /// still current. Stale basis is a typed engine rejection.
    CommitCandidate {
        handle: String,
    },
    /// Register a host-owned scenario label over an existing OxCalc candidate
    /// handle. Values remain candidate projection truth.
    CreateScenarioFromCandidate {
        scenario_id: String,
        name: String,
        candidate_handle: String,
    },
    /// Create a host-managed scenario from the published state or a visible
    /// managed base scenario. The host opens and retains the OxCalc candidate,
    /// making the scenario reconstructable from typed overrides during
    /// document reload.
    CreateScenario {
        scenario_id: String,
        name: String,
        base_scenario_id: Option<String>,
    },
    /// Set or clear the active scenario rail selection.
    ActivateScenario {
        scenario_id: Option<String>,
    },
    /// Delete a host-owned scenario label and release its candidate retention
    /// pin.
    DeleteScenario {
        scenario_id: String,
    },
    /// Set a typed scenario override value by stable node key. The host turns
    /// supported values into authored input text and applies it to the
    /// scenario's backing candidate.
    SetScenarioOverride {
        scenario_id: String,
        node: NodeKey,
        value: NodeValueProjection,
    },
    /// Clear a typed scenario override by restoring the node input captured
    /// when the override was first set.
    ClearScenarioOverride {
        scenario_id: String,
        node: NodeKey,
    },
    /// Create a host-owned direct sensitivity sweep over one input node.
    /// Each point is materialized as an evaluated OxCalc candidate-backed
    /// scenario; skins read typed sweep/comparison/series projections instead
    /// of computing formula results.
    CreateScenarioSweep {
        sweep_id: String,
        name: String,
        base_scenario_id: Option<String>,
        input_node: NodeKey,
        points: Vec<SweepPointInput>,
    },
    /// Set or clear the active sweep rail selection.
    ActivateSweep {
        sweep_id: Option<String>,
    },
    /// Delete a host-owned sweep and release its scenario-backed points.
    DeleteSweep {
        sweep_id: String,
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
    /// Create a new table anchored on a fresh node under `parent` (root when
    /// `None`), pre-populated as a 2×1 starter grid. Author-only (the
    /// `Persona::allows` Reviewer/ReadOnly allow-lists omit it).
    CreateTable {
        parent: Option<NodeId>,
        symbol: String,
    },
    NewWorkspace,
    SwitchWorkspace {
        workspace_id: String,
    },
    /// Rename a workspace's human display label. The `workspace_id` stays the
    /// immutable key; only the catalog's display name changes. Author-only
    /// (the `Persona::allows` Reviewer/ReadOnly allow-lists omit it).
    RenameWorkspace {
        workspace_id: String,
        new_name: String,
    },
    /// Navigate the active workspace to an OxCalc-retained revision. This is
    /// not inverse replay; OxCalc owns the actual restoration semantics.
    NavigateRevision {
        revision_id: String,
    },
    /// Navigate to the previous host command boundary using OxCalc-retained
    /// revisions. The host owns the cursor stack; OxCalc owns restoration.
    Undo,
    /// Navigate forward after undo using OxCalc-retained revisions.
    Redo,
    /// Switch the governing persona. Travels as an intent so persona changes
    /// are audited like everything else; the dispatcher enforces the policy
    /// per intent origin (tenet 9). First slice: any persona may switch.
    SetPersona {
        persona: crate::permissions::Persona,
    },
    /// Register the visible cell window of a grid-backed sheet node ("viewing is
    /// subscribing"): the host scopes OxCalc's grid projection to this row/column
    /// rectangle and streams back the windowed cells. Read-shaping only -- it does
    /// not mutate the document or advance the revision.
    SetGridInterest {
        grid: NodeId,
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCellInput {
    pub column_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRowInput {
    pub row_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
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
    #[error("formula reference insertion failed for {node}: {detail}")]
    FormulaReferenceInsertionFailed { node: String, detail: String },
    #[error("duplicate subtree failed for {node}: {detail}")]
    DuplicateSubtreeUnsupported { node: String, detail: String },
    #[error("unknown candidate {handle}")]
    UnknownCandidate { handle: String },
    #[error(
        "candidate {handle} basis {basis_revision_id} is not current workspace revision {current_revision_id}"
    )]
    CandidateBasisNotCurrent {
        handle: String,
        basis_revision_id: String,
        current_revision_id: String,
    },
    #[error(
        "candidate {handle} rebase from {basis_revision_id} to {current_revision_id} conflicts on {overlapping_nodes:?}"
    )]
    CandidateRebaseConflict {
        handle: String,
        basis_revision_id: String,
        current_revision_id: String,
        overlapping_nodes: Vec<NodeKey>,
    },
    #[error("candidate {handle} has retained child candidate {child_handle}")]
    CandidateHasRetainedChild {
        handle: String,
        child_handle: String,
    },
    #[error("scenario {scenario_id} already exists")]
    ScenarioAlreadyExists { scenario_id: String },
    #[error("unknown scenario {scenario_id}")]
    UnknownScenario { scenario_id: String },
    #[error("unknown scenario override {scenario_id}:{node}")]
    UnknownScenarioOverride { scenario_id: String, node: NodeKey },
    #[error("unsupported scenario override value for {scenario_id}: {detail}")]
    UnsupportedScenarioOverrideValue { scenario_id: String, detail: String },
    #[error("sweep {sweep_id} already exists")]
    SweepAlreadyExists { sweep_id: String },
    #[error("unknown sweep {sweep_id}")]
    UnknownSweep { sweep_id: String },
    #[error("sweep {sweep_id} has duplicate point {point_id}")]
    DuplicateSweepPoint { sweep_id: String, point_id: String },
    #[error("sweep {sweep_id} requires at least one point")]
    EmptySweep { sweep_id: String },
    #[error("engine rejected the intent: {0}")]
    EngineRejected(String),
    #[error("intent is forbidden for the {persona} persona")]
    Forbidden { persona: String },
    #[error("host failed to dispatch the intent: {0}")]
    HostFailure(String),
    /// The active document model family does not support this intent — e.g.
    /// `CreateScenario` on a `Workbook` session. Host-core produces this typed
    /// receipt per intent (proof doc §Model-Neutral Sessions); the affordance is
    /// capability-gated in skins so it is normally never shown, but the receipt
    /// exists for transports and audit. `intent` names the rejected intent kind
    /// and `model` names the family that rejected it.
    #[error("intent {intent} is not supported by the {model} document model")]
    UnsupportedByModel { intent: String, model: String },
}

/// One audited entry in the dispatcher's intent log (tenet 9): the intent,
/// its outcome, and the governance/projection context it executed under.
/// Serializable so a session's log can be exported and replayed elsewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentRecord {
    pub seq: u64,
    pub intent: WorkspaceIntent,
    pub accepted: bool,
    pub error: Option<IntentError>,
    pub transaction_id: Option<String>,
    pub produced_revision: Option<String>,
    /// The published value epoch AFTER this intent executed.
    pub value_epoch: u64,
    /// The persona that governed this dispatch.
    pub persona: crate::permissions::Persona,
}

/// Outcome of replaying a recorded log against a dispatcher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplayOutcome {
    pub dispatched: usize,
    pub accepted: usize,
    /// Sequence numbers whose acceptance differed from the recording — a
    /// non-empty list means the replay target diverged from the original
    /// session (different fixture, engine version, or non-determinism).
    pub mismatches: Vec<u64>,
}

/// Replay a recorded intent log, in order, against a dispatcher.
///
/// Every record is re-dispatched — including originally-rejected ones, which
/// must reject again for the replay to be faithful. Determinism is the
/// engine's published contract; this function only measures it.
pub fn replay(records: &[IntentRecord], dispatcher: &dyn Dispatcher) -> ReplayOutcome {
    let mut outcome = ReplayOutcome::default();
    for record in records {
        let receipt = dispatcher.dispatch(record.intent.clone());
        outcome.dispatched += 1;
        if receipt.accepted {
            outcome.accepted += 1;
        }
        if receipt.accepted != record.accepted {
            outcome.mismatches.push(record.seq);
        }
    }
    outcome
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceDeltaChange {
    FullReset,
    Structural(StructuralDeltaProjection),
    NodesChanged(Vec<NodeKey>),
    ValuesChanged(Vec<NodeValueDeltaProjection>),
    DepsChanged(Vec<DependencyDeltaProjection>),
    CalcRun(CalcRunProjection),
    ClipboardChanged(Option<ClipboardProjection>),
    FormulaReferenceInserted(FormulaReferenceInsertionProjection),
    CandidateChanged(CandidateProjection),
    CandidateRemoved(String),
    ScenarioChanged(ScenarioProjection),
    ScenarioRemoved(String),
    SweepChanged(SweepProjection),
    SweepRemoved(String),
    /// A grid-backed node's windowed projection changed (cells recomputed, or the
    /// interest window moved). Carries the complete new windowed grid projection,
    /// so the mirror applies it in place.
    GridChanged(GridProjection),
    /// Only a grid's overlay descriptors changed (the cell window held steady).
    /// The narrow path: ships just the new bundle + epoch so an overlay-only tick
    /// does not force the whole cell window through the channel, and the mirror
    /// patches `overlays`/`overlay_epoch` in place without disturbing the cells.
    GridOverlaysChanged {
        grid_node_id: NodeId,
        overlays: GridOverlayBundle,
        overlay_epoch: u64,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralDeltaProjection {
    pub added: Vec<NodeKey>,
    pub removed: Vec<NodeKey>,
    pub changed: Vec<NodeKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeValueDeltaProjection {
    pub node: NodeKey,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyDeltaProjection {
    pub owner: NodeKey,
    pub kinds: Vec<DependencyKindProjection>,
}

/// The only path through which a skin may ask the host to change anything
/// outside its own typed state.
///
/// The skeleton ships an `InMemoryDispatcher` used by tests; the host
/// crate wires a real dispatcher backed by direct OxCalc context for the live
/// shell.
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, intent: WorkspaceIntent) -> IntentReceipt;
}

#[cfg(test)]
mod serde_round_trip_tests {
    use super::*;

    fn round_trip<T>(value: &T) -> T
    where
        T: Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_string(value).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn structural_move_node_intent_round_trips() {
        let intent = WorkspaceIntent::MoveNode {
            node: NodeId::new("Accounts.2005.Q1.Margin"),
            new_parent: Some(NodeId::new("Accounts.2006")),
            new_index: Some(2),
        };
        assert_eq!(round_trip(&intent), intent);
    }

    #[test]
    fn add_candidate_node_intent_round_trips() {
        let intent = WorkspaceIntent::AddCandidateNode {
            handle: "cand-7".to_string(),
            parent: Some(NodeKey::new("tree-node:42")),
            symbol: "Margin".to_string(),
            initial: InitialNodeContentProjection::Literal {
                content: "=Revenue - Costs".to_string(),
            },
            is_meta: false,
        };
        assert_eq!(round_trip(&intent), intent);
    }

    #[test]
    fn create_scenario_sweep_intent_round_trips() {
        let intent = WorkspaceIntent::CreateScenarioSweep {
            sweep_id: "sweep-1".to_string(),
            name: "Discount rate".to_string(),
            base_scenario_id: Some("scenario-base".to_string()),
            input_node: NodeKey::new("tree-node:9"),
            points: vec![
                SweepPointInput {
                    point_id: "p1".to_string(),
                    label: "Low".to_string(),
                    value: NodeValueProjection::Number {
                        raw: "0.05".to_string(),
                        display: "5%".to_string(),
                    },
                },
                SweepPointInput {
                    point_id: "p2".to_string(),
                    label: "High".to_string(),
                    value: NodeValueProjection::Number {
                        raw: "0.12".to_string(),
                        display: "12%".to_string(),
                    },
                },
            ],
        };
        assert_eq!(round_trip(&intent), intent);
    }

    #[test]
    fn candidate_rebase_conflict_error_round_trips() {
        let error = IntentError::CandidateRebaseConflict {
            handle: "cand-7".to_string(),
            basis_revision_id: "rev-10".to_string(),
            current_revision_id: "rev-12".to_string(),
            overlapping_nodes: vec![NodeKey::new("tree-node:3"), NodeKey::new("tree-node:8")],
        };
        assert_eq!(round_trip(&error), error);
    }

    #[test]
    fn workspace_delta_with_mixed_changes_round_trips() {
        let delta = WorkspaceDelta {
            from_seq: 41,
            to_seq: 42,
            changes: vec![
                WorkspaceDeltaChange::FullReset,
                WorkspaceDeltaChange::Structural(StructuralDeltaProjection {
                    added: vec![NodeKey::new("tree-node:1")],
                    removed: vec![NodeKey::new("tree-node:2")],
                    changed: vec![NodeKey::new("tree-node:3")],
                }),
                WorkspaceDeltaChange::NodesChanged(vec![NodeKey::new("tree-node:4")]),
                WorkspaceDeltaChange::ValuesChanged(vec![NodeValueDeltaProjection {
                    node: NodeKey::new("tree-node:5"),
                    value: NodeValueProjection::Logical {
                        value: true,
                        display: "TRUE".to_string(),
                    },
                }]),
                WorkspaceDeltaChange::DepsChanged(vec![DependencyDeltaProjection {
                    owner: NodeKey::new("tree-node:6"),
                    kinds: vec![
                        DependencyKindProjection::StaticDirect,
                        DependencyKindProjection::ShapeTopology,
                    ],
                }]),
                WorkspaceDeltaChange::ClipboardChanged(None),
                WorkspaceDeltaChange::FormulaReferenceInserted(
                    FormulaReferenceInsertionProjection {
                        node: NodeKey::new("tree-node:7"),
                        target: FormulaReferenceInsertionTarget::HostReferenceCollection {
                            base: Some(NodeKey::new("tree-node:8")),
                            collection_family: "children".to_string(),
                        },
                        inserted_text: "@CHILDREN".to_string(),
                        updated_formula_text: "=SUM(@CHILDREN)".to_string(),
                        applied_start: 5,
                        applied_len: 9,
                    },
                ),
                WorkspaceDeltaChange::CandidateRemoved("cand-7".to_string()),
                WorkspaceDeltaChange::ScenarioRemoved("scenario-1".to_string()),
                WorkspaceDeltaChange::SweepRemoved("sweep-1".to_string()),
            ],
        };
        assert_eq!(round_trip(&delta), delta);
    }
}

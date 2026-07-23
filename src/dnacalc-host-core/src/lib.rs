//! DNA Calc host-core — the Leptos-free reference host (the Gap-4
//! SessionEngine crate).
//!
//! Host-core owns the document session and the model→intent seam between the
//! Skin IR wire protocol ([`dnacalc_skin_ir`]) and the OxCalc document surface
//! ([`oxcalc_core`]). It carries **no Leptos dependency anywhere in its tree**
//! (the TC-gate, pinned by `dnacalc-arch-gates`'s `tc_gate_host_core_has_no_leptos`
//! over `cargo tree`), so a worker, a CLI/MCP host, or the browser UI can each
//! drive the same session logic without pulling in a UI framework.
//!
//! ## Model-neutral sessions
//!
//! The common abstraction over document model families is a **closed enum, not
//! a trait** ([`DocumentSession`]): a general tree workspace
//! ([`DocumentSession::RichTree`]) or a strict-Excel workbook
//! ([`DocumentSession::Workbook`]). The two share almost no lifecycle beyond
//! "consume a `WorkspaceIntent`, publish a projection" — that pair is the common
//! surface for now; a trait is extracted only when a third family exists
//! (proof doc §Model-Neutral Sessions). Host-core matches per intent and returns
//! a typed [`IntentError::UnsupportedByModel`] receipt for an intent a family
//! does not support (e.g. `CreateScenario` on a workbook).
//!
//! ## H2 scope
//!
//! H2 stands up the crate seam: the [`DocumentSession`] enum, [`WorkbookSession`]
//! over one [`OxCalcDocumentContext`] (create workspace + add sheets + the
//! `set_grid_cell_value` write path), the [`HostCommand`] skeleton, the
//! [`ProjectionPublisher`] publication seam, and the Send/Sync audit below. The
//! universal `EnterGridCell` authored-entry verb, the tree-session migration
//! into host-core, xlsx, and the worker are all out of H2 scope.
//!
//! ## H6 scope
//!
//! H6 wires the cell-entry family end to end: `WorkspaceIntent::EnterGridCell`/
//! `ClearGridCell` dispatch (this module), the three-way
//! `GridCellEntered { outcome }` receipt payload, and the `present.rs` A.4
//! error-presentation map from a rejected write to a typed [`IntentError`].
//! Cross-sheet poll fan-out and skins are out of H6 scope (H7 and later).

pub mod calc;
pub mod command;
pub mod defined_names;
pub mod demo;
pub mod grid_publication;
pub mod persistence;
pub mod skin_protocol;
pub mod present;
pub mod tree;
pub mod workbook;

pub use calc::{
    calc_mode_from_projection, calc_mode_projection, present_calc_rejection,
    value_provenance_projection,
};
pub use command::{HostCommand, HostCommandOutcome, ProjectionPublisher, RecordingPublisher};
pub use defined_names::{
    DefinedNameTargetIntentInput, NAMES_BACKING_SHEET, present_defined_name_rejection,
};
pub use demo::build_demo_workbook;
pub use grid_publication::{
    grid_authored_cell_projection, grid_overlay_bundle_for, grid_projection_for,
    grid_value_projection,
};
pub use present::present_grid_entry_rejection;
pub use persistence::LocalFileSkinStatePersistenceStore;
pub use skin_protocol::SkinProtocolSession;
pub use workbook::{
    WorkbookSession, WorkbookSessionError, parse_sheet_grid_node_id, sheet_grid_node_id,
};

use dnacalc_skin_ir::{
    DefinedNameTargetIntent, GridEntryOutcomeProjection, IntentError, IntentReceipt,
    NodeValueProjection, WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent, WorkspaceState,
};
use oxcalc_core::consumer::GridCellEntryOutcome;
use oxfunc_core::value::CalcValue;

// Re-export the engine document surface name the enum is built over, so callers
// name it through host-core rather than reaching into `oxcalc_core` directly.
pub use oxcalc_core::consumer::OxCalcDocumentContext;

/// The general-tree document model family — the seam placeholder for the
/// existing `TreeWorkspaceSession` (scenarios, sweeps, revision cursors,
/// `.dnatree` persistence), which lives in `dnatreecalc-host` today.
///
/// H2's NON-goals forbid a tree-session refactor "beyond the enum seam", and the
/// full `TreeWorkspaceSession` is reachable only through `dnatreecalc-host`,
/// which unconditionally links Leptos — pulling it into host-core would break
/// the no-Leptos gate. So the `RichTree` arm is a leptos-free **marker** in H2:
/// it establishes the closed-enum seam and gives the model-family dispatch a
/// second arm to distinguish, without moving any tree-session code. Migrating
/// the tree session into host-core is a later bead.
#[derive(Debug, Default)]
pub struct RichTreeSession {
    _seam: (),
}

impl RichTreeSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A host document session: exactly one open document, of exactly one model
/// family. A closed enum (not a trait) per the model-neutral-sessions decision.
// `large_enum_variant`: the `RichTree` arm is a temporary 0-byte seam
// placeholder in H2 (the real tree session lives in `dnatreecalc-host`); once
// the tree session migrates into host-core the two variants balance. Boxing the
// workbook now would diverge from that end state and hand every caller an extra
// indirection for the enum's only live arm.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
#[non_exhaustive]
pub enum DocumentSession {
    /// The general-tree workspace model (seam placeholder in H2 — see
    /// [`RichTreeSession`]).
    RichTree(RichTreeSession),
    /// The strict-Excel workbook model, backed by one [`OxCalcDocumentContext`].
    Workbook(WorkbookSession),
}

impl DocumentSession {
    /// The model family's stable name, for diagnostics and the
    /// [`IntentError::UnsupportedByModel`] receipt.
    #[must_use]
    pub fn model_name(&self) -> &'static str {
        match self {
            DocumentSession::RichTree(_) => "RichTree",
            DocumentSession::Workbook(_) => "Workbook",
        }
    }

    /// The full read-side [`WorkspaceState`] projection for the open document.
    ///
    /// A `Workbook` session projects its whole workspace (every grid-backed
    /// sheet, the defined-name catalog, and calc state) via
    /// [`WorkbookSession::snapshot`]; a `RichTree` session is a Leptos-free
    /// seam placeholder here (the real tree session lives in
    /// `dnatreecalc-host`) and projects the empty default. The workbook arm
    /// falls back to `WorkspaceState::default()` on the internal-invariant
    /// error `snapshot()` can surface, so the mount surface is infallible: an
    /// initial projection never fails a caller, it degrades to empty.
    #[must_use]
    pub fn snapshot(&self) -> WorkspaceState {
        match self {
            DocumentSession::Workbook(session) => session.snapshot().unwrap_or_default(),
            DocumentSession::RichTree(_) => WorkspaceState::default(),
        }
    }

    /// Route a `WorkspaceIntent` to the session's model family.
    ///
    /// H6 wires the universal cell-entry family (`EnterGridCell`/
    /// `ClearGridCell`) for `Workbook` sessions — the sole executable lane
    /// this dispatcher carries so far. Every other intent, and the entry
    /// family on a `RichTree` session (a seam placeholder with no grid at
    /// all), is answered with a typed [`IntentError::UnsupportedByModel`]
    /// receipt. This is the per-intent model-family gate the proof doc
    /// specifies; later beads (H4/H5/H7/…) attach their own lanes the same
    /// way.
    #[must_use]
    pub fn dispatch(&mut self, intent: WorkspaceIntent) -> IntentReceipt {
        match (self, intent) {
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::EnterGridCell {
                    grid,
                    row,
                    col,
                    text,
                },
            ) => dispatch_enter_grid_cell(session, &grid, row, col, &text),
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::ClearGridCell { grid, row, col },
            ) => dispatch_clear_grid_cell(session, &grid, row, col),
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::SetDefinedName {
                    scope,
                    name,
                    target,
                },
            ) => dispatch_set_defined_name(session, &scope, name, target),
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::RenameDefinedName {
                    scope,
                    old_name,
                    new_name,
                },
            ) => dispatch_rename_defined_name(session, &scope, &old_name, new_name),
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::DeleteDefinedName { scope, name },
            ) => dispatch_delete_defined_name(session, &scope, &name),
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::CreateNamedValue { name, value_text },
            ) => dispatch_create_named_value(session, &name, &value_text),
            (DocumentSession::Workbook(session), WorkspaceIntent::SetCalcMode { mode }) => {
                dispatch_set_calc_mode(session, mode)
            }
            (DocumentSession::Workbook(session), WorkspaceIntent::Recalculate) => {
                dispatch_recalculate(session)
            }
            (DocumentSession::Workbook(session), WorkspaceIntent::AddSheet { name }) => {
                dispatch_add_sheet(session, name)
            }
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::RenameSheet { grid, new_name },
            ) => dispatch_rename_sheet(session, &grid, &new_name),
            (DocumentSession::Workbook(session), WorkspaceIntent::DeleteSheet { grid }) => {
                dispatch_delete_sheet(session, &grid)
            }
            (
                DocumentSession::Workbook(session),
                WorkspaceIntent::MoveSheet { grid, new_position },
            ) => dispatch_move_sheet(session, &grid, new_position),
            (session, intent) => IntentReceipt::rejected(IntentError::UnsupportedByModel {
                intent: workspace_intent_kind(&intent).to_string(),
                model: session.model_name().to_string(),
            }),
        }
    }
}

/// Resolve an `EnterGridCell`/`ClearGridCell` intent's `grid: NodeId` to the
/// workbook's engine sheet node, or the typed `UnsupportedByModel` shape a
/// stale/unknown grid id gets (never a panic on an unrecognized address).
fn resolve_sheet_node(
    grid: &dnacalc_skin_ir::NodeId,
) -> Result<oxcalc_core::structural::TreeNodeId, IntentError> {
    workbook::parse_sheet_grid_node_id(grid).ok_or_else(|| IntentError::GenericEngineRejection {
        debug: format!("unknown grid node id {grid:?}"),
    })
}

fn dispatch_enter_grid_cell(
    session: &mut WorkbookSession,
    grid: &dnacalc_skin_ir::NodeId,
    row: u32,
    col: u32,
    text: &str,
) -> IntentReceipt {
    let sheet = match resolve_sheet_node(grid) {
        Ok(sheet) => sheet,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.enter_grid_cell(sheet, row, col, text) {
        Ok(outcome) => grid_cell_entered_receipt(grid, row, col, &outcome),
        Err(error) => IntentReceipt::rejected(present_grid_entry_rejection(&error)),
    }
}

fn dispatch_clear_grid_cell(
    session: &mut WorkbookSession,
    grid: &dnacalc_skin_ir::NodeId,
    row: u32,
    col: u32,
) -> IntentReceipt {
    let sheet = match resolve_sheet_node(grid) {
        Ok(sheet) => sheet,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.clear_grid_cell(sheet, row, col) {
        Ok(_view) => {
            let outcome = GridEntryOutcomeProjection::Cleared;
            IntentReceipt::accepted().with_delta(WorkspaceDelta {
                from_seq: 0,
                to_seq: 0,
                changes: vec![WorkspaceDeltaChange::GridCellEntered {
                    grid_node_id: grid.clone(),
                    row,
                    col,
                    outcome,
                }],
            })
        }
        Err(error) => IntentReceipt::rejected(present_grid_entry_rejection(&error)),
    }
}

/// Resolve the sheet a defined-name write anchors on: `Sheet(grid)` scope
/// anchors on that exact sheet; `Workbook` scope anchors on any grid-backed
/// sheet (a workbook-scoped name is authored on one sheet's grid but visible
/// workbook-wide, per `defined_names.rs`'s doc comment) — the workbook's
/// first sheet in sheet order, since H4's intents (§A.2) carry no explicit
/// sheet field for the workbook-scope case. A workbook with no sheets yet is
/// the typed `GenericEngineRejection` fallback, never a panic.
fn resolve_defined_name_anchor(
    session: &WorkbookSession,
    scope: &dnacalc_skin_ir::DefinedNameScopeProjection,
) -> Result<oxcalc_core::structural::TreeNodeId, IntentError> {
    use dnacalc_skin_ir::DefinedNameScopeProjection;
    match scope {
        DefinedNameScopeProjection::Sheet(grid) => resolve_sheet_node(grid),
        DefinedNameScopeProjection::Workbook => session
            .sheets()
            .ok()
            .and_then(|rows| rows.first().map(|row| row.node_id))
            .ok_or_else(|| IntentError::GenericEngineRejection {
                debug: "workbook-scoped defined name requires at least one sheet".to_string(),
            }),
    }
}

fn defined_name_target_intent_input(
    target: DefinedNameTargetIntent,
) -> DefinedNameTargetIntentInput {
    match target {
        DefinedNameTargetIntent::Static(rect) => DefinedNameTargetIntentInput::Static(rect),
        DefinedNameTargetIntent::Dynamic { source_text } => {
            DefinedNameTargetIntentInput::Dynamic { source_text }
        }
    }
}

/// Build the `DefinedNamesChanged` delta carrying the workbook's complete,
/// freshly-read catalog (§A.3: complete-replacement patch, matching
/// `CalcRun`/`ClipboardChanged`).
fn defined_names_changed_receipt(session: &WorkbookSession) -> IntentReceipt {
    match session.defined_names() {
        Ok(catalog) => IntentReceipt::accepted().with_delta(WorkspaceDelta {
            from_seq: 0,
            to_seq: 0,
            changes: vec![WorkspaceDeltaChange::DefinedNamesChanged(catalog)],
        }),
        Err(error) => IntentReceipt::rejected(present_defined_name_rejection(&error)),
    }
}

fn dispatch_set_defined_name(
    session: &mut WorkbookSession,
    scope: &dnacalc_skin_ir::DefinedNameScopeProjection,
    name: String,
    target: DefinedNameTargetIntent,
) -> IntentReceipt {
    let anchor = match resolve_defined_name_anchor(session, scope) {
        Ok(anchor) => anchor,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.set_defined_name(
        anchor,
        scope.clone(),
        name,
        defined_name_target_intent_input(target),
    ) {
        Ok(()) => defined_names_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_defined_name_rejection(&error)),
    }
}

fn dispatch_rename_defined_name(
    session: &mut WorkbookSession,
    scope: &dnacalc_skin_ir::DefinedNameScopeProjection,
    old_name: &str,
    new_name: String,
) -> IntentReceipt {
    let anchor = match resolve_defined_name_anchor(session, scope) {
        Ok(anchor) => anchor,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.rename_defined_name(anchor, scope.clone(), old_name, new_name) {
        Ok(()) => defined_names_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_defined_name_rejection(&error)),
    }
}

fn dispatch_delete_defined_name(
    session: &mut WorkbookSession,
    scope: &dnacalc_skin_ir::DefinedNameScopeProjection,
    name: &str,
) -> IntentReceipt {
    let anchor = match resolve_defined_name_anchor(session, scope) {
        Ok(anchor) => anchor,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.delete_defined_name(anchor, scope.clone(), name) {
        Ok(()) => defined_names_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_defined_name_rejection(&error)),
    }
}

/// Dispatch `CreateNamedValue` (N3's atomic `+ name`): host-core owns the whole
/// named-value creation ([`WorkbookSession::create_named_value`] — allocate the
/// `_names` backing cell, write the value, define the name workbook-wide), so
/// the skin never guesses a backing cell. On success it surfaces the same
/// `DefinedNamesChanged` receipt the other defined-name verbs produce; on any
/// step's rejection, the typed defined-name rejection map.
fn dispatch_create_named_value(
    session: &mut WorkbookSession,
    name: &str,
    value_text: &str,
) -> IntentReceipt {
    match session.create_named_value(name, value_text) {
        Ok(()) => defined_names_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_defined_name_rejection(&error)),
    }
}

/// Build the `CalcStateChanged` delta carrying the workbook's freshly-read
/// calc-mode/recalc projection (§A.3: complete-replacement patch, matching
/// `DefinedNamesChanged`). `last_recalc_tick` is the tick a just-completed
/// `Recalculate` minted (`None` for a plain `SetCalcMode` write, which mints
/// no tick of its own — a scheduling fact never a value fact, D1 §5).
fn calc_state_changed_receipt(
    session: &WorkbookSession,
    last_recalc_tick: Option<u64>,
) -> IntentReceipt {
    match session.workbook_calc_projection(last_recalc_tick) {
        Ok(projection) => IntentReceipt::accepted().with_delta(WorkspaceDelta {
            from_seq: 0,
            to_seq: 0,
            changes: vec![WorkspaceDeltaChange::CalcStateChanged(projection)],
        }),
        Err(error) => IntentReceipt::rejected(present_calc_rejection(&error)),
    }
}

fn dispatch_set_calc_mode(
    session: &mut WorkbookSession,
    mode: dnacalc_skin_ir::CalcModeProjection,
) -> IntentReceipt {
    match session.set_calc_mode(mode) {
        Ok(()) => calc_state_changed_receipt(session, None),
        Err(error) => IntentReceipt::rejected(present_calc_rejection(&error)),
    }
}

/// Dispatch `Recalculate` for a workbook session (H5, §A.2: routes to
/// `recalculate_workbook`). Acceptance (3): with nothing dirty, the outcome's
/// `drained_any()` is `false` and no tick is minted — the receipt still
/// carries a `CalcStateChanged` delta (the projection is cheap to re-read and
/// a caller may not have seen the mode before), but no `GridChanged` is
/// emitted (cross-sheet fan-out on a genuine drain is H7's scope; a no-op
/// recalc has nothing to fan out regardless).
fn dispatch_recalculate(session: &mut WorkbookSession) -> IntentReceipt {
    match session.recalculate() {
        Ok(outcome) => calc_state_changed_receipt(session, outcome.tick_id),
        Err(error) => IntentReceipt::rejected(present_calc_rejection(&error)),
    }
}

// ---------------------------------------------------------------------------
// Sheet-lifecycle dispatch (Phase 1 Part A): add/rename/delete/move sheets.
// Mirrors the defined-name dispatch pattern — each verb succeeds into a
// complete-replacement `SheetsChanged` delta built from a fresh read, or a
// typed rejection.
// ---------------------------------------------------------------------------

/// Build the `SheetsChanged` delta carrying the workbook's complete,
/// freshly-read sheet list (§A.3: complete-replacement patch, matching
/// `DefinedNamesChanged`/`CalcStateChanged`).
fn sheets_changed_receipt(session: &WorkbookSession) -> IntentReceipt {
    match session.sheet_projections() {
        Ok(sheets) => IntentReceipt::accepted().with_delta(WorkspaceDelta {
            from_seq: 0,
            to_seq: 0,
            changes: vec![WorkspaceDeltaChange::SheetsChanged(sheets)],
        }),
        Err(error) => IntentReceipt::rejected(present_sheet_rejection(&error)),
    }
}

/// Map a rejected sheet-lifecycle write to its typed [`IntentError`]. The
/// engine's sheet errors (`SheetPositionOutOfRange`, `SheetHasNonMetaChildren`,
/// a duplicate name, an unknown node) have no dedicated skin-IR variant yet, so
/// they present as the documented `GenericEngineRejection` fallback — the same
/// decision `present.rs` makes for unmapped engine errors, never a panic.
fn present_sheet_rejection(error: &WorkbookSessionError) -> IntentError {
    IntentError::GenericEngineRejection {
        debug: format!("{error:?}"),
    }
}

/// The next default sheet name (`Sheet{n+1}`) for an `AddSheet { name: None }`,
/// computed from the current sheet count (a fresh workbook with no sheets
/// yields `Sheet1`).
fn default_sheet_name(session: &WorkbookSession) -> String {
    let count = session.sheets().map(|rows| rows.len()).unwrap_or(0);
    format!("Sheet{}", count + 1)
}

fn dispatch_add_sheet(session: &mut WorkbookSession, name: Option<String>) -> IntentReceipt {
    let name = name.unwrap_or_else(|| default_sheet_name(session));
    match session.add_sheet(name) {
        Ok(_node) => sheets_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_sheet_rejection(&error)),
    }
}

fn dispatch_rename_sheet(
    session: &mut WorkbookSession,
    grid: &dnacalc_skin_ir::NodeId,
    new_name: &str,
) -> IntentReceipt {
    let sheet = match resolve_sheet_node(grid) {
        Ok(sheet) => sheet,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.rename_sheet(sheet, new_name) {
        Ok(()) => sheets_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_sheet_rejection(&error)),
    }
}

fn dispatch_delete_sheet(
    session: &mut WorkbookSession,
    grid: &dnacalc_skin_ir::NodeId,
) -> IntentReceipt {
    let sheet = match resolve_sheet_node(grid) {
        Ok(sheet) => sheet,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.delete_sheet(sheet) {
        Ok(()) => sheets_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_sheet_rejection(&error)),
    }
}

fn dispatch_move_sheet(
    session: &mut WorkbookSession,
    grid: &dnacalc_skin_ir::NodeId,
    new_position: u32,
) -> IntentReceipt {
    let sheet = match resolve_sheet_node(grid) {
        Ok(sheet) => sheet,
        Err(error) => return IntentReceipt::rejected(error),
    };
    match session.move_sheet(sheet, new_position) {
        Ok(()) => sheets_changed_receipt(session),
        Err(error) => IntentReceipt::rejected(present_sheet_rejection(&error)),
    }
}

/// Build the accepted `GridCellEntered` receipt (§A.2's verb-façade row) from
/// the engine's three-way [`GridCellEntryOutcome`], mirroring its
/// literal/formula/cleared value(s) into the wire projection.
fn grid_cell_entered_receipt(
    grid: &dnacalc_skin_ir::NodeId,
    row: u32,
    col: u32,
    outcome: &GridCellEntryOutcome,
) -> IntentReceipt {
    let projected = match outcome {
        GridCellEntryOutcome::Literal { value, .. } => GridEntryOutcomeProjection::Literal {
            value: calc_value_projection(value),
        },
        GridCellEntryOutcome::Formula {
            unresolved_names,
            view,
            ..
        } => GridEntryOutcomeProjection::Formula {
            unresolved_names: unresolved_names.clone(),
            value: view
                .cells
                .iter()
                .find(|cell| cell.address.row == row && cell.address.col == col)
                .map(|cell| calc_value_projection(&cell.value))
                .unwrap_or(NodeValueProjection::Unevaluated),
        },
        GridCellEntryOutcome::Cleared { .. } => GridEntryOutcomeProjection::Cleared,
    };
    IntentReceipt::accepted().with_delta(WorkspaceDelta {
        from_seq: 0,
        to_seq: 0,
        changes: vec![WorkspaceDeltaChange::GridCellEntered {
            grid_node_id: grid.clone(),
            row,
            col,
            outcome: projected,
        }],
    })
}

/// A `CalcValue` -> `NodeValueProjection` rendering for the `GridCellEntered`
/// receipt payload (H6's entry receipt's literal/formula value). Routes through
/// the single grid-value projection host-core owns
/// ([`grid_publication::grid_value_projection`]) so the entry-receipt path and
/// the full snapshot path can never disagree about a value's shape.
fn calc_value_projection(value: &CalcValue) -> NodeValueProjection {
    grid_publication::grid_value_projection(value)
}

/// A stable, human-readable kind name for a `WorkspaceIntent`, used in the
/// [`IntentError::UnsupportedByModel`] receipt. Covers the families H2 must name
/// (notably the scenario family, per acceptance assertion 3) and falls back to a
/// generic label for the rest — the receipt's `model` field carries the
/// dispositive fact (which family rejected), so an exhaustive per-variant name
/// is not required in H2.
fn workspace_intent_kind(intent: &WorkspaceIntent) -> &'static str {
    match intent {
        WorkspaceIntent::CreateScenario { .. } => "CreateScenario",
        WorkspaceIntent::CreateScenarioFromCandidate { .. } => "CreateScenarioFromCandidate",
        WorkspaceIntent::ActivateScenario { .. } => "ActivateScenario",
        WorkspaceIntent::DeleteScenario { .. } => "DeleteScenario",
        WorkspaceIntent::SetScenarioOverride { .. } => "SetScenarioOverride",
        WorkspaceIntent::ClearScenarioOverride { .. } => "ClearScenarioOverride",
        WorkspaceIntent::CreateScenarioSweep { .. } => "CreateScenarioSweep",
        _ => "WorkspaceIntent",
    }
}

/// A [`HostCommand`] executor over a document session. H2 executes only the
/// `DispatchWorkspaceIntent` arm (the enum's sole H2 variant), routing through
/// [`DocumentSession::dispatch`].
impl DocumentSession {
    pub fn execute(&mut self, command: HostCommand) -> HostCommandOutcome {
        match command {
            HostCommand::DispatchWorkspaceIntent(intent) => {
                HostCommandOutcome::Dispatched(self.dispatch(intent))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Send/Sync audit (bead-required, W011 §"Send/Sync decision").
//
// The W011 decision hinged on whether `OxCalcDocumentContext` is `Send`.
//
// GROUND TRUTH (verified 2026-07-05 by attempting a `Send`/`Sync` static
// assertion — see the removed assertions in the bead's diff history):
// `OxCalcDocumentContext` is **NEITHER `Send` NOR `Sync`**. It transitively
// embeds `oxfunc_core::value::CalcValue`, whose `RichValue` payload is held
// behind a non-atomic `Rc<RichValue>`; the workspace-state map additionally
// holds a `NodeRef<Owned, ...>` handle that is itself `!Sync`. `WorkbookSession`
// and `DocumentSession` inherit `!Send + !Sync` from the context.
//
// W011 DISPOSITION (the `!Send` branch the proof doc pre-authored): host-core
// sessions are **single-threaded values**, not `Send` cross-thread handles.
// The existing `dnatreecalc-host` already reflects this — its `HOST_SESSIONS`
// registry is `thread_local` precisely because these engine types are `!Send`.
// So:
//   * Do NOT expose a `Dispatcher: Send + Sync` impl backed directly by a live
//     `DocumentSession`; a session stays on its owning thread (the wasm main
//     thread, or a single worker thread that owns its own context).
//   * A worker transport owns the context inside the worker thread and speaks
//     the serde wire protocol across `postMessage` — the context never crosses
//     a thread boundary as a value. This matches A.5's "engine placement is a
//     shell concern; front-end code binds only the `Dispatcher` trait + delta
//     mirror" and is the model-neutral seam H10 builds on.
//   * The `thread_local` registry is NOT re-invented in host-core; host-core
//     hands out plain owned `DocumentSession` values and lets the transport
//     decide affinity.
//
// The compile-time proof of `!Send` is the *absence* of a `Send` bound anywhere
// in this crate's public API — no `DocumentSession` field or return type claims
// `Send`, so a future edit that assumes it will fail to compile at the use site.
// The correct-direction static check we CAN enforce: the wire-protocol receipt
// type IS `Send + Sync` (it is pure serde data), which is what actually crosses
// the worker boundary.
// ---------------------------------------------------------------------------
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    // The receipt is pure serde data and DOES cross the thread boundary.
    assert_send_sync::<IntentReceipt>();
    assert_send_sync::<IntentError>();
    assert_send_sync::<HostCommand>();
    assert_send_sync::<HostCommandOutcome>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    /// Acceptance (2): create a workbook session → `sheets()` projects one
    /// sheet → `set_grid_cell_value(A1, 7)` → snapshot shows `7`.
    #[test]
    fn workbook_session_add_sheet_write_a1_reads_back_seven() {
        let mut session = WorkbookSession::create("workbook:h2-accept").unwrap();

        // A freshly-created workbook has no sheets yet; adding one projects
        // exactly one sheet through `sheets()`.
        assert!(session.sheets().unwrap().is_empty());
        let sheet = session.add_sheet("Sheet1").unwrap();
        let rows = session.sheets().unwrap();
        assert_eq!(rows.len(), 1, "exactly one sheet after add_sheet");
        assert_eq!(rows[0].display_name, "Sheet1");
        assert_eq!(rows[0].node_id, sheet);
        assert!(rows[0].grid_backed, "added sheet is grid-backed");

        // Write A1 = 7 via the H2 write path (`set_grid_cell_value`, row 1 /
        // col 1 = A1), then read the published value back from the snapshot.
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();
        let a1 = session.grid_cell_value(sheet, 1, 1).unwrap();
        assert_eq!(
            a1.and_then(|value| value.as_number()),
            Some(7.0),
            "snapshot shows 7 at A1"
        );
    }

    /// Acceptance (3): `IntentError::UnsupportedByModel` receipt for
    /// `CreateScenario` on a Workbook session.
    #[test]
    fn create_scenario_on_workbook_is_unsupported_by_model() {
        let session = WorkbookSession::create("workbook:h2-unsupported").unwrap();
        let mut document = DocumentSession::Workbook(session);

        let receipt = document.dispatch(WorkspaceIntent::CreateScenario {
            scenario_id: "s1".to_string(),
            name: "Downside".to_string(),
            base_scenario_id: None,
        });

        assert!(!receipt.accepted, "workbook rejects CreateScenario");
        match receipt.error {
            Some(IntentError::UnsupportedByModel { intent, model }) => {
                assert_eq!(intent, "CreateScenario");
                assert_eq!(model, "Workbook");
            }
            other => panic!("expected UnsupportedByModel receipt, got {other:?}"),
        }
    }

    /// The publication seam records receipts a host publishes through it.
    #[test]
    fn recording_publisher_captures_published_receipts() {
        let publisher = RecordingPublisher::new();
        let mut document =
            DocumentSession::Workbook(WorkbookSession::create("workbook:h2-publish").unwrap());

        let outcome = document.execute(HostCommand::DispatchWorkspaceIntent(
            WorkspaceIntent::CreateScenario {
                scenario_id: "s1".to_string(),
                name: "Downside".to_string(),
                base_scenario_id: None,
            },
        ));
        let HostCommandOutcome::Dispatched(receipt) = outcome;
        publisher.publish(&receipt);

        let published = publisher.published();
        assert_eq!(published.len(), 1);
        assert!(!published[0].accepted);
    }

    // ------------------------------------------------------------------
    // H6 acceptance: cell-entry intents end-to-end.
    // ------------------------------------------------------------------

    fn workbook_with_one_sheet(workspace_id: &str) -> (DocumentSession, dnacalc_skin_ir::NodeId) {
        let mut session = WorkbookSession::create(workspace_id).unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        let grid = sheet_grid_node_id(sheet);
        (DocumentSession::Workbook(session), grid)
    }

    /// H6 acceptance (1), literal half: `EnterGridCell` with a plain number
    /// dispatches to `Literal`, and the receipt's projected value matches.
    #[test]
    fn enter_grid_cell_literal_dispatches_and_projects_value() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-literal");

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });

        assert!(receipt.accepted, "a plain number literal is accepted");
        match receipt.delta.changes.as_slice() {
            [
                WorkspaceDeltaChange::GridCellEntered {
                    grid_node_id,
                    row,
                    col,
                    outcome,
                },
            ] => {
                assert_eq!(*grid_node_id, grid);
                assert_eq!(*row, 1);
                assert_eq!(*col, 1);
                match outcome {
                    GridEntryOutcomeProjection::Literal { value } => assert_eq!(
                        *value,
                        NodeValueProjection::Number {
                            raw: "10".to_string(),
                            display: "10".to_string(),
                        }
                    ),
                    other => panic!("expected Literal outcome, got {other:?}"),
                }
            }
            other => panic!("expected exactly one GridCellEntered change, got {other:?}"),
        }
    }

    /// H6 acceptance (1) + (2), formula half: `EnterGridCell` with a formula
    /// referencing an as-yet-undefined name dispatches to `Formula`, and
    /// `unresolved_names` surfaces the undefined name on the receipt
    /// (acceptance assertion 2).
    #[test]
    fn enter_grid_cell_formula_dispatches_and_surfaces_unresolved_names() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-formula");

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "=TaxRate*2".to_string(),
        });

        assert!(
            receipt.accepted,
            "a formula referencing an undefined name is still accepted (it self-heals)"
        );
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::GridCellEntered { outcome, .. }] => match outcome {
                GridEntryOutcomeProjection::Formula {
                    unresolved_names, ..
                } => {
                    assert_eq!(
                        unresolved_names,
                        &vec!["TaxRate".to_string()],
                        "the undefined name surfaces on the Formula receipt"
                    );
                }
                other => panic!("expected Formula outcome, got {other:?}"),
            },
            other => panic!("expected exactly one GridCellEntered change, got {other:?}"),
        }
    }

    /// H6 acceptance (1), empty-clears half: committing empty text through
    /// `EnterGridCell` resolves to `Cleared` (Excel's empty-commit contract).
    #[test]
    fn enter_grid_cell_empty_text_dispatches_to_cleared() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-empty-clears");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: String::new(),
        });

        assert!(receipt.accepted, "committing empty text is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::GridCellEntered { outcome, .. }] => {
                assert!(matches!(outcome, GridEntryOutcomeProjection::Cleared));
            }
            other => panic!("expected exactly one GridCellEntered change, got {other:?}"),
        }
    }

    /// H6 acceptance (1), `ClearGridCell` half: clearing a literal cell
    /// directly also resolves to `Cleared`, and a re-read shows the cell
    /// authored-empty.
    #[test]
    fn clear_grid_cell_dispatches_to_cleared_and_authored_view_shows_empty() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-clear");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });

        let receipt = document.dispatch(WorkspaceIntent::ClearGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
        });

        assert!(
            receipt.accepted,
            "ClearGridCell on a literal cell is accepted"
        );
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::GridCellEntered { outcome, .. }] => {
                assert!(matches!(outcome, GridEntryOutcomeProjection::Cleared));
            }
            other => panic!("expected exactly one GridCellEntered change, got {other:?}"),
        }

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid).unwrap();
        let cells = session.grid_authored_cells(sheet, 1, 1, 1, 1).unwrap();
        let cell = cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        assert_eq!(
            cell.kind,
            dnacalc_skin_ir::GridAuthoredKindProjection::Empty,
            "the cleared cell's authored kind is Empty"
        );
    }

    /// H6 acceptance (1), rejected half: an invalid formula (`=1+`) is
    /// rejected with typed diagnostics carried on the receipt, AND a re-read
    /// of the authored view proves no mutation happened (the engine's
    /// no-mutation-on-diagnostics contract, asserted from the host-core
    /// dispatch boundary — not just the engine's own unit test).
    #[test]
    fn enter_grid_cell_rejected_formula_carries_diagnostics_and_does_not_mutate() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-rejected");
        // Seed A1 with a literal first, so the re-read below has a concrete
        // baseline the rejection must leave untouched.
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "7".to_string(),
        });

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "=1+".to_string(),
        });

        assert!(!receipt.accepted, "an unparseable formula is rejected");
        match receipt.error {
            Some(IntentError::GridEntryRejected { diagnostics }) => {
                assert!(
                    !diagnostics.is_empty(),
                    "the rejection receipt carries at least one typed diagnostic"
                );
            }
            other => panic!("expected GridEntryRejected, got {other:?}"),
        }

        // No mutation on Err: A1 is still the literal 7, not the rejected
        // formula text.
        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid).unwrap();
        let cells = session.grid_authored_cells(sheet, 1, 1, 1, 1).unwrap();
        let cell = cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        assert_eq!(
            cell.kind,
            dnacalc_skin_ir::GridAuthoredKindProjection::Literal,
            "A1's authored kind is unchanged (still Literal, not Formula)"
        );
        assert_eq!(
            cell.literal_text.as_deref(),
            Some("7"),
            "A1's literal text is unchanged by the rejected write"
        );
    }

    /// H6 acceptance (3), one A.4 table row: `GridCellNotEditable` (a spill
    /// follower) maps to a rejection carrying the classifier's anchor, never
    /// a panic.
    #[test]
    fn enter_grid_cell_on_spill_follower_is_rejected_with_anchor() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h6-spill-reject");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "=SEQUENCE(3,1)".to_string(),
        });

        // A2 is a spill-display follower of A1's spilling formula.
        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 2,
            col: 1,
            text: "99".to_string(),
        });

        assert!(
            !receipt.accepted,
            "writing into a spill follower is rejected"
        );
        match receipt.error {
            Some(IntentError::GridCellNotEditable { anchor }) => {
                assert_eq!(
                    anchor,
                    Some(dnacalc_skin_ir::GridCellRefProjection { row: 1, col: 1 }),
                    "the anchor is the spilling formula's own cell"
                );
            }
            other => panic!("expected GridCellNotEditable, got {other:?}"),
        }
    }

    /// H6 acceptance (3), the map's fallback row: an intent addressing an
    /// unknown/stale grid id is a typed rejection, never a panic.
    #[test]
    fn enter_grid_cell_on_unknown_grid_id_is_generic_rejection_never_panics() {
        let (mut document, _grid) = workbook_with_one_sheet("workbook:h6-unknown-grid");

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: dnacalc_skin_ir::NodeId::new("not-a-real-grid-id"),
            row: 1,
            col: 1,
            text: "1".to_string(),
        });

        assert!(!receipt.accepted);
        assert!(matches!(
            receipt.error,
            Some(IntentError::GenericEngineRejection { .. })
        ));
    }

    // ------------------------------------------------------------------
    // H4 acceptance: defined-names intents end-to-end.
    // ------------------------------------------------------------------

    use dnacalc_skin_ir::{
        DefinedNameScopeProjection, DefinedNameTargetProjection, GridRectProjection,
    };

    fn static_rect(
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    ) -> DefinedNameTargetIntent {
        DefinedNameTargetIntent::Static(GridRectProjection {
            top_row,
            left_col,
            bottom_row,
            right_col,
        })
    }

    /// H4 acceptance (1)/(2): `SetDefinedName` (static, workbook scope)
    /// dispatches, and the resulting `DefinedNamesChanged` delta lists it
    /// with scope + rect.
    #[test]
    fn set_defined_name_dispatches_and_projects_defined_names_changed() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h4-set");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid,
            row: 2,
            col: 2,
            text: "5".to_string(),
        });

        let receipt = document.dispatch(WorkspaceIntent::SetDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Rate".to_string(),
            target: static_rect(2, 2, 2, 2),
        });

        assert!(receipt.accepted, "a fresh defined name is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::DefinedNamesChanged(catalog)] => {
                assert_eq!(catalog.entries.len(), 1);
                let entry = &catalog.entries[0];
                assert_eq!(entry.name, "Rate");
                assert_eq!(entry.scope, DefinedNameScopeProjection::Workbook);
                assert_eq!(
                    entry.target,
                    DefinedNameTargetProjection::Static(GridRectProjection {
                        top_row: 2,
                        left_col: 2,
                        bottom_row: 2,
                        right_col: 2,
                    })
                );
            }
            other => panic!("expected exactly one DefinedNamesChanged change, got {other:?}"),
        }
    }

    /// H4 acceptance (2), rename half: rename dispatches -> the catalog shows
    /// the old name gone and the new name present.
    #[test]
    fn rename_defined_name_dispatches_and_projects_old_gone_new_present() {
        let (mut document, _grid) = workbook_with_one_sheet("workbook:h4-rename-dispatch");
        let _ = document.dispatch(WorkspaceIntent::SetDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Rate".to_string(),
            target: static_rect(1, 1, 1, 1),
        });

        let receipt = document.dispatch(WorkspaceIntent::RenameDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            old_name: "Rate".to_string(),
            new_name: "TaxRate".to_string(),
        });

        assert!(receipt.accepted, "renaming an existing name is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::DefinedNamesChanged(catalog)] => {
                assert_eq!(catalog.entries.len(), 1);
                assert_eq!(catalog.entries[0].name, "TaxRate");
                assert!(!catalog.entries.iter().any(|entry| entry.name == "Rate"));
            }
            other => panic!("expected exactly one DefinedNamesChanged change, got {other:?}"),
        }
    }

    /// H4 acceptance (2), delete + recreate half: delete dispatches -> the
    /// catalog no longer lists the name and the dependent shows a non-numeric
    /// (#NAME?-shaped) value; recreating it self-heals the dependent.
    #[test]
    fn delete_then_recreate_defined_name_dispatches_and_self_heals_dependent() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h4-delete-dispatch");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "7".to_string(),
        });
        let _ = document.dispatch(WorkspaceIntent::SetDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Total".to_string(),
            target: static_rect(1, 1, 1, 1),
        });
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 2,
            col: 1,
            text: "=Total".to_string(),
        });

        let receipt = document.dispatch(WorkspaceIntent::DeleteDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Total".to_string(),
        });
        assert!(receipt.accepted, "deleting an existing name is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::DefinedNamesChanged(catalog)] => {
                assert!(catalog.entries.is_empty(), "Total is gone from the catalog");
            }
            other => panic!("expected exactly one DefinedNamesChanged change, got {other:?}"),
        }

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid).unwrap();
        let after_delete = session.grid_cell_value(sheet, 2, 1).unwrap().unwrap();
        assert!(
            after_delete.as_number().is_none(),
            "A2 no longer resolves to a plain number once Total is deleted, got {after_delete:?}"
        );

        // Recreate: self-heals.
        let _ = document.dispatch(WorkspaceIntent::SetDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Total".to_string(),
            target: static_rect(1, 1, 1, 1),
        });
        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        assert_eq!(
            session
                .grid_cell_value(sheet, 2, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(7.0),
            "recreating Total heals A2 back to 7"
        );
    }

    /// H4 acceptance (3): a duplicate name (workbook-scope name colliding
    /// with a root tree node's symbol) is a typed rejection receipt, and the
    /// projection's defined-name catalog is unchanged.
    #[test]
    fn set_defined_name_collision_is_typed_rejection_and_projection_unchanged() {
        let (mut document, _grid) = workbook_with_one_sheet("workbook:h4-collision-dispatch");
        let DocumentSession::Workbook(session) = &mut document else {
            unreachable!("workbook session")
        };
        session.add_root_calc_node_for_test("Rate", "5");

        let receipt = document.dispatch(WorkspaceIntent::SetDefinedName {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Rate".to_string(),
            target: static_rect(3, 3, 3, 3),
        });

        assert!(!receipt.accepted, "a tree-node-colliding name is rejected");
        match receipt.error {
            Some(IntentError::DefinedNameCollision { name }) => {
                assert_eq!(name, "Rate");
            }
            other => panic!("expected DefinedNameCollision, got {other:?}"),
        }

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        assert!(
            session.defined_names().unwrap().entries.is_empty(),
            "the rejected write leaves the projection's catalog unchanged"
        );
    }

    /// N3: `CreateNamedValue` dispatches atomically — the receipt's
    /// `DefinedNamesChanged` delta lists the new workbook-scoped name, and a
    /// formula on the user's own sheet referencing it resolves. Host-core owns
    /// the `_names` backing-cell allocation, so the skin dispatches a single
    /// intent with no backing-cell guess (the fix for the `?wb=1` `+ name` bug).
    #[test]
    fn create_named_value_dispatches_and_defines_resolvable_workbook_name() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:n3-create-named-dispatch");

        let receipt = document.dispatch(WorkspaceIntent::CreateNamedValue {
            name: "rate".to_string(),
            value_text: "0.065".to_string(),
        });
        assert!(receipt.accepted, "creating a named value is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::DefinedNamesChanged(catalog)] => {
                let rate = catalog
                    .entries
                    .iter()
                    .find(|entry| entry.name == "rate")
                    .expect("rate is listed in the catalog");
                assert_eq!(rate.scope, DefinedNameScopeProjection::Workbook);
            }
            other => panic!("expected exactly one DefinedNamesChanged change, got {other:?}"),
        }

        // A formula on the user's own sheet resolves the workbook-wide name.
        let entry = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "=rate*2".to_string(),
        });
        assert!(
            entry.accepted,
            "a formula referencing the new name is accepted"
        );

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid).unwrap();
        let got = session
            .grid_cell_value(sheet, 1, 1)
            .unwrap()
            .and_then(|v| v.as_number())
            .expect("=rate*2 resolves to a number");
        assert!(
            (got - 0.13).abs() < 1e-9,
            "=rate*2 resolves to 0.065*2 = 0.13, got {got}"
        );
    }

    // ------------------------------------------------------------------
    // H5 acceptance: calc mode + provenance + recalc intents end-to-end.
    // ------------------------------------------------------------------

    use dnacalc_skin_ir::{CalcModeProjection, ValueProvenanceProjection};

    /// H5 acceptance (2): Manual mode -> edit -> cell provenance
    /// `Stale{since}` and the value unchanged -> `Recalculate` -> the
    /// `CalcStateChanged` receipt, and a re-read shows `Calculated{tick}` and
    /// the updated value. Driven entirely through `WorkspaceIntent` dispatch
    /// (`SetCalcMode`/`EnterGridCell`/`Recalculate`), not the session's own
    /// narrower methods (those are `calc.rs`'s own unit tests).
    #[test]
    fn manual_mode_edit_stales_then_recalculate_intent_refreshes() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h5-manual-dispatch");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "7".to_string(),
        });
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 2,
            text: "=A1*3".to_string(),
        });

        let mode_receipt = document.dispatch(WorkspaceIntent::SetCalcMode {
            mode: CalcModeProjection::Manual,
        });
        assert!(mode_receipt.accepted, "SetCalcMode(Manual) is accepted");
        match mode_receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::CalcStateChanged(projection)] => {
                assert_eq!(projection.mode, CalcModeProjection::Manual);
            }
            other => panic!("expected exactly one CalcStateChanged change, got {other:?}"),
        }

        // Manual-mode edit: A1 = 10. Nothing recalculates.
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid).unwrap();
        assert_eq!(
            session.grid_cell_value(sheet, 1, 1).unwrap(),
            Some(CalcValue::number(7.0)),
            "A1's published value stays the pre-edit 7 under Manual"
        );
        assert!(
            matches!(
                session.grid_cell_provenance(sheet, 1, 1).unwrap(),
                Some(ValueProvenanceProjection::Stale { .. })
            ),
            "A1's published value is tagged Stale, not silently fresh"
        );

        // Recalculate (F9) via the intent.
        let recalc_receipt = document.dispatch(WorkspaceIntent::Recalculate);
        assert!(recalc_receipt.accepted, "Recalculate is accepted");
        let tick = match recalc_receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::CalcStateChanged(projection)] => projection
                .last_recalc_tick
                .expect("a genuine drain mints a tick"),
            other => panic!("expected exactly one CalcStateChanged change, got {other:?}"),
        };

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(30.0)),
            "after Recalculate, B1 = A1*3 = 30 (fresh value)"
        );
        assert_eq!(
            session.grid_cell_provenance(sheet, 1, 2).unwrap(),
            Some(ValueProvenanceProjection::Calculated { tick_id: tick }),
            "after Recalculate, B1's provenance is Calculated with the drain's tick"
        );
    }

    /// H5 acceptance (3): `Recalculate` with nothing dirty carries
    /// `drained_any == false` on the underlying outcome, surfaced as
    /// `last_recalc_tick == None` on the `CalcStateChanged` delta — the
    /// receipt carries no `GridChanged` change (H5 emits no per-sheet
    /// fan-out; that is H7's scope, and a no-op recalc has nothing to fan out
    /// regardless).
    #[test]
    fn recalculate_intent_with_nothing_dirty_is_a_noop_receipt() {
        let (mut document, grid) = workbook_with_one_sheet("workbook:h5-noop-dispatch");
        let _ = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid,
            row: 1,
            col: 1,
            text: "7".to_string(),
        });

        // Automatic mode already recalculated on write, so nothing is dirty.
        let receipt = document.dispatch(WorkspaceIntent::Recalculate);
        assert!(receipt.accepted, "Recalculate is accepted even as a no-op");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::CalcStateChanged(projection)] => {
                assert_eq!(
                    projection.last_recalc_tick, None,
                    "a no-op recalc mints no tick"
                );
            }
            other => panic!("expected exactly one CalcStateChanged change, got {other:?}"),
        }
        assert!(
            !receipt
                .delta
                .changes
                .iter()
                .any(|change| matches!(change, WorkspaceDeltaChange::GridChanged(_))),
            "a no-op Recalculate emits no GridChanged"
        );
    }

    // ------------------------------------------------------------------
    // Phase 1 Part A acceptance: sheet-lifecycle intents end-to-end.
    // ------------------------------------------------------------------

    use dnacalc_skin_ir::SheetProjection;

    /// The projected sheet list, read through the full snapshot (the same
    /// surface a future tab strip mounts from).
    fn projected_sheets(document: &DocumentSession) -> Vec<SheetProjection> {
        document.snapshot().sheets
    }

    /// Phase 1 Part A: `AddSheet` lists the new sheet in the projection, in
    /// order; a second `AddSheet` with no name defaults to a fresh unique name
    /// computed from the current sheet count.
    #[test]
    fn add_sheet_intent_lists_sheets_in_order_and_defaults_unique_name() {
        let mut document =
            DocumentSession::Workbook(WorkbookSession::create("workbook:p1a-add").unwrap());

        let receipt = document.dispatch(WorkspaceIntent::AddSheet {
            name: Some("Budget".to_string()),
        });
        assert!(receipt.accepted, "adding a named sheet is accepted");
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::SheetsChanged(sheets)] => {
                assert_eq!(sheets.len(), 1);
                assert_eq!(sheets[0].display_name, "Budget");
                assert_eq!(sheets[0].position, 0);
            }
            other => panic!("expected exactly one SheetsChanged change, got {other:?}"),
        }

        // A second AddSheet with no name defaults to a fresh unique name.
        let receipt = document.dispatch(WorkspaceIntent::AddSheet { name: None });
        assert!(receipt.accepted, "a defaulted AddSheet is accepted");
        let sheets = projected_sheets(&document);
        assert_eq!(sheets.len(), 2, "the snapshot lists both sheets in order");
        assert_eq!(sheets[0].display_name, "Budget");
        assert_eq!(sheets[0].position, 0);
        assert_eq!(
            sheets[1].display_name, "Sheet2",
            "the defaulted name is fresh and unique (Sheet{{count+1}})"
        );
        assert_eq!(sheets[1].position, 1);
    }

    /// Phase 1 Part A: `RenameSheet` shows the new display name in the
    /// projection while preserving the sheet's stable grid identity.
    #[test]
    fn rename_sheet_intent_updates_projection_display_name() {
        let mut document =
            DocumentSession::Workbook(WorkbookSession::create("workbook:p1a-rename").unwrap());
        let _ = document.dispatch(WorkspaceIntent::AddSheet {
            name: Some("Sheet1".to_string()),
        });
        let grid = projected_sheets(&document)[0].grid_node_id.clone();

        let receipt = document.dispatch(WorkspaceIntent::RenameSheet {
            grid: grid.clone(),
            new_name: "Revenue".to_string(),
        });
        assert!(receipt.accepted, "renaming a sheet is accepted");

        let sheets = projected_sheets(&document);
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].display_name, "Revenue");
        assert_eq!(
            sheets[0].grid_node_id, grid,
            "the stable grid identity is preserved across the rename"
        );
    }

    /// Phase 1 Part A: `MoveSheet` reorders the projection.
    #[test]
    fn move_sheet_intent_changes_order() {
        let mut document =
            DocumentSession::Workbook(WorkbookSession::create("workbook:p1a-move").unwrap());
        let _ = document.dispatch(WorkspaceIntent::AddSheet {
            name: Some("First".to_string()),
        });
        let _ = document.dispatch(WorkspaceIntent::AddSheet {
            name: Some("Second".to_string()),
        });
        let names_before: Vec<_> = projected_sheets(&document)
            .iter()
            .map(|s| s.display_name.clone())
            .collect();
        assert_eq!(names_before, vec!["First", "Second"]);
        let second_grid = projected_sheets(&document)[1].grid_node_id.clone();

        // Move "Second" to position 0.
        let receipt = document.dispatch(WorkspaceIntent::MoveSheet {
            grid: second_grid,
            new_position: 0,
        });
        assert!(receipt.accepted, "moving a sheet is accepted");

        let names_after: Vec<_> = projected_sheets(&document)
            .iter()
            .map(|s| s.display_name.clone())
            .collect();
        assert_eq!(
            names_after,
            vec!["Second", "First"],
            "the projection order reflects the move"
        );
    }

    /// Phase 1 Part A: `DeleteSheet` removes the sheet from the projection, and
    /// a cross-sheet formula referencing the deleted sheet now yields a
    /// non-numeric (`#REF!`-shaped) value.
    #[test]
    fn delete_sheet_intent_removes_from_projection_and_breaks_cross_sheet_reference() {
        // Sheet1 (A1=1, A5=5); Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 6.
        let mut session = WorkbookSession::create("workbook:p1a-delete").unwrap();
        let sheet1 = session.add_sheet("Sheet1").unwrap();
        let sheet2 = session.add_sheet("Sheet2").unwrap();
        session.enter_grid_cell(sheet1, 1, 1, "1").unwrap();
        session.enter_grid_cell(sheet1, 5, 1, "5").unwrap();
        session
            .enter_grid_cell(sheet2, 1, 1, "=Sheet1!A1+Sheet1!A5")
            .unwrap();
        assert_eq!(
            session
                .grid_cell_value(sheet2, 1, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(6.0),
            "Sheet2!A1 = 6 before the delete"
        );
        let sheet1_grid = sheet_grid_node_id(sheet1);
        let mut document = DocumentSession::Workbook(session);

        let receipt = document.dispatch(WorkspaceIntent::DeleteSheet {
            grid: sheet1_grid.clone(),
        });
        assert!(receipt.accepted, "deleting a sheet is accepted");
        let sheets = projected_sheets(&document);
        assert!(
            !sheets.iter().any(|s| s.grid_node_id == sheet1_grid),
            "Sheet1 is gone from the projection"
        );
        assert!(
            sheets.iter().any(|s| s.display_name == "Sheet2"),
            "Sheet2 remains in the projection"
        );

        // The cross-sheet reference to the deleted sheet no longer resolves to
        // a plain number (a #REF!-shaped error); don't over-assert the exact
        // error shape.
        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let after = session.grid_cell_value(sheet2, 1, 1).unwrap().unwrap();
        assert!(
            after.as_number().is_none(),
            "Sheet2!A1 no longer resolves to a plain number once Sheet1 is deleted, got {after:?}"
        );
    }

    /// Phase 1 Part A cross-sheet integrity: adding a sheet leaves an existing
    /// cross-sheet formula (`=Sheet1!A1+Sheet1!A5`) evaluating.
    #[test]
    fn add_sheet_preserves_existing_cross_sheet_formula() {
        let mut session = WorkbookSession::create("workbook:p1a-add-preserves").unwrap();
        let sheet1 = session.add_sheet("Sheet1").unwrap();
        let sheet2 = session.add_sheet("Sheet2").unwrap();
        session.enter_grid_cell(sheet1, 1, 1, "1").unwrap();
        session.enter_grid_cell(sheet1, 5, 1, "5").unwrap();
        session
            .enter_grid_cell(sheet2, 1, 1, "=Sheet1!A1+Sheet1!A5")
            .unwrap();
        let mut document = DocumentSession::Workbook(session);

        let receipt = document.dispatch(WorkspaceIntent::AddSheet {
            name: Some("Sheet3".to_string()),
        });
        assert!(receipt.accepted, "adding a third sheet is accepted");

        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        assert_eq!(
            session
                .grid_cell_value(sheet2, 1, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(6.0),
            "the existing cross-sheet formula still evaluates to 6 after adding a sheet"
        );
    }
}

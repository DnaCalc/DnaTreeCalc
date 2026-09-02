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
//! H2 stood up the crate seam: the [`DocumentSession`] enum, [`WorkbookSession`]
//! over one [`OxCalcDocumentContext`] (create workspace + add sheets + the
//! `set_grid_cell_value` write path), the [`HostCommand`] skeleton, the
//! [`ProjectionPublisher`] publication seam, and the Send/Sync audit below. The
//! universal `EnterGridCell` authored-entry verb landed in H6 (below); the
//! RichTree session migration into host-core (S4.P3 — the tree model layer
//! already moved here in S4.P2, see [`tree`]) and the worker are still
//! pending. xlsx is no longer excluded: the file-backed workbook lifecycle
//! lands with the W011 successor slice (epic `dtc-j7n8`) — host-core takes
//! `oxdoc_model`/`oxdoc_xlsx` directly (dtc-j7n8.1) and OxCalc's R6
//! `oxdoc-model` ingest has landed upstream, so `HostCommand::OpenXlsxBytes`/
//! `SaveActiveXlsx` are host wiring of real `.xlsx` bytes through OxDoc, not a
//! new engine.
//!
//! ## H6 scope
//!
//! H6 wires the cell-entry family end to end: `WorkspaceIntent::EnterGridCell`/
//! `ClearGridCell` dispatch (this module), the three-way
//! `GridCellEntered { outcome }` receipt payload, and the `present.rs` A.4
//! error-presentation map from a rejected write to a typed [`IntentError`].
//! Since W011 (dtc-j7n8.18) an accepted entry receipt also carries, beside
//! the `GridCellEntered` hint, the edited sheet's complete `GridChanged`
//! projection AND one `GridChanged` per other sheet whose projection the
//! edit's cross-sheet recalc moved (found by a host-side pre/post peer diff,
//! [`WorkbookSession::peer_grid_projections`]), so a retained mirror
//! (`session_channel::apply_delta`) patches every changed sheet in place
//! without a full snapshot. `Recalculate` fan-out (a genuine drain's
//! `GridChanged`s) and skins are out of H6 scope (H7 and later).

pub mod calc;
pub mod command;
pub mod defined_names;
pub mod demo;
pub mod grid_publication;
pub mod persistence;
pub mod present;
pub mod skin_protocol;
pub mod tree;
pub mod workbook;
// W011 (dtc-j7n8.2): test-only access to the committed `a1_times_three` xlsx
// fixture (parts zipped in memory through the dev-only `oxdoc_conformance`
// crate, hence `cfg(test)`). Later W011 beads open the same bytes.
#[cfg(test)]
pub(crate) mod xlsx_fixture;

pub use calc::{
    calc_mode_from_projection, calc_mode_projection, present_calc_rejection,
    value_provenance_projection,
};
pub use command::{
    HostCommand, HostCommandError, HostCommandOutcome, ProjectionPublisher, RecordingPublisher,
};
pub use defined_names::{
    DefinedNameTargetIntentInput, NAMES_BACKING_SHEET, present_defined_name_rejection,
};
pub use demo::build_demo_workbook;
pub use grid_publication::{
    grid_authored_cell_projection, grid_overlay_bundle_for, grid_projection_for,
    grid_value_projection,
};
pub use persistence::LocalFileSkinStatePersistenceStore;
pub use present::present_grid_entry_rejection;
pub use skin_protocol::SkinProtocolSession;
pub use workbook::{
    WorkbookSession, WorkbookSessionError, XLSX_WORKSPACE_ID, parse_sheet_grid_node_id,
    sheet_grid_node_id,
};

use std::collections::BTreeMap;

use dnacalc_skin_ir::{
    DefinedNameTargetIntent, GridEntryOutcomeProjection, GridProjection, IntentError,
    IntentReceipt, NodeValueProjection, WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent,
    WorkspaceState,
};
use oxcalc_core::consumer::GridCellEntryOutcome;
use oxcalc_core::structural::TreeNodeId;
use oxfunc_core::value::CalcValue;

// Re-export the engine document surface name the enum is built over, so callers
// name it through host-core rather than reaching into `oxcalc_core` directly.
pub use oxcalc_core::consumer::OxCalcDocumentContext;
// Likewise the engine's load-report types an `Opened` outcome / a
// `WorkbookSession::load_report` reader needs to name (dtc-j7n8.4).
pub use oxcalc_core::oxdoc_ingest::{LoadRecalcPath, WorkbookLoadReport};

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
    // Read BEFORE the engine mutates (see `grid_entry_receipt`): the only
    // way to tell a peer sheet the edit's cross-sheet recalc moved from one
    // it left alone. A rejected entry leaves the engine untouched, so the
    // baseline is simply dropped on that path.
    let peers_before = session.peer_grid_projections(sheet);
    match session.enter_grid_cell(sheet, row, col, text) {
        Ok(outcome) => {
            grid_cell_entered_receipt(session, sheet, grid, row, col, &outcome, peers_before)
        }
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
    // Pre-edit peer baseline, as in `dispatch_enter_grid_cell`: a clear
    // dirties cross-sheet readers of the cleared cell exactly like an entry.
    let peers_before = session.peer_grid_projections(sheet);
    match session.clear_grid_cell(sheet, row, col) {
        Ok(view) => grid_entry_receipt(
            session,
            sheet,
            grid,
            row,
            col,
            GridEntryOutcomeProjection::Cleared,
            &view,
            peers_before,
        ),
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

/// Every OTHER grid-backed sheet's projection in sheet order, as
/// [`WorkbookSession::peer_grid_projections`] reads it — taken once before
/// and once after an entry so the receipt can carry a `GridChanged` for each
/// cross-sheet dependent the edit recalculated (see [`grid_entry_receipt`]).
type PeerGridProjections = Result<Vec<(TreeNodeId, GridProjection)>, WorkbookSessionError>;

/// Build the accepted entry receipt (§A.2's verb-façade row) from the engine's
/// three-way [`GridCellEntryOutcome`]: the outcome's literal/formula/cleared
/// value(s) mirrored into the `GridCellEntered` wire projection, plus the
/// edited sheet's `GridChanged` built from the post-edit view the outcome
/// already carries and a `GridChanged` per peer sheet that moved against
/// `peers_before` (see [`grid_entry_receipt`]).
fn grid_cell_entered_receipt(
    session: &WorkbookSession,
    sheet: TreeNodeId,
    grid: &dnacalc_skin_ir::NodeId,
    row: u32,
    col: u32,
    outcome: &GridCellEntryOutcome,
    peers_before: PeerGridProjections,
) -> IntentReceipt {
    let (projected, view) = match outcome {
        GridCellEntryOutcome::Literal { value, view } => (
            GridEntryOutcomeProjection::Literal {
                value: calc_value_projection(value),
            },
            view,
        ),
        GridCellEntryOutcome::Formula {
            unresolved_names,
            view,
            ..
        } => (
            GridEntryOutcomeProjection::Formula {
                unresolved_names: unresolved_names.clone(),
                value: view
                    .cells
                    .iter()
                    .find(|cell| cell.address.row == row && cell.address.col == col)
                    .map(|cell| calc_value_projection(&cell.value))
                    .unwrap_or(NodeValueProjection::Unevaluated),
            },
            view,
        ),
        GridCellEntryOutcome::Cleared { view } => (GridEntryOutcomeProjection::Cleared, view),
    };
    grid_entry_receipt(
        session,
        sheet,
        grid,
        row,
        col,
        projected,
        view,
        peers_before,
    )
}

/// The accepted receipt every grid entry verb (`EnterGridCell` in all three
/// outcomes, `ClearGridCell`) answers with — the UI hint plus one
/// `GridChanged` per sheet the edit moved, in ONE delta:
///
/// 1. `GridCellEntered { outcome }` — the UI hint (what the entered text
///    resolved to); no projection-state effect of its own.
/// 2. `GridChanged(projection)` for the EDITED sheet — its complete windowed
///    projection (values, epochs, provenance AND the authored layer), built
///    by [`WorkbookSession::grid_projection_from_view`] from the post-edit
///    `view` the engine handed back with the outcome — the very readout a
///    fresh `snapshot()` projects for that sheet, through the same function.
/// 3. `GridChanged(projection)` for every OTHER sheet whose projection
///    moved: under Automatic calc mode the engine recalculates cross-sheet
///    dependents in the same transaction (OxCalc `propagate_cross_sheet_edit`
///    — `Sheet2!A1 = =Sheet1!A1+Sheet1!A5` in the demo workbook) but hands
///    back only the edited sheet's view and no "sheets I recalculated" fact,
///    so the dispatch reads every peer sheet's projection BEFORE the edit
///    (`peers_before`, [`WorkbookSession::peer_grid_projections`]) and again
///    after, and emits a patch for each peer whose `GridProjection` is no
///    longer equal — in sheet order, after the edited sheet's. A peer the
///    edit did not touch (no cross-sheet reader of the edited cell; or any
///    edit under Manual mode, where propagation is suppressed) stays out of
///    the receipt, so an unrelated sheet never re-renders.
///
/// Every patch is the projection a fresh `snapshot()` would carry for that
/// sheet, and a sheet without a patch has a projection equal to its pre-edit
/// one, so a mirror that applies this delta (`session_channel::apply_delta`)
/// ends up grid-for-grid equal to a full snapshot without shipping one
/// (dtc-j7n8.18; W011's "mirrors can patch" contract) — proven on the
/// single-sheet fixture and on the two-sheet demo workbook, under Automatic.
/// What the delta does NOT cover: `workbook_calc` — under Manual mode an
/// entry dirties the sheet without a recalc and the receipt carries no
/// `CalcStateChanged`, so a mirror's dirty flag lags (dtc-j7n8.20).
///
/// `from_seq`/`to_seq` stay `0`: host-core owns no projection-sequence
/// authority yet — the executor that stamps sequences (the app dispatcher's
/// snapshot republish, or a worker executor's `SessionResponse::for_receipt`)
/// decides what a mirror sees. Emitting the patches here is what makes the
/// delta-only path possible; choosing it is the executor's call.
///
/// The edit has ALREADY mutated the engine by the time this runs, so the
/// receipt must stay `accepted` whatever happens next: if any patch cannot
/// be vouched for — the edited sheet's projection cannot be built, either
/// peer readout failed, or the sheet list somehow moved under the edit (all
/// unreachable by construction, but not a reason to `unwrap`) — the receipt
/// carries `FullReset` in place of the patches. `FullReset` is deliberately
/// NOT mirror-applicable, so `apply_delta` reports a resync and a snapshot is
/// shipped instead — the honest degrade: the edit landed, the mirror must
/// resync — never a silent "nothing changed" delta over a changed grid.
// `too_many_arguments`: the eighth argument is the pre-edit peer baseline the
// dispatch arms must capture before the engine call (dtc-j7n8.18, round 2);
// bundling the address triple or the outcome into a struct for one private
// call site would add a type without removing a fact.
#[allow(clippy::too_many_arguments)]
fn grid_entry_receipt(
    session: &WorkbookSession,
    sheet: TreeNodeId,
    grid: &dnacalc_skin_ir::NodeId,
    row: u32,
    col: u32,
    outcome: GridEntryOutcomeProjection,
    view: &oxcalc_core::consumer::OxCalcTreeGridView,
    peers_before: PeerGridProjections,
) -> IntentReceipt {
    let mut changes = vec![WorkspaceDeltaChange::GridCellEntered {
        grid_node_id: grid.clone(),
        row,
        col,
        outcome,
    }];
    match grid_entry_patches(session, sheet, view, peers_before) {
        Some(patches) => changes.extend(patches.into_iter().map(WorkspaceDeltaChange::GridChanged)),
        None => changes.push(WorkspaceDeltaChange::FullReset),
    }
    IntentReceipt::accepted().with_delta(WorkspaceDelta {
        from_seq: 0,
        to_seq: 0,
        changes,
    })
}

/// The `GridChanged` payloads an accepted entry receipt carries, in order:
/// the edited sheet's post-edit projection, then every peer sheet whose
/// projection differs from its `peers_before` reading (see
/// [`grid_entry_receipt`], point 3). `None` when a patch cannot be vouched
/// for — a failed projection build, a failed peer readout, or a peer sheet
/// list that no longer matches the pre-edit one — which the receipt turns
/// into `FullReset`.
fn grid_entry_patches(
    session: &WorkbookSession,
    sheet: TreeNodeId,
    view: &oxcalc_core::consumer::OxCalcTreeGridView,
    peers_before: PeerGridProjections,
) -> Option<Vec<GridProjection>> {
    let edited = session.grid_projection_from_view(sheet, view).ok()?;
    let before = peers_before.ok()?;
    let after = session.peer_grid_projections(sheet).ok()?;
    // An entry never adds, removes or reorders sheets; if the peer list
    // moved anyway, nothing below can be vouched for.
    if !before
        .iter()
        .map(|(peer, _)| *peer)
        .eq(after.iter().map(|(peer, _)| *peer))
    {
        return None;
    }
    let before: BTreeMap<TreeNodeId, GridProjection> = before.into_iter().collect();
    let mut patches = vec![edited];
    patches.extend(
        after
            .into_iter()
            .filter(|(peer, projection)| before.get(peer) != Some(projection))
            .map(|(_, projection)| projection),
    );
    Some(patches)
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

/// A [`HostCommand`] executor over a document session: the H2 dispatch arm
/// (routing through [`DocumentSession::dispatch`]) and the W011 document-open
/// (dtc-j7n8.3) and document-save (dtc-j7n8.7) arms.
// `result_large_err`: `HostCommandError` wraps `WorkbookSessionError` by
// value, which in turn wraps the engine's `OxCalcDocumentError` by value — the
// same by-value convention `workbook.rs` documents; a command execution is a
// single host call, not a hot inner loop, so boxing buys nothing here.
#[allow(clippy::result_large_err)]
impl DocumentSession {
    /// Execute one host command against this session.
    ///
    /// - `DispatchWorkspaceIntent` is infallible at this level: it always
    ///   returns `Ok(Dispatched(receipt))`, and any *rejection* of the intent
    ///   travels **inside** the [`IntentReceipt`] (the H2 contract, unchanged
    ///   by W011).
    /// - `OpenXlsxBytes` opens the bytes through OxDoc under
    ///   `LoadProfile::full()` and, on success, **replaces** `self` with the
    ///   opened [`WorkbookSession`] (the previous session drops — opening a
    ///   document is what a future `SessionEngine::init` does). On failure
    ///   `self` is left untouched and OxDoc's typed `XlsxError` comes back as
    ///   [`HostCommandError::Workbook`]`(`[`WorkbookSessionError::Xlsx`]`)`.
    /// - `SaveActiveXlsx` (dtc-j7n8.7) saves a `Workbook` session back to
    ///   `.xlsx` bytes through [`WorkbookSession::save_xlsx_bytes`] — the
    ///   engine's whole-model projection (fresh formula caches) round-tripped
    ///   by OxDoc against the opened package — and returns
    ///   [`HostCommandOutcome::Saved`]`{ bytes, save_ledger }`. A save never
    ///   replaces or mutates the session. Typed refusals, never panics: a
    ///   `RichTree` session is [`HostCommandError::UnsupportedByModel`]; a
    ///   workbook not opened from bytes is
    ///   [`WorkbookSessionError::NoBackingSource`]; an edit outside OxDoc's
    ///   round-trip policy (a cell add, a formula-text change) is OxDoc's
    ///   `XlsxError::UnsupportedRoundTripFeature` inside
    ///   [`WorkbookSessionError::Xlsx`], with the live model left intact.
    pub fn execute(
        &mut self,
        command: HostCommand,
    ) -> Result<HostCommandOutcome, HostCommandError> {
        match command {
            HostCommand::DispatchWorkspaceIntent(intent) => {
                Ok(HostCommandOutcome::Dispatched(self.dispatch(intent)))
            }
            HostCommand::SaveActiveXlsx => {
                let model = self.model_name();
                let DocumentSession::Workbook(session) = self else {
                    return Err(HostCommandError::UnsupportedByModel {
                        model,
                        command: "SaveActiveXlsx",
                    });
                };
                // Read-only on the session: the engine projects LIVE truth
                // (fresh caches), OxDoc merges it onto the opened package,
                // and the bytes go back to the caller — nothing here is
                // replaced or rebased (dirty tracking and rebasing the source
                // on the saved bytes are later beads).
                let (bytes, save_ledger) = session.save_xlsx_bytes()?;
                Ok(HostCommandOutcome::Saved { bytes, save_ledger })
            }
            HostCommand::OpenXlsxBytes { bytes, name } => {
                let session =
                    WorkbookSession::open_xlsx_bytes(XLSX_WORKSPACE_ID, &bytes, name.clone())?;
                // The sheet count is the engine's own enumeration after
                // ingest (dtc-j7n8.4); the load facts come from the engine's
                // report and the ledger from the host-owned OxDoc bundle —
                // `open_xlsx_bytes` always stores both, by construction.
                let sheet_count = session.sheets()?.len();
                let report = session
                    .load_report()
                    .expect("a workbook opened from xlsx bytes carries its engine load report");
                let (cells, formulas_bound, recalc_path) =
                    (report.cells, report.formulas_bound, report.recalc_path);
                let load_ledger = session
                    .xlsx_source()
                    .expect("a workbook opened from xlsx bytes owns its OxDoc source")
                    .load_ledger
                    .clone();
                *self = DocumentSession::Workbook(session);
                Ok(HostCommandOutcome::Opened {
                    name,
                    sheet_count,
                    cells,
                    formulas_bound,
                    recalc_path,
                    load_ledger,
                })
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

        let outcome = document
            .execute(HostCommand::DispatchWorkspaceIntent(
                WorkspaceIntent::CreateScenario {
                    scenario_id: "s1".to_string(),
                    name: "Downside".to_string(),
                    base_scenario_id: None,
                },
            ))
            .expect("dispatch never fails at the command level; rejections ride the receipt");
        let HostCommandOutcome::Dispatched(receipt) = outcome else {
            panic!("DispatchWorkspaceIntent yields Dispatched, got {outcome:?}");
        };
        publisher.publish(&receipt);

        let published = publisher.published();
        assert_eq!(published.len(), 1);
        assert!(!published[0].accepted);
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.3): `HostCommand::OpenXlsxBytes` end to end.
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::w011_fixture_bytes;
    use oxdoc_model::FidelityDisposition;

    /// Executing `OpenXlsxBytes` with the real W011 fixture bytes over the
    /// demo workbook replaces the active session with the opened, ingested
    /// workbook (the demo's `Sheet2` and `workbook:demo` identity are gone;
    /// the OxDoc source is owned; the engine holds `Sheet1` with `B1 = 21`)
    /// and returns the typed `Opened` outcome carrying the engine's load
    /// facts (`sheet_count`, `cells`, `formulas_bound`, `recalc_path`) and
    /// OxDoc's load ledger.
    #[test]
    fn execute_open_xlsx_bytes_replaces_session_and_returns_opened() {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        assert_eq!(document.model_name(), "Workbook");

        let outcome = document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: w011_fixture_bytes(),
                name: Some("a1_times_three.xlsx".to_string()),
            })
            .expect("OxDoc opens the committed W011 fixture");
        println!("OpenXlsxBytes outcome: {outcome:?}");

        let HostCommandOutcome::Opened {
            name,
            sheet_count,
            cells,
            formulas_bound,
            recalc_path,
            load_ledger,
        } = &outcome
        else {
            panic!("expected Opened, got {outcome:?}");
        };
        assert_eq!(name.as_deref(), Some("a1_times_three.xlsx"));
        assert_eq!(
            *sheet_count, 1,
            "the engine enumerates one sheet after ingest"
        );
        assert_eq!(*cells, 1, "A1 is the one literal");
        assert_eq!(*formulas_bound, 1, "B1 bound");
        assert_eq!(
            *recalc_path,
            LoadRecalcPath::Automatic,
            "the fixture's calcMode=auto took the open-recalc path"
        );
        assert!(
            !load_ledger
                .entries
                .iter()
                .any(|entry| matches!(entry.disposition, FidelityDisposition::Dropped { .. })),
            "the fixture loads without dropped parts: {load_ledger:?}"
        );

        // The active session is now the opened workbook, still of the
        // Workbook model family.
        assert_eq!(document.model_name(), "Workbook");
        let DocumentSession::Workbook(session) = &document else {
            panic!("the opened document is a Workbook session, got {document:?}");
        };
        assert_eq!(session.workspace_id().as_str(), XLSX_WORKSPACE_ID);
        assert_eq!(session.document_name(), Some("a1_times_three.xlsx"));
        let source = session
            .xlsx_source()
            .expect("the host owns the OxDoc source after open");
        assert_eq!(
            source.load_ledger, *load_ledger,
            "the outcome echoes the owned ledger"
        );

        // The demo session really was replaced, not merged: its second sheet
        // and its `workbook:demo` identity are gone, and the engine holds the
        // ingested fixture (`Sheet1`, `B1 = A1*3 = 21`) instead.
        assert_ne!(session.workspace_id().as_str(), "workbook:demo");
        let sheets = session.sheets().unwrap();
        assert!(
            !sheets.iter().any(|row| row.display_name == "Sheet2"),
            "the demo's Sheet2 must not survive the open"
        );
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].display_name, "Sheet1");
        assert_eq!(
            session.grid_cell_value(sheets[0].node_id, 1, 2).unwrap(),
            Some(CalcValue::number(21.0)),
            "the opened session's engine state is the ingested fixture"
        );
        assert_eq!(
            session.load_report().map(|report| report.sheets),
            Some(1),
            "the session keeps the engine's full load report"
        );
    }

    use dnacalc_skin_ir::GridAuthoredKindProjection;

    /// dtc-j7n8.5, the infallible mount surface: `DocumentSession::snapshot`
    /// degrades an internal-invariant error to `WorkspaceState::default()`,
    /// so a skin mounting an opened workbook could be handed an empty
    /// default that "renders" nothing and fails no caller — the silent-pass
    /// mode `workbook.rs`'s `snapshot_of_loaded_fixture_projects_authored_and_provenance`
    /// closes at the `Result` surface. This closes it at the surface a skin
    /// actually calls: after `OpenXlsxBytes`, the snapshot is the loaded
    /// fixture — its identity, `Sheet1`, and both cells with their authored
    /// kinds (asserted before the values, the token-mismatch blank order)
    /// and `Calculated` values.
    #[test]
    fn snapshot_of_loaded_fixture_through_document_session_is_not_defaulted() {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: w011_fixture_bytes(),
                name: Some("a1_times_three.xlsx".to_string()),
            })
            .expect("OxDoc opens and the engine ingests the committed W011 fixture");

        let state = document.snapshot();
        assert_ne!(
            state,
            WorkspaceState::default(),
            "the mount surface handed a skin the empty default instead of the opened workbook"
        );
        assert_eq!(state.workspace_id, XLSX_WORKSPACE_ID);
        assert_eq!(
            state.grids.len(),
            1,
            "one grid, Sheet1's: {:?}",
            state.grids.keys()
        );
        assert_eq!(state.sheets.len(), 1);
        assert_eq!(state.sheets[0].display_name, "Sheet1");
        let grid = &state.grids[&state.sheets[0].grid_node_id];
        assert!(!grid.cells.is_empty(), "Sheet1's projected cells are empty");
        assert_eq!(grid.cells.len(), 2, "A1 and B1: {:#?}", grid.cells);

        let cell = |row: u32, col: u32| {
            grid.cells
                .iter()
                .find(|cell| cell.row == row && cell.col == col)
                .unwrap_or_else(|| {
                    panic!("no projected cell at ({row}, {col}) in {:#?}", grid.cells)
                })
        };
        let a1 = cell(1, 1);
        let b1 = cell(1, 2);
        for cell in [a1, b1] {
            println!(
                "W011 mount snapshot: cell ({}, {}) kind={:?} source_text={:?} value={:?} provenance={:?}",
                cell.row,
                cell.col,
                cell.authored.as_ref().map(|authored| authored.kind),
                cell.authored
                    .as_ref()
                    .and_then(|authored| authored.source_text.as_deref()),
                cell.value,
                cell.provenance
            );
        }
        // Authored kinds first: a `None` is the GridRect token-mismatch blank.
        assert_eq!(
            a1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Literal),
            "A1 authored Literal"
        );
        assert_eq!(
            b1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Formula),
            "B1 authored Formula"
        );
        assert_eq!(
            b1.authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=A1*3")
        );
        assert_eq!(
            a1.value,
            NodeValueProjection::Number {
                raw: "7".to_string(),
                display: "7".to_string(),
            }
        );
        assert_eq!(
            b1.value,
            NodeValueProjection::Number {
                raw: "21".to_string(),
                display: "21".to_string(),
            }
        );
        assert!(
            matches!(
                b1.provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "B1 is Calculated by the open-recalc, not FileCached: {:?}",
            b1.provenance
        );
        assert!(grid.authored_epoch > 0, "authored_epoch > 0");
    }

    /// Executing `OpenXlsxBytes` with bytes that are not a zip is a typed
    /// error — OxDoc's `XlsxError` as data inside
    /// `HostCommandError::Workbook(WorkbookSessionError::Xlsx(_))` — never a
    /// string and never a panic; and the failed open leaves the active
    /// session exactly as it was.
    #[test]
    fn open_invalid_bytes_is_typed_error_not_panic() {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());

        let error = document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: b"not a zip".to_vec(),
                name: Some("garbage.xlsx".to_string()),
            })
            .expect_err("garbage bytes are rejected");
        match &error {
            HostCommandError::Workbook(WorkbookSessionError::Xlsx(xlsx)) => {
                println!("typed OxDoc rejection (Display): {xlsx}");
                println!("typed OxDoc rejection (Debug): {xlsx:?}");
            }
            other => panic!("expected HostCommandError::Workbook(Xlsx(_)), got {other:?}"),
        }
        println!("HostCommandError (Display): {error}");

        // The active session is untouched by the failed open: still the demo
        // workbook, with no OxDoc source and both of its sheets.
        let DocumentSession::Workbook(session) = &document else {
            panic!("the demo session survives a failed open, got {document:?}");
        };
        assert_eq!(session.workspace_id().as_str(), "workbook:demo");
        assert!(session.xlsx_source().is_none());
        assert_eq!(session.document_name(), None);
        assert_eq!(session.sheets().unwrap().len(), 2);
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.6): the campaign's EDIT proof — `EnterGridCell` on the
    // xlsx-LOADED session, through the very `dispatch` path the hand-built
    // H6 tests below prove. `A1` 7 -> 10 makes `B1` (`=A1*3`) publish 30;
    // the receipt shape is `GridCellEntered` (a `GridChanged` alongside it
    // is dtc-j7n8.18's scope, so these tests search the receipt for the
    // `GridCellEntered` change rather than pattern-matching exactly one).
    // ------------------------------------------------------------------

    use dnacalc_skin_ir::{GridCellProjection, GridProjection};

    /// Open the W011 fixture through the command surface a host actually
    /// calls (`HostCommand::OpenXlsxBytes` over a live session) — the shape
    /// every dtc-j7n8.6 test starts from.
    fn open_w011_fixture_document() -> DocumentSession {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: w011_fixture_bytes(),
                name: Some("a1_times_three.xlsx".to_string()),
            })
            .expect("OxDoc opens and the engine ingests the committed W011 fixture");
        document
    }

    /// The loaded fixture's single grid, addressed exactly the way a skin
    /// addresses it — `sheets[0].grid_node_id` off the snapshot, never a
    /// host-composed string — with dtc-j7n8.5's silent-pass closure kept: the
    /// grid exists and carries cells before anything else is read from it.
    fn loaded_sheet1_grid(state: &WorkspaceState) -> (dnacalc_skin_ir::NodeId, &GridProjection) {
        assert_eq!(
            state.sheets.len(),
            1,
            "one tab-strip row (a defaulted snapshot has none): {:?}",
            state.sheets
        );
        let grid_id = state.sheets[0].grid_node_id.clone();
        let grid = state.grids.get(&grid_id).unwrap_or_else(|| {
            panic!(
                "Sheet1's grid {grid_id:?} is projected; grids = {:?}",
                state.grids.keys().collect::<Vec<_>>()
            )
        });
        assert!(
            !grid.cells.is_empty(),
            "Sheet1's projected cell list is empty (the defaulted-snapshot silent pass)"
        );
        (grid_id, grid)
    }

    /// Locate one projected cell by 1-based `(row, col)`, failing with the
    /// whole projected cell list when it is missing.
    fn projected_cell(grid: &GridProjection, row: u32, col: u32) -> &GridCellProjection {
        grid.cells
            .iter()
            .find(|cell| cell.row == row && cell.col == col)
            .unwrap_or_else(|| panic!("no projected cell at ({row}, {col}) in {:#?}", grid.cells))
    }

    /// The `Calculated` tick a projected cell's provenance carries; any other
    /// provenance (`Stale`, `FileCached`, none) fails naming the cell.
    fn calculated_tick(label: &str, cell: &GridCellProjection) -> u64 {
        match cell.provenance {
            Some(ValueProvenanceProjection::Calculated { tick_id }) => tick_id,
            ref other => panic!("{label} is not engine-Calculated: {other:?}"),
        }
    }

    fn number(raw: &str) -> NodeValueProjection {
        NodeValueProjection::Number {
            raw: raw.to_string(),
            display: raw.to_string(),
        }
    }

    fn log_cell(stage: &str, label: &str, cell: &GridCellProjection) {
        println!(
            "W011 edit [{stage}] {label} ({}, {}) kind={:?} literal_text={:?} source_text={:?} \
             value={:?} value_epoch={} provenance={:?}",
            cell.row,
            cell.col,
            cell.authored.as_ref().map(|authored| authored.kind),
            cell.authored
                .as_ref()
                .and_then(|authored| authored.literal_text.as_deref()),
            cell.authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            cell.value,
            cell.value_epoch,
            cell.provenance
        );
    }

    /// The receipt's `GridChanged` change naming `grid` — that sheet's
    /// complete windowed projection a mirror patches in place (dtc-j7n8.18).
    /// Exactly one per sheet on an accepted entry receipt (the edited sheet
    /// always; a peer sheet only when the edit's cross-sheet recalc moved
    /// it), and never the `FullReset` degrade beside it: a missing, doubled,
    /// or misaddressed patch is a contract break, not noise.
    fn grid_changed_for<'a>(
        receipt: &'a IntentReceipt,
        grid: &dnacalc_skin_ir::NodeId,
    ) -> &'a GridProjection {
        let kinds: Vec<_> = receipt
            .delta
            .changes
            .iter()
            .map(dnacalc_skin_ir::session_channel::change_kind)
            .collect();
        let changed: Vec<&GridProjection> = receipt
            .delta
            .changes
            .iter()
            .filter_map(|change| match change {
                WorkspaceDeltaChange::GridChanged(projection)
                    if projection.grid_node_id == *grid =>
                {
                    Some(projection)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            changed.len(),
            1,
            "exactly one GridChanged naming {grid:?} on an accepted entry receipt: {kinds:?} \
             (patched grids: {:?})",
            grid_changed_targets(receipt)
        );
        assert!(
            !receipt
                .delta
                .changes
                .iter()
                .any(|change| matches!(change, WorkspaceDeltaChange::FullReset)),
            "no FullReset degrade rides beside a built GridChanged: {kinds:?}"
        );
        changed[0]
    }

    /// The grids the receipt's `GridChanged` changes name, in delta order —
    /// the edited sheet first, then any peer sheet the edit moved
    /// (dtc-j7n8.18, round 2).
    fn grid_changed_targets(receipt: &IntentReceipt) -> Vec<dnacalc_skin_ir::NodeId> {
        receipt
            .delta
            .changes
            .iter()
            .filter_map(|change| match change {
                WorkspaceDeltaChange::GridChanged(projection) => {
                    Some(projection.grid_node_id.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The receipt's single `GridCellEntered` change: `(grid, row, col,
    /// outcome)`. Searched, not slice-matched, so dtc-j7n8.18's `GridChanged`
    /// alongside it does not change this contract.
    fn grid_cell_entered_change(
        receipt: &IntentReceipt,
    ) -> (
        &dnacalc_skin_ir::NodeId,
        u32,
        u32,
        &GridEntryOutcomeProjection,
    ) {
        let entered: Vec<_> = receipt
            .delta
            .changes
            .iter()
            .filter_map(|change| match change {
                WorkspaceDeltaChange::GridCellEntered {
                    grid_node_id,
                    row,
                    col,
                    outcome,
                } => Some((grid_node_id, *row, *col, outcome)),
                _ => None,
            })
            .collect();
        assert_eq!(
            entered.len(),
            1,
            "exactly one GridCellEntered change on an accepted entry receipt: {:?}",
            receipt.delta.changes
        );
        entered[0]
    }

    /// dtc-j7n8.6 acceptance (1) — THE campaign edit proof, on real bytes.
    /// Open the fixture through `OpenXlsxBytes`, dispatch
    /// `WorkspaceIntent::EnterGridCell { A1, "10" }` addressed by the grid id
    /// the snapshot handed out, and prove from the receipt AND a fresh
    /// snapshot that the LOADED workbook really recalculated: the receipt is
    /// accepted with a `GridCellEntered { Literal 10 }` change; post-edit,
    /// `A1` = 10, `B1` = 30 with `Calculated` provenance on a tick NEWER than
    /// the open-recalc tick and an advanced `value_epoch`; and `B1`'s
    /// authored source text is STILL `=A1*3` — an edit of `A1` never
    /// rewrites `B1`'s authored truth. Authored kinds are asserted before
    /// values on both snapshots (the GridRect token-mismatch blank order,
    /// dtc-j7n8.5). Pre/post values, epochs and tick ids are logged.
    #[test]
    fn enter_grid_cell_on_loaded_fixture_recalculates_dependent() {
        let mut document = open_w011_fixture_document();

        // PRE: the loaded truth, read through the skin's own mount surface.
        let before = document.snapshot();
        let (grid_id, grid_before) = loaded_sheet1_grid(&before);
        let a1_before = projected_cell(grid_before, 1, 1);
        let b1_before = projected_cell(grid_before, 1, 2);
        log_cell("pre", "A1", a1_before);
        log_cell("pre", "B1", b1_before);
        assert_eq!(
            a1_before.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Literal),
            "A1 authored Literal at open (None = token-mismatch blank)"
        );
        assert_eq!(
            b1_before.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Formula),
            "B1 authored Formula at open (None = token-mismatch blank)"
        );
        assert_eq!(a1_before.value, number("7"), "A1 = 7 at open");
        assert_eq!(b1_before.value, number("21"), "B1 = A1*3 = 21 at open");
        let open_tick = calculated_tick("B1 at open", b1_before);
        let b1_epoch_before = b1_before.value_epoch;
        let projection_epoch_before = grid_before.projection_epoch;
        println!(
            "W011 edit [pre] open-recalc tick={open_tick} B1 value_epoch={b1_epoch_before} \
             projection_epoch={projection_epoch_before}"
        );

        // EDIT: A1 7 -> 10 through the intent. The grid id is the one the
        // snapshot projected — the same `sheet:{node}` address a skin
        // round-trips — never composed here from engine internals.
        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        println!("W011 edit: receipt = {receipt:?}");
        assert!(
            receipt.accepted,
            "'10' into A1 of the loaded fixture is accepted: {:?}",
            receipt.error
        );
        let (entered_grid, row, col, outcome) = grid_cell_entered_change(&receipt);
        assert_eq!(*entered_grid, grid_id, "the receipt names the edited grid");
        assert_eq!((row, col), (1, 1), "the receipt names A1");
        match outcome {
            GridEntryOutcomeProjection::Literal { value } => assert_eq!(
                *value,
                number("10"),
                "OxFml classified '10' as a literal and the receipt carries it"
            ),
            other => panic!("expected the Literal outcome, got {other:?}"),
        }

        // POST: a fresh snapshot of LIVE truth.
        let after = document.snapshot();
        let (grid_id_after, grid_after) = loaded_sheet1_grid(&after);
        assert_eq!(
            grid_id_after, grid_id,
            "the grid identity is stable across the edit"
        );
        let a1_after = projected_cell(grid_after, 1, 1);
        let b1_after = projected_cell(grid_after, 1, 2);
        log_cell("post", "A1", a1_after);
        log_cell("post", "B1", b1_after);
        let a1_authored = a1_after
            .authored
            .as_ref()
            .expect("A1 carries authored metadata after the edit");
        let b1_authored = b1_after
            .authored
            .as_ref()
            .expect("B1 carries authored metadata after the edit");
        assert_eq!(a1_authored.kind, GridAuthoredKindProjection::Literal);
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("10"),
            "A1's authored literal text is the entered 10"
        );
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*3"),
            "an edit of A1 never rewrites B1's authored truth"
        );
        assert_eq!(a1_after.value, number("10"), "A1 = 10 after the edit");
        assert_eq!(
            b1_after.value,
            number("30"),
            "B1 = A1*3 = 30: the edit really recalculated the LOADED workbook"
        );
        let edit_tick = calculated_tick("B1 after the edit", b1_after);
        println!(
            "W011 edit [post] edit tick={edit_tick} (open-recalc tick={open_tick}) \
             B1 value_epoch={} (was {b1_epoch_before}) projection_epoch={} (was {projection_epoch_before})",
            b1_after.value_epoch, grid_after.projection_epoch
        );
        assert!(
            edit_tick > open_tick,
            "B1's 30 is Calculated on a tick NEWER than the open-recalc tick ({edit_tick} > {open_tick})"
        );
        assert!(
            b1_after.value_epoch > b1_epoch_before,
            "B1's value_epoch advanced ({} > {b1_epoch_before})",
            b1_after.value_epoch
        );
        assert!(
            grid_after.projection_epoch > projection_epoch_before,
            "the grid's projection_epoch advanced ({} > {projection_epoch_before})",
            grid_after.projection_epoch
        );
        assert_eq!(
            grid_after.cells.len(),
            2,
            "the edit added no cell (exactly A1 and B1 remain): {:#?}",
            grid_after.cells
        );

        // Automatic mode: the edit itself drained; nothing is left dirty.
        let calc = after.workbook_calc.as_ref().expect("workbook_calc is Some");
        assert_eq!(calc.mode, CalcModeProjection::Automatic);
        assert!(
            calc.sheets.iter().all(|sheet| !sheet.dirty),
            "under Automatic the edit drained itself: {:?}",
            calc.sheets
        );
    }

    /// dtc-j7n8.6 acceptance (1), formula half: formula text into `B1`
    /// itself is ACCEPTED by the ENGINE on the loaded session — OxCalc's
    /// three-way literal/formula/clear branch with OxFml as the sole
    /// interpretation authority; no host-side `=` classification exists and
    /// the former `=`-prefix typed rejection exists nowhere in code. The
    /// receipt is `GridCellEntered { Formula }` with no unresolved names and
    /// the freshly published 40 (`A1` was first edited to 10, so the new
    /// formula reads LIVE truth), and a snapshot shows `B1` authored
    /// `=A1*4` = 40, `Calculated`. This retires the stale "literal-only +
    /// typed rejection" register line IN CODE.
    ///
    /// SAVE SCOPE, kept honest: a formula-TEXT change on an existing cell is
    /// save-RESTRICTED — OxDoc's round-trip policy needs a synchronized
    /// `FormulaTopology` and typed-rejects it otherwise (dtc-j7n8.7 documents
    /// that `UnsupportedRoundTripFeature` path). This test therefore never
    /// saves, and any assertion that must see the file's truth again rebuilds
    /// a fresh session rather than reusing this one.
    #[test]
    fn enter_grid_cell_formula_text_on_loaded_fixture_is_accepted() {
        let mut document = open_w011_fixture_document();
        let before = document.snapshot();
        let (grid_id, _) = loaded_sheet1_grid(&before);

        // A1 -> 10 first, so the new formula reads live truth (B1 = 30 here).
        let literal = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        assert!(
            literal.accepted,
            "A1 -> 10 is accepted: {:?}",
            literal.error
        );

        // Formula text into B1 itself. Nothing host-side looks at the `=`.
        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 2,
            text: "=A1*4".to_string(),
        });
        println!("W011 formula edit: receipt = {receipt:?}");
        assert!(
            receipt.accepted,
            "formula text into a loaded formula cell is ACCEPTED by the engine's three-way branch: {:?}",
            receipt.error
        );
        let (entered_grid, row, col, outcome) = grid_cell_entered_change(&receipt);
        assert_eq!(*entered_grid, grid_id);
        assert_eq!((row, col), (1, 2), "the receipt names B1");
        match outcome {
            GridEntryOutcomeProjection::Formula {
                unresolved_names,
                value,
            } => {
                assert!(
                    unresolved_names.is_empty(),
                    "=A1*4 references no defined names: {unresolved_names:?}"
                );
                assert_eq!(
                    *value,
                    number("40"),
                    "the receipt carries the freshly published B1 = A1*4 = 40"
                );
            }
            other => panic!("expected the Formula outcome, got {other:?}"),
        }

        let after = document.snapshot();
        let (_, grid_after) = loaded_sheet1_grid(&after);
        let b1 = projected_cell(grid_after, 1, 2);
        log_cell("post-formula", "B1", b1);
        let b1_authored = b1
            .authored
            .as_ref()
            .expect("B1 carries authored metadata after the formula edit");
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*4"),
            "B1's authored truth is now the entered formula text"
        );
        assert_eq!(b1.value, number("40"), "B1 = A1*4 = 40");
        assert!(
            matches!(
                b1.provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "B1's 40 is a fresh engine value: {:?}",
            b1.provenance
        );

        // This session is now save-RESTRICTED (formula text changed) and is
        // never saved. Later assertions rebuild a fresh session: the file's
        // truth is untouched by the live edit.
        drop(document);
        let fresh = open_w011_fixture_document().snapshot();
        let (_, grid_fresh) = loaded_sheet1_grid(&fresh);
        let b1_fresh = projected_cell(grid_fresh, 1, 2);
        assert_eq!(
            b1_fresh
                .authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=A1*3"),
            "a fresh session is the file's truth again"
        );
        assert_eq!(b1_fresh.value, number("21"));
    }

    /// dtc-j7n8.6 acceptance (1), Recalculate half: after the Automatic-mode
    /// edit the existing no-op receipt contract holds on the LOADED session
    /// (the mirror of `recalculate_intent_with_nothing_dirty_is_a_noop_receipt`
    /// below, on real bytes): `Recalculate` is accepted, its `CalcStateChanged`
    /// delta carries `last_recalc_tick == None` (nothing was dirty, no tick
    /// minted), no sheet is dirty, no `GridChanged` is emitted — and LIVE
    /// truth is exactly what the edit published: `B1` = 30 on the edit's own
    /// tick and epoch.
    #[test]
    fn recalculate_intent_after_automatic_edit_on_loaded_fixture_is_a_noop_receipt() {
        let mut document = open_w011_fixture_document();
        let before = document.snapshot();
        let (grid_id, _) = loaded_sheet1_grid(&before);

        let edit = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        assert!(edit.accepted, "A1 -> 10 is accepted: {:?}", edit.error);
        let edited = document.snapshot();
        let (_, grid_edited) = loaded_sheet1_grid(&edited);
        let b1_edited = projected_cell(grid_edited, 1, 2);
        log_cell("post-edit", "B1", b1_edited);
        assert_eq!(
            b1_edited.value,
            number("30"),
            "Automatic mode: the edit itself recalculated; no F9 needed"
        );
        let edit_tick = calculated_tick("B1 after the edit", b1_edited);

        // F9 with nothing dirty.
        let receipt = document.dispatch(WorkspaceIntent::Recalculate);
        println!("W011 recalc no-op: receipt = {receipt:?}");
        assert!(
            receipt.accepted,
            "Recalculate is accepted even as a no-op on the loaded session"
        );
        match receipt.delta.changes.as_slice() {
            [WorkspaceDeltaChange::CalcStateChanged(projection)] => {
                assert_eq!(projection.mode, CalcModeProjection::Automatic);
                assert_eq!(
                    projection.last_recalc_tick, None,
                    "nothing was dirty after the Automatic edit, so no tick is minted"
                );
                assert!(
                    projection.sheets.iter().all(|sheet| !sheet.dirty),
                    "no sheet is dirty: {:?}",
                    projection.sheets
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

        // The no-op left LIVE truth exactly as the edit published it.
        let after = document.snapshot();
        let (_, grid_after) = loaded_sheet1_grid(&after);
        let b1_after = projected_cell(grid_after, 1, 2);
        log_cell("post-recalc", "B1", b1_after);
        assert_eq!(b1_after.value, number("30"), "B1 stays 30");
        assert_eq!(
            calculated_tick("B1 after the no-op", b1_after),
            edit_tick,
            "a no-op recalc mints no tick: B1 keeps the edit's tick"
        );
        assert_eq!(
            b1_after.value_epoch, b1_edited.value_epoch,
            "a no-op recalc advances no value epoch"
        );
    }

    /// dtc-j7n8.18 acceptance — mirror-patchable deltas, on real bytes. The
    /// receipt from `EnterGridCell { A1, "10" }` on the loaded fixture carries
    /// BOTH `GridCellEntered { Literal 10 }` AND `GridChanged` for Sheet1's
    /// grid — and nothing else, this being a ONE-sheet workbook with no peer
    /// to move — and the `GridChanged` projection already holds the refreshed
    /// `B1` = 30 (`Calculated` on a tick newer than the open recalc) plus the
    /// authored layer (`A1` literal `10`, `B1` still `=A1*3`). Then the
    /// decisive check: `session_channel::apply_delta` patches the PRE-edit
    /// snapshot with the receipt's delta alone, and the patched mirror equals
    /// a fresh post-edit `snapshot()` — the grid cell for cell, and the whole
    /// `WorkspaceState` (under Automatic on a single-sheet workbook an entry
    /// changes nothing outside the edited grid; the cross-sheet half of the
    /// contract is the demo-workbook tests below). No snapshot crossed the
    /// boundary; the mirror shows `B1` = 30 from the delta.
    #[test]
    fn enter_grid_cell_on_loaded_fixture_receipt_carries_grid_changed_that_patches_a_mirror() {
        use dnacalc_skin_ir::session_channel::{
            apply_delta, change_kind, delta_is_fully_applicable,
        };

        let mut document = open_w011_fixture_document();
        let before = document.snapshot();
        let (grid_id, grid_before) = loaded_sheet1_grid(&before);
        let b1_before = projected_cell(grid_before, 1, 2);
        assert_eq!(b1_before.value, number("21"), "B1 = 21 at open");
        let open_tick = calculated_tick("B1 at open", b1_before);

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        let kinds: Vec<_> = receipt.delta.changes.iter().map(change_kind).collect();
        println!(
            "W011 mirror: receipt accepted={} changes={kinds:?}",
            receipt.accepted
        );
        assert!(
            receipt.accepted,
            "A1 -> 10 is accepted: {:?}",
            receipt.error
        );

        // BOTH changes on the ONE receipt, and only those two.
        let (entered_grid, row, col, outcome) = grid_cell_entered_change(&receipt);
        assert_eq!(*entered_grid, grid_id, "the hint names the edited grid");
        assert_eq!((row, col), (1, 1), "the hint names A1");
        match outcome {
            GridEntryOutcomeProjection::Literal { value } => {
                assert_eq!(*value, number("10"), "the hint carries the literal 10");
            }
            other => panic!("expected the Literal outcome, got {other:?}"),
        }
        let changed = grid_changed_for(&receipt, &grid_id);
        assert_eq!(
            kinds,
            vec!["grid_cell_entered", "grid_changed"],
            "exactly the UI hint and the mirror patch, in that order"
        );

        // The patch already carries the refreshed truth — values, provenance
        // AND the authored layer — so a mirror needs nothing else.
        let a1 = projected_cell(changed, 1, 1);
        let b1 = projected_cell(changed, 1, 2);
        log_cell("grid-changed", "A1", a1);
        log_cell("grid-changed", "B1", b1);
        assert_eq!(a1.value, number("10"), "the patch carries A1 = 10");
        assert_eq!(
            a1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Literal)
        );
        assert_eq!(
            a1.authored
                .as_ref()
                .and_then(|authored| authored.literal_text.as_deref()),
            Some("10"),
            "the patch carries A1's authored literal text"
        );
        assert_eq!(
            b1.value,
            number("30"),
            "the GridChanged carries the REFRESHED B1 = A1*3 = 30"
        );
        assert_eq!(
            b1.authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=A1*3"),
            "the patch keeps B1's authored formula text"
        );
        let patch_tick = calculated_tick("B1 in the GridChanged", b1);
        assert!(
            patch_tick > open_tick,
            "the patch's B1 is Calculated on the edit tick ({patch_tick} > {open_tick})"
        );
        assert_eq!(
            changed.cells.len(),
            2,
            "the patch carries exactly A1 and B1: {:#?}",
            changed.cells
        );

        // Mirror patch: pre-edit snapshot + this delta == fresh post-edit
        // snapshot. The mirror never receives a snapshot.
        assert!(
            delta_is_fully_applicable(&receipt.delta),
            "an executor may send this delta WITHOUT a snapshot: {kinds:?}"
        );
        let mut mirror = before.clone();
        apply_delta(&mut mirror, &receipt.delta)
            .expect("the entry receipt's delta patches a retained mirror in place");
        let after = document.snapshot();
        assert_eq!(
            mirror.grids.get(&grid_id),
            after.grids.get(&grid_id),
            "the patched mirror's Sheet1 grid equals the fresh snapshot's, cell for cell"
        );
        assert_eq!(
            mirror, after,
            "under Automatic on the single-sheet fixture an entry changes nothing outside the \
             edited grid: the patched mirror IS the fresh snapshot"
        );
        let b1_mirror = projected_cell(
            mirror.grids.get(&grid_id).expect("the mirror holds Sheet1"),
            1,
            2,
        );
        log_cell("mirror", "B1", b1_mirror);
        assert_eq!(
            b1_mirror.value,
            number("30"),
            "the mirror shows B1 = 30 without ever receiving a snapshot"
        );
        assert_eq!(
            calculated_tick("B1 in the mirror", b1_mirror),
            patch_tick,
            "the mirror's B1 carries the edit tick the patch carried"
        );
    }

    // ------------------------------------------------------------------
    // dtc-j7n8.18 (round 2): the cross-sheet half of the mirror contract.
    // The one-sheet fixture cannot show it; the demo workbook's
    // `Sheet2!A1 = =Sheet1!A1+Sheet1!A5` can. An independent verifier
    // refuted round 1 here: the engine recalculates Sheet2 inside the
    // Sheet1 edit (OxCalc `propagate_cross_sheet_edit`) but the receipt
    // patched Sheet1 only, so a mirror trusting the fully-applicable delta
    // kept a stale Sheet2 with no resync signal.
    // ------------------------------------------------------------------

    /// The demo workbook as a document plus its two grids, addressed the way
    /// a skin addresses them (`sheets[i].grid_node_id` off the snapshot).
    fn demo_document_with_two_grids() -> (
        DocumentSession,
        dnacalc_skin_ir::NodeId,
        dnacalc_skin_ir::NodeId,
    ) {
        let document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        let state = document.snapshot();
        assert_eq!(
            state.sheets.len(),
            2,
            "the demo publishes two sheets: {:?}",
            state.sheets
        );
        let sheet1 = state.sheets[0].grid_node_id.clone();
        let sheet2 = state.sheets[1].grid_node_id.clone();
        (document, sheet1, sheet2)
    }

    /// The projected grid for `grid` in a `WorkspaceState`, failing with the
    /// projected grid keys when it is missing.
    fn grid_in<'a>(
        state: &'a WorkspaceState,
        grid: &dnacalc_skin_ir::NodeId,
    ) -> &'a GridProjection {
        state.grids.get(grid).unwrap_or_else(|| {
            panic!(
                "grid {grid:?} is projected; grids = {:?}",
                state.grids.keys().collect::<Vec<_>>()
            )
        })
    }

    /// The `workbook_calc` dirty flag a state carries for `grid`'s sheet.
    fn sheet_dirty_in(state: &WorkspaceState, grid: &dnacalc_skin_ir::NodeId) -> bool {
        state
            .workbook_calc
            .as_ref()
            .expect("a workbook snapshot carries workbook_calc")
            .sheets
            .iter()
            .find(|sheet| sheet.grid_node_id == *grid)
            .unwrap_or_else(|| panic!("workbook_calc has a row for {grid:?}"))
            .dirty
    }

    /// dtc-j7n8.18 (round 2) — the cross-sheet half of the mirror contract.
    /// On the demo workbook (`Sheet2!A1 = =Sheet1!A1+Sheet1!A5`), entering
    /// `Sheet1!A1 = 7` makes the engine recalculate Sheet2 in the same
    /// transaction (Automatic mode) — and the receipt says so: exactly
    /// `[grid_cell_entered, grid_changed (Sheet1), grid_changed (Sheet2)]`,
    /// Sheet2's patch carrying the RECALCULATED `A1` = 7 + 5 = 12 as
    /// `Calculated` on the edit's own tick (one tick for the whole edit
    /// transaction, W062 R4.8) with its authored cross-sheet formula intact.
    /// The delta is fully applicable, and `apply_delta` over the pre-edit
    /// snapshot equals a fresh post-edit `snapshot()` as a WHOLE
    /// `WorkspaceState` — Sheet2 included: the mirror shows Sheet2!A1 = 12
    /// without ever receiving a snapshot.
    #[test]
    fn enter_grid_cell_on_demo_workbook_receipt_patches_the_cross_sheet_dependent_too() {
        use dnacalc_skin_ir::session_channel::{
            apply_delta, change_kind, delta_is_fully_applicable,
        };

        let (mut document, sheet1, sheet2) = demo_document_with_two_grids();
        let before = document.snapshot();
        let sheet2_a1_before = projected_cell(grid_in(&before, &sheet2), 1, 1);
        log_cell("pre-edit Sheet2", "A1", sheet2_a1_before);
        assert_eq!(
            sheet2_a1_before.value,
            number("6"),
            "Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 1 + 5 = 6 at mount"
        );
        let mount_tick = calculated_tick("Sheet2!A1 at mount", sheet2_a1_before);

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: sheet1.clone(),
            row: 1,
            col: 1,
            text: "7".to_string(),
        });
        let kinds: Vec<_> = receipt.delta.changes.iter().map(change_kind).collect();
        println!(
            "W011 cross-sheet mirror: accepted={} changes={kinds:?} patched={:?}",
            receipt.accepted,
            grid_changed_targets(&receipt)
        );
        assert!(
            receipt.accepted,
            "Sheet1!A1 -> 7 is accepted: {:?}",
            receipt.error
        );
        assert_eq!(
            kinds,
            vec!["grid_cell_entered", "grid_changed", "grid_changed"],
            "the hint, the edited sheet's patch, then the recalculated dependent sheet's patch"
        );
        assert_eq!(
            grid_changed_targets(&receipt),
            vec![sheet1.clone(), sheet2.clone()],
            "the edited sheet first, then the dependent it moved, in sheet order"
        );

        // The edited sheet's patch: A1 = 7, B1 = A1*10 = 70 on the edit tick.
        let sheet1_patch = grid_changed_for(&receipt, &sheet1);
        assert_eq!(projected_cell(sheet1_patch, 1, 1).value, number("7"));
        let b1 = projected_cell(sheet1_patch, 1, 2);
        log_cell("grid-changed Sheet1", "B1", b1);
        assert_eq!(
            b1.value,
            number("70"),
            "Sheet1!B1 = A1*10 recalculated to 70"
        );
        let edit_tick = calculated_tick("Sheet1!B1 in the patch", b1);
        assert!(
            edit_tick > mount_tick,
            "the edit minted a tick newer than the mount ({edit_tick} > {mount_tick})"
        );

        // The dependent sheet's patch: the recalculated cross-sheet value,
        // on the SAME tick, with the authored formula and the untouched
        // literals beside it.
        let sheet2_patch = grid_changed_for(&receipt, &sheet2);
        let sheet2_a1 = projected_cell(sheet2_patch, 1, 1);
        log_cell("grid-changed Sheet2", "A1", sheet2_a1);
        assert_eq!(
            sheet2_a1.value,
            number("12"),
            "the receipt carries the RECALCULATED Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 7 + 5 = 12"
        );
        assert_eq!(
            sheet2_a1
                .authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=Sheet1!A1+Sheet1!A5"),
            "Sheet2!A1 keeps its authored cross-sheet formula"
        );
        assert_eq!(
            calculated_tick("Sheet2!A1 in the patch", sheet2_a1),
            edit_tick,
            "one tick for the whole edit transaction: the dependent sheet recalculated on the \
             edit's tick"
        );
        assert_eq!(projected_cell(sheet2_patch, 1, 2).value, number("100"));
        assert_eq!(projected_cell(sheet2_patch, 2, 2).value, number("200"));

        // Mirror: pre-edit snapshot + this delta == fresh post-edit
        // snapshot, as a whole. The mirror never receives a snapshot.
        assert!(
            delta_is_fully_applicable(&receipt.delta),
            "an executor may send this delta WITHOUT a snapshot: {kinds:?}"
        );
        let mut mirror = before.clone();
        apply_delta(&mut mirror, &receipt.delta)
            .expect("the entry receipt's delta patches a retained mirror in place");
        let after = document.snapshot();
        assert_eq!(
            mirror.grids.get(&sheet2),
            after.grids.get(&sheet2),
            "the patched mirror's Sheet2 grid equals the fresh snapshot's, cell for cell"
        );
        assert_eq!(
            mirror, after,
            "the patched mirror IS the fresh snapshot: every sheet the edit moved rode the delta"
        );
        let sheet2_a1_mirror = projected_cell(grid_in(&mirror, &sheet2), 1, 1);
        log_cell("mirror Sheet2", "A1", sheet2_a1_mirror);
        assert_eq!(
            sheet2_a1_mirror.value,
            number("12"),
            "the mirror shows Sheet2!A1 = 12 without ever receiving a snapshot"
        );
    }

    /// dtc-j7n8.18 (round 2) — the other direction: a peer sheet the edit
    /// did NOT move stays out of the receipt. `Sheet1!A2` has no cross-sheet
    /// reader (Sheet2!A1 reads A1 and A5 only), so entering `Sheet1!A2 = 20`
    /// recalculates Sheet1 alone: exactly `[grid_cell_entered, grid_changed
    /// (Sheet1)]` with `B2` = 200 in the patch and no Sheet2 patch — and the
    /// patched mirror still equals the fresh snapshot as a whole, because
    /// Sheet2's projection did not change (A1 stays 6 on its mount tick).
    /// This is the pin the app adapter's demo test relies on: an unrelated
    /// sheet never re-renders on a keystroke.
    #[test]
    fn enter_grid_cell_on_demo_workbook_leaves_an_unmoved_peer_sheet_out_of_the_receipt() {
        use dnacalc_skin_ir::session_channel::{apply_delta, change_kind};

        let (mut document, sheet1, sheet2) = demo_document_with_two_grids();
        let before = document.snapshot();

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: sheet1.clone(),
            row: 2,
            col: 1,
            text: "20".to_string(),
        });
        let kinds: Vec<_> = receipt.delta.changes.iter().map(change_kind).collect();
        println!(
            "W011 unmoved peer: accepted={} changes={kinds:?} patched={:?}",
            receipt.accepted,
            grid_changed_targets(&receipt)
        );
        assert!(
            receipt.accepted,
            "Sheet1!A2 -> 20 is accepted: {:?}",
            receipt.error
        );
        assert_eq!(
            kinds,
            vec!["grid_cell_entered", "grid_changed"],
            "the hint and the edited sheet's patch only: Sheet2 read nothing the edit touched"
        );
        assert_eq!(
            grid_changed_targets(&receipt),
            vec![sheet1.clone()],
            "no GridChanged names the unmoved Sheet2"
        );
        let sheet1_patch = grid_changed_for(&receipt, &sheet1);
        assert_eq!(projected_cell(sheet1_patch, 2, 1).value, number("20"));
        assert_eq!(
            projected_cell(sheet1_patch, 2, 2).value,
            number("200"),
            "Sheet1!B2 = A2*10 recalculated to 200"
        );

        let mut mirror = before.clone();
        apply_delta(&mut mirror, &receipt.delta)
            .expect("the entry receipt's delta patches a retained mirror in place");
        let after = document.snapshot();
        assert_eq!(
            after.grids.get(&sheet2),
            before.grids.get(&sheet2),
            "Sheet2's projection is untouched by an edit it does not read"
        );
        assert_eq!(
            mirror, after,
            "the patched mirror IS the fresh snapshot: nothing that did not ride the delta moved"
        );
        let sheet2_a1 = projected_cell(grid_in(&after, &sheet2), 1, 1);
        assert_eq!(sheet2_a1.value, number("6"), "Sheet2!A1 stays 6");
        assert_eq!(
            calculated_tick("Sheet2!A1 after the unrelated edit", sheet2_a1),
            calculated_tick(
                "Sheet2!A1 at mount",
                projected_cell(grid_in(&before, &sheet2), 1, 1)
            ),
            "Sheet2!A1 keeps its mount tick: it was not recalculated"
        );
    }

    /// dtc-j7n8.18 (round 2), Manual mode on the two-sheet demo — what the
    /// delta proves there and what it does not. Under Manual the engine
    /// suppresses the edit's recalc AND its cross-sheet propagation, so
    /// entering `Sheet1!A1 = 7` leaves Sheet2's projection untouched (A1
    /// still 6, still `Calculated` on its mount tick): the receipt is exactly
    /// `[grid_cell_entered, grid_changed (Sheet1)]`, Sheet1's patch carrying
    /// the stale-but-honest published values (`A1` still 1, `Stale`) under
    /// the new authored literal `7`, and the patched mirror's GRIDS equal
    /// the fresh snapshot's. The known gap, pinned so its closure is visible:
    /// the receipt carries no `CalcStateChanged`, so the mirror's
    /// `workbook_calc` (Sheet1 is now dirty on a fresh read) lags — that is
    /// dtc-j7n8.20's scope; when it lands, the last two asserts flip.
    #[test]
    fn manual_mode_entry_on_demo_workbook_patches_grids_only_and_pins_the_calc_state_gap() {
        use dnacalc_skin_ir::session_channel::{apply_delta, change_kind};

        let (mut document, sheet1, sheet2) = demo_document_with_two_grids();
        let mode_receipt = document.dispatch(WorkspaceIntent::SetCalcMode {
            mode: CalcModeProjection::Manual,
        });
        assert!(mode_receipt.accepted, "SetCalcMode(Manual) is accepted");
        // The mirror's baseline is taken AFTER the mode switch, so it holds
        // Manual and every sheet clean — exactly what a retained mirror that
        // applied the SetCalcMode receipt would hold.
        let before = document.snapshot();
        assert!(
            !sheet_dirty_in(&before, &sheet1),
            "Sheet1 is clean at the baseline"
        );

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: sheet1.clone(),
            row: 1,
            col: 1,
            text: "7".to_string(),
        });
        let kinds: Vec<_> = receipt.delta.changes.iter().map(change_kind).collect();
        println!(
            "W011 manual cross-sheet: accepted={} changes={kinds:?} patched={:?}",
            receipt.accepted,
            grid_changed_targets(&receipt)
        );
        assert!(
            receipt.accepted,
            "Sheet1!A1 -> 7 is accepted under Manual: {:?}",
            receipt.error
        );
        assert_eq!(
            kinds,
            vec!["grid_cell_entered", "grid_changed"],
            "Manual suppresses cross-sheet propagation: no Sheet2 patch, and no CalcStateChanged \
             yet (dtc-j7n8.20)"
        );
        let sheet1_patch = grid_changed_for(&receipt, &sheet1);
        let a1 = projected_cell(sheet1_patch, 1, 1);
        log_cell("grid-changed Sheet1 (Manual)", "A1", a1);
        assert_eq!(
            a1.authored
                .as_ref()
                .and_then(|authored| authored.literal_text.as_deref()),
            Some("7"),
            "the patch carries the new authored literal"
        );
        assert_eq!(
            a1.value,
            number("1"),
            "Manual: the published value stays the pre-edit 1, not silently 7"
        );
        assert!(
            matches!(a1.provenance, Some(ValueProvenanceProjection::Stale { .. })),
            "Manual: the stale published value is tagged Stale: {:?}",
            a1.provenance
        );

        let mut mirror = before.clone();
        apply_delta(&mut mirror, &receipt.delta)
            .expect("the entry receipt's delta patches a retained mirror in place");
        let after = document.snapshot();
        assert_eq!(
            after.grids.get(&sheet2),
            before.grids.get(&sheet2),
            "Manual: Sheet2 was not recalculated, so its projection is exactly the baseline's"
        );
        assert_eq!(
            mirror.grids, after.grids,
            "every GRID the mirror holds equals the fresh snapshot's"
        );

        // The pinned known gap (dtc-j7n8.20): the mirror's calc state lags.
        assert!(
            sheet_dirty_in(&after, &sheet1),
            "a fresh read shows Sheet1 dirty after the Manual-mode edit"
        );
        assert!(
            !sheet_dirty_in(&mirror, &sheet1),
            "KNOWN GAP dtc-j7n8.20: the receipt carries no CalcStateChanged, so the mirror's \
             Sheet1 dirty flag lags the fresh snapshot — flip this assert when that bead lands"
        );
        assert_ne!(
            mirror.workbook_calc, after.workbook_calc,
            "KNOWN GAP dtc-j7n8.20: workbook_calc is the one field the entry delta does not \
             cover under Manual — flip this assert when that bead lands"
        );
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.7): `HostCommand::SaveActiveXlsx` end to end — the
    // save proof at the command surface a host actually calls, and the
    // typed-refusal lanes (RichTree, no backing source, out-of-scope
    // edit). The decisive assertion is on the REOPENED BYTES' raw OxDoc
    // events, never on an engine readout after a reload (which would
    // recalculate and mask a stale cache).
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::{
        OXDOC_CELL_ADD_REJECTION, dropped_entries, log_ledger, open_xlsx_raw, raw_cell_payload,
        raw_sheet_cells,
    };
    use oxdoc_model::CellPayload;
    use oxdoc_xlsx::XlsxError;
    use oxdoc_xlsx::model::DocumentFidelityLedger;

    /// Execute `SaveActiveXlsx`, destructure the `Saved` outcome, and log
    /// the byte count and every ledger entry.
    fn execute_save(
        stage: &str,
        document: &mut DocumentSession,
    ) -> (Vec<u8>, DocumentFidelityLedger) {
        let outcome = document
            .execute(HostCommand::SaveActiveXlsx)
            .unwrap_or_else(|err| panic!("SaveActiveXlsx [{stage}] failed: {err} / {err:?}"));
        match outcome {
            HostCommandOutcome::Saved { bytes, save_ledger } => {
                println!("W011 SaveActiveXlsx [{stage}]: {} bytes", bytes.len());
                log_ledger(
                    &format!("W011 SaveActiveXlsx [{stage}] ledger"),
                    &save_ledger,
                );
                (bytes, save_ledger)
            }
            other => panic!("expected Saved, got {other:?}"),
        }
    }

    /// dtc-j7n8.7 acceptance (1)/(2) at the command surface — THE campaign
    /// save proof through the commands a host actually issues:
    /// `OpenXlsxBytes` -> `EnterGridCell { A1, "10" }` (dispatch; LIVE `B1`
    /// = 30) -> `SaveActiveXlsx` returns `Saved { bytes, save_ledger }` with
    /// no `Dropped` entry; the SAVED bytes reopened RAW through OxDoc (no
    /// engine) carry `A1 = Number(10)` and `B1 = Formula { text: "A1*3",
    /// cached: Number(30) }` — the cached `<v>` is the fresh 30, not the
    /// file's stale 21. The save replaces nothing (same opened workbook,
    /// LIVE still 30), and a second `OpenXlsxBytes` of the saved bytes
    /// closes the full loop at the command surface: `Opened` reports one
    /// literal + one bound formula, the snapshot shows `A1` authored 10 and
    /// `B1` authored `=A1*3` = 30 (authored kinds asserted before values,
    /// the token-mismatch blank order).
    #[test]
    fn execute_save_active_xlsx_after_edit_returns_bytes_with_cached_30() {
        let mut document = open_w011_fixture_document();
        let (grid_id, _) = loaded_sheet1_grid(&document.snapshot());

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        assert!(
            receipt.accepted,
            "A1 -> 10 is accepted: {:?}",
            receipt.error
        );
        let live = document.snapshot();
        let (_, grid_live) = loaded_sheet1_grid(&live);
        let b1_live = projected_cell(grid_live, 1, 2);
        log_cell("pre-save", "B1", b1_live);
        assert_eq!(
            b1_live.value,
            number("30"),
            "LIVE truth before the save: B1 = A1*3 = 30"
        );

        let (bytes, save_ledger) = execute_save("after edit", &mut document);
        assert!(!bytes.is_empty(), "the save produced package bytes");
        let dropped = dropped_entries(&save_ledger);
        assert!(
            dropped.is_empty(),
            "no Dropped ledger entries (the fixture has no calc chain to drop): {dropped:?}"
        );

        // A save replaces nothing: still the opened workbook, LIVE still 30.
        assert_eq!(document.model_name(), "Workbook");
        let DocumentSession::Workbook(session) = &document else {
            panic!("the session survives the save as a Workbook, got {document:?}");
        };
        assert_eq!(session.workspace_id().as_str(), XLSX_WORKSPACE_ID);
        assert_eq!(session.document_name(), Some("a1_times_three.xlsx"));
        let after_save = document.snapshot();
        let (_, grid_after_save) = loaded_sheet1_grid(&after_save);
        assert_eq!(
            projected_cell(grid_after_save, 1, 2).value,
            number("30"),
            "LIVE truth is untouched by the save"
        );

        // FILE truth of the SAVED bytes — raw OxDoc events, no engine.
        let reopened = open_xlsx_raw(&bytes);
        log_ledger(
            "W011 SaveActiveXlsx reopen load ledger",
            &reopened.load_ledger,
        );
        let cells = raw_sheet_cells(&reopened, "Sheet1");
        println!("W011 SaveActiveXlsx reopen: raw Sheet1 cells = {cells:?}");
        assert_eq!(cells.len(), 2, "exactly A1 and B1: {cells:?}");
        let a1 = raw_cell_payload(&cells, 1, 1);
        let b1 = raw_cell_payload(&cells, 1, 2);
        println!("W011 SaveActiveXlsx reopen: A1 payload = {a1:?}");
        println!("W011 SaveActiveXlsx reopen: B1 payload = {b1:?}");
        assert_eq!(
            a1,
            &CellPayload::Number(10.0),
            "A1 is saved as the edited literal 10"
        );
        assert_eq!(
            b1,
            &CellPayload::Formula {
                region: None,
                text: Some("A1*3".to_string()),
                cached: Some(Box::new(CellPayload::Number(30.0))),
            },
            "THE TRAP: B1 keeps its formula text A1*3 AND its cached <v> is the fresh 30, \
             not the file's stale 21"
        );

        // Full loop at the command surface: open the saved bytes.
        let outcome = document
            .execute(HostCommand::OpenXlsxBytes {
                bytes,
                name: Some("a1_times_three_saved.xlsx".to_string()),
            })
            .expect("the saved bytes open through OxDoc and ingest into the engine");
        let HostCommandOutcome::Opened {
            cells,
            formulas_bound,
            recalc_path,
            ..
        } = &outcome
        else {
            panic!("expected Opened, got {outcome:?}");
        };
        assert_eq!((*cells, *formulas_bound), (1, 1), "A1 literal, B1 bound");
        assert_eq!(*recalc_path, LoadRecalcPath::Automatic);
        let reloaded = document.snapshot();
        let (_, grid_reloaded) = loaded_sheet1_grid(&reloaded);
        let a1_reloaded = projected_cell(grid_reloaded, 1, 1);
        let b1_reloaded = projected_cell(grid_reloaded, 1, 2);
        log_cell("reloaded", "A1", a1_reloaded);
        log_cell("reloaded", "B1", b1_reloaded);
        let a1_authored = a1_reloaded
            .authored
            .as_ref()
            .expect("A1 carries authored metadata (None = token-mismatch blank)");
        let b1_authored = b1_reloaded
            .authored
            .as_ref()
            .expect("B1 carries authored metadata (None = token-mismatch blank)");
        assert_eq!(a1_authored.kind, GridAuthoredKindProjection::Literal);
        assert_eq!(a1_authored.literal_text.as_deref(), Some("10"));
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(b1_authored.source_text.as_deref(), Some("=A1*3"));
        assert_eq!(a1_reloaded.value, number("10"), "A1 = 10 reloaded");
        assert_eq!(
            b1_reloaded.value,
            number("30"),
            "B1 = A1*3 = 30 on the reloaded document"
        );
    }

    /// dtc-j7n8.14 (Wave 3b) at the command surface, on the LOADED
    /// cross-sheet fixture (`Sheet1!A1 = 2`; `Sheet2!A1 = =Sheet1!A1*5`
    /// cached 10): `OpenXlsxBytes` reports two sheets / one literal / one
    /// bound formula and the snapshot shows `Sheet2!A1` = 10 `Calculated`;
    /// `EnterGridCell { Sheet1!A1, "4" }` is accepted and — the dtc-j7n8.18
    /// receipt shape, here proven on real bytes rather than the in-memory
    /// demo — carries exactly `[grid_cell_entered, grid_changed (Sheet1),
    /// grid_changed (Sheet2)]`, Sheet2's patch holding the RECALCULATED
    /// `A1` = 20 on the edit's tick with its authored cross-sheet formula,
    /// so `apply_delta` over the pre-edit snapshot equals a fresh snapshot
    /// as a whole; then `SaveActiveXlsx` returns bytes whose raw OxDoc
    /// events say `Sheet1!A1 = Number(4)` and `Sheet2!A1 = Formula {
    /// "Sheet1!A1*5", cached: Number(20) }` — the bead's own cached-20
    /// proof, at the surface a host actually calls. The `workbook.rs` lane
    /// carries the session-level assertions; this one adds the receipt.
    #[test]
    fn execute_cross_sheet_edit_receipt_patches_sheet2_and_save_reopens_with_cached_20() {
        use crate::xlsx_fixture::w011_cross_sheet_fixture_bytes;
        use dnacalc_skin_ir::session_channel::{
            apply_delta, change_kind, delta_is_fully_applicable,
        };

        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        let outcome = document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: w011_cross_sheet_fixture_bytes(),
                name: Some("cross_sheet.xlsx".to_string()),
            })
            .expect("OxDoc opens and the engine ingests the committed cross-sheet fixture");
        let HostCommandOutcome::Opened {
            sheet_count,
            cells,
            formulas_bound,
            recalc_path,
            ..
        } = &outcome
        else {
            panic!("expected Opened, got {outcome:?}");
        };
        assert_eq!(
            (*sheet_count, *cells, *formulas_bound),
            (2, 1, 1),
            "two sheets; Sheet1!A1 the literal; Sheet2!A1 bound"
        );
        assert_eq!(*recalc_path, LoadRecalcPath::Automatic);

        // PRE: the loaded truth through the skin's mount surface.
        let before = document.snapshot();
        assert_eq!(
            before
                .sheets
                .iter()
                .map(|sheet| sheet.display_name.as_str())
                .collect::<Vec<_>>(),
            ["Sheet1", "Sheet2"],
            "two tab-strip rows in workbook order: {:?}",
            before.sheets
        );
        let sheet1 = before.sheets[0].grid_node_id.clone();
        let sheet2 = before.sheets[1].grid_node_id.clone();
        let s2a1_before = projected_cell(grid_in(&before, &sheet2), 1, 1);
        log_cell("cross-sheet pre", "Sheet2!A1", s2a1_before);
        assert_eq!(
            s2a1_before
                .authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=Sheet1!A1*5"),
            "Sheet2!A1 authored Formula at open (None = token-mismatch blank)"
        );
        assert_eq!(
            s2a1_before.value,
            number("10"),
            "Sheet2!A1 = Sheet1!A1*5 = 10 at open"
        );
        let open_tick = calculated_tick("Sheet2!A1 at open", s2a1_before);

        // EDIT: Sheet1!A1 2 -> 4 through the intent.
        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: sheet1.clone(),
            row: 1,
            col: 1,
            text: "4".to_string(),
        });
        let kinds: Vec<_> = receipt.delta.changes.iter().map(change_kind).collect();
        println!(
            "W011 cross-sheet fixture receipt: accepted={} changes={kinds:?} patched={:?}",
            receipt.accepted,
            grid_changed_targets(&receipt)
        );
        assert!(
            receipt.accepted,
            "Sheet1!A1 -> 4 on the loaded fixture is accepted: {:?}",
            receipt.error
        );
        let (entered_grid, row, col, _) = grid_cell_entered_change(&receipt);
        assert_eq!((entered_grid, row, col), (&sheet1, 1, 1));
        assert_eq!(
            kinds,
            vec!["grid_cell_entered", "grid_changed", "grid_changed"],
            "the hint, the edited sheet's patch, then the recalculated dependent sheet's patch"
        );
        assert_eq!(
            grid_changed_targets(&receipt),
            vec![sheet1.clone(), sheet2.clone()],
            "the edited sheet first, then the dependent it moved"
        );
        let sheet1_patch = grid_changed_for(&receipt, &sheet1);
        let s1a1_patch = projected_cell(sheet1_patch, 1, 1);
        log_cell("cross-sheet grid-changed Sheet1", "A1", s1a1_patch);
        assert_eq!(s1a1_patch.value, number("4"), "Sheet1!A1 = 4 in the patch");
        let edit_tick = calculated_tick("Sheet1!A1 in the patch", s1a1_patch);
        assert!(
            edit_tick > open_tick,
            "the edit minted a tick newer than the open-recalc ({edit_tick} > {open_tick})"
        );
        let sheet2_patch = grid_changed_for(&receipt, &sheet2);
        let s2a1_patch = projected_cell(sheet2_patch, 1, 1);
        log_cell("cross-sheet grid-changed Sheet2", "A1", s2a1_patch);
        assert_eq!(
            s2a1_patch.value,
            number("20"),
            "the receipt carries the RECALCULATED Sheet2!A1 = Sheet1!A1*5 = 4*5 = 20"
        );
        assert_eq!(
            s2a1_patch
                .authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=Sheet1!A1*5"),
            "Sheet2!A1 keeps its authored cross-sheet formula"
        );
        assert_eq!(
            calculated_tick("Sheet2!A1 in the patch", s2a1_patch),
            edit_tick,
            "one tick for the whole edit transaction: Sheet2 recalculated on the edit's tick"
        );

        // Mirror: pre-edit snapshot + this delta == fresh post-edit snapshot.
        assert!(
            delta_is_fully_applicable(&receipt.delta),
            "an executor may send this delta WITHOUT a snapshot: {kinds:?}"
        );
        let mut mirror = before.clone();
        apply_delta(&mut mirror, &receipt.delta)
            .expect("the entry receipt's delta patches a retained mirror in place");
        let after = document.snapshot();
        assert_eq!(
            mirror, after,
            "the patched mirror IS the fresh snapshot: both sheets the edit moved rode the delta"
        );
        assert_eq!(
            projected_cell(grid_in(&mirror, &sheet2), 1, 1).value,
            number("20"),
            "the mirror shows Sheet2!A1 = 20 without ever receiving a snapshot"
        );

        // SAVE: the bytes' raw OxDoc events, per sheet, no engine.
        let (bytes, save_ledger) = execute_save("cross-sheet, after edit", &mut document);
        let dropped = dropped_entries(&save_ledger);
        assert!(dropped.is_empty(), "no Dropped ledger entries: {dropped:?}");
        let reopened = open_xlsx_raw(&bytes);
        log_ledger(
            "W011 cross-sheet SaveActiveXlsx reopen load ledger",
            &reopened.load_ledger,
        );
        let sheet1_cells = raw_sheet_cells(&reopened, "Sheet1");
        let sheet2_cells = raw_sheet_cells(&reopened, "Sheet2");
        println!("W011 cross-sheet SaveActiveXlsx reopen: raw Sheet1 cells = {sheet1_cells:?}");
        println!("W011 cross-sheet SaveActiveXlsx reopen: raw Sheet2 cells = {sheet2_cells:?}");
        assert_eq!(sheet1_cells.len(), 1, "exactly Sheet1!A1: {sheet1_cells:?}");
        assert_eq!(sheet2_cells.len(), 1, "exactly Sheet2!A1: {sheet2_cells:?}");
        assert_eq!(
            raw_cell_payload(&sheet1_cells, 1, 1),
            &CellPayload::Number(4.0),
            "Sheet1!A1 is saved as the edited literal 4"
        );
        assert_eq!(
            raw_cell_payload(&sheet2_cells, 1, 1),
            &CellPayload::Formula {
                region: None,
                text: Some("Sheet1!A1*5".to_string()),
                cached: Some(Box::new(CellPayload::Number(20.0))),
            },
            "THE TRAP, cross-sheet: Sheet2!A1 keeps its formula text Sheet1!A1*5 AND its cached \
             <v> is the fresh 20, not the file's stale 10"
        );
    }

    /// dtc-j7n8.7 acceptance (2): `SaveActiveXlsx` on a `RichTree` session
    /// is the typed [`HostCommandError::UnsupportedByModel`] — no workbook,
    /// no OxDoc source — never a panic; the session is untouched.
    #[test]
    fn save_active_xlsx_on_rich_tree_session_is_typed_unsupported_by_model() {
        let mut document = DocumentSession::RichTree(RichTreeSession::new());

        let error = document
            .execute(HostCommand::SaveActiveXlsx)
            .expect_err("a RichTree session has no workbook to save");
        match &error {
            HostCommandError::UnsupportedByModel { model, command } => {
                assert_eq!(*model, "RichTree");
                assert_eq!(*command, "SaveActiveXlsx");
            }
            other => panic!("expected UnsupportedByModel, got {other:?}"),
        }
        println!("HostCommandError (Display): {error}");
        assert_eq!(
            document.model_name(),
            "RichTree",
            "the session is untouched"
        );
    }

    /// dtc-j7n8.7 acceptance (2): `SaveActiveXlsx` on an in-memory workbook
    /// (the demo, no OxDoc source) is the typed
    /// `HostCommandError::Workbook(WorkbookSessionError::NoBackingSource)`;
    /// the demo session is untouched.
    #[test]
    fn save_active_xlsx_on_workbook_without_source_is_typed_error() {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());

        let error = document
            .execute(HostCommand::SaveActiveXlsx)
            .expect_err("an in-memory workbook has no OxDoc source to round-trip against");
        match &error {
            HostCommandError::Workbook(WorkbookSessionError::NoBackingSource) => {}
            other => panic!("expected Workbook(NoBackingSource), got {other:?}"),
        }
        println!("HostCommandError (Display): {error}");

        let DocumentSession::Workbook(session) = &document else {
            panic!("the demo session survives the refused save, got {document:?}");
        };
        assert_eq!(session.workspace_id().as_str(), "workbook:demo");
        assert!(session.xlsx_source().is_none());
        assert_eq!(session.sheets().unwrap().len(), 2);
    }

    /// dtc-j7n8.7 acceptance (2), errors typed end to end: a cell ADD (`C1`
    /// is empty in the fixture) is accepted into the live model but is
    /// save-restricted, and `SaveActiveXlsx` returns OxDoc's
    /// `UnsupportedRoundTripFeature` — pinned to the exact observed text,
    /// `OXDOC_CELL_ADD_REJECTION`, which does not name the cell (OxDoc's
    /// surgical merge compares cell key sets) — inside
    /// `HostCommandError::Workbook(WorkbookSessionError::Xlsx(_))` — no
    /// bytes, no panic — while the live session keeps `C1 = 5`.
    #[test]
    fn save_active_xlsx_of_out_of_scope_edit_is_typed_rejection_end_to_end() {
        let mut document = open_w011_fixture_document();
        let (grid_id, _) = loaded_sheet1_grid(&document.snapshot());

        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 3,
            text: "5".to_string(),
        });
        assert!(
            receipt.accepted,
            "C1 = 5 is accepted live (edit scope is wider than save scope): {:?}",
            receipt.error
        );

        let error = document
            .execute(HostCommand::SaveActiveXlsx)
            .expect_err("a cell add is refused by OxDoc's round-trip policy");
        match &error {
            HostCommandError::Workbook(WorkbookSessionError::Xlsx(
                XlsxError::UnsupportedRoundTripFeature(message),
            )) => {
                println!("W011 SaveActiveXlsx [C1 add]: typed OxDoc rejection = {message:?}");
                assert_eq!(
                    message.as_str(),
                    OXDOC_CELL_ADD_REJECTION,
                    "the rejection is OxDoc's cell add/remove refusal, pinned to the observed \
                     text (it does not name C1: the surgical merge compares cell key sets)"
                );
            }
            other => {
                panic!("expected Workbook(Xlsx(UnsupportedRoundTripFeature(_))), got {other:?}")
            }
        }
        println!("HostCommandError (Display): {error}");

        // The refused save left LIVE truth intact: C1 = 5 alongside A1/B1.
        let state = document.snapshot();
        let (_, grid) = loaded_sheet1_grid(&state);
        assert_eq!(
            grid.cells.len(),
            3,
            "A1, B1 and the added C1: {:#?}",
            grid.cells
        );
        assert_eq!(projected_cell(grid, 1, 3).value, number("5"), "C1 = 5 live");
        assert_eq!(
            projected_cell(grid, 1, 2).value,
            number("21"),
            "B1 = 21 live"
        );
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
        // dtc-j7n8.18: the receipt carries the `GridCellEntered` hint AND
        // the edited sheet's `GridChanged`; the hint is located, not
        // slice-matched — every original assertion on it stands.
        let (grid_node_id, row, col, outcome) = grid_cell_entered_change(&receipt);
        assert_eq!(*grid_node_id, grid);
        assert_eq!(row, 1);
        assert_eq!(col, 1);
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
        // The patch beside the hint already carries the entered literal.
        let changed = grid_changed_for(&receipt, &grid);
        let a1 = projected_cell(changed, 1, 1);
        assert_eq!(
            a1.value,
            NodeValueProjection::Number {
                raw: "10".to_string(),
                display: "10".to_string(),
            },
            "the GridChanged carries A1 = 10"
        );
        assert_eq!(
            a1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Literal),
            "the GridChanged carries A1's authored layer"
        );
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
        // dtc-j7n8.18: hint located beside the `GridChanged`, not
        // slice-matched; the original assertion stands.
        let (_, _, _, outcome) = grid_cell_entered_change(&receipt);
        let hint_value = match outcome {
            GridEntryOutcomeProjection::Formula {
                unresolved_names,
                value,
            } => {
                assert_eq!(
                    unresolved_names,
                    &vec!["TaxRate".to_string()],
                    "the undefined name surfaces on the Formula receipt"
                );
                value
            }
            other => panic!("expected Formula outcome, got {other:?}"),
        };
        // The patch carries the bound formula's authored text and the same
        // (unresolved-name, `#NAME?`) value the hint projected.
        let changed = grid_changed_for(&receipt, &grid);
        let a1 = projected_cell(changed, 1, 1);
        println!("H6 formula: GridChanged A1 = {:?}", a1.value);
        assert_eq!(
            a1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Formula)
        );
        assert_eq!(
            a1.authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=TaxRate*2"),
            "the GridChanged carries the authored formula text"
        );
        assert_eq!(
            a1.value, *hint_value,
            "the patch and the hint agree on the entered cell's value"
        );
        assert!(
            matches!(a1.value, NodeValueProjection::Error(_)),
            "an unresolved name evaluates to an error (#NAME?) until seeded: {:?}",
            a1.value
        );
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
        // dtc-j7n8.18: hint located beside the `GridChanged`, not
        // slice-matched; the original assertion stands.
        let (_, _, _, outcome) = grid_cell_entered_change(&receipt);
        assert!(matches!(outcome, GridEntryOutcomeProjection::Cleared));
        // The patch no longer carries the 10 that was in A1.
        let changed = grid_changed_for(&receipt, &grid);
        assert!(
            !changed.cells.iter().any(|cell| {
                cell.row == 1 && cell.col == 1 && cell.value != NodeValueProjection::Empty
            }),
            "no non-empty value remains at A1 in the GridChanged: {:#?}",
            changed.cells
        );
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
        // dtc-j7n8.18: hint located beside the `GridChanged`, not
        // slice-matched; the original assertion stands.
        let (_, _, _, outcome) = grid_cell_entered_change(&receipt);
        assert!(matches!(outcome, GridEntryOutcomeProjection::Cleared));
        // `ClearGridCell` patches the sheet too: A1 carries no value and no
        // authored content in the GridChanged.
        let changed = grid_changed_for(&receipt, &grid);
        assert!(
            changed
                .cells
                .iter()
                .filter(|cell| cell.row == 1 && cell.col == 1)
                .all(|cell| {
                    cell.value == NodeValueProjection::Empty
                        && cell.authored.as_ref().is_none_or(|authored| {
                            authored.kind == dnacalc_skin_ir::GridAuthoredKindProjection::Empty
                        })
                }),
            "A1 is value-empty and authored-empty in the GridChanged: {:#?}",
            changed.cells
        );

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

    /// dtc-j7n8.13 (W011 Wave 3a) — the Manual calc-mode lane on REAL bytes,
    /// through the commands and intents a host actually issues. The Manual
    /// twin of the fixture (`calcMode="manual"`, otherwise `a1_times_three`)
    /// opens with `Opened.recalc_path == Manual`; the mount snapshot shows
    /// `workbook_calc` Manual, the sheet DIRTY (the load seeds it; F9 owed),
    /// and `B1` = 21 with provenance `FileCached` (the file's cache, never
    /// evaluated — the first time that variant is populated on a live
    /// document). Pinned as observed (dtc-j7n8.24; flip when it lands): the
    /// literal `A1` has NO projected cell at all before F9 — the Manual load
    /// publishes formula caches only, and the grid projection is keyed off
    /// published cells — although its authored `literal_text` is readable
    /// through the session's authored window. `EnterGridCell { A1, "10" }`
    /// is accepted (an entry receipt with its `GridChanged` patch, per
    /// dtc-j7n8.18) but under Manual nothing recalculates: `B1` still 21,
    /// still `FileCached` (the engine re-tags only `Calculated` values
    /// `Stale`, see `manual_mode_save_before_recalc_writes_last_calculated_cache`
    /// in `workbook.rs`), the sheet dirty on a fresh read. Then `Recalculate`:
    /// accepted, its `CalcStateChanged` carries the drain's tick and Manual,
    /// and the snapshot shows `B1` = 30 `Calculated { tick }`, `A1` = 10,
    /// nothing dirty. Authored kinds are asserted before values (the
    /// token-mismatch blank order, dtc-j7n8.5). The entry receipt's
    /// `CalcStateChanged` gap under Manual is dtc-j7n8.20's, not asserted
    /// here either way.
    #[test]
    fn manual_fixture_dispatch_keeps_file_cached_21_until_recalculate_then_30() {
        let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
        let outcome = document
            .execute(HostCommand::OpenXlsxBytes {
                bytes: crate::xlsx_fixture::w011_manual_fixture_bytes(),
                name: Some("a1_times_three_manual.xlsx".to_string()),
            })
            .expect("OxDoc opens and the engine ingests the committed W011 Manual twin");
        println!("W011 manual OpenXlsxBytes outcome: {outcome:?}");
        let HostCommandOutcome::Opened {
            cells,
            formulas_bound,
            recalc_path,
            ..
        } = &outcome
        else {
            panic!("expected Opened, got {outcome:?}");
        };
        assert_eq!((*cells, *formulas_bound), (1, 1), "A1 literal, B1 bound");
        assert_eq!(
            *recalc_path,
            LoadRecalcPath::Manual,
            "the Manual twin takes the engine's Manual load path"
        );

        // Mount: Manual, dirty (F9 owed), and B1 renders the FILE's cache.
        let mounted = document.snapshot();
        let (grid_id, grid) = loaded_sheet1_grid(&mounted);
        let calc = mounted
            .workbook_calc
            .as_ref()
            .expect("a workbook snapshot carries workbook_calc");
        assert_eq!(calc.mode, CalcModeProjection::Manual, "the file's mode");
        assert!(
            sheet_dirty_in(&mounted, &grid_id),
            "OBSERVED: a Manual load seeds the sheet dirty — the first F9 is owed"
        );
        let projected_a1 = |grid: &GridProjection| {
            grid.cells
                .iter()
                .find(|cell| cell.row == 1 && cell.col == 1)
                .cloned()
        };
        println!("W011 manual mount: projected A1 = {:?}", projected_a1(grid));
        assert_eq!(
            projected_a1(grid),
            None,
            "OBSERVED (dtc-j7n8.24): the literal A1 has no projected cell before F9 — \
             the Manual load publishes formula caches only"
        );
        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let sheet = workbook::parse_sheet_grid_node_id(&grid_id).unwrap();
        let a1_authored = session
            .grid_authored_cells(sheet, 1, 1, 1, 1)
            .unwrap()
            .into_iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the authored window even though nothing published it");
        assert_eq!(a1_authored.literal_text.as_deref(), Some("7"));
        let b1 = projected_cell(grid, 1, 2);
        log_cell("manual mount", "B1", b1);
        assert_eq!(
            b1.authored.as_ref().map(|authored| authored.kind),
            Some(GridAuthoredKindProjection::Formula),
            "B1 authored Formula (None = token-mismatch blank)"
        );
        assert_eq!(
            b1.authored
                .as_ref()
                .and_then(|authored| authored.source_text.as_deref()),
            Some("=A1*3")
        );
        assert_eq!(b1.value, number("21"), "B1 renders the file's cached 21");
        assert_eq!(
            b1.provenance,
            Some(ValueProvenanceProjection::FileCached),
            "B1's 21 is FileCached on the live document: no engine pass ran"
        );

        // Edit under Manual: accepted, patched, NOT recalculated.
        let receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id.clone(),
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        let kinds: Vec<_> = receipt
            .delta
            .changes
            .iter()
            .map(dnacalc_skin_ir::session_channel::change_kind)
            .collect();
        println!(
            "W011 manual edit: accepted={} changes={kinds:?}",
            receipt.accepted
        );
        assert!(
            receipt.accepted,
            "A1 -> 10 is accepted under Manual: {:?}",
            receipt.error
        );
        assert!(
            kinds.contains(&"grid_cell_entered") && kinds.contains(&"grid_changed"),
            "the entry receipt carries its entered change and the Sheet1 patch: {kinds:?}"
        );
        let edited = document.snapshot();
        let (_, grid) = loaded_sheet1_grid(&edited);
        println!(
            "W011 manual edited: projected A1 = {:?}",
            projected_a1(grid)
        );
        assert_eq!(
            projected_a1(grid),
            None,
            "Manual: the edit published nothing, so A1 still has no projected cell \
             (dtc-j7n8.24); its authored truth is 10"
        );
        let DocumentSession::Workbook(session) = &document else {
            unreachable!("workbook session")
        };
        let a1_authored = session
            .grid_authored_cells(sheet, 1, 1, 1, 1)
            .unwrap()
            .into_iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the authored window");
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("10"),
            "authored truth is the edit"
        );
        let b1 = projected_cell(grid, 1, 2);
        log_cell("manual edited", "B1", b1);
        assert_eq!(b1.value, number("21"), "Manual: B1 still shows 21, not 30");
        assert_eq!(
            b1.provenance,
            Some(ValueProvenanceProjection::FileCached),
            "B1's cache is still the file's — never Calculated, so never re-tagged Stale"
        );
        assert!(
            sheet_dirty_in(&edited, &grid_id),
            "a fresh read shows Sheet1 dirty behind the undrained edit"
        );

        // F9: the drain mints a tick, B1 = 30 Calculated on it.
        let recalc_receipt = document.dispatch(WorkspaceIntent::Recalculate);
        assert!(
            recalc_receipt.accepted,
            "Recalculate is accepted: {:?}",
            recalc_receipt.error
        );
        let projection = recalc_receipt
            .delta
            .changes
            .iter()
            .find_map(|change| match change {
                WorkspaceDeltaChange::CalcStateChanged(projection) => Some(projection),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "Recalculate carries a CalcStateChanged: {:?}",
                    recalc_receipt.delta.changes
                )
            });
        println!("W011 manual recalc: CalcStateChanged = {projection:?}");
        assert_eq!(
            projection.mode,
            CalcModeProjection::Manual,
            "F9 never flips the mode"
        );
        let tick = projection
            .last_recalc_tick
            .expect("a genuine drain mints a tick");
        assert!(
            projection.sheets.iter().all(|sheet| !sheet.dirty),
            "the drain cleared every dirty flag: {:?}",
            projection.sheets
        );
        let recalculated = document.snapshot();
        let (_, grid) = loaded_sheet1_grid(&recalculated);
        let a1 = projected_cell(grid, 1, 1);
        let b1 = projected_cell(grid, 1, 2);
        log_cell("manual recalculated", "A1", a1);
        log_cell("manual recalculated", "B1", b1);
        assert_eq!(b1.value, number("30"), "after Recalculate, B1 = A1*3 = 30");
        assert_eq!(
            b1.provenance,
            Some(ValueProvenanceProjection::Calculated { tick_id: tick }),
            "B1's 30 is Calculated on the drain's tick"
        );
        assert_eq!(a1.value, number("10"), "A1 publishes 10 after the drain");
        assert!(!sheet_dirty_in(&recalculated, &grid_id));
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

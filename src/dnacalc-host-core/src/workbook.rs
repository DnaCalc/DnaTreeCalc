//! `WorkbookSession` — a document session over a single strict-Excel
//! workbook, backed by one [`OxCalcDocumentContext`] (the W062 R5.8 document
//! surface).
//!
//! Per the front-end design (`FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §A.1):
//! a workbook session holds **one** context with **one** workspace whose root
//! carries [`NodeRole::Workbook`], and **one grid-backed node per sheet** — the
//! exact shape the engine's `sheets()` / `add_sheet` lifecycle verbs already
//! manage (consumer.rs). The host adds document identity and (in later beads)
//! dirty state and projection publication — never a second calculation-state
//! pot (D4 §1: one context, no wrapper).
//!
//! H2 scope is the crate seam: create the workbook workspace, add sheets, give
//! each new sheet a grid backing, and read/write single grid cells via the
//! engine's `set_grid_cell_value` verb. `EnterGridCell` (the universal authored
//! entry verb) and its receipt projection are **H6**, deliberately out of scope
//! here.
//!
//! H6 promotes the universal entry verbs (`enter_grid_cell`/`clear_grid_cell`)
//! to public API, and adds the `NodeId` <-> `TreeNodeId` sheet-address seam
//! (§A.2: "skins never see engine addresses or `TreeNodeId`") that the
//! `WorkspaceIntent`-level dispatch in `crate::lib` needs to resolve
//! `EnterGridCell { grid: NodeId, .. }` to an engine sheet node.
//!
//! ## The three truths (W011) — a constraint, not narration
//!
//! An opened workbook has three owners; "what is the value of B1" has a
//! different correct answer at each. **FILE truth** is the OxDoc source
//! (`xlsx_source`): the loaded bytes, the fidelity ledger, what a round-trip
//! save preserves — B1 is the file's cached `21` until a save projects a fresh
//! cache. **LIVE truth** is the [`OxCalcDocumentContext`]: authored cells,
//! published values, provenance — B1 is what the engine last published.
//! **SKIN truth** is a [`WorkspaceState`] snapshot: what renders, a capture of
//! LIVE truth that is stale the moment it is taken. Never read one truth
//! expecting another's answer; ingest copies FILE into LIVE exactly once.
//! A save ([`WorkbookSession::save_xlsx_bytes`]) goes the other way, once per
//! call: the engine projects LIVE truth (fresh formula caches) and OxDoc
//! merges it onto the FILE truth's package image; the bytes go back to the
//! caller, and FILE truth here stays the opened package.

use std::collections::BTreeMap;

use dnacalc_skin_ir::{
    GridAuthoredCellProjection, GridProjection, NodeId, NodeKey, SheetProjection, WorkspaceState,
};
use oxcalc_core::consumer::{
    GridBackingSeed, GridCellEntryOutcome, OxCalcDocumentContext, OxCalcDocumentError,
    OxCalcTreeGridView, OxCalcTreeWorkspaceCreate, OxCalcTreeWorkspaceId, SheetEnumerationRow,
};
use oxcalc_core::grid::coords::{ExcelGridBounds, ExcelGridCellAddress};
use oxcalc_core::grid::geometry::GridRect;
use oxcalc_core::grid::machine::GridEngineValidationMode;
use oxcalc_core::oxdoc_ingest::WorkbookLoadReport;
use oxcalc_core::structural::TreeNodeId;
use oxdoc_xlsx::model::DocumentFidelityLedger;
use oxdoc_xlsx::{
    HostOwnedXlsxSource, LoadProfile, XlsxError, XlsxSaveRequest, open_host_owned_xlsx_source,
    write_save_request,
};
use oxfunc_core::value::CalcValue;

use crate::grid_publication::{grid_authored_cell_projection, grid_projection_for};

/// The stable `NodeId` a workbook session projects for a sheet's grid-backed
/// node — the same `sheet:{node}` string this session already uses as the
/// grid id internally (`add_sheet`), so a skin's `EnterGridCell { grid, .. }`
/// round-trips through the identical stable address. Skins never see the raw
/// engine `TreeNodeId` (§A.2).
#[must_use]
pub fn sheet_grid_node_id(sheet: TreeNodeId) -> NodeId {
    NodeId::new(format!("sheet:{}", sheet.0))
}

/// Parse a [`sheet_grid_node_id`] projection back to its engine `TreeNodeId`.
/// `None` if `node_id` is not that exact stable shape (a skin never
/// constructs one itself; it only round-trips an id the host handed it).
#[must_use]
pub fn parse_sheet_grid_node_id(node_id: &NodeId) -> Option<TreeNodeId> {
    node_id
        .as_str()
        .strip_prefix("sheet:")
        .and_then(|rest| rest.parse::<u64>().ok())
        .map(TreeNodeId)
}

/// The engine root symbol for a workbook workspace. Kept distinct from the
/// tree-model root symbol so a workbook root is never confused for a general
/// tree root in diagnostics.
const WORKBOOK_ROOT_SYMBOL: &str = "__dnacalc_workbook__";

/// The stable workspace id [`crate::DocumentSession::execute`] gives a
/// workbook opened from `.xlsx` bytes (`HostCommand::OpenXlsxBytes`). One
/// constant, not the file name: the workspace id is an engine key that ends
/// up inside every grid address, while the user-facing file name is carried
/// separately as [`WorkbookSession::document_name`]. Opening a document
/// replaces the active session, so a single open document never collides with
/// itself.
pub const XLSX_WORKSPACE_ID: &str = "workbook:xlsx";

/// The workbook token OxCalc's ingest stamps on every grid it creates:
/// `book:{workspace}` (OxCalc `consumer.rs`, the Tier-A load plan's
/// `workbook_token`; `oxdoc_ingest.rs`'s own `ingested_address` test helper
/// derives the same). Half of the address-token trap: a grid loaded through
/// `load_workbook_model` is addressable **only** under this token.
fn ingested_workbook_token(workspace_id: &OxCalcTreeWorkspaceId) -> String {
    format!("book:{}", workspace_id.as_str())
}

/// The workbook token a hand-seeded grid carries: the bare workspace id, the
/// `GridBackingSeed.workbook_id` [`WorkbookSession::add_sheet`] has always
/// given a sheet created in memory ([`WorkbookSession::create`], the demo).
fn seeded_workbook_token(workspace_id: &OxCalcTreeWorkspaceId) -> String {
    workspace_id.as_str().to_string()
}

/// The host-owned OxDoc side of a workbook opened from `.xlsx` bytes (W011,
/// dtc-j7n8.3): the source package session kept for the later round-trip
/// save, the byte-free model context, and the load ledger — plus the
/// user-facing document name the bytes arrived under and, since dtc-j7n8.4,
/// the engine's own [`WorkbookLoadReport`] for the ingest of that source.
///
/// Per the OxDoc host-boundary contract the **host** owns this bundle; OxDoc
/// keeps no live document. It lives next to (never inside) the engine context
/// — one calculation-state pot, one source-package pot, both owned here. The
/// event stream is **not** copied here: the engine reads it straight from
/// `source.source_context.events()` at ingest and the source keeps it.
#[derive(Debug)]
struct XlsxSourceState {
    source: HostOwnedXlsxSource,
    name: Option<String>,
    load_report: WorkbookLoadReport,
}

/// One open strict-Excel workbook: a single [`OxCalcDocumentContext`] plus the
/// stable workspace id addressing its one workbook workspace — and, when the
/// workbook was opened from `.xlsx` bytes, the host-owned OxDoc source it came
/// from ([`WorkbookSession::xlsx_source`]).
///
/// The context — and therefore this session — is **neither `Send` nor `Sync`**
/// (it transitively holds a non-atomic `Rc<RichValue>` inside `CalcValue`); a
/// session is a single-threaded value that stays on its owning thread. See the
/// Send/Sync audit block in [`crate`] for the full finding and the W011 !Send
/// disposition (the worker owns its own context; only serde receipts cross the
/// thread boundary). The OxDoc source adds nothing new here: its package
/// session holds `Arc<Mutex<..>>` caches, but it is owned by this
/// single-threaded value and no `Send` bound is claimed anywhere.
#[derive(Debug)]
pub struct WorkbookSession {
    context: OxCalcDocumentContext,
    workspace_id: OxCalcTreeWorkspaceId,
    /// The single workbook-token authority (dtc-j7n8.4): the `workbook_id`
    /// half of every engine grid address this session composes. Set once per
    /// origin at construction — [`seeded_workbook_token`] for
    /// [`WorkbookSession::create`], [`ingested_workbook_token`] for
    /// [`WorkbookSession::open_xlsx_bytes`] — and read only through
    /// [`WorkbookSession::workbook_token`]. See that accessor for why a
    /// mismatch is a silent failure, never an error.
    workbook_token: String,
    /// Default grid geometry for freshly-added sheets (strict-Excel bounds).
    bounds: ExcelGridBounds,
    /// `Some` iff this workbook was opened from `.xlsx` bytes; an in-memory
    /// workbook ([`WorkbookSession::create`], the demo) has no source.
    xlsx: Option<XlsxSourceState>,
}

/// Errors a workbook session surfaces. Every arm wraps a typed engine error as
/// data (never a formatted string) so a host can present it structurally — the
/// W011 "engine errors are data" rule, kept.
// `large_enum_variant`: the `OxCalc` arm is inherently large (it wraps the
// engine's typed error) and dwarfs the small internal-invariant arm. Boxing
// would diverge from the sibling `TreeWorkspaceSessionError` by-value shape; see
// the `result_large_err` rationale on the impl below.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum WorkbookSessionError {
    #[error("engine rejected the workbook operation")]
    OxCalc(#[from] OxCalcDocumentError),
    /// OxDoc rejected the `.xlsx` bytes on open (W011, dtc-j7n8.3) or the
    /// round-trip save (dtc-j7n8.7). OxDoc's typed [`XlsxError`] travels as
    /// data (a corrupt zip, a missing part, an XML error in a named part, an
    /// `UnsupportedRoundTripFeature` describing the edit the save policy
    /// refuses), never flattened to a string.
    #[error("OxDoc rejected the xlsx package")]
    Xlsx(#[from] XlsxError),
    /// The workbook was not opened from `.xlsx` bytes, so there is no OxDoc
    /// source package to round-trip a save against (W011, dtc-j7n8.7): an
    /// in-memory workbook ([`WorkbookSession::create`], the demo) cannot be
    /// saved as xlsx until a fresh-export lane exists. A typed refusal, never
    /// a panic and never an empty package.
    #[error("the workbook session has no backing xlsx source to save against")]
    NoBackingSource,
    /// A cell write or read addressed a node that carries no grid backing —
    /// `set_grid_cell_value` / `grid_view` returned `Ok(None)`. In H2 every
    /// sheet is grid-backed at creation, so this is an internal-invariant
    /// violation, never a user error.
    #[error("sheet node {node:?} has no grid backing")]
    SheetNotGridBacked { node: TreeNodeId },
}

// `result_large_err`: `WorkbookSessionError` wraps `OxCalcDocumentError` by
// value, matching the established `TreeWorkspaceSessionError` convention in
// `dnatreecalc-host`. The engine error is inherently large; boxing it here would
// make host-core's error shape diverge from the sibling tree-session error for
// no caller benefit (these are single-session native/wasm calls, not a hot
// `Result`-returning inner loop). Kept by-value for cross-session consistency.
#[allow(clippy::result_large_err)]
impl WorkbookSession {
    /// Create an empty workbook session: one context, one workbook workspace
    /// (root carries [`NodeRole::Workbook`] via
    /// [`OxCalcTreeWorkspaceCreate::as_workbook`]), no sheets yet.
    ///
    /// The `workspace_id` is the caller-chosen stable document identity.
    pub fn create(workspace_id: impl Into<String>) -> Result<Self, WorkbookSessionError> {
        let mut context = Self::new_context();
        let workspace_id = context.create_workspace(Self::workspace_create(workspace_id))?;
        // Hand-seeded grids (`add_sheet`) carry the bare workspace id as their
        // workbook token — the create-origin half of the token authority.
        let workbook_token = seeded_workbook_token(&workspace_id);
        Ok(Self::from_parts(
            context,
            workspace_id,
            workbook_token,
            None,
        ))
    }

    /// Open a workbook session from `.xlsx` bytes through OxDoc and ingest it
    /// into the engine (W011, dtc-j7n8.3 + dtc-j7n8.4): OxDoc parses the
    /// package under [`LoadProfile::full()`], the host takes ownership of the
    /// resulting [`HostOwnedXlsxSource`] (source package session for the
    /// later save, byte-free model context, load ledger), and the engine's
    /// own `load_workbook_model` verb creates the workbook workspace and loads
    /// the source's `DocumentEvent` stream into it in one transaction — the
    /// same choreography OxCalc's `w011_five_step_round_trip_contract` proves
    /// on a hand-built stream, here on the real bytes. Under the file's
    /// `calcMode="auto"` the load issues Excel's open-recalc, so published
    /// values come back engine-`Calculated` (the file's cached values are
    /// replaced, not trusted); a `manual` file would render `FileCached` until
    /// an explicit recalculate. The report of what loaded is engine truth,
    /// surfaced as [`WorkbookSession::load_report`], never re-derived here.
    ///
    /// The events go straight from `source.source_context.events()` into the
    /// engine — no host-side `DocumentEvent -> GridBackingSeed` translation
    /// (that 2026-07 shim is superseded by the engine verb), no second copy of
    /// the stream, no host-side formula classification: bind authority is the
    /// engine's single key mint. Ingest-created grids are addressed under the
    /// `book:`-prefixed workbook token ([`ingested_workbook_token`]), which is
    /// why this origin sets the token authority differently from
    /// [`WorkbookSession::create`].
    ///
    /// `LoadProfile::full()` is mandatory, not a preference: the `Default`
    /// values-only profile omits `formula_topology`, and OxDoc's round-trip
    /// save later rejects formula-cell work without it — the W011 cached
    /// `B1 = 30` reopen depends on the topology being materialized here.
    ///
    /// Any OxDoc rejection (a corrupt zip, a missing part, an XML error) is
    /// returned as [`WorkbookSessionError::Xlsx`] carrying the typed
    /// [`XlsxError`]; an engine rejection of the stream comes back as
    /// [`WorkbookSessionError::OxCalc`] — never a panic, never a string. Host
    /// code never parses zip or XML itself; OxDoc is the xlsx crate.
    ///
    /// `name` is the user-facing document name the bytes arrived under (a
    /// file name, typically); it is identity for people, not for the engine —
    /// the engine key is `workspace_id`.
    pub fn open_xlsx_bytes(
        workspace_id: impl Into<String>,
        bytes: &[u8],
        name: Option<String>,
    ) -> Result<Self, WorkbookSessionError> {
        let source = open_host_owned_xlsx_source(std::io::Cursor::new(bytes), LoadProfile::full())?;
        let mut context = Self::new_context();
        // `load_workbook_model` creates the workspace itself (forcing the
        // Workbook role) and commits the whole stream as one revision; a
        // prior `create_workspace` here would collide, so there is none.
        let (workspace_id, load_report) = context.load_workbook_model(
            Self::workspace_create(workspace_id),
            source.source_context.events(),
        )?;
        let workbook_token = ingested_workbook_token(&workspace_id);
        Ok(Self::from_parts(
            context,
            workspace_id,
            workbook_token,
            Some(XlsxSourceState {
                source,
                name,
                load_report,
            }),
        ))
    }

    /// The one engine context both origins stand on. Engine validation spend
    /// is an explicit choice (OxCalc O-20, no Default on purpose). The
    /// interactive host samples the dual-engine oracle: every 16th recalc (and
    /// always the first recalc of a fresh backing) also runs the brute-force
    /// reference engine and compares — a live correctness heartbeat without
    /// paying the full-sheet oracle sweep on every keystroke. Suites/CI use
    /// DualValidated.
    fn new_context() -> OxCalcDocumentContext {
        OxCalcDocumentContext::new(GridEngineValidationMode::DualValidatedSampled { one_in: 16 })
    }

    /// The one workspace-create request both origins use: the caller's stable
    /// id, the workbook root symbol, the Workbook role — so an opened workbook
    /// can never drift from an in-memory one in engine shape.
    fn workspace_create(workspace_id: impl Into<String>) -> OxCalcTreeWorkspaceCreate {
        OxCalcTreeWorkspaceCreate::new(workspace_id)
            .with_root_symbol(WORKBOOK_ROOT_SYMBOL)
            .as_workbook()
    }

    /// Assemble the session once the engine side exists; strict-Excel bounds
    /// for both origins.
    fn from_parts(
        context: OxCalcDocumentContext,
        workspace_id: OxCalcTreeWorkspaceId,
        workbook_token: String,
        xlsx: Option<XlsxSourceState>,
    ) -> Self {
        Self {
            context,
            workspace_id,
            workbook_token,
            bounds: ExcelGridBounds::strict_excel(),
            xlsx,
        }
    }

    /// The workbook's stable workspace id.
    #[must_use]
    pub fn workspace_id(&self) -> &OxCalcTreeWorkspaceId {
        &self.workspace_id
    }

    /// The single workbook-token authority (dtc-j7n8.4): the `workbook_id`
    /// half of every [`ExcelGridCellAddress`] / [`GridRect`] this session
    /// composes for the engine. Every token consumer — `address_for`,
    /// `grid_authored_cells`, `defined_names.rs`'s `grid_rect_for`, and the
    /// `GridBackingSeed` in `add_sheet` — routes through here, never through
    /// `workspace_id` directly.
    ///
    /// Why this is an authority and not a convention: the engine looks grid
    /// addresses up by **equality**, workbook token included. A mismatched
    /// token is never an error — `enter_grid_cell` misses (`Ok(None)`), and
    /// `grid_authored_view(Some(rect))` synthesizes addresses from
    /// `rect.workbook_id` and returns a **blank** readout for every cell. The
    /// two origins differ exactly here: `create()` seeds grids under the bare
    /// workspace id, `open_xlsx_bytes()` gets ingest-created grids under
    /// `book:{workspace}`.
    #[must_use]
    pub(crate) fn workbook_token(&self) -> &str {
        &self.workbook_token
    }

    /// The engine's own report of what [`WorkbookSession::open_xlsx_bytes`]
    /// loaded (dtc-j7n8.4) — sheet/literal/bound-formula counts, the recalc
    /// path the load ran (`Automatic` open-recalc vs `Manual` render-from-
    /// cache), the ingest fidelity ledger, bind degradations — or `None` for
    /// an in-memory workbook. Engine truth, surfaced as-is: the host never
    /// re-derives these counts from the source.
    #[must_use]
    pub fn load_report(&self) -> Option<&WorkbookLoadReport> {
        self.xlsx.as_ref().map(|state| &state.load_report)
    }

    /// The host-owned OxDoc source this workbook was opened from — `None` for
    /// an in-memory workbook ([`WorkbookSession::create`], the demo). The
    /// bundle is OxDoc's plain public-field triple: `source_context` (the
    /// package session, whose `events()` are the document stream the ingest
    /// bead reads and whose image the save bead round-trips), `model_context`
    /// (byte-free sheet summaries and capabilities), and `load_ledger` (what
    /// the load preserved, projected, or dropped).
    #[must_use]
    pub fn xlsx_source(&self) -> Option<&HostOwnedXlsxSource> {
        self.xlsx.as_ref().map(|state| &state.source)
    }

    /// The user-facing document name the workbook was opened under (the file
    /// name a host handed to `open_xlsx_bytes`) — `None` for an in-memory
    /// workbook, and also `None` for bytes opened without a name.
    #[must_use]
    pub fn document_name(&self) -> Option<&str> {
        self.xlsx.as_ref().and_then(|state| state.name.as_deref())
    }

    /// Save the workbook back to `.xlsx` bytes through OxDoc (W011,
    /// dtc-j7n8.7): the engine projects the WHOLE model to a neutral
    /// `oxdoc-model` output stream (`project_workbook_model_output`, OxCalc
    /// W062 R6.6 / C12) and OxDoc round-trips it against the source package
    /// this session was opened from (`write_save_request`, round-trip mode).
    /// Returns the package bytes plus OxDoc's save ledger — what was
    /// preserved, projected, or dropped — as typed data. Read-only on the
    /// session: the projection never advances the revision, and the OxDoc
    /// source is borrowed, not consumed.
    ///
    /// ## The stale-cache trap, and why this is the only save path
    ///
    /// The silent failure this seam is most likely to produce is a file whose
    /// `A1` says 10 while `B1`'s cached `<v>` still says 21. Two things close
    /// it. (1) The event stream is never hand-patched here: the projection is
    /// engine truth end to end — Tier A (header from calc settings, sheets in
    /// registry order, authored cells as literals or `Formula { text, cached }`
    /// with the leading `=` stripped for the wire) re-derived from the calc
    /// model, Tier B (style tables, sheet views, dimensions, the
    /// present-but-empty differential style table) replayed verbatim from the
    /// sealed ingest facts, `CalcChainHint` omitted — and every formula
    /// cell's `cached` is read FRESH from the published readout at projection
    /// time (C12: fresh-cache-by-construction). The 2026-07 "clone the source
    /// events and patch `A1`/`B1`" recipe is superseded and must not return.
    /// (2) The proof is file-level: the acceptance test reopens the SAVED
    /// bytes through OxDoc and asserts the raw `B1` payload
    /// (`Formula { text: "A1*3", cached: Number(30) }`); an engine readout
    /// after a reload would recalculate and mask a stale cache.
    ///
    /// The projection reads the current publication whatever its provenance:
    /// under `CalcMode::Manual` with undrained edits it writes the last
    /// CALCULATED caches — the pre-edit values, Excel's own last-calculated
    /// semantics for a manual workbook saved without a recalc. The Wave 1
    /// fixture is `Automatic`, so an edit drains before any save and the
    /// caches are always fresh; the Manual lane is dtc-j7n8.13.
    ///
    /// ## What OxDoc's round-trip policy accepts
    ///
    /// Existing-cell literal edits and cached-value refresh of existing
    /// formula cells (OxDoc regenerates the whole `<c>` element) — the W011
    /// scope. Refused with a typed [`XlsxError::UnsupportedRoundTripFeature`]
    /// (a `String`-payload variant; discriminate by observed message, never
    /// by wording assumed in advance): cell add/remove, formula add/remove, a
    /// formula-text change without a synchronized `FormulaTopology`, styled
    /// cell start tags. Any formula-cell payload change — a pure cached
    /// refresh included — makes OxDoc drop `xl/calcChain.xml` (a perf hint
    /// with no fidelity content) with a `Dropped` ledger entry when the
    /// package carries one; the W011 fixture carries none. Never assert
    /// calc-chain byte identity.
    ///
    /// The session's own FILE truth is untouched by a save: `xlsx_source()`
    /// still holds the package the workbook was opened from, and the returned
    /// bytes are the caller's to persist (the shell owns file I/O). Dirty
    /// tracking / `DocumentStatus` and rebasing the source on the saved bytes
    /// are later beads. An in-memory workbook ([`WorkbookSession::create`],
    /// the demo) has no source to round-trip against and is refused with
    /// [`WorkbookSessionError::NoBackingSource`].
    pub fn save_xlsx_bytes(
        &self,
    ) -> Result<(Vec<u8>, DocumentFidelityLedger), WorkbookSessionError> {
        let source = self
            .xlsx_source()
            .ok_or(WorkbookSessionError::NoBackingSource)?;
        // Engine truth, whole model, fresh caches (C12). Never hand-patched.
        let output = self
            .context
            .project_workbook_model_output(&self.workspace_id)?;
        // OxDoc's writer needs `Write + Seek` (the zip central directory), so
        // a bare `Vec<u8>` is not enough — a cursor over one is.
        let mut cursor = std::io::Cursor::new(Vec::new());
        let save_ledger = write_save_request(
            XlsxSaveRequest::round_trip(&source.source_context, &output),
            &mut cursor,
        )?;
        Ok((cursor.into_inner(), save_ledger))
    }

    /// The underlying engine context (H4, `defined_names.rs`'s own module
    /// seam: the defined-name verb set is document-surface API on
    /// `OxCalcDocumentContext` directly, not re-wrapped per-verb here).
    #[must_use]
    pub(crate) fn context(&self) -> &OxCalcDocumentContext {
        &self.context
    }

    pub(crate) fn context_mut(&mut self) -> &mut OxCalcDocumentContext {
        &mut self.context
    }

    /// The session's default grid geometry (H4's `grid_rect_for` seam).
    #[must_use]
    pub(crate) fn bounds(&self) -> ExcelGridBounds {
        self.bounds
    }

    /// Add a sheet by display name and give it an empty grid backing, so a
    /// caller can immediately write cells into it. Returns the sheet's stable
    /// node id (the identity a rename preserves and a delete tombstones).
    ///
    /// The engine's `add_sheet` inserts a Sheet-role node but does **not**
    /// attach a grid; the host attaches an empty grid via `set_node_grid` so
    /// every enumerated sheet is grid-backed (the §A.1 "one grid-backed node
    /// per sheet" shape).
    pub fn add_sheet(
        &mut self,
        display_name: impl Into<String>,
    ) -> Result<TreeNodeId, WorkbookSessionError> {
        let display_name = display_name.into();
        let node_id = self.context.add_sheet(&self.workspace_id, &display_name)?;
        // Grid identity is derived from the workspace + sheet node so cell
        // addresses are stable and unique per sheet without a separate id
        // allocator. `add_sheet` guarantees a fresh node id, so this is unique.
        let sheet_grid_id = format!("sheet:{}", node_id.0);
        let seed = GridBackingSeed {
            // Routed through the token authority so a sheet added to an
            // opened workbook is addressable under the same token as its
            // ingested siblings (the fourth token site, dormant on the open
            // path itself).
            workbook_id: self.workbook_token().to_string(),
            sheet_id: sheet_grid_id,
            bounds: self.bounds,
            authored: Vec::new(),
            table_overlays: Vec::new(),
            merged_regions: Vec::new(),
        };
        self.context
            .set_node_grid(&self.workspace_id, node_id, seed)?;
        Ok(node_id)
    }

    /// The workbook's sheets in sheet order (dense `0..sheet_count`), as the
    /// engine enumerates them. Each row carries the sheet's stable node id,
    /// display name, normalized key, position, and grid-backed flag.
    pub fn sheets(&self) -> Result<Vec<SheetEnumerationRow>, WorkbookSessionError> {
        Ok(self.context.sheets(&self.workspace_id)?)
    }

    /// Rename a sheet in place (Phase 1 Part A), preserving its stable node
    /// identity — cross-sheet references keyed on the sheet node heal across
    /// the rename. Thin wrapper over the engine's `rename_sheet` verb.
    pub fn rename_sheet(
        &mut self,
        sheet: TreeNodeId,
        new_name: &str,
    ) -> Result<(), WorkbookSessionError> {
        self.context
            .rename_sheet(&self.workspace_id, sheet, new_name)?;
        Ok(())
    }

    /// Delete a sheet from the workbook (Phase 1 Part A). Thin wrapper over the
    /// engine's `delete_sheet` verb; the returned `DeletedSheetFact` is
    /// workspace history (undo restores the sheet), not projection state, so
    /// this discards it — the next [`WorkbookSession::sheet_projections`] read
    /// is authoritative.
    pub fn delete_sheet(&mut self, sheet: TreeNodeId) -> Result<(), WorkbookSessionError> {
        self.context.delete_sheet(&self.workspace_id, sheet)?;
        Ok(())
    }

    /// Move a sheet to a new 0-based sheet-order position (Phase 1 Part A).
    /// Thin wrapper over the engine's `move_sheet` verb (`new_position` maps to
    /// the engine's `usize` sheet-position; an out-of-range position is the
    /// engine's own typed rejection).
    pub fn move_sheet(
        &mut self,
        sheet: TreeNodeId,
        new_position: u32,
    ) -> Result<(), WorkbookSessionError> {
        self.context
            .move_sheet(&self.workspace_id, sheet, new_position as usize)?;
        Ok(())
    }

    /// The workbook's grid-backed sheets as tab-strip identity rows (Phase 1
    /// Part A): one [`SheetProjection`] per grid-backed sheet, in sheet order,
    /// mapping each enumeration row's stable node id to its
    /// [`sheet_grid_node_id`] address. Non-grid-backed enumeration rows (none
    /// in the current shape, where every sheet is grid-backed at creation) are
    /// filtered out, so the projection is exactly the set a tab strip renders.
    ///
    /// [`SheetProjection`]: dnacalc_skin_ir::SheetProjection
    pub fn sheet_projections(&self) -> Result<Vec<SheetProjection>, WorkbookSessionError> {
        Ok(self
            .sheets()?
            .into_iter()
            .filter(|row| row.grid_backed)
            .map(|row| SheetProjection {
                grid_node_id: sheet_grid_node_id(row.node_id),
                display_name: row.display_name,
                position: row.sheet_position as u32,
            })
            .collect())
    }

    /// Build a strict-Excel cell address in a sheet's own grid namespace —
    /// the workbook half from the token authority, the sheet half the
    /// `sheet:{node}` string both origins share.
    fn address_for(&self, sheet: TreeNodeId, row: u32, col: u32) -> ExcelGridCellAddress {
        ExcelGridCellAddress::new(
            self.workbook_token(),
            format!("sheet:{}", sheet.0),
            row,
            col,
        )
    }

    /// Write a literal value into a single grid cell (H2's write path — the
    /// `set_grid_cell_value` engine verb, **not** the universal `enter_grid_cell`
    /// authored-entry verb, which is H6). Dependents recompute through the
    /// engine's normal seed path; the returned view is the post-edit derived
    /// readout of the edited sheet.
    ///
    /// `row`/`col` are 1-based strict-Excel coordinates.
    pub fn set_grid_cell_value(
        &mut self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
        value: CalcValue,
    ) -> Result<OxCalcTreeGridView, WorkbookSessionError> {
        let address = self.address_for(sheet, row, col);
        self.context
            .set_grid_cell_value(&self.workspace_id, sheet, &address, value)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })
    }

    /// Read back the current published value at a single grid cell, or `None`
    /// if the cell has no published value yet. Reads the derived readout
    /// (`grid_view`) — the post-recalc computed value, matching what a snapshot
    /// would show.
    pub fn grid_cell_value(
        &self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
    ) -> Result<Option<CalcValue>, WorkbookSessionError> {
        let address = self.address_for(sheet, row, col);
        let view = self
            .context
            .grid_view(&self.workspace_id, sheet)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })?;
        Ok(view
            .cells
            .iter()
            .find(|cell| cell.address == address)
            .map(|cell| cell.value.clone()))
    }

    /// Read back a single published grid cell's provenance (H5, §A.3): the
    /// skin-IR mirror of OxCalc's `PublishedValueProvenance` — `None` if the
    /// cell has no published value yet (same absence contract as
    /// [`WorkbookSession::grid_cell_value`]).
    pub fn grid_cell_provenance(
        &self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
    ) -> Result<Option<dnacalc_skin_ir::ValueProvenanceProjection>, WorkbookSessionError> {
        let address = self.address_for(sheet, row, col);
        let view = self
            .context
            .grid_view(&self.workspace_id, sheet)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })?;
        Ok(view
            .cells
            .iter()
            .find(|cell| cell.address == address)
            .map(|cell| crate::calc::value_provenance_projection(cell.provenance)))
    }

    /// The windowed authored-metadata projection for a sheet's interest
    /// window (H3, §A.3): the skin-IR mirror of `grid_authored_view`, filled
    /// for exactly the requested rectangle — never the whole sheet — so the
    /// host publishes `GridCellProjection::authored` only for cells a client
    /// is actually viewing.
    ///
    /// `row`/`col` bounds are 1-based, inclusive, in strict-Excel coordinates
    /// (the same window shape `SetGridInterest` registers).
    pub fn grid_authored_cells(
        &self,
        sheet: TreeNodeId,
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    ) -> Result<Vec<GridAuthoredCellProjection>, WorkbookSessionError> {
        // The GridRect half of the token trap: the engine synthesizes every
        // address in the window from `rect.workbook_id`, so a wrong token here
        // reads back blanks, never an error. Token authority only.
        let window = GridRect::new(
            self.workbook_token(),
            format!("sheet:{}", sheet.0),
            top_row,
            left_col,
            bottom_row,
            right_col,
            self.bounds,
        )
        .map_err(|_| WorkbookSessionError::SheetNotGridBacked { node: sheet })?;
        let readouts = self
            .context
            .grid_authored_view(&self.workspace_id, sheet, Some(window))?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })?;
        Ok(readouts.iter().map(grid_authored_cell_projection).collect())
    }

    /// Author a cell's text through the engine's universal entry verb
    /// (`enter_grid_cell`, H6 §A.2): the three-way
    /// literal/formula/cleared interpretation `WorkspaceIntent::EnterGridCell`
    /// dispatches to. OxFml is the sole text-to-value interpretation
    /// authority; empty `text` is Excel's empty-commit-clears contract and
    /// resolves through the engine's own `Cleared` arm — no skin-side
    /// classification of a leading `=` happens here or anywhere upstream.
    ///
    /// On `Err`, the engine guarantees no mutation — a rejected entry leaves
    /// the authored cell exactly as it was (asserted by H6's acceptance tests
    /// via a re-read through [`WorkbookSession::grid_authored_cells`]).
    pub fn enter_grid_cell(
        &mut self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
        text: &str,
    ) -> Result<GridCellEntryOutcome, WorkbookSessionError> {
        let address = self.address_for(sheet, row, col);
        self.context
            .enter_grid_cell(&self.workspace_id, sheet, &address, text)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })
    }

    /// Clear a grid cell's authored content directly (`clear_grid_cell`, H6
    /// §A.2), the `WorkspaceIntent::ClearGridCell` target — as opposed to
    /// committing empty text through [`WorkbookSession::enter_grid_cell`].
    /// Idempotent and revision-visible per the engine's own contract.
    pub fn clear_grid_cell(
        &mut self,
        sheet: TreeNodeId,
        row: u32,
        col: u32,
    ) -> Result<OxCalcTreeGridView, WorkbookSessionError> {
        let address = self.address_for(sheet, row, col);
        self.context
            .clear_grid_cell(&self.workspace_id, sheet, &address)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })
    }

    /// The ONE recipe that turns an engine grid readout into the sheet's
    /// windowed [`GridProjection`] — used by [`WorkbookSession::snapshot`] for
    /// every sheet (through [`WorkbookSession::grid_projection`]) AND by the
    /// entry-verb dispatch (`lib.rs`) to build the edited sheet's
    /// `GridChanged` delta from the post-edit view the engine hands back
    /// (`GridCellEntryOutcome::{Literal, Formula, Cleared}::view`, dtc-j7n8.18).
    /// One function, so the patch a receipt carries for a sheet and the fresh
    /// snapshot a caller re-reads can never disagree about THAT sheet's grid.
    /// It says nothing about the other sheets: an edit's cross-sheet recalc
    /// (OxCalc `propagate_cross_sheet_edit`, Automatic mode) moves dependent
    /// sheets the engine does not hand back with the outcome — the dispatch
    /// finds those with [`WorkbookSession::peer_grid_projections`] before and
    /// after the edit.
    ///
    /// The authored window is the bounding box of the view's populated cells
    /// (min/max row/col across `view.cells`), or `1..=1` when the sheet is
    /// empty — the same "match the cells actually projected" window
    /// `dnatreecalc-host`'s `authored_cells_for` derives, so every projected
    /// cell finds its authored record. The grid is keyed by its stable
    /// [`sheet_grid_node_id`] and carries `grid_node_key =
    /// NodeKey::from_engine_id(sheet.0)`.
    ///
    /// `view` must be `sheet`'s readout (a `grid_view` / entry-verb view of
    /// that node); the authored lookup reads `sheet` through the session's
    /// own workbook token, so a view of another sheet would pair values with
    /// the wrong authored records rather than fail.
    pub fn grid_projection_from_view(
        &self,
        sheet: TreeNodeId,
        view: &OxCalcTreeGridView,
    ) -> Result<GridProjection, WorkbookSessionError> {
        let (top_row, left_col, bottom_row, right_col) = view
            .cells
            .iter()
            .fold(None, |acc: Option<(u32, u32, u32, u32)>, cell| {
                let (r, c) = (cell.address.row, cell.address.col);
                Some(match acc {
                    None => (r, c, r, c),
                    Some((tr, lc, br, rc)) => (tr.min(r), lc.min(c), br.max(r), rc.max(c)),
                })
            })
            .unwrap_or((1, 1, 1, 1));

        let authored = self
            .grid_authored_cells(sheet, top_row, left_col, bottom_row, right_col)?
            .into_iter()
            .map(|cell| ((cell.row, cell.col), cell))
            .collect::<BTreeMap<(u32, u32), _>>();

        Ok(grid_projection_for(
            view,
            sheet_grid_node_id(sheet),
            NodeKey::from_engine_id(sheet.0),
            &authored,
        ))
    }

    /// One sheet's current windowed [`GridProjection`], read fresh from the
    /// engine (`grid_view`) and built by the single
    /// [`WorkbookSession::grid_projection_from_view`] recipe — the per-sheet
    /// unit [`WorkbookSession::snapshot`] and
    /// [`WorkbookSession::peer_grid_projections`] are both assembled from, so
    /// a snapshot grid and a receipt patch for the same sheet are the same
    /// bytes. A sheet without a grid backing is the internal-invariant
    /// [`WorkbookSessionError::SheetNotGridBacked`].
    ///
    /// [`GridProjection`]: dnacalc_skin_ir::GridProjection
    pub fn grid_projection(
        &self,
        sheet: TreeNodeId,
    ) -> Result<GridProjection, WorkbookSessionError> {
        let view = self
            .context
            .grid_view(&self.workspace_id, sheet)?
            .ok_or(WorkbookSessionError::SheetNotGridBacked { node: sheet })?;
        self.grid_projection_from_view(sheet, &view)
    }

    /// Every grid-backed sheet's projection EXCEPT `edited`'s, in sheet order
    /// (`(sheet node, projection)` pairs) — the peer readout the entry-verb
    /// dispatch (`lib.rs`, dtc-j7n8.18) takes once BEFORE and once AFTER an
    /// edit to find the cross-sheet dependents the engine recalculated in the
    /// same transaction (OxCalc `propagate_cross_sheet_edit`, Automatic
    /// mode): a peer whose projection moved gets its own `GridChanged` on the
    /// receipt; one that did not is left out, so an unrelated sheet never
    /// re-renders. The engine hands back only the edited sheet's view with
    /// the outcome and surfaces no "sheets I recalculated" fact, so this is a
    /// host-side diff over every peer, twice per edit — the same order of
    /// work as one `snapshot()`, acceptable for the W011 slice; an
    /// engine-surfaced recalculated-sheet set is the successor that retires
    /// the pre-edit read (dtc-j7n8.22, filed beside dtc-j7n8.20).
    ///
    /// Each projection comes from [`WorkbookSession::grid_projection`], so a
    /// "moved" peer is decided by `GridProjection` equality — the exact value
    /// a mirror holds — never by an epoch heuristic.
    pub fn peer_grid_projections(
        &self,
        edited: TreeNodeId,
    ) -> Result<Vec<(TreeNodeId, GridProjection)>, WorkbookSessionError> {
        self.sheets()?
            .into_iter()
            .map(|row| row.node_id)
            .filter(|sheet| *sheet != edited)
            .map(|sheet| Ok((sheet, self.grid_projection(sheet)?)))
            .collect()
    }

    /// The full-workspace projection: every grid-backed sheet's windowed
    /// [`GridProjection`], the workbook's complete defined-name catalog, and
    /// its calc-mode/recalc state, assembled into one [`WorkspaceState`] a skin
    /// (or the app's initial mount) can render directly.
    ///
    /// For each enumerated sheet the window is the bounding box of the view's
    /// currently-populated cells (min/max row/col across `grid_view().cells`),
    /// or `1..=1` when the sheet is empty — the same "match the cells actually
    /// projected" window `dnatreecalc-host`'s `authored_cells_for` derives, so
    /// every projected cell finds its authored record. The grid is keyed by its
    /// stable [`sheet_grid_node_id`] and carries `grid_node_key =
    /// NodeKey::from_engine_id(sheet.0)`.
    ///
    /// `profile` is `"strict-excel-grid"` (the workbook model family). Unlike a
    /// windowed poll, this is the whole workbook at once — intended for the
    /// initial mount / demo seed, not the hot per-edit path.
    ///
    /// [`GridProjection`]: dnacalc_skin_ir::GridProjection
    pub fn snapshot(&self) -> Result<WorkspaceState, WorkbookSessionError> {
        let mut grids = BTreeMap::new();
        for row in self.sheets()? {
            let projection = self.grid_projection(row.node_id)?;
            grids.insert(projection.grid_node_id.clone(), projection);
        }

        Ok(WorkspaceState {
            workspace_id: self.workspace_id().as_str().to_string(),
            profile: "strict-excel-grid".to_string(),
            grids,
            defined_names: self.defined_names()?,
            workbook_calc: Some(self.workbook_calc_projection(None)?),
            sheets: self.sheet_projections()?,
            ..Default::default()
        })
    }

    /// Add a root Calculation tree-node participant directly on the
    /// underlying context (H4 test-only seam): the engine's
    /// `DefinedNameCollidesWithTreeNode` rejection (D2 §4.3 rule 4 / V8) only
    /// fires against a root tree node's symbol, and `WorkbookSession`'s public
    /// API deliberately exposes sheets only (§A.1's "one grid-backed node per
    /// sheet" shape) — so H4's own acceptance test for the collision path
    /// needs this narrow escape hatch, mirroring H3's `enter_grid_cell_text`
    /// test-only-helper precedent. Never used outside `#[cfg(test)]`.
    #[cfg(test)]
    pub(crate) fn add_root_calc_node_for_test(&mut self, symbol: &str, formula: &str) {
        self.context
            .add_node(
                &self.workspace_id,
                oxcalc_core::consumer::OxCalcTreeNodeCreate::new(symbol, formula),
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::{
        GridAuthoredKindProjection, GridCellRefProjection, GridEditabilityProjection,
    };

    /// H3 acceptance (3), formula half: a formula cell's authored projection
    /// carries `kind = Formula`, the exact authored source text (never a
    /// computed value), and `editability = Editable` (an ordinary formula
    /// cell is a normal write target).
    #[test]
    fn grid_authored_cells_projects_formula_cell() {
        let mut session = WorkbookSession::create("workbook:h3-formula").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(5.0))
            .unwrap();
        session.enter_grid_cell(sheet, 2, 1, "=A1*3").unwrap();

        let cells = session.grid_authored_cells(sheet, 1, 1, 2, 1).unwrap();
        let formula_cell = cells
            .iter()
            .find(|cell| cell.row == 2 && cell.col == 1)
            .expect("A2 is in the requested window");
        assert_eq!(formula_cell.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(formula_cell.source_text.as_deref(), Some("=A1*3"));
        assert_eq!(
            formula_cell.editability,
            GridEditabilityProjection::Editable
        );
    }

    /// H3 acceptance (3), spill half: entering a spilling array formula
    /// (`SEQUENCE(3,1)`, a 3-row vertical spill) makes its non-anchor
    /// followers project `editability = SpillDisplay { anchor }` — the
    /// anchor being the spilling formula's own cell, not the follower's.
    #[test]
    fn grid_authored_cells_projects_spill_display_member() {
        let mut session = WorkbookSession::create("workbook:h3-spill").unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        session
            .enter_grid_cell(sheet, 1, 1, "=SEQUENCE(3,1)")
            .unwrap();

        let cells = session.grid_authored_cells(sheet, 1, 1, 3, 1).unwrap();
        let anchor_cell = cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 (the spill anchor) is in the requested window");
        assert_eq!(anchor_cell.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(
            anchor_cell.editability,
            GridEditabilityProjection::Editable,
            "the spilling formula's own cell is an ordinary authored formula, not a SpillDisplay follower"
        );

        let follower = cells
            .iter()
            .find(|cell| cell.row == 2 && cell.col == 1)
            .expect("A2 (a spill follower) is in the requested window");
        assert_eq!(
            follower.editability,
            GridEditabilityProjection::SpillDisplay {
                anchor: GridCellRefProjection { row: 1, col: 1 }
            }
        );
    }

    // ------------------------------------------------------------------
    // Full-workspace snapshot.
    // ------------------------------------------------------------------

    use crate::demo::build_demo_workbook;
    use dnacalc_skin_ir::NodeValueProjection;

    /// `snapshot()` over the demo workbook projects both sheets keyed by their
    /// stable `sheet_grid_node_id`, carries the computed value and authored
    /// source text for a dependent formula (`Sheet1!B1 = A1*10 = 10`, authored
    /// `"=A1*10"` — leading `=` included, the exact engine convention), fills
    /// `workbook_calc`, and leaves `defined_names` empty.
    #[test]
    fn snapshot_of_demo_workbook_projects_grids_values_and_calc_state() {
        let session = build_demo_workbook().unwrap();
        let sheets = session.sheets().unwrap();
        let sheet1 = sheets[0].node_id;
        let sheet2 = sheets[1].node_id;

        let state = session.snapshot().unwrap();

        assert_eq!(state.profile, "strict-excel-grid");
        assert_eq!(state.workspace_id, "workbook:demo");

        // Two grids, keyed by the stable sheet grid node ids.
        assert_eq!(state.grids.len(), 2, "one grid per sheet");
        let grid1_id = sheet_grid_node_id(sheet1);
        let grid2_id = sheet_grid_node_id(sheet2);
        assert!(state.grids.contains_key(&grid1_id), "Sheet1 grid present");
        assert!(state.grids.contains_key(&grid2_id), "Sheet2 grid present");

        let grid1 = &state.grids[&grid1_id];
        assert_eq!(grid1.grid_node_key, NodeKey::from_engine_id(sheet1.0));

        // Sheet1!B1 = A1*10 = 10, authored "=A1*10".
        let b1 = grid1
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 is projected");
        assert_eq!(
            b1.value,
            NodeValueProjection::Number {
                raw: "10".to_string(),
                display: "10".to_string(),
            },
            "B1's computed value is 10"
        );
        let authored = b1.authored.as_ref().expect("B1 carries authored metadata");
        assert_eq!(
            authored.source_text.as_deref(),
            Some("=A1*10"),
            "the engine returns the formula source text with its leading `=`"
        );
        assert!(
            b1.provenance.is_some(),
            "a snapshot cell carries its published provenance"
        );

        // Cross-sheet formula on Sheet2: A1 = Sheet1!A1 + Sheet1!A5 = 6.
        let grid2 = &state.grids[&grid2_id];
        let s2a1 = grid2
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("Sheet2!A1 is projected");
        assert_eq!(
            s2a1.value,
            NodeValueProjection::Number {
                raw: "6".to_string(),
                display: "6".to_string(),
            },
            "Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 6"
        );

        // Calc state is present; no defined names authored in the demo.
        let calc = state.workbook_calc.expect("workbook_calc is Some");
        assert_eq!(calc.sheets.len(), 2, "one calc summary row per sheet");
        assert!(
            state.defined_names.entries.is_empty(),
            "the demo authors no defined names"
        );
    }

    /// Live-recalc heartbeat at the host-core layer: editing `Sheet1!A1` and
    /// taking a fresh `snapshot()` shows the dependent `B1 = A1*10` recomputed
    /// (7 -> 70), proving the projection reflects the current engine state, not
    /// a stale capture.
    #[test]
    fn snapshot_reflects_a_live_edit_and_its_dependent_recalc() {
        let mut session = build_demo_workbook().unwrap();
        let sheet1 = session.sheets().unwrap()[0].node_id;

        // Baseline: B1 = A1*10 = 1*10 = 10.
        let before = session.snapshot().unwrap();
        let grid1_id = sheet_grid_node_id(sheet1);
        let b1_before = before.grids[&grid1_id]
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 projected")
            .value
            .clone();
        assert_eq!(
            b1_before,
            NodeValueProjection::Number {
                raw: "10".to_string(),
                display: "10".to_string(),
            }
        );

        // Edit A1 = 7 through the universal entry verb; under Automatic mode the
        // dependent B1 recomputes immediately.
        session.enter_grid_cell(sheet1, 1, 1, "7").unwrap();

        let after = session.snapshot().unwrap();
        let b1_after = after.grids[&grid1_id]
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 projected")
            .value
            .clone();
        assert_eq!(
            b1_after,
            NodeValueProjection::Number {
                raw: "70".to_string(),
                display: "70".to_string(),
            },
            "a fresh snapshot shows B1 = A1*10 = 70 after editing A1 to 7"
        );
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.3): host ownership of the OxDoc source on open.
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::w011_fixture_bytes;
    use oxdoc_model::FidelityDisposition;

    /// Opening the real W011 fixture bytes hands the host the OxDoc source
    /// bundle: `xlsx_source()` is `Some`, its model context lists exactly the
    /// one sheet `Sheet1`, its load ledger dropped nothing, and the document
    /// name round-trips. Since dtc-j7n8.4 the engine side is populated from
    /// these very events, so the engine's own sheet list agrees with OxDoc's.
    #[test]
    fn open_xlsx_bytes_owns_source_and_reports_ledger() {
        let bytes = w011_fixture_bytes();
        let session = WorkbookSession::open_xlsx_bytes(
            XLSX_WORKSPACE_ID,
            &bytes,
            Some("a1_times_three.xlsx".to_string()),
        )
        .expect("OxDoc opens the committed W011 fixture");

        assert_eq!(session.workspace_id().as_str(), XLSX_WORKSPACE_ID);
        assert_eq!(session.document_name(), Some("a1_times_three.xlsx"));

        let source = session
            .xlsx_source()
            .expect("a workbook opened from bytes owns its OxDoc source");

        // Sheet summaries come from OxDoc's byte-free model context.
        let sheet_names: Vec<&str> = source
            .model_context
            .sheets
            .iter()
            .map(|sheet| sheet.name.as_str())
            .collect();
        println!("W011 open: model_context sheets = {sheet_names:?}");
        assert_eq!(sheet_names, ["Sheet1"], "exactly one sheet, named Sheet1");

        // The load ledger dropped nothing (the fixture is five plain parts).
        println!(
            "W011 open: load ledger entries = {}",
            source.load_ledger.entries.len()
        );
        let dropped: Vec<_> = source
            .load_ledger
            .entries
            .iter()
            .filter(|entry| matches!(entry.disposition, FidelityDisposition::Dropped { .. }))
            .collect();
        assert!(dropped.is_empty(), "no Dropped ledger entries: {dropped:?}");

        // The source context carries the document stream the ingest bead
        // reads (the fixture acceptance test pins its exact contents).
        assert!(
            !source.source_context.events().is_empty(),
            "the package session exposes the eager document event stream"
        );

        // The engine side was ingested from the same events (dtc-j7n8.4): its
        // own sheet list is OxDoc's sheet list.
        let engine_sheet_names: Vec<String> = session
            .sheets()
            .unwrap()
            .into_iter()
            .map(|row| row.display_name)
            .collect();
        assert_eq!(
            engine_sheet_names, sheet_names,
            "the engine enumerates exactly the sheets OxDoc's model context lists"
        );
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.4): ingest through the engine's `load_workbook_model`.
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::w011_fixture_parts_dir;
    use dnacalc_skin_ir::ValueProvenanceProjection;
    use oxcalc_core::oxdoc_ingest::LoadRecalcPath;

    /// Open the W011 fixture bytes into a session (the shape every ingest
    /// test below starts from), logging the fixture path for `--nocapture`.
    fn open_w011_fixture() -> WorkbookSession {
        println!("W011 fixture parts: {}", w011_fixture_parts_dir().display());
        WorkbookSession::open_xlsx_bytes(
            XLSX_WORKSPACE_ID,
            &w011_fixture_bytes(),
            Some("a1_times_three.xlsx".to_string()),
        )
        .expect("OxDoc opens and the engine ingests the committed W011 fixture")
    }

    /// The single grid-backed sheet of an opened W011 fixture session.
    fn only_sheet(session: &WorkbookSession) -> TreeNodeId {
        let sheets = session.sheets().unwrap();
        assert_eq!(
            sheets.len(),
            1,
            "the fixture has exactly one sheet: {sheets:?}"
        );
        sheets[0].node_id
    }

    /// dtc-j7n8.4 acceptance (2)/(6): opening the real fixture bytes INGESTS.
    /// The engine's load report says one sheet, one literal (`A1`), one bound
    /// formula (`B1`), the `Automatic` open-recalc path; `Sheet1` enumerates
    /// grid-backed from the engine; `A1` publishes `7` and `B1` publishes
    /// `21` with `Calculated` provenance (the open-recalc replaced the file's
    /// cached value, not trusted it); and the authored readout over the
    /// `(1,1)-(1,2)` window — the GridRect half of the token trap, which
    /// returns blanks on a wrong token — shows `A1` `Literal` and `B1`
    /// `Formula` with source text `=A1*3` (the leading `=` restored by the
    /// engine on ingest, exactly as OxCalc's own contract test proves).
    #[test]
    fn open_fixture_ingests_and_publishes_calculated_21() {
        let session = open_w011_fixture();

        // The load report is engine truth, surfaced — not re-derived.
        let report = session
            .load_report()
            .expect("a workbook opened from xlsx bytes carries its load report");
        println!(
            "W011 ingest: load report sheets={} cells={} formulas_bound={} recalc_path={:?} \
             engine_recalcs_at_load={} bind_degradations={} not_calc_modeled={} ledger_rows={}",
            report.sheets,
            report.cells,
            report.formulas_bound,
            report.recalc_path,
            report.engine_recalcs_at_load,
            report.bind_degradations.len(),
            report.not_calc_modeled,
            report.ledger.len()
        );
        assert_eq!(report.sheets, 1, "one sheet created");
        assert_eq!(
            report.cells, 1,
            "A1 is the one literal; B1 is a formula and is counted in formulas_bound, not here"
        );
        assert_eq!(
            report.formulas_bound, 1,
            "B1 bound through the engine's single key mint"
        );
        assert_eq!(
            report.recalc_path,
            LoadRecalcPath::Automatic,
            "the fixture pins calcMode=auto, so the load took the open-recalc path"
        );
        // `engine_recalcs_at_load` counts drained sheets, not calls; a
        // literal-only sheet contributes 0, so never assert `== sheet count`.
        // The formula-bearing Sheet1 must have drained at least once.
        assert!(
            report.engine_recalcs_at_load > 0,
            "the Automatic open-recalc ran at least one engine pass, got {}",
            report.engine_recalcs_at_load
        );
        assert!(
            report.bind_degradations.is_empty(),
            "=A1*3 binds cleanly: {:?}",
            report.bind_degradations
        );
        assert!(
            session.xlsx_source().is_some(),
            "the OxDoc source stays owned for the save"
        );

        // Sheets now come from the engine.
        let sheets = session.sheets().unwrap();
        assert_eq!(sheets.len(), 1, "exactly one engine sheet: {sheets:?}");
        assert_eq!(sheets[0].display_name, "Sheet1");
        assert!(sheets[0].grid_backed, "the ingested sheet is grid-backed");
        assert_eq!(sheets[0].sheet_position, 0);
        let sheet = sheets[0].node_id;

        // Published values: the ExcelGridCellAddress half of the token trap.
        let a1 = session.grid_cell_value(sheet, 1, 1).unwrap();
        let b1 = session.grid_cell_value(sheet, 1, 2).unwrap();
        let b1_provenance = session.grid_cell_provenance(sheet, 1, 2).unwrap();
        println!("W011 ingest: A1 published value = {a1:?}");
        println!("W011 ingest: B1 published value = {b1:?} provenance = {b1_provenance:?}");
        assert_eq!(
            a1,
            Some(CalcValue::number(7.0)),
            "A1 = 7 (the file's literal)"
        );
        assert_eq!(
            b1,
            Some(CalcValue::number(21.0)),
            "B1 = A1*3 = 21, computed by the open-recalc"
        );
        assert!(
            matches!(
                b1_provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "the Automatic open-recalc made B1 engine-Calculated, not FileCached: {b1_provenance:?}"
        );

        // Authored metadata over the (1,1)-(1,2) window: the GridRect half of
        // the token trap. Blank readouts here are the silent failure this
        // assertion exists to catch.
        let authored = session.grid_authored_cells(sheet, 1, 1, 1, 2).unwrap();
        let a1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        let b1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 is in the requested window");
        println!(
            "W011 ingest: A1 authored kind={:?} literal_text={:?}",
            a1_authored.kind, a1_authored.literal_text
        );
        println!(
            "W011 ingest: B1 authored kind={:?} source_text={:?} editability={:?}",
            b1_authored.kind, b1_authored.source_text, b1_authored.editability
        );
        assert_eq!(
            a1_authored.kind,
            GridAuthoredKindProjection::Literal,
            "A1 is an authored literal, not a blank readout"
        );
        assert_eq!(a1_authored.literal_text.as_deref(), Some("7"));
        assert_eq!(
            b1_authored.kind,
            GridAuthoredKindProjection::Formula,
            "B1 is an authored formula, not a blank readout"
        );
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*3"),
            "the engine restores the leading `=` the file stores without"
        );
        assert_eq!(b1_authored.editability, GridEditabilityProjection::Editable);
    }

    /// dtc-j7n8.4 acceptance (3), the token-trap sentinel: entering `A1 = 10`
    /// through the session's own `enter_grid_cell` on the LOADED fixture takes
    /// the engine's literal branch and recalculates the dependent `B1` to
    /// `30`. If this fails with `SheetNotGridBacked` (the engine's `Ok(None)`
    /// miss) or `B1` stays `21`, the workbook token is wrong — fix the address
    /// authority, never bypass with raw engine calls.
    #[test]
    fn open_fixture_edit_smoke_dependent_recalculates() {
        let mut session = open_w011_fixture();
        let sheet = only_sheet(&session);

        // Baseline straight from the load.
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(21.0)),
            "B1 = 21 at load"
        );

        let outcome = session
            .enter_grid_cell(sheet, 1, 1, "10")
            .expect("A1 on the loaded sheet is addressable under the session's workbook token");
        assert!(
            matches!(outcome, GridCellEntryOutcome::Literal { .. }),
            "'10' takes the engine's literal branch, got {outcome:?}"
        );

        let a1 = session.grid_cell_value(sheet, 1, 1).unwrap();
        let b1 = session.grid_cell_value(sheet, 1, 2).unwrap();
        let b1_provenance = session.grid_cell_provenance(sheet, 1, 2).unwrap();
        println!("W011 edit smoke: A1 = {a1:?}");
        println!("W011 edit smoke: B1 = {b1:?} provenance = {b1_provenance:?}");
        assert_eq!(a1, Some(CalcValue::number(10.0)), "A1 = 10 after the edit");
        assert_eq!(
            b1,
            Some(CalcValue::number(30.0)),
            "B1 = A1*3 = 30: the edit really recalculated the loaded workbook"
        );
        assert!(
            matches!(
                b1_provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "B1's 30 is a fresh engine value: {b1_provenance:?}"
        );

        // An edit of A1 never rewrites B1's authored truth.
        let b1_authored = session
            .grid_authored_cells(sheet, 1, 2, 1, 2)
            .unwrap()
            .into_iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 is in the requested window");
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(b1_authored.source_text.as_deref(), Some("=A1*3"));
    }

    /// dtc-j7n8.4 acceptance (7): the workbook-token authority yields the
    /// token the ENGINE actually keys each origin's grids under — proven
    /// against the engine's own published and authored addresses, not against
    /// a string the host chose. `create()` grids carry the bare workspace id;
    /// `open_xlsx_bytes()` grids carry `book:{workspace}`; and a sheet added
    /// to an opened workbook (the fourth site, the `GridBackingSeed`) lands
    /// under the same token as its ingested siblings.
    #[test]
    fn workbook_token_matches_engine_addresses_for_both_origins() {
        // create() origin.
        let mut created = WorkbookSession::create("workbook:token-create").unwrap();
        let created_sheet = created.add_sheet("Sheet1").unwrap();
        created.enter_grid_cell(created_sheet, 1, 1, "1").unwrap();
        let created_view = created
            .context()
            .grid_view(created.workspace_id(), created_sheet)
            .unwrap()
            .unwrap();
        assert!(
            !created_view.cells.is_empty(),
            "the seeded grid published A1"
        );
        for cell in &created_view.cells {
            assert_eq!(
                cell.address.workbook_id,
                created.workbook_token(),
                "create(): the engine keys the seeded grid under the authority's token"
            );
        }
        assert_eq!(created.workbook_token(), "workbook:token-create");

        // open_xlsx_bytes() origin.
        let mut opened = open_w011_fixture();
        let opened_sheet = only_sheet(&opened);
        let opened_view = opened
            .context()
            .grid_view(opened.workspace_id(), opened_sheet)
            .unwrap()
            .unwrap();
        assert!(!opened_view.cells.is_empty(), "ingest published A1 and B1");
        for cell in &opened_view.cells {
            assert_eq!(
                cell.address.workbook_id,
                opened.workbook_token(),
                "open: the engine keys the ingested grid under the authority's token"
            );
        }
        // The authored side too (no window = the engine's own sparse keys).
        let opened_authored = opened
            .context()
            .grid_authored_view(opened.workspace_id(), opened_sheet, None)
            .unwrap()
            .unwrap();
        assert_eq!(opened_authored.len(), 2, "A1 and B1 are authored");
        for readout in &opened_authored {
            assert_eq!(readout.address.workbook_id, opened.workbook_token());
        }
        assert_eq!(
            opened.workbook_token(),
            format!("book:{XLSX_WORKSPACE_ID}"),
            "ingest-created grids live under the `book:` prefix"
        );
        assert_eq!(opened.workspace_id().as_str(), XLSX_WORKSPACE_ID);
        println!(
            "W011 token authority: create() -> {:?}, open_xlsx_bytes() -> {:?}",
            created.workbook_token(),
            opened.workbook_token()
        );

        // Fourth site: a sheet added to the OPENED workbook is seeded under
        // the ingested token, so it is addressable through the same authority.
        let added = opened.add_sheet("Sheet2").unwrap();
        opened.enter_grid_cell(added, 1, 1, "5").unwrap();
        assert_eq!(
            opened.grid_cell_value(added, 1, 1).unwrap(),
            Some(CalcValue::number(5.0)),
            "a sheet added after open is addressable under the session's token"
        );
        let added_view = opened
            .context()
            .grid_view(opened.workspace_id(), added)
            .unwrap()
            .unwrap();
        for cell in &added_view.cells {
            assert_eq!(cell.address.workbook_id, opened.workbook_token());
        }
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.5): SKIN truth of the loaded workbook — the snapshot
    // a skin mounts from carries authored metadata + provenance.
    // ------------------------------------------------------------------

    use dnacalc_skin_ir::{CalcModeProjection, GridCellProjection};

    /// Locate one projected cell by 1-based `(row, col)`, failing with the
    /// whole projected cell list when it is missing.
    fn projected_cell(cells: &[GridCellProjection], row: u32, col: u32) -> &GridCellProjection {
        cells
            .iter()
            .find(|cell| cell.row == row && cell.col == col)
            .unwrap_or_else(|| panic!("no projected cell at ({row}, {col}) in {cells:#?}"))
    }

    /// dtc-j7n8.5 acceptance: the SKIN truth of the loaded fixture.
    /// `snapshot()` over the xlsx-opened session — the exact
    /// [`WorkspaceState`] a skin mounts from, assembled by the existing
    /// `grid_projection_for` fold (no new IR) — carries `Sheet1`'s grid with
    /// `A1` and `B1` populated, each with its authored metadata AND its
    /// provenance: `A1` `Literal` `"7"` = `7`, `B1` `Formula` `"=A1*3"` =
    /// `21`, both `Calculated` (the fixture's `calcMode="auto"` open-recalc
    /// replaced the file cache, so `FileCached` never shows on this lane —
    /// its first live assertion is the Wave 3 Manual-mode lane,
    /// dtc-j7n8.13), and `authored_epoch > 0`. This projection is what the
    /// skins consume, so it is what makes the `B1` notebook render real
    /// later without any skin change.
    ///
    /// Assertion ORDER is the point. Two silent-pass modes exist because the
    /// mount surface (`DocumentSession::snapshot`, `lib.rs`) is infallible
    /// via `unwrap_or_default`, so a defaulted snapshot "passes" vacuous
    /// assertions: (i) an empty grids map / empty cell list — the cells are
    /// asserted non-empty FIRST; (ii) populated values with EMPTY authored
    /// metadata — exactly what a mismatched `GridRect` workbook token yields,
    /// since `grid_authored_view` returns blanks, never an error, on a token
    /// miss (see [`WorkbookSession::workbook_token`]) — so `authored.is_some()`
    /// with the right kind is asserted for BOTH `A1` and `B1` before any
    /// value, provenance, or epoch assertion. No direct engine readout: the
    /// snapshot is the surface under test.
    #[test]
    fn snapshot_of_loaded_fixture_projects_authored_and_provenance() {
        let session = open_w011_fixture();
        let sheet = only_sheet(&session);

        let state = session
            .snapshot()
            .expect("snapshot() over the loaded fixture is Ok, not an internal-invariant error");

        // (i) Close the empty-snapshot silent pass FIRST: the grids map holds
        // Sheet1's grid, and that grid holds cells.
        assert!(
            !state.grids.is_empty(),
            "an empty grids map is the defaulted-snapshot silent pass this test exists to catch: {state:#?}"
        );
        let grid_id = sheet_grid_node_id(sheet);
        let grid = state.grids.get(&grid_id).unwrap_or_else(|| {
            panic!(
                "Sheet1's grid {grid_id:?} is projected; grids = {:?}",
                state.grids.keys().collect::<Vec<_>>()
            )
        });
        println!(
            "W011 snapshot: grid {:?} grid_id={:?} cells={} projection_epoch={} authored_epoch={}",
            grid.grid_node_id,
            grid.grid_id,
            grid.cells.len(),
            grid.projection_epoch,
            grid.authored_epoch
        );
        assert!(
            !grid.cells.is_empty(),
            "Sheet1's projected cell list is empty: the ingest published nothing into the snapshot"
        );
        for cell in &grid.cells {
            println!(
                "W011 snapshot: cell ({}, {}) kind={:?} literal_text={:?} source_text={:?} \
                 editability={:?} value={:?} value_epoch={} provenance={:?}",
                cell.row,
                cell.col,
                cell.authored.as_ref().map(|authored| authored.kind),
                cell.authored
                    .as_ref()
                    .and_then(|authored| authored.literal_text.as_deref()),
                cell.authored
                    .as_ref()
                    .and_then(|authored| authored.source_text.as_deref()),
                cell.authored.as_ref().map(|authored| &authored.editability),
                cell.value,
                cell.value_epoch,
                cell.provenance
            );
        }
        assert_eq!(
            grid.cells.len(),
            2,
            "exactly A1 and B1 are published (no extra cells the conservative save would reject): {:#?}",
            grid.cells
        );

        // (ii) Close the blank-authored silent pass BEFORE any value
        // assertion: both cells carry authored metadata of the right kind.
        // A `None` here is the GridRect token-mismatch blank, not a missing
        // cell — the cell list above already proved the cells exist.
        let a1 = projected_cell(&grid.cells, 1, 1);
        let b1 = projected_cell(&grid.cells, 1, 2);
        let a1_authored = a1.authored.as_ref().expect(
            "A1 carries authored metadata (None = blank readout from a mismatched GridRect workbook token)",
        );
        let b1_authored = b1.authored.as_ref().expect(
            "B1 carries authored metadata (None = blank readout from a mismatched GridRect workbook token)",
        );
        assert_eq!(
            a1_authored.kind,
            GridAuthoredKindProjection::Literal,
            "A1 is an authored literal, not a blank (Empty) readout"
        );
        assert_eq!(
            b1_authored.kind,
            GridAuthoredKindProjection::Formula,
            "B1 is an authored formula, not a blank (Empty) readout"
        );

        // A1: the file's literal 7 — authored text, computed value, provenance.
        assert_eq!((a1_authored.row, a1_authored.col), (1, 1));
        assert_eq!(a1_authored.literal_text.as_deref(), Some("7"));
        assert_eq!(
            a1_authored.source_text, None,
            "a literal carries no formula source text"
        );
        assert_eq!(a1_authored.editability, GridEditabilityProjection::Editable);
        assert_eq!(
            a1.value,
            NodeValueProjection::Number {
                raw: "7".to_string(),
                display: "7".to_string(),
            },
            "A1 = 7"
        );
        assert!(
            matches!(
                a1.provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "A1's published 7 is engine-Calculated by the open-recalc: {:?}",
            a1.provenance
        );

        // B1: the formula, recalculated on open — authored source text with
        // the leading `=` the engine restores, computed 21, Calculated (NOT
        // FileCached: the Automatic open-recalc replaced the file's cache).
        assert_eq!((b1_authored.row, b1_authored.col), (1, 2));
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*3"),
            "B1's authored source text is the formula, never its computed value"
        );
        assert_eq!(
            b1_authored.literal_text, None,
            "a formula cell carries no literal text (its 21 is a computed value, not authored)"
        );
        assert_eq!(b1_authored.editability, GridEditabilityProjection::Editable);
        assert_eq!(
            b1.value,
            NodeValueProjection::Number {
                raw: "21".to_string(),
                display: "21".to_string(),
            },
            "B1 = A1*3 = 21"
        );
        assert!(
            matches!(
                b1.provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "B1's 21 is engine-Calculated under calcMode=auto, not FileCached: {:?}",
            b1.provenance
        );

        // Epochs: the loaded window carries authored data, so the projection
        // reports a live authored epoch — not the un-authored default 0.
        assert!(
            grid.authored_epoch > 0,
            "authored_epoch must be > 0 for a window carrying authored cells, got {}",
            grid.authored_epoch
        );
        assert!(
            grid.projection_epoch > 0,
            "projection_epoch reflects the open-recalc, got {}",
            grid.projection_epoch
        );

        // The rest of the state is the loaded workbook, not a default.
        assert_eq!(state.workspace_id, XLSX_WORKSPACE_ID);
        assert_eq!(state.profile, "strict-excel-grid");
        assert_eq!(grid.grid_node_key, NodeKey::from_engine_id(sheet.0));
        assert_eq!(
            state.sheets.len(),
            1,
            "one tab-strip row: {:?}",
            state.sheets
        );
        assert_eq!(state.sheets[0].display_name, "Sheet1");
        assert_eq!(state.sheets[0].grid_node_id, grid_id);
        let calc = state.workbook_calc.as_ref().expect("workbook_calc is Some");
        assert_eq!(
            calc.mode,
            CalcModeProjection::Automatic,
            "the fixture pins calcMode=\"auto\""
        );
        assert_eq!(calc.sheets.len(), 1);
        assert!(
            !calc.sheets[0].dirty,
            "the open-recalc drained everything: nothing is stale after load"
        );
        assert!(
            state.defined_names.entries.is_empty(),
            "the fixture defines no names"
        );
    }

    /// An in-memory workbook has no OxDoc source and no document name — the
    /// demo path is untouched by the open lane.
    #[test]
    fn in_memory_workbook_has_no_xlsx_source() {
        let session = build_demo_workbook().unwrap();
        assert!(session.xlsx_source().is_none());
        assert_eq!(session.document_name(), None);
    }

    /// Garbage bytes surface OxDoc's typed [`XlsxError`] as data inside
    /// [`WorkbookSessionError::Xlsx`] — no panic, no string-typed error.
    #[test]
    fn open_xlsx_bytes_rejects_garbage_with_typed_xlsx_error() {
        let error = WorkbookSession::open_xlsx_bytes(XLSX_WORKSPACE_ID, b"not a zip", None)
            .expect_err("garbage bytes are rejected");
        match &error {
            WorkbookSessionError::Xlsx(xlsx) => {
                println!("typed OxDoc rejection: {xlsx} / {xlsx:?}");
            }
            other => panic!("expected WorkbookSessionError::Xlsx, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // W011 (dtc-j7n8.7): SAVE — the campaign's decisive seam. The engine
    // projects the whole model with fresh formula caches (C12), OxDoc
    // round-trips it against the opened package, and the proof is on the
    // REOPENED BYTES' raw OxDoc events — never on an engine readout after
    // a reload, which would recalculate and mask a stale cached value.
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::{
        OXDOC_CELL_ADD_REJECTION, dropped_entries, log_ledger, open_xlsx_raw, raw_cell_payload,
        raw_sheet_cells, w011_saved_fixture_target_path,
    };
    use oxdoc_model::{CellPayload, DocumentEvent, FormulaRecordKind, PackedCellAddr};

    /// `B1`'s wire payload: the Normal formula `A1*3` (leading `=` stripped,
    /// the xlsx `<f>` convention) with `cached` as the file's `<v>`.
    fn b1_formula_cached(cached: f64) -> CellPayload {
        CellPayload::Formula {
            region: None,
            text: Some("A1*3".to_string()),
            cached: Some(Box::new(CellPayload::Number(cached))),
        }
    }

    /// Save the session, log the byte count and every ledger entry, and
    /// assert the ledger dropped nothing (the fixture has no calc chain, the
    /// one part a formula-cache refresh legitimately drops).
    fn save_and_log(stage: &str, session: &WorkbookSession) -> (Vec<u8>, DocumentFidelityLedger) {
        let (bytes, ledger) = session
            .save_xlsx_bytes()
            .unwrap_or_else(|err| panic!("save_xlsx_bytes [{stage}] failed: {err} / {err:?}"));
        println!("W011 save [{stage}]: {} bytes", bytes.len());
        log_ledger(&format!("W011 save [{stage}] ledger"), &ledger);
        assert!(!bytes.is_empty(), "the save produced package bytes");
        let dropped = dropped_entries(&ledger);
        assert!(
            dropped.is_empty(),
            "no Dropped ledger entries [{stage}]: {dropped:?}"
        );
        (bytes, ledger)
    }

    /// Reopen saved bytes RAW through OxDoc (no engine), logging the load
    /// ledger.
    fn reopen_raw(stage: &str, bytes: &[u8]) -> HostOwnedXlsxSource {
        let reopened = open_xlsx_raw(bytes);
        log_ledger(
            &format!("W011 reopen [{stage}] load ledger"),
            &reopened.load_ledger,
        );
        reopened
    }

    /// Sheet1's raw `(A1, B1)` payloads of a reopened package, logged, after
    /// asserting they are the only two cells (no cell the conservative
    /// round-trip policy would have had to add or drop).
    fn a1_b1(stage: &str, reopened: &HostOwnedXlsxSource) -> (CellPayload, CellPayload) {
        let cells = raw_sheet_cells(reopened, "Sheet1");
        println!("W011 reopen [{stage}]: raw Sheet1 cells = {cells:?}");
        assert_eq!(
            cells.len(),
            2,
            "exactly A1 and B1 in the saved file [{stage}]: {cells:?}"
        );
        let a1 = raw_cell_payload(&cells, 1, 1).clone();
        let b1 = raw_cell_payload(&cells, 1, 2).clone();
        println!("W011 reopen [{stage}]: A1 payload = {a1:?}");
        println!("W011 reopen [{stage}]: B1 payload = {b1:?}");
        (a1, b1)
    }

    /// dtc-j7n8.7 acceptance (1) — THE campaign save proof, on real bytes.
    /// Open the fixture -> `enter_grid_cell(A1, "10")` (LIVE `B1` = 30) ->
    /// `save_xlsx_bytes` -> the ledger dropped nothing -> reopen the SAVED
    /// bytes RAW through OxDoc and walk the events: `A1` is `Number(10.0)`
    /// and `B1` is exactly `Formula { region: None, text: Some("A1*3"),
    /// cached: Some(Number(30.0)) }` — formula text preserved AND the cached
    /// `<v>` refreshed to 30, not the file's stale 21. This file-level
    /// assertion is the trap-killer: an engine readout after a reload would
    /// recalculate 30 from `A1 = 10` and mask a stale cache. Then: the
    /// reopened package still materializes `B1`'s `FormulaTopology` record
    /// (a later save from the saved file stays possible); the session's own
    /// FILE truth is untouched by the save (its source still says cached
    /// 21, LIVE still says 30); and the full loop closes — the saved bytes
    /// open into a fresh session with `A1` authored 10, `B1` authored
    /// `=A1*3`, published 30.
    #[test]
    fn save_after_edit_reopens_with_cached_30() {
        let mut session = open_w011_fixture();
        let sheet = only_sheet(&session);
        session
            .enter_grid_cell(sheet, 1, 1, "10")
            .expect("A1 -> 10 on the loaded fixture");
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(30.0)),
            "LIVE truth before the save: B1 = A1*3 = 30 (Automatic mode drained the edit)"
        );

        let (bytes, _ledger) = save_and_log("after edit", &session);

        // FILE truth of the SAVED bytes — raw OxDoc events, no engine.
        let reopened = reopen_raw("after edit", &bytes);
        let (a1, b1) = a1_b1("after edit", &reopened);
        assert_eq!(
            a1,
            CellPayload::Number(10.0),
            "A1 is saved as the edited literal 10"
        );
        assert_eq!(
            b1,
            b1_formula_cached(30.0),
            "THE TRAP: B1 keeps its formula text A1*3 AND its cached <v> is the fresh 30, \
             not the file's stale 21"
        );

        // The saved package still carries B1's formula record under full().
        let records: Vec<_> = reopened
            .source_context
            .events()
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::FormulaTopology(topology) => Some(&topology.records),
                _ => None,
            })
            .flatten()
            .collect();
        println!("W011 reopen [after edit]: formula records = {records:?}");
        assert_eq!(
            records.len(),
            1,
            "B1 is still the only formula record in the saved file: {records:?}"
        );
        assert_eq!(
            records[0].address,
            PackedCellAddr::from_one_based(1, 2).unwrap(),
            "the record is B1's"
        );
        assert_eq!(records[0].kind, FormulaRecordKind::Normal);
        assert_eq!(records[0].text.as_deref(), Some("A1*3"));

        // The session's own FILE truth is untouched by the save: its source is
        // still the opened package (cached 21); LIVE truth is still 30. The
        // saved bytes are the caller's — the three truths stay distinct.
        let source_cells = raw_sheet_cells(
            session
                .xlsx_source()
                .expect("the OxDoc source stays owned across a save"),
            "Sheet1",
        );
        assert_eq!(
            raw_cell_payload(&source_cells, 1, 2),
            &b1_formula_cached(21.0),
            "the opened source keeps the file's cached 21 after a save"
        );
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(30.0)),
            "LIVE truth is untouched by the save"
        );

        // Full loop: the saved bytes open into a fresh session.
        let reloaded = WorkbookSession::open_xlsx_bytes(
            XLSX_WORKSPACE_ID,
            &bytes,
            Some("a1_times_three_saved.xlsx".to_string()),
        )
        .expect("the saved bytes open through OxDoc and ingest into the engine");
        let reloaded_sheet = only_sheet(&reloaded);
        let authored = reloaded
            .grid_authored_cells(reloaded_sheet, 1, 1, 1, 2)
            .unwrap();
        let a1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        let b1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 is in the requested window");
        let a1_value = reloaded.grid_cell_value(reloaded_sheet, 1, 1).unwrap();
        let b1_value = reloaded.grid_cell_value(reloaded_sheet, 1, 2).unwrap();
        println!(
            "W011 full loop: A1 authored kind={:?} literal_text={:?} value={a1_value:?}",
            a1_authored.kind, a1_authored.literal_text
        );
        println!(
            "W011 full loop: B1 authored kind={:?} source_text={:?} value={b1_value:?}",
            b1_authored.kind, b1_authored.source_text
        );
        assert_eq!(a1_authored.kind, GridAuthoredKindProjection::Literal);
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("10"),
            "A1 reloads as the authored literal 10"
        );
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*3"),
            "B1 reloads as the authored formula, leading `=` restored by the engine"
        );
        assert_eq!(a1_value, Some(CalcValue::number(10.0)), "A1 = 10");
        assert_eq!(
            b1_value,
            Some(CalcValue::number(30.0)),
            "B1 = A1*3 = 30 on the reloaded session"
        );
    }

    /// dtc-j7n8.7 acceptance (1), the no-edit lane: saving straight after
    /// the open round-trips cleanly (no Dropped entry) and the reopened
    /// bytes are the file's own truth — `A1` 7, `B1` `A1*3` cached 21. The
    /// cached 21 is PROJECTED, not copied: the Automatic open-recalc
    /// recomputed 21, so the projection's fresh cache equals the stored one.
    /// The reopened stream is the opened source's stream: a no-op save
    /// neither gains nor loses a document event.
    #[test]
    fn save_without_edit_round_trips_cleanly() {
        let session = open_w011_fixture();
        let (bytes, _ledger) = save_and_log("no edit", &session);

        let reopened = reopen_raw("no edit", &bytes);
        let (a1, b1) = a1_b1("no edit", &reopened);
        assert_eq!(a1, CellPayload::Number(7.0), "A1 is still the file's 7");
        assert_eq!(
            b1,
            b1_formula_cached(21.0),
            "B1 keeps its formula text and the recomputed-equals-stored cached 21"
        );
        assert_eq!(
            reopened.source_context.events(),
            session
                .xlsx_source()
                .expect("the OxDoc source stays owned")
                .source_context
                .events(),
            "a no-op save reopens as the same document event stream"
        );
    }

    /// dtc-j7n8.7 acceptance (1), the refusal lane: `C1` is EMPTY in the
    /// fixture, so entering `C1 = 5` is a cell ADD — accepted into the live
    /// model (edit scope is wider than save scope) but outside OxDoc's
    /// conservative round-trip policy. The save is refused with the typed
    /// `WorkbookSessionError::Xlsx(XlsxError::UnsupportedRoundTripFeature(msg))`
    /// whose text is pinned to the EXACT message observed
    /// (`OXDOC_CELL_ADD_REJECTION`): OxDoc's surgical worksheet merge
    /// compares the original and projected cell KEY SETS and refuses
    /// without naming the cell — the bead's pre-registered "does not name
    /// C1" branch, so the assertion was widened to the observed text, never
    /// dropped. No bytes are produced (the `Ok` arm with its bytes is the
    /// failure the match refuses), and the live model survives the refusal
    /// untouched. The full message is logged for the close report.
    #[test]
    fn save_of_out_of_scope_edit_is_typed_rejection() {
        let mut session = open_w011_fixture();
        let sheet = only_sheet(&session);
        session
            .enter_grid_cell(sheet, 1, 3, "5")
            .expect("the live model accepts C1 = 5: edit scope is wider than save scope");
        assert_eq!(
            session.grid_cell_value(sheet, 1, 3).unwrap(),
            Some(CalcValue::number(5.0)),
            "C1 = 5 live"
        );

        let refusal = session.save_xlsx_bytes();
        match &refusal {
            Ok((bytes, ledger)) => panic!(
                "a cell add must be refused by OxDoc's round-trip policy, but the save \
                 produced {} bytes with ledger {ledger:?}",
                bytes.len()
            ),
            Err(WorkbookSessionError::Xlsx(
                xlsx @ XlsxError::UnsupportedRoundTripFeature(message),
            )) => {
                println!("W011 save [C1 add]: typed OxDoc rejection = {message:?}");
                println!("W011 save [C1 add]: Display = {xlsx}");
                assert_eq!(
                    message.as_str(),
                    OXDOC_CELL_ADD_REJECTION,
                    "the rejection is OxDoc's cell add/remove refusal, pinned to the observed \
                     text (it does not name C1: the surgical merge compares cell key sets)"
                );
            }
            Err(other) => panic!(
                "expected WorkbookSessionError::Xlsx(UnsupportedRoundTripFeature(_)), \
                 got {other} / {other:?}"
            ),
        }

        // The refused save mutated nothing: LIVE truth still holds C1 = 5
        // and B1 = 21; FILE truth is still the two-cell opened package.
        assert_eq!(
            session.grid_cell_value(sheet, 1, 3).unwrap(),
            Some(CalcValue::number(5.0)),
            "C1 = 5 survives the refused save"
        );
        assert_eq!(
            session.grid_cell_value(sheet, 1, 2).unwrap(),
            Some(CalcValue::number(21.0)),
            "B1 = 21 survives the refused save"
        );
        assert_eq!(
            raw_sheet_cells(session.xlsx_source().unwrap(), "Sheet1").len(),
            2,
            "the opened source is still the two-cell package"
        );
    }

    /// dtc-j7n8.7 acceptance (1)/(2): an in-memory workbook (the demo) has no
    /// OxDoc source to round-trip against — `save_xlsx_bytes` refuses with
    /// the typed [`WorkbookSessionError::NoBackingSource`], never a panic
    /// and never an empty package.
    #[test]
    fn save_on_session_without_source_is_typed_error() {
        let session = build_demo_workbook().unwrap();
        assert!(session.xlsx_source().is_none(), "the demo has no source");

        match session.save_xlsx_bytes() {
            Err(WorkbookSessionError::NoBackingSource) => {
                println!(
                    "W011 save [no source]: typed refusal = {}",
                    WorkbookSessionError::NoBackingSource
                );
            }
            Ok((bytes, ledger)) => panic!(
                "an in-memory workbook has no source to round-trip against, but the save \
                 produced {} bytes with ledger {ledger:?}",
                bytes.len()
            ),
            Err(other) => panic!("expected NoBackingSource, got {other} / {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // W011 Wave 3a (dtc-j7n8.13): the Manual calc-mode twin. Same bytes as
    // the fixture above except `calcMode="manual"`, so the load runs ZERO
    // engine passes and the workbook renders the file's caches (the first
    // live `ValueProvenanceProjection::FileCached` assertions) until an
    // explicit `recalculate()`; and the save-trap corollary — a save before
    // the recalc writes the LAST CALCULATED cache (Excel's own semantics for
    // a manual workbook saved without F9), a save after it writes 30.
    // ------------------------------------------------------------------

    use crate::xlsx_fixture::{w011_manual_fixture_bytes, w011_manual_fixture_parts_dir};
    use oxdoc_model::CalcMode;

    /// Open the Manual twin into a session, logging its parts path.
    fn open_w011_manual_fixture() -> WorkbookSession {
        println!(
            "W011 manual fixture parts: {}",
            w011_manual_fixture_parts_dir().display()
        );
        WorkbookSession::open_xlsx_bytes(
            XLSX_WORKSPACE_ID,
            &w011_manual_fixture_bytes(),
            Some("a1_times_three_manual.xlsx".to_string()),
        )
        .expect("OxDoc opens and the engine ingests the committed W011 Manual twin")
    }

    /// The calc mode the one `WorkbookHeader` of a reopened package carries.
    fn raw_header_calc_mode(stage: &str, reopened: &HostOwnedXlsxSource) -> CalcMode {
        let modes: Vec<CalcMode> = reopened
            .source_context
            .events()
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::WorkbookHeader(header) => Some(header.calc_mode),
                _ => None,
            })
            .collect();
        println!("W011 reopen [{stage}]: header calc modes = {modes:?}");
        assert_eq!(modes.len(), 1, "exactly one WorkbookHeader [{stage}]");
        modes[0]
    }

    /// Log and return one cell's published value + provenance.
    fn published(
        stage: &str,
        session: &WorkbookSession,
        sheet: TreeNodeId,
        label: &str,
        row: u32,
        col: u32,
    ) -> (Option<CalcValue>, Option<ValueProvenanceProjection>) {
        let value = session.grid_cell_value(sheet, row, col).unwrap();
        let provenance = session.grid_cell_provenance(sheet, row, col).unwrap();
        println!(
            "W011 manual [{stage}]: {label} published = {value:?} provenance = {provenance:?}"
        );
        (value, provenance)
    }

    /// dtc-j7n8.13 acceptance (1): opening the Manual twin takes the engine's
    /// Manual load path — `recalc_path == Manual`, `engine_recalcs_at_load ==
    /// 0` (the perf-counter proof that no engine pass ran), `B1` still bound
    /// through the single key mint — and the workbook renders the FILE's
    /// caches: `B1` publishes `21` with provenance `FileCached` (the first
    /// live assertion of `ValueProvenanceProjection::FileCached`; the auto
    /// lane's open-recalc never shows it). The calc projection says `Manual`
    /// and the sheet DIRTY — the load seeds it so the first F9 drains it —
    /// and `B1`'s authored text is `=A1*3` — bound, not evaluated.
    ///
    /// Two engine facts pinned as observed (dtc-j7n8.24 tracks them; flip
    /// the asserts when it lands): on a formula-bearing sheet the load
    /// publishes `FileCached` values for formula CACHES only — authored
    /// literals are published by the sheet's own recalc (OxCalc load
    /// staging, calc-5kqg.65), which under Manual never runs before F9 — so
    /// `A1` has NO published value (`None`, provenance `None`) even though
    /// its authored `literal_text` `"7"` reads back fine; and the sheet is
    /// dirty straight after the open (F9 owed), not clean.
    #[test]
    fn open_manual_fixture_renders_file_cached_21_with_zero_engine_passes() {
        let session = open_w011_manual_fixture();
        let report = session
            .load_report()
            .expect("a workbook opened from xlsx bytes carries its load report");
        println!(
            "W011 manual ingest: load report sheets={} cells={} formulas_bound={} \
             recalc_path={:?} engine_recalcs_at_load={} bind_degradations={}",
            report.sheets,
            report.cells,
            report.formulas_bound,
            report.recalc_path,
            report.engine_recalcs_at_load,
            report.bind_degradations.len(),
        );
        assert_eq!(
            report.recalc_path,
            LoadRecalcPath::Manual,
            "calcMode=\"manual\" takes the engine's Manual render-from-cache path"
        );
        assert_eq!(
            report.engine_recalcs_at_load, 0,
            "a Manual-mode load runs ZERO engine passes (the perf-counter proof)"
        );
        assert_eq!(report.sheets, 1);
        assert_eq!(report.cells, 1, "A1 is the one literal");
        assert_eq!(
            report.formulas_bound, 1,
            "B1 is bound at load even though nothing is evaluated"
        );
        assert!(
            report.bind_degradations.is_empty(),
            "=A1*3 binds cleanly: {:?}",
            report.bind_degradations
        );

        let sheet = only_sheet(&session);
        let (a1, a1_provenance) = published("load", &session, sheet, "A1", 1, 1);
        let (b1, b1_provenance) = published("load", &session, sheet, "B1", 1, 2);
        assert_eq!(
            b1,
            Some(CalcValue::number(21.0)),
            "B1 renders the file's cached 21 — no engine value exists yet"
        );
        assert_eq!(
            b1_provenance,
            Some(ValueProvenanceProjection::FileCached),
            "B1's 21 is the FILE's cache, never evaluated by this engine"
        );
        assert_eq!(
            (a1, a1_provenance),
            (None, None),
            "OBSERVED (dtc-j7n8.24): the Manual load publishes formula caches only; the \
             literal A1 has no published value until the first F9"
        );

        let calc = session.workbook_calc_projection(None).unwrap();
        assert_eq!(calc.mode, CalcModeProjection::Manual, "the file's mode");
        assert_eq!(calc.sheets.len(), 1);
        assert!(
            calc.sheets[0].dirty,
            "OBSERVED: a Manual load seeds the sheet dirty — the first F9 is owed"
        );

        let authored = session.grid_authored_cells(sheet, 1, 1, 1, 2).unwrap();
        let a1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        let b1_authored = authored
            .iter()
            .find(|cell| cell.row == 1 && cell.col == 2)
            .expect("B1 is in the requested window");
        assert_eq!(a1_authored.kind, GridAuthoredKindProjection::Literal);
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("7"),
            "A1's authored literal is readable even though nothing published it"
        );
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(
            b1_authored.source_text.as_deref(),
            Some("=A1*3"),
            "B1's formula is bound (leading `=` restored), just not evaluated"
        );
    }

    /// dtc-j7n8.13 acceptance (2), the save-trap corollary, documented in
    /// the name: under Manual, a save BEFORE `Recalculate` writes the LAST
    /// CALCULATED cache. Open the Manual twin -> `A1 -> 10` (accepted; the
    /// engine suppresses the recalc, so `B1` still publishes the file's 21,
    /// `A1` still has no publication — see dtc-j7n8.24 — the sheet is dirty) ->
    /// save -> the SAVED bytes reopened RAW through OxDoc carry `A1 =
    /// Number(10)` beside `B1 = Formula { text: "A1*3", cached: Number(21)
    /// }` and keep `calcMode` Manual. That is Excel's own behavior for a
    /// manual workbook saved without F9 — the projection reads the current
    /// publication whatever its provenance (`save_xlsx_bytes`'s contract),
    /// and this test is the file-level proof it does, not a bug. Then
    /// `recalculate()` (F9): a genuine drain, `B1` = 30 `Calculated` -> save
    /// -> the saved bytes carry cached 30 -> and the full loop under Manual:
    /// those bytes open into a fresh session on the Manual path with ZERO
    /// engine passes and `B1` publishes 30 `FileCached` — the cached-30
    /// reopen with no engine pass that could mask a stale cache.
    ///
    /// Provenance fact pinned here, straight from the engine's own contract
    /// (`GridDerivedState::mark_published_stale`, OxCalc `consumer.rs`): the
    /// Manual-mode edit re-tags only `Calculated` publications `Stale`; a
    /// publication that is still `FileCached` (never freshly calculated, so
    /// there is no live tick to carry into a stale marker) STAYS
    /// `FileCached`. So `B1` after the edit is 21 `FileCached`, not `Stale`;
    /// the undrained edit is visible through the sheet-level `dirty` flag
    /// (`has_undrained_edits`), which is what the calc projection reports.
    #[test]
    fn manual_mode_save_before_recalc_writes_last_calculated_cache() {
        let mut session = open_w011_manual_fixture();
        let sheet = only_sheet(&session);

        let outcome = session
            .enter_grid_cell(sheet, 1, 1, "10")
            .expect("A1 -> 10 on the Manual twin is accepted");
        assert!(
            matches!(outcome, GridCellEntryOutcome::Literal { .. }),
            "'10' takes the engine's literal branch, got {outcome:?}"
        );
        let (a1, a1_provenance) = published("after edit", &session, sheet, "A1", 1, 1);
        let (b1, b1_provenance) = published("after edit", &session, sheet, "B1", 1, 2);
        assert_eq!(
            b1,
            Some(CalcValue::number(21.0)),
            "Manual suppresses the recalc: B1 still publishes the file's 21, not 30"
        );
        assert_eq!(
            b1_provenance,
            Some(ValueProvenanceProjection::FileCached),
            "a FileCached publication stays FileCached behind the undrained edit \
             (the engine re-tags only Calculated values Stale)"
        );
        assert_eq!(
            (a1, a1_provenance),
            (None, None),
            "Manual: A1's authored truth is 10 but nothing published it (dtc-j7n8.24) — \
             the projection reads authored literals directly, so the save still writes 10"
        );
        let calc = session.workbook_calc_projection(None).unwrap();
        assert_eq!(calc.mode, CalcModeProjection::Manual);
        assert!(
            calc.sheets[0].dirty,
            "the undrained edit shows as the sheet's dirty flag"
        );
        let a1_authored = session
            .grid_authored_cells(sheet, 1, 1, 1, 1)
            .unwrap()
            .into_iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("10"),
            "authored truth is the edit"
        );

        // Save BEFORE the recalc: A1 = 10 beside B1's LAST CALCULATED 21.
        let (stale_bytes, _ledger) = save_and_log("manual, before recalc", &session);
        let reopened = reopen_raw("manual, before recalc", &stale_bytes);
        let (a1_raw, b1_raw) = a1_b1("manual, before recalc", &reopened);
        assert_eq!(
            a1_raw,
            CellPayload::Number(10.0),
            "A1 is saved as the edited literal 10"
        );
        assert_eq!(
            b1_raw,
            b1_formula_cached(21.0),
            "EXCEL LAST-CALCULATED SEMANTICS: saved before F9, B1 keeps its formula text and \
             its cached <v> is the last calculated 21, not 30 — the file is honestly stale"
        );
        assert_eq!(
            raw_header_calc_mode("manual, before recalc", &reopened),
            CalcMode::Manual,
            "the saved package keeps the workbook's Manual calc mode"
        );

        // F9: a genuine drain replaces the cache with the engine's own 30.
        let outcome = session.recalculate().expect("Recalculate is accepted");
        assert!(outcome.drained_any(), "F9 drains the undrained edit");
        let (a1, a1_provenance) = published("after recalc", &session, sheet, "A1", 1, 1);
        let (b1, b1_provenance) = published("after recalc", &session, sheet, "B1", 1, 2);
        assert_eq!(b1, Some(CalcValue::number(30.0)), "B1 = A1*3 = 30 after F9");
        assert!(
            matches!(
                b1_provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "B1's 30 is engine-Calculated: {b1_provenance:?}"
        );
        assert_eq!(
            a1,
            Some(CalcValue::number(10.0)),
            "A1 publishes 10 after F9"
        );
        assert!(
            matches!(
                a1_provenance,
                Some(ValueProvenanceProjection::Calculated { .. })
            ),
            "A1's 10 is engine-Calculated: {a1_provenance:?}"
        );
        assert!(
            !session.workbook_calc_projection(None).unwrap().sheets[0].dirty,
            "the drain cleared the dirty flag"
        );

        // Save AFTER the recalc: cached 30.
        let (fresh_bytes, _ledger) = save_and_log("manual, after recalc", &session);
        let reopened = reopen_raw("manual, after recalc", &fresh_bytes);
        let (a1_raw, b1_raw) = a1_b1("manual, after recalc", &reopened);
        assert_eq!(a1_raw, CellPayload::Number(10.0));
        assert_eq!(
            b1_raw,
            b1_formula_cached(30.0),
            "after F9 the save writes the fresh cached 30"
        );
        assert_eq!(
            raw_header_calc_mode("manual, after recalc", &reopened),
            CalcMode::Manual
        );

        // Full loop under Manual: the saved bytes reopen with ZERO engine
        // passes and B1 renders the saved 30 as the FILE's cache — nothing
        // ran that could have recomputed it.
        let reloaded = WorkbookSession::open_xlsx_bytes(
            XLSX_WORKSPACE_ID,
            &fresh_bytes,
            Some("a1_times_three_manual_saved.xlsx".to_string()),
        )
        .expect("the saved Manual bytes open through OxDoc and ingest into the engine");
        let report = reloaded.load_report().unwrap();
        assert_eq!(report.recalc_path, LoadRecalcPath::Manual);
        assert_eq!(
            report.engine_recalcs_at_load, 0,
            "the reload ran no engine pass either"
        );
        let reloaded_sheet = only_sheet(&reloaded);
        let (a1, _) = published("reloaded", &reloaded, reloaded_sheet, "A1", 1, 1);
        let (b1, b1_provenance) = published("reloaded", &reloaded, reloaded_sheet, "B1", 1, 2);
        assert_eq!(
            a1, None,
            "A1 is unpublished again on the Manual reload (dtc-j7n8.24)"
        );
        let a1_authored = reloaded
            .grid_authored_cells(reloaded_sheet, 1, 1, 1, 1)
            .unwrap()
            .into_iter()
            .find(|cell| cell.row == 1 && cell.col == 1)
            .expect("A1 is in the requested window");
        assert_eq!(
            a1_authored.literal_text.as_deref(),
            Some("10"),
            "A1 reloads as the authored literal 10"
        );
        assert_eq!(
            b1,
            Some(CalcValue::number(30.0)),
            "CACHED B1 = 30 ON REOPEN, rendered from the file with no engine pass"
        );
        assert_eq!(
            b1_provenance,
            Some(ValueProvenanceProjection::FileCached),
            "the reloaded 30 is the saved file's cache, not a recomputation"
        );
    }

    /// Generator, not a check (dtc-j7n8.7 acceptance (4)): writes the
    /// POST-EDIT saved bytes (`A1` = 10, `B1` cached 30) to
    /// `target/w011/a1_times_three_saved.xlsx` — the build dir, never the
    /// repo — so Wave 2 (dtc-j7n8.11) has an Excel-openable artifact to
    /// compare against. `#[ignore]`d so the normal suite writes nothing; run
    /// it manually:
    ///
    /// `cargo test -p dnacalc-host-core --offline emit_saved_fixture_for_excel_compare -- --ignored --nocapture`
    #[test]
    #[ignore = "generator: writes target/w011/a1_times_three_saved.xlsx (the post-edit save) for the Wave 2 Excel comparison; run with --ignored"]
    fn emit_saved_fixture_for_excel_compare() {
        let mut session = open_w011_fixture();
        let sheet = only_sheet(&session);
        session.enter_grid_cell(sheet, 1, 1, "10").unwrap();
        let (bytes, _ledger) = save_and_log("excel compare", &session);

        let path = w011_saved_fixture_target_path();
        let dir = path.parent().expect("the target path has a parent dir");
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|err| panic!("failed to create {}: {err}", dir.display()));
        std::fs::write(&path, &bytes)
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
        println!("wrote {} ({} bytes)", path.display(), bytes.len());

        // What is on disk reopens with the fresh cache — the acceptance
        // test's file-level check, on the written file.
        let on_disk = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read back {}: {err}", path.display()));
        let reopened = reopen_raw("excel compare, from disk", &on_disk);
        let (a1, b1) = a1_b1("excel compare, from disk", &reopened);
        assert_eq!(a1, CellPayload::Number(10.0));
        assert_eq!(b1, b1_formula_cached(30.0));
    }
}

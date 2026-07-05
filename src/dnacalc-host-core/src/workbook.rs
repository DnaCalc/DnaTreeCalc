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

use oxcalc_core::consumer::{
    GridBackingSeed, OxCalcDocumentContext, OxCalcDocumentError, OxCalcTreeGridView,
    OxCalcTreeWorkspaceCreate, OxCalcTreeWorkspaceId, SheetEnumerationRow,
};
use oxcalc_core::grid::coords::{ExcelGridBounds, ExcelGridCellAddress};
use oxcalc_core::structural::TreeNodeId;
use oxfunc_core::value::CalcValue;

/// The engine root symbol for a workbook workspace. Kept distinct from the
/// tree-model root symbol so a workbook root is never confused for a general
/// tree root in diagnostics.
const WORKBOOK_ROOT_SYMBOL: &str = "__dnacalc_workbook__";

/// One open strict-Excel workbook: a single [`OxCalcDocumentContext`] plus the
/// stable workspace id addressing its one workbook workspace.
///
/// The context — and therefore this session — is **neither `Send` nor `Sync`**
/// (it transitively holds a non-atomic `Rc<RichValue>` inside `CalcValue`); a
/// session is a single-threaded value that stays on its owning thread. See the
/// Send/Sync audit block in [`crate`] for the full finding and the W011 !Send
/// disposition (the worker owns its own context; only serde receipts cross the
/// thread boundary).
#[derive(Debug)]
pub struct WorkbookSession {
    context: OxCalcDocumentContext,
    workspace_id: OxCalcTreeWorkspaceId,
    /// Default grid geometry for freshly-added sheets (strict-Excel bounds).
    bounds: ExcelGridBounds,
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
        let mut context = OxCalcDocumentContext::default();
        let workspace_id = context.create_workspace(
            OxCalcTreeWorkspaceCreate::new(workspace_id)
                .with_root_symbol(WORKBOOK_ROOT_SYMBOL)
                .as_workbook(),
        )?;
        Ok(Self {
            context,
            workspace_id,
            bounds: ExcelGridBounds::strict_excel(),
        })
    }

    /// The workbook's stable workspace id.
    #[must_use]
    pub fn workspace_id(&self) -> &OxCalcTreeWorkspaceId {
        &self.workspace_id
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
            workbook_id: self.workspace_id.as_str().to_string(),
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

    /// Build a strict-Excel cell address in a sheet's own grid namespace.
    fn address_for(&self, sheet: TreeNodeId, row: u32, col: u32) -> ExcelGridCellAddress {
        ExcelGridCellAddress::new(
            self.workspace_id.as_str(),
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
}

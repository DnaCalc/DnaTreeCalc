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

use dnacalc_skin_ir::{GridAuthoredCellProjection, NodeId};
use oxcalc_core::consumer::{
    GridBackingSeed, GridCellEntryOutcome, OxCalcDocumentContext, OxCalcDocumentError,
    OxCalcTreeGridView, OxCalcTreeWorkspaceCreate, OxCalcTreeWorkspaceId, SheetEnumerationRow,
};
use oxcalc_core::grid::coords::{ExcelGridBounds, ExcelGridCellAddress};
use oxcalc_core::grid::geometry::GridRect;
use oxcalc_core::structural::TreeNodeId;
use oxfunc_core::value::CalcValue;

use crate::grid_publication::grid_authored_cell_projection;

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
        let window = GridRect::new(
            self.workspace_id.as_str(),
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
}

//! EDIT — the Sheet stage's pure cell-entry adapter (S3.6).
//!
//! The Sheet stage authors the *active* grid cell through the **universal entry
//! verb** ([`WorkspaceIntent::EnterGridCell`]) and reads the host's three-way
//! outcome back off the [`IntentReceipt`]. This is the same commit seam the
//! Notebook stage crosses (`dnacalc-stage-notebook`'s `edit.rs`) — the two
//! stages are independent TP crates, so the Sheet carries its own honest copy
//! rather than sharing a lower crate. A future shared-extraction (a small
//! `dnacalc-cell-entry` crate both stages depend on) is possible and would delete
//! this duplication; until then the shape is deliberately mirror-identical so the
//! two seams cannot drift.
//!
//! Layering law (SHELL_SPEC §6): the engine classifies `=`-vs-literal; the skin
//! never sniffs a leading `=`. This module carries **no** target constants —
//! [`enter_cell_intent`] takes the address as arguments, so a caller can only ever
//! build an intent for the cell it names explicitly (the click handler passes the
//! *selected* cell's own `grid`/`row`/`col`, the structural guard against a
//! wrong-cell write proven by the crate tests).
//!
//! P-gate: skin-IR only — no `dnacalc-app`, no `ox*`.

use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{IntentError, WorkspaceDeltaChange, WorkspaceIntent};
use dnacalc_skin_ir::workspace::{
    GridAuthoredCellProjection, GridEditabilityProjection, GridEntryDiagnosticProjection,
    GridEntryOutcomeProjection, NodeValueProjection,
};
use dnacalc_skin_ir::{IntentReceipt, WorkspaceState};

/// Build the authored-entry intent for one specific cell — **always** from the
/// caller's own `grid` + `row` + `col`, never a fixed target. This is the one
/// entry verb the Sheet's overlay editor dispatches (SHELL_SPEC §6); building it
/// from the *selected* cell's own address is what makes a wrong-cell write
/// structurally impossible.
#[must_use]
pub fn enter_cell_intent(grid: NodeId, row: u32, col: u32, text: String) -> WorkspaceIntent {
    WorkspaceIntent::EnterGridCell {
        grid,
        row,
        col,
        text,
    }
}

/// The honestly-rendered result of one commit through the entry verb — the
/// host's three-way success plus the typed-rejection case. Nothing here is
/// inferred by the skin; every arm is read from the host receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellOutcome {
    /// The text classified as a literal; carries the rendered value.
    Literal { value: String },
    /// The text classified as a formula; carries the value plus any
    /// not-yet-resolved defined names (a first-class success field).
    Formula {
        value: String,
        unresolved: Vec<String>,
    },
    /// Empty commit / clear (Excel's empty-commit-clears contract).
    Cleared,
    /// The entry verb rejected the text; carries the typed diagnostics the
    /// overlay editor underlines from.
    Rejected(Vec<GridEntryDiagnosticProjection>),
    /// The receipt carried neither a grid-entry change nor a typed rejection
    /// (never expected on this path — surfaced honestly rather than guessed).
    NoChange,
}

impl CellOutcome {
    /// A short, stable label for the outcome — the `data-outcome` a test reads.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            CellOutcome::Literal { .. } => "literal",
            CellOutcome::Formula { .. } => "formula",
            CellOutcome::Cleared => "cleared",
            CellOutcome::Rejected(_) => "rejected",
            CellOutcome::NoChange => "no-change",
        }
    }
}

/// Interpret a dispatcher receipt for an `EnterGridCell` commit into the
/// three-way outcome (plus rejection). Reads host truth only.
#[must_use]
pub fn interpret_receipt(receipt: &IntentReceipt) -> CellOutcome {
    if !receipt.accepted {
        return match &receipt.error {
            Some(IntentError::GridEntryRejected { diagnostics }) => {
                CellOutcome::Rejected(diagnostics.clone())
            }
            Some(other) => CellOutcome::Rejected(vec![GridEntryDiagnosticProjection {
                message: format!("{other:?}"),
                span: None,
            }]),
            None => CellOutcome::Rejected(vec![GridEntryDiagnosticProjection {
                message: "rejected without a diagnostic".to_string(),
                span: None,
            }]),
        };
    }
    for change in &receipt.delta.changes {
        if let WorkspaceDeltaChange::GridCellEntered { outcome, .. } = change {
            return match outcome {
                GridEntryOutcomeProjection::Literal { value } => CellOutcome::Literal {
                    value: node_value_display(value),
                },
                GridEntryOutcomeProjection::Formula {
                    unresolved_names,
                    value,
                } => CellOutcome::Formula {
                    value: node_value_display(value),
                    unresolved: unresolved_names.clone(),
                },
                GridEntryOutcomeProjection::Cleared => CellOutcome::Cleared,
            };
        }
    }
    CellOutcome::NoChange
}

/// The grid address the receipt's `GridCellEntered` change actually targeted,
/// if any — the cell the entry verb wrote. Pure read of host truth; the
/// wrong-cell guard test asserts this equals the selected cell's own address,
/// proving a commit hit the intended cell and nothing else.
#[must_use]
pub fn entered_cell(receipt: &IntentReceipt) -> Option<(NodeId, u32, u32)> {
    receipt.delta.changes.iter().find_map(|change| match change {
        WorkspaceDeltaChange::GridCellEntered {
            grid_node_id,
            row,
            col,
            ..
        } => Some((grid_node_id.clone(), *row, *col)),
        _ => None,
    })
}

/// A compact display string for a projected cell value (the outcome's value
/// text).
#[must_use]
pub fn node_value_display(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Unevaluated => "(unevaluated)".to_string(),
        NodeValueProjection::Pending => "(pending)".to_string(),
        NodeValueProjection::Scalar(text) => text.clone(),
        NodeValueProjection::Number { display, .. } => display.clone(),
        NodeValueProjection::Text(text) => text.clone(),
        NodeValueProjection::Logical { display, .. } => display.clone(),
        NodeValueProjection::Empty => "(empty)".to_string(),
        NodeValueProjection::Missing => "(missing)".to_string(),
        NodeValueProjection::Reference { target } => target.clone(),
        other => format!("{other:?}"),
    }
}

/// The editor seed for a cell: its formula source text, else its literal text,
/// else empty — never a computed value (the seam authors *what the user typed*,
/// not the engine's result).
#[must_use]
pub fn authored_seed_text(authored: &GridAuthoredCellProjection) -> String {
    authored
        .source_text
        .clone()
        .or_else(|| authored.literal_text.clone())
        .unwrap_or_default()
}

/// The current authored seed text for `(grid, row, col)`, read live from the
/// workspace projection — what the overlay editor seeds from when a cell is
/// selected. Empty when the cell has no authored record in the current window
/// (a blank cell: Excel authors into it from empty).
#[must_use]
pub fn current_authored_seed(ws: &WorkspaceState, grid: &NodeId, row: u32, col: u32) -> String {
    ws.grids
        .get(grid)
        .and_then(|grid| {
            grid.cells
                .iter()
                .find(|cell| cell.row == row && cell.col == col)
        })
        .and_then(|cell| cell.authored.as_ref())
        .map(authored_seed_text)
        .unwrap_or_default()
}

/// Whether a cell's authored editability admits an editor, and if not, the
/// honest reason to show instead of a fake editor. `Editable` → `None` (build an
/// editor); every other variant is a typed read-only reason (repeated-region /
/// merged / spill / table-structural follower). Mirrors the Notebook's
/// `editability_note`; the reasons are identical so the two stages read a cell's
/// role the same way.
#[must_use]
pub fn editability_note(editability: &GridEditabilityProjection) -> Option<&'static str> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::RepeatedRegionMember { .. } => {
            Some("read-only — part of a repeated-formula region")
        }
        GridEditabilityProjection::MergedFollower { .. } => {
            Some("read-only — follower of a merged region")
        }
        GridEditabilityProjection::SpillDisplay { .. } => {
            Some("read-only — spilled from an array formula")
        }
        GridEditabilityProjection::TableStructural { .. } => {
            Some("read-only — structural table cell")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_grid;
    use dnacalc_host_core::{DocumentSession, build_demo_workbook};

    /// The (grid id, value display text) of the demo's first-sheet cell at
    /// `(row, col)`, resolved exactly as the overlay resolves it: through
    /// [`active_grid`] and [`node_value_display`], so the intent the test builds
    /// is the intent the UI builds.
    fn cell(ws: &WorkspaceState, row: u32, col: u32) -> (NodeId, String) {
        let grid = active_grid(ws).expect("the demo's first-sheet grid resolves");
        let value = grid
            .cells
            .iter()
            .find(|c| c.row == row && c.col == col)
            .map(|c| node_value_display(&c.value))
            .unwrap_or_default();
        (grid.grid_node_id.clone(), value)
    }

    /// The wrong-cell guard (mirrors the Notebook's
    /// `commit_writes_the_blocks_own_cell_not_a_sibling`). Drive the REAL
    /// host-core demo workbook: commit `=A1+A5` into Sheet1 `A3` (R3C1, value 3),
    /// building the intent from THAT cell's own row/col, and prove:
    ///  1. the receipt's `GridCellEntered` targeted R3C1 — not another cell;
    ///  2. the interpreted outcome is `Formula` value `6`;
    ///  3. after re-projection, R3C1 reads `6` and the SIBLING R2C1 (A2, value 2)
    ///     is completely unchanged.
    #[test]
    fn commit_writes_the_selected_cell_not_a_sibling() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);

        let before = document.snapshot();
        let (grid, r3c1_before) = cell(&before, 3, 1);
        let (_, r2c1_before) = cell(&before, 2, 1);
        assert_eq!(r3c1_before, "3", "Sheet1 A3 seeds as literal 3");
        assert_eq!(r2c1_before, "2", "Sheet1 A2 seeds as literal 2");

        // Commit through the cell's OWN address (the guard: the intent can only
        // carry the coordinates the selected cell handed us).
        let receipt = document.dispatch(enter_cell_intent(grid.clone(), 3, 1, "=A1+A5".to_string()));

        // (1) The write targeted exactly R3C1 on this grid.
        assert_eq!(
            entered_cell(&receipt),
            Some((grid.clone(), 3, 1)),
            "the entry verb wrote R3C1 and nothing else"
        );
        // (2) The interpreted outcome is Formula = 6 (A1 + A5 = 1 + 5).
        match interpret_receipt(&receipt) {
            CellOutcome::Formula { value, unresolved } => {
                assert_eq!(value, "6");
                assert!(unresolved.is_empty());
            }
            other => panic!("expected Formula, got {other:?}"),
        }

        // (3) Re-derive from the re-projected workspace: R3C1 now 6, R2C1 still 2.
        let after = document.snapshot();
        let (_, r3c1_after) = cell(&after, 3, 1);
        let (_, r2c1_after) = cell(&after, 2, 1);
        assert_eq!(r3c1_after, "6", "the target cell now shows the formula result");
        assert_eq!(r2c1_after, "2", "the sibling cell is untouched by the commit");
    }

    /// A literal commit into the cell's own address classifies as `Literal` and
    /// updates only that cell.
    #[test]
    fn commit_literal_flows_through_interpret_receipt() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = document.snapshot();
        let (grid, _) = cell(&before, 2, 1);

        let receipt = document.dispatch(enter_cell_intent(grid.clone(), 2, 1, "99".to_string()));
        assert_eq!(entered_cell(&receipt), Some((grid, 2, 1)));
        assert_eq!(
            interpret_receipt(&receipt),
            CellOutcome::Literal {
                value: "99".to_string()
            }
        );
    }

    /// An empty commit clears the cell (Excel's empty-commit-clears contract).
    #[test]
    fn commit_empty_clears_the_cell() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = document.snapshot();
        let (grid, _) = cell(&before, 4, 1);

        let receipt = document.dispatch(enter_cell_intent(grid.clone(), 4, 1, String::new()));
        assert_eq!(entered_cell(&receipt), Some((grid, 4, 1)));
        assert_eq!(interpret_receipt(&receipt), CellOutcome::Cleared);
    }

    /// An unparseable formula is rejected with typed diagnostics and no mutation
    /// — the cell keeps its value.
    #[test]
    fn commit_bad_formula_is_rejected_with_diagnostics() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = document.snapshot();
        let (grid, r5c1_before) = cell(&before, 5, 1);

        let receipt = document.dispatch(enter_cell_intent(grid, 5, 1, "=1+".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Rejected(diagnostics) => assert!(!diagnostics.is_empty()),
            other => panic!("expected Rejected, got {other:?}"),
        }
        // No mutation on the rejected path.
        let after = document.snapshot();
        let (_, r5c1_after) = cell(&after, 5, 1);
        assert_eq!(r5c1_after, r5c1_before, "a rejected commit does not mutate the cell");
    }

    /// The overlay seed + editability read host truth: a literal cell seeds its
    /// literal text, a formula cell seeds its source text, and an `Editable`
    /// cell admits an editor while the non-`Editable` roles carry an honest
    /// read-only reason.
    #[test]
    fn seed_and_editability_read_host_truth() {
        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();
        let grid = active_grid(&ws).expect("demo grid").grid_node_id.clone();

        // Sheet1 A1 is literal `1`; B1 is the formula `=A1*10` — the seam seeds
        // from the authored source/literal text, never the computed value.
        assert_eq!(current_authored_seed(&ws, &grid, 1, 1), "1");
        assert_eq!(current_authored_seed(&ws, &grid, 1, 2), "=A1*10");
        // A cell with no authored record (blank in the window) seeds empty.
        assert_eq!(current_authored_seed(&ws, &grid, 50, 50), "");

        // Editability: `Editable` admits an editor; every other role is a typed
        // read-only reason, never a fake editor.
        assert_eq!(editability_note(&GridEditabilityProjection::Editable), None);
        assert!(
            editability_note(&GridEditabilityProjection::SpillDisplay {
                anchor: dnacalc_skin_ir::GridCellRefProjection { row: 1, col: 1 },
            })
            .is_some()
        );
    }
}

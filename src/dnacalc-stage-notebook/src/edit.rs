//! EDIT — the Notebook stage's pure cell-entry adapter (S2.6).
//!
//! The Notebook makes each authored Cell block editable through the *universal
//! entry verb* (`WorkspaceIntent::EnterGridCell`) and reads the host's three-way
//! outcome back off the [`IntentReceipt`] (Literal / Formula / Cleared, or a
//! typed rejection). This is the exact seam `dnacalc-app`'s `adapter.rs` crosses
//! for its single fixed-cell degrade editor — **ported here** so the TP crate
//! owns its own copy and stays skin-IR-only (P-gate: no `dnacalc-app`, no `ox*`).
//! The engine classifies `=`-vs-literal; the skin never sniffs a leading `=`
//! (SHELL_SPEC §6 layering law).
//!
//! Unlike `adapter.rs` — which authors one fixed demo cell (`A6`) — this module
//! carries **no** target constants: [`enter_cell_intent`] takes the address as
//! arguments, so a caller can only ever build an intent for a cell it names
//! explicitly. Each Notebook block passes *its own* `grid`/`row`/`col`, which is
//! the structural guard against wrong-cell writes (proven by the crate tests).

use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{IntentError, WorkspaceDeltaChange, WorkspaceIntent};
use dnacalc_skin_ir::workspace::{
    GridEntryDiagnosticProjection, GridEntryOutcomeProjection, NodeValueProjection,
};

/// Build the authored-entry intent for one specific cell — **always** from the
/// caller's own `grid` + `row` + `col`, never a fixed target. This is the one
/// entry verb the Notebook's degrade editors dispatch (SHELL_SPEC §6); building
/// it from the block's own address is what makes a wrong-cell write structurally
/// impossible.
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
    /// degrade editor underlines from.
    Rejected(Vec<GridEntryDiagnosticProjection>),
    /// The receipt carried neither a grid-entry change nor a typed rejection
    /// (never expected on this path — surfaced honestly rather than guessed).
    NoChange,
}

impl CellOutcome {
    /// A short, stable label for the outcome — the `data-outcome` a test reads
    /// and the chip the UI shows.
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
/// wrong-cell guard tests assert this equals the block's own address, proving a
/// commit hit the intended cell and nothing else.
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

/// A compact display string for a projected cell value (the outcome chip's
/// value text).
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

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_host_core::{DocumentSession, build_demo_workbook};

    use crate::model::{NotebookEntry, NotebookEntryKind, derive_entries};

    /// Resolve the Cell block at `(row, col)` on the first (and only) grid the
    /// demo derives into, returning its own `grid` + `authored` metadata + the
    /// currently-projected value text. Mirrors exactly what `render_block` hands
    /// a block's editor, so the intent the test builds is the intent the UI
    /// builds.
    fn cell_block(entries: &[NotebookEntry], row: u32, col: u32) -> (NodeId, u32, u32, String) {
        for entry in entries {
            if let NotebookEntryKind::Cell {
                grid,
                authored,
                value,
            } = &entry.kind
                && authored.row == row
                && authored.col == col
            {
                return (
                    grid.clone(),
                    authored.row,
                    authored.col,
                    node_value_display(value),
                );
            }
        }
        panic!("no Cell block at R{row}C{col} in {} entries", entries.len());
    }

    /// The wrong-cell guard (the critique's key risk). Drive the REAL host-core
    /// demo workbook: commit `=A1+A5` into the block for Sheet1 `A3` (R3C1,
    /// value 3), building the intent from THAT block's own row/col, and prove:
    ///  1. the receipt's `GridCellEntered` targeted R3C1 — not another cell;
    ///  2. the interpreted outcome is `Formula` value `6`;
    ///  3. after re-projection, R3C1's block reads `6` and the SIBLING block
    ///     R2C1 (A2, value 2) is completely unchanged.
    #[test]
    fn commit_writes_the_blocks_own_cell_not_a_sibling() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);

        // Derive the notebook exactly as the stage does, then pick the R3C1
        // block and its R2C1 sibling from the derivation.
        let before = derive_entries(&document.snapshot());
        let (grid, row, col, r3c1_before) = cell_block(&before, 3, 1);
        let (_, _, _, r2c1_before) = cell_block(&before, 2, 1);
        assert_eq!(r3c1_before, "3", "Sheet1 A3 seeds as literal 3");
        assert_eq!(r2c1_before, "2", "Sheet1 A2 seeds as literal 2");

        // Commit through the block's OWN address (this is the guard: the intent
        // can only carry the coordinates the block handed us).
        let receipt = document.dispatch(enter_cell_intent(
            grid.clone(),
            row,
            col,
            "=A1+A5".to_string(),
        ));

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
        let after = derive_entries(&document.snapshot());
        let (_, _, _, r3c1_after) = cell_block(&after, 3, 1);
        let (_, _, _, r2c1_after) = cell_block(&after, 2, 1);
        assert_eq!(r3c1_after, "6", "the target block now shows the formula result");
        assert_eq!(r2c1_after, "2", "the sibling block is untouched by the commit");
    }

    /// A literal commit into the block's own cell classifies as `Literal` and
    /// updates only that cell.
    #[test]
    fn commit_literal_flows_through_interpret_receipt() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = derive_entries(&document.snapshot());
        let (grid, row, col, _) = cell_block(&before, 2, 1);

        let receipt = document.dispatch(enter_cell_intent(grid.clone(), row, col, "99".to_string()));
        assert_eq!(entered_cell(&receipt), Some((grid, 2, 1)));
        assert_eq!(
            interpret_receipt(&receipt),
            CellOutcome::Literal {
                value: "99".to_string()
            }
        );
    }

    /// An empty commit clears the block's own cell (Excel's
    /// empty-commit-clears contract).
    #[test]
    fn commit_empty_clears_the_cell() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = derive_entries(&document.snapshot());
        let (grid, row, col, _) = cell_block(&before, 4, 1);

        let receipt = document.dispatch(enter_cell_intent(grid.clone(), row, col, String::new()));
        assert_eq!(entered_cell(&receipt), Some((grid, 4, 1)));
        assert_eq!(interpret_receipt(&receipt), CellOutcome::Cleared);
    }

    /// An unparseable formula is rejected with typed diagnostics and no
    /// mutation — the block keeps its value.
    #[test]
    fn commit_bad_formula_is_rejected_with_diagnostics() {
        let session = build_demo_workbook().expect("demo workbook");
        let mut document = DocumentSession::Workbook(session);
        let before = derive_entries(&document.snapshot());
        let (grid, row, col, r5c1_before) = cell_block(&before, 5, 1);

        let receipt = document.dispatch(enter_cell_intent(grid, row, col, "=1+".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Rejected(diagnostics) => assert!(!diagnostics.is_empty()),
            other => panic!("expected Rejected, got {other:?}"),
        }
        // No mutation on the rejected path.
        let after = derive_entries(&document.snapshot());
        let (_, _, _, r5c1_after) = cell_block(&after, 5, 1);
        assert_eq!(r5c1_after, r5c1_before, "a rejected commit does not mutate the cell");
    }
}

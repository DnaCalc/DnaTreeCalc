//! The Calc degrade-path host adapter — the seam the bridge does NOT cross.
//!
//! `dnacalc-bridge`'s `FormulaBridgeDegrade` emits semantic [`BridgeEvent`]s
//! only; this module maps a commit into `WorkspaceIntent::EnterGridCell` and
//! reads the host's three-way outcome back off the [`IntentReceipt`]
//! (Literal / Formula / Cleared, or a typed rejection). All engine truth comes
//! from `dnacalc-host-core` through the dispatcher — the skin never classifies
//! `=`-vs-literal itself (SHELL_SPEC §6 layering law).

use dnacalc_skin_ir::IntentReceipt;
use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{IntentError, WorkspaceDeltaChange, WorkspaceIntent};
use dnacalc_skin_ir::workspace::{
    GridEntryDiagnosticProjection, GridEntryOutcomeProjection, NodeValueProjection,
};

/// The demo cell the degrade editor authors. `A6` (row 6, col 1) is empty in
/// the two-sheet demo workbook (`A1..A5`/`B1..B5` are seeded), so authoring it
/// exercises the entry verb without clobbering the demo's live formulas.
pub const TARGET_ROW: u32 = 6;
pub const TARGET_COL: u32 = 1;

/// Build the single authored-entry intent for the target cell (the degrade
/// path's one entry verb, SHELL_SPEC §6).
#[must_use]
pub fn enter_grid_cell_intent(grid: NodeId, text: String) -> WorkspaceIntent {
    WorkspaceIntent::EnterGridCell {
        grid,
        row: TARGET_ROW,
        col: TARGET_COL,
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
    use dnacalc_host_core::{DocumentSession, build_demo_workbook, sheet_grid_node_id};

    /// Drive the REAL host-core demo workbook through the exact intent the
    /// degrade adapter builds, and prove all three success outcomes plus the
    /// typed rejection flow through `interpret_receipt`.
    #[test]
    fn three_way_outcome_flows_from_the_real_engine() {
        let session = build_demo_workbook().expect("demo workbook");
        let sheet1 = session.sheets().unwrap()[0].node_id;
        let grid = sheet_grid_node_id(sheet1);
        let mut document = DocumentSession::Workbook(session);

        // Literal.
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), "42".to_string()));
        assert_eq!(
            interpret_receipt(&receipt),
            CellOutcome::Literal {
                value: "42".to_string()
            }
        );

        // Formula (references seeded cells A1 + A5 = 1 + 5 = 6).
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), "=A1+A5".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Formula { value, unresolved } => {
                assert_eq!(value, "6");
                assert!(unresolved.is_empty());
            }
            other => panic!("expected Formula outcome, got {other:?}"),
        }

        // Cleared (empty commit).
        let receipt = document.dispatch(enter_grid_cell_intent(grid.clone(), String::new()));
        assert_eq!(interpret_receipt(&receipt), CellOutcome::Cleared);

        // Rejected (an unparseable formula) — typed diagnostics, no mutation.
        let receipt = document.dispatch(enter_grid_cell_intent(grid, "=1+".to_string()));
        match interpret_receipt(&receipt) {
            CellOutcome::Rejected(diagnostics) => assert!(!diagnostics.is_empty()),
            other => panic!("expected Rejected outcome, got {other:?}"),
        }
    }
}

//! Error presentation — the A.4 mapping layer
//! (`FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §A.4).
//!
//! Engine errors are data, not strings (the W011 rule, kept). This module is
//! the **sole** place host-core turns `WorkbookSessionError` (which wraps
//! OxCalc's `OxCalcDocumentError`) into the wire-protocol
//! [`IntentError`], for the cell-entry verbs (`EnterGridCell`/`ClearGridCell`,
//! H6). Every arm below is a literal row of the §A.4 table; anything the
//! table does not name falls through to the documented "unknown/unmapped"
//! decision — a generic rejection with the Debug payload behind a disclosure,
//! never a panic and never a silently-dropped error.

use dnacalc_skin_ir::{GridCellRefProjection, GridEntryDiagnosticProjection, IntentError};
use oxcalc_core::consumer::OxCalcDocumentError;
use oxcalc_core::grid::authored::GridCellNotEditable;
use oxcalc_core::grid::error::EntryRejectionDiagnostic;

use crate::grid_publication::grid_cell_ref_projection;
use crate::workbook::WorkbookSessionError;

/// Map a rejected cell-entry write to its typed [`IntentError`] (§A.4).
///
/// Covers exactly the entry-verb-relevant table rows
/// (`AuthoredInputDiagnostics`, `GridFormulaBindRejected`,
/// `GridCellNotEditable`); every other `OxCalcDocumentError` variant, and the
/// host-internal `SheetNotGridBacked` invariant violation, falls through to
/// [`IntentError::GenericEngineRejection`] — the table's documented fallback,
/// never a panic.
#[must_use]
pub fn present_grid_entry_rejection(error: &WorkbookSessionError) -> IntentError {
    match error {
        WorkbookSessionError::OxCalc(OxCalcDocumentError::AuthoredInputDiagnostics {
            diagnostics,
            ..
        })
        | WorkbookSessionError::OxCalc(OxCalcDocumentError::GridFormulaBindRejected {
            diagnostics,
            ..
        }) => IntentError::GridEntryRejected {
            diagnostics: diagnostics.iter().map(present_entry_diagnostic).collect(),
        },
        WorkbookSessionError::OxCalc(OxCalcDocumentError::GridCellNotEditable {
            reason, ..
        }) => IntentError::GridCellNotEditable {
            anchor: present_not_editable_anchor(reason),
        },
        other => IntentError::GenericEngineRejection {
            debug: format!("{other:?}"),
        },
    }
}

/// Mirror one [`EntryRejectionDiagnostic`] into its wire projection, verbatim
/// (`message` + optional span — §A.4: "the UI MUST handle both").
fn present_entry_diagnostic(
    diagnostic: &EntryRejectionDiagnostic,
) -> GridEntryDiagnosticProjection {
    GridEntryDiagnosticProjection {
        message: diagnostic.message.clone(),
        span: diagnostic.span,
    }
}

/// The classifier's anchor cell for a non-editable rejection (§A.4:
/// "`anchor` = the classifier's anchor cell, remedy 'Edit the anchor'").
/// `TableStructural` has no single-cell anchor, so it projects `None`.
fn present_not_editable_anchor(reason: &GridCellNotEditable) -> Option<GridCellRefProjection> {
    match reason {
        GridCellNotEditable::RepeatedRegionMember { anchor }
        | GridCellNotEditable::MergedFollower { anchor }
        | GridCellNotEditable::SpillDisplaced { anchor } => Some(grid_cell_ref_projection(anchor)),
        GridCellNotEditable::TableStructural { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcalc_core::grid::coords::ExcelGridCellAddress;
    use oxcalc_core::structural::TreeNodeId;

    fn node() -> TreeNodeId {
        TreeNodeId(0)
    }

    fn address(row: u32, col: u32) -> ExcelGridCellAddress {
        ExcelGridCellAddress::new("wb", "sheet", row, col)
    }

    /// A.4 row: `AuthoredInputDiagnostics` -> `GridEntryRejected`, diagnostics
    /// mirrored verbatim (message + span, both `Some` and `None` handled).
    #[test]
    fn maps_authored_input_diagnostics_to_grid_entry_rejected() {
        let error = WorkbookSessionError::OxCalc(OxCalcDocumentError::AuthoredInputDiagnostics {
            node_id: node(),
            diagnostics: vec![
                EntryRejectionDiagnostic {
                    message: "unexpected end of formula".to_string(),
                    span: Some((1, 3)),
                },
                EntryRejectionDiagnostic {
                    message: "no span available".to_string(),
                    span: None,
                },
            ],
        });
        match present_grid_entry_rejection(&error) {
            IntentError::GridEntryRejected { diagnostics } => {
                assert_eq!(diagnostics.len(), 2);
                assert_eq!(diagnostics[0].message, "unexpected end of formula");
                assert_eq!(diagnostics[0].span, Some((1, 3)));
                assert_eq!(diagnostics[1].message, "no span available");
                assert_eq!(diagnostics[1].span, None);
            }
            other => panic!("expected GridEntryRejected, got {other:?}"),
        }
    }

    /// A.4 row: `GridFormulaBindRejected` -> `GridEntryRejected` (same
    /// surface, bind-stage wording — §A.4's "same span-optional handling").
    #[test]
    fn maps_grid_formula_bind_rejected_to_grid_entry_rejected() {
        let error = WorkbookSessionError::OxCalc(OxCalcDocumentError::GridFormulaBindRejected {
            node_id: node(),
            diagnostics: vec![EntryRejectionDiagnostic {
                message: "not a recognized function".to_string(),
                span: Some((0, 5)),
            }],
        });
        match present_grid_entry_rejection(&error) {
            IntentError::GridEntryRejected { diagnostics } => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].message, "not a recognized function");
            }
            other => panic!("expected GridEntryRejected, got {other:?}"),
        }
    }

    /// A.4 row: `GridCellNotEditable` (region variants) -> anchor carried
    /// through as the classifier's anchor cell.
    #[test]
    fn maps_grid_cell_not_editable_region_member_to_anchor() {
        let error = WorkbookSessionError::OxCalc(OxCalcDocumentError::GridCellNotEditable {
            node_id: node(),
            address: address(2, 1),
            reason: GridCellNotEditable::SpillDisplaced {
                anchor: address(1, 1),
            },
        });
        match present_grid_entry_rejection(&error) {
            IntentError::GridCellNotEditable { anchor } => {
                assert_eq!(anchor, Some(GridCellRefProjection { row: 1, col: 1 }));
            }
            other => panic!("expected GridCellNotEditable, got {other:?}"),
        }
    }

    /// A.4 row: `GridCellNotEditable` (`TableStructural`) -> no single-cell
    /// anchor exists, so it projects `None` (never a fabricated anchor).
    #[test]
    fn maps_grid_cell_not_editable_table_structural_to_no_anchor() {
        let error = WorkbookSessionError::OxCalc(OxCalcDocumentError::GridCellNotEditable {
            node_id: node(),
            address: address(1, 1),
            reason: GridCellNotEditable::TableStructural {
                table_id: "Table1".to_string(),
            },
        });
        match present_grid_entry_rejection(&error) {
            IntentError::GridCellNotEditable { anchor } => assert_eq!(anchor, None),
            other => panic!("expected GridCellNotEditable, got {other:?}"),
        }
    }

    /// A.4's "unknown/unmapped engine errors" decision: an OxCalcDocumentError
    /// variant this map does not special-case renders as the generic
    /// rejection with the Debug payload — never a panic.
    #[test]
    fn unmapped_variant_falls_through_to_generic_rejection_never_panics() {
        let error = WorkbookSessionError::OxCalc(OxCalcDocumentError::UnknownWorkspace {
            workspace_id: "workspace:missing".to_string(),
        });
        match present_grid_entry_rejection(&error) {
            IntentError::GenericEngineRejection { debug } => {
                assert!(debug.contains("UnknownWorkspace"));
            }
            other => panic!("expected GenericEngineRejection, got {other:?}"),
        }
    }

    /// The host-internal invariant-violation arm also falls through to the
    /// generic rejection rather than panicking — the map is total over
    /// `WorkbookSessionError`, not just its `OxCalc` arm.
    #[test]
    fn sheet_not_grid_backed_invariant_falls_through_to_generic_rejection() {
        let error = WorkbookSessionError::SheetNotGridBacked { node: node() };
        match present_grid_entry_rejection(&error) {
            IntentError::GenericEngineRejection { debug } => {
                assert!(debug.contains("SheetNotGridBacked"));
            }
            other => panic!("expected GenericEngineRejection, got {other:?}"),
        }
    }
}

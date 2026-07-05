//! Grid publication — the host-core fill of the Skin IR's authored-metadata
//! projection (H3, `FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §A.3) from OxCalc's
//! `grid_authored_view`.
//!
//! `GridCellProjection::authored` (skin-IR `workspace.rs`) is a serde-clean
//! mirror of OxCalc's `GridAuthoredCellReadout` (`grid/authored.rs:505`): the
//! host fills it for exactly the interest window a client has already
//! registered — never the whole sheet — matching the existing
//! `grid_projection_for` value-fill shape in `dnatreecalc-host` (which this
//! module deliberately does not touch; H3 scopes the workbook fill path to
//! host-core only).

use dnacalc_skin_ir::{
    GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellRefProjection,
    GridEditabilityProjection,
};
use oxcalc_core::grid::authored::{GridAuthoredCellReadout, GridAuthoredKind, GridCellEditability};
use oxcalc_core::grid::coords::ExcelGridCellAddress;

/// Project one engine authored-cell readout into its skin-IR mirror.
///
/// `literal_text` is a plain display-text rendering of the literal
/// (`CalcValue::to_string`-equivalent via the core-value match below) — H3's
/// NON-goals exclude format-aware rendering (that lives with the full value
/// projection in the skin layer); this is authored-metadata text only, not the
/// computed-value projection `GridCellProjection::value` already carries.
#[must_use]
pub fn grid_authored_cell_projection(
    readout: &GridAuthoredCellReadout,
) -> GridAuthoredCellProjection {
    GridAuthoredCellProjection {
        row: readout.address.row,
        col: readout.address.col,
        kind: grid_authored_kind_projection(&readout.kind),
        literal_text: readout.literal.as_ref().map(literal_display_text),
        source_text: readout.source_text.clone(),
        editability: grid_editability_projection(&readout.editability),
    }
}

fn grid_authored_kind_projection(kind: &GridAuthoredKind) -> GridAuthoredKindProjection {
    match kind {
        GridAuthoredKind::Empty => GridAuthoredKindProjection::Empty,
        GridAuthoredKind::Literal => GridAuthoredKindProjection::Literal,
        GridAuthoredKind::Formula => GridAuthoredKindProjection::Formula,
        GridAuthoredKind::RichStub => GridAuthoredKindProjection::RichStub,
    }
}

fn grid_editability_projection(editability: &GridCellEditability) -> GridEditabilityProjection {
    match editability {
        GridCellEditability::Editable => GridEditabilityProjection::Editable,
        GridCellEditability::RepeatedRegionMember { anchor } => {
            GridEditabilityProjection::RepeatedRegionMember {
                anchor: grid_cell_ref_projection(anchor),
            }
        }
        GridCellEditability::MergedFollower { anchor } => {
            GridEditabilityProjection::MergedFollower {
                anchor: grid_cell_ref_projection(anchor),
            }
        }
        GridCellEditability::SpillDisplay { anchor } => GridEditabilityProjection::SpillDisplay {
            anchor: grid_cell_ref_projection(anchor),
        },
        GridCellEditability::TableStructural { table_id } => {
            GridEditabilityProjection::TableStructural {
                table_id: table_id.clone(),
            }
        }
    }
}

fn grid_cell_ref_projection(address: &ExcelGridCellAddress) -> GridCellRefProjection {
    GridCellRefProjection {
        row: address.row,
        col: address.col,
    }
}

/// A plain display-text rendering of a literal `CalcValue`, for
/// `GridAuthoredCellProjection::literal_text`. Deliberately minimal (no
/// number-format awareness — that is skin/format work, out of H3's scope):
/// numbers render via their canonical `Display`, text unwraps its UTF-16
/// payload, booleans render Excel's `TRUE`/`FALSE`, and anything else falls
/// back to the core value's `Debug` so no literal silently disappears.
fn literal_display_text(value: &oxfunc_core::value::CalcValue) -> String {
    use oxfunc_core::value::CoreValue;
    match value.core() {
        CoreValue::Number(number) => number.to_string(),
        CoreValue::Text(text) => text.to_string_lossy(),
        CoreValue::Logical(logical) => if *logical { "TRUE" } else { "FALSE" }.to_string(),
        CoreValue::Empty => String::new(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    fn address(row: u32, col: u32) -> ExcelGridCellAddress {
        ExcelGridCellAddress::new("wb", "sheet", row, col)
    }

    #[test]
    fn projects_formula_cell_kind_source_text_and_editable() {
        let readout = GridAuthoredCellReadout {
            address: address(1, 1),
            kind: GridAuthoredKind::Formula,
            literal: None,
            source_text: Some("=A1*3".to_string()),
            channel: None,
            editability: GridCellEditability::Editable,
        };
        let projection = grid_authored_cell_projection(&readout);
        assert_eq!(projection.kind, GridAuthoredKindProjection::Formula);
        assert_eq!(projection.source_text.as_deref(), Some("=A1*3"));
        assert_eq!(projection.editability, GridEditabilityProjection::Editable);
    }

    #[test]
    fn projects_spill_display_member_with_anchor() {
        let readout = GridAuthoredCellReadout {
            address: address(2, 1),
            kind: GridAuthoredKind::Empty,
            literal: None,
            source_text: None,
            channel: None,
            editability: GridCellEditability::SpillDisplay {
                anchor: address(1, 1),
            },
        };
        let projection = grid_authored_cell_projection(&readout);
        assert_eq!(
            projection.editability,
            GridEditabilityProjection::SpillDisplay {
                anchor: GridCellRefProjection { row: 1, col: 1 }
            }
        );
    }

    #[test]
    fn projects_literal_cell_display_text() {
        let readout = GridAuthoredCellReadout {
            address: address(1, 1),
            kind: GridAuthoredKind::Literal,
            literal: Some(CalcValue::number(7.0)),
            source_text: None,
            channel: None,
            editability: GridCellEditability::Editable,
        };
        let projection = grid_authored_cell_projection(&readout);
        assert_eq!(projection.kind, GridAuthoredKindProjection::Literal);
        assert_eq!(projection.literal_text.as_deref(), Some("7"));
    }
}

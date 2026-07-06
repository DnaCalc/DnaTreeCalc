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

use std::collections::BTreeMap;

use dnacalc_skin_ir::{
    GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellProjection,
    GridCellRefProjection, GridEditabilityProjection, GridMergedOverlayDescriptor,
    GridOverlayBundle, GridOverlayRect, GridProjection, GridSpillOverlayDescriptor,
    GridTableColumnBand, GridTableOverlayDescriptor, NodeId, NodeKey, NodeValueProjection,
};
use oxcalc_core::consumer::{OxCalcTreeGridOverlayRect, OxCalcTreeGridView};
use oxcalc_core::grid::authored::{GridAuthoredCellReadout, GridAuthoredKind, GridCellEditability};
use oxcalc_core::grid::coords::ExcelGridCellAddress;
use oxfunc_core::value::CalcValue;

use crate::calc::value_provenance_projection;

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

pub(crate) fn grid_cell_ref_projection(address: &ExcelGridCellAddress) -> GridCellRefProjection {
    GridCellRefProjection {
        row: address.row,
        col: address.col,
    }
}

/// Project an engine grid `CalcValue` into its render [`NodeValueProjection`].
///
/// Host-core's structural rendering of the computed value a `GridCellProjection`
/// carries. Deliberately narrow — no number-format awareness (that is skin/format
/// work): numbers render via their canonical `Display`, text unwraps its UTF-16
/// payload, booleans render Excel's `TRUE`/`FALSE`, arrays recurse cell-by-cell,
/// and anything else falls back to a Debug-derived error/text rendering so no
/// value silently disappears. This is the single grid-value projection host-core
/// owns; the entry-receipt path (`lib.rs`) and the snapshot path both route
/// through it so they can never disagree about a value's shape.
#[must_use]
pub fn grid_value_projection(value: &CalcValue) -> NodeValueProjection {
    use oxfunc_core::value::CoreValue;
    match value.core() {
        CoreValue::Number(number) => NodeValueProjection::Number {
            raw: number.to_string(),
            display: number.to_string(),
        },
        CoreValue::Text(text) => NodeValueProjection::Text(text.to_string_lossy()),
        CoreValue::Logical(logical) => NodeValueProjection::Logical {
            value: *logical,
            display: if *logical { "TRUE" } else { "FALSE" }.to_string(),
        },
        CoreValue::Empty => NodeValueProjection::Empty,
        CoreValue::Missing => NodeValueProjection::Missing,
        CoreValue::Reference(reference) => NodeValueProjection::Reference {
            target: reference.target().to_string(),
        },
        CoreValue::Array(array) => {
            let shape = array.shape();
            let cells = (0..shape.rows)
                .map(|row| {
                    (0..shape.cols)
                        .map(|col| {
                            array
                                .get(row, col)
                                .map(grid_value_projection)
                                .unwrap_or(NodeValueProjection::Empty)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            NodeValueProjection::Array {
                rows: shape.rows,
                cols: shape.cols,
                cells,
            }
        }
        // `CalcValue` has no `Display`; host-core renders the error value's
        // Debug form (format-aware `#DIV/0!`-style text is skin work, out of
        // scope) so no error silently disappears from the projection.
        CoreValue::Error(_) => NodeValueProjection::Error(format!("{value:?}")),
    }
}

/// Assemble a windowed [`GridProjection`] from an engine grid view plus the
/// authored-metadata window that matches it, mirroring `dnatreecalc-host`'s
/// `grid_projection_for` (`session.rs`) exactly — but Leptos-free, so host-core
/// can build the same projection a skin renders without linking a UI framework.
///
/// Each engine readout's `CalcValue` becomes a render [`NodeValueProjection`],
/// the sheet bounds size the viewport, per-cell provenance mirrors the engine's
/// `PublishedValueProvenance`, and the differential mismatch set collapses to a
/// clean flag. `authored` is the caller-supplied window keyed by `(row, col)`,
/// filled the same way (`grid_authored_cell_projection` over the same window),
/// so every projected cell finds its authored record. Format is a later phase,
/// so cells carry no effective format yet.
#[must_use]
pub fn grid_projection_for(
    view: &OxCalcTreeGridView,
    grid_node_id: NodeId,
    grid_node_key: NodeKey,
    authored: &BTreeMap<(u32, u32), GridAuthoredCellProjection>,
) -> GridProjection {
    let cells = view
        .cells
        .iter()
        .map(|cell| GridCellProjection {
            row: cell.address.row,
            col: cell.address.col,
            value: grid_value_projection(&cell.value),
            value_epoch: cell.value_epoch,
            authored: authored.get(&(cell.address.row, cell.address.col)).cloned(),
            provenance: Some(value_provenance_projection(cell.provenance)),
        })
        .collect::<Vec<_>>();
    let projection_epoch = cells.iter().map(|cell| cell.value_epoch).max().unwrap_or(0);
    // The engine does not track an authored-only change epoch independent of the
    // computed-value epochs (`OxCalcTreeGridView` carries no such field), so —
    // matching `session.rs`'s `grid_projection_for` — reuse `projection_epoch`
    // as the authored epoch whenever the window actually carries authored data,
    // and leave an un-authored window at the pre-H3 default `0`.
    let authored_epoch = if cells.iter().any(|cell| cell.authored.is_some()) {
        projection_epoch
    } else {
        0
    };
    GridProjection {
        grid_node_key,
        grid_node_id,
        grid_id: view.grid_id.clone(),
        max_rows: view.bounds.max_rows,
        max_cols: view.bounds.max_cols,
        cells,
        projection_epoch,
        overlays: grid_overlay_bundle_for(view),
        overlay_epoch: view.overlay_epoch,
        differential_clean: view.differential_mismatches.is_empty(),
        authored_epoch,
    }
}

fn grid_overlay_rect(rect: &OxCalcTreeGridOverlayRect) -> GridOverlayRect {
    GridOverlayRect {
        top_row: rect.top_row,
        left_col: rect.left_col,
        bottom_row: rect.bottom_row,
        right_col: rect.right_col,
        clipped_top: rect.clipped_top,
        clipped_left: rect.clipped_left,
        clipped_bottom: rect.clipped_bottom,
        clipped_right: rect.clipped_right,
    }
}

/// Map the engine view's window-clipped overlay readouts into skin-IR overlay
/// descriptors, mirroring `dnatreecalc-host`'s `grid_overlay_bundle_for`. Table
/// descriptors carry geometry + identity only; their row values live in the
/// shared table projection (`table_node_key` is left `None` — no grid-table /
/// tree-table linkage is modeled in host-core yet).
#[must_use]
pub fn grid_overlay_bundle_for(view: &OxCalcTreeGridView) -> GridOverlayBundle {
    GridOverlayBundle {
        tables: view
            .overlays
            .tables
            .iter()
            .map(|table| GridTableOverlayDescriptor {
                table_id: table.table_id.clone(),
                table_name: table.table_name.clone(),
                table_node_key: None,
                table_range: grid_overlay_rect(&table.table_range),
                header_rect: table.header_rect.as_ref().map(grid_overlay_rect),
                totals_rect: table.totals_rect.as_ref().map(grid_overlay_rect),
                columns: table
                    .columns
                    .iter()
                    .map(|column| GridTableColumnBand {
                        column_id: column.column_id.clone(),
                        column_name: column.column_name.clone(),
                        ordinal: column.ordinal,
                        data_rect: grid_overlay_rect(&column.data_rect),
                    })
                    .collect(),
            })
            .collect(),
        spills: view
            .overlays
            .spills
            .iter()
            .map(|spill| GridSpillOverlayDescriptor {
                anchor_row: spill.anchor.row,
                anchor_col: spill.anchor.col,
                extent: grid_overlay_rect(&spill.extent),
                blocked: spill.blocked,
            })
            .collect(),
        merged: view
            .overlays
            .merged
            .iter()
            .map(|region| GridMergedOverlayDescriptor {
                rect: grid_overlay_rect(&region.rect),
            })
            .collect(),
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

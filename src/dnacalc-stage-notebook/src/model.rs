//! MODEL — the Notebook stage's pure entry model (S2.4).
//!
//! A pure, framework-free derivation of the notebook's ordered entry list from
//! a [`WorkspaceState`] projection: [`derive_entries`] turns the workspace into
//! an ordered [`Vec<NotebookEntry>`] (defined names, then authored grid cells
//! not covered by a name or table, then table overlays), and
//! [`entry_classification`] badges each entry from its own authored metadata.
//!
//! This module depends **only** on `dnacalc-skin-ir` projection types — no
//! Leptos, no view code, no framework state. Grid-backed entries never mint a
//! synthetic `NodeKey`: a name or an uncovered cell is backed by a grid address
//! (`grid` + `row`/`col`), so [`entry_classification`] maps the classification
//! categories from the entry's own [`GridAuthoredCellProjection`] and its
//! covering [`DefinedNameProjection`] (if any), never through a
//! dependency-graph node lookup.
//!
//! Entry order: names in catalog order, then uncovered cells in grid/row/col
//! order, then tables in grid/declaration order.

use dnacalc_skin_ir::{
    DefinedNameProjection, DefinedNameScopeProjection, DefinedNameTargetProjection,
    GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellProjection,
    GridTableOverlayDescriptor, NodeClassification, NodeId, NodeValueProjection, WorkspaceState,
};

/// One row in the notebook's entry list: a defined name, an uncovered authored
/// grid cell, or a table overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum NotebookEntryKind {
    /// A defined name (`rate = 0.065`); `backing_cell` is the full windowed
    /// cell projection at the name's static target (authored metadata +
    /// computed value), when the grid window covers it (`None` for a dynamic
    /// name, or a static name whose backing cell projection is outside the
    /// current window).
    Name {
        name: DefinedNameProjection,
        backing_cell: Option<GridCellProjection>,
    },
    /// An authored grid cell not covered by any defined name or table (the
    /// "Other cells" escape hatch) — the notebook's total-surface guarantee.
    Cell {
        grid: NodeId,
        authored: GridAuthoredCellProjection,
        value: NodeValueProjection,
    },
    /// A structured-table overlay.
    Table {
        grid: NodeId,
        table: GridTableOverlayDescriptor,
    },
}

/// One derived notebook entry: its kind plus the classification badge
/// ([`entry_classification`]) and a display label.
#[derive(Debug, Clone, PartialEq)]
pub struct NotebookEntry {
    pub display_name: String,
    pub kind: NotebookEntryKind,
    pub classification: NodeClassification,
}

/// Classify one notebook entry's authored metadata into the same categories the
/// tree model uses ([`NodeClassification`]) — **without** minting a synthetic
/// `NodeKey`: this is a pure mapping from the entry's own authored `kind` (plus
/// the covering name's `is_dynamic`, when present) to a classification, never a
/// dependency-graph node lookup.
///
/// Axis mapping (mirrors the tree model's rule at the content-kind level; grid
/// entries carry no dependency-graph "consumed" signal today, so every
/// grid-backed entry classifies as the *unconsumed* arm of its content axis —
/// `FreeValue` for a literal, `Output` for a formula — **except** a name's own
/// target: a name is *definitionally* something other formulas can reference,
/// so a literal-backed name classifies as `Input` and a dynamic (formula-backed)
/// name classifies as `Intermediate`):
///
/// | authored kind | name?          | classification |
/// |---|---|---|
/// | `Empty`       | —              | `Empty`        |
/// | `Literal`     | name (static)  | `Input`        |
/// | `Literal`     | no name        | `FreeValue`    |
/// | `Formula`     | name (dynamic) | `Intermediate` |
/// | `Formula`     | no name        | `Output`       |
/// | `RichStub`    | —              | `Empty` (inert, no content) |
#[must_use]
pub fn entry_classification(
    authored: &GridAuthoredCellProjection,
    name: Option<&DefinedNameProjection>,
) -> NodeClassification {
    match authored.kind {
        GridAuthoredKindProjection::Empty | GridAuthoredKindProjection::RichStub => {
            NodeClassification::Empty
        }
        GridAuthoredKindProjection::Literal => {
            if name.is_some() {
                NodeClassification::Input
            } else {
                NodeClassification::FreeValue
            }
        }
        GridAuthoredKindProjection::Formula => {
            if name.is_some_and(|name| name.is_dynamic) {
                NodeClassification::Intermediate
            } else {
                NodeClassification::Output
            }
        }
    }
}

/// Resolve a defined name's authored backing cell from the covering grid's
/// windowed projection, when both the name is `Static` and the grid window
/// currently covers the target's anchor (top-left) cell. `None` for a dynamic
/// name or when the window does not (yet) cover the backing cell — the caller
/// must not treat `None` as "no content", only as "not resolved from the
/// current window".
fn resolve_backing_cell<'a>(
    workspace: &'a WorkspaceState,
    name: &DefinedNameProjection,
) -> Option<&'a GridCellProjection> {
    let DefinedNameTargetProjection::Static(rect) = &name.target else {
        return None;
    };
    let grid_id = name_scope_grid(workspace, name)?;
    let grid = workspace.grids.get(&grid_id)?;
    grid.cells
        .iter()
        .find(|cell| cell.row == rect.top_row && cell.col == rect.left_col)
}

/// The grid a name's static target resolves against: the sheet the scope names
/// when `Sheet`-scoped, or the sole grid in the workspace for a `Workbook`-scoped
/// name (the notebook's single-sheet assumption; a multi-sheet workbook-scoped
/// name simply yields no resolved backing cell rather than guessing).
fn name_scope_grid(workspace: &WorkspaceState, name: &DefinedNameProjection) -> Option<NodeId> {
    match &name.scope {
        DefinedNameScopeProjection::Sheet(id) => Some(id.clone()),
        DefinedNameScopeProjection::Workbook => {
            if workspace.grids.len() == 1 {
                workspace.grids.keys().next().cloned()
            } else {
                None
            }
        }
    }
}

/// True when a static name's target rect covers the given cell address (used to
/// exclude name-covered cells from the "Other cells" uncovered-cell section).
fn name_covers_cell(name: &DefinedNameProjection, grid: &NodeId, row: u32, col: u32) -> bool {
    let DefinedNameTargetProjection::Static(rect) = &name.target else {
        return false;
    };
    let Some(scope_grid) = name_scope_matches(name, grid) else {
        return false;
    };
    scope_grid
        && row >= rect.top_row
        && row <= rect.bottom_row
        && col >= rect.left_col
        && col <= rect.right_col
}

/// Whether `name`'s scope could possibly cover `grid` — `Sheet` scope must match
/// the grid exactly; `Workbook` scope covers every grid (single-sheet
/// assumption, as [`name_scope_grid`]).
fn name_scope_matches(name: &DefinedNameProjection, grid: &NodeId) -> Option<bool> {
    match &name.scope {
        DefinedNameScopeProjection::Sheet(id) => Some(id == grid),
        DefinedNameScopeProjection::Workbook => Some(true),
    }
}

/// True when `(row, col)` falls inside a table overlay's `table_range` (table
/// cells render inside the table entry, never doubled into "Other cells").
fn cell_in_any_table(table: &GridTableOverlayDescriptor, row: u32, col: u32) -> bool {
    let rect = &table.table_range;
    row >= rect.top_row && row <= rect.bottom_row && col >= rect.left_col && col <= rect.right_col
}

/// Derive the notebook's ordered entry list from the published workspace
/// projection: every defined name (catalog order), then every authored grid
/// cell not covered by a name or a table ("Other cells", grid/row/col order),
/// then every table overlay (grid/declaration order) — in that grouped order.
#[must_use]
pub fn derive_entries(workspace: &WorkspaceState) -> Vec<NotebookEntry> {
    let mut entries = Vec::new();

    // 1. Names, in catalog order.
    for name in &workspace.defined_names.entries {
        let backing_cell = resolve_backing_cell(workspace, name);
        let backing_authored = backing_cell.and_then(|cell| cell.authored.as_ref());
        let classification = backing_authored.map_or(
            // A name with no resolved authored backing cell yet (dynamic, or
            // window does not cover it): classify from the name's own
            // dynamic-ness alone, matching the Formula/no-authored-cell arm's
            // intent.
            if name.is_dynamic {
                NodeClassification::Intermediate
            } else {
                NodeClassification::Input
            },
            |authored| entry_classification(authored, Some(name)),
        );
        entries.push(NotebookEntry {
            display_name: name.name.clone(),
            kind: NotebookEntryKind::Name {
                name: name.clone(),
                backing_cell: backing_cell.cloned(),
            },
            classification,
        });
    }

    // 2. Uncovered cells, grid order then sheet/row/col order.
    for (grid_id, grid) in &workspace.grids {
        for cell in &grid.cells {
            let Some(authored) = &cell.authored else {
                continue;
            };
            if matches!(authored.kind, GridAuthoredKindProjection::Empty) {
                continue;
            }
            let covered_by_name = workspace
                .defined_names
                .entries
                .iter()
                .any(|name| name_covers_cell(name, grid_id, cell.row, cell.col));
            if covered_by_name {
                continue;
            }
            let covered_by_table = grid
                .overlays
                .tables
                .iter()
                .any(|table| cell_in_any_table(table, cell.row, cell.col));
            if covered_by_table {
                continue;
            }
            entries.push(NotebookEntry {
                display_name: cell_display_name(grid_id, cell.row, cell.col),
                classification: entry_classification(authored, None),
                kind: NotebookEntryKind::Cell {
                    grid: grid_id.clone(),
                    authored: authored.clone(),
                    value: cell.value.clone(),
                },
            });
        }
    }

    // 3. Tables, grid order then declaration order.
    for (grid_id, grid) in &workspace.grids {
        for table in &grid.overlays.tables {
            entries.push(NotebookEntry {
                display_name: table.table_name.clone(),
                classification: NodeClassification::Intermediate,
                kind: NotebookEntryKind::Table {
                    grid: grid_id.clone(),
                    table: table.clone(),
                },
            });
        }
    }

    entries
}

/// A stable, human-readable label for an uncovered cell entry: the grid's
/// display path plus its numeric R{row}C{col} address (the notebook needs a
/// readable, unambiguous label, not a second A1 address-format authority).
fn cell_display_name(grid: &NodeId, row: u32, col: u32) -> String {
    format!("{grid}!R{row}C{col}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::{
        GridEditabilityProjection, GridOverlayBundle, GridOverlayRect, GridProjection,
        GridRectProjection, NodeKey,
    };

    fn name(
        name_text: &str,
        target: DefinedNameTargetProjection,
        is_dynamic: bool,
    ) -> DefinedNameProjection {
        DefinedNameProjection {
            scope: DefinedNameScopeProjection::Sheet(NodeId::new("Sheet1")),
            name: name_text.to_string(),
            target,
            is_dynamic,
        }
    }

    fn static_rect(row: u32, col: u32) -> GridRectProjection {
        GridRectProjection {
            top_row: row,
            left_col: col,
            bottom_row: row,
            right_col: col,
        }
    }

    fn authored_cell(
        row: u32,
        col: u32,
        kind: GridAuthoredKindProjection,
        literal_text: Option<&str>,
        source_text: Option<&str>,
    ) -> GridAuthoredCellProjection {
        GridAuthoredCellProjection {
            row,
            col,
            kind,
            literal_text: literal_text.map(str::to_string),
            source_text: source_text.map(str::to_string),
            editability: GridEditabilityProjection::Editable,
        }
    }

    fn value_number(display: &str) -> NodeValueProjection {
        NodeValueProjection::Number {
            raw: display.to_string(),
            display: display.to_string(),
        }
    }

    fn cell_projection(
        row: u32,
        col: u32,
        value: NodeValueProjection,
        authored: Option<GridAuthoredCellProjection>,
    ) -> GridCellProjection {
        GridCellProjection {
            row,
            col,
            value,
            value_epoch: 1,
            authored,
            provenance: None,
        }
    }

    fn table_overlay(
        name: &str,
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    ) -> GridTableOverlayDescriptor {
        GridTableOverlayDescriptor {
            table_id: format!("tbl:{name}"),
            table_name: name.to_string(),
            table_node_key: None,
            table_range: GridOverlayRect {
                top_row,
                left_col,
                bottom_row,
                right_col,
                clipped_top: false,
                clipped_left: false,
                clipped_bottom: false,
                clipped_right: false,
            },
            header_rect: None,
            totals_rect: None,
            columns: Vec::new(),
        }
    }

    fn grid_projection(
        cells: Vec<GridCellProjection>,
        tables: Vec<GridTableOverlayDescriptor>,
    ) -> GridProjection {
        GridProjection {
            grid_node_key: NodeKey::new("k:Sheet1"),
            grid_node_id: NodeId::new("Sheet1"),
            grid_id: "grid:Sheet1".to_string(),
            max_rows: 1000,
            max_cols: 100,
            cells,
            projection_epoch: 1,
            overlays: GridOverlayBundle {
                tables,
                spills: Vec::new(),
                merged: Vec::new(),
            },
            overlay_epoch: 1,
            differential_clean: true,
            authored_epoch: 1,
        }
    }

    /// Derive order: 2 names + 1 uncovered cell + 1 table -> 4 entries, in
    /// names-then-cells-then-tables order.
    #[test]
    fn derive_entries_returns_four_entries_in_default_order() {
        let mut ws = WorkspaceState::default();

        // Two names: `rate` (literal, static, row 1 col 1) and `monthly`
        // (dynamic formula, no backing cell in this fixture).
        ws.defined_names.entries.push(name(
            "rate",
            DefinedNameTargetProjection::Static(static_rect(1, 1)),
            false,
        ));
        ws.defined_names.entries.push(name(
            "monthly",
            DefinedNameTargetProjection::Dynamic {
                source_text: "=PMT(rate/12,360,-1)".to_string(),
            },
            true,
        ));

        // Grid: row 1 col 1 backs `rate` (covered -> not an "Other cells"
        // entry); row 2 col 1 is an uncovered authored formula cell; rows
        // 3..4 col 1..2 sit under a table overlay (also excluded).
        let rate_cell = authored_cell(
            1,
            1,
            GridAuthoredKindProjection::Literal,
            Some("0.065"),
            None,
        );
        let uncovered_cell = authored_cell(
            2,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=1+1"),
        );
        let table_cell = authored_cell(
            3,
            1,
            GridAuthoredKindProjection::Literal,
            Some("base"),
            None,
        );
        let cells = vec![
            cell_projection(1, 1, value_number("0.065"), Some(rate_cell)),
            cell_projection(2, 1, value_number("2"), Some(uncovered_cell)),
            cell_projection(
                3,
                1,
                NodeValueProjection::Text("base".to_string()),
                Some(table_cell),
            ),
        ];
        let tables = vec![table_overlay("Scenarios", 3, 1, 4, 2)];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, tables));

        let entries = derive_entries(&ws);
        assert_eq!(entries.len(), 4, "2 names + 1 uncovered cell + 1 table");

        // Default order: names first (catalog order), then uncovered cells,
        // then tables.
        assert_eq!(entries[0].display_name, "rate");
        assert!(matches!(entries[0].kind, NotebookEntryKind::Name { .. }));
        assert_eq!(entries[1].display_name, "monthly");
        assert!(matches!(entries[1].kind, NotebookEntryKind::Name { .. }));
        assert!(matches!(entries[2].kind, NotebookEntryKind::Cell { .. }));
        assert_eq!(entries[2].display_name, "Sheet1!R2C1");
        assert!(matches!(entries[3].kind, NotebookEntryKind::Table { .. }));
        assert_eq!(entries[3].display_name, "Scenarios");
    }

    /// The classification badge is `entry_classification`'s output from the
    /// entry's own authored metadata, asserted per authored kind.
    #[test]
    fn entry_classification_maps_authored_metadata_per_kind() {
        // Empty / RichStub -> Empty, regardless of a covering name.
        let empty = authored_cell(1, 1, GridAuthoredKindProjection::Empty, None, None);
        assert_eq!(entry_classification(&empty, None), NodeClassification::Empty);
        let rich_stub = authored_cell(1, 1, GridAuthoredKindProjection::RichStub, None, None);
        assert_eq!(
            entry_classification(&rich_stub, None),
            NodeClassification::Empty
        );

        // Literal, no name -> FreeValue (an unconsumed constant).
        let literal = authored_cell(1, 1, GridAuthoredKindProjection::Literal, Some("1"), None);
        assert_eq!(
            entry_classification(&literal, None),
            NodeClassification::FreeValue
        );

        // Literal, backed by a (static) name -> Input.
        let static_name = name(
            "rate",
            DefinedNameTargetProjection::Static(static_rect(1, 1)),
            false,
        );
        assert_eq!(
            entry_classification(&literal, Some(&static_name)),
            NodeClassification::Input
        );

        // Formula, no name -> Output (an unconsumed terminal formula).
        let formula = authored_cell(
            2,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=1+1"),
        );
        assert_eq!(
            entry_classification(&formula, None),
            NodeClassification::Output
        );

        // Formula, backed by a dynamic name -> Intermediate.
        let dynamic_name = name(
            "monthly",
            DefinedNameTargetProjection::Dynamic {
                source_text: "=1+1".to_string(),
            },
            true,
        );
        assert_eq!(
            entry_classification(&formula, Some(&dynamic_name)),
            NodeClassification::Intermediate
        );

        // Formula, backed by a NON-dynamic name -> still Output: only
        // `is_dynamic` flips the axis, per the mapping table.
        let static_name_over_formula = name(
            "static_over_formula",
            DefinedNameTargetProjection::Static(static_rect(2, 1)),
            false,
        );
        assert_eq!(
            entry_classification(&formula, Some(&static_name_over_formula)),
            NodeClassification::Output
        );
    }

    /// A cell covered by a workbook-scoped name is excluded from "Other cells":
    /// only the name entry remains.
    #[test]
    fn derive_entries_excludes_cells_covered_by_a_workbook_scoped_name() {
        let mut ws = WorkspaceState::default();
        ws.defined_names.entries.push(DefinedNameProjection {
            scope: DefinedNameScopeProjection::Workbook,
            name: "total".to_string(),
            target: DefinedNameTargetProjection::Static(static_rect(5, 5)),
            is_dynamic: false,
        });
        let covered = authored_cell(5, 5, GridAuthoredKindProjection::Literal, Some("1"), None);
        let cells = vec![cell_projection(5, 5, value_number("1"), Some(covered))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        // Only the name entry — the covered cell must not double as an
        // "Other cells" entry.
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, NotebookEntryKind::Name { .. }));
    }

    /// Empty authored cells are skipped entirely (never an "Other cells" entry).
    #[test]
    fn derive_entries_skips_empty_authored_cells() {
        let mut ws = WorkspaceState::default();
        let empty = authored_cell(1, 1, GridAuthoredKindProjection::Empty, None, None);
        let cells = vec![cell_projection(1, 1, NodeValueProjection::Empty, Some(empty))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        assert_eq!(derive_entries(&ws).len(), 0);
    }
}

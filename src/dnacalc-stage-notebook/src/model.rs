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

/// True when static `name` covers the cell at (`grid`, `row`, `col`) — i.e. that
/// cell is the name's backing cell, so it must not also render as a bare "Other
/// cells" entry.
///
/// A name covers a cell only on its OWN backing grid. That grid is resolvable
/// via [`name_scope_grid`] only when unambiguous (a Sheet-scoped name, or a
/// single-grid workbook); for a Workbook-scoped name in a multi-grid workbook it
/// degrades to `None` because `GridRectProjection` carries no grid identity. In
/// that degraded case the name covers NOTHING — never a cell that merely shares
/// its `(row, col)` on another sheet (the S2.8 vanishing bug: a `_names!R1C1`-
/// backed name must not hide `Sheet1!A1`/`Sheet2!A1`). The unresolved backing
/// cell itself is kept out of the list by skipping the `_names` sheet in
/// [`derive_entries`].
fn name_covers_cell(
    workspace: &WorkspaceState,
    name: &DefinedNameProjection,
    grid: &NodeId,
    row: u32,
    col: u32,
) -> bool {
    let DefinedNameTargetProjection::Static(rect) = &name.target else {
        return false;
    };
    let Some(scope_grid) = name_scope_grid(workspace, name) else {
        return false;
    };
    &scope_grid == grid
        && row >= rect.top_row
        && row <= rect.bottom_row
        && col >= rect.left_col
        && col <= rect.right_col
}

/// The host's `_names` backing-sheet display name — host-core's
/// `NAMES_BACKING_SHEET`, owner-ratified 2026-07-05 as "hidden-in-notebook": its
/// append-only column-A cells are an implementation detail of defined-name
/// storage, never notebook content, so the notebook is the designated surface
/// that hides this sheet. A TP crate can't depend on host-core for the const, so
/// it mirrors the ratified convention here.
const NAMES_BACKING_SHEET: &str = "_names";

/// Whether `grid` is the hidden `_names` defined-name backing sheet.
fn is_names_backing_grid(workspace: &WorkspaceState, grid: &NodeId) -> bool {
    workspace
        .sheets
        .iter()
        .any(|sheet| &sheet.grid_node_id == grid && sheet.display_name == NAMES_BACKING_SHEET)
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
        // The `_names` backing sheet is hidden-in-notebook (host convention):
        // its column-A cells store defined-name values, not user content.
        if is_names_backing_grid(workspace, grid_id) {
            continue;
        }
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
                .any(|name| name_covers_cell(workspace, name, grid_id, cell.row, cell.col));
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
        if is_names_backing_grid(workspace, grid_id) {
            continue;
        }
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
        assert_eq!(
            entry_classification(&empty, None),
            NodeClassification::Empty
        );
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
        let cells = vec![cell_projection(
            1,
            1,
            NodeValueProjection::Empty,
            Some(empty),
        )];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        assert_eq!(derive_entries(&ws).len(), 0);
    }

    /// S2.8 — the `+ name` affordance's seam: dispatching the atomic
    /// `CreateNamedValue` intent against the REAL two-sheet demo workbook (i)
    /// derives a `Name` entry for the new name, (ii) does not leak the
    /// `_names` backing sheet's own cell as a spurious "Other cells" `Cell`
    /// entry, and (iii) the name is live-referenceable: a formula elsewhere in
    /// the workbook resolves it.
    ///
    /// Regression guard for the S2.8 vanishing bug (fixed in this bead):
    /// `name_covers_cell` now resolves a name's backing grid via
    /// `name_scope_grid` (punting to `None` for a Workbook-scoped name once the
    /// workbook has >1 grid), so creating a `_names!R1C1`-backed name no longer
    /// wrongly hides `Sheet1!A1`/`Sheet2!A1`, which share only its `(row, col)`
    /// on other sheets. Assertion (iv) pins that; the coordinates-only
    /// `GridRectProjection` carrying no grid identity is why the backing cell
    /// itself stays unresolved (assertion (i)) and is instead hidden by the
    /// `_names`-sheet skip (assertion (ii)).
    #[test]
    fn create_named_value_derives_name_entry_without_leaking_backing_cell_and_is_referenceable() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};
        use dnacalc_skin_ir::intent::WorkspaceIntent;

        let session = build_demo_workbook().expect("demo workbook");
        // Capture the two demo sheets' stable grid ids before the session is
        // moved into the model-neutral `DocumentSession` wrapper (mirrors
        // `adapter.rs`'s own test setup).
        let sheet_rows = session.sheets().expect("demo sheets");
        let sheet1_grid = dnacalc_host_core::sheet_grid_node_id(sheet_rows[0].node_id);
        let sheet2_grid = dnacalc_host_core::sheet_grid_node_id(sheet_rows[1].node_id);
        let mut document = DocumentSession::Workbook(session);

        // Dispatch the SAME atomic intent the `+ name` affordance's Create
        // button dispatches.
        let receipt = document.dispatch(WorkspaceIntent::CreateNamedValue {
            name: "GrowthRate".to_string(),
            value_text: "0.12".to_string(),
        });
        assert!(
            receipt.accepted,
            "CreateNamedValue is accepted: {receipt:?}"
        );

        let after_snapshot = document.snapshot();
        let after = derive_entries(&after_snapshot);

        // (i) A Name entry for GrowthRate now exists (Workbook scope, the
        // literal value's static rect).
        let name_entry = after
            .iter()
            .find(|entry| entry.display_name == "GrowthRate")
            .expect("GrowthRate derives as a Name entry");
        let NotebookEntryKind::Name { name, backing_cell } = &name_entry.kind else {
            panic!(
                "GrowthRate must derive as NotebookEntryKind::Name, got {:?}",
                name_entry.kind
            );
        };
        assert_eq!(name.scope, DefinedNameScopeProjection::Workbook);
        assert!(!name.is_dynamic);
        // The workspace now has 3 grids (Sheet1, Sheet2, the lazily-created
        // `_names` sheet), so `resolve_backing_cell`'s documented single-sheet
        // assumption degrades to `None` rather than guessing which grid the
        // name's coordinates-only rect addresses — an honest degrade, not a
        // bug (see `name_scope_grid`'s doc comment).
        assert_eq!(
            *backing_cell, None,
            "backing cell does not resolve across >1 grid (documented single-sheet degrade)"
        );

        // (ii) No spurious Cell entry leaks for the `_names` backing sheet
        // itself: the third (newly-created) grid contributes ZERO "Other
        // cells" entries to the notebook.
        assert_eq!(
            after_snapshot.grids.len(),
            3,
            "the `_names` sheet was lazily created"
        );
        let names_grid = after_snapshot
            .grids
            .keys()
            .find(|grid| **grid != sheet1_grid && **grid != sheet2_grid)
            .cloned()
            .expect("a third grid (the `_names` sheet) now exists");
        let leaked_names_cells: Vec<_> = after
            .iter()
            .filter(|entry| matches!(&entry.kind, NotebookEntryKind::Cell { grid, .. } if *grid == names_grid))
            .collect();
        assert!(
            leaked_names_cells.is_empty(),
            "the `_names` backing cell must not leak as a spurious Cell entry, got {leaked_names_cells:?}"
        );

        // (iv) Regression: creating the name must NOT hide same-(row,col) cells
        // on the OTHER sheets. `Sheet1!A1` (value 1) and `Sheet2!A1` (value 6)
        // share only the (1,1) coordinates of the `_names` backing rect, so they
        // must still derive as Cell entries (the S2.8 vanishing bug, fixed).
        let cell_present = |grid: &dnacalc_skin_ir::NodeId, row: u32, col: u32| {
            after.iter().any(|entry| {
                matches!(&entry.kind,
                    NotebookEntryKind::Cell { grid: g, authored, .. }
                        if g == grid && authored.row == row && authored.col == col)
            })
        };
        assert!(
            cell_present(&sheet1_grid, 1, 1),
            "Sheet1!A1 must survive name creation (S2.8 vanishing bug fixed)"
        );
        assert!(
            cell_present(&sheet2_grid, 1, 1),
            "Sheet2!A1 must survive name creation (S2.8 vanishing bug fixed)"
        );

        // (iii) The name is live-referenceable: author a formula elsewhere
        // (Sheet1 A6 — empty in the demo, and outside the name's (1,1,1,1)
        // rect so it is unaffected by the limitation noted above) and prove it
        // resolves through the freshly-created name.
        let entry_receipt = document.dispatch(WorkspaceIntent::EnterGridCell {
            grid: sheet1_grid.clone(),
            row: 6,
            col: 1,
            text: "=GrowthRate*2".to_string(),
        });
        assert!(
            entry_receipt.accepted,
            "the referencing formula is accepted: {entry_receipt:?}"
        );

        let after2 = derive_entries(&document.snapshot());
        let a6 = after2
            .iter()
            .find_map(|entry| match &entry.kind {
                NotebookEntryKind::Cell {
                    grid,
                    authored,
                    value,
                } if *grid == sheet1_grid && authored.row == 6 && authored.col == 1 => {
                    Some(value.clone())
                }
                _ => None,
            })
            .expect("Sheet1 A6 derives as a Cell entry after authoring it");
        assert_eq!(
            a6,
            NodeValueProjection::Number {
                raw: "0.24".to_string(),
                display: "0.24".to_string(),
            },
            "=GrowthRate*2 resolves live to 0.24 (GrowthRate = 0.12)"
        );
    }
}

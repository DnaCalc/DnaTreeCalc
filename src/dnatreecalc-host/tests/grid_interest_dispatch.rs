//! Bead 3.6: dispatching `SetGridInterest` through the live host dispatcher
//! scopes a grid-backed node's windowed projection ("viewing is subscribing")
//! and streams a `GridChanged` delta the worker mirror can patch in place.

use std::sync::Arc;

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel, WorkspaceNodeFixture};
use dnatreecalc_skin_framework::{
    Dispatcher, NodeId, NodeValueProjection, SelectionState, WorkspaceDelta, WorkspaceDeltaChange,
    WorkspaceIntent,
};
use leptos::prelude::*;
use oxcalc_core::consumer::GridBackingSeed;
use oxcalc_core::grid::authored::{GridAuthoredCell, GridFormulaCell};
use oxcalc_core::grid::coords::{ExcelGridBounds, ExcelGridCellAddress};
use oxfunc_core::value::CalcValue;

#[test]
fn dispatch_set_grid_interest_scopes_projection_and_streams_grid_changed() {
    let _owner = Owner::new();

    // A workspace with one sheet node, given a three-cell grid backing.
    let fixture = WorkspaceFixture {
        schema_version: "treecalc-workspace-v1".to_string(),
        workspace_id: "grid-dispatch".to_string(),
        description: None,
        profile: None,
        nodes: vec![WorkspaceNodeFixture {
            node_id: "Sheet1".to_string(),
            formula: String::new(),
            is_meta: false,
            table: None,
        }],
    };
    let model = WorkspaceModel::try_from(fixture).unwrap();
    let mut session = TreeWorkspaceSession::from_model(&model).unwrap();
    session.recalculate().unwrap();

    let bounds = ExcelGridBounds::strict_excel();
    let address = |row, col| ExcelGridCellAddress::new("book:gd", "sheet:gd", row, col);
    let seed = GridBackingSeed {
        workbook_id: "book:gd".to_string(),
        sheet_id: "sheet:gd".to_string(),
        bounds,
        authored: vec![
            (
                address(1, 1),
                GridAuthoredCell::Literal(CalcValue::number(7.0)),
            ),
            (
                address(1, 2),
                GridAuthoredCell::Formula(GridFormulaCell::new(
                    "=A1*3",
                    "excel.grid.v1:cell:R[0]C[-1]*3",
                )),
            ),
            (
                address(2, 1),
                GridAuthoredCell::Literal(CalcValue::number(5.0)),
            ),
        ],
    };
    let sheet = NodeId::new("Sheet1");
    session.set_node_grid(&sheet, seed).unwrap();

    let session = Arc::new(std::sync::Mutex::new(session));
    let workspace_state = session.lock().unwrap().workspace_state().unwrap();
    // Before any interest window the projection carries all three cells.
    assert_eq!(workspace_state.grids[&sheet].cells.len(), 3);

    let workspace = RwSignal::new(workspace_state);
    let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(
        workspace.get_untracked().projection_seq,
    ));
    let selection = RwSignal::new(SelectionState::default());
    let dispatcher = Arc::new(HostDispatcher::with_session(
        selection,
        workspace,
        latest_delta,
        session,
    ));

    let receipt = dispatcher.dispatch(WorkspaceIntent::SetGridInterest {
        grid: sheet.clone(),
        top_row: 1,
        left_col: 1,
        bottom_row: 1,
        right_col: 2,
    });
    assert!(receipt.accepted, "{:?}", receipt.error);

    // The reactive projection scoped to the A1:B1 window: the off-window literal
    // in row 2 dropped out, and B1 still computes 21.
    let grid = workspace.get_untracked().grids[&sheet].clone();
    assert_eq!(grid.cells.len(), 2);
    assert!(grid.cells.iter().all(|cell| cell.row == 1));
    let b1 = grid
        .cells
        .iter()
        .find(|cell| cell.col == 2)
        .expect("B1 is in the window");
    assert!(matches!(&b1.value, NodeValueProjection::Number { display, .. } if display == "21"));

    // The streamed delta carries a mirror-applicable GridChanged patch.
    let delta = latest_delta.get_untracked();
    assert!(
        delta
            .changes
            .iter()
            .any(|change| matches!(change, WorkspaceDeltaChange::GridChanged(_))),
        "delta should carry GridChanged, got {:?}",
        delta.changes
    );
}

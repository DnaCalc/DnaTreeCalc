//! DNA TreeCalc built-in skin implementations.
//!
//! Each module implements one `WorkspaceSkin` from the framework crate.
//! The walking skeleton ships `triple_editor`, `outline_table`, and
//! the first showcase skins built directly on the Skin IR: formula tree,
//! dependency inspector, and value board.
//!
//! The Flow lens builds a deep nested view tree; raise the type-checker
//! recursion limit so the tachys view types resolve.
#![recursion_limit = "512"]

pub mod bench;
pub mod capture;
pub mod companions;
pub mod dependency_inspector;
pub mod flow;
pub mod formula_tree;
pub mod grid_canvas;
pub mod ledger;
mod node_management;
pub mod notebook;
pub mod outline_table;
pub mod sheet;
mod spine_widgets;
pub mod transport;
pub mod tree;
pub mod triple_editor;
pub mod value_board;
mod value_render;
pub mod workbook;

pub use bench::{BENCH_ID, Bench, BenchState};
pub use capture::{CAPTURE_ID, CaptureLens, CaptureState};
pub use companions::{
    CONSOLE_COMPANION_ID, ConsoleCompanion, ConsoleCompanionState, LENS_COMPANION_ID,
    LensCompanion, LensCompanionState,
};
pub use dependency_inspector::{
    DEPENDENCY_INSPECTOR_ID, DependencyInspector, DependencyInspectorState,
};
pub use flow::{FLOW_ID, FlowLens, FlowState};
pub use formula_tree::{FORMULA_TREE_ID, FormulaTree, FormulaTreeState};
pub use ledger::{LEDGER_ID, LedgerLens, LedgerState};
pub use notebook::{NOTEBOOK_ID, NotebookLens, NotebookState};
pub use outline_table::{OUTLINE_TABLE_ID, OutlineTable, OutlineTableState};
pub use sheet::{SHEET_ID, SheetLens, SheetState};
pub use transport::{TRANSPORT_ID, TransportLens, TransportState};
pub use tree::{TREE_ID, TreeLens, TreeState};
pub use triple_editor::{TRIPLE_EDITOR_ID, TripleEditor, TripleEditorState};
pub use value_board::{VALUE_BOARD_ID, ValueBoard, ValueBoardState};
pub use workbook::{WORKBOOK_ID, WorkbookLens, WorkbookState};

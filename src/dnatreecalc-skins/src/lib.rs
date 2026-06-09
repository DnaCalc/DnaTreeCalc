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

pub mod dependency_inspector;
pub mod flow;
pub mod formula_tree;
mod node_management;
pub mod outline_table;
pub mod triple_editor;
pub mod value_board;
mod value_render;

pub use dependency_inspector::{
    DEPENDENCY_INSPECTOR_ID, DependencyInspector, DependencyInspectorState,
};
pub use flow::{FLOW_ID, FlowLens, FlowState};
pub use formula_tree::{FORMULA_TREE_ID, FormulaTree, FormulaTreeState};
pub use outline_table::{OUTLINE_TABLE_ID, OutlineTable, OutlineTableState};
pub use triple_editor::{TRIPLE_EDITOR_ID, TripleEditor, TripleEditorState};
pub use value_board::{VALUE_BOARD_ID, ValueBoard, ValueBoardState};

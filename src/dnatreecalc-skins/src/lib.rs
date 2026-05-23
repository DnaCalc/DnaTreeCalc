//! DNA TreeCalc built-in skin implementations.
//!
//! Each module implements one `WorkspaceSkin` from the framework crate.
//! The walking skeleton ships `triple_editor` (an Editor-category skin
//! with nav-rail + formula pane + value pane) and `outline_table`
//! (an Overview-category skin rendering nodes as a flat table).
//! `cell_view` lands with W006 as the v1 skin-bar completion;
//! `canvas_flow` and `nodes_across` are enrichment per the register.

pub mod outline_table;
pub mod triple_editor;

pub use outline_table::{OUTLINE_TABLE_ID, OutlineTable, OutlineTableState};
pub use triple_editor::{TRIPLE_EDITOR_ID, TripleEditor, TripleEditorState};

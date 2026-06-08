use crate::identity::NodeId;

/// Host-wide selection shared across all mounted skins.
///
/// Skins read this through [`SkinContext::selection`](crate::SkinContext::selection);
/// changes go through the [`crate::Dispatcher`] using
/// [`crate::WorkspaceIntent::SelectNode`] so selection updates are
/// observable through the same path as any other intent and stay
/// auditable in tests. Table-cell focus is still selection state rather
/// than engine state; multi-select / anchor / navigation history arrive
/// with later work.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    pub primary: Option<NodeId>,
    pub table_cell: Option<TableCellSelection>,
}

impl SelectionState {
    #[must_use]
    pub fn with_primary(primary: Option<NodeId>) -> Self {
        Self {
            primary,
            table_cell: None,
        }
    }

    #[must_use]
    pub fn with_table_cell(table_cell: TableCellSelection) -> Self {
        Self {
            primary: Some(table_cell.table.clone()),
            table_cell: Some(table_cell),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellSelection {
    pub table: NodeId,
    pub row_id: Option<String>,
    pub column_id: String,
}

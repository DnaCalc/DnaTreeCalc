use crate::identity::NodeId;

/// Host-wide selection shared across all mounted skins.
///
/// Skins read this through [`SkinContext::selection`](crate::SkinContext::selection);
/// changes go through the [`crate::Dispatcher`] using
/// [`crate::WorkspaceIntent::SelectNode`] so selection updates are
/// observable through the same path as any other intent and stay
/// auditable in tests. Multi-select / anchor / navigation history
/// arrive with W003 — the spec carries them but the skeleton uses only
/// the primary slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    pub primary: Option<NodeId>,
}

impl SelectionState {
    #[must_use]
    pub fn with_primary(primary: Option<NodeId>) -> Self {
        Self { primary }
    }
}

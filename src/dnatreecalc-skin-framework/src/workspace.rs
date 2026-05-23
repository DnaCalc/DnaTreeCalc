use std::collections::BTreeMap;

use crate::identity::NodeId;

/// Read-side projection of the workspace, as seen by a mounted skin.
///
/// The host owns the canonical model and the bridge; this struct is what
/// the host publishes through the [`SkinContext::workspace`](crate::SkinContext::workspace)
/// signal so skins can render without knowing the OxCalc bridge or the
/// persistence format. Mirrors the spec shape in `docs/ux/SKINS.md` §2.7,
/// narrowed for the walking skeleton — meta-namespaces, templates, formats,
/// and cross-workspace aliases land as later worksets extend the projection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceState {
    pub workspace_id: String,
    pub profile: &'static str,
    pub node_order: Vec<NodeId>,
    pub root_paths: Vec<NodeId>,
    pub nodes: BTreeMap<NodeId, NodeView>,
}

impl WorkspaceState {
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&NodeView> {
        self.nodes.get(id)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeView {
    pub id: NodeId,
    pub display_name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub depth: u32,
    pub content_kind: NodeContentKind,
    pub content_text: String,
    pub computed_value: NodeValueProjection,
    pub is_meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeContentKind {
    Empty,
    Constant,
    Formula,
}

/// What the skin should render for a node's value cell.
///
/// The walking skeleton renders a small set of value shapes; richer
/// shape diff / array virtualization / table rendering arrives with
/// `UX-VA-002`/`UX-VA-003` in W003+W006.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NodeValueProjection {
    /// The node has never been evaluated by the bridge.
    #[default]
    Unevaluated,
    /// Evaluation is in flight; previous value (if any) is the responsibility
    /// of the renderer — the projection only records the in-flight state.
    Pending,
    /// A formatted scalar ready for display. Formatting comes from the host's
    /// format resolver (W007); the walking skeleton uses raw debug text.
    Scalar(String),
    /// The bridge reported a typed diagnostic for this node.
    Error(String),
}

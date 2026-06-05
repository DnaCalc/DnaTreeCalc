use std::collections::BTreeMap;

use crate::identity::NodeId;

/// Read-side projection of the workspace, as seen by a mounted skin.
///
/// The host owns the UI projection while OxCalc owns the canonical model; this struct is what
/// the host publishes through the [`SkinContext::workspace`](crate::SkinContext::workspace)
/// signal so skins can render without knowing the OxCalc context or the
/// persistence format. Mirrors the spec shape in `docs/ux/SKINS.md` §2.7,
/// narrowed for the walking skeleton — meta-namespaces, templates, formats,
/// and cross-workspace aliases land as later worksets extend the projection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceState {
    pub workspace_id: String,
    pub profile: &'static str,
    pub revision: WorkspaceRevisionProjection,
    pub last_run: Option<CalcRunProjection>,
    pub node_order: Vec<NodeId>,
    pub root_paths: Vec<NodeId>,
    pub nodes: BTreeMap<NodeId, NodeView>,
    pub dependencies: DependencyGraphProjection,
    pub tables: BTreeMap<NodeId, TableProjection>,
    pub diagnostics: Vec<String>,
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
    pub calc_state: Option<NodeCalcStateProjection>,
    pub is_meta: bool,
    pub table: Option<TableProjection>,
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
    /// The node has never been evaluated by OxCalc.
    #[default]
    Unevaluated,
    /// Evaluation is in flight; previous value (if any) is the responsibility
    /// of the renderer — the projection only records the in-flight state.
    Pending,
    /// A formatted scalar ready for display. Formatting comes from the host's
    /// format resolver (W007); the walking skeleton uses raw debug text.
    Scalar(String),
    /// OxCalc reported a typed diagnostic for this node.
    Error(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceRevisionProjection {
    pub structural_snapshot_id: Option<String>,
    pub workspace_revision_id: Option<String>,
    pub node_input_snapshot_id: Option<String>,
    pub namespace_snapshot_id: Option<String>,
    pub formula_binding_snapshot_id: Option<String>,
    pub dependency_shape_snapshot_id: Option<String>,
    pub publication_snapshot_id: Option<String>,
    pub runtime_overlay_set_id: Option<String>,
    pub value_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcRunStateProjection {
    Published,
    VerifiedClean,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalcRunProjection {
    pub run_state: CalcRunStateProjection,
    pub evaluation_order: Vec<NodeId>,
    pub runtime_effect_count: usize,
    pub runtime_overlay_count: usize,
    pub derivation_trace_count: usize,
    pub invalidated_nodes: Vec<NodeInvalidationProjection>,
    pub phase_timings_micros: BTreeMap<String, u128>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCalcStateProjection {
    Clean,
    DirtyPending,
    Needed,
    Evaluating,
    VerifiedClean,
    PublishReady,
    RejectedPendingRepair,
    CycleBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInvalidationProjection {
    pub node: NodeId,
    pub calc_state: NodeCalcStateProjection,
    pub requires_rebind: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyGraphProjection {
    pub descriptors_by_owner: BTreeMap<NodeId, Vec<DependencyDescriptorProjection>>,
    pub edges_by_owner: BTreeMap<NodeId, Vec<DependencyEdgeProjection>>,
    pub reverse_edges: BTreeMap<NodeId, Vec<DependencyEdgeProjection>>,
    pub cycle_groups: Vec<Vec<NodeId>>,
    pub diagnostics: Vec<String>,
}

impl DependencyGraphProjection {
    #[must_use]
    pub fn outgoing_count(&self, owner: &NodeId) -> usize {
        self.edges_by_owner.get(owner).map_or(0, Vec::len)
    }

    #[must_use]
    pub fn incoming_count(&self, target: &NodeId) -> usize {
        self.reverse_edges.get(target).map_or(0, Vec::len)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDescriptorProjection {
    pub descriptor_id: String,
    pub source_reference_handle: Option<String>,
    pub target: Option<NodeId>,
    pub workspace_target: Option<String>,
    pub kind: String,
    pub carrier_detail: String,
    pub collection: Option<TreeReferenceCollectionProjection>,
    pub requires_rebind_on_structural_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdgeProjection {
    pub edge_id: String,
    pub descriptor_id: String,
    pub owner: NodeId,
    pub target: NodeId,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeReferenceCollectionProjection {
    pub family: String,
    pub source_reference_handle: String,
    pub base_node: Option<NodeId>,
    pub membership_version: String,
    pub order_version: String,
    pub members: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableProjection {
    pub table_id: String,
    pub table_name: String,
    pub display_path: String,
    pub canonical_path: String,
    pub row_count: usize,
    pub column_count: usize,
    pub header_row_present: bool,
    pub totals_row_present: bool,
    pub table_namespace_version: String,
    pub row_membership_version: String,
    pub row_order_version: String,
    pub column_identity_version: String,
    pub dependency_inventory_summary: Vec<String>,
}

use std::collections::BTreeMap;

use std::fmt;

use crate::identity::{NodeId, NodeKey};

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
    pub key_order: Vec<NodeKey>,
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
    pub key: NodeKey,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NodeValueProjection {
    /// The node has never been evaluated by OxCalc.
    #[default]
    Unevaluated,
    /// Evaluation is in flight; previous value (if any) is the responsibility
    /// of the renderer — the projection only records the in-flight state.
    Pending,
    /// Legacy text-only fallback used when the host has display text but no typed engine value.
    Scalar(String),
    /// Numeric scalar with both canonical raw text and rendered display text.
    Number { raw: String, display: String },
    /// Text scalar.
    Text(String),
    /// Boolean scalar.
    Logical { value: bool, display: String },
    /// Empty scalar.
    Empty,
    /// Missing argument/value marker.
    Missing,
    /// Reference value projected as an engine target string.
    Reference { target: String },
    /// A rectangular or ragged array result with typed projected cells.
    Array {
        rows: usize,
        cols: usize,
        cells: Vec<Vec<NodeValueProjection>>,
    },
    /// OxCalc reported a typed diagnostic for this node.
    Error(String),
}

impl NodeValueProjection {
    #[must_use]
    pub fn display_text(&self) -> String {
        match self {
            Self::Unevaluated => "-".to_string(),
            Self::Pending => "...".to_string(),
            Self::Scalar(text)
            | Self::Number { display: text, .. }
            | Self::Text(text)
            | Self::Logical { display: text, .. }
            | Self::Error(text) => text.clone(),
            Self::Empty => String::new(),
            Self::Missing => "missing".to_string(),
            Self::Reference { target } => target.clone(),
            Self::Array { cells, .. } => cells
                .iter()
                .map(|row| {
                    row.iter()
                        .map(Self::display_text)
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    #[must_use]
    pub fn scalar_display_text(&self) -> Option<&str> {
        match self {
            Self::Scalar(text)
            | Self::Number { display: text, .. }
            | Self::Text(text)
            | Self::Logical { display: text, .. }
            | Self::Error(text) => Some(text.as_str()),
            Self::Empty => Some(""),
            Self::Missing => Some("missing"),
            Self::Reference { target } => Some(target.as_str()),
            Self::Unevaluated | Self::Pending | Self::Array { .. } => None,
        }
    }
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
    pub phase_timings_micros: BTreeMap<PhaseKeyProjection, u128>,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhaseKeyProjection {
    OxfmlPrepareFormulas,
    DependencyDescriptorLowering,
    DependencyDescriptorOwnerIndex,
    DependencyGraphBuildAndCycleScan,
    InvalidationClosureDerivation,
    RuntimeSetup,
    DiagnosticSeedCollection,
    RecalcTrackerMarkDirtyNeeded,
    TopologicalFormulaOrder,
    RebindGateScan,
    DependencyDiagnosticRejectScan,
    EdgeValueCacheLookup,
    OxfmlFormulaEvaluation,
    DerivationTraceRecord,
    EdgeValueCacheStore,
    EvaluationLoopTotal,
    VerifiedCleanFinalize,
    CandidatePublication,
    RejectionRecording,
    TotalEngineExecute,
    Other(String),
}

impl PhaseKeyProjection {
    #[must_use]
    pub fn stable_id(&self) -> &str {
        match self {
            Self::OxfmlPrepareFormulas => "oxfml_prepare_formulas",
            Self::DependencyDescriptorLowering => "dependency_descriptor_lowering",
            Self::DependencyDescriptorOwnerIndex => "dependency_descriptor_owner_index",
            Self::DependencyGraphBuildAndCycleScan => "dependency_graph_build_and_cycle_scan",
            Self::InvalidationClosureDerivation => "invalidation_closure_derivation",
            Self::RuntimeSetup => "runtime_setup",
            Self::DiagnosticSeedCollection => "diagnostic_seed_collection",
            Self::RecalcTrackerMarkDirtyNeeded => "recalc_tracker_mark_dirty_needed",
            Self::TopologicalFormulaOrder => "topological_formula_order",
            Self::RebindGateScan => "rebind_gate_scan",
            Self::DependencyDiagnosticRejectScan => "dependency_diagnostic_reject_scan",
            Self::EdgeValueCacheLookup => "edge_value_cache_lookup",
            Self::OxfmlFormulaEvaluation => "oxfml_formula_evaluation",
            Self::DerivationTraceRecord => "derivation_trace_record",
            Self::EdgeValueCacheStore => "edge_value_cache_store",
            Self::EvaluationLoopTotal => "evaluation_loop_total",
            Self::VerifiedCleanFinalize => "verified_clean_finalize",
            Self::CandidatePublication => "candidate_publication",
            Self::RejectionRecording => "rejection_recording",
            Self::TotalEngineExecute => "total_engine_execute",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl fmt::Display for PhaseKeyProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInvalidationProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub calc_state: NodeCalcStateProjection,
    pub requires_rebind: bool,
    pub reasons: Vec<InvalidationReasonProjection>,
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
    pub kind: DependencyKindProjection,
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
    pub kind: DependencyKindProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeReferenceCollectionProjection {
    pub family: TreeReferenceCollectionFamilyProjection,
    pub source_reference_handle: String,
    pub base_node: Option<NodeId>,
    pub membership_version: String,
    pub order_version: String,
    pub members: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyKindProjection {
    StaticDirect,
    RelativeBound,
    TreeReferenceCollectionMembership,
    TreeReferenceCollectionMemberValue,
    StructuredTableIdentity,
    StructuredTableRowMembership,
    StructuredTableRowOrder,
    StructuredTableColumnIdentity,
    StructuredTableHeaderText,
    StructuredTableHeaderRegion,
    StructuredTableDataRegion,
    StructuredTableTotalsRegion,
    StructuredTableCallerContext,
    StructuredTableEnclosingTable,
    DynamicPotential,
    HostSensitive,
    CapabilitySensitive,
    ShapeTopology,
    Unresolved,
}

impl DependencyKindProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::StaticDirect => "static_direct",
            Self::RelativeBound => "relative_bound",
            Self::TreeReferenceCollectionMembership => "tree_reference_collection_membership",
            Self::TreeReferenceCollectionMemberValue => "tree_reference_collection_member_value",
            Self::StructuredTableIdentity => "structured_table_identity",
            Self::StructuredTableRowMembership => "structured_table_row_membership",
            Self::StructuredTableRowOrder => "structured_table_row_order",
            Self::StructuredTableColumnIdentity => "structured_table_column_identity",
            Self::StructuredTableHeaderText => "structured_table_header_text",
            Self::StructuredTableHeaderRegion => "structured_table_header_region",
            Self::StructuredTableDataRegion => "structured_table_data_region",
            Self::StructuredTableTotalsRegion => "structured_table_totals_region",
            Self::StructuredTableCallerContext => "structured_table_caller_context",
            Self::StructuredTableEnclosingTable => "structured_table_enclosing_table",
            Self::DynamicPotential => "dynamic_potential",
            Self::HostSensitive => "host_sensitive",
            Self::CapabilitySensitive => "capability_sensitive",
            Self::ShapeTopology => "shape_topology",
            Self::Unresolved => "unresolved",
        }
    }
}

impl fmt::Display for DependencyKindProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidationReasonProjection {
    StructuralRebindRequired,
    StructuralRecalcOnly,
    UpstreamPublication,
    ExternallyInvalidated,
    TreeReferenceMembershipChanged,
    TreeReferenceOrderChanged,
    StructuredTableContextChanged,
    StructuredTableRowMembershipChanged,
    StructuredTableRowOrderChanged,
    StructuredTableColumnChanged,
    StructuredTableRegionChanged,
    StructuredTableCallerContextChanged,
    DependencyAdded,
    DependencyRemoved,
    DependencyReclassified,
    DynamicDependencyActivated,
    DynamicDependencyReleased,
    DynamicDependencyReclassified,
}

impl InvalidationReasonProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::StructuralRebindRequired => "structural_rebind_required",
            Self::StructuralRecalcOnly => "structural_recalc_only",
            Self::UpstreamPublication => "upstream_publication",
            Self::ExternallyInvalidated => "externally_invalidated",
            Self::TreeReferenceMembershipChanged => "tree_reference_membership_changed",
            Self::TreeReferenceOrderChanged => "tree_reference_order_changed",
            Self::StructuredTableContextChanged => "structured_table_context_changed",
            Self::StructuredTableRowMembershipChanged => "structured_table_row_membership_changed",
            Self::StructuredTableRowOrderChanged => "structured_table_row_order_changed",
            Self::StructuredTableColumnChanged => "structured_table_column_changed",
            Self::StructuredTableRegionChanged => "structured_table_region_changed",
            Self::StructuredTableCallerContextChanged => "structured_table_caller_context_changed",
            Self::DependencyAdded => "dependency_added",
            Self::DependencyRemoved => "dependency_removed",
            Self::DependencyReclassified => "dependency_reclassified",
            Self::DynamicDependencyActivated => "dynamic_dependency_activated",
            Self::DynamicDependencyReleased => "dynamic_dependency_released",
            Self::DynamicDependencyReclassified => "dynamic_dependency_reclassified",
        }
    }
}

impl fmt::Display for InvalidationReasonProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TreeReferenceCollectionFamilyProjection {
    Children,
    ReferenceLiteralArray,
    Siblings,
    Preceding,
    Following,
    Ancestors,
    RecursiveDescendants,
}

impl TreeReferenceCollectionFamilyProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Children => "children",
            Self::ReferenceLiteralArray => "reference_literal_array",
            Self::Siblings => "siblings",
            Self::Preceding => "preceding",
            Self::Following => "following",
            Self::Ancestors => "ancestors",
            Self::RecursiveDescendants => "recursive_descendants",
        }
    }
}

impl fmt::Display for TreeReferenceCollectionFamilyProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
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

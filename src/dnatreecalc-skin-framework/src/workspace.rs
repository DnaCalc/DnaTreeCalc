use std::collections::{BTreeMap, BTreeSet};

use std::fmt;

use crate::identity::{NodeId, NodeKey};
use crate::intent::{AuthoringScope, TableCellInput, TableRowInput};
use crate::selection::SelectionState;

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
    pub projection_seq: u64,
    pub revision: WorkspaceRevisionProjection,
    pub last_run: Option<CalcRunProjection>,
    pub node_order: Vec<NodeId>,
    pub key_order: Vec<NodeKey>,
    pub paths_by_key: BTreeMap<NodeKey, NodeId>,
    pub root_keys: Vec<NodeKey>,
    pub root_paths: Vec<NodeId>,
    pub nodes_by_key: BTreeMap<NodeKey, NodeView>,
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
    pub fn node_id_for_key(&self, key: &NodeKey) -> Option<&NodeId> {
        self.paths_by_key.get(key)
    }

    #[must_use]
    pub fn node_by_key(&self, key: &NodeKey) -> Option<&NodeView> {
        self.nodes_by_key.get(key)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn active_node_detail(
        &self,
        selection: &SelectionState,
    ) -> Option<ActiveNodeDetailProjection> {
        let node_id = selection.primary.as_ref()?;
        let node = self.node(node_id)?;
        Some(ActiveNodeDetailProjection {
            node: node.id.clone(),
            node_key: node.key.clone(),
            display_name: node.display_name.clone(),
            content_kind: node.content_kind,
            content_text: node.content_text.clone(),
            value: node.computed_value.clone(),
            value_epoch: node.value_epoch,
            calc_state: node.calc_state,
            effective_format: node.effective_format.clone(),
            note: node.note.clone(),
            binding_diagnostics: node.binding_diagnostics.clone(),
            outgoing_references: self
                .dependencies
                .reference_resolutions
                .values()
                .filter(|resolution| resolution.owner_key == node.key)
                .cloned()
                .collect(),
            incoming_reference_handles: self
                .dependencies
                .reverse_references
                .get(&node.key)
                .cloned()
                .unwrap_or_default(),
        })
    }

    #[must_use]
    pub fn active_selection_detail(
        &self,
        selection: &SelectionState,
    ) -> Option<ActiveSelectionDetailProjection> {
        if let Some(table_cell) = self.active_table_cell_detail(selection) {
            return Some(ActiveSelectionDetailProjection::TableCell(table_cell));
        }
        self.active_node_detail(selection)
            .map(ActiveSelectionDetailProjection::Node)
    }

    #[must_use]
    pub fn active_table_cell_detail(
        &self,
        selection: &SelectionState,
    ) -> Option<ActiveTableCellDetailProjection> {
        let selected = selection.table_cell.as_ref()?;
        let table = self.tables.get(&selected.table)?;
        let cells = table.cells.as_ref()?;
        let column_index = table
            .columns
            .iter()
            .position(|column| column.column_id == selected.column_id)?;
        let column = &table.columns[column_index];

        let (region, row_ordinal, cell) = match selected.row_id.as_deref() {
            Some(row_id) => {
                let row_index = table.rows.iter().position(|row| row.row_id == row_id)?;
                let row = &table.rows[row_index];
                let cell = cells
                    .body_rows
                    .get(row_index)?
                    .get(column_index)?
                    .as_ref()?;
                (TableCellRegionProjection::Body, Some(row.ordinal), cell)
            }
            None => {
                let cell = cells.totals_row.get(column_index)?.as_ref()?;
                (TableCellRegionProjection::Totals, None, cell)
            }
        };
        if cell.row_id.as_deref() != selected.row_id.as_deref()
            || cell.column_id != selected.column_id
        {
            return None;
        }

        Some(ActiveTableCellDetailProjection {
            table: selected.table.clone(),
            table_id: table.table_id.clone(),
            table_name: table.table_name.clone(),
            row_id: cell.row_id.clone(),
            row_ordinal,
            column_id: column.column_id.clone(),
            column_name: column.name.clone(),
            column_ordinal: column.ordinal,
            region,
            formula: active_table_cell_formula(region, column),
            editability: active_table_cell_editability(region, column),
            node_key: cell.node_key.clone(),
            value: cell.value.clone(),
            outgoing_references: self
                .dependencies
                .reference_resolutions
                .values()
                .filter(|resolution| resolution.owner_key == cell.node_key)
                .cloned()
                .collect(),
            incoming_reference_handles: self
                .dependencies
                .reverse_references
                .get(&cell.node_key)
                .cloned()
                .unwrap_or_default(),
        })
    }

    pub fn expand_authoring_scope(
        &self,
        scope: &AuthoringScope,
    ) -> Result<Vec<NodeKey>, AuthoringScopeExpansionError> {
        match scope {
            AuthoringScope::Node(node) => {
                self.require_node_key(node)?;
                Ok(vec![node.clone()])
            }
            AuthoringScope::Nodes(nodes) => nodes
                .iter()
                .map(|node| {
                    self.require_node_key(node)?;
                    Ok(node.clone())
                })
                .collect::<Result<Vec<_>, _>>()
                .map(dedupe_preserving_order),
            AuthoringScope::Subtree(root) => self.expand_subtree_scope(root),
            AuthoringScope::Collection {
                owner,
                source_reference_handle,
            } => self.expand_collection_scope(owner, source_reference_handle),
        }
    }

    fn require_node_key(&self, node: &NodeKey) -> Result<&NodeView, AuthoringScopeExpansionError> {
        self.node_by_key(node)
            .ok_or_else(|| AuthoringScopeExpansionError::UnknownNode { node: node.clone() })
    }

    fn expand_subtree_scope(
        &self,
        root: &NodeKey,
    ) -> Result<Vec<NodeKey>, AuthoringScopeExpansionError> {
        self.require_node_key(root)?;
        let mut expanded = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![root.clone()];
        while let Some(node_key) = stack.pop() {
            if !seen.insert(node_key.clone()) {
                continue;
            }
            let node = self.require_node_key(&node_key)?;
            expanded.push(node.key.clone());
            for child_id in node.children.iter().rev() {
                let child = self.node(child_id).ok_or_else(|| {
                    AuthoringScopeExpansionError::ProjectionOutOfSync {
                        detail: format!(
                            "node {} lists child {} that is not in the projection",
                            node.key, child_id
                        ),
                    }
                })?;
                stack.push(child.key.clone());
            }
        }
        Ok(expanded)
    }

    fn expand_collection_scope(
        &self,
        owner: &NodeKey,
        source_reference_handle: &str,
    ) -> Result<Vec<NodeKey>, AuthoringScopeExpansionError> {
        self.require_node_key(owner)?;
        let resolution = self
            .dependencies
            .reference_resolutions
            .get(source_reference_handle)
            .ok_or_else(|| AuthoringScopeExpansionError::UnknownCollection {
                owner: owner.clone(),
                source_reference_handle: source_reference_handle.to_string(),
            })?;
        if &resolution.owner_key != owner {
            return Err(AuthoringScopeExpansionError::UnknownCollection {
                owner: owner.clone(),
                source_reference_handle: source_reference_handle.to_string(),
            });
        }
        let ReferenceTargetProjection::Collection { member_keys, .. } = &resolution.target else {
            return Err(AuthoringScopeExpansionError::NotCollection {
                owner: owner.clone(),
                source_reference_handle: source_reference_handle.to_string(),
            });
        };
        member_keys
            .iter()
            .map(|node| {
                self.require_node_key(node)?;
                Ok(node.clone())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(dedupe_preserving_order)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthoringScopeExpansionError {
    #[error("unknown authoring-scope node {node}")]
    UnknownNode { node: NodeKey },
    #[error("unknown collection handle {source_reference_handle} on owner {owner}")]
    UnknownCollection {
        owner: NodeKey,
        source_reference_handle: String,
    },
    #[error("reference handle {source_reference_handle} on owner {owner} is not a collection")]
    NotCollection {
        owner: NodeKey,
        source_reference_handle: String,
    },
    #[error("workspace projection is out of sync: {detail}")]
    ProjectionOutOfSync { detail: String },
}

fn dedupe_preserving_order(nodes: Vec<NodeKey>) -> Vec<NodeKey> {
    let mut seen = BTreeSet::new();
    nodes
        .into_iter()
        .filter(|node| seen.insert(node.clone()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveNodeDetailProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub display_name: String,
    pub content_kind: NodeContentKind,
    pub content_text: String,
    pub value: NodeValueProjection,
    pub value_epoch: Option<u64>,
    pub calc_state: Option<NodeCalcStateProjection>,
    pub effective_format: Option<EffectiveFormatProjection>,
    pub note: Option<NodeNoteProjection>,
    pub binding_diagnostics: Vec<BindingDiagnosticProjection>,
    pub outgoing_references: Vec<ReferenceResolutionProjection>,
    pub incoming_reference_handles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveSelectionDetailProjection {
    Node(ActiveNodeDetailProjection),
    TableCell(ActiveTableCellDetailProjection),
}

impl ActiveSelectionDetailProjection {
    #[must_use]
    pub const fn stable_id(&self) -> &'static str {
        match self {
            Self::Node(_) => "node",
            Self::TableCell(_) => "table_cell",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTableCellDetailProjection {
    pub table: NodeId,
    pub table_id: String,
    pub table_name: String,
    pub row_id: Option<String>,
    pub row_ordinal: Option<usize>,
    pub column_id: String,
    pub column_name: String,
    pub column_ordinal: u32,
    pub region: TableCellRegionProjection,
    pub formula: Option<TableFormulaMetadataProjection>,
    pub editability: TableCellEditabilityProjection,
    pub node_key: NodeKey,
    pub value: NodeValueProjection,
    pub outgoing_references: Vec<ReferenceResolutionProjection>,
    pub incoming_reference_handles: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCellRegionProjection {
    Body,
    Totals,
}

impl TableCellRegionProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Totals => "totals",
        }
    }
}

impl fmt::Display for TableCellRegionProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCellEditabilityProjection {
    DirectInput,
    FormulaBacked,
    TotalsFormula,
    ReadOnly,
}

impl TableCellEditabilityProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::DirectInput => "direct_input",
            Self::FormulaBacked => "formula_backed",
            Self::TotalsFormula => "totals_formula",
            Self::ReadOnly => "read_only",
        }
    }
}

impl fmt::Display for TableCellEditabilityProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

fn active_table_cell_formula(
    region: TableCellRegionProjection,
    column: &TableColumnProjection,
) -> Option<TableFormulaMetadataProjection> {
    match region {
        TableCellRegionProjection::Body => match &column.body {
            TableColumnBodyProjection::Formula(formula) => Some(formula.clone()),
            TableColumnBodyProjection::ConstantCells => None,
        },
        TableCellRegionProjection::Totals => column.totals_formula.clone(),
    }
}

fn active_table_cell_editability(
    region: TableCellRegionProjection,
    column: &TableColumnProjection,
) -> TableCellEditabilityProjection {
    match region {
        TableCellRegionProjection::Body => match &column.body {
            TableColumnBodyProjection::ConstantCells => TableCellEditabilityProjection::DirectInput,
            TableColumnBodyProjection::Formula(_) => TableCellEditabilityProjection::FormulaBacked,
        },
        TableCellRegionProjection::Totals => {
            if column.totals_formula.is_some() {
                TableCellEditabilityProjection::TotalsFormula
            } else {
                TableCellEditabilityProjection::ReadOnly
            }
        }
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
    pub value_epoch: Option<u64>,
    pub calc_state: Option<NodeCalcStateProjection>,
    pub effective_format: Option<EffectiveFormatProjection>,
    pub note: Option<NodeNoteProjection>,
    pub binding_diagnostics: Vec<BindingDiagnosticProjection>,
    pub is_meta: bool,
    pub table: Option<TableProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeNoteProjection {
    pub text: String,
    pub source: NoteSourceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSourceProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveFormatProjection {
    pub number_format_code: Option<String>,
    pub inherited_from: Option<FormatSourceProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatSourceProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingDiagnosticProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeContentKind {
    Empty,
    Constant,
    Formula,
}

impl NodeContentKind {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Constant => "constant",
            Self::Formula => "formula",
        }
    }
}

impl fmt::Display for NodeContentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
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
    pub runtime_effects: Vec<RuntimeEffectProjection>,
    pub runtime_overlay_count: usize,
    pub runtime_overlays: Vec<RuntimeOverlayProjection>,
    pub derivation_trace_count: usize,
    pub derivation_traces: Vec<DerivationTraceProjection>,
    pub invalidated_nodes: Vec<NodeInvalidationProjection>,
    pub binding_diagnostics: Vec<BindingDiagnosticProjection>,
    pub phase_timings_micros: BTreeMap<PhaseKeyProjection, u128>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecalcPlanMutation {
    SetNodeInput {
        node: NodeId,
    },
    EditContent {
        node: NodeId,
        content: String,
    },
    EditScopedContent {
        scope: AuthoringScope,
        content: String,
    },
    AddTableFormulaColumn {
        table: NodeId,
        column_id: String,
        name: String,
        formula_text: String,
    },
    AddTableRow {
        table: NodeId,
        row_id: String,
        values: Vec<TableCellInput>,
    },
    DeleteTableRow {
        table: NodeId,
        row_id: String,
    },
    RenameTableRow {
        table: NodeId,
        row_id: String,
        new_row_id: String,
    },
    ReorderTableRow {
        table: NodeId,
        row_id: String,
        new_index: usize,
    },
    AddTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
        values: Vec<TableRowInput>,
    },
    DeleteTableColumn {
        table: NodeId,
        column_id: String,
    },
    RenameTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
    },
    ReorderTableColumn {
        table: NodeId,
        column_id: String,
        new_index: usize,
    },
    RenameNode {
        node: NodeId,
    },
    MoveNode {
        node: NodeId,
    },
    ReorderNode {
        node: NodeId,
    },
    DeleteNode {
        node: NodeId,
    },
    InvalidateNode {
        node: NodeId,
        reason: InvalidationReasonProjection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialNodeContentProjection {
    Empty,
    Literal { content: String },
    InheritColumnFormula,
    TemplateBound { template_id: String },
}

impl InitialNodeContentProjection {
    #[must_use]
    pub const fn stable_id(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Literal { .. } => "literal",
            Self::InheritColumnFormula => "inherit_column_formula",
            Self::TemplateBound { .. } => "template_bound",
        }
    }

    #[must_use]
    pub fn supported_content(&self) -> Option<&str> {
        match self {
            Self::Empty => Some(""),
            Self::Literal { content } => Some(content),
            Self::InheritColumnFormula | Self::TemplateBound { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalcPlanInvalidationProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub requires_rebind: bool,
    pub reasons: Vec<InvalidationReasonProjection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecalcPlanProjection {
    pub invalidated_nodes: Vec<RecalcPlanInvalidationProjection>,
    pub evaluation_order: Vec<NodeId>,
    pub requires_rebind: Vec<NodeId>,
    pub estimated_node_count: usize,
    pub cycle_risk: Vec<Vec<NodeId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaBindPreviewInputKind {
    Literal,
    Formula,
}

impl FormulaBindPreviewInputKind {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Formula => "formula",
        }
    }
}

impl fmt::Display for FormulaBindPreviewInputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaBindPreviewDiagnosticStage {
    Syntax,
    Bind,
}

impl FormulaBindPreviewDiagnosticStage {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Bind => "bind",
        }
    }
}

impl fmt::Display for FormulaBindPreviewDiagnosticStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaBindPreviewDiagnosticProjection {
    pub stage: FormulaBindPreviewDiagnosticStage,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaBindPreviewProfileViolationKindProjection {
    FunctionUnavailable {
        function_id: String,
        function_name: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaBindPreviewProfileViolationProjection {
    pub kind: FormulaBindPreviewProfileViolationKindProjection,
    pub feature: String,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaBindPreviewProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub input_kind: FormulaBindPreviewInputKind,
    pub legal: bool,
    pub diagnostics: Vec<FormulaBindPreviewDiagnosticProjection>,
    pub profile_violations: Vec<FormulaBindPreviewProfileViolationProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableFormulaBindPreviewProjection {
    pub table: NodeId,
    pub table_key: NodeKey,
    pub table_id: String,
    pub column_id: String,
    pub region: TableCellRegionProjection,
    pub input_kind: FormulaBindPreviewInputKind,
    pub legal: bool,
    pub diagnostics: Vec<FormulaBindPreviewDiagnosticProjection>,
    pub profile_violations: Vec<FormulaBindPreviewProfileViolationProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationImpactIntentProjection {
    AddNode {
        parent: Option<NodeId>,
        symbol: String,
        initial: InitialNodeContentProjection,
        is_meta: bool,
    },
    EditContent {
        node: NodeId,
        content: String,
    },
    EditScopedContent {
        scope: AuthoringScope,
        content: String,
    },
    RenameNode {
        node: NodeId,
        new_symbol: String,
    },
    MoveNode {
        node: NodeId,
        new_parent: Option<NodeId>,
        new_index: Option<usize>,
    },
    DeleteNode {
        node: NodeId,
    },
    AddTableFormulaColumn {
        table: NodeId,
        column_id: String,
        name: String,
        formula_text: String,
    },
    AddTableRow {
        table: NodeId,
        row_id: String,
        values: Vec<TableCellInput>,
    },
    DeleteTableRow {
        table: NodeId,
        row_id: String,
    },
    RenameTableRow {
        table: NodeId,
        row_id: String,
        new_row_id: String,
    },
    ReorderTableRow {
        table: NodeId,
        row_id: String,
        new_index: usize,
    },
    AddTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
        values: Vec<TableRowInput>,
    },
    DeleteTableColumn {
        table: NodeId,
        column_id: String,
    },
    RenameTableColumn {
        table: NodeId,
        column_id: String,
        name: String,
    },
    ReorderTableColumn {
        table: NodeId,
        column_id: String,
        new_index: usize,
    },
}

impl MutationImpactIntentProjection {
    #[must_use]
    pub const fn stable_id(&self) -> &'static str {
        match self {
            Self::AddNode { .. } => "add_node",
            Self::EditContent { .. } => "edit_content",
            Self::EditScopedContent { .. } => "edit_scoped_content",
            Self::RenameNode { .. } => "rename_node",
            Self::MoveNode { .. } => "move_node",
            Self::DeleteNode { .. } => "delete_node",
            Self::AddTableFormulaColumn { .. } => "add_table_formula_column",
            Self::AddTableRow { .. } => "add_table_row",
            Self::DeleteTableRow { .. } => "delete_table_row",
            Self::RenameTableRow { .. } => "rename_table_row",
            Self::ReorderTableRow { .. } => "reorder_table_row",
            Self::AddTableColumn { .. } => "add_table_column",
            Self::DeleteTableColumn { .. } => "delete_table_column",
            Self::RenameTableColumn { .. } => "rename_table_column",
            Self::ReorderTableColumn { .. } => "reorder_table_column",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationImpactBlockedReasonProjection {
    SyntaxDiagnostics,
    BindDiagnostics,
    ProfileViolation,
    NameCollision,
    InvalidDrop,
    UnsupportedInitialContent,
    TableCollision,
    DuplicateInput,
}

impl MutationImpactBlockedReasonProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SyntaxDiagnostics => "syntax_diagnostics",
            Self::BindDiagnostics => "bind_diagnostics",
            Self::ProfileViolation => "profile_violation",
            Self::NameCollision => "name_collision",
            Self::InvalidDrop => "invalid_drop",
            Self::UnsupportedInitialContent => "unsupported_initial_content",
            Self::TableCollision => "table_collision",
            Self::DuplicateInput => "duplicate_input",
        }
    }
}

impl fmt::Display for MutationImpactBlockedReasonProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameCollisionProjection {
    pub attempted: String,
    pub existing: NodeId,
    pub existing_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationImpactProjection {
    pub intent: MutationImpactIntentProjection,
    pub legal: bool,
    pub blocked_reason: Option<MutationImpactBlockedReasonProjection>,
    pub profile_violations: Vec<FormulaBindPreviewProfileViolationProjection>,
    pub bind_diagnostics: Vec<FormulaBindPreviewDiagnosticProjection>,
    pub invalidation_plan: RecalcPlanProjection,
    pub requires_rebind: Vec<NodeId>,
    pub affected_refs: Vec<NodeId>,
    pub orphaned_dependents: Vec<NodeId>,
    pub collisions: Vec<NameCollisionProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEffectProjection {
    pub kind: String,
    pub family: RuntimeEffectFamilyProjection,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeEffectFamilyProjection {
    DynamicDependency,
    ExecutionRestriction,
    CapabilitySensitive,
    ShapeTopology,
}

impl RuntimeEffectFamilyProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::DynamicDependency => "dynamic_dependency",
            Self::ExecutionRestriction => "execution_restriction",
            Self::CapabilitySensitive => "capability_sensitive",
            Self::ShapeTopology => "shape_topology",
        }
    }
}

impl fmt::Display for RuntimeEffectFamilyProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverlayProjection {
    pub owner: NodeId,
    pub owner_key: NodeKey,
    pub kind: RuntimeOverlayKindProjection,
    pub structural_snapshot_id: String,
    pub compatibility_basis: String,
    pub payload_identity: Option<String>,
    pub is_protected: bool,
    pub is_eviction_eligible: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeOverlayKindProjection {
    InvalidationExecutionState,
    DynamicDependency,
    ExecutionRestriction,
    ShapeTopology,
    CapabilityFenceAttachment,
    ObserverPriorityMetadata,
}

impl RuntimeOverlayKindProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::InvalidationExecutionState => "invalidation_execution_state",
            Self::DynamicDependency => "dynamic_dependency",
            Self::ExecutionRestriction => "execution_restriction",
            Self::ShapeTopology => "shape_topology",
            Self::CapabilityFenceAttachment => "capability_fence_attachment",
            Self::ObserverPriorityMetadata => "observer_priority_metadata",
        }
    }
}

impl fmt::Display for RuntimeOverlayKindProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationTraceProjection {
    pub trace_schema_id: String,
    pub owner: NodeId,
    pub owner_key: NodeKey,
    pub formula_artifact_id: String,
    pub bind_artifact_id: Option<String>,
    pub formula_stable_id: String,
    pub trace_mode: String,
    pub template_selection: DerivationTemplateSelectionProjection,
    pub hole_bindings: Vec<DerivationHoleBindingProjection>,
    pub sub_invocation_tree: Vec<DerivationInvocationProjection>,
    pub kernel_returned_value: String,
    pub kernel_returned_value_typed: Option<NodeValueProjection>,
    pub oxfml_trace_events: Vec<DerivationOxfmlTraceEventProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationTemplateSelectionProjection {
    pub prepared_formula_key: String,
    pub shape_key: String,
    pub dispatch_skeleton_key: String,
    pub plan_template_key: String,
    pub template_holes: Vec<DerivationTemplateHoleProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationTemplateHoleProjection {
    pub hole_id: String,
    pub ordinal: usize,
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationHoleBindingProjection {
    pub hole_id: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationInvocationProjection {
    pub invocation_ordinal: usize,
    pub invocation_kind: String,
    pub function_name: String,
    pub function_id: String,
    pub arg_preparation_profile: Option<String>,
    pub prepared_arguments: Vec<DerivationPreparedArgumentProjection>,
    pub kernel_returned_value: Option<String>,
    pub kernel_returned_value_typed: Option<NodeValueProjection>,
    pub children: Vec<DerivationInvocationProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationPreparedArgumentProjection {
    pub ordinal: usize,
    pub structure_class: String,
    pub source_class: String,
    pub evaluation_mode: String,
    pub blankness_class: String,
    pub caller_context_sensitive: bool,
    pub reference_target: Option<String>,
    pub opaque_reason: Option<String>,
    pub resolved_value: Option<String>,
    pub resolved_value_typed: Option<NodeValueProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationOxfmlTraceEventProjection {
    pub trace_schema_id: String,
    pub event_kind: String,
    pub formula_stable_id: String,
    pub session_id: Option<String>,
    pub candidate_result_id: Option<String>,
    pub commit_attempt_id: Option<String>,
    pub event_order_key: u64,
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

impl NodeCalcStateProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::DirtyPending => "dirty_pending",
            Self::Needed => "needed",
            Self::Evaluating => "evaluating",
            Self::VerifiedClean => "verified_clean",
            Self::PublishReady => "publish_ready",
            Self::RejectedPendingRepair => "rejected_pending_repair",
            Self::CycleBlocked => "cycle_blocked",
        }
    }
}

impl fmt::Display for NodeCalcStateProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
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
    pub descriptors_by_owner_key: BTreeMap<NodeKey, Vec<DependencyDescriptorProjection>>,
    pub edges_by_owner_key: BTreeMap<NodeKey, Vec<DependencyEdgeProjection>>,
    pub reverse_edges_by_key: BTreeMap<NodeKey, Vec<DependencyEdgeProjection>>,
    pub descriptors_by_owner: BTreeMap<NodeId, Vec<DependencyDescriptorProjection>>,
    pub edges_by_owner: BTreeMap<NodeId, Vec<DependencyEdgeProjection>>,
    pub reverse_edges: BTreeMap<NodeId, Vec<DependencyEdgeProjection>>,
    pub reference_resolutions: BTreeMap<String, ReferenceResolutionProjection>,
    pub reverse_references: BTreeMap<NodeKey, Vec<String>>,
    pub cycle_group_keys: Vec<Vec<NodeKey>>,
    pub cycle_groups: Vec<Vec<NodeId>>,
    pub diagnostics: Vec<String>,
}

impl DependencyGraphProjection {
    #[must_use]
    pub fn outgoing_count_by_key(&self, owner: &NodeKey) -> usize {
        self.edges_by_owner_key.get(owner).map_or(0, Vec::len)
    }

    #[must_use]
    pub fn incoming_count_by_key(&self, target: &NodeKey) -> usize {
        self.reverse_edges_by_key.get(target).map_or(0, Vec::len)
    }

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
    pub target_key: Option<NodeKey>,
    pub workspace_target: Option<String>,
    pub kind: DependencyKindProjection,
    pub carrier_detail: String,
    pub collection: Option<TreeReferenceCollectionProjection>,
    pub requires_rebind_on_structural_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceResolutionProjection {
    pub source_reference_handle: String,
    pub owner: NodeId,
    pub owner_key: NodeKey,
    pub descriptor_ids: Vec<String>,
    pub token_span: Option<SourceSpanProjection>,
    pub target: ReferenceTargetProjection,
    pub primary_kind: DependencyKindProjection,
    pub requires_rebind_on_structural_change: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpanProjection {
    pub start_utf8: usize,
    pub end_utf8: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceTargetProjection {
    Node {
        node: NodeId,
        key: NodeKey,
    },
    Collection {
        collection: TreeReferenceCollectionProjection,
        member_keys: Vec<NodeKey>,
    },
    External {
        target: String,
    },
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdgeProjection {
    pub edge_id: String,
    pub descriptor_id: String,
    pub owner: NodeId,
    pub owner_key: NodeKey,
    pub target: NodeId,
    pub target_key: NodeKey,
    pub kind: DependencyKindProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeReferenceCollectionProjection {
    pub family: TreeReferenceCollectionFamilyProjection,
    pub source_reference_handle: String,
    pub base_node: Option<NodeId>,
    pub base_node_key: Option<NodeKey>,
    pub membership_version: String,
    pub order_version: String,
    pub members: Vec<NodeId>,
    pub member_keys: Vec<NodeKey>,
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
    pub virtual_anchor: TableAnchorProjection,
    pub rows: Vec<TableRowProjection>,
    pub columns: Vec<TableColumnProjection>,
    pub cells: Option<TableCellsProjection>,
    pub row_count: usize,
    pub column_count: usize,
    pub header_row_present: bool,
    pub totals_row_present: bool,
    pub table_namespace_version: String,
    pub row_membership_version: String,
    pub row_order_version: String,
    pub column_identity_version: String,
    pub dependency_inventory: Vec<TableDependencyFactProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableAnchorProjection {
    pub workbook_scope_ref: String,
    pub sheet_scope_ref: String,
    pub start_row: u32,
    pub start_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDependencyFactProjection {
    pub fact_id: String,
    pub kind: TableDependencyFactKindProjection,
    pub status: TableDependencyFactStatusProjection,
    pub table_id: Option<String>,
    pub column_id: Option<String>,
    pub identity: Option<String>,
    pub blocker: Option<TableDependencyFactBlockerProjection>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TableDependencyFactKindProjection {
    TableIdentity,
    RowMembership,
    RowOrder,
    RowValue,
    ColumnIdentity,
    ColumnOrder,
    HeaderText,
    HeaderRegion,
    DataRegion,
    TotalsRegion,
    TotalsValue,
    TotalsFormula,
    CallerRowContext,
    OmittedTableNameEnclosingTable,
    VirtualAnchorRange,
    WorkspaceAvailability,
    FunctionRegistrySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableDependencyFactStatusProjection {
    Lowered,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TableDependencyFactBlockerProjection {
    MissingTableCatalogEntry,
    MissingEnclosingTableContext,
    MissingStableRowMembershipAndOrderPacket,
    MissingSelectedColumn,
    MissingHeaderRegionRange,
    MissingTotalsRegionRange,
    HeaderRowAbsent,
    TotalsRowAbsent,
    MissingCallerTableRegion,
    CallerTableMismatch,
    CallerRegionNotData,
    CallerDataRowOffsetMissing,
    OmittedTableEnclosingMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellsProjection {
    pub body_rows: Vec<Vec<Option<TableCellProjection>>>,
    pub totals_row: Vec<Option<TableCellProjection>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellProjection {
    pub row_id: Option<String>,
    pub column_id: String,
    pub node_key: NodeKey,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRowProjection {
    pub row_id: String,
    pub ordinal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumnProjection {
    pub column_id: String,
    pub name: String,
    pub ordinal: u32,
    pub body: TableColumnBodyProjection,
    pub totals_formula: Option<TableFormulaMetadataProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableColumnBodyProjection {
    ConstantCells,
    Formula(TableFormulaMetadataProjection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableFormulaMetadataProjection {
    pub formula_artifact_id: String,
    pub bind_artifact_id: Option<String>,
    pub formula_text_version: String,
    pub formula_text: String,
}

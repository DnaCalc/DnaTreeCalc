use std::collections::{BTreeMap, BTreeSet};

use std::fmt;

use crate::identity::{NodeId, NodeKey};
use crate::intent::{AuthoringScope, TableCellInput, TableRowInput};
use crate::selection::SelectionState;
use serde::{Deserialize, Serialize};

/// Read-side projection of the workspace, as seen by a mounted skin.
///
/// The host owns the UI projection while OxCalc owns the canonical model; this struct is what
/// the host publishes through the `SkinContext::workspace`
/// signal so skins can render without knowing the OxCalc context or the
/// persistence format. Mirrors the spec shape in `docs/ux/SKINS.md` §2.7,
/// narrowed for the walking skeleton — meta-namespaces, templates, formats,
/// and cross-workspace aliases land as later worksets extend the projection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub workspace_id: String,
    pub profile: String,
    pub projection_seq: u64,
    pub revision: WorkspaceRevisionProjection,
    pub revision_history: RevisionHistoryProjection,
    pub candidates: Vec<CandidateProjection>,
    pub speculation_pressure: SpeculationPressureProjection,
    pub scenarios: ScenarioManifestProjection,
    pub sweeps: SweepManifestProjection,
    pub templates: TemplateManifestProjection,
    pub comparison: ComparativeProjection,
    pub series: SeriesManifestProjection,
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
    /// Windowed grid projections, keyed by the grid-backed sheet node. Only grids
    /// the client is interested in are present, and each carries only the cells
    /// inside the registered interest window ("viewing is subscribing").
    pub grids: BTreeMap<NodeId, GridProjection>,
    /// The workbook's complete defined-name catalog (H4, §A.3). Default-empty
    /// for a pre-H4 mirror or a `RichTree` session that carries no names.
    #[serde(default)]
    pub defined_names: DefinedNamesProjection,
    pub clipboard: Option<ClipboardProjection>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeculationPressureProjection {
    pub retained_candidate_count: usize,
    pub child_protected_candidate_count: usize,
    pub host_pinned_candidate_count: usize,
    pub protected_candidate_count: usize,
    pub reclaimable_candidate_count: usize,
    pub over_budget_candidate_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioManifestProjection {
    pub active: Option<String>,
    pub entries: Vec<ScenarioProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioProjection {
    pub id: String,
    pub name: String,
    pub source: ScenarioSourceProjection,
    pub override_count: usize,
    pub overridden_nodes: Vec<NodeKey>,
    pub override_values: BTreeMap<NodeKey, NodeValueProjection>,
    pub value_epoch: Option<u64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioSourceProjection {
    Candidate { handle: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepManifestProjection {
    pub active: Option<String>,
    pub entries: Vec<SweepProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepProjection {
    pub id: String,
    pub name: String,
    pub input_node: NodeKey,
    pub base_scenario_id: Option<String>,
    pub points: Vec<SweepPointProjection>,
    pub value_epoch: Option<u64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepPointProjection {
    pub id: String,
    pub label: String,
    pub input_value: NodeValueProjection,
    pub scenario_id: String,
    pub value_epoch: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeProjection {
    pub basis: ComparativeColumnProjection,
    pub columns: Vec<ComparativeColumnProjection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeColumnProjection {
    pub label: String,
    pub source: ComparativeSourceProjection,
    pub value_epoch: Option<u64>,
    pub values: BTreeMap<NodeKey, NodeValueProjection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparativeSourceProjection {
    #[default]
    Published,
    Candidate {
        handle: String,
    },
    Scenario {
        id: String,
    },
    SweepPoint {
        sweep_id: String,
        point_id: String,
        scenario_id: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesManifestProjection {
    pub entries: Vec<SeriesProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesProjection {
    pub id: String,
    pub label: String,
    pub source: ComparativeSourceProjection,
    pub unit: Option<String>,
    pub points: Vec<SeriesPointProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesPointProjection {
    pub key: NodeKey,
    pub label: String,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateManifestProjection {
    pub entries: Vec<TemplateProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateProjection {
    pub template_id: String,
    pub name: String,
    pub description: Option<String>,
    pub initial: InitialNodeContentProjection,
    pub preview_content: Option<String>,
    pub built_in: bool,
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

    /// Classify a node by key into its derived [`NodeClassification`]
    /// (canonical contract **C2**). Pure derivation from the node's
    /// `content_kind` and its incoming dependency-edge count — nothing is
    /// stored. An unknown key classifies as [`NodeClassification::Empty`].
    #[must_use]
    pub fn node_classification(&self, key: &NodeKey) -> NodeClassification {
        let Some(node) = self.node_by_key(key) else {
            return NodeClassification::Empty;
        };
        let consumed = self.dependencies.incoming_count_by_key(key) > 0;
        match node.content_kind {
            NodeContentKind::Empty => NodeClassification::Empty,
            NodeContentKind::Constant => {
                if consumed {
                    NodeClassification::Input
                } else {
                    NodeClassification::FreeValue
                }
            }
            NodeContentKind::Formula => {
                if consumed {
                    NodeClassification::Intermediate
                } else {
                    NodeClassification::Output
                }
            }
        }
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
            attributes: node.attributes.clone(),
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

    #[must_use]
    pub fn series_for_keys(&self, keys: &[NodeKey]) -> SeriesManifestProjection {
        series_manifest_for_keys(
            &self.comparison,
            keys,
            &self.paths_by_key,
            &self.nodes_by_key,
        )
    }

    pub fn series_for_scope(
        &self,
        scope: &AuthoringScope,
    ) -> Result<SeriesManifestProjection, AuthoringScopeExpansionError> {
        let keys = self.expand_authoring_scope(scope)?;
        Ok(self.series_for_keys(&keys))
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

fn series_manifest_for_keys(
    comparison: &ComparativeProjection,
    keys: &[NodeKey],
    paths_by_key: &BTreeMap<NodeKey, NodeId>,
    nodes_by_key: &BTreeMap<NodeKey, NodeView>,
) -> SeriesManifestProjection {
    let mut entries = Vec::with_capacity(1 + comparison.columns.len());
    entries.push(series_projection_from_column(
        "series:published".to_string(),
        &comparison.basis,
        keys,
        paths_by_key,
        nodes_by_key,
    ));
    entries.extend(comparison.columns.iter().map(|column| {
        series_projection_from_column(
            series_id_for_comparative_source(&column.source),
            column,
            keys,
            paths_by_key,
            nodes_by_key,
        )
    }));
    SeriesManifestProjection { entries }
}

fn series_projection_from_column(
    id: String,
    column: &ComparativeColumnProjection,
    keys: &[NodeKey],
    paths_by_key: &BTreeMap<NodeKey, NodeId>,
    nodes_by_key: &BTreeMap<NodeKey, NodeView>,
) -> SeriesProjection {
    let points: Vec<SeriesPointProjection> = keys
        .iter()
        .filter_map(|key| {
            let value = column.values.get(key)?;
            Some(SeriesPointProjection {
                key: key.clone(),
                label: paths_by_key
                    .get(key)
                    .map_or_else(|| key.to_string(), |path| path.as_str().to_string()),
                value: value.clone(),
            })
        })
        .collect();
    SeriesProjection {
        id,
        label: column.label.clone(),
        source: column.source.clone(),
        unit: if points.is_empty() {
            None
        } else {
            common_series_unit(keys, nodes_by_key)
        },
        points,
    }
}

fn series_id_for_comparative_source(source: &ComparativeSourceProjection) -> String {
    match source {
        ComparativeSourceProjection::Published => "series:published".to_string(),
        ComparativeSourceProjection::Candidate { handle } => format!("series:candidate:{handle}"),
        ComparativeSourceProjection::Scenario { id } => format!("series:scenario:{id}"),
        ComparativeSourceProjection::SweepPoint {
            sweep_id, point_id, ..
        } => {
            format!("series:sweep:{sweep_id}:{point_id}")
        }
    }
}

fn common_series_unit(
    keys: &[NodeKey],
    nodes_by_key: &BTreeMap<NodeKey, NodeView>,
) -> Option<String> {
    if keys.is_empty() {
        return None;
    }
    let mut common = None::<String>;
    for key in keys {
        let node = nodes_by_key.get(key)?;
        let unit = node
            .attributes
            .get("series_unit")
            .or_else(|| node.attributes.get("unit"))
            .map(|unit| unit.trim())
            .filter(|unit| !unit.is_empty())?;
        match &common {
            Some(existing) if existing != unit => return None,
            Some(_) => {}
            None => common = Some(unit.to_string()),
        }
    }
    common
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub attributes: BTreeMap<String, String>,
    pub binding_diagnostics: Vec<BindingDiagnosticProjection>,
    pub outgoing_references: Vec<ReferenceResolutionProjection>,
    pub incoming_reference_handles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardProjection {
    pub operation: ClipboardOperationProjection,
    pub payload: ClipboardPayloadProjection,
    /// Plain-text representation suitable for platform clipboard export.
    /// The host prepares this; skins/platform code perform the actual OS
    /// clipboard write.
    pub plain_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardOperationProjection {
    Copy,
    Cut,
}

impl ClipboardOperationProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Cut => "cut",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardPayloadProjection {
    Values {
        nodes: Vec<ClipboardNodeValueProjection>,
    },
    Formula {
        source: NodeKey,
        source_path: NodeId,
        content: String,
    },
    Format {
        nodes: Vec<ClipboardNodeFormatProjection>,
    },
    Subtree {
        root: NodeKey,
        root_path: NodeId,
        nodes: Vec<NodeKey>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardNodeValueProjection {
    pub node: NodeKey,
    pub path: NodeId,
    pub content_kind: NodeContentKind,
    /// Authored input text that can be pasted back without converting a
    /// rendered value into source text. Present only for constant inputs.
    pub constant_input_text: Option<String>,
    /// OxFml-authored literal text for supported computed values.
    /// Consumers may paste this only as a value-literal operation, not as a
    /// replacement for formula rewrite or subtree rebind semantics.
    pub literalized_input_text: Option<String>,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardNodeFormatProjection {
    pub node: NodeKey,
    pub path: NodeId,
    pub effective_format: Option<EffectiveFormatProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// A windowed projection of a grid-backed sheet node's computed cells, scoped to
/// the client's registered interest region(s). The host fills it from OxCalc's
/// `grid_view`; a huge grid projects only the cells inside the viewed window, so
/// the projection stays bounded regardless of the sheet's extent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridProjection {
    /// The grid-backed sheet node (stable engine identity).
    pub grid_node_key: NodeKey,
    /// The grid-backed sheet node (display path); the key into `WorkspaceState.grids`.
    pub grid_node_id: NodeId,
    /// Stable grid id from the engine backing.
    pub grid_id: String,
    /// Grid bounds (the Excel sheet extent), so a viewer can size scrollbars
    /// without materializing cells.
    pub max_rows: u32,
    pub max_cols: u32,
    /// The windowed computed cells — only those inside the registered interest.
    pub cells: Vec<GridCellProjection>,
    /// The recalc epoch this projection reflects (the max cell value epoch); a
    /// client pulls "changes since" by comparing against it.
    pub projection_epoch: u64,
    /// Read-only overlay descriptors (tables, spills, merged regions) clipped to
    /// the viewed window. A forward-compatible bundle: a new family is a new
    /// field that older mirrors ignore.
    #[serde(default, skip_serializing_if = "GridOverlayBundle::is_empty")]
    pub overlays: GridOverlayBundle,
    /// Bumped whenever the window-clipped overlay set changes, independently of
    /// the cell value epochs.
    #[serde(default)]
    pub overlay_epoch: u64,
    /// True when the permanent-pair differential found no mismatch for this view
    /// (the optimized and reference grid engines agreed).
    pub differential_clean: bool,
    /// Bumped whenever the windowed authored layer (`GridCellProjection::authored`)
    /// changes, independently of the computed-value epochs — a client pulls
    /// "authored changes since" by comparing against it (H3, §A.3). `0` for a
    /// pre-H3 mirror or wire payload that never carried authored fields.
    #[serde(default)]
    pub authored_epoch: u64,
}

/// The window-clipped, read-only overlay descriptors for a [`GridProjection`].
/// A forward-compatible bundle of named vectors: a new overlay family is added
/// as a new `#[serde(default)]` field that older mirrors deserialize as empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridOverlayBundle {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<GridTableOverlayDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spills: Vec<GridSpillOverlayDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged: Vec<GridMergedOverlayDescriptor>,
}

impl GridOverlayBundle {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty() && self.spills.is_empty() && self.merged.is_empty()
    }
}

/// An overlay rectangle in absolute 1-based grid coordinates, clipped to the
/// viewed window. Each `clipped_*` flag records whether that edge was cut by the
/// window (so a renderer can draw a "continues beyond the window" affordance
/// rather than a hard border).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridOverlayRect {
    pub top_row: u32,
    pub left_col: u32,
    pub bottom_row: u32,
    pub right_col: u32,
    pub clipped_top: bool,
    pub clipped_left: bool,
    pub clipped_bottom: bool,
    pub clipped_right: bool,
}

/// One column band of a table overlay, clipped to the viewed window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridTableColumnBand {
    pub column_id: String,
    pub column_name: String,
    pub ordinal: u32,
    pub data_rect: GridOverlayRect,
}

/// A structured-table overlay descriptor (geometry + identity), clipped to the
/// viewed window. When the grid table maps to a shared workspace table node, it
/// points at that canonical [`TableProjection`] by key (`table_node_key`) rather
/// than duplicating its rows; `None` when no such linkage exists yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridTableOverlayDescriptor {
    pub table_id: String,
    pub table_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_node_key: Option<NodeKey>,
    pub table_range: GridOverlayRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_rect: Option<GridOverlayRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub totals_rect: Option<GridOverlayRect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<GridTableColumnBand>,
}

/// A spilled-array overlay descriptor: the (unclipped) anchor cell that produced
/// the spill, the window-clipped extent, and whether the spill is blocked
/// (`#SPILL!`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSpillOverlayDescriptor {
    pub anchor_row: u32,
    pub anchor_col: u32,
    pub extent: GridOverlayRect,
    pub blocked: bool,
}

/// A merged-region overlay descriptor, clipped to the viewed window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridMergedOverlayDescriptor {
    pub rect: GridOverlayRect,
}

/// One computed grid cell in a windowed [`GridProjection`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridCellProjection {
    pub row: u32,
    pub col: u32,
    pub value: NodeValueProjection,
    /// The recalc epoch at which this cell's value last changed.
    pub value_epoch: u64,
    /// Authored-metadata mirror of OxCalc's `GridAuthoredCellReadout`
    /// (`grid/authored.rs:505`) for this cell — kind, source text, and
    /// editability, as opposed to `value` (the computed readout). `None` for
    /// a pre-H3 mirror or wire payload that never carried authored fields;
    /// `#[serde(default)]` so an older serialized `GridProjection` still
    /// deserializes cleanly (the `GridOverlayBundle` forward-compatibility
    /// pattern, `workspace.rs:822` at H3 authoring time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored: Option<GridAuthoredCellProjection>,
}

/// Authored-metadata mirror of one grid cell (W062 R5.5 `grid_authored_view`,
/// `grid/authored.rs:505`): the authored kind, source text, and editability
/// classification — never a computed value (that is `GridCellProjection::value`).
///
/// Carries its own `row`/`col` so it doubles as the element type of
/// [`WorkspaceDeltaChange::GridAuthoredChanged`]'s flat cell list (§A.3); when
/// nested inside a [`GridCellProjection`] the coordinates are redundant with
/// the parent's own `row`/`col` (both describe the same cell) but are kept for
/// a single reusable wire shape rather than a second, delta-only type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridAuthoredCellProjection {
    pub row: u32,
    pub col: u32,
    pub kind: GridAuthoredKindProjection,
    /// The literal's display text for a `Literal` cell; `None` otherwise.
    pub literal_text: Option<String>,
    /// The formula display text for a `Formula` cell (e.g. `"=A1*3"`); `None`
    /// otherwise. Never a computed value.
    pub source_text: Option<String>,
    pub editability: GridEditabilityProjection,
}

/// Mirror of OxCalc's `GridAuthoredKind` (`grid/authored.rs:389`): what, if
/// anything, is authored at a grid address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridAuthoredKindProjection {
    /// No authored record at this address.
    Empty,
    /// A typed literal value.
    Literal,
    /// A formula cell; the projection carries its source text.
    Formula,
    /// An inert rich-object stub retained from ingest (D4 §12).
    RichStub,
}

/// A projected grid address (row/col only — skins never see engine addresses
/// or workbook/sheet ids, per §A.2), used as the `anchor` payload of the
/// non-`Editable` [`GridEditabilityProjection`] variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridCellRefProjection {
    pub row: u32,
    pub col: u32,
}

/// Mirror of OxCalc's `GridCellEditability` (`grid/authored.rs:419`), the
/// single classifier shared by the authored readout and the entry verbs'
/// enforcement. Only `Editable` admits a write; every other variant is a typed
/// rejection at entry time (H6) and a read-only affordance in a skin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridEditabilityProjection {
    /// A plain cell: editing is admitted by every entry verb.
    Editable,
    /// A member (non-anchor) of a repeated-formula region.
    RepeatedRegionMember { anchor: GridCellRefProjection },
    /// A follower (non-anchor) cell of a merged region.
    MergedFollower { anchor: GridCellRefProjection },
    /// A cell displaying a value spilled from a neighbouring array formula.
    SpillDisplay { anchor: GridCellRefProjection },
    /// A structural (header / totals) cell of a structured table.
    TableStructural { table_id: String },
}

/// The three-way outcome of a universal cell-entry write (H6, §A.2/§A.3),
/// mirroring OxCalc's `GridCellEntryOutcome` (`consumer.rs`) verbatim: a
/// successful `enter_grid_cell`/`clear_grid_cell` call resolves to exactly one
/// of these — never a fourth case — matching the engine's own three-way
/// contract. The typed rejection half of the entry verb travels as
/// `IntentError::GridEntryRejected` (§A.4), not as a variant here, so this
/// projection carries only the success arms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridEntryOutcomeProjection {
    /// The entered text classified as a literal value; the projection carries
    /// the resulting cell's rendered value.
    Literal { value: NodeValueProjection },
    /// The entered text classified as a formula. `unresolved_names` are the
    /// defined names the bound formula references that are not yet resolved
    /// on this sheet — a first-class success field (OxCalc W062 R5.9,
    /// `calc-5kqg.55`), not a rejection: each evaluates `#NAME?` until seeded,
    /// then self-heals (§B.3's edit-commit loop, step 3).
    Formula {
        unresolved_names: Vec<String>,
        value: NodeValueProjection,
    },
    /// The entered text was empty (Excel's empty-commit-clears contract) or
    /// the cell was cleared directly via `ClearGridCell`.
    Cleared,
}

/// One rejection diagnostic on a cell-entry write, mirroring OxCalc's
/// `EntryRejectionDiagnostic` (`grid/error.rs`, W062 R5.9) verbatim:
/// `message` plus an *optional* UTF-8 byte span. `span` is `None` when OxFml
/// has no span to offer for this diagnostic — the UI (§A.4) renders that row
/// message-only, without a highlight, rather than fabricating a span.
///
/// Deliberately **not** [`SourceSpanProjection`] (§A.4's explicit decision):
/// rejection diagnostics and bind-preview diagnostics are distinct engine
/// types that arrive on different receipts and are not unified by R5.9; they
/// share only the rendering component, not the wire type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridEntryDiagnosticProjection {
    pub message: String,
    pub span: Option<(u32, u32)>,
}

/// The complete defined-name catalog for a workbook session (H4, §A.3): the
/// skin-IR mirror of OxCalc's `document_defined_names` readout-all verb
/// (`consumer.rs`). `WorkspaceState::defined_names` and this projection's own
/// `WorkspaceDeltaChange::DefinedNamesChanged` payload share this one type —
/// the whole catalog is small (one workbook, not a windowed grid), so it is a
/// complete-replacement patch every time, matching the doctrine documented at
/// `session_channel.rs:145-158`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedNamesProjection {
    pub entries: Vec<DefinedNameProjection>,
}

/// One defined name, mirroring OxCalc's `DefinedNameReadout` (`consumer.rs`)
/// verbatim: scope, name text, target (a static rect or a dynamic formula),
/// and whether it is dynamic. Never carries a computed value — a dynamic
/// name's current value is read the same way any other formula's is (through
/// the grid/value projection), not duplicated here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedNameProjection {
    pub scope: DefinedNameScopeProjection,
    pub name: String,
    pub target: DefinedNameTargetProjection,
    pub is_dynamic: bool,
}

/// Mirror of OxCalc's `OxCalcTreeDefinedNameScope`: a name is scoped to the
/// whole workbook or to one sheet. `Sheet` carries the sheet's stable grid
/// [`NodeId`] (the same `sheet:{node}` address `EnterGridCell`/`ClearGridCell`
/// already use, §A.2) — never the raw engine sheet-id string — so skins never
/// see engine addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefinedNameScopeProjection {
    Workbook,
    Sheet(NodeId),
}

/// Mirror of OxCalc's `DefinedNameTargetReadout`: a defined name resolves
/// either to a fixed grid rectangle (`Static`) or to a formula the engine
/// re-evaluates on lookup (`Dynamic`). `Dynamic` carries only the authored
/// source text — H4's NON-goals explicitly exclude any dynamic-name UI
/// semantics beyond this passthrough (no live-value preview, no channel
/// detail).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefinedNameTargetProjection {
    Static(GridRectProjection),
    Dynamic { source_text: String },
}

/// A defined name's static target rectangle, in absolute 1-based grid
/// coordinates. Deliberately distinct from [`GridOverlayRect`]: that type's
/// `clipped_*` flags describe a window-clipped *overlay* geometry, which a
/// name's target (never windowed — the whole catalog is a complete-replacement
/// patch, §A.3) does not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridRectProjection {
    pub top_row: u32,
    pub left_col: u32,
    pub bottom_row: u32,
    pub right_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub scenario_override: Option<NodeValueProjection>,
    pub literalized_value_input: Option<String>,
    pub value_epoch: Option<u64>,
    pub calc_state: Option<NodeCalcStateProjection>,
    pub effective_format: Option<EffectiveFormatProjection>,
    pub note: Option<NodeNoteProjection>,
    pub attributes: BTreeMap<String, String>,
    pub binding_diagnostics: Vec<BindingDiagnosticProjection>,
    pub is_meta: bool,
    pub table: Option<TableProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeNoteProjection {
    pub text: String,
    pub source: NoteSourceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSourceProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveFormatProjection {
    pub number_format_code: Option<String>,
    pub inherited_from: Option<FormatSourceProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatSourceProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingDiagnosticProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Derived role of a node, computed from its [`NodeContentKind`] and the
/// dependency graph — never stored (canonical contract **C2**). This is the
/// single classifier; every skin (B1's tint, B3's `list_inputs`/`list_outputs`)
/// calls it, so they can never disagree about what a node is.
///
/// **Axis rule:** content-kind decides literal-vs-computed; the node's
/// *incoming* edge count decides consumed-vs-terminal. The outgoing count is
/// deliberately unused. Spill children are `Formula` non-anchors and classify
/// normally here; their read-only-ness is the spill marker's concern, separate
/// from classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeClassification {
    /// A literal value that other nodes consume — a model input.
    Input,
    /// A literal value nothing consumes — a free/unused constant.
    FreeValue,
    /// A formula other nodes consume — an intermediate result.
    Intermediate,
    /// A formula nothing consumes — a terminal output.
    Output,
    /// No content.
    Empty,
}

impl NodeClassification {
    /// Stable, lower-snake string form for transport (e.g. B3 JSON), mirroring
    /// [`NodeContentKind::stable_id`].
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::FreeValue => "free_value",
            Self::Intermediate => "intermediate",
            Self::Output => "output",
            Self::Empty => "empty",
        }
    }
}

/// What the skin should render for a node's value cell.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
            // A compact summary, never the unrolled cells — inline value cells
            // must stay narrow (a large array otherwise blows out the layout).
            // The 2D grid view (skin `render_value`) shows the actual cells,
            // truncated and width-bounded.
            Self::Array { rows, cols, .. } => format!("{rows}x{cols} array"),
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionHistoryProjection {
    pub current_revision_id: Option<String>,
    pub entries: Vec<RevisionHistoryEntryProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionHistoryEntryProjection {
    pub revision_id: String,
    pub parent_revision_id: Option<String>,
    pub structural_snapshot_id: String,
    pub node_input_snapshot_id: String,
    pub namespace_snapshot_id: String,
    pub transaction_id: Option<String>,
    pub transaction_summary: Option<RevisionTransactionSummaryProjection>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionTransactionSummaryProjection {
    pub transaction_id: String,
    pub invalidated_nodes: Vec<RevisionInvalidationSummaryEntryProjection>,
    pub requires_rebind: Vec<NodeKey>,
    pub estimated_node_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionInvalidationSummaryEntryProjection {
    pub node: NodeKey,
    pub requires_rebind: bool,
    pub reasons: Vec<InvalidationReasonProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateProjection {
    pub handle: String,
    pub basis_revision_id: String,
    pub parent_handle: Option<String>,
    pub retention_pin_count: usize,
    pub workspace_revision_id: String,
    pub revision_history: RevisionHistoryProjection,
    pub nodes: Vec<CandidateNodeProjection>,
    pub values_by_key: BTreeMap<NodeKey, NodeValueProjection>,
    pub run: Option<CalcRunProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateNodeProjection {
    pub key: NodeKey,
    pub id: NodeId,
    pub display_name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub depth: u32,
    pub content_kind: NodeContentKind,
    pub content_text: String,
    pub is_meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalcRunStateProjection {
    Published,
    VerifiedClean,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitialNodeContentProjection {
    Empty,
    Literal { content: String },
    InheritColumnFormula { table: NodeId, column_id: String },
    TemplateBound { template_id: String },
}

impl InitialNodeContentProjection {
    #[must_use]
    pub const fn stable_id(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Literal { .. } => "literal",
            Self::InheritColumnFormula { .. } => "inherit_column_formula",
            Self::TemplateBound { .. } => "template_bound",
        }
    }

    #[must_use]
    pub fn supported_content(&self) -> Option<&str> {
        match self {
            Self::Empty => Some(""),
            Self::Literal { content } => Some(content),
            Self::InheritColumnFormula { .. } | Self::TemplateBound { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalcPlanInvalidationProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub requires_rebind: bool,
    pub reasons: Vec<InvalidationReasonProjection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalcPlanProjection {
    pub invalidated_nodes: Vec<RecalcPlanInvalidationProjection>,
    pub evaluation_order: Vec<NodeId>,
    pub requires_rebind: Vec<NodeId>,
    pub estimated_node_count: usize,
    pub cycle_risk: Vec<Vec<NodeId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaBindPreviewDiagnosticProjection {
    pub stage: FormulaBindPreviewDiagnosticStage,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormulaBindPreviewProfileViolationKindProjection {
    FunctionUnavailable {
        function_id: String,
        function_name: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaBindPreviewProfileViolationProjection {
    pub kind: FormulaBindPreviewProfileViolationKindProjection,
    pub feature: String,
    pub message: String,
    pub span: SourceSpanProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaBindPreviewProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub input_kind: FormulaBindPreviewInputKind,
    pub legal: bool,
    pub diagnostics: Vec<FormulaBindPreviewDiagnosticProjection>,
    pub profile_violations: Vec<FormulaBindPreviewProfileViolationProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameCollisionProjection {
    pub attempted: String,
    pub existing: NodeId,
    pub existing_key: NodeKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEffectProjection {
    pub kind: String,
    pub family: RuntimeEffectFamilyProjection,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationTemplateSelectionProjection {
    pub prepared_formula_key: String,
    pub shape_key: String,
    pub dispatch_skeleton_key: String,
    pub plan_template_key: String,
    pub template_holes: Vec<DerivationTemplateHoleProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationTemplateHoleProjection {
    pub hole_id: String,
    pub ordinal: usize,
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationHoleBindingProjection {
    pub hole_id: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationOxfmlTraceEventProjection {
    pub trace_schema_id: String,
    pub event_kind: String,
    pub formula_stable_id: String,
    pub session_id: Option<String>,
    pub candidate_result_id: Option<String>,
    pub commit_attempt_id: Option<String>,
    pub event_order_key: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Inverse of [`PhaseKeyProjection::stable_id`]; unknown ids project as
    /// [`PhaseKeyProjection::Other`] so newer engines stay readable.
    #[must_use]
    pub fn from_stable_id(value: String) -> Self {
        match value.as_str() {
            "oxfml_prepare_formulas" => Self::OxfmlPrepareFormulas,
            "dependency_descriptor_lowering" => Self::DependencyDescriptorLowering,
            "dependency_descriptor_owner_index" => Self::DependencyDescriptorOwnerIndex,
            "dependency_graph_build_and_cycle_scan" => Self::DependencyGraphBuildAndCycleScan,
            "invalidation_closure_derivation" => Self::InvalidationClosureDerivation,
            "runtime_setup" => Self::RuntimeSetup,
            "diagnostic_seed_collection" => Self::DiagnosticSeedCollection,
            "recalc_tracker_mark_dirty_needed" => Self::RecalcTrackerMarkDirtyNeeded,
            "topological_formula_order" => Self::TopologicalFormulaOrder,
            "rebind_gate_scan" => Self::RebindGateScan,
            "dependency_diagnostic_reject_scan" => Self::DependencyDiagnosticRejectScan,
            "edge_value_cache_lookup" => Self::EdgeValueCacheLookup,
            "oxfml_formula_evaluation" => Self::OxfmlFormulaEvaluation,
            "derivation_trace_record" => Self::DerivationTraceRecord,
            "edge_value_cache_store" => Self::EdgeValueCacheStore,
            "evaluation_loop_total" => Self::EvaluationLoopTotal,
            "verified_clean_finalize" => Self::VerifiedCleanFinalize,
            "candidate_publication" => Self::CandidatePublication,
            "rejection_recording" => Self::RejectionRecording,
            "total_engine_execute" => Self::TotalEngineExecute,
            _ => Self::Other(value),
        }
    }
}

/// Manual serde impls: `PhaseKeyProjection` keys `phase_timings_micros`, and
/// JSON map keys must be plain strings, which rules out the derived enum
/// representation (the `Other(String)` variant would serialize as a
/// single-entry object). The stable-id string is the wire form instead.
impl Serialize for PhaseKeyProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.stable_id())
    }
}

impl<'de> Deserialize<'de> for PhaseKeyProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_stable_id(value))
    }
}

impl fmt::Display for PhaseKeyProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInvalidationProjection {
    pub node: NodeId,
    pub node_key: NodeKey,
    pub calc_state: NodeCalcStateProjection,
    pub requires_rebind: bool,
    pub reasons: Vec<InvalidationReasonProjection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpanProjection {
    pub start_utf8: usize,
    pub end_utf8: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdgeProjection {
    pub edge_id: String,
    pub descriptor_id: String,
    pub owner: NodeId,
    pub owner_key: NodeKey,
    pub target: NodeId,
    pub target_key: NodeKey,
    pub kind: DependencyKindProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableAnchorProjection {
    pub workbook_scope_ref: String,
    pub sheet_scope_ref: String,
    pub start_row: u32,
    pub start_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableDependencyFactStatusProjection {
    Lowered,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCellsProjection {
    pub body_rows: Vec<Vec<Option<TableCellProjection>>>,
    pub totals_row: Vec<Option<TableCellProjection>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCellProjection {
    pub row_id: Option<String>,
    pub column_id: String,
    pub node_key: NodeKey,
    pub value: NodeValueProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRowProjection {
    pub row_id: String,
    pub ordinal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableColumnProjection {
    pub column_id: String,
    pub name: String,
    pub ordinal: u32,
    pub body: TableColumnBodyProjection,
    pub totals_formula: Option<TableFormulaMetadataProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableColumnBodyProjection {
    ConstantCells,
    Formula(TableFormulaMetadataProjection),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableFormulaMetadataProjection {
    pub formula_artifact_id: String,
    pub bind_artifact_id: Option<String>,
    pub formula_text_version: String,
    pub formula_text: String,
}

/// The shared cleave predicate — ATLAS's "filter/sort as continuity".
///
/// It is stored once in shared view-state (`SharedSkinState::cleave`) and each
/// lens re-applies it to its own projection via
/// [`WorkspaceState::cleave_filtered_keys`]. The *predicate* is shared
/// continuity; the materialized result is not (it is recomputed per lens, per
/// the continuity caveat in `docs/ux/skin-suite/README.md`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleavePredicate {
    pub filter: Option<CleaveFilter>,
    pub sort: Option<CleaveSort>,
}

impl CleavePredicate {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.filter.is_none() && self.sort.is_none()
    }
}

/// A typed, lens-agnostic node filter. No prose to grep — every variant reads
/// typed projected truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleaveFilter {
    /// Keep nodes whose calc-state matches.
    CalcState(NodeCalcStateProjection),
    /// Keep nodes whose computed value is an error.
    HasError,
    /// Keep nodes whose name, content, or rendered value contains the text
    /// (case-insensitive).
    TextMatch(String),
    /// Keep nodes with an outgoing dependency edge to the given target.
    DependsOn(NodeKey),
    /// Keep nodes with an outgoing dependency edge of the given kind.
    Kind(DependencyKindProjection),
}

/// A typed, lens-agnostic sort key over projected truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleaveSort {
    NameAsc,
    NameDesc,
    ValueAsc,
    ValueDesc,
    DepthAsc,
    DepthDesc,
}

impl WorkspaceState {
    /// Apply the shared cleave predicate to this projection, returning the
    /// surviving node keys in the requested order. Numeric values sort before
    /// non-numeric ones; unknown nodes are dropped.
    #[must_use]
    pub fn cleave_filtered_keys(&self, predicate: &CleavePredicate) -> Vec<NodeKey> {
        let mut keys: Vec<NodeKey> = self
            .key_order
            .iter()
            .filter(|key| match (&predicate.filter, self.node_by_key(key)) {
                (_, None) => false,
                (None, Some(_)) => true,
                (Some(filter), Some(node)) => self.cleave_filter_matches(filter, node, key),
            })
            .cloned()
            .collect();
        if let Some(sort) = predicate.sort {
            self.cleave_sort_keys(&mut keys, sort);
        }
        keys
    }

    fn cleave_filter_matches(&self, filter: &CleaveFilter, node: &NodeView, key: &NodeKey) -> bool {
        match filter {
            CleaveFilter::CalcState(state) => node.calc_state == Some(*state),
            CleaveFilter::HasError => matches!(node.computed_value, NodeValueProjection::Error(_)),
            CleaveFilter::TextMatch(needle) => {
                let needle = needle.to_lowercase();
                node.display_name.to_lowercase().contains(&needle)
                    || node.content_text.to_lowercase().contains(&needle)
                    || node
                        .computed_value
                        .display_text()
                        .to_lowercase()
                        .contains(&needle)
            }
            CleaveFilter::DependsOn(target) => self
                .dependencies
                .edges_by_owner_key
                .get(key)
                .is_some_and(|edges| edges.iter().any(|edge| &edge.target_key == target)),
            CleaveFilter::Kind(kind) => self
                .dependencies
                .edges_by_owner_key
                .get(key)
                .is_some_and(|edges| edges.iter().any(|edge| edge.kind == *kind)),
        }
    }

    fn cleave_sort_keys(&self, keys: &mut [NodeKey], sort: CleaveSort) {
        keys.sort_by(|a, b| {
            let na = self.node_by_key(a);
            let nb = self.node_by_key(b);
            match sort {
                CleaveSort::NameAsc | CleaveSort::NameDesc => {
                    let ordering = na
                        .map(|n| n.display_name.as_str())
                        .unwrap_or_default()
                        .cmp(nb.map(|n| n.display_name.as_str()).unwrap_or_default());
                    if sort == CleaveSort::NameDesc {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                }
                CleaveSort::DepthAsc | CleaveSort::DepthDesc => {
                    let ordering = na
                        .map(|n| n.depth)
                        .unwrap_or(0)
                        .cmp(&nb.map(|n| n.depth).unwrap_or(0));
                    if sort == CleaveSort::DepthDesc {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                }
                // Value sorts reverse only the within-class comparison: numbers
                // always sort before non-numbers, ascending or descending.
                CleaveSort::ValueAsc | CleaveSort::ValueDesc => {
                    match (cleave_number(na), cleave_number(nb)) {
                        (Some(x), Some(y)) => {
                            let ordering = x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal);
                            if sort == CleaveSort::ValueDesc {
                                ordering.reverse()
                            } else {
                                ordering
                            }
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
            }
        });
    }

    /// Effective meta visibility per `docs/model/META_NODES.md`: a node is
    /// effectively meta when it or ANY ancestor carries `is_meta`. The walk is
    /// defined once here so skins never re-derive the contagion rule.
    #[must_use]
    pub fn is_effective_meta(&self, key: &NodeKey) -> bool {
        let Some(mut node) = self.node_by_key(key) else {
            return false;
        };
        let mut hops = 0usize;
        loop {
            if node.is_meta {
                return true;
            }
            let Some(parent_id) = node.parent.as_ref() else {
                return false;
            };
            let Some(parent) = self.node(parent_id) else {
                return false;
            };
            node = parent;
            hops += 1;
            if hops > self.nodes.len() {
                return false; // projection out of sync; fail open as visible
            }
        }
    }
}

fn cleave_number(node: Option<&NodeView>) -> Option<f64> {
    match node.map(|n| &n.computed_value) {
        Some(NodeValueProjection::Number { raw, .. }) => raw.parse::<f64>().ok(),
        _ => None,
    }
}

/// Serializable-IR seam tests (stack requirement B.2.1): every projection that
/// crosses the future worker `postMessage` boundary must survive
/// serialize → deserialize unchanged, through `serde_json` specifically
/// (the structured-clone fallback wire format).
#[cfg(test)]
mod classification_tests {
    use super::*;

    fn node(id: &str, kind: NodeContentKind) -> NodeView {
        NodeView {
            key: NodeKey::new(format!("key:{id}")),
            id: NodeId::new(id),
            display_name: id.to_string(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            content_kind: kind,
            content_text: String::new(),
            computed_value: NodeValueProjection::default(),
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta: false,
            table: None,
        }
    }

    fn key(id: &str) -> NodeKey {
        NodeKey::new(format!("key:{id}"))
    }

    /// Build a workspace from nodes plus directed `(owner, target)` dependency
    /// edges. Only the reverse-edge index is populated — that is what
    /// `incoming_count_by_key` (and therefore the classifier) reads.
    fn workspace(nodes: Vec<NodeView>, edges: &[(&str, &str)]) -> WorkspaceState {
        let mut state = WorkspaceState::default();
        for view in nodes {
            state.key_order.push(view.key.clone());
            state.nodes.insert(view.id.clone(), view.clone());
            state.nodes_by_key.insert(view.key.clone(), view);
        }
        for (owner, target) in edges {
            let edge = DependencyEdgeProjection {
                edge_id: format!("{owner}->{target}"),
                descriptor_id: format!("desc:{owner}->{target}"),
                owner: NodeId::new(*owner),
                owner_key: key(owner),
                target: NodeId::new(*target),
                target_key: key(target),
                kind: DependencyKindProjection::StaticDirect,
            };
            state
                .dependencies
                .reverse_edges_by_key
                .entry(edge.target_key.clone())
                .or_default()
                .push(edge);
        }
        state
    }

    #[test]
    fn constant_is_input_when_consumed_else_free_value() {
        // `rate` is a constant consumed by `monthly`; `weights` is unused.
        let state = workspace(
            vec![
                node("rate", NodeContentKind::Constant),
                node("monthly", NodeContentKind::Formula),
                node("weights", NodeContentKind::Constant),
            ],
            &[("monthly", "rate")],
        );
        assert_eq!(
            state.node_classification(&key("rate")),
            NodeClassification::Input
        );
        assert_eq!(
            state.node_classification(&key("weights")),
            NodeClassification::FreeValue
        );
    }

    #[test]
    fn formula_is_intermediate_when_consumed_else_output() {
        // base <- mid <- top: base/mid are consumed, top is terminal.
        let state = workspace(
            vec![
                node("base", NodeContentKind::Formula),
                node("mid", NodeContentKind::Formula),
                node("top", NodeContentKind::Formula),
            ],
            &[("mid", "base"), ("top", "mid")],
        );
        assert_eq!(
            state.node_classification(&key("base")),
            NodeClassification::Intermediate
        );
        assert_eq!(
            state.node_classification(&key("mid")),
            NodeClassification::Intermediate
        );
        assert_eq!(
            state.node_classification(&key("top")),
            NodeClassification::Output
        );
    }

    #[test]
    fn empty_content_is_empty_even_when_depended_on() {
        let state = workspace(
            vec![
                node("blank", NodeContentKind::Empty),
                node("f", NodeContentKind::Formula),
            ],
            &[("f", "blank")],
        );
        assert_eq!(
            state.node_classification(&key("blank")),
            NodeClassification::Empty
        );
    }

    #[test]
    fn unknown_key_is_empty() {
        let state = WorkspaceState::default();
        assert_eq!(
            state.node_classification(&key("ghost")),
            NodeClassification::Empty
        );
    }

    #[test]
    fn only_incoming_edges_matter_not_outgoing() {
        // `lonely` (constant) has an OUTGOING edge to `other` but no incoming;
        // it must stay FreeValue. `other` gains an incoming edge -> Input.
        let state = workspace(
            vec![
                node("lonely", NodeContentKind::Constant),
                node("other", NodeContentKind::Constant),
            ],
            &[("lonely", "other")],
        );
        assert_eq!(
            state.node_classification(&key("lonely")),
            NodeClassification::FreeValue
        );
        assert_eq!(
            state.node_classification(&key("other")),
            NodeClassification::Input
        );
    }

    #[test]
    fn stable_ids_are_lower_snake() {
        assert_eq!(NodeClassification::Input.stable_id(), "input");
        assert_eq!(NodeClassification::FreeValue.stable_id(), "free_value");
        assert_eq!(NodeClassification::Intermediate.stable_id(), "intermediate");
        assert_eq!(NodeClassification::Output.stable_id(), "output");
        assert_eq!(NodeClassification::Empty.stable_id(), "empty");
    }
}

#[cfg(test)]
mod serde_round_trip_tests {
    use super::*;

    fn round_trip<T>(value: &T) -> T
    where
        T: Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_string(value).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn node_value_projection_nested_array_round_trips() {
        let value = NodeValueProjection::Array {
            rows: 2,
            cols: 2,
            cells: vec![
                vec![
                    NodeValueProjection::Number {
                        raw: "1.5".to_string(),
                        display: "1.50".to_string(),
                    },
                    NodeValueProjection::Array {
                        rows: 1,
                        cols: 2,
                        cells: vec![vec![
                            NodeValueProjection::Logical {
                                value: true,
                                display: "TRUE".to_string(),
                            },
                            NodeValueProjection::Reference {
                                target: "model-path:Accounts.Income".to_string(),
                            },
                        ]],
                    },
                ],
                vec![
                    NodeValueProjection::Missing,
                    NodeValueProjection::Error("#REF!".to_string()),
                ],
            ],
        };
        assert_eq!(round_trip(&value), value);
    }

    #[test]
    fn array_display_text_is_a_compact_summary() {
        // Inline value cells must never render the unrolled array (it blows out
        // the layout); display_text is a compact shape summary.
        let value = NodeValueProjection::Array {
            rows: 5,
            cols: 8,
            cells: Vec::new(),
        };
        assert_eq!(value.display_text(), "5x8 array");
    }

    #[test]
    fn workspace_state_default_round_trips() {
        let state = WorkspaceState::default();
        assert_eq!(round_trip(&state), state);
    }

    /// H3 acceptance (2): a `GridProjection` serialized before H3 added
    /// `GridCellProjection::authored` and `GridProjection::authored_epoch`
    /// (i.e. JSON with neither field present) still deserializes — both
    /// default (`authored: None`, `authored_epoch: 0`), the same
    /// forward-compatibility contract `GridOverlayBundle`'s `#[serde(default)]`
    /// fields already prove for older mirrors (`workspace.rs:822`).
    #[test]
    fn pre_h3_grid_projection_json_deserializes_with_authored_defaults() {
        let pre_h3_json = r#"{
            "grid_node_key": "sheet",
            "grid_node_id": "Sheet1",
            "grid_id": "book:grid:sheet:grid",
            "max_rows": 1048576,
            "max_cols": 16384,
            "cells": [
                {
                    "row": 1,
                    "col": 1,
                    "value": { "Number": { "raw": "7", "display": "7" } },
                    "value_epoch": 1
                }
            ],
            "projection_epoch": 1,
            "differential_clean": true
        }"#;
        let grid: GridProjection = serde_json::from_str(pre_h3_json).expect("deserialize");
        assert_eq!(grid.authored_epoch, 0, "authored_epoch defaults to 0");
        assert_eq!(
            grid.cells[0].authored, None,
            "a pre-H3 cell has no authored field, so it defaults to None"
        );
        assert_eq!(grid.overlays, GridOverlayBundle::default());
    }

    #[test]
    fn calc_run_projection_with_phase_timings_round_trips() {
        let mut phase_timings_micros = BTreeMap::new();
        phase_timings_micros.insert(PhaseKeyProjection::TotalEngineExecute, 1_234_u128);
        phase_timings_micros.insert(
            PhaseKeyProjection::Other("custom_phase".to_string()),
            56_789_u128,
        );
        let run = CalcRunProjection {
            run_state: CalcRunStateProjection::VerifiedClean,
            evaluation_order: vec![NodeId::new("Accounts.Income")],
            runtime_effect_count: 1,
            runtime_effects: vec![RuntimeEffectProjection {
                kind: "volatile".to_string(),
                family: RuntimeEffectFamilyProjection::DynamicDependency,
                detail: "detail".to_string(),
            }],
            runtime_overlay_count: 0,
            runtime_overlays: Vec::new(),
            derivation_trace_count: 0,
            derivation_traces: Vec::new(),
            invalidated_nodes: vec![NodeInvalidationProjection {
                node: NodeId::new("Accounts.Income"),
                node_key: NodeKey::new("key:income"),
                calc_state: NodeCalcStateProjection::DirtyPending,
                requires_rebind: true,
                reasons: vec![
                    InvalidationReasonProjection::StructuralRebindRequired,
                    InvalidationReasonProjection::DependencyAdded,
                ],
            }],
            binding_diagnostics: Vec::new(),
            phase_timings_micros,
            diagnostics: vec!["diag".to_string()],
        };
        assert_eq!(round_trip(&run), run);

        // The phase-timing map must key by stable id on the wire — JSON map
        // keys are strings, so the manual impls are load-bearing here.
        let json = serde_json::to_value(&run).expect("serialize");
        assert_eq!(
            json["phase_timings_micros"]["total_engine_execute"],
            serde_json::json!(1_234)
        );
        assert_eq!(
            json["phase_timings_micros"]["custom_phase"],
            serde_json::json!(56_789)
        );
    }

    #[test]
    fn phase_key_round_trips_as_stable_id_including_unknown() {
        let known: PhaseKeyProjection =
            serde_json::from_str("\"oxfml_formula_evaluation\"").expect("deserialize");
        assert_eq!(known, PhaseKeyProjection::OxfmlFormulaEvaluation);
        let unknown: PhaseKeyProjection =
            serde_json::from_str("\"some_future_phase\"").expect("deserialize");
        assert_eq!(
            unknown,
            PhaseKeyProjection::Other("some_future_phase".to_string())
        );
        assert_eq!(
            serde_json::to_string(&unknown).expect("serialize"),
            "\"some_future_phase\""
        );
    }

    #[test]
    fn derivation_trace_projection_round_trips() {
        let trace = DerivationTraceProjection {
            trace_schema_id: "trace-v1".to_string(),
            owner: NodeId::new("Accounts.Total"),
            owner_key: NodeKey::new("key:total"),
            formula_artifact_id: "formula:1".to_string(),
            bind_artifact_id: None,
            formula_stable_id: "stable:1".to_string(),
            trace_mode: "full".to_string(),
            template_selection: DerivationTemplateSelectionProjection {
                prepared_formula_key: "pf:1".to_string(),
                shape_key: "shape:1".to_string(),
                dispatch_skeleton_key: "skeleton:1".to_string(),
                plan_template_key: "plan:1".to_string(),
                template_holes: Vec::new(),
            },
            hole_bindings: vec![DerivationHoleBindingProjection {
                hole_id: "hole:0".to_string(),
                payload: "payload".to_string(),
            }],
            sub_invocation_tree: vec![DerivationInvocationProjection {
                invocation_ordinal: 0,
                invocation_kind: "function".to_string(),
                function_name: "SUM".to_string(),
                function_id: "fn:sum".to_string(),
                arg_preparation_profile: None,
                prepared_arguments: Vec::new(),
                kernel_returned_value: Some("3".to_string()),
                kernel_returned_value_typed: Some(NodeValueProjection::Number {
                    raw: "3".to_string(),
                    display: "3".to_string(),
                }),
                children: Vec::new(),
            }],
            kernel_returned_value: "3".to_string(),
            kernel_returned_value_typed: None,
            oxfml_trace_events: Vec::new(),
        };
        assert_eq!(round_trip(&trace), trace);
    }

    #[test]
    fn revision_history_projection_round_trips() {
        let history = RevisionHistoryProjection {
            current_revision_id: Some("rev:2".to_string()),
            entries: vec![RevisionHistoryEntryProjection {
                revision_id: "rev:2".to_string(),
                parent_revision_id: Some("rev:1".to_string()),
                structural_snapshot_id: "structural:2".to_string(),
                node_input_snapshot_id: "input:2".to_string(),
                namespace_snapshot_id: "namespace:2".to_string(),
                transaction_id: Some("txn:9".to_string()),
                transaction_summary: Some(RevisionTransactionSummaryProjection {
                    transaction_id: "txn:9".to_string(),
                    invalidated_nodes: vec![RevisionInvalidationSummaryEntryProjection {
                        node: NodeKey::new("key:income"),
                        requires_rebind: true,
                        reasons: vec![InvalidationReasonProjection::TreeReferenceMembershipChanged],
                    }],
                    requires_rebind: vec![NodeKey::new("key:income")],
                    estimated_node_count: 3,
                }),
                is_current: true,
            }],
        };
        assert_eq!(round_trip(&history), history);
    }
}

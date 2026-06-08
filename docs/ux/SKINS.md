# DNA TreeCalc — Skin Architecture

This document specifies the skin architecture for DNA TreeCalc's UI. It refactors the earlier "alternative layouts" framing from [`TECHNICAL.md`](TECHNICAL.md) §3.5 into a first-class skinning model: multiple parallel front-ends to the same core, switchable at runtime, each with its own persisted state, coexisting in the same workspace file.

The mental model is Winamp-style skinning: a tight, stable core; a clean API exposed to the skin layer; declarative skin definitions; multiple skins installed at once; fast switching. Adopting that idiom early gives us the freedom to keep accumulating front-ends without re-litigating the architecture every time.

The motivation is empirical: the existing eight mockups aren't competing layouts to choose between. They're different mental models of the same data — cell-view for data entry, across for pipelines, canvas for structure, triple-editor for deep formula work. A modeler shifts between these throughout a session.

---

## 1. Layered architecture

```
+---------------------------------------------------------------+
|  Layer 6 — Skin registry & switcher                           |
|  Lists installed skins; orchestrates activation, teardown,    |
|  meta-state hydration.                                        |
+---------------------------------------------------------------+
|  Layer 5 — Active skins (one or many)                         |
|  Concrete WorkspaceSkin implementations.                      |
|  e.g. CanvasFlow, CellView, NodesAcross, TripleEditor, ...    |
+---------------------------------------------------------------+
|  Layer 4 — WorkspaceSkin trait (the contract)                 |
|  Defines what a skin gets, what it provides, what it persists.|
+---------------------------------------------------------------+
|  Layer 3 — Shared UI primitives                               |
|  FormulaBar, FormulaEditor, ValueDisplay, ArrayGrid,          |
|  DrillPanel, NodeCard, TreeRow, WireRenderer, ZoomControls,   |
|  Minimap, LassoSelection, ...                                 |
+---------------------------------------------------------------+
|  Layer 2 — OxCalc engine context                              |
|  OxCalcTreeContext — canonical calculation state and recalc.  |
+---------------------------------------------------------------+
|  Layer 1 — Core (skin-agnostic)                               |
|  WorkspaceState: UI/session projection, selection, skin state,|
|  visible rows, edit buffers, save/reopen workflow.            |
+---------------------------------------------------------------+
```

Layers 1 and 2 already exist (engineered for the technical plan). Layer 3 is the existing OneCalc-derived primitives plus a few additions. Layers 4–6 are the new skin infrastructure.

---

## 2. The skin contract — abstract model

A skin is, abstractly, a node with four roles:

```
                  +-----------------------------+
                  |          WORKSPACE          |
                  | (single source of truth)    |
                  +-----------------------------+
                     ^      |      ^      |
                     |      |      |      |
                READS values FIRES intents
                from core   to mutate core
                     |      |      |      |
                     |      v      |      v
                  +-----------------------------+
                  |            SKIN             |
                  |  Observer  +  Mutator       |
                  |  HasState  +  Renderer      |
                  +-----------------------------+
                     |              ^
                     |              |
                   READS         WRITES
                   own state    own state
                   (typed)      (typed)
                     |              |
                     v              |
                  +-----------------------------+
                  |     PER-SKIN STATE          |
                  |  (typed, persisted as       |
                  |   meta-nodes)               |
                  +-----------------------------+
```

The four roles:

1. **Observer.** Subscribes to the workspace's read signals — tree shape, formulas, values, formats, calc state, selection. The skin sees what's current; reactive re-render fires when any of these change.
2. **Mutator.** Asks the workspace to change by sending **intents** through a `Dispatcher`. Skins never directly write to workspace state and never directly call the OxCalc context — both are mediated by the host/session.
3. **HasState.** Owns a typed, persistent block of view-specific state (canvas positions, column widths, collapse state). Mutates its own state directly; the host serializes for persistence.
4. **Renderer.** Produces a UI fragment given the current observer + state inputs. Pure of side effects (other than dispatching intents in response to user actions and updating its own state).

This is roughly the Elm / MVU pattern adapted to multi-source state (core workspace + skin state + cross-skin shared state) and to the fact that rendering is reactive (Leptos signals), not redrawn-from-scratch.

The remainder of this section spells out each role with concrete typed interfaces.

## 2.0 Current implementation checkpoint

The live framework now exposes the same boundary in code:

- `WorkspaceIntent` covers selection, recalculation, content edits, and structural edits (`AddNode`, `RenameNode`, `MoveNode`, `ReorderNode`, `DeleteNode`).
- `WorkspaceState` carries the skin-facing projection for tree nodes, values, calc state, last run status, diagnostics, dependency graph summaries, invalidation summaries, table identity/lifecycle summaries, and active table-cell detail readback for body/totals cells.
- `HostDispatcher` routes accepted calc-affecting intents through `TreeWorkspaceSession`, which calls `OxCalcTreeContext` and republishes the typed projection. Selection and shared skin state remain facade state and do not call OxCalc.
- `src/dnatreecalc-host/tests/programmable_skin_ir.rs` mounts a test-only programmable skin and drives the IR from the outside with a compact Rust DSL. Product behavior tests should prefer that harness over direct session calls when the skin contract is the behavior under test.
- `ValueBoard` consumes that projection directly for table highlighting, keyboard table-cell navigation, and selected-cell summaries. It does not derive table semantics from formula text or re-resolve structured references.

## 2.1 The `WorkspaceSkin` trait

```rust
pub trait WorkspaceSkin: Send + Sync + 'static {
    /// The typed state this skin persists. See §2.3.
    type State: SkinState;

    /// Stable identifier — used as the meta-namespace path component
    /// and as the persistent skin handle. Must not change between releases
    /// of the same skin without a migration plan.
    fn id(&self) -> SkinId;

    /// Display-time identity. May be localized.
    fn manifest(&self) -> SkinManifest;

    /// Declared capabilities (drives feature-detection in chrome).
    fn capabilities(&self) -> SkinCapabilities;

    /// Build and mount the skin's root UI. Called once per activation.
    /// The returned `SkinHandle` exposes lifecycle hooks for the host.
    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle;
}

pub struct RegisteredSkin {
    pub id: SkinId,
    pub manifest: SkinManifest,
    pub capabilities: SkinCapabilities,
    pub factory: Arc<dyn ErasedSkinFactory>,
}

pub trait ErasedSkinFactory: Send + Sync + 'static {
    fn mount_erased(&self, cx: ErasedSkinContext) -> SkinHandle;
}

pub struct SkinHandle {
    /// Root DOM/View element produced by the skin.
    pub view: View,
    /// Called when the skin is about to be torn down.
    /// Last chance to flush state, save scroll positions, etc.
    pub on_deactivate: Option<Box<dyn FnOnce()>>,
    /// Optional: subscribe to specific workspace events for incremental update.
    pub event_handler: Option<Box<dyn Fn(&WorkspaceEvent)>>,
}
```

Concrete skins implement `WorkspaceSkin` with a typed `State`. The skin registry stores `RegisteredSkin`, an object-safe wrapper that owns the manifest, capabilities, and an erased mount factory. This keeps skin implementation strongly typed without forcing `Vec<Box<dyn WorkspaceSkin>>` through Rust's associated-type/object-safety constraints. Capabilities and manifest are pure data. The core lifecycle operation is `mount`, which receives the typed context and returns the rendered fragment plus optional lifecycle callbacks.

## 2.2 `SkinContext<S>` — what a skin receives

```rust
pub struct SkinContext<S: SkinState> {
    // === Observer-role inputs (read-only) ===

    /// Canonical workspace state — tree, formulas, values, formats, templates.
    pub workspace: ReadSignal<WorkspaceState>,

    /// Per-node calc state (clean / dirty / evaluating / error / cycle).
    pub calc_state: ReadSignal<HashMap<TreeNodeId, NodeCalcState>>,

    /// Capability profile in effect (e.g., "treecalc-v1").
    pub profile: CapabilityProfileId,

    /// Format-resolution helper: walks Format meta-child inheritance to give the
    /// effective format for a node. Skins call this instead of walking themselves.
    pub format: FormatResolver,

    // === Shared mutable state ===

    /// Workspace-level selection, shared across all skins.
    /// One skin sets the selection; switching skins preserves it.
    pub selection: RwSignal<SelectionState>,

    /// Cross-skin shared state (tree-collapse, pinned nodes, recent selection).
    /// Read by any skin that wants it; written by any skin that owns the relevant action.
    pub shared: SharedSkinStateHandle,

    // === Skin's own state (typed) ===

    /// The skin's typed, persisted state. Reads and writes go through this signal.
    /// The host serializes/deserializes against the workspace meta-tree.
    pub state: SkinStateHandle<S>,

    // === Mutator-role outputs ===

    /// The dispatcher — the skin's only path to mutating workspace state.
    /// All structural edits, formula edits, format edits, template ops go through here.
    pub dispatch: Dispatcher,
}
```

Each field has a clear role. Nothing is generic JSON — the typed state `S` is the skin's own struct; the workspace is a typed `WorkspaceState`; the dispatch takes typed intents. `state` and `shared` are DnaTreeCalc host handles over meta-node persistence. Updating them does not call OxCalc, does not rebind formulas, and does not recalculate values.

## 2.3 `SkinState` — typed per-skin state

Each skin declares its own state type implementing `SkinState`. The host serializes this trait via `serde` for persistence to meta-nodes.

```rust
pub trait SkinState: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static {
    /// Current schema version of this state shape.
    /// Bumped when the struct's fields change incompatibly.
    fn schema_version() -> u32;

    /// Migrate a value persisted under an older schema version.
    /// Default impl rejects; skins implementing breaking changes override.
    fn migrate(_prior_version: u32, _prior_value: serde_json::Value) -> Result<Self, MigrationError> {
        Err(MigrationError::NoMigrationDefined)
    }

    /// Optional: garbage-collect stale entries (e.g., positions for deleted nodes).
    /// Called by the host periodically; default is no-op.
    fn gc(&mut self, _live_nodes: &HashSet<TreeNodeId>) {}
}
```

A canvas skin's concrete state:

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CanvasFlowState {
    pub positions: HashMap<TreeNodeId, Point>,
    pub groups: HashMap<GroupId, Vec<TreeNodeId>>,
    pub zoom: f32,
    pub pan: Point,
    pub layout_mode: CanvasLayoutMode,
    pub routing_mode: WireRoutingMode,
}

impl SkinState for CanvasFlowState {
    fn schema_version() -> u32 { 1 }
    fn gc(&mut self, live: &HashSet<TreeNodeId>) {
        self.positions.retain(|node, _| live.contains(node));
        for nodes in self.groups.values_mut() {
            nodes.retain(|n| live.contains(n));
        }
        self.groups.retain(|_, nodes| !nodes.is_empty());
    }
}
```

An outline-table skin's concrete state:

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct OutlineTableState {
    pub columns: Vec<OutlineColumnSpec>,
    pub sort: Option<SortSpec>,
    pub filter: Option<String>,
    pub scroll_position: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OutlineColumnSpec {
    pub kind: OutlineColumnKind,    // Name, Formula, Value, Format, Status, Custom(String)
    pub width: f32,
    pub visible: bool,
}

impl SkinState for OutlineTableState {
    fn schema_version() -> u32 { 1 }
}
```

A triple-editor skin's concrete state:

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TripleEditorState {
    pub nav_rail_width: f32,
    pub drill_panel_open: bool,
    pub value_pane_height: f32,
    pub developer_mode: bool,
}

impl SkinState for TripleEditorState {
    fn schema_version() -> u32 { 1 }
}
```

Each is small, typed, persistable. Skins access via `cx.state.with(|s| s.field)` for reads and `cx.state.update(|s| s.field = new_value)` for writes. The DnaTreeCalc host handles serialization to `skins.<skin_id>` meta-nodes. This is facade state, not engine state.

## 2.4 `SharedSkinState` — cross-skin state

Some state benefits multiple skins. Lives in the `skins.shared` meta-node subtree and is typed too:

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SharedSkinState {
    /// Per-node tree-row collapse state. Any tree-rendering skin reads this.
    pub tree_collapsed: HashSet<TreeNodeId>,

    /// User-pinned nodes (across all skins).
    pub pinned: Vec<TreeNodeId>,

    /// Recent selection history for back/forward navigation.
    pub recent_selections: VecDeque<TreeNodeId>,
}
```

When the triple-editor's nav rail toggles a node's collapse, it writes to `cx.shared.update(|s| s.tree_collapsed.insert(id))`. When the cell-view skin renders, it reads `cx.shared.with(|s| s.tree_collapsed.contains(&id))`. Consistency falls out of the shared host signal and its meta-node persistence.

## 2.5 `Dispatcher` — the intent gateway

A skin **never** writes directly to calculation workspace state and never calls OxCalc. Structural/content/template/external-value changes go through typed intents. Skin-specific and cross-skin facade state goes through `cx.state` / `cx.shared`, which are handled entirely by DnaTreeCalc.

```rust
pub struct Dispatcher {
    inner: Arc<DispatcherImpl>,
}

impl Dispatcher {
    /// Send a single intent. Returns a receipt that completes when the
    /// intent's effects are visible in the workspace signal.
    pub fn send(&self, intent: WorkspaceIntent) -> IntentReceipt;

    /// Send a batch of intents to be applied transactionally.
    /// Either all succeed and become one workspace update, or all roll back.
    /// (Backed by OxCalc transactional batch editing — engine prereq item 8.
    /// Until that lands, the host applies N sequentially and groups in undo.)
    pub fn send_batch(&self, intents: Vec<WorkspaceIntent>) -> IntentReceipt;

    /// Begin an explicit transaction. Use when the batch is built up
    /// across multiple user gestures (e.g., a multi-step structural refactor).
    pub fn begin_transaction(&self) -> TransactionScope;
}

pub struct TransactionScope { /* ... */ }
impl TransactionScope {
    pub fn send(&self, intent: WorkspaceIntent);
    pub fn commit(self) -> IntentReceipt;
    pub fn rollback(self);
}

pub struct IntentReceipt {
    pub accepted: bool,
    pub error: Option<IntentError>,
    pub completed_signal: ReadSignal<Option<IntentOutcome>>,
}
```

## 2.6 `WorkspaceIntent` — closed enumeration of asks

```rust
pub enum WorkspaceIntent {
    // --- Selection (cheap, no engine call) ---
    SelectNode(TreeNodeId),
    SelectMany(BTreeSet<TreeNodeId>),
    ExtendSelection { anchor: TreeNodeId, target: TreeNodeId },
    ClearSelection,

    // --- Structural edits (engine: rebind + recalc dependents) ---
    InsertChild { parent: TreeNodeId, position: usize, name: String, formula: String },
    InsertSibling { reference: TreeNodeId, side: SiblingSide, name: String, formula: String },
    RenameNode { node: TreeNodeId, new_name: String, propagate: PropagatePolicy },
    MoveNode { node: TreeNodeId, new_parent: TreeNodeId, new_position: usize },
    DeleteNode { node: TreeNodeId, cascade: bool },

    // --- Content edits (single content string: "" = Empty, leading `=` = formula, else literal constant) ---
    EditFormula { node: TreeNodeId, formula: String },           // set the content string (constant or =formula)
    ConvertToFormula { node: TreeNodeId, formula: String },      // turn a constant into a =formula
    ConvertToLiteral { node: TreeNodeId },                       // freeze: replace a formula with its computed value, written as a constant

    // --- Format / CF edits (writes to Format meta-children) ---
    SetFormatProperty { node: TreeNodeId, property: FormatPath, value: FormatValue },
    AddCfRule { node: TreeNodeId, rule: ConditionalFormatRule, position: usize },
    RemoveCfRule { node: TreeNodeId, position: usize },
    ReorderCfRules { node: TreeNodeId, new_order: Vec<usize> },

    // --- Template operations ---
    PromoteToTemplate { subtree: TreeNodeId, template_id: TemplateId },
    InstantiateTemplate { template_id: TemplateId, parent: TreeNodeId, name: String },
    EditTemplateStructure { template_id: TemplateId, edit: TemplateEdit },
    SyncTemplateToInstances { template_id: TemplateId },
    DetachInstance { instance_root: TreeNodeId },

    // --- Cross-workspace aliases ---
    RegisterWorkspaceAlias { alias: String, path: WorkspacePath },
    UnregisterWorkspaceAlias { alias: String },

    // --- External value updates (RTD / async streams) ---
    UpdateExternalValue { node: TreeNodeId, value: EvalValue, source: ExternalSourceId },
    InvalidateExternal { source: ExternalSourceId },

}
```

This closed set is also the **canonical command taxonomy** the host builds on beyond skins: model-affecting intents are the unit of undo/redo (CORE_MODEL §8a), so one intent — or one batched group — is one undoable step, and the command palette (REQUIREMENTS §3.11) surfaces the same set. Defining commands once here keeps skins, undo, and the palette in lockstep.

This is a closed set. Skins compose user gestures into these intents. The dispatcher routes:

- **Selection intents** → direct write to DnaTreeCalc host state (cheap).
- **Structural / formula intents** → OxCalc context call → engine recalc → workspace projection update.
- **Format intents** → DnaTreeCalc writes the node's `Format` meta-child, persists it, and updates render state; OxCalc sees only the normal `is_meta` behavior and does not evaluate that subtree.
- **Template intents** → host-level orchestration (read template structure, generate N structural edits to instances).
- **External-value intents** → OxCalc context external-value entry point → engine invalidation → cascade.

The skin doesn't know which path; it just dispatches and waits on the receipt.

## 2.7 `WorkspaceState` — what observers read

```rust
pub struct WorkspaceState {
    pub file_handle: Option<WorkspaceFileHandle>,
    pub dirty: bool,
    pub root: TreeNodeId,
    pub nodes: HashMap<TreeNodeId, TreeNodeState>,
    pub template_index: TemplateIndex,             // derived host bookkeeping over template meta-subtrees and rollout tags
    pub external_aliases: ExternalWorkspaceAliases,
    pub capability_profile_id: CapabilityProfileId,
    pub last_published_result: Option<OxCalcTreeCalculationOutcome>,
}

pub struct TreeNodeState {
    pub id: TreeNodeId,
    pub parent_id: Option<TreeNodeId>,
    pub sibling_index: usize,
    pub name: String,
    pub is_meta: bool,
    pub formula: String,                       // single content string: "" = Empty, leading `=` = formula, else literal constant
    pub computed_value: Option<EvalValue>,
    pub children: Vec<TreeNodeId>,
}
```

Skin reads via `cx.workspace.with(|ws| ws.nodes.get(&id))`. The signal fires on any change; the skin re-renders the affected portion.

Helper accessors are provided by the host (so skins don't reimplement them):

```rust
impl WorkspaceState {
    pub fn child(&self, parent: TreeNodeId, name: &str) -> Option<TreeNodeId>;
    pub fn ancestors_of(&self, node: TreeNodeId) -> impl Iterator<Item = TreeNodeId>;
    pub fn regular_children_of(&self, parent: TreeNodeId) -> impl Iterator<Item = TreeNodeId>;
    pub fn meta_children_of(&self, parent: TreeNodeId) -> impl Iterator<Item = TreeNodeId>;
    pub fn path_of(&self, node: TreeNodeId) -> Vec<String>;
    pub fn template_of_instance(&self, instance_root: TreeNodeId) -> Option<&InstanceLink>;
}
```

## 2.8 `FormatResolver` — format inheritance helper

Formats inherit through ancestor `Format` meta-children. Skins shouldn't re-implement the walk; they call:

```rust
pub struct FormatResolver { /* ... */ }

impl FormatResolver {
    /// Resolves a single format property at the given node.
    /// Walks the node's Format meta-child then its ancestors' Format meta-children, merging defaults.
    pub fn resolve(&self, node: TreeNodeId, property: FormatPath) -> Option<FormatValue>;

    /// Resolves the full effective format for a node (all properties merged).
    pub fn resolve_full(&self, node: TreeNodeId) -> EffectiveFormat;

    /// Evaluates conditional-formatting rules for a node's value (or each cell of an array).
    /// Returns the post-CF appearance per cell.
    pub fn apply_cf(&self, node: TreeNodeId, value: &EvalValue) -> CfResult;
}
```

A canvas-flow skin rendering a node's card calls `cx.format.resolve_full(node)` once per node and applies the result to its card's value-display region.

## 2.9 `SkinHandle` and lifecycle

```rust
pub struct SkinHandle {
    pub view: View,
    pub on_deactivate: Option<Box<dyn FnOnce()>>,
    pub event_handler: Option<Box<dyn Fn(&WorkspaceEvent)>>,
}
```

Lifecycle is straightforward:

```
host calls skin.mount(cx)
     |
     v
+---------------+    user actions     +-----------+
|  view mounts  | <-----------------> | dispatch  |
+---------------+    re-renders       +-----------+
     |
     | (active period — could be long)
     |
     v
host signals deactivate
     |
     v
on_deactivate() fires (flush state)
     |
     v
view unmounts
```

The host always calls `on_deactivate` before tearing down, giving the skin a chance to persist final state — though in practice, state should be persisted continuously through `cx.state.update(...)` so `on_deactivate` is rarely load-bearing.

---

## 2.10 What the manifest and capabilities look like

```rust
pub struct SkinManifest {
    pub display_name: &'static str,
    pub icon: SkinIcon,
    pub category: SkinCategory,
    pub description: &'static str,
    pub author: Option<&'static str>,
    pub version: &'static str,
}

pub enum SkinCategory {
    Editor,        // primary editing surface (TripleEditor, CellView, NodesAcross, Canvas)
    Overview,      // panoramic view (OutlineTable)
    Inspector,     // specialty deep-dive (FormatEditor, TemplateEditor, DependencyMap)
    Presentation,  // read-only viewer for sharing
}

pub struct SkinCapabilities {
    pub supports_multi_select: bool,
    pub supports_drag_reorder: bool,
    pub supports_canvas_positioning: bool,
    pub supports_inline_formula_edit: bool,
    pub supports_meta_node_display: bool,
    pub supports_drill_panel: bool,
    pub supports_zoom: bool,
    pub supports_search: bool,
    pub renders_arrays_inline: bool,
    pub renders_table_values: bool,
    pub renders_conditional_formatting: bool,
    pub renders_data_bars: bool,
    pub renders_icon_sets: bool,
    pub format_properties_rendered: FormatPropertySet,
}
```

`FormatPropertySet` is a bitset (or HashSet) over the available format properties:

```rust
pub enum FormatPropertyKind {
    NumberFormat, FontFamily, FontSize, FontWeight, FontColor,
    FillColor, BorderColor, DataBar, IconSet, ConditionalFormat,
}
```

A minimalist presentation skin might declare `format_properties_rendered: {NumberFormat, FontColor, FillColor}` and silently ignore the rest. The format-editor UI uses this to grey out properties not supported by the current skin (with a note: "Switch skins to see this effect").

---

## 2.11 Call traces — what actually happens

Five representative traces, with ASCII timing diagrams.

### Trace A: Skin activation and initial render

```
HOST                  SKIN              CONTEXT          STATE STORE      WORKSPACE
  |                     |                  |                  |               |
  |--build SkinContext->|                  |                  |               |
  |    (signals, dispatcher, etc.)                            |               |
  |                     |                  |                  |               |
  |--skin.mount(cx)---->|                  |                  |               |
  |                     |--cx.state.with()|                  |               |
  |                     |   (read initial state)              |               |
  |                     |                  |--load from-----> |               |
  |                     |                  |   skins.canvas-flow meta-state   |
  |                     |                  |<--deserialized---|               |
  |                     |<--CanvasFlowState{...}              |               |
  |                     |                                     |               |
  |                     |--cx.workspace.with() (read tree)----+-------------->|
  |                     |<--WorkspaceState---------+----------+---------------|
  |                     |                                     |               |
  |                     |--render View------------>           |               |
  |                     |   (compose primitives,              |               |
  |                     |    apply state.positions,           |               |
  |                     |    apply format.resolve_full(...))  |               |
  |                     |                                     |               |
  |<--SkinHandle--------|                                     |               |
  |                                                           |               |
  |--mount view in shell                                      |               |
```

### Trace B: User drags a node on the canvas

```
BROWSER          CANVAS SKIN        cx.state         STATE STORE    META TREE
   |                |                   |                  |              |
   |--drag event--->|                   |                  |              |
   |                |--compute new (x,y)|                  |              |
   |                |--cx.state.update(|s| s.positions.insert(id, p))     |
   |                |                   |                  |              |
   |                |                   |--state changed-->|              |
   |                |                   |                  |--serialize-->|
   |                |                   |                  |   meta-node  |
   |                |                   |                  |   update     |
   |                |                   |                  |              |
   |                |<--signal fires----|                  |              |
   |                |--re-render        |                  |              |
   |                |   (only repositioned node)           |              |
   |<--DOM patch----|                   |                  |              |
```

Note: no engine call. State change is a pure host-level mutation to the meta-tree. OxCalc is not involved.

### Trace C: User edits a formula in the formula bar

```
BROWSER  FORMULA BAR  CELL-VIEW SKIN    DISPATCHER         HOST          OXCALC CONTEXT
   |        |              |                  |              |              |             |
   |--type->|              |                  |              |              |             |
   |        |--on_change-->|                  |              |              |             |
   |        |   (txt, pos) |                  |              |              |             |
   |        |              |--dispatch.send(EditFormula{...})|              |             |
   |        |              |                  |--Intent ---->|              |             |
   |        |              |                  |              |--set formula + recalculate---->
   |        |              |                  |              |              |--bind_formula
   |        |              |                  |              |              |  evaluate
   |        |              |                  |              |<--workspace/node views----------|
   |        |              |                  |              |--apply to workspace state  |
   |        |              |                  |              |   (formula, value, diags)  |
   |        |              |                  |              |              |             |
   |        |              |                  |              |--workspace signal fires    |
   |        |              |<--workspace.with() (new value)--+-------------+--------------+
   |        |              |--rerender                       |              |             |
   |        |<--update     |   (FormulaBar shows new formula text;          |             |
   |<--DOM--|   tokens     |    cell value re-renders with new value)       |             |
```

The skin doesn't know about the OxCalc context or the engine. It dispatches an `EditFormula` intent; the workspace state subsequently updates. Re-render follows the signal.

### Trace D: Template edit triggers sync to instances

```
USER  TEMPLATE-EDITOR SKIN     DISPATCHER     HOST  TEMPLATE-SYNC SERVICE  OXCALC CONTEXT
  |          |                       |          |              |              |          |
  |--save--->|                       |          |              |              |          |
  |          |--dispatch.send(EditTemplateStructure{...})       |              |          |
  |          |                       |--Intent->|              |              |          |
  |          |                       |          |--diff current instance vs template      |
  |          |                       |          |--for each instance:                     |
  |          |                       |          |    use mapping/tags; plan structural edits
  |          |                       |          |              |              |          |
  |          |                       |          |--invoke------>|              |          |
  |          |                       |          |  template-sync-service       |          |
  |          |                       |          |              |--dispatcher.send_batch(N intents)
  |          |                       |          |              |              |          |
  |          |                       |          |<-------------|--each goes through OxCalc context|
  |          |                       |          |              |              |--rebind->|
  |          |                       |          |              |              |  recalc
  |          |                       |          |              |              |<-result--|
  |          |                       |          |<----all instance updates aggregated     |
  |          |                       |          |--single workspace signal fire           |
  |          |                       |<--receipt|              |              |          |
  |          |<--complete signal----|          |              |              |          |
  |          |--rerender (instance badges update)              |              |          |
```

Multi-step orchestration. The template-editor skin sends one logical intent; the host's template-sync service uses the stored template mapping/tags to diff instances on demand, expands accepted changes into ordinary structural edits, the dispatcher batches them, OxCalc applies them, and one signal fire propagates to all subscribers.

### Trace E: External RTD value pushes asynchronously

```
EXTERNAL SOURCE   RTD ADAPTER   OXCALC CONTEXT         WORKSPACE        ACTIVE SKIN
       |              |            |             |              |                |
       |--push value->|            |             |              |                |
       |  for node N  |--update external value(N, v)            |                |
       |              |            |--inject---->|              |                |
       |              |            |             |--mark N dirty                 |
       |              |            |             |--compute invalidation closure |
       |              |            |             |--evaluate dirty nodes         |
       |              |            |             |--publish new values           |
       |              |            |<--invalidation+values--|   |                |
       |              |            |             |              |                |
       |              |            |--apply to workspace.nodes (values diff)     |
       |              |            |--workspace signal fires    |                |
       |              |            |             |              |--signal fires->|
       |              |            |             |              |                |--re-render
       |              |            |             |              |                |  affected nodes
       |              |            |             |              |                |
       |              |            |             |              |                |  (RtdAdapter is
       |              |            |             |              |                |   host-side; skin
       |              |            |             |              |                |   sees only the
       |              |            |             |              |                |   workspace update.)
```

Critical: the skin sees no special "RTD mode." External value updates arrive through the same workspace-state signal as any other change. The reactive render pattern handles them. No skin-specific code for streaming sources.

---

## 2.12 What the `JsonValue` blob model would have been — and what we're not doing

Strawman (the *over-generic* approach):

```rust
// What we're NOT doing — the rejected design.
pub trait WorkspaceSkin {
    fn render(&self, ctx: SkinContext) -> Element;
}
pub struct SkinContext {
    pub workspace: ReadSignal<serde_json::Value>,    // generic
    pub skin_state: RwSignal<serde_json::Value>,     // generic
    pub context: Arc<OxCalcTreeContext>,             // raw engine context — leaks engine
}
```

Problems with that:
- Skins parse JSON to find their state at every render — slow and error-prone.
- No schema versioning or migration — silent breakage on format changes.
- OxCalc calls from inside skins — skin authors learn engine plumbing, leak abstractions.
- Workspace shape is a blob — autocomplete and type-check don't help.

The model we're adopting (§2.1–§2.11) replaces every blob with a typed struct:
- `S: SkinState` for skin state.
- `WorkspaceState` for workspace observation.
- `WorkspaceIntent` (closed enum) for mutation requests.
- `Dispatcher` for routing intents.
- `FormatResolver` for inheritance walks.

The persistence layer serializes the typed state to JSON for meta-node storage; the skin never sees the JSON. The intent layer prevents skins from directly calling the OxCalc context; the host routes correctly based on the intent kind.

The cost is a moderately larger trait surface and more types to define per skin. The payoff is type-checked skin authoring, schema-versioned state, decoupling from engine plumbing, and reviewable intent flow.

---

## 3. Shared UI primitives (Layer 3)

A reusable component library. Skins compose these. The library is the "Winamp button kit" — stable, well-tested building blocks every skin draws from.

**Current OneCalc-derived primitives** (lifted to a shared crate):
- `FormulaEditor` — the textarea + overlay + commands surface.
- `FormulaBar` — Excel-style top-bar variant of the editor.
- `ValueDisplay` — scalar / array / error / reference / lambda renderer with a configurable wrapper.
- `ArrayGrid` — virtualized array view with per-cell formatting.
- `DrillPanel` — walk-tree drill.
- `CompletionPopup`, `SignatureHelp`, `FunctionHelp` — editor support.
- `DiagnosticSquiggle` — error / warning rendering.

**New TreeCalc primitives:**
- `NodeCard` — a node-as-card component (configurable: header, value, formula-glimpse, footer).
- `TreeRow` — a node-as-row component (configurable: name column, value column, extra columns).
- `WireRenderer` — bezier / orthogonal connection lines between elements.
- `ZoomControls` — zoom in/out/fit/reset cluster.
- `Minimap` — bird's-eye view component.
- `LassoSelection` — drag-rectangle multi-select.
- `GroupHandle` — group banner + collapse/promote actions.
- `StatusFoot` — bottom status bar with calc state, deps, hints.
- `ContextStrip` — top context bar with filename, profile, and skin switcher.
- `BreadcrumbPath` — node-path breadcrumb.

Primitives are pure Leptos components with well-defined props. They don't read core state directly; the skin reads state and passes the relevant slice to each primitive.

---

## 4. Per-skin meta-state

Each skin owns a meta-namespace under the workspace root. Skin state is persisted as meta-nodes (`is_meta = true`), invisible to formulas, host-managed by DnaTreeCalc.

**Path notation (canonical).** The meta subtrees — `skins` (holding `skins.shared` plus one `skins.<skin-id>` per skin), each node's `Format`, and `Templates` — are `is_meta` nodes addressed by ordinary `.`-separated tree paths (e.g. `skins.canvas-flow`, `skins.shared`, `Format.NumberFormat`). There is no `::` prefix and no `/` separator; the `[#…]` form in [`../model/CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) §3.3 is a separate structured-reference specifier, not meta-namespace addressing.

### 4.1 Namespace layout

```
.
├── (regular content)
└── skins                            (meta; auto-hidden)
    ├── shared                       (state used by multiple skins)
    │   ├── tree-state
    │   │   └── <nodeId>.collapsed = true | false
    │   ├── pinned-nodes
    │   └── recent-selection
    ├── triple-editor                (the TripleEditor skin's state)
    │   ├── panels.nav-rail.width = 280
    │   ├── panels.drill.open = true
    │   └── panels.value.height = 240
    ├── cell-view
    │   └── array-expanded.<nodeId> = true | false
    ├── nodes-across
    │   ├── scroll-position = 240
    │   ├── visible-scope = <nodePath>
    │   └── columns.<nodeId>.width = 200
    ├── canvas-flow
    │   ├── positions.<nodeId> = { x: 470, y: 100 }
    │   ├── groups.<groupId> = [<nodeId>, ...]
    │   ├── zoom = 1.0
    │   ├── pan = { x: 0, y: 0 }
    │   ├── layout-mode = "free"
    │   └── routing-mode = "bezier"
    ├── outline-table
    │   ├── columns.order = [name, formula, value, format, status]
    │   ├── columns.<colName>.width = 220
    │   ├── sort.key = "name"
    │   ├── sort.direction = "asc"
    │   └── filter.text = ""
    └── format-editor
        └── selected-property = "Font.Color"
```

### 4.2 Shared meta-namespace

`skins.shared` holds state that multiple skins want to read. Cross-skin continuity:

- **`tree-state.<nodeId>.collapsed`** — collapse/expand state. Any tree-rendering skin honors it.
- **`pinned-nodes`** — list of "favorite" nodes a user has pinned across the workspace.
- **`recent-selection`** — recent selection history.

A skin reads from `shared_skin_state` and `skin_state` while mounted. Writes to its own state go through `skin_state`; writes to shared facade state (e.g., toggle collapse) go through `shared_skin_state`. These writes are persisted by DnaTreeCalc as meta-node changes, but they are not OxCalc transactions and do not participate in formula dependency invalidation.

### 4.3 Coexistence

All skins' meta-namespaces persist simultaneously in the workspace file. Switching skins doesn't migrate data; it just changes which namespace is being rendered. A user who arranges nodes on the canvas, then switches to cell view to do data entry, then switches back to canvas — finds their canvas layout intact.

Adding a new skin: registers its namespace; first activation seeds defaults. Removing a skin (e.g., disabled): the meta-namespace remains in the file; can be re-enabled later.

### 4.4 Garbage collection

When a node is deleted from the regular tree, the host walks all skin meta-namespaces and prunes any keys referring to the deleted node id. This is a host-level housekeeping step, run on save or on demand.

---

## 5. Skin lifecycle

### 5.1 Initial mount

1. Workspace loads. Core state hydrates from persistence.
2. Skin registry initializes with built-in skins.
3. User's preferred default skin (from workspace meta or app preferences) is assigned to the main mount slot.
4. Host hydrates that skin instance's typed state from its meta-namespace.
5. Host builds `SkinContext` and calls `skin.mount(cx)`.
6. The returned `SkinHandle.view` mounts into the shell.

### 5.2 Switching skins

1. User triggers switch (via skin-switcher chrome, command palette, or keyboard shortcut).
2. The outgoing skin handle's `on_deactivate` runs: flushes any pending state to its meta-namespace.
3. The outgoing skin's component tree tears down. Shared signals (workspace, selection, calc state) persist; skin-specific signals go away.
4. Host hydrates the new skin's typed state from its meta-namespace.
5. Host builds `SkinContext` and calls `skin.mount(cx)`.
6. The returned `SkinHandle.view` mounts into the shell.

### 5.3 Workspace edits during a session

When a user edits in any skin:
- The skin dispatches typed intents to DnaTreeCalc.
- DnaTreeCalc routes only calc-affecting intents through the OxCalc context; facade/meta-only changes stay in the host.
- Core state's signal updates trigger re-renders in mounted skins (and any subscribed primitives).
- Other skins' meta-namespaces are unaffected unless the host explicitly updates shared facade state.

### 5.4 Skin-meta updates

When a user reorganizes the canvas (drag a node to a new position):
- Canvas skin writes `{x: ..., y: ...}` to its meta-namespace via `skin_state.update(...)`.
- DnaTreeCalc persists the meta-node value and updates the mounted skin signal.
- No OxCalc context call, formula rebind, or value recalc occurs.
- These writes are outside the calculation undo stack by default; a skin may offer its own local view-state undo where useful.
- The change persists with the workspace.
- Other skins ignore it.

### 5.5 Focus management

Because skins mount and unmount at runtime, **focus is a contract concern, not a late accessibility detail.** On a switch (or a pane-composition change) the host restores a deterministic focus target: the element bound to the shared `selection` if the incoming skin can render it, else the skin's declared primary surface. Each skin declares its focusable entry point; the host owns focus on the chrome (context strip, skin switcher) and hands focus to the mounted skin on activation. Keyboard-completeness ([`REQUIREMENTS.md`](REQUIREMENTS.md) §6.3) depends on focus surviving switches — a skin must never leave focus orphaned on a torn-down node.

---

## 6. Mapping existing mockups to skins

This section gives the short identity map. The detailed feature-by-feature mapping lives in [`TRACEABILITY.md`](TRACEABILITY.md), which traces each prototype affordance through primitives, state ownership, DnaTreeCalc services, intents, and OxFml/OxCalc boundaries.

| Mockup | Skin id | Category | State |
|---|---|---|---|
| 01 — Workspace shell | `triple-editor` | Editor | nav-rail width, drill open, value pane height; shared tree-state |
| 02 — Array value | (a render state of `triple-editor`) | — | reuses TripleEditor; ArrayGrid renders for array-valued nodes |
| 03 — Template editor | `template-editor` | Specialty | selected template id, pinned instances |
| 04 — Outline-table | `outline-table` | Overview | column order/widths, sort key, filter text, scroll position |
| 05 — Format editor | `format-editor` | Inspector | selected format property, computed-vs-literal preference |
| 06 — Excel-style cell | `cell-view` | Editor | array-expanded set; shared tree-state |
| 07 — Nodes-across | `nodes-across` | Editor | scroll position, scope, column widths |
| 08 — Canvas flow | `canvas-flow` | Editor | positions per node, groups, zoom, pan, layout-mode, routing-mode |

Some skins (TripleEditor, Outline-table) read the shared tree-state so collapsing a subtree in one skin shows it collapsed in the other. Other skins (Canvas) don't use tree-state at all — they have their own spatial organization.

The Format Editor and Template Editor are "Inspector" / "Specialty" category skins: they can be mounted alongside an Editor skin when the shell uses split panes, or they can occupy the main slot in a simpler one-pane shell.

---

## 7. Skin Composition

Layer 6 is a composition layer, not a hard "one active skin" switch. The default v1 shell can mount one editor skin in one main slot, but the interface is flexible enough to mount multiple skin instances when a skin or shell wants split panes, inspectors, or subtree-scoped views:

- a `SkinMountSlot` identifies where the skin instance is mounted (`main`, `right-inspector`, `split-left`, `split-right`, etc.);
- a `SkinContext` may carry an optional focus scope such as a selected node, subtree root, or template id;
- each mounted instance keeps its own typed state where needed, while shared state (`selection`, collapse state, pinned nodes) remains host-managed;
- inspector/specialty skins such as FormatEditor and TemplateEditor can therefore be mounted beside an editor skin without changing the core contract.

This makes adaptive layout a skin/shell composition policy rather than a separate core mode system. V1 can start with the simple one-main-slot shell, but the trait and state model do not need to be redesigned for multi-pane or per-subtree presentation later.

---

## 8. User-authored skins (deferred to v3+)

Long-term, a user might want a custom skin. Two paths:
- **Declarative skin definitions** — a JSON/YAML manifest describing layout regions, primitive bindings, styling tokens. Similar to VS Code themes/extensions.
- **Programmatic skins** — Rust-implementing `WorkspaceSkin` distributed as crates.

v1 ships with built-in skins only. The architecture doesn't preclude either future path.

---

## 9. Formatting vs. skin styling — the boundary

Two distinct visual concerns:

**Format (user's data appearance).** Per-node `Format` meta-children describe how the *data value* looks: number format, font, fill color, conditional rules, data bars, icon sets. These are properties the user owns and edits via the Format Editor.

**Skin styling (chrome appearance).** Skin-level theming describes how the *application* looks: panel colors, accent, font in chrome, layout shape, spacing. These are properties the skin owns.

**The rule:** *formats describe the value; skins describe the frame.* Where the value appears in a skin's layout, the user's format wins. Surrounding chrome stays in the skin's theme.

Concrete: node `MyNode` has a meta-child path `Format.Fill.Color = "yellow"`.
- Cell view: cell's value background = yellow.
- Canvas: card's value-display region = yellow background; card's header/footer = skin's theme.
- Outline-table: value column cell = yellow; row chrome = skin's theme.
- Across: column's value rows = yellow; column header = skin's theme.

The value's appearance stays consistent across skins. The user can format their data confident that switching skins doesn't change what the data looks like.

### 9.1 Carve-outs

1. **Skin capability declares supported format properties.** A minimalist or read-only "presentation" skin may declare it doesn't render data bars or icon sets; those format properties are silently ignored on that skin. The user sees the value with reduced visual decoration. Switching to a fuller skin restores all formatting.
2. **Skin defaults fill in for unset formats.** When a property is unset, the skin's theme provides the default (e.g., default text color = skin accent). Once the user sets an explicit format, that wins. This lets unstyled workspaces still look coherent under any skin.
3. **Format inheritance is a host-level walk, not a skin concern.** When `Account` has `Format.NumberFormat` and a descendant has no local `Format.NumberFormat`, the host's format-resolution walker returns the ancestor's value. Skins call the host's `resolve_format(node_id, property)` API rather than implementing inheritance themselves.

### 9.2 Conditional formatting

CF rules are a property of format. They evaluate against the node's value and produce an `ArrayCellFormat` (or scalar equivalent) that modifies what the skin renders.

Excel's CF model — multiple rules per cell, ordered evaluation, "Stop If True", multiple rule types — is the target. **OxFml's current CF support needs design review** to confirm it handles:
- Multiple rules per node/cell with ordered evaluation.
- "Stop If True" rule attribute that halts evaluation on a true match.
- Action accumulation across rules (rule 1 sets font, rule 2 sets icon — both apply unless one stops).
- Subtree-level CF (a CF rule at an ancestor's `Format.ConditionalFormat` applies to descendants too).

The TreeCalc CF UX (mockup 05 §Conditional formatting) presents the user with an ordered rule list and per-rule edit/delete/reorder. The engine must support the underlying semantics for this UX to be honest about what it does.

Tracked as engine prerequisite §6 item 10 in [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) and raised as a concrete handover: [`../handovers/HANDOVER_OXFML_conditional_formatting.md`](../handovers/HANDOVER_OXFML_conditional_formatting.md).

---

## 10. Skin registry and switching chrome

The shell provides a thin layer above the mounted skin composition:

- **Top-of-shell** context strip (filename, profile, recalc status) — universal, not skin-controlled.
- **Skin switcher** in the context strip — quick switch between installed skins in the focused mount slot. Could be a tab strip (as shown in mockups 06–08), a dropdown, or a command palette command.
- **Bottom-of-shell** status foot — partially skin-driven (skin contributes hints/keybindings), partially universal (calc state, deps count).

The shell occupies a thin border around the skin; everything inside is the skin's domain.

---

## 11. Performance and resource model

- **Each mounted skin instantiates its own components.** Component tree tears down on unmount/deactivation.
- **Shared signals survive switches and composition changes.** Workspace state, selection state, calc state — all in the core, unaffected by skin lifecycle.
- **Heavy components are lazy.** Drill panel, dependency graph, format editor instantiate only when the user opens them, not on every skin activation.
- **Meta-state hydration is cheap.** Per-skin meta is small (a few hundred entries for canvas positions in a moderate workspace). Loading on activate is sub-millisecond.
- **Switching is fast** by construction: tear down DOM of old skin instance, mount DOM of new skin instance, hydrate from meta. No data conversion, no engine recalc, no formula re-bind.

---

## 12. Versioning and migration

Each skin's `state_schema` includes a version. When the workspace loads:

1. Host reads each skin's meta-namespace.
2. Compares persisted state's version to the skin's current schema.
3. If mismatch, runs the skin's migration function (e.g., "v1 to v2: position field renamed from `pos` to `position`").
4. Migrated state hydrates the skin.

Skin removal (a skin unregistered from a build) preserves its meta-namespace in the workspace file. If re-added in a later build, state hydrates as expected. If never re-added, the meta data just sits unused.

---

## 13. Implementation phasing

A reasonable build order:

**Phase A — Skin scaffold.** Define `WorkspaceSkin`, `RegisteredSkin`, `SkinContext`, `SkinManifest`, `SkinCapabilities`, and the host-side skin-state handles. Implement the registry/composition layer and mount TripleEditor through it from the first usable shell.

**Phase B — Lift primitives.** Extract OneCalc-derived primitives into a shared crate. Wire them into TripleEditor as proof.

**Phase C — Second skin.** Implement CellView or OutlineTable. Force the design to handle a second registered/mountable skin (per-skin state, shared tree state, switching or side-by-side mounting). Iron out the shared-skin-state pattern.

**Phase D — Canvas.** Implement CanvasFlow. Most demanding for state model (positions per node, groups, layout modes). Tests the meta-namespace model under pressure.

**Phase E — Across.** Implement NodesAcross. Smaller scope than Canvas; useful to validate the primitives work in horizontal flow.

**Phase F — Specialty skins.** TemplateEditor, FormatEditor. These are smaller, less-frequently-active skins that test the Inspector category.

**Phase G — Polish.** Skin-switcher UX in chrome. Keyboard shortcuts. Skin discovery in command palette. Skin documentation.

After Phase G, the architecture is mature. Future skins, custom user skins, and richer pane composition layer on without further scaffold changes.

---

## 14. Cross-references

- [`REQUIREMENTS.md`](REQUIREMENTS.md) — the skin surfaces sections (§4) carry the per-skin functional requirements.
- [`TECHNICAL.md`](TECHNICAL.md) — §3.5 (LayoutState) is replaced by the SkinState model described here.
- [`META_NODES.md`](../model/META_NODES.md) — per-skin meta-namespaces are the canonical application of meta-nodes (alongside templates and formats).
- [`prototypes/`](prototypes/) — the eight mockups are the first eight skin implementations.
- [`TRACEABILITY.md`](TRACEABILITY.md) — detailed prototype-to-skin and skin-to-engine traceability.
- [`IMPLEMENTATION_MATRIX.md`](IMPLEMENTATION_MATRIX.md) — trace IDs and scenario cards that drive W003/W005/W006 implementation from the skin side.

---

## 15. Status

The skin architecture is the right factoring for DNA TreeCalc's UI given the empirical diversity of useful views. Adopting it from the start avoids the technical debt of repeatedly refactoring a single-front-end design as new view ideas arise.

The cost is one additional layer of abstraction (the `WorkspaceSkin` trait and primitive library) for an open-ended payoff (every future view is a new file, not a rewrite). For a product whose value proposition is "deep flexibility in how you reason about a calculation graph," this is exactly the right place to spend complexity budget.

Conditional formatting needs an engine-side design review (multi-rule, ordered, stop-if-true) to ensure the format-editing UX is honest. Listed as engine prereq.

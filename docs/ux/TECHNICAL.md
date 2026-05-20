# DNA TreeCalc — UX Technical Plan

This document covers the implementation and integration layer for DNA TreeCalc's UX. It assumes the conceptual model from [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md), the user-facing surface from [`REQUIREMENTS.md`](REQUIREMENTS.md), and the **skin architecture** from [`SKINS.md`](SKINS.md). It does not redesign the user-facing surface — it specifies how to build it.

> **Reframing 2026-05-19:** the eight existing UI mockups are not alternative layouts to choose between; they are **skins** — parallel front-ends to the same core, switchable at runtime, each with its own persisted state stored in dedicated meta-namespaces. The Winamp-skinning idiom is the right architectural metaphor. The earlier `LayoutMode`/`LayoutState` framing in §3.5 is superseded by the skin model. Read [`SKINS.md`](SKINS.md) for the full architecture before this document; the rest of this document is unchanged except for the layout-related sections.

The plan extends DNA OneCalc's existing tech stack rather than replacing it. Components proven in OneCalc (formula editor, bridge pattern, drill panel, completion, persistence) carry over unchanged or with minor extensions. They are now factored as **shared UI primitives** (skin architecture §3) consumed by individual skins. New components (tree outline, canvas, table editor, format editor, template editor) are additional primitives or are themselves the bodies of specialty skins.

---

## 1. Tech stack

Inherited from DNA OneCalc, unchanged baseline:

| Layer | Choice |
|---|---|
| UI framework | Leptos 0.8+ (Rust reactive) |
| Runtime target | CSR on WASM (browser); SSR/CLI for verification |
| DOM binding | `web-sys` 0.3 |
| Engine access | Direct via re-exported `oxfml_core`, `oxfunc_core`, and new `oxcalc-core` |
| Persistence (browser) | localStorage with explicit file save/load via the File System Access API or download/upload pattern |
| Persistence (native) | File I/O for `.dnatree` workspace files |
| Serialization | `serde_json` with `arbitrary_precision` (consistent with OneCalc's choice for numeric safety) |
| Styling | CSS design tokens, embedded `<style>` block (consistent with OneCalc) |
| Build | `wasm-pack` / `trunk` for browser; `cargo` for native verification host |

New dependencies for TreeCalc:

| Purpose | Crate / approach |
|---|---|
| Engine access (multi-node) | `oxcalc-core` re-export (new — currently only OneCalc re-exports OxFml directly) |
| Canvas layout | Custom — small canvas drawing layer over Leptos + a vector-graphics primitive set |
| Virtualized lists for tree outline | Custom — virtualization over Leptos signals; or a lightweight crate if a Leptos-friendly one exists |
| Drag-and-drop | `web-sys` Drag/Drop API via thin wrappers; no heavy library needed |
| UDF hosting (VBA + .xll) | Shared UDF-hosting core (see §2.4) — consumed, not reimplemented |

No Tauri, no Electron, no React/Vue. The TreeCalc shell is pure Leptos/WASM in browser; a parallel native CLI for verification (analogous to OneCalc's split).

### 1.1 UDF hosting (VBA and .xll) — consumed via a shared core

TreeCalc supports user-defined functions through two Excel-compatible mechanisms:

- **VBA UDFs** — hosted via OxVba (`oxvba_compiler`, `oxvba_host`, `oxvba_runtime`), as DnaOneCalc already does.
- **.xll native add-ins** — native code add-ins using the Excel C API from the Excel SDK.

**Intent:** both capabilities are developed **first in DNA OneCalc**, then a **shared core is extracted** that multiple hosts (OneCalc, TreeCalc, future DNA Calc) consume. TreeCalc does **not** reimplement UDF hosting — it depends on the shared core directly. This matches the "right stuff in the right repo" principle: UDF hosting is a cross-host concern, owned by a shared crate, consumed by hosts.

Practical consequence for TreeCalc planning: the UDF-hosting dependency is not on TreeCalc's critical path for v1 (the core tree/calc/skin work doesn't need it), and lands when the shared core is extracted from OneCalc. TreeCalc's formula evaluation routes UDF calls through the shared core the same way OneCalc does. The `.xll` native path is newer than the VBA path; both arrive in TreeCalc by consuming the shared core, not by per-host work.

---

## 2. Crate / module layout

### 2.1 Top-level repository structure

Following DnaOneCalc's structure:

```
DnaTreeCalc/
├── Cargo.toml                          # workspace manifest
├── README.md
├── src/
│   ├── dnatreecalc-host/               # the main host crate (analogous to dnaonecalc-host)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # WASM entry: mount_treecalc(element_id)
│   │       ├── main.rs                 # native CLI entry for verification host
│   │       ├── adapters/
│   │       │   ├── oxfml/              # reused as-is from OneCalc (formula editor bridge)
│   │       │   └── oxcalc/             # NEW: tree-substrate bridge
│   │       │       ├── bridge.rs       # OxCalcTreeBridge trait + request/response
│   │       │       ├── live_bridge.rs  # caching, recalc orchestration
│   │       │       ├── types.rs        # re-exports + UI projections
│   │       │       └── mod.rs
│   │       ├── app/
│   │       │   ├── reducer.rs          # state mutations (tree edits, node selection, layout)
│   │       │   ├── host_mount.rs       # bootstrap + bridge init
│   │       │   ├── intents.rs          # high-level actions
│   │       │   └── mod.rs
│   │       ├── state/
│   │       │   ├── types.rs            # TreeCalcHostState
│   │       │   ├── workspace.rs        # workspace tree, formats, templates
│   │       │   ├── selection.rs        # selection set, multi-select state
│   │       │   ├── skin_registry.rs    # active skin + installed set (per-skin state lives in tree meta-namespaces)
│   │       │   └── mod.rs
│   │       ├── ui/
│   │       │   ├── components/
│   │       │   │   ├── workspace_shell.rs   # top-level shell layout
│   │       │   │   ├── nav_rail.rs          # tree outline (left rail)
│   │       │   │   ├── tree_row.rs          # individual tree row
│   │       │   │   ├── node_editor.rs       # three-pane node editor layout
│   │       │   │   ├── value_detail.rs      # value rendering
│   │       │   │   ├── array_grid.rs        # virtualized array view
│   │       │   │   ├── table_editor.rs      # structured-table editor
│   │       │   │   ├── format_editor.rs     # format meta-child editor
│   │       │   │   ├── template_editor.rs   # template editing
│   │       │   │   ├── canvas.rs            # free-canvas layout
│   │       │   │   ├── outline_table.rs     # outline-table hybrid view
│   │       │   │   ├── notebook.rs          # notebook layout
│   │       │   │   ├── dependency_map.rs    # dependencies panel
│   │       │   │   ├── search.rs            # workspace search
│   │       │   │   ├── command_palette.rs   # Ctrl+. palette
│   │       │   │   └── ...
│   │       │   ├── editor/                  # REUSED from OneCalc — formula editor primitives
│   │       │   └── design_tokens/
│   │       │       └── theme.rs             # extended TreeCalc theme
│   │       ├── services/
│   │       │   ├── tree_view_model.rs       # state → tree-row projection
│   │       │   ├── selection_service.rs     # multi-select operations
│   │       │   ├── structural_edit.rs       # add/remove/rename/move ops
│   │       │   ├── template_sync.rs         # template instantiation, sync
│   │       │   ├── format_inheritance.rs    # walk-up format merging
│   │       │   ├── search_service.rs        # name/formula/value search
│   │       │   ├── command_registry.rs      # command palette entries
│   │       │   ├── import_excel.rs          # Excel import per spec §10
│   │       │   ├── export_excel.rs          # Excel export (bidirectional fidelity)
│   │       │   ├── live_edit.rs             # debounced bridge round-trips
│   │       │   └── ...
│   │       ├── persistence/
│   │       │   ├── workspace_storage.rs     # save/load .dnatree files
│   │       │   ├── workspace_format.rs      # the file format itself
│   │       │   ├── localstorage.rs          # browser autosave
│   │       │   └── ...
│   │       └── verification/                # native CLI verification host
│   │           ├── commands.rs              # verify-workspace, verify-formula, etc.
│   │           └── ...
│   └── dnatreecalc-shared/                  # shared types if needed (e.g., for the verification CLI)
└── docs/
    ├── APP_UX_TREECALC_SPEC.md              # the in-repo UX spec (mirrors docs/ux/REQUIREMENTS.md)
    └── ...
```

### 2.2 Reused from DnaOneCalc

The following modules transfer with no semantic changes:

- `ui/editor/` — the formula editor surface (commands, geometry, render projection, bracket matcher, etc.).
- `adapters/oxfml/` — the formula bridge.
- `services/completion_popup.rs` — completion handling.
- `services/formula_drill_audit.rs` — drill panel rendering.
- `services/live_edit.rs` — bridge orchestration (extended for tree context).
- Design tokens (extended with TreeCalc-specific tokens for tree rows, canvas, table grid).

These crates are likely structured as `oxfml-host-components` shared between OneCalc and TreeCalc, or vendored. The natural move is to factor them into a shared crate that both host applications depend on.

### 2.3 New for TreeCalc

The following are TreeCalc-specific:

- `adapters/oxcalc/` — tree-substrate bridge (analogous to OneCalc's `oxfml` adapter but for the multi-node engine).
- `ui/components/nav_rail.rs`, `tree_row.rs` — tree outline.
- `ui/components/canvas.rs` — free-canvas layout.
- `ui/components/outline_table.rs` — tree-table hybrid.
- `ui/components/notebook.rs` — notebook layout.
- `ui/components/table_editor.rs` — structured-table editor.
- `ui/components/format_editor.rs` — format meta editor.
- `ui/components/template_editor.rs` — template editing.
- `services/structural_edit.rs` — multi-node structural operations.
- `services/template_sync.rs` — template instantiation and sync.
- `services/format_inheritance.rs` — walk-up format merging.
- `services/import_excel.rs` — Excel-defined-name import per spec §10.

---

## 3. State model

### 3.1 Top-level state

```rust
pub struct TreeCalcHostState {
    pub workspace: WorkspaceState,
    pub selection: SelectionState,
    pub ui_chrome: UiChromeState,
    pub skins: SkinRegistryState,        // active skin + installed set (see §3.5); per-skin state lives in the tree's meta-namespaces
    pub capability_and_environment: CapabilityAndEnvironmentState,
    pub ambient_app_context: AmbientAppContext,
    pub command_palette: CommandPaletteState,
    pub search: SearchState,
    pub undo_redo: UndoRedoState,
}
```

### 3.2 Workspace state

```rust
pub struct WorkspaceState {
    pub file_handle: Option<WorkspaceFileHandle>,    // open file path or local-only
    pub dirty: bool,                                  // unsaved changes
    pub root: TreeNodeId,
    pub nodes: HashMap<TreeNodeId, TreeNodeState>,
    pub templates: TemplateRegistry,
    pub instance_links: HashMap<TreeNodeId, InstanceLink>,
    pub external_aliases: ExternalWorkspaceAliases,
    pub capability_profile_id: String,
    pub last_published_result: Option<OxCalcTreeRecalcResult>,
}

pub struct TreeNodeState {
    pub id: TreeNodeId,
    pub parent_id: Option<TreeNodeId>,
    pub sibling_index: usize,
    pub name: String,
    pub formula: String,                              // single content field: "" = Empty, leading `=` = formula, else literal constant (CORE_MODEL_SPEC §6)
    pub is_meta: bool,
    pub hidden: bool,
    pub layout_metadata: Option<NodeLayoutMetadata>,  // canvas position, etc.
    pub children: Vec<TreeNodeId>,                    // ordered
}
```

### 3.3 Selection state

```rust
pub struct SelectionState {
    pub primary: Option<TreeNodeId>,                  // the "focused" node
    pub selected: BTreeSet<TreeNodeId>,               // multi-select set
    pub last_anchor: Option<TreeNodeId>,              // for shift-click range selection
    pub navigation_history: Vec<TreeNodeId>,          // for back-button navigation
}
```

### 3.4 UI chrome state

```rust
pub struct UiChromeState {
    pub nav_rail_open: bool,
    pub nav_rail_width: f32,
    pub drill_panel_open: bool,
    pub dependency_panel_open: bool,
    pub format_editor_open: bool,
    pub template_editor_open: Option<TemplateId>,
    pub show_meta_nodes: bool,
    pub view_mode: ViewMode,                          // Edit / ReadOnly / Developer
    pub theme: ThemeChoice,
    pub modal_dialogs: VecDeque<ModalDialog>,         // queue of open modals
    pub toasts: VecDeque<Toast>,                      // notification stack
}
```

### 3.5 Skin state (superseding LayoutState)

The earlier `LayoutMode` / `LayoutState` design has been replaced by the skin architecture (see [`SKINS.md`](SKINS.md)). The TreeCalc host carries:

```rust
pub struct SkinRegistryState {
    /// Skin currently rendering the main UI.
    pub active_skin_id: SkinId,
    /// All registered (installed) skins, available to switch to.
    pub installed: Vec<Box<dyn WorkspaceSkin>>,
    /// User's default skin preference (loaded from workspace meta or app prefs).
    pub default_skin_id: SkinId,
}
```

Per-skin state lives in the workspace tree itself as meta-nodes under the `skins/<skin_id>` meta-subtree (see skin architecture §4). The state model has no global "layout choice" field; each skin owns its state, persisted alongside the workspace data. Shared cross-skin state (e.g., tree-collapse) lives under `skins/shared`.

This change pushes layout state out of the in-memory `TreeCalcHostState` and into the persisted tree, which is the right place for state that survives sessions, switches between skins, and respects the meta-node lifecycle.

### 3.6 Reactivity pattern

Follow OneCalc's pattern:

- Root state is a `RwSignal<TreeCalcHostState>`.
- Sub-views subscribe via `state.with()` for read access.
- Mutations go through reducer functions in `app/reducer.rs` that take the state and an intent, return new state.
- View-model projections via `Memo` for derived views (rendered tree rows, filtered search results, etc.).
- `Effect` for side effects (autosave, bridge calls, etc.).

The bridge pattern from OneCalc generalizes:

- `OxFmlEditorBridge` for per-node formula editing — unchanged from OneCalc.
- `OxCalcTreeBridge` for tree-level operations — new. Wraps `OxCalcTreeRuntimeFacade` calls.

---

## 4. OxCalc bridge pattern

### 4.1 Tree bridge interface

```rust
pub trait OxCalcTreeBridge {
    /// Submit a structural snapshot for evaluation.
    fn execute_recalc(&self, request: TreeRecalcRequest) -> TreeRecalcResult;

    /// Begin a transactional batch of structural edits.
    fn begin_transaction(&self) -> TransactionHandle;

    /// Commit a transaction; produces a publication candidate.
    fn commit_transaction(&self, handle: TransactionHandle) -> Result<TreeRecalcResult, TreeRecalcError>;

    /// Roll back a transaction.
    fn rollback_transaction(&self, handle: TransactionHandle);

    /// Subscribe to invalidation events for incremental UI updates.
    fn subscribe_invalidation(&self, callback: Box<dyn Fn(InvalidationEvent)>);
}
```

### 4.2 Request and result shapes

```rust
pub struct TreeRecalcRequest {
    pub structural_snapshot: StructuralSnapshot,      // current tree state
    pub formula_catalog: TreeFormulaCatalog,          // formulas per node
    pub published_values: HashMap<TreeNodeId, EvalValue>, // seed values for previously-clean nodes
    pub host_capability_snapshot: OxCalcTreeHostCapabilitySnapshot,
    pub runtime_policy: OxCalcTreeRuntimePolicy,
    pub candidate_result_id: String,
    pub publication_id: String,
}

pub struct TreeRecalcResult {
    pub run_state: TreeRunState,                      // Published / VerifiedClean / Rejected
    pub dependency_graph: TreeDependencyGraph,
    pub invalidation_closure: InvalidationClosure,
    pub evaluation_order: Vec<TreeNodeId>,
    pub runtime_effects: Vec<RuntimeEffect>,
    pub published_values: HashMap<TreeNodeId, PublishedValue>,
    pub node_states: HashMap<TreeNodeId, NodeCalcState>,
    pub diagnostics: Vec<TreeDiagnostic>,
}
```

### 4.3 Live bridge implementation

```rust
pub struct LiveOxCalcTreeBridge {
    runtime: OxCalcTreeRuntimeFacade,
    cache: TreeRecalcCache,                            // keyed by snapshot+catalog hash
    subscriptions: Vec<Box<dyn Fn(InvalidationEvent)>>,
}
```

Caching strategy:
- Per-node value cache survives across recalcs when the node's formula and inputs are unchanged.
- Invalidation closure from each recalc result tells the bridge which cache entries to drop.
- Transaction handle owns a snapshot delta; commit applies the delta to engine state.

### 4.4 Engine integration with formula editor

The OneCalc bridge edits a single formula at a time. For TreeCalc:

1. User edits a node's formula in the formula editor.
2. OneCalc-style `LiveOxfmlBridge` produces a bind result with diagnostics.
3. Bind result is fed to `LiveOxCalcTreeBridge`: "this formula now binds to these dependencies; please rebind / re-evaluate."
4. OxCalc-tree updates its catalog, computes the new dependency closure, recomputes affected nodes.
5. UI receives the updated value and renders.

This composition keeps the formula-editor surface unchanged but layers tree-level recalc on top.

---

## 5. Persistence

### 5.1 Workspace file format

A `.dnatree` file is a JSON document (or msgpack for size; decide on a flag). Schema:

```json
{
  "schema_version": 1,
  "capability_profile_id": "treecalc-v1",
  "cycle_config": {
    "profile": "cycle.non_iterative_stage1",
    "max_iterations": 100,
    "max_change": 0.001
  },
  "metadata": {
    "created_at": "...",
    "modified_at": "...",
    "host_version": "..."
  },
  "external_workspaces": {
    "aliases": [
      { "alias": "reports", "path": "./reports.dnatree" }
    ]
  },
  "tree": {
    "root": "<TreeNodeId>",
    "nodes": [
      {
        "id": "<TreeNodeId>",
        "parent_id": "<TreeNodeId or null>",
        "sibling_index": 0,
        "name": "Accounts",
        "formula": "",
        "is_meta": false,
        "hidden": false,
        "children": ["<TreeNodeId>", "..."],
        "layout_metadata": null
      },
      ...
    ]
  },
  "templates": [
    {
      "id": "templates.quarter_shape",
      "root_node_id": "<TreeNodeId>",
      "version": 7,
      "source_node_ids": ["<TemplateNodeId>", "..."]
    }
  ],
  "instance_links": [
    {
      "root_node_id": "<TreeNodeId>",
      "template_id": "templates.quarter_shape",
      "bound_version": 7,
      "node_map": [
        { "template_node_id": "<TemplateNodeId>", "instance_node_id": "<TreeNodeId>" }
      ]
    }
  ],
  "ui_layout": {
    "active_layout_per_subtree": {...},
    "canvas_positions": {...}
  }
}
```

### 5.2 Save / load flow

- **In-browser:** primary persistence is localStorage (auto-save key `dnatreecalc.workspace.v1`). Explicit "Save to disk" prompts a download. "Open from disk" uses file-input or File System Access API.
- **Native:** standard file I/O. Watch the file for external changes; prompt user on conflict.

### 5.3 Migration

Schema version bumped on format changes. Loader detects version and runs migration steps. Old workspaces opened in newer hosts: migrated in place (with backup). Newer workspaces opened in older hosts: refused with a clear error.

### 5.4 Backups

Auto-backup to a sibling `.dnatree.bak` file on each save. Configurable retention.

---

## 6. UI components — detailed integration

### 6.1 Tree outline (`nav_rail.rs`, `tree_row.rs`)

- Renders a virtualized list of `TreeRowView` items derived from `WorkspaceState`.
- `TreeRowView` carries: node id, name, depth, value summary, status icon, is-expanded, is-selected.
- Virtualization: only render rows in the viewport plus a buffer (e.g., 50 above and below).
- Lazy expansion: collapsed subtrees don't materialize TreeRowView for their descendants.
- Drag-and-drop: HTML5 drag API, with hit-testing for drop indicators (between rows = move-as-sibling; on row body = move-as-child).
- Inline rename: F2 swaps the row's name display for a text input; commit on Enter, cancel on Esc.

### 6.2 Formula editor (reused)

- Drops in unchanged from OneCalc.
- Bound to the currently-selected node's formula text.
- Bridge requests use the node's id as `formula_stable_id`.
- Switching nodes commits the previous (or prompts).

### 6.3 Value detail (`value_detail.rs`)

- Branches on the value type from `EvalValue`.
- Scalar: simple display with formatted text from the format-inheritance service.
- Array: delegates to `array_grid.rs`.
- Reference: shows resolved target with click-through.
- Table: delegates to `table_editor.rs`.
- Error: prominent error display with diagnostic context.

### 6.4 Array grid (`array_grid.rs`)

- Virtualized grid for arrays larger than ~1000 cells.
- Per-cell rendering uses `ArrayCellFormat` from OneCalc.
- Scroll position preserved on size changes.
- Lazy-fetch for arrays exceeding memory budget: bridge returns cell ranges on demand.
- Editable when the content is a constant (per-cell typing); read-only when it is a `=`-formula.

### 6.5 Canvas (`canvas.rs`)

- HTML5 canvas + SVG layer for connecting lines.
- Each node rendered as a card (a Leptos component positioned absolutely).
- Drag to reposition; position saved as `NodeLayoutMetadata`.
- Connections drawn between nodes that reference each other (computed from the dependency graph).
- Zoom and pan.

### 6.6 Outline table (`outline_table.rs`)

- A virtualized table component with sortable columns.
- Row per node; cells per attribute (name, formula, value summary, format summary, status).
- Editable cells for name and formula; read-only for value and status.
- Indentation in the name column indicates depth.
- Sub-tree expand/collapse via row-prefix triangles.

### 6.7 Notebook (`notebook.rs`)

- Vertical scroll of node cells.
- Each cell shows name + formula + value, with depth-based indentation or border treatment.
- Supports inline editing.
- Smaller / simpler than `outline_table.rs`; closer to Jupyter notebook style.

### 6.8 Table editor (`table_editor.rs`)

- For Table-valued nodes.
- Standard grid editor with column headers, optional totals row.
- Column-level formula editing (per cell evaluates the column formula in row context).
- Add/remove rows and columns.
- Structured-reference autocomplete (`[@Col]`, `[#Headers]`, etc.) using the existing OneCalc completion machinery extended with structured-ref tokens.

### 6.9 Format editor (`format_editor.rs`)

- Reads/writes the selected node's `Format` meta-child (creating it lazily).
- Property-by-property editors: number format with live preview, font, fill, conditional rules, data bars, icon sets.
- Each property can be a literal or a formula (toggle).
- "Inherited from..." indicator and "override" action when a format is inherited.

### 6.10 Template editor (`template_editor.rs`)

- Opens in a dedicated panel or modal.
- Renders the template's structure as a sub-tree outline.
- Same editing affordances as the main tree outline, scoped to the template.
- Side panel lists instances; click to navigate.
- "Sync" / "Validate" action triggers the template-sync service. The service uses the persisted template id mapping plus hidden rollout meta-node tags to diff the current instance against the current template on demand; it does not maintain detailed live template-state records while the user edits an instance.

### 6.11 Dependency map (`dependency_map.rs`)

- Two sub-views: list (depends-on / depended-on-by) and graph (optional).
- List view: simple categorized list of node references with click-to-navigate.
- Graph view: when invoked, renders a force-directed layout (or a hierarchical layout for shallow trees). Uses SVG; the graph is a separate canvas-like layer.

### 6.12 Search (`search.rs`)

- Inline search field invoked by Ctrl+F.
- Live-filter as the user types.
- Result list with click-to-navigate.
- Filter chips for the search scope (name / formula / value / all).

### 6.13 Command palette (`command_palette.rs`)

- Ctrl+. opens a centered modal.
- Fuzzy-search over `CommandRegistry`.
- Each command has: name, description, keyboard shortcut, action handler.
- Selectable with arrows; Enter to execute.

---

## 7. Performance considerations

### 7.1 Large trees

- Tree outline virtualizes aggressively (only visible rows render).
- Subtree state can be lazily loaded from persistence if the workspace file is large (only load the top-N levels eagerly, rest on demand).
- Workspace status overview (heatmap) provides a navigation aid for large workspaces.

### 7.2 Large arrays

- Array values rendered with virtualized grid.
- Cell formatting computed lazily per visible cell.
- For arrays exceeding ~1M cells, the bridge supports range queries (host requests slice [i..j]).

### 7.3 Frequent recalcs

- Bridge debouncing (per OneCalc pattern): formula edits trigger bridge requests at ~150ms intervals during typing.
- Cache hits avoid re-evaluation when the formula and inputs haven't changed.
- OxCalc's incremental evaluation (publish-only-changed values) limits work per recalc.

### 7.4 Memory budget

- Per-node memory: name + formula text + value (most are scalars or small arrays). Average node ~1 KB.
- Workspaces up to 100K nodes: ~100 MB workspace state. Manageable in browser memory.
- Workspaces beyond 100K nodes: consider streaming load and lazy materialization.

### 7.5 UI responsiveness

- All bridge calls async / non-blocking.
- Recalcs trigger UI updates via `Effect`, not synchronous within reducers.
- Long operations (template sync, Excel import) show progress and remain cancellable.

---

## 8. New low-level requirements (build or borrow)

### 8.1 Components to build

- Virtualized tree outline (no off-the-shelf Leptos component fits cleanly).
- Free canvas with drag-positioning and connection-line rendering.
- Format editor (number format, font, etc.) — Leptos UI bindings to a format-code parser (OxFml owns the parser).
- Structured-table editor for Table-valued nodes.
- Template editor.

### 8.2 Components to borrow / reuse

- Formula editor surface (DnaOneCalc — direct reuse).
- Bridge pattern primitives (DnaOneCalc — extended).
- Drill panel (DnaOneCalc — direct reuse).
- Completion / signature help (DnaOneCalc — direct reuse).
- Diagnostic squiggle rendering (DnaOneCalc — direct reuse).
- Design token theme system (DnaOneCalc — extended with TreeCalc-specific tokens).

### 8.3 Engine prerequisites

From the engine prerequisites in [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) §6, TreeCalc needs the sibling lanes to provide:

1. Unified reference/range abstraction: tree references and opaque tree-reference arrays must coexist with Excel grid references/ranges, preserving reference identity for reference-sensitive functions and dereferencing to values for ordinary functions.
2. `SelfNode` base variant in `RelativePath`.
3. Set-membership dependency edge type.
4. Reference-array literals.
5. Cross-workspace orchestration.
6. Structural-edit semantics resolved.
7. Profile-aware `INDIRECT`.
8. Transactional batch editing.
9. `is_meta` per-node attribute.
10. Conditional-formatting rule semantics needed by the format surface.
11. Constant-entry classification on the TreeCalc channel, including `""` -> `Empty`, with the formula branch parsing tree paths under `treecalc-v1`.
12. Circular-reference cycle profiles and iterative bounds carried in the host/recalc compatibility basis.

The host UX depends on items 1, 2, 8, and 9 directly; the others are formula-language extensions visible through the editor.

---

## 9. Cross-references and coherence check

| Requirement (UX doc §) | Implementation (this doc §) |
|---|---|
| §2.1 Workspace shell chrome | §6 components — workspace_shell.rs, status foot inside it |
| §2.2 Tree outline | §6.1 nav_rail.rs, tree_row.rs |
| §2.3 Formula editor | §6.2 reused from OneCalc |
| §2.4 Value detail | §6.3 value_detail.rs |
| §2.5 Drill panel | reused from OneCalc, no new component |
| §2.6 Dependency map | §6.11 dependency_map.rs |
| §2.7 Table value editor | §6.8 table_editor.rs |
| §2.8 Format editor | §6.9 format_editor.rs |
| §2.9 Template editor | §6.10 template_editor.rs |
| §2.10 Diagnostics | inline in editor; reused from OneCalc |
| §2.11 Workspace-level views | individual components — settings panel, templates registry list |
| §3 Editing actions | §3 state model + §6 components; reducer in app/reducer.rs |
| §3.10 Keybindings | command_palette + per-component key handlers |
| §4 Layout modes (skins) | §3.5 SkinRegistryState + per-skin meta-namespaces; SKINS.md; §6.5/§6.6/§6.7 skin-specific components |
| §5 Specific patterns | various components (array_grid for §5.1; tree_row + template_editor for §5.4; format_editor for §5.5) |
| §6 Adaptive behaviors | per-skin state in meta-namespaces (§3.5) + adaptive rendering selection within a skin |
| §7 Cross-cutting | §5 persistence; §7 performance; §3 state model |

| Spec section (DESIGN doc §) | Tech enables |
|---|---|
| §2 Core model | §3.2 WorkspaceState, §6.1 tree outline |
| §3 Reference syntax | §6.2 formula editor binding |
| §4 Capability profile | §3.5 CapabilityAndEnvironmentState; profile passed in bridge request |
| §5 Excel alignment | inherited from OneCalc baseline |
| §6 Engine prereqs | §8.3 listed; bridge enables once engine ships them |
| §7b Templates | §6.10 template_editor + services/template_sync.rs |
| §10 Excel import | §6 components — import_excel.rs service + import preview UI |

---

## 10. Phasing / build order

A natural build order based on dependencies:

1. **Phase 0 — Foundation.** Factor reusable OneCalc components into a shared crate. Verify TreeCalc consumes them.
2. **Phase 1 — Tree shell.** Workspace state, tree outline (nav rail), basic node creation/deletion/rename. Three-pane editor with formula editor reused. Single-node evaluation via bridge. Persistence via localStorage and explicit file save.
3. **Phase 2 — Multi-node calc.** OxCalc bridge integration. Recalc and dependency graph in place. Status display per node. Reference resolution with walk-up.
4. **Phase 3 — Editing breadth.** Multi-select, move, drag-and-drop. Rename-propagation prompt. Search.
5. **Phase 4 — Layout alternatives.** Outline-table view. Notebook view. Adaptive layout selection.
6. **Phase 5 — Meta-nodes and formatting.** is_meta flag plumbing. Format editor. Format inheritance walking.
7. **Phase 6 — Templates.** Template editor. Instance link tracking, hidden rollout tags, and on-demand validate/sync (initially N individual edits; later transactional once OxCalc supports it).
8. **Phase 7 — Tables.** Table editor for Table-valued nodes. Structured-reference syntax in the editor.
9. **Phase 8 — Canvas.** Free-canvas layout. Connection rendering. Drag-positioning.
10. **Phase 9 — Excel I/O.** Import per spec §10. Export with bidirectional fidelity caveats. Cross-workspace alias management.
11. **Phase 10 — Polish.** Accessibility, theming, command palette, undo/redo refinement, performance tuning for large workspaces.

Each phase ships an end-to-end usable product, just with progressively more capability.

---

## 11. Verification host

Parallel to the web shell, a CLI verification host (analogous to DnaOneCalc's `verify-formula`, `verify-xml-cell`, etc.):

```
verify-workspace --workspace <path>                  # full recalc; output published values
verify-node --workspace <path> --node-path <path>    # eval single node; output value + walk-tree
verify-template --workspace <path> --template-id <id># validate template + instances
verify-excel-import --xlsx <path>                    # import + report any incompatibilities
verify-roundtrip --workspace <path>                  # save to Excel, load back, compare
```

Same crate hierarchy; native target builds the CLI, WASM target builds the web shell.

---

## 12. Outstanding tech-level decisions

A handful of choices the technical plan defers:

1. **Workspace file format**: JSON vs. msgpack. JSON is human-readable and debuggable; msgpack is compact and fast. Pick JSON for v1; msgpack as future optimization.
2. **Canvas implementation**: pure HTML/CSS positioning vs. canvas drawing API vs. SVG. SVG is most likely (good Leptos integration, supports drawing connection lines, exportable).
3. **Tree row drag-drop library**: roll own with `web-sys` Drag API or pull in a lightweight crate. Roll own — the drag-drop interactions are specific to the tree-row use case.
4. **Theme palette**: extend OneCalc's beige palette or design TreeCalc-specific theme. Extend; the visual continuity helps users moving between products.
5. **WebAssembly bundle size**: the engine crates are substantial; bundle splitting may help startup. Defer until measured.
6. **Service-worker / offline mode**: nice-to-have; defer.

---

## 13. Status

This technical plan covers all components, services, and integration points required to build the UX specified in [`REQUIREMENTS.md`](REQUIREMENTS.md). It builds on the proven DnaOneCalc tech stack with minimum invention of new infrastructure.

The phasing in §10 gives a buildable path from empty TreeCalc to full-featured product. Each phase delivers a usable subset.

Engine prerequisites (§8.3) are tracked as local spec content and raised through targeted handovers in [`../handovers/`](../handovers/) when cross-repo work is needed. TreeCalc UX can begin once the foundational items (multi-node bridge, is_meta flag) are available, with later items unblocking later phases.

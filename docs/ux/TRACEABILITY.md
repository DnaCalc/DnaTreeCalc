# DNA TreeCalc - UX Prototype Traceability

Status: Living traceability spec for the current prototype set.

This document connects the visual prototypes to the skin contract, host internals, DnaTreeCalc services, and OxFml/OxCalc calls. It exists because the prototypes are no longer "layout options"; they are concrete slices of a skin system. A feature shown in a prototype should be traceable from visible control to state owner, intent path, engine interaction, and render update.

Read with:

- [`REQUIREMENTS.md`](REQUIREMENTS.md) for user-facing requirements.
- [`SKINS.md`](SKINS.md) for the skin contract.
- [`TECHNICAL.md`](TECHNICAL.md) for module layout and direct-context shapes.
- [`prototypes/`](prototypes/) for visual sketches.
- [`IMPLEMENTATION_MATRIX.md`](IMPLEMENTATION_MATRIX.md) for trace IDs, scenario cards, contracts, and harness expectations derived from this map.

## 1. Traceability Rule

Every prototype feature must land in one of these lanes:

| Lane | Owner | Persistence | OxCalc involvement | Examples |
|---|---|---|---|---|
| Calculation workspace | DnaTreeCalc host, via `WorkspaceIntent` | regular tree nodes in `.dnatree` | yes, for bind/recalc/invalidation | node formula, node name, node move, delete |
| Host facade state | DnaTreeCalc host state | host state or `skins.shared` meta-node subtree | no | selection, recent selections, pinned nodes, meta visibility |
| Per-skin state | mounted skin through host `SkinStateHandle<S>` | `skins.<skin_id>` meta-node subtree | no | canvas positions, zoom, column widths, expanded array rows |
| Host meta data | DnaTreeCalc host services | meta-node subtree | usually no; sync may generate regular edits | `Format`, `Templates`, rollout tags, workspace config |
| Engine result state | OxCalc result published into host state | optionally cached in workspace/session | source is OxCalc | values, calc state, dependency graph, diagnostics |
| Visual-only style | skin/theme/component CSS | skin code/theme tokens | no | panel colors, font pairing, density, card rhythm |

Skins never call OxCalc directly. A skin either:

1. reads host signals and renders;
2. updates its own host-managed skin state;
3. updates shared facade state; or
4. dispatches a typed `WorkspaceIntent`.

The dispatcher decides whether the request is a cheap host update, a meta-node write, a template orchestration, or an OxCalc context call.

The current executable harness for this rule is the Rust programmable skin in
`src/dnatreecalc-host/tests/programmable_skin_ir.rs`. It mounts through the
normal `WorkspaceSkin` contract, drives `WorkspaceIntent` from test code, and
reads only `WorkspaceState`, selection, and shared skin state. Use it for
product behavior that should be proven from outside the host/session adapter.

## 2. Prototype-to-Skin Map

| Prototype | Skin or mount role | Primary primitives | Skin state | Shared state | Workspace reads | Workspace writes | Engine path |
|---|---|---|---|---|---|---|---|
| `01_workspace_shell.html` | `triple-editor` in `main` slot | `ContextStrip`, `TreeRow`, `FormulaEditor`, `ValueDisplay`, `DrillPanel`, `StatusFoot` | nav width, value/drill pane sizes, drill open, developer/detail mode | selected node, tree collapse, recent selections, pins | node tree, formula text, effective value, format, calc state, dependency counts, drill trace | formula edit, rename, insert/delete/move from tree, selection | formula/content edits go OxFml -> OxCalc; selection and pane changes do not |
| `02_array_value.html` | array render state inside `triple-editor` or any value-detail skin | `FormulaEditor`, `ArrayGrid`, `ValueToolbar`, `StatusFoot` | array grid scroll, column widths, local expansion/detail options | pinned values, selected node | array value shape/cells, cell formats, shape delta, stats | constant-array edits when node content is a constant; pin/export/show-in-canvas actions | formula edits and constant-array edits recalc; pin/show-in-canvas are host/skin state |
| `03_template_editor.html` | `template-editor` in `main` or inspector slot | `TemplateTree`, `InstanceList`, `MetaTreeRow`, `StatusFoot` | selected template, expanded template rows, selected instance/filter | selection, meta visibility | template meta-subtree, derived `TemplateIndex`, rollout tags, instance status | edit template meta-subtree; sync/instantiate/detach via template intents | editing template definition itself is calc-ignored; instantiation/sync generates regular structural edits and then OxCalc recalc |
| `04_outline_table.html` | `outline-table` in `main` slot | `TreeTable`, `TreeRow`, `FormulaCell`, `ValueCell`, `FormatSummaryCell`, `StatusCell` | column order/widths, sort/filter, scroll, visible columns | selected node, tree collapse | regular nodes in visible scope, formulas, values, formats, calc state, dependency counts, template tags | inline name/formula edits, reorder, collapse/filter/sort | name/formula edits can rebind/recalc; sort/filter/collapse are host or skin state |
| `05_format_editor.html` | `format-editor` inspector or main slot | `FormatPropertyEditor`, `LiveFormatPreview`, `CfRuleList`, `StatusFoot` | selected property, section expansion, preview options | selected node | selected node value, inherited/effective format, format meta-child, current calc value | format meta-child edits; optional computed format expressions | literal format edits do not call OxCalc; computed format expressions are host-render-time evaluation and must not join the node dependency graph unless explicitly designed |
| `06_excel_style_cell.html` | `cell-view` in `main` slot | `FormulaBar`, `CellList`, `ArrayInlineExpansion`, `StatusFoot` | expanded arrays, scroll, row heights, formula-bar expansion | selected node, tree collapse | tree order projection, values, formulas, array value for expanded row | formula edit, constant edit, selection, expand/collapse | formula/constant edits recalc; row expansion and navigation do not |
| `07_nodes_across.html` | `nodes-across` in `main` slot | `FormulaBar`, `NodeColumn`, `ArrayColumn`, `WireRenderer`, `StatusFoot` | visible scope, horizontal scroll, column widths, focused column | selected node | dependency graph, formulas, values, calc state for scope | formula edits from selected column, column sizing/scope changes | formula edits recalc; dependency arrows are read from OxCalc graph and rerendered |
| `08_canvas_flow.html` | `canvas-flow` in `main` slot | `FormulaBar`, `NodeCard`, `WireRenderer`, `LassoSelection`, `GroupHandle`, `ZoomControls`, `Minimap` | node positions, groups, collapsed groups, zoom, pan, layout mode, routing mode | selected node(s), pins/recent selections | dependency graph, node values, formulas, calc states, effective formats | formula edit, drag positions, group/ungroup, promote group to template, auto-layout | formula edit and promote/sync paths may recalc; drag/pan/zoom/routing/auto-layout do not |

## 3. Feature Ownership Matrix

| Visible feature | Prototype source | Primitive/component | State owner | Intent or API | DnaTreeCalc service | OxFml/OxCalc boundary | Render update source |
|---|---|---|---|---|---|---|---|
| Filename/profile/recalc strip | 01-08 | `ContextStrip` | `UiChromeState` plus `WorkspaceState` | none or app command | workspace/session services | reads last OxCalc result only | host signal |
| Skin tabs/pills | 04, 06-08 | `SkinSwitcher` | `SkinRegistryState` | mount/switch slot command | skin registry | none | skin mount/unmount |
| Tree row | 01-06 | `TreeRow` | workspace + shared collapse | `SelectNode`, `RenameNode`, `MoveNode`, shared collapse update | tree view model, structural edit | rename/move may rebind/recalc | workspace and shared-state signals |
| Meta visibility toggle | 01-03 | `MetaToggle` | host facade state | host chrome/shared state update | tree view model filters | none | tree projection recomputes |
| Formula editor panel | 01-05 | `FormulaEditor` | selected node + editor local draft | `EditFormula` on commit; live bind request while editing | live edit service | OxFml parse/bind; OxCalc rebind/recalc on accepted edit | diagnostics, workspace value/calc signals |
| Formula bar | 06-08 | `FormulaBar` | selected node + editor local draft | `EditFormula` | live edit service | same as FormulaEditor | workspace value/calc signals |
| Reference hover/resolution | 01, 03-08 | editor token hover | editor local state | read-only bind query | OxFml editor integration | OxFml bind; no publication | hover overlay |
| Value hero/scalar | 01, 04-08 | `ValueDisplay` | none beyond selected node | none | format resolver | reads published OxCalc value | workspace value signal |
| Array grid | 02, 06, 07 | `ArrayGrid` | grid viewport/scroll/widths | constant-array edit if editable | array view model, format resolver | reads OxCalc array; edits recalc only when node content is constant | value shape/cell signal plus grid state |
| Shape-change indicator | 02, 06 | `ArrayShapeBadge` | view model cache | none | value diff service | compares prior/current OxCalc value shape | workspace value signal |
| Diff highlight | 02 | `ArrayGrid` | transient render state | none | value diff service | no engine call | prior/current value diff |
| Pin value | 02 | `PinButton` | `skins.shared.pinned` | shared state update | shared skin-state service | none | shared state signal |
| Export value | 02 | `ExportButton` | no semantic state | export command | export service | may use published value only | toast/progress |
| Show in canvas | 02 | `ShowInCanvasButton` | canvas skin state + mount slot | skin switch/mount plus canvas focus | skin registry, canvas state | none | mounted skin + canvas state |
| Drill tree | 01 | `DrillPanel` | panel open/details | trace request | drill/audit service | OxFml prepared-call trace; value source from OxCalc | trace result signal |
| Dependency counts | 01, 04 | `DependencySummary` | none | none | dependency projection service | OxCalc dependency graph | workspace graph signal |
| Dependency wires | 07, 08 | `WireRenderer` | routing mode + measured positions | none | graph projection service | reads OxCalc dependency graph | graph signal plus layout measurements |
| Outline-table sort/filter | 04 | `TreeTable` | `OutlineTableState` | skin-state update | table projection service | none | skin state + workspace projection |
| Inline formula cell | 04 | `FormulaCell` | editor local draft | `EditFormula` | live edit service | OxFml/OxCalc | workspace signal |
| Format summary cell | 04 | `FormatSummaryCell` | none | none | format resolver | none | format meta signal |
| Format property editor | 05 | `FormatPropertyEditor` | selected property/section | `SetFormatProperty`, `AddCfRule`, etc. | format service | literal edits no OxCalc; computed format TBD host evaluation | format meta signal |
| Live format preview | 05 | `LiveFormatPreview` | preview options | none | format resolver + optional computed format evaluator | reads selected node value; computed properties are host-render-time | value and format signals |
| Conditional-format rule list | 05 | `CfRuleList` | selection/order UI | `AddCfRule`, `RemoveCfRule`, `ReorderCfRules` | format service | semantic target is Excel-aligned CF, engine/library support required | format meta signal |
| Template tree | 03 | `TemplateTree` | expanded rows, selected template row | `EditTemplateStructure` | template service | template body ignored by OxCalc until instantiated/synced | template meta signal |
| Instance list | 03 | `InstanceList` | selected instance/filter | select/navigate; validate/sync | template index/service | sync emits structural edits that recalc | template index + workspace signal |
| Sync to instances | 03 | action button | no long-lived UI state | `SyncTemplateToInstances` | template sync service | structural batch -> OxCalc transaction when available | workspace/template index signal |
| Promote to template | 08 | `GroupHandle` | canvas group state consumed | `PromoteToTemplate` | template service | creates template meta-subtree; no recalc unless regular structure changes | template index + workspace signal |
| Canvas drag | 08 | `NodeCard` drag wrapper | `CanvasFlowState.positions` | skin-state update | canvas state service | none | canvas state signal |
| Canvas lasso | 08 | `LassoSelection` | transient drag + selection | `SelectMany` | selection service | none | selection signal |
| Canvas group/collapse | 08 | `GroupHandle` | `CanvasFlowState.groups` | skin-state update | canvas state service | none | canvas state signal |
| Canvas auto-layout | 08 | layout toolbar | `CanvasFlowState.positions/layout_mode` | skin-state update after algorithm | canvas layout service | reads dependency graph only | graph + canvas state signals |
| Zoom/pan/minimap | 08 | `ZoomControls`, `Minimap` | `CanvasFlowState.zoom/pan` | skin-state update | canvas viewport service | none | canvas state + ResizeObserver |
| Status foot shortcuts | 01-08 | `StatusFoot` | chrome + skin hint provider | command palette actions | command registry | depends on command | host/skin signals |
| Search/goto | 01 index surface | `Search`, `CommandPalette` | search state | `SelectNode` on result | search service | none for name search; formula search reads text | search + selection signals |
| Resize/split panes | implied by skin composition | `WorkspaceShell`, `PaneHost` | `SkinRegistryState.mounted`, per-skin pane sizes | mount/update slot, skin-state update | skin registry, shell layout service | none | ResizeObserver + skin state |

## 4. Flow Traces

### F1. Formula textbox to OxCalc publication

1. User focuses `FormulaEditor` or `FormulaBar` for selected node `N`.
2. Editor holds a local draft string. No workspace mutation is made for every keystroke unless the live-edit policy accepts it.
3. Editor asks OxFml for parse/bind diagnostics using `formula_stable_id = N`.
4. OxFml returns syntax runs, completions, signature help, reference resolution, and diagnostics.
5. On commit or debounced accepted live edit, the skin dispatches `WorkspaceIntent::EditFormula { node: N, formula }`.
6. DnaTreeCalc reducer validates the selected node is regular and calls `OxCalcTreeContext::set_node_formula_text`.
7. The host/session calls `OxCalcTreeContext::recalculate`.
8. OxCalc rebinding/evaluation computes dependency graph changes, invalidation closure, node states, diagnostics, and published values.
9. DnaTreeCalc publishes the returned views/outcome into UI projections and calc-state signals.
10. Mounted skins rerender any subscribed primitive: value displays, status pips, dependency counts, wires, drill availability, array grids, and status foot.
11. Autosave observes dirty state and persists the updated formula text plus any changed host state.

### F2. Constant entry and empty-node behavior

1. User types into a formula/value entry surface for node `N`.
2. If the entry text is blank, DnaTreeCalc stores blank node content; the node is empty and has the `Empty` value.
3. If the text starts with `=`, it follows F1 as a formula.
4. If the text does not start with `=`, the same `EditFormula` intent stores that text as the node content string.
5. OxFml classifies the no-equals content as an Excel-style literal constant during bind/evaluation.
6. OxCalc publishes the typed value. It cannot publish the special `Empty` value as a formula result; formula results may be empty string, which is different.

### F3. Tree rename or move

1. User renames in a tree row, outline-table cell, or command palette action.
2. Skin dispatches `RenameNode` or `MoveNode`.
3. DnaTreeCalc structural edit service computes affected references and prompts for propagation/break/cancel when needed.
4. Accepted edit becomes a structural snapshot delta. When OxCalc transactions exist, the whole edit is one transaction; until then, the host groups sequential edits in undo.
5. OxCalc rebinds affected formulas and publishes new dependency/value/calc state.
6. Tree rows, outline rows, formula reference highlighting, dependency wires, and status badges update from the new workspace and graph signals.
7. Per-skin state keyed by node id remains valid across rename/move; state keyed by path must be recomputed or avoided.

### F4. Array value shape change

1. A formula edit, external value update, or upstream dependency change causes node `A` to recalc.
2. OxCalc publishes a new array value for `A`, including the new shape.
3. DnaTreeCalc value projection compares prior shape/cells with the new value for UI diff metadata.
4. `ArrayGrid` receives the new shape and visible cell slice.
5. Grid scroll and column width state remain in the skin state; the grid does not jump just because the array grew.
6. Changed rows/cells highlight transiently; tree row summaries and status foot update immediately.
7. For very large arrays, the visible range query is host/OxCalc-mediated; skins still see a virtualized `ArrayGrid` model, not raw engine plumbing.

### F5. Format property edit

1. User edits a property in `FormatEditor`.
2. Skin dispatches `SetFormatProperty` or a CF-rule intent.
3. DnaTreeCalc creates or edits the selected node's `Format` meta-child.
4. Because `Format` is meta, OxCalc does not bind or evaluate that subtree.
5. `FormatResolver` recomputes the effective format by walking ancestors and local overrides.
6. Value renderers in all mounted skins receive the new effective format and rerender the data region.
7. Literal format properties end here. Computed format properties are a design extension: the host may evaluate them as render-time decoration against the selected node value, but they must stay outside the normal node dependency graph unless a later design explicitly promotes them.

### F6. Template edit, instantiate, and sync

1. `TemplateEditor` edits a template meta-subtree. The body formulas are stored as text and not bound.
2. DnaTreeCalc updates the template meta-subtree and derived `TemplateIndex`.
3. Existing instances do not change until validate/sync is requested.
4. On sync, the template service uses template id/version/source-node mapping plus hidden rollout tags to diff the instance subtree against the current template.
5. Accepted changes become regular structural/content edits against instance nodes.
6. Those regular edits go through the dispatcher and OxCalc context like any other structural edit.
7. The instance list and tree/outline/canvas badges update from the new template index and workspace signals.

### F7. Canvas drag, grouping, and auto-layout

1. User drags a node card or lasso-selects a group in `canvas-flow`.
2. Selection changes use `SelectNode` / `SelectMany`; canvas-specific positions/groups use `cx.state.update(...)`.
3. DnaTreeCalc persists positions/groups under `skins.canvas-flow`.
4. No OxCalc call occurs for pan, zoom, drag, grouping, routing mode, or manual layout.
5. Wires are redrawn from the OxCalc dependency graph plus current card geometry.
6. Auto-layout reads the dependency graph and current canvas state, computes new positions, and writes positions back to `CanvasFlowState`.
7. Promote-to-template is different: it dispatches `PromoteToTemplate`, which creates template meta data and may later generate structural edits when instances are created or synced.

### F8. Skin switch and split-pane composition

1. User chooses a skin in the context strip or opens an inspector beside the main editor.
2. `SkinRegistryState.mounted` changes for the focused mount slot.
3. Outgoing `SkinHandle.on_deactivate` flushes pending state.
4. Host hydrates the incoming skin's typed state from `skins.<skin_id>` and builds `SkinContext<S>`.
5. Host calls `skin.mount(cx)` and mounts the returned view.
6. Workspace, selection, calc state, format resolver, and shared skin state survive the switch.
7. No OxCalc rebind/recalc happens merely because a skin changed.

### F9. OxCalc invalidation to display update

1. OxCalc reports invalidation/evaluation/publish events through `OxCalcTreeContext`.
2. DnaTreeCalc updates calc-state signals first when useful (`dirty`, `evaluating`, `error`, `cycle-blocked`), then publishes values and graph when the candidate result is accepted.
3. Tree rows update pips and value summaries.
4. Formula editors keep their local draft unless the accepted edit belongs to the same node and policy says to reconcile.
5. Value renderers update scalar/array/table/error displays.
6. Dependency summaries and graph/wire renderers update from the new dependency graph.
7. Canvas and nodes-across may rearrange only if their skin policy says the changed graph/value shape triggers auto-layout; otherwise they preserve user-authored positions and scroll.

### F10. Resize to display rearrange

1. Browser/container resize fires a shell `ResizeObserver`.
2. `WorkspaceShell` updates mount-slot dimensions and passes container metrics to mounted skins.
3. Each skin chooses its renderer adaptation: hide/show panels, virtualize more aggressively, compress columns, move inspector from side pane to stacked pane, or adjust canvas viewport.
4. Persistent user-controlled sizes go to skin state; transient measurements stay in component-local state.
5. No OxCalc call occurs for resize.
6. If resize exposes a new array/canvas/table viewport range, the host may ask for more already-published value slices; this is data access, not recalculation.

### F11. External streaming value update

1. External connector pushes a value update or invalidation into the DnaTreeCalc host session.
2. Host sends `UpdateExternalValue` or `InvalidateExternal`.
3. OxCalc invalidates dependents and computes the affected publication candidate.
4. DnaTreeCalc publishes updated values/calc states into workspace signals.
5. Skins observe the same signals as for ordinary formula edits. No skin-specific RTD mode exists.

## 5. State Ownership Checklist

| Data or UI fact | Canonical owner | Keyed by | Persisted where | Allowed writers | Recalc effect |
|---|---|---|---|---|---|
| Node name | regular tree node | `TreeNodeId` | `.dnatree` tree | structural edit service | rebind/recalc as needed |
| Node formula/content text | regular tree node | `TreeNodeId` | `.dnatree` tree | formula editor via dispatcher | bind/recalc |
| Node computed value | OxCalc published result | `TreeNodeId` | cache/session or saved value if chosen | OxCalc context only | source result |
| Node calc state | OxCalc published result | `TreeNodeId` | session/result cache | OxCalc context only | source result |
| Dependency graph | OxCalc published result | node ids/edges | session/result cache | OxCalc context only | source result |
| Selection | DnaTreeCalc host | node ids | session and/or `skins.shared` | skins via selection intents | none |
| Collapse state | shared skin state | node ids | `skins.shared` meta | skins via shared state handle | none |
| Pins/recent selections | shared skin state | node ids | `skins.shared` meta | skins via shared state handle | none |
| Canvas positions | canvas skin state | node ids | `skins.canvas-flow` meta | canvas skin via state handle | none |
| Canvas groups | canvas skin state | group id -> node ids | `skins.canvas-flow` meta | canvas skin via state handle | none |
| Table/outline column widths | skin state | column ids | skin meta namespace | owning skin via state handle | none |
| Expanded array rows | skin state | node ids | skin meta namespace | owning skin via state handle | none |
| Format properties | DnaTreeCalc format meta service | node id/property path | `Format` meta-child | format intents | none for literal format |
| Template definitions | DnaTreeCalc template service | template id/root id | template meta-subtrees | template intents | none until instantiated/synced |
| Template rollout tags | DnaTreeCalc template service | instance node ids | hidden meta-node children | template service | none |
| Export/import manifests | DnaTreeCalc interop service | workspace/import/export id | workspace metadata/artifacts | interop service | import may create regular edits |
| Skin theme tokens | skin/theme code | skin id | app config or skin bundle | skin/theme author | none |

## 6. Prototype Obligations

### 6.1 TripleEditor obligations (`01`, plus `02` as value state)

- The nav rail must be a projection over `WorkspaceState` plus shared collapse/meta visibility.
- Formula panel must reuse OneCalc editor primitives and not introduce a second formula model.
- Value panel must use the same `ValueDisplay`/`ArrayGrid` primitives that other skins use.
- Drill panel must be lazy because prepared-call traces can be expensive.
- Pane widths/open flags are `TripleEditorState`, not `UiChromeState`.
- Status pips and dependency counts come from OxCalc result projections.

### 6.2 TemplateEditor obligations (`03`)

- Template body rows are meta-node rows. They are not bound while in the template.
- The instance panel is driven by `TemplateIndex`, derived from meta-subtrees and rollout tags.
- `Sync to instances` is an explicit action; template edits do not silently mutate instances.
- Template formulas shown in the editor must still use formula syntax highlighting, but diagnostics should be template-aware: parse/text diagnostics are useful; live binding is not the normal state until a simulated instance context exists.
- The status line must continue to say "not bound / ignored from calculation" for template definitions.

### 6.3 OutlineTable obligations (`04`)

- The table is a skin over the same regular tree, not a separate table model.
- Sorting/filtering affects display order only unless the user explicitly performs a structural reorder.
- Editable formula/name cells dispatch the same intents as TripleEditor.
- Format/status/dependency columns are read-only projections.
- Template badges and override markers come from rollout tags and `TemplateIndex`.

### 6.4 FormatEditor obligations (`05`)

- It writes `Format` meta-children through DnaTreeCalc, not OxCalc.
- Effective/inherited values are always resolved by `FormatResolver`, never by the skin walking ancestors ad hoc.
- User data format wins inside value-rendering regions across all skins.
- Skin theme remains the frame around the value.
- Computed format properties need a careful later design. The current UI may show them as intended affordances, but implementation must keep the calculation boundary explicit.

### 6.5 CellView obligations (`06`)

- The rows are a cell-like rendering of tree nodes, not a grid coordinate system.
- Formula bar edits the selected node's single content string.
- Inline array expansion is a skin state keyed by node id.
- Keyboard behavior should feel Excel-adjacent while preserving tree navigation semantics.
- The skin switcher controls mount slots; switching to CellView must not recalc.

### 6.6 NodesAcross obligations (`07`)

- Columns are node projections in a selected scope.
- Horizontal flow order should default from dependency/topological information, but user column widths/scroll are skin state.
- Arrows are visual dependency projections from the OxCalc graph.
- Array columns reuse `ArrayGrid`/array rendering logic rather than creating a separate array semantics.
- Formula edits follow the FormulaBar flow.

### 6.7 CanvasFlow obligations (`08`)

- Card position, groups, zoom, pan, routing, and layout mode live in `CanvasFlowState`.
- Dependency wires are graph projections and must update when OxCalc publishes a new graph.
- Manual user positions win over automatic layout unless the user invokes auto-layout or changes the canvas policy.
- Group collapse is canvas facade state, not structural deletion or engine grouping.
- Promote-to-template crosses from canvas facade state into template service only at the explicit action boundary.
- Resize adjusts viewport, minimap, and visible details without touching calculation state.

## 7. Implementation Component Map

| Module/service | Needed by prototypes | Responsibility |
|---|---|---|
| `ui/components/workspace_shell.rs` | all | mount slots, context strip, status foot, resize observation, skin switcher |
| `state/skin_registry.rs` | all | registered skins, mounted slots, state hydration, lifecycle |
| `app/intents.rs` | all editing skins | closed `WorkspaceIntent` enum and receipts |
| `app/reducer.rs` | all | route intents to host state, meta writes, services, or direct OxCalc context calls |
| `services/live_edit.rs` | 01, 04, 06-08 | editor draft policy, OxFml diagnostics, debounced recalc commits |
| `adapters/oxfml/` | 01, 03-08 | formula parse/bind/completion/signature/hover support |
| `adapters/oxcalc/` | calc-affecting features | tree recalc, transactions, invalidation subscription |
| `services/tree_view_model.rs` | 01, 03, 04, 06 | visible tree rows, meta filtering, collapse projection |
| `services/selection_service.rs` | all | shared selection semantics, range selection, recent history |
| `services/structural_edit.rs` | 01, 03, 04, 06 | insert/delete/rename/move, propagation prompts |
| `services/format_inheritance.rs` | 01, 02, 04, 05-08 | effective format lookup, data-region format projection |
| `services/template_sync.rs` | 03, 04, 08 | template index, instantiate, validate, sync, promote |
| `ui/components/array_grid.rs` | 02, 06, 07 | virtualized arrays, shape/diff presentation, cell format rendering |
| `ui/components/format_editor.rs` | 05 | format property UI, CF rule list, live preview |
| `ui/components/template_editor.rs` | 03 | template tree and instance panel |
| `ui/components/canvas.rs` | 08 | cards, wires, lasso, groups, minimap, pan/zoom, layout |
| `ui/components/outline_table.rs` | 04 | tree-table projection, editable cells, sort/filter |
| `ui/components/node_columns.rs` | 07 | nodes-across columns and flow arrows |
| `services/export_excel.rs` | 02, workspace actions | value/export command path |
| `services/import_excel.rs` | workspace actions | import preview and structural import |
| `services/command_registry.rs` | all | keyboard/menu/command palette action dispatch |

## 8. Review Gates For This UX Area

Before W003/W005/W006 implementation, a review pass should be able to answer yes to each item:

| Question | Evidence location |
|---|---|
| Does each visible prototype affordance have a skin, primitive, state owner, and intent/API path? | this document sections 2-7 |
| Can formula text be traced from textbox to OxFml diagnostics and OxCalc publication? | F1, `TECHNICAL.md` section 4 |
| Can an OxCalc invalidation be traced back to visible status/value/wire updates? | F9 |
| Can resize and adaptive display changes be explained without recalc? | F10 |
| Are template definitions clearly calc-ignored until sync/instantiate emits regular edits? | F6, `META_NODES.md` |
| Are format edits meta/facade changes rather than engine state? | F5, `SKINS.md` section 9 |
| Does every skin persist only its own view state under `skins.<skin_id>`? | section 5, `SKINS.md` section 4 |
| Are OneCalc visual cues optional rather than normative? | `prototypes/index.html`, design-reference notes |

The prototype HTML remains static and illustrative. This traceability document connects those pictures to implementable UX contracts.

For implementation planning, use [`IMPLEMENTATION_MATRIX.md`](IMPLEMENTATION_MATRIX.md). It assigns stable trace IDs to this document's flows and turns them into workset-oriented slices, scenario checks, contract cards, and harness expectations.

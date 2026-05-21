# DNA TreeCalc - UX Implementation Matrix

Status: UX-side implementation driver, derived from [`TRACEABILITY.md`](TRACEABILITY.md).

This document turns the traceability review into a work driver. The test corpus anchors the model/language side with concrete cases. This matrix is the parallel UX-side anchor: stable trace IDs, implementation slices, component contracts, scenario cards, and verification hooks that let W003/W005/W006 build from visible behavior down into DnaTreeCalc internals and OxFml/OxCalc boundaries.

It is not a second requirements document. It answers: "If a prototype shows this behavior, what implementation slice proves it, what state does it touch, what must not call OxCalc, and what scenario should keep proving it?"

## 1. How To Use This Matrix

For each UX-facing bead:

1. Pick the smallest trace slice IDs that describe the outcome.
2. Implement the listed component/service contracts, not an ad hoc path.
3. Add or extend the listed check shape: reducer test, state round-trip, render projection check, or browser click-through.
4. Verify the "must not" boundary for facade-only flows.
5. Update this matrix when implementation reveals a missing slice or wrong boundary.

The expected loop is:

```
prototype affordance -> UX trace ID -> component/service contract -> scenario check -> bead closure
```

## 2. Trace ID Scheme

| Prefix | Area | Primary worksets |
|---|---|---|
| `UX-SK` | Skin registry, mount slots, lifecycle, state hydration | W005, W003 |
| `UX-SH` | Shell chrome, context strip, status foot, resize/pane host | W005, W003 |
| `UX-TR` | Tree projection, selection, meta visibility, navigation | W003, W005 |
| `UX-FE` | Formula editor/bar, live diagnostics, commit policy | W003, W002 |
| `UX-VA` | Value display, arrays, shape/diff, value pinning | W003, W006 |
| `UX-ST` | Structural edits: insert, rename, move, delete, undo grouping | W003, W004 |
| `UX-FM` | Format meta, format inheritance, live preview, CF UI | W007, W005 |
| `UX-TP` | Templates, template index, instance list, sync | W007, W006 |
| `UX-GR` | Dependency graph projections, wires, counts, drill | W002, W003, W006 |
| `UX-CV` | Canvas, nodes-across, layout/routing/viewport facade state | W006, W005 |
| `UX-IO` | Save/load/import/export UX surfaces and manifests | W003, W008, W009 |
| `UX-RX` | Reactivity, invalidation, external updates, async progress | W002, W003, W006 |

## 3. Implementation Slice Matrix

| ID | Slice | Prototypes | Workset | Build surface | Required checks |
|---|---|---|---|---|---|
| `UX-SK-001` | Register built-in skins and mount one main-slot skin | 01, index | W005 | `RegisteredSkin`, `SkinRegistryState`, `WorkspaceShell` | Unit: registry lists built-ins; render smoke mounts TripleEditor |
| `UX-SK-002` | Hydrate typed skin state from meta-nodes | 01, 08 | W005 | `SkinStateHandle<S>`, meta loader | Round-trip: `skins.<skin_id>` state survives save/load |
| `UX-SK-003` | Switch focused mount slot without recalc | 04, 06-08 | W005 | skin switcher, mount lifecycle | No-engine-call assertion; selection survives switch |
| `UX-SK-004` | Mount inspector/specialty skin beside editor | 03, 05 | W005/W006 | `SkinMountSlot`, pane host | Browser: editor plus inspector both observe same selection |
| `UX-SH-001` | Context strip shows filename/profile/recalc state | 01-08 | W003 | `ContextStrip`, workspace/session state | Projection check from host state to strip model |
| `UX-SH-002` | Status foot merges universal status and skin hints | 01-08 | W003/W005 | `StatusFoot`, command registry | Render projection check for clean/error/evaluating states |
| `UX-SH-003` | Resize dispatches container metrics to mounted skins | 06-08 | W005/W006 | `WorkspaceShell`, `ResizeObserver`, pane host | Browser resize: no recalc; viewport/columns adapt |
| `UX-TR-001` | Tree rows project regular nodes with values/status | 01-06 | W003 | `tree_view_model`, `TreeRow` | Projection test from workspace fixture |
| `UX-TR-002` | Collapse state is shared skin state | 01, 04, 06 | W005/W003 | `SharedSkinStateHandle` | Round-trip and switch-skin continuity test |
| `UX-TR-003` | Meta visibility reveals meta rows with distinct styling | 03 | W003/W007 | tree projection, `show_meta_nodes` | Projection test: hidden by default, visible when toggled |
| `UX-TR-004` | Selection is shared across skins and panes | all | W005/W003 | selection service, mount contexts | Browser: select in one skin, observe in another |
| `UX-FE-001` | Formula editor binds to selected node content string | 01, 04 | W003/W002 | OneCalc editor primitive, live edit service | Unit: selected-node switch updates editor draft policy |
| `UX-FE-002` | Live diagnostics use OxFml without publishing workspace edits | 01, 06-08 | W003/W002 | OxFml adapter | No-workspace-mutation assertion during draft bind |
| `UX-FE-003` | Commit formula dispatches `EditFormula` and recalc request | 01, 06-08 | W003/W002 | dispatcher, live edit, OxCalc bridge | Trace: intent -> recalc request -> published value |
| `UX-FE-004` | Constant entry and empty string semantics are represented in UI | 04, 06 | W003/W002 | editor policy, value display | Unit: blank content text -> empty node; no-equals constant; equals formula |
| `UX-VA-001` | Scalar value display applies effective format | 01, 04-08 | W003/W007 | `ValueDisplay`, `FormatResolver` | Projection check for raw/effective display |
| `UX-VA-002` | Array grid virtualizes and preserves scroll on shape change | 02, 06, 07 | W003/W006 | `ArrayGrid`, value diff service | UI scenario: grow array, scroll stays stable |
| `UX-VA-003` | Array shape/diff badges update from published value changes | 02, 06 | W003/W006 | value projection/diff service | Unit: prior/current shape diff projection |
| `UX-VA-004` | Pin value writes shared facade state only | 02 | W005/W003 | shared state handle, value toolbar | No-engine-call; persistence of pin |
| `UX-ST-001` | Inline rename validates siblings and prompts propagation | 01, 04 | W003/W004 | structural edit service, modal prompt | Unit: collision rejection and affected refs preview |
| `UX-ST-002` | Move/reorder updates structure and preserves node-id skin state | 01, 04 | W003/W004 | structural edit, tree projection | Unit: move keeps canvas/outline state keyed by node id |
| `UX-ST-003` | Delete prunes regular subtree and skin meta references | 01, 08 | W003/W005 | structural edit, skin-state GC | Unit: deleted node id removed from skin state |
| `UX-FM-001` | Format editor reads/writes `Format` meta-child | 05 | W007 | format service, format editor | No-engine-call for literal format edit |
| `UX-FM-002` | Format inheritance resolves by host service | 01, 04, 05 | W007 | `FormatResolver` | Unit: ancestor/local override resolution |
| `UX-FM-003` | Live preview updates when value or format changes | 05 | W007/W003 | format preview projection | Projection test from value+format inputs |
| `UX-FM-004` | Computed format properties are explicit design extension | 05 | W007 | format service guard | Guard test: cannot silently join dependency graph |
| `UX-TP-001` | Template editor edits meta-subtree without binding | 03 | W007 | template editor, meta service | No-engine-call for template body edit |
| `UX-TP-002` | Template index derives from meta-subtrees and rollout tags | 03, 04 | W007 | template index service | Unit: index rebuild from workspace tree |
| `UX-TP-003` | Validate/sync computes diff on demand | 03 | W007 | template sync service | Unit: current template+instance -> diff summary |
| `UX-TP-004` | Sync accepted changes emit regular structural/content edits | 03 | W007/W004 | dispatcher batch, structural edit | Trace: sync -> intents -> recalc |
| `UX-GR-001` | Dependency counts project from OxCalc graph | 01, 04 | W002/W003 | graph projection service | Unit: graph fixture -> in/out counts |
| `UX-GR-002` | Wires project graph edges through measured geometry | 07, 08 | W006/W002 | wire renderer, geometry registry | Render projection check: edges resolve to endpoints |
| `UX-GR-003` | Drill trace is requested lazily | 01 | W003/W002 | drill/audit service | No eager trace on selection; trace on open/request |
| `UX-CV-001` | Canvas positions, groups, zoom, pan persist as skin state | 08 | W006/W005 | `CanvasFlowState` | Round-trip save/load |
| `UX-CV-002` | Canvas drag/group/pan/zoom never calls OxCalc | 08 | W006 | canvas interactions | No-engine-call interaction test |
| `UX-CV-003` | Canvas auto-layout reads graph and writes positions | 08 | W006/W002 | canvas layout service | Unit: graph+state -> new positions |
| `UX-CV-004` | NodesAcross orders columns from scope and graph | 07 | W006/W002 | node column projection | Projection test from graph fixture |
| `UX-IO-001` | Save/load rehydrates workspace plus skin namespaces | all | W003/W005 | persistence service | Round-trip: tree + `skins.*` + templates/format |
| `UX-IO-002` | Export/copy value uses published values, not skin display state | 02 | W009 | export/value command service | Unit: copy/export from value projection |
| `UX-RX-001` | OxCalc invalidation updates calc pips before final value | 01, 04, 08 | W002/W003 | bridge subscription, calc-state projection | Scenario: dirty -> evaluating -> clean/error |
| `UX-RX-002` | External streaming update uses same render path as recalc | 01, 08 | W002/W003 | external value adapter | Trace: external update -> invalidation -> render |
| `UX-RX-003` | Async operations show progress and remain cancellable | 03, import/export | W007/W008/W009 | progress model, command registry | UI scenario: long sync/import shows progress |

## 4. Contract Cards

### C1. `WorkspaceShell`

Inputs:
- `TreeCalcHostState`
- `SkinRegistryState`
- command registry
- app-level file/session state

Outputs:
- context strip model
- mounted slot host metrics
- status foot model
- skin mount/unmount actions

Invariants:
- resizing never calls OxCalc;
- switching skins never calls OxCalc;
- universal chrome cannot directly mutate calculation workspace state except through commands/intents;
- inspector slots and main slot share selection and workspace signals.

### C2. `SkinRegistryState`

Inputs:
- registered built-in skins;
- persisted default skin id;
- persisted per-skin meta namespaces;
- current mount descriptors.

Outputs:
- mounted `SkinHandle`s;
- typed `SkinContext<S>` per mounted instance;
- lifecycle events.

Invariants:
- concrete skin state remains typed;
- erased registration only crosses the registry boundary;
- unregistered skin meta data is preserved on load/save;
- mount/unmount preserves workspace, selection, calc state, and shared skin state.

### C3. `SkinStateHandle<S>` and `SharedSkinStateHandle`

Inputs:
- typed state value;
- meta-node serializer/deserializer;
- schema version/migration;
- live node id set for GC.

Outputs:
- reactive read/update API;
- persisted meta-node updates;
- optional GC result.

Invariants:
- state updates do not call OxCalc;
- state updates do not rebind formulas;
- state is keyed by stable ids where possible;
- deleted regular nodes are pruned from all skin state that references them.

### C4. `Dispatcher`

Inputs:
- `WorkspaceIntent`;
- current host state;
- bridge availability;
- service registry.

Outputs:
- `IntentReceipt`;
- host state update;
- optional OxFml/OxCalc request;
- undo grouping.

Invariants:
- skins do not bypass the dispatcher for calculation workspace changes;
- facade-only changes are not routed to OxCalc;
- format/template meta writes remain host-level unless they emit regular structural edits;
- accepted calc-affecting edits produce a publication or a rejected candidate with diagnostics.

### C5. `TreeViewModel`

Inputs:
- workspace tree;
- shared collapse state;
- meta visibility policy;
- selection;
- calc/value projections.

Outputs:
- virtualized `TreeRowView` stream;
- row status/value summaries;
- drop targets.

Invariants:
- meta nodes are hidden by default;
- positional display order follows stable child order unless a skin explicitly sorts a projection;
- selecting a row never edits workspace structure;
- collapse state is shared facade state.

### C6. `FormulaEditorHost`

Inputs:
- selected node id;
- selected node content string;
- capability profile;
- OxFml editor bridge;
- commit policy.

Outputs:
- local draft state;
- syntax/diagnostic/completion projection;
- `EditFormula` intent on accepted commit.

Invariants:
- live diagnostics do not publish workspace edits;
- blank node content text means the node has the empty value;
- no formula evaluation result can produce the empty value;
- no-equals text is a literal constant channel;
- equals text is formula channel;
- template editor may use parse/syntax services without live binding as a regular node.

### C7. `ValueRenderer` and `ArrayGrid`

Inputs:
- published `EvalValue`;
- effective format projection;
- prior value projection for diff;
- viewport/virtualization state.

Outputs:
- scalar/array/table/reference/lambda/error render model;
- array shape badges;
- visible cell requests;
- transient diff highlights.

Invariants:
- value shape comes from OxCalc publication;
- formatting comes from host resolver;
- array viewport state is skin state;
- scrolling/resizing/expanding arrays does not recalc.

### C8. `FormatResolver`

Inputs:
- workspace tree;
- effective meta status;
- ancestor chain;
- `Format` meta-children;
- selected node value for preview only.

Outputs:
- effective format property map;
- inheritance source metadata;
- render-ready value-format projection.

Invariants:
- skins do not implement inheritance walks;
- format properties affect value regions, not skin chrome;
- literal format edits do not call OxCalc;
- computed format remains a guarded design area and cannot silently become a dependency edge.

### C9. `TemplateIndex` and `TemplateSync`

Inputs:
- template meta-subtrees;
- hidden rollout tags;
- regular instance subtrees;
- accepted user sync decisions.

Outputs:
- template list;
- instance status;
- diff/validate report;
- generated regular structural/content intents.

Invariants:
- template definitions are not bound/evaluated;
- index is derived and disposable;
- instance edits are ordinary edits;
- sync is explicit and grouped for undo.

### C10. `GraphProjection`

Inputs:
- OxCalc dependency graph;
- visible scope/focus;
- geometry registry where needed;
- skin routing/layout state.

Outputs:
- dependency counts;
- graph edge render model;
- wire endpoints and labels;
- topological/default flow ordering.

Invariants:
- graph is engine result state;
- wires/counts are projections only;
- geometry/routing changes do not recalc;
- graph-driven auto-layout reads graph and writes only skin positions.

## 5. Scenario Cards

Scenarios reuse the corpus workspaces under [`../test-corpus/workspaces/`](../test-corpus/workspaces/): `accounts` is the canonical example tree (`Accounts.2005.…`); `arrays`, `templates`, `formatting`, `cycles`, and `dynamic` cover the specialized cases. Sharing examples with the model corpus avoids drift.

### S1. First shell mount

Trace IDs: `UX-SK-001`, `UX-SH-001`, `UX-TR-001`.

Setup:
- the `accounts` corpus workspace (`Accounts.2005.Q1.{Income, Margin, Net}`);
- `triple-editor` is default skin;
- no persisted skin state yet.

Actions:
- load workspace;
- host mounts main slot.

Expected:
- TripleEditor appears;
- context strip shows filename/profile;
- tree rows render regular nodes;
- selected node is visible;
- default `skins.triple-editor` state is seeded only when needed;
- no recalc happens unless loading policy requires an initial calculation.

Check shape:
- render smoke plus registry state assertion.

### S2. Formula edit to value update

Trace IDs: `UX-FE-001`, `UX-FE-002`, `UX-FE-003`, `UX-RX-001`.

Setup:
- selected regular node `Net`;
- dependencies `Income` and `Expenses` exist;
- prior value is clean.

Actions:
- type `=Income-Expenses`;
- observe live diagnostics;
- commit.

Expected:
- draft diagnostics use OxFml and do not mutate workspace;
- commit dispatches `EditFormula`;
- DnaTreeCalc builds a tree recalc request;
- OxCalc publishes value/calc state/graph;
- value panel, status foot, tree pips, dependency counts, and graph projections update.

Check shape:
- trace-event assertion: `OxFmlBindRequested` before `IntentDispatched`, `OxCalcRecalcRequested` only after commit.

### S3. Constant and empty content

Trace IDs: `UX-FE-004`, `UX-VA-001`.

Setup:
- selected regular node with existing formula.

Actions:
- replace content with empty text;
- replace content with `123.4`;
- replace content with `=123.4`.

Expected:
- empty text makes node empty;
- `123.4` is a literal constant content string;
- `=123.4` is formula content;
- the UI differentiates empty value from empty string result.

Check shape:
- reducer/unit tests plus value projection assertions.

### S4. Shared selection across skins

Trace IDs: `UX-SK-003`, `UX-TR-004`, `UX-CV-004`.

Setup:
- TripleEditor and CellView are registered;
- selected node is `MonthlyForecast`.

Actions:
- switch from TripleEditor to CellView;
- select `TaxRate`;
- switch to NodesAcross.

Expected:
- no recalc on either switch;
- selected node is preserved and visible/focused where the target skin supports it;
- skin-specific scroll/expanded-array state stays with each skin.

Check shape:
- browser click-through with no-engine-call assertion.

### S5. Collapse/meta visibility

Trace IDs: `UX-TR-002`, `UX-TR-003`.

Setup:
- workspace has regular tree plus `Templates`, `Format`, and `skins` meta-subtrees.

Actions:
- collapse `Accounts`;
- reveal meta nodes;
- switch to OutlineTable and back.

Expected:
- collapse state is shared where the target skin honors tree collapse;
- meta nodes appear only when visibility is enabled and with distinct styling;
- formulas/completion still cannot see meta nodes.

Check shape:
- tree projection test plus editor completion exclusion test.

### S6. Array growth

Trace IDs: `UX-VA-002`, `UX-VA-003`, `UX-RX-001`.

Setup:
- selected node `MonthlyForecast` has an array value;
- grid scrolled into the middle.

Actions:
- upstream dependency changes so array grows by two rows.

Expected:
- shape badge updates;
- changed cells/rows highlight;
- scroll position is preserved;
- status summaries update;
- no grid-local state becomes calculation state.

Check shape:
- array projection test plus browser scroll preservation scenario.

### S7. Format edit propagates visually, not computationally

Trace IDs: `UX-FM-001`, `UX-FM-002`, `UX-FM-003`.

Setup:
- selected node inherits number format from `.Accounts.Format`;
- TripleEditor and OutlineTable are mounted or switchable.

Actions:
- open FormatEditor;
- override number format locally;
- switch to OutlineTable.

Expected:
- `Format` meta-child is created/updated;
- no OxCalc recalc is requested for literal format;
- effective format source changes from inherited to local;
- value regions in both skins render with the override.

Check shape:
- no-engine-call assertion plus format resolver unit test.

### S8. Template edit and sync

Trace IDs: `UX-TP-001`, `UX-TP-002`, `UX-TP-003`, `UX-TP-004`.

Setup:
- `QuarterShape` template meta-subtree exists;
- Q1-Q4 instances carry rollout tags;
- Q4 is behind version.

Actions:
- edit template structure;
- validate;
- accept sync for Q4.

Expected:
- template edit itself is meta-only and not bound;
- template index updates;
- validate computes current diff;
- accepted sync emits regular structural/content edits to Q4;
- sync edits recalc as ordinary workspace edits.

Check shape:
- template index rebuild unit test and sync trace test.

### S9. Canvas facade state

Trace IDs: `UX-CV-001`, `UX-CV-002`, `UX-GR-002`.

Setup:
- canvas shows dependency cards and wires;
- positions already persisted.

Actions:
- drag a card;
- pan/zoom;
- change wire routing;
- save/reopen.

Expected:
- positions/pan/zoom/routing persist under `skins.canvas-flow`;
- no OxCalc call happens for those interactions;
- wires redraw from existing graph and new geometry;
- reopened workspace restores view state.

Check shape:
- browser interaction plus save/load round-trip.

### S10. Canvas auto-layout

Trace IDs: `UX-CV-003`, `UX-GR-002`.

Setup:
- dependency graph available;
- canvas positions in free mode.

Actions:
- invoke auto-layout or hierarchy layout.

Expected:
- layout service reads graph and current positions;
- writes new canvas positions;
- does not edit tree structure or formula text;
- does not recalc.

Check shape:
- pure layout unit test and no-engine-call assertion.

### S11. Dependency wires after formula edit

Trace IDs: `UX-FE-003`, `UX-GR-002`, `UX-RX-001`.

Setup:
- CanvasFlow or NodesAcross visible;
- `AfterTax` depends on `Margin` and `TaxRate`.

Actions:
- edit formula to add `Adjustment`.

Expected:
- formula commit recalcs/rebinds;
- dependency graph includes new edge;
- wire renderer adds visible edge after publication;
- user-authored canvas positions are preserved.

Check shape:
- graph projection test plus browser edge count assertion.

### S12. Resize/adaptive display

Trace IDs: `UX-SH-003`, `UX-VA-002`, `UX-CV-001`.

Setup:
- CellView has expanded array;
- CanvasFlow has zoom/pan;
- shell is resizable.

Actions:
- narrow and widen viewport.

Expected:
- shell sends metrics to mounted skins;
- skins adapt columns/panes/viewport;
- persistent user sizes update only when the user resizes a pane, not for every measured layout pass;
- no recalc occurs.

Check shape:
- browser resize check with no-engine-call assertion.

### S13. External streaming update

Trace IDs: `UX-RX-002`, `UX-VA-001`, `UX-GR-001`.

Setup:
- node depends on external source;
- TripleEditor or CanvasFlow visible.

Actions:
- adapter receives new external value.

Expected:
- host dispatches external update/invalidation;
- OxCalc computes affected closure;
- value displays, pips, and dependency summaries update through normal signals;
- skins do not enter a special streaming mode.

Check shape:
- bridge fake emits external update; UI projection changes.

### S14. Save/load skin state and host meta

Trace IDs: `UX-SK-002`, `UX-IO-001`, `UX-FM-001`, `UX-TP-002`.

Setup:
- canvas positions, cell expanded rows, format meta, and template meta exist.

Actions:
- save workspace;
- reload workspace.

Expected:
- regular tree is intact;
- `skins.*` state rehydrates by skin id;
- format resolver sees format meta;
- template index rebuilds from meta-subtrees and rollout tags;
- unused skin namespaces are preserved.

Check shape:
- persistence round-trip fixture.

### S15. Import/export command surfaces

Trace IDs: `UX-IO-002`, `UX-SH-002`.

Setup:
- selected array/scalar value exists;
- export command available from value toolbar or command palette.

Actions:
- copy/export value;
- later W008/W009 import/export workspace.

Expected:
- value export reads published values and format projection, not canvas/table display state;
- long-running import/export emits progress to shell/status surfaces;
- canonical Excel comparison remains OxXlPlay/OxReplay owned.

Check shape:
- value projection unit test; later Excel-anchored workset checks.

## 6. Trace Events For Harnesses

The implementation should expose a test-only trace sink in host builds, not a user feature. It lets tests assert that UX flows cross the right boundaries.

| Event | Emitted by | Use |
|---|---|---|
| `SkinRegistered { skin_id }` | skin registry | Built-in registration checks |
| `SkinMounted { skin_id, slot }` | skin registry | Mount/switch scenarios |
| `SkinUnmounted { skin_id, slot }` | skin registry | Lifecycle checks |
| `SkinStateWritten { skin_id, path }` | skin state handle | Facade persistence checks |
| `SharedStateWritten { path }` | shared state handle | Selection/collapse/pin checks |
| `IntentDispatched { kind }` | dispatcher | Gesture-to-intent checks |
| `OxFmlBindRequested { node }` | live edit service | Draft diagnostics checks |
| `OxCalcRecalcRequested { reason }` | OxCalc bridge adapter | Calc boundary checks |
| `WorkspacePublished { publication_id }` | bridge/reducer | Render update checks |
| `RenderProjectionUpdated { projection }` | projection services | Value/tree/graph checks |
| `ResizeObserved { slot, size }` | workspace shell | Adaptive/resize checks |
| `NoEngineCallWindow { label }` | test harness | Assert facade-only interactions stay facade-only |

Trace events must be deterministic enough for tests and cheap enough to leave compiled behind a feature flag. They are not telemetry doctrine.

## 7. Minimum Harness Set

| Harness | First useful workset | Proves |
|---|---|---|
| Skin registry unit harness | W005 | registration, object-safe mount, lifecycle |
| Skin-state round-trip fixture | W005 | typed state persists through `skins.*` meta nodes |
| Tree projection fixture | W003 | visible rows, collapse, meta visibility, selection |
| Intent/dispatcher trace fixture | W003/W005 | gestures route through the right boundary |
| Fake OxFml/OxCalc bridge harness | W002/W003 | editor-to-publication and invalidation-to-display flows |
| Value projection fixture | W003 | scalar/array/error/reference render models |
| Browser click-through harness | W003/W006 | shell, switching, resize, key gestures, visible affordances |
| Canvas geometry harness | W006 | card positions, wire endpoints, minimap, viewport |
| Format resolver fixture | W007 | inheritance and literal format edit boundaries |
| Template index/sync fixture | W007 | meta index rebuild and explicit sync edits |

## 8. Workset Entry Criteria

### W005 skin scaffold can start when:

- `UX-SK-001` through `UX-SK-003` are understood;
- `C1`, `C2`, `C3`, and `C4` are accepted as contracts;
- the skin-state round-trip harness exists or is part of the first W005 bead.

### W003 tree shell can start when:

- W005 provides a minimal main-slot mount path;
- `UX-TR-001`, `UX-TR-002`, `UX-FE-001`, and `UX-VA-001` have fixtures or planned first-bead checks;
- the fake bridge can publish a small value/result projection.

### W006 additional skins can start when:

- skin switching is proven by `UX-SK-003`;
- value renderer and selection are shared;
- graph projection has at least a fake graph fixture;
- each new skin chooses the scenario cards it must pass before "done".

### W007 format/template UX can start when:

- meta-node persistence works;
- format/template services can write and read meta-subtrees;
- no-engine-call checks exist for literal format and template definition edits.

## 9. Completeness Checklist

Before implementing a visible prototype feature, answer:

| Question | Why it matters |
|---|---|
| Which trace ID owns this feature? | Prevents one-off paths |
| Which state lane owns it? | Prevents facade state leaking into engine state |
| What is the user-visible scenario card? | Keeps implementation grounded in UX behavior |
| Does the flow call OxFml, OxCalc, both, or neither? | Protects repository boundaries |
| What projection updates after the operation? | Makes reactivity explicit |
| What persists after save/load? | Avoids accidental session-only behavior |
| What must not change? | Catches recalc/layout/selection regressions |
| Which harness proves it now? | Turns the matrix into executable pressure |

If no row fits, add one here before implementing. That is the UX-side equivalent of adding a test-corpus case for a new semantic surface.

## 10. Status

**Machine-checkable index.** The trace slices, scenarios, and harnesses here are mirrored in [`ux-trace-manifest.json`](ux-trace-manifest.json), each tagged with a `workset` + `status` (`pending`/`active`) and validated by [`tools/validate-ux-matrix.ps1`](tools/validate-ux-matrix.ps1). That manifest is the **UX analog of the model test corpus** (`docs/test-corpus/`): it prints a by-workset/by-status coverage matrix so "tests ↔ work areas, progressively activated" is uniform across model and UX. Today everything is `pending`; the **W005 walking skeleton** flips its first thin slice to `active` (the corpus runner + click-through harness come online there), and each remaining slice flips as its workset delivers it and a harness covers it.

This matrix is intentionally more implementation-shaped than the requirements and traceability documents. It should evolve as soon as code exists: rows that prove useful can become test fixture names, scenario IDs, or bead acceptance checks. Rows that prove wrong should be corrected here rather than worked around in code.

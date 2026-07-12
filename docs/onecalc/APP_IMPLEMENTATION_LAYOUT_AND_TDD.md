# DNA OneCalc Implementation Layout And TDD

Status: `draft_implementation_authority`
Date: 2026-04-05
Scope: active code layout, UI toolkit policy, and red/green TDD structure for the greenfield `DnaOneCalc` rebuild

Authority chain:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) owns product scope, host boundary, runtime shape, artifact obligations, and upstream dependency constitution.
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md) owns intended application UX.
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md) owns shell, mode, surface, and ownership structure.
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md) owns current UX scope boundaries.
5. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md), [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md), [APP_UX_USE_CASES.md](APP_UX_USE_CASES.md), [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md), and the mode screen specs own mode-specific UX behavior and traceability.
6. [APP_UX_FORMULA_EDITOR_SPEC.md](APP_UX_FORMULA_EDITOR_SPEC.md) owns the specialized custom formula-editor scope, compatibility floor, overlay design, and editor TDD obligations.
7. [APP_UX_HOST_STATE_SLICING.md](APP_UX_HOST_STATE_SLICING.md) owns the implementation-facing host state slices that this layout must respect.
8. [APP_PROGRAMMATIC_FORMULA_TESTING.md](APP_PROGRAMMATIC_FORMULA_TESTING.md) records the intended headless and scripted formula-corpus execution path that should feed the same retained artifact model as the interactive app.
9. This note owns implementation layout, production UI toolkit policy, and TDD structure only.

## 1. Purpose
This note defines how the greenfield `DnaOneCalc` implementation should be organized so that:
1. the active production host is rebuilt directly against the current scope and UX doc set,
2. desktop and browser hosts share one application and UI core,
3. the codebase can grow through red/green TDD without collapsing into an undifferentiated shell,
4. and the archived implementation remains a reference source instead of a migration target.

It does not:
1. redefine product behavior,
2. restate screen logic already captured in the UX docs,
3. freeze upstream seam details that belong in the `Ox*` repos,
4. or authorize gradual migration from the archived implementation.

## 2. Runtime And Host Topology
The active implementation shape is:
1. shared application core in Rust,
2. shared UI core in Rust with `Leptos`,
3. desktop host as a thin `Tauri` shell,
4. browser host as a thin WASM/web shell over the same shared core,
5. no separate legacy or parallel production UI stack.

Interpretation rules:
1. desktop and browser are separate hosts over one shared application and UI core,
2. `Tauri` is a desktop shell, not the web host,
3. host wrappers own startup, packaging, and host-specific wiring only,
4. the shared app crate owns product behavior.

Implementation posture:
1. use one primary active app crate first,
2. keep strong internal module boundaries inside that crate,
3. do not split into many small crates yet,
4. and do not preserve the archived file structure as the new internal shape.

Decision rule:
1. if a concern belongs to both desktop and browser, it belongs in the shared app crate,
2. if a concern belongs only to shell wiring, packaging, or platform boot, it belongs in a host wrapper,
3. if a concern is product behavior, it must not be implemented first in a host wrapper.

## 3. Active Implementation Root
The active implementation root is:
1. `src/dnaonecalc-host`

The archive-reference root is:
1. `src_archive_ref/dnaonecalc-host`

Archive rule:
1. `src_archive_ref` is read-only reference material,
2. archived code may be inspected for prior Ox* integrations, retained behavior, and seam assumptions,
3. archived code must not be copied structurally into the active implementation without re-justification against the current scope and UX docs,
4. any archive-derived behavior must be re-expressed through the new module boundaries and tests.

## 4. Production UI Toolkit Policy
The production UI stack is:
1. shared UI in `Leptos`,
2. custom OneCalc design-system primitives and application widgets,
3. CSS custom properties for theme, spacing, state, and semantic color tokens,
4. OneCalc-owned widget vocabulary for the visible product shell.

Platform shortcut rule:
1. Windows desktop is the strict-first target for Excel-style editing shortcuts and formula-entry interaction,
2. browser/WASM is best-effort for shortcut parity and should prefer equivalent outcomes over exact keybinding duplication where the platform prevents it.

The production toolkit is not:
1. a library-first widget stack,
2. a direct reuse of Figma-generated React scaffolding,
3. or a generic admin dashboard component kit.

### 4.1 Widget Layers
The internal widget layers should be:
1. `design_tokens`
2. `primitives`
3. `composites`
4. `mode_surfaces`

Interpretation:
1. `design_tokens` owns colors, spacing, type scale, radii, borders, elevations, motion timings, and semantic state tokens,
2. `primitives` owns reusable base controls and layout atoms,
3. `composites` owns app-shaped assembled controls such as panel frames, capability badges, completion lists, help cards, and evidence rows,
4. `mode_surfaces` owns the actual `Explore`, `Inspect`, and `Workbench` visible compositions.

### 4.2 Initial Widget Vocabulary
The initial OneCalc-owned widget set should include:
1. button
2. icon button
3. text field
4. select
5. switch
6. tabs
7. drawer
8. dialog
9. menu
10. tooltip
11. scroll area
12. splitter
13. panel frame
14. status chip
15. tree node
16. table-like evidence list
17. inline diagnostic marker
18. completion list
19. help card
20. capability badge
21. structured text editor surface

Rule:
1. accessibility and browser primitives may be wrapped where needed,
2. but the public widget vocabulary should remain OneCalc-owned and product-specific.

## 5. Formula Editor Substrate
The formula editor is a custom OneCalc editor surface.

It is explicitly not:
1. `CodeMirror`,
2. `Monaco`,
3. or a plain textarea with syntax highlighting layered on later.

Implementation rules:
1. the editor lives inside the shared `Leptos` UI core,
2. editor logic must be isolated behind its own module boundary,
3. editor behavior must integrate with OxFml edit packets rather than inventing a local second language-service truth,
4. the custom editor path is allowed only with a heavier test burden than the rest of the UI,
5. the concrete editor architecture should follow [APP_UX_FORMULA_EDITOR_SPEC.md](APP_UX_FORMULA_EDITOR_SPEC.md).

Coupling rule:
1. the production editor should be intentionally coupled to OxFml's immutable syntax and language-service packet model,
2. including `FormulaEditRequest`, `FormulaEditResult`, `EditorSyntaxSnapshot`, live diagnostics, completion packets, signature-help context, green-tree keys, and reuse summaries,
3. because OneCalc is the UX complement to OxFml rather than a generic editor platform.

Input interpretation rule:
1. the editor accepts any Excel cell-entry text, not only leading-`=` formulas,
2. this includes direct value entry and apostrophe-forced string entry,
3. interpretation of that entry text and its effective-display behavior remains upstream semantic responsibility rather than host-local logic.

The editor substructure should be explicit:
1. text buffer model
2. selection and caret model
3. diagnostics overlay model
4. completion model
5. signature-help model
6. keyboard command map
7. long-formula navigation and resize behavior
8. OxFml edit-packet integration seam
9. native-input-backed fallback path
10. overlay and decoration planes

Simplification rule:
1. do not insert a broad generic parser or language-service abstraction between the active editor and OxFml,
2. keep abstraction only where needed for test doubles, host isolation, and seam evolution,
3. and let the production implementation name OxFml packet concepts directly where that removes duplication.

## 6. Target Code Layout
The target structure under `src/dnaonecalc-host/src` should be:
1. `app/`
2. `state/`
3. `domain/`
4. `services/`
5. `adapters/`
6. `ui/`
7. `platform/`
8. `persistence/`
9. `extensions/`
10. `test_support/`

### 6.1 app
Responsibilities:
1. startup
2. host boot
3. high-level command routing
4. mode routing between `Explore`, `Inspect`, and `Workbench`
5. top-level composition of the shared app core

### 6.2 state
Responsibilities:
1. host-state slices from [APP_UX_HOST_STATE_SLICING.md](APP_UX_HOST_STATE_SLICING.md)
2. workspace shell state
3. formula-space collection state
4. active formula-space view state
5. retained artifact open state
6. capability and environment state
7. extension surface state
8. global UI chrome state

Rule:
1. state layout should follow the sliced ownership model from the UX docs,
2. not the archived shell's earlier single-space booleans and ad hoc summaries.

### 6.3 domain
Responsibilities:
1. artifact-spine types
2. pure business rules
3. stable domain invariants
4. IDs, refs, and provenance vocabulary

Expected domain families:
1. `Scenario`
2. `ScenarioRun`
3. `Observation`
4. `Comparison`
5. `Witness`
6. `HandoffPacket`
7. `CapabilityLedgerSnapshot`
8. `ScenarioCapsule`

Rule:
1. `domain` must stay pure and deterministic.

### 6.4 services
Responsibilities:
1. use-case orchestration for `Explore`
2. use-case orchestration for `Inspect`
3. use-case orchestration for `Workbench`
4. persistence flows
5. capability snapshot emission and lookup
6. extension and host-policy coordination

Service families should include:
1. explorer service
2. inspect service
3. workbench service
4. persistence service
5. capability service
6. extension service
7. scenario lifecycle service
8. editor session service

Rule:
1. services may depend on `domain`, `state`, and adapter traits,
2. but not directly on concrete UI widgets.

### 6.5 adapters
Responsibilities:
1. Ox* integration
2. translation between app-facing models and upstream seam models
3. fakeable adapter traits for testing

Adapter families should include:
1. `oxfml`
2. `oxfunc`
3. `oxreplay`
4. `oxxlplay`

Rules:
1. adapters are the only layer that talks to Ox* crates directly,
2. adapters should expose app-facing traits and result shapes,
3. adapter seams must stay narrow and testable.

For the formula editor specifically:
1. the adapter seam should preserve OxFml packet structure closely enough that green-tree keys, syntax snapshots, text-change ranges, and reuse summaries are not lost,
2. the adapter must not downgrade OxFml editor packets into free-text summaries too early,
3. and test doubles should mimic OxFml packet behavior rather than a generic parse service.

### 6.6 ui
The shared `Leptos` UI core should contain:
1. `design_tokens/`
2. `primitives/`
3. `editor/`
4. `panels/`
5. `modes/`
6. `layout/`

Interpretation:
1. `editor/` owns the custom formula editor subsystem,
2. `panels/` owns the panel-level view projections aligned to [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md),
3. `modes/` owns `Explore`, `Inspect`, and `Workbench` visible compositions,
4. `layout/` owns shell composition such as left rail, context bar, drawer structure, and footer.

The initial `ui/editor/` layout should be prepared for:
1. `buffer/`
2. `commands/`
3. `selection/`
4. `history/`
5. `render_projection/`
6. `overlays/`
7. `oxfml_bridge/`
8. `fallback/`

Rules:
1. `ui/` depends on `services`, `state`, and view models,
2. `ui/` must not depend directly on Ox* crates,
3. panel and mode surfaces should align to the UX panel and screen-spec docs rather than inventing a second surface vocabulary.

### 6.7 platform
Responsibilities:
1. host discrimination between desktop and browser
2. platform gates
3. Windows-only compare admission
4. runtime capability interpretation
5. host shell interfaces needed by Tauri or browser wiring

Rule:
1. `platform/` is the only layer that decides admitted, blocked, lossy, or unavailable host capabilities.

### 6.8 persistence
Responsibilities:
1. document storage
2. scenario capsule import and export
3. retained artifact storage
4. capability snapshot persistence
5. artifact-ref resolution

Rule:
1. `persistence/` owns storage and transport formats,
2. not panel state or mode composition.

### 6.9 extensions
Responsibilities:
1. extension state
2. extension loading and failure status
3. extension gating and visible truth
4. extension-facing app contracts where admitted

Rule:
1. `extensions/` owns extension lifecycle truth,
2. not generic workspace or panel state.

### 6.10 test_support
Responsibilities:
1. builders
2. fake adapters
3. fixture loading
4. scenario corpus helpers
5. UI event helpers
6. assertion helpers for recurring app patterns

## 7. Shared Interaction And Async Model
The shared application core should use a typed command-and-query interaction model.

The intended shape is:
1. UI surfaces emit typed intents and commands,
2. services interpret those commands,
3. adapters execute upstream or platform work where needed,
4. state slices are updated from structured results rather than panel-local ad hoc mutation,
5. long-running operations are represented explicitly as operation state rather than hidden spinners.

The command model should cover, at minimum:
1. formula edit and re-evaluate
2. completion request and completion application
3. function-help request
4. mode switch
5. scenario-policy update
6. open or close right-drawer detail
7. retain run
8. open retained artifact
9. replay
10. compare
11. export capsule
12. generate handoff packet

Editor command rule:
1. formula-edit commands should carry the raw entered cell text plus the current editing context needed to drive OxFml incremental edit packets,
2. and edit results should return structured editor-document state rather than panel-specific fragments.

The query model should cover, at minimum:
1. active formula-space view state
2. panel view models
3. capability and gate summaries
4. retained artifact summaries
5. mode availability
6. operation status

Operation-state rules:
1. every long-running action should carry an operation identity and explicit status,
2. statuses should at least distinguish `ready`, `running`, `succeeded`, `failed`, `blocked`, and `lossy`,
3. operation state should live in structured app state, not only in ephemeral UI controls,
4. platform and capability blocking should resolve to typed blocked outcomes, not free-text errors.

State-of-the-art rule:
1. significant user operations should be modeled as replayable app intents over structured state,
2. even when the resulting UI trace is not yet exported as a formal replay artifact,
3. because this improves desktop/browser parity, TDD coverage, and future observability.

## 8. Thin Host Wrappers
The active implementation should reserve thin host-wrapper areas for:
1. `tauri_host/` or equivalent desktop shell wiring
2. `web_host/` or equivalent browser/WASM shell wiring

Rules:
1. host wrappers own startup, packaging, filesystem or platform entry points, and host glue only,
2. host wrappers must not become a second app core,
3. shared app and UI logic stays in the main active implementation root.

## 9. Layering Rules
The following dependency rules are frozen:
1. `domain/` is pure and deterministic.
2. `services/` depends on `domain/`, `state/`, and adapter traits.
3. `ui/` depends on `services/`, `state/`, and view models, not directly on Ox* crates.
4. `adapters/` are the only layer that talks to Ox* crates directly.
5. `platform/` is the only layer that decides admitted, blocked, lossy, or unavailable host capabilities.
6. `persistence/` owns storage and transport formats, not panels.
7. `extensions/` owns extension lifecycle truth, not generic UI state.
8. Windows-only Excel-observed compare must remain isolated behind `platform/` plus `adapters/oxxlplay`.
9. richer future function guidance must enter through `adapters/oxfunc` plus app services, not through local panel-only data models.

## 10. Test Layout
The test layers should be:
1. inline unit tests for pure `domain`, `state`, and low-level editor models
2. integration tests under the app crate for service and adapter behavior
3. browser/WASM UI tests for shared `Leptos` surfaces
4. end-to-end tests for the browser host
5. end-to-end smoke tests for the `Tauri` desktop host
6. contract tests for Ox* adapter seams using pinned fixtures and fakes

## 11. Test Directory Structure
The target test structure should define:
1. `tests/domain/`
2. `tests/explore/`
3. `tests/inspect/`
4. `tests/workbench/`
5. `tests/persistence/`
6. `tests/capability/`
7. `tests/extensions/`
8. `tests/platform/`
9. `tests/editor/`
10. `tests/upstream_contracts/`
11. `tests/e2e_web/`
12. `tests/e2e_tauri/`
13. `tests/fixtures/`

Interpretation:
1. mode-centered tests should follow the product order and mode names,
2. editor tests should be isolated because the editor is a custom subsystem,
3. end-to-end host tests should stay thin and focus on host wiring, gating, and smoke coverage rather than replacing service and integration tests.

## 12. TDD Workflow
The active development posture is red/green TDD.

Working rules:
1. start from scenario-level failing tests keyed to `EX-*`, `IN-*`, and `WB-*`,
2. implement the smallest green path in `domain`, `services`, `adapters`, or `ui`,
3. refactor only after the scenario test is green,
4. never start from panel polish or visual styling without a failing behavior test,
5. every capability-gated surface must have an explicit honesty test,
6. every mode must have acceptance tests for default composition, primary interaction path, blocked or gated state, and transition to another mode.

## 13. Editor-Specific TDD Burden
Because the formula editor is custom, it carries the highest TDD burden in the UI stack.

The editor must have dedicated red/green slices for:
1. multiline entry
2. `Tab` and `Shift+Tab` indent and outdent with spaces
3. long-formula scrolling and resize behavior
4. diagnostics span placement
5. completion invocation and acceptance
6. signature-help tracking
7. undo and redo
8. keyboard-first navigation
9. paste behavior
10. result visibility while editing
11. OxFml green-tree-key continuity and reuse-summary handling across ordinary edits
12. `EditorSyntaxSnapshot`-driven overlay projection

Rule:
1. editor behavior should be proven at buffer-model level, interaction-model level, and scenario-acceptance level,
2. not only through later end-to-end smoke coverage.

Integration-test rule:
1. the editor test tree should exercise real or pinned OxFml packet shapes wherever practical,
2. because the main implementation risk is the host/editor fit to OxFml's immutable syntax model rather than a generic UI-only concern.

## 14. Acceptance Traceability
Every implementation area should trace back to:
1. `EX-*`, `IN-*`, and `WB-*` use cases from [APP_UX_USE_CASES.md](APP_UX_USE_CASES.md),
2. panel ids from [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md),
3. screen anchors from [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md),
4. screen behavior from the mode screen-spec notes.

Interpretation:
1. use-case ids are the narrative acceptance anchors,
2. panel ids are the structural UI anchors,
3. screen anchors are the layout and visibility anchors,
4. the active test tree should be organized so these three references are easy to map.

## 15. Immediate Build Order
The first implementation slices should be laid down in this order:
1. domain artifact spine and host-state slices
2. adapter traits and fake adapters
3. scenario lifecycle and capability services
4. shell layout and mode routing
5. custom editor core
6. `Explore` mode
7. `Inspect` mode
8. `Workbench` mode
9. persistence and capsule flows
10. desktop and browser host wrappers

Reason:
1. this order supports early red/green progress without locking the UI around unfinished adapters or replay/compare lanes,
2. while still respecting the product order from the scope and UX docs.

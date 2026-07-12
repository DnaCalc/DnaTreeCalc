# DNA OneCalc UX Architecture

Status: `working_architecture_note`
Date: 2026-04-04
Scope: product UX architecture for the desktop-first and web-capable `DnaOneCalc` host

## 1. Purpose
This note turns the application UX brief into a more explicit product architecture for design and implementation work.

It exists to:
1. keep the OneCalc UX coherent across `Explore`, `Inspect`, and `Workbench`,
2. separate workspace truth from formula-space truth,
3. define where important surfaces belong before more visual refinement,
4. identify likely upstream seam pressure for richer semantic and help surfaces,
5. give Figma work and later host implementation one shared reference.

It is not:
1. a replacement for [APP_UX_BRIEF.md](APP_UX_BRIEF.md),
2. a replacement for [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md),
3. a visual-style guide,
4. or an execution-status note.

Companion synthesis note:
1. [APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md](APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md) records what the current Figma exploration contributes after scope filtering.
2. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md) records the named panel set that the current shell and mode model should use.
3. [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md) records the greenfield implementation layout, `Leptos`/`Tauri` host topology, custom widget-toolkit policy, and TDD structure derived from this architecture.
4. [APP_UX_FORMULA_EDITOR_SPEC.md](APP_UX_FORMULA_EDITOR_SPEC.md) records the dedicated custom-editor scope, Excel-compatibility floor, overlay architecture, and staged feature ladder used by `Explore` and later DNA Calc hosts.

## 2. Product Reading
`DnaOneCalc` is one product with three ordered task perspectives:
1. `Explore`
2. `Inspect`
3. `Workbench`

These are not separate apps.
They are coordinated perspectives over one active formula space.

Interpretation rule:
1. `Explore` is the first and default product face,
2. `Inspect` is the semantic and mechanism-reading perspective,
3. `Workbench` is the retained evidence, compare, replay, and handoff perspective,
4. later UX refinement must deepen this one shell rather than split the app into disconnected routes.

## 3. Core Product Objects
The UX should distinguish these object types explicitly:
1. `Workspace`
2. `FormulaSpace`
3. `ScenarioRun`
4. `Observation`
5. `Comparison`
6. `ReplayLineage`
7. `CapabilitySnapshot`
8. `ExtensionState`

Rule:
1. object types must not be blurred into one generic panel language,
2. the shell should make it clear whether the user is editing a scenario, inspecting a run, or reviewing retained evidence.

Entry-text rule:
1. a `FormulaSpace` is still the right product term,
2. but its primary authored text is Excel cell-entry text, not necessarily a leading-`=` formula string,
3. interpretation of that text remains upstream semantic truth rather than host-local parsing policy.

Syntax-substrate rule:
1. the formula-space editing surface is intentionally coupled to OxFml's immutable syntax and language-service substrate,
2. so OneCalc should project OxFml green-tree, red-projection, and editor-snapshot truth rather than owning a second syntax model.

## 4. Shell Model
The preferred shell model is:
1. left rail for workspace sections and formula spaces,
2. top context bar for active formula-space identity, current mode, and compact host truth,
3. main canvas for the primary content of the current mode,
4. right drawer for secondary detail rather than primary navigation,
5. footer for compact operational truth and version/runtime status.

Working rules:
1. the left rail answers which space the user is in,
2. the mode switch answers what kind of task the user is doing in that space,
3. drawers answer what supporting detail is open,
4. top-level and side-level navigation must not duplicate each other.

## 5. Mode Model
### 5.1 Explore
Primary purpose:
1. author formulas quickly,
2. discover functions,
3. read the result and effective display comfortably.

Primary surfaces:
1. formula editor,
2. result,
3. effective display,
4. diagnostics,
5. completions,
6. current help,
7. array preview when relevant.

Secondary surfaces:
1. scenario policy controls,
2. formatting entry points,
3. conditional-formatting entry points,
4. lightweight host and capability cues.

Reference surfaces:
1. deep formula walk,
2. replay lineage,
3. compare detail,
4. handoff detail.

### 5.2 Inspect
Primary purpose:
1. understand semantic structure,
2. inspect bindings and host-driving context,
3. inspect partial evaluations and blocked or opaque areas.

Primary surfaces:
1. formula walk,
2. parse summary,
3. bind summary,
4. eval summary,
5. provenance,
6. host context.

Secondary surfaces:
1. source formula,
2. current result,
3. scenario policy summary,
4. function-specific semantic guidance where admitted.

Reference surfaces:
1. full retained evidence,
2. compare workflow,
3. export and handoff operations.

### 5.3 Workbench
Primary purpose:
1. compare outputs,
2. inspect replay lineage,
3. retain evidence,
4. generate widening and handoff actions.

Primary surfaces:
1. comparison outcome,
2. reliability,
3. blocked dimensions,
4. replay lineage,
5. evidence bundle,
6. primary actions.

Secondary surfaces:
1. source formula summary,
2. source run summary,
3. observation envelope,
4. compact scenario policy summary.

Reference surfaces:
1. full editor,
2. full function help,
3. deeper semantic inspection.

## 6. Information Hierarchy
The UX should use five ranks:
1. `Primary`
2. `Secondary`
3. `Supporting`
4. `Reference`
5. `Administrative`

Interpretation:
1. `Primary` is the focal content of the current mode,
2. `Secondary` is needed nearby to interpret or act,
3. `Supporting` is important but should not dominate,
4. `Reference` is on demand,
5. `Administrative` describes workspace or environment truth rather than scenario-center work.

## 7. Ownership Model
### 7.1 Workspace-Level
Workspace-level UX owns:
1. workspace navigation,
2. open, recent, and pinned spaces,
3. extension management,
4. capability center,
5. host profile summary,
6. platform gate summary,
7. environment truth.

### 7.2 Formula-Space-Level
Formula-space-level UX owns:
1. formula text,
2. editor state,
3. current help and completion context,
4. diagnostics,
5. result and effective display,
6. scenario policy controls,
7. formatting and conditional formatting for that scenario,
8. inspect state,
9. retained runs and compare state for that scenario.

### 7.3 Run-Level
Run-level UX owns:
1. run metadata,
2. packet kind,
3. timing,
4. replay capture,
5. lineage entries.

### 7.4 Comparison-Level
Comparison-level UX owns:
1. comparison envelope,
2. reliability,
3. mismatches,
4. blocked dimensions,
5. widening and handoff readiness.

Rule:
1. if a control changes the meaning or reproducibility of one scenario, it should normally live on the formula space,
2. if a control describes the admitted environment shared by many spaces, it should normally live at workspace level.

## 8. Surface Inventory
The current target surface inventory is:
1. formula editor,
2. completion list,
3. current help,
4. diagnostics,
5. result card,
6. effective display,
7. array preview,
8. formatting editor,
9. conditional formatting editor,
10. scenario policy controls,
11. host state summary,
12. formula walk,
13. parse summary,
14. bind summary,
15. eval summary,
16. provenance,
17. capability summary,
18. compare result,
19. observation envelope,
20. replay lineage,
21. evidence bundle,
22. handoff panel,
23. extension state.

Interpretation rule:
1. the formula editor surface must also support direct value entry and apostrophe-forced string entry,
2. while continuing to project formula-specific help and diagnostics only when the current entry meaning admits them.

## 9. Scenario Policy Inventory
The scenario policy area should be explicitly modeled rather than treated as a miscellaneous menu.

Current expected scenario-level controls include:
1. deterministic versus real time policy,
2. deterministic versus real random policy,
3. scenario-affecting host support flags,
4. formatting controls,
5. conditional-formatting controls,
6. display-affecting options,
7. later additional admitted scenario policy controls as upstream seams widen.

Placement rule:
1. these controls should be accessible from `Explore`,
2. visible as summary truth in `Inspect`,
3. and preserved as evidence-bearing context in `Workbench`.

## 10. Function Interaction Model
Function interaction is not only a lookup sidebar.
It is a first-class companion to formula authoring and later semantic inspection.

The UX should be prepared for richer future OxFunc-backed function interaction, including:
1. richer help prose,
2. argument-level semantic guidance,
3. function caveats and constraints,
4. category and admission-status cues,
5. usage guidance relevant to the current scenario,
6. inspect-mode explanation of how the function participates in the current evaluation.

Rule:
1. future richer function guidance should deepen the main product surfaces,
2. it should not force OneCalc into a separate function-browser product.

Editor-language-service rule:
1. function interaction should be driven through the same OxFml editor packet flow used for completions and signature help,
2. not through a disconnected local sidebar data model.

## 11. Upstream Seam Pressure
The current UX direction suggests likely upstream library pressure.

### 11.1 OxFml
The inspect direction likely wants:
1. tree-addressable formula nodes,
2. stable semantic identities for nodes,
3. partial evaluation projections where admitted,
4. explicit blocked or opaque reasons,
5. richer bind and provenance packet detail suitable for durable UI correlation.

The explore and inspect directions also now assume:
1. immutable green-tree identity and green-tree-key continuity across edits,
2. contextual red projections suitable for cursor and selection correlation,
3. editor-grade syntax snapshots with owned trivia for rendering and span overlays,
4. incremental reuse evidence that can support both responsiveness and later operational truth surfaces.

### 11.2 OxFunc
The richer function interaction direction likely wants:
1. stronger help payloads,
2. argument semantics,
3. usage guidance and caveats,
4. surfaced admission and support status,
5. richer metadata suitable for both authoring and inspect-mode explanation.

### 11.3 OneCalc Host
The host likely needs:
1. a stable scenario policy model,
2. a panel/state model that can preserve mode and drawer state per formula space,
3. clearer retained-run and comparison summaries per active formula space,
4. explicit handling for Windows-only compare versus browser-capable replay and inspection.

## 12. Responsive Rule
Responsive behavior should preserve meaning before preserving every panel.

Preferred degradation order:
1. compress the left rail before reducing the editor and result,
2. collapse right-drawer detail before collapsing primary mode content,
3. preserve mode identity and active formula-space identity at all widths,
4. preserve honest host and platform gating in narrow layouts.

## 13. Design And Implementation Follow-Up
The next work should produce:
1. a mode-by-mode default visibility map,
2. a panel inventory for the chosen shell,
3. a scenario policy control register,
4. a function interaction proposal for future richer OxFunc guidance,
5. a seam-pressure note for likely `OxFml` and `OxFunc` asks,
6. and later an implementation-facing screen spec for `Explore`, `Inspect`, and `Workbench`.

Scope rule:
1. all later mockup synthesis and screen-spec work should remain constrained by [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md),
2. and no mockup should be treated as adding new product scope by itself.

# DNA OneCalc Application UX Brief

Status: `draft_authority_note`
Date: 2026-04-03
Scope: application UX direction for the desktop-first and web-capable `DnaOneCalc` product

Companion architecture note:
1. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md) carries the shell model, mode model, surface inventory, ownership split, and likely upstream seam pressure implied by this brief.
2. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md) carries the current UX scope breakdown, scope labels, and the rule that mockups do not expand product scope.
3. [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md) carries the active `Leptos`/`Tauri` implementation topology, custom widget-toolkit policy, and TDD structure that realize this brief.
4. [APP_UX_FORMULA_EDITOR_SPEC.md](APP_UX_FORMULA_EDITOR_SPEC.md) carries the dedicated scope, compatibility floor, staged feature set, implementation design, and TDD obligations for the custom formula editor.

## 1. Purpose
This note defines the intended user experience for `DnaOneCalc` as a serious modern application.

It is not a terminal or TUI brief.

It is a brief for:
1. a Rust-based product,
2. with a Windows desktop host expected to be first-class,
3. with a chosen `Tauri`-hosted desktop expression,
4. with a chosen browser and `wasm` expression over the same shared UI core,
5. and with the final execution location of some logic still open.

The brief should guide:
1. UX design,
2. information architecture,
3. interaction design,
4. visual design,
5. responsive behavior,
6. and later implementation work across desktop and browser hosts.

## 1A. User-Facing Terminology

The persistable work unit — the formula text plus its presentation
context (locale, format, host profile, scenario policy, attached
verification bundle, etc.) — is a single concept. To keep that
concept readable to the user, the user-visible vocabulary is:

1. `Formula` is the user-facing name for this work unit.
   Buttons, dropdown items, status-foot chips, save dialogs,
   tooltip prose, command-palette entries, and any other
   string the user reads should call it a `formula`.
2. `Scenario` remains the internal architectural name.
   Type names (`ScenarioBreadcrumbView`, …), state fields
   (`scenario_label`, `scenario_breadcrumb_open`, …), reducer
   functions (`toggle_scenario_breadcrumb`, …), CSS class
   stems (`__scenario-menu`, …), bead and workset prose,
   on-disk struct shapes, and this brief itself continue to
   say `scenario`.
3. The reason internal docs keep `scenario`:
   1. the term has a long history in the codebase, in
      `WorkspaceShellState`, in retained-run identities,
      in `.beads/` records, and in upstream contracts,
   2. renaming the internal term would touch hundreds of
      identifiers without changing any user-observable
      behaviour,
   3. and the existing word distinguishes `scenario`
      (the persistable work unit) from `formula` (the
      formula-text content of a scenario) cleanly inside
      the code.

Rule of thumb when editing:
1. Strings inside `view!` / JSX, `aria-label` / `title` /
   placeholder text, dropdown labels, file-dialog filter
   names, command-palette entries, and visible chips are
   user-facing — write `formula`.
2. Identifiers, comments, code docstrings, internal
   markdown, mockup CSS class names, and on-disk fields
   are internal — write `scenario`.

The file extension (today proposed `.dnascenario`) is
borderline because the user encounters it in save dialogs.
The current rule is to rename the file extension to
`.dnafml` only when the persistence slice lands; until
then the placeholder stays `.dnascenario` in plan docs and
no on-disk file is written.

## 2. Product Position
The ordered product expression remains:
1. `Formula / Function Explorer`,
2. `Live Formula Semantic X-Ray`,
3. `Twin Oracle Workbench`.

The product is:
1. a focused single-formula application,
2. a user-facing proving host,
3. a serious evidence and comparison tool,
4. and a downstream pressure surface for the `Ox*` libraries.

Input interpretation rule:
1. the editor accepts any text that could be entered into an Excel cell,
2. this includes leading-`=` formulas, direct value entry without `=`, and apostrophe-forced string entry such as `'123.4`,
3. OneCalc owns the editor surface and presentation,
4. OxFml owns interpretation of the entered text and the resulting typed or formatted outcome.

Editor-substrate rule:
1. the OneCalc formula editor is intentionally specialized for OxFml rather than treated as a generic text-editor platform,
2. so the UX should assume immutable edit packets, green-tree identity, contextual red projections, syntax snapshots, diagnostics, completion, and signature-help truth come from OxFml,
3. and OneCalc should add interaction, presentation, and workflow value on top of that substrate rather than rebuilding it locally.

The product is not:
1. a worksheet grid clone,
2. a terminal application,
3. a generic code editor with some calculation features,
4. or a visual spreadsheet workbook host.

## 3. Platform And Host Assumptions
UX should be designed for a family of hosts that share one product identity.

Initial host assumptions:
1. Windows desktop is first and must feel like a polished application.
2. `Tauri` is the chosen desktop shell for the first production host family.
3. A browser-hosted and `wasm`-capable version is part of the active implementation direction, not just a later design consideration.
4. The UX must not depend on desktop-only controls, native window chrome tricks, or terminal idioms.
5. The UX must remain credible if some compute moves between local process, `wasm`, helper service, or remote execution later.

Practical implication:
1. the visual system should be web-native and responsive,
2. the interaction model should be keyboard-capable and pointer-capable,
3. the state model should tolerate latency, loading, offline, and capability gates honestly,
4. the UI should be implementable in a shared `Leptos` core used by both `Tauri` and browser hosts,
5. and the product should not look or behave like a tool accidentally trapped inside a shell.

## 4. Core UX Principles
The application should follow these principles:

1. `Explorer first`.
   The primary task is authoring and understanding one formula quickly and comfortably.
2. `Support surfaces stay subordinate`.
   X-Ray, Capability, replay, and comparison surfaces must support the main task rather than obscure it.
3. `Evidence, not theater`.
   Reliability, lossiness, blocked dimensions, and capability limits must be visible and plain.
4. `One scenario, many perspectives`.
   Explorer, X-Ray, compare, witness, and handoff should feel like coordinated views over the same scenario, not unrelated screens.
5. `Modern and calm`.
   The UI should feel deliberate, contemporary, and aesthetically confident without turning into glossy noise.
6. `Desktop-class, web-ready`.
   Interactions should feel at home in a desktop app while remaining implementable and coherent in browser form.
7. `Keyboard-first, not keyboard-only`.
   The product should be efficient for heavy users while staying clear for pointer-first interaction.

## 5. Primary User Types
The brief should optimize for three main user modes:

1. `Explorer`
   Wants to type a formula, discover functions, understand arguments, and see a result quickly.
2. `Mechanism Investigator`
   Wants to inspect parse, bind, evaluation, capability, and provenance state for the current scenario.
3. `Compare / Replay Operator`
   Wants to retain runs, compare against observed Excel evidence, understand mismatch meaning, and prepare handoff or widening work.

These are modes of one product, not separate products.

## 6. Information Architecture
The application should be organized around one central work area with supporting views.

### 6.1 Primary Regions
The intended default composition is:
1. top navigation and workspace context,
2. primary formula editing region,
3. primary result region,
4. supporting inspector region,
5. optional secondary perspective panels such as X-Ray or compare details.

The primary formula and primary result must stay visible in ordinary use.

### 6.2 Perspective Hierarchy
The hierarchy should be:
1. `Formula Editor`
2. `Result / Effective Display`
3. `Current Function Help / Diagnostics`
4. `X-Ray`
5. `Compare / Replay / Observation`
6. `Capability and workspace support`
7. `Extension state`

### 6.3 Navigation Model
The user should be able to move between:
1. the active formula space,
2. explorer support,
3. X-Ray,
4. compare or replay details,
5. retained evidence,
6. workspace views,
7. extension state,
without losing orientation.

The app should favor:
1. docked panels,
2. drawers,
3. tabs within a region,
4. and workspace-level navigation,
over modal interruption for ordinary work.

### 6.4 Shell Model
The application should use one shell with clearly separated navigation levels.

Preferred shell structure:
1. a left rail for workspace sections and formula spaces,
2. a top context bar for active formula-space identity, mode switching, and compact host truth,
3. a main canvas that changes by task mode,
4. and a right drawer for secondary detail rather than primary navigation.

Rule:
1. do not give the left rail and the top bar the same navigation job,
2. the left rail should answer which space the user is in,
3. the mode switch should answer what kind of task the user is doing in that space,
4. drawers and inspectors should answer what supporting detail is currently open.

### 6.5 Task Modes
The product should use three explicit task modes within a formula space:
1. `Explore`
2. `Inspect`
3. `Workbench`

These map to the product order:
1. `Explore` for `Formula / Function Explorer`,
2. `Inspect` for `Live Formula Semantic X-Ray`,
3. `Workbench` for compare, replay, observation, witness, and handoff work.

The modes should feel like coordinated perspectives over the same active formula space,
not separate products or unrelated routes.

### 6.6 Information Hierarchy
The UX should distinguish five information ranks:
1. `Primary`
2. `Secondary`
3. `Supporting`
4. `Reference`
5. `Administrative`

Interpretation:
1. `Primary` is the main task content for the current mode,
2. `Secondary` is needed nearby for interpretation or next action,
3. `Supporting` is important but should not crowd the focal task,
4. `Reference` is available on demand rather than always visible,
5. `Administrative` is workspace or environment truth, not scenario-center content.

### 6.7 Ownership Model
The UX should distinguish workspace-level, formula-space-level, run-level, and comparison-level truth.

Workspace-level:
1. workspace navigation,
2. open, recent, and pinned spaces,
3. host profile summary,
4. capability center,
5. extension management,
6. platform gates,
7. environment truth.

Formula-space-level:
1. formula text,
2. edit state,
3. completions and current help,
4. diagnostics,
5. result and effective display,
6. scenario policy controls,
7. formatting and conditional formatting,
8. inspect/X-Ray state,
9. retained runs and compare state for that formula space.

Run-level:
1. run metadata,
2. packet kind,
3. timing,
4. replay capture,
5. lineage entry.

Comparison-level:
1. comparison envelope,
2. reliability,
3. mismatches,
4. blocked dimensions,
5. widening and handoff status.

Rule:
1. if a control changes the meaning or reproducibility of one formula scenario, it should normally live on that formula space,
2. if a control describes the admitted environment shared by many spaces, it should normally live at workspace level.

## 7. Formula Editing Experience
Formula editing is a first-class product problem and should receive unusual care.

In this product, “formula editing” is shorthand for editing the entered cell text of one Excel-like calculation cell.
That means the same editor must support:
1. leading-`=` formulas,
2. direct value entry,
3. apostrophe-forced string entry,
4. and the resulting Excel-like effective-display behavior under the admitted host policy.

### 7.1 Editing Goals
Editing should feel:
1. easy,
2. precise,
3. forgiving,
4. informative,
5. and pleasant under repeated use.

The editor should not feel like a generic textarea with syntax colors added later.
It also should not assume that every valid entry begins with `=`.

### 7.2 Core Editing Behaviors
The desired baseline includes:
1. multi-line editing,
2. syntax-aware diagnostics,
3. deterministic completion,
4. current function and argument help,
5. visible cursor and selection fidelity,
6. stable undo and redo,
7. good paste behavior,
8. keyboard shortcuts that match ordinary editor expectations where reasonable.

Interpretation rule:
1. when the entry is a direct value or apostrophe-forced string rather than a leading-`=` formula, the editor should still use the same surface,
2. but formula-specific assists should activate only when semantically relevant,
3. effective display should reflect OxFml-driven interpretation plus current formatting context,
4. and the editing experience should be read as a UX projection over OxFml's immutable syntax-tree and language-service packets rather than as a second local parser or semantic engine.

### 7.3 Indentation And Structure
Indented multi-line formulas should be treated as a normal and encouraged workflow.

Required behavior:
1. `Tab` indents the current line or selected lines using spaces,
2. `Shift+Tab` outdents the current line or selected lines using spaces,
3. enter and newline behavior should preserve readable indentation where possible,
4. formatting support may later assist readability, but manual editing must already feel good.

### 7.4 Editing Affordances
The editor should present:
1. completion lists that feel immediate and non-intrusive,
2. signature help that anchors to the active argument,
3. diagnostics with clear spans and readable messages,
4. effective display or result feedback without displacing the current edit context,
5. lightweight formatting cues for nested or multi-line formulas.

The editor should also be designed to admit richer future OxFunc-backed function interaction.
This should include room for:
1. richer function help prose,
2. argument-level semantic guidance,
3. surfaced function constraints or caveats,
4. category and admission-status cues,
5. and context-sensitive usage guidance while authoring.

### 7.5 Non-Goals For The First UX
The brief does not require:
1. a full IDE,
2. arbitrary custom editor scripting,
3. workbook-grid references as a public model,
4. or premature advanced refactoring tools.

## 8. Multiple Formula Spaces
The UX should be designed with the expectation that multiple formula spaces may be open at once.

This is not optional future trivia. It should shape the app model now.

### 8.1 Product Goal
Users should be able to keep multiple formula scenarios open without feeling that the app only really supports one ephemeral scratchpad.

### 8.2 Preferred Model
The preferred model is:
1. a persistent workspace rail plus tab strip as the default multi-space affordance,
2. with optional split panes later for comparison-heavy workflows,
3. with secondary windows treated as a later expansion path rather than the default model.

Reasoning:
1. tabs are compact and keep the active scenario obvious,
2. a rail can show scenario identity, dirty state, retained evidence, and grouping without consuming the editor surface,
3. split panes are useful when comparing two live spaces, but they should not become the primary way to use the app,
4. multiple windows are valuable for advanced monitoring and side-by-side inspection, but they should remain optional so the core host stays simple and browser-capable.

### 8.3 Required Multi-Space UX Properties
The product should make these clear:
1. which formula space is active,
2. which spaces are dirty,
3. which spaces have retained runs or comparison artifacts,
4. which spaces are blocked or capability-limited,
5. which spaces have loaded or blocked extensions attached,
6. and how a user moves between them without losing context.

### 8.4 State Preservation
Each formula space should retain:
1. formula text,
2. edit state,
3. help and diagnostics context,
4. result state,
5. relevant X-Ray and compare state where appropriate,
6. capability snapshot and platform gate context when the space is reopened or shared.

## 9. Explorer Surface
The explorer is the primary face of the product.

It should emphasize:
1. entered cell text,
2. result,
3. effective display,
4. function discovery,
5. diagnostics,
6. and keyboard flow.

The explorer should make scalar and array results both feel intentional.
It should also make non-formula entries feel honest and Excel-like, including direct number entry under `General` formatting and apostrophe-forced string entry.

For arrays:
1. a bounded preview is correct,
2. truncation must be explicit,
3. and larger array inspection should feel like a purposeful drill-down rather than a broken result area.

The explorer should also keep the primary result visible while support surfaces are open.
The main editing and reading path should not disappear behind drawers, overlays, or modal interruptions.

Function discovery and function understanding should be treated as a first-class companion to formula authoring.
The UX should be prepared for a richer function interaction area that can later project improved OxFunc-backed metadata and semantic guidance without redesigning the explorer around a completely different shell.

## 10. X-Ray Surface
X-Ray is the second perspective and should feel like a powerful supporting lens over the same scenario.

It should:
1. be clearly available,
2. be easy to open and close,
3. not crowd out editing and result reading,
4. and present sections that are legible to a technically serious user.

The X-Ray surface should prioritize:
1. parse,
2. bind,
3. eval,
4. trace,
5. provenance,
6. capability context.

Its visual style should read as structured and trustworthy, not dump-like.
It should remain visibly linked to the active formula space and never read as a separate developer-only page.

Where future upstream packets admit it, the X-Ray and inspect surfaces should also be able to project richer function-specific semantic guidance, including:
1. why a function behaves the way it does in the current scenario,
2. what bindings or flags affect interpretation,
3. and what current host or capability limits affect the shown semantics.

## 11. Twin Oracle Workbench
Compare, replay, and observation are the third major product perspective.

This area should feel more like an evidence workbench than an error panel.

It should foreground:
1. reliability badge,
2. comparison envelope,
3. mismatch meaning,
4. projection limitations,
5. blocked dimensions,
6. widening or handoff next actions.

The user should understand:
1. what matches,
2. what differs,
3. what the current observation envelope cannot support,
4. what is lossy,
5. and what additional capture or upstream work is needed.
The workbench should keep the comparison result readable while still exposing the scenario, run, and evidence lineage that produced it.

## 12. Workspace And Capability Support
Workspace and capability surfaces are supporting product infrastructure.

They should be useful without becoming dominant.

### 12.1 Workspace
Workspace UX should cover:
1. multiple open formula spaces,
2. recent and retained scenarios,
3. scenario grouping or filters,
4. quick reopening,
5. visible dirty and retained state,
6. no accidental implication of workbook-style shared semantics,
7. tab and rail navigation that preserves the active scenario while moving across spaces.

The left rail should be the primary durable navigator for:
1. workspace sections,
2. open formula spaces,
3. pinned spaces,
4. compact retained or blocked state.

The top bar should not duplicate that navigation.
It should carry:
1. active space identity,
2. task-mode switching,
3. compact host and capability truth,
4. lightweight global actions.

### 12.2 Capability Center
Capability UX should:
1. explain what the current build and host admit,
2. show mode availability honestly,
3. support diffing and export later,
4. and remain a supporting honesty surface rather than a central workbench obstruction.

Capability and extension status should be visible as partial truth, not hidden behind generic failure copy.
If a surface is blocked, the UI should name the capability gap, the platform gate, or the admission rule that caused it.

## 13. Extension UX
Extension hosting is not a large first-wave UX surface, but it should be planned now.

Expected scope:
1. activation options,
2. extension presence and state display,
3. capability implications,
4. blocked or incompatible conditions,
5. and lightweight trust messaging.

The extension surface should likely appear as:
1. a workspace or settings-adjacent panel,
2. a small status area,
3. and contextual activation affordances when extension-backed behavior matters.

It should not dominate the explorer.

The user should be able to see:
1. what extensions are loaded,
2. what is enabled,
3. what host or platform gate applies,
4. what failed,
5. and what extension-backed capabilities are currently active,
6. and whether an extension is merely declared, loaded, blocked, failed, or incompatible.

## 14. Visual Direction
The application should be pretty, modern, and distinctive.

It should not look like:
1. a generic enterprise dashboard,
2. a default purple SaaS template,
3. or a developer tool with accidental colors.

### 14.1 Style Thesis
The preferred direction is:
1. contemporary,
2. warm,
3. confident,
4. slightly retro in palette and tone,
5. but still sharp and modern in layout and typography.

### 14.2 Palette Direction
The palette should favor:
1. warm neutrals, parchment, sand, ink, or smoke as grounding tones,
2. rich greens, teals, rusts, amber, terracotta, muted red, or deep blue as accent families,
3. strong but controlled contrast for results, warnings, compare state, and active focus.

The palette should feel memorable and tasteful rather than nostalgic for its own sake.

### 14.3 Typography And Density
Typography should be:
1. expressive enough to give the app a point of view,
2. highly legible for dense technical content,
3. and capable of separating editorial labels, code-like formula text, and artifact truth cleanly.

Density should be:
1. efficient for serious work,
2. but not cramped,
3. with enough whitespace to make panels and priorities obvious.

### 14.4 Motion
Motion should be present but restrained.

Use motion for:
1. panel reveal,
2. workspace switching,
3. compare-state emphasis,
4. and graceful result or help updates.

Avoid ornamental movement that competes with technical reading.

## 15. Responsive And Cross-Host Behavior
The UX should degrade and adapt cleanly across:
1. desktop windows,
2. narrow laptop widths,
3. browser-hosted wide layouts,
4. and constrained web widths.

Responsive behavior should preserve:
1. formula editing viability,
2. result visibility,
3. clear access to support surfaces,
4. and stable navigation between formula spaces.

On narrower layouts, supporting surfaces should collapse or stack before primary formula and result surfaces are sacrificed.

Mode switching, rail behavior, and drawers should also degrade cleanly:
1. the left rail may compress before the editor and result are reduced,
2. the right drawer may collapse into overlay or tabbed detail before displacing the main canvas,
3. the current mode and active formula space must remain obvious at narrow widths.

## 16. State And Latency Model
Because compute placement may vary, the UX should explicitly support:
1. instant local response,
2. brief loading states,
3. longer-running retained or compare operations,
4. capability-gated unavailable states,
5. offline or degraded states.

The UX should make these states distinct:
1. `ready`,
2. `evaluating`,
3. `reopening`,
4. `comparing`,
5. `blocked`,
6. `lossy`,
7. `error`,
8. `stale`.

## 17. Accessibility And Quality Bar
Accessibility is part of the brief, not a later add-on.

The UX should support:
1. keyboard reachability,
2. readable contrast,
3. visible focus,
4. scroll independence without trap behavior,
5. meaningful labels for state and action.

Quality bar:
1. formula and result must remain visible in ordinary use,
2. support surfaces must not accidentally hide the main task,
3. compare and replay limitations must be intelligible,
4. the app must feel like one coherent product across desktop and web hosts.

Coherence rule:
1. richer future help, semantic guidance, and function-specific interaction should deepen the existing product surfaces,
2. they should not require OneCalc to fork into a separate reference browser or a separate function-help product.

## 18. Acceptance Outcomes
This brief should be considered successful when:
1. the explorer feels pleasant and efficient for real repeated use,
2. multi-line formula editing feels intentional rather than improvised,
3. multiple formula spaces feel designed-in rather than bolted on,
4. X-Ray and compare surfaces are powerful without crowding the primary task,
5. extension state is visible and honest with minimal clutter,
6. the visual system is recognizably distinctive and implementation-ready for desktop and web hosts,
7. and future implementation can proceed without falling back into terminal, harness, or generic-dashboard thinking.

It should also be successful when:
1. `Explore`, `Inspect`, and `Workbench` have clearly different default information hierarchies,
2. workspace-level truth and formula-space-level truth are not blurred together,
3. scenario policy controls have a coherent home,
4. comparison, replay, and evidence surfaces read as part of the same product shell without crowding ordinary exploration.

## 19. Immediate Design Follow-Up
The next UX-design work should produce:
1. canonical screen or panel compositions for explorer, X-Ray, compare, workspace, and extension states,
2. a formula-editing interaction spec,
3. a multiple-formula-space navigation model,
4. a visual system token proposal,
5. a component or panel inventory suitable for Tauri and browser implementation,
6. and a UX acceptance checklist that can later be tied to beads and regression coverage.

The next architecture-facing UX work should also produce:
1. a surface inventory,
2. an ownership matrix,
3. a mode-by-mode default visibility map,
4. a scenario-policy control inventory,
5. and an explicit statement of which surfaces are primary, secondary, supporting, reference, or administrative in each mode.

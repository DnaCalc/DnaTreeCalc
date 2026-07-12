# DNA OneCalc Explore Screen Spec

Status: `working_screen_spec`
Date: 2026-04-05
Scope: constrained screen spec for the `Explore` mode of `DnaOneCalc`

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)
5. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)
6. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)
7. [APP_UX_USE_CASES.md](APP_UX_USE_CASES.md)
8. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)

## 1. Purpose
This note defines the current constrained screen spec for the `Explore` mode.

It exists to:
1. describe the default `Explore` composition,
2. place the named panels from the panel inventory,
3. define what the user can see and change,
4. validate the screen against the `EX-*` use cases,
5. and keep the screen within current scope.

It is not:
1. a pixel-perfect layout spec,
2. an implementation component tree,
3. or a commitment to any panel that is still marked `needs_clarification` or `future_hook`.

## 2. Screen Identity
`Explore` is the first and default product face.

Its job is:
1. author a formula,
2. discover functions,
3. inspect diagnostics,
4. inspect result and effective display,
5. and preserve scenario meaning clearly.

Input rule:
1. the primary entry surface accepts any Excel cell-entry text,
2. including leading-`=` formulas, direct value entry, and apostrophe-forced string entry,
3. while preserving one coherent editing experience.

It must feel like:
1. a serious formula editor,
2. a result-reading surface,
3. and a function-discovery companion.

It must not feel like:
1. a full IDE,
2. a worksheet grid,
3. or a host-settings dashboard.

## 3. Screen Anchors
This screen resolves the following anchors from [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md):
1. `shell.left_rail`
2. `shell.context_bar`
3. `shell.footer`
4. `explore.editor_cluster`
5. `explore.result_cluster`
6. `explore.help_cluster`
7. `explore.policy_entry`
8. `explore.drawer.detail`

## 4. Default Composition
### 4.1 Shared Shell
Always visible:
1. `workspace_nav_panel`
2. `formula_space_list_panel`
3. `extension_state_panel`
4. `environment_truth_panel`
5. `formula_space_context_panel`
6. `mode_switch_panel`
7. `host_truth_panel`
8. `status_footer_panel`

### 4.2 Main Canvas
The main canvas should resolve into three functional clusters:

1. `explore.editor_cluster`
   1. `formula_editor_panel`
   2. `diagnostics_panel`

2. `explore.result_cluster`
   1. `result_panel`
   2. `effective_display_panel`
   3. `array_preview_panel` when relevant

3. `explore.help_cluster`
   1. `completion_help_panel`

### 4.3 Context Bar
The context bar should visibly carry:
1. active formula-space identity,
2. dirty or retained cues,
3. `scenario_policy_panel` summary,
4. mode identity,
5. compact host truth.

### 4.4 Right Drawer
The right drawer is secondary detail only.

It may contain:
1. `formatting_panel` if admitted,
2. `conditional_formatting_panel` if admitted,
3. expanded current help,
4. expanded completion browser,
5. compact capability detail.

It must not become:
1. the primary editing area,
2. the primary diagnostics area,
3. or the primary result-reading area.

## 5. Panel Rank
### 5.1 Primary
1. `formula_editor_panel`
2. `result_panel`

### 5.2 Secondary
1. `diagnostics_panel`
2. `completion_help_panel`
3. `effective_display_panel`
4. `scenario_policy_panel`

### 5.3 Supporting
1. `array_preview_panel`
2. `host_truth_panel`
3. `environment_truth_panel`
4. `extension_state_panel`

### 5.4 Reference
1. `formatting_panel`
2. `conditional_formatting_panel`
3. `capability_center_panel`

## 6. Updateability
### 6.1 User Can Change
1. formula text
2. cursor and selection
3. edit structure and indentation
4. active completion choice
5. scenario policy for the current formula space
6. formatting or conditional-formatting settings only if those surfaces are admitted

Interpretation rule:
1. “formula text” here means the raw entered cell text for the active formula space,
2. typed meaning and effective-display outcomes remain OxFml-derived,
3. syntax coloration, diagnostics, completion, and signature-help surfaces should be projected from OxFml editor packets rather than reconstructed from host-local parser state.

### 6.2 User Cannot Change Here
1. result values directly
2. diagnostics directly
3. capability truth
4. platform gates
5. retained comparison history except through explicit actions

## 7. Editing Requirements
The `formula_editor_panel` must support:
1. multi-line authoring,
2. large pasted formulas,
3. vertical internal scrolling without losing context,
4. clear line and structure orientation,
5. `Tab` and `Shift+Tab` indentation with spaces,
6. stable cursor movement and selection behavior,
7. readable editing under long-formula conditions.

Long-formula behavior:
1. the editor should remain the dominant visible surface,
2. scrolling inside the editor should not displace the result entirely,
3. the user should not need to leave `Explore` just to navigate the formula,
4. the editor may expand within limits, but the result and diagnostics should remain reachable.

## 8. Attention Paths
### 8.1 Authoring Path
1. `formula_editor_panel`
2. `completion_help_panel`
3. `diagnostics_panel`
4. `result_panel`

### 8.2 Repair Path
1. error location in `formula_editor_panel`
2. `diagnostics_panel`
3. `completion_help_panel`
4. repaired `result_panel`

### 8.3 Meaning And Reproducibility Path
1. `scenario_policy_panel`
2. `formula_editor_panel`
3. `result_panel`
4. `host_truth_panel`

## 9. Interaction Paths
### 9.1 Keyboard-First
1. author or paste formula,
2. move through text with ordinary editor navigation,
3. indent and outdent with `Tab` and `Shift+Tab`,
4. inspect completions without leaving the editor,
5. evaluate,
6. review diagnostics and result,
7. switch to `Inspect` or `Workbench` if needed.

### 9.2 Pointer-Capable
1. focus editor,
2. use visible help and completion surfaces nearby,
3. open drawer detail only when secondary detail is needed,
4. keep the edit-result-help loop intact.

## 10. State Cases
The screen must handle:
1. clean editable formula,
2. invalid formula with readable diagnostics,
3. valid formula with unexpected result,
4. array-shaped or array-using formula,
5. long structured formula,
6. deterministic policy versus live policy,
7. retained run available for later workbench use.

## 11. Responsiveness
Responsive behavior must preserve meaning before preserving every panel.

Degradation order:
1. compress the left rail before shrinking the editor cluster too far,
2. keep editor and result visible together as long as possible,
3. collapse or defer deeper help detail before sacrificing core editor readability,
4. treat the right drawer as optional secondary detail on narrow widths,
5. preserve context bar identity and policy summary.

## 12. Use Case Coverage
This screen is the primary home for:
1. `EX-01`
2. `EX-02`
3. `EX-03`
4. `EX-04`
5. `EX-05`
6. `EX-06`
7. `EX-07`
8. `EX-08`
9. `EX-09`
10. `EX-10`

It also provides entry paths for:
1. `IN-09`
2. `WB-10`

## 13. Out Of Scope Or Unfrozen
The following are not committed by this screen spec:
1. full IDE-grade refactoring features,
2. workbook-grid navigation,
3. arbitrary modular panel docking,
4. rich OxFunc prose help beyond current admitted payloads,
5. `formatting_panel` and `conditional_formatting_panel` beyond their current `needs_clarification` status.

## 14. Derived Next Step
This screen spec should feed:
1. later screen-detail elements under `explore.editor_cluster`,
2. later screen-detail elements under `explore.result_cluster`,
3. later screen-detail elements under `explore.help_cluster`,
4. and host-state slicing for `Explore`.


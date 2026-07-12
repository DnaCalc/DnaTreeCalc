# DNA OneCalc Inspect Screen Spec

Status: `working_screen_spec`
Date: 2026-04-05
Scope: constrained screen spec for the `Inspect` mode of `DnaOneCalc`

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
This note defines the current constrained screen spec for the `Inspect` mode.

It exists to:
1. describe the default semantic-inspection composition,
2. place the named inspect panels,
3. define what the user can inspect and what remains read-only,
4. validate the screen against the `IN-*` use cases,
5. and keep the mode within current scope.

It is not:
1. a debugger spec,
2. a raw engine dump spec,
3. or permission to overexpose coordinator or packet internals.

## 2. Screen Identity
`Inspect` is the semantic and mechanism-reading perspective over the current formula space.

Its job is:
1. show structure,
2. show bindings,
3. show evaluation-facing summaries,
4. show provenance and interpretation truth,
5. and support movement back to authoring or forward to workbench.

It must feel like:
1. a structured semantic lens,
2. a read-only analysis mode,
3. and a trustworthy explanation surface.

It must not feel like:
1. a scrolling log dump,
2. a step debugger,
3. or a second editor.

## 3. Screen Anchors
This screen resolves the following anchors from [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md):
1. `shell.left_rail`
2. `shell.context_bar`
3. `shell.footer`
4. `inspect.source_cluster`
5. `inspect.walk_cluster`
6. `inspect.summary_cluster`
7. `inspect.drawer.detail`

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

1. `inspect.source_cluster`
   1. `source_formula_panel`
   2. `inspect_result_panel`

2. `inspect.walk_cluster`
   1. `formula_walk_panel`

3. `inspect.summary_cluster`
   1. `parse_summary_panel`
   2. `bind_summary_panel`
   3. `eval_summary_panel`
   4. `provenance_summary_panel`
   5. `host_context_panel`

### 4.3 Context Bar
The context bar should visibly carry:
1. active formula-space identity,
2. mode identity,
3. compact scenario-policy summary,
4. compact host truth,
5. clear path back to `Explore` and onward to `Workbench`.

### 4.4 Right Drawer
The right drawer is secondary detail only.

It may contain:
1. `inspect_detail_panel`
2. deeper node detail,
3. deeper provenance detail,
4. deeper host context detail,
5. richer function guidance if later admitted.

It must not become:
1. the primary semantic surface,
2. the main tree-reading surface,
3. or a raw internal dump.

## 5. Panel Rank
### 5.1 Primary
1. `formula_walk_panel`

### 5.2 Secondary
1. `source_formula_panel`
2. `inspect_result_panel`
3. `parse_summary_panel`
4. `bind_summary_panel`
5. `eval_summary_panel`
6. `provenance_summary_panel`

### 5.3 Supporting
1. `host_context_panel`
2. `scenario_policy_panel` as compact summary
3. `host_truth_panel`

### 5.4 Reference
1. `inspect_detail_panel`
2. `capability_center_panel`
3. future richer `function_detail_panel`

## 6. Updateability
### 6.1 User Can Change
`Inspect` is primarily read-only.

The user may change:
1. selected inspect focus,
2. tree expansion and collapse state,
3. open or closed detail drawer state,
4. mode transition back to `Explore` or onward to `Workbench`.

### 6.2 User Cannot Change Here
1. formula text directly
2. result directly
3. comparison evidence directly
4. workspace-global host truth
5. deep scenario policy as a primary editing flow

Scenario policy in this mode should be:
1. visible as summary,
2. low-emphasis,
3. and normally edited by returning to `Explore`.

## 7. Formula Walk Requirements
The `formula_walk_panel` must support:
1. readable hierarchical structure,
2. expansion and collapse by branch,
3. visible state categories such as evaluated, bound, opaque, and blocked,
4. the ability to keep attention on one branch without losing overall orientation,
5. a path to deeper detail without making the drawer mandatory.

The formula walk must not degrade into:
1. an unreadable tree wall,
2. a flat chronological trace,
3. or a raw packet transcript.

## 8. Attention Paths
### 8.1 Mechanism Path
1. `formula_walk_panel`
2. selected subtree
3. `bind_summary_panel` or `eval_summary_panel`
4. `inspect_detail_panel` only if needed

### 8.2 Structural Path
1. `source_formula_panel`
2. `formula_walk_panel`
3. `parse_summary_panel`

### 8.3 Context Path
1. `eval_summary_panel`
2. `provenance_summary_panel`
3. `host_context_panel`
4. compact `scenario_policy_panel`

## 9. Interaction Paths
### 9.1 Keyboard-Capable
1. enter `Inspect`,
2. move focus through the walk,
3. expand or collapse branches,
4. open detail on the focused node,
5. return to `Explore` if changes are needed.

### 9.2 Pointer-Capable
1. select relevant branch,
2. inspect summaries,
3. open drawer detail only when the summary layer is insufficient,
4. use the mode switch to move to another perspective.

## 10. State Cases
The screen must handle:
1. valid formula with expected result,
2. valid formula with unexpected inner-function behavior,
3. parse-shape confusion,
4. blocked or opaque inspect regions,
5. deep nested structure,
6. host- or policy-sensitive interpretation,
7. transition to Windows-only compare where admitted.

## 11. Responsiveness
Responsive behavior must preserve the semantic lens.

Degradation order:
1. compress left-rail detail before weakening the formula walk,
2. compress secondary source and summary clusters before sacrificing walk readability,
3. keep the detail drawer optional,
4. preserve mode identity and compact policy/host truth.

## 12. Use Case Coverage
This screen is the primary home for:
1. `IN-01`
2. `IN-02`
3. `IN-03`
4. `IN-04`
5. `IN-05`
6. `IN-06`
7. `IN-07`
8. `IN-08`
9. `IN-09`
10. `IN-10`

It also supports:
1. `EX-05`
2. `WB-09`

## 13. Out Of Scope Or Unfrozen
The following are not committed by this screen spec:
1. debugger-like stepping controls,
2. arbitrary timeline replay inside `Inspect`,
3. raw packet or engine dumps as the primary experience,
4. fine-grained partial evaluation beyond what the current seam honestly admits,
5. richer function semantics beyond current admitted payloads.

## 14. Derived Next Step
This screen spec should feed:
1. later screen-detail elements under `inspect.source_cluster`,
2. later screen-detail elements under `inspect.walk_cluster`,
3. later screen-detail elements under `inspect.summary_cluster`,
4. and host-state slicing for `Inspect`.


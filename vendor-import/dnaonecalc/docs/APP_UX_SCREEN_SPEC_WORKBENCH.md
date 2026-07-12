# DNA OneCalc Workbench Screen Spec

Status: `working_screen_spec`
Date: 2026-04-05
Scope: constrained screen spec for the `Workbench` mode of `DnaOneCalc`

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
This note defines the current constrained screen spec for the `Workbench` mode.

It exists to:
1. describe the retained-evidence and comparison composition,
2. place the named workbench panels,
3. define what the user can act on,
4. validate the screen against the `WB-*` use cases,
5. and keep the mode within current product scope.

It is not:
1. a workflow-automation console,
2. a case-management system,
3. or permission to overclaim compare lanes that the current platform cannot honestly support.

## 2. Screen Identity
`Workbench` is the retained evidence, compare, replay, and handoff perspective over the same active scenario.

Its job is:
1. show comparison outcome,
2. show replay lineage,
3. show evidence bundle identity,
4. explain blocked dimensions and comparison limits,
5. and let the user retain, export, widen, or hand off when admitted.

It must feel like:
1. an evidence workbench,
2. a proving-host surface,
3. and an action-bearing but disciplined mode.

It must not feel like:
1. a generic admin dashboard,
2. a broad back-office artifact manager,
3. or a separate application disconnected from the active formula space.

## 3. Screen Anchors
This screen resolves the following anchors from [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md):
1. `shell.left_rail`
2. `shell.context_bar`
3. `shell.footer`
4. `workbench.outcome_cluster`
5. `workbench.lineage_cluster`
6. `workbench.evidence_cluster`
7. `workbench.action_cluster`
8. `workbench.drawer.detail`

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
The main canvas should resolve into four functional clusters:

1. `workbench.outcome_cluster`
   1. `comparison_outcome_panel`
   2. `blocked_dimensions_panel`

2. `workbench.lineage_cluster`
   1. `replay_lineage_panel`
   2. `source_run_panel`

3. `workbench.evidence_cluster`
   1. `evidence_bundle_panel`
   2. `observation_envelope_panel`

4. `workbench.action_cluster`
   1. `handoff_panel`

### 4.3 Context Bar
The context bar should visibly carry:
1. active formula-space identity,
2. mode identity,
3. compact scenario-policy summary,
4. compact host truth,
5. visible indication of platform and compare-lane truth.

### 4.4 Right Drawer
The right drawer is secondary detail only.

It may contain:
1. `workbench_detail_panel`
2. fuller observation-envelope detail,
3. fuller evidence-bundle detail,
4. fuller handoff detail,
5. future witness detail if later admitted.

It must not become:
1. the primary workbench logic surface,
2. the place where compare meaning is first understood,
3. or a substitute for clear primary action surfaces.

## 5. Panel Rank
### 5.1 Primary
1. `comparison_outcome_panel`
2. `replay_lineage_panel`

### 5.2 Secondary
1. `evidence_bundle_panel`
2. `blocked_dimensions_panel`
3. `handoff_panel`
4. `observation_envelope_panel`
5. `source_run_panel`

### 5.3 Supporting
1. compact `scenario_policy_panel`
2. `host_truth_panel`
3. `environment_truth_panel`

### 5.4 Reference
1. `workbench_detail_panel`
2. `capability_center_panel`
3. future `witness_chain_panel`

## 6. Updateability
### 6.1 User Can Change
1. retained state
2. selected comparison focus
3. action state for replay, compare, export, widen, or handoff when admitted
4. drawer detail state

### 6.2 User Cannot Change Here
1. primary formula text as the main authoring flow
2. scenario policy as the main editing flow
3. workspace-global host truth
4. platform gates
5. unsupported compare lanes hidden behind capability limits

## 7. Evidence Requirements
The workbench must make the following clear:
1. what was compared,
2. what matched,
3. what differed,
4. what was blocked,
5. what evidence objects exist,
6. what next action is available.

The workbench must not:
1. hide comparison incompleteness,
2. imply a stronger evidence floor than actually exists,
3. or blur source run, observation, comparison, and handoff into one generic artifact.

## 8. Attention Paths
### 8.1 Compare Path
1. `comparison_outcome_panel`
2. `blocked_dimensions_panel`
3. `replay_lineage_panel`
4. `observation_envelope_panel`

### 8.2 Escalation Path
1. `source_run_panel`
2. `evidence_bundle_panel`
3. `handoff_panel`
4. optional `workbench_detail_panel`

### 8.3 Honest Degradation Path
1. `host_truth_panel`
2. `environment_truth_panel`
3. `comparison_outcome_panel`
4. `blocked_dimensions_panel`

## 9. Interaction Paths
### 9.1 Native Compare Path
1. enter `Workbench`,
2. inspect or trigger compare if admitted,
3. inspect outcome and lineage,
4. interpret blocked dimensions,
5. retain, export, or hand off.

### 9.2 Browser-Honest Path
1. enter `Workbench`,
2. inspect admitted capability and platform truth,
3. inspect replay and retained evidence where compare is unavailable,
4. understand absence or disabling of Windows-only compare surfaces.

## 10. State Cases
The screen must handle:
1. known-good comparison,
2. mismatch requiring interpretation,
3. partial or lossy observation,
4. blocked dimensions,
5. retained evidence without Excel-observed compare,
6. Windows-admitted compare path,
7. browser-hosted honest degradation,
8. escalation or handoff preparation.

## 11. Responsiveness
Responsive behavior must preserve evidence meaning.

Degradation order:
1. compress left-rail detail before weakening outcome and lineage comprehension,
2. keep outcome and lineage visible before secondary bundle detail,
3. treat the right drawer as optional,
4. preserve platform and capability truth clearly in all widths.

## 12. Use Case Coverage
This screen is the primary home for:
1. `WB-01`
2. `WB-02`
3. `WB-03`
4. `WB-04`
5. `WB-05`
6. `WB-06`
7. `WB-07`
8. `WB-08`
9. `WB-09`
10. `WB-10`

It also receives entry transitions from:
1. `EX-10`
2. `IN-10`

## 13. Out Of Scope Or Unfrozen
The following are not committed by this screen spec:
1. full case-management structure,
2. workflow automation UI,
3. a separate replay sub-application,
4. broad witness-chain tooling beyond current `future_hook` status,
5. overclaiming Windows-only Excel compare in browser-hosted builds.

## 14. Derived Next Step
This screen spec should feed:
1. later screen-detail elements under `workbench.outcome_cluster`,
2. later screen-detail elements under `workbench.lineage_cluster`,
3. later screen-detail elements under `workbench.evidence_cluster`,
4. later screen-detail elements under `workbench.action_cluster`,
5. and host-state slicing for `Workbench`.


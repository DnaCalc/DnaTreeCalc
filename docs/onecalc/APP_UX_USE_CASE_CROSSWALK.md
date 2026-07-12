# DNA OneCalc UX Use Case Crosswalk

Status: `working_crosswalk_note`
Date: 2026-04-05
Scope: cross-correlation of OneCalc use cases with modes, panels, mutable surfaces, gates, and future screen-detail anchors

Companion notes:
1. [APP_UX_USE_CASES.md](APP_UX_USE_CASES.md)
2. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)
3. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)
5. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)

## 1. Purpose
This note cross-correlates the scenario-driven use cases with the current shell, mode, panel, and mutability model.

It exists to:
1. show which panels each use case actually depends on,
2. show which surfaces are mutable during each task,
3. show which cases need mode transitions,
4. capture platform and capability gates,
5. and define stable future screen-detail anchors for later screen specs.

It is not:
1. a screen spec,
2. a component map,
3. or permission to invent new panels for uncovered cases without updating current scope.

## 2. How To Read This Note
Each use case is mapped across:
1. `Primary Mode`
2. `Mode Transitions`
3. `Primary Panels`
4. `Supporting Panels`
5. `Mutable Surfaces`
6. `Platform / Gate Notes`
7. `Future Screen Anchors`

Interpretation:
1. `Primary Panels` are the panels that must work well for the use case to succeed,
2. `Supporting Panels` improve the flow but should not dominate it,
3. `Mutable Surfaces` are where user change is expected,
4. `Future Screen Anchors` are stable placeholders for later screen-detail specs.

## 3. Future Screen Anchor Set
The following anchors are reserved for later screen specs.

### 3.1 Shell Anchors
1. `shell.left_rail`
2. `shell.context_bar`
3. `shell.footer`

### 3.2 Explore Anchors
1. `explore.editor_cluster`
2. `explore.result_cluster`
3. `explore.help_cluster`
4. `explore.policy_entry`
5. `explore.drawer.detail`

### 3.3 Inspect Anchors
1. `inspect.source_cluster`
2. `inspect.walk_cluster`
3. `inspect.summary_cluster`
4. `inspect.drawer.detail`

### 3.4 Workbench Anchors
1. `workbench.outcome_cluster`
2. `workbench.lineage_cluster`
3. `workbench.evidence_cluster`
4. `workbench.action_cluster`
5. `workbench.drawer.detail`

Rule:
1. later screen-detail elements should hang off these anchors rather than creating a fresh naming system.

## 4. Explore Crosswalk
| Use Case | Primary Mode | Mode Transitions | Primary Panels | Supporting Panels | Mutable Surfaces | Platform / Gate Notes | Future Screen Anchors |
|---|---|---|---|---|---|---|---|
| `EX-01` Unexpected Scalar Result | Explore | None or `Explore -> Inspect` | `formula_editor_panel`, `diagnostics_panel`, `result_panel` | `completion_help_panel`, `scenario_policy_panel` | Formula text, edit state | No special gate | `shell.context_bar`, `explore.editor_cluster`, `explore.result_cluster`, `explore.help_cluster` |
| `EX-02` Very Long Formula Authoring | Explore | None | `formula_editor_panel` | `diagnostics_panel`, `result_panel` | Formula text, cursor, selection, indentation | Core editor-quality case; no special gate | `explore.editor_cluster`, `shell.footer` |
| `EX-03` Completion-Led Function Discovery | Explore | None | `formula_editor_panel`, `completion_help_panel` | `diagnostics_panel`, `result_panel` | Formula text, completion acceptance | Richer help remains a future hook | `explore.editor_cluster`, `explore.help_cluster` |
| `EX-04` Signature And Argument Guidance | Explore | None | `formula_editor_panel`, `completion_help_panel` | `diagnostics_panel` | Formula text | Help richness depends on upstream payload floor | `explore.editor_cluster`, `explore.help_cluster` |
| `EX-05` Array-Aware Exploration | Explore | Optional `Explore -> Inspect` | `result_panel`, `array_preview_panel`, `formula_editor_panel` | `completion_help_panel` | Formula text | Array preview is supporting, not grid semantics | `explore.editor_cluster`, `explore.result_cluster` |
| `EX-06` Formatting Truth Check | Explore | None | `result_panel`, `effective_display_panel` | `formatting_panel` | Formatting if admitted | `formatting_panel` is `needs_clarification` | `explore.result_cluster`, `explore.drawer.detail` |
| `EX-07` Deterministic Versus Real-Time Policy Check | Explore | None | `scenario_policy_panel`, `formula_editor_panel`, `result_panel` | `host_truth_panel` | Scenario policy | Core planned policy case | `shell.context_bar`, `explore.policy_entry`, `explore.result_cluster` |
| `EX-08` Multiple Formula Spaces | Explore | Space switching only | `formula_space_list_panel`, `formula_editor_panel` | `result_panel`, `diagnostics_panel`, `workspace_nav_panel` | Active space, pinned state, formula text per space | Shell coherence case | `shell.left_rail`, `explore.editor_cluster`, `explore.result_cluster` |
| `EX-09` Invalid Formula Repair | Explore | None | `formula_editor_panel`, `diagnostics_panel` | `completion_help_panel`, `result_panel` | Formula text, cursor and selection | Diagnostics must stay near editor | `explore.editor_cluster`, `explore.help_cluster` |
| `EX-10` Capture A Run From Explore | Explore | Optional `Explore -> Workbench` | `formula_editor_panel`, `result_panel`, `scenario_policy_panel` | `formula_space_context_panel`, `host_truth_panel` | Formula text, scenario policy, retain action entry point | Retention path should preserve context | `shell.context_bar`, `explore.editor_cluster`, `explore.result_cluster`, `workbench.evidence_cluster` |

## 5. Inspect Crosswalk
| Use Case | Primary Mode | Mode Transitions | Primary Panels | Supporting Panels | Mutable Surfaces | Platform / Gate Notes | Future Screen Anchors |
|---|---|---|---|---|---|---|---|
| `IN-01` Unexpected Inner Function Behavior | Inspect | None or `Inspect -> Explore` | `formula_walk_panel`, `bind_summary_panel`, `eval_summary_panel` | `source_formula_panel`, `inspect_detail_panel` | Tree expansion, selected inspect focus | Fine-grained node detail may pressure upstream seams | `inspect.walk_cluster`, `inspect.summary_cluster`, `inspect.drawer.detail` |
| `IN-02` Bound Name Investigation | Inspect | None | `formula_walk_panel`, `bind_summary_panel` | `inspect_detail_panel`, `source_formula_panel` | Tree expansion, selected node | Core semantic-inspection case | `inspect.walk_cluster`, `inspect.summary_cluster` |
| `IN-03` Blocked Or Opaque Reason Check | Inspect | None | `formula_walk_panel`, `provenance_summary_panel` | `inspect_detail_panel`, `host_context_panel` | Selected inspect focus | Honesty-surface case; reason quality may depend on seams | `inspect.walk_cluster`, `inspect.summary_cluster`, `inspect.drawer.detail` |
| `IN-04` Parse Structure Investigation | Inspect | None | `formula_walk_panel`, `parse_summary_panel`, `source_formula_panel` | `inspect_result_panel` | Tree expansion | Parse structure is planned scope | `inspect.source_cluster`, `inspect.walk_cluster`, `inspect.summary_cluster` |
| `IN-05` Evaluation Context Check | Inspect | None | `eval_summary_panel`, `host_context_panel`, `provenance_summary_panel` | `scenario_policy_panel` summary, `inspect_detail_panel` | Inspect focus only | Packet detail must remain interpretive | `shell.context_bar`, `inspect.summary_cluster`, `inspect.drawer.detail` |
| `IN-06` Read-Only Confidence Path | Inspect | `Inspect -> Explore` only if needed | `source_formula_panel`, `formula_walk_panel`, `inspect_result_panel` | `formula_space_context_panel` | None beyond navigation and inspect focus | Read-only discipline case | `inspect.source_cluster`, `inspect.walk_cluster` |
| `IN-07` Function Support Status During Inspection | Inspect | None | `formula_walk_panel`, `host_context_panel` | `inspect_detail_panel`, `capability_center_panel` | Inspect focus only | Richer function semantics are a future hook | `inspect.walk_cluster`, `inspect.summary_cluster`, `inspect.drawer.detail` |
| `IN-08` Attention Path For Deep Trees | Inspect | None | `formula_walk_panel` | `inspect_result_panel`, `provenance_summary_panel` | Tree expansion and collapse | Visual hierarchy stress case | `inspect.walk_cluster`, `inspect.summary_cluster` |
| `IN-09` Transition Back To Explore | Inspect | `Inspect -> Explore` | `formula_walk_panel`, `source_formula_panel` | `mode_switch_panel`, `formula_editor_panel` after switch | Navigation, then formula text in `Explore` | One-shell continuity case | `shell.context_bar`, `inspect.source_cluster`, `explore.editor_cluster` |
| `IN-10` Prepare For Excel Cross-Check | Inspect | `Inspect -> Workbench` | `formula_walk_panel`, `host_context_panel`, `scenario_policy_panel` summary | `mode_switch_panel`, `provenance_summary_panel` | Inspect focus, navigation | Windows-only compare path must be explicit later | `shell.context_bar`, `inspect.walk_cluster`, `workbench.outcome_cluster` |

## 6. Workbench Crosswalk
| Use Case | Primary Mode | Mode Transitions | Primary Panels | Supporting Panels | Mutable Surfaces | Platform / Gate Notes | Future Screen Anchors |
|---|---|---|---|---|---|---|---|
| `WB-01` Native Excel Comparison For Unexpected Result | Workbench | None | `comparison_outcome_panel`, `replay_lineage_panel`, `blocked_dimensions_panel` | `observation_envelope_panel`, `host_truth_panel` | Compare action state if admitted | Windows-gated compare lane | `workbench.outcome_cluster`, `workbench.lineage_cluster`, `workbench.action_cluster` |
| `WB-02` Browser-Only Honest Degradation | Workbench | None | `comparison_outcome_panel`, `environment_truth_panel`, `host_truth_panel` | `capability_center_panel`, `blocked_dimensions_panel` | None or limited navigation | Browser must not imply Excel-observed lane | `shell.left_rail`, `shell.context_bar`, `workbench.outcome_cluster` |
| `WB-03` Capture An Anomaly For Escalation | Workbench | None | `evidence_bundle_panel`, `handoff_panel`, `source_run_panel` | `scenario_policy_panel` summary, `workbench_detail_panel` | Retain/export/handoff actions | Core proving-host case | `workbench.evidence_cluster`, `workbench.action_cluster`, `workbench.drawer.detail` |
| `WB-04` Replay Lineage Review | Workbench | None | `replay_lineage_panel`, `comparison_outcome_panel`, `source_run_panel` | `evidence_bundle_panel` | Selected comparison focus | Replay lineage is planned scope | `workbench.lineage_cluster`, `workbench.outcome_cluster` |
| `WB-05` Mismatch Interpretation | Workbench | Optional `Workbench -> Inspect` | `comparison_outcome_panel`, `blocked_dimensions_panel`, `observation_envelope_panel` | `replay_lineage_panel`, `mode_switch_panel` | Selected comparison focus, navigation | Compare incompleteness must be visible | `workbench.outcome_cluster`, `workbench.evidence_cluster`, `inspect.walk_cluster` |
| `WB-06` Noisy Or Partial Observation | Workbench | None | `observation_envelope_panel`, `blocked_dimensions_panel`, `handoff_panel` | `comparison_outcome_panel`, `evidence_bundle_panel` | Widen/handoff actions if admitted | Evidence-quality honesty case | `workbench.evidence_cluster`, `workbench.action_cluster`, `workbench.drawer.detail` |
| `WB-07` Retain A Known-Good Witness | Workbench | None | `comparison_outcome_panel`, `replay_lineage_panel`, `evidence_bundle_panel` | `handoff_panel` | Retain action | Durable reference evidence case | `workbench.outcome_cluster`, `workbench.lineage_cluster`, `workbench.evidence_cluster` |
| `WB-08` Escalation Packet With Context Preservation | Workbench | None | `source_run_panel`, `evidence_bundle_panel`, `handoff_panel` | `scenario_policy_panel` summary, `host_truth_panel` | Handoff action state | Core proving-host case | `workbench.evidence_cluster`, `workbench.action_cluster` |
| `WB-09` Compare After Semantic Investigation | Workbench | `Inspect -> Workbench` | `comparison_outcome_panel`, `replay_lineage_panel` | `source_run_panel`, `mode_switch_panel` | Compare action state | Transition must be obvious on Windows and honest elsewhere | `inspect.walk_cluster`, `workbench.outcome_cluster`, `workbench.lineage_cluster` |
| `WB-10` Decide Next Action | Workbench | None | `comparison_outcome_panel`, `blocked_dimensions_panel`, `handoff_panel` | `reliability` content within `comparison_outcome_panel`, `evidence_bundle_panel` | Retain, replay, export, handoff actions | Closure case for workbench | `workbench.outcome_cluster`, `workbench.action_cluster`, `workbench.evidence_cluster` |

## 7. Coverage Notes
### 7.1 Panel Coverage
The following panels are heavily exercised by current use cases:
1. `formula_editor_panel`
2. `diagnostics_panel`
3. `result_panel`
4. `completion_help_panel`
5. `scenario_policy_panel`
6. `formula_walk_panel`
7. `parse_summary_panel`
8. `bind_summary_panel`
9. `eval_summary_panel`
10. `comparison_outcome_panel`
11. `replay_lineage_panel`
12. `evidence_bundle_panel`
13. `handoff_panel`

### 7.2 Panels With Lighter Current Coverage
These panels are present but still deserve later validation:
1. `conditional_formatting_panel`
2. `function_detail_panel`
3. `witness_chain_panel`
4. `capability_center_panel`

Interpretation:
1. lighter current coverage does not remove them automatically,
2. but it does mean they should stay secondary unless stronger current-scope use cases appear.

### 7.3 Cross-Mode Transition Hotspots
The most important transition paths are:
1. `Explore -> Inspect`
2. `Inspect -> Explore`
3. `Inspect -> Workbench`
4. `Explore -> Workbench`

Rule:
1. these paths should feel like perspective shifts within one scenario,
2. not like app exits or route jumps.

## 8. Derived Next Step
This crosswalk should now feed:
1. the constrained `Explore` screen spec,
2. the constrained `Inspect` screen spec,
3. the constrained `Workbench` screen spec,
4. and later a screen-detail-element map that resolves the anchors in Section 3 into concrete screen elements.

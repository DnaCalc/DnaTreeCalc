# DNA OneCalc UX Host State Slicing

Status: `working_host_state_note`
Date: 2026-04-05
Scope: implementation-facing state slicing for the current `DnaOneCalc` shell, panels, and modes

Companion notes:
1. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
2. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)
3. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)
4. [APP_UX_SCREEN_SPEC_EXPLORE.md](APP_UX_SCREEN_SPEC_EXPLORE.md)
5. [APP_UX_SCREEN_SPEC_INSPECT.md](APP_UX_SCREEN_SPEC_INSPECT.md)
6. [APP_UX_SCREEN_SPEC_WORKBENCH.md](APP_UX_SCREEN_SPEC_WORKBENCH.md)
7. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)
8. [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md)

Relevant archived reference code:
1. [shell.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/shell.rs)
2. [runtime.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/runtime.rs)
3. [retained.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/retained.rs)
4. [extension.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/extension.rs)

## 1. Purpose
This note maps the current UX model onto implementation-facing state slices for the shared `DnaOneCalc` application core.

It exists to:
1. say which panels depend on which state,
2. separate durable state from derived state,
3. distinguish workspace-level from formula-space-level state,
4. distinguish active-authoring state from retained artifact state,
5. make the gap between the archived single-space shell and the intended multi-space shell explicit,
6. and support both `Tauri` desktop and browser/WASM hosts over one shared state model.

It is not:
1. a final Rust type design,
2. a reducer or event-model spec,
3. or permission to widen UX scope.

## 2. Current Code Read
### 2.1 Archived Shell Reality
The archived pre-greenfield host already contains useful state slices in [shell.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/shell.rs):
1. `FormulaEditorState`
2. `CapabilityCenterState`
3. `EffectiveDisplayRenderState`
4. `OneCalcShellApp`

The current `OneCalcShellApp` already carries:
1. formula edit session state,
2. latest edit packet,
3. latest evaluation,
4. completion items,
5. function help,
6. rendered diagnostics,
7. compact host and capability truth text,
8. visibility toggles for support sidebar, capability center, and X-Ray.

This is useful, but it is still fundamentally:
1. single active formula-space state,
2. string-heavy in some UI-facing fields,
3. and not yet shaped around the newer multi-space and mode-driven UX.

### 2.2 Archived Runtime Summary Reality
The archived pre-greenfield runtime already exposes summary types in [runtime.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/runtime.rs) that align with the intended UX:
1. `FormulaEditPacketSummary`
2. `FormulaEvaluationSummary`
3. `ArrayPreviewSummary`
4. `CompletionProposalSummary`
5. `FunctionHelpSummary`
6. `OpenedCapabilitySnapshotSummary`
7. `OpenedReplayCaptureSummary`
8. `OpenedXRaySummary`
9. `OpenedTwinCompareSummary`
10. `OpenedWitnessSummary`
11. `OpenedHandoffPacketSummary`
12. `ScenarioSelectionDetail`

Interpretation:
1. the greenfield host does not need to invent everything from scratch,
2. but it does need a better state-owner structure to present these summaries coherently across modes and multiple formula spaces.

## 3. State-Slicing Principles
The host state should be sliced by:
1. ownership,
2. persistence,
3. mode,
4. and derivation.

Working rule:
1. persisted truth should not be mixed with transient view state,
2. formula authoring state should not be mixed with retained evidence state,
3. compact shell truth should be derivable from deeper runtime and retained summaries rather than hand-built ad hoc strings.

## 4. Proposed Top-Level State Slices
The intended shell is best modeled with these top-level slices:

1. `WorkspaceShellState`
2. `FormulaSpaceCollectionState`
3. `ActiveFormulaSpaceViewState`
4. `RetainedArtifactOpenState`
5. `CapabilityAndEnvironmentState`
6. `ExtensionSurfaceState`
7. `GlobalUiChromeState`

## 5. WorkspaceShellState
This slice should own:
1. active formula-space ID,
2. open formula-space order,
3. pinned formula-space IDs,
4. workspace navigation selection,
5. mode per formula space,
6. last-focused panel or cluster per formula space,
7. current drawer target per formula space.

This slice should feed:
1. `workspace_nav_panel`
2. `formula_space_list_panel`
3. `formula_space_context_panel`
4. `mode_switch_panel`

Current code relation:
1. `OneCalcShellApp` currently has visibility toggles and one active formula,
2. but not a proper multi-space collection or per-space mode memory.

## 6. FormulaSpaceCollectionState
This slice should be a map keyed by formula-space ID.

Each `FormulaSpaceState` should own:
1. authored formula identity,
2. raw entered cell text,
3. editor state,
4. latest OxFml editor document or equivalent structured edit result,
5. current `EditorSyntaxSnapshot`,
6. current green-tree key,
7. current text-change-range summary,
8. current reuse summary,
9. current diagnostics,
10. completion items,
11. current help,
12. latest evaluation summary,
13. effective display summary,
14. array preview summary,
15. scenario policy summary,
16. inspect selection state,
17. retained-run references relevant to the space,
18. compare-entry references relevant to the space.

Interpretation rule:
1. the stored authored text is the raw cell-entry text for the space,
2. not a host-reinterpreted leading-`=` formula string,
3. so direct values and apostrophe-forced strings travel through the same state path,
4. and syntax tokenization, trivia ownership, diagnostics staging, and red-context projection should be retained from OxFml packets rather than re-derived locally.

This slice should feed:
1. `formula_editor_panel`
2. `diagnostics_panel`
3. `result_panel`
4. `effective_display_panel`
5. `array_preview_panel`
6. `completion_help_panel`
7. `scenario_policy_panel`
8. `source_formula_panel`
9. `inspect_result_panel`

Current code relation:
1. `FormulaEditorState`, `FormulaEditorSession`, `FormulaEditPacketSummary`, `FormulaEvaluationSummary`, `CompletionProposalSummary`, and `FunctionHelpSummary` are already real ingredients,
2. but they currently live as one active bundle in `OneCalcShellApp`.

Forward rule:
1. the greenfield host should be willing to store OxFml-native editor packet concepts directly,
2. because OneCalc is intentionally co-evolving with OxFml rather than hiding it behind a generic editor service layer.

## 7. ActiveFormulaSpaceViewState
This slice should be transient and per active space.

It should own:
1. active mode,
2. selected inspect node,
3. formula-walk tree expansion state,
4. selected comparison focus,
5. selected lineage focus,
6. selected evidence focus,
7. visible secondary cluster choices,
8. local scroll or expansion preferences where needed.

This slice should feed:
1. `formula_walk_panel`
2. `parse_summary_panel`
3. `bind_summary_panel`
4. `eval_summary_panel`
5. `provenance_summary_panel`
6. `comparison_outcome_panel`
7. `replay_lineage_panel`
8. `evidence_bundle_panel`
9. `blocked_dimensions_panel`
10. `observation_envelope_panel`

Current code relation:
1. current shell visibility is represented by booleans such as `support_sidebar_visible`, `capability_center_visible`, and `xray_visible`,
2. the future shell should replace these global booleans with per-space and per-mode view state.

## 8. RetainedArtifactOpenState
This slice should own the currently opened retained artifacts and their summaries.

It should include:
1. opened capability snapshot summary,
2. opened replay capture summary,
3. opened X-Ray summary,
4. opened twin compare summary,
5. opened witness summary,
6. opened handoff packet summary,
7. opened workspace summary,
8. scenario selection detail.

This slice should feed:
1. `capability_center_panel`
2. `inspect_detail_panel`
3. `workbench_detail_panel`
4. `comparison_outcome_panel`
5. `replay_lineage_panel`
6. `evidence_bundle_panel`
7. `handoff_panel`

Current code relation:
1. `OpenedCapabilitySnapshotSummary`
2. `OpenedReplayCaptureSummary`
3. `OpenedXRaySummary`
4. `OpenedTwinCompareSummary`
5. `OpenedWitnessSummary`
6. `OpenedHandoffPacketSummary`
7. `OpenedOneCalcWorkspace`
8. `ScenarioSelectionDetail`
are already good summary carriers for this slice.

## 9. CapabilityAndEnvironmentState
This slice should own environment truth that is not specific to one formula text edit.

It should include:
1. host profile ID,
2. platform gate summary,
3. packet-kind register,
4. function-surface policy summary,
5. conditional-formatting policy summary,
6. latest capability snapshot ID,
7. active capability snapshot,
8. capability diff summary,
9. mode availability summary,
10. runtime class and dependency-set truth.

This slice should feed:
1. `environment_truth_panel`
2. `host_truth_panel`
3. `capability_center_panel`
4. platform and gate portions of `comparison_outcome_panel` and `handoff_panel`

Current code relation:
1. current shell stores many of these as text fields:
   1. `host_profile_id`
   2. `packet_register_text`
   3. `platform_gate_text`
   4. `function_policy_text`
   5. `conditional_formatting_policy_text`
2. a fuller shell should prefer structured data first, text rendering second.

## 10. ExtensionSurfaceState
This slice should own extension truth separately from generic capability truth.

It should include:
1. extension-root load summary,
2. extension runtime truth summary,
3. provider-level status list,
4. manifest failures,
5. selected provider detail,
6. extension-action status if the host later exposes activation flows.

This slice should feed:
1. `extension_state_panel`
2. extension-related portions of `environment_truth_panel`

Current code relation:
1. extension truth types already exist in [extension.rs](/C:/Work/DnaCalc/DnaOneCalc/src_archive_ref/dnaonecalc-host/src/extension.rs),
2. but they are not yet shaped into the newer shell model.

## 11. GlobalUiChromeState
This slice should own UI-chrome concerns that do not belong to formula semantics.

It should include:
1. right-drawer open or closed state,
2. active drawer panel ID,
3. pane widths or splits if any remain configurable,
4. focus-return targets,
5. ephemeral error banners,
6. notification or command feedback state.

This slice should feed:
1. `formula_space_context_panel`
2. right-drawer shell behavior
3. `status_footer_panel`

Rule:
1. this slice must not absorb business or semantic truth just because it is globally convenient.

## 12. Panel-To-State Mapping
### 12.1 Shell Panels
| Panel ID | State Slices |
|---|---|
| `workspace_nav_panel` | `WorkspaceShellState` |
| `formula_space_list_panel` | `WorkspaceShellState`, `FormulaSpaceCollectionState` |
| `extension_state_panel` | `ExtensionSurfaceState`, `CapabilityAndEnvironmentState` |
| `environment_truth_panel` | `CapabilityAndEnvironmentState`, `ExtensionSurfaceState` |
| `formula_space_context_panel` | `WorkspaceShellState`, `FormulaSpaceCollectionState`, `GlobalUiChromeState` |
| `mode_switch_panel` | `WorkspaceShellState`, `ActiveFormulaSpaceViewState` |
| `host_truth_panel` | `CapabilityAndEnvironmentState`, `FormulaSpaceCollectionState` |
| `status_footer_panel` | `GlobalUiChromeState`, `CapabilityAndEnvironmentState` |

### 12.2 Explore Panels
| Panel ID | State Slices |
|---|---|
| `formula_editor_panel` | `FormulaSpaceCollectionState` |
| `diagnostics_panel` | `FormulaSpaceCollectionState` |
| `result_panel` | `FormulaSpaceCollectionState` |
| `effective_display_panel` | `FormulaSpaceCollectionState` |
| `array_preview_panel` | `FormulaSpaceCollectionState` |
| `completion_help_panel` | `FormulaSpaceCollectionState` |
| `scenario_policy_panel` | `FormulaSpaceCollectionState` |
| `formatting_panel` | `FormulaSpaceCollectionState`, `GlobalUiChromeState` |
| `conditional_formatting_panel` | `FormulaSpaceCollectionState`, `GlobalUiChromeState` |

### 12.3 Inspect Panels
| Panel ID | State Slices |
|---|---|
| `formula_walk_panel` | `RetainedArtifactOpenState`, `FormulaSpaceCollectionState`, `ActiveFormulaSpaceViewState` |
| `source_formula_panel` | `FormulaSpaceCollectionState` |
| `inspect_result_panel` | `FormulaSpaceCollectionState` |
| `parse_summary_panel` | `RetainedArtifactOpenState`, `FormulaSpaceCollectionState` |
| `bind_summary_panel` | `RetainedArtifactOpenState`, `FormulaSpaceCollectionState` |
| `eval_summary_panel` | `RetainedArtifactOpenState`, `FormulaSpaceCollectionState` |
| `provenance_summary_panel` | `RetainedArtifactOpenState`, `CapabilityAndEnvironmentState` |
| `host_context_panel` | `CapabilityAndEnvironmentState`, `FormulaSpaceCollectionState` |
| `inspect_detail_panel` | `RetainedArtifactOpenState`, `ActiveFormulaSpaceViewState`, `GlobalUiChromeState` |

### 12.4 Workbench Panels
| Panel ID | State Slices |
|---|---|
| `comparison_outcome_panel` | `RetainedArtifactOpenState`, `ActiveFormulaSpaceViewState` |
| `replay_lineage_panel` | `RetainedArtifactOpenState`, `ActiveFormulaSpaceViewState` |
| `evidence_bundle_panel` | `RetainedArtifactOpenState` |
| `blocked_dimensions_panel` | `RetainedArtifactOpenState`, `CapabilityAndEnvironmentState` |
| `observation_envelope_panel` | `RetainedArtifactOpenState` |
| `source_run_panel` | `RetainedArtifactOpenState`, `FormulaSpaceCollectionState` |
| `handoff_panel` | `RetainedArtifactOpenState`, `ActiveFormulaSpaceViewState` |
| `workbench_detail_panel` | `RetainedArtifactOpenState`, `GlobalUiChromeState` |

## 13. Derived State Versus Source State
The host should distinguish:

### 13.1 Source State
1. raw entered cell text
2. editor selection
3. current OxFml editor document or equivalent structured edit result
4. current `EditorSyntaxSnapshot`
5. current green-tree key
6. current reuse summary
7. scenario policy choices
8. retained artifact summaries
9. capability snapshot summaries
10. extension runtime truth

### 13.2 Derived State
1. formatted display text
2. compact host truth lines
3. dirty indicators
4. retained badges
5. mode availability strings
6. blocked-dimension summary chips
7. syntax coloration and token overlay projection if they are rendered from retained syntax snapshots rather than separately persisted

Rule:
1. source state should be structured,
2. derived UI strings should be recomputed from source state,
3. the host should avoid storing long-lived duplicated summary text where structured data already exists.

## 14. Immediate Reshaping Guidance
The current host can evolve toward this model in stages.

### 14.1 Near-Term
1. split `OneCalcShellApp` state into explicit groups even before multiple spaces exist,
2. replace ad hoc strings with structured summary owners where practical,
3. introduce explicit per-mode view state instead of only visibility booleans,
4. keep one active formula space but shape its state as `FormulaSpaceState`.

### 14.2 Next
1. add `WorkspaceShellState` and `FormulaSpaceCollectionState`,
2. preserve mode and drawer state per formula space,
3. route retained artifact summaries through `RetainedArtifactOpenState`,
4. centralize capability and environment truth.

### 14.3 Later
1. add richer function-detail state only when upstream payloads justify it,
2. add richer witness-chain state only when product scope admits it.

## 15. Screen Coverage
This state model is intended to support:
1. `Explore` without remote, stringly support surfaces,
2. `Inspect` without flattening everything into X-Ray toggles,
3. `Workbench` without conflating retained evidence with live authoring state,
4. multi-space persistence without losing per-space context.

## 16. Derived Next Step
This note should now feed:
1. a screen-detail-element map under the existing anchors,
2. a host-state-to-Rust-type proposal,
3. the shared app-core implementation layout in [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md),
4. and later a narrow implementation plan for replacing the archived shell with the new shared app core without prematurely coding the full UI.

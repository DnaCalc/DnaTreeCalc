# DNA OneCalc UX Panel Inventory

Status: `working_panel_inventory`
Date: 2026-04-05
Scope: named panel inventory for the current `DnaOneCalc` shell, modes, and supporting surfaces

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)
5. [APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md](APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md)
6. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)
7. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)
8. [APP_UX_SCREEN_SPEC_EXPLORE.md](APP_UX_SCREEN_SPEC_EXPLORE.md)
9. [APP_UX_SCREEN_SPEC_INSPECT.md](APP_UX_SCREEN_SPEC_INSPECT.md)
10. [APP_UX_SCREEN_SPEC_WORKBENCH.md](APP_UX_SCREEN_SPEC_WORKBENCH.md)
11. [APP_UX_HOST_STATE_SLICING.md](APP_UX_HOST_STATE_SLICING.md)

## 1. Purpose
This note defines the named product panels that make up the current OneCalc UX.

It exists to:
1. give the shell a stable panel vocabulary,
2. separate durable product panels from mockup-only visual groupings,
3. assign each panel an owner, region, and visibility pattern,
4. state which panels are currently in scope and mutable,
5. give later screen specs and implementation work a stable reference set.

It is not:
1. a pixel spec,
2. a component tree,
3. or permission to introduce panels that are not already supported by current scope.

## 2. Panel Naming Rule
Panel names in this note should be treated as the current product vocabulary.

Interpretation rule:
1. a panel may contain multiple controls,
2. a panel is a product-level surface, not a low-level widget,
3. a panel name should be stable enough to survive visual redesign,
4. if a mockup uses a different label for the same product surface, prefer the name in this note.

## 3. Regions
Panels are assigned to one of these shell regions:
1. `left_rail`
2. `top_context_bar`
3. `main_canvas`
4. `right_drawer`
5. `footer`

Rule:
1. a panel may appear in more than one mode,
2. but it should normally have one preferred home and one preferred level of emphasis.

## 4. Shell Panels
| Panel ID | Panel Name | Ownership | Preferred Region | Primary Modes | Mutable | Scope | Purpose |
|---|---|---|---|---|---|---|---|
| `workspace_nav_panel` | Workspace Navigation Panel | Workspace | `left_rail` | All | Yes | planned | Navigate overview, recent, pinned, and workspace sections |
| `formula_space_list_panel` | Formula Space List Panel | Workspace | `left_rail` | All | Yes | planned | Show open and pinned formula spaces and active-space identity |
| `extension_state_panel` | Extension State Panel | Workspace | `left_rail` | All | Limited | planned | Show compact extension truth and extension entry points |
| `environment_truth_panel` | Environment Truth Panel | Workspace | `left_rail` | All | No | planned | Show host profile, capability, and platform gate summary |
| `formula_space_context_panel` | Formula Space Context Panel | Formula-space | `top_context_bar` | All | Yes | planned | Show active formula-space identity, mode, and compact scenario truth |
| `mode_switch_panel` | Mode Switch Panel | Formula-space | `top_context_bar` | All | Yes | planned | Switch between `Explore`, `Inspect`, and `Workbench` |
| `host_truth_panel` | Host Truth Panel | Workspace / Formula-space | `top_context_bar` | All | No | planned | Show compact runtime, timing, and profile truth |
| `status_footer_panel` | Status Footer Panel | Workspace | `footer` | All | No | planned | Show readiness, version, runtime, and compact operational truth |

## 5. Explore Panels
| Panel ID | Panel Name | Ownership | Preferred Region | Visibility | Mutable | Scope | Purpose |
|---|---|---|---|---|---|---|---|
| `formula_editor_panel` | Formula Editor Panel | Formula-space | `main_canvas` | Always visible in `Explore` | Yes | planned | Primary formula authoring surface |
| `diagnostics_panel` | Diagnostics Panel | Formula-space | `main_canvas` | Always visible in `Explore` | No | planned | Show live diagnostics near the editor |
| `result_panel` | Result Panel | Formula-space | `main_canvas` | Always visible in `Explore` | No | planned | Show current evaluated result and shape/type summary |
| `effective_display_panel` | Effective Display Panel | Formula-space | `main_canvas` | Always visible in `Explore` | Limited | planned | Show current display truth and display-oriented summary |
| `array_preview_panel` | Array Preview Panel | Formula-space | `main_canvas` | Secondary in `Explore` when relevant | No | planned | Show intermediate arrays or shape-relevant result detail |
| `completion_help_panel` | Completion and Current Help Panel | Formula-space | `main_canvas` | Always visible in `Explore` | No | planned | Support authoring with completion and help near the editor |
| `scenario_policy_panel` | Scenario Policy Panel | Formula-space | `top_context_bar` | Summary always visible in `Explore` | Yes | planned | Show and edit scenario-level meaning and reproducibility controls |
| `formatting_panel` | Formatting Panel | Formula-space | `right_drawer` | On demand in `Explore` | Yes, if admitted | needs_clarification | Show richer formatting controls beyond the always-visible summary |
| `conditional_formatting_panel` | Conditional Formatting Panel | Formula-space | `right_drawer` | On demand in `Explore` | Yes, if admitted | needs_clarification | Show conditional-formatting controls if product scope admits them |

## 6. Inspect Panels
| Panel ID | Panel Name | Ownership | Preferred Region | Visibility | Mutable | Scope | Purpose |
|---|---|---|---|---|---|---|---|
| `formula_walk_panel` | Formula Walk Panel | Formula-space | `main_canvas` | Always visible in `Inspect` | No | planned | Primary semantic and mechanism-reading surface |
| `source_formula_panel` | Source Formula Panel | Formula-space | `main_canvas` | Always visible in `Inspect` as secondary context | No | planned | Show compact authored formula while in read-only inspect mode |
| `inspect_result_panel` | Inspect Result Panel | Formula-space | `main_canvas` | Always visible in `Inspect` as secondary context | No | planned | Show current result while inspecting semantics |
| `parse_summary_panel` | Parse Summary Panel | Formula-space | `main_canvas` | Always visible in `Inspect` | No | planned | Summarize parse status and formula structure facts |
| `bind_summary_panel` | Bind Summary Panel | Formula-space | `main_canvas` | Always visible in `Inspect` | No | planned | Summarize bindings and semantic references |
| `eval_summary_panel` | Eval Summary Panel | Formula-space / Run | `main_canvas` | Always visible in `Inspect` | No | planned | Summarize evaluation status, timing, and admitted eval facts |
| `provenance_summary_panel` | Provenance Summary Panel | Formula-space / Run | `main_canvas` | Always visible in `Inspect` | No | planned | Summarize provenance and interpretation-facing truth |
| `host_context_panel` | Host Context Panel | Workspace / Formula-space | `main_canvas` | Secondary in `Inspect` | No | planned | Show host state relevant to interpretation |
| `inspect_detail_panel` | Inspect Detail Panel | Formula-space / Run | `right_drawer` | On demand in `Inspect` | Limited | planned | Show deeper node, provenance, function, or correlation detail |

## 7. Workbench Panels
| Panel ID | Panel Name | Ownership | Preferred Region | Visibility | Mutable | Scope | Purpose |
|---|---|---|---|---|---|---|---|
| `comparison_outcome_panel` | Comparison Outcome Panel | Comparison | `main_canvas` | Always visible in `Workbench` | No | planned | Show outcome, reliability framing, and mismatch meaning |
| `replay_lineage_panel` | Replay Lineage Panel | Run / Comparison | `main_canvas` | Always visible in `Workbench` | No | planned | Show run and comparison lineage over time |
| `evidence_bundle_panel` | Evidence Bundle Panel | Comparison | `main_canvas` | Always visible in `Workbench` | Limited | planned | Show evidence object identity and bundle summary |
| `blocked_dimensions_panel` | Blocked Dimensions Panel | Comparison | `main_canvas` | Always visible in `Workbench` when relevant | No | planned | Explain comparison limits and blocked dimensions honestly |
| `observation_envelope_panel` | Observation Envelope Panel | Comparison | `main_canvas` | Secondary in `Workbench` | No | planned | Summarize observed comparison envelope and capture breadth |
| `source_run_panel` | Source Run Panel | Run | `main_canvas` | Secondary in `Workbench` | No | planned | Show source run identity and compact run facts |
| `handoff_panel` | Handoff Panel | Comparison | `main_canvas` or `right_drawer` | Nearby secondary in `Workbench` | Yes | planned | Hold handoff, widening, export, and related evidence actions |
| `workbench_detail_panel` | Workbench Detail Panel | Comparison | `right_drawer` | On demand in `Workbench` | Limited | planned | Show fuller evidence, envelope, or handoff detail |

## 8. Cross-Mode Supporting Panels
| Panel ID | Panel Name | Ownership | Preferred Region | Primary Modes | Mutable | Scope | Purpose |
|---|---|---|---|---|---|---|---|
| `capability_center_panel` | Capability Center Panel | Workspace | `right_drawer` | All | No | planned | Show deeper capability and admission truth on demand |
| `function_detail_panel` | Function Detail Panel | Formula-space / OxFunc-facing | `right_drawer` | Explore, Inspect | No | future_hook | Hold richer future function guidance without creating a separate function browser |
| `witness_chain_panel` | Witness Chain Panel | Comparison | `right_drawer` | Workbench | Limited | future_hook | Show richer witness or distillation detail if later admitted |

## 9. Panel Rules
### 9.1 Planned Panels
The current planned panel baseline is:
1. shell panels in Section 4,
2. `Explore` panels in Section 5 except those marked `needs_clarification`,
3. `Inspect` panels in Section 6,
4. `Workbench` panels in Section 7,
5. `capability_center_panel` in Section 8.

### 9.2 Needs Clarification
The current panels that should not be treated as committed implementation scope yet are:
1. `formatting_panel`
2. `conditional_formatting_panel`

Interpretation:
1. their existence as panel names is useful,
2. but their implementation floor still depends on scope confirmation.

### 9.3 Future Hooks
The current reserved but not committed panels are:
1. `function_detail_panel`
2. `witness_chain_panel`

Rule:
1. these may shape layout reservation and seam thinking,
2. but they should not be treated as immediate product obligations.

## 10. Panel Interaction Rules
### 10.1 Main Canvas Rule
Main-canvas panels should carry the primary work of the current mode.

Therefore:
1. `formula_editor_panel` should dominate `Explore`,
2. `formula_walk_panel` should dominate `Inspect`,
3. `comparison_outcome_panel` and `replay_lineage_panel` should dominate `Workbench`.

### 10.2 Right Drawer Rule
Right-drawer panels are secondary detail only.

Therefore:
1. a drawer may deepen, not replace, the active mode,
2. drawer content should not become primary navigation,
3. drawer content should not silently carry product scope that is absent from the main mode model.

### 10.3 Shell Rule
Shell panels should answer:
1. where the user is,
2. what mode they are in,
3. what compact environment truth applies,
4. and what secondary detail is open.

They should not answer:
1. the full semantic story,
2. the full evidence story,
3. or the full editing story by themselves.

## 11. Derived Next Step
This panel inventory should now feed:
1. a constrained `Explore` screen spec,
2. a constrained `Inspect` screen spec,
3. a constrained `Workbench` screen spec,
4. and later an implementation-facing mapping of panel IDs to host-state slices.

Current derived artifacts:
1. [APP_UX_SCREEN_SPEC_EXPLORE.md](APP_UX_SCREEN_SPEC_EXPLORE.md)
2. [APP_UX_SCREEN_SPEC_INSPECT.md](APP_UX_SCREEN_SPEC_INSPECT.md)
3. [APP_UX_SCREEN_SPEC_WORKBENCH.md](APP_UX_SCREEN_SPEC_WORKBENCH.md)
4. [APP_UX_HOST_STATE_SLICING.md](APP_UX_HOST_STATE_SLICING.md)

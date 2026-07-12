# DNA OneCalc UX Scope Formalization

Status: `working_scope_formalization`
Date: 2026-04-04
Scope: formal UX-scope breakdown for the desktop-first and web-capable `DnaOneCalc` host

Companion notes:
1. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
2. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
3. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
4. [APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md](APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md)
5. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)

## 1. Purpose
This note formalizes the current UX scope for `DnaOneCalc`.

It exists to:
1. keep UX work anchored to the ordered product direction in [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md),
2. stop Figma exploration from silently widening scope,
3. classify surfaced UI areas as current scope, future hook, or out of scope,
4. state which surfaces are visible, editable, or action-bearing in each task mode,
5. give later implementation work a narrower product contract than the mockups.

It is not:
1. a replacement for the product direction in [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md),
2. a visual design guide,
3. a pixel or widget-level UI spec,
4. or permission to add features because they appeared in a mockup.

## 2. Authority And Reading
This note should be interpreted under the following authority chain:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) owns repo mission, host boundary, and ordered product direction,
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md) owns intended application UX direction,
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md) owns shell, modes, ownership, and surface inventory,
4. this note formalizes what that means for current UX scope.

Current product-direction anchor from [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md):
1. `Formula / Function Explorer` first,
2. `Live Formula Semantic X-Ray` second,
3. `Twin Oracle Workbench` third.

Interpretation rule:
1. the first implementation-grade UX commitment is a serious formula host with integrated result, help, diagnostics, and effective display,
2. inspect and workbench surfaces are real product commitments,
3. but their concrete surface breadth must remain subordinate to the ordered product direction above.

## 3. Scope Labels
Every surfaced UX area should be classified using one of these labels:

1. `planned`
   Already implied by current product direction, brief, and architecture.
2. `future_hook`
   Legitimate future-ready placeholder, but not a current implementation commitment.
3. `out_of_scope`
   Not part of the current product commitment and should not drive near-term design or implementation.
4. `needs_clarification`
   Directionally plausible, but not yet explicit enough to treat as either planned or rejected.

Rule:
1. mockups may illustrate `future_hook` areas,
2. but only `planned` areas should drive implementation-facing UX commitments.

## 4. Mockup Interpretation Rule
Figma and Figma Make outputs are exploratory artifacts only.

They may:
1. test layout,
2. test information hierarchy,
3. expose seam pressure,
4. and help compare alternative compositions.

They may not:
1. add product scope,
2. freeze a widget contract,
3. redefine ownership boundaries,
4. or override [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md), [APP_UX_BRIEF.md](APP_UX_BRIEF.md), or [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md).

## 5. Shell-Level Scope
The current shell commitment is:
1. one coordinated application shell,
2. a left rail for workspace sections and formula spaces,
3. a top context bar for active formula-space identity, mode, and compact host truth,
4. a main canvas that changes by task mode,
5. a right drawer for secondary detail,
6. a footer for compact operational truth.

Classification:
1. single shell: `planned`
2. explicit modes `Explore`, `Inspect`, `Workbench`: `planned`
3. persistent left rail plus formula-space tabs: `planned`
4. split panes as a first-class default shell model: `future_hook`
5. secondary windows as a first-class default shell model: `future_hook`
6. route-separated mini-apps for Explorer/X-Ray/Workbench: `out_of_scope`

## 6. Ownership And Mutability Rules
### 6.1 Workspace-Level
Workspace-level surfaces may show:
1. workspace navigation,
2. open, recent, and pinned spaces,
3. host profile summary,
4. capability summary,
5. platform gates,
6. extension summary or management entry.

Workspace-level surfaces may update:
1. open-space order and focus,
2. pinned status,
3. workspace navigation state.

Workspace-level surfaces must not become:
1. the place where scenario meaning is edited,
2. or a dumping ground for every host option the mockups happened to surface.

### 6.2 Formula-Space-Level
Formula-space-level surfaces may show and preserve:
1. formula text,
2. edit state,
3. diagnostics,
4. current help and completion context,
5. result and effective display,
6. scenario policy,
7. inspect state,
8. retained-run and compare state for that formula space.

Formula-space-level surfaces may update:
1. formula text,
2. edit state,
3. scenario policy committed for that scenario,
4. formatting and conditional-formatting settings if and when admitted,
5. retained-run and comparison actions bound to that space.

### 6.3 Run-Level And Comparison-Level
Run and comparison surfaces may show:
1. run metadata,
2. replay lineage,
3. comparison outcome,
4. evidence bundle state,
5. blocked dimensions,
6. handoff readiness.

They may update:
1. retained or archived state,
2. selected comparison focus,
3. handoff/export action state.

They should generally not update:
1. the authored formula itself,
2. primary scenario policy,
3. or workspace-global host policy.

## 7. Mode Scope
### 7.1 Explore
`Explore` is the default and primary mode.

Current `planned` surfaces:
1. formula editor,
2. result,
3. effective display summary,
4. diagnostics,
5. completion list,
6. current function or signature help,
7. array preview when relevant,
8. scenario policy entry and summary.

Current `future_hook` surfaces:
1. richer OxFunc-backed help prose,
2. argument-level semantic guidance,
3. richer function caveats and support annotations,
4. more sophisticated formatting assistance,
5. advanced conditional-formatting authoring.

Current `out_of_scope` surfaces:
1. a full IDE feature set,
2. workbook-grid modeling,
3. arbitrary scripting or macro tooling,
4. design-first panel modularity that obscures the edit-result loop.

Mutability rule:
1. authored formula and scenario policy are updateable here,
2. result and diagnostics are read-only outputs,
3. help and completion are assistive rather than authoritative editing state.

### 7.2 Inspect
`Inspect` is the semantic and mechanism-reading mode.

Current `planned` surfaces:
1. formula walk or equivalent X-Ray primary inspection surface,
2. parse summary,
3. bind summary,
4. eval summary,
5. provenance summary,
6. host context relevant to interpretation,
7. source formula and current result as secondary context.

Current `future_hook` surfaces:
1. tree-addressable partial evaluation at fine node granularity,
2. richer stable node identities and correlations,
3. richer function-specific semantic explanation from OxFunc,
4. deeper provenance chains,
5. packet-level inspection that depends on upstream seam widening.

Current `out_of_scope` surfaces:
1. debugger-style step control over the formula engine,
2. arbitrary timeline replay inside inspect mode,
3. raw coordinator or engine dump panels as a primary product surface.

Mutability rule:
1. inspect is primarily read-only,
2. any scenario policy controls shown here should be summary-first,
3. returning to `Explore` is the normal way to change authored formula content.

### 7.3 Workbench
`Workbench` is the retained evidence and compare/replay/handoff mode.

Current `planned` surfaces:
1. replay lineage,
2. observation and comparison state,
3. comparison outcome,
4. blocked dimensions,
5. evidence bundle summary,
6. handoff readiness,
7. primary evidence actions where the capability floor honestly admits them.

Current `future_hook` surfaces:
1. richer witness and distillation flows,
2. wider observation-envelope authoring or widening,
3. richer explanation narratives,
4. advanced handoff packaging variants,
5. broader multi-run evidence libraries.

Current `out_of_scope` surfaces:
1. a generic case-management system,
2. full workflow automation UI,
3. a separate replay-only sub-application.

Mutability rule:
1. workbench actions may retain, compare, replay, export, or hand off,
2. they should not act like the primary place to author the formula.

## 8. Surface Scope Matrix
| Surface | Ownership | Default Mode | Visibility Rank | User Can Update? | Scope |
|---|---|---:|---|---|---|
| Formula editor | Formula-space | Explore | Primary | Yes | planned |
| Diagnostics | Formula-space | Explore | Secondary | No | planned |
| Completion list | Formula-space | Explore | Secondary | No | planned |
| Current help | Formula-space | Explore | Secondary | No | planned |
| Result card | Formula-space | Explore | Primary | No | planned |
| Effective display summary | Formula-space | Explore | Secondary | Limited | planned |
| Array preview | Formula-space | Explore | Supporting | No | planned |
| Scenario policy summary | Formula-space | Explore/Inspect/Workbench | Secondary | Limited | planned |
| Formatting editor | Formula-space | Explore | Reference | Yes, if admitted | needs_clarification |
| Conditional-formatting editor | Formula-space | Explore | Reference | Yes, if admitted | needs_clarification |
| Formula walk / X-Ray | Formula-space | Inspect | Primary | No | planned |
| Parse summary | Formula-space | Inspect | Secondary | No | planned |
| Bind summary | Formula-space | Inspect | Secondary | No | planned |
| Eval summary | Formula-space | Inspect | Secondary | No | planned |
| Provenance summary | Formula-space | Inspect | Secondary | No | planned |
| Host context summary | Workspace / Formula-space | Inspect | Supporting | No | planned |
| Capability center | Workspace | Any | Supporting | No | planned |
| Extension state | Workspace | Any | Supporting | Limited | planned |
| Replay lineage | Run / Comparison | Workbench | Primary | No | planned |
| Observation envelope summary | Comparison | Workbench | Secondary | No | planned |
| Comparison outcome | Comparison | Workbench | Primary | No | planned |
| Blocked dimensions | Comparison | Workbench | Secondary | No | planned |
| Evidence bundle summary | Comparison | Workbench | Secondary | Limited | planned |
| Handoff panel | Comparison | Workbench | Secondary | Yes, action-bearing | planned |
| Full witness chain authoring | Comparison | Workbench | Reference | Yes | future_hook |
| Arbitrary panel docking system | Shell | Any | Administrative | Yes | out_of_scope |
| Workbook sheet navigator | Shell | Any | Administrative | Yes | out_of_scope |

## 9. Scenario Policy Scope
The scenario policy area is real product scope, but its breadth must stay disciplined.

### 9.1 Current Planned Policy
Current `planned` scenario-policy truth is:
1. deterministic versus real-time policy,
2. deterministic versus real-random policy,
3. scenario-affecting host flags that materially change meaning or reproducibility,
4. visible preservation of that policy in retained evidence.

### 9.2 Needs Clarification
These are plausible but not yet frozen:
1. how far formatting belongs in scenario policy versus a separate display area,
2. whether conditional formatting is a first implementation-floor editing surface or only a prepared hook,
3. which host flags are user-editable versus merely visible,
4. how much of host-driving packet truth is directly user-configurable.

### 9.3 Out Of Scope For Now
The scenario policy area should not grow into:
1. a generic advanced-settings sink,
2. a hidden host-debugger console,
3. a replacement for the capability center,
4. or a speculative coordinator packet editor.

## 10. Function Interaction Scope
Current `planned` function interaction:
1. deterministic completion,
2. current help during editing,
3. signature-help context where admitted,
4. visible function support or admission cues where available.

Current `future_hook` function interaction:
1. richer prose help from OxFunc,
2. argument-level semantic guidance,
3. function caveats and usage guidance,
4. inspect-mode explanation of function participation in evaluation.

Rule:
1. richer function interaction should deepen `Explore` and `Inspect`,
2. it should not create a separate function-browser product or displace the main formula workflow.

## 11. Platform And Capability Scope
Current `planned` honesty surfaces:
1. host profile summary,
2. platform gates,
3. capability summary,
4. explicit visibility for blocked, lossy, provisional, or unsupported states.

Current platform reading:
1. browser and wasm credibility are in scope,
2. Windows-first Excel-observed comparison is in scope where the runtime floor supports it,
3. the browser-hosted product must not overclaim Excel-observed lanes it cannot honestly execute.

Rule:
1. platform-specific compare surfaces may be shown only with explicit gates and reasons,
2. cross-platform replay or inspect surfaces must not imply Windows-only observation capability.

## 12. Explicit Non-Commitments
The following are not implied by the current UX work:
1. full implementation of every surfaced Figma panel,
2. arbitrary dark-mode parity,
3. a modular dashboard shell,
4. a generalized extension marketplace UX,
5. a broad evidence-management back office,
6. frozen packet-debugger surfaces,
7. or any commitment to build directly from generated Figma Make `tsx` or `css`.

## 13. Practical Design Rule
From this point forward, every design refinement should answer:
1. which current `planned` surface is being improved,
2. whether any `future_hook` is only being reserved rather than committed,
3. whether any surface should be demoted or removed because it is out of scope,
4. and what upstream seam pressure the design exposes without normalizing local divergence.

## 14. Follow-Up
The next UX documents should derive from this note:
1. a mode-by-mode visibility and mutability map,
2. a constrained mockup synthesis note,
3. and later an implementation-facing screen spec that uses only `planned` scope plus explicitly marked `future_hook` placeholders.

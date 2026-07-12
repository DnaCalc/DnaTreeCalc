# DNA OneCalc UX Mode Visibility Map

Status: `working_visibility_map`
Date: 2026-04-05
Scope: mode-by-mode visibility and updateability map for the current `DnaOneCalc` UX scope

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)
5. [APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md](APP_UX_CONSTRAINED_MOCKUP_SYNTHESIS.md)
6. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)

## 1. Purpose
This note turns the current UX scope and mockup synthesis into a mode-by-mode visibility and updateability map.

It exists to answer:
1. what is always visible in each mode,
2. what is nearby secondary,
3. what belongs in the right drawer,
4. what is only on demand,
5. what the user may update in that mode,
6. and what remains a future hook rather than a current implementation commitment.

It is not:
1. a pixel layout spec,
2. a widget contract,
3. or permission to widen scope beyond the current formalization.

## 2. Reading Rule
Interpret this note under the following authority:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) defines ordered product direction,
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md) defines intended UX direction,
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md) defines shell, ownership, and surface inventory,
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md) defines what is currently in scope,
5. this note maps that scope into visible and mutable mode behavior.

## 3. Shared Shell Visibility
These shell surfaces are shared across all three modes.

### 3.1 Always Visible
1. left rail:
   1. workspace navigation,
   2. open formula spaces,
   3. pinned spaces,
   4. compact environment and extension truth
2. top context bar:
   1. active formula-space identity,
   2. active mode,
   3. compact scenario policy summary,
   4. compact host truth
3. footer:
   1. readiness state,
   2. version and runtime truth,
   3. mode identity

### 3.2 Updateability
The user may update:
1. active formula space,
2. pinned and focus state,
3. active mode,
4. drawer open or closed state.

The user should not update here:
1. deep scenario meaning through workspace-global controls,
2. comparison evidence state directly from the workspace shell,
3. arbitrary host internals.

## 4. Explore Mode
`Explore` is the default authoring and discovery mode.

### 4.1 Always Visible
1. formula editor
2. diagnostics tied to the editor
3. result card
4. effective display summary
5. current help
6. completion list or completion-aware help companion
7. scenario policy summary

### 4.2 Nearby Secondary
1. array preview when relevant
2. lightweight host and capability cues
3. formatting entry point
4. conditional-formatting entry point if admitted
5. quick mode-relevant actions such as:
   1. move to inspect,
   2. retain run,
   3. open richer help

### 4.3 Right Drawer
The right drawer may hold:
1. expanded formatting controls if admitted,
2. expanded current help,
3. expanded completion browser,
4. scenario policy detail,
5. compact capability details.

The right drawer should not become:
1. the primary place where formula authoring happens,
2. a remote dump of diagnostics,
3. a catch-all advanced settings console.

### 4.4 On-Demand Only
1. deeper function metadata
2. richer formatting detail
3. conditional-formatting authoring if admitted
4. capability center detail
5. extension-state detail

### 4.5 User Can Update
1. formula text
2. cursor, selection, and edit state
3. scenario policy committed for that formula space
4. formatting settings if admitted
5. conditional-formatting settings if admitted

### 4.6 User Cannot Update Here
1. result values directly
2. diagnostics directly
3. capability truth
4. platform gates
5. replay or comparison history except through explicit actions

### 4.7 Future Hooks
Reserved but not committed:
1. richer OxFunc prose help
2. argument-level semantic guidance
3. stronger caveat and support-status presentation
4. more advanced formatting assistance

## 5. Inspect Mode
`Inspect` is the semantic and mechanism-reading mode.

### 5.1 Always Visible
1. formula walk or equivalent X-Ray primary inspection surface
2. source formula summary
3. current result summary
4. parse summary
5. bind summary
6. eval summary
7. provenance summary
8. host context relevant to interpretation

### 5.2 Nearby Secondary
1. scenario policy summary
2. function support or semantic cues where available
3. packet-kind and run-context summary where interpretively useful
4. blocked and opaque state cues

### 5.3 Right Drawer
The right drawer may hold:
1. node detail,
2. deeper provenance detail,
3. deeper host context detail,
4. expanded function guidance,
5. related diagnostic or correlation detail.

The right drawer should not become:
1. the primary location of inspect mode,
2. a raw engine dump,
3. or a debugger console.

### 5.4 On-Demand Only
1. expanded provenance chain
2. deeper packet or host context
3. function-level semantic detail
4. retained evidence detail
5. compare and handoff detail

### 5.5 User Can Update
Inspect is primarily read-only.

The user may update:
1. tree expansion and collapse state,
2. selected inspect focus,
3. open drawer detail,
4. navigation back to `Explore` or onward to `Workbench`.

### 5.6 User Cannot Update Here
1. primary formula text
2. semantic result directly
3. comparison evidence
4. workspace-global host truth

Scenario policy shown here should be:
1. summary-first,
2. low-emphasis,
3. and normally updated by returning to `Explore`.

### 5.7 Future Hooks
Reserved but not committed:
1. fine-grained partial evaluation per subexpression node
2. richer stable node identity and cross-surface correlation
3. richer function-specific semantic explanation
4. deeper provenance chains

## 6. Workbench Mode
`Workbench` is the retained evidence, compare, replay, and handoff mode.

### 6.1 Always Visible
1. comparison outcome
2. replay lineage
3. evidence bundle summary
4. reliability summary
5. blocked dimensions
6. source formula summary
7. compact scenario policy summary

### 6.2 Nearby Secondary
1. observation envelope summary
2. source run summary
3. next recommended action
4. platform and capability gating relevant to the current comparison lane

### 6.3 Right Drawer
The right drawer may hold:
1. full observation envelope detail,
2. evidence bundle detail,
3. handoff packet detail,
4. witness or lineage detail if later admitted.

The right drawer should not become:
1. the main workbench logic surface,
2. an admin console,
3. or a substitute for clear primary evidence panels.

### 6.4 On-Demand Only
1. full witness chain detail
2. deeper handoff packaging detail
3. richer explanation narratives
4. broader evidence-library detail
5. full editor or full inspect detail

### 6.5 User Can Update
1. retained state
2. selected comparison focus
3. action state for replay, compare, export, widen, or handoff when admitted
4. drawer detail state

### 6.6 User Cannot Update Here
1. formula text as the primary authoring flow
2. scenario policy as the primary editing flow
3. workspace-global host truth
4. unsupported compare lanes hidden behind platform gates

### 6.7 Future Hooks
Reserved but not committed:
1. richer witness and distillation flows
2. wider observation-envelope authoring
3. stronger explanation narratives
4. broader multi-run evidence library tooling

## 7. Visibility Summary Matrix
| Surface | Explore | Inspect | Workbench |
|---|---|---|---|
| Formula editor | Always visible | On demand or compact source summary | On demand only |
| Diagnostics | Always visible | On demand or related detail | On demand only |
| Completion and current help | Always visible | Secondary or on demand | On demand only |
| Result card / result summary | Always visible | Always visible as secondary | Secondary |
| Effective display summary | Always visible | Secondary if interpretively relevant | Secondary if evidence-relevant |
| Array preview | Secondary when relevant | On demand or secondary | Usually hidden |
| Scenario policy summary | Always visible | Always visible as summary | Always visible as summary |
| Formatting detail | Drawer or on demand | Hidden | Hidden |
| Conditional-formatting detail | Drawer or on demand if admitted | Hidden | Hidden |
| Formula walk | Hidden by default | Always visible | On demand only |
| Parse/bind/eval/provenance summaries | Hidden by default | Always visible | On demand only |
| Host context summary | Secondary | Always visible | Secondary |
| Capability summary | Secondary | Secondary | Secondary |
| Replay lineage | Action-driven or hidden | Reference only | Always visible |
| Comparison outcome | Hidden unless entered | Reference only | Always visible |
| Observation envelope | Hidden unless entered | Reference only | Secondary / drawer |
| Evidence bundle | Hidden unless entered | Reference only | Always visible |
| Handoff panel | Hidden unless entered | Reference only | Nearby secondary or drawer |
| Extension state | Supporting shell truth | Supporting shell truth | Supporting shell truth |

## 8. Updateability Summary Matrix
| Domain | Explore | Inspect | Workbench |
|---|---|---|---|
| Formula authoring | Yes | No | No |
| Scenario policy | Yes | Usually no, summary-first | Usually no, summary preserved |
| Formatting | If admitted | No | No |
| Conditional formatting | If admitted and in scope | No | No |
| Inspect focus | Limited | Yes | No |
| Replay and compare actions | Limited entry points | No primary actions | Yes |
| Handoff actions | No primary actions | No primary actions | Yes |
| Workspace navigation | Yes | Yes | Yes |
| Drawer detail state | Yes | Yes | Yes |

## 9. Platform And Gating Rule
This visibility map assumes:
1. browser-capable `Explore` and `Inspect` are genuine product expectations,
2. `Workbench` must degrade honestly when Excel-observed comparison is not admitted,
3. platform-specific compare lanes are shown only with explicit gates, reasons, or absence.

Interpretation:
1. the browser-hosted UX must not visually promise Windows-only observation behavior,
2. the native host may admit richer compare and replay lanes,
3. but the shell and mode model should remain coherent across hosts.

## 10. Derived Next Step
This note should now feed:
1. a panel inventory,
2. a constrained screen spec for `Explore`,
3. a constrained screen spec for `Inspect`,
4. a constrained screen spec for `Workbench`,
5. and a separate seam-pressure note where the UI asks more of `OxFml` or `OxFunc`.

Current derived artifact:
1. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)

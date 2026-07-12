# DNA OneCalc Constrained Mockup Synthesis

Status: `working_synthesis_note`
Date: 2026-04-04
Scope: constrained synthesis of current Figma and Figma Make exploration under existing product and UX scope

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)

Design evidence reviewed:
1. Figma design file `DNA OneCalc`
2. copied `Explore`, `Inspect`, and `Workbench` frames
3. extracted Figma Make bundles under [ux_artifacts/figma_make/2026-04-04](/C:/Work/DnaCalc/DnaOneCalc/docs/ux_artifacts/figma_make/2026-04-04)

## 1. Purpose
This note captures what should be kept from the current mockup exploration without allowing that exploration to widen product scope.

It exists to:
1. preserve useful layout and hierarchy decisions,
2. reject mockup drift that is not supported by the current product direction,
3. mark future-ready hooks separately from current commitments,
4. give later screen-spec work a constrained baseline.

Rule:
1. this note synthesizes mockups under current scope,
2. it does not grant new scope by synthesis.

## 2. Synthesis Method
Each mockup-derived idea is interpreted through this filter:
1. if already implied by [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md), [APP_UX_BRIEF.md](APP_UX_BRIEF.md), and [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md), keep it as `adopt`,
2. if directionally useful but not yet committed, keep it only as `reserve`,
3. if it introduces accidental scope or misleading emphasis, mark it `reject`,
4. if the idea exposes upstream seam pressure, note that separately rather than normalizing it locally.

The labels in this note are:
1. `adopt`
2. `adapt`
3. `reserve`
4. `reject`

Interpretation:
1. `adopt` means the mockup aligns well with current scope and should shape the next spec pass,
2. `adapt` means the mockup direction is useful but should be narrowed or repositioned,
3. `reserve` means keep room for it without committing implementation scope,
4. `reject` means do not carry it forward as a baseline.

## 3. Global Synthesis
### 3.1 Adopt
1. one coordinated shell across all three task modes,
2. left rail for workspace sections and formula spaces,
3. top context bar for active formula-space identity, mode, and compact host truth,
4. right drawer for secondary detail,
5. light-mode base with a warmer, more distinctive palette than the earliest conservative mockups,
6. persistent sense that `Explore`, `Inspect`, and `Workbench` are perspectives over one active scenario.

### 3.2 Adapt
1. rail plus tabs should remain, but the rail must carry the primary space-navigation weight,
2. top-level mode switching should not become a second competing space-navigation system,
3. support surfaces should remain visible enough to guide the user but must not dominate the primary mode content,
4. platform and capability truth should stay present but compact.

### 3.3 Reserve
1. richer palette ambition from the darker modular concept,
2. stronger footer and status-bar truth,
3. richer future-ready function interaction zones,
4. more detailed panel states for evidence-heavy workflows.

### 3.4 Reject
1. dashboard-style modular cockpit as the primary product shell,
2. equal emphasis for all panels at once,
3. route-separated or app-like fragmentation between `Explore`, `Inspect`, and `Workbench`,
4. worksheet-grid or workbook-navigation metaphors,
5. generated React component structure as an implementation baseline.

## 4. Explore Mode Synthesis
### 4.1 Adopt
1. formula editor as the dominant surface,
2. result visible in the same view as the editor,
3. diagnostics immediately below or adjacent to the editor,
4. current help and completion close to the editing surface,
5. effective display visible without leaving the formula flow,
6. array preview present only when relevant,
7. scenario policy visible as compact scenario truth in the context bar.

### 4.2 Adapt
1. the completion and help area should feel like an editing companion rather than a reference shelf,
2. formatting entry points may exist, but they must remain secondary and not imply a richer formatting product than currently admitted,
3. quick actions should stay sparse and mode-relevant,
4. host truth should stay compact and should not compete with the edit-result-help loop.

### 4.3 Reserve
1. richer OxFunc-backed function explanations,
2. argument-level semantic usage guidance,
3. stronger inline semantic assistance during formula authoring,
4. richer formatting assistance if upstream and product scope later admit it.

### 4.4 Reject
1. full IDE framing,
2. persistent high-detail inspector as part of the default `Explore` composition,
3. large settings surfaces that crowd the editor,
4. turning `Explore` into a generic control panel for every host toggle.

## 5. Inspect Mode Synthesis
### 5.1 Adopt
1. formula walk as the primary inspect surface,
2. source formula and current result as secondary context,
3. parse, bind, eval, and provenance summaries grouped near the formula walk,
4. visible state categories such as evaluated, bound, opaque, and blocked,
5. host context relevant to interpretation.

### 5.2 Adapt
1. the inspect view should be tree- and semantics-aligned rather than step-log-oriented,
2. any editable scenario controls shown here should be summary-first and low-emphasis,
3. packet and host context should be present only to the degree they help interpretation,
4. the right drawer may deepen detail but should not be the main location where inspect mode happens.

### 5.3 Reserve
1. fine-grained partial evaluation per subexpression node,
2. richer function-specific semantic help inside inspect mode,
3. deeper provenance chains,
4. stronger node-to-diagnostic and node-to-result correlation.

### 5.4 Reject
1. raw engine dump views as the main inspect experience,
2. debugger-like stepping controls,
3. exposing internal packet families as first-class user-facing concepts unless the product later proves they belong there.

## 6. Workbench Mode Synthesis
### 6.1 Adopt
1. comparison outcome as the main workbench surface,
2. replay lineage as a visually strong primary support surface,
3. evidence bundle and reliability as distinct supporting objects,
4. blocked dimensions and honesty about comparison limits,
5. action surfaces for retain, export, replay, compare, or handoff when capability floors admit them,
6. clear linkage back to the same active formula space.

### 6.2 Adapt
1. the workbench should remain evidence-centered, not administration-centered,
2. Excel-observed comparison must be clearly gated by runtime and platform truth,
3. browser-capable workbench states must not imply Windows-only observation lanes,
4. source formula summary should remain compact and secondary.

### 6.3 Reserve
1. richer witness-chain or distillation views,
2. broader evidence-library tooling,
3. stronger explanation and next-step narratives,
4. more advanced handoff packaging.

### 6.4 Reject
1. broad back-office case management,
2. a separate replay sub-application,
3. making workbench the default first product face,
4. showing evidence-management structure that exceeds current artifact commitments.

## 7. Scenario Policy Synthesis
### 7.1 Adopt
1. scenario policy as a first-class concept rather than hidden miscellaneous settings,
2. deterministic versus real-time and real-random truth visible on the formula space,
3. policy preserved into retained evidence and visible in workbench summaries.

### 7.2 Adapt
1. keep scenario policy compact in `Explore`,
2. show it as summary truth in `Inspect`,
3. preserve it as evidence-bearing context in `Workbench`,
4. do not let it absorb all host and capability controls.

### 7.3 Reserve
1. richer policy editing once host policy and upstream seams are clearer,
2. more nuanced display-affecting options if they are later admitted,
3. conditional-formatting authoring if and only if product scope confirms it belongs in the first implementation floor.

### 7.4 Reject
1. a generic advanced-settings sink,
2. speculative packet editing,
3. moving workspace-global environment controls into per-scenario policy.

## 8. Function Interaction Synthesis
### 8.1 Adopt
1. completion and current help integrated into the main authoring flow,
2. visible function support or admission cues where available,
3. space reserved in `Explore` and `Inspect` for richer function-specific interaction later.

### 8.2 Adapt
1. help should stay close to the active edit context,
2. support cues should inform rather than clutter,
3. function metadata should not force a separate browser-first workflow.

### 8.3 Reserve
1. richer OxFunc prose help,
2. argument-level semantic guidance,
3. caveat and constraint messaging,
4. inspect-mode explanation of a function’s role in the current evaluation.

### 8.4 Reject
1. a separate function browser as a primary app mode,
2. encyclopedic documentation panels that overpower the editor.

## 9. Visual Synthesis
### 9.1 Adopt
1. warm light-mode base,
2. stronger contrast and accent identity than the earliest conservative palette,
3. editorial calm rather than generic SaaS chrome,
4. high editor readability.

### 9.2 Adapt
1. the richer modular/dark palette should influence light-mode accents rather than pull the product into a dark default,
2. visual richness should come from hierarchy, color discipline, and spacing rather than from more panels.

### 9.3 Reserve
1. a later dark mode,
2. deeper palette refinement once the information architecture is stable.

### 9.4 Reject
1. dark-mode assumptions as a current scope obligation,
2. palette complexity that weakens readability in the editor.

## 10. Upstream Seam Pressure Exposed By Mockups
The mockups expose real seam pressure, but those pressures are not themselves local scope expansion.

### 10.1 OxFml Pressure
1. tree-addressable formula structure,
2. stable node identities,
3. partial evaluation projection where admitted,
4. explicit blocked and opaque reasons,
5. stronger bind and provenance detail.

### 10.2 OxFunc Pressure
1. richer help payloads,
2. stronger signature-help payloads,
3. argument semantics and usage guidance,
4. function support and caveat metadata.

### 10.3 OneCalc Host Pressure
1. stable scenario-policy model,
2. mode and drawer state per formula space,
3. retained-run and comparison summaries per space,
4. explicit native-versus-browser gating behavior.

## 11. Derived Next Step
The next implementation-facing UX work should use this synthesis to produce:
1. a mode-by-mode visibility map,
2. a mode-by-mode updateability map,
3. a panel inventory for the chosen shell,
4. a constrained screen spec using only `planned` scope plus explicitly labeled `reserve` hooks.

Current derived artifact:
1. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)

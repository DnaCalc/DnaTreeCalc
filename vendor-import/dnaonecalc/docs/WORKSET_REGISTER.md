# DNA OneCalc Workset Register

Status: `active_register`
Date: 2026-04-02

## 1. Purpose
This is the living ordered workset register for `DnaOneCalc`.

It translates [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) into one coherent
implementation sequence for the whole scoped product.

This file is not an execution-status board.
It defines the high-level work themes, their dependency order, and their intended
rollout shape.

## 2. Planning-Surface Clarification
Planning and execution truth in this repo is split as follows:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) owns scope, design, artifact, and product truth.
2. this register owns the ordered set of worksets and their dependency shape.
3. `.beads/` owns epics, beads, readiness, blockers, in-progress state, and closure.

Working rule:
1. worksets do not carry `active`, `ready`, `blocked`, or `complete` status fields,
2. epics and leaf beads are the units that become ready, in progress, blocked, and
   closed,
3. this register and `.beads/` are the planning surfaces for implementation work,
4. after planning is in place, default execution outputs should be code, tests, and
   narrowly-scoped spec or seam corrections rather than local documentation rollout.

## 3. Use Rule
Use this document as:
1. the repo-local workset authority,
2. the source for `workset -> epic -> bead` rollout,
3. the full-scope implementation map for the product,
4. the sequencing guide for broad engineering themes.

Do not use this document as:
1. semantic authority over the OneCalc scope,
2. a substitute for the live bead graph,
3. a second blocker or readiness tracker,
4. a reason to create one document per workset or per bead.

## 4. Register Contract
Each workset in this register carries:
1. stable workset id,
2. title,
3. purpose,
4. depends_on,
5. parent spec sections,
6. primary upstream repo dependencies,
7. closure condition,
8. initial epic lanes.

## 5. Sequencing Rule
The sequence below is the default expansion order for the repo.

It does mean:
1. earlier worksets establish the product spine and implementation substrate that
   later work depends on,
2. early rollout should bias toward the shortest honest path to real dependency-backed
   formula entry and evaluation,
3. later supporting surfaces should not crowd out the first working slice unless they
   are true prerequisites,
4. once an upstream seam packet is frozen and landed for active OneCalc
   integration, the corresponding migration workset should execute before later
   breadth that would deepen the superseded seam shape.

## 6. Workset Sequence

### WS-01 Host Runtime Integration And Product Shell
1. purpose:
   establish the desktop-first OneCalc shell, the runtime boundary between host and
   upstream libraries, the host-profile and packet-kind substrate, and real
   code-level integration of the primary runtime dependencies.
2. depends_on: none
3. parent_spec_sections:
   `3`, `4`, `7.0`, `9.1` through `9.4`, `13.2`, `14.1`, `15`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, and the `OxCalc` seam-reference slice
5. closure_condition:
   a desktop-first host shell builds against real upstream dependencies, exposes the
   current host profile and packet kind honestly, and provides a runnable shell path
   that can support formula entry and evaluation work without locally redefining
   upstream semantics.
6. initial_epic_lanes:
   dependency pin and package integration, host-profile and packet-kind substrate,
   desktop shell bootstrap, platform-honesty and secondary-host gating

### WS-02 Formula Editing And Language-Service Integration
1. purpose:
   implement real formula entry and editor behavior in the shell and integrate the
   OxFml edit and diagnostics path as the authoritative language-service substrate.
2. depends_on:
   `WS-01`
3. parent_spec_sections:
   `4.2`, `5`, `9.3`, `9.4`, `16`
4. upstream_dependencies:
   `OxFml`, with `OxFunc` metadata pressure where relevant
5. closure_condition:
   a user can type and edit formulas in the real shell, OxFml edit packets are wired
   into host state, and trustworthy diagnostics are visible with runnable proof of
   the editing flow.
6. initial_epic_lanes:
   editor shell and interaction model, OxFml edit-packet integration, diagnostics
   projection, keyboard-first verification

### WS-03 Function Surface, Metadata Projection, And First Evaluation Slice
1. purpose:
   wire the admitted OxFunc surface into OneCalc, project the current stable and
   usable metadata honestly, and deliver the first real dependency-backed
   formula-evaluation slice that behaves like a usable formula/function explorer.
2. depends_on:
   `WS-01`, `WS-02`
3. parent_spec_sections:
   `3`, `4.3`, `7.1`, `7.4`, `9.6`, `14.2`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`
5. closure_condition:
   OneCalc derives the admitted function surface from OxFunc in code, a user can
   enter an admitted formula, inspect deterministic completion and current help,
   and see a real evaluated result plus honest diagnostics and surface labels, with
   runnable verification.
6. initial_epic_lanes:
   library-context and function-surface integration, evaluate action and result
   surface, OxFunc metadata and help projection, explorer baseline verification

### WS-04 OxFml_V1 Consumer And Downstream Contract Alignment
1. purpose:
   move ordinary host runtime and editor behavior onto the landed `OxFml_V1`
   downstream-consumer surfaces so later work deepens the right seam rather than the
   historical direct-consumer path.
2. depends_on:
   `WS-03`
3. parent_spec_sections:
   `4`, `5`, `6`, `7.1`, `7.2`, `9.1.2` through `9.1.5`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`
5. closure_condition:
   OneCalc consumes the landed `oxfml_core::consumer` runtime and editor surfaces
   for its ordinary host path, historical direct-consumer substrate use is removed
   from ordinary host code, and the migrated slice has runnable verification proving
   semantic behavior stayed intact.
6. initial_epic_lanes:
   runtime facade migration, editor facade migration, replay-aware seam alignment,
   contract verification and cleanup

### WS-05 Formula/Function Explorer UX And Result Surface Stabilization
1. purpose:
   prioritize interactive use of OneCalc as a formula/function explorer by
   stabilizing workbench information architecture, result visibility,
   effective-display presentation, and keyboard-first interaction.
2. depends_on:
   `WS-03`, `WS-04`
3. parent_spec_sections:
   `3`, `5.1`, `7.1`, `8`, `9.1`, `9.6`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`
5. closure_condition:
   the primary workbench is usable and stable for authoring, exploring functions,
   and reading results; result and context surfaces do not disappear under resize;
   and the promoted explorer path has runnable UX regression coverage.
6. initial_epic_lanes:
   workbench information architecture, result and effective-display presentation,
   function-help and explorer affordances, layout and resize behavior, UX
   regression verification

### WS-06 Engine X-Ray And Mechanism Inspection
1. purpose:
   make `Live Formula Semantic X-Ray` the second major product perspective by
   surfacing parse, bind, evaluation, provenance, and host-driving truth over the
   same active scenario.
2. depends_on:
   `WS-04`, `WS-05`
3. parent_spec_sections:
   `3`, `5`, `9.6`, `10.0`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`
5. closure_condition:
   durable X-Ray surfaces show current edit and eval mechanism truth, provenance,
   and capability or context facts without displacing primary authoring use, and the
   promoted X-Ray families have runnable verification.
6. initial_epic_lanes:
   parse, bind, and eval artifact projection, trace and provenance surfaces,
   mechanism drawer or panel UX, X-Ray verification families

### WS-07 Test Scaffolding, Scenario Corpora, And Acceptance Harness
1. purpose:
   build deterministic scaffolding and high-coverage integration harnesses that make
   explorer, X-Ray, replay, and observation work faster to develop and harder to
   regress.
2. depends_on:
   `WS-03`, `WS-04`
3. parent_spec_sections:
   `9.1.5`, `9.1.6`, `10.3`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`
5. closure_condition:
   OneCalc has reusable fixture hosts, scenario corpora, golden artifact assertions,
   UI interaction and layout harnesses, and host acceptance automation that expose
   regressions quickly across the promoted surfaces.
6. initial_epic_lanes:
   deterministic fixture hosts and stubs, retained scenario corpus and promoted
   families, golden artifact and persistence assertions, UI and layout harnesses,
   host acceptance matrix automation

### WS-08 Driven Single-Formula Host, Retained Runs, Persistence, Capability Snapshots, And ScenarioCapsule
1. purpose:
   realize the H1 driven single-formula host model and the durable artifact spine
   needed before serious replay work.
2. depends_on:
   `WS-05`, `WS-06`, `WS-07`
3. parent_spec_sections:
   `5.3`, `6.0` through `6.10`, `7.2`, `10.3`, `10.4`, `11`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`
5. closure_condition:
   the driven host model, retained scenarios and runs, document round-trip,
   capability snapshots, and `ScenarioCapsule` transport work as one coherent
   durable spine without drifting toward worksheet semantics.
6. initial_epic_lanes:
   driven host model and recalc triggers, retained scenario and scenario-run
   handling, document persistence and invariants, capability snapshot and capsule
   flows

### WS-09 Replay Substrate Proving Ground, Semantic Logging, And Handoff Foundations
1. purpose:
   treat replay as the highest-risk current proving lane by determining how retained
   operations, semantic logging, reproducible runs, witness production, and handoff
   output should flow through `OxReplay` and the adjacent `Ox*` seams.
2. depends_on:
   `WS-06`, `WS-07`, `WS-08`
3. parent_spec_sections:
   `10.0` through `10.4`, `14.3`, `16`, `17`
4. upstream_dependencies:
   `OxReplay`, `OxFml`, `OxFunc`, `OxXlPlay`
5. closure_condition:
   OneCalc can emit and reopen honest replay-bearing retained artifacts for the
   current lane floor, the replay operation model is explicit, and unresolved seam
   gaps are captured as structured upstream pressure rather than local invention.
6. initial_epic_lanes:
   replay operation and event model, replay capture and retained lineage,
   diff/explain/witness/handoff baseline, cross-repo replay seam pressure, replay
   proving tests

### WS-10 Windows Observation And Twin-Oracle Comparison
1. purpose:
   make replay, observation, and compare the third major product perspective through
   Windows `OxXlPlay` capture and honest retained comparison.
2. depends_on:
   `WS-08`, `WS-09`
3. parent_spec_sections:
   `5.1`, `10.1` through `10.3`, `14.3`, `16`, `17.1`
4. upstream_dependencies:
   `OxXlPlay`, `OxReplay`
5. closure_condition:
   the Windows desktop host can capture Excel observations, persist observation and
   comparison artifacts with provenance and lossiness surfaced explicitly, and drive
   a twin compare workflow with reliability badges and widening-pressure output.
6. initial_epic_lanes:
   Windows observation capture integration, observation and comparison persistence,
   twin compare workbench surfaces, widening-request workflow, compare regression
   families

### WS-11 Workspace Management, Capability Center, Scenario Library, And Acceptance
1. purpose:
   complete supporting workspace and honesty surfaces once the explorer, X-Ray, and
   replay lanes exist, without letting them crowd out the main workbench.
2. depends_on:
   `WS-08`, `WS-09`, `WS-10`
3. parent_spec_sections:
   `5.2`, `5.3`, `9.1.1`, `9.1.5`, `9.1.6`, `9.5`, `9.6`, `10.3`, `16`, `17`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`, later `OxVba`
5. closure_condition:
   multi-file workspace management exists without introducing cross-instance
   semantics, the capability center and diff flows surface actual executable truth
   as a supporting surface, promoted scenarios and acceptance evidence are managed
   in-product, and corpus hardening becomes an ordinary product operation.
6. initial_epic_lanes:
   workspace management, capability center UX, scenario library and promotion, host
   acceptance matrix and regression evidence, corpus hardening and release evidence

### WS-12 Shared Extension Host Core And Adapter Readiness
1. purpose:
   establish and consume the shared DNA Calc provider, capability, registration,
   invalidation, ticking, and teardown contracts after the explorer, X-Ray, and
   replay floors are stable; prepare OxVba, Windows XLL, and Windows RTD COM
   adapters without implementing those adapters in this workset.
2. depends_on:
   `WS-08`, `WS-11`
3. parent_spec_sections:
   `7.3`, `12`, `14.4`, `16`, `17.2`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, `OxReplay`, later `OxVba`
5. closure_condition:
   DnaOneCalc consumes `dnacalc-extension-host-core`, local speculative extension
   contracts are removed, runtime capabilities and invalidation/ticking behavior
   are explicit and tested, and implementation-ready adapter proposals exist for
   current OxVba, Windows XLL, and Windows RTD COM without claiming those adapters
   are already implemented.
6. initial_epic_lanes:
   shared-core handoff and adoption, provider/catalog lifecycle, host invalidation
   and ticking, capability-state and diagnostics, OxVba/XLL/RTD adapter proposals,
   removal of OneCalc-local speculative ABI and deferred VBA structures

### WS-13 DNA OneCalc UX Revamp Across Editor, Case Management, Host Config, And Value / Parity Surfaces
1. purpose:
   land a coordinated UX improvement wave across the formula editor, case /
   formula-space lifecycle, host/caller configuration (full Excel `Format Cells`
   and `Conditional Formatting` surface), and the cross-mode Value Panel and
   Workbench Parity Matrix surfaces, so the full intended UX is visible to users
   with every engine-dependent control either live or explicitly marked
   `<NOT IMPLEMENTED>` with a `SEAM-*` id that names the required engine work.
2. depends_on:
   `WS-02`, `WS-05`, `WS-08`, `WS-11`
3. parent_spec_sections:
   `3`, `4.2`, `5`, `5.3`, `6.0` through `6.10`, `7.1`, `7.2`, `8`, `9.1`,
   `9.3`, `9.4`, `9.6`, `10`, `11`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc` (including the `oxfunc_value_types` crate and the
   `oxfunc_core` locale/format-code engine), `OxXlPlay`, `OxReplay` trace events
5. closure_condition:
   the Explore / Inspect / Workbench shells expose every surface named in
   [worksets/WS-13_dna_onecalc_ux_revamp.md](worksets/WS-13_dna_onecalc_ux_revamp.md);
   every engine-dependent control renders with a visible `<NOT IMPLEMENTED>`
   badge carrying its `SEAM-*` id until the corresponding engine work lands;
   workspace JSON v1 round-trips every affected field; and the seam status
   board on the workspace settings page enumerates the live pending-seams set.
6. initial_epic_lanes:
   formula editor control, case / formula-space lifecycle, configure drawer
   chrome plus the six Excel `Format Cells` parity tabs and the CF rules
   manager, scenario policy / host bindings / calc options tabs, workspace
   settings page and seam status board, Value Panel component and cross-mode
   integration, Workbench Parity Matrix and trace consumption, Excel parity
   test harness

### WS-15 VBA Integration
1. purpose:
   integrate OxVba as a direct runtime-hosting surface for DnaOneCalc workspaces
   and formula spaces: associate OxVba projects, inject the DnaOneCalc
   `Application` root object, discover admitted public module functions as UDF
   candidates, register those functions with OxFml, and invoke them from
   DnaOneCalc formulas through the registered-external provider path.
2. depends_on:
   `WS-08`, `WS-11`, `WS-12`
3. parent_spec_sections:
   `4`, `5.3`, `6`, `7.3`, `9.1`, `12`, `14.4`, `16`, `17.2`
4. upstream_dependencies:
   `OxVba`, `OxFml`, `OxFunc`, `OxReplay`
5. closure_condition:
   a desktop DnaOneCalc workspace can associate one or more OxVba projects,
   expose the first DnaOneCalc host root object (`Application.Version`), admit
   Excel-observed typed scalar public module functions as registered UDFs through
   OxFml, call them from formulas, compare the same UDF cases against retained
   Excel evidence through OxXlPlay/OxReplay, persist and replay the association
   state honestly, and surface unresolved OxVba/OxFml/OxFunc/OxXlPlay/OxReplay
   seam pressure explicitly.
6. initial_epic_lanes:
   runtime host module and association persistence, DnaOneCalc host root object
   bridge, OxVba project load/session lifecycle, UDF discovery and admission
   policy, typed UDF invocation and value conversion, OxFml registered-external
   registration bridge, OxXlPlay/OxReplay Excel-oracle harness, capability/
   evidence/diagnostics integration, upstream pressure handoffs

### WS-16 Shared Host Architecture And Runtime Profiles
1. purpose:
   align DnaOneCalc with the shared DNA Calc host architecture while preserving
   its direct OxFml/OxFunc and null-reference boundary: consume the canonical
   Skin IR and shared formula UX, separate the platform-neutral host core from
   Leptos/Tauri/CLI shells, adopt shared runtime profiles and session envelopes,
   and prepare shared extension-host attachment without adding OxCalc or OxDoc.
2. depends_on:
   `WS-04`, `WS-12`, `WS-13`, and DnaTreeCalc `W011` shared-crate delivery
3. parent_spec_sections:
   `4`, `5`, `8`, `9`, `11`, `12`, `14`, `16`
4. upstream_dependencies:
   `OxFml`, `OxFunc`, and the DnaTreeCalc-owned `dnacalc-*` shared crates
5. closure_condition:
   OneCalc has a Leptos-free host core, shared formula projection and rendering,
   browser/Tauri/CLI adapters over one Skin IR session contract, honest runtime
   capability profiles, working save/open/recent flows, and an attachment point
   for the shared extension core; dependency guards continue to exclude
   `oxcalc*`, `oxdoc*`, and `oxvba*` in this tranche.
6. initial_epic_lanes:
   TreeCalc shared-foundation handoff, canonical Skin IR adoption, shared formula
   UX consumption, host-core and shell split, runtime-profile/session alignment,
   persistence intent completion, shared extension-core adoption readiness

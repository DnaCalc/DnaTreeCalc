# HANDOVER_OXCALC_iterative_cycle_config

Status: Responded
Target: OxCalc
Ask: Confirm the production host-facing contract by which a TreeCalc workspace (a) selects a cycle profile and (b) supplies iterative bounds (Maximum Iterations, Maximum Change), plus (c) the circular-reference diagnostic surface returned to the host.
Context: TreeCalc exposes Excel-style circular-reference handling at the workspace level (CORE_MODEL_SPEC §7a). OxCalc W048 owns the cycle profiles and the iteration itself; current W048 evidence covers the declared single-host-scoped TreeCalc/TraceCalc surface. We still need the exact production host-facing contract so TreeCalc can pass config and surface results faithfully.
Evidence: OxCalc `docs/spec/core-engine/w048-cycles/` — `W048_ITERATIVE_PROFILE_DECISION_AND_EXCEL_DISPOSITION.md` (§3.4 stop metric/bound, §3.5 publication policy, §6 deterministic profile defaults), `W048_TREECALC_OPTIMIZED_CYCLE_BEHAVIOR.md` (§4 behavior, `publish_excel_match_iterative_cycle`); `CORE_ENGINE_RECALC_AND_INCREMENTAL_MODEL.md` §11.3; DnaTreeCalc `docs/model/CORE_MODEL_SPEC.md` §7a, §6 item 12.

## What TreeCalc needs

1. **Profile selection.** How does the host select among `cycle.non_iterative_stage1` (default),
   `cycle.excel_match_iterative`, and `cycle.iterative_deterministic_v0`? The W048 TreeCalc fixtures
   opt in via the "compatibility basis"; confirm the production field on
   `OxCalcTreeHostCapabilitySnapshot` (or the recalc submission) that carries the cycle profile id,
   and whether it sits alongside `capability_profile_id`.
2. **Bounds.** Where do Maximum Iterations and Maximum Change ride — same snapshot, per-recalc, or
   per-profile defaults (100 / `0.001`)? Confirm the stop metric (max absolute visible numeric delta)
   and that the host may override the defaults.
3. **Diagnostics back.** The surface for reporting cycles to the host: cycle-region membership, the
   `cycle_iteration_trace`, terminal-state classification (converged / max-iteration / oscillation /
   divergent), and a `CircularReference`-equivalent for the non-iterative error case (Excel exposes
   `Worksheet.CircularReference`).
4. **Excel-match coverage status.** TreeCalc will label `cycle.excel_match_iterative` as
   "Excel-faithful (covered surfaces)". Confirm the current single-host-scoped covered-fixture scope,
   including the cleared root/report-cell and non-numeric/blank/error prior-state lanes, and name any
   remaining scope dimensions such as cross-version and multithread behavior so our UX labels partial
   coverage honestly rather than overclaiming.

## Expected disposition

Part **confirm** (profiles, defaults, current single-host-scoped evidence, and the TreeCalc publish path
already exist), part **coordinate** (production host-facing field names + the diagnostic surface).

## TreeCalc W002 integration note (2026-05-21)

TreeCalc now targets direct `OxCalcTreeContext` use for local Rust execution:

- local crate: `DnaTreeCalc/src/dnatreecalc-host`;
- consumed OxCalc surface: `OxCalcTreeContext`, workspace/node edit APIs,
  `OxCalcTreeCalculationOutcome`, and context views;
- local smoke fixture: `Root.A = 2`, `Root.B = A + 3`, with published value,
  dependency edge, diagnostics, and node-state projection observed.

The current OxCalc context gives TreeCalc a good acyclic smoke path, but cycle
configuration is still not a production host contract from TreeCalc's point of
view:

1. `OxCalcTreeHostCapabilitySnapshot` currently carries `capability_profile_id` and runtime-effect
   booleans, but TreeCalc does not see a typed cycle-profile field or iterative bounds field there.
2. TreeCalc's local `CycleConfig` (`profile_id`, `maximum_iterations`, `maximum_change`) is currently
   host-side only and is **not submitted to OxCalc** by the W002 smoke path. It exists to preserve the
   workspace/config boundary while waiting for this response.
3. `docs/test-corpus/cycles/cycle-profiles.json` remains pending until TreeCalc can submit cycle config
   and assert typed diagnostics/results against the real context path.

Minimum unblocker for TreeCalc W002/W005:

- a typed place to submit cycle profile id and iterative bounds, or an explicit instruction that cycle
  cases stay out of the first W002 active corpus slice;
- the typed diagnostic/result fields TreeCalc should project for non-iterative cycle blocking and
  iterative terminal states;
- the current coverage label TreeCalc should use for `cycle.excel_match_iterative` in UI/spec text.

Sibling-review checklist:

- confirm whether cycle config belongs in `OxCalcTreeHostCapabilitySnapshot`,
  `OxCalcTreeRuntimePolicy`, `OxCalcTreeContext`, or a new structured context option;
- name the exact fields and defaults for Maximum Iterations and Maximum Change;
- name the host-facing cycle diagnostic/result fields (`CircularReference` equivalent,
  cycle-region membership, iteration trace, terminal classification);
- state whether W002 should keep cycle corpus cases pending until a later OxCalc workset, or can activate
  a non-iterative subset against the current context surface.

## Resolution (from OxCalc consumer contract §6.3 / §6.4, W055)

OxCalc answered this in `CORE_ENGINE_OXCALCTREE_CONSUMER_INTERFACE_AND_HOST_CONTRACT_V1.md`:

- **Profile + bounds channel:** a typed `cycle_config` field on the OxCalc context recalc surface (§6.3, W055).
  `cycle_config.cycle_profile_id` admits `cycle.non_iterative_stage1` (the default when `cycle_config`
  is absent), `cycle.excel_match_iterative`, and `cycle.iterative_deterministic_v0`;
  `cycle_config.maximum_iterations` / `maximum_change` carry host overrides, profile defaults otherwise.
  No compatibility-basis string is allowed as the cycle channel.
- **Diagnostics back:** a typed `cycle_diagnostics` field on `OxCalcTreeCalculationOutcome` (§6.4) — cycle
  region, selected profile, region source, members, root/report node, member order, terminal state,
  publication decision, reject kind, and iteration-trace summary; plus a typed
  `Worksheet.CircularReference` equivalent for non-iterative rejection. Hosts read these typed facts,
  not string diagnostics.

Spec updated accordingly: `CORE_MODEL_SPEC.md` §7a + §6 item 12 and `ux/TECHNICAL.md` §8.3 now reference
the typed `cycle_config` / `cycle_diagnostics` fields rather than the compatibility basis.

Residual (scoped, not blocking): the **coverage label** for `cycle.excel_match_iterative` — W048
evidence is single-host-scoped, and cross-version / multithread scope remain dimensions; §7a's
Excel-alignment-boundary note already hedges this. The W002 corpus (`cycles/cycle-profiles.json`) can
activate the non-iterative subset against the typed field; iterative-value assertions stay
Excel/engine-anchored.

# HANDOVER_OXCALC_iterative_cycle_config

Status: Open
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

# Bench — the scenario bench / wind-tunnel

Consequence-free what-if over engine candidates: scenarios, typed overrides,
side-by-side comparison, sensitivity sweeps. `Ctrl+6`,
`dnatreecalc-skins/src/bench.rs`.

Built in **Phase A** (not the originally-gated Phase C): the W4b/W4c substrate
— candidates, host-managed scenarios, sweeps, `comparison`, `series` — is fully
landed and projected.

**Scenario rail:** Base (published) + one row per `scenarios.entries`;
create (`CreateScenario`, minted `scenario:` slug ids, branching from the
active scenario), activate (`ActivateScenario`, `None` = base), delete.

**Overrides:** typed by stable `NodeKey` — the input builds
`Number`/`Logical`/`Empty`/`Text` payloads (input handling only; the **host**
literalizes through OxFml) → `SetScenarioOverride`/`ClearScenarioOverride`.
The active scenario's full override list renders with jump + per-row clear.

**Comparison grid:** basis + scenario/sweep columns from
`WorkspaceState.comparison`; rows are the interesting population (keys valued
in any non-basis column, or overridden, plus the selection). Unevaluated cells
render a dash — never fabricated. Scenario/sweep cells carry the structural
scenario tint; candidate-sourced columns the speculative tint (engine-typed
sources only).

**Sweeps:** points parsed from a comma list over the selected node →
`CreateScenarioSweep` (each point an engine-evaluated candidate-backed
scenario); the series strip renders `WorkspaceState.series` with proportional
bars and unit badges.

**Honesty:** rename-scenario / edit-sweep-points intents don't exist yet
(follow-up #8) — Bench edits are delete-and-recreate.

**Tests:** id minting (slug + collision), override value arms, sweep-point
parsing (junk token errors), comparison row selection.

# Bench — the scenario bench / wind-tunnel

Consequence-free what-if over engine candidates: scenarios, typed overrides,
side-by-side comparison, sensitivity sweeps. `Ctrl+6`,
`dnatreecalc-skins/src/bench.rs`.

Built in **Phase A** (not the originally-gated Phase C): the W4b/W4c substrate
— candidates, host-managed scenarios, sweeps, `comparison`, `series` — is fully
landed and projected.

## Intent

> **Audit yardstick.** What Bench is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Bench is a **wind tunnel**: the
model is suspended in labeled scenarios (Base + Bull/Bear/custom), and you
inhabit one at a time to test typed overrides and sweep an input across a range,
watching dependents ripple. It is consequence-free — nothing touches the
published model until you keep it; unevaluated cells are honest dashes, and
hypothetical values carry a structural tint. The question it answers: *"What if X
= 5 and Y = 10 — without committing to it?"*

**What you can do here**
- Activate a scenario (Base or a named what-if) and author a **typed override** (Number/Logical/Text/Empty) on the selected node, shown against its published value.
- **Create** a scenario branching from the active one; **delete** a scenario; **clear** a single override (with a jump-to-node list).
- Read a **side-by-side comparison grid** — basis + scenario / sweep / candidate columns — over the interesting population.
- Define a **sensitivity sweep** (comma-separated points) over the selected input; activate a point; read the series strip's proportional bars.
- Keep selection continuity — jumping from an override row selects that node in every lens.

**What it deliberately leaves to other lenses**
- Never fabricates / interpolates — unevaluated is a dash; reads published projections only.
- Type-classifies override input at the UI boundary but never parses formula text — the host literalizes.
- Rename-scenario / edit-sweep-points don't exist yet — edits are delete-and-recreate (follow-up #8).
- Reads, never owns, the comparison-column set; no table expansion or general authoring; speculation is ephemeral until promoted (via Transport).

**Audit checklist — does the build realize the intent?**
1. The scenario rail is Base + exactly one row per projected scenario; minted ids are collision-checked.
2. Override input classifies to a typed value before dispatch; rejections come from the host, not UI guessing.
3. Comparison rows are exactly the interesting population (valued in a non-basis column, or overridden, plus the selection), minus effective-meta.
4. Provenance tints are engine-sourced: scenario tint for scenario columns, speculative tint only for candidate-sourced columns, dash for unevaluated — never hand-rolled.
5. Sweep create validates numeric points and rejects bad tokens by name; series bars are proportional to real values only.
6. Activating a scenario / sweep never mutates the published base; selection and active scenario survive a lens switch.

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

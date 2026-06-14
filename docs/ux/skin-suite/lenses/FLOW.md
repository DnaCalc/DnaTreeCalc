# Flow — the reference lens

Flow renders the model as a dependency-flow "sentence": inputs on the left,
results on the right, in structure-stable layered columns with dependency wires
between them. It is the first ATLAS lens and the reference every later lens
conforms to, so it is the canonical realization of [the spine](../SPINE.md).

> Status: **built** (ATLAS Phase A) — `dnatreecalc-skins/src/flow.rs`,
> registered as `Ctrl+5` (the canonical suite order) and the web entrypoint's
> default lens. Prototype: [`../../prototypes/09_flow.html`](../../prototypes/09_flow.html).

## Intent

> **Audit yardstick.** What Flow is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Flow puts you *inside the
computation* — causality flows left→right as topological columns, dependency
wires are the syntax, and calc-state pips are the signal. `F9` sweeps a reading
head through the engine's *real* evaluation order, rolling only the values that
changed. As the reference lens, it is the canonical reading of the spine. The
question it answers: *"Why did this value become this, and what depends on it
changing?"*

**What you can do here**
- `F9` **reading-head sweep** across chips in true `evaluation_order`, igniting wires and rolling changed values — causal theater, never a re-sorted layout order.
- `]` / `[` **trace-as-layout**: grow the dependent / precedent trace from the selection; off-trace chips dim.
- `E` **explain**: unfold a recursive derivation stack (precedent values, call sites) for the focus node.
- Edit a chip's content modelessly (`Enter` to edit/commit, `Esc` to cancel); jump by `/` Name-Box.
- Navigate by arrows (↑↓ siblings, ←→ toward inputs/results); read the inspector (value, format, diagnostics, precedent/dependent counts) and the console health tallies.

**What it deliberately leaves to other lenses**
- Never parses formula text or fabricates a value — every number and provenance mark is engine-published.
- No grid/A1 coordinates — references are node-addressed and survive rename/move.
- **Structure-stable** layout: value changes never reshuffle columns; the sweep axis (evaluation order) is distinct from the layout axis (precedent rank).
- No spatial-coordinate persistence (Canvas); single-select today.

**Audit checklist — does the build realize the intent?**
1. `F9` advances the head over `last_run.evaluation_order` (true causal order), not a values- or layout-sorted order.
2. `]` / `[` highlight the dependent / precedent subgraph from the focus and dim the rest; the lit set grows on repeats.
3. Layout recomputes only on topology change (`dependency_shape_snapshot_id`), never on value change.
4. `E` reads `derivation_traces` filtered to the focus owner and renders recursively.
5. `/` jumps via `SelectNode` with **no recalc**, and excludes effectively-meta nodes.
6. Modeless edit: `Enter` edits, every key is verbatim, `Esc` cancels, `Enter` commits-and-advances; bare-key verbs never fire while typing.
7. Selection, `focus_key`, and trace context live in shared continuity and survive a lens switch.

## What it reads (published projection only)

- `nodes_by_key` / `key_order` — chips (meta nodes excluded from the flow stage).
- `dependencies.edges_by_owner_key` — layered rank + the SVG wires (a node's
  precedents are its edge targets).
- `dependencies.reverse_edges_by_key` — dependents, for the forward trace.
- `last_run.evaluation_order` — the reading-head causal sweep.
- `last_run.derivation_traces` — the recursive `E` explain stack (filtered to the
  selection's `owner_key`).
- `active_node_detail(selection)` — the inspector: value, effective format,
  precedent/dependent counts, binding diagnostics.

## What it writes (closed intents only)

`SelectNode`, `EditContent` / `EditContentDeferred` (respecting the shared
recalc mode), `Recalculate`, and the shell-owned `Undo`/`Redo`. It never parses
formula text, fabricates a value, or uses grid coordinates.

## Layout rules

- **Structure-stable.** Column = longest precedent-chain rank (`compute_layout`),
  so the layout depends on topology, not values — value changes don't reshuffle
  it. Cycles are broken with a visiting guard.
- **1:1 wires.** Chips sit on a fixed cell grid (`COL_W`/`ROW_H`), so wire
  endpoints are computed analytically and the SVG overlay maps 1:1 to chip pixels
  (the proto-09 fixed-viewBox lesson). Wires on the active trace are emphasized.
- **Trace-as-layout.** `]` / `[` grow the dependent / precedent trace depth from
  the selection; on-trace chips and wires are highlighted, off-trace chips dimmed.

## The grammar in Flow

`Enter` enters/commits the modeless 1-bit edit (the inspector's content buffer;
`Esc` cancels). `E` toggles the explain panel. `/` opens the Name-Box jump. `F9`
recalculates. Selection/`focus_key` are written as shared continuity so the next
lens lands on the same node. (`Fold`/`Unfold`/`Fill` are reserved verbs, not yet
meaningful in Flow.)

## Embedded Lens + Console (Phase-A mono-lens)

The **Lens** inspector and **Console** strip are the shared
`spine_widgets::{NodeInspector, ConsoleBar, NameBoxBar}` components — identical
in every mono-lens; Flow contributes its explain stack as the inspector's
lens-specific children. In the Phase-B cockpit they become real companion slots
over the same shared selection/continuity — no rework to Flow's reads/writes.

## Reading head

A scrubber over the engine's real `evaluation_order`: `F9 sweep` resets to the
start, `◀`/`▶` step the head, and chips not yet reached are dimmed — reading the
model in causal order. **Follow-up:** auto-play animation (timed advance) and
syncing the sweep to an in-flight recalc.

## Known scope / follow-ups

- Auto-play reading-head animation.
- Multi-select on the stage (the continuity field exists; Flow uses single
  selection today).
- `Fold`/`Unfold`/`Fill` behaviors in Flow.
- Per-chip fine-grained reactivity / virtualization for very large models (the
  stage currently re-renders on projection change; layout is memo-cacheable on
  `dependency_shape_snapshot_id`).

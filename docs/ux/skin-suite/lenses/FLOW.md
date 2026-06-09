# Flow — the reference lens

Flow renders the model as a dependency-flow "sentence": inputs on the left,
results on the right, in structure-stable layered columns with dependency wires
between them. It is the first ATLAS lens and the reference every later lens
conforms to, so it is the canonical realization of [the spine](../SPINE.md).

> Status: **built** (ATLAS Phase A, slice 1) — `dnatreecalc-skins/src/flow.rs`,
> registered as `Ctrl+6`. Prototype: [`../../prototypes/09_flow.html`](../../prototypes/09_flow.html).

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

For the mono-lens core, the **Lens** inspector (selection detail + recursive
explain) and the **Console** strip (calc-state health tallies + Name-Box) live
*inside* Flow. In the Phase-B cockpit they become real companion slots over the
same shared selection/continuity — no rework to Flow's reads/writes.

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

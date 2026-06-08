# FLOW — Design

The concrete design, in two tiers: **what ships on today's Skin IR** (the entire first-touch wow) and
**what the [stack requirements](../stack-requirements/) deepen**. The expansive future is in
[`FLOW_VISION.md`](FLOW_VISION.md); the openable mockup is
[`../prototypes/09_flow.html`](../prototypes/09_flow.html).

---

## Thesis & mental model

FLOW lays a **focused slice** of the model out as a left-to-right "sentence" of node-**chips** in
stable dependency (topological) order, so causality becomes one legible reading motion you can arrow
through. It fuses the three deepest Excel reflexes — **F9** recalc, **Ctrl+[ / Ctrl+]** trace
precedents/dependents, the **Name Box** jump — except the trace *is* the permanent layout, the recalc
is visible theatre played in the engine's true evaluation order, and the Name Box is a fuzzy command
bar.

It is **modeless**, like Excel itself. The only mode is the 1-bit distinction every spreadsheet has: a
chip is either **SELECTED** (bare keys are commands) or **EDITING** (every key, including `/` and `?`,
is verbatim formula text). No Vim normal/insert/visual; no "press i". The status bar always shows which
state you're in (mirroring Excel's *Ready* / *Edit*), and the focus chip's border colour switches, so
you always know whether `/` will *jump* or *divide*.

## The first 90 seconds

1. **0s** — FLOW mounts. A ribbon of rounded chips in stable topo order:
   `Accounts.2026.Q1.Revenue → Q1.COGS → Q1.Margin → FY.Margin`. Each shows `display_name`, value, and
   a `calc_state` pip; faint wires arc back from each chip to the chips that feed it. The selected chip,
   `Q1.Margin`, is enlarged with a prose band reading its `content_text` verbatim: `= Revenue − COGS`.
   It already reads like a sentence — and because the axis is topo, not the last run's dirty cone, it
   does not vanish or reshuffle on mount.
2. **4s** — The user presses **↓** → selection moves to the next sibling (`Q1.COGS`). This is
   `SelectNode` — routed purely to the selection signal, zero recalc, glass-smooth. Left/Right step the
   topo axis; Up/Down stay within siblings (Excel's column-walk).
3. **9s** — `Q1.Margin` is tinted amber and pulsing — `calc_state = DirtyPending` from an edit last
   session. Two downstream chips share the amber; the wires between go dotted. The **pending blast
   radius is visible before any action** (`reverse_edges` closure of the dirty set, pure read).
4. **14s** — Muscle memory fires: **F9**. FLOW dispatches `Recalculate`. The reading-head (a vertical
   light-bar) snaps to the leftmost chip in `last_run.evaluation_order` and sweeps right in that exact
   order; at each chip whose value *actually* changed, the pip flips to VerifiedClean, the number does
   an odometer-roll, and the outgoing wire ignites. Clean chips are skipped — no animation lies.
5. **22s** — The user presses **]** on `COGS`: every dependent lifts and brightens (`reverse_edges`),
   the rest dim. **[** does precedents (`edges_by_owner`). Repeated presses walk one transitive hop
   further; the lit subgraph stays on screen; Esc clears. They *walk* the graph with the same keys
   Excel *traces* with — but the structure persists.
6. **30s** — They press **/** and type `q1marg` → fuzzy-jumps to `Accounts.2026.Q1.Margin` (the Name
   Box, fuzzy over dotted paths; `is_meta` excluded).
7. **38s** — **Space d** lights every stale node; the bar reads "3 stale". Whole-model health in one
   chord — on keys that can never collide with a formula's `?`.
8. **48s** — Back on `Margin` they just **start typing** `= Revenue / COGS`. The `/` is eaten as
   division because an edit buffer is open. Enter commits (`EditContent`) and drops to the next
   **sibling**; the chip greys to DirtyPending and the downstream wave goes amber. F9 replays the
   recompute. Under 60s: navigated, audited health, walked the graph, edited a division-bearing formula
   safely, watched it recompute — no mouse, no mode beyond selected-vs-editing.

## Surfaces

- **The Sentence (primary lane).** A focused horizontal ribbon of chips in a **stable host-side topo
  sort** (of `node_order` + `edges_by_owner` — always populated, deterministic across runs, never empty
  at mount), with backward dependency wires. Focused, not the whole model: the selected node + its
  eval-order neighbourhood + its precedent/dependent chain, hard-capped to a fixed chip budget
  (≤40 visible) *before* layout, virtualized over `node_order`. The selected chip is enlarged (~1.6×)
  and held near centre; collapsed subtrees (`tree_collapsed`) render as a single summary chip and
  aren't wire-routed.
- **Focus chip + explain stack.** The enlarged centred chip: `display_name`, big value, `calc_state`
  pip, editable prose `content_text` band. **E** unfolds its precedents downward as a recursive
  mini-chip stack with live values — the self-explaining derivation.
- **Command + health bar.** The always-available omnibox: `/` fuzzy path jump (pre-edit only). Health
  and blast-radius queries live on the **Space-leader** palette / **Ctrl+/** (never bare `?`), so they
  can never be confused with formula text. Typing here only moves a host-side caret — no intent until
  you act.
- **Scenario rail + delta ledger.** Number-key scenario chips, each labelled by its live headline
  output, plus a baseline-tick + magnitude delta ledger summarising "what moved and by how much" after
  the last recalc or what-if.
- **Which-key footer + state indicator.** A self-documenting modeline showing the selection path,
  `calc_state`, value, and the active buffer state (**SELECTED** vs **EDITING**) — and, on Space, the
  leader palette. Defuses the keyboard-discovery cliff.

## Full keyboard map

| Keys | Mode | Action | IR |
|---|---|---|---|
| ↑ / ↓ | selected | prev/next **sibling** (Excel up/down a row) | `SelectNode` |
| ← / → | selected | prev/next chip along the topo axis | `SelectNode` |
| Ctrl+↑ / Ctrl+↓ | selected | first/last sibling at this depth (structural boundary) | `SelectNode` |
| *any printable* / F2 | selected→editing | open inline prose editor, enter EDIT; all keys verbatim | `EditContent`/`EditFormula` on commit |
| Enter | editing | commit + advance to next **sibling** | `EditContent` + `SelectNode` |
| Ctrl+Enter / Tab | editing | commit + topological-next hop (opt-in) | `EditContent` + `SelectNode` |
| Esc | both | editing: cancel · selected: clear highlight / discard ghost / close omnibox | host-side |
| F9 / Ctrl+Enter | selected | Recalculate + reading-head sweep | `Recalculate` + reads |
| ] / [ | selected | walk dependents / precedents one hop; accumulate lit subgraph | `reverse_edges`/`edges_by_owner` + `SelectNode` |
| E | selected | explain: unfold precedents in-place (recursive); aggregates enumerate members | `edges_by_owner` + `descriptors.collection.members` |
| Backspace | selected | collapse the deepest explain level | SkinState |
| / | selected→omnibox | fuzzy path jump (only when no edit buffer open) | `SelectNode` on commit |
| Space d \| c \| e (or Ctrl+/) | leader | health lens: dirty \| cycle \| err | `calc_state`, `cycle_groups`, `Error` |
| Space r ‹name› \| Space p ‹path› | leader | blast-radius-by-name (works on undefined names) | `content_text` + `reverse_edges` |
| h / l | selected | collapse / expand subtree (persisted cross-skin) | `tree_collapsed` |
| Shift+← / Shift+→ | selected | extend selection to an adjacent run | **NEEDS `SelectNodes`** |
| c / d / s | selected | grow selection to children / descendants / like-named siblings | **NEEDS `SelectNodes`** |
| Ctrl+Enter / Ctrl+D | selected (multi) | fill formula across selection as one name-relative rule | **NEEDS `ReplicateContent`** |
| o / O | selected | add child after / sibling before | `AddNode` |
| Delete | selected | delete — shows `reverse_edges` blast-radius preview first | `DeleteNode` |
| Alt+↑ / Alt+↓ | selected | reorder among siblings (rule re-threads) | `ReorderNode` |
| drag (or m, target) | selected | reparent subtree; name-relative refs re-thread | `MoveNode` |
| 1–9 | selected | flip to scenario N | `EditContentDeferred` batch + `Recalculate` (→ `scenario-substrate`) |
| Space w-i | leader | ghost what-if entry | **NEEDS `PreviewEdit`** |
| Space (leader) | leader | which-key palette (m manual-batch, r recalc-mode, w workspace, p pin, d/c/e health) | various |
| Ctrl+1..5 | global | switch skin (host global keymap, untouched) | host |

## Signature magic moments

Each is tagged **[today]** (ships on the current IR + 12 intents) or **[+stack]** (deepened by a
named requirement from [`../stack-requirements/`](../stack-requirements/)).

- **The model thinks out loud (F9 reading-head sweep).** `[today]` Dispatch `Recalculate`; animate a
  reading-head over the chips in `last_run.evaluation_order`, odometer-rolling only values that
  actually changed (strict diff, gated on `value_epoch`), igniting wires as the head passes. Excel's F9
  is a black box; FLOW shows the topological order the engine actually used. `[+stack]` With
  `value-shape-diff` + `projection-delta-channel`, the sweep animates over real incremental deltas (not
  a full-republish replay), and array nodes **unfurl their changed rows in causal order** via
  `overlay-resize-deltas`.
- **Walk the dependency graph as a motion.** `[today]` `]`/`[` bring forward dependents/precedents and
  accumulate a lit subgraph that stays on screen; the trace *is* the layout. `[+stack]` With
  `reference-resolution-map`, the walk becomes precise jump-to-definition / find-references at the
  token level, and `typed-dependency-kinds` colour-codes edges by kind.
- **Blast-radius before you break it.** `[today]` Before `DeleteNode`, light every chip that will break
  (`reverse_edges` closure) — see the impact before the `#REF!`. In manual-batch, each
  `EditContentDeferred` grows the amber `DirtyPending` downstream set. `[+stack]` With
  `legality-impact-preview` + `recalc-plan-preview`, the count is exact and pre-commit ("this rebinds 7
  dependents, risks a cycle"), delegated to engine dry-bind / plan-invalidation.
- **Path-as-Name-Box + model-health search.** `[today]` `/` fuzzy-jumps any node by dotted path; the
  Space-leader health lenses light dirty / cycle / error / by-name blast-radius over engine truth — on
  keys that can never hijack formula text. `[+stack]` `model-query-projection` makes this scale to
  100k-node models (search + filter + jump), and `command-palette-metadata` gives it titles, shortcuts,
  and enablement.
- **Recursive explain: the number derives itself.** `[today]` **E** unfolds the focus chip's direct
  precedents in place as mini-chips with live values; a SUM-over-`.*` enumerates the collection's
  actual members with a `membership_version` stamp. `[+stack]` With `full-derivation-trace`, the unfold
  becomes a true kernel-level call stack (template selection, hole bindings, per-call inputs/outputs) —
  Evaluate-Formula on steroids, N levels deep.
- **Fill one rule across a named set that survives reorder.** `[+stack: SelectNodes + ReplicateContent]`
  Multi-select a run of sibling chips, type one formula, Ctrl+Enter → one `ReplicateContent`; OxFml
  rebinds `Revenue`/`COGS` by lexical walk-up inside *each* target's own quarter. Drag Q4 above Q2
  (`MoveNode`) and the rule re-threads — name-relative, not A1-relative, so reorder is free.
- **Ghost what-if: peek without committing.** `[+stack: PreviewEdit]` Enter a hypothetical in a
  clearly-marked ghost editor; the focus chip and its downstream closure show overlay "ghost" values
  (provenance-tagged, never mistaken for real) without publishing. Esc discards (zero undo debt, zero
  history pollution); Enter promotes to a real `EditContent`.

## Skin IR usage

**Reads (today):** `node_order` + `edges_by_owner` (the stable topo layout axis); `last_run.
evaluation_order` (animation axis only); per-node `calc_state`; `computed_value`; `reverse_edges` +
`cycle_groups`; `descriptors.collection.{members, membership_version}`; `phase_timings_micros`;
`content_text`; `nodes` (display_name/parent/children/depth/is_meta); `revision.value_epoch` +
`dependency_shape_snapshot_id` (memo/render gating); `profile` (gate treecalc-v1 affordances).

**Writes (today, existing intents):** `SelectNode`, `Recalculate`, `EditContent`/`EditFormula`/
`EditContentDeferred`, `AddNode`, `DeleteNode`, `RenameNode`, `MoveNode`, `ReorderNode`,
`SwitchWorkspace`/`NewWorkspace`.

**The three reviewed IR additions** (each *v1-ships-without*; full detail in the stack requirements):
1. **`SelectNodes` / multi-select** (`SelectionState.anchor + selected`) — backs Shift+arrow run-select
   and c/d/s set-growth. Calc-free, mirrors `SelectNode`. *Highest leverage.*
2. **`ReplicateContent{source, targets}`** — fill-by-name; ids only, OxFml rebinds; one republish. *The
   payoff that turns "watch the model think" into "bend it".*
3. **`PreviewEdit{node, content}` → non-published `CalcRunProjection` + overlay values** — the ghost
   what-if; off the publish path, no revision pollution.

## `FlowSkinState`

```rust
struct FlowSkinState {
    editing: Option<EditBuffer>,            // 1-bit Excel-native edit mode; { node, draft }
    unfolded: HashSet<NodeKey>,             // explain-stack expansion (memo on (NodeKey, value_epoch))
    baseline: HashMap<NodeKey, (String, String)>, // content + value at interaction start (delta + Esc-to-baseline)
    scenarios: Vec<ScenarioChip>,           // { label, overrides: HashMap<NodeKey, String> }
    active_scenario: Option<usize>,
    templates: Vec<FillRule>,               // promote-to-template; { name, source, targets }
    lit_subgraph: Vec<NodeKey>,             // ]/[ accumulation; cleared on Esc
    layout_cache: Option<(String /*dependency_shape_snapshot_id*/, Vec<NodeKey>)>,
    lane_anchor: Option<NodeKey>,           // restore the focused slice on remount
    // omnibox/query state is transient (not persisted)
}
```

## Shared state usage

`tree_collapsed` (h/l fold subtrees to summary chips; bounds the hairball at scale; persists across
skin switches) · `recalc_mode` + `manual_recalc_pending` (Space-m manual-batch: stage
`EditContentDeferred` with a growing amber blast radius, pay one `Recalculate` on F9) · `pinned`
(Space-p pins key drivers/outputs in the lane regardless of focus) · `workspace_ids` +
`active_workspace_id` (Space-w switch). All mutated per the cross-skin contract — and after
`audited-shared-state` lands, the calc-affecting `recalc_mode` switch routes through the dispatcher.

## Zippiness design (honest, against the real host)

- **Decouple typing from the engine.** Omnibox match, health/blast-radius queries, dependency-walk
  highlighting, and arrow nav are all host-side reads over the last published `WorkspaceState`; they
  dispatch at most `SelectNode`/`SelectNodes`, which route to the selection signal with zero session
  call. The passive engine never wakes while you explore.
- **Engineer render cheapness — don't assume it.** Today the host publishes one monolithic
  `RwSignal<WorkspaceState>` and no skin uses a keyed `<For>`. So S1/S2 **build**: derived `Memo`s keyed
  on `value_epoch` / `dependency_shape_snapshot_id` / `publication_snapshot_id`; a keyed `leptos::For`
  by `NodeKey` with per-node `Memo`s so only changed chips re-render; skip wire/layout recompute when
  `dependency_shape_snapshot_id` is unchanged. (`projection-delta-channel` makes this first-class.)
- **The sweep is a host-side replay** over already-published `evaluation_order` + before/after deltas —
  not further engine calls. On a slow recalc the head starts on the first frame *after* the single
  blocking republish returns.
- **Animate only what changed**, and only the on-screen intersection of `evaluation_order`; never
  animate unchanged nodes; never route wires for off-screen/collapsed chips.
- **Collapse N edits into one republish** — manual-batch, one-shot `ReplicateContent`, batched scenario
  flips. The only lever a skin controls against the full-struct `.set()` cost (until the delta channel
  and `host-worker-calc` land).
- **Honest freeze UX.** `Recalculate` triggers recalc *plus* a full `workspace_state()` rebuild
  (currently an `O(N·E)` reverse-scan on the main thread before the signal fires). FLOW shows a
  synchronous "computing…" state on F9 and starts the sweep on the next frame. (Indexing that scan, and
  `host-worker-calc`, remove the freeze — both in the stack requirements.)

## Visual language

A departure from the warm utilitarian prototypes toward something **alive, precise, instrument-grade**
— oscilloscope/synth-panel meets a code editor's minimap. Near-black graphite canvas (`#0E1116`) with
cool desaturated chips, so `calc_state` colours are the only saturated thing on screen and read as
signal: DirtyPending amber, Evaluating electric-cyan pulse, VerifiedClean phosphor green,
Error/CycleBlocked hot magenta-red, Clean muted slate. The focus chip's border switches between a calm
**SELECTED** outline and a bright **EDITING** outline (Excel's Ready/Edit), so the 1-bit mode is always
visible. The reading-head is a single bright vertical light-bar with a motion-blur trail; wires are
thin bezier arcs that ignite cyan-then-green as the head passes (CSS, off the calc path). Tabular-figure
mono for values (odometer-rolls align digit-for-digit; gated to plain-numeric on both ends, hard-swap
otherwise); humanist sans for prose formula bands. Implemented today as the inline `dtc-*` CSS string
with semantic classes (`dtc-chip`, `dtc-chip--editing`, `dtc-pip--evaluating`, `dtc-wire--live`,
`dtc-head`, `dtc-ghost`) so the later `design-token-layer` can swap hex without touching markup.

## Build plan

| Slice | Scope | IR deps | Demoable wow |
|---|---|---|---|
| **S1** | The Sentence (stable-topo ribbon, ≤40, virtualized) + modeless nav + the 1-bit edit flag + `/` omnibox + which-key footer + the keyed-`For`/memo render foundation | today only | arrow + type-to-edit (division-safe); fuzzy jump; the model reads like a sentence |
| **S2** | F9 reading-head sweep + cycle knots + odometer-roll + wire-ignition + phase-timing badges + "computing…" freeze UX | today only | press F9 and watch the model think — the headline Excel-can't moment |
| **S3** | `]`/`[` dependency walk + Space health/blast-radius lenses + delete blast-radius + recursive E-explain + manual-batch | today only | walk the graph by keyboard; one-chord health; see what breaks before you delete |
| **S4** | multi-select + `ReplicateContent` fill + promote-to-template + scenario chips | **+`SelectNodes`, +`ReplicateContent`** | fill one name-relative rule across a set, reorder, watch it re-thread |
| **S5** | `PreviewEdit` ghost what-if + speculative sweep + delta ledger + sensitivity brightening | **+`PreviewEdit`** | type a hypothetical and watch it ripple as ghosts; zero undo debt |

The first three slices — the entire "blow them away" experience — need **zero engine changes**.

## Honest limitations

- The F9 sweep is a host-side **replay** over before/after deltas, not a live engine mirror — the
  synchronous full-republish gives only before/after, no intermediate Evaluating frames. We label it
  "the order the engine used" and use `phase_timings` for real timing, so it never misleads, but
  per-step timing is choreography, not measurement (until incremental deltas land).
- **Layout axis ≠ animation axis** by design: the lane is a stable topo sort; only the sweep uses the
  volatile `evaluation_order`. The visible left-to-right order is FLOW's topo choice, not literally
  "the order the engine ran" for the static layout. Stated in copy.
- **Scale** is a focused ≤40-chip slice with virtualization and `tree_collapsed` folding — you trade
  "whole model at a glance" for "legible focused slice".
- **Zippiness on the publish path is built, not free** (see above); on a large tree F9 has a visible
  pre-sweep hang until the delta channel + worker land.
- The two boldest exploration powers (reorder-surviving fill, ghost what-if) live behind the three IR
  additions; v1 ships a strong observe/audit/edit shell on today's IR with those honestly flagged.
- `ReplicateContent` rebind correctness is verifiable only **post-hoc** (read back `content_text` +
  `requires_rebind`) until a pre-bind `reference_bindings` preview exists — a real trust risk for a
  financial modeler if a fill silently binds to a shadowed ancestor; we surface what happened, not
  predictively.
- `computed_value.Scalar` is raw debug text today — odometer-rolls and delta magnitudes are best-effort
  and gated to plain numerics until `format-resolver-on-context` lands.

## Name rationale

FLOW names the one thing it makes visible that Excel never has: the model in the act of computing —
values flowing downhill through the dependency graph, animated in the engine's real evaluation order.
It doubles as the visual metaphor (the reading-head sweep, igniting wires, left-to-right current) and
the felt quality (zippy, fluid, modeless). The word carries zero Vim/modal baggage and reads as alive
and precise rather than warm-utilitarian. *"The model that thinks out loud"* captures the thesis: F9
stops being a black box and the model narrates its own computation.

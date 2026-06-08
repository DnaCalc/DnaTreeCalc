# FLOW — Vision

> *Assuming the [stack requirements](../stack-requirements/) are in place.* This is the north-star
> capture for the next design + implementation stages — where FLOW goes from "watch the model think"
> to a full **modeling instrument**. The concrete, ships-today design is in
> [`FLOW_DESIGN.md`](FLOW_DESIGN.md); this doc is deliberately further-reaching.

---

## The shift: from spreadsheet skin to modeling instrument

FLOW today is already an Excel-can't moment: a model laid out as a sentence that narrates its own
recomputation. But that is the *seed*. With the stack in place, FLOW stops being "a nicer way to look
at cells" and becomes an **instrument for thinking with a model** — something you *interrogate,
simulate, time-travel, compare, and explore*, the way a debugger, a synthesizer, and a wind-tunnel are
instruments. The spreadsheet was a static photograph of a calculation; FLOW becomes a *live
performance* of one, with a transport bar, a multimeter, and a what-if bench.

The crucial discipline never changes: **every faculty below is still frame-only over engine truth.**
FLOW never computes, never re-derives, never fakes a value or a format. The magic is entirely in
*surfacing* what OxCalc/OxFml already know — which is exactly what makes a tool this expressive also
*provably faithful* to Excel and amenable to the program's formal-correctness aim. The instrument is
trustworthy because it is a window, not a paint job.

What follows is organised as seven **faculties** the stack unlocks. Each names the requirements that
power it, so the vision stays grounded in the matrix rather than hand-waving.

---

## I. Read it like prose — *faithful display*

`richer-typed-value` · `format-resolver-on-context` · `per-node-effective-format` · `active-node-detail`

The Sentence stops reading "raw debug text" and starts reading like a finished model: numbers as
currency/percent/dates exactly as Excel would render them (engine-resolved, never host-parsed), typed
glyphs for logicals and errors, lambda chips with arity, reference chips you can click, units where the
value carries them. The formula band shows the live binding diagnostics inline (a red squiggle under an
unresolved name, a tooltip explaining a profile rejection). A non-author can read
`Revenue · COGS · Margin = Revenue − COGS · FY.Margin = average of quarters` as **English at reading
speed** — audit becomes literacy.

## II. Ask it *why* — *the calc debugger*

`full-derivation-trace` · `EvaluateFragment` · `derivation-trace-for-candidate` · `typed-cycle-diagnostics`

**E** on any chip stops being "show my inputs" and becomes a **kernel-level call stack**: template
selection, hole bindings, the ordered child prepared-call tree, each call's inputs and result, down to
the leaves. Hover a sub-expression and see its value (Excel's Evaluate-Formula, but recursive,
navigable, and never opaque). A SUM-over-`.*` enumerates its live members with a version stamp; a
`CycleBlocked` knot shows its convergence curve and terminal state instead of a dead badge. You don't
guess why a number is what it is — **the number derives itself, on demand, to any depth.** And because
the same trace is addressable per candidate (faculty III), you can ask "*why is this different under the
Bear scenario?*" and get a side-by-side derivation diff.

## III. Ask it *what-if* — *consequence-free simulation*

`candidate-overlay-handle` · `value-published-pending-flag` · `preview-edit-intent` ·
`speculation-discard-commit` · `scenario-substrate` · `comparative-multi-overlay-projection` ·
`goal-seek-substrate` · `sensitivity-sweep-substrate`

This is the band that most changes what FLOW *is*. On the engine's addressable, layerable,
**non-publishing** candidate overlays, exploration becomes free of consequence and provenanced:

- **Ghost what-if.** Type a hypothetical into a node and watch the consequence ripple through the
  Sentence as *ghost* values — provenance-tagged so a ghost can never be mistaken for real — without
  touching history. Esc forgets it for one keypress; Enter blesses it into a real edit. You probe ten
  times more hypotheses because each costs nothing.
- **Scenario rail.** Base / Bull / Bear as overlays you flip with number keys; the `comparative`
  projection renders them as **adjacent columns per node** — a side-by-side no Excel data-table can do
  over a tree shape. Each scenario chip is labelled by its live headline output.
- **Goal-seek with a visible convergence trace.** "Set `FY.Margin` to 1M by varying `GrowthRate`" runs
  as a Newton loop of candidate evals — you *watch* it converge, each step a candidate that never
  touches history.
- **Sensitivity sweep / tornado.** Pick an output, vary tree-shaped inputs across N points, observe the
  grid, and the drivers rank themselves into a tornado — separating load-bearing assumptions from inert
  ones at a glance.

## IV. Travel its time — *the model has a past*

`revision-graph-retention` · `revision-history-projection` · `undo-redo-revision-nav` · `value-shape-diff`

With a retained, navigable revision **DAG**, FLOW grows a **transport bar**. Scrub the workspace to any
past revision and watch prior *computed values* re-appear (the engine retained them) — undo as
time-travel, not a flat stack. Because edit-after-undo *branches*, history is a DAG you can see: the
point where you tried something, backed out, and went another way is a visible fork. `value-shape-diff`
turns any two revisions (or a baseline vs a what-if) into a **change-pulse**: precisely which
nodes/cells moved, as a heat overlay, with arrays showing only the changed cells. The model stops being
a single "now" and becomes a landscape you move through.

## V. Hold the whole thing — *scale and the cockpit*

`projection-delta-channel` · `host-worker-calc` · `virtualization-window-projection` ·
`multi-slot-composition` · `keybinding-registry` · `shared-focus-arbitration` · `design-token-layer` ·
`a11y-primitives`

Incremental deltas + a host-owned worker + windowed projection mean a **100k-node model stays buttery**:
the wavefront animates over real deltas (not a full republish), a slow recalc never freezes the frame
(the engine stays single-threaded and passive; the *host* pumps slices), and only the ~100 visible rows
materialise. And `multi-slot-composition` turns FLOW from one lane into a **cockpit**:

- **Main** — the Sentence, with the reading-head and wires.
- **Right inspector** — the focus chip's full derivation stack (faculty II), following selection.
- **Split** — the scenario-compare columns or the what-if bench (faculty III).
- **A rail** — the history transport (faculty IV).

All four are independent skins over one shared truth (selection in one highlights in all), arbitrated
for keys and focus, themed (dark / high-contrast / re-skin) and accessible by construction. The cockpit
is the destination; FLOW the Sentence is its centrepiece.

## VI. Author fearlessly — *predict before you pay*

`selection-subject-model` · `scope-value` · `replicate-by-id` · `reference-insertion` ·
`f4-toggle-binding` · `rename-move-ref-integrity` · `legality-impact-preview` · `engine-dry-bind` ·
`recalc-plan-preview` · `set-membership-write` · `duplicate-subtree` · `template-subsystem`

Structural modeling becomes *clay you can't accidentally break*. Multi-select a named set and **fill one
name-relative rule** that rebinds per target by lexical role and survives reorder (Excel's most-loved,
most-fragile gesture, fixed). Point-mode insert a reference by clicking a node (OxFml composes the
text, the host splices at the caret). Rename or drag-reparent and watch every reference **re-thread
automatically**, with a pre-commit "this rebinds 7 dependents, risks a cycle" answer *before* you
commit — the interrogative mood: the same dispatcher that mutates will *predict* legality, blast-radius,
collisions, and cost without mutating. Promote a sub-model to a **template**, stamp it per region, push
edits to all instances and see drift. Edit a SUM-over-set's **membership** directly. The model bends to
restructuring instead of resisting it.

## VII. Explore together — *recorded performances & collaboration*

`intent-log-replay` · `readonly-reviewer-persona` · `collab-presence-markers` · `intent-conflict-policy`

Because every change funnels through one dispatcher as a recordable `(intent, receipt, delta, revision)`
stream, an *exploration is a first-class artifact*. Record a what-if session and **replay it as a guided
narrative** — for onboarding ("here's how this model works"), for audit ("here's exactly what changed
and why"), for teaching. A reviewer persona is enforced centrally per intent origin. And the same
immutable-revision substrate that powers time-travel is the substrate for **multi-analyst exploration**
(research-grade): presence cursors, advisory edit-claims, and what-if branches that different people
own, compare, and merge — *git for models*, where a branch is a kept candidate and a merge is a rebase
over the revision DAG.

---

## New ideas worth keeping

The faculties compose into things that aren't in the original FLOW and are worth capturing now:

- **What-if branches as first-class.** The revision DAG + candidate overlays + provenance mean a ghost
  can be *promoted to a branch you keep* — name it, pin it, compare it column-against-column with the
  baseline and with other branches, and eventually merge it. Modeling gets a version-control mental
  model that Excel's "Save As copy" only gestures at.
- **The tornado as a gesture, not a wizard.** Select an output, press one key: FLOW sweeps every
  upstream driver and ranks them. Sensitivity analysis becomes a reflex, not a multi-dialog chore.
- **Live instruments.** With external/RTD intake, the Sentence becomes a live dashboard: the reading-head
  pulses on each tick, only the affected sub-graph re-animates, and provenance distinguishes
  live-external from computed.
- **Explain-anywhere.** Any value — published, pending, in any scenario, at any revision — can unfold its
  full derivation. "Why" is never gated by *when* or *which what-if*.
- **Calc-as-conversation.** The interrogative mood (legality preview, plan preview, explain, what-if
  ghost) means you *ask the model questions and it answers before you commit*. The loop stops being
  edit→hope→F9→check and becomes ask→see→decide→commit.
- **The model that teaches itself.** Recorded performances + read-it-like-prose + explain-anywhere mean
  a finished model can ship with a *replayable walkthrough of its own reasoning* — a living model card.

---

## The through-line (why this is sound, not just shiny)

Every faculty above is a **read over engine-published truth or a deliberate closed-enum intent** — never
a host/skin reinterpretation of semantics. Ghosts are explicitly non-authoritative and structurally
off the publish path; undo is revision navigation, never inverse-replay; formats and values are the
engine's, surfaced not recomputed; speculation cannot pollute history; provenance prevents a single
uncommitted number from being trusted. That discipline is *why* FLOW can be this expressive without
betraying the program's core aim — an Excel-faithful stack that can be *changed while staying provably
correct*. A more honest instrument is, here, a more *powerful* one: the better FLOW surfaces engine
truth, the more it can do, because it never has to defend a fiction.

## What it means for the next stages

- **Design.** The cockpit (faculty V) is the layout north star; each pane is a focused skin over shared
  truth. Design the four-pane composition, the transport bar (IV), the what-if bench and scenario
  columns (III), and the derivation inspector (II) as a coherent whole — FLOW the Sentence is the
  centrepiece, not the entirety.
- **Implementation.** The [waves](../stack-requirements/ROADMAP.md) deliver the faculties in dependency
  order: W0–W3 give faculties I, II (read side), VI; W4 gives III and IV; W5 gives V; W6 extends III/VI;
  W7 is VII. FLOW's own [build plan](FLOW_DESIGN.md#build-plan) S1–S3 ship the seed on today's IR and
  *prove the architecture* — that one core carries a genuinely new way of seeing a model — before the
  engine workstreams land.
- **The bet.** If FLOW the Sentence already makes Excel power users say "I didn't know I could *see*
  that," the cockpit makes them say "I can't model any other way now." That is the "something special"
  — and it is reachable, in order, without ever asking the engine to lie.

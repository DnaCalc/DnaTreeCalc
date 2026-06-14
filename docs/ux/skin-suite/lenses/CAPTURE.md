# Capture — structure-by-typing

Type a dotted path that doesn't exist and the tree scaffolds itself. `Ctrl+1`,
`dnatreecalc-skins/src/capture.rs`.

## Intent

> **Audit yardstick.** This section states what Capture is *for* — the design
> intent. A later audit scores the built lens against the **Audit checklist**;
> a gap there is a finding, not a doc error.

**Perspective — how you look at the model here.** Capture is throughput-first
*first entry*: the tree grows from a top-to-bottom stream of typed lines, and
you are *writing input*, not manipulating shape. Hierarchy comes from the dotted
path, not from where you click; the model unfolds in the echo pane as you go
while your frontier line stays hot. It is frame-only — Capture stores exactly
what you typed and never re-derives it. The question it answers: *"How do I get
structure and content into the model as fast as I can type?"*

**What you can do here**
- Type a dotted path (`Accounts.Q1.Sales`) and scaffold every missing segment as **one transaction — one undo**.
- `Tab` to the content box and author a value or an `=formula`; the engine classifies it, Capture never parses it.
- Let the path box prefill the parent path after each line, so `Enter` drops the next sibling without retyping.
- Arm a **template** chip so the next content-less entry clones that starter verbatim.
- Stage a pasted multi-row block as a **dry-run** and commit it as a batch *(design intent — see suite follow-ups)*.
- Read a typed **accept/reject history** — every rejection shows the engine's own error, never a silent failure.
- Watch the **echo outline** (effective-meta filtered) grow as lines land, and select any node to edit its content modelessly.

**What it deliberately leaves to other lenses**
- Reshaping existing structure (move / reorder / refactor) → **Tree**.
- Population filter/sort and bulk authoring → **Ledger**.
- The formula bar, point-mode references, and table cells → **Sheet**.
- No grid/A1 coordinates, no multi-select, no live legality net during entry (rejections are post-attempt).

**Audit checklist — does the build realize the intent?**
1. A multi-segment path lands as **one revision** (a single `Ctrl+Z` removes the whole scaffold), not N separate adds.
2. A leading `=` is accepted only in the content box; an `=` in the path box is rejected with a guiding message.
3. After a successful line the path box holds `Parent.` and `Enter` creates the next sibling.
4. Every rejected line surfaces the engine's typed error on the history strip; nothing partially commits.
5. The echo pane hides effectively-meta nodes via the shared contagion walk, not a local `is_meta` check.
6. Selection, fold state, and an armed template survive a switch to another lens and back.

## How it works

**The capture line — two fields.** A **path box** (`Dotted.Path.Leaf`) and a
**content box**; `Tab` toggles between them, `Enter` commits, `Esc` clears both.
The path box is split on `.` into segments; the content box is handed to the
host **verbatim** (`=Net/Sales` authors a formula, `5` a constant — the host
classifies; the lens never parses formula text). A leading `=` belongs only in
the content box, so the formula sigil is unambiguous; an `=` typed in the path
box is rejected with a guiding message. After a successful line the path box
resets to the parent path + `.` so the next `Enter` drops the next sibling.

**One transaction = one undo — via the candidate lane.** A single missing
segment is a plain `AddNode`. Multiple missing segments ride
`OpenCandidate → AddCandidateNode` per segment (parented by the key read back
from the candidate projection) `→ EvaluateCandidate → CommitCandidate`: the
whole scaffold publishes as **one revision**. Any typed rejection triggers
`DiscardCandidate` — nothing published, true atomic rollback through existing
closed intents.

**Also:** template starter chips (projected `templates.entries`, armed initial
content cloned verbatim), a typed accept/reject history strip, and a live
outline echo pane (effective-meta filtered).

**Tests:** path parsing + the two-field builder (value/formula passthrough, the
`=`-in-path guard, junk), the history-line display form, scaffold planning
(longest existing prefix), AddNode payloads, sibling prefill. The candidate
walk itself is host-level test territory (the in-memory dispatcher doesn't
materialize candidate projections).

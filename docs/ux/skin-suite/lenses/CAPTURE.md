# Capture — structure-by-typing

Type a dotted path that doesn't exist and the tree scaffolds itself. `Ctrl+1`,
`dnatreecalc-skins/src/capture.rs`.

**The capture line:** `Dotted.Path.Leaf` or `Dotted.Path.Leaf = content`. The
line splits at the FIRST `=`; everything after passes to the host **verbatim**
(`Margin = =Net/Sales` authors a formula, `Margin = 5` a constant — the host
classifies; the lens never parses formula text). After a successful line the
input resets to the parent path + `.` so Enter drops the next sibling.

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

**Tests:** line parsing (constant/formula passthrough/junk), scaffold planning
(longest existing prefix), AddNode payloads, sibling prefill. The candidate
walk itself is host-level test territory (the in-memory dispatcher doesn't
materialize candidate projections).

# Transport — revision time-travel

The revision-DAG scrubber: undo as navigation, never replay. `Ctrl+7`,
`dnatreecalc-skins/src/transport.rs`.

Built in **Phase A**: the W4a retained revision graph is landed and projected.

## Intent

> **Audit yardstick.** What Transport is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Transport reframes **undo as
navigation**, not inverse replay — the model is a history you walk, anchored in
the engine's retained revision DAG. Click a past revision and you *jump* there
(the engine restores that state); undo-then-edit **forks** the DAG, and Transport
shows the branch. Each card is honest about cost — what it invalidated, what
rebound, what still exists — so you read change between two points without
diffing. The question it answers: *"How did the model get here, and what changed
at each step?"*

**What you can do here**
- Click any non-current revision to **navigate** there (engine-owned restoration), with every other lens re-projecting to that moment.
- See **branch points** (child-count badges) where undo-then-edit forked history.
- Read per-revision **invalidation summaries** (node count, rebinds, typed reasons) and click an invalidated-node chip to select it live.
- Watch the **latest-delta pulse** ("3 values changed · structure +2 −1") and see open candidates badged to their basis revision.
- Step with `Ctrl+Z/Y` (shell-global); enablement + disabled-reason are read from the command catalog.

**What it deliberately leaves to other lenses**
- Reads engine-projected history facts only — never reconstructs, diffs, or replays state.
- Selection clicks dispatch `SelectNode`; no inline edits.
- No timestamp / author / label on revisions yet (cards identify by short id + summary — follow-up #7).
- Candidate *verbs* stay in **Bench**; narrative replay is a later deliverable — Transport is the scrubber, not the story timeline.

**Audit checklist — does the build realize the intent?**
1. Entries are newest-first; scrolling down walks backward in time.
2. A revision with >1 child shows a branch-point badge (children counted by `parent_revision_id`).
3. A non-current card navigates via `NavigateRevision`; the current card is an inert "you are here" marker.
4. Invalidated-node chips resolve through `paths_by_key`: resolvable → clickable `SelectNode`, otherwise faded.
5. Per-revision summaries are engine-projected and rendered as-is (not re-derived).
6. The delta pulse summarizes the real projection delta; a nil delta renders nothing.
7. Undo/Redo enablement + disabled reasons come from the command catalog; navigating updates the shared revision cursor so every lens follows.

**Timeline:** newest-first rail from `revision_history` — current marker,
short ids (full in tooltips), branch-point badges (children counted by
`parent_revision_id` links), and per-revision **transaction summaries**:
invalidated-node count, rebind count, and node chips that resolve to *current*
display paths where the node still exists (clickable → `SelectNode`; faded raw
key otherwise), with typed invalidation reasons in their titles.

**Navigation:** clicking a non-current revision → `NavigateRevision`
(engine-owned restoration — never inverse replay); Undo/Redo buttons read
enablement **and disabled reasons** from the command catalog; `Ctrl+Z/Y`
remain shell-global.

**Live pulse:** the projection delta channel (`latest_delta`) summarized to one
line ("3 values changed · structure: +2 −1").

**Candidates in time:** open candidates badge their basis revision on the
timeline ("candidate based here") — speculation visibly anchored to history.
Candidate *verbs* stay in Bench; Transport is read-and-navigate.

**Honesty:** entries carry no timestamp/author/label (follow-up #7), so cards
identify by ids + summaries; the value-shape change-pulse between two arbitrary
revisions awaits the value-shape-diff substrate (Phase C).

**Tests:** newest-first ordering + branch counting (branched fixture), pulse
summaries, navigate-intent guard (current entry inert).

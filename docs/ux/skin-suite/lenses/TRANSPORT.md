# Transport — revision time-travel

The revision-DAG scrubber: undo as navigation, never replay. `Ctrl+7`,
`dnatreecalc-skins/src/transport.rs`.

Built in **Phase A**: the W4a retained revision graph is landed and projected.

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

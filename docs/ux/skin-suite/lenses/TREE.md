# Tree — the structure/refactor lens

The hierarchy made spatial: an indented, foldable outline for reshaping the
model. `Ctrl+2`, `dnatreecalc-skins/src/tree.rs`.

## Intent

> **Audit yardstick.** What Tree is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Tree puts you *inside the
containment structure* — hierarchy IS the interface. Parent / child / sibling
read as indentation and vertical position; the model is a shape you sculpt. You
grab a subtree, see where it will land and what will rebind, and commit one
atomic move. The question it answers: *"What is the structure right now, and how
do I reshape it safely?"*

**What you can do here**
- Fold / unfold subtrees (`h` / `l`); the fold set is **shared**, so every lens collapses with you.
- Add, rename, move, reorder, delete, and duplicate nodes through the structure itself.
- Reshape "like clay" with a **live legality net** that says what a move/rename will rebind *before* you commit *(design intent; today a post-attempt rejection strip — follow-up #1)*.
- Duplicate a formula-free subtree (`_copy`), or copy a subtree to the clipboard for paste elsewhere.
- See the **dependents badge** (incoming count) so you refactor with awareness of who depends on a node.
- Show / hide effectively-meta scaffolding; navigate by arrows (↑↓ siblings, ← parent, → first child) and `/` Name-Box.
- Edit the selected node modelessly (`Enter` to edit, `Enter` to commit-and-advance, `Esc` to cancel).

**What it deliberately leaves to other lenses**
- The formula bar, point-mode references, and table cells → **Sheet**.
- Population filter/sort and bulk authoring → **Ledger**.
- No grid/A1 coordinates — depth is indentation, order is sibling sequence.
- Single-select today; `drag` and `Tab` are lens-local chords, not the universal grammar.

**Audit checklist — does the build realize the intent?**
1. Rows walk **depth-first in engine child order** (never sorted); a collapsed subtree hides its descendants.
2. The fold set lives in **shared continuity** (`collapsed_keys`) — folding in Tree shows folded in Flow/Sheet.
3. A structural verb (rename/move/delete/duplicate) is **one transaction — one undo**; a rejection lands in a visible strip with the engine's typed reason (no silent failure, no half-applied move).
4. The dependents badge reads the projection's incoming-count, never a re-derived graph.
5. Effectively-meta nodes render faint and vanish entirely when hidden, via the shared contagion walk.
6. `calc_state` is the only saturated color; SELECTED-vs-EDITING is a single 1-bit border.
7. Selection survives a lens switch (re-projection, not reload).

**Reads:** `root_paths` + `NodeView.children` (engine child order, never
sorted), `is_effective_meta` (meta shown faint, hideable via lens state),
`dependencies.incoming_count_by_key` (the dependents badge — refactor with
awareness), `calc_state`, `templates`, `command_catalog`.

**Writes:** `SelectNode`; add/rename/move/reorder/delete via the embedded
`NodeManagementPanel`; `DuplicateSubtree` (formula-free subtrees; `_copy`
symbol); `CopyToClipboard` (Subtree payload); content edits through the shared
inspector → `commit_content_edit`.

**Continuity:** the fold set is `SharedSkinState.collapsed_keys` (NodeKey-keyed,
shared suite-wide) — `h`/`l` fold/unfold the selected node; leaves are never
added to the set. Selection survives lens switches.

**Honesty:** no live legality net yet (the preview seam is follow-up #1 in the
suite README); a dismissible strip surfaces typed receipt rejections instead.

**Tests:** depth-first row walk (fold skips descendants, meta contagion,
children order), fold toggle, duplicate/copy intent payloads.

# Tree — the structure/refactor lens

The hierarchy made spatial: an indented, foldable outline for reshaping the
model. `Ctrl+2`, `dnatreecalc-skins/src/tree.rs`.

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

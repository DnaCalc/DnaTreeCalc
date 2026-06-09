# Ledger — the population lens

Think in populations: cleave the model to the rows that matter, sort by typed
value, select a class, author it at once. `Ctrl+3`,
`dnatreecalc-skins/src/ledger.rs`.

**Reads:** rows via `cleave_filtered_keys` (effective-meta dropped); per-row
typed columns — content kind, content (inline-editable), typed value,
calc-state dot, effective format, dependency in/out counts, note marker.
Optional calc-state cohort grouping (presentation only).

**Writes:** the **cleave bar** writes `SharedSkinState.cleave` — Ledger is the
predicate's home and it carries to every lens (predicate-only; each lens
re-applies it). Filters: Errors / Stale / Text-match (foreign predicate
variants written by other lenses display as “(custom)”). Sorts: name / typed
value / depth, both directions (numbers always before non-numbers). Inline
content edits via `commit_content_edit`. **Bulk bar** over the shared
`selection_set`: `EditScopedContent` / `SetNumberFormat` (set + clear) /
`CopyToClipboard(Values)` — each ONE dispatch over `AuthoringScope::Nodes`,
one host transaction, one undo.

**Continuity:** `selection_set` + `selection_anchor` are shared view-state; the
dispatcher-routed `SelectNode` primary stays the audited anchor (bulk verbs are
auditable because they carry the scope in the intent).

**Tests:** row building through cleave, exact bulk intent payloads, selection
toggling, cleave-choice round-trips.

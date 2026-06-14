# Ledger — the population lens

Think in populations: cleave the model to the rows that matter, sort by typed
value, select a class, author it at once. `Ctrl+3`,
`dnatreecalc-skins/src/ledger.rs`.

## Intent

> **Audit yardstick.** What Ledger is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Ledger inverts hierarchy into
a **population-first audit grid** — a flat, scannable, queryable set of rows. The
organizing spine is the *column / predicate*, not the edge. The model becomes an
attribute matrix you slice by filter, order by real typed value, band into
health cohorts, and command in bulk. The question it answers: *"Which nodes match
this property, and how do I author them all at once?"*

**What you can do here**
- **Cleave** the population by predicate (errors / stale / text match) and carry that predicate into every other lens.
- Sort by name, **real typed value**, or depth (numbers before non-numbers), both directions.
- Band rows into calc-state **health cohorts** (presentation-only).
- Multi-select a class (and select-all-visible) and **author it at once** — content, number format, copy values — each **one transaction, one undo**.
- Inline-edit a single row's content, honoring the Auto/Manual recalc mode.
- Read typed per-row columns (kind, content, value, calc-state, format, dep in/out, note) and drill the selection in the Lens companion.

**What it deliberately leaves to other lenses**
- The formula bar, point-mode references, table authoring → **Sheet**.
- Structural refactor (move / reorder) → **Tree**.
- The **predicate** is shared continuity; the health banding and presentation are Ledger-local, never shared.
- Single predicate only today (no and/or, no numeric ranges); foreign predicates show as "(custom)".

**Audit checklist — does the build realize the intent?**
1. The visible rows are exactly the cleave predicate's result minus effective-meta — no stragglers.
2. Sorting by value orders numbers numerically and before non-numbers, in both directions.
3. A bulk verb is **one dispatch over the selected set → one host transaction → one undo** (a single revision).
4. The cleave predicate is shared: set it in Ledger, switch to Flow, and the same predicate re-applies to the fresh projection (no frozen materialized set).
5. Multi-select routes through the auditable scope; selection drives the shared primary anchor.
6. `calc_state` is the only saturated channel; provenance is structural tint; SELECTED/EDITING is 1-bit.

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

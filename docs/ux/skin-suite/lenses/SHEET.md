# Sheet — the Excel edit loop

Values by default, a real formula bar, tables expand in place — no A1, no
coords. `Ctrl+4`, `dnatreecalc-skins/src/sheet.rs`.

## Intent

> **Audit yardstick.** What Sheet is *for* — the design intent. A later audit
> scores the built lens against the **Audit checklist**; a gap there is a
> finding, not a doc error.

**Perspective — how you look at the model here.** Sheet is the **Excel edit loop
on a tree** — value-first cells, one cell at a time, with a real formula bar.
It's depth-first and one-cell-deep: attention moves from structure to *content*.
This is the closest-to-Excel onramp in the suite. The question it answers: *"What
is this cell's value, what formula drives it, and what does it depend on?"*

**What you can do here**
- Select any node or table cell and see/author its content in a **formula bar**, badged by editability (node content / table cell / column formula / totals / read-only).
- Type values by default; a leading `=` makes a formula — the host classifies, Sheet passes the text verbatim.
- **Point-mode reference insert**: arm "Insert ref", click a node row, and the host splices a name-based reference at the caret (no hand-typed coordinates) *(commits the recomposed formula — follow-up #2)*.
- **Tables expand in place** as real grids (header / body / totals); click a cell to select it, add rows/columns, and **author a new 2×1 table** from the toolbar.
- Cycle a reference's **binding** with `F4` and fill with `Ctrl+D` *(reserved verbs, pending their intents)*.
- Trace (`]` / `[`), explain (`E`), jump (`/`), recalc (`F9`), undo/redo (`Ctrl+Z/Y`) — the universal grammar.

**What it deliberately leaves to other lenses**
- Population cleave / bulk audit → **Ledger**; structural shape → **Tree** (Sheet demotes hierarchy to indentation).
- No A1/grid coordinates — references are node-addressed handles; never parses formula text or invents values.
- Intrinsic geometry (column widths) is skin-local, not re-projected across lens switches.
- Deep table *authoring* (schemas, calculated-column management at scale) is a later deliverable.

**Audit checklist — does the build realize the intent?**
1. The formula bar binds to the active selection and routes each commit to the correct intent per editability (node content / `EditTableCell` / column formula / totals); read-only disables the bar.
2. Point-mode insert moves a caret over **node rows** (not grid cells) and inserts a **name-based** reference, not an A1 coordinate.
3. `F4` cycles a binding on node identity / lexical role, not a column/row offset.
4. Values render first/primary; the formula appears on selection or via the show-formulas toggle.
5. Tables expand in place from the projection; a cell click selects the cell; the toolbar's **New table** creates a 2×1 starter grid.
6. `Tab` (next column) and drag are badged lens-local; `Enter` is the universal commit-and-advance; `F9` works from inside the edit buffer.
7. `calc_state` is the only saturated channel; SELECTED/EDITING is 1-bit; continuity re-projects from other lenses.

**The formula bar** reads `active_selection_detail` and routes commits by typed
editability: node content → the shared `commit_content_edit` path; table body
`DirectInput` → `EditTableCell`; `FormulaBacked` → `EditTableColumnFormula`
(badged "column formula" — editing the cell edits the column, honestly);
totals → `SetTableTotalsFormula`; `ReadOnly` disables the bar. Text passes
verbatim; the host classifies.

**Tables in place:** real grids from `TableProjection.cells` (header/body/
totals), cell click → `SelectTableCell`, the active cell carries the shared
SELECTED border, add-row/add-column verbs under each table.

**Point-mode reference insert (armed):** with a node's formula in the bar,
arm "Insert ref", click any node row → `InsertFormulaReference` at the caret
(char-offset clamped); OxFml composes the text, the buffer refreshes from the
receipt's `FormulaReferenceInserted` delta. *Substrate honesty:* this intent
**commits** the recomposed formula — the armed mode makes that explicit; a
compose-without-commit OxFml seam is follow-up #2 in the suite README. `Ctrl+D`
fill and the F4 binding cycle remain reserved verbs pending their W3 intents.

**Tests:** commit routing per editability arm (exact payloads), caret clamp,
add-row/column payloads, selection labels.

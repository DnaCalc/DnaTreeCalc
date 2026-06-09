# Sheet — the Excel edit loop

Values by default, a real formula bar, tables expand in place — no A1, no
coords. `Ctrl+4`, `dnatreecalc-skins/src/sheet.rs`.

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

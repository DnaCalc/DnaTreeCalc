# HANDOVER_OXCALC_table_node_model

Status: Open
Target: OxCalc (with OxFml bind-layer coordination)
Ask: Define how the engine **unpacks a TreeCalc table-node concept** into engine constructs. A Table is a first-class TreeCalc node concept (columns, headers, optional totals, column formulas, structured references) — it is **not** a value in the OxFunc universe. The engine must decompose it so structured references and column formulas bind and evaluate, Excel-Table-faithfully.
Context: Tables are in scope and must be fully realized (CORE_MODEL_SPEC §7c; REQUIREMENTS §2.7; export already carries `TableSpec` in interop/EXCEL_EXPORT_AND_REPLAY.md §3.1/§4.4). The structured-reference grammar (`path[Col]`, `path[@Col]`, `path[[#Headers],[Col]]`) is already in CORE_MODEL §3.3; what's missing is how the table node lowers into things OxFml/OxCalc evaluate.
Evidence: CORE_MODEL_SPEC §3.3 (structured-ref tail), §7c (table node concept), §6 values; interop TableSpec.

## What TreeCalc needs

1. **Lowering model.** How a table node (its column children, header row, totals, per-column formula) is unpacked: a column reference resolves to an array / reference; a column formula evaluates per row; `[#Headers]`/`[#Data]`/`[#Totals]` select the right slice — without introducing a Table value type in OxFunc.
2. **Structured-reference binding (OxFml).** Bind `LHS[Col]`, `LHS[@Col]`, and composite `[[#Headers],[Col]]` on a table-typed node to the unpacked carriers, Excel-aligned (TreeCalc adopted Excel's structured-ref syntax verbatim).
3. **Dependency edges (OxCalc).** What a formula referencing a table column depends on, and how row add/remove or column-formula edits invalidate dependents.
4. **Value surface back to the host.** What the host renders in the table editor and emits for Excel replay (per-cell values, column arrays), given there is no Table `EvalValue`.

## Expected disposition

Mostly **design / coordinate** — Tables are a real cross-repo build area. The first step is agreeing the lowering model (item 1) and the OxFml/OxCalc split so the table editor (REQUIREMENTS §2.7) and Excel export (interop §4.4) rest on engine behavior, not host reconstruction.

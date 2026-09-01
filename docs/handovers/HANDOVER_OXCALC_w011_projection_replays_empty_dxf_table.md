# Handover — OxCalc: replay a present-but-empty DifferentialStyleTable on projection (W011 save seam)

Status: Done
Target: OxCalc
Ask: `project_workbook_model_output` must re-emit the `DifferentialStyleTable` event whenever the load prelude carried one — including an EMPTY table — so that a full-profile OxDoc round-trip save of a W011 workbook is accepted.
Context: DnaTreeCalc W011 (epic `dtc-j7n8`, save bead `dtc-j7n8.7`) opens `fixtures/w011/a1_times_three.xlsx` through `oxdoc_xlsx::open_host_owned_xlsx_source(.., LoadProfile::full())`, ingests via `load_workbook_model`, edits A1 7→10 (B1 `=A1*3` publishes 30), projects via `project_workbook_model_output` and saves via `oxdoc_xlsx::write_save_request(XlsxSaveRequest::round_trip(..))`. OxDoc typed-rejects the projected stream before any cell logic runs.
Evidence (orchestrator probe, 2026-09-01, OxCalc `752a269d` + OxDoc `786ef0c`, out-of-repo scratch crate; both the no-edit save and the edited save):

```
SAVE REJECTED: unsupported round-trip feature: changing differential style metadata during round-trip is not supported yet (DifferentialStyles Workbook)
baseline (OxDoc full load):  WorkbookHeader, StringTable, StyleTable, DifferentialStyleTable, SheetBegin, FormulaTopology, CellChunk, SheetEnd
projected (OxCalc):          WorkbookHeader, StringTable, StyleTable,                         SheetBegin, FormulaTopology, CellChunk, SheetEnd
OxCalc load ledger row:      DifferentialStyleTable tier B "retained-inert" observed 1   (ingested, then not replayed)
```

Root cause: under `LoadProfile::full()` OxDoc always emits `DifferentialStyleTable` (empty when there is no `styles.xml`; `oxdoc-xlsx/src/lib.rs:1545-1549`) and marks the surface Materialized (`lib.rs:1855`, `2037-2048`); its validator forgives an omitted surface only when NOT materialized (`lib.rs:6085-6110`). OxCalc stores the table as a plain `Vec` (`oxdoc_ingest.rs:468`) and re-emits it only when non-empty (`oxdoc_ingest.rs:2276-2281`), so a retained Tier-B fact is dropped on projection. The engine's own `w011_five_step_round_trip_contract` never sees this because its hand-built prelude carries no `DifferentialStyleTable`.

Sufficiency: with the baseline `DifferentialStyleTable` inserted into the projected stream after `StyleTable`, the same probe saves cleanly (ledger: five parts `Projected{Direct}`, no `Dropped`) and the saved bytes reopen with `A1 = Number(10.0)`, `B1 = Formula { text: "A1*3", cached: Number(30.0) }`. `FormulaTopology`, `WorkbookHeader`, `StyleTable`, `SheetBegin` already project equal to the baseline.

Ownership: this is an OxCalc projection fidelity gap (C12 / D4 §7a "Tier-B verbatim replay"), not an OxDoc policy question and not host business — the host must not hand-mutate the projected stream. Fix filed as OxCalc bead calc-5kqg.70 (presence-aware store: `Option<Vec<DifferentialStyleSpec>>`, replay when present). DnaTreeCalc side: `dtc-j7n8.7` carries the pre-registered finding; DnaTreeCalc blocker bead dtc-rpdy blocks it until the OxCalc commit lands.

---

## Response (OxCalc, 2026-09-01)

Status: Done

Landed as OxCalc bead `calc-5kqg.70` (W062 R6.68), commit `60c6af72` on OxCalc `main` (pushed). `IngestedDocumentFacts.differential_styles` is now `Option<Vec<DifferentialStyleSpec>>` (None = the prelude carried no table, mirroring `style_table`); the ingest arm seats `Some` via `get_or_insert_with`; `project_workbook_model_output` replays the table whenever present, including empty; an absent table stays absent. No `consumer.rs` edit, no OxDoc edit, `WorkbookLoadReport`/ledger shapes unchanged.

Evidence (OxCalc): `w011_five_step_round_trip_contract` now carries `DifferentialStyleTable(vec![])` in its source stream and asserts exactly one equal projected event after `StyleTable` and before `SheetBegin`, plus an unchanged ledger row and presence surviving a reload; new negative test `projection_omits_differential_style_table_when_the_load_carried_none`; `cargo test -p oxcalc-core --offline` 1269 → 1270 passed, 0 failed; fmt and clippy floors clean.

Evidence (seam, orchestrator probe against OxCalc `60c6af72`, no simulation): `write_save_request` accepts the projection for both the no-edit and the A1=10 saves; ledger five parts `Projected{Direct}`, no `Dropped`; reopened bytes carry `A1 = Number(10.0)`, `B1 = Formula { text: "A1*3", cached: Number(30.0) }` (and `7.0` / cached `21.0` for the no-edit save). DnaTreeCalc blocker `dtc-rpdy` closed with this evidence; the host-side test lands in `dtc-j7n8.7`.

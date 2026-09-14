//! W012 (dtc-5zet.2): **the FileCached false green**, proven on the product's
//! own open path — the executable spec for the W1 verdict ladder
//! (`Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1, "The decisive
//! discovery, and its trap").
//!
//! Every formula cell an Excel-saved workbook carries also carries Excel's
//! own computed result as `<v>`, so a cell-by-cell Excel-vs-DNA differential
//! needs no live Excel. The trap: when OxFml **rejects** a formula, OxCalc's
//! loader publishes Excel's cached value as a **persistent pin** tagged
//! `PublishedValueProvenance::FileCached` (OxCalc `consumer.rs`, the
//! `Err(GridRefError::FormulaBindRejected { .. })` arm of the ingest loop)
//! and never evaluates the cell. A comparator that reads values without
//! provenance scores that cell as a MATCH with Excel — laundering the
//! engine's refusal into a green number. The tests here make that trap a
//! committed, passing fact so the instrument can be built against it:
//!
//! > Provenance decides before value does. Only `Calculated { tick_id }` may
//! > be scored `Match` / `Mismatch`; `FileCached` / `Stale` / `Degraded` /
//! > `NotCalcModeled` are `NotEvaluated { reason }`.
//!
//! Fixtures (`fixtures/w012/false_green/{parts/,README.md}` and the Manual
//! twin `false_green_manual/`): `Sheet1` with `A1 = 7`, `B1 = =A1*3` cached
//! `21` (binds fine — the control), and `C1 = =Is?Valid*2` cached `#NAME?`
//! (`<c r="C1" t="e"><f>Is?Valid*2</f><v>#NAME?</v></c>`). `=Is?Valid*2` is
//! an Excel-legal formula — `?` is a legal character inside a defined name
//! by the file format's grammar ([MS-XLSX] §2.2.2, `name-character` includes
//! `questionmark`) — and with no name `Is?Valid` defined, `#NAME?` is exactly
//! what Excel evaluates and writes for it: the `C1` element is byte-for-byte
//! what Excel 16.0 build 20326 saved for that entry (COM, 2026-09-15; see the
//! fixture README). OxFml's lexer tokenizes `?` as `Unknown`, so the load
//! never binds `C1`. Why the cache is not a number: the Excel-faithful
//! numeric form needs the name DECLARED, and declaring a name with `?` in it
//! makes the whole open fail (`InvalidDefinedName`: OxCalc's name-key
//! validator accepts only ASCII alphanumerics, `_` and `.` after the first
//! character) — a reader-ceiling fact filed as dtc-5zet.5, not this bead's.
//!
//! The rejection premise is pinned verbatim (the exact OxFml diagnostics as
//! observed 2026-09-15 against OxFml `63a17b1` / OxCalc `03e42664`). This
//! is the fixture's second text: it was cut on `=Größe*2` (a non-ASCII name,
//! dtc-5zet.2), and when OxFml fml-kt8.2 (`cbb3099`) made non-ASCII
//! identifiers bind, this module failed loudly at the diagnostics pin, on
//! purpose, and dtc-5zet.20 re-cut it on a text OxFml still rejects. When
//! OxFml learns `?` in names the same thing happens again — re-point the
//! fixture at another Excel-legal text OxFml still rejects (the fixture
//! README lists the probed candidates), never delete the test.
//!
//! Compiled only under `cfg(test)` (the parts are zipped in memory through
//! the dev-only `oxdoc_conformance` crate, like every W011 fixture).

use std::path::PathBuf;

use dnacalc_skin_ir::{CalcModeProjection, GridAuthoredKindProjection, ValueProvenanceProjection};
use oxcalc_core::consumer::{GridCellEntryOutcome, OxCalcTreeGridCellReadout};
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::workbook_settings::PublishedValueProvenance;
use oxdoc_model::{CalcMode, CellPayload, DocumentEvent};
use oxfunc_core::value::{CalcValue, WorksheetErrorCode};

use crate::xlsx_fixture::{open_xlsx_raw, raw_cell_payload, raw_sheet_cells, repo_root};
use crate::{
    DocumentSession, HostCommand, HostCommandOutcome, LoadRecalcPath, WorkbookSession,
    build_demo_workbook,
};

/// Repo-relative location of the readable fixture parts (source of truth).
const W012_FALSE_GREEN_PARTS_REL: &str = "fixtures/w012/false_green/parts";

/// Repo-relative location of the Manual twin's readable parts.
const W012_FALSE_GREEN_MANUAL_PARTS_REL: &str = "fixtures/w012/false_green_manual/parts";

/// The `C1` cell exactly as the fixture's `xl/worksheets/sheet1.xml` carries
/// it — Excel's own encoding of an error-valued formula cell: `t="e"` and the
/// error text as `<v>`. The test asserts this literal is in the part so the
/// "equals Excel's cached `<v>`" claim is anchored in the file's bytes.
const C1_FILE_XML: &str = "<c r=\"C1\" t=\"e\"><f>Is?Valid*2</f><v>#NAME?</v></c>";

/// The rejected formula as the engine records it in the `BindDegradation`
/// row (leading `=` restored).
const C1_REJECTED_TEXT: &str = "=Is?Valid*2";

/// OxDoc's `CellPayload::Error` code for `#NAME?` — the value Excel wrote for
/// `C1`, decoded by oxdoc-xlsx's `"#NAME?" => 29` table (Excel's own error
/// code) and resolved by OxCalc's ingest to `WorksheetErrorCode::Name`.
const NAME_ERROR_PAYLOAD_CODE: u8 = 29;

/// The exact diagnostics OxFml returned for `=Is?Valid*2` when it rejected
/// the text as a formula (observed 2026-09-15, OxFml `63a17b1`): the lexer
/// tokenizes `?` as `Unknown`, so `Is` parses as a bare name and everything
/// after it is a trailing token. Spans are char offsets into the
/// `=`-prefixed text. Pinned verbatim, like W011's `OXDOC_CELL_ADD_REJECTION`:
/// an upstream change (OxFml learning `?` in names) fails this module loudly
/// instead of silently voiding its premise — exactly what happened to the
/// first text, `=Größe*2`, when OxFml fml-kt8.2 landed (dtc-5zet.20).
const C1_REJECTION_DIAGNOSTICS: [&str; 4] = [
    "unexpected trailing token Unknown [3..4]",
    "unexpected trailing token Identifier [4..9]",
    "unexpected trailing token Star [9..10]",
    "unexpected trailing token Number [10..11]",
];

fn false_green_parts_dir() -> PathBuf {
    repo_root().join(W012_FALSE_GREEN_PARTS_REL)
}

fn false_green_manual_parts_dir() -> PathBuf {
    repo_root().join(W012_FALSE_GREEN_MANUAL_PARTS_REL)
}

/// Zip a fixture's readable parts into `.xlsx` bytes in memory through
/// OxDoc's own conformance zipper (the committed-parts pattern; no zip crate
/// in this repo).
fn fixture_bytes(parts_dir: &std::path::Path) -> Vec<u8> {
    oxdoc_conformance::read_fixture_parts_as_xlsx(parts_dir).unwrap_or_else(|err| {
        panic!(
            "failed to zip the W012 fixture parts under {}: {err}",
            parts_dir.display()
        )
    })
}

/// The value the engine publishes for `C1` in every stage of both tests:
/// the file's `#NAME?`, exactly as OxCalc's ingest resolved the payload.
fn name_error() -> CalcValue {
    CalcValue::error(WorksheetErrorCode::Name)
}

/// Open fixture bytes through the command surface the product actually runs
/// (`HostCommand::OpenXlsxBytes` over a live session — the same path the W1
/// instrument must take), returning the opened session and the typed
/// `Opened` outcome.
fn open_through_host_command(
    parts_dir: &std::path::Path,
    name: &str,
) -> (DocumentSession, HostCommandOutcome) {
    println!("W012 fixture parts: {}", parts_dir.display());
    let mut document = DocumentSession::Workbook(build_demo_workbook().unwrap());
    let outcome = document
        .execute(HostCommand::OpenXlsxBytes {
            bytes: fixture_bytes(parts_dir),
            name: Some(name.to_string()),
        })
        .expect(
            "OxDoc opens and the engine ingests the W012 fixture (degradation never fails a load)",
        );
    println!("W012 OpenXlsxBytes outcome: {outcome:?}");
    (document, outcome)
}

/// The opened workbook session behind a document (the product replaces the
/// active session on open).
fn workbook(document: &mut DocumentSession) -> &mut WorkbookSession {
    match document {
        DocumentSession::Workbook(session) => session,
        other => panic!("the opened document is a Workbook session, got {other:?}"),
    }
}

/// The single grid-backed sheet of an opened fixture session.
fn only_sheet(session: &WorkbookSession) -> TreeNodeId {
    let sheets = session.sheets().unwrap();
    assert_eq!(
        sheets.len(),
        1,
        "the fixture has exactly one sheet: {sheets:?}"
    );
    sheets[0].node_id
}

/// The engine's own published cells for a sheet (`grid_view`), read at the
/// OxCalc level so the assertion is on `PublishedValueProvenance` itself —
/// the skin-IR mirror folds `Degraded` into `FileCached`
/// (`calc::value_provenance_projection`), and this bead's claim is
/// specifically `FileCached`: a value read FROM THE FILE, not a `#NAME?` the
/// engine fabricated for a cache-less rejection.
fn published_cells(
    stage: &str,
    session: &WorkbookSession,
    sheet: TreeNodeId,
) -> Vec<OxCalcTreeGridCellReadout> {
    let view = session
        .context()
        .grid_view(session.workspace_id(), sheet)
        .unwrap()
        .expect("the fixture's sheet is grid-backed");
    for cell in &view.cells {
        println!(
            "W012 [{stage}]: R{}C{} = {:?} provenance = {:?}",
            cell.address.row, cell.address.col, cell.value, cell.provenance
        );
    }
    view.cells
}

/// One published cell by 1-based `(row, col)`, failing with the whole list.
fn published_cell(
    cells: &[OxCalcTreeGridCellReadout],
    row: u32,
    col: u32,
) -> &OxCalcTreeGridCellReadout {
    cells
        .iter()
        .find(|cell| cell.address.row == row && cell.address.col == col)
        .unwrap_or_else(|| panic!("no published cell at R{row}C{col} in {cells:#?}"))
}

/// The `Calculated` tick a published cell carries; any other provenance
/// fails naming the cell.
fn calculated_tick(label: &str, cell: &OxCalcTreeGridCellReadout) -> u64 {
    match cell.provenance {
        PublishedValueProvenance::Calculated { tick_id } => tick_id,
        other => panic!("{label} is not engine-Calculated: {other:?}"),
    }
}

/// THE FALSE GREEN, as one assertion reused at every stage: `C1` publishes
/// exactly the value Excel cached in the file (`#NAME?`) — so a comparator
/// reading values alone calls it a match — and its provenance is
/// `FileCached`, not `Calculated`: the engine never evaluated it.
fn assert_c1_is_the_file_cached_pin(stage: &str, cells: &[OxCalcTreeGridCellReadout]) {
    let c1 = published_cell(cells, 1, 3);
    assert_eq!(
        c1.value,
        name_error(),
        "[{stage}] C1 publishes the file's cached #NAME? — the value a naive comparator would \
         score as a MATCH with Excel"
    );
    assert_eq!(
        c1.provenance,
        PublishedValueProvenance::FileCached,
        "[{stage}] C1's value was read from the file, never computed (not Calculated, so never \
         scorable as Match/Mismatch): provenance decides"
    );
}

/// The engine's load report, with its bind degradations pinned to exactly
/// one row naming `C1` and carrying OxFml's verbatim rejection.
fn assert_load_report_names_c1_rejected(session: &WorkbookSession) {
    let report = session
        .load_report()
        .expect("a workbook opened from xlsx bytes carries its load report");
    println!(
        "W012 load report: sheets={} cells={} formulas_bound={} recalc_path={:?} \
         engine_recalcs_at_load={} bind_degradations={:#?}",
        report.sheets,
        report.cells,
        report.formulas_bound,
        report.recalc_path,
        report.engine_recalcs_at_load,
        report.bind_degradations,
    );
    assert_eq!(report.sheets, 1);
    assert_eq!(report.cells, 1, "A1 is the one literal");
    assert_eq!(
        report.formulas_bound, 1,
        "B1 is the one BOUND formula: C1 never entered the calc graph"
    );
    assert_eq!(
        report.bind_degradations.len(),
        1,
        "exactly one degraded cell: {:?}",
        report.bind_degradations
    );
    let degradation = &report.bind_degradations[0];
    assert_eq!(degradation.address, "R1C3", "the degradation names C1");
    assert_eq!(
        degradation.text, C1_REJECTED_TEXT,
        "the authored text is retained verbatim (=-restored)"
    );
    assert_eq!(
        degradation.diagnostics, C1_REJECTION_DIAGNOSTICS,
        "OxFml's exact rejection of =Is?Valid*2 (the premise of this fixture; if OxFml now \
         accepts `?` in names, re-point the fixture at another Excel-legal text it still \
         rejects — the fixture README lists the probed candidates)"
    );
}

/// The file's own truth for `C1`, with no engine involved: the part's XML
/// literally carries Excel's `t="e"` / `<v>#NAME?</v>` encoding, and OxDoc's
/// event for the cell is `Formula { text: "Is?Valid*2", cached: Error(29) }` —
/// the cache the engine will publish unchanged.
fn assert_file_truth_of_c1(parts_dir: &std::path::Path) {
    let sheet_xml = std::fs::read_to_string(parts_dir.join("xl/worksheets/sheet1.xml")).unwrap();
    assert!(
        sheet_xml.contains(C1_FILE_XML),
        "the fixture part carries Excel's error-cached C1 verbatim: {sheet_xml}"
    );
    let raw = open_xlsx_raw(&fixture_bytes(parts_dir));
    let cells = raw_sheet_cells(&raw, "Sheet1");
    assert_eq!(cells.len(), 3, "A1, B1, C1 and nothing else: {cells:?}");
    assert_eq!(raw_cell_payload(&cells, 1, 1), &CellPayload::Number(7.0));
    assert_eq!(
        raw_cell_payload(&cells, 1, 2),
        &CellPayload::Formula {
            region: None,
            text: Some("A1*3".to_string()),
            cached: Some(Box::new(CellPayload::Number(21.0))),
        }
    );
    assert_eq!(
        raw_cell_payload(&cells, 1, 3),
        &CellPayload::Formula {
            region: None,
            text: Some("Is?Valid*2".to_string()),
            cached: Some(Box::new(CellPayload::Error(NAME_ERROR_PAYLOAD_CODE))),
        },
        "OxDoc decodes <v>#NAME?</v> under t=\"e\" to Error(29) — Excel's cached value for C1"
    );
}

/// **This is the false green.** Any comparator that reads values without
/// provenance scores `C1` as a match with Excel. See
/// `Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1.
///
/// Open the auto fixture through `HostCommand::OpenXlsxBytes` (the product's
/// path) and pin, in order: the file's own encoding of `C1`; the load report
/// (one bound formula, one `BindDegradation` naming `R1C3` with OxFml's
/// verbatim diagnostics); the open-recalc's published cells — `B1 = 21`
/// `Calculated` (control) beside `C1 = #NAME?` **`FileCached`**, equal to
/// the file's cache as OxDoc decoded it; the skin-IR mirror a host renders
/// from; and the
/// persistence of the pin — it survives `recalculate_workbook`, an `A1 -> 10`
/// edit that makes `B1 = 30` `Calculated` under a fresh tick (the engine
/// really recalculated the workbook), and a second `recalculate_workbook`.
/// The pin is persistent, not transient: no drain ever touches `C1`.
///
/// Also pinned as OBSERVED (dtc-5zet.6): the engine's authored
/// projection shows the rejected `C1` as `Empty` with no `source_text` — the
/// retained text lives only in the `BindDegradation` row. The W1 coverage
/// denominator (`total_formula_cells`) must therefore be counted from the
/// FILE's events, never from the engine's authored view, or every refused
/// cell silently leaves the denominator and coverage inflates.
#[test]
fn file_cached_pin_is_a_false_green_for_a_naive_value_comparator() {
    let parts_dir = false_green_parts_dir();
    assert_file_truth_of_c1(&parts_dir);

    let (mut document, outcome) = open_through_host_command(&parts_dir, "false_green.xlsx");
    let HostCommandOutcome::Opened {
        formulas_bound,
        recalc_path,
        ..
    } = &outcome
    else {
        panic!("expected Opened, got {outcome:?}");
    };
    assert_eq!(
        *formulas_bound, 1,
        "the outcome echoes B1 as the one bound formula"
    );
    assert_eq!(
        *recalc_path,
        LoadRecalcPath::Automatic,
        "calcMode=\"auto\" took the open-recalc path"
    );
    let session = workbook(&mut document);
    assert_load_report_names_c1_rejected(session);
    assert_eq!(
        session.load_report().unwrap().engine_recalcs_at_load,
        1,
        "an Automatic-mode load runs exactly one engine pass (the open-recalc)"
    );
    let sheet = only_sheet(session);

    // After the open-recalc: A1/B1 engine-Calculated under one tick; C1 the
    // file's cache, FileCached. The false green, stage 1.
    let cells = published_cells("after open", session, sheet);
    assert_eq!(cells.len(), 3, "A1, B1, C1 are published: {cells:#?}");
    let a1 = published_cell(&cells, 1, 1);
    let b1 = published_cell(&cells, 1, 2);
    assert_eq!(a1.value, CalcValue::number(7.0));
    assert_eq!(
        b1.value,
        CalcValue::number(21.0),
        "B1 = A1*3 = 21 (control)"
    );
    let open_tick = calculated_tick("B1 after open", b1);
    assert_eq!(
        calculated_tick("A1 after open", a1),
        open_tick,
        "the open-recalc publishes A1 and B1 under one tick"
    );
    assert_c1_is_the_file_cached_pin("after open", &cells);

    // What a host sees through the skin-IR mirror: the same fact, projected.
    assert_eq!(
        session.grid_cell_value(sheet, 1, 3).unwrap(),
        Some(name_error()),
        "the host-level value readout is the file's #NAME?"
    );
    assert_eq!(
        session.grid_cell_provenance(sheet, 1, 3).unwrap(),
        Some(ValueProvenanceProjection::FileCached),
        "the host-level provenance readout says FileCached"
    );
    assert!(
        matches!(
            session.grid_cell_provenance(sheet, 1, 2).unwrap(),
            Some(ValueProvenanceProjection::Calculated { tick_id }) if tick_id == open_tick
        ),
        "B1's host-level provenance is the open-recalc's Calculated tick"
    );

    // recalculate_workbook (F9) on an Automatic workbook with nothing dirty
    // is a genuine no-op — and C1 is still the pin, unchanged.
    let outcome = session.recalculate().unwrap();
    println!("W012 recalculate after open: {outcome:?}");
    assert!(
        !outcome.drained_any(),
        "Automatic mode: the open-recalc already drained; F9 has nothing to do"
    );
    let cells = published_cells("after F9", session, sheet);
    assert_c1_is_the_file_cached_pin("after F9", &cells);
    assert_eq!(
        calculated_tick("B1 after F9", published_cell(&cells, 1, 2)),
        open_tick,
        "no new tick was minted"
    );

    // A real edit: A1 -> 10 recalculates B1 to 30 under a fresh tick (the
    // control proves the engine really evaluated this workbook) — and C1 is
    // STILL the file's #NAME?, FileCached. The pin is persistent.
    let entry = session
        .enter_grid_cell(sheet, 1, 1, "10")
        .expect("A1 -> 10 is accepted");
    assert!(
        matches!(entry, GridCellEntryOutcome::Literal { .. }),
        "'10' takes the engine's literal branch, got {entry:?}"
    );
    let cells = published_cells("after A1 = 10", session, sheet);
    let b1 = published_cell(&cells, 1, 2);
    assert_eq!(
        b1.value,
        CalcValue::number(30.0),
        "B1 = A1*3 = 30: the edit really recalculated the loaded workbook"
    );
    let edit_tick = calculated_tick("B1 after edit", b1);
    assert_ne!(edit_tick, open_tick, "the edit minted a fresh tick");
    assert_c1_is_the_file_cached_pin("after A1 = 10", &cells);

    // And after a second explicit recalculate_workbook: still the pin.
    let outcome = session.recalculate().unwrap();
    println!("W012 recalculate after edit: {outcome:?}");
    assert!(
        !outcome.drained_any(),
        "Automatic mode: the edit itself recalculated; F9 has nothing to do"
    );
    let cells = published_cells("after second F9", session, sheet);
    assert_c1_is_the_file_cached_pin("after second F9", &cells);
    assert_eq!(
        calculated_tick("B1 after second F9", published_cell(&cells, 1, 2)),
        edit_tick
    );

    // OBSERVED: the authored projection does not know C1 is a formula. The
    // file says `<f>Is?Valid*2</f>`; the engine's authored view says Empty. A
    // formula-cell census taken from this view would not count C1 at all.
    let authored = session.grid_authored_cells(sheet, 1, 1, 1, 3).unwrap();
    let c1_authored = authored
        .iter()
        .find(|cell| cell.row == 1 && cell.col == 3)
        .expect("C1 is in the requested window");
    println!("W012 authored C1: {c1_authored:?}");
    assert_eq!(
        (c1_authored.kind, c1_authored.source_text.as_deref()),
        (GridAuthoredKindProjection::Empty, None),
        "OBSERVED (dtc-5zet.6): a rejected formula's authored text is retained only in \
         the BindDegradation row; the authored view reads Empty. The W1 coverage denominator \
         must come from the file's events, not from this view. Flip this assert when the \
         engine retains rejected text in the authored record."
    );
    let b1_authored = authored
        .iter()
        .find(|cell| cell.row == 1 && cell.col == 2)
        .expect("B1 is in the requested window");
    assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
    assert_eq!(b1_authored.source_text.as_deref(), Some("=A1*3"));
}

/// The second half of the trap, on the Manual twin: under
/// `calcMode="manual"` (62 of the corpus's 174 Excel-authored workbooks) the
/// load runs ZERO engine passes and EVERY published formula cell is the
/// file's cache, `FileCached` — `B1 = 21` and `C1 = #NAME?` alike — until
/// `recalculate_workbook`. A value-only comparator would score the whole
/// workbook green without one cell having been evaluated. After F9 the
/// bound `B1` becomes engine-`Calculated` under the drain's tick, and the
/// rejected `C1` is STILL `FileCached`: the drain's counter says two cells
/// (`A1`, `B1`) were evaluated on a three-cell sheet — never the pin.
#[test]
fn manual_mode_renders_every_formula_file_cached_until_f9_and_the_rejected_cell_stays_pinned() {
    let parts_dir = false_green_manual_parts_dir();
    assert_file_truth_of_c1(&parts_dir);

    let (mut document, outcome) = open_through_host_command(&parts_dir, "false_green_manual.xlsx");
    let HostCommandOutcome::Opened { recalc_path, .. } = &outcome else {
        panic!("expected Opened, got {outcome:?}");
    };
    assert_eq!(
        *recalc_path,
        LoadRecalcPath::Manual,
        "calcMode=\"manual\" takes the engine's render-from-cache path"
    );
    let session = workbook(&mut document);
    assert_load_report_names_c1_rejected(session);
    assert_eq!(
        session.load_report().unwrap().engine_recalcs_at_load,
        0,
        "a Manual-mode load runs ZERO engine passes (the perf-counter proof)"
    );
    let sheet = only_sheet(session);
    let calc = session.workbook_calc_projection(None).unwrap();
    assert_eq!(calc.mode, CalcModeProjection::Manual, "the file's mode");
    assert!(
        calc.sheets[0].dirty,
        "a Manual load seeds the sheet dirty — the first F9 is owed"
    );

    // Before F9: every published cell is FileCached, and the published set
    // is exactly the two formula caches. A1 has no publication yet (the
    // Manual load publishes formula caches only; dtc-j7n8.24's observed
    // fact, unchanged here).
    let cells = published_cells("manual, before F9", session, sheet);
    let mut published: Vec<(u32, u32)> = cells
        .iter()
        .map(|cell| (cell.address.row, cell.address.col))
        .collect();
    published.sort_unstable();
    assert_eq!(
        published,
        [(1, 2), (1, 3)],
        "before F9 exactly B1 and C1 are published — both formula caches"
    );
    for cell in &cells {
        assert_eq!(
            cell.provenance,
            PublishedValueProvenance::FileCached,
            "before F9 every published cell is the file's cache: R{}C{} = {:?}",
            cell.address.row,
            cell.address.col,
            cell.value
        );
    }
    assert_eq!(
        published_cell(&cells, 1, 2).value,
        CalcValue::number(21.0),
        "B1 renders the file's cached 21 — a value-only comparator would score it a match \
         with Excel, and it was never evaluated either"
    );
    assert_c1_is_the_file_cached_pin("manual, before F9", &cells);

    // F9: a genuine drain under one tick. B1 flips to Calculated; C1 does not.
    let outcome = session.recalculate().unwrap();
    println!("W012 manual recalculate: {outcome:?}");
    let tick = outcome
        .tick_id
        .expect("the first F9 on a Manual load drains the seeded sheet");
    assert!(outcome.drained_any());
    assert_eq!(
        outcome.total_cells_evaluated(),
        2,
        "the drain evaluated 2 of the sheet's 3 cells — A1 and B1, which flip to Calculated \
         below; the pinned C1 is not among them"
    );
    let cells = published_cells("manual, after F9", session, sheet);
    assert_eq!(cells.len(), 3, "A1 is published by its own recalc now");
    assert_eq!(published_cell(&cells, 1, 1).value, CalcValue::number(7.0));
    let b1 = published_cell(&cells, 1, 2);
    assert_eq!(b1.value, CalcValue::number(21.0));
    assert_eq!(
        calculated_tick("B1 after F9", b1),
        tick,
        "B1 is now engine-Calculated under the drain's tick — scorable"
    );
    assert_c1_is_the_file_cached_pin("manual, after F9", &cells);
    assert!(
        !session.workbook_calc_projection(None).unwrap().sheets[0].dirty,
        "the drain cleared the sheet's dirty flag"
    );

    // Manual edit + F9: the control moves (B1 = 30), the pin does not.
    let entry = session
        .enter_grid_cell(sheet, 1, 1, "10")
        .expect("A1 -> 10 on the Manual twin is accepted");
    assert!(matches!(entry, GridCellEntryOutcome::Literal { .. }));
    let cells = published_cells("manual, after A1 = 10", session, sheet);
    assert!(
        matches!(
            published_cell(&cells, 1, 2).provenance,
            PublishedValueProvenance::Stale { since_tick_id } if since_tick_id == tick
        ),
        "Manual suppresses the recalc: B1 is honestly Stale behind the undrained edit"
    );
    assert_c1_is_the_file_cached_pin("manual, after A1 = 10", &cells);
    let outcome = session.recalculate().unwrap();
    println!("W012 manual recalculate after edit: {outcome:?}");
    let second_tick = outcome.tick_id.expect("the edit is drained");
    assert_ne!(second_tick, tick);
    let cells = published_cells("manual, after second F9", session, sheet);
    let b1 = published_cell(&cells, 1, 2);
    assert_eq!(b1.value, CalcValue::number(30.0), "B1 = 10*3 = 30");
    assert_eq!(calculated_tick("B1 after second F9", b1), second_tick);
    assert_c1_is_the_file_cached_pin("manual, after second F9", &cells);
}

/// The Manual twin is the auto fixture with exactly one difference —
/// `calcMode="manual"` — pinned at the OxDoc event-stream level (the W011
/// twin discipline): with the header's calc mode swapped back the streams
/// are equal, so the two fixtures can never drift apart in anything but the
/// mode.
#[test]
fn w012_manual_twin_differs_from_the_auto_fixture_only_in_calc_mode() {
    fn header_calc_mode(events: &[DocumentEvent]) -> CalcMode {
        let modes: Vec<CalcMode> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::WorkbookHeader(header) => Some(header.calc_mode),
                _ => None,
            })
            .collect();
        assert_eq!(modes.len(), 1, "exactly one WorkbookHeader in {events:#?}");
        modes[0]
    }

    let auto = open_xlsx_raw(&fixture_bytes(&false_green_parts_dir()));
    let manual = open_xlsx_raw(&fixture_bytes(&false_green_manual_parts_dir()));
    let auto_events = auto.source_context.events();
    let manual_events = manual.source_context.events();
    assert_eq!(header_calc_mode(auto_events), CalcMode::Automatic);
    assert_eq!(header_calc_mode(manual_events), CalcMode::Manual);
    let manual_as_auto: Vec<DocumentEvent> = manual_events
        .iter()
        .cloned()
        .map(|event| match event {
            DocumentEvent::WorkbookHeader(mut header) => {
                header.calc_mode = CalcMode::Automatic;
                DocumentEvent::WorkbookHeader(header)
            }
            other => other,
        })
        .collect();
    assert_eq!(
        manual_as_auto.as_slice(),
        auto_events,
        "with the header's calc mode swapped back, the Manual twin's event stream IS the auto \
         fixture's: same three cells, same topology, nothing else"
    );
    assert_eq!(manual.load_ledger, auto.load_ledger);
}

//! The bead's acceptance tests, on the committed fixtures:
//!
//! - (a) the W012 false-green fixture (dtc-5zet.2): the rejected cell is
//!   `not_evaluated:bind_degraded`, the control cell `match` — the test that
//!   proves the ladder is provenance-first;
//! - (b) `fixtures/w011/a1_times_three.xlsx`: one formula cell, Calculated,
//!   Match, coverage 1.0;
//! - (c) the Manual twin `fixtures/w011/a1_times_three_manual.xlsx`:
//!   coverage 1.0 because the recalc was forced, and 0 with
//!   `not_evaluated:manual_mode` under `--no-force-recalc`;
//! - (d) a whole `run_diff` over a temp corpus round-trips through serde
//!   with `unclassified == 0` and the arithmetic identities intact;
//! - (e) W012-1.5 (dtc-5zet.12), the reference twin fails loudly: every
//!   fixture runs under `DualValidated` with a clean twin (the control); a
//!   leaf-arithmetic mismatch injected at the engine→instrument seam scores
//!   `mismatch:ReferenceTwin`, lands in the workbook's `reference_twin`
//!   rows and the totals, and makes `--fail-on-twin` exit 3; a twin row on
//!   a pinned cell keeps rung 1's verdict and is still counted; a
//!   pre-W012-1.5 artifact still deserializes;
//! - (f) W012-1.7 (dtc-5zet.14), escalation beside coverage: an Automatic
//!   fixture's forced F9 is a no-op (0 recalcs, rates undefined, the
//!   open-recalc reported apart); a Manual fixture's forced F9 is one drain
//!   = one contract rebuild (lower bound 1.0, `escalation_rate` null: no
//!   lane outcome is exposed); the corpus fold and its identities hold on
//!   the artifact; a pre-W012-1.7 artifact still deserializes.
//!
//! Fixture locations are `CARGO_MANIFEST_DIR` arithmetic to the repo root;
//! the parts-only W012 fixtures are zipped in memory through the dev-only
//! `oxdoc_conformance` crate (the committed-parts pattern; no zip crate).

use std::path::{Path, PathBuf};

use crate::diff::{
    DIFF_VALIDATION_MODE, DiffOptions, EXIT_TWIN_FAILED, EngineSheetView, assemble,
    diff_workbook_bytes, engine_sheet_views, exit_code, run_diff, score_session_views, sha256_hex,
};
use crate::report::{ARTIFACT_FILE_NAME, DivergenceReport, OpenStatus, SCHEMA, WorkbookReport};
use dnacalc_host_core::{GridEngineValidationMode, WorkbookSession, WorkbookSessionError};
use oxcalc_core::consumer::WorkbookRecalcOutcome;
use oxcalc_core::grid::machine::GridDifferentialMismatch;
use oxfunc_core::value::CalcValue;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_fixture(rel: &str) -> Vec<u8> {
    let path = repo_root().join(rel);
    std::fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn zip_parts(rel: &str) -> Vec<u8> {
    let parts = repo_root().join(rel);
    oxdoc_conformance::read_fixture_parts_as_xlsx(&parts)
        .unwrap_or_else(|err| panic!("zip {}: {err}", parts.display()))
}

fn false_green_bytes() -> Vec<u8> {
    zip_parts("fixtures/w012/false_green/parts")
}

fn false_green_manual_bytes() -> Vec<u8> {
    zip_parts("fixtures/w012/false_green_manual/parts")
}

fn a1_times_three_bytes() -> Vec<u8> {
    read_fixture("fixtures/w011/a1_times_three.xlsx")
}

fn a1_times_three_manual_bytes() -> Vec<u8> {
    read_fixture("fixtures/w011/a1_times_three_manual.xlsx")
}

fn verdict_of<'a>(report: &'a WorkbookReport, address: &str) -> &'a crate::report::CellVerdict {
    report
        .verdicts
        .iter()
        .find(|row| row.address == address)
        .unwrap_or_else(|| panic!("no verdict row for {address} in {:#?}", report.verdicts))
}

/// (a) THE ladder test. `C1 = =Is?Valid*2` is OxFml-rejected (a `?` inside
/// a name is Excel-legal, [MS-XLSX] §2.2.2; re-cut by dtc-5zet.20 after OxFml
/// fml-kt8.2 learned the fixture's first text, `=Größe*2`); OxCalc publishes
/// the file's cached `#NAME?` as a `FileCached` pin — equal to Excel's
/// cache. A value-only comparator scores it a match; this runner scores it
/// `not_evaluated:bind_degraded`, and the control `B1 = =A1*3` `match` under
/// a `Calculated` tick. Coverage is 1/2, not 2/2.
#[test]
fn false_green_rejected_cell_is_not_evaluated_bind_degraded_and_control_matches() {
    let report = diff_workbook_bytes(
        &false_green_bytes(),
        "w012/false_green.xlsx",
        &DiffOptions::default(),
    );
    println!("{report:#?}");
    assert_eq!(report.open, OpenStatus::Opened);
    assert_eq!(report.calc_mode.as_deref(), Some("Automatic"));
    assert_eq!(report.sheets, 1);
    assert_eq!(report.cells, 3, "A1, B1, C1");
    assert_eq!(
        report.formula_cells, 2,
        "the denominator counts the rejected C1 (from the file's events), not just the bound B1"
    );
    assert_eq!(report.engine_calculated, 1, "only B1 was engine-evaluated");
    assert_eq!(report.coverage, Some(0.5));
    assert_eq!(report.matched, 1);
    assert!(report.mismatch.is_empty(), "{:?}", report.mismatch);
    assert!(report.excluded.is_empty(), "{:?}", report.excluded);
    assert_eq!(report.unclassified, 0);
    assert_eq!(
        report.not_evaluated,
        [("bind_degraded".to_string(), 1)].into_iter().collect()
    );

    let c1 = verdict_of(&report, "C1");
    assert_eq!(c1.verdict, "not_evaluated:bind_degraded");
    assert_eq!(c1.provenance.as_deref(), Some("FileCached"));
    assert_eq!(c1.tick_id, None, "a pinned cell carries no Calculated tick");
    assert_eq!(c1.formula.as_deref(), Some("Is?Valid*2"));
    assert_eq!(
        (c1.engine.as_deref(), c1.excel.as_deref()),
        (Some("#NAME?"), Some("#NAME?")),
        "the values are EQUAL — that is the false green — and the verdict is still not a match"
    );

    let b1 = verdict_of(&report, "B1");
    assert_eq!(b1.verdict, "match");
    assert!(
        b1.provenance
            .as_deref()
            .is_some_and(|p| p.starts_with("Calculated")),
        "{:?}",
        b1.provenance
    );
    assert!(b1.tick_id.is_some(), "a match is audited back to its tick");
    assert_eq!(
        (b1.engine.as_deref(), b1.excel.as_deref()),
        (Some("21.0"), Some("21.0"))
    );

    let load = report.load_report.as_ref().unwrap();
    assert_eq!(load.formulas_bound, 1);
    assert_eq!(load.bind_degradations, 1);
    assert_eq!(load.recalc_path, "Automatic");
    let recalc = report.recalc.as_ref().unwrap();
    assert_eq!(
        recalc.tick_id, None,
        "an Automatic workbook is drained by its open-recalc; the forced F9 is a no-op"
    );
    assert_eq!(recalc.error, None);
}

/// (a, Manual twin) The same fixture under `calcMode="manual"`, forced: the
/// forced recalc mints a tick and evaluates `B1` (match), and the rejected
/// `C1` is STILL the pin — `bind_degraded`, not `manual_mode`, because the
/// more specific reason wins.
#[test]
fn false_green_manual_twin_forced_scores_the_control_and_keeps_the_pin_bind_degraded() {
    let report = diff_workbook_bytes(
        &false_green_manual_bytes(),
        "w012/false_green_manual.xlsx",
        &DiffOptions::default(),
    );
    println!("{report:#?}");
    assert_eq!(report.calc_mode.as_deref(), Some("Manual"));
    assert_eq!(report.load_report.as_ref().unwrap().recalc_path, "Manual");
    let recalc = report.recalc.as_ref().unwrap();
    assert!(
        recalc.tick_id.is_some(),
        "the forced F9 drained the seeded sheet"
    );
    assert_eq!(recalc.cells_evaluated, 2, "A1 and B1; never the pinned C1");
    assert_eq!(report.formula_cells, 2);
    assert_eq!(report.engine_calculated, 1);
    assert_eq!(report.coverage, Some(0.5));
    assert_eq!(verdict_of(&report, "B1").verdict, "match");
    assert_eq!(verdict_of(&report, "B1").tick_id, recalc.tick_id);
    assert_eq!(
        verdict_of(&report, "C1").verdict,
        "not_evaluated:bind_degraded"
    );
}

/// (b) The W011 fixture: one formula cell, Calculated, Match, coverage 1.0.
#[test]
fn a1_times_three_is_one_calculated_match_with_full_coverage() {
    let report = diff_workbook_bytes(
        &a1_times_three_bytes(),
        "w011/a1_times_three.xlsx",
        &DiffOptions::default(),
    );
    println!("{report:#?}");
    assert_eq!(report.open, OpenStatus::Opened);
    assert_eq!(report.formula_cells, 1);
    assert_eq!(report.engine_calculated, 1);
    assert_eq!(report.coverage, Some(1.0));
    assert_eq!(report.matched, 1);
    assert!(report.not_evaluated.is_empty());
    assert!(report.mismatch.is_empty());
    assert!(report.excluded.is_empty());
    let b1 = verdict_of(&report, "B1");
    assert_eq!(b1.verdict, "match");
    assert_eq!(b1.formula.as_deref(), Some("A1*3"));
    assert!(b1.tick_id.is_some());

    // (f) Escalation beside coverage: the Automatic load ran the
    // open-recalc (1 sheet), the forced F9 found nothing to drain, so the
    // workbook issued NO recalc after the open — both rates undefined,
    // nothing unobserved because nothing ran.
    let escalation = report
        .escalation
        .expect("an opened workbook carries its accounting");
    assert_eq!(
        (escalation.open_recalcs, escalation.open_recalc_sheet_drains),
        (1, 1)
    );
    assert_eq!(escalation.f9_noops, 1);
    assert_eq!(escalation.f9_drains, 0);
    assert_eq!(escalation.recalcs, 0);
    assert_eq!(escalation.contract_rebuilds, 0);
    assert_eq!(escalation.escalation_rate, None);
    assert_eq!(escalation.escalation_rate_lower_bound, None);
    assert_eq!(escalation.lane_outcomes.unobserved, 0);
    escalation.check_identities("a1_times_three").unwrap();
}

/// (c) The Manual-mode trap. Forced (the product path): coverage 1.0.
/// Unforced (the hidden `--no-force-recalc`): the load ran zero engine
/// passes, `B1` renders the file's cached `21` `FileCached` — equal to
/// Excel — and the verdict is `not_evaluated:manual_mode`, coverage 0.
#[test]
fn manual_mode_fixture_has_coverage_only_because_the_recalc_is_forced() {
    let forced = diff_workbook_bytes(
        &a1_times_three_manual_bytes(),
        "w011/a1_times_three_manual.xlsx",
        &DiffOptions {
            force_recalc: true,
            ..DiffOptions::default()
        },
    );
    println!("forced: {forced:#?}");
    assert_eq!(forced.calc_mode.as_deref(), Some("Manual"));
    assert_eq!(forced.formula_cells, 1);
    assert_eq!(forced.engine_calculated, 1);
    assert_eq!(forced.coverage, Some(1.0));
    assert_eq!(verdict_of(&forced, "B1").verdict, "match");
    assert!(forced.recalc.as_ref().unwrap().tick_id.is_some());
    assert!(forced.forced_recalc);
    // (f) The forced F9 on a Manual load is ONE recalc transaction, and by
    // OxCalc's `recalculate_workbook` contract every drained sheet is
    // rebuilt from authored input and recalculated in full: one contract
    // rebuild over one recalc — lower bound 1.0. The exact rate stays
    // null: the engine reported no lane outcome (1 of 1 unobserved). No
    // open-recalc: a Manual load runs zero engine passes.
    let escalation = forced
        .escalation
        .expect("an opened workbook carries its accounting");
    assert_eq!(
        (escalation.open_recalcs, escalation.open_recalc_sheet_drains),
        (0, 0)
    );
    assert_eq!((escalation.f9_drains, escalation.f9_sheet_drains), (1, 1));
    assert_eq!(
        escalation.f9_cells_evaluated,
        forced.recalc.as_ref().unwrap().cells_evaluated,
        "the accounting retains the outcome's own seeded-pass counter"
    );
    assert_eq!(escalation.recalcs, 1);
    assert_eq!(escalation.contract_rebuilds, 1);
    assert_eq!(escalation.escalation_rate_lower_bound, Some(1.0));
    assert_eq!(escalation.escalation_rate, None);
    assert!(!escalation.lane_outcomes_exposed);
    assert_eq!(escalation.lane_outcomes.unobserved, 1);
    escalation
        .check_identities("a1_times_three_manual forced")
        .unwrap();

    let unforced = diff_workbook_bytes(
        &a1_times_three_manual_bytes(),
        "w011/a1_times_three_manual.xlsx",
        &DiffOptions {
            force_recalc: false,
            ..DiffOptions::default()
        },
    );
    println!("unforced: {unforced:#?}");
    assert!(!unforced.forced_recalc);
    assert_eq!(unforced.recalc, None, "no recalc was invoked");
    assert_eq!(
        unforced
            .load_report
            .as_ref()
            .unwrap()
            .engine_recalcs_at_load,
        0,
        "a Manual load runs zero engine passes"
    );
    assert_eq!(unforced.formula_cells, 1);
    assert_eq!(unforced.engine_calculated, 0);
    assert_eq!(unforced.coverage, Some(0.0));
    assert_eq!(unforced.matched, 0);
    assert_eq!(
        unforced.not_evaluated,
        [("manual_mode".to_string(), 1)].into_iter().collect()
    );
    let b1 = verdict_of(&unforced, "B1");
    assert_eq!(b1.verdict, "not_evaluated:manual_mode");
    assert_eq!(b1.provenance.as_deref(), Some("FileCached"));
    assert_eq!(
        (b1.engine.as_deref(), b1.excel.as_deref()),
        (Some("21.0"), Some("21.0")),
        "equal values, never a match: the engine did not compute this"
    );
    let escalation = unforced.escalation.unwrap();
    assert_eq!(
        (
            escalation.open_recalcs,
            escalation.f9_drains,
            escalation.f9_noops,
            escalation.recalcs
        ),
        (0, 0, 0, 0),
        "unforced: no engine pass at all, nothing to account"
    );
}

/// Bytes OxDoc cannot open at all are an `unreadable` row: no denominator
/// is known, and the row says so instead of vanishing.
#[test]
fn unreadable_bytes_are_a_typed_row_with_no_denominator() {
    let report = diff_workbook_bytes(b"not a zip", "junk.xlsx", &DiffOptions::default());
    println!("{report:#?}");
    assert!(
        matches!(&report.open, OpenStatus::Unreadable { kind, .. } if kind.starts_with("xlsx:")),
        "{:?}",
        report.open
    );
    assert_eq!(report.formula_cells, 0);
    assert_eq!(report.coverage, None);
    assert!(report.verdicts.is_empty());
}

/// (d) A whole run over a temp corpus: the artifact is written, reads back
/// as the same report, `unclassified == 0`, the identities hold, the
/// worst-covered list is ordered, and the corpus sha256 set is retained.
#[test]
fn run_diff_writes_an_artifact_that_round_trips_with_zero_unclassified() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "dnacalc-conform-test-{}-{stamp}",
        std::process::id()
    ));
    let corpus = root.join("corpus");
    let nested = corpus.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(corpus.join("a1_times_three.xlsx"), a1_times_three_bytes()).unwrap();
    std::fs::write(
        corpus.join("a1_times_three_manual.xlsx"),
        a1_times_three_manual_bytes(),
    )
    .unwrap();
    std::fs::write(nested.join("false_green.xlsx"), false_green_bytes()).unwrap();
    std::fs::write(nested.join("~$false_green.xlsx"), b"lock file").unwrap();
    std::fs::write(nested.join("notes.txt"), b"not a workbook").unwrap();
    let out = root.join("out");

    let report = run_diff(&corpus, &out, &DiffOptions::default()).unwrap();

    let artifact = out.join(ARTIFACT_FILE_NAME);
    let json = std::fs::read_to_string(&artifact).unwrap();
    let read_back: DivergenceReport = serde_json::from_str(&json).unwrap();
    assert_eq!(read_back, report, "the artifact round-trips through serde");
    assert!(
        json.trim_start()
            .starts_with(&format!("{{\n  \"schema\": \"{SCHEMA}\",\n  \"coverage\":")),
        "coverage is the first number in the file:\n{}",
        &json[..json.len().min(200)]
    );

    assert_eq!(
        report.totals.workbooks, 3,
        "the lock file and the .txt are not workbooks"
    );
    assert_eq!(report.totals.opened, 3);
    assert_eq!(report.totals.formula_cells, 4, "1 + 1 + 2");
    assert_eq!(report.totals.engine_calculated, 3, "1 + 1 + 1");
    assert_eq!(report.totals.matched, 3);
    assert_eq!(report.totals.not_evaluated_total, 1);
    assert_eq!(report.totals.unclassified, 0);
    assert_eq!(report.coverage, 0.75);
    assert_eq!(report.totals.coverage, 0.75);
    assert_eq!(
        report.totals.engine_calculated,
        report.totals.matched + report.totals.mismatch_total + report.totals.excluded_total
    );
    assert_eq!(
        report.totals.formula_cells,
        report.totals.engine_calculated + report.totals.not_evaluated_total
    );

    let paths: Vec<&str> = report.workbooks.iter().map(|w| w.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "a1_times_three.xlsx",
            "a1_times_three_manual.xlsx",
            "nested/false_green.xlsx"
        ],
        "sorted, relative, forward slashes"
    );
    assert_eq!(report.worst_covered.len(), 3);
    assert_eq!(report.worst_covered[0].path, "nested/false_green.xlsx");
    assert_eq!(report.worst_covered[0].coverage, 0.5);
    assert_eq!(report.corpus.workbooks, 3);
    assert_eq!(report.corpus.sha256_set.len(), 3);
    let mut sorted = report.corpus.sha256_set.clone();
    sorted.sort();
    assert_eq!(sorted, report.corpus.sha256_set);
    assert_eq!(report.corpus.set_sha256.len(), 64);
    assert!(report.run.forced_recalc);

    // (f) The corpus fold: two Automatic loads (open-recalc each, forced
    // F9 a no-op) and one Manual load (no open-recalc, forced F9 drains
    // one sheet = one contract rebuild). One recalc corpus-wide, at the
    // lower bound 1.0 and with its lane outcome unobserved; the identities
    // `assemble` checked hold on the read-back artifact too.
    let e = &report.totals.escalation;
    assert_eq!((e.open_recalcs, e.open_recalc_sheet_drains), (2, 2));
    assert_eq!((e.f9_drains, e.f9_noops, e.f9_sheet_drains), (1, 2, 1));
    assert_eq!((e.recalcs, e.contract_rebuilds), (1, 1));
    assert_eq!(e.escalation_rate, None);
    assert_eq!(e.escalation_rate_lower_bound, Some(1.0));
    assert_eq!(e.lane_outcomes.unobserved, 1);
    assert!(!e.lane_outcomes_exposed);
    read_back
        .totals
        .escalation
        .check_identities("read back")
        .unwrap();
    assert_eq!(
        report
            .workbooks
            .iter()
            .map(|w| w.escalation.unwrap().recalcs)
            .collect::<Vec<_>>(),
        [0, 1, 0],
        "only the Manual load's forced F9 was a recalc"
    );

    let mut summary = Vec::new();
    crate::diff::write_summary(&report, &artifact, &mut summary).unwrap();
    let summary = String::from_utf8(summary).unwrap();
    println!("{summary}");
    assert!(summary.contains("COVERAGE 0.7500  = engine_calculated 3 / formula_cells 4"));
    assert!(
        summary.contains(
            "ESCALATION escalation_rate UNOBSERVABLE (lane outcomes engine-reported 0 / 1;"
        ),
        "{summary}"
    );
    assert!(
        summary.contains("lower bound 1.0000 = contract_rebuilds 1 / recalcs 1"),
        "{summary}"
    );
    assert!(
        summary.contains("1 of 3 opened workbooks issued a recalc after the open"),
        "{summary}"
    );
    assert!(summary.contains("not_evaluated 1: bind_degraded=1"));
    assert!(summary.contains("unclassified 0"));

    std::fs::remove_dir_all(&root).unwrap();
}

/// (e, control) Every committed fixture runs under `DualValidated` — the
/// reference lane on every recalc — and the twin agrees with the optimized
/// lane on all of them: `reference_twin` is present, says so, and is empty.
/// This is what `diff --fail-on-twin` over the Foundation corpus exiting 0
/// rests on, pinned on fixtures that never leave the repo.
#[test]
fn every_fixture_runs_dual_validated_with_a_clean_reference_twin() {
    assert_eq!(
        DIFF_VALIDATION_MODE,
        GridEngineValidationMode::DualValidated
    );
    for (bytes, name) in [
        (a1_times_three_bytes(), "w011/a1_times_three.xlsx"),
        (
            a1_times_three_manual_bytes(),
            "w011/a1_times_three_manual.xlsx",
        ),
        (
            read_fixture("fixtures/w011/cross_sheet.xlsx"),
            "w011/cross_sheet.xlsx",
        ),
        (
            read_fixture("fixtures/w011/named_input.xlsx"),
            "w011/named_input.xlsx",
        ),
        (false_green_bytes(), "w012/false_green.xlsx"),
        (false_green_manual_bytes(), "w012/false_green_manual.xlsx"),
    ] {
        let report = diff_workbook_bytes(&bytes, name, &DiffOptions::default());
        assert_eq!(report.open, OpenStatus::Opened, "{name}");
        let twin = report
            .reference_twin
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: an opened workbook carries its twin summary"));
        assert_eq!(twin.validation_mode, "DualValidated", "{name}");
        assert!(twin.checked_every_recalc, "{name}");
        assert!(
            twin.mismatches.is_empty(),
            "{name}: the reference twin disputes {:?}",
            twin.mismatches
        );
        assert!(
            !report.mismatch.contains_key("ReferenceTwin"),
            "{name}: {:?}",
            report.mismatch
        );
        assert!(
            report
                .verdicts
                .iter()
                .all(|row| row.twin_reference.is_none()),
            "{name}"
        );
    }
}

/// Open a fixture the way the runner does, force the recalc, and hand back
/// the session, its recalc outcome and its sheet views — the seam the
/// injection tests below write into.
type OpenedFixture = (
    WorkbookSession,
    Option<Result<WorkbookRecalcOutcome, WorkbookSessionError>>,
    Vec<EngineSheetView>,
);

fn open_for_injection(bytes: &[u8], name: &str) -> OpenedFixture {
    let mut session = WorkbookSession::open_xlsx_bytes_with_validation(
        format!("conform-test:{name}"),
        bytes,
        Some(name.to_string()),
        DIFF_VALIDATION_MODE,
    )
    .unwrap_or_else(|err| panic!("{name} opens on the product path: {err:?}"));
    let recalc = Some(session.recalculate());
    let views = engine_sheet_views(&session).expect("sheets() succeeds");
    (session, recalc, views)
}

fn view_cell(
    views: &[EngineSheetView],
    row: u32,
    col: u32,
) -> &oxcalc_core::consumer::OxCalcTreeGridCellReadout {
    views[0]
        .view
        .as_ref()
        .unwrap()
        .cells
        .iter()
        .find(|cell| cell.address.row == row && cell.address.col == col)
        .unwrap_or_else(|| panic!("R{row}C{col} is published"))
}

/// (e) THE injection test, host side. The W011 fixture (`A1 = 7`, `B1 =
/// =A1*3` cached `21`) is opened under `DualValidated`; the control scores
/// `match`. Then a deliberately wrong REFERENCE value for `B1` is injected
/// into the engine's view at the seam the runner reads
/// (`differential_mismatches`), exactly the shape OxCalc's differential
/// would report if the optimized lane had published a wrong 21 against a
/// reference 22: the verdict flips to `mismatch:ReferenceTwin`, no Excel
/// comparison is made, the row carries both lane values, the workbook's
/// `reference_twin` lists it as a formula cell, the totals count it, and
/// `--fail-on-twin` turns it into exit 3 — with the artifact still written.
/// A second injected row at `A1` (a literal — no formula cell in the file)
/// is counted at the workbook level and nowhere else, so nothing the engine
/// reports is dropped.
///
/// What this does NOT prove: that OxCalc's differential catches a wrong
/// value at the three blind sites the bead names (the >10k mark-all
/// fall-through, the cross-sheet escalation, a FileCached pin). Those live
/// inside the engine, have no test-only injection seam, and are the subject
/// of `docs/handovers/HANDOVER_OXCALC_twin_injection_hooks.md`.
#[test]
fn injected_leaf_arithmetic_twin_mismatch_scores_reference_twin_and_fails_on_twin() {
    let (session, recalc, mut views) =
        open_for_injection(&a1_times_three_bytes(), "w011/a1_times_three.xlsx");
    assert_eq!(views.len(), 1);
    assert!(
        views[0]
            .view
            .as_ref()
            .unwrap()
            .differential_mismatches
            .is_empty(),
        "control: the real twin agrees"
    );
    let b1 = view_cell(&views, 1, 2);
    assert_eq!(b1.value, CalcValue::number(21.0));
    let b1_address = b1.address.clone();
    let a1_address = view_cell(&views, 1, 1).address.clone();

    // Control: unmodified views score the fixture as (b) does.
    let control = score_session_views(
        &session,
        &recalc,
        Ok(views.clone()),
        "w011/a1_times_three.xlsx",
        sha256_hex(&a1_times_three_bytes()),
        &DiffOptions::default(),
    );
    assert_eq!(verdict_of(&control, "B1").verdict, "match");
    assert!(
        control
            .reference_twin
            .as_ref()
            .unwrap()
            .mismatches
            .is_empty()
    );

    // Injection at the seam.
    let view = views[0].view.as_mut().unwrap();
    view.differential_mismatches.push(GridDifferentialMismatch {
        address: b1_address,
        reference: CalcValue::number(22.0),
        optimized: CalcValue::number(21.0),
    });
    view.differential_mismatches.push(GridDifferentialMismatch {
        address: a1_address,
        reference: CalcValue::number(8.0),
        optimized: CalcValue::number(7.0),
    });
    let options = DiffOptions {
        fail_on_twin: true,
        ..DiffOptions::default()
    };
    let report = score_session_views(
        &session,
        &recalc,
        Ok(views),
        "w011/a1_times_three.xlsx",
        sha256_hex(&a1_times_three_bytes()),
        &options,
    );
    println!("{report:#?}");

    let b1 = verdict_of(&report, "B1");
    assert_eq!(b1.verdict, "mismatch:ReferenceTwin");
    assert_eq!(
        (
            b1.engine.as_deref(),
            b1.twin_reference.as_deref(),
            b1.excel.as_deref()
        ),
        (Some("21.0"), Some("22.0"), Some("21.0")),
        "the optimized value equals Excel's cache and is STILL not a match: the engine disagrees with itself"
    );
    assert!(b1.tick_id.is_some(), "the cell was engine-Calculated");
    assert_eq!(report.formula_cells, 1);
    assert_eq!(report.engine_calculated, 1);
    assert_eq!(report.matched, 0);
    assert_eq!(
        report.mismatch,
        [("ReferenceTwin".to_string(), 1)].into_iter().collect()
    );
    assert_eq!(report.unclassified, 0);

    let twin = report.reference_twin.as_ref().unwrap();
    assert_eq!(twin.validation_mode, "DualValidated");
    assert!(twin.checked_every_recalc);
    assert_eq!(twin.mismatches.len(), 2);
    let twin_b1 = twin
        .mismatches
        .iter()
        .find(|row| row.address == "B1")
        .unwrap();
    assert_eq!(twin_b1.sheet, "Sheet1");
    assert_eq!(
        (twin_b1.reference.as_str(), twin_b1.optimized.as_str()),
        ("22.0", "21.0")
    );
    assert!(!twin_b1.core_equal, "21 and 22 disagree on the core value");
    assert_eq!(
        (
            twin_b1.reference_rich.as_deref(),
            twin_b1.optimized_rich.as_deref()
        ),
        (None, None)
    );
    assert!(twin_b1.formula_cell);
    let twin_a1 = twin
        .mismatches
        .iter()
        .find(|row| row.address == "A1")
        .unwrap();
    assert!(
        !twin_a1.formula_cell,
        "A1 is a literal: no verdict row, but the engine's row is still reported"
    );

    // Corpus level: the totals carry both rows, the identities hold (the
    // A1 row is not a verdict), and the exit code is the requested failure.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let scratch = std::env::temp_dir().join(format!("dnacalc-conform-twin-{stamp}"));
    let assembled = assemble(vec![report], &scratch, &scratch, &options).unwrap();
    assert_eq!(assembled.totals.reference_twin_mismatches, 2);
    assert_eq!(assembled.totals.reference_twin_workbooks, 1);
    assert_eq!(assembled.totals.mismatch_total, 1);
    assert_eq!(assembled.totals.engine_calculated, 1);
    assert_eq!(assembled.totals.formula_cells, 1);
    assert_eq!(
        assembled.run.validation_mode.as_deref(),
        Some("DualValidated")
    );
    assert!(assembled.run.fail_on_twin);
    assert_eq!(exit_code(&assembled, &options), EXIT_TWIN_FAILED);
    assert_eq!(EXIT_TWIN_FAILED, 3);
    assert_eq!(
        exit_code(&assembled, &DiffOptions::default()),
        0,
        "without --fail-on-twin the mismatch is a result, not a failure"
    );
    let mut summary = Vec::new();
    crate::diff::write_summary(&assembled, &scratch.join(ARTIFACT_FILE_NAME), &mut summary)
        .unwrap();
    let summary = String::from_utf8(summary).unwrap();
    println!("{summary}");
    assert!(summary.contains(
        "reference_twin (DualValidated, every recalc): mismatches 2 in 1 workbooks  <-- --fail-on-twin: exit 3"
    ));
    assert!(summary.contains("Sheet1!B1  reference 22.0 vs optimized 21.0"));
    assert!(summary.contains(
        "Sheet1!A1  reference 8.0 vs optimized 7.0  (no formula cell in the file at this address)"
    ));
    assert!(summary.contains("mismatch 1: ReferenceTwin=1"));
    // And it survives the artifact.
    let json = serde_json::to_string_pretty(&assembled).unwrap();
    let read_back: DivergenceReport = serde_json::from_str(&json).unwrap();
    assert_eq!(read_back, assembled);
}

/// (e, ordering) A twin row on a cell the ladder stops at rung 1 — the
/// false-green fixture's pinned `C1` — does not change that verdict
/// (provenance decides before value does, and a `FileCached` value is not
/// the engine's): `not_evaluated:bind_degraded` stays, `twin_reference`
/// stays empty, and the row is STILL reported and counted at the workbook
/// level with `formula_cell: true`, so `--fail-on-twin` still fails.
#[test]
fn twin_row_on_a_pinned_cell_keeps_rung_one_and_is_still_counted() {
    let (session, recalc, mut views) =
        open_for_injection(&false_green_bytes(), "w012/false_green.xlsx");
    let c1 = view_cell(&views, 1, 3).clone();
    assert_eq!(
        c1.provenance,
        oxcalc_core::workbook_settings::PublishedValueProvenance::FileCached,
        "the fixture's premise: C1 is the pin"
    );
    views[0]
        .view
        .as_mut()
        .unwrap()
        .differential_mismatches
        .push(GridDifferentialMismatch {
            address: c1.address.clone(),
            reference: CalcValue::number(0.0),
            optimized: c1.value.clone(),
        });
    let options = DiffOptions {
        fail_on_twin: true,
        ..DiffOptions::default()
    };
    let report = score_session_views(
        &session,
        &recalc,
        Ok(views),
        "w012/false_green.xlsx",
        sha256_hex(&false_green_bytes()),
        &options,
    );
    println!("{report:#?}");
    let c1 = verdict_of(&report, "C1");
    assert_eq!(c1.verdict, "not_evaluated:bind_degraded");
    assert_eq!(c1.twin_reference, None);
    assert_eq!(verdict_of(&report, "B1").verdict, "match");
    assert!(report.mismatch.is_empty(), "{:?}", report.mismatch);
    assert_eq!(report.engine_calculated, 1);
    let twin = report.reference_twin.as_ref().unwrap();
    assert_eq!(twin.mismatches.len(), 1);
    assert_eq!(twin.mismatches[0].address, "C1");
    assert!(twin.mismatches[0].formula_cell);
    let scratch = std::env::temp_dir();
    let assembled = assemble(vec![report], &scratch, &scratch, &options).unwrap();
    assert_eq!(assembled.totals.reference_twin_mismatches, 1);
    assert_eq!(exit_code(&assembled, &options), EXIT_TWIN_FAILED);
}

/// (e, wire) An artifact written before W012-1.5 — no `reference_twin`,
/// no `twin_reference`, no twin totals, no `run.validation_mode` — still
/// deserializes, with the new fields at their defaults.
#[test]
fn pre_w012_1_5_artifact_deserializes_with_twin_fields_defaulted() {
    let report = diff_workbook_bytes(
        &a1_times_three_bytes(),
        "w011/a1_times_three.xlsx",
        &DiffOptions::default(),
    );
    let scratch = std::env::temp_dir();
    let assembled = assemble(vec![report], &scratch, &scratch, &DiffOptions::default()).unwrap();
    let mut value: serde_json::Value = serde_json::to_value(&assembled).unwrap();
    let totals = value["totals"].as_object_mut().unwrap();
    assert!(totals.remove("reference_twin_mismatches").is_some());
    assert!(totals.remove("reference_twin_workbooks").is_some());
    // Pre-W012-1.7 too: no escalation block anywhere.
    assert!(totals.remove("escalation").is_some());
    let run = value["run"].as_object_mut().unwrap();
    assert!(run.remove("validation_mode").is_some());
    assert!(run.remove("fail_on_twin").is_some());
    for workbook in value["workbooks"].as_array_mut().unwrap() {
        assert!(
            workbook
                .as_object_mut()
                .unwrap()
                .remove("reference_twin")
                .is_some()
        );
        assert!(
            workbook
                .as_object_mut()
                .unwrap()
                .remove("escalation")
                .is_some()
        );
        for verdict in workbook["verdicts"].as_array_mut().unwrap() {
            assert!(
                verdict.get("twin_reference").is_none(),
                "absent when the twin does not dispute the cell"
            );
        }
    }
    let read_back: DivergenceReport = serde_json::from_value(value).unwrap();
    assert_eq!(read_back.totals.reference_twin_mismatches, 0);
    assert_eq!(read_back.run.validation_mode, None);
    assert!(!read_back.run.fail_on_twin);
    assert_eq!(read_back.workbooks[0].reference_twin, None);
    assert_eq!(read_back.workbooks[0].escalation, None);
    assert_eq!(
        read_back.totals.escalation,
        crate::escalation::EscalationSummary::default(),
        "pre-W012-1.7: the corpus escalation block defaults to zeros"
    );
    assert_eq!(read_back.coverage, assembled.coverage);
}

/// (e, rich-only) The shape the twin ACTUALLY reported on the Foundation
/// corpus on 2026-09-15 (`SCN-EK011-FUNCTION-RECOGNITION.xlsx` `H18 =
/// =_xlfn.LAMBDA(1)`): both lanes publish `#CALC!`, the cores are equal,
/// and the `CalcValue`s still compare unequal because OxFunc's `Callable`
/// rich payload compares by handle identity and each lane minted its own.
/// Reproduced here with two `Presentation` payloads that differ by value
/// (the same equality path, no callable needed): the verdict is the
/// distinct `mismatch:ReferenceTwin:RichOnly`, the row says `core_equal`
/// and names both payload kinds, and `--fail-on-twin` still fails — the
/// instrument types the engine's own verdict, it does not overrule it.
#[test]
fn twin_row_with_equal_cores_is_typed_rich_only_and_still_fails_on_twin() {
    use oxfunc_core::value::{CoreValue, NumberFormatHint, PresentationHint, WorksheetErrorCode};

    let (session, recalc, mut views) =
        open_for_injection(&a1_times_three_bytes(), "w011/a1_times_three.xlsx");
    let b1 = view_cell(&views, 1, 2).clone();
    views[0]
        .view
        .as_mut()
        .unwrap()
        .differential_mismatches
        .push(GridDifferentialMismatch {
            address: b1.address.clone(),
            reference: CalcValue::with_presentation(
                CoreValue::Error(WorksheetErrorCode::Calc),
                PresentationHint::number_format(NumberFormatHint::General),
            ),
            optimized: CalcValue::with_presentation(
                CoreValue::Error(WorksheetErrorCode::Calc),
                PresentationHint::number_format(NumberFormatHint::Currency),
            ),
        });
    let options = DiffOptions {
        fail_on_twin: true,
        ..DiffOptions::default()
    };
    let report = score_session_views(
        &session,
        &recalc,
        Ok(views),
        "w011/a1_times_three.xlsx",
        sha256_hex(&a1_times_three_bytes()),
        &options,
    );
    println!("{report:#?}");
    let verdict = verdict_of(&report, "B1");
    assert_eq!(verdict.verdict, "mismatch:ReferenceTwin:RichOnly");
    assert_eq!(verdict.twin_reference.as_deref(), Some("#CALC!"));
    assert_eq!(
        report.mismatch,
        [("ReferenceTwin:RichOnly".to_string(), 1)]
            .into_iter()
            .collect(),
        "a payload-identity disagreement never shares a count with a value disagreement"
    );
    let row = &report.reference_twin.as_ref().unwrap().mismatches[0];
    assert!(row.core_equal);
    assert_eq!(
        (row.reference_rich.as_deref(), row.optimized_rich.as_deref()),
        (Some("Presentation"), Some("Presentation"))
    );
    assert_eq!(
        (row.reference.as_str(), row.optimized.as_str()),
        ("#CALC!", "#CALC!")
    );
    let scratch = std::env::temp_dir();
    let assembled = assemble(vec![report], &scratch, &scratch, &options).unwrap();
    assert_eq!(exit_code(&assembled, &options), EXIT_TWIN_FAILED);
    let mut summary = Vec::new();
    crate::diff::write_summary(&assembled, &scratch.join(ARTIFACT_FILE_NAME), &mut summary)
        .unwrap();
    let summary = String::from_utf8(summary).unwrap();
    assert!(
        summary.contains(
            "Sheet1!B1  reference #CALC! vs optimized #CALC!  (cores equal; rich payload Presentation vs Presentation)"
        ),
        "{summary}"
    );
}

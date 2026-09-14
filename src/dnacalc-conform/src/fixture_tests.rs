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
//!   with `unclassified == 0` and the arithmetic identities intact.
//!
//! Fixture locations are `CARGO_MANIFEST_DIR` arithmetic to the repo root;
//! the parts-only W012 fixtures are zipped in memory through the dev-only
//! `oxdoc_conformance` crate (the committed-parts pattern; no zip crate).

use std::path::{Path, PathBuf};

use crate::diff::{DiffOptions, diff_workbook_bytes, run_diff};
use crate::report::{ARTIFACT_FILE_NAME, DivergenceReport, OpenStatus, SCHEMA, WorkbookReport};

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
        &DiffOptions { force_recalc: true },
    );
    println!("forced: {forced:#?}");
    assert_eq!(forced.calc_mode.as_deref(), Some("Manual"));
    assert_eq!(forced.formula_cells, 1);
    assert_eq!(forced.engine_calculated, 1);
    assert_eq!(forced.coverage, Some(1.0));
    assert_eq!(verdict_of(&forced, "B1").verdict, "match");
    assert!(forced.recalc.as_ref().unwrap().tick_id.is_some());
    assert!(forced.forced_recalc);

    let unforced = diff_workbook_bytes(
        &a1_times_three_manual_bytes(),
        "w011/a1_times_three_manual.xlsx",
        &DiffOptions {
            force_recalc: false,
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

    let mut summary = Vec::new();
    crate::diff::write_summary(&report, &artifact, &mut summary).unwrap();
    let summary = String::from_utf8(summary).unwrap();
    println!("{summary}");
    assert!(summary.contains("COVERAGE 0.7500  = engine_calculated 3 / formula_cells 4"));
    assert!(summary.contains("not_evaluated 1: bind_degraded=1"));
    assert!(summary.contains("unclassified 0"));

    std::fs::remove_dir_all(&root).unwrap();
}

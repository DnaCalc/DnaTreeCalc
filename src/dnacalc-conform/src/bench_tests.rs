//! The bench's acceptance tests (dtc-5zet.13), on a shape that finishes in
//! seconds: a temp corpus of the committed fixtures plus tiny synthetic
//! subjects. They pin what the bench MEASURES, not the wall-clock:
//!
//! - selection ranks by formula cells and records what it chose;
//! - every timed edit is proven drained (`Calculated { tick }` on the edited
//!   cell) and the dependent formula moved;
//! - the F9 counters: the cold F9 of a synthetic subject counts every
//!   authored cell; a warm F9 after a single-cell Manual edit counts the
//!   seeded pass only — 2 cells on the single-sheet fill-down (the cone),
//!   but EVERY cell of the origin sheet on the two-sheet workbook, because
//!   the cross-sheet propagation re-enters the origin with a mark-all. If
//!   either number moves, the engine's F9/propagation behaviour changed:
//!   record it against dtc-5zet.13's baseline, never "fix" the test;
//! - the artifact round-trips through serde with the schema tag first.

use std::path::{Path, PathBuf};

use crate::bench::{BenchOptions, run_bench, write_bench_summary};
use crate::bench_report::{
    BENCH_ARTIFACT_FILE_NAME, BENCH_SCHEMA, BenchReport, EditTarget, SubjectKind,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_fixture(rel: &str) -> Vec<u8> {
    let path = repo_root().join(rel);
    std::fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// Structural equality of two JSON values with numbers compared to one
/// part in 10^9 (see the serde_json note at the call site).
fn assert_values_agree(a: &serde_json::Value, b: &serde_json::Value, path: &str) {
    use serde_json::Value;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            let scale = x.abs().max(y.abs()).max(1.0);
            assert!((x - y).abs() <= scale * 1e-9, "{path}: {x} vs {y}");
        }
        (Value::Array(xs), Value::Array(ys)) => {
            assert_eq!(xs.len(), ys.len(), "{path}: array length");
            for (i, (x, y)) in xs.iter().zip(ys).enumerate() {
                assert_values_agree(x, y, &format!("{path}[{i}]"));
            }
        }
        (Value::Object(xs), Value::Object(ys)) => {
            assert_eq!(
                xs.keys().collect::<Vec<_>>(),
                ys.keys().collect::<Vec<_>>(),
                "{path}: object keys"
            );
            for (key, x) in xs {
                assert_values_agree(x, &ys[key], &format!("{path}.{key}"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}

fn temp_root(tag: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "dnacalc-conform-bench-{tag}-{}-{stamp}",
        std::process::id()
    ))
}

const FAST: BenchOptions = BenchOptions {
    corpus_subjects: 2,
    filldown_rows: 20,
    cross_sheet_rows: 5,
    open_repeats: 2,
    edit_repeats: 3,
    full_recalc_repeats: 2,
};

fn run_fast_bench() -> (BenchReport, PathBuf) {
    let root = temp_root("fast");
    let corpus = root.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    // Three W011 fixtures with one formula cell each: they tie on formula
    // cells and rank by path, so a1_times_three and cross_sheet are chosen
    // and named_input is not.
    std::fs::write(
        corpus.join("a1_times_three.xlsx"),
        read_fixture("fixtures/w011/a1_times_three.xlsx"),
    )
    .unwrap();
    std::fs::write(
        corpus.join("cross_sheet.xlsx"),
        read_fixture("fixtures/w011/cross_sheet.xlsx"),
    )
    .unwrap();
    std::fs::write(
        corpus.join("named_input.xlsx"),
        read_fixture("fixtures/w011/named_input.xlsx"),
    )
    .unwrap();
    std::fs::write(corpus.join("notes.txt"), b"not a workbook").unwrap();
    let out = root.join("out");
    let report = run_bench(&corpus, &out, &FAST).unwrap();
    (report, root)
}

#[test]
fn bench_measures_selection_edits_and_f9_counters_and_round_trips() {
    let (report, root) = run_fast_bench();
    let out = root.join("out");
    let artifact = out.join(BENCH_ARTIFACT_FILE_NAME);
    let json = std::fs::read_to_string(&artifact).unwrap();
    let read_back: BenchReport = serde_json::from_str(&json).unwrap();
    // The workspace's serde_json has no `float_roundtrip`, so a parsed
    // wall-clock sample can differ from the written one in the last ulp;
    // everything else round-trips exactly.
    assert_values_agree(
        &serde_json::to_value(&report).unwrap(),
        &serde_json::to_value(&read_back).unwrap(),
        "$",
    );
    assert!(
        json.trim_start()
            .starts_with(&format!("{{\n  \"schema\": \"{BENCH_SCHEMA}\",")),
        "the schema tag is first:\n{}",
        &json[..json.len().min(120)]
    );

    // Machine identity: whatever could not be probed is named.
    assert_eq!(report.machine.debug_assertions, cfg!(debug_assertions));
    assert_eq!(report.run.filldown_rows, 20);
    assert_eq!(report.run.cross_sheet_rows, 5);
    assert!(report.run.total_wall_ms > 0.0);
    assert!(
        !report.observability.lane_outcome_exposed
            && !report.observability.phase_timers_exposed
            && !report.observability.edit_cells_evaluated_exposed,
        "OxCalc exposes none of these today; if one becomes exposed, wire it and flip the flag"
    );

    // Selection: ranked by formula cells, the .txt is not a workbook, the
    // two largest were chosen, nothing was skipped (all three open).
    let s = &report.selection;
    assert_eq!(s.workbooks_scanned, 3);
    assert_eq!(s.unreadable, 0);
    let ranked: Vec<(&str, u64)> = s
        .ranked
        .iter()
        .map(|c| (c.path.as_str(), c.formula_cells))
        .collect();
    assert_eq!(
        ranked,
        [
            ("a1_times_three.xlsx", 1),
            ("cross_sheet.xlsx", 1),
            ("named_input.xlsx", 1)
        ],
        "ranked by formula cells descending, ties by path"
    );
    assert_eq!(s.chosen, ["a1_times_three.xlsx", "cross_sheet.xlsx"]);
    assert!(
        s.skipped_open_failed.is_empty(),
        "{:?}",
        s.skipped_open_failed
    );
    assert!(
        s.ranked.iter().all(|c| !c.excel_authored),
        "the hand-authored fixtures carry no docProps/app.xml and are not Excel-authored"
    );

    // Subjects: 2 corpus + 2 synthetic, in that order.
    let ids: Vec<&str> = report.subjects.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(report.subjects.len(), 4, "{ids:?}");
    assert_eq!(ids[2], "synthetic:filldown-20");
    assert_eq!(ids[3], "synthetic:cross-sheet-5");

    for subject in &report.subjects {
        // Every timed edit was drained inline under Automatic: the edited
        // cell carries a Calculated tick after each repeat, and the ticks
        // advance.
        assert_eq!(subject.edit.wall.samples_ms.len(), 3, "{}", subject.id);
        assert_eq!(subject.edit.entered.len(), 3);
        assert!(
            subject.edit.post_edit_ticks.iter().all(Option::is_some),
            "{}: {:?}",
            subject.id,
            subject.edit.post_edit_ticks
        );
        assert!(
            subject.edit.post_edit_ticks.windows(2).all(|w| w[0] < w[1]),
            "{}: ticks advance per edit: {:?}",
            subject.id,
            subject.edit.post_edit_ticks
        );
        assert_eq!(subject.edit.lane_outcome, None);
        assert_eq!(subject.full_recalc.len(), 2, "{}", subject.id);
        for sample in &subject.full_recalc {
            assert_eq!(sample.error, None, "{}", subject.id);
            assert!(
                sample.tick_id.is_some(),
                "{}: an F9 that drained mints a tick",
                subject.id
            );
        }
    }

    // Corpus subjects: open timings with the load report's facts; the edit
    // target is an existing literal a formula references (a1_times_three's
    // B1 = =A1*3 on the same sheet; cross_sheet's Sheet2!A1 = =Sheet1!A1*5
    // across sheets); the dependent moved.
    for subject in &report.subjects[..2] {
        let open = subject
            .open
            .as_ref()
            .expect("corpus subjects have open timings");
        assert_eq!(open.oxdoc_only.samples_ms.len(), 2);
        assert_eq!(open.product_open.samples_ms.len(), 2);
        assert_eq!(
            open.engine_recalcs_at_load, 1,
            "{}: Automatic open-recalc",
            subject.id
        );
        assert_eq!(open.recalc_path, "Automatic");
        assert!(open.formulas_bound >= 1);
        assert!(subject.setup.is_none());
        assert!(
            matches!(&subject.kind, SubjectKind::Corpus { calc_mode: Some(mode), .. } if mode == "Automatic"),
            "{:?}",
            subject.kind
        );
        assert!(
            matches!(
                subject.edit.target,
                EditTarget::ExistingLiteral {
                    referenced_by_formula: true,
                    ..
                }
            ),
            "{}: {:?}",
            subject.id,
            subject.edit.target
        );
        let dependent = subject
            .edit
            .dependent
            .as_ref()
            .expect("a referenced literal has a dependent");
        assert_ne!(
            dependent.value_before, dependent.value_after,
            "{}: the dependent {}!{} moved",
            subject.id, dependent.sheet, dependent.address
        );
        assert!(
            dependent
                .provenance_after
                .as_deref()
                .is_some_and(|p| p.starts_with("Calculated")),
            "{:?}",
            dependent.provenance_after
        );
        // Corpus F9s are never cold (the open-recalc was).
        assert!(subject.full_recalc.iter().all(|s| !s.cold));
    }
    let a1x3 = &report.subjects[0];
    assert_eq!(a1x3.id, "corpus:a1_times_three.xlsx");
    assert_eq!(a1x3.open.as_ref().unwrap().formulas_bound, 1);
    assert_eq!(a1x3.open.as_ref().unwrap().bind_degradations, 0);
    let dependent = a1x3.edit.dependent.as_ref().unwrap();
    assert_eq!(
        (dependent.sheet.as_str(), dependent.address.as_str()),
        ("Sheet1", "B1")
    );
    assert_eq!(dependent.value_before.as_deref(), Some("21.0"));
    assert_eq!(
        dependent.value_after.as_deref(),
        Some("30.0"),
        "A1 = 7 -> 10, B1 = A1*3"
    );
    assert_eq!(
        a1x3.full_recalc[0].cells_evaluated, 2,
        "the seeded pass after the rebuild: A1 + B1"
    );
    let cross_file = &report.subjects[1];
    assert_eq!(cross_file.id, "corpus:cross_sheet.xlsx");
    assert_eq!(cross_file.sheets, 2);
    assert_eq!(cross_file.edit.sheet, "Sheet1");
    assert_eq!(cross_file.edit.address, "A1");
    let dependent = cross_file.edit.dependent.as_ref().unwrap();
    assert_eq!(
        (dependent.sheet.as_str(), dependent.address.as_str()),
        ("Sheet2", "A1")
    );
    assert_eq!(dependent.value_before.as_deref(), Some("10.0"));
    assert_eq!(
        dependent.value_after.as_deref(),
        Some("25.0"),
        "A1 = 2 -> 5, Sheet2!A1 = A1*5"
    );

    // The fill-down: 20 literals + 20 formulas authored under Manual; the
    // cold F9 counts all 40; the warm F9 after the Manual edit of A1 counts
    // the seeded pass only — A1 and B1.
    let filldown = &report.subjects[2];
    let setup = filldown.setup.as_ref().unwrap();
    assert_eq!(setup.verb_calls, 40);
    assert_eq!(filldown.formula_cells, 20);
    assert_eq!(filldown.cells, 40);
    assert!(filldown.full_recalc[0].cold);
    assert_eq!(filldown.full_recalc[0].cells_evaluated, 40);
    assert_eq!(filldown.full_recalc[0].drained_sheets, 1);
    assert!(!filldown.full_recalc[1].cold);
    assert_eq!(
        filldown.full_recalc[1].cells_evaluated, 2,
        "the F9 outcome counts the seeded pass after the rebuild's mark-all: A1 + B1"
    );
    let dependent = filldown.edit.dependent.as_ref().unwrap();
    assert_eq!(dependent.address, "B1");
    assert_eq!(dependent.value_before.as_deref(), Some("2.0"));
    assert_eq!(
        dependent.value_after.as_deref(),
        Some("8.0"),
        "A1 was 1, the third Automatic edit entered 4, B1 = A1*2"
    );
    assert_eq!(filldown.edit.entered, vec!["2", "3", "4"]);

    // The two-sheet workbook: the cold F9 drains both sheets (all 10 cells);
    // the warm F9 after a Manual edit of Inputs!A1 drains Inputs only, and
    // its counter is EVERY Inputs cell (5), not the 1-cell cone — the
    // cross-sheet propagation re-entered the origin sheet with a mark-all
    // after the seeded pass.
    let cross = &report.subjects[3];
    assert_eq!(cross.setup.as_ref().unwrap().verb_calls, 10);
    assert_eq!(cross.sheets, 2);
    assert!(cross.full_recalc[0].cold);
    assert_eq!(cross.full_recalc[0].drained_sheets, 2);
    assert_eq!(cross.full_recalc[0].cells_evaluated, 10);
    assert_eq!(
        cross.full_recalc[1].drained_sheets, 1,
        "only Inputs carried a seed"
    );
    assert_eq!(
        cross.full_recalc[1].cells_evaluated, 5,
        "the origin sheet's last recalc was the propagation's mark-all, not the 1-cell seeded pass"
    );
    let dependent = cross.edit.dependent.as_ref().unwrap();
    assert_eq!(
        (dependent.sheet.as_str(), dependent.address.as_str()),
        ("Calc", "A1")
    );
    assert_eq!(dependent.value_before.as_deref(), Some("2.0"));
    assert_eq!(dependent.value_after.as_deref(), Some("8.0"));

    let mut summary = Vec::new();
    write_bench_summary(&report, &artifact, &mut summary).unwrap();
    let summary = String::from_utf8(summary).unwrap();
    println!("{summary}");
    assert!(summary.contains("subject synthetic:filldown-20"));
    assert!(summary.contains("full_recalc[0] (cold):"));
    assert!(summary.contains("lane_outcome UNOBSERVABLE"));
    assert!(summary.contains("observability: lane_outcome_exposed=false"));

    std::fs::remove_dir_all(&root).unwrap();
}

/// A corpus with no formula-bearing workbook still benches the synthetic
/// subjects; the selection says so instead of failing.
#[test]
fn bench_with_no_formula_workbooks_benches_only_the_synthetic_subjects() {
    let root = temp_root("empty");
    let corpus = root.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::write(corpus.join("junk.xlsx"), b"not a zip").unwrap();
    let out = root.join("out");
    let options = BenchOptions {
        corpus_subjects: 1,
        filldown_rows: 3,
        cross_sheet_rows: 2,
        open_repeats: 1,
        edit_repeats: 1,
        full_recalc_repeats: 1,
    };
    let report = run_bench(&corpus, &out, &options).unwrap();
    assert_eq!(report.selection.workbooks_scanned, 1);
    assert_eq!(report.selection.unreadable, 1);
    assert!(report.selection.ranked.is_empty());
    assert!(report.selection.chosen.is_empty());
    assert_eq!(report.subjects.len(), 2);
    assert_eq!(report.subjects[0].id, "synthetic:filldown-3");
    assert_eq!(
        report.subjects[0].full_recalc.len(),
        1,
        "the cold F9 is the single cycle"
    );
    assert!(report.subjects[0].full_recalc[0].cold);
    assert_eq!(report.subjects[0].full_recalc[0].cells_evaluated, 6);
    assert_eq!(report.subjects[1].id, "synthetic:cross-sheet-2");
    assert_eq!(report.subjects[1].full_recalc[0].cells_evaluated, 4);
    std::fs::remove_dir_all(&root).unwrap();
}

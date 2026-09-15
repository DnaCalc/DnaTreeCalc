//! The `diff` runner. Corpus dir, then per workbook: open through the
//! product's own path (`WorkbookSession::open_xlsx_bytes`, OxDoc
//! `LoadProfile::full()` plus OxCalc ingest), force `recalculate()`, run the
//! verdict ladder over every formula cell the file carries, and write
//! `divergence.v1.json`.
//!
//! Two rules from the W0 findings are load-bearing here and are tested by
//! the crate's fixture tests, not just stated:
//!
//! - **Provenance decides before value does** (dtc-5zet.2): a cell whose
//!   published value is `FileCached` is `NotEvaluated`, however well it
//!   agrees with the file.
//! - **The denominator is the file's** (dtc-5zet.6): `formula_cells` counts
//!   `CellPayload::Formula` payloads in OxDoc's event stream, never the
//!   engine's authored view. A workbook the engine refuses to open still
//!   contributes its formula cells — counted from an engine-free OxDoc open
//!   of the same bytes — as `not_evaluated:open_failed:*` (dtc-5zet.5).
//!
//! A third rule landed with W012-1.5 (dtc-5zet.12) — **the reference twin
//! fails loudly**: every session is opened under
//! [`GridEngineValidationMode::DualValidated`] ([`DIFF_VALIDATION_MODE`]),
//! so OxCalc's brute-force reference lane runs on every recalc, and every
//! `differential_mismatches` row the engine reports is (a) a
//! `reference_twin` row on the workbook, (b) `mismatch:ReferenceTwin` on the
//! cell's verdict when the file has a `Calculated` formula cell there, and
//! (c) with `--fail-on-twin`, exit code [`EXIT_TWIN_FAILED`]. The host
//! never re-compares the two lanes: the rows are the engine's own list,
//! read off `WorkbookSession::grid_view`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};

use dnacalc_host_core::{
    GridEngineValidationMode, LoadRecalcPath, WorkbookLoadReport, WorkbookSession,
    WorkbookSessionError,
};
use oxcalc_core::consumer::{OxCalcTreeGridCellReadout, OxCalcTreeGridView, WorkbookRecalcOutcome};
use oxcalc_core::grid::machine::GridDifferentialMismatch;
use oxcalc_core::workbook_settings::PublishedValueProvenance;
use oxdoc_xlsx::{LoadProfile, open_host_owned_xlsx_source};
use oxfunc_core::value::{CalcValue, CoreValue};
use sha2::{Digest, Sha256};

use crate::escalation::EscalationSummary;
use crate::oracle::{Oracle, OracleFormulaCell, OracleFormulaKind, column_letters, error_literal};
use crate::report::{
    ARTIFACT_FILE_NAME, CellVerdict, CorpusIdentity, DivergenceReport, LoadReportSummary,
    OpenStatus, RecalcSummary, ReferenceTwinMismatchRow, ReferenceTwinSummary, RunSettings, SCHEMA,
    Totals, WorkbookReport, WorstCovered,
};
use crate::verdict::{
    ExcludedReason, MismatchClass, NotEvaluatedReason, PinFacts, Verdict, compare,
    not_evaluated_reason,
};
use crate::volatile::nondeterministic_call;

/// How many workbooks the summary lists under "worst covered".
pub const WORST_COVERED_LIMIT: usize = 10;

/// The engine-validation profile every `diff` session runs under (W012-1.5):
/// the reference (oracle) lane on EVERY recalc, so the engine's
/// `differential_mismatches` track every recalc exactly. The interactive
/// host samples instead (`WorkbookSession::INTERACTIVE_VALIDATION_MODE`),
/// and under a sampled mode an empty list can mean "not checked this
/// recalc" — the engine exposes no per-recalc "lane ran" fact to the host —
/// so the instrument never runs sampled. `bench` keeps the interactive
/// profile on purpose: its retained baseline was measured under it.
pub const DIFF_VALIDATION_MODE: GridEngineValidationMode = GridEngineValidationMode::DualValidated;

/// The process exit code when `--fail-on-twin` is set and at least one
/// workbook's reference twin reported a mismatch. Distinct from `1` (the run
/// could not complete): the artifact IS written — a twin failure is a
/// result, and the non-zero exit is the requested consequence of it.
pub const EXIT_TWIN_FAILED: u8 = 3;

/// Runner settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffOptions {
    /// Call `recalculate()` after every open, whatever the file's `calcMode`
    /// (the product path; Appendix A constraint 3). `false` exists only so a
    /// test can prove the Manual-mode trap; the CLI exposes it as the
    /// undocumented `--no-force-recalc`.
    pub force_recalc: bool,
    /// `--fail-on-twin`: exit [`EXIT_TWIN_FAILED`] when
    /// `totals.reference_twin_mismatches > 0`. The artifact and the summary
    /// are written either way.
    pub fail_on_twin: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            force_recalc: true,
            fail_on_twin: false,
        }
    }
}

/// The binary's exit code for a completed run: [`EXIT_TWIN_FAILED`] iff
/// `--fail-on-twin` was set and the engine's reference twin disputed at
/// least one cell anywhere in the corpus; `0` otherwise. RED coverage and
/// Excel mismatches never change it (floors are a later bead).
#[must_use]
pub fn exit_code(report: &DivergenceReport, options: &DiffOptions) -> u8 {
    if options.fail_on_twin && report.totals.reference_twin_mismatches > 0 {
        EXIT_TWIN_FAILED
    } else {
        0
    }
}

/// What can stop a run outright (a scoring outcome never does: every
/// workbook-level failure is a row in the artifact).
#[derive(Debug)]
pub enum DiffError {
    Io {
        path: PathBuf,
        error: io::Error,
    },
    NoWorkbooks {
        corpus_dir: PathBuf,
    },
    /// The artifact's own arithmetic identity failed — a bug in this crate,
    /// and the artifact is not written.
    Invariant(String),
    Json(serde_json::Error),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::NoWorkbooks { corpus_dir } => {
                write!(f, "no *.xlsx under {}", corpus_dir.display())
            }
            Self::Invariant(message) => write!(f, "invariant violated: {message}"),
            Self::Json(error) => write!(f, "artifact serialization failed: {error}"),
        }
    }
}

impl std::error::Error for DiffError {}

/// Hex sha256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every `*.xlsx` under `corpus_dir`, recursively, sorted by path; Excel's
/// `~$` lock files are skipped.
pub fn collect_workbooks(corpus_dir: &Path) -> Result<Vec<PathBuf>, DiffError> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), DiffError> {
        let entries = std::fs::read_dir(dir).map_err(|error| DiffError::Io {
            path: dir.to_path_buf(),
            error,
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| DiffError::Io {
                path: dir.to_path_buf(),
                error,
            })?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
                continue;
            }
            let is_xlsx = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"));
            let is_lock_file = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("~$"));
            if is_xlsx && !is_lock_file {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    walk(corpus_dir, &mut paths)?;
    paths.sort();
    Ok(paths)
}

/// Score one workbook's bytes. `path` is the display path recorded in the
/// artifact (relative to the corpus dir). Never fails: an open failure is a
/// row with `not_evaluated:open_failed:*` cells, an unreadable file a row
/// with no cells.
#[must_use]
pub fn diff_workbook_bytes(bytes: &[u8], path: &str, options: &DiffOptions) -> WorkbookReport {
    let sha256 = sha256_hex(bytes);
    match WorkbookSession::open_xlsx_bytes_with_validation(
        format!("conform:{path}"),
        bytes,
        Some(path.to_string()),
        DIFF_VALIDATION_MODE,
    ) {
        Ok(session) => score_opened_session(session, path, sha256, options),
        Err(error) => {
            let kind = error_kind(&error);
            let debug = format!("{error:?}");
            // The engine-free second open: the denominator comes from the
            // file even when the engine refused it.
            match open_host_owned_xlsx_source(Cursor::new(bytes), LoadProfile::full()) {
                Ok(source) => {
                    let oracle = Oracle::from_events(source.source_context.events());
                    score_unopened(
                        &oracle,
                        path,
                        sha256,
                        options,
                        OpenStatus::OpenFailed {
                            kind: kind.clone(),
                            error: debug,
                        },
                        NotEvaluatedReason::OpenFailed { kind },
                    )
                }
                Err(xlsx_error) => WorkbookReport {
                    path: path.to_string(),
                    sha256,
                    open: OpenStatus::Unreadable {
                        kind: format!("xlsx:{}", variant_path(&format!("{xlsx_error:?}"))),
                        error: format!("{xlsx_error:?}"),
                    },
                    calc_mode: None,
                    forced_recalc: options.force_recalc,
                    sheets: 0,
                    cells: 0,
                    formula_cells: 0,
                    engine_calculated: 0,
                    coverage: None,
                    not_evaluated: BTreeMap::new(),
                    excluded: BTreeMap::new(),
                    matched: 0,
                    mismatch: BTreeMap::new(),
                    unclassified: 0,
                    load_report: None,
                    recalc: None,
                    reference_twin: None,
                    escalation: None,
                    verdicts: Vec::new(),
                },
            }
        }
    }
}

/// The per-workbook verdict tally.
#[derive(Debug, Default)]
struct Tally {
    engine_calculated: u64,
    not_evaluated: BTreeMap<String, u64>,
    excluded: BTreeMap<String, u64>,
    matched: u64,
    mismatch: BTreeMap<String, u64>,
    unclassified: u64,
}

impl Tally {
    fn record(&mut self, verdict: &Verdict, provenance: Option<PublishedValueProvenance>) {
        if matches!(
            provenance,
            Some(PublishedValueProvenance::Calculated { .. })
        ) {
            self.engine_calculated += 1;
        }
        match verdict {
            Verdict::NotEvaluated(reason) => {
                *self.not_evaluated.entry(reason.key()).or_default() += 1;
            }
            Verdict::Excluded(reason) => {
                *self.excluded.entry(reason.key()).or_default() += 1;
            }
            Verdict::Match => self.matched += 1,
            Verdict::Mismatch(class) => {
                *self.mismatch.entry(class.key()).or_default() += 1;
                if class.is_unclassified() {
                    self.unclassified += 1;
                }
            }
        }
    }
}

/// One engine sheet's readout for the ladder: the sheet's display name (the
/// key the oracle's sheets are matched on) and its `grid_view` — or the
/// typed error kind when the session refused the read.
#[derive(Debug, Clone)]
pub struct EngineSheetView {
    pub name: String,
    pub view: Result<OxCalcTreeGridView, String>,
}

/// Every engine sheet's `grid_view` after the recalc, in sheet order — the
/// published cells the ladder scores AND the `differential_mismatches` the
/// reference twin reported (W012-1.5). `Err` is `sheets()` itself failing
/// (its typed error kind); a per-sheet `grid_view` failure is that sheet's
/// `Err`. Public so a test can score a session with a mismatch injected at
/// this seam (the leaf-arithmetic injection the bead allows the host to
/// prove; the engine-internal blind sites are the OxCalc handover).
pub fn engine_sheet_views(session: &WorkbookSession) -> Result<Vec<EngineSheetView>, String> {
    let rows = session.sheets().map_err(|error| error_kind(&error))?;
    Ok(rows
        .into_iter()
        .map(|row| EngineSheetView {
            name: row.display_name,
            view: session
                .grid_view(row.node_id)
                .map_err(|error| error_kind(&error)),
        })
        .collect())
}

/// The opened-session path: forced recalc, then the ladder over every
/// formula cell the file carries, read against the engine's own published
/// readout per sheet.
fn score_opened_session(
    mut session: WorkbookSession,
    path: &str,
    sha256: String,
    options: &DiffOptions,
) -> WorkbookReport {
    let recalc = if options.force_recalc {
        Some(session.recalculate())
    } else {
        None
    };
    let views = engine_sheet_views(&session);
    score_session_views(&session, &recalc, views, path, sha256, options)
}

/// The ladder over an opened session's sheet views (the second half of
/// [`score_opened_session`], separated so the views can be handed in — see
/// [`engine_sheet_views`]). `recalc` is the forced recalc's outcome (`None`
/// under `--no-force-recalc`); `views` is what the engine published after it.
pub fn score_session_views(
    session: &WorkbookSession,
    recalc: &Option<Result<WorkbookRecalcOutcome, WorkbookSessionError>>,
    views: Result<Vec<EngineSheetView>, String>,
    path: &str,
    sha256: String,
    options: &DiffOptions,
) -> WorkbookReport {
    let recalc_summary = recalc.as_ref().map(recalc_summary);
    // Did the forced recalc actually run the engine over anything? A Manual
    // load whose cells were never drained is the whole-file trap; a cell
    // still `FileCached` AFTER a real drain is something else (`file_cached`
    // unless the load report explains it).
    let drained_any = matches!(&recalc, Some(Ok(outcome)) if outcome.drained_any());

    let source = session
        .xlsx_source()
        .expect("a session opened from xlsx bytes carries its OxDoc source");
    let oracle = Oracle::from_events(source.source_context.events());
    let report = session
        .load_report()
        .expect("a session opened from xlsx bytes carries its load report");
    let bind_degraded: BTreeSet<&str> = report
        .bind_degradations
        .iter()
        .map(|row| row.address.as_str())
        .collect();
    let external_pinned: BTreeSet<&str> = report
        .external_reference_pins
        .iter()
        .map(|row| row.address.as_str())
        .collect();
    let manual_load_undrained = report.recalc_path == LoadRecalcPath::Manual && !drained_any;

    // An engine read failure (sheets() or grid_view()) is an internal
    // invariant of the session, but on a corpus run it must be a typed row,
    // not a panic and not a silent "no sheets".
    let mut tally = Tally::default();
    let mut verdicts = Vec::new();
    // The reference twin's rows, every engine sheet, whether or not the file
    // has a formula cell at the address (W012-1.5): nothing the engine
    // reported is dropped. `formula_cell` is filled as the oracle loop
    // scores the matching verdict rows.
    let mut twin_rows: Vec<ReferenceTwinMismatchRow> = match &views {
        Ok(views) => views
            .iter()
            .filter_map(|sheet| sheet.view.as_ref().ok().map(|view| (sheet, view)))
            .flat_map(|(sheet, view)| {
                view.differential_mismatches
                    .iter()
                    .map(|mismatch| ReferenceTwinMismatchRow {
                        sheet: sheet.name.clone(),
                        address: a1(mismatch.address.row, mismatch.address.col),
                        reference: render_value(&mismatch.reference),
                        optimized: render_value(&mismatch.optimized),
                        core_equal: mismatch.reference.core == mismatch.optimized.core,
                        reference_rich: rich_kind(&mismatch.reference),
                        optimized_rich: rich_kind(&mismatch.optimized),
                        formula_cell: false,
                    })
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    for oracle_sheet in &oracle.sheets {
        let published: Result<PublishedSheet<'_>, NotEvaluatedReason> = match &views {
            Err(kind) => Err(NotEvaluatedReason::EngineReadFailed { kind: kind.clone() }),
            Ok(views) => match views.iter().find(|sheet| sheet.name == oracle_sheet.name) {
                None => Err(NotEvaluatedReason::SheetUnmatched),
                Some(sheet) => match &sheet.view {
                    Ok(view) => Ok((
                        view.cells
                            .iter()
                            .map(|cell| ((cell.address.row, cell.address.col), cell))
                            .collect(),
                        view.differential_mismatches
                            .iter()
                            .map(|mismatch| {
                                ((mismatch.address.row, mismatch.address.col), mismatch)
                            })
                            .collect(),
                    )),
                    Err(kind) => Err(NotEvaluatedReason::EngineReadFailed { kind: kind.clone() }),
                },
            },
        };

        for cell in &oracle_sheet.formula_cells {
            let (verdict, readout) = match &published {
                Err(reason) => (Verdict::NotEvaluated(reason.clone()), None),
                Ok((published, twins)) => match published.get(&(cell.row, cell.col)) {
                    None => (
                        Verdict::NotEvaluated(NotEvaluatedReason::NotPublished),
                        None,
                    ),
                    Some(readout) => {
                        let facts = PinFacts {
                            bind_degraded: bind_degraded.contains(cell.r1c1().as_str()),
                            external_reference_pinned: external_pinned
                                .contains(cell.r1c1().as_str()),
                            not_calc_modeled_kind: matches!(
                                cell.kind,
                                OracleFormulaKind::DataTable | OracleFormulaKind::Unknown
                            ),
                            manual_load_undrained,
                        };
                        let twin = twins.get(&(cell.row, cell.col)).copied();
                        (ladder(cell, readout, twin, &oracle, &facts), Some(*readout))
                    }
                },
            };
            // The file carries a formula cell at a disputed address: say so
            // on the twin row, whatever verdict the ladder gave the cell.
            if matches!(&published, Ok((_, twins)) if twins.contains_key(&(cell.row, cell.col))) {
                let a1 = cell.a1();
                for row in twin_rows
                    .iter_mut()
                    .filter(|row| row.sheet == oracle_sheet.name && row.address == a1)
                {
                    row.formula_cell = true;
                }
            }
            let provenance = readout.map(|readout| readout.provenance);
            tally.record(&verdict, provenance);
            verdicts.push(CellVerdict {
                sheet: oracle_sheet.name.clone(),
                address: cell.a1(),
                formula: cell.text.clone(),
                verdict: verdict.key(),
                provenance: provenance.map(|provenance| format!("{provenance:?}")),
                tick_id: provenance.and_then(|provenance| match provenance {
                    PublishedValueProvenance::Calculated { tick_id } => Some(tick_id),
                    _ => None,
                }),
                engine: readout.map(|readout| render_value(&readout.value)),
                excel: cell.cached.as_ref().map(|payload| {
                    oracle
                        .resolve_cached(payload)
                        .map(|value| render_value(&value))
                        .unwrap_or_else(|gap| format!("<{gap:?}>"))
                }),
                twin_reference: match &verdict {
                    Verdict::Mismatch(MismatchClass::ReferenceTwin { reference, .. }) => {
                        Some(reference.clone())
                    }
                    _ => None,
                },
            });
        }
    }

    let validation_mode = session.validation_mode();
    let reference_twin = Some(ReferenceTwinSummary {
        validation_mode: format!("{validation_mode:?}"),
        checked_every_recalc: matches!(validation_mode, GridEngineValidationMode::DualValidated),
        mismatches: twin_rows,
    });
    let formula_cells = oracle.formula_cell_count();
    WorkbookReport {
        path: path.to_string(),
        sha256,
        open: OpenStatus::Opened,
        calc_mode: oracle.calc_mode.map(|mode| format!("{mode:?}")),
        forced_recalc: options.force_recalc,
        sheets: oracle.sheets.len() as u64,
        cells: oracle.cell_count(),
        formula_cells,
        engine_calculated: tally.engine_calculated,
        coverage: coverage(tally.engine_calculated, formula_cells),
        not_evaluated: tally.not_evaluated,
        excluded: tally.excluded,
        matched: tally.matched,
        mismatch: tally.mismatch,
        unclassified: tally.unclassified,
        load_report: Some(load_report_summary(report)),
        recalc: recalc_summary,
        reference_twin,
        // W012-1.7: the session's accounting after the forced recalc — the
        // only host verb `diff` issues — read, never derived.
        escalation: Some(EscalationSummary::from_accounting(
            &session.recalc_accounting(),
        )),
        verdicts,
    }
}

/// One engine sheet's readout indexed for the ladder: the published cells
/// and the reference twin's disputed cells, both by 1-based `(row, col)`.
type PublishedSheet<'a> = (
    BTreeMap<(u32, u32), &'a OxCalcTreeGridCellReadout>,
    BTreeMap<(u32, u32), &'a GridDifferentialMismatch>,
);

/// A1 notation for a 1-based `(row, col)`.
fn a1(row: u32, col: u32) -> String {
    format!("{}{row}", column_letters(col))
}

/// The variant name of a value's rich payload, if it carries one.
fn rich_kind(value: &CalcValue) -> Option<String> {
    use oxfunc_core::value::RichValue;
    value.rich.as_ref().map(|rich| {
        match rich.as_ref() {
            RichValue::Object(_) => "Object",
            RichValue::Callable(_) => "Callable",
            RichValue::Presentation(_) => "Presentation",
            RichValue::ErrorMetadata(_) => "ErrorMetadata",
        }
        .to_string()
    })
}

/// Rungs 1–4 for one published formula cell. `twin` is the engine's own
/// differential row for the address, when the reference lane disputed it.
fn ladder(
    cell: &OracleFormulaCell,
    readout: &OxCalcTreeGridCellReadout,
    twin: Option<&GridDifferentialMismatch>,
    oracle: &Oracle,
    facts: &PinFacts,
) -> Verdict {
    // Rung 1: provenance decides before value does.
    if let Some(reason) = not_evaluated_reason(readout.provenance, facts) {
        return Verdict::NotEvaluated(reason);
    }
    // Rung 1b (W012-1.5): the engine disagrees with itself on this cell —
    // its published value is not compared with Excel at all.
    if let Some(mismatch) = twin {
        return Verdict::Mismatch(MismatchClass::ReferenceTwin {
            reference: render_value(&mismatch.reference),
            optimized: render_value(&mismatch.optimized),
            core_equal: mismatch.reference.core == mismatch.optimized.core,
        });
    }
    // Rung 2: nondeterministic formulas are excluded, never scored.
    let function = cell.text.as_deref().and_then(nondeterministic_call);
    if function.is_some() || cell.calculate_cell {
        return Verdict::Excluded(ExcludedReason::Nondeterministic {
            function,
            ca_marker: cell.calculate_cell,
        });
    }
    // Rung 3: no scorable oracle.
    let Some(payload) = &cell.cached else {
        return Verdict::Excluded(ExcludedReason::NoFileCache);
    };
    let excel = match oracle.resolve_cached(payload) {
        Ok(value) => value,
        Err(gap) => return Verdict::Excluded(gap.into()),
    };
    // Rung 4: Calculated with a cache — compare.
    compare(&readout.value, &excel)
}

/// The engine refused the stream: every formula cell the file carries is
/// `not_evaluated:open_failed:<kind>`, in the denominator, with no
/// provenance and no tick.
fn score_unopened(
    oracle: &Oracle,
    path: &str,
    sha256: String,
    options: &DiffOptions,
    open: OpenStatus,
    reason: NotEvaluatedReason,
) -> WorkbookReport {
    let verdict = Verdict::NotEvaluated(reason);
    let mut tally = Tally::default();
    let mut verdicts = Vec::new();
    for sheet in &oracle.sheets {
        for cell in &sheet.formula_cells {
            tally.record(&verdict, None);
            verdicts.push(CellVerdict {
                sheet: sheet.name.clone(),
                address: cell.a1(),
                formula: cell.text.clone(),
                verdict: verdict.key(),
                provenance: None,
                tick_id: None,
                engine: None,
                excel: cell.cached.as_ref().map(|payload| {
                    oracle
                        .resolve_cached(payload)
                        .map(|value| render_value(&value))
                        .unwrap_or_else(|gap| format!("<{gap:?}>"))
                }),
                twin_reference: None,
            });
        }
    }
    let formula_cells = oracle.formula_cell_count();
    WorkbookReport {
        path: path.to_string(),
        sha256,
        open,
        calc_mode: oracle.calc_mode.map(|mode| format!("{mode:?}")),
        forced_recalc: options.force_recalc,
        sheets: oracle.sheets.len() as u64,
        cells: oracle.cell_count(),
        formula_cells,
        engine_calculated: 0,
        coverage: coverage(0, formula_cells),
        not_evaluated: tally.not_evaluated,
        excluded: tally.excluded,
        matched: 0,
        mismatch: BTreeMap::new(),
        unclassified: 0,
        load_report: None,
        recalc: None,
        reference_twin: None,
        escalation: None,
        verdicts,
    }
}

/// `engine_calculated / formula_cells`, undefined (`None`) with no formula
/// cells.
#[must_use]
pub fn coverage(engine_calculated: u64, formula_cells: u64) -> Option<f64> {
    (formula_cells > 0).then(|| engine_calculated as f64 / formula_cells as f64)
}

fn recalc_summary(outcome: &Result<WorkbookRecalcOutcome, WorkbookSessionError>) -> RecalcSummary {
    match outcome {
        Ok(outcome) => RecalcSummary {
            tick_id: outcome.tick_id,
            drained_sheets: outcome.drained.len() as u64,
            cells_evaluated: outcome.total_cells_evaluated(),
            error: None,
        },
        Err(error) => RecalcSummary {
            tick_id: None,
            drained_sheets: 0,
            cells_evaluated: 0,
            error: Some(format!("{error:?}")),
        },
    }
}

fn load_report_summary(report: &WorkbookLoadReport) -> LoadReportSummary {
    LoadReportSummary {
        sheets: report.sheets,
        cells: report.cells,
        formulas_bound: report.formulas_bound,
        not_calc_modeled: report.not_calc_modeled,
        region_cells_unbacked: report.region_cells_unbacked,
        external_ref_cells_pinned: report.external_ref_cells_pinned,
        bind_degradations: report.bind_degradations.len() as u32,
        names: report.names,
        tables: report.tables,
        engine_recalcs_at_load: report.engine_recalcs_at_load,
        recalc_path: format!("{:?}", report.recalc_path),
    }
}

/// A short typed key for a session open error: `xlsx:<Variant>` for an
/// OxDoc rejection, `oxcalc:<Variant>:<Inner>` for an engine rejection,
/// `session:<Variant>` otherwise.
#[must_use]
pub fn error_kind(error: &WorkbookSessionError) -> String {
    match error {
        WorkbookSessionError::Xlsx(inner) => {
            format!("xlsx:{}", variant_path(&format!("{inner:?}")))
        }
        WorkbookSessionError::OxCalc(inner) => {
            format!("oxcalc:{}", variant_path(&format!("{inner:?}")))
        }
        other => format!("session:{}", variant_path(&format!("{other:?}"))),
    }
}

/// The nested variant names of a `Debug`-rendered enum value, joined by
/// `:` (`GridEngine { error: InvalidDefinedName { name: "x" } }` →
/// `GridEngine:InvalidDefinedName`): the capitalized identifiers outside
/// string literals, at most two deep (the third is typically a payload
/// struct such as `ExcelGridCellAddress`, not a variant).
#[must_use]
pub fn variant_path(debug: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for c in debug.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            current.push(c);
            continue;
        }
        push_variant(&mut names, &mut current);
    }
    push_variant(&mut names, &mut current);
    names.truncate(2);
    if names.is_empty() {
        "Unknown".to_string()
    } else {
        names.join(":")
    }
}

fn push_variant(names: &mut Vec<String>, current: &mut String) {
    if current
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        names.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// A compact, human-readable rendering of a value for the artifact.
#[must_use]
pub fn render_value(value: &CalcValue) -> String {
    match value.core() {
        CoreValue::Number(n) => format!("{n:?}"),
        CoreValue::Text(text) => format!("{:?}", text.to_string_lossy()),
        CoreValue::Logical(true) => "TRUE".to_string(),
        CoreValue::Logical(false) => "FALSE".to_string(),
        CoreValue::Error(code) => error_literal(*code).to_string(),
        CoreValue::Empty => "<empty>".to_string(),
        CoreValue::Missing => "<missing>".to_string(),
        CoreValue::Array(array) => {
            format!("<array {}x{}>", array.shape().rows, array.shape().cols)
        }
        CoreValue::Reference(_) => "<reference>".to_string(),
    }
}

/// Run the differential over every workbook under `corpus_dir`, write
/// `<out_dir>/divergence.v1.json`, and return the report. The artifact is
/// written only after its arithmetic identities hold.
pub fn run_diff(
    corpus_dir: &Path,
    out_dir: &Path,
    options: &DiffOptions,
) -> Result<DivergenceReport, DiffError> {
    let paths = collect_workbooks(corpus_dir)?;
    if paths.is_empty() {
        return Err(DiffError::NoWorkbooks {
            corpus_dir: corpus_dir.to_path_buf(),
        });
    }
    let mut workbooks = Vec::with_capacity(paths.len());
    for path in &paths {
        let bytes = std::fs::read(path).map_err(|error| DiffError::Io {
            path: path.clone(),
            error,
        })?;
        let display = path
            .strip_prefix(corpus_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        workbooks.push(diff_workbook_bytes(&bytes, &display, options));
    }
    let report = assemble(workbooks, corpus_dir, out_dir, options)?;
    std::fs::create_dir_all(out_dir).map_err(|error| DiffError::Io {
        path: out_dir.to_path_buf(),
        error,
    })?;
    let artifact = out_dir.join(ARTIFACT_FILE_NAME);
    let json = serde_json::to_string_pretty(&report).map_err(DiffError::Json)?;
    std::fs::write(&artifact, json).map_err(|error| DiffError::Io {
        path: artifact,
        error,
    })?;
    Ok(report)
}

/// Totals, worst-covered list and corpus identity over scored workbooks,
/// with the identities checked: `engine_calculated == match + mismatch +
/// excluded` and `formula_cells == engine_calculated + not_evaluated`, per
/// workbook and corpus-wide.
pub fn assemble(
    workbooks: Vec<WorkbookReport>,
    corpus_dir: &Path,
    out_dir: &Path,
    options: &DiffOptions,
) -> Result<DivergenceReport, DiffError> {
    let mut totals = Totals {
        workbooks: workbooks.len() as u64,
        ..Totals::default()
    };
    for workbook in &workbooks {
        match &workbook.open {
            OpenStatus::Opened => totals.opened += 1,
            OpenStatus::OpenFailed { .. } => totals.open_failed += 1,
            OpenStatus::Unreadable { .. } => totals.unreadable += 1,
        }
        totals.sheets += workbook.sheets;
        totals.cells += workbook.cells;
        totals.formula_cells += workbook.formula_cells;
        totals.engine_calculated += workbook.engine_calculated;
        totals.matched += workbook.matched;
        totals.unclassified += workbook.unclassified;
        if let Some(twin) = &workbook.reference_twin {
            if !twin.mismatches.is_empty() {
                totals.reference_twin_workbooks += 1;
            }
            totals.reference_twin_mismatches += twin.mismatches.len() as u64;
        }
        let mut not_evaluated = 0;
        for (key, count) in &workbook.not_evaluated {
            *totals.not_evaluated.entry(key.clone()).or_default() += count;
            not_evaluated += count;
        }
        let mut excluded = 0;
        for (key, count) in &workbook.excluded {
            *totals.excluded.entry(key.clone()).or_default() += count;
            excluded += count;
        }
        let mut mismatch = 0;
        for (key, count) in &workbook.mismatch {
            *totals.mismatch.entry(key.clone()).or_default() += count;
            mismatch += count;
        }
        if workbook.engine_calculated != workbook.matched + mismatch + excluded {
            return Err(DiffError::Invariant(format!(
                "{}: engine_calculated {} != match {} + mismatch {} + excluded {}",
                workbook.path, workbook.engine_calculated, workbook.matched, mismatch, excluded
            )));
        }
        if workbook.formula_cells != workbook.engine_calculated + not_evaluated {
            return Err(DiffError::Invariant(format!(
                "{}: formula_cells {} != engine_calculated {} + not_evaluated {}",
                workbook.path, workbook.formula_cells, workbook.engine_calculated, not_evaluated
            )));
        }
        if workbook.verdicts.len() as u64 != workbook.formula_cells {
            return Err(DiffError::Invariant(format!(
                "{}: {} verdict rows for {} formula cells",
                workbook.path,
                workbook.verdicts.len(),
                workbook.formula_cells
            )));
        }
        if let Some(escalation) = &workbook.escalation {
            escalation
                .check_identities(&workbook.path)
                .map_err(DiffError::Invariant)?;
        }
    }
    totals.escalation =
        EscalationSummary::fold(workbooks.iter().filter_map(|w| w.escalation.as_ref()));
    totals
        .escalation
        .check_identities("totals")
        .map_err(DiffError::Invariant)?;
    totals.not_evaluated_total = totals.not_evaluated.values().sum();
    totals.excluded_total = totals.excluded.values().sum();
    totals.mismatch_total = totals.mismatch.values().sum();
    totals.coverage = coverage(totals.engine_calculated, totals.formula_cells).unwrap_or(0.0);

    let mut worst: Vec<WorstCovered> = workbooks
        .iter()
        .filter_map(|workbook| {
            workbook.coverage.map(|coverage| WorstCovered {
                path: workbook.path.clone(),
                coverage,
                formula_cells: workbook.formula_cells,
                engine_calculated: workbook.engine_calculated,
            })
        })
        .collect();
    worst.sort_by(|a, b| {
        a.coverage
            .partial_cmp(&b.coverage)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.formula_cells.cmp(&a.formula_cells))
            .then(a.path.cmp(&b.path))
    });
    worst.truncate(WORST_COVERED_LIMIT);

    let mut sha256_set: Vec<String> = workbooks.iter().map(|w| w.sha256.clone()).collect();
    sha256_set.sort();
    let set_sha256 = sha256_hex(sha256_set.join("\n").as_bytes());

    Ok(DivergenceReport {
        schema: SCHEMA.to_string(),
        coverage: totals.coverage,
        totals,
        worst_covered: worst,
        corpus: CorpusIdentity {
            workbooks: workbooks.len() as u64,
            sha256_set,
            set_sha256,
        },
        run: RunSettings {
            corpus_dir: corpus_dir.to_string_lossy().replace('\\', "/"),
            out_dir: out_dir.to_string_lossy().replace('\\', "/"),
            forced_recalc: options.force_recalc,
            validation_mode: Some(format!("{DIFF_VALIDATION_MODE:?}")),
            fail_on_twin: options.fail_on_twin,
        },
        workbooks,
    })
}

/// The stdout summary: coverage first, every class with its denominator,
/// the worst-covered workbooks, and where the artifact went.
pub fn write_summary(
    report: &DivergenceReport,
    artifact_path: &Path,
    out: &mut impl Write,
) -> io::Result<()> {
    let t = &report.totals;
    writeln!(
        out,
        "dnacalc-conform diff — provenance-first Excel-vs-DNA differential ({})",
        report.schema
    )?;
    writeln!(
        out,
        "corpus: {} — {} workbooks (set sha256 {})",
        report.run.corpus_dir, t.workbooks, report.corpus.set_sha256
    )?;
    writeln!(
        out,
        "COVERAGE {:.4}  = engine_calculated {} / formula_cells {}",
        report.coverage, t.engine_calculated, t.formula_cells
    )?;
    // W012-1.7: beside coverage, every recalc the run caused and what the
    // engine let the host see of it — corpus-wide here, per workbook in
    // the artifact (`workbooks[*].escalation`).
    t.escalation.write_line("ESCALATION", out)?;
    let with_recalcs = report
        .workbooks
        .iter()
        .filter(|w| w.escalation.is_some_and(|e| e.recalcs > 0))
        .count();
    let at_bound_one = report
        .workbooks
        .iter()
        .filter(|w| {
            w.escalation
                .and_then(|e| e.escalation_rate_lower_bound)
                .is_some_and(|b| b == 1.0)
        })
        .count();
    writeln!(
        out,
        "  escalation per workbook (retained in the artifact): {with_recalcs} of {} opened workbooks issued a recalc after the open (the forced F9 drains a Manual load; on an Automatic load it is a no-op); {at_bound_one} at lower bound 1.0",
        t.opened
    )?;
    writeln!(
        out,
        "  workbooks: opened {}, open_failed {}, unreadable {}; sheets {}, cells {}; forced_recalc {}",
        t.opened, t.open_failed, t.unreadable, t.sheets, t.cells, report.run.forced_recalc
    )?;
    write_map(
        out,
        "not_evaluated",
        t.not_evaluated_total,
        &t.not_evaluated,
    )?;
    write_map(out, "excluded", t.excluded_total, &t.excluded)?;
    writeln!(out, "  match {}", t.matched)?;
    write_map(out, "mismatch", t.mismatch_total, &t.mismatch)?;
    writeln!(
        out,
        "  unclassified {}{}",
        t.unclassified,
        if t.unclassified > 0 {
            "  <-- MUST BE 0: define the class in a named bead, never widen silently"
        } else {
            ""
        }
    )?;
    writeln!(
        out,
        "  reference_twin ({}, every recalc): mismatches {} in {} workbooks{}",
        report
            .run
            .validation_mode
            .as_deref()
            .unwrap_or("<pre-W012-1.5 artifact: sampled>"),
        t.reference_twin_mismatches,
        t.reference_twin_workbooks,
        if report.run.fail_on_twin && t.reference_twin_mismatches > 0 {
            format!("  <-- --fail-on-twin: exit {EXIT_TWIN_FAILED}")
        } else {
            String::new()
        }
    )?;
    for workbook in &report.workbooks {
        let Some(twin) = &workbook.reference_twin else {
            continue;
        };
        for row in &twin.mismatches {
            writeln!(
                out,
                "    {}  {}!{}  reference {} vs optimized {}{}{}",
                workbook.path,
                row.sheet,
                row.address,
                row.reference,
                row.optimized,
                if row.core_equal {
                    format!(
                        "  (cores equal; rich payload {} vs {})",
                        row.reference_rich.as_deref().unwrap_or("none"),
                        row.optimized_rich.as_deref().unwrap_or("none")
                    )
                } else {
                    String::new()
                },
                if row.formula_cell {
                    ""
                } else {
                    "  (no formula cell in the file at this address)"
                }
            )?;
        }
    }
    writeln!(
        out,
        "worst-covered {} workbooks (coverage, engine_calculated/formula_cells):",
        report.worst_covered.len()
    )?;
    for row in &report.worst_covered {
        writeln!(
            out,
            "  {:.4}  {}/{}  {}",
            row.coverage, row.engine_calculated, row.formula_cells, row.path
        )?;
    }
    writeln!(
        out,
        "artifact: {}",
        artifact_path.to_string_lossy().replace('\\', "/")
    )?;
    Ok(())
}

fn write_map(
    out: &mut impl Write,
    label: &str,
    total: u64,
    map: &BTreeMap<String, u64>,
) -> io::Result<()> {
    write!(out, "  {label} {total}")?;
    if map.is_empty() {
        return writeln!(out);
    }
    write!(out, ":")?;
    for (key, count) in map {
        write!(out, " {key}={count}")?;
    }
    writeln!(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::{ExcelText, WorksheetErrorCode};

    #[test]
    fn variant_path_extracts_nested_variant_names_outside_string_literals() {
        assert_eq!(
            variant_path("GridEngine { error: InvalidDefinedName { name: \"Größe\" } }"),
            "GridEngine:InvalidDefinedName"
        );
        assert_eq!(
            variant_path(
                "InvalidCellValue { part: \"xl/Sheet1.xml\", cell_ref: \"A1\", value: \"x\", \
                 expected: \"number\" }"
            ),
            "InvalidCellValue"
        );
        assert_eq!(variant_path("NoBackingSource"), "NoBackingSource");
        assert_eq!(
            variant_path("A { b: B { c: C { d: D } } }"),
            "A:B",
            "at most two deep"
        );
        assert_eq!(variant_path("\"OnlyAString\""), "Unknown");
        assert_eq!(variant_path("lower { case: only }"), "Unknown");
    }

    #[test]
    fn render_value_is_compact_and_uses_excel_error_literals() {
        assert_eq!(render_value(&CalcValue::number(21.0)), "21.0");
        assert_eq!(render_value(&CalcValue::number(0.1)), "0.1");
        assert_eq!(
            render_value(&CalcValue::text(ExcelText::from_interop_assignment("a\"b"))),
            "\"a\\\"b\""
        );
        assert_eq!(render_value(&CalcValue::logical(true)), "TRUE");
        assert_eq!(
            render_value(&CalcValue::error(WorksheetErrorCode::Div0)),
            "#DIV/0!"
        );
        assert_eq!(render_value(&CalcValue::empty()), "<empty>");
    }

    #[test]
    fn coverage_is_undefined_without_formula_cells() {
        assert_eq!(coverage(0, 0), None);
        assert_eq!(coverage(0, 4), Some(0.0));
        assert_eq!(coverage(3, 4), Some(0.75));
    }

    #[test]
    fn sha256_hex_matches_the_known_empty_and_abc_digests() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

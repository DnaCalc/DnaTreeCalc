//! The `bench` runner (dtc-5zet.13): the program's first retained
//! wall-clock baseline for the grid engine — the lane that runs `.xlsx` —
//! measured through the path the product runs.
//!
//! Subjects:
//!
//! - the N largest-formula-cell corpus workbooks that open on the product
//!   path (Excel-authored ranked first; higher-ranked files the engine
//!   refuses are listed as skipped with their typed error);
//! - a synthetic fill-down (`A{r}` literal, `B{r} = =A{r}*2`, `rows` rows)
//!   authored in memory through `WorkbookSession::enter_grid_cell`;
//! - a synthetic two-sheet cross-reference workbook (`Inputs!A{r}` literal,
//!   `Calc!A{r} = =Inputs!A{r}*2`) so the cross-sheet propagation's
//!   escalation (`consumer.rs` `recalculate_sheet_with_cross_sheet_view`
//!   sets `topology_grew_since_recalc` unconditionally) is in the numbers.
//!
//! Per subject, in this order: open (corpus) or author under `Manual` and
//! F9 (synthetic: the cold full recalc, with `cells_evaluated`); then the
//! Automatic-mode single-cell edit series on an input cell
//! (`enter_grid_cell`, the product's keystroke commit); then Manual-edit +
//! F9 cycles, each a rebuild-from-input mark-all followed by a seeded pass
//! whose `cells_evaluated` the outcome reports (the rebuild's own mark-all
//! is not in the counter — see [`crate::bench_report::FullRecalcSample`]).
//!
//! What the engine does not expose is recorded as absent — see
//! [`crate::bench_report::Observability`] — never estimated.

use std::collections::BTreeMap;
use std::io::{self, Cursor, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use dnacalc_host_core::{WorkbookSession, WorkbookSessionError};
use dnacalc_skin_ir::CalcModeProjection;
use oxcalc_core::consumer::{GridCellEntryOutcome, OxCalcTreeGridView, WorkbookRecalcOutcome};
use oxcalc_core::structural::TreeNodeId;
use oxcalc_core::workbook_settings::PublishedValueProvenance;
use oxdoc_model::{CellPayload, DocumentEvent};
use oxdoc_xlsx::{LoadProfile, open_host_owned_xlsx_source};

use crate::bench_report::{
    BENCH_ARTIFACT_FILE_NAME, BENCH_SCHEMA, BenchReport, BenchRunSettings, Candidate,
    DependentReadback, EditTarget, EditTimings, FullRecalcSample, Observability, OpenTimings,
    Samples, Selection, SetupTimings, SkippedCandidate, SubjectKind, SubjectReport,
};
use crate::diff::{DiffError, collect_workbooks, error_kind, render_value, sha256_hex};
use crate::escalation::EscalationSummary;
use crate::machine::{peak_working_set_bytes, probe_machine};
use crate::oracle::{Oracle, column_letters};

/// The validation mode host-core constructs every session with
/// (`workbook.rs`, `new_context`). Recorded, not chosen here.
pub const HOST_VALIDATION_MODE: &str = "DualValidatedSampled { one_in: 16 }";

/// Runner settings. The defaults are the baseline shape the committed
/// `docs/test-runs/w012/BASELINE_*.md` was measured with and finish in
/// about two minutes in `--release`; the knobs exist so the crate's tests
/// can run the same code on a shape that finishes in seconds, and so the
/// bead's 10,000-formula-cell fill-down can be measured as an explicitly
/// longer run (`--filldown-rows 10000`: every `enter_grid_cell` is
/// O(authored cells) on the product path, so authoring 20,000 cells takes
/// tens of minutes — measured, see the baseline doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchOptions {
    pub corpus_subjects: usize,
    pub filldown_rows: u32,
    pub cross_sheet_rows: u32,
    pub open_repeats: u32,
    pub edit_repeats: u32,
    pub full_recalc_repeats: u32,
}

impl Default for BenchOptions {
    fn default() -> Self {
        Self {
            corpus_subjects: 3,
            filldown_rows: 2_000,
            cross_sheet_rows: 500,
            open_repeats: 5,
            edit_repeats: 5,
            full_recalc_repeats: 3,
        }
    }
}

/// Run `f`, returning its result and the wall-clock it took in milliseconds.
fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let started = Instant::now();
    let value = f();
    (value, started.elapsed().as_secs_f64() * 1000.0)
}

fn a1(row: u32, col: u32) -> String {
    format!("{}{}", column_letters(col), row)
}

fn provenance_text(provenance: PublishedValueProvenance) -> String {
    format!("{provenance:?}")
}

fn calculated_tick(provenance: PublishedValueProvenance) -> Option<u64> {
    match provenance {
        PublishedValueProvenance::Calculated { tick_id } => Some(tick_id),
        _ => None,
    }
}

fn view_of(outcome: &GridCellEntryOutcome) -> &OxCalcTreeGridView {
    match outcome {
        GridCellEntryOutcome::Literal { view, .. }
        | GridCellEntryOutcome::Formula { view, .. }
        | GridCellEntryOutcome::Cleared { view } => view,
    }
}

/// The `Calculated { tick_id }` the cell at `(row, col)` carries in `view`.
fn tick_in_view(view: &OxCalcTreeGridView, row: u32, col: u32) -> Option<u64> {
    view.cells
        .iter()
        .find(|cell| cell.address.row == row && cell.address.col == col)
        .and_then(|cell| calculated_tick(cell.provenance))
}

/// One literal cell of a corpus workbook, as the file carries it.
#[derive(Debug, Clone)]
struct LiteralCell {
    sheet_index: usize,
    row: u32,
    col: u32,
    /// The numeric reading of the literal (`Number` verbatim, `Bool` as
    /// 0/1, text as 0): the base the bench's replacement values count up
    /// from, so every repeat is a real change.
    number: f64,
    original: String,
}

/// Every literal (non-formula, non-empty) cell in the stream, sheet by
/// sheet in stream order, row-major within a sheet.
fn literal_cells(events: &[DocumentEvent], string_table: &[String]) -> Vec<LiteralCell> {
    let mut out = Vec::new();
    let mut sheet_index: Option<usize> = None;
    let mut seen_sheets = 0usize;
    let mut per_sheet: Vec<Vec<LiteralCell>> = Vec::new();
    for event in events {
        match event {
            DocumentEvent::SheetBegin(_) => {
                sheet_index = Some(seen_sheets);
                seen_sheets += 1;
                per_sheet.push(Vec::new());
            }
            DocumentEvent::SheetEnd { .. } => sheet_index = None,
            DocumentEvent::CellChunk(chunk) => {
                let Some(index) = sheet_index else {
                    continue;
                };
                for (addr, payload) in &chunk.cells {
                    let (number, original) = match payload {
                        CellPayload::Number(n) => (*n, format!("{n}")),
                        CellPayload::Bool(b) => (f64::from(u8::from(*b)), format!("{b}")),
                        CellPayload::InlineText(text) => (0.0, text.clone()),
                        CellPayload::SharedText(i) => (
                            0.0,
                            string_table
                                .get(*i as usize)
                                .cloned()
                                .unwrap_or_else(|| format!("<shared {i}>")),
                        ),
                        _ => continue,
                    };
                    per_sheet[index].push(LiteralCell {
                        sheet_index: index,
                        row: addr.row_one_based(),
                        col: addr.col_one_based(),
                        number,
                        original,
                    });
                }
            }
            _ => {}
        }
    }
    for mut cells in per_sheet {
        cells.sort_by_key(|cell| (cell.row, cell.col));
        out.extend(cells);
    }
    out
}

/// The sheet qualifiers of every whole-token occurrence of the A1 address
/// `(row, col)` in `formula` — `A1`, `$A$1`, `a1` — not inside `AA1` or
/// `A10`: `None` for an unqualified token, `Some(name)` for `Name!A1` /
/// `'Name'!A1`. A best-effort text scan: string literals are not excluded,
/// and a range `A2:A9` does not count as referencing `A5`. Used only to
/// pick a cell an edit will visibly propagate from and a formula to read
/// back.
fn address_token_qualifiers(formula: &str, row: u32, col: u32) -> Vec<Option<String>> {
    let letters = column_letters(col);
    let digits = row.to_string();
    let chars: Vec<char> = formula.chars().collect();
    let n = chars.len();
    let mut found = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i;
        if chars[j] == '$' {
            j += 1;
        }
        let mut matched = true;
        for letter in letters.chars() {
            if j < n && chars[j].eq_ignore_ascii_case(&letter) {
                j += 1;
            } else {
                matched = false;
                break;
            }
        }
        if matched {
            if j < n && chars[j] == '$' {
                j += 1;
            }
            for digit in digits.chars() {
                if j < n && chars[j] == digit {
                    j += 1;
                } else {
                    matched = false;
                    break;
                }
            }
        }
        if matched {
            let before_ok = i == 0 || !is_ident_char(chars[i - 1]);
            let after_ok = j >= n || (!is_ident_char(chars[j]) && chars[j] != '(');
            if before_ok && after_ok {
                found.push(sheet_qualifier_before(&chars, i));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// The `Name!` / `'Name'!` qualifier ending at `end` (exclusive), if the
/// token there is sheet-qualified.
fn sheet_qualifier_before(chars: &[char], end: usize) -> Option<String> {
    if end == 0 || chars[end - 1] != '!' {
        return None;
    }
    let mut k = end - 1;
    if k > 0 && chars[k - 1] == '\'' {
        // Quoted: back to the opening quote.
        let close = k - 1;
        let mut open = close;
        while open > 0 && chars[open - 1] != '\'' {
            open -= 1;
        }
        return Some(chars[open..close].iter().collect());
    }
    while k > 0 && (is_ident_char(chars[k - 1])) {
        k -= 1;
    }
    Some(chars[k..end - 1].iter().collect())
}

/// Whether a formula on `formula_sheet` references the cell `(row, col)`
/// on `cell_sheet`: an unqualified token counts only on the cell's own
/// sheet; a qualified token counts when its qualifier names the cell's
/// sheet (case-insensitively, as Excel resolves sheet names).
fn references_cell(
    formula: &str,
    formula_sheet: &str,
    cell_sheet: &str,
    row: u32,
    col: u32,
) -> bool {
    address_token_qualifiers(formula, row, col)
        .into_iter()
        .any(|qualifier| match qualifier {
            None => formula_sheet.eq_ignore_ascii_case(cell_sheet),
            Some(name) => name.eq_ignore_ascii_case(cell_sheet),
        })
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

/// What the corpus subjects' selection found.
struct Selected {
    selection: Selection,
    /// `(display path, bytes)` in rank order.
    subjects: Vec<(String, Vec<u8>)>,
}

/// Rank every readable workbook under `corpus_dir` (Excel-authored first,
/// then formula cells descending, then path) and open them in rank order
/// on the product path until `corpus_subjects` opened.
fn select_corpus_subjects(
    corpus_dir: &Path,
    options: &BenchOptions,
) -> Result<Selected, DiffError> {
    let paths = collect_workbooks(corpus_dir)?;
    let mut ranked: Vec<(Candidate, Vec<u8>)> = Vec::new();
    let mut unreadable = 0u64;
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
        let Ok(source) = open_host_owned_xlsx_source(Cursor::new(&bytes), LoadProfile::full())
        else {
            unreadable += 1;
            continue;
        };
        let oracle = Oracle::from_events(source.source_context.events());
        let formula_cells = oracle.formula_cell_count();
        if formula_cells == 0 {
            continue;
        }
        let excel_authored = source
            .source_context
            .package_snapshot()
            .parts
            .iter()
            .any(|part| {
                part.part_name.trim_start_matches('/') == "docProps/app.xml"
                    && String::from_utf8_lossy(&part.bytes)
                        .contains("<Application>Microsoft Excel</Application>")
            });
        ranked.push((
            Candidate {
                path: display,
                formula_cells,
                cells: oracle.cell_count(),
                sheets: oracle.sheets.len() as u64,
                excel_authored,
            },
            bytes,
        ));
    }
    ranked.sort_by(|(a, _), (b, _)| {
        b.excel_authored
            .cmp(&a.excel_authored)
            .then(b.formula_cells.cmp(&a.formula_cells))
            .then(a.path.cmp(&b.path))
    });

    let mut chosen = Vec::new();
    let mut skipped_open_failed = Vec::new();
    let mut subjects = Vec::new();
    for (candidate, bytes) in &ranked {
        if subjects.len() >= options.corpus_subjects {
            break;
        }
        match WorkbookSession::open_xlsx_bytes(
            format!("bench-select:{}", candidate.path),
            bytes,
            Some(candidate.path.clone()),
        ) {
            Ok(_) => {
                chosen.push(candidate.path.clone());
                subjects.push((candidate.path.clone(), bytes.clone()));
            }
            Err(error) => skipped_open_failed.push(SkippedCandidate {
                path: candidate.path.clone(),
                formula_cells: candidate.formula_cells,
                open_error_kind: error_kind(&error),
            }),
        }
    }
    Ok(Selected {
        selection: Selection {
            workbooks_scanned: paths.len() as u64,
            unreadable,
            ranked: ranked.into_iter().map(|(candidate, _)| candidate).collect(),
            chosen,
            skipped_open_failed,
        },
        subjects,
    })
}

/// What stops one subject (and therefore the run: a baseline with a
/// missing subject is not a baseline). The session error is boxed: host-core
/// carries its typed engine error by value (248 bytes), and this crate's
/// `Result`s would otherwise all be that wide.
#[derive(Debug)]
pub enum BenchError {
    Session(Box<WorkbookSessionError>),
    /// The file names a sheet the engine did not enumerate.
    SheetNotFound(String),
}

impl From<WorkbookSessionError> for BenchError {
    fn from(error: WorkbookSessionError) -> Self {
        Self::Session(Box::new(error))
    }
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session(error) => write!(f, "{error:?}"),
            Self::SheetNotFound(name) => write!(f, "engine enumerates no sheet named {name:?}"),
        }
    }
}

/// The engine sheet node for an oracle sheet name.
fn sheet_node(session: &WorkbookSession, name: &str) -> Result<TreeNodeId, BenchError> {
    let rows = session.sheets()?;
    rows.iter()
        .find(|row| row.display_name == name)
        .map(|row| row.node_id)
        .ok_or_else(|| BenchError::SheetNotFound(name.to_string()))
}

/// A formula cell that reads the edited cell, read back after the series.
struct Dependent {
    node: TreeNodeId,
    sheet_name: String,
    row: u32,
    col: u32,
    formula: String,
}

/// The edit target and its dependent on an opened corpus workbook.
struct CorpusEditPlan {
    sheet_index: usize,
    row: u32,
    col: u32,
    base_number: f64,
    target: EditTarget,
    dependent: Option<(usize, u32, u32, String)>,
}

fn plan_corpus_edit(oracle: &Oracle, events: &[DocumentEvent]) -> CorpusEditPlan {
    let literals = literal_cells(events, &oracle.string_table);
    // A formula on any sheet that references the literal, if any (own sheet
    // first, then the others in stream order).
    let dependent_of = |cell: &LiteralCell| {
        let cell_sheet = oracle.sheets[cell.sheet_index].name.as_str();
        let order = std::iter::once(cell.sheet_index)
            .chain((0..oracle.sheets.len()).filter(|index| *index != cell.sheet_index));
        order.into_iter().find_map(|index| {
            let sheet = &oracle.sheets[index];
            sheet
                .formula_cells
                .iter()
                .find(|formula| {
                    formula.text.as_deref().is_some_and(|text| {
                        references_cell(text, &sheet.name, cell_sheet, cell.row, cell.col)
                    })
                })
                .map(|formula| {
                    (
                        index,
                        formula.row,
                        formula.col,
                        formula.text.clone().unwrap_or_default(),
                    )
                })
        })
    };
    if let Some((cell, dependent)) = literals
        .iter()
        .find_map(|cell| dependent_of(cell).map(|dependent| (cell, dependent)))
    {
        return CorpusEditPlan {
            sheet_index: cell.sheet_index,
            row: cell.row,
            col: cell.col,
            base_number: cell.number,
            target: EditTarget::ExistingLiteral {
                referenced_by_formula: true,
                original: cell.original.clone(),
            },
            dependent: Some(dependent),
        };
    }
    if let Some(cell) = literals.first() {
        return CorpusEditPlan {
            sheet_index: cell.sheet_index,
            row: cell.row,
            col: cell.col,
            base_number: cell.number,
            target: EditTarget::ExistingLiteral {
                referenced_by_formula: false,
                original: cell.original.clone(),
            },
            dependent: None,
        };
    }
    // No literal anywhere: a new literal in column A of the first sheet,
    // below every occupied row of that column.
    let sheet_index = 0;
    let occupied_rows = oracle
        .sheets
        .first()
        .map(|sheet| {
            sheet
                .formula_cells
                .iter()
                .filter(|cell| cell.col == 1)
                .map(|cell| cell.row)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    CorpusEditPlan {
        sheet_index,
        row: occupied_rows + 1,
        col: 1,
        base_number: 0.0,
        target: EditTarget::NewLiteralCell,
        dependent: None,
    }
}

/// The Automatic-mode edit series plus the Manual/F9 cycles, shared by the
/// corpus and synthetic subjects once a session exists.
struct EditSeries<'a> {
    session: &'a mut WorkbookSession,
    sheet: TreeNodeId,
    sheet_name: String,
    row: u32,
    col: u32,
    base_number: f64,
    target: EditTarget,
    dependent: Option<Dependent>,
}

/// The Automatic edit repeats, the dependent read-back, then enough
/// Manual-edit + F9 cycles to bring `full_recalc` up to
/// `options.full_recalc_repeats` entries (a synthetic subject arrives with
/// its cold F9 already recorded).
fn run_edit_series(
    series: EditSeries<'_>,
    options: &BenchOptions,
    full_recalc: &mut Vec<FullRecalcSample>,
) -> Result<EditTimings, BenchError> {
    let EditSeries {
        session,
        sheet,
        sheet_name,
        row,
        col,
        base_number,
        target,
        dependent,
    } = series;
    let value_before = dependent
        .as_ref()
        .map(|d| {
            session
                .grid_cell_value(d.node, d.row, d.col)
                .map_err(BenchError::from)
        })
        .transpose()?
        .flatten()
        .map(|value| render_value(&value));

    session.set_calc_mode(CalcModeProjection::Automatic)?;
    let mut entered = Vec::new();
    let mut samples = Vec::new();
    let mut post_edit_ticks = Vec::new();
    for k in 1..=options.edit_repeats {
        let text = format!("{}", base_number + f64::from(k));
        let (outcome, ms) = timed(|| {
            session
                .enter_grid_cell(sheet, row, col, &text)
                .map_err(BenchError::from)
        });
        let outcome = outcome?;
        post_edit_ticks.push(tick_in_view(view_of(&outcome), row, col));
        entered.push(text);
        samples.push(ms);
    }

    let dependent = dependent
        .map(|d| {
            let view = session.grid_view(d.node)?;
            let cell = view
                .cells
                .iter()
                .find(|cell| cell.address.row == d.row && cell.address.col == d.col);
            Ok::<_, BenchError>(DependentReadback {
                sheet: d.sheet_name,
                address: a1(d.row, d.col),
                formula: d.formula,
                value_before: value_before.clone(),
                value_after: cell.map(|cell| render_value(&cell.value)),
                provenance_after: cell.map(|cell| provenance_text(cell.provenance)),
            })
        })
        .transpose()?;

    // Manual edit + F9 cycles: each F9 rebuilds the derived state from
    // authored input (a mark-all) and then drains the seed (a second pass,
    // the one the outcome's cells_evaluated counts). Never cold here: a
    // synthetic subject's cold F9 preceded the series and a corpus
    // subject's cold recalc was its Automatic open-recalc.
    let already = full_recalc.len() as u32;
    for c in 0..options.full_recalc_repeats.saturating_sub(already) {
        session.set_calc_mode(CalcModeProjection::Manual)?;
        let text = format!(
            "{}",
            base_number + f64::from(options.edit_repeats) + f64::from(c) + 1.0
        );
        session.enter_grid_cell(sheet, row, col, &text)?;
        let (outcome, ms) = timed(|| session.recalculate().map_err(BenchError::from));
        full_recalc.push(full_recalc_sample(&outcome, ms, false));
    }

    Ok(EditTimings {
        sheet: sheet_name,
        address: a1(row, col),
        target,
        entered,
        wall: Samples::from_ms(samples),
        post_edit_ticks,
        dependent,
        lane_outcome: None,
        lane_outcome_note: "GridSeededLaneOutcome is not exposed by OxCalcDocumentContext \
                            (private GridDerivedState::last_lane_outcome); see the \
                            HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor handover"
            .to_string(),
    })
}

fn full_recalc_sample(
    outcome: &Result<WorkbookRecalcOutcome, BenchError>,
    wall_ms: f64,
    cold: bool,
) -> FullRecalcSample {
    match outcome {
        Ok(outcome) => FullRecalcSample {
            cold,
            wall_ms,
            cells_evaluated: outcome.total_cells_evaluated(),
            drained_sheets: outcome.drained.len() as u64,
            tick_id: outcome.tick_id,
            error: None,
        },
        Err(error) => FullRecalcSample {
            cold,
            wall_ms,
            cells_evaluated: 0,
            drained_sheets: 0,
            tick_id: None,
            error: Some(format!("{error:?}")),
        },
    }
}

/// Bench one corpus workbook: timed opens, then the edit series on its
/// input cell, then the F9 cycles.
fn bench_corpus_subject(
    path: &str,
    bytes: &[u8],
    excel_authored: bool,
    options: &BenchOptions,
) -> Result<SubjectReport, BenchError> {
    let mut oxdoc_only = Vec::new();
    let mut product_open = Vec::new();
    let mut last_session: Option<WorkbookSession> = None;
    for _ in 0..options.open_repeats.max(1) {
        let (source, ms) =
            timed(|| open_host_owned_xlsx_source(Cursor::new(bytes), LoadProfile::full()));
        source.map_err(|error| BenchError::from(WorkbookSessionError::Xlsx(error)))?;
        oxdoc_only.push(ms);
        let (session, ms) = timed(|| {
            WorkbookSession::open_xlsx_bytes(format!("bench:{path}"), bytes, Some(path.to_string()))
                .map_err(BenchError::from)
        });
        last_session = Some(session?);
        product_open.push(ms);
    }
    let mut session = last_session.expect("open_repeats.max(1) opens at least once");
    let report = session
        .load_report()
        .expect("a session opened from xlsx bytes carries its load report");
    let oxdoc_only = Samples::from_ms(oxdoc_only);
    let product_open = Samples::from_ms(product_open);
    let open = OpenTimings {
        ingest_plus_open_recalc_median_ms: product_open.median_ms - oxdoc_only.median_ms,
        oxdoc_only,
        product_open,
        engine_recalcs_at_load: report.engine_recalcs_at_load,
        recalc_path: format!("{:?}", report.recalc_path),
        formulas_bound: report.formulas_bound,
        bind_degradations: report.bind_degradations.len() as u32,
        not_calc_modeled: report.not_calc_modeled,
    };

    let source = session
        .xlsx_source()
        .expect("a session opened from xlsx bytes carries its OxDoc source");
    let events = source.source_context.events().to_vec();
    let oracle = Oracle::from_events(&events);
    let plan = plan_corpus_edit(&oracle, &events);
    let sheet_name = oracle.sheets[plan.sheet_index].name.clone();
    let sheet = sheet_node(&session, &sheet_name)?;
    let dependent = plan
        .dependent
        .map(|(index, row, col, formula)| {
            let sheet_name = oracle.sheets[index].name.clone();
            Ok::<_, BenchError>(Dependent {
                node: sheet_node(&session, &sheet_name)?,
                sheet_name,
                row,
                col,
                formula,
            })
        })
        .transpose()?;

    let mut full_recalc = Vec::new();
    let edit = run_edit_series(
        EditSeries {
            session: &mut session,
            sheet,
            sheet_name,
            row: plan.row,
            col: plan.col,
            base_number: plan.base_number,
            target: plan.target,
            dependent,
        },
        options,
        &mut full_recalc,
    )?;

    Ok(SubjectReport {
        id: format!("corpus:{path}"),
        kind: SubjectKind::Corpus {
            path: path.to_string(),
            sha256: sha256_hex(bytes),
            excel_authored,
            calc_mode: oracle.calc_mode.map(|mode| format!("{mode:?}")),
        },
        sheets: oracle.sheets.len() as u64,
        cells: oracle.cell_count(),
        formula_cells: oracle.formula_cell_count(),
        open: Some(open),
        setup: None,
        edit,
        full_recalc,
        escalation: EscalationSummary::from_accounting(&session.recalc_accounting()),
    })
}

/// The synthetic fill-down: `A{r}` = `r`, `B{r}` = `=A{r}*2`, `rows` rows,
/// authored under `Manual`; the first F9 is the cold full recalc; the edit
/// is `A1` (dependent `B1`).
fn bench_synthetic_filldown(options: &BenchOptions) -> Result<SubjectReport, BenchError> {
    let rows = options.filldown_rows.max(1);
    let mut session = WorkbookSession::create("bench:filldown")?;
    let sheet = session.add_sheet("Sheet1")?;
    session.set_calc_mode(CalcModeProjection::Manual)?;
    let (setup, setup_ms) = timed(|| {
        for r in 1..=rows {
            session.enter_grid_cell(sheet, r, 1, &r.to_string())?;
            session.enter_grid_cell(sheet, r, 2, &format!("=A{r}*2"))?;
        }
        Ok::<_, BenchError>(())
    });
    setup?;
    let verb_calls = u64::from(rows) * 2;
    let mut full_recalc = Vec::new();
    let (outcome, ms) = timed(|| session.recalculate().map_err(BenchError::from));
    full_recalc.push(full_recalc_sample(&outcome, ms, true));

    let edit = run_edit_series(
        EditSeries {
            session: &mut session,
            sheet,
            sheet_name: "Sheet1".to_string(),
            row: 1,
            col: 1,
            base_number: 1.0,
            target: EditTarget::ExistingLiteral {
                referenced_by_formula: true,
                original: "1".to_string(),
            },
            dependent: Some(Dependent {
                node: sheet,
                sheet_name: "Sheet1".to_string(),
                row: 1,
                col: 2,
                formula: "=A1*2".to_string(),
            }),
        },
        options,
        &mut full_recalc,
    )?;

    Ok(SubjectReport {
        id: format!("synthetic:filldown-{rows}"),
        kind: SubjectKind::Synthetic {
            shape: format!(
                "one sheet; A1..A{rows} literals 1..{rows}; B1..B{rows} = =A{{r}}*2 ({rows} formula cells)"
            ),
        },
        sheets: 1,
        cells: verb_calls,
        formula_cells: u64::from(rows),
        open: None,
        setup: Some(SetupTimings {
            verb_calls,
            wall_ms: setup_ms,
            mean_ms_per_verb: setup_ms / verb_calls as f64,
        }),
        edit,
        full_recalc,
        escalation: EscalationSummary::from_accounting(&session.recalc_accounting()),
    })
}

/// The synthetic two-sheet workbook: `Inputs!A{r}` = `r`, `Calc!A{r}` =
/// `=Inputs!A{r}*2`; the edit is `Inputs!A1` (dependent `Calc!A1`, on the
/// other sheet, so the edit's propagation runs
/// `recalculate_sheet_with_cross_sheet_view`).
fn bench_synthetic_cross_sheet(options: &BenchOptions) -> Result<SubjectReport, BenchError> {
    let rows = options.cross_sheet_rows.max(1);
    let mut session = WorkbookSession::create("bench:cross-sheet")?;
    let inputs = session.add_sheet("Inputs")?;
    let calc = session.add_sheet("Calc")?;
    session.set_calc_mode(CalcModeProjection::Manual)?;
    let (setup, setup_ms) = timed(|| {
        for r in 1..=rows {
            session.enter_grid_cell(inputs, r, 1, &r.to_string())?;
        }
        for r in 1..=rows {
            session.enter_grid_cell(calc, r, 1, &format!("=Inputs!A{r}*2"))?;
        }
        Ok::<_, BenchError>(())
    });
    setup?;
    let verb_calls = u64::from(rows) * 2;
    let mut full_recalc = Vec::new();
    let (outcome, ms) = timed(|| session.recalculate().map_err(BenchError::from));
    full_recalc.push(full_recalc_sample(&outcome, ms, true));

    let edit = run_edit_series(
        EditSeries {
            session: &mut session,
            sheet: inputs,
            sheet_name: "Inputs".to_string(),
            row: 1,
            col: 1,
            base_number: 1.0,
            target: EditTarget::ExistingLiteral {
                referenced_by_formula: true,
                original: "1".to_string(),
            },
            dependent: Some(Dependent {
                node: calc,
                sheet_name: "Calc".to_string(),
                row: 1,
                col: 1,
                formula: "=Inputs!A1*2".to_string(),
            }),
        },
        options,
        &mut full_recalc,
    )?;

    Ok(SubjectReport {
        id: format!("synthetic:cross-sheet-{rows}"),
        kind: SubjectKind::Synthetic {
            shape: format!(
                "two sheets; Inputs!A1..A{rows} literals 1..{rows}; Calc!A1..A{rows} = =Inputs!A{{r}}*2 ({rows} cross-sheet formula cells)"
            ),
        },
        sheets: 2,
        cells: verb_calls,
        formula_cells: u64::from(rows),
        open: None,
        setup: Some(SetupTimings {
            verb_calls,
            wall_ms: setup_ms,
            mean_ms_per_verb: setup_ms / verb_calls as f64,
        }),
        edit,
        full_recalc,
        escalation: EscalationSummary::from_accounting(&session.recalc_accounting()),
    })
}

/// Run the bench: select and measure the corpus subjects, author and
/// measure the two synthetic subjects, probe the machine, write
/// `<out_dir>/bench.v1.json`, and return the report.
pub fn run_bench(
    corpus_dir: &Path,
    out_dir: &Path,
    options: &BenchOptions,
) -> Result<BenchReport, DiffError> {
    let started = Instant::now();
    let started_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let machine = probe_machine();

    let selected = select_corpus_subjects(corpus_dir, options)?;
    let excel_authored_of: BTreeMap<&str, bool> = selected
        .selection
        .ranked
        .iter()
        .map(|candidate| (candidate.path.as_str(), candidate.excel_authored))
        .collect();
    let mut subjects = Vec::new();
    for (path, bytes) in &selected.subjects {
        let excel_authored = excel_authored_of
            .get(path.as_str())
            .copied()
            .unwrap_or(false);
        let subject = bench_corpus_subject(path, bytes, excel_authored, options)
            .map_err(|error| DiffError::Invariant(format!("{path}: {error}")))?;
        subjects.push(subject);
    }
    subjects.push(
        bench_synthetic_filldown(options)
            .map_err(|error| DiffError::Invariant(format!("synthetic filldown: {error}")))?,
    );
    subjects.push(
        bench_synthetic_cross_sheet(options)
            .map_err(|error| DiffError::Invariant(format!("synthetic cross-sheet: {error}")))?,
    );

    // W012-1.7: the run-wide fold of every subject's accounting, with the
    // same identities `diff` checks before writing.
    let escalation = EscalationSummary::fold(subjects.iter().map(|subject| &subject.escalation));
    for subject in &subjects {
        subject
            .escalation
            .check_identities(&subject.id)
            .map_err(DiffError::Invariant)?;
    }
    escalation
        .check_identities("bench totals")
        .map_err(DiffError::Invariant)?;

    let (peak_working_set_bytes, peak_working_set_source) = match peak_working_set_bytes() {
        Ok(bytes) => (
            Some(bytes),
            "Get-Process PeakWorkingSet64 of this process, sampled after every subject ran \
             (process-wide; includes OxDoc parses, the retained revisions per edit, and the \
             artifact assembly)"
                .to_string(),
        ),
        Err(reason) => (None, format!("unavailable: {reason}")),
    };

    let report = BenchReport {
        schema: BENCH_SCHEMA.to_string(),
        machine,
        run: BenchRunSettings {
            corpus_dir: corpus_dir.to_string_lossy().replace('\\', "/"),
            out_dir: out_dir.to_string_lossy().replace('\\', "/"),
            started_unix_secs,
            total_wall_ms: started.elapsed().as_secs_f64() * 1000.0,
            corpus_subjects: options.corpus_subjects as u64,
            filldown_rows: u64::from(options.filldown_rows),
            cross_sheet_rows: u64::from(options.cross_sheet_rows),
            open_repeats: u64::from(options.open_repeats),
            edit_repeats: u64::from(options.edit_repeats),
            full_recalc_repeats: u64::from(options.full_recalc_repeats),
            validation_mode: HOST_VALIDATION_MODE.to_string(),
        },
        observability: Observability {
            lane_outcome_exposed: false,
            phase_timers_exposed: false,
            edit_cells_evaluated_exposed: false,
            allocation_counter: "not available: the workspace forbids unsafe_code, and a counting \
                                 GlobalAlloc needs `unsafe impl`; peak working set is the memory \
                                 number instead"
                .to_string(),
            notes: vec![
                "Every F9 (`recalculate`) rebuilds each seeded sheet's derived state from authored \
                 input (`rebuild_from_input`: build + a whole-sheet mark-all with no retained \
                 valuation), then re-seeds the accumulated edits and runs a second, seeded recalc. \
                 The F9 wall-clock covers both passes; `cells_evaluated` reports only the second \
                 (seeded) pass, so the rebuild's mark-all is unaccounted in the counter."
                    .to_string(),
                "The first recalc of every rebuilt derived state runs the reference twin under the \
                 host's DualValidatedSampled { one_in: 16 }; every F9 sample and every Automatic \
                 open-recalc therefore includes the oracle sweep, uncounted."
                    .to_string(),
                "`cells_evaluated` on an F9 counts only the sheets that carried accumulated seeds; \
                 sheets recalculated by cross-sheet propagation are not in the outcome — and on a \
                 workbook with cross-sheet edges the propagation re-enters the origin sheet with a \
                 mark-all, so the origin's counter is every one of its cells, not the edit's cone."
                    .to_string(),
                "The Automatic edit wall-clock is the whole `enter_grid_cell` verb: OxFml \
                 interpretation, the authored write, the content-address identity folds, the \
                 inline recalc, cross-sheet propagation, the revision mint, and the returned \
                 post-edit readout of the edited sheet."
                    .to_string(),
                "`escalation` (W012-1.7, dtc-5zet.14) is host-core's RecalcAccounting per subject \
                 and folded for the run: `escalation_rate` is null while every recalc's lane \
                 outcome is unobserved (the same accessor gap as `lane_outcome`); \
                 `escalation_rate_lower_bound` counts the verbs that rebuild every drained sheet \
                 by contract (each F9 drain; an Automatic Delete) over the recalcs the host \
                 issued (the Automatic edits plus those drains). The open-recalc is reported \
                 apart and is in neither number."
                    .to_string(),
            ],
        },
        selection: selected.selection,
        subjects,
        escalation,
        peak_working_set_bytes,
        peak_working_set_source,
    };

    std::fs::create_dir_all(out_dir).map_err(|error| DiffError::Io {
        path: out_dir.to_path_buf(),
        error,
    })?;
    let artifact = out_dir.join(BENCH_ARTIFACT_FILE_NAME);
    let json = serde_json::to_string_pretty(&report).map_err(DiffError::Json)?;
    std::fs::write(&artifact, json).map_err(|error| DiffError::Io {
        path: artifact,
        error,
    })?;
    Ok(report)
}

/// The stdout summary: machine, selection, then one block per subject with
/// every headline number and its counter.
pub fn write_bench_summary(
    report: &BenchReport,
    artifact_path: &Path,
    out: &mut impl Write,
) -> io::Result<()> {
    writeln!(
        out,
        "dnacalc-conform bench — grid-engine wall-clock baseline ({})",
        report.schema
    )?;
    let m = &report.machine;
    writeln!(
        out,
        "machine: {} {} — cpu {} ({} logical) — ram {} — rustc {} — debug_assertions {}",
        m.os,
        m.arch,
        m.cpu_name
            .as_deref()
            .or(m.processor_identifier.as_deref())
            .unwrap_or("?"),
        m.logical_cpus.map_or("?".to_string(), |n| n.to_string()),
        m.total_physical_memory_bytes
            .map_or("?".to_string(), |b| format!(
                "{:.1} GiB",
                b as f64 / (1u64 << 30) as f64
            )),
        m.rustc_version.as_deref().unwrap_or("?"),
        m.debug_assertions
    )?;
    if !m.probe_failures.is_empty() {
        writeln!(out, "  probe failures: {}", m.probe_failures.join(" | "))?;
    }
    let s = &report.selection;
    writeln!(
        out,
        "corpus: {} — {} workbooks scanned, {} unreadable, {} ranked (formula cells > 0); chosen {}: {}",
        report.run.corpus_dir,
        s.workbooks_scanned,
        s.unreadable,
        s.ranked.len(),
        s.chosen.len(),
        s.chosen.join(", ")
    )?;
    for skipped in &s.skipped_open_failed {
        writeln!(
            out,
            "  skipped (open failed on the product path): {} [{} formula cells] {}",
            skipped.path, skipped.formula_cells, skipped.open_error_kind
        )?;
    }
    writeln!(
        out,
        "observability: lane_outcome_exposed={} phase_timers_exposed={} edit_cells_evaluated_exposed={}",
        report.observability.lane_outcome_exposed,
        report.observability.phase_timers_exposed,
        report.observability.edit_cells_evaluated_exposed
    )?;
    for subject in &report.subjects {
        writeln!(out, "subject {}", subject.id)?;
        match &subject.kind {
            SubjectKind::Corpus {
                sha256,
                excel_authored,
                calc_mode,
                ..
            } => writeln!(
                out,
                "  corpus workbook sha256 {sha256}; excel_authored {excel_authored}; calc_mode {}; sheets {}, cells {}, formula_cells {}",
                calc_mode.as_deref().unwrap_or("?"),
                subject.sheets,
                subject.cells,
                subject.formula_cells
            )?,
            SubjectKind::Synthetic { shape } => writeln!(
                out,
                "  synthetic: {shape}; sheets {}, cells {}, formula_cells {}",
                subject.sheets, subject.cells, subject.formula_cells
            )?,
        }
        if let Some(open) = &subject.open {
            writeln!(
                out,
                "  open: oxdoc_only median {:.3} ms (min {:.3}, max {:.3}); product_open median {:.3} ms (min {:.3}, max {:.3}); ingest+open-recalc (derived) {:.3} ms; engine_recalcs_at_load {}; recalc_path {}; formulas_bound {}",
                open.oxdoc_only.median_ms,
                open.oxdoc_only.min_ms,
                open.oxdoc_only.max_ms,
                open.product_open.median_ms,
                open.product_open.min_ms,
                open.product_open.max_ms,
                open.ingest_plus_open_recalc_median_ms,
                open.engine_recalcs_at_load,
                open.recalc_path,
                open.formulas_bound
            )?;
        }
        if let Some(setup) = &subject.setup {
            writeln!(
                out,
                "  setup: {} enter_grid_cell calls under Manual in {:.1} ms ({:.4} ms/verb)",
                setup.verb_calls, setup.wall_ms, setup.mean_ms_per_verb
            )?;
        }
        for (i, sample) in subject.full_recalc.iter().enumerate() {
            writeln!(
                out,
                "  full_recalc[{i}]{}: {:.3} ms; cells_evaluated {}; drained_sheets {}; tick {}{}",
                if sample.cold { " (cold)" } else { "" },
                sample.wall_ms,
                sample.cells_evaluated,
                sample.drained_sheets,
                sample.tick_id.map_or("none".to_string(), |t| t.to_string()),
                sample
                    .error
                    .as_deref()
                    .map_or(String::new(), |e| format!("; ERROR {e}"))
            )?;
        }
        let e = &subject.edit;
        writeln!(
            out,
            "  edit {}!{} ({:?}): median {:.3} ms (min {:.3}, max {:.3}) over {} repeats; post_edit_ticks {:?}; lane_outcome {}",
            e.sheet,
            e.address,
            e.target,
            e.wall.median_ms,
            e.wall.min_ms,
            e.wall.max_ms,
            e.wall.samples_ms.len(),
            e.post_edit_ticks,
            e.lane_outcome.as_deref().unwrap_or("UNOBSERVABLE")
        )?;
        if let Some(dependent) = &e.dependent {
            writeln!(
                out,
                "    dependent {}!{} {}: {} -> {} ({})",
                dependent.sheet,
                dependent.address,
                dependent.formula,
                dependent.value_before.as_deref().unwrap_or("<none>"),
                dependent.value_after.as_deref().unwrap_or("<none>"),
                dependent.provenance_after.as_deref().unwrap_or("<none>")
            )?;
        }
        subject.escalation.write_line("  escalation:", out)?;
    }
    report.escalation.write_line("ESCALATION (run-wide)", out)?;
    writeln!(
        out,
        "peak_working_set: {} ({})",
        report
            .peak_working_set_bytes
            .map_or("unavailable".to_string(), |b| format!(
                "{b} bytes = {:.1} MiB",
                b as f64 / (1u64 << 20) as f64
            )),
        report.peak_working_set_source
    )?;
    writeln!(out, "total wall {:.1} ms", report.run.total_wall_ms)?;
    writeln!(
        out,
        "artifact: {}",
        artifact_path.to_string_lossy().replace('\\', "/")
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_cell_matches_whole_tokens_with_sheet_qualifiers() {
        let same = |formula: &str, row, col| references_cell(formula, "Sheet1", "Sheet1", row, col);
        assert!(same("A1*3", 1, 1));
        assert!(same("SUM(A1:A3)", 1, 1));
        assert!(same("$A$1+1", 1, 1));
        assert!(same("a1+B2", 1, 1));
        assert!(same("SUM(A1,,B1)", 1, 2));
        assert!(same("Sheet1!A1*2", 1, 1), "self-qualified");
        assert!(
            same("sheet1!A1*2", 1, 1),
            "sheet names are case-insensitive"
        );
        assert!(!same("A10*2", 1, 1), "A10 is not A1");
        assert!(!same("AA1*2", 1, 1), "AA1 is not A1");
        assert!(!same("A1(", 1, 1), "a function-like token");
        assert!(!same("B1*3", 1, 1));
        assert!(!same("SUM(A2:A9)", 5, 1), "range membership is not scanned");
        assert!(!same("Sheet2!A1*5", 1, 1), "qualified to another sheet");
        // From another sheet: only a qualified token counts.
        assert!(references_cell("Sheet1!A1*5", "Sheet2", "Sheet1", 1, 1));
        assert!(references_cell("'My Sheet'!$A$1", "Calc", "My Sheet", 1, 1));
        assert!(!references_cell("A1*5", "Sheet2", "Sheet1", 1, 1));
        assert_eq!(
            address_token_qualifiers("A1+Sheet2!A1+'Two words'!A1", 1, 1),
            vec![
                None,
                Some("Sheet2".to_string()),
                Some("Two words".to_string())
            ]
        );
    }

    #[test]
    fn default_options_are_the_baseline_shape() {
        let options = BenchOptions::default();
        assert_eq!(options.corpus_subjects, 3);
        assert_eq!(options.filldown_rows, 2_000);
        assert_eq!(options.cross_sheet_rows, 500);
        assert_eq!(options.open_repeats, 5);
        assert_eq!(options.edit_repeats, 5);
        assert_eq!(options.full_recalc_repeats, 3);
    }
}

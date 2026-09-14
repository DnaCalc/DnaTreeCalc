//! `divergence.v1.json` — the retained artifact of one `diff` run.
//!
//! Field order is the reading order: the PRIMARY number, `coverage`
//! (`engine_calculated / formula_cells`, corpus-wide), comes first; then the
//! corpus totals with every verdict broken down by reason or class; then the
//! ten worst-covered workbooks; the corpus identity (every workbook's sha256
//! and the sha256 of that sorted set); the run settings; and finally every
//! workbook with every formula cell's verdict, so a `Match` can be audited
//! back to its `Calculated { tick_id }`.
//!
//! "Green" must always mean *we evaluated X% of the corpus and matched*,
//! never *we matched what we chose to evaluate* — which is why a class
//! count is never printed without its denominator.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The schema tag every artifact carries.
pub const SCHEMA: &str = "dnacalc-conform/divergence.v1";

/// The artifact's file name inside `--out`.
pub const ARTIFACT_FILE_NAME: &str = "divergence.v1.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivergenceReport {
    pub schema: String,
    /// PRIMARY: `totals.engine_calculated / totals.formula_cells`; `0.0`
    /// when the corpus has no formula cells.
    pub coverage: f64,
    pub totals: Totals,
    /// The ten workbooks with the lowest coverage (formula-bearing workbooks
    /// only), lowest first.
    pub worst_covered: Vec<WorstCovered>,
    pub corpus: CorpusIdentity,
    pub run: RunSettings,
    pub workbooks: Vec<WorkbookReport>,
}

/// Corpus-wide sums of the per-workbook counts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Totals {
    pub workbooks: u64,
    /// Opened through `WorkbookSession::open_xlsx_bytes` (OxDoc + ingest).
    pub opened: u64,
    /// OxDoc opened the bytes but the engine refused the stream; the formula
    /// cells still count in the denominator (`not_evaluated:open_failed:*`).
    pub open_failed: u64,
    /// OxDoc itself rejected the bytes: no event stream, so no formula-cell
    /// count is known. Reported, never silently dropped — but these
    /// workbooks contribute NOTHING to the denominator.
    pub unreadable: u64,
    pub sheets: u64,
    pub cells: u64,
    /// The denominator: every `CellPayload::Formula` in every readable
    /// workbook's event stream.
    pub formula_cells: u64,
    /// The numerator: formula cells whose published provenance is
    /// `Calculated { .. }`. Equals `matched + mismatch_total + excluded_total`.
    pub engine_calculated: u64,
    pub coverage: f64,
    pub not_evaluated: BTreeMap<String, u64>,
    pub not_evaluated_total: u64,
    pub excluded: BTreeMap<String, u64>,
    pub excluded_total: u64,
    #[serde(rename = "match")]
    pub matched: u64,
    pub mismatch: BTreeMap<String, u64>,
    pub mismatch_total: u64,
    /// Mismatches in the `Unclassified` class. Must be zero.
    pub unclassified: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorstCovered {
    pub path: String,
    pub coverage: f64,
    pub formula_cells: u64,
    pub engine_calculated: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorpusIdentity {
    pub workbooks: u64,
    /// Every workbook's sha256 (hex), sorted.
    pub sha256_set: Vec<String>,
    /// sha256 of `sha256_set` joined by `\n` — one line that identifies the
    /// corpus this run measured.
    pub set_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSettings {
    /// The `--corpus` argument as given (not canonicalized: no machine paths
    /// are invented, but the caller's own path is retained).
    pub corpus_dir: String,
    pub out_dir: String,
    /// Whether `recalculate()` was called after every open (always true on
    /// the product path; false only under the test-only `--no-force-recalc`).
    pub forced_recalc: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbookReport {
    /// Path relative to the corpus dir, `/`-separated.
    pub path: String,
    pub sha256: String,
    pub open: OpenStatus,
    /// The file's `<calcPr calcMode>` (`Automatic` / `Manual`), from the
    /// header event; `None` when unreadable.
    pub calc_mode: Option<String>,
    pub forced_recalc: bool,
    pub sheets: u64,
    pub cells: u64,
    pub formula_cells: u64,
    pub engine_calculated: u64,
    /// `engine_calculated / formula_cells`; `None` when the workbook has no
    /// formula cells (coverage is undefined, not 100%).
    pub coverage: Option<f64>,
    pub not_evaluated: BTreeMap<String, u64>,
    pub excluded: BTreeMap<String, u64>,
    #[serde(rename = "match")]
    pub matched: u64,
    pub mismatch: BTreeMap<String, u64>,
    pub unclassified: u64,
    pub load_report: Option<LoadReportSummary>,
    pub recalc: Option<RecalcSummary>,
    pub verdicts: Vec<CellVerdict>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OpenStatus {
    Opened,
    /// OxDoc read the bytes; the engine refused the stream. `kind` is the
    /// typed error's variant path; `error` its `Debug` rendering.
    OpenFailed {
        kind: String,
        error: String,
    },
    /// OxDoc rejected the bytes.
    Unreadable {
        kind: String,
        error: String,
    },
}

/// The engine's own `WorkbookLoadReport`, counts only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadReportSummary {
    pub sheets: u32,
    pub cells: u32,
    pub formulas_bound: u32,
    pub not_calc_modeled: u32,
    pub region_cells_unbacked: u32,
    pub external_ref_cells_pinned: u32,
    pub bind_degradations: u32,
    pub names: u32,
    pub tables: u32,
    pub engine_recalcs_at_load: u32,
    /// `Automatic` / `Manual` / `None` — the `LoadRecalcPath` the load ran.
    pub recalc_path: String,
}

/// What `WorkbookSession::recalculate()` reported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecalcSummary {
    /// The tick minted by the forced recalc; `None` when it was a no-op (an
    /// `Automatic` workbook already drained by its open-recalc).
    pub tick_id: Option<u64>,
    pub drained_sheets: u64,
    pub cells_evaluated: u64,
    /// The engine's typed error when the recalc itself failed (the ladder
    /// still runs on whatever provenance the cells carry).
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellVerdict {
    pub sheet: String,
    /// A1 notation.
    pub address: String,
    /// The formula text as the file stores it (no leading `=`).
    pub formula: Option<String>,
    /// `not_evaluated:<reason>` | `excluded:<reason>` | `match` |
    /// `mismatch:<class>`.
    pub verdict: String,
    /// The published `PublishedValueProvenance`, `Debug`-rendered; `None`
    /// when nothing was published or the workbook did not open.
    pub provenance: Option<String>,
    /// The `Calculated` tick — present iff the verdict is `match`,
    /// `mismatch:*` or `excluded:*`.
    pub tick_id: Option<u64>,
    /// The engine's published value, rendered.
    pub engine: Option<String>,
    /// Excel's cached value, rendered.
    pub excel: Option<String>,
}

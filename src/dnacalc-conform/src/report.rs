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
//!
//! W012-1.5 (dtc-5zet.12) added the reference-twin fields — additive, every
//! one `#[serde(default)]`, so a `divergence.v1.json` written before them
//! still reads back: `run.validation_mode` / `run.fail_on_twin`,
//! `totals.reference_twin_mismatches` / `totals.reference_twin_workbooks`,
//! per workbook `reference_twin` (every row the engine's differential
//! reported, whether or not the file has a formula cell there), and per
//! verdict `twin_reference` (the oracle lane's value when the row is
//! `mismatch:ReferenceTwin`).

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
    /// Every row the engine's reference-vs-optimized differential reported,
    /// summed over `workbooks[*].reference_twin.mismatches` (W012-1.5). Not
    /// a subset of `mismatch`: a twin row on a cell the ladder stopped at
    /// rung 1 (a pin) or on an address the file carries no formula cell for
    /// is counted here and nowhere else. `--fail-on-twin` exits non-zero
    /// when this is non-zero.
    #[serde(default)]
    pub reference_twin_mismatches: u64,
    /// Workbooks with at least one such row.
    #[serde(default)]
    pub reference_twin_workbooks: u64,
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
    /// The `GridEngineValidationMode` every session was opened with,
    /// `Debug`-rendered (`DualValidated` for `diff`: the reference twin runs
    /// on every recalc, W012-1.5). Absent on a pre-W012-1.5 artifact, which
    /// ran under the interactive host's `DualValidatedSampled { one_in: 16 }`.
    #[serde(default)]
    pub validation_mode: Option<String>,
    /// Whether `--fail-on-twin` was set (a non-zero `reference_twin_mismatches`
    /// then makes the binary exit `3`).
    #[serde(default)]
    pub fail_on_twin: bool,
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
    /// The engine's reference-vs-optimized differential for this workbook
    /// (W012-1.5); `None` when the workbook did not open.
    #[serde(default)]
    pub reference_twin: Option<ReferenceTwinSummary>,
    pub verdicts: Vec<CellVerdict>,
}

/// What the engine's permanent-pair differential reported for one opened
/// workbook (W012-1.5, dtc-5zet.12): every `differential_mismatches` row on
/// every engine sheet's `grid_view`, read AFTER the forced recalc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceTwinSummary {
    /// The session's `GridEngineValidationMode`, `Debug`-rendered.
    pub validation_mode: String,
    /// True iff that mode runs the reference lane on every recalc (OxCalc's
    /// `runs_reference_lane` contract: `DualValidated`), so an empty
    /// `mismatches` means *checked and clean*. Under a sampled mode the
    /// engine leaves the last checked result in place and exposes no
    /// "checked this recalc" fact to the host, so an empty list would mean
    /// nothing — the instrument therefore never runs sampled.
    pub checked_every_recalc: bool,
    pub mismatches: Vec<ReferenceTwinMismatchRow>,
}

/// One cell the two engine lanes disagreed on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceTwinMismatchRow {
    pub sheet: String,
    /// A1 notation.
    pub address: String,
    /// The reference (brute-force oracle) lane's value, rendered.
    pub reference: String,
    /// The optimized lane's value — the one the engine published — rendered.
    pub optimized: String,
    /// True when the two `CoreValue`s are equal and the engine's `CalcValue`
    /// inequality comes from the rich payload alone (see `reference_rich` /
    /// `optimized_rich`): the lanes agree on the value and disagree on its
    /// payload identity.
    pub core_equal: bool,
    /// The rich payload's variant name on each side (`Callable`, `Object`,
    /// `Presentation`, `ErrorMetadata`), `None` when the side has none.
    pub reference_rich: Option<String>,
    pub optimized_rich: Option<String>,
    /// Whether the file carries a formula cell at this address, i.e. whether
    /// a verdict row exists for it (and scored `mismatch:ReferenceTwin` if
    /// the cell was engine-`Calculated`).
    pub formula_cell: bool,
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
    /// The reference (oracle) lane's value, rendered — present iff the
    /// engine's differential disputes this cell (`mismatch:ReferenceTwin`,
    /// W012-1.5); `engine` is then the optimized lane's value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twin_reference: Option<String>,
}
